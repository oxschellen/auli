// SEF-SC servicos scraper — Next.js JSON API backend (no headless browser, no HTML parsing).
//
// SC's portal (www.sef.sc.gov.br) is a Next.js app: every page exposes its data as JSON under
// `/_next/data/<buildId>/<path>.json`. We:
//   1. read the current `buildId` from any page's embedded `__NEXT_DATA__`,
//   2. page through the service listing (`servicos/buscar.json?pagina=N`),
//   3. fetch each service's detail (`servicos/<slug>.json?slug=<slug>`) for the rich body,
//   4. map each service into the shared `Servico` model, once per audience (`publicos`), and
//   5. hand the per-público buckets to `aggregate_servicos` for the snapshot; the per-público
//      fan-out files are derived later by `auli-collections process`, not written here.
//
// Cache: pages are cached by a *logical* URL (without the buildId) so a SC deploy that changes the
// buildId doesn't invalidate the on-disk cache (`auli_scraper_kit::cache`).

use std::collections::BTreeMap;
use std::sync::LazyLock;
use std::thread::sleep;
use std::time::Duration;

use regex::Regex;
use serde::Deserialize;
use ureq::Agent;

use auli_scraper_kit::Servico;

const BASE: &str = "https://www.sef.sc.gov.br";
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

// SC embeds links as wiki-style `[[https://url anchor text]]` (and bare `[[https://url]]`) inside its
// text fields. Normalize them to the `anchor "url"` form RS uses, so the RAG text reads consistently.
static SC_LINK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[\[\s*(https?://\S+)(?:\s+([^\]]*?))?\s*\]\]").unwrap());

fn normalize_links(text: &str) -> String {
    SC_LINK_RE
        .replace_all(text, |caps: &regex::Captures| {
            let url = &caps[1];
            match caps.get(2).map(|m| m.as_str().trim()) {
                Some(anchor) if !anchor.is_empty() => format!("{} \"{}\"", anchor, url),
                _ => format!("\"{}\"", url),
            }
        })
        .into_owned()
}

/// The 5 SC audiences (públicos), in display order, each with its output filename. The `id` matches
/// the portal's público id; services are grouped into these files by their `publicos` list.
fn publicos() -> Vec<(i64, &'static str, &'static str)> {
    vec![
        (3, "Cidadão", "servicos-ao-cidadao"),
        (4, "Empresa", "servicos-a-empresas"),
        (5, "Servidor Público", "servicos-a-servidores-publicos"),
        (8, "Estudante", "servicos-a-estudantes"),
        (12, "Prefeitura", "servicos-a-prefeituras"),
    ]
}

// --- API response shapes (only the fields we use; everything else is ignored) ---

#[derive(Deserialize)]
struct NextData<T> {
    #[serde(rename = "pageProps")]
    page_props: PageProps<T>,
}

#[derive(Deserialize)]
struct PageProps<T> {
    #[serde(rename = "respostaApi")]
    resposta_api: T,
}

#[derive(Deserialize)]
struct ListingApi {
    #[serde(rename = "responseServicos")]
    response_servicos: ResponseServicos,
}

#[derive(Deserialize)]
struct ResponseServicos {
    itens: Vec<ListingItem>,
    #[serde(rename = "paginasTotais", default)]
    paginas_totais: StringOrNum,
}

#[derive(Deserialize)]
struct ListingItem {
    nome: String,
    slug: String,
    #[serde(default)]
    publicos: Vec<Publico>,
}

#[derive(Deserialize)]
struct Publico {
    id: i64,
}

#[derive(Deserialize)]
struct DetailApi {
    servico: DetailServico,
}

#[derive(Deserialize)]
struct DetailServico {
    #[serde(rename = "dadosJson")]
    dados_json: DadosJson,
}

#[derive(Deserialize, Default)]
struct DadosJson {
    #[serde(default)]
    finalidade: String,
    #[serde(rename = "etapasProcesso", default)]
    etapas_processo: Vec<String>,
    #[serde(rename = "requisitosExigidosus", default)]
    requisitos: Vec<String>,
    #[serde(rename = "grupoServico", default)]
    grupo_servico: Option<Named>,
    #[serde(default)]
    orgao: Option<Named>,
    #[serde(default)]
    publicos: Vec<Publico>,
}

#[derive(Deserialize)]
struct Named {
    #[serde(default)]
    nome: String,
}

/// Some SC fields come back as either a JSON number or a string; accept both.
#[derive(Default)]
struct StringOrNum(String);

impl<'de> Deserialize<'de> for StringOrNum {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = serde_json::Value::deserialize(d)?;
        Ok(StringOrNum(match v {
            serde_json::Value::String(s) => s,
            serde_json::Value::Number(n) => n.to_string(),
            _ => String::new(),
        }))
    }
}

/// Scrapes all SC services and returns them grouped per público (in display order) plus the
/// `publicos_ordem`, ready for `aggregate_servicos` to fold into the snapshot. SC no longer writes
/// per-público files during the scrape — the fan-out is now `process`'s job.
type ScrapeResult = (auli_scraper_kit::PerPublicoServicos, Vec<auli_contract::Publico>);
pub fn scrape(data_dir: &str, use_cache: bool) -> Result<ScrapeResult, Box<dyn std::error::Error>> {
    let agent = auli_scraper_kit::build_agent(USER_AGENT, Some(Duration::from_secs(30)));

    let build_id = discover_build_id(data_dir, &agent, use_cache)?;
    println!("SC buildId: {}", build_id);

    // 1. Collect every service from the paged listing.
    let items = fetch_all_listing(data_dir, &agent, &build_id, use_cache)?;
    println!("SC: {} serviços na listagem", items.len());

    // 2. Group services into the per-público buckets, fetching each detail page once.
    let pubs = publicos();
    let mut buckets: BTreeMap<i64, Vec<Servico>> = pubs.iter().map(|(id, ..)| (*id, Vec::new())).collect();

    for (i, item) in items.iter().enumerate() {
        let detail = match fetch_detail(data_dir, &agent, &build_id, &item.slug, use_cache) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("⚠️  SC: detalhe falhou para '{}': {}", item.slug, e);
                continue;
            }
        };
        let dados = detail.servico.dados_json;

        // Prefer the detail page's públicos; fall back to the listing's.
        let mut pub_ids: Vec<i64> = dados.publicos.iter().map(|p| p.id).collect();
        if pub_ids.is_empty() {
            pub_ids = item.publicos.iter().map(|p| p.id).collect();
        }

        let classe = dados
            .grupo_servico
            .as_ref()
            .map(|g| g.nome.clone())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "Outros".to_string());
        let orgao = dados
            .orgao
            .as_ref()
            .map(|o| o.nome.clone())
            .unwrap_or_default();
        let link = format!("{}/servicos/{}", BASE, item.slug);

        // Add the service to each of its audiences' buckets (id is assigned per file below).
        for (pub_id, pub_nome, _) in &pubs {
            if !pub_ids.contains(pub_id) {
                continue;
            }
            let descricao = build_descricao(pub_nome, &classe, &item.nome, &dados);
            buckets.entry(*pub_id).or_default().push(Servico {
                id: 0, // re-numbered per file after grouping
                tipo: pub_nome.to_string(),
                classe: classe.clone(),
                orgao: orgao.clone(),
                link: link.clone(),
                titulo: item.nome.clone(),
                descricao,
            });
        }

        if (i + 1) % 25 == 0 {
            println!("SC: {}/{} detalhes processados", i + 1, items.len());
        }
    }

    // 3. Emit the per-público buckets in display order, plus `publicos_ordem`, for aggregation into
    //    the snapshot. Ids stay 0 here — `aggregate_servicos` positions by link and `process`
    //    re-numbers the per-público output.
    let mut inputs = Vec::new();
    let mut publicos_ordem = Vec::new();
    for (pub_id, pub_nome, filename) in &pubs {
        let services = buckets.remove(pub_id).unwrap_or_default();
        publicos_ordem.push(auli_contract::Publico {
            nome: pub_nome.to_string(),
            slug: filename.to_string(),
        });
        inputs.push((pub_nome.to_string(), services));
    }

    Ok((inputs, publicos_ordem))
}

/// Builds the `descricao` with the 3-line `tipo / classe / titulo` header that
/// `auli_scraper_kit::descricao_body` strips (via `skip(3)`), followed by the service body
/// (finalidade + etapas + requisitos). Keep exactly three header lines.
fn build_descricao(tipo: &str, classe: &str, titulo: &str, d: &DadosJson) -> String {
    let mut out = String::new();
    out.push_str(tipo);
    out.push('\n');
    out.push_str(classe);
    out.push('\n');
    out.push_str(titulo);
    out.push('\n');

    let finalidade = normalize_links(d.finalidade.trim());
    if !finalidade.is_empty() {
        out.push_str(&finalidade);
        out.push('\n');
    }

    let etapas: Vec<String> = d
        .etapas_processo
        .iter()
        .map(|e| normalize_links(e.trim()))
        .filter(|e| !e.is_empty())
        .collect();
    if !etapas.is_empty() {
        out.push_str("\nEtapas para realização do serviço:\n");
        for e in etapas {
            out.push_str("- ");
            out.push_str(&e);
            out.push('\n');
        }
    }

    let reqs: Vec<String> = d
        .requisitos
        .iter()
        .map(|r| normalize_links(r.trim()))
        .filter(|r| !r.is_empty())
        .collect();
    if !reqs.is_empty() {
        out.push_str("\nRequisitos:\n");
        for r in reqs {
            out.push_str("- ");
            out.push_str(&r);
            out.push('\n');
        }
    }

    out
}

/// Reads the current Next.js `buildId` from the `__NEXT_DATA__` script embedded in any portal page.
fn discover_build_id(
    data_dir: &str,
    agent: &Agent,
    use_cache: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    let logical = format!("{}/servicos/buscar", BASE);
    let html = fetch_cached(data_dir, agent, &logical, &logical, use_cache)?;
    parse_build_id(&html)
}

/// Extracts the Next.js `buildId` from the `"buildId":"…"` marker in a portal page's HTML.
fn parse_build_id(html: &str) -> Result<String, Box<dyn std::error::Error>> {
    let marker = "\"buildId\":\"";
    let start = html
        .find(marker)
        .ok_or("não foi possível localizar buildId no HTML do portal SC")?
        + marker.len();
    let rest = &html[start..];
    let end = rest
        .find('"')
        .ok_or("buildId malformado no HTML do portal SC")?;
    Ok(rest[..end].to_string())
}

/// Pages through `servicos/buscar.json?pagina=N`, returning every listing item.
fn fetch_all_listing(
    data_dir: &str,
    agent: &Agent,
    build_id: &str,
    use_cache: bool,
) -> Result<Vec<ListingItem>, Box<dyn std::error::Error>> {
    let mut all = Vec::new();
    let mut page = 1;
    loop {
        let logical = format!("{}/servicos/buscar?pagina={}", BASE, page);
        let data_url = format!(
            "{}/_next/data/{}/servicos/buscar.json?pagina={}",
            BASE, build_id, page
        );
        let body = fetch_cached(data_dir, agent, &logical, &data_url, use_cache)?;
        let parsed: NextData<ListingApi> = serde_json::from_str(&body)?;
        let resp = parsed.page_props.resposta_api.response_servicos;

        let total_pages: usize = resp.paginas_totais.0.parse().unwrap_or(page);
        all.extend(resp.itens);

        if page >= total_pages {
            break;
        }
        page += 1;
    }
    Ok(all)
}

/// Fetches one service's detail JSON.
fn fetch_detail(
    data_dir: &str,
    agent: &Agent,
    build_id: &str,
    slug: &str,
    use_cache: bool,
) -> Result<DetailApi, Box<dyn std::error::Error>> {
    let logical = format!("{}/servicos/{}", BASE, slug);
    let data_url = format!(
        "{}/_next/data/{}/servicos/{}.json?slug={}",
        BASE, build_id, slug, slug
    );
    let body = fetch_cached(data_dir, agent, &logical, &data_url, use_cache)?;
    let parsed: NextData<DetailApi> = serde_json::from_str(&body)?;
    Ok(parsed.page_props.resposta_api)
}

/// Fetches `data_url`, caching the response under `logical_url` (so buildId changes don't bust the
/// cache). Retries transient failures up to 3× with exponential backoff. In `use_cache` mode a cache
/// miss is a hard error (no network).
fn fetch_cached(
    data_dir: &str,
    agent: &Agent,
    logical_url: &str,
    data_url: &str,
    use_cache: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    if let Some(cached) = auli_scraper_kit::cache::read(data_dir, logical_url) {
        return Ok(cached);
    }
    if use_cache {
        return Err(format!("cache miss para {} (modo --usecache, sem rede)", logical_url).into());
    }

    let max_attempts = 3;
    let mut delay = Duration::from_millis(800);
    let mut last_error = String::new();

    for attempt in 1..=max_attempts {
        match agent.get(data_url).call() {
            Ok(mut resp) => match resp.body_mut().read_to_string() {
                Ok(body) if !body.trim().is_empty() => {
                    auli_scraper_kit::cache::write(data_dir, logical_url, &body);
                    return Ok(body);
                }
                Ok(_) => last_error = "resposta vazia".to_string(),
                Err(e) => last_error = e.to_string(),
            },
            Err(e) => last_error = e.to_string(),
        }

        if attempt < max_attempts {
            eprintln!(
                "SC: requisição falhou para {} (tentativa {}/{}): {}. Retentando em {:?}...",
                data_url, attempt, max_attempts, last_error, delay
            );
            sleep(delay);
            delay = delay.saturating_mul(2);
        }
    }

    Err(format!("falha ao buscar {}: {}", data_url, last_error).into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_links_cobre_os_formatos() {
        // [[url âncora]] -> `âncora "url"`; [[url]] -> `"url"`; texto sem link fica igual.
        assert_eq!(
            normalize_links("[[https://x.sc.gov.br/a Clique aqui]]"),
            "Clique aqui \"https://x.sc.gov.br/a\""
        );
        assert_eq!(normalize_links("[[https://x.sc.gov.br/a]]"), "\"https://x.sc.gov.br/a\"");
        assert_eq!(normalize_links("sem link nenhum"), "sem link nenhum");
        // Amostra real do portal.
        assert_eq!(
            normalize_links("[[https://caf2.sef.sc.gov.br/Views/x.aspx Dúvidas? Acesse aqui.]]"),
            "Dúvidas? Acesse aqui. \"https://caf2.sef.sc.gov.br/Views/x.aspx\""
        );
    }

    #[test]
    fn parse_build_id_extrai_do_marker() {
        let html = r#"<script>{"props":{},"buildId":"LEoHxAKKyLD8ldaA4Q9Wm","isFallback":false}</script>"#;
        assert_eq!(parse_build_id(html).unwrap(), "LEoHxAKKyLD8ldaA4Q9Wm");
        assert!(parse_build_id("<html>sem buildId</html>").is_err());
    }

    #[test]
    fn string_or_num_aceita_string_e_numero() {
        // `paginasTotais` volta ora número, ora string — os dois devem virar a mesma String.
        let n: StringOrNum = serde_json::from_str("22").unwrap();
        assert_eq!(n.0, "22");
        let s: StringOrNum = serde_json::from_str("\"22\"").unwrap();
        assert_eq!(s.0, "22");
    }

    // Fixture fiel ao shape real da listagem Next.js (`_next/data/<buildId>/servicos/buscar.json`).
    const LISTING_JSON: &str = r#"{"pageProps":{"respostaApi":{"responseServicos":{
      "paginasTotais": 22,
      "itens":[
        {"id":1,"nome":"Acessar Anexos de Documentos de Fiscalização",
         "slug":"acessar-anexos-de-documentos-de-fiscalizacao",
         "publicos":[{"id":3,"nome":"Cidadão"},{"id":4,"nome":"Empresa"}]},
        {"id":2,"nome":"Outro Serviço","slug":"outro-servico","publicos":[]}
      ]}}}}"#;

    #[test]
    fn parse_listagem_le_itens_paginas_e_publicos() {
        let parsed: NextData<ListingApi> = serde_json::from_str(LISTING_JSON).unwrap();
        let rs = parsed.page_props.resposta_api.response_servicos;
        assert_eq!(rs.paginas_totais.0, "22");
        assert_eq!(rs.itens.len(), 2);
        assert_eq!(rs.itens[0].slug, "acessar-anexos-de-documentos-de-fiscalizacao");
        let ids: Vec<i64> = rs.itens[0].publicos.iter().map(|p| p.id).collect();
        assert_eq!(ids, vec![3, 4]);
        assert!(rs.itens[1].publicos.is_empty());
    }

    // Fixture fiel ao shape real do detalhe (`_next/data/<buildId>/servicos/<slug>.json`).
    const DETAIL_JSON: &str = r#"{"pageProps":{"respostaApi":{"servico":{"dadosJson":{
      "finalidade":"Garantir o contraditório.",
      "etapasProcesso":["Acessar o link;","Preencher o formulário."],
      "requisitosExigidosus":["Estar cadastrado"],
      "grupoServico":{"nome":"Contencioso Tributário","id":181},
      "orgao":{"nome":"Secretaria de Estado da Fazenda"},
      "publicos":[{"id":3,"nome":"Cidadão"}]
    }}}}}"#;

    #[test]
    fn parse_detalhe_le_dados_json() {
        let parsed: NextData<DetailApi> = serde_json::from_str(DETAIL_JSON).unwrap();
        let d = parsed.page_props.resposta_api.servico.dados_json;
        assert_eq!(d.finalidade, "Garantir o contraditório.");
        assert_eq!(d.etapas_processo.len(), 2);
        assert_eq!(d.requisitos, vec!["Estar cadastrado"]);
        assert_eq!(d.grupo_servico.as_ref().unwrap().nome, "Contencioso Tributário");
        assert_eq!(d.orgao.as_ref().unwrap().nome, "Secretaria de Estado da Fazenda");
        assert_eq!(d.publicos[0].id, 3);
    }

    #[test]
    fn build_descricao_tem_header_de_3_linhas_e_corpo() {
        let d = DadosJson {
            finalidade: "Fim do serviço.".to_string(),
            etapas_processo: vec!["Passo 1".to_string(), "".to_string()],
            requisitos: vec!["Requisito A".to_string()],
            grupo_servico: None,
            orgao: None,
            publicos: vec![],
        };
        let s = build_descricao("Cidadão", "Grupo X", "Meu Título", &d);
        let linhas: Vec<&str> = s.lines().collect();
        // As 3 primeiras linhas são o header tipo/classe/titulo (que descricao_body descarta).
        assert_eq!(&linhas[0..3], &["Cidadão", "Grupo X", "Meu Título"]);
        assert!(s.contains("Fim do serviço."));
        assert!(s.contains("Etapas para realização do serviço:"));
        assert!(s.contains("- Passo 1"));
        assert!(!s.contains("- \n"), "etapa vazia é filtrada");
        assert!(s.contains("Requisitos:"));
        assert!(s.contains("- Requisito A"));
        // O corpo (sem o header) começa na finalidade.
        assert!(auli_scraper_kit::descricao_body(&s).starts_with("Fim do serviço."));
    }
}
