// SEF-MG servicos scraper — ServiceNow CSM Service Portal, via a page API JSON (sem headless
// browser).
//
// O portal (atendimento2.fazenda.mg.gov.br/csm) é um Service Portal do ServiceNow renderizado em
// Angular; o HTML é um shell vazio, mas a page API (`/api/now/sp/page`) devolve o JSON completo de
// cada página — widgets com dados — desde que a requisição leve o header `X-Portal` e o parâmetro
// `portal_id` (sem eles o server omite os dados dos widgets). Não precisa de cookie nem CSRF token.
//
// Fluxo:
//   1. página inicial do catálogo (`id=csm_catalogo_de_servicos`) -> widget
//      `sef_catalog_category_page` -> as categorias;
//   2. página de cada categoria (`id=sef_service_catalog&category=<sys_id>`) -> widget
//      `sef_service_catalog` -> os itens (sys_id, nome, tags de público);
//   3. página de cada item (`id=catalog_item_info&sys_id=<sys_id>`) -> widget
//      `edx_article_header` -> as seções do artigo KB (`kbContentData.data[]`), que viram o corpo
//      da descrição.
//
// Públicos: cada item traz `representative_type_value` (tags `pf,mei,pj,ie,prpf`); o próprio
// portal as agrupa nos filtros Cidadão=[pf], Empresas=[mei,pj,ie] e Produtor Rural=[prpf]
// (client script do widget) — reproduzimos esse agrupamento nos per-público.
//
// Cache: cada página JSON é cacheada pela URL lógica de navegação (`/csm?...`) via
// `auli_scraper_kit::cache`, então re-runs não re-batem no portal.

use std::collections::BTreeMap;
use std::sync::LazyLock;
use std::thread::sleep;
use std::time::Duration;

use regex::Regex;
use serde::Deserialize;
use serde_json::Value;
use ureq::Agent;

use auli_scraper_kit::Servico;

const BASE: &str = "https://atendimento2.fazenda.mg.gov.br";
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

/// sys_id do Service Portal "csm" (header `X-Portal` + param `portal_id`). Sem ele a page API
/// responde sem os dados dos widgets. Estável enquanto a SEF-MG não recriar o portal.
const PORTAL_ID: &str = "89275a53cb13020000f8d856634c9c51";

/// Pausa entre requisições de rede (educação com o portal; não se aplica a cache hits).
const FETCH_DELAY: Duration = Duration::from_millis(200);

/// Categoria meta do portal (página de contato, não é serviço) — fora da coleta.
const META_CATEGORY: &str = "Não Encontrou o Serviço Desejado?";

// Links nos HTMLs dos artigos viram o formato `texto "url"` (o mesmo dos demais scrapers), para o
// texto do RAG ler consistente.
static LINK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"<a[^>]*href=["'](https?://[^"']+)["'][^>]*>(.*?)</a>"#).unwrap()
});
// Fechamentos de bloco viram quebra de linha antes da extração de texto, para listas/parágrafos
// não colarem numa linha só.
static BLOCK_END_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)</(p|li|div|h[1-6]|tr)>|<br\s*/?>").unwrap());

/// Os 3 públicos do portal (filtro do widget), na ordem de exibição, cada um com seu slug de
/// arquivo per-público e as tags de `representative_type_value` que agrupa.
fn publicos() -> Vec<(&'static str, &'static str, Vec<&'static str>)> {
    vec![
        ("Cidadão", "servicos-ao-cidadao", vec!["pf"]),
        ("Empresas", "servicos-a-empresas", vec!["mei", "pj", "ie"]),
        ("Produtor Rural", "servicos-ao-produtor-rural", vec!["prpf"]),
    ]
}

// --- shapes dos dados dos widgets (só os campos usados) ---

#[derive(Deserialize)]
struct Category {
    sys_id: String,
    name: String,
}

// `description`/`representative_type_value` chegam como `null` em alguns itens — `Option` em vez
// de `#[serde(default)]`, que não cobre null explícito.
#[derive(Deserialize)]
struct CatalogItem {
    sys_id: String,
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    representative_type_value: Option<String>,
}

#[derive(Deserialize)]
struct Section {
    #[serde(default)]
    label: String,
    #[serde(default)]
    content: Option<String>,
}

/// Raspa todos os serviços do MG e devolve-os agrupados per-público (na ordem de exibição) mais o
/// `publicos_ordem`, prontos para o `aggregate_servicos` dobrar no snapshot.
type ScrapeResult = (auli_scraper_kit::PerPublicoServicos, Vec<auli_contract::Publico>);
pub fn scrape(data_dir: &str, use_cache: bool) -> Result<ScrapeResult, Box<dyn std::error::Error>> {
    let agent = auli_scraper_kit::build_agent(USER_AGENT, Some(Duration::from_secs(30)));

    // 1. Categorias, da página inicial do catálogo.
    let home = fetch_page(data_dir, &agent, "csm_catalogo_de_servicos", &[], use_cache)?;
    let categories: Vec<Category> =
        widget_data_field(&home, "sef_catalog_category_page", "categories")?;
    println!("MG: {} categorias no catálogo", categories.len());

    // 2. Itens por categoria. Um item pode aparecer em mais de uma categoria (cross-listing) —
    //    registramos cada ocorrência (classe = nome da categoria) e buscamos o detalhe uma vez só.
    let mut occurrences: Vec<(String, CatalogItem)> = Vec::new(); // (categoria, item)
    for cat in &categories {
        if cat.name == META_CATEGORY {
            println!("MG: pulando categoria meta '{}'", cat.name);
            continue;
        }
        let page = fetch_page(
            data_dir,
            &agent,
            "sef_service_catalog",
            &[("category", &cat.sys_id)],
            use_cache,
        )?;
        let items: Vec<CatalogItem> = match widget_data_field(&page, "sef_service_catalog", "items")
        {
            Ok(items) => items,
            Err(e) => {
                eprintln!("⚠️  MG: categoria '{}' sem itens legíveis: {}", cat.name, e);
                continue;
            }
        };
        println!("MG: {} — {} itens", cat.name, items.len());
        for item in items {
            occurrences.push((cat.name.clone(), item));
        }
    }

    // 3. Corpo (artigo KB) por item único; itens repetidos entre categorias reusam o corpo.
    let mut bodies: BTreeMap<String, String> = BTreeMap::new();
    let unique: Vec<&CatalogItem> = {
        let mut seen = std::collections::HashSet::new();
        occurrences.iter().map(|(_, i)| i).filter(|i| seen.insert(i.sys_id.clone())).collect()
    };
    println!("MG: {} itens únicos ({} ocorrências)", unique.len(), occurrences.len());
    for (n, item) in unique.iter().enumerate() {
        let body = match fetch_article_body(data_dir, &agent, &item.sys_id, use_cache) {
            Ok(body) if !body.trim().is_empty() => body,
            Ok(_) | Err(_) => {
                // Fallback: a descrição curta da listagem (melhor que perder o item).
                eprintln!(
                    "⚠️  MG: detalhe sem corpo para '{}' ({}) — usando a descrição da listagem.",
                    item.name, item.sys_id
                );
                html_to_text(item.description.as_deref().unwrap_or_default())
            }
        };
        bodies.insert(item.sys_id.clone(), body);
        if (n + 1) % 25 == 0 {
            println!("MG: {}/{} detalhes processados", n + 1, unique.len());
        }
    }

    // 4. Fan-out per-público pelas tags do item, no agrupamento do próprio portal.
    let pubs = publicos();
    let mut buckets: BTreeMap<usize, Vec<Servico>> = (0..pubs.len()).map(|i| (i, Vec::new())).collect();
    for (categoria, item) in &occurrences {
        let tags: Vec<&str> = item
            .representative_type_value
            .as_deref()
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .collect();
        let mut matched = false;
        for (idx, (nome, _, pub_tags)) in pubs.iter().enumerate() {
            if !tags.iter().any(|t| pub_tags.contains(t)) {
                continue;
            }
            matched = true;
            push_servico(&mut buckets, idx, nome, categoria, item, &bodies);
        }
        if !matched {
            // Sem tag mapeada (ou sem tag): visível só no filtro "Todos" do portal — entra em
            // todos os públicos para não sumir dos per-público.
            if !tags.is_empty() {
                eprintln!("⚠️  MG: tags não mapeadas {:?} em '{}'", tags, item.name);
            }
            for (idx, (nome, ..)) in pubs.iter().enumerate() {
                push_servico(&mut buckets, idx, nome, categoria, item, &bodies);
            }
        }
    }

    // 5. Buckets na ordem de exibição + publicos_ordem para o snapshot.
    let mut inputs = Vec::new();
    let mut publicos_ordem = Vec::new();
    for (idx, (nome, slug, _)) in pubs.iter().enumerate() {
        publicos_ordem.push(auli_contract::Publico { nome: nome.to_string(), slug: slug.to_string() });
        inputs.push((nome.to_string(), buckets.remove(&idx).unwrap_or_default()));
    }
    Ok((inputs, publicos_ordem))
}

/// Monta o `Servico` de uma ocorrência (público × categoria) e o empurra no bucket do público.
/// A descrição leva o header de 3 linhas `tipo/classe/titulo` que o `aggregate_servicos` remove.
fn push_servico(
    buckets: &mut BTreeMap<usize, Vec<Servico>>,
    idx: usize,
    publico: &str,
    categoria: &str,
    item: &CatalogItem,
    bodies: &BTreeMap<String, String>,
) {
    let titulo = item.name.trim().to_string();
    let body = bodies.get(&item.sys_id).map(String::as_str).unwrap_or_default();
    let descricao = format!("{}\n{}\n{}\n{}", publico, categoria, titulo, body);
    buckets.entry(idx).or_default().push(Servico {
        id: 0, // renumerado per-arquivo pelo process
        tipo: publico.to_string(),
        classe: categoria.to_string(),
        orgao: "SEF/MG".to_string(),
        link: format!("{}/csm?id=catalog_item_info&sys_id={}", BASE, item.sys_id),
        titulo,
        descricao,
    });
}

/// Busca a página de um item e materializa o corpo do artigo KB: as seções `kbContentData.data[]`
/// do widget `edx_article_header`, como `label:\ntexto`, na ordem do artigo.
fn fetch_article_body(
    data_dir: &str,
    agent: &Agent,
    sys_id: &str,
    use_cache: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    let page = fetch_page(data_dir, agent, "catalog_item_info", &[("sys_id", sys_id)], use_cache)?;
    let data = widget_data(&page, "edx_article_header")
        .ok_or("widget edx_article_header ausente na página do item")?;
    let sections: Vec<Section> =
        serde_json::from_value(data["kbContentData"]["data"].clone())?;

    let mut body = String::new();
    for sec in &sections {
        let raw = sec.content.as_deref().unwrap_or_default();
        let text = if raw.contains('<') { html_to_text(raw) } else { raw.trim().to_string() };
        if text.is_empty() {
            continue;
        }
        let label = sec.label.trim();
        if !label.is_empty() {
            body.push_str(label);
            body.push_str(":\n");
        }
        body.push_str(&text);
        body.push_str("\n\n");
    }
    Ok(body.trim_end().to_string())
}

// --- page API ---

/// Busca (ou lê do cache) o JSON da page API para a página `id` com os `params` extras. A chave de
/// cache é a URL lógica de navegação (`/csm?id=...&...`), estável a mudanças da API.
fn fetch_page(
    data_dir: &str,
    agent: &Agent,
    page_id: &str,
    params: &[(&str, &str)],
    use_cache: bool,
) -> Result<Value, Box<dyn std::error::Error>> {
    let query: String =
        params.iter().map(|(k, v)| format!("&{}={}", k, v)).collect();
    let logical = format!("{}/csm?id={}{}", BASE, page_id, query);
    let api_url = format!(
        "{}/api/now/sp/page?id={}{}&portal_id={}",
        BASE, page_id, query, PORTAL_ID
    );

    if let Some(cached) = auli_scraper_kit::cache::read(data_dir, &logical) {
        return Ok(serde_json::from_str(&cached)?);
    }
    if use_cache {
        return Err(format!("cache miss para {} (modo --usecache, sem rede)", logical).into());
    }

    let max_attempts = 3;
    let mut delay = Duration::from_millis(800);
    let mut last_error = String::new();

    for attempt in 1..=max_attempts {
        match agent
            .get(&api_url)
            .header("Accept", "application/json")
            .header("X-Portal", PORTAL_ID)
            .header("X-Requested-With", "XMLHttpRequest")
            .call()
        {
            Ok(mut resp) => match resp.body_mut().read_to_string() {
                Ok(text) if !text.trim().is_empty() => {
                    // Só cacheia o que parseia: um JSON truncado/estranho não pode envenenar o cache.
                    let value: Value = serde_json::from_str(&text)?;
                    auli_scraper_kit::cache::write(data_dir, &logical, &text);
                    sleep(FETCH_DELAY);
                    return Ok(value);
                }
                Ok(_) => last_error = "resposta vazia".to_string(),
                Err(e) => last_error = e.to_string(),
            },
            Err(e) => last_error = e.to_string(),
        }

        if attempt < max_attempts {
            eprintln!(
                "MG: requisição falhou para {} (tentativa {}/{}): {}. Retentando em {:?}...",
                api_url, attempt, max_attempts, last_error, delay
            );
            sleep(delay);
            delay = delay.saturating_mul(2);
        }
    }

    Err(format!("falha ao buscar {}: {}", api_url, last_error).into())
}

/// Acha, na árvore `containers/rows/columns/widgets` da page API, o primeiro widget com o `id`
/// dado e devolve o seu `data`.
fn widget_data<'a>(page: &'a Value, widget_id: &str) -> Option<&'a Value> {
    fn walk<'a>(v: &'a Value, widget_id: &str) -> Option<&'a Value> {
        match v {
            Value::Object(map) => {
                if let Some(w) = map.get("widget")
                    && w["id"].as_str() == Some(widget_id)
                    && w["data"].is_object()
                {
                    return Some(&w["data"]);
                }
                map.values().find_map(|v| walk(v, widget_id))
            }
            Value::Array(items) => items.iter().find_map(|v| walk(v, widget_id)),
            _ => None,
        }
    }
    walk(&page["result"], widget_id)
}

/// `widget_data` + desserialização de um campo do `data` (ex.: `categories`, `items`).
fn widget_data_field<T: serde::de::DeserializeOwned>(
    page: &Value,
    widget_id: &str,
    field: &str,
) -> Result<T, Box<dyn std::error::Error>> {
    let data = widget_data(page, widget_id)
        .ok_or_else(|| format!("widget '{}' ausente na página", widget_id))?;
    Ok(serde_json::from_value(data[field].clone())
        .map_err(|e| format!("campo '{}' do widget '{}': {}", field, widget_id, e))?)
}

// --- html -> texto ---

/// Extrai texto legível do HTML de uma seção do artigo: âncoras viram `texto "url"`, fins de
/// bloco viram quebra de linha, e o restante das tags cai fora (via parser HTML de verdade).
fn html_to_text(html: &str) -> String {
    let with_links = LINK_RE.replace_all(html, |caps: &regex::Captures| {
        let url = &caps[1];
        let inner = strip_tags(&caps[2]);
        let text = inner.trim();
        if text.is_empty() { format!("\"{}\"", url) } else { format!("{} \"{}\"", text, url) }
    });
    let with_breaks = BLOCK_END_RE.replace_all(&with_links, "\n");
    clean_text(&strip_tags(&with_breaks))
}

/// Só o texto de um fragmento HTML (sem tags, entidades decodificadas).
fn strip_tags(fragment: &str) -> String {
    scraper::Html::parse_fragment(fragment).root_element().text().collect::<Vec<_>>().join("")
}

/// Normaliza espaços por linha e descarta linhas vazias.
fn clean_text(text: &str) -> String {
    text.lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_to_text_preserves_links_and_blocks() {
        let html = r#"<p>Emita pelo <a href="https://cdt.fazenda.mg.gov.br/">sistema CDT</a>.</p><ul><li>Item um</li><li>Item dois</li></ul>"#;
        let text = html_to_text(html);
        assert_eq!(text, "Emita pelo sistema CDT \"https://cdt.fazenda.mg.gov.br/\".\nItem um\nItem dois");
    }

    #[test]
    fn html_to_text_handles_bare_anchor_and_entities() {
        let html = r#"<p>Consulte: <a href="https://x.mg.gov.br"><b></b></a> — d&eacute;bitos</p>"#;
        let text = html_to_text(html);
        assert_eq!(text, "Consulte: \"https://x.mg.gov.br\" — débitos");
    }

    #[test]
    fn widget_data_finds_nested_widget() {
        let page: Value = serde_json::json!({
            "result": { "containers": [ { "rows": [ { "columns": [ { "widgets": [
                { "widget": { "id": "outro", "data": {"x": 1} } },
                { "widget": { "id": "sef_service_catalog", "data": {"items": [{"sys_id": "a", "name": "N"}]} } }
            ] } ] } ] } ] }
        });
        let items: Vec<CatalogItem> =
            widget_data_field(&page, "sef_service_catalog", "items").unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].sys_id, "a");
    }
}
