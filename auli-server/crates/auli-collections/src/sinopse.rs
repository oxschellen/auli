//! Subcomando `sinopse` — passo `auli-sinopse` (§6/§7 do plano). Carga do `.raw.json`, mescla por
//! `numero` (retomável/incremental), contagem, `--dry-run`, geração (LLM real via `auli-llm`, ou
//! `--fake` dev-only) com validação de formato e gravação atômica.
//!
//! Fronteira = snapshot tipado `Table<Consulta>` (o `.txt` é só print legível — F5). O `resumo`
//! vazio marca "pendente"; preenchido marca "reaproveitado" (sinopse já gerada ou sumário autorado
//! legado). Invariante do passo: `reaproveitados + gerados + falhas + pendentes_restantes == total`.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use auli_contract::{Consulta, Table};

use crate::domain::entities::EntityConfig;
use crate::errors::Result;

/// Versão do prompt (gravada em `SinopseInfo.prompt_versao`). Bump a cada mudança do
/// `data/prompts/sinopse.txt`. `0` é reservado ao `--fake`.
pub const SINOPSE_PROMPT_VERSION: u32 = 1;
/// Teto de entrada do corpo (chars). Acima disso, trunca com aviso no log (v1: sem chunking).
const CORPO_MAX_CHARS: usize = 24_000;

/// Opções do subcomando (parseadas no `main`).
pub struct SinopseOpts {
    pub dry_run: bool,
    pub limit: Option<usize>,
    /// `numero` a regenerar mesmo com `resumo` preenchido.
    pub force: Option<String>,
    /// Dev-only: gera placeholder determinístico em vez de LLM.
    pub fake: bool,
}

/// Único ponto que compõe o `text_to_embed` de consultas (decisão §2.5 do plano). Indexa o
/// **título** (`numero`), a **ementa** (`assunto`) e a **sinopse** (`resumo`), nesta ordem, uma por
/// linha — pulando os vazios. O `numero` entrou para que buscas que citam a consulta (ex.:
/// "CONSULTA COPAT nº 0037/26") a alcancem. Mudança de fórmula: re-vetorizar os packs de pareceres.
pub fn compose_text_to_embed(numero: &str, assunto: &str, resumo: &str) -> String {
    [numero, assunto, resumo].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join("\n")
}

/// Timestamp atual em ISO-8601 UTC (`YYYY-MM-DDTHH:MM:SSZ`), só com `std` (sem chrono — F3 não
/// acrescenta dependências). Algoritmo civil-from-days de Howard Hinnant.
fn now_iso8601() -> String {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64;
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let (h, mi, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    // civil_from_days: dias desde 1970-01-01 -> (ano, mês, dia).
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

/// Caminho do snapshot bruto (entrada) e da saída (`.raw.json` → `.json`).
fn raw_path(entity: &EntityConfig) -> PathBuf {
    PathBuf::from(&entity.data_dir).join(format!("{}-pareceres.raw.json", entity.id))
}
fn out_path(entity: &EntityConfig) -> PathBuf {
    PathBuf::from(&entity.data_dir).join(format!("{}-pareceres.json", entity.id))
}

/// `true` se o registro precisa de sinopse (resumo vazio, ignorando espaços).
fn pendente(c: &Consulta) -> bool {
    c.resumo.trim().is_empty()
}

/// Serializa a `Table` inteira para `<out>.tmp` e faz `rename` atômico sobre `<out>` (mesmo dir).
/// Queda em qualquer ponto perde no máximo o documento em voo.
fn write_atomic(out: &Path, id: &str, items: &[Consulta]) -> Result<()> {
    let table = Table::new(id, "pareceres", items.to_vec());
    let tmp = PathBuf::from(format!("{}.tmp", out.display()));
    std::fs::write(&tmp, serde_json::to_string_pretty(&table)?)?;
    std::fs::rename(&tmp, out)?;
    Ok(())
}

/// Print legível `ref/<id>-portal-pareceres.txt` (irmão de `raw/`). `data_dir` é `../data/<id>/raw`.
fn print_path(entity: &EntityConfig) -> Result<PathBuf> {
    let base = Path::new(&entity.data_dir)
        .parent()
        .ok_or_else(|| format!("data_dir sem pai: {}", entity.data_dir))?;
    Ok(base.join("ref").join(format!("{}-portal-pareceres.txt", entity.id)))
}

/// Regrava o print no formato de blocos que o `parse_pareceres` entende (round-trippável até a F6):
/// `// N` / `## pergunta:` com `descricao:`/`assunto  :`/`resumo   :` (só se não-vazio)/`link:` /
/// `## resposta:` + corpo. `sinopse_info`/`text_to_embed` NÃO vão ao print — o `.txt` é print, a
/// fonte tipada é o JSON. Escrita atômica (`.tmp` + rename).
fn write_print(path: &Path, items: &[Consulta]) -> Result<()> {
    let mut out = String::new();
    for (i, c) in items.iter().enumerate() {
        out.push_str(&format!("// {}\n", i + 1));
        out.push_str("## pergunta:\n");
        out.push_str(&format!("descricao: {}\n", c.numero));
        out.push_str(&format!("assunto  : {}\n", c.assunto));
        if !c.resumo.trim().is_empty() {
            out.push_str(&format!("resumo   : {}\n", c.resumo));
        }
        out.push_str(&format!("link: {}\n", c.link));
        out.push_str("## resposta:\n");
        out.push_str(c.corpo.trim());
        out.push_str("\n\n");
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = PathBuf::from(format!("{}.tmp", path.display()));
    std::fs::write(&tmp, out)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// Mescla o `.raw.json` (ordem = fonte da verdade) com a saída anterior, por `numero`. Registros
/// pendentes cujo anterior já tem `resumo` são reaproveitados inteiros; `--force` re-pendura o alvo.
fn merge(raw: Vec<Consulta>, prev: &HashMap<String, Consulta>, force: Option<&str>) -> Result<Vec<Consulta>> {
    let mut seen = HashSet::new();
    let mut merged = Vec::with_capacity(raw.len());
    let mut force_hit = false;
    for rc in raw {
        if !seen.insert(rc.numero.clone()) {
            return Err(format!("numero duplicado no raw: {:?} — viola a identidade da listagem.", rc.numero).into());
        }
        if force == Some(rc.numero.as_str()) {
            // Re-pendura: zera resumo/sinopse/key para forçar regeneração.
            force_hit = true;
            merged.push(Consulta { resumo: String::new(), text_to_embed: String::new(), sinopse_info: None, ..rc });
            continue;
        }
        // Reaproveita a saída anterior só quando o bruto está pendente e o anterior já tem resumo.
        if pendente(&rc)
            && let Some(prev_c) = prev.get(&rc.numero)
            && !pendente(prev_c)
        {
            merged.push(prev_c.clone());
            continue;
        }
        merged.push(rc);
    }
    if let Some(n) = force
        && !force_hit
    {
        return Err(format!("--force: numero inexistente no raw: {n:?}").into());
    }
    Ok(merged)
}

/// Placeholder determinístico (dev-only). `prompt_versao: 0` marca fake, distinguível da geração real.
fn fake_resumo(assunto: &str) -> String {
    format!("### Descrição Resumida do Assunto\n[FAKE] {assunto}\n\n### Palavras Chave do Tema\n- **fake**")
}

const SECAO_DESC: &str = "### Descrição Resumida do Assunto";
const SECAO_KW: &str = "### Palavras Chave do Tema";

/// Lê `primary` no ambiente; se ausente/vazia, cai em `fallback` (decisão D2). Erro nomeando AMBAS.
fn sinopse_env(primary: &str, fallback: &str) -> Result<String> {
    for k in [primary, fallback] {
        if let Ok(v) = std::env::var(k)
            && !v.trim().is_empty()
        {
            return Ok(v);
        }
    }
    Err(format!("variável de ambiente ausente: defina {primary} (ou {fallback}).").into())
}

/// System prompt da sinopse. `data_dir` é `../data/<id>/raw`, logo o prompt cai em
/// `../data/prompts/sinopse.txt` (dois níveis acima). Erro claro se ausente.
fn load_prompt(entity: &EntityConfig) -> Result<String> {
    let path = Path::new(&entity.data_dir)
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| format!("data_dir inesperado: {}", entity.data_dir))?
        .join("prompts/sinopse.txt");
    std::fs::read_to_string(&path)
        .map_err(|e| format!("prompt de sinopse ausente ({}): {e}", path.display()).into())
}

/// Trunca o corpo em `max` chars (v1: sem chunking). Devolve `(texto, truncou)`.
fn truncar_corpo(corpo: &str, max: usize) -> (String, bool) {
    if corpo.chars().count() <= max {
        (corpo.to_string(), false)
    } else {
        (corpo.chars().take(max).collect(), true)
    }
}

/// `true` se a resposta é a mensagem de erro-como-`Ok` do `auli-llm` (erro de API/HTTP).
fn resposta_e_erro_de_api(answer: &str) -> bool {
    answer.starts_with("Erro na chamada da API")
}

/// Valida o formato da sinopse (pura, testável): as duas seções na ordem certa, descrição
/// não-vazia e ≤ 2.000 chars, e ≥ 3 linhas de palavra-chave (`- `).
fn validar_sinopse(answer: &str) -> std::result::Result<(), String> {
    let p1 = answer.find(SECAO_DESC).ok_or_else(|| format!("seção ausente: {SECAO_DESC}"))?;
    let p2 = answer.find(SECAO_KW).ok_or_else(|| format!("seção ausente: {SECAO_KW}"))?;
    if p2 <= p1 {
        return Err("seções fora de ordem (Descrição deve vir antes de Palavras Chave)".into());
    }
    let descricao = answer[p1 + SECAO_DESC.len()..p2].trim();
    if descricao.is_empty() {
        return Err("descrição vazia".into());
    }
    let desc_chars = descricao.chars().count();
    if desc_chars > 2_000 {
        return Err(format!("descrição longa demais ({desc_chars} chars) — possível despejo do corpo"));
    }
    let keywords = answer[p2 + SECAO_KW.len()..]
        .lines()
        .filter(|l| l.trim_start().starts_with("- "))
        .count();
    if keywords < 3 {
        return Err(format!("poucas palavras-chave ({keywords}; mínimo 3)"));
    }
    Ok(())
}

/// Gera a sinopse real de um documento: trunca o corpo, chama o LLM, detecta erro-como-`Ok`,
/// valida (com **uma** re-tentativa). Devolve o `resumo` aparado, ou o motivo da falha (o chamador
/// conta a falha e segue — nunca aborta o lote).
fn gerar_sinopse(
    rt: &tokio::runtime::Runtime,
    params: &auli_llm::LlmParams,
    system_prompt: &str,
    assunto: &str,
    corpo: &str,
    numero: &str,
) -> std::result::Result<String, String> {
    let (corpo_trunc, truncou) = truncar_corpo(corpo, CORPO_MAX_CHARS);
    if truncou {
        eprintln!("ℹ️  {numero}: corpo truncado em {CORPO_MAX_CHARS} chars (v1 sem chunking).");
    }
    let user_msg = format!("Assunto: {assunto}\n\nDocumento:\n{corpo_trunc}");

    for tentativa in 1..=2u32 {
        let answer = rt
            .block_on(auli_llm::chat(params, system_prompt, &user_msg))
            .map_err(|e| format!("transporte: {e}"))?;
        if resposta_e_erro_de_api(&answer) {
            return Err(format!("API: {answer}"));
        }
        match validar_sinopse(&answer) {
            Ok(()) => return Ok(answer.trim().to_string()),
            Err(motivo) if tentativa == 2 => return Err(format!("validação: {motivo}")),
            Err(motivo) => eprintln!("↻ {numero}: validação falhou ({motivo}); re-tentando 1×."),
        }
    }
    unreachable!("o loop retorna em ambas as tentativas")
}

pub fn run(entity: &EntityConfig, opts: SinopseOpts) -> Result<()> {
    // 1. Carga do snapshot bruto.
    let raw_file = raw_path(entity);
    if !raw_file.exists() {
        return Err(format!(
            "snapshot bruto ausente: {} — rode o scraper (ou derive) antes.",
            raw_file.display()
        )
        .into());
    }
    let raw: Table<Consulta> = serde_json::from_str(&std::fs::read_to_string(&raw_file)?)?;

    // 2. Mescla por numero com a saída anterior (se houver).
    let out_file = out_path(entity);
    let prev: HashMap<String, Consulta> = if out_file.exists() {
        let t: Table<Consulta> = serde_json::from_str(&std::fs::read_to_string(&out_file)?)?;
        t.items.into_iter().map(|c| (c.numero.clone(), c)).collect()
    } else {
        HashMap::new()
    };
    let mut merged = merge(raw.items, &prev, opts.force.as_deref())?;

    // 2b. Recompõe a key de TODOS os registros com a fórmula vigente (numero + assunto + resumo) —
    //     assim a fórmula se aplica uniformemente a reaproveitados e gerados, sem depender de como/
    //     quando cada um foi produzido. Pendentes ficam com numero+assunto (resumo vazio), inofensivo
    //     (o update recusa resumo vazio). Idempotente.
    for c in &mut merged {
        c.text_to_embed = compose_text_to_embed(&c.numero, &c.assunto, &c.resumo);
    }

    // 3. Contagem (invariante dinâmico: reaproveitados + pendentes == total).
    let total = merged.len();
    let pendentes = merged.iter().filter(|c| pendente(c)).count();
    let reaproveitados = total - pendentes;
    debug_assert_eq!(reaproveitados + pendentes, total);
    println!("📊 {}: total {total} | reaproveitados {reaproveitados} | pendentes {pendentes}", entity.id);

    // 4. Dry-run: estimativa de tokens dos pendentes e retorna SEM escrever.
    if opts.dry_run {
        let chars: usize = merged
            .iter()
            .filter(|c| pendente(c))
            .map(|c| c.assunto.chars().count() + c.corpo.chars().count())
            .sum();
        let tokens_est = chars / 4;
        println!("🔎 dry-run: ~{} tokens de entrada nos {pendentes} pendentes (nada foi escrito).", milhar(tokens_est));
        return Ok(());
    }

    // 5. Config LLM: lida UMA vez, antes do loop, e SÓ na geração real (nem dry-run nem fake nem
    //    "sem pendentes" tocam env/rede). `--fake` continua dev-only, sem LLM.
    let real = !opts.fake && pendentes > 0;
    let llm = if real {
        let params = auli_llm::LlmParams {
            api_url: sinopse_env("SINOPSE_API_URL", "LLM_API_URL")?,
            api_key: sinopse_env("SINOPSE_API_KEY", "LLM_API_KEY")?,
            model: sinopse_env("SINOPSE_API_MODEL", "LLM_API_MODEL")?,
            temperature: 0.1, // fidelidade ao documento, não diversidade (mesma racional do chat)
            max_completion_tokens: 1024, // sinopse é curta (parágrafo + palavras-chave)
            timeout: Duration::from_secs(60), // corpo longo na entrada; offline, sem budget de front
        };
        let system_prompt = load_prompt(entity)?;
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
        Some((params, system_prompt, rt))
    } else {
        None
    };

    // 6. Geração (respeitando --limit) com gravação atômica após cada documento. `--limit` limita os
    //    documentos PROCESSADOS na rodada (gerados + falhas), não só os sucessos.
    let mut gerados = 0usize;
    let mut falhas = 0usize;
    let limit = opts.limit.unwrap_or(usize::MAX);
    for i in 0..merged.len() {
        if gerados + falhas >= limit {
            break;
        }
        if !pendente(&merged[i]) {
            continue;
        }
        let assunto = merged[i].assunto.clone();
        let (resumo, modelo, versao) = if let Some((params, system_prompt, rt)) = &llm {
            let corpo = merged[i].corpo.clone();
            let numero = merged[i].numero.clone();
            match gerar_sinopse(rt, params, system_prompt, &assunto, &corpo, &numero) {
                Ok(r) => (r, params.model.clone(), SINOPSE_PROMPT_VERSION),
                Err(motivo) => {
                    eprintln!("⚠️  {numero}: falha — {motivo}");
                    falhas += 1;
                    continue;
                }
            }
        } else {
            (fake_resumo(&assunto), "fake".to_string(), 0)
        };

        merged[i].resumo = resumo;
        merged[i].text_to_embed =
            compose_text_to_embed(&merged[i].numero, &merged[i].assunto, &merged[i].resumo);
        merged[i].sinopse_info = Some(auli_contract::SinopseInfo {
            modelo,
            prompt_versao: versao,
            gerada_em: now_iso8601(),
        });
        write_atomic(&out_file, &entity.id, &merged)?;
        gerados += 1;
    }

    // 6b. Promoção raw→final: SEMPRE grava o JSON mesclado (dry-run já retornou antes), inclusive com
    //     zero gerados — o caso RS legado (tudo reaproveitado) precisa promover a saída para o update
    //     ter fonte. As gravações por documento no loop permanecem (proteção contra queda); esta é a
    //     de promoção. Regrava também o print. Idempotente: rodar de novo dá o mesmo resultado.
    write_atomic(&out_file, &entity.id, &merged)?;
    let pp = print_path(entity)?;
    write_print(&pp, &merged)?;
    println!("📝 saída promovida: {} (+ print)", out_file.display());

    // 7. Relatório final + invariante de guarda do passo. Pendentes-restantes = não processados
    //    (falhas contam à parte); a soma fecha em `total`.
    let pendentes_restantes = pendentes - gerados - falhas;
    println!(
        "✅ {}: total {total} | reaproveitados {reaproveitados} | gerados {gerados} | falhas {falhas} | pendentes-restantes {pendentes_restantes}",
        entity.id
    );
    assert_eq!(
        reaproveitados + gerados + falhas + pendentes_restantes,
        total,
        "invariante de guarda violado"
    );
    Ok(())
}

/// Formata um inteiro com separador de milhar (`1234567` → `1.234.567`).
fn milhar(n: usize) -> String {
    let s = n.to_string();
    let bytes = s.as_bytes();
    let mut out = String::new();
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(*b as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn consulta(numero: &str, resumo: &str) -> Consulta {
        Consulta {
            numero: numero.into(),
            assunto: format!("assunto de {numero}"),
            resumo: resumo.into(),
            corpo: format!("corpo integral de {numero}"),
            link: format!("https://exemplo/{numero}"),
            text_to_embed: String::new(),
            sinopse_info: None,
        }
    }

    /// EntityConfig apontando para um dir temporário exclusivo do teste (limpo no início). `data_dir`
    /// termina em `/raw` (como o real `../data/<id>/raw`), para que `print_path` (irmão `ref/`) caia
    /// dentro da árvore do próprio teste — senão vários testes compartilhariam `temp/ref/` e correriam.
    fn temp_entity(tag: &str) -> EntityConfig {
        let base = std::env::temp_dir().join(format!("auli_sinopse_test_{tag}"));
        let _ = std::fs::remove_dir_all(&base);
        let raw = base.join("raw");
        std::fs::create_dir_all(&raw).unwrap();
        EntityConfig {
            id: "xx".into(),
            name: "Teste".into(),
            system_prompt: String::new(),
            data_dir: raw.to_string_lossy().into_owned(),
        }
    }

    fn write_table(entity: &EntityConfig, suffix: &str, items: Vec<Consulta>) {
        let path = PathBuf::from(&entity.data_dir).join(format!("{}-pareceres{suffix}", entity.id));
        let table = Table::new(&entity.id, "pareceres", items);
        std::fs::write(&path, serde_json::to_string_pretty(&table).unwrap()).unwrap();
    }

    fn read_out(entity: &EntityConfig) -> Vec<Consulta> {
        let t: Table<Consulta> =
            serde_json::from_str(&std::fs::read_to_string(out_path(entity)).unwrap()).unwrap();
        t.items
    }

    #[test]
    fn compose_indexa_numero_assunto_resumo() {
        assert_eq!(compose_text_to_embed("N", "A", "R"), "N\nA\nR");
        assert_eq!(compose_text_to_embed("N", "A", ""), "N\nA");
        assert_eq!(compose_text_to_embed("N", "", ""), "N");
        assert_eq!(compose_text_to_embed("", "A", "R"), "A\nR"); // vazios pulados
        assert_eq!(compose_text_to_embed("", "", ""), "");
    }

    #[test]
    fn key_recomposta_inclui_numero_nos_reaproveitados() {
        // Reaproveitado (resumo já preenchido) deve ganhar o numero na key na promoção.
        let e = temp_entity("recompoe");
        write_table(&e, ".raw.json", vec![consulta("PARECER Nº 9", "resumo autorado")]);
        run(&e, SinopseOpts { dry_run: false, limit: None, force: None, fake: false }).unwrap();
        let out = read_out(&e);
        assert_eq!(out[0].text_to_embed, "PARECER Nº 9\nassunto de PARECER Nº 9\nresumo autorado");
    }

    #[test]
    fn mescla_reaproveita_registro_com_resumo() {
        let e = temp_entity("mescla");
        write_table(&e, ".raw.json", vec![consulta("A", ""), consulta("B", ""), consulta("C", "")]);
        // Saída anterior: B já tem resumo (e sinopse_info) — deve ser reaproveitado.
        let mut b = consulta("B", "### Descrição\nresumo de B\n\n### Palavras Chave\n- **x**");
        b.sinopse_info = Some(auli_contract::SinopseInfo { modelo: "m".into(), prompt_versao: 1, gerada_em: "2026-01-01T00:00:00Z".into() });
        b.text_to_embed = "assunto de B\n...".into();
        write_table(&e, ".json", vec![b.clone()]);

        run(&e, SinopseOpts { dry_run: false, limit: None, force: None, fake: true }).unwrap();
        let out = read_out(&e);
        assert_eq!(out.len(), 3);
        // B preservado byte a byte na mescla (resumo/sinopse_info/text_to_embed do anterior).
        let out_b = out.iter().find(|c| c.numero == "B").unwrap();
        assert_eq!(out_b.resumo, b.resumo);
        assert_eq!(out_b.sinopse_info, b.sinopse_info);
        // A e C eram os 2 pendentes → gerados como fake.
        for n in ["A", "C"] {
            let c = out.iter().find(|c| c.numero == n).unwrap();
            assert!(c.resumo.contains("[FAKE]"));
            assert_eq!(c.sinopse_info.as_ref().unwrap().prompt_versao, 0);
        }
    }

    #[test]
    fn force_rependuza_registro_reaproveitavel() {
        let e = temp_entity("force");
        write_table(&e, ".raw.json", vec![consulta("A", ""), consulta("B", ""), consulta("C", "")]);
        let b = consulta("B", "resumo pronto de B");
        write_table(&e, ".json", vec![b]);
        // --force B: mesmo com resumo no anterior, B volta a pendente → 3 pendentes.
        run(&e, SinopseOpts { dry_run: false, limit: None, force: Some("B".into()), fake: true }).unwrap();
        let out = read_out(&e);
        // Todos foram (re)gerados como fake.
        assert!(out.iter().all(|c| c.resumo.contains("[FAKE]")));
    }

    #[test]
    fn duplicata_de_numero_no_raw_e_erro() {
        let e = temp_entity("dup");
        write_table(&e, ".raw.json", vec![consulta("A", ""), consulta("A", "")]);
        let err = run(&e, SinopseOpts { dry_run: false, limit: None, force: None, fake: true }).unwrap_err();
        assert!(format!("{err}").contains("duplicado"));
    }

    #[test]
    fn fake_com_limit_e_retomada() {
        let e = temp_entity("retomada");
        write_table(&e, ".raw.json", vec![consulta("A", ""), consulta("B", ""), consulta("C", "")]);
        // 1ª rodada: fake + limit 1 → 1 gerado, 2 pendentes restantes.
        run(&e, SinopseOpts { dry_run: false, limit: Some(1), force: None, fake: true }).unwrap();
        let out1 = read_out(&e);
        assert_eq!(out1.iter().filter(|c| c.resumo.contains("[FAKE]")).count(), 1);
        assert_eq!(out1.iter().filter(|c| pendente(c)).count(), 2);
        // 2ª rodada: fake sem limit → o gerado é reaproveitado; os 2 restantes geram.
        run(&e, SinopseOpts { dry_run: false, limit: None, force: None, fake: true }).unwrap();
        let out2 = read_out(&e);
        assert_eq!(out2.len(), 3);
        assert!(out2.iter().all(|c| c.resumo.contains("[FAKE]")));
    }

    #[test]
    fn dry_run_nao_escreve() {
        let e = temp_entity("dryrun");
        write_table(&e, ".raw.json", vec![consulta("A", ""), consulta("B", "")]);
        run(&e, SinopseOpts { dry_run: true, limit: None, force: None, fake: false }).unwrap();
        assert!(!out_path(&e).exists(), "dry-run não deve criar o .json");
    }

    // --- F4: validação/truncamento/erro-como-Ok (funções puras) ---

    fn sinopse_valida(n_kw: usize) -> String {
        let kws: String = (0..n_kw).map(|i| format!("- **termo{i}**\n")).collect();
        format!("{SECAO_DESC}\nParágrafo denso sobre crédito fiscal e não-cumulatividade.\n\n{SECAO_KW}\n{kws}")
    }

    #[test]
    fn validar_aprova_formato_correto() {
        assert!(validar_sinopse(&sinopse_valida(3)).is_ok());
        assert!(validar_sinopse(&sinopse_valida(12)).is_ok());
    }

    #[test]
    fn validar_reprova_secao_ausente() {
        let sem_desc = format!("{SECAO_KW}\n- **a**\n- **b**\n- **c**");
        assert!(validar_sinopse(&sem_desc).is_err());
        let sem_kw = format!("{SECAO_DESC}\nDescrição qualquer.");
        assert!(validar_sinopse(&sem_kw).is_err());
    }

    #[test]
    fn validar_reprova_ordem_invertida() {
        let invertido = format!("{SECAO_KW}\n- **a**\n- **b**\n- **c**\n\n{SECAO_DESC}\nDescrição.");
        assert!(validar_sinopse(&invertido).unwrap_err().contains("ordem"));
    }

    #[test]
    fn validar_reprova_descricao_vazia() {
        let vazia = format!("{SECAO_DESC}\n\n{SECAO_KW}\n- **a**\n- **b**\n- **c**");
        assert!(validar_sinopse(&vazia).unwrap_err().contains("vazia"));
    }

    #[test]
    fn validar_reprova_descricao_longa() {
        let longa = format!("{SECAO_DESC}\n{}\n\n{SECAO_KW}\n- **a**\n- **b**\n- **c**", "x".repeat(2_001));
        assert!(validar_sinopse(&longa).unwrap_err().contains("longa"));
    }

    #[test]
    fn validar_reprova_poucas_keywords() {
        assert!(validar_sinopse(&sinopse_valida(2)).unwrap_err().contains("palavras-chave"));
        assert!(validar_sinopse(&sinopse_valida(3)).is_ok());
    }

    #[test]
    fn trunca_corpo_no_teto() {
        let (t, cortou) = truncar_corpo("abcde", 10);
        assert_eq!(t, "abcde");
        assert!(!cortou);
        let (t, cortou) = truncar_corpo(&"x".repeat(100), 10);
        assert_eq!(t.chars().count(), 10);
        assert!(cortou);
    }

    #[test]
    fn detecta_erro_como_ok() {
        assert!(resposta_e_erro_de_api("Erro na chamada da API do modelo AI: foo!"));
        assert!(!resposta_e_erro_de_api("### Descrição Resumida do Assunto\n..."));
    }

    #[test]
    fn print_round_trip_via_parse_pareceres() {
        use crate::derive_pareceres::parse_pareceres;
        // 1 com sinopse (resumo multi-linha), 1 reaproveitada (resumo simples).
        let mut c1 = consulta(
            "PARECER Nº 1",
            "### Descrição Resumida do Assunto\nParágrafo denso sobre crédito fiscal.\n\n### Palavras Chave do Tema\n- **ICMS**\n- **crédito**",
        );
        c1.corpo = "Corpo integral do parecer 1.\n\nCom dois parágrafos.".into();
        c1.sinopse_info = Some(auli_contract::SinopseInfo {
            modelo: "m".into(),
            prompt_versao: 1,
            gerada_em: "2026-01-01T00:00:00Z".into(),
        });
        let c2 = consulta("PARECER Nº 2", "resumo simples reaproveitado");
        let items = vec![c1, c2];

        let e = temp_entity("print");
        let pp = print_path(&e).unwrap();
        write_print(&pp, &items).unwrap();
        let parsed = parse_pareceres(&std::fs::read_to_string(&pp).unwrap());

        assert_eq!(parsed.len(), 2);
        for (orig, got) in items.iter().zip(&parsed) {
            assert_eq!(got.numero, orig.numero);
            assert_eq!(got.assunto, orig.assunto);
            assert_eq!(got.resumo, orig.resumo.trim());
            assert_eq!(got.corpo, orig.corpo.trim());
            assert_eq!(got.link, orig.link);
        }
        // sinopse_info/text_to_embed não sobrevivem ao print (esperado — o .txt é print).
    }

    #[test]
    fn zero_pendentes_promove_sem_env_nem_rede() {
        // RS legado: todos os registros já têm resumo. `run` sem fake e SEM env de LLM deve
        // promover a saída (0 gerados) e regravar o print — sem tocar env/rede.
        let e = temp_entity("promove");
        let itens = vec![
            consulta("PARECER Nº 1", "resumo autorado 1"),
            consulta("PARECER Nº 2", "resumo autorado 2"),
        ];
        write_table(&e, ".raw.json", itens.clone());
        // sem env de LLM no ambiente (o teste não define SINOPSE_*/LLM_*); pendentes==0 ⇒ não lê env.
        run(&e, SinopseOpts { dry_run: false, limit: None, force: None, fake: false }).unwrap();

        let out = read_out(&e);
        assert_eq!(out.len(), 2);
        for (orig, got) in itens.iter().zip(&out) {
            assert_eq!(got.numero, orig.numero);
            assert_eq!(got.resumo, orig.resumo);
        }
        assert!(print_path(&e).unwrap().exists(), "print deve ser regravado na promoção");
    }
}
