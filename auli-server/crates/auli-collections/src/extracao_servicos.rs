//! Subcomando `extrair-servicos` — extração de metadados de grafo da carta de serviços.
//!
//! Espelho do `extrair` (pareceres) sobre a OUTRA árvore: lê `data/<id>/docs/servicos/*.md` e, por
//! serviço, pede ao LLM um objeto `{sistemas, temas}` — os sistemas/portais que o contribuinte
//! precisa atravessar (literais, a canonização é o passo seguinte, determinístico) e os temas
//! centrais. Grava `data/<id>/extracao/servicos-extracao.jsonl` (sucessos) e `servicos-erros.jsonl`
//! (falhas, só diagnóstico). Os `.md` NÃO são tocados.
//!
//! **Identidade é o `titulo`, não o arquivo** (E-SERV-1): a árvore repete o mesmo serviço uma vez por
//! público (no RS, 586 arquivos = 422 serviços; 156 títulos repetem com corpo IDÊNTICO). Extrair por
//! arquivo pagaria 28% a mais ao LLM e inflaria o peso das arestas de quem é ofertado a dois públicos.
//! O público entra no grafo como atributo, relido da árvore no passo `grafo-servicos`. Se dois
//! arquivos compartilham o título mas divergem no corpo, é **erro alto**: a premissa da dedup caiu e
//! o remédio é humano (renomear no portal ou passar a chavear por link).
//!
//! Retomada, `--force`, cauda truncada e proactive-stop de RPD: idênticos ao `extrair` — a pendência
//! mora na SAÍDA (título sem linha no JSONL). Invariante: `ja + gerados + falhas + restantes == total`.

use std::collections::{HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use auli_contract::mddoc_servico;

use crate::domain::entities::EntityConfig;
use crate::errors::Result;
use crate::extracao::{ExtracaoOpts, append_linha, descercar, extracao_dir};
use crate::sinopse::{
    CORPO_MAX_CHARS, RPD_MARGEM_PARADA, e_rate_limit, env_com_fallback, milhar, now_iso8601,
    resposta_e_erro_de_api, truncar_corpo,
};

/// Versão do prompt (gravada em cada linha). Bump a cada mudança do
/// `data/prompts/extracao-servicos.txt`. `0` é reservado ao `--fake`.
pub const EXTRACAO_SERVICOS_PROMPT_VERSION: u32 = 1;

/// Um sistema/portal citado, LITERAL como no texto do serviço.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sistema {
    pub texto: String,
}

/// O objeto que o LLM devolve. `deny_unknown_fields` + campos obrigatórios = schema EXATO.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtracaoServico {
    pub sistemas: Vec<Sistema>,
    pub temas: Vec<String>,
}

/// Uma linha do `servicos-extracao.jsonl`. Guarda só identidade + extração: público, classe e órgão
/// são atributos do snapshot e são relidos da árvore pelo `grafo-servicos` (não congelam aqui).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Linha {
    titulo: String,
    link: String,
    prompt_versao: u32,
    modelo: String,
    gerada_em: String,
    extracao: ExtracaoServico,
}

/// Uma linha do `servicos-erros.jsonl` (diagnóstico humano; não participa da retomada).
#[derive(Debug, Serialize)]
struct LinhaErro<'a> {
    titulo: &'a str,
    motivo: &'a str,
    quando: String,
}

/// Árvore-fonte: `data/<id>/docs/servicos` (irmã da `docs/pareceres` que o `extrair` consome).
fn docs_servicos_dir(entity: &EntityConfig) -> Result<PathBuf> {
    let base = Path::new(&entity.data_dir)
        .parent()
        .ok_or_else(|| format!("data_dir sem pai: {}", entity.data_dir))?;
    Ok(base.join("docs").join("servicos"))
}

/// System prompt: mesmo cálculo de caminho do `extrair` (`data_dir` é `../data/<id>/raw`).
fn load_prompt(entity: &EntityConfig) -> Result<String> {
    let path = Path::new(&entity.data_dir)
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| format!("data_dir inesperado: {}", entity.data_dir))?
        .join("prompts/extracao-servicos.txt");
    std::fs::read_to_string(&path).map_err(|e| {
        format!(
            "prompt de extração de serviços ausente ({}): {e}",
            path.display()
        )
        .into()
    })
}

/// Um serviço canônico no índice leve: onde reler o corpo + identidade. NÃO carrega o corpo.
struct DocIndex {
    caminho: PathBuf,
    titulo: String,
    link: String,
}

fn hash_corpo(corpo: &str) -> u64 {
    let mut h = DefaultHasher::new();
    corpo.trim().hash(&mut h);
    h.finish()
}

/// Varre a árvore e monta o índice leve **deduplicado por título**, em ordem estável (nome do
/// arquivo). A 1ª ocorrência vence (título/link vêm dela). Arquivo que não parseia é erro; título
/// repetido com corpo divergente também (ver o cabeçalho do módulo).
fn indexar(dir: &Path) -> Result<(Vec<DocIndex>, usize)> {
    let mut caminhos: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "md"))
        .collect();
    caminhos.sort();
    let arquivos = caminhos.len();

    let mut out: Vec<DocIndex> = Vec::new();
    let mut visto: HashMap<String, (u64, PathBuf)> = HashMap::new();
    for caminho in caminhos {
        let texto = std::fs::read_to_string(&caminho)?;
        let (header, corpo) = mddoc_servico::parse_doc_servico(&texto).map_err(|e| {
            format!(
                "`{}` não parseia ({e}) — corrija antes de rodar o extrair-servicos",
                caminho.display()
            )
        })?;
        let h = hash_corpo(&corpo);
        if let Some((h0, primeiro)) = visto.get(&header.titulo) {
            if *h0 != h {
                return Err(format!(
                    "título repetido com corpo DIVERGENTE: {:?}\n  {}\n  {}\n\
                     A dedup por título pressupõe corpos idênticos entre públicos. \
                     Corrija a fonte ou passe a chavear a extração por link.",
                    header.titulo,
                    primeiro.display(),
                    caminho.display()
                )
                .into());
            }
            continue;
        }
        visto.insert(header.titulo.clone(), (h, caminho.clone()));
        out.push(DocIndex {
            caminho,
            titulo: header.titulo,
            link: header.link,
        });
    }
    Ok((out, arquivos))
}

/// Lê as linhas válidas do JSONL + se havia cauda truncada. Mesma regra do `extrair`: malformada no
/// MEIO é erro alto; no FIM é queda no meio de um append — avisa, descarta e o serviço volta a pendente.
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
                     descartada, o serviço será re-extraído.",
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

/// Regrava o JSONL inteiro, atômico. SÓ para descartar cauda truncada ou aplicar `--force`.
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

/// Valida e desserializa a resposta (pura, testável). Schema exato via serde + `texto` não-vazio.
fn validar_extracao(answer: &str) -> std::result::Result<ExtracaoServico, String> {
    let limpo = descercar(answer);
    let ex: ExtracaoServico =
        serde_json::from_str(limpo).map_err(|e| format!("JSON inválido: {e}"))?;
    if ex.sistemas.iter().any(|s| s.texto.trim().is_empty()) {
        return Err("sistema com `texto` vazio".into());
    }
    Ok(ex)
}

/// Extração placeholder (dev-only). `prompt_versao: 0` marca fake.
fn fake_extracao(titulo: &str) -> ExtracaoServico {
    ExtracaoServico {
        sistemas: vec![Sistema {
            texto: format!("[FAKE] Portal e-CAC citado em {titulo}"),
        }],
        temas: vec!["fake".into()],
    }
}

/// Extração gerada + o headroom de RPD lido no header da resposta (para o proactive-stop).
struct Gerada {
    extracao: ExtracaoServico,
    remaining_requests: Option<u64>,
    reset_requests: Option<String>,
}

/// Gera a extração real de um serviço, com **uma** re-tentativa (resgate com `reasoning_effort=low`,
/// mesma doutrina do `extrair`: quebra o runaway de reasoning sem perder precisão no caminho feliz).
fn gerar_extracao(
    rt: &tokio::runtime::Runtime,
    params: &auli_llm::LlmParams,
    system_prompt: &str,
    titulo: &str,
    corpo: &str,
) -> std::result::Result<Gerada, String> {
    let (corpo_trunc, truncou) = truncar_corpo(corpo, CORPO_MAX_CHARS);
    if truncou {
        eprintln!("ℹ️  {titulo}: corpo truncado em {CORPO_MAX_CHARS} chars.");
    }
    // Só título + descrição: classe e público NÃO vão ao LLM de propósito — se fossem, os temas
    // ecoariam a taxonomia do portal e o grafo viraria tautologia. Eles entram como overlay
    // determinístico no `grafo-servicos`.
    let user_msg = format!("Serviço: {titulo}\n\nDescrição:\n{corpo_trunc}");

    for tentativa in 1..=2u32 {
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
            Err(motivo) => eprintln!(
                "↻ {titulo}: validação falhou ({motivo}); re-tentando 1× com reasoning_effort=low."
            ),
        }
    }
    unreachable!("o loop retorna em ambas as tentativas")
}

pub fn run(entity: &EntityConfig, opts: ExtracaoOpts) -> Result<()> {
    // 1. A árvore de serviços é a fonte.
    let dir = docs_servicos_dir(entity)?;
    if !dir.exists() {
        return Err(format!(
            "árvore ausente: {} — rode `auli update --entity {}` antes (ela materializa os `.md`).",
            dir.display(),
            entity.id
        )
        .into());
    }

    // 2. Índice leve deduplicado por título + estado da saída (a retomada mora no JSONL).
    let (docs, arquivos) = indexar(&dir)?;
    let out_dir = extracao_dir(entity)?;
    let out_path = out_dir.join("servicos-extracao.jsonl");
    let err_path = out_dir.join("servicos-erros.jsonl");
    let (mut linhas, cauda_truncada) = ler_linhas(&out_path)?;

    // `--force <titulo>`: precisa existir na árvore; a linha dele (se houver) sai — vira pendente.
    let mut force_removeu = false;
    if let Some(alvo) = opts.force.as_deref() {
        if !docs.iter().any(|d| d.titulo == alvo) {
            return Err(
                format!("--force {alvo:?}: nenhum serviço com esse `titulo` na árvore.").into(),
            );
        }
        let antes = linhas.len();
        linhas.retain(|l| l.titulo != alvo);
        force_removeu = linhas.len() != antes;
    }

    if !opts.dry_run && (cauda_truncada || force_removeu) {
        std::fs::create_dir_all(&out_dir)?;
        regravar_linhas(&out_path, &linhas)?;
    }

    let ja: HashSet<&str> = linhas.iter().map(|l| l.titulo.as_str()).collect();
    let total = docs.len();
    let pendentes_idx: Vec<&DocIndex> = docs
        .iter()
        .filter(|d| !ja.contains(d.titulo.as_str()))
        .collect();
    let pendentes = pendentes_idx.len();
    let ja_extraidos = total - pendentes;
    println!(
        "📊 {}: arquivos {arquivos} → serviços {total} (dedup por título) | já-extraídos {ja_extraidos} | pendentes {pendentes}",
        entity.id
    );

    // 3. Dry-run: estimativa de tokens dos pendentes, sem escrever nada.
    if opts.dry_run {
        let mut chars = 0usize;
        for d in &pendentes_idx {
            let texto = std::fs::read_to_string(&d.caminho)?;
            if let Ok((h, corpo)) = mddoc_servico::parse_doc_servico(&texto) {
                chars += h.titulo.chars().count() + corpo.chars().count();
            }
        }
        println!(
            "🔎 dry-run: ~{} tokens de entrada nos {pendentes} pendentes (nada foi escrito).",
            milhar(chars / 4)
        );
        return Ok(());
    }

    // 4. Config LLM: lida UMA vez, e SÓ na geração real.
    let real = !opts.fake && pendentes > 0;
    let llm = if real {
        let params = auli_llm::LlmParams {
            api_url: env_com_fallback("EXTRACAO_API_URL", "LLM_API_URL")?,
            api_key: env_com_fallback("EXTRACAO_API_KEY", "LLM_API_KEY")?,
            model: env_com_fallback("EXTRACAO_API_MODEL", "LLM_API_MODEL")?,
            temperature: 0.1, // extração literal: fidelidade, não diversidade
            max_completion_tokens: 16384, // reasoning + JSON dividem o orçamento (ver `extracao.rs`)
            timeout: Duration::from_secs(60),
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

    // 5. Serviço a serviço: lê → gera → append. `--limit` limita os PROCESSADOS (gerados + falhas).
    let mut gerados = 0usize;
    let mut falhas = 0usize;
    let limit = opts.limit.unwrap_or(usize::MAX);
    for d in &pendentes_idx {
        if gerados + falhas >= limit {
            break;
        }
        let texto = std::fs::read_to_string(&d.caminho)?;
        let (_header, corpo) = mddoc_servico::parse_doc_servico(&texto)
            .map_err(|e| format!("`{}` não parseia ({e})", d.caminho.display()))?;

        let (extracao, modelo, versao, headroom) = if let Some((params, system_prompt, rt)) = &llm {
            match gerar_extracao(rt, params, system_prompt, &d.titulo, &corpo) {
                Ok(g) => {
                    let hr = (g.remaining_requests, g.reset_requests);
                    (
                        g.extracao,
                        params.model.clone(),
                        EXTRACAO_SERVICOS_PROMPT_VERSION,
                        Some(hr),
                    )
                }
                Err(motivo) if e_rate_limit(&motivo) => {
                    eprintln!(
                        "🛑 {}: cota da API esgotada (rate limit) em '{}'. Abortando o lote — \
                         os pendentes ficam para a próxima rodada (idempotente).",
                        entity.id, d.titulo
                    );
                    break;
                }
                Err(motivo) => {
                    eprintln!("⚠️  {}: falha — {motivo}", d.titulo);
                    let erro = LinhaErro {
                        titulo: &d.titulo,
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
            (fake_extracao(&d.titulo), "fake".to_string(), 0, None)
        };

        let linha = Linha {
            titulo: d.titulo.clone(),
            link: d.link.clone(),
            prompt_versao: versao,
            modelo,
            gerada_em: now_iso8601(),
            extracao,
        };
        let json = serde_json::to_string(&linha).map_err(|e| format!("serializando linha: {e}"))?;
        append_linha(&out_path, &json)?;
        gerados += 1;

        if let Some((Some(restantes), reset)) = headroom
            && restantes <= RPD_MARGEM_PARADA
        {
            let quando = reset.as_deref().unwrap_or("desconhecido");
            println!(
                "🧮 {}: headroom de RPD esgotando ({restantes} restantes ≤ margem {RPD_MARGEM_PARADA}). \
                 Parando limpo (zero rejeição) — reset em {quando}.",
                entity.id
            );
            break;
        }
    }

    // 6. Relatório final + invariante de guarda.
    let pendentes_restantes = pendentes - gerados - falhas;
    println!(
        "✅ {}: serviços {total} | já-extraídos {ja_extraidos} | gerados {gerados} | falhas {falhas} | pendentes-restantes {pendentes_restantes}",
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

    /// EntityConfig num dir temporário exclusivo (limpo no início), com a MESMA geometria da
    /// produção: `data_dir` termina em `/raw`, a árvore cai em `<base>/docs/servicos`.
    fn temp_entity(tag: &str) -> EntityConfig {
        let base =
            std::env::temp_dir().join(format!("auli_extr_serv_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("raw")).unwrap();
        std::fs::create_dir_all(base.join("docs").join("servicos")).unwrap();
        EntityConfig {
            id: "xx".into(),
            name: "Teste".into(),
            system_prompt: String::new(),
            data_dir: base.join("raw").to_string_lossy().into_owned(),
        }
    }

    fn escrever_doc(entity: &EntityConfig, arquivo: &str, titulo: &str, tipo: &str, corpo: &str) {
        let header = mddoc_servico::ServicoHeader {
            titulo: titulo.into(),
            tipo: tipo.into(),
            classe: "Cadastro".into(),
            orgao: "RECEITA".into(),
            link: format!("https://exemplo/{tipo}/{titulo}"),
        };
        let dir = docs_servicos_dir(entity).unwrap();
        std::fs::write(
            dir.join(format!("{arquivo}.md")),
            mddoc_servico::render_doc_servico(&header, corpo),
        )
        .unwrap();
    }

    fn saida_path(entity: &EntityConfig) -> PathBuf {
        extracao_dir(entity)
            .unwrap()
            .join("servicos-extracao.jsonl")
    }

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
        std::fs::remove_dir_all(docs_servicos_dir(&e).unwrap()).unwrap();
        let err = run(&e, opts(true, None, None, false))
            .unwrap_err()
            .to_string();
        assert!(err.contains("árvore ausente"), "erro: {err}");
        assert!(err.contains("auli update"), "deve ensinar o remédio: {err}");
    }

    #[test]
    fn dedup_por_titulo_extrai_uma_vez_por_servico() {
        let e = temp_entity("dedup");
        // O MESMO serviço em dois públicos (corpo idêntico) + um outro.
        escrever_doc(
            &e,
            "a-cidadao",
            "Emitir Guia",
            "Cidadãos",
            "use o Portal e-CAC",
        );
        escrever_doc(
            &e,
            "a-empresa",
            "Emitir Guia",
            "Empresas",
            "use o Portal e-CAC",
        );
        escrever_doc(
            &e,
            "b-empresa",
            "Consultar Débito",
            "Empresas",
            "veja no SEF",
        );

        run(&e, opts(true, None, None, false)).unwrap();
        let linhas = ler_saida(&e);
        assert_eq!(
            linhas.len(),
            2,
            "586 arquivos → 422 serviços: dedup por título"
        );
        assert_eq!(linhas[0].titulo, "Emitir Guia");
        assert_eq!(
            linhas[0].link, "https://exemplo/Cidadãos/Emitir Guia",
            "a 1ª ocorrência (ordem de arquivo) vence"
        );
    }

    #[test]
    fn titulo_repetido_com_corpo_divergente_e_erro_alto() {
        let e = temp_entity("divergente");
        escrever_doc(&e, "a-cidadao", "Emitir Guia", "Cidadãos", "corpo A");
        escrever_doc(
            &e,
            "a-empresa",
            "Emitir Guia",
            "Empresas",
            "corpo B DIFERENTE",
        );
        let err = run(&e, opts(true, None, None, false))
            .unwrap_err()
            .to_string();
        assert!(err.contains("corpo DIVERGENTE"), "erro: {err}");
        assert!(err.contains("por link"), "deve apontar a saída: {err}");
    }

    #[test]
    fn fake_respeita_limit_e_retoma_sem_duplicar() {
        let e = temp_entity("fakelimit");
        escrever_doc(&e, "a-1", "Serviço A", "Cidadãos", "corpo A");
        escrever_doc(&e, "b-2", "Serviço B", "Empresas", "corpo B");

        run(&e, opts(true, Some(1), None, false)).unwrap();
        let l1 = ler_saida(&e);
        assert_eq!(l1.len(), 1);
        assert_eq!(l1[0].titulo, "Serviço A");
        assert_eq!(l1[0].prompt_versao, 0, "fake grava versão 0");

        run(&e, opts(true, None, None, false)).unwrap();
        run(&e, opts(true, None, None, false)).unwrap();
        let l2 = ler_saida(&e);
        assert_eq!(l2.len(), 2, "sem duplicatas na re-execução");
    }

    #[test]
    fn force_repende_e_substitui_sem_duplicar() {
        let e = temp_entity("force");
        escrever_doc(&e, "a-1", "Serviço A", "Cidadãos", "corpo A");
        escrever_doc(&e, "b-2", "Serviço B", "Empresas", "corpo B");
        run(&e, opts(true, None, None, false)).unwrap();

        run(&e, opts(true, None, Some("Serviço A"), false)).unwrap();
        let linhas = ler_saida(&e);
        assert_eq!(linhas.len(), 2, "force substitui, não duplica");
        assert_eq!(linhas.last().unwrap().titulo, "Serviço A");
    }

    #[test]
    fn dry_run_nao_escreve_nada() {
        let e = temp_entity("dry");
        escrever_doc(&e, "a-1", "Serviço A", "Cidadãos", "corpo A");
        run(&e, opts(true, None, None, true)).unwrap();
        assert!(
            !extracao_dir(&e).unwrap().exists(),
            "dry-run não pode criar nem escrever nada"
        );
    }

    #[test]
    fn cauda_truncada_e_descartada_e_regenerada() {
        let e = temp_entity("cauda");
        escrever_doc(&e, "a-1", "Serviço A", "Cidadãos", "corpo A");
        run(&e, opts(true, None, None, false)).unwrap();
        use std::io::Write as _;
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(saida_path(&e))
            .unwrap();
        f.write_all(b"{\"titulo\":\"TRUNC").unwrap();
        drop(f);

        escrever_doc(&e, "b-2", "Serviço B", "Empresas", "corpo B");
        run(&e, opts(true, None, None, false)).unwrap();
        let linhas = ler_saida(&e);
        assert_eq!(linhas.len(), 2, "cauda descartada; A preservado; B gerado");
    }

    // ── validar_extracao (pura) ──────────────────────────────────────────────

    #[test]
    fn validar_aceita_json_puro_e_com_cerca_markdown() {
        let puro = r#"{"sistemas":[{"texto":"Portal e-CAC"}],"temas":["parcelamento"]}"#;
        assert!(validar_extracao(puro).is_ok());
        let ex = validar_extracao(&format!("```json\n{puro}\n```"))
            .expect("cerca markdown deve ser tolerada");
        assert_eq!(ex.sistemas[0].texto, "Portal e-CAC");
    }

    #[test]
    fn validar_rejeita_chave_desconhecida_ausente_e_texto_vazio() {
        assert!(
            validar_extracao(r#"{"sistemas":[],"temas":[],"bonus":1}"#).is_err(),
            "deny_unknown_fields"
        );
        assert!(
            validar_extracao(r#"{"sistemas":[]}"#).is_err(),
            "campos obrigatórios"
        );
        assert!(
            validar_extracao(r#"{"sistemas":[{"texto":" "}],"temas":[]}"#).is_err(),
            "texto vazio é inútil no grafo"
        );
        assert!(
            validar_extracao(r#"{"sistemas":[],"temas":[]}"#).is_ok(),
            "arrays vazios são válidos (regra 10)"
        );
    }
}
