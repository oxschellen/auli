//! Coleta dos serviços da SEFAZ-CE a partir da API JSON do "Portal de Serviços" (Sydle ONE).
//!
//! A página é uma SPA pura (D-CE1): não há HTML server-rendered. A listagem vem do método público
//! `getChildren` (POST) sobre o catálogo `servico-geral` — resposta `{ hits, return[] }` paginada
//! por `params.pageIndex`/`pageSize`. Cada item já traz `name` (título) e `description` não-vazia,
//! então NÃO há chamada de detalhe (a listagem é rica).
//!
//! Auth (D-CE1): o portal usa um Bearer token **anônimo e público** embutido no shell HTML
//! (`useCookieAuthentication: false`). O token é efêmero, então o extraímos fresh do shell a cada
//! rodada. O servidor autoriza `getChildren` ao anônimo (o genérico `_get` é bloqueado por ACL).
//!
//! Modelagem (D-CE2/3): identidade = `_id` do documento (único); `link` = URL canônica de detalhe
//! `…/servico-geral+<identifier>+<_id>`. Os itens não têm `_tags` → sem eixo de público (cenário A,
//! padrão RJ): público único "Serviços", classe "Geral".
//!
//! Guards (princípio D-RJ5): falha alto se o catálogo vier capado; o cache só grava DEPOIS dos
//! guards. Nota: `hits` (392) > itens entregues ao anônimo (~292): os inativos não vêm, e o scrape
//! público pega os ativos — por isso a paginação para na página vazia, não no `hits`.

use std::collections::HashSet;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use auli_contract::{Ocorrencia, Publico, ServicoRaw};
use serde::Deserialize;

const BASE: &str = "https://portalservicos.sefaz.ce.gov.br";
/// Shell público de onde extraímos o Bearer anônimo (efêmero).
const SHELL_URL: &str = "https://portalservicos.sefaz.ce.gov.br/";
/// Endpoint do método público de catálogo (classe API do service-desk + método `getChildren`).
const GETCHILDREN_URL: &str = "https://portalservicos.sefaz.ce.gov.br/api/1/servicedesk-embedded/_classId/5cd5e83d59f8170cd4dc2c43/getChildren";
/// `application` no corpo (≠ do header de tenant abaixo).
const APP_IN_BODY: &str = "sefaz-ceara";
/// Tenant no header `X-Explorer-Account-Token`.
const ORG_HEADER: &str = "sefazce";
/// `_id` do catálogo `servico-geral` (= o ObjectId da URL de listagem).
const CATALOGO_ID: &str = "648af76264778b7336c470a3";

const USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64; rv:124.0) Gecko/20100101 Firefox/124.0";
const PAGE_SIZE: u32 = 100;
/// Teto de páginas (guarda contra loop; 292 itens ÷ 100 = 3 páginas + a vazia).
const MAX_PAGES: u32 = 30;
/// Cortesia entre páginas.
const COURTESY: Duration = Duration::from_millis(400);

/// Público único do CE (cenário A — os itens não têm faceta de público).
const PUBLICO_NOME: &str = "Serviços";
const PUBLICO_SLUG: &str = "servicos-gerais";
/// Classe única (o catálogo é plano; sem tema por item).
const CLASSE_GERAL: &str = "Geral";
/// Órgão de origem.
const ORGAO: &str = "SEFAZ-CE";

/// Guard D-CE (princípio D-RJ5): mínimo de serviços ativos (folga sobre os ~292 observados, mas
/// aperta o bastante para rejeitar catálogo capado).
const MIN_SERVICOS: usize = 200;

/// Uma página da resposta de `getChildren`.
#[derive(Debug, Deserialize)]
struct Page {
    #[serde(default)]
    hits: i64,
    #[serde(rename = "return", default)]
    items: Vec<Item>,
}

/// Um item de serviço do `return[]`. Só os campos que usamos; serde ignora o resto.
#[derive(Debug, Deserialize)]
struct Item {
    #[serde(rename = "_id")]
    id: String,
    #[serde(default)]
    identifier: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    description: String,
}

/// Raspa o catálogo e devolve `(items, publicos_ordem)` prontos para o snapshot v3.
pub fn scrape(
    data_dir: &str,
    use_cache: bool,
) -> Result<(Vec<ServicoRaw>, Vec<Publico>), Box<dyn std::error::Error>> {
    let agent = auli_scraper_kit::build_agent(USER_AGENT, Some(Duration::from_secs(30)));

    // Páginas cruas (logical_url, json), na ordem. O cache grava só DEPOIS dos guards (D-RJ5).
    let mut raw_pages: Vec<(String, String)> = Vec::new();
    let mut token: Option<String> = None; // buscado de forma preguiçosa, uma vez.
    let mut fetched_any = false;

    let mut page = 1u32;
    loop {
        let logical = format!("{}#page={}", GETCHILDREN_URL, page);
        let (json, from_cache) = match auli_scraper_kit::cache::read(data_dir, &logical) {
            Some(cached) => {
                println!("Cache hit (página {}): {}", page, GETCHILDREN_URL);
                (cached, true)
            }
            None => {
                if use_cache {
                    return Err(anyhow!(
                        "cache miss para página {} (modo --usecache, sem rede)",
                        page
                    )
                    .into());
                }
                if token.is_none() {
                    token = Some(fetch_token(&agent)?);
                }
                let body = fetch_page(&agent, token.as_ref().unwrap(), page)?;
                fetched_any = true;
                (body, false)
            }
        };

        let parsed = parse_page(&json)?;
        let n = parsed.items.len();
        println!("CE: página {} -> {} itens (hits={})", page, n, parsed.hits);
        raw_pages.push((logical, json));

        if n < PAGE_SIZE as usize {
            break; // última página (parcial ou vazia)
        }
        page += 1;
        if page > MAX_PAGES {
            eprintln!("⚠️  CE: atingiu MAX_PAGES ({}); parando a paginação.", MAX_PAGES);
            break;
        }
        if !from_cache {
            sleep(COURTESY);
        }
    }

    // Monta os serviços (dedup por `_id`) e valida antes de gravar o cache.
    let pages: Vec<Page> = raw_pages.iter().map(|(_, j)| parse_page(j)).collect::<Result<_>>()?;
    let items = build_servicos(&pages);
    validar(&items)?;

    // Cache só DEPOIS dos guards (D-RJ5): uma resposta capada nunca envenena o cache.
    if fetched_any {
        for (logical, json) in &raw_pages {
            auli_scraper_kit::cache::write(data_dir, logical, json);
        }
    }

    println!("CE: {} serviços ativos (dedup por _id)", items.len());
    let publicos_ordem =
        vec![Publico { nome: PUBLICO_NOME.to_string(), slug: PUBLICO_SLUG.to_string() }];
    Ok((items, publicos_ordem))
}

/// Extrai o Bearer token anônimo do shell público (`"Authorization":"Bearer …"`). Efêmero.
fn fetch_token(agent: &ureq::Agent) -> Result<String> {
    println!("Fetching token (shell): {}", SHELL_URL);
    let shell = get_string(agent, SHELL_URL)?;
    parse_token(&shell)
}

/// POST `getChildren` de uma página; devolve o corpo JSON cru. Retenta com backoff.
fn fetch_page(agent: &ureq::Agent, token: &str, page: u32) -> Result<String> {
    let body = build_body(page);
    let max_attempts = 3;
    let mut delay = Duration::from_millis(800);
    let mut last = anyhow!("sem tentativa");
    println!("POST getChildren (página {})", page);
    for attempt in 1..=max_attempts {
        let sent = agent
            .post(GETCHILDREN_URL)
            .header("Authorization", token)
            .header("X-Explorer-Account-Token", ORG_HEADER)
            .header("Accept", "application/json")
            .send_json(&body);
        match sent {
            Ok(mut resp) => match resp.body_mut().read_to_string() {
                Ok(s) if !s.trim().is_empty() => return Ok(s),
                Ok(_) => last = anyhow!("resposta vazia"),
                Err(e) => last = anyhow!(e.to_string()),
            },
            Err(e) => last = anyhow!(e.to_string()),
        }
        if attempt < max_attempts {
            eprintln!("⚠️  CE: página {} tentativa {} falhou ({}); retentando…", page, attempt, last);
            sleep(delay);
            delay *= 2;
        }
    }
    Err(anyhow!("falha ao buscar página {} após {} tentativas: {}", page, max_attempts, last))
}

/// GET simples que devolve o corpo como String (com retentativas).
fn get_string(agent: &ureq::Agent, url: &str) -> Result<String> {
    let max_attempts = 3;
    let mut delay = Duration::from_millis(800);
    let mut last = anyhow!("sem tentativa");
    for attempt in 1..=max_attempts {
        match agent.get(url).call() {
            Ok(mut resp) => match resp.body_mut().read_to_string() {
                Ok(s) if !s.trim().is_empty() => return Ok(s),
                Ok(_) => last = anyhow!("resposta vazia"),
                Err(e) => last = anyhow!(e.to_string()),
            },
            Err(e) => last = anyhow!(e.to_string()),
        }
        if attempt < max_attempts {
            sleep(delay);
            delay *= 2;
        }
    }
    Err(anyhow!("falha ao buscar {} após {} tentativas: {}", url, max_attempts, last))
}

/// Corpo mínimo do POST `getChildren` (o `searchConfig` completo do front é dispensável).
fn build_body(page: u32) -> serde_json::Value {
    serde_json::json!({
        "application": APP_IN_BODY,
        "id": CATALOGO_ID,
        "rootId": CATALOGO_ID,
        "path": "servico-geral",
        "view": "list",
        "params": {
            "filters": {},
            "pageIndex": page,
            "pageSize": PAGE_SIZE,
            "sort": { "sort": { "order": "asc" } }
        }
    })
}

/// Extrai `Bearer …` do primeiro `"Authorization":"…"` do shell. Sem regex (evita a dependência).
fn parse_token(shell: &str) -> Result<String> {
    const KEY: &str = "\"Authorization\":\"";
    let start = shell.find(KEY).ok_or_else(|| anyhow!("token não encontrado no shell (markup mudou?)"))?
        + KEY.len();
    let rest = &shell[start..];
    let end = rest.find('"').ok_or_else(|| anyhow!("token malformado no shell"))?;
    let tok = rest[..end].trim();
    if !tok.starts_with("Bearer ") || tok.len() < 20 {
        bail!("valor de Authorization inesperado no shell");
    }
    Ok(tok.to_string())
}

/// Parseia uma página `{ hits, return[] }`.
fn parse_page(json: &str) -> Result<Page> {
    serde_json::from_str::<Page>(json).map_err(|e| anyhow!("JSON de getChildren inválido: {}", e))
}

/// Monta os `ServicoRaw` a partir das páginas, deduplicando por `_id` (ordem de descoberta).
fn build_servicos(pages: &[Page]) -> Vec<ServicoRaw> {
    let mut vistos: HashSet<&str> = HashSet::new();
    let mut out: Vec<ServicoRaw> = Vec::new();
    for page in pages {
        for it in &page.items {
            if it.id.is_empty() || !vistos.insert(it.id.as_str()) {
                continue;
            }
            let titulo = clean(&it.name);
            if titulo.is_empty() {
                continue;
            }
            out.push(ServicoRaw {
                titulo,
                descricao: clean(&it.description),
                link: canonical(&it.identifier, &it.id),
                orgao: ORGAO.to_string(),
                ocorrencias: vec![Ocorrencia {
                    publico: PUBLICO_NOME.to_string(),
                    classe: CLASSE_GERAL.to_string(),
                }],
            });
        }
    }
    out
}

/// URL canônica de detalhe: `…/servico-geral+<identifier>+<_id>`.
fn canonical(identifier: &str, id: &str) -> String {
    format!("{}/servico-geral+{}+{}", BASE, identifier, id)
}

/// Normaliza texto: tira zero-width/nbsp e comprime espaços (padrão dos demais scrapers).
fn clean(s: &str) -> String {
    s.replace('\u{200b}', "").replace('\u{00a0}', " ").split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Guard D-CE (princípio D-RJ5): reprova catálogo capado.
fn validar(items: &[ServicoRaw]) -> Result<()> {
    if items.len() < MIN_SERVICOS {
        bail!(
            "catálogo capado? só {} serviço(s) (mínimo {}). Se veio do cache, limpe \
             data/ce/raw/cache/ e re-raspe.",
            items.len(),
            MIN_SERVICOS
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Fixture derivada de itens reais do `getChildren` (campos que usamos).
    const PAGE_JSON: &str = r#"{
      "hits": 392,
      "return": [
        {"_id":"69459012f6e0ff51fef22a9d","identifier":"homologar-di-declaracao-de-importacao",
         "name":"Homologar DI - Declaração de Importação",
         "description":"rEQUISIÇÃO DE AÇÃO FISCAL DE D I AINDA NÃO HOMOLOGADA NO SITRAM"},
        {"_id":"aaa111","identifier":"impugnar-debito-itcd",
         "name":"  Impugnar Débito  Inscrito ",
         "description":"Impugnação de débitos de ITCD."}
      ]
    }"#;

    #[test]
    fn parse_page_le_hits_e_itens() {
        let p = parse_page(PAGE_JSON).unwrap();
        assert_eq!(p.hits, 392);
        assert_eq!(p.items.len(), 2);
        assert_eq!(p.items[0].identifier, "homologar-di-declaracao-de-importacao");
    }

    #[test]
    fn build_mapeia_campos_e_link_canonico() {
        let pages = vec![parse_page(PAGE_JSON).unwrap()];
        let items = build_servicos(&pages);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].titulo, "Homologar DI - Declaração de Importação");
        assert_eq!(items[0].descricao, "rEQUISIÇÃO DE AÇÃO FISCAL DE D I AINDA NÃO HOMOLOGADA NO SITRAM");
        assert_eq!(
            items[0].link,
            "https://portalservicos.sefaz.ce.gov.br/servico-geral+homologar-di-declaracao-de-importacao+69459012f6e0ff51fef22a9d"
        );
        assert_eq!(items[0].orgao, "SEFAZ-CE");
        assert_eq!(items[0].ocorrencias.len(), 1);
        assert_eq!(items[0].ocorrencias[0].publico, "Serviços");
        assert_eq!(items[0].ocorrencias[0].classe, "Geral");
        // clean() comprime espaços do segundo item.
        assert_eq!(items[1].titulo, "Impugnar Débito Inscrito");
    }

    #[test]
    fn dedup_por_id_mesmo_id_uma_vez() {
        // O mesmo `_id` em duas páginas vira um único serviço.
        let pages = vec![parse_page(PAGE_JSON).unwrap(), parse_page(PAGE_JSON).unwrap()];
        let items = build_servicos(&pages);
        assert_eq!(items.len(), 2, "dois _id distintos, mesmo repetidos entre páginas");
    }

    #[test]
    fn identifier_repetido_com_ids_distintos_sao_dois() {
        // A identidade é o `_id` (identifier não é único no catálogo real).
        let json = r#"{"return":[
          {"_id":"id1","identifier":"parcelar-auto-de-infracao","name":"Parcelar Auto A","description":"x"},
          {"_id":"id2","identifier":"parcelar-auto-de-infracao","name":"Parcelar Auto B","description":"y"}
        ]}"#;
        let items = build_servicos(&[parse_page(json).unwrap()]);
        assert_eq!(items.len(), 2);
        assert_ne!(items[0].link, items[1].link);
    }

    #[test]
    fn parse_token_extrai_bearer() {
        let shell = r#"...<script>window.SYDLE.config = {"ui-api":{"REQUEST_PARAMS":{"headers":{"Authorization":"Bearer eyJabc.def.ghi_LONG_TOKEN_VALUE"}}}}</script>..."#;
        assert_eq!(parse_token(shell).unwrap(), "Bearer eyJabc.def.ghi_LONG_TOKEN_VALUE");
    }

    #[test]
    fn validar_reprova_catalogo_capado() {
        let poucos = vec![ServicoRaw {
            titulo: "x".into(),
            descricao: String::new(),
            link: "l".into(),
            orgao: ORGAO.into(),
            ocorrencias: vec![],
        }];
        assert!(validar(&poucos).is_err());
    }
}
