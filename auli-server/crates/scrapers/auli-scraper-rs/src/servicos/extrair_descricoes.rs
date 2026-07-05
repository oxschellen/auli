use regex::Regex;
use scraper::{Html, Selector};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::LazyLock;
use std::thread::sleep;
use std::time::Duration;
use ureq::Agent;

use super::types::{Servico, TipoServicos};
use super::utils::{get_tipo_servicos, save_servicos_to_json, scrape_recovery_path};
use auli_scraper_kit::http::GetOpts;

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
    let http_client = auli_scraper_kit::build_agent(auli_scraper_kit::USER_AGENT, Some(Duration::from_secs(30)));

    // Initializa o Vetor de Tipos de Serviços
    let vec_tipo_servicos = get_tipo_servicos();

    // URLs de páginas de detalhe que falharam ao carregar; reportadas ao final do programa.
    let mut failed_urls: Vec<String> = Vec::new();

    // Subdiretório próprio da recuperação incremental (não colide com os per-público do `process`).
    std::fs::create_dir_all(format!("{}/scrape", data_dir))?;

    // Para cada Tipo de Serviço extrai a lista de Serviços
    for tipo_servicos in &vec_tipo_servicos {
        let tipo_s = &tipo_servicos.tipo;
        let file_s = &tipo_servicos.filename;
        let url_s = &tipo_servicos.url;
        println!("Tipo: {}, File: {} Url: {}", tipo_s, file_s, url_s);

        let filename_json = scrape_recovery_path(data_dir, file_s);

        let servicos =
            extrair_servicos_da_api(data_dir, &http_client, tipo_servicos, use_cache)?;

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

/// Base do endpoint interno do CMS Matriz (PROCERGS) que devolve os serviços de cada público em
/// JSON. As 5 listagens do RS são montadas NO CLIENTE por `capaservicos.js` a partir dele — não há
/// barreira de JS de fato, então o ureq basta e dispensa navegador headless.
const SERVICOS_API_BASE: &str = "https://www.fazenda.rs.gov.br/_service/tudofacil/capaservicos";

/// O `parent` (ids separados por vírgula) que a API espera vem do atributo `data-servico-parent` do
/// HTML-esqueleto, que já vem server-rendered (visível sem executar JS).
static PARENT_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"data-servico-parent="([^"]+)""#).unwrap());

/// Um serviço no JSON da API. Campos que não usamos (`id`, `link`) são ignorados na desserialização.
#[derive(Deserialize)]
struct ApiServico {
    href: String,
    text: String,
    #[serde(rename = "siglaOrgao", default)]
    sigla_orgao: String,
}

/// Uma página da API: `categoria -> serviços`, mais o flag de paginação.
#[derive(Deserialize)]
struct ApiPage {
    resultados: HashMap<String, Vec<ApiServico>>,
    #[serde(rename = "mais-resultados", default)]
    mais_resultados: bool,
}

/// Deriva o rótulo curto do órgão exibido no card, replicando `getOrgao(siglaOrgao)` de
/// capaservicos.js (substring case-insensitive, nesta ordem de prioridade). É fiel ao JS — inclusive
/// no `"te"` genérico do fallback — porque o objetivo é reproduzir exatamente o card renderizado.
fn orgao_do_card(sigla: &str) -> &'static str {
    let s = sigla.to_lowercase();
    let contem = |palavras: &[&str]| palavras.iter().any(|p| s.contains(p));
    if contem(&["fazenda", "sefaz"]) {
        "FAZENDA"
    } else if contem(&["receita"]) {
        "RECEITA"
    } else if contem(&["cage"]) {
        "CAGE"
    } else if contem(&["tesouro", "te"]) {
        "TESOURO"
    } else {
        ""
    }
}

/// Monta os `Servico` de um público a partir das páginas da API do mesmo jeito que
/// `capaservicos.js` monta os cards: categorias em ordem alfabética com `"Outros"` por último, órgão
/// derivado do 1º serviço da categoria (aplicado a todos), link `<url>/<href>` e título aparado.
/// Produz a MESMA lista e ordem que o parser do DOM renderizado pelo Chrome produzia (mesmos `id`s).
fn montar_servicos(tipo: &TipoServicos, paginas: Vec<ApiPage>) -> Vec<Servico> {
    // Consolida por categoria preservando a ordem de chegada DENTRO da categoria (Vec).
    let mut categorias: Vec<String> = Vec::new();
    let mut por_categoria: HashMap<String, Vec<ApiServico>> = HashMap::new();
    for page in paginas {
        for (categoria, lista) in page.resultados {
            if !por_categoria.contains_key(&categoria) {
                categorias.push(categoria.clone());
            }
            por_categoria.entry(categoria).or_default().extend(lista);
        }
    }

    // Ordem de renderização do capaservicos.js: alfabética, com "Outros" sempre por último.
    categorias.sort();
    if let Some(pos) = categorias.iter().position(|c| c == "Outros") {
        let outros = categorias.remove(pos);
        categorias.push(outros);
    }

    let mut servicos = Vec::new();
    let mut id: usize = 1;
    for categoria in &categorias {
        let lista = &por_categoria[categoria];
        let Some(primeiro) = lista.first() else {
            continue;
        };
        let orgao = orgao_do_card(&primeiro.sigla_orgao).to_string();
        for s in lista {
            servicos.push(Servico {
                id,
                tipo: tipo.tipo.clone(),
                classe: categoria.clone(),
                orgao: orgao.clone(),
                // `${window.location.href}/${href}` no JS — reproduz o href exato do card do Chrome.
                link: format!("{}/{}", tipo.url, s.href),
                titulo: s.text.trim().to_string(),
                descricao: String::new(),
            });
            id += 1;
        }
    }
    servicos
}

/// Busca os serviços de um público direto do endpoint JSON (sem navegador): lê o `parent` do shell,
/// pagina a API e delega a montagem/ordenação a `montar_servicos`. O `fetch_html` cuida do
/// cache-first, do modo `--usecache` e das retentativas — igual às páginas de detalhe.
fn extrair_servicos_da_api(
    data_dir: &str,
    client: &Agent,
    tipo: &TipoServicos,
    use_cache: bool,
) -> Result<Vec<Servico>, Box<dyn std::error::Error>> {
    let shell = fetch_html(data_dir, client, &tipo.url, use_cache)?;
    let parent = PARENT_REGEX
        .captures(&shell)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .ok_or_else(|| format!("atributo data-servico-parent ausente em {}", tipo.url))?;

    let mut paginas = Vec::new();
    let mut pagina = 1;
    loop {
        let url = format!("{}?parent={}&page={}", SERVICOS_API_BASE, parent, pagina);
        let raw = fetch_html(data_dir, client, &url, use_cache)?;
        let page: ApiPage = serde_json::from_str(&raw)
            .map_err(|e| format!("JSON inválido da API de serviços em {}: {}", url, e))?;
        let mais = page.mais_resultados;
        paginas.push(page);
        if !mais {
            break;
        }
        pagina += 1;
    }

    Ok(montar_servicos(tipo, paginas))
}

fn fetch_html(
    data_dir: &str,
    client: &Agent,
    url: &str,
    use_cache: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    // Retry/backoff é o kit::http::get_string (headers Accept + Accept-Language via GetOpts);
    // bridge map_err p/ o Box<dyn Error> deste crate. Cache-write no wrapper.
    if let Some(cached) = auli_scraper_kit::cache::read_or_bail(data_dir, url, use_cache)
        .map_err(|e| e.to_string())?
    {
        return Ok(cached);
    }
    let body = auli_scraper_kit::http::get_string(
        client,
        url,
        &GetOpts {
            log_prefix: "RS",
            headers: &[("Accept", ACCEPT), ("Accept-Language", ACCEPT_LANGUAGE)],
            ..Default::default()
        },
    )
    .map_err(|e| e.to_string())?;
    auli_scraper_kit::cache::write(data_dir, url, &body);
    Ok(body)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orgao_do_card_replica_o_getorgao_do_js() {
        assert_eq!(orgao_do_card("SECRETARIA DA FAZENDA"), "FAZENDA");
        assert_eq!(orgao_do_card("SEFAZ/RS"), "FAZENDA");
        assert_eq!(orgao_do_card("RECEITA ESTADUAL  - ICMS, IPVA E ITDC"), "RECEITA");
        assert_eq!(orgao_do_card("CAGE - Contadoria e Auditoria-Geral"), "CAGE");
        assert_eq!(orgao_do_card("TESOURO DO ESTADO"), "TESOURO");
        // Fallback genérico "te" do JS (peculiar, mas replicado fielmente).
        assert_eq!(orgao_do_card("COMITE GESTOR"), "TESOURO");
        // Nada casa -> rótulo vazio (o card renderiza um `.label` vazio, que o pipeline lê como "").
        assert_eq!(orgao_do_card("PROCERGS"), "");
    }

    #[test]
    fn montar_servicos_ordena_e_mapeia_como_o_js() {
        let tipo = TipoServicos {
            tipo: "Empresas".into(),
            filename: "servicos-a-empresas".into(),
            url: "https://www.fazenda.rs.gov.br/servicos-a-empresas".into(),
        };
        // "Outros" declarado primeiro e categorias fora de ordem, de propósito.
        let json = r#"{
            "resultados": {
                "Outros": [
                    {"id":9,"href":"servicos?servico=9","text":"Zeta","siglaOrgao":"PROCERGS"}
                ],
                "Cadastro": [
                    {"id":1,"href":"servicos?servico=1","text":"  Alfa  ","siglaOrgao":"RECEITA ESTADUAL"},
                    {"id":2,"href":"servicos?servico=2","text":"Beta","siglaOrgao":"SEFAZ"}
                ],
                "Atendimento": [
                    {"id":3,"href":"servicos?servico=3","text":"Gama","siglaOrgao":"SECRETARIA DA FAZENDA"}
                ]
            },
            "mais-resultados": false
        }"#;
        let page: ApiPage = serde_json::from_str(json).unwrap();
        let servicos = montar_servicos(&tipo, vec![page]);

        // Ordem: categorias alfabéticas, "Outros" por último; ids sequenciais na ordem de render.
        let vistos: Vec<(&str, &str)> =
            servicos.iter().map(|s| (s.classe.as_str(), s.titulo.as_str())).collect();
        assert_eq!(
            vistos,
            vec![
                ("Atendimento", "Gama"),
                ("Cadastro", "Alfa"),
                ("Cadastro", "Beta"),
                ("Outros", "Zeta"),
            ]
        );
        assert_eq!(servicos.iter().map(|s| s.id).collect::<Vec<_>>(), vec![1, 2, 3, 4]);

        // Órgão vem do 1º serviço da categoria e vale para todos: Cadastro -> RECEITA (não SEFAZ).
        assert_eq!(servicos[1].orgao, "RECEITA");
        assert_eq!(servicos[2].orgao, "RECEITA");
        assert_eq!(servicos[0].orgao, "FAZENDA");
        assert_eq!(servicos[3].orgao, ""); // PROCERGS não casa

        // Link reconstruído e título aparado.
        assert_eq!(
            servicos[1].link,
            "https://www.fazenda.rs.gov.br/servicos-a-empresas/servicos?servico=1"
        );
        assert_eq!(servicos[1].titulo, "Alfa");
    }
}
