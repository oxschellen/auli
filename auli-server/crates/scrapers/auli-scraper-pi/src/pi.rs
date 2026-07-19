//! Coleta dos serviços da SEFAZ-PI a partir da API JSON do portal (Sydle ONE, molde CE).
//!
//! A página é uma SPA pura: não há HTML server-rendered. A listagem vem do método público `_search`
//! (GET, ElasticSearch) sobre a classe de conteúdo `5cd32901…`, filtrando pelo catálogo cidadão
//! **"Carta de Serviços"** (`parent._id`). Resposta ES `{ hits: { total, hits[] } }`. Cada item já
//! traz `name`, `description` (texto plano) e `friendlyUrl`, então NÃO há chamada de detalhe.
//!
//! Auth: o portal usa um Bearer token **anônimo e público** embutido no shell HTML
//! (`useCookieAuthentication: false`). O token é efêmero, então o extraímos fresh do shell a cada
//! rodada (idêntico ao CE). Sem token, `_search` devolve 403.
//!
//! Transporte: o edge Azion **reseta todo POST** do nosso cliente (curl/ureq/Chromium falham). Mas
//! `_search` é **GET** (corpo na query `?_body=<json url-encoded>`) e GET passa normalmente — então
//! só usamos GET (ureq HTTP/1.1). Nenhum header de browser (Origin/Referer/Sec-*) é necessário.
//!
//! Modelagem (Cenário A, padrão RJ/CE): os serviços têm `tags`/`classification`, mas as classes
//! dessas facetas NÃO autorizam `_search` anônimo (403) e o `getTags` é POST (bloqueado) — logo não
//! são resolvíveis sem login. Público único "Serviços", classe única "Geral". Identidade = `_id`;
//! `link` = `https://portal.sefaz.pi.gov.br/<friendlyUrl>` (rota SPA `/:pathWithId`).

use std::collections::HashSet;
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use auli_contract::{Ocorrencia, Publico, ServicoRaw};
use auli_scraper_kit::clean;
use auli_scraper_kit::http::GetOpts;
use serde::Deserialize;

const BASE: &str = "https://portal.sefaz.pi.gov.br";
/// Shell público de onde extraímos o Bearer anônimo (efêmero).
const SHELL_URL: &str = "https://portal.sefaz.pi.gov.br/";
/// Endpoint do `_search` (GET) sobre a classe de conteúdo. O corpo ES vai em `?_body=`.
const SEARCH_URL: &str =
    "https://portal.sefaz.pi.gov.br/api/1/govPiPortalInstitucional/_classId/5cd32901df14eb3d461160f0/_search";
/// `_id` do catálogo cidadão "Carta de Serviços" (o `parent._id` dos serviços). Os demais catálogos
/// do portal (Notícias, Legislações, Tesouro, Serviços de Pessoal…) NÃO entram — não são a carta.
const CATALOGO_ID: &str = "69381ceceecdd6684a84c49c";
/// `size` do `_search`: teto de itens numa GET (headroom sobre os ~29 atuais). Se o catálogo crescer
/// além disso, `validar` reprova pedindo para aumentar (a paginação por `from` fica para quando ≠ 1
/// página for real). É `_search` do ES (não o `getChildren` do CE) — `size` grande NÃO reduz o total.
const SIZE: u32 = 500;

/// Público único do PI (cenário A — as facetas de público/tema não são resolvíveis anonimamente).
const PUBLICO_NOME: &str = "Serviços";
const PUBLICO_SLUG: &str = "servicos-gerais";
/// Classe única (o catálogo é tratado plano; sem tema resolvível por item).
const CLASSE_GERAL: &str = "Geral";
/// Órgão de origem.
const ORGAO: &str = "SEFAZ-PI";

/// Guard (princípio D-RJ5): mínimo de serviços. Folga abaixo dos 29 observados, mas aperta o bastante
/// para rejeitar catálogo capado / falha de auth (que devolveria 0).
const MIN_SERVICOS: usize = 25;

/// A resposta ES de `_search`.
#[derive(Debug, Deserialize)]
struct SearchResp {
    hits: Hits,
}

#[derive(Debug, Deserialize)]
struct Hits {
    /// Total de matches. O ES desta instância devolve inteiro puro (não o objeto `{value,…}` do ES7+).
    #[serde(default)]
    total: i64,
    #[serde(default)]
    hits: Vec<Hit>,
}

#[derive(Debug, Deserialize)]
struct Hit {
    #[serde(rename = "_source")]
    source: Source,
}

/// Campos do serviço que usamos (`_source`); serde ignora o resto. `name`/`description`/`friendlyUrl`
/// vêm como `null` em alguns docs (ex.: SAC sem `friendlyUrl`), então são `Option` (null → ausente).
#[derive(Debug, Deserialize)]
struct Source {
    #[serde(rename = "_id", default)]
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(rename = "friendlyUrl", default)]
    friendly_url: Option<String>,
}

/// Raspa o catálogo e devolve `(items, publicos_ordem)` prontos para o snapshot v3.
pub fn scrape(
    data_dir: &str,
    use_cache: bool,
) -> Result<(Vec<ServicoRaw>, Vec<Publico>), Box<dyn std::error::Error>> {
    let agent =
        auli_scraper_kit::build_agent(auli_scraper_kit::USER_AGENT, Some(Duration::from_secs(30)));

    // Chave de cache: lógica e CURTA (`SEARCH_URL#<catálogo>`). A URL real de fetch carrega o `_body`
    // ES url-encoded, longo demais para virar nome de arquivo (os error 36) — não serve de chave.
    let cache_key = format!("{}#{}", SEARCH_URL, CATALOGO_ID);
    // O JSON cru vai para o cache — mas só DEPOIS dos guards (D-RJ5): resposta capada nunca envenena.
    let (json, fetched) = match auli_scraper_kit::cache::read(data_dir, "servicos", &cache_key) {
        Some(cached) => {
            println!("Cache hit: {}", SEARCH_URL);
            (cached, false)
        }
        None => {
            if use_cache {
                return Err(anyhow!(
                    "cache vazio para o catálogo (modo --usecache, sem rede). Rode uma coleta com \
                     rede primeiro."
                )
                .into());
            }
            let token = fetch_token(&agent)?;
            (fetch_services(&agent, &token, &search_url())?, true)
        }
    };

    let resp = parse(&json)?;
    let items = build_servicos(&resp.hits.hits);
    validar(&items, resp.hits.total)?;

    if fetched {
        auli_scraper_kit::cache::write(data_dir, "servicos", &cache_key, &json);
    }

    println!("PI: {} serviços (total ES={}, dedup por _id)", items.len(), resp.hits.total);
    let publicos_ordem =
        vec![Publico { nome: PUBLICO_NOME.to_string(), slug: PUBLICO_SLUG.to_string() }];
    Ok((items, publicos_ordem))
}

/// Extrai o Bearer token anônimo do shell público (`"Authorization":"Bearer …"`). Efêmero.
fn fetch_token(agent: &ureq::Agent) -> Result<String> {
    println!("Fetching token (shell): {}", SHELL_URL);
    let shell = auli_scraper_kit::http::get_string(
        agent,
        SHELL_URL,
        &GetOpts { log_prefix: "PI", ..Default::default() },
    )?;
    parse_token(&shell)
}

/// GET `_search`; devolve o corpo JSON cru. Retenta com backoff (via kit).
fn fetch_services(agent: &ureq::Agent, token: &str, url: &str) -> Result<String> {
    println!("GET _search (Carta de Serviços): {}", SEARCH_URL);
    let body = auli_scraper_kit::http::get_string(
        agent,
        url,
        &GetOpts {
            log_prefix: "PI",
            accept: Some("application/json"),
            headers: &[("Authorization", token)],
            ..Default::default()
        },
    )?;
    // Defesa: um erro/negação do edge vem como HTML ou como `{generalMessages:[…]}`; o corpo bom é o
    // envelope ES. Se não começa com `{`, aborta (não deixa lixo virar snapshot).
    if !body.trim_start().starts_with('{') {
        bail!("resposta inesperada do _search (não-JSON) — edge/erro? primeiros bytes: {:?}",
            &body.chars().take(60).collect::<String>());
    }
    Ok(body)
}

/// Monta a URL de `_search` com o corpo ES url-encoded em `?_body=`.
fn search_url() -> String {
    let body = serde_json::json!({
        "query": { "bool": { "must": [
            { "term": { "active": true } },
            { "terms": { "parent._id": [CATALOGO_ID] } }
        ] } },
        "size": SIZE
    })
    .to_string();
    format!("{}?_body={}", SEARCH_URL, pct_encode(&body))
}

/// Percent-encode para valor de query string: preserva os "unreserved" do RFC 3986 e escapa o resto
/// (inclusive `{ } [ ] " : ,` do JSON). Sem crate — evita a dependência (padrão do fleet).
fn pct_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for &b in s.as_bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push('%');
            out.push(hex(b >> 4));
            out.push(hex(b & 0xf));
        }
    }
    out
}

fn hex(n: u8) -> char {
    match n {
        0..=9 => (b'0' + n) as char,
        _ => (b'A' + (n - 10)) as char,
    }
}

/// Extrai `Bearer …` do primeiro `"Authorization":"…"` do shell. Sem regex (evita a dependência).
/// Idêntico ao CE — o shell Sydle ONE embute o token no `REQUEST_PARAMS.headers`.
fn parse_token(shell: &str) -> Result<String> {
    const KEY: &str = "\"Authorization\":\"";
    let start = shell
        .find(KEY)
        .ok_or_else(|| anyhow!("token não encontrado no shell (markup mudou?)"))?
        + KEY.len();
    let rest = &shell[start..];
    let end = rest.find('"').ok_or_else(|| anyhow!("token malformado no shell"))?;
    let tok = rest[..end].trim();
    if !tok.starts_with("Bearer ") || tok.len() < 20 {
        bail!("valor de Authorization inesperado no shell");
    }
    Ok(tok.to_string())
}

/// Parseia a resposta ES `{ hits: { total, hits[] } }`.
fn parse(json: &str) -> Result<SearchResp> {
    serde_json::from_str::<SearchResp>(json).map_err(|e| anyhow!("JSON de _search inválido: {}", e))
}

/// Monta os `ServicoRaw` a partir dos hits, deduplicando por `_id` (ordem de descoberta).
fn build_servicos(hits: &[Hit]) -> Vec<ServicoRaw> {
    let mut vistos: HashSet<String> = HashSet::new();
    let mut out: Vec<ServicoRaw> = Vec::new();
    for h in hits {
        let s = &h.source;
        if s.id.is_empty() || !vistos.insert(s.id.clone()) {
            continue;
        }
        let titulo = clean(s.name.as_deref().unwrap_or_default());
        if titulo.is_empty() {
            continue;
        }
        out.push(ServicoRaw {
            titulo,
            descricao: clean(s.description.as_deref().unwrap_or_default()),
            link: link(s.friendly_url.as_deref().unwrap_or_default(), &s.id),
            orgao: ORGAO.to_string(),
            ocorrencias: vec![Ocorrencia {
                publico: PUBLICO_NOME.to_string(),
                classe: CLASSE_GERAL.to_string(),
            }],
        });
    }
    out
}

/// Link canônico de detalhe: `…/<friendlyUrl>` (rota SPA `/:pathWithId`). Sem `friendlyUrl` (raro),
/// cai no `/<_id>` (rota `/:id`), evitando um link vazio.
fn link(friendly_url: &str, id: &str) -> String {
    let slug = friendly_url.trim();
    if slug.is_empty() {
        format!("{}/{}", BASE, id)
    } else {
        format!("{}/{}", BASE, slug)
    }
}

/// Guard (princípio D-RJ5): reprova catálogo capado (abaixo do mínimo) e a subcobertura de página
/// (o `_search` anunciou mais matches do que coletamos → aumentar `SIZE`/paginar).
fn validar(items: &[ServicoRaw], total: i64) -> Result<()> {
    let unicos = items.len();
    if total > unicos as i64 {
        bail!(
            "coletados {} < total ES {} — a página não cobre o catálogo. Aumente SIZE (hoje {}) ou \
             pagine por `from`.",
            unicos,
            total,
            SIZE
        );
    }
    if unicos < MIN_SERVICOS {
        bail!(
            "catálogo capado? só {} serviço(s) (mínimo {}). Se veio do cache, limpe data/pi/raw/cache/ \
             e re-raspe (token pode ter expirado).",
            unicos,
            MIN_SERVICOS
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Fixture derivada de hits reais do `_search` (campos que usamos).
    const RESP_JSON: &str = r#"{
      "hits": {
        "total": 3,
        "hits": [
          {"_source": {"_id":"69d527d9c41c7a090156fb4d","name":"Nota Fiscal Avulsa Eletrônica",
            "friendlyUrl":"nota-fiscal-avulsa-eletronica",
            "description":"Emissão de nota fiscal avulsa pelo contribuinte."}},
          {"_source": {"_id":"aaa111","name":"  Agendamento de  Atendimento ",
            "friendlyUrl":"agendamento","description":"Agende seu atendimento presencial."}},
          {"_source": {"_id":"bbb222","name":"SAC","friendlyUrl":"",
            "description":"Serviço de Atendimento ao Consumidor."}}
        ]
      }
    }"#;

    #[test]
    fn parse_le_total_e_hits() {
        let r = parse(RESP_JSON).unwrap();
        assert_eq!(r.hits.total, 3);
        assert_eq!(r.hits.hits.len(), 3);
        assert_eq!(r.hits.hits[0].source.friendly_url.as_deref(), Some("nota-fiscal-avulsa-eletronica"));
    }

    #[test]
    fn build_mapeia_campos_e_link() {
        let r = parse(RESP_JSON).unwrap();
        let items = build_servicos(&r.hits.hits);
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].titulo, "Nota Fiscal Avulsa Eletrônica");
        assert_eq!(items[0].descricao, "Emissão de nota fiscal avulsa pelo contribuinte.");
        assert_eq!(items[0].link, "https://portal.sefaz.pi.gov.br/nota-fiscal-avulsa-eletronica");
        assert_eq!(items[0].orgao, "SEFAZ-PI");
        assert_eq!(items[0].ocorrencias.len(), 1);
        assert_eq!(items[0].ocorrencias[0].publico, "Serviços");
        assert_eq!(items[0].ocorrencias[0].classe, "Geral");
        // clean() comprime espaços do título do segundo item.
        assert_eq!(items[1].titulo, "Agendamento de Atendimento");
        // friendlyUrl vazio -> link cai no _id.
        assert_eq!(items[2].link, "https://portal.sefaz.pi.gov.br/bbb222");
    }

    #[test]
    fn campos_null_nao_quebram() {
        // Docs reais trazem `description`/`friendlyUrl` = null; null vira ausente (não erro).
        let json = r#"{"hits":{"total":1,"hits":[
          {"_source":{"_id":"z9","name":"Serviço X","friendlyUrl":null,"description":null}}
        ]}}"#;
        let r = parse(json).unwrap();
        let items = build_servicos(&r.hits.hits);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].descricao, "");
        assert_eq!(items[0].link, "https://portal.sefaz.pi.gov.br/z9");
    }

    #[test]
    fn dedup_por_id() {
        // O mesmo `_id` repetido vira um único serviço.
        let json = r#"{"hits":{"total":2,"hits":[
          {"_source":{"_id":"x1","name":"A","friendlyUrl":"a","description":"d"}},
          {"_source":{"_id":"x1","name":"A dup","friendlyUrl":"a","description":"d"}}
        ]}}"#;
        let r = parse(json).unwrap();
        assert_eq!(build_servicos(&r.hits.hits).len(), 1);
    }

    #[test]
    fn pct_encode_escapa_json() {
        let enc = pct_encode(r#"{"a":["b"]}"#);
        assert_eq!(enc, "%7B%22a%22%3A%5B%22b%22%5D%7D");
        // unreserved preservados.
        assert_eq!(pct_encode("Az9-_.~"), "Az9-_.~");
    }

    #[test]
    fn search_url_carrega_catalogo_e_size() {
        let u = search_url();
        assert!(u.starts_with(SEARCH_URL));
        // o corpo url-encoded deve conter o id do catálogo (dígitos ficam literais).
        assert!(u.contains(CATALOGO_ID), "url deve filtrar pelo catálogo Carta de Serviços");
    }

    #[test]
    fn parse_token_extrai_bearer() {
        let shell = r#"...window.SYDLE.config = {"ui-api":{"REQUEST_PARAMS":{"headers":{"Authorization":"Bearer eyJabc.def.ghi_LONG_TOKEN"}}}}..."#;
        assert_eq!(parse_token(shell).unwrap(), "Bearer eyJabc.def.ghi_LONG_TOKEN");
    }

    #[test]
    fn validar_reprova_subcobertura() {
        // total ES > coletados: a página não cobre o catálogo.
        let items: Vec<ServicoRaw> = (0..MIN_SERVICOS)
            .map(|i| ServicoRaw {
                titulo: format!("S{i}"),
                descricao: String::new(),
                link: format!("l{i}"),
                orgao: ORGAO.into(),
                ocorrencias: vec![],
            })
            .collect();
        let err = validar(&items, MIN_SERVICOS as i64 + 1).unwrap_err().to_string();
        assert!(err.contains("não cobre"), "esperava guard de subcobertura, veio: {err}");
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
        let err = validar(&poucos, 1).unwrap_err().to_string();
        assert!(err.contains("capado"), "esperava guard de mínimo, veio: {err}");
    }

    #[test]
    fn validar_aceita_catalogo_ok() {
        let items: Vec<ServicoRaw> = (0..MIN_SERVICOS)
            .map(|i| ServicoRaw {
                titulo: format!("S{i}"),
                descricao: String::new(),
                link: format!("l{i}"),
                orgao: ORGAO.into(),
                ocorrencias: vec![],
            })
            .collect();
        assert!(validar(&items, MIN_SERVICOS as i64).is_ok());
    }
}
