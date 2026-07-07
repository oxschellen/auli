//! Coleta dos serviços da SEFAZ-RN a partir da WP REST (`www.sefaz.rn.gov.br`).
//!
//! O portal é WordPress + SPA React e **não tem uma Carta de Serviços descritiva** (descoberta-rn.md).
//! O único catálogo estruturado é o custom post type **`servicos`** (`/wp-json/wp/v2/servicos`,
//! 15 itens): cards de atalho com `title`, `acf.categories` (classe) e `acf.link` (destino), **sem
//! corpo próprio**. Modelagem (decisão B): montamos os 15 cards e **enriquecemos** os que apontam
//! para um post (`/postagem/<slug>/`) buscando o corpo do post no campo ACF **`Matéria`**; os demais
//! (apps externos UVT/SEI, ou `link=false`) ficam com descrição vazia e o link como identidade.
//!
//! Público único "Serviços" (não há eixo de audiência); `classe` = categoria WP; `link` = `acf.link`
//! (ou o permalink do card quando `acf.link` é `false`); identidade = o link.

use std::collections::HashSet;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Result, bail};
use auli_contract::{Ocorrencia, Publico, ServicoRaw};
use auli_scraper_kit::clean;
use auli_scraper_kit::http::GetOpts;
use regex::Regex;
use scraper::Html;
use serde_json::Value;
use ureq::Agent;

const USER_AGENT: &str =
    "AuliBot/0.1 (+https://github.com/oxschellen/auli; carlos.schellenberger@gmail.com)";

const BASE: &str = "https://www.sefaz.rn.gov.br";
const SERVICOS_URL: &str = "https://www.sefaz.rn.gov.br/wp-json/wp/v2/servicos?per_page=100";

const ORGAO: &str = "SEFAZ-RN";
const PUBLICO_NOME: &str = "Serviços";
const PUBLICO_SLUG: &str = "servicos-gerais";
const CLASSE_FALLBACK: &str = "Geral";
/// Cortesia entre GETs (a listagem + ~5 posts).
const COURTESY: Duration = Duration::from_millis(500);
/// Guard: piso de serviços (o CPT tem 15).
const MIN_SERVICOS: usize = 12;

/// Raspa a WP REST e devolve `(items, publicos_ordem)` prontos para o snapshot v3.
pub fn scrape(
    data_dir: &str,
    use_cache: bool,
) -> Result<(Vec<ServicoRaw>, Vec<Publico>), Box<dyn std::error::Error>> {
    let agent = auli_scraper_kit::build_agent(USER_AGENT, Some(Duration::from_secs(30)));
    let mut pending: Vec<(String, String)> = Vec::new();

    // 1) Os 15 cards do CPT `servicos`.
    let json = load(&agent, data_dir, SERVICOS_URL, use_cache, &mut pending)?;
    let cards: Vec<Value> = serde_json::from_str(&json)
        .map_err(|e| format!("JSON de wp/v2/servicos inválido: {e}"))?;
    println!("RN: {} cards no CPT servicos", cards.len());

    let post_slug_re = Regex::new(r"/postagem/([^/]+)/?$").unwrap();

    // 2) Um `ServicoRaw` por card; enriquece os que linkam para um post.
    let mut items: Vec<ServicoRaw> = Vec::new();
    let mut vistos: HashSet<String> = HashSet::new();
    for card in &cards {
        let titulo = html_to_text(str_at(card, &["title", "rendered"]));
        if titulo.is_empty() {
            continue;
        }
        // `acf.link` é string (destino) ou `false` (sem destino → usa o permalink do card).
        let link_ext = card
            .pointer("/acf/link")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty());
        let link = match link_ext {
            // `acf.link` pode vir como caminho relativo (ex.: "/documentos/…") → absolutiza.
            Some(l) if l.starts_with('/') => format!("{}{}", BASE, l),
            Some(l) => l.to_string(),
            None => str_at(card, &["link"]).to_string(),
        };
        if link.is_empty() || !vistos.insert(link.clone()) {
            continue;
        }

        // Enriquecimento: se o destino é um post, o corpo está no ACF `Matéria`.
        let descricao = match link_ext.and_then(|l| post_slug_re.captures(l)) {
            Some(c) => {
                let slug = &c[1];
                let url = format!("{}/wp-json/wp/v2/postagem?slug={}", BASE, slug);
                let post_json = load(&agent, data_dir, &url, use_cache, &mut pending)?;
                materia_do_post(&post_json)
            }
            None => String::new(),
        };

        let classes = classes_do_card(card);
        let ocorrencias = classes
            .into_iter()
            .map(|classe| Ocorrencia { publico: PUBLICO_NOME.to_string(), classe })
            .collect();

        items.push(ServicoRaw { titulo, descricao, link, orgao: ORGAO.to_string(), ocorrencias });
    }

    validar(&items)?;

    for (url, raw) in &pending {
        auli_scraper_kit::cache::write(data_dir, url, raw);
    }

    let com_desc = items.iter().filter(|s| !s.descricao.is_empty()).count();
    println!("RN: {} serviços ({} com descrição rica de post)", items.len(), com_desc);
    let publicos_ordem =
        vec![Publico { nome: PUBLICO_NOME.to_string(), slug: PUBLICO_SLUG.to_string() }];
    Ok((items, publicos_ordem))
}

/// GET (JSON) com cache. Miss + `--usecache` = erro. Rede -> `pending` + cortesia.
fn load(
    agent: &Agent,
    data_dir: &str,
    url: &str,
    use_cache: bool,
    pending: &mut Vec<(String, String)>,
) -> Result<String> {
    if let Some(cached) = auli_scraper_kit::cache::read(data_dir, url) {
        return Ok(cached);
    }
    if use_cache {
        bail!("cache vazio para {} (--usecache, sem rede). Rode uma coleta com rede primeiro.", url);
    }
    let body = auli_scraper_kit::http::get_string(
        agent,
        url,
        &GetOpts { log_prefix: "RN", accept: Some("application/json"), ..Default::default() },
    )?;
    pending.push((url.to_string(), body.clone()));
    sleep(COURTESY);
    Ok(body)
}

/// Corpo do post: o ACF `Matéria` (HTML) do primeiro resultado da busca por slug (`content.rendered`
/// desse tema é `null`). Vazio se o post sumiu ou não tem `Matéria`.
fn materia_do_post(post_json: &str) -> String {
    let posts: Value = match serde_json::from_str(post_json) {
        Ok(v) => v,
        Err(_) => return String::new(),
    };
    let materia = posts
        .get(0)
        .and_then(|p| p.pointer("/acf/Matéria"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    html_to_text(materia)
}

/// Classes do card = nomes das categorias WP (`acf.categories[].name`), dedup na ordem; `Geral` se vazio.
fn classes_do_card(card: &Value) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    if let Some(cats) = card.pointer("/acf/categories").and_then(Value::as_array) {
        for cat in cats {
            if let Some(nome) = cat.get("name").and_then(Value::as_str) {
                let nome = html_to_text(nome);
                if !nome.is_empty() && !out.contains(&nome) {
                    out.push(nome);
                }
            }
        }
    }
    if out.is_empty() {
        out.push(CLASSE_FALLBACK.to_string());
    }
    out
}

/// Atalho: string em um caminho de chaves aninhadas (`""` se ausente).
fn str_at<'a>(v: &'a Value, path: &[&str]) -> &'a str {
    let mut cur = v;
    for k in path {
        match cur.get(k) {
            Some(next) => cur = next,
            None => return "",
        }
    }
    cur.as_str().unwrap_or("")
}

/// HTML -> texto: tags viram espaço, entidades decodificadas (html5ever), clean.
fn html_to_text(html: &str) -> String {
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

/// Guard (princípio D-RJ5): reprova coleta capada (o CPT/markup mudou).
fn validar(items: &[ServicoRaw]) -> Result<()> {
    if items.len() < MIN_SERVICOS {
        bail!(
            "catálogo capado? só {} serviço(s) (mínimo {}). O CPT `servicos` da WP REST pode ter \
             mudado; se veio do cache, limpe data/rn/raw/cache/ e re-raspe.",
            items.len(),
            MIN_SERVICOS
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Um card que aponta para um post (enriquecível) e outro para app externo (só link).
    const SERVICOS: &str = r#"[
      {
        "title": {"rendered": "Calend&#225;rio IPVA 2025"},
        "link": "https://www.sefaz.rn.gov.br/servicos/ipva/",
        "acf": {
          "link": "https://www.sefaz.rn.gov.br/postagem/ipva-calendario/",
          "categories": [{"name": "Finan&#231;as e Impostos"}]
        }
      },
      {
        "title": {"rendered": "Certid&#227;o Negativa"},
        "link": "https://www.sefaz.rn.gov.br/servicos/certidao/",
        "acf": {
          "link": "https://uvt.sefaz.rn.gov.br/#/services/certidao-negativa/emitir",
          "categories": [{"name": "Finan&#231;as e Impostos"}]
        }
      },
      {
        "title": {"rendered": "Carta de Servi&#231;os"},
        "link": "https://www.sefaz.rn.gov.br/servicos/carta-de-servicos/",
        "acf": {"link": false, "categories": []}
      }
    ]"#;

    const POSTAGEM: &str = r#"[
      {"acf": {"resumo": "", "Matéria": "<p>A SEFAZ RN divulga o <b>calend&#225;rio</b> do IPVA 2025.</p>"}}
    ]"#;

    #[test]
    fn materia_extrai_corpo_do_post() {
        let d = materia_do_post(POSTAGEM);
        assert_eq!(d, "A SEFAZ RN divulga o calendário do IPVA 2025.");
        // busca sem resultado -> vazio
        assert_eq!(materia_do_post("[]"), "");
    }

    #[test]
    fn classes_e_titulo_decodificam_entidades() {
        let cards: Vec<Value> = serde_json::from_str(SERVICOS).unwrap();
        assert_eq!(html_to_text(str_at(&cards[0], &["title", "rendered"])), "Calendário IPVA 2025");
        assert_eq!(classes_do_card(&cards[0]), vec!["Finanças e Impostos"]);
        // card "Carta de Serviços" sem categorias -> fallback "Geral"
        assert_eq!(classes_do_card(&cards[2]), vec!["Geral"]);
    }

    #[test]
    fn link_usa_permalink_quando_acf_link_e_false() {
        let cards: Vec<Value> = serde_json::from_str(SERVICOS).unwrap();
        // acf.link = false -> identidade = permalink do card
        let link_ext = cards[2].pointer("/acf/link").and_then(Value::as_str).filter(|s| !s.is_empty());
        assert!(link_ext.is_none());
        assert_eq!(str_at(&cards[2], &["link"]), "https://www.sefaz.rn.gov.br/servicos/carta-de-servicos/");
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
