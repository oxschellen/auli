use headless_chrome::{Browser, LaunchOptions};
use regex::Regex;
use scraper::{Html, Selector};
use std::sync::LazyLock;
use std::thread::sleep;
use std::time::Duration;
use ureq::Agent;

use super::types::Servico;
use super::utils::{get_tipo_servicos, save_servicos_to_json};

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";
const ACCEPT: &str = "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8";
const ACCEPT_LANGUAGE: &str = "pt-BR,pt;q=0.9,en-US;q=0.8,en;q=0.7";

static LINK_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"<a[^>]*href=["'](https://[^"']+)["'][^>]*>([^<]*)</a>"#).unwrap()
});

pub fn extrair_descricoes_json(
    data_dir: &str,
    use_cache: bool,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    // Initialize the HTTP agent (ureq). Accept headers are set per request in fetch_html.
    let http_client: Agent = Agent::config_builder()
        .user_agent(USER_AGENT)
        .timeout_global(Some(Duration::from_secs(30)))
        .build()
        .into();

    // Initializa o Vetor de Tipos de Serviços
    let vec_tipo_servicos = get_tipo_servicos();

    // URLs de páginas de detalhe que falharam ao carregar; reportadas ao final do programa.
    let mut failed_urls: Vec<String> = Vec::new();

    // Para cada Tipo de Serviço extrai a lista de Serviços
    for tipo_servicos in &vec_tipo_servicos {
        let tipo_s = &tipo_servicos.tipo;
        let file_s = &tipo_servicos.filename;
        let url_s = &tipo_servicos.url;
        println!("Tipo: {}, File: {} Url: {}", tipo_s, file_s, url_s);

        let filename_json = format!("{}/{}.json", data_dir, file_s);

        let html_content = fetch_webpage(data_dir, url_s, use_cache)?;

        let servicos = parse_html_and_extract_cards(&html_content, tipo_s)?;

        let mut new_vec_servicos_com_descricao: Vec<Servico> = Vec::new();

        for service in &servicos {
            println!(" ");
            println!("------------------------------");
            println!("ID: {}", service.id);
            println!("Tipo: {}", service.tipo);
            println!("Classe: {}", service.classe);
            println!("Orgao: {}", service.orgao);
            println!("Link: {}", service.link);
            println!("Titulo: {}", service.titulo);
            println!();

            let cleaned_output = match fetch_cleaned_output_with_retry(
                data_dir,
                &http_client,
                &service.link,
                use_cache,
            ) {
                Ok(content) => content,
                Err(error) => {
                    eprintln!("Error loading content from {}: {}", service.link, error);
                    failed_urls.push(service.link.clone());
                    continue;
                }
            };

            let descricao = build_descricao(&cleaned_output, service);

            println!("--------------------------------------------------------------");
            println!("Descricao: {:?}\n", descricao);
            println!("--------------------------------------------------------------");
            println!("Cleaned Output:\n{}", cleaned_output);
            println!("--------------------------------------------------------------");

            new_vec_servicos_com_descricao.push(Servico {
                id: service.id,
                tipo: service.tipo.clone(),
                classe: service.classe.clone(),
                orgao: service.orgao.clone(),
                link: service.link.clone(),
                titulo: service.titulo.clone(),
                descricao,
            });

            // Save incrementally so progress is not lost on crash
            save_servicos_to_json(&new_vec_servicos_com_descricao, &filename_json)?;
        }
    }

    Ok(failed_urls)
}

fn fetch_webpage(
    data_dir: &str,
    url: &str,
    use_cache: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    if let Some(cached) = super::cache::read(data_dir, url) {
        println!("Cache hit (listagem): {}", url);
        return Ok(cached);
    }
    if use_cache {
        return Err(format!(
            "cache miss para a listagem {} (modo --usecache, sem rede)",
            url
        )
        .into());
    }

    println!("Fetching webpage from: {}", url);

    let browser = Browser::new(
        LaunchOptions::default_builder()
            .headless(true)
            .window_size(Some((1920, 1080)))
            .build()
            .map_err(|e| e.to_string())?,
    )?;

    let tab = browser.new_tab()?;

    println!("Navigating to: {}", url);
    tab.navigate_to(url)?;

    println!("Waiting for page to load...");
    tab.wait_until_navigated()?;

    println!("Waiting for network activity to settle...");
    tab.wait_for_element("body")?;

    // Wait for the JS-rendered service cards (the elements parse_html_and_extract_cards needs)
    // rather than sleeping a fixed amount: this returns as soon as the cards appear but tolerates
    // a slow render up to the timeout. If they never show, the page did not load properly — abort
    // with a descriptive error instead of caching/parsing an incomplete page.
    tab.wait_for_element_with_custom_timeout(".card", Duration::from_secs(15))
        .map_err(|e| {
            format!(
                "a página de serviços não carregou corretamente (nenhum card encontrado) em {}: {}",
                url, e
            )
        })?;

    println!("Extracting final HTML content...");
    let html_content = tab.get_content()?;

    println!(
        "Successfully fetched {} bytes (after JS execution)",
        html_content.len()
    );

    super::cache::write(data_dir, url, &html_content);

    Ok(html_content)
}

// Parses the stored HTML file and extracts card information into `Service` objects.
//  `tipo` is the category label (e.g. "Cidadãos") assigned to every service
//  extracted from this page. The card title is used as the `classe` field.
fn parse_html_and_extract_cards(
    html_content: &str,
    tipo: &str,
) -> Result<Vec<Servico>, Box<dyn std::error::Error>> {
    let document = Html::parse_document(html_content);

    let services_selector = Selector::parse(".services").unwrap();
    let card_selector = Selector::parse(".card").unwrap();
    let card_title_selector = Selector::parse(".card-title").unwrap();
    let card_label_selector = Selector::parse(".label").unwrap();
    let link_selector = Selector::parse("ul li a").unwrap();

    let mut services: Vec<Servico> = Vec::new();
    let mut global_id = 1;

    let services_count = document.select(&services_selector).count();
    println!("Found {} services containers", services_count);
    println!("Extracting service data from cards...");

    for (card_index, card_element) in document.select(&card_selector).enumerate() {
        let card_title =
            if let Some(title_element) = card_element.select(&card_title_selector).next() {
                title_element
                    .text()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .trim()
                    .to_string()
            } else {
                format!("Card #{}", card_index + 1)
            };

        let card_label =
            if let Some(label_element) = card_element.select(&card_label_selector).next() {
                label_element
                    .text()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .trim()
                    .to_string()
            } else {
                "N/A".to_string()
            };

        let mut link_count = 0;
        for link_element in card_element.select(&link_selector) {
            if let Some(href) = link_element.value().attr("href") {
                let link_text = link_element
                    .text()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .trim()
                    .to_string();

                services.push(Servico {
                    id: global_id,
                    tipo: tipo.to_string(),
                    classe: card_title.clone(),
                    orgao: card_label.clone(),
                    link: href.to_string(),
                    titulo: link_text,
                    descricao: String::new(),
                });
                global_id += 1;
                link_count += 1;
            }
        }

        println!("Processed card: '{}' ({} services)", card_title, link_count);
    }

    println!("\nTotal services extracted: {}", services.len());
    Ok(services)
}

fn fetch_html(
    data_dir: &str,
    client: &Agent,
    url: &str,
    use_cache: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    if let Some(cached) = super::cache::read(data_dir, url) {
        return Ok(cached);
    }
    if use_cache {
        return Err(format!("cache miss para {} (modo --usecache, sem rede)", url).into());
    }

    let max_attempts = 3;
    let mut retry_delay = Duration::from_millis(800);
    let mut last_error: Option<ureq::Error> = None;

    // ureq returns Err on non-2xx by default (http_status_as_error), so there's no error_for_status.
    for attempt in 1..=max_attempts {
        match client
            .get(url)
            .header("Accept", ACCEPT)
            .header("Accept-Language", ACCEPT_LANGUAGE)
            .call()
        {
            Ok(mut response) => match response.body_mut().read_to_string() {
                Ok(body) => {
                    super::cache::write(data_dir, url, &body);
                    return Ok(body);
                }
                Err(error) => last_error = Some(error),
            },
            Err(error) => last_error = Some(error),
        }

        if attempt < max_attempts {
            eprintln!(
                "Request failed for {} (attempt {}/{}). Retrying in {:?}...",
                url, attempt, max_attempts, retry_delay
            );
            sleep(retry_delay);
            retry_delay = retry_delay.saturating_mul(2);
        }
    }

    Err(last_error
        .expect("at least one request attempt should fail before returning error")
        .into())
}

fn fetch_cleaned_output_with_retry(
    data_dir: &str,
    client: &Agent,
    url: &str,
    use_cache: bool,
) -> Result<String, String> {
    // In --usecache mode a miss is permanent, so don't retry (and don't sleep between attempts).
    let max_attempts = if use_cache { 1 } else { 3 };
    let mut retry_delay = Duration::from_millis(1000);
    let mut last_error = String::new();

    for attempt in 1..=max_attempts {
        match fetch_html(data_dir, client, url, use_cache) {
            Ok(html_code) => match extract_selector_content(&html_code, "div.artigo__texto") {
                Ok(output_content) => {
                    let cleaned_output = clean_text(&output_content);
                    if !cleaned_output.trim().is_empty() {
                        return Ok(cleaned_output);
                    }
                    last_error = "empty extracted content".to_string();
                }
                Err(error) => {
                    last_error = format!("selector extraction failed: {}", error);
                }
            },
            Err(error) => {
                last_error = format!("http request failed: {}", error);
            }
        }

        if attempt < max_attempts {
            eprintln!(
                "Load failed for {} (attempt {}/{}): {}. Retrying in {:?}...",
                url, attempt, max_attempts, last_error, retry_delay
            );
            sleep(retry_delay);
            retry_delay = retry_delay.saturating_mul(2);
        }
    }

    Err(last_error)
}

fn clean_text(text: &str) -> String {
    text.lines()
        .map(|line| {
            line.replace('\t', " ")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        })
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn build_descricao(cleaned_output: &str, service: &Servico) -> String {
    let mut descricao = String::new();
    let mut flag_descricao = false;

    for line in cleaned_output.lines() {
        if line == "Descrição" {
            flag_descricao = true;
            descricao.push_str(service.tipo.as_str());
            descricao.push(' ');
            descricao.push('\n');
            descricao.push_str(service.classe.as_str());
            descricao.push(' ');
            descricao.push('\n');
            descricao.push_str(service.titulo.as_str());
            descricao.push(' ');
            descricao.push('\n');
            continue;
        }

        if line == "Público"
            || line == "Etapas para realização do serviço"
            || line == "Documentos Necessários"
            || line == "Prazo"
            || line == "Mecanismos de Comunicação"
            || line == "Legislação Aplicada"
            || line == "Perguntas Frequentes"
        {
            break;
        }

        if flag_descricao {
            descricao.push_str(line);
            descricao.push(' ');
            descricao.push('\n');
            continue;
        }
    }

    descricao
}

fn extract_and_replace_links(html: &str) -> String {
    LINK_REGEX
        .replace_all(html, |caps: &regex::Captures| {
            let url = &caps[1];
            let text = &caps[2];

            if text.trim().is_empty() {
                format!("\"{}\"", url)
            } else {
                format!("{} \"{}\"", text.trim(), url)
            }
        })
        .to_string()
}

fn extract_selector_content(
    html: &str,
    selector: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let document = Html::parse_document(html);
    let css_selector = Selector::parse(selector)
        .map_err(|e| format!("Invalid CSS selector '{}': {:?}", selector, e))?;

    let mut result = String::new();

    for element in document.select(&css_selector) {
        let html_with_links = extract_and_replace_links(&element.html());
        let temp_doc = Html::parse_fragment(&html_with_links);
        let text = temp_doc.root_element().text().collect::<Vec<_>>().join(" ");
        if !text.trim().is_empty() {
            result.push_str(&clean_text(text.trim()));
            result.push('\n');
        }
    }

    if result.is_empty() {
        return Err(format!("No elements found for selector: {}", selector).into());
    }

    Ok(result)
}
