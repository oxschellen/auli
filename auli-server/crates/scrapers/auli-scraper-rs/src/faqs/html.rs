// Low-level HTML → text helpers for the faqs scraper.
//
// The portal markup is hand-parsed (no DOM crate) because we only need a few targeted pieces:
// an attribute value, a breadcrumb, the og:title, and the question/answer text inside Bootstrap
// `panel` blocks. `format_html` pretty-prints raw HTML before it is cached on disk.

/// Reads the value of `attribute` from the first ` name="value"` occurrence in `html`.
pub fn extract_attribute(html: &str, attribute: &str) -> Option<String> {
    let needle = format!(r#" {}=""#, attribute);
    let start = html.find(&needle)? + needle.len();
    let end = html[start..].find('"')?;
    Some(html[start..start + end].to_string())
}

/// Returns the breadcrumb trail from `<ol class="breadcrumb">` as items joined by ` | `.
pub fn extract_breadcrumbs(html: &str) -> String {
    let inner = extract_element_inner_html(html, "breadcrumb", "ol");
    inner
        .split("<li")
        .skip(1)
        .filter_map(|chunk| {
            let content_start = chunk.find('>')? + 1;
            let content_end = chunk.find("</li>").unwrap_or(chunk.len());
            let text = remove_html_tags(&chunk[content_start..content_end]);
            let trimmed = text.split_whitespace().collect::<Vec<_>>().join(" ");
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

/// Extracts the page title from `<meta property="og:title" content="...">`.
/// Returns an empty string if the tag is not found.
pub fn extract_page_title(html: &str) -> String {
    let marker = r#"property="og:title""#;
    let tag_start = match html.find(marker) {
        Some(p) => p,
        None => return String::new(),
    };
    let tag_end = match html[tag_start..].find('>') {
        Some(p) => tag_start + p,
        None => return String::new(),
    };
    let tag = &html[tag_start..tag_end];

    let content_key = r#"content=""#;
    let val_start = match tag.find(content_key) {
        Some(p) => p + content_key.len(),
        None => return String::new(),
    };
    match tag[val_start..].find('"') {
        Some(val_end) => tag[val_start..val_start + val_end].to_string(),
        None => String::new(),
    }
}

/// Extracts the plain-text question from the `panel-title` h4.
pub fn extract_panel_title(panel_html: &str) -> String {
    let inner = extract_element_inner_html(panel_html, "panel-title", "h4");
    remove_html_tags(inner)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Extracts the answer text from `panel-body`, preserving links as `[text](url)`.
pub fn extract_panel_body_text(panel_html: &str) -> String {
    let inner = extract_element_inner_html(panel_html, "panel-body", "div");
    html_to_text_with_links(inner)
}

/// Title text of a menu item, read from the element matching `class_attr` up to its `</div>`.
pub fn extract_question_title_from_class(html_chunk: &str, class_attr: &str) -> String {
    if let Some(class_pos) = html_chunk.find(class_attr)
        && let Some(open_tag_end) = html_chunk[class_pos..].find('>')
    {
        let content_start = class_pos + open_tag_end + 1;
        if let Some(close_pos) = html_chunk[content_start..].find("</div>") {
            return remove_html_tags(&html_chunk[content_start..content_start + close_pos]);
        }
    }
    String::new()
}

/// `href` of the first `<a>` inside the element matching `class_attr`.
pub fn extract_question_href_from_class(html_chunk: &str, class_attr: &str) -> String {
    if let Some(class_pos) = html_chunk.find(class_attr)
        && let Some(a_pos) = html_chunk[class_pos..].find("<a ")
    {
        let a_start = class_pos + a_pos;
        if let Some(href_pos) = html_chunk[a_start..].find("href=\"") {
            let href_value_start = a_start + href_pos + 6;
            if let Some(href_end) = html_chunk[href_value_start..].find('"') {
                return html_chunk[href_value_start..href_value_start + href_end].to_string();
            }
        }
    }
    String::new()
}

/// Finds the element whose opening tag contains `class="<class_attr>"`, then walks forward
/// counting open/close `tag_name` tags to locate the matching closing tag, returning the inner HTML.
fn extract_element_inner_html<'a>(html: &'a str, class_attr: &str, tag_name: &str) -> &'a str {
    let needle = format!(r#"class="{}""#, class_attr);
    let class_pos = match html.find(&needle) {
        Some(p) => p,
        None => return "",
    };
    let content_start = match html[class_pos..].find('>') {
        Some(p) => class_pos + p + 1,
        None => return "",
    };

    let open = format!("<{}", tag_name);
    let close = format!("</{}>", tag_name);
    let mut depth = 0i32;
    let mut pos = content_start;

    while pos < html.len() {
        if html[pos..].starts_with(&close) {
            if depth == 0 {
                return &html[content_start..pos];
            }
            depth -= 1;
            pos += close.len();
        } else if html[pos..].starts_with(&open) {
            depth += 1;
            pos += open.len();
        } else {
            pos += html[pos..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(1);
        }
    }

    &html[content_start..]
}

/// Converts an HTML fragment to plain text.
/// `<a href="...">text</a>` becomes `[text](url)`.
/// Block-level tags (`<p>`, `<li>`, `<br>`, `<ul>`, `<ol>`) insert newlines.
/// All other tags are stripped.
fn html_to_text_with_links(html: &str) -> String {
    let mut result = String::new();
    let mut pos = 0;

    while pos < html.len() {
        if html[pos..].starts_with('<') {
            let rest = &html[pos..];

            // Anchor tag — emit [text](href)
            if (rest.starts_with("<a ") || rest.starts_with("<a\n") || rest.starts_with("<a\t"))
                && let Some(tag_end) = rest.find('>')
            {
                let tag = &rest[..tag_end + 1];
                let content_start = pos + tag_end + 1;

                let href = tag.find("href=\"").and_then(|i| {
                    let start = i + 6;
                    tag[start..]
                        .find('"')
                        .map(|end| tag[start..start + end].to_string())
                });

                if let Some(close_offset) = html[content_start..].find("</a>") {
                    let inner_raw = &html[content_start..content_start + close_offset];
                    let inner = remove_html_tags(inner_raw)
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ");

                    if !inner.is_empty() {
                        match href {
                            Some(h) => result.push_str(&format!("[{}]({})", inner, h)),
                            None => result.push_str(&inner),
                        }
                    }

                    pos = content_start + close_offset + 4;
                    continue;
                }
            }

            // Other tags: strip, but insert newline for block elements
            if let Some(tag_end) = html[pos..].find('>') {
                let tag = &html[pos..pos + tag_end + 1];
                let t = tag.to_ascii_lowercase();
                let is_block = t.starts_with("<p")
                    || t.starts_with("</p")
                    || t.starts_with("<li")
                    || t.starts_with("</li")
                    || t.starts_with("<br")
                    || t.starts_with("<ul")
                    || t.starts_with("</ul")
                    || t.starts_with("<ol")
                    || t.starts_with("</ol");

                if is_block && !result.is_empty() && !result.ends_with('\n') {
                    result.push('\n');
                }
                pos += tag_end + 1;
                continue;
            }

            pos += 1;
        } else {
            let next = html[pos..].find('<').map(|p| pos + p).unwrap_or(html.len());
            let text = html[pos..next]
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            if !text.is_empty() {
                if !result.is_empty() && !result.ends_with('\n') && !result.ends_with(' ') {
                    result.push(' ');
                }
                result.push_str(&text);
            }
            pos = next;
        }
    }

    result
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Strips all HTML tags and decodes HTML entities, collapsing whitespace.
pub fn remove_html_tags(text: &str) -> String {
    let mut result = String::new();
    let mut inside_tag = false;

    for c in text.chars() {
        match c {
            '<' => inside_tag = true,
            '>' => inside_tag = false,
            _ if !inside_tag => result.push(c),
            _ => {}
        }
    }

    let decoded = decode_html_entities(&result);
    decoded.split_whitespace().collect::<Vec<&str>>().join(" ")
}

fn decode_html_entities(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;

    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        rest = &rest[amp..];

        if let Some(semi) = rest.find(';') {
            let entity = &rest[..semi + 1];
            let decoded = match entity {
                "&amp;" => Some("&"),
                "&lt;" => Some("<"),
                "&gt;" => Some(">"),
                "&quot;" => Some("\""),
                "&apos;" => Some("'"),
                "&nbsp;" => Some(" "),
                "&agrave;" => Some("à"),
                "&Agrave;" => Some("À"),
                "&aacute;" => Some("á"),
                "&Aacute;" => Some("Á"),
                "&acirc;" => Some("â"),
                "&Acirc;" => Some("Â"),
                "&atilde;" => Some("ã"),
                "&Atilde;" => Some("Ã"),
                "&auml;" => Some("ä"),
                "&Auml;" => Some("Ä"),
                "&egrave;" => Some("è"),
                "&Egrave;" => Some("È"),
                "&eacute;" => Some("é"),
                "&Eacute;" => Some("É"),
                "&ecirc;" => Some("ê"),
                "&Ecirc;" => Some("Ê"),
                "&euml;" => Some("ë"),
                "&Euml;" => Some("Ë"),
                "&igrave;" => Some("ì"),
                "&Igrave;" => Some("Ì"),
                "&iacute;" => Some("í"),
                "&Iacute;" => Some("Í"),
                "&icirc;" => Some("î"),
                "&Icirc;" => Some("Î"),
                "&iuml;" => Some("ï"),
                "&Iuml;" => Some("Ï"),
                "&ograve;" => Some("ò"),
                "&Ograve;" => Some("Ò"),
                "&oacute;" => Some("ó"),
                "&Oacute;" => Some("Ó"),
                "&ocirc;" => Some("ô"),
                "&Ocirc;" => Some("Ô"),
                "&otilde;" => Some("õ"),
                "&Otilde;" => Some("Õ"),
                "&ouml;" => Some("ö"),
                "&Ouml;" => Some("Ö"),
                "&ugrave;" => Some("ù"),
                "&Ugrave;" => Some("Ù"),
                "&uacute;" => Some("ú"),
                "&Uacute;" => Some("Ú"),
                "&ucirc;" => Some("û"),
                "&Ucirc;" => Some("Û"),
                "&uuml;" => Some("ü"),
                "&Uuml;" => Some("Ü"),
                "&ccedil;" => Some("ç"),
                "&Ccedil;" => Some("Ç"),
                "&ntilde;" => Some("ñ"),
                "&Ntilde;" => Some("Ñ"),
                _ => None,
            };

            if let Some(ch) = decoded {
                out.push_str(ch);
                rest = &rest[semi + 1..];
                continue;
            }

            // numeric entity: &#NNN; or &#xHH;
            if let Some(code_point) = rest[1..semi].strip_prefix('#').and_then(|s| {
                if let Some(hex) = s.strip_prefix('x').or_else(|| s.strip_prefix('X')) {
                    u32::from_str_radix(hex, 16).ok()
                } else {
                    s.parse::<u32>().ok()
                }
            }) && let Some(ch) = char::from_u32(code_point)
            {
                out.push(ch);
                rest = &rest[semi + 1..];
                continue;
            }

            // unknown entity — emit as-is
            out.push('&');
            rest = &rest[1..];
        } else {
            // no closing semicolon — emit as-is
            out.push('&');
            rest = &rest[1..];
        }
    }

    out.push_str(rest);
    out
}

/// Pretty-prints raw HTML with two-space indentation (used before caching pages to disk).
pub fn format_html(input: &str) -> String {
    let mut formatted = String::new();
    let mut indent_level = 0usize;
    let mut index = 0usize;
    let mut raw_text_tag: Option<String> = None;

    while index < input.len() {
        if let Some(tag_name) = raw_text_tag.clone() {
            if let Some(close_index) = find_closing_tag(&input[index..], &tag_name) {
                append_raw_text(
                    &mut formatted,
                    &input[index..index + close_index],
                    indent_level,
                );
                index += close_index;
                raw_text_tag = None;
                continue;
            }

            append_raw_text(&mut formatted, &input[index..], indent_level);
            break;
        }

        if input.as_bytes()[index] == b'<' {
            let tag_end = find_tag_end(input, index);
            let token = input[index..=tag_end].trim();

            if is_closing_tag(token) {
                indent_level = indent_level.saturating_sub(1);
                append_line(&mut formatted, indent_level, token);
            } else {
                append_line(&mut formatted, indent_level, token);

                if let Some(tag_name) = opening_tag_name(token) {
                    if is_raw_text_tag(&tag_name) {
                        indent_level += 1;
                        raw_text_tag = Some(tag_name);
                    } else if !is_self_closing_tag(token, &tag_name) {
                        indent_level += 1;
                    }
                }
            }

            index = tag_end + 1;
        } else {
            let next_tag = input[index..]
                .find('<')
                .map_or(input.len(), |offset| index + offset);
            append_text(&mut formatted, &input[index..next_tag], indent_level);
            index = next_tag;
        }
    }

    formatted.trim_end().to_string()
}

fn find_tag_end(input: &str, start: usize) -> usize {
    let mut in_single_quotes = false;
    let mut in_double_quotes = false;

    for (offset, character) in input[start + 1..].char_indices() {
        match character {
            '\'' if !in_double_quotes => in_single_quotes = !in_single_quotes,
            '"' if !in_single_quotes => in_double_quotes = !in_double_quotes,
            '>' if !in_single_quotes && !in_double_quotes => return start + offset + 1,
            _ => {}
        }
    }

    input.len().saturating_sub(1)
}

fn is_closing_tag(token: &str) -> bool {
    token.starts_with("</")
}

fn opening_tag_name(token: &str) -> Option<String> {
    let tag = token.strip_prefix('<')?;

    if tag.starts_with('/') || tag.starts_with('!') || tag.starts_with('?') {
        return None;
    }

    let name_end = tag
        .find(|c: char| c.is_whitespace() || matches!(c, '>' | '/'))
        .unwrap_or(tag.len());

    Some(tag[..name_end].to_ascii_lowercase())
}

fn is_self_closing_tag(token: &str, tag_name: &str) -> bool {
    token.ends_with("/>") || is_void_tag(tag_name)
}

fn is_void_tag(tag_name: &str) -> bool {
    matches!(
        tag_name,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

fn is_raw_text_tag(tag_name: &str) -> bool {
    matches!(tag_name, "pre" | "script" | "style" | "textarea")
}

fn find_closing_tag(input: &str, tag_name: &str) -> Option<usize> {
    // tag_name is already lowercased by opening_tag_name(), so direct search is correct
    input.find(&format!("</{tag_name}"))
}

fn append_text(output: &mut String, text: &str, indent_level: usize) {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if !normalized.is_empty() {
        append_line(output, indent_level, &normalized);
    }
}

fn append_raw_text(output: &mut String, text: &str, indent_level: usize) {
    for line in text.lines() {
        if !line.trim().is_empty() {
            append_line(output, indent_level, line.trim_end());
        }
    }
}

fn append_line(output: &mut String, indent_level: usize, content: &str) {
    output.push_str(&"  ".repeat(indent_level));
    output.push_str(content);
    output.push('\n');
}
