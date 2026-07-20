//! Subcomando `sinopse` — passo `auli-sinopse`, agora **editor da árvore** (G4).
//!
//! Fonte e destino são os `.md` de `data/<id>/docs/pareceres/`: **pendente = arquivo sem a seção
//! `## sinopse`**. O passo lê cada arquivo, gera a sinopse do que falta (LLM real via `auli-llm`, ou
//! `--fake` dev-only) e regrava o próprio `.md` — frontmatter `sinopse_*` + seção — atomicamente.
//! O JSON não é mais tocado aqui; ele é só a origem estrutural da materialização (`auli update`).
//!
//! Retomada é implícita: o que já tem `## sinopse` é pulado, então re-rodar continua de onde parou.
//! `--force <numero>` re-pendura um documento específico. Invariante do passo:
//! `reaproveitados + gerados + falhas + pendentes_restantes == total`.
//!
//! Memória: os documentos são processados **um a um** (lê → gera → grava), nunca todos na RAM — a
//! árvore de SP tem 15,6 mil arquivos.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use auli_contract::mddoc;

use crate::domain::entities::EntityConfig;
use crate::errors::Result;

/// Versão do prompt (gravada em `SinopseInfo.prompt_versao`). Bump a cada mudança do
/// `data/prompts/sinopse.txt`. `0` é reservado ao `--fake`.
pub const SINOPSE_PROMPT_VERSION: u32 = 1;
/// Teto de entrada do corpo (chars). Acima disso, trunca com aviso no log (v1: sem chunking).
const CORPO_MAX_CHARS: usize = 24_000;
/// Margem de parada do RPD: quando o header `x-ratelimit-remaining-requests` cai a ≤ isto, o lote
/// para ANTES de mandar a requisição que seria rejeitada — zero rejeição. A folga não é 0 de
/// propósito: reserva para o chat do RAG (que compartilha a mesma quota) e absorve contagem levemente
/// defasada. O que sobrar de pendente fica para a próxima rodada (idempotente).
const RPD_MARGEM_PARADA: u64 = 5;

/// Opções do subcomando (parseadas no `main`).
pub struct SinopseOpts {
    pub dry_run: bool,
    pub limit: Option<usize>,
    /// `numero` a regenerar mesmo com `resumo` preenchido.
    pub force: Option<String>,
    /// Dev-only: gera placeholder determinístico em vez de LLM.
    pub fake: bool,
}

/// Re-export: a fórmula mora no contrato (ponto único visível ao produtor E ao `auli update`).
pub use auli_contract::compose_text_to_embed;

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

/// Diretório da árvore de pareceres: `data/<id>/docs/pareceres` (irmão de `raw/`, que é o `data_dir`).
pub(crate) fn docs_dir(entity: &EntityConfig) -> Result<PathBuf> {
    let base = Path::new(&entity.data_dir)
        .parent()
        .ok_or_else(|| format!("data_dir sem pai: {}", entity.data_dir))?;
    Ok(base.join("docs").join("pareceres"))
}

/// Um documento da árvore no índice leve: caminho + identidade + se falta sinopse. NÃO carrega o
/// corpo — ele é relido só na hora de gerar (a árvore de SP tem 15,6 mil arquivos).
struct DocIndex {
    caminho: PathBuf,
    numero: String,
    pendente: bool,
}

/// Varre a árvore e monta o índice leve, em ordem estável (nome do arquivo). Arquivo que não parseia
/// é **erro**: melhor falhar alto do que pular silenciosamente um documento que precisava de sinopse.
fn indexar(dir: &Path, force: Option<&str>) -> Result<Vec<DocIndex>> {
    let mut caminhos: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "md"))
        .collect();
    caminhos.sort();

    let mut out = Vec::with_capacity(caminhos.len());
    for caminho in caminhos {
        let texto = std::fs::read_to_string(&caminho)?;
        let (header, sinopse, _corpo) = mddoc::parse_doc(&texto)
            .map_err(|e| format!("`{}` não parseia ({e}) — corrija antes de rodar o sinopse", caminho.display()))?;
        // `--force` re-pendura exatamente o `numero` alvo, mesmo que já tenha seção.
        let forcado = force.is_some_and(|f| f == header.numero);
        out.push(DocIndex {
            caminho,
            pendente: sinopse.is_none() || forcado,
            numero: header.numero,
        });
    }
    Ok(out)
}

/// Grava a sinopse no `.md`, preservando header e corpo. Escrita atômica (`.tmp` + rename): uma
/// queda no meio nunca deixa um documento truncado no lugar do bom.
fn escrever_sinopse(
    caminho: &Path,
    resumo: &str,
    modelo: &str,
    prompt_versao: u32,
    gerada_em: &str,
) -> Result<()> {
    let texto = std::fs::read_to_string(caminho)?;
    let (mut header, _sinopse_antiga, corpo) = mddoc::parse_doc(&texto)
        .map_err(|e| format!("`{}` não parseia ({e})", caminho.display()))?;
    header.sinopse_info = Some(auli_contract::SinopseInfo {
        modelo: modelo.to_string(),
        prompt_versao,
        gerada_em: gerada_em.to_string(),
    });
    let novo = mddoc::render_doc(&header, Some(resumo), &corpo);
    let tmp = PathBuf::from(format!("{}.tmp", caminho.display()));
    std::fs::write(&tmp, novo)?;
    std::fs::rename(&tmp, caminho)?;
    Ok(())
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

/// `true` se o motivo da falha é estouro de cota (rate limit). Diferente de uma falha de validação
/// (transiente, específica do doc), rate limit é uma parede global: todo request seguinte também vai
/// falhar. O lote aborta no 1º — 1 rejeição em vez de centenas queimando quota.
fn e_rate_limit(motivo: &str) -> bool {
    motivo.contains("rate_limit_exceeded")
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

/// Sinopse gerada + o headroom de RPD lido no header da resposta (para o proactive-stop).
struct Gerada {
    resumo: String,
    remaining_requests: Option<u64>,
    reset_requests: Option<String>,
}

/// Gera a sinopse real de um documento: trunca o corpo, chama o LLM, detecta erro-como-`Ok`,
/// valida (com **uma** re-tentativa). Devolve o `resumo` aparado + headroom, ou o motivo da falha (o
/// chamador conta a falha e segue — nunca aborta o lote).
fn gerar_sinopse(
    rt: &tokio::runtime::Runtime,
    params: &auli_llm::LlmParams,
    system_prompt: &str,
    assunto: &str,
    corpo: &str,
    numero: &str,
) -> std::result::Result<Gerada, String> {
    let (corpo_trunc, truncou) = truncar_corpo(corpo, CORPO_MAX_CHARS);
    if truncou {
        eprintln!("ℹ️  {numero}: corpo truncado em {CORPO_MAX_CHARS} chars (v1 sem chunking).");
    }
    let user_msg = format!("Assunto: {assunto}\n\nDocumento:\n{corpo_trunc}");

    for tentativa in 1..=2u32 {
        let resp = rt
            .block_on(auli_llm::chat(params, system_prompt, &user_msg))
            .map_err(|e| format!("transporte: {e}"))?;
        if resposta_e_erro_de_api(&resp.text) {
            return Err(format!("API: {}", resp.text));
        }
        match validar_sinopse(&resp.text) {
            Ok(()) => {
                return Ok(Gerada {
                    resumo: resp.text.trim().to_string(),
                    remaining_requests: resp.remaining_requests,
                    reset_requests: resp.reset_requests,
                });
            }
            Err(motivo) if tentativa == 2 => return Err(format!("validação: {motivo}")),
            Err(motivo) => eprintln!("↻ {numero}: validação falhou ({motivo}); re-tentando 1×."),
        }
    }
    unreachable!("o loop retorna em ambas as tentativas")
}

pub fn run(entity: &EntityConfig, opts: SinopseOpts) -> Result<()> {
    // 1. A árvore é a fonte. Sem ela não há o que fazer — o `auli update` é quem a materializa.
    let dir = docs_dir(entity)?;
    if !dir.exists() {
        return Err(format!(
            "árvore ausente: {} — rode `auli update --entity {}` antes (ela materializa os `.md`).",
            dir.display(),
            entity.id
        )
        .into());
    }

    // 2. Índice leve (sem corpos): total, quem já tem sinopse, quem falta.
    let docs = indexar(&dir, opts.force.as_deref())?;
    if let Some(alvo) = opts.force.as_deref()
        && !docs.iter().any(|d| d.numero == alvo)
    {
        return Err(format!("--force {alvo:?}: nenhum documento com esse `numero` na árvore.").into());
    }
    let total = docs.len();
    let pendentes = docs.iter().filter(|d| d.pendente).count();
    let reaproveitados = total - pendentes;
    println!("📊 {}: total {total} | reaproveitados {reaproveitados} | pendentes {pendentes}", entity.id);

    // 3. Dry-run: estimativa de tokens dos pendentes, sem escrever nada.
    if opts.dry_run {
        let mut chars = 0usize;
        for d in docs.iter().filter(|d| d.pendente) {
            let texto = std::fs::read_to_string(&d.caminho)?;
            if let Ok((h, _s, corpo)) = mddoc::parse_doc(&texto) {
                chars += h.assunto.chars().count() + corpo.chars().count();
            }
        }
        println!("🔎 dry-run: ~{} tokens de entrada nos {pendentes} pendentes (nada foi escrito).", milhar(chars / 4));
        return Ok(());
    }

    // 4. Config LLM: lida UMA vez, e SÓ na geração real (nem dry-run nem fake nem "sem pendentes"
    //    tocam env/rede). `--fake` continua dev-only, sem LLM.
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

    // 5. Geração documento a documento: lê → gera → grava o próprio `.md`. `--limit` limita os
    //    PROCESSADOS (gerados + falhas). Cada gravação é atômica, então a retomada é grátis.
    let mut gerados = 0usize;
    let mut falhas = 0usize;
    let limit = opts.limit.unwrap_or(usize::MAX);
    for d in docs.iter().filter(|d| d.pendente) {
        if gerados + falhas >= limit {
            break;
        }
        let texto = std::fs::read_to_string(&d.caminho)?;
        let (header, _sin, corpo) = mddoc::parse_doc(&texto)
            .map_err(|e| format!("`{}` não parseia ({e})", d.caminho.display()))?;

        let (resumo, modelo, versao, headroom) = if let Some((params, system_prompt, rt)) = &llm {
            match gerar_sinopse(rt, params, system_prompt, &header.assunto, &corpo, &header.numero) {
                Ok(g) => {
                    let hr = (g.remaining_requests, g.reset_requests);
                    (g.resumo, params.model.clone(), SINOPSE_PROMPT_VERSION, Some(hr))
                }
                Err(motivo) if e_rate_limit(&motivo) => {
                    // Rede de segurança: aborta no 1º 429 (não conta como falha) em vez de queimar
                    // quota com rejeições. Os restantes ficam para a próxima rodada (idempotente).
                    eprintln!(
                        "🛑 {}: cota da API esgotada (rate limit) em '{}'. Abortando o lote — \
                         os pendentes ficam para a próxima rodada (idempotente).",
                        entity.id, header.numero
                    );
                    break;
                }
                Err(motivo) => {
                    eprintln!("⚠️  {}: falha — {motivo}", header.numero);
                    falhas += 1;
                    continue;
                }
            }
        } else {
            (fake_resumo(&header.assunto), "fake".to_string(), 0, None)
        };

        escrever_sinopse(&d.caminho, &resumo, &modelo, versao, &now_iso8601())?;
        gerados += 1;

        // Proactive-stop: este doc já foi gravado; se o RPD restante caiu à margem, parar ANTES de
        // mandar a próxima (que seria rejeitada) — zero rejeição. Idempotente: o retry segue daqui.
        if let Some((Some(restantes), reset)) = headroom
            && restantes <= RPD_MARGEM_PARADA
        {
            let quando = reset.as_deref().unwrap_or("desconhecido");
            println!(
                "🧮 {}: headroom de RPD esgotando ({restantes} restantes ≤ margem {RPD_MARGEM_PARADA}). \
                 Parando limpo (zero rejeição) — reset em {quando}. Suba o teto ou rode de novo após o reset.",
                entity.id
            );
            break;
        }
    }

    // 6. Relatório final + invariante de guarda do passo. Pendentes-restantes = não processados
    //    (falhas contam à parte); a soma fecha em `total`.
    let pendentes_restantes = pendentes - gerados - falhas;
    println!(
        "✅ {}: total {total} | reaproveitados {reaproveitados} | gerados {gerados} | falhas {falhas} | pendentes-restantes {pendentes_restantes}",
        entity.id
    );
    println!("📝 árvore atualizada: {} (rode `auli update --entity {}` para vetorizar)", dir.display(), entity.id);
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

    /// EntityConfig apontando para um dir temporário exclusivo do teste (limpo no início).
    /// `data_dir` termina em `/raw` (como o real `../data/<id>/raw`), então a árvore do teste cai em
    /// `<base>/docs/pareceres` — irmã do `raw/`, igual à produção.
    fn temp_entity(tag: &str) -> EntityConfig {
        let base = std::env::temp_dir().join(format!("auli_sinopse_g4_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("raw")).unwrap();
        std::fs::create_dir_all(base.join("docs").join("pareceres")).unwrap();
        EntityConfig {
            id: "xx".into(),
            name: "Teste".into(),
            system_prompt: String::new(),
            data_dir: base.join("raw").to_string_lossy().into_owned(),
        }
    }

    /// Escreve um `.md` na árvore do teste. `sinopse: None` = documento pendente.
    fn escrever_doc(entity: &EntityConfig, slug: &str, numero: &str, sinopse: Option<&str>) {
        let header = mddoc::DocHeader {
            numero: numero.into(),
            assunto: format!("assunto de {numero}"),
            link: format!("https://exemplo/{numero}"),
            sinopse_info: sinopse.map(|_| auli_contract::SinopseInfo {
                modelo: "previa".into(),
                prompt_versao: 1,
                gerada_em: "2026-01-01T00:00:00Z".into(),
            }),
        };
        let corpo = format!("corpo integral de {numero}");
        let dir = docs_dir(entity).unwrap();
        std::fs::write(dir.join(format!("{slug}.md")), mddoc::render_doc(&header, sinopse, &corpo)).unwrap();
    }

    /// Lê `(sinopse, corpo, modelo)` de um `.md` da árvore.
    fn ler_doc(entity: &EntityConfig, slug: &str) -> (Option<String>, String, Option<String>) {
        let dir = docs_dir(entity).unwrap();
        let texto = std::fs::read_to_string(dir.join(format!("{slug}.md"))).unwrap();
        let (h, sin, corpo) = mddoc::parse_doc(&texto).unwrap();
        (sin, corpo, h.sinopse_info.map(|i| i.modelo))
    }

    fn opts(fake: bool, limit: Option<usize>, force: Option<&str>, dry_run: bool) -> SinopseOpts {
        SinopseOpts { dry_run, limit, force: force.map(String::from), fake }
    }

    #[test]
    fn compose_indexa_numero_assunto_resumo() {
        assert_eq!(compose_text_to_embed("N", "A", "R"), "N\nA\nR");
        // Vazios são pulados (pendente indexa só numero+assunto).
        assert_eq!(compose_text_to_embed("N", "A", ""), "N\nA");
    }

    #[test]
    fn arvore_ausente_e_erro_que_ensina_o_remedio() {
        let e = temp_entity("semarvore");
        std::fs::remove_dir_all(docs_dir(&e).unwrap()).unwrap();
        let err = run(&e, opts(true, None, None, false)).unwrap_err().to_string();
        assert!(err.contains("árvore ausente"), "erro: {err}");
        assert!(err.contains("auli update"), "deve mandar materializar antes: {err}");
    }

    #[test]
    fn pendente_e_quem_nao_tem_secao_sinopse() {
        let e = temp_entity("indexa");
        escrever_doc(&e, "a-1", "A 1", None);
        escrever_doc(&e, "b-2", "B 2", Some("### Descrição Resumida do Assunto\nja tem"));
        let docs = indexar(&docs_dir(&e).unwrap(), None).unwrap();
        assert_eq!(docs.len(), 2);
        assert!(docs.iter().find(|d| d.numero == "A 1").unwrap().pendente);
        assert!(!docs.iter().find(|d| d.numero == "B 2").unwrap().pendente);
    }

    #[test]
    fn fake_preenche_a_arvore_respeita_limit_e_retoma() {
        let e = temp_entity("fakelimit");
        escrever_doc(&e, "a-1", "A 1", None);
        escrever_doc(&e, "b-2", "B 2", None);

        // 1ª rodada com limit=1: só o primeiro (ordem estável por nome de arquivo).
        run(&e, opts(true, Some(1), None, false)).unwrap();
        assert!(ler_doc(&e, "a-1").0.is_some(), "a-1 devia ter sinopse");
        assert!(ler_doc(&e, "b-2").0.is_none(), "b-2 ainda pendente");

        // 2ª rodada: retoma o que faltou, sem re-gerar o que já tinha.
        run(&e, opts(true, None, None, false)).unwrap();
        let (sin_b, corpo_b, modelo_b) = ler_doc(&e, "b-2");
        assert!(sin_b.unwrap().contains("[FAKE]"));
        assert_eq!(corpo_b, "corpo integral de B 2", "corpo preservado na regravação");
        assert_eq!(modelo_b.as_deref(), Some("fake"), "proveniência do fake gravada");
    }

    #[test]
    fn force_repende_documento_que_ja_tem_sinopse() {
        let e = temp_entity("force");
        escrever_doc(&e, "a-1", "A 1", Some("### Descrição Resumida do Assunto\nantiga"));
        // Sem --force: nada a fazer.
        run(&e, opts(true, None, None, false)).unwrap();
        assert!(ler_doc(&e, "a-1").0.unwrap().contains("antiga"), "sem force não regenera");
        // Com --force: regenera a seção.
        run(&e, opts(true, None, Some("A 1"), false)).unwrap();
        assert!(ler_doc(&e, "a-1").0.unwrap().contains("[FAKE]"), "force devia regenerar");
    }

    #[test]
    fn force_com_numero_inexistente_e_erro() {
        let e = temp_entity("forceruim");
        escrever_doc(&e, "a-1", "A 1", None);
        let err = run(&e, opts(true, None, Some("NAO EXISTE"), false)).unwrap_err().to_string();
        assert!(err.contains("nenhum documento"), "erro: {err}");
    }

    #[test]
    fn dry_run_nao_escreve_nada() {
        let e = temp_entity("dry");
        escrever_doc(&e, "a-1", "A 1", None);
        run(&e, opts(true, None, None, true)).unwrap();
        assert!(ler_doc(&e, "a-1").0.is_none(), "dry-run não pode escrever");
    }

    #[test]
    fn escrever_sinopse_preserva_corpo_e_campos_do_header() {
        let e = temp_entity("preserva");
        escrever_doc(&e, "a-1", "A 1", None);
        let dir = docs_dir(&e).unwrap();
        escrever_sinopse(&dir.join("a-1.md"), "NOVA SINOPSE", "modelo-x", 7, "2026-07-20T00:00:00Z").unwrap();

        let texto = std::fs::read_to_string(dir.join("a-1.md")).unwrap();
        let (h, sin, corpo) = mddoc::parse_doc(&texto).unwrap();
        assert_eq!(sin.as_deref(), Some("NOVA SINOPSE"));
        assert_eq!(corpo, "corpo integral de A 1", "corpo intocado");
        assert_eq!(h.numero, "A 1");
        assert_eq!(h.link, "https://exemplo/A 1", "link intocado");
        let info = h.sinopse_info.unwrap();
        assert_eq!(info.modelo, "modelo-x");
        assert_eq!(info.prompt_versao, 7);
    }

    #[test]
    fn documento_ilegivel_na_arvore_e_erro_alto() {
        let e = temp_entity("ilegivel");
        std::fs::write(docs_dir(&e).unwrap().join("ruim.md"), "sem frontmatter").unwrap();
        let err = run(&e, opts(true, None, None, false)).unwrap_err().to_string();
        assert!(err.contains("não parseia"), "erro: {err}");
    }

    #[test]
    fn nao_deixa_tmp_para_tras() {
        let e = temp_entity("tmp");
        escrever_doc(&e, "a-1", "A 1", None);
        run(&e, opts(true, None, None, false)).unwrap();
        let sobras: Vec<_> = std::fs::read_dir(docs_dir(&e).unwrap())
            .unwrap()
            .filter_map(|x| x.ok())
            .filter(|x| x.path().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(sobras.is_empty(), "escrita atômica não pode deixar .tmp");
    }
}
