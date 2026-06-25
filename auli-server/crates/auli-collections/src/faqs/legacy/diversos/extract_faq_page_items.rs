use crate::html_utils::remove_html_tags;
use crate::types::{Entry, PageType};

pub fn extract_faq_page_items(html: &str) -> Result<Vec<Entry>, Box<dyn std::error::Error>> {
    let mut vec_urls: Vec<Entry> = Vec::new();
    let mut start = 0;

    while let Some(panel_start) = html[start..].find(r#"class="panel panel-default""#) {
        let panel_start = start + panel_start;

        // Find the end of this panel (next panel or end of section)
        let next_panel = html[panel_start + 30..]
            .find(r#"class="panel panel-default""#)
            .map(|p| panel_start + 30 + p)
            .unwrap_or(html.len());

        let panel_html = &html[panel_start..next_panel];

        // Extract text from panel-title
        let question_title =
            extract_question_title_from_class(panel_html, r#"class="panel-title""#);

        // Extract href from panel-heading anchor
        let question_href =
            extract_question_href_from_class(panel_html, r#"class="panel-heading""#);

        let menu_item_url = format!("https://atendimento.receita.rs.gov.br{}", question_href);

        let url = menu_item_url;
        let url_entry = Entry::new(question_title.clone(), url, PageType::new("Faq"));

        vec_urls.push(url_entry);

        // println!("{} - {}", id, question_title);
        // println!("{}", menu_item_url);
        // println!(" ");

        start = next_panel;
    }

    Ok(vec_urls)
}

fn extract_question_title_from_class(html_chunk: &str, class_attr: &str) -> String {
    if let Some(class_pos) = html_chunk.find(class_attr) {
        // Find the opening > after the class
        if let Some(open_tag_end) = html_chunk[class_pos..].find('>') {
            let content_start = class_pos + open_tag_end + 1;

            // Find the matching closing tag (simple heuristic: next </div> after this point)
            if let Some(close_pos) = html_chunk[content_start..].find("</div>") {
                let content = &html_chunk[content_start..content_start + close_pos];

                // Remove all HTML tags and clean whitespace
                let clean_text = remove_html_tags(content);
                return clean_text;
            }
        }
    }
    String::new()
}

fn extract_question_href_from_class(html_chunk: &str, class_attr: &str) -> String {
    if let Some(class_pos) = html_chunk.find(class_attr) {
        if let Some(a_pos) = html_chunk[class_pos..].find("<a ") {
            let a_start = class_pos + a_pos;
            if let Some(href_pos) = html_chunk[a_start..].find("href=\"") {
                let href_value_start = a_start + href_pos + 6; // skip past href="
                if let Some(href_end) = html_chunk[href_value_start..].find('"') {
                    return html_chunk[href_value_start..href_value_start + href_end].to_string();
                }
            }
        }
    }
    String::new()
}
