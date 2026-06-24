// The `faqs` collection scraper.
//
// Walks a FAQ portal starting from a root menu page and produces a `FaqNode` tree, which is
// serialized to the collection's standard output file `<collection>.json` (e.g. `faqs.json`).
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
mod portal;

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
    /// Collection name, e.g. `"faqs"` — also the `Table::nome` of the contract output.
    pub collection: String,
    /// Directory the output JSON is written to, e.g. `"../data/rs/raw"`.
    pub data_dir: String,
    /// Directory where fetched pages are cached, e.g. `"../data/rs/cache/faqs"`.
    pub cache_dir: String,
    /// Offline mode: read only cached pages, never fetch (a cache miss becomes an error).
    pub use_cache: bool,
}

impl FaqSource {
    /// Full path of the contract output (the single structured output): `<data_dir>/<id>-<collection>.json`,
    /// holding a `Table<Faq>`. Replaces the legacy `<collection>.json` tree dump.
    pub fn contract_path(&self) -> String {
        format!("{}/{}-{}.json", self.data_dir, self.id, self.collection)
    }

    /// Full path of the human-readable print output: `<data_dir>/portal-<collection>.txt`.
    pub fn portal_path(&self) -> String {
        format!("{}/portal-{}.txt", self.data_dir, self.collection)
    }
}

/// A child link discovered on a menu page.
struct MenuItem {
    title: String,
    url: String,
}

/// Scrape the FAQ tree, then write the two outputs: the contract `Table<Faq>`
/// (`<id>-<collection>.json`, the single structured output) and the human-readable print
/// `portal-<collection>.txt`. The legacy tree dump `<collection>.json` is no longer written.
pub fn run(source: &FaqSource) -> Result<()> {
    let tree = scrape(source)?;

    // Contrato: achata a árvore para Vec<Faq> e grava Table<Faq>. Esta passa a ser a ÚNICA saída
    // estruturada (o engine lê isto); `faqs.json` (árvore) foi descartado.
    let items = flatten_faqs(&tree);
    let table = auli_contract::Table::new(source.id.clone(), source.collection.clone(), items);
    let contract_path = source.contract_path();
    let json = serde_json::to_string_pretty(&table)?;
    if let Some(parent) = Path::new(&contract_path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&contract_path, &json)?;
    println!("Wrote {} ({} faqs)", contract_path, table.len());

    // Print legível (auditoria) — formato inalterado, nunca lido de volta.
    let portal_path = source.portal_path();
    let portal = portal::render_portal_faqs(&tree);
    std::fs::write(&portal_path, &portal)?;
    println!("Wrote {} ({} bytes)", portal_path, portal.len());

    Ok(())
}

/// Flattens the FAQ tree into the contract's `Vec<Faq>`, using the SAME traversal as
/// `portal::render_portal_faqs` (start from the root's children; one `Faq` per `FaqItem` of each
/// leaf `Faq` node), so contract order matches the print order.
///
/// `text_to_embed` (D2) reproduces the key of the old engine `EmbedStrategy::QuestionKey`, which
/// embedded the `## pergunta` field = breadcrumb `origin` + the question text.
pub fn flatten_faqs(root: &FaqNode) -> Vec<auli_contract::Faq> {
    let mut out = Vec::new();
    for child in &root.children {
        collect_faqs(child, &mut out);
    }
    out
}

fn collect_faqs(node: &FaqNode, out: &mut Vec<auli_contract::Faq>) {
    if node.page_type == PageType::Faq {
        for item in &node.faq_items {
            let text_to_embed = if node.origin.is_empty() {
                item.pergunta.clone()
            } else {
                format!("{} {}", node.origin, item.pergunta)
            };
            out.push(auli_contract::Faq {
                pergunta: item.pergunta.clone(),
                resposta: item.resposta.clone(),
                origin: node.origin.clone(),
                url: node.url.clone(),
                text_to_embed,
            });
        }
    }
    for child in &node.children {
        collect_faqs(child, out);
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
    fn flatten_mirrors_portal_order_and_builds_embed_key() {
        // root(menu) -> [ leaf A (2 items, with origin), menu -> leaf B (1 item, no origin) ]
        let root = menu(
            "root",
            vec![
                faq_leaf("ua", "Inicial | A", &[("q1", "r1"), ("q2", "r2")]),
                menu("um", vec![faq_leaf("ub", "", &[("q3", "r3")])]),
            ],
        );

        let faqs = flatten_faqs(&root);

        // Depth-first from the root's children: q1, q2, q3 (same order as render_portal_faqs).
        assert_eq!(faqs.len(), 3);
        assert_eq!(faqs[0].pergunta, "q1");
        assert_eq!(faqs[0].url, "ua");
        assert_eq!(faqs[0].origin, "Inicial | A");
        // Key reproduces the old QuestionKey strategy: origin + question.
        assert_eq!(faqs[0].text_to_embed, "Inicial | A q1");
        assert_eq!(faqs[2].pergunta, "q3");
        // Empty origin -> the key is just the question.
        assert_eq!(faqs[2].text_to_embed, "q3");
    }
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
