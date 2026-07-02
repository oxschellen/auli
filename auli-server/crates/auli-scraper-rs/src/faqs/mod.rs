// The `faqs` scraper (SEFAZ-RS).
//
// Walks a FAQ portal starting from a root menu page and produces an in-memory `FaqNode` tree. `run`
// flattens the tree into the collection snapshot (`colecoes.faqs`, a `Vec<FaqRaw>`); the derivation of
// the engine artifacts (`<id>-faqs.json`, `portal-faqs.txt`) is the `auli-collections process` step.
// The tree is never serialized to disk.
//
// How the portal works:
//   - Each page declares a `data-matriz-source-uri` attribute. Its template id tells us whether the
//     page is a menu (`categoriafaq`) or a FAQ list (`sanfona.comnavegacao`); anything else is a
//     plain page (`Geral`).
//   - The actual list/FAQ markup is loaded from an AJAX endpoint (`<base><source_uri>&...&pageSize=100`)
//     that returns JSON with the rendered HTML in a `body` field.
//   - We recurse through menus, collecting FAQ question/answer pairs at the leaves.

mod faq;
mod fetch;
mod html;

pub use faq::{FaqItem, FaqNode, PageType};

use std::collections::HashSet;
use std::path::Path;

use ureq::Agent;

use crate::errors::Result;

/// Configuration for one FAQ scrape. Kept generic so other entities/portals can reuse it.
pub struct FaqSource {
    /// Entity id (e.g. `"rs"`), used as `Table::id` in the contract output.
    pub id: String,
    /// Origin used to resolve relative menu hrefs and build AJAX URLs, e.g.
    /// `"https://atendimento.receita.rs.gov.br"`.
    pub base_url: String,
    /// Root menu page to start walking from.
    pub root_url: String,
    /// Title for the root node of the tree.
    pub root_title: String,
    /// Directory the output JSON is written to, e.g. `"../data/rs/raw"`.
    pub data_dir: String,
    /// Directory where fetched pages are cached, e.g. `"../data/rs/cache/faqs"`.
    pub cache_dir: String,
    /// Offline mode: read only cached pages, never fetch (a cache miss becomes an error).
    pub use_cache: bool,
}

/// A child link discovered on a menu page.
struct MenuItem {
    title: String,
    url: String,
}

/// Scrape the FAQ tree and write the collection *snapshot* (`colecoes.faqs`). The engine artifacts
/// (`<id>-faqs.json`, `portal-faqs.txt`) are no longer written here — [`process`] derives them from
/// the snapshot, offline.
pub fn run(source: &FaqSource) -> Result<()> {
    let tree = scrape(source)?;
    auli_scraper_kit::snapshot::write_faqs(
        &source.id,
        &source.data_dir,
        &crate::scraper_info(),
        flatten_faqs_raw(&tree),
    )?;
    Ok(())
}

/// Achata a árvore em `Vec<FaqRaw>` para o snapshot: começa pelos filhos da raiz e emite um `FaqRaw`
/// por `FaqItem` de cada nó-folha `Faq`, na ordem em que aparecem. Não materializa `text_to_embed`
/// (isso é do [`process`]).
pub fn flatten_faqs_raw(root: &FaqNode) -> Vec<auli_contract::FaqRaw> {
    let mut out = Vec::new();
    for child in &root.children {
        collect_faqs_raw(child, &mut out);
    }
    out
}

fn collect_faqs_raw(node: &FaqNode, out: &mut Vec<auli_contract::FaqRaw>) {
    if node.page_type == PageType::Faq {
        for item in &node.faq_items {
            out.push(auli_contract::FaqRaw {
                pergunta: item.pergunta.clone(),
                resposta: item.resposta.clone(),
                origin: node.origin.clone(),
                url: node.url.clone(),
            });
        }
    }
    for child in &node.children {
        collect_faqs_raw(child, out);
    }
}

/// Walk the portal and return the FAQ tree without writing anything.
pub fn scrape(source: &FaqSource) -> Result<FaqNode> {
    let agent = fetch::build_agent();
    let mut visited: HashSet<String> = HashSet::new();
    visited.insert(source.root_url.clone());

    println!("Walking site from: {}", source.root_url);
    walk(
        &agent,
        source,
        &source.root_url,
        &source.root_title,
        &mut visited,
    )
}

fn walk(
    agent: &Agent,
    source: &FaqSource,
    url: &str,
    title: &str,
    visited: &mut HashSet<String>,
) -> Result<FaqNode> {
    let filename = url_to_filename(url);
    let cache_dir = Path::new(&source.cache_dir);
    let path_html = cache_dir.join(format!("{}.html", filename));
    let path_body = cache_dir.join(format!("{}_body.html", filename));

    let page_html = fetch::get_web_page_html(agent, url, &path_html, source.use_cache)?;
    let (source_uri, page_type) = get_page_info(&page_html);

    let mut node = FaqNode {
        title: title.to_string(),
        url: url.to_string(),
        page_type,
        origin: String::new(),
        children: Vec::new(),
        faq_items: Vec::new(),
    };

    if source_uri.is_empty() {
        return Ok(node);
    }

    let ajax_url = format!(
        "{}{}&currentPage=1&pageSize=100",
        source.base_url, source_uri
    );
    let body_html = match fetch::get_web_page_ajax_body_html(
        agent,
        url,
        &path_body,
        &ajax_url,
        source.use_cache,
    ) {
        Ok(html) => html,
        Err(e) => {
            eprintln!("Error fetching body for {}: {}", url, e);
            return Ok(node);
        }
    };

    match page_type {
        PageType::Faq => {
            node.origin = html::extract_breadcrumbs(&page_html);
            node.title = html::extract_page_title(&page_html);
            node.faq_items = extract_faq_items(&body_html);
        }
        PageType::Menu => {
            for item in extract_menu_items(&source.base_url, &body_html) {
                if item.url.is_empty() || visited.contains(&item.url) {
                    continue;
                }
                visited.insert(item.url.clone());
                println!("  {}", item.url);
                match walk(agent, source, &item.url, &item.title, visited) {
                    Ok(child) => node.children.push(child),
                    Err(e) => eprintln!("Error walking {}: {}", item.url, e),
                }
            }
        }
        PageType::Geral => {}
    }

    Ok(node)
}

/// Reads `data-matriz-source-uri` and classifies the page by its template id.
fn get_page_info(html: &str) -> (String, PageType) {
    let source_uri = html::extract_attribute(html, "data-matriz-source-uri").unwrap_or_default();

    let page_type = if source_uri.contains("pagina.listapagina.sanfona.comnavegacao") {
        PageType::Faq
    } else if source_uri.contains("pagina.listapagina.categoriafaq") {
        PageType::Menu
    } else {
        PageType::Geral
    };

    (source_uri, page_type)
}

/// Extracts child menu links from a menu page's AJAX body (one per `panel panel-default`).
fn extract_menu_items(base_url: &str, html: &str) -> Vec<MenuItem> {
    let mut items = Vec::new();

    for panel in panels(html) {
        let title = html::extract_question_title_from_class(panel, r#"class="panel-title""#);
        let href = html::extract_question_href_from_class(panel, r#"class="panel-heading""#);
        let url = if href.is_empty() {
            String::new()
        } else {
            format!("{}{}", base_url, href)
        };
        items.push(MenuItem { title, url });
    }

    items
}

/// Extracts question/answer pairs from a FAQ page's AJAX body (one per `panel panel-default`).
fn extract_faq_items(html: &str) -> Vec<FaqItem> {
    let mut items = Vec::new();

    for panel in panels(html) {
        let pergunta = html::extract_panel_title(panel);
        let resposta = html::extract_panel_body_text(panel).trim().to_string();
        if pergunta.is_empty() || resposta.is_empty() {
            continue;
        }
        items.push(FaqItem { pergunta, resposta });
    }

    items
}

/// Splits an AJAX body into `class="panel panel-default"` slices.
fn panels(html: &str) -> Vec<&str> {
    const MARKER: &str = r#"class="panel panel-default""#;
    let mut slices = Vec::new();
    let mut start = 0;

    while let Some(rel) = html[start..].find(MARKER) {
        let panel_start = start + rel;
        let next = html[panel_start + 30..]
            .find(MARKER)
            .map(|p| panel_start + 30 + p)
            .unwrap_or(html.len());
        slices.push(&html[panel_start..next]);
        start = next;
    }

    slices
}

/// Turns a URL into a safe cache filename (non `[A-Za-z0-9-_.]` chars become `_`).
fn url_to_filename(url: &str) -> String {
    url.chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' => c,
            _ => '_',
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn faq_leaf(url: &str, origin: &str, items: &[(&str, &str)]) -> FaqNode {
        FaqNode {
            title: url.to_string(),
            url: url.to_string(),
            page_type: PageType::Faq,
            origin: origin.to_string(),
            children: Vec::new(),
            faq_items: items
                .iter()
                .map(|(p, r)| FaqItem { pergunta: p.to_string(), resposta: r.to_string() })
                .collect(),
        }
    }

    fn menu(url: &str, children: Vec<FaqNode>) -> FaqNode {
        FaqNode {
            title: url.to_string(),
            url: url.to_string(),
            page_type: PageType::Menu,
            origin: String::new(),
            children,
            faq_items: Vec::new(),
        }
    }

    #[test]
    fn flatten_raw_mirrors_portal_order_and_derives_embed_key() {
        // root(menu) -> [ leaf A (2 items, with origin), menu -> leaf B (1 item, no origin) ]
        let root = menu(
            "root",
            vec![
                faq_leaf("ua", "Inicial | A", &[("q1", "r1"), ("q2", "r2")]),
                menu("um", vec![faq_leaf("ub", "", &[("q3", "r3")])]),
            ],
        );

        let raw = flatten_faqs_raw(&root);
        // Depth-first from the root's children: q1, q2, q3 (a derivação em collections mantém a ordem).
        assert_eq!(raw.iter().map(|f| f.pergunta.as_str()).collect::<Vec<_>>(), ["q1", "q2", "q3"]);
        assert_eq!(raw[0].url, "ua");
        assert_eq!(raw[0].origin, "Inicial | A");
        assert_eq!(raw[2].origin, "");
    }
}
