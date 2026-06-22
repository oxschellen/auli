// use std::collections::HashSet;
// use std::fs::File;
// use std::io::Write;

use std::path::Path;

use crate::types::FaqItem; //, Result};

pub fn extract_attribute(html: &str, attribute: &str) -> Option<String> {
    let needle = format!(r#" {}=""#, attribute);
    let start = html.find(&needle)? + needle.len();
    let end = html[start..].find('"')?;
    Some(html[start..start + end].to_string())
}

pub fn read_html(filename: &Path) -> crate::types::Result<String> {
    Ok(std::fs::read_to_string(filename)?)
}

pub fn save_html(filename: &Path, html: &str) -> crate::types::Result<()> {
    std::fs::create_dir_all("./data")?;
    std::fs::write(filename, html)?;
    Ok(())
}

pub fn url_to_filename(url: &str) -> String {
    url.chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' => c,
            _ => '_',
        })
        .collect()
}

// pub fn read_faq_urls(filename: &str) -> Vec<String> {
//     std::fs::read_to_string(filename)
//         .unwrap_or_default()
//         .lines()
//         .filter(|l| !l.is_empty())
//         .map(|l| l.to_string())
//         .collect()
// }

// pub fn write_faq_urls(visited: &HashSet<String>, filename: &str) -> Result<()> {
//     let mut file = File::create(filename)?;
//     for url in visited {
//         writeln!(file, "{}", url)?;
//     }
//     Ok(())
//}

pub fn parse_faq_items(text: &str) -> Vec<FaqItem> {
    text.split("\n\n// ")
        .filter_map(|block| {
            let block = block
                .trim_start_matches("// ")
                .trim_start_matches(|c: char| c.is_ascii_digit())
                .trim_start_matches('\n');
            let pq_marker = "Pergunta:\n";
            let rs_marker = "Resposta:\n";
            let pq_pos = block.find(pq_marker)? + pq_marker.len();
            let rs_pos = block.find(rs_marker)?;
            let pergunta = block[pq_pos..rs_pos].trim().to_string();
            let resposta = block[rs_pos + rs_marker.len()..].trim().to_string();
            if pergunta.is_empty() || resposta.is_empty() {
                return None;
            }
            Some(FaqItem { pergunta, resposta })
        })
        .collect()
}
