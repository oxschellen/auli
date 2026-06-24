use std::path::Path; // PathBuf};

use crate::types::{Entry, PageType, Result};
use crate::utils::{extract_attribute, read_html, save_html}; //, url_to_filename};

use crate::html_utils::{
    extract_question_href_from_class, extract_question_title_from_class, format_html,
};

use reqwest::blocking::Client;

pub const USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64; rv:124.0) Gecko/20100101 Firefox/124.0";
pub const BASE_URL: &str = "https://atendimento.receita.rs.gov.br";

fn build_client() -> Result<Client> {
    use reqwest::header::{ACCEPT, ACCEPT_LANGUAGE, HeaderMap, HeaderValue};
    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"));
    headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("pt-BR,pt;q=0.9,en-US;q=0.8,en;q=0.7"));
    Ok(Client::builder()
        .user_agent(USER_AGENT)
        .default_headers(headers)
        .build()?)
}

pub fn get_web_page_html(url: &str, path: &Path) -> Result<String> {
    let html = if path.exists() {
        read_html(path)?
    } else {
        // println!("Fazendo chamada html: {:?}", url);
        let html = build_client()?.get(url).send()?.text()?;

        let html = format_html(&html);
        save_html(path, &html)?;

        html
    };

    Ok(html)
}

pub fn get_web_page_ajax_body_html(url: &str, path: &Path, ajax_url: &str) -> Result<String> {
    let body_html = if path.exists() {
        read_html(path)?
    } else {
        // println!("Fazendo chamada body_html: {:?}", url);
        let response = build_client()?
            .get(ajax_url)
            .header("X-Requested-With", "XMLHttpRequest")
            .header("Referer", url)
            .send()
            .map_err(|e| format!("AJAX request failed for {}: {}", url, e))?
            .json::<serde_json::Value>()
            .map_err(|e| format!("AJAX parse failed for {}: {}", url, e))?;

        let body_html = response["body"]
            .as_str()
            .ok_or_else(|| format!("Missing 'body' field in AJAX response for {}", url))?
            .to_string();

        let body_html = format_html(&body_html);
        save_html(path, &body_html)?;

        body_html
    };

    Ok(body_html)
}

// pub fn extract_page_urls(entry: Entry) -> Result<Vec<Entry>> {
//     let url = entry.url;
//     let filename = url_to_filename(&url);

//     let path_html = PathBuf::from(format!("./data/{}.html", filename));
//     let path_body = PathBuf::from(format!("./data/{}_body.html", filename));

//     let html = get_web_page_html(&url, &path_html)?;
//     let (source_uri, page_type) = get_page_info(&html)?;

//     println!("Page info: {:?} - {:?} - {:?}", page_type, url, source_uri);

//     let ajax_url = format!("{}{}&currentPage=1&pageSize=100", BASE_URL, source_uri);
//     let body_html = get_web_page_ajax_body_html(&url, &path_body, &ajax_url)?;

//     if page_type == PageType::Menu || page_type == PageType::Faq {
//         return extract_menu_page_items(page_type, &body_html);
//     }

//     Ok(Vec::new())
// }

pub fn get_page_info(html: &str) -> Result<(String, PageType)> {
    // templatename=pagina.listapagina.sanfona.comnavegacao       → Faq
    // templatename=pagina.listapagina.categoriafaq[.comnavegacao] → Menu

    // data-matriz-source-uri="/_service/conteudo/pagedlistfilho?id=2573&templatename=pagina.listapagina.sanfona.comnavegacao"
    let source_uri = extract_attribute(&html, "data-matriz-source-uri").unwrap_or_default();

    let page_type = if source_uri.contains("pagina.listapagina.sanfona.comnavegacao") {
        PageType::Faq
    } else if source_uri.contains("pagina.listapagina.categoriafaq") {
        PageType::Menu
    } else {
        PageType::Geral
    };

    Ok((source_uri, page_type))
}

pub fn extract_menu_page_items(page_type: PageType, html: &str) -> Result<Vec<Entry>> {
    let mut entries: Vec<Entry> = Vec::new();
    let mut start = 0;

    while let Some(panel_start) = html[start..].find(r#"class="panel panel-default""#) {
        let panel_start = start + panel_start;

        let next_panel = html[panel_start + 30..]
            .find(r#"class="panel panel-default""#)
            .map(|p| panel_start + 30 + p)
            .unwrap_or(html.len());

        let panel_html = &html[panel_start..next_panel];

        let title = extract_question_title_from_class(panel_html, r#"class="panel-title""#);
        let href = extract_question_href_from_class(panel_html, r#"class="panel-heading""#);

        let mut url = String::new();
        if page_type == PageType::Menu {
            url = format!("{}{}", BASE_URL, href);
        }

        let mut description = String::new();
        if page_type == PageType::Menu {
            description = format!("Description Menu");
        } else if page_type == PageType::Faq {
            description = format!("Description Faq");
        }

        let new_entry = Entry::new(title, url, page_type.clone(), description);

        println!("new_entry: {:?}", new_entry);

        entries.push(new_entry);
        start = next_panel;
    }

    Ok(entries)
}
