//! Subcomando `extrair` — extração one-shot de metadados de grafo (TAREFA-EXTRACAO, E1–E4).
//!
//! Lê a MESMA árvore `data/<id>/docs/pareceres/*.md` do `sinopse` e, por documento, pede ao LLM
//! um objeto `{dispositivos, ncm, temas}`: citações legais LITERAIS (a canonização é passo futuro,
//! determinístico), códigos NCM e temas centrais. Grava um JSONL consolidado em
//! `data/<id>/extracao/extracao.jsonl` (sucessos) e `erros.jsonl` (falhas, só diagnóstico).
//! Os `.md` NÃO são tocados — passo exploratório: a análise do JSONL decide o desenho do
//! canonizador e da integração ao pipeline (fora de escopo aqui).
//!
//! Retomada é implícita e mora na SAÍDA: pendente = `numero` sem linha no `extracao.jsonl`.
//! Falha não entra no principal, logo é re-tentada na próxima rodada. `--force <numero>` remove a
//! linha do documento e o re-pendura. O caminho quente é append linha a linha; regravação integral
//! (atômica) só para descartar cauda truncada ou aplicar o `--force`. Invariante do passo:
//! `ja_extraidos + gerados + falhas + pendentes_restantes == total`.
//!
//! Memória: como no sinopse, os corpos são lidos um a um na hora de gerar, nunca todos na RAM.

use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use auli_contract::mddoc;

use crate::domain::entities::EntityConfig;
use crate::errors::Result;
use crate::sinopse::{
    CORPO_MAX_CHARS, RPD_MARGEM_PARADA, docs_dir, e_rate_limit, env_com_fallback, milhar,
    now_iso8601, resposta_e_erro_de_api, truncar_corpo,
};

/// Versão do prompt (gravada em cada linha do JSONL). Bump a cada mudança do
/// `data/prompts/extracao.txt`. `0` é reservado ao `--fake`.
pub const EXTRACAO_PROMPT_VERSION: u32 = 1;

/// Opções do subcomando (parseadas no `main`).
pub struct ExtracaoOpts {
    pub dry_run: bool,
    pub limit: Option<usize>,
    /// `numero` a re-extrair mesmo que já tenha linha no JSONL.
    pub force: Option<String>,
    /// Dev-only: gera extração placeholder determinística em vez de LLM.
    pub fake: bool,
}

/// Um dispositivo citado, LITERAL como no texto do documento.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dispositivo {
    pub texto: String,
}

/// O objeto que o LLM devolve. `deny_unknown_fields` + campos obrigatórios (sem `default`) =
/// schema EXATO: chave a mais ou a menos é falha de validação, nunca parse frouxo.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Extracao {
    pub dispositivos: Vec<Dispositivo>,
    pub ncm: Vec<String>,
    pub temas: Vec<String>,
}

/// Uma linha do `extracao.jsonl`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Linha {
    numero: String,
    link: String,
    prompt_versao: u32,
    modelo: String,
    gerada_em: String,
    extracao: Extracao,
}

/// Uma linha do `erros.jsonl` (diagnóstico humano; não participa da retomada).
#[derive(Debug, Serialize)]
struct LinhaErro<'a> {
    numero: &'a str,
    motivo: &'a str,
    quando: String,
}

/// Diretório de saída: `data/<id>/extracao` (irmão de `raw/` e `docs/`), criado sob demanda.
fn extracao_dir(entity: &EntityConfig) -> Result<PathBuf> {
    let base = Path::new(&entity.data_dir)
        .parent()
        .ok_or_else(|| format!("data_dir sem pai: {}", entity.data_dir))?;
    Ok(base.join("extracao"))
}

/// System prompt da extração. Mesmo cálculo de caminho do sinopse: `data_dir` é
/// `../data/<id>/raw`, logo o prompt cai em `../data/prompts/extracao.txt`.
fn load_prompt(entity: &EntityConfig) -> Result<String> {
    let path = Path::new(&entity.data_dir)
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| format!("data_dir inesperado: {}", entity.data_dir))?
        .join("prompts/extracao.txt");
    std::fs::read_to_string(&path)
        .map_err(|e| format!("prompt de extração ausente ({}): {e}", path.display()).into())
}

/// Um documento da árvore no índice leve: caminho + identidade. NÃO carrega o corpo — ele é
/// relido só na hora de gerar. A pendência NÃO mora aqui (mora no JSONL de saída).
struct DocIndex {
    caminho: PathBuf,
    numero: String,
}

/// Varre a árvore e monta o índice leve, em ordem estável (nome do arquivo). Arquivo que não
/// parseia é **erro**: melhor falhar alto do que extrair um corpus com buraco silencioso.
fn indexar(dir: &Path) -> Result<Vec<DocIndex>> {
    let mut caminhos: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "md"))
        .collect();
    caminhos.sort();

    let mut out = Vec::with_capacity(caminhos.len());
    for caminho in caminhos {
        let texto = std::fs::read_to_string(&caminho)?;
        let (header, _sinopse, _corpo) = mddoc::parse_doc(&texto).map_err(|e| {
            format!(
                "`{}` não parseia ({e}) — corrija antes de rodar o extrair",
                caminho.display()
            )
        })?;
        out.push(DocIndex {
            caminho,
            numero: header.numero,
        });
    }
    Ok(out)
}

/// Lê as linhas válidas do JSONL + se havia uma cauda truncada (queda no meio de um append).
/// Linha malformada no MEIO é erro alto (arquivo corrompido — corrija ou remova); no FIM é a
/// cauda truncada: avisa, descarta e o documento volta a pendente (D5).
fn ler_linhas(path: &Path) -> Result<(Vec<Linha>, bool)> {
    let mut out = Vec::new();
    if !path.exists() {
        return Ok((out, false));
    }
    let texto = std::fs::read_to_string(path)?;
    let linhas: Vec<&str> = texto.lines().filter(|l| !l.trim().is_empty()).collect();
    let mut cauda_truncada = false;
    for (i, l) in linhas.iter().enumerate() {
        match serde_json::from_str::<Linha>(l) {
            Ok(linha) => out.push(linha),
            Err(e) if i + 1 == linhas.len() => {
                eprintln!(
                    "⚠️  {}: última linha malformada ({e}) — provável queda no meio da escrita; \
                     descartada, o documento será re-extraído.",
                    path.display()
                );
                cauda_truncada = true;
            }
            Err(e) => {
                return Err(format!(
                    "{}: linha {} malformada ({e}) — arquivo corrompido; corrija ou remova antes de rodar.",
                    path.display(),
                    i + 1
                )
                .into());
            }
        }
    }
    Ok((out, cauda_truncada))
}

/// Regrava o JSONL inteiro, atômico (`.tmp` + rename). SÓ para os casos raros (D4): descartar a
/// cauda truncada ou remover a linha de um `--force`. O caminho quente é sempre append.
fn regravar_linhas(path: &Path, linhas: &[Linha]) -> Result<()> {
    let mut buf = String::new();
    for l in linhas {
        buf.push_str(&serde_json::to_string(l).map_err(|e| format!("serializando linha: {e}"))?);
        buf.push('\n');
    }
    let tmp = PathBuf::from(format!("{}.tmp", path.display()));
    std::fs::write(&tmp, buf)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// Anexa uma linha (JSON já serializado) + `\n`, com flush. O `ler_linhas` tolera a cauda
/// truncada que uma queda aqui poderia deixar.
fn append_linha(path: &Path, json: &str) -> Result<()> {
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    f.write_all(json.as_bytes())?;
    f.write_all(b"\n")?;
    f.flush()?;
    Ok(())
}

/// Remove uma cerca markdown (```` ```json … ``` ````) se o modelo desobedecer o "JSON puro".
/// Tolerância barata: o conteúdo é o mesmo, só a embalagem que erra.
fn descercar(answer: &str) -> &str {
    let t = answer.trim();
    let t = t
        .strip_prefix("```json")
        .or_else(|| t.strip_prefix("```"))
        .unwrap_or(t);
    let t = t.strip_suffix("```").unwrap_or(t);
    t.trim()
}

/// Valida e desserializa a resposta (pura, testável). Schema exato via serde (D8) + regra local:
/// `dispositivos[].texto` não-vazio. Arrays vazios são válidos (regra 9 do prompt).
fn validar_extracao(answer: &str) -> std::result::Result<Extracao, String> {
    let limpo = descercar(answer);
    let ex: Extracao = serde_json::from_str(limpo).map_err(|e| format!("JSON inválido: {e}"))?;
    if ex.dispositivos.iter().any(|d| d.texto.trim().is_empty()) {
        return Err("dispositivo com `texto` vazio".into());
    }
    Ok(ex)
}

/// Extração placeholder (dev-only). `prompt_versao: 0` marca fake, distinguível da geração real.
fn fake_extracao(numero: &str) -> Extracao {
    Extracao {
        dispositivos: vec![Dispositivo {
            texto: format!("[FAKE] art. 1º citado em {numero}"),
        }],
        ncm: vec![],
        temas: vec!["fake".into()],
    }
}

/// Extração gerada + o headroom de RPD lido no header da resposta (para o proactive-stop).
struct Gerada {
    extracao: Extracao,
    remaining_requests: Option<u64>,
    reset_requests: Option<String>,
}

/// Gera a extração real de um documento: trunca o corpo, chama o LLM, detecta erro-como-`Ok`,
/// valida (com **uma** re-tentativa). Devolve a extração + headroom, ou o motivo da falha (o
/// chamador registra no `erros.jsonl` e segue — nunca aborta o lote, exceto rate-limit).
fn gerar_extracao(
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
        // Resgate dirigido: a re-tentativa baixa o reasoning para `low` SÓ neste doc já-falho. Isso
        // quebra o runaway de reasoning (doc patológico que consome todo o teto raciocinando e
        // devolve vazio — nenhum teto resolve) e ainda recupera empties transitórios. A 1ª tentativa
        // mantém o reasoning cheio, então o caminho feliz nunca perde precisão.
        let rescue = (tentativa == 2).then(|| auli_llm::LlmParams {
            reasoning_effort: Some(auli_llm::ReasoningEffort::Low),
            ..params.clone()
        });
        let call = rescue.as_ref().unwrap_or(params);
        let resp = rt
            .block_on(auli_llm::chat(call, system_prompt, &user_msg))
            .map_err(|e| format!("transporte: {e}"))?;
        if resposta_e_erro_de_api(&resp.text) {
            return Err(format!("API: {}", resp.text));
        }
        match validar_extracao(&resp.text) {
            Ok(extracao) => {
                return Ok(Gerada {
                    extracao,
                    remaining_requests: resp.remaining_requests,
                    reset_requests: resp.reset_requests,
                });
            }
            Err(motivo) if tentativa == 2 => return Err(format!("validação: {motivo}")),
            Err(motivo) => {
                eprintln!(
                    "↻ {numero}: validação falhou ({motivo}); re-tentando 1× com reasoning_effort=low."
                )
            }
        }
    }
    unreachable!("o loop retorna em ambas as tentativas")
}

pub fn run(entity: &EntityConfig, opts: ExtracaoOpts) -> Result<()> {
    // 1. A árvore é a fonte (a MESMA do sinopse). Sem ela não há o que extrair.
    let dir = docs_dir(entity)?;
    if !dir.exists() {
        return Err(format!(
            "árvore ausente: {} — rode `auli update --entity {}` antes (ela materializa os `.md`).",
            dir.display(),
            entity.id
        )
        .into());
    }

    // 2. Índice leve da árvore (sem corpos) + estado da saída (a retomada mora no JSONL — D3).
    let docs = indexar(&dir)?;
    let out_dir = extracao_dir(entity)?;
    let out_path = out_dir.join("extracao.jsonl");
    let err_path = out_dir.join("erros.jsonl");
    let (mut linhas, cauda_truncada) = ler_linhas(&out_path)?;

    // `--force <numero>`: precisa existir na árvore; a linha dele (se houver) sai — vira pendente.
    let mut force_removeu = false;
    if let Some(alvo) = opts.force.as_deref() {
        if !docs.iter().any(|d| d.numero == alvo) {
            return Err(
                format!("--force {alvo:?}: nenhum documento com esse `numero` na árvore.").into(),
            );
        }
        let antes = linhas.len();
        linhas.retain(|l| l.numero != alvo);
        force_removeu = linhas.len() != antes;
    }

    // Saneamento (nunca no dry-run): cauda truncada descartada e/ou linha do --force removida ⇒
    // regrava UMA vez, atômico (D4). Sem isso, o próximo append colaria na cauda truncada.
    if !opts.dry_run && (cauda_truncada || force_removeu) {
        std::fs::create_dir_all(&out_dir)?;
        regravar_linhas(&out_path, &linhas)?;
    }

    let ja: HashSet<&str> = linhas.iter().map(|l| l.numero.as_str()).collect();
    let total = docs.len();
    let pendentes_idx: Vec<&DocIndex> = docs
        .iter()
        .filter(|d| !ja.contains(d.numero.as_str()))
        .collect();
    let pendentes = pendentes_idx.len();
    let ja_extraidos = total - pendentes;
    println!(
        "📊 {}: total {total} | já-extraídos {ja_extraidos} | pendentes {pendentes}",
        entity.id
    );

    // 3. Dry-run: estimativa de tokens dos pendentes, sem escrever nada.
    if opts.dry_run {
        let mut chars = 0usize;
        for d in &pendentes_idx {
            let texto = std::fs::read_to_string(&d.caminho)?;
            if let Ok((h, _s, corpo)) = mddoc::parse_doc(&texto) {
                chars += h.assunto.chars().count() + corpo.chars().count();
            }
        }
        println!(
            "🔎 dry-run: ~{} tokens de entrada nos {pendentes} pendentes (nada foi escrito).",
            milhar(chars / 4)
        );
        return Ok(());
    }

    // 4. Config LLM: lida UMA vez, e SÓ na geração real (nem fake nem "sem pendentes" tocam
    //    env/rede) — mesmo desenho do sinopse.
    let real = !opts.fake && pendentes > 0;
    let llm = if real {
        let params = auli_llm::LlmParams {
            api_url: env_com_fallback("EXTRACAO_API_URL", "LLM_API_URL")?,
            api_key: env_com_fallback("EXTRACAO_API_KEY", "LLM_API_KEY")?,
            model: env_com_fallback("EXTRACAO_API_MODEL", "LLM_API_MODEL")?,
            temperature: 0.1, // extração literal: fidelidade, não diversidade
            // 16384 = teto de saída folgado (5,5× o pior caso medido no RS: ~2,9k tokens). O
            // gpt-oss é modelo de RACIOCÍNIO: reasoning + JSON dividem este orçamento, e o reasoning
            // sozinho chega a ~2,5k nos pareceres longos — 2048 clipava o reasoning antes do JSON,
            // devolvendo resposta vazia. O teto alto é quase grátis (cobra-se o gerado, não o teto).
            // Revisão do D7 pós-análise do RS.
            max_completion_tokens: 16384,
            timeout: Duration::from_secs(60),
            // 1ª tentativa com reasoning cheio (precisão intacta no caminho feliz); a re-tentativa
            // baixa para `low` — ver `gerar_extracao`.
            reasoning_effort: None,
        };
        let system_prompt = load_prompt(entity)?;
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        Some((params, system_prompt, rt))
    } else {
        None
    };

    if pendentes > 0 {
        std::fs::create_dir_all(&out_dir)?;
    }

    // 5. Extração documento a documento: lê → gera → append no JSONL. `--limit` limita os
    //    PROCESSADOS (gerados + falhas). Cada append é durável, então a retomada é grátis.
    let mut gerados = 0usize;
    let mut falhas = 0usize;
    let limit = opts.limit.unwrap_or(usize::MAX);
    for d in &pendentes_idx {
        if gerados + falhas >= limit {
            break;
        }
        let texto = std::fs::read_to_string(&d.caminho)?;
        let (header, _sin, corpo) = mddoc::parse_doc(&texto)
            .map_err(|e| format!("`{}` não parseia ({e})", d.caminho.display()))?;

        let (extracao, modelo, versao, headroom) = if let Some((params, system_prompt, rt)) = &llm {
            match gerar_extracao(
                rt,
                params,
                system_prompt,
                &header.assunto,
                &corpo,
                &header.numero,
            ) {
                Ok(g) => {
                    let hr = (g.remaining_requests, g.reset_requests);
                    (
                        g.extracao,
                        params.model.clone(),
                        EXTRACAO_PROMPT_VERSION,
                        Some(hr),
                    )
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
                    let erro = LinhaErro {
                        numero: &header.numero,
                        motivo: &motivo,
                        quando: now_iso8601(),
                    };
                    let json = serde_json::to_string(&erro)
                        .map_err(|e| format!("serializando erro: {e}"))?;
                    append_linha(&err_path, &json)?;
                    falhas += 1;
                    continue;
                }
            }
        } else {
            (fake_extracao(&header.numero), "fake".to_string(), 0, None)
        };

        let linha = Linha {
            numero: header.numero.clone(),
            link: header.link.clone(),
            prompt_versao: versao,
            modelo,
            gerada_em: now_iso8601(),
            extracao,
        };
        let json = serde_json::to_string(&linha).map_err(|e| format!("serializando linha: {e}"))?;
        append_linha(&out_path, &json)?;
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

    // 6. Relatório final + invariante de guarda do passo (D9).
    let pendentes_restantes = pendentes - gerados - falhas;
    println!(
        "✅ {}: total {total} | já-extraídos {ja_extraidos} | gerados {gerados} | falhas {falhas} | pendentes-restantes {pendentes_restantes}",
        entity.id
    );
    println!(
        "📄 saída: {} (falhas em {})",
        out_path.display(),
        err_path.display()
    );
    assert_eq!(
        ja_extraidos + gerados + falhas + pendentes_restantes,
        total,
        "invariante de guarda violado"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// EntityConfig apontando para um dir temporário exclusivo do teste (limpo no início).
    /// `data_dir` termina em `/raw` (como o real), então a árvore cai em `<base>/docs/pareceres`
    /// e a saída em `<base>/extracao` — mesma geometria da produção.
    fn temp_entity(tag: &str) -> EntityConfig {
        let base = std::env::temp_dir().join(format!("auli_extracao_{tag}_{}", std::process::id()));
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

    /// Escreve um `.md` mínimo na árvore do teste (sem sinopse — a extração não depende dela).
    fn escrever_doc(entity: &EntityConfig, slug: &str, numero: &str) {
        let header = mddoc::DocHeader {
            numero: numero.into(),
            assunto: format!("assunto de {numero}"),
            link: format!("https://exemplo/{numero}"),
            sinopse_info: None,
        };
        let corpo = format!("corpo integral de {numero}");
        let dir = docs_dir(entity).unwrap();
        std::fs::write(
            dir.join(format!("{slug}.md")),
            mddoc::render_doc(&header, None, &corpo),
        )
        .unwrap();
    }

    fn saida_path(entity: &EntityConfig) -> PathBuf {
        extracao_dir(entity).unwrap().join("extracao.jsonl")
    }

    /// Lê e parseia todas as linhas do `extracao.jsonl` do teste.
    fn ler_saida(entity: &EntityConfig) -> Vec<Linha> {
        let texto = std::fs::read_to_string(saida_path(entity)).unwrap_or_default();
        texto
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str::<Linha>(l).unwrap())
            .collect()
    }

    fn opts(fake: bool, limit: Option<usize>, force: Option<&str>, dry_run: bool) -> ExtracaoOpts {
        ExtracaoOpts {
            dry_run,
            limit,
            force: force.map(String::from),
            fake,
        }
    }

    #[test]
    fn arvore_ausente_e_erro_que_ensina_o_remedio() {
        let e = temp_entity("semarvore");
        std::fs::remove_dir_all(docs_dir(&e).unwrap()).unwrap();
        let err = run(&e, opts(true, None, None, false))
            .unwrap_err()
            .to_string();
        assert!(err.contains("árvore ausente"), "erro: {err}");
        assert!(
            err.contains("auli update"),
            "deve mandar materializar antes: {err}"
        );
    }

    #[test]
    fn fake_gera_jsonl_respeita_limit_e_retoma_sem_duplicar() {
        let e = temp_entity("fakelimit");
        escrever_doc(&e, "a-1", "A 1");
        escrever_doc(&e, "b-2", "B 2");

        // 1ª rodada com limit=1: só o primeiro (ordem estável por nome de arquivo).
        run(&e, opts(true, Some(1), None, false)).unwrap();
        let l1 = ler_saida(&e);
        assert_eq!(l1.len(), 1);
        assert_eq!(l1[0].numero, "A 1");
        assert_eq!(l1[0].modelo, "fake");
        assert_eq!(l1[0].prompt_versao, 0, "fake grava versão 0");
        assert_eq!(l1[0].link, "https://exemplo/A 1", "link vem do frontmatter");
        assert!(l1[0].extracao.dispositivos[0].texto.contains("[FAKE]"));

        // 2ª rodada: retoma o que faltou. 3ª rodada: nada a fazer, zero duplicatas.
        run(&e, opts(true, None, None, false)).unwrap();
        run(&e, opts(true, None, None, false)).unwrap();
        let l2 = ler_saida(&e);
        assert_eq!(l2.len(), 2, "sem duplicatas na re-execução");
        assert_eq!(l2[1].numero, "B 2");
    }

    #[test]
    fn force_repende_e_substitui_sem_duplicar() {
        let e = temp_entity("force");
        escrever_doc(&e, "a-1", "A 1");
        escrever_doc(&e, "b-2", "B 2");
        run(&e, opts(true, None, None, false)).unwrap();
        assert_eq!(ler_saida(&e).len(), 2);

        run(&e, opts(true, None, Some("A 1"), false)).unwrap();
        let linhas = ler_saida(&e);
        assert_eq!(linhas.len(), 2, "force substitui, não duplica");
        assert_eq!(linhas.iter().filter(|l| l.numero == "A 1").count(), 1);
        // A linha re-gerada vai para o FIM (remove + append).
        assert_eq!(linhas.last().unwrap().numero, "A 1");
    }

    #[test]
    fn force_com_numero_inexistente_e_erro() {
        let e = temp_entity("forceruim");
        escrever_doc(&e, "a-1", "A 1");
        let err = run(&e, opts(true, None, Some("NAO EXISTE"), false))
            .unwrap_err()
            .to_string();
        assert!(err.contains("nenhum documento"), "erro: {err}");
    }

    #[test]
    fn dry_run_nao_escreve_nada() {
        let e = temp_entity("dry");
        escrever_doc(&e, "a-1", "A 1");
        run(&e, opts(true, None, None, true)).unwrap();
        assert!(
            !extracao_dir(&e).unwrap().exists(),
            "dry-run não pode criar nem escrever nada"
        );
    }

    #[test]
    fn cauda_truncada_e_descartada_e_regenerada() {
        let e = temp_entity("cauda");
        escrever_doc(&e, "a-1", "A 1");
        run(&e, opts(true, None, None, false)).unwrap();
        // Simula queda no meio de um append: cauda parcial SEM \n no fim.
        use std::io::Write as _;
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(saida_path(&e))
            .unwrap();
        f.write_all(b"{\"numero\":\"TRUNC").unwrap();
        drop(f);

        escrever_doc(&e, "b-2", "B 2");
        run(&e, opts(true, None, None, false)).unwrap();
        let linhas = ler_saida(&e);
        assert_eq!(
            linhas.len(),
            2,
            "cauda descartada; A 1 preservado; B 2 gerado"
        );
        assert!(
            linhas
                .iter()
                .all(|l| l.numero == "A 1" || l.numero == "B 2")
        );
    }

    #[test]
    fn linha_malformada_no_meio_e_erro_alto() {
        let e = temp_entity("meio");
        escrever_doc(&e, "a-1", "A 1");
        escrever_doc(&e, "b-2", "B 2");
        run(&e, opts(true, None, None, false)).unwrap();
        // Injeta lixo ENTRE as duas linhas válidas.
        let texto = std::fs::read_to_string(saida_path(&e)).unwrap();
        let mut ls: Vec<&str> = texto.lines().collect();
        ls.insert(1, "LIXO NO MEIO");
        std::fs::write(saida_path(&e), ls.join("\n") + "\n").unwrap();

        let err = run(&e, opts(true, None, None, false))
            .unwrap_err()
            .to_string();
        assert!(err.contains("malformada"), "erro: {err}");
        assert!(err.contains("linha 2"), "aponta a linha exata: {err}");
    }

    #[test]
    fn nao_deixa_tmp_para_tras() {
        let e = temp_entity("tmp");
        escrever_doc(&e, "a-1", "A 1");
        run(&e, opts(true, None, None, false)).unwrap();
        // Um --force força o caminho da regravação atômica.
        run(&e, opts(true, None, Some("A 1"), false)).unwrap();
        let sobras: Vec<_> = std::fs::read_dir(extracao_dir(&e).unwrap())
            .unwrap()
            .filter_map(|x| x.ok())
            .filter(|x| x.path().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(sobras.is_empty(), "regravação atômica não pode deixar .tmp");
    }

    // ── validar_extracao (pura) ──────────────────────────────────────────────

    #[test]
    fn validar_aceita_json_puro_e_com_cerca_markdown() {
        let puro = r#"{"dispositivos":[{"texto":"art. 12 do Anexo 3 do RICMS/SC"}],"ncm":["8708.99.90"],"temas":["substituição tributária"]}"#;
        assert!(validar_extracao(puro).is_ok());
        let cercado = format!("```json\n{puro}\n```");
        let ex = validar_extracao(&cercado).expect("cerca markdown deve ser tolerada");
        assert_eq!(ex.dispositivos.len(), 1);
        assert_eq!(ex.ncm[0], "8708.99.90");
    }

    #[test]
    fn validar_aceita_arrays_vazios() {
        let vazio = r#"{"dispositivos":[],"ncm":[],"temas":[]}"#;
        assert!(
            validar_extracao(vazio).is_ok(),
            "regra 9: arrays vazios são válidos"
        );
    }

    #[test]
    fn validar_rejeita_chave_desconhecida_ausente_e_texto_vazio() {
        // Chave a mais.
        let extra = r#"{"dispositivos":[],"ncm":[],"temas":[],"bonus":1}"#;
        assert!(validar_extracao(extra).is_err(), "deny_unknown_fields");
        // Chave a menos.
        let falta = r#"{"dispositivos":[],"ncm":[]}"#;
        assert!(validar_extracao(falta).is_err(), "campos obrigatórios");
        // Dispositivo com texto vazio.
        let vazio = r#"{"dispositivos":[{"texto":"  "}],"ncm":[],"temas":[]}"#;
        assert!(
            validar_extracao(vazio).is_err(),
            "texto vazio é inútil no grafo"
        );
        // Prosa em volta do JSON não é tolerada (só a cerca é).
        let prosa = r#"Aqui está: {"dispositivos":[],"ncm":[],"temas":[]}"#;
        assert!(
            validar_extracao(prosa).is_err(),
            "JSON com prosa em volta é falha de formato"
        );
    }
}
