use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};

mod extract_page_urls;
use extract_page_urls::{
    BASE_URL, extract_menu_page_items, get_page_info, get_web_page_ajax_body_html,
    get_web_page_html,
};

mod types;
use types::{Entry, FaqItem, PageType, Result, SiteNode};

mod utils;
use utils::{parse_faq_items, url_to_filename};

mod html_utils;
use html_utils::{
    extract_breadcrumbs, extract_page_title, extract_panel_body_text, extract_panel_title,
};

fn main() -> Result<()> {
    let root_entry = Entry::new(
        "Perguntas Frequentes".to_string(),
        "https://atendimento.receita.rs.gov.br/perguntas-frequentes".to_string(),
        PageType::Menu,
        "".to_string(),
    );

    let mut visited: HashSet<String> = HashSet::new();
    visited.insert(root_entry.url.clone());

    println!("Walking site from: {}", root_entry.url);
    let faq_site_tree = walk_site(&root_entry, &mut visited)?;

    let json = serde_json::to_string_pretty(&faq_site_tree)?;
    std::fs::write("faq_site_tree.json", &json)?;
    println!(
        "Done. Output written to faq_site_tree.json ({} bytes)\n",
        json.len()
    );

    print_faq_site_tree("faq_site_tree.json", "faq_site_tree.txt")?;
    println!("Tree written to faq_site_tree.txt");

    let portal_faqs = gerar_arquivo_portal_faqs_txt(json);
    std::fs::write("portal-faqs.txt", &portal_faqs)?;
    println!("Portal FAQs written to portal_faqs.txt");

    Ok(())
}

pub fn print_faq_site_tree(input_path: &str, output_path: &str) -> Result<()> {
    let json = std::fs::read_to_string(input_path)?;
    let root: SiteNode = serde_json::from_str(&json)?;
    let file = std::fs::File::create(output_path)?;
    let mut out = std::io::BufWriter::new(file);
    print_node(&root, 0, &mut out)
}

pub fn gerar_arquivo_portal_faqs_txt(json: String) -> String {
    let root: SiteNode = match serde_json::from_str(&json) {
        Ok(r) => r,
        Err(_) => return String::new(),
    };
    let mut out = String::new();
    let mut counter = 1usize;
    for child in &root.children {
        write_portal_faq_items(child, &[], &mut counter, &mut out);
    }
    out
}

fn write_portal_faq_items(
    node: &SiteNode,
    ancestors: &[String],
    counter: &mut usize,
    out: &mut String,
) {
    if node.page_type == PageType::Faq {
        for item in &node.faq_items {
            out.push_str(&format!("// {}.\n", counter));
            out.push_str("## pergunta\n");
            if !node.origin.is_empty() {
                out.push_str(&node.origin);
                out.push('\n');
            }
            // for ancestor in ancestors {
            //     out.push_str(ancestor);
            //     out.push('\n');
            // }
            // out.push_str(&node.title);
            // out.push('\n');
            out.push_str(&item.pergunta);
            out.push('\n');
            out.push('\n');
            out.push_str("## resposta\n");
            out.push_str(&item.resposta);
            out.push('\n');
            out.push_str(&format!("Link: {}\n", node.url));
            out.push('\n');
            *counter += 1;
        }
    }

    let mut child_ancestors = ancestors.to_vec();
    child_ancestors.push(node.title.clone());
    for child in &node.children {
        write_portal_faq_items(child, &child_ancestors, counter, out);
    }
}

fn print_node(node: &SiteNode, depth: usize, out: &mut dyn Write) -> Result<()> {
    let pad = "  ".repeat(depth);
    writeln!(out, "{}{}", pad, node.title)?;

    let q_pad = "  ".repeat(depth + 1);
    for item in &node.faq_items {
        writeln!(out, "{}Q: {}", q_pad, item.pergunta)?;
        let first_line = item.resposta.lines().next().unwrap_or("");
        let answer: String = first_line.chars().take(100).collect();
        let ellipsis = if first_line.len() > 100 { "…" } else { "" };
        writeln!(out, "{}A: {}{}", q_pad, answer, ellipsis)?;
    }

    for child in &node.children {
        print_node(child, depth + 1, out)?;
    }
    Ok(())
}

fn walk_site(entry: &Entry, visited: &mut HashSet<String>) -> Result<SiteNode> {
    let url = &entry.url;
    let filename = url_to_filename(url);
    let path_html = PathBuf::from(format!("./data/{}.html", filename));
    let path_body = PathBuf::from(format!("./data/{}_body.html", filename));

    let html = get_web_page_html(url, &path_html)?;
    let (source_uri, page_type) = get_page_info(&html)?;

    let mut node = SiteNode {
        title: entry.title.clone(),
        url: url.clone(),
        page_type: page_type.clone(),
        origin: String::new(),
        children: Vec::new(),
        faq_items: Vec::new(),
    };

    if source_uri.is_empty() {
        return Ok(node);
    }

    let ajax_url = format!("{}{}&currentPage=1&pageSize=100", BASE_URL, source_uri);
    let body_html = match get_web_page_ajax_body_html(url, &path_body, &ajax_url) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("Error fetching body for {}: {}", url, e);
            return Ok(node);
        }
    };

    match page_type {
        PageType::Faq => {
            node.origin = extract_breadcrumbs(&html);
            node.title = extract_page_title(&html);
            let content = extract_faq_content(&body_html)?;
            node.faq_items = parse_faq_items(&content);
        }
        PageType::Menu => {
            let children_entries = extract_menu_page_items(PageType::Menu, &body_html)?;
            for child_entry in children_entries {
                if child_entry.url.is_empty() || visited.contains(&child_entry.url) {
                    continue;
                }
                visited.insert(child_entry.url.clone());
                println!("  {}", child_entry.url);
                match walk_site(&child_entry, visited) {
                    Ok(child_node) => node.children.push(child_node),
                    Err(e) => eprintln!("Error walking {}: {}", child_entry.url, e),
                }
            }
        }
        PageType::Geral => {}
    }

    Ok(node)
}

pub fn save_content(filename: &Path, html: &str) -> crate::types::Result<()> {
    std::fs::create_dir_all("./faq_data")?;
    std::fs::write(filename, html)?;
    Ok(())
}

pub fn save_content_to_json(filename: &Path, items: &[FaqItem]) -> crate::types::Result<()> {
    std::fs::create_dir_all("./faq_data")?;
    let json = serde_json::to_string_pretty(items)?;
    std::fs::write(filename, json)?;
    Ok(())
}

pub fn extract_faq_content(html: &str) -> Result<String> {
    let mut content: String = String::new();
    let mut start = 0;
    let mut counter = 1;
    while let Some(panel_start) = html[start..].find(r#"class="panel panel-default""#) {
        let panel_start = start + panel_start;

        let next_panel = html[panel_start + 30..]
            .find(r#"class="panel panel-default""#)
            .map(|p| panel_start + 30 + p)
            .unwrap_or(html.len());

        let panel_html = &html[panel_start..next_panel];

        let pergunta = extract_panel_title(panel_html);
        let resposta = extract_panel_body_text(panel_html);

        let item = format!(
            "// {}\nPergunta:\n{}\nResposta:\n{}",
            counter, pergunta, resposta
        );

        if !content.is_empty() {
            content.push_str("\n\n");
        }
        content.push_str(&item);

        start = next_panel;
        counter += 1;
    }

    Ok(content)
}
