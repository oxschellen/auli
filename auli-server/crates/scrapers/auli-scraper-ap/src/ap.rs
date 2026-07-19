//! Coleta dos serviços da SEFAZ-AP a partir do bundle Angular de `www.sefaz.ap.gov.br`.
//!
//! O portal é uma SPA Angular (FUSE). O catálogo de serviços COM descrição está **hardcoded no bundle
//! JS** — arrays `mock*` no chunk lazy `categorias_routes` — e é renderizado em runtime na página
//! `#/categorias/{cat}/{servico}` (nenhuma API dispara). O HTML servido é só o shell vazio, então
//! pegamos o dado de onde ele vive: o JS. Sem headless.
//!
//! Descoberta do chunk (o hash muda por deploy; o NOME é estável):
//! 1. `GET /` → o shell referencia `runtime.<hash>.js`.
//! 2. `GET runtime.<hash>.js` → mapa `"<CHUNK_NAME>":"<hash>"`.
//! 3. `GET <CHUNK_NAME>.<hash>.js` → o chunk com os `mock*`.
//!
//! Parser: por categoria (fatia de `const mock<X> =`), casa cada serviço
//! `route → introducao.titulo → introducao.descricao`. As chaves NÃO estão minificadas (estável). A
//! `descricao` é um template literal HTML autocontido (traz "o que é" + Quem Pode/Setor/Tipo) →
//! `html_to_text`. Modelagem (Cenário A): público único "Serviços"; `classe` = a categoria; `link` =
//! `…/#/categorias/{slug}/{route}`; identidade = o link.

use std::collections::HashSet;
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use auli_contract::{Ocorrencia, Publico, ServicoRaw};
use auli_scraper_kit::clean;
use auli_scraper_kit::http::GetOpts;
use regex::Regex;
use scraper::Html;

const USER_AGENT: &str =
    "AuliBot/0.1 (+https://github.com/oxschellen/auli; carlos.schellenberger@gmail.com)";

const BASE: &str = "https://www.sefaz.ap.gov.br";
/// Nome (estável) do chunk lazy das categorias; o hash é descoberto via runtime.
const CHUNK_NAME: &str = "src_app_modules_landing_page_categorias_categorias_routes_ts";

/// As 5 categorias: (nome do array `mock`, slug da rota, nome exibido = classe).
const CATEGORIAS: [(&str, &str, &str); 5] = [
    ("mockCadastro", "cadastro", "Cadastro"),
    ("mockIcms", "icms", "ICMS"),
    ("mockItcd", "itcmd", "ITCMD"),
    ("mockRegimeEspecial", "regime-especial", "Regime Especial"),
    ("mockVeiculo", "veiculos", "Veículos"),
];

/// Público único (não há eixo de audiência no dado).
const PUBLICO_NOME: &str = "Serviços";
const PUBLICO_SLUG: &str = "servicos-gerais";
/// Órgão de origem.
const ORGAO: &str = "SEFAZ-AP";
/// Guard: piso de serviços (os `mock*` têm 49; folga p/ baixo, pega chunk/parse quebrado).
const MIN_SERVICOS: usize = 44;
/// Chave de cache lógica do chunk (o hash real muda por deploy; a chave é fixa).
const CHUNK_CACHE_KEY: &str = "https://www.sefaz.ap.gov.br/categorias-chunk.js";

/// Um serviço parseado do mock.
struct Servico<'a> {
    slug: &'a str,
    classe: &'a str,
    route: String,
    titulo: String,
    descricao_html: String,
}

/// Raspa o catálogo e devolve `(items, publicos_ordem)` prontos para o snapshot v3.
pub fn scrape(
    data_dir: &str,
    use_cache: bool,
) -> Result<(Vec<ServicoRaw>, Vec<Publico>), Box<dyn std::error::Error>> {
    let agent = auli_scraper_kit::build_agent(USER_AGENT, Some(Duration::from_secs(30)));

    // O chunk é a fonte; cache lógico. Rede = descoberta em 3 saltos (shell → runtime → chunk).
    let (chunk, fetched) = match auli_scraper_kit::cache::read(data_dir, "servicos", CHUNK_CACHE_KEY) {
        Some(c) => {
            println!("Cache hit: chunk categorias");
            (c, false)
        }
        None => {
            if use_cache {
                return Err(anyhow!(
                    "cache vazio para o chunk (modo --usecache, sem rede). Rode uma coleta com rede \
                     primeiro."
                )
                .into());
            }
            let chunk_url = descobrir_chunk_url(&agent)?;
            let c = fetch(&agent, &chunk_url)?;
            if !c.contains("mockCadastro") {
                return Err(
                    format!("chunk sem 'mockCadastro' — estrutura mudou? ({})", chunk_url).into()
                );
            }
            (c, true)
        }
    };

    let servicos = parse_servicos(&chunk);
    let items = build_servicos(&servicos);
    validar(&items)?;

    if fetched {
        auli_scraper_kit::cache::write(data_dir, "servicos", CHUNK_CACHE_KEY, &chunk);
    }

    println!("AP: {} serviços (dedup por link)", items.len());
    let publicos_ordem =
        vec![Publico { nome: PUBLICO_NOME.to_string(), slug: PUBLICO_SLUG.to_string() }];
    Ok((items, publicos_ordem))
}

/// Descobre a URL do chunk: shell → `runtime.<hash>.js` → mapa nome→hash → `<CHUNK_NAME>.<hash>.js`.
fn descobrir_chunk_url(agent: &ureq::Agent) -> Result<String> {
    let shell = fetch(agent, &format!("{}/", BASE))?;
    let runtime_name = Regex::new(r"runtime\.[a-f0-9]+\.js")
        .unwrap()
        .find(&shell)
        .ok_or_else(|| anyhow!("runtime.<hash>.js não encontrado no shell (markup mudou?)"))?
        .as_str()
        .to_string();

    let runtime = fetch(agent, &format!("{}/{}", BASE, runtime_name))?;
    let hash = Regex::new(&format!(r#""{}"\s*:\s*"([a-f0-9]+)""#, regex::escape(CHUNK_NAME)))
        .unwrap()
        .captures(&runtime)
        .and_then(|c| c.get(1))
        .ok_or_else(|| anyhow!("hash do chunk '{}' não encontrado no runtime", CHUNK_NAME))?
        .as_str()
        .to_string();

    Ok(format!("{}/{}.{}.js", BASE, CHUNK_NAME, hash))
}

/// GET simples (texto), com retry/backoff via kit.
fn fetch(agent: &ureq::Agent, url: &str) -> Result<String> {
    auli_scraper_kit::http::get_string(agent, url, &GetOpts { log_prefix: "AP", ..Default::default() })
}

/// Parseia os arrays `mock*`: por categoria, casa `route → introducao.titulo → introducao.descricao`.
fn parse_servicos(chunk: &str) -> Vec<Servico<'static>> {
    // route (com/sem aspas) ... titulo ("..." ou '...') ... descricao (template literal `...`).
    let re = Regex::new(
        r#"(?s)["']?route["']?\s*:\s*["']([^"']+)["'].*?["']?titulo["']?\s*:\s*(?:"([^"]*)"|'([^']*)').*?["']?descricao["']?\s*:\s*`([^`]*)`"#,
    )
    .unwrap();

    // Fronteiras das 5 categorias (posições de `const mock<X> =`), ordenadas.
    let mut marks: Vec<(usize, usize)> = CATEGORIAS
        .iter()
        .enumerate()
        .filter_map(|(i, (mock, _, _))| chunk.find(&format!("const {} =", mock)).map(|p| (p, i)))
        .collect();
    marks.sort_by_key(|(p, _)| *p);

    let mut out = Vec::new();
    for k in 0..marks.len() {
        let (start, cat_idx) = marks[k];
        let end = marks.get(k + 1).map(|(p, _)| *p).unwrap_or(chunk.len());
        let (_, slug, classe) = CATEGORIAS[cat_idx];
        for cap in re.captures_iter(&chunk[start..end]) {
            let route = cap.get(1).map(|m| m.as_str()).unwrap_or_default();
            let titulo = cap.get(2).or_else(|| cap.get(3)).map(|m| m.as_str()).unwrap_or_default();
            let descricao_html = cap.get(4).map(|m| m.as_str()).unwrap_or_default();
            out.push(Servico {
                slug,
                classe,
                route: route.to_string(),
                titulo: titulo.to_string(),
                descricao_html: descricao_html.to_string(),
            });
        }
    }
    out
}

/// Monta os `ServicoRaw` (dedup por link).
fn build_servicos(servicos: &[Servico]) -> Vec<ServicoRaw> {
    let mut vistos: HashSet<String> = HashSet::new();
    let mut out: Vec<ServicoRaw> = Vec::new();
    for s in servicos {
        let titulo = clean(&s.titulo);
        if titulo.is_empty() || s.route.is_empty() {
            continue;
        }
        let link = format!("{}/#/categorias/{}/{}", BASE, s.slug, s.route);
        if !vistos.insert(link.clone()) {
            continue;
        }
        out.push(ServicoRaw {
            titulo,
            descricao: html_to_text(&s.descricao_html),
            link,
            orgao: ORGAO.to_string(),
            ocorrencias: vec![Ocorrencia {
                publico: PUBLICO_NOME.to_string(),
                classe: s.classe.to_string(),
            }],
        });
    }
    out
}

/// HTML da `descricao` -> texto (tags viram espaço; html5ever decodifica entidades; clean comprime).
fn html_to_text(html: &str) -> String {
    if html.trim().is_empty() {
        return String::new();
    }
    let mut spaced = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                spaced.push(' ');
            }
            _ if !in_tag => spaced.push(c),
            _ => {}
        }
    }
    let decoded: String = Html::parse_fragment(&spaced).root_element().text().collect();
    clean(&decoded)
}

/// Guard (princípio D-RJ5): reprova coleta capada (chunk/parse quebrado).
fn validar(items: &[ServicoRaw]) -> Result<()> {
    if items.len() < MIN_SERVICOS {
        bail!(
            "catálogo capado? só {} serviço(s) (mínimo {}). O chunk/estrutura `mock*` pode ter mudado; \
             se veio do cache, limpe data/ap/raw/cache/ e re-raspe.",
            items.len(),
            MIN_SERVICOS
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Fixture: 2 categorias, chaves JS (Cadastro) e JSON (Icms), com introducao + documentos (ruído).
    const CHUNK: &str = r#"
      const mockCadastro = [{
        route: 'mei',
        introducao: [{ titulo: 'Inscrição MEI', descricao: `Este serviço permite o MEI.<br><b>Quem Pode:</b> MEI ativo.` }],
        documentos: [{ titulo: 'RG', descricao: `documento` }]
      }, {
        route: 'simples-nacional',
        introducao: [{ titulo: 'Simples Nacional', descricao: `Adesão ao Simples.` }]
      }];
      const mockIcms = [{
        "route": "restituicao-icms",
        "introducao": [{ "titulo": "Restituição de ICMS", "descricao": `Pedido de restitui&ccedil;&atilde;o.` }]
      }];
    "#;

    #[test]
    fn parse_extrai_introducao_por_categoria() {
        let s = parse_servicos(CHUNK);
        assert_eq!(s.len(), 3, "2 do Cadastro + 1 do ICMS");
        let mei = s.iter().find(|x| x.route == "mei").unwrap();
        assert_eq!(mei.titulo, "Inscrição MEI");
        assert_eq!(mei.classe, "Cadastro");
        assert!(mei.descricao_html.contains("Quem Pode"));
        // pega a descricao da INTRODUCAO, não a de documentos.
        assert!(!mei.descricao_html.contains("documento"));
        let icms = s.iter().find(|x| x.route == "restituicao-icms").unwrap();
        assert_eq!(icms.classe, "ICMS"); // slug/classe do array JSON
    }

    #[test]
    fn build_monta_link_classe_descricao() {
        let items = build_servicos(&parse_servicos(CHUNK));
        assert_eq!(items.len(), 3);
        let mei = items.iter().find(|s| s.titulo == "Inscrição MEI").unwrap();
        assert_eq!(mei.link, "https://www.sefaz.ap.gov.br/#/categorias/cadastro/mei");
        assert_eq!(mei.orgao, "SEFAZ-AP");
        assert_eq!(mei.ocorrencias[0].publico, "Serviços");
        assert_eq!(mei.ocorrencias[0].classe, "Cadastro");
        // descricao: HTML -> texto (tags fora, entidade decodificada).
        assert!(mei.descricao.starts_with("Este serviço permite o MEI."));
        assert!(mei.descricao.contains("Quem Pode: MEI ativo."));
        assert!(!mei.descricao.contains('<'));
        let icms = items.iter().find(|s| s.link.contains("/icms/")).unwrap();
        assert_eq!(icms.descricao, "Pedido de restituição."); // &ccedil;&atilde; decodificados
    }

    #[test]
    fn dedup_por_link() {
        // O mesmo (slug, route) repetido DENTRO do array vira um único serviço.
        let c = r#"
          const mockCadastro = [
            { route: 'mei', introducao: [{ titulo: 'MEI', descricao: `a` }] },
            { route: 'mei', introducao: [{ titulo: 'MEI dup', descricao: `b` }] }
          ];
        "#;
        let items = build_servicos(&parse_servicos(c));
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn validar_reprova_capado() {
        let poucos = vec![ServicoRaw {
            titulo: "x".into(),
            descricao: String::new(),
            link: "l".into(),
            orgao: ORGAO.into(),
            ocorrencias: vec![],
        }];
        assert!(validar(&poucos).unwrap_err().to_string().contains("capado"));
    }
}
