//! Coleta dos serviços da SEFIN-RO a partir da "Agência Virtual" (Sydle ONE, geração **conecta-360**).
//!
//! A Agência Virtual é uma SPA Sydle ONE — a MESMA geração do PI (produto "conecta-360"), **não** a do
//! CE. A listagem vem do método público `_search` (GET, ElasticSearch) sobre a classe de conteúdo
//! `5cd32901…` (compartilhada com o PI), filtrando pelo catálogo **"Serviços"** (`parent._id`).
//! Resposta ES `{ hits: { total, hits[] } }`. Cada item já traz `name`, `description` (curta) e
//! `identifier`, então NÃO há chamada de detalhe.
//!
//! Auth: Bearer token **anônimo** embutido no shell da Agência Virtual (`useCookieAuthentication:
//! false`), efêmero → re-extraído do shell a cada rodada (idêntico ao CE/PI). O shell fica em
//! `agenciavirtual.…` mas a API fica em `sydleone.…` (tenant por host, não por header como o CE).
//!
//! Modelagem (Cenário A, padrão CE/PI): os serviços não têm `tags` (público) e a `classification`
//! (tema) não é resolvível anonimamente (403) — público único "Serviços", classe única "Geral".
//! Identidade = `_id`; `link` = `…/catalogo-servicos+<identifier>+<_id>` (gramática Sydle).
//!
//! Escopo (como o CE): só o catálogo "Serviços" (os catálogos "Temas"/"Conteúdos" são informativos).
//! D-PA-ROBOTS cobre o RO como caso preventivo — UA institucional AuliBot; nunca autenticar.

use std::collections::HashSet;
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use auli_contract::{Ocorrencia, Publico, ServicoRaw};
use auli_scraper_kit::clean;
use auli_scraper_kit::http::GetOpts;
use serde::Deserialize;

/// UA institucional do projeto (mitigação D-PA-ROBOTS): nunca UA de browser falso.
const USER_AGENT: &str =
    "AuliBot/0.1 (+https://github.com/oxschellen/auli; carlos.schellenberger@gmail.com)";

/// Base do link cidadão (página do serviço na Agência Virtual, gramática Sydle `{slug}+{id}`).
const LINK_BASE: &str = "https://agenciavirtual.sefin.ro.gov.br/catalogo-servicos";
/// Shell público de onde extraímos o Bearer anônimo (efêmero).
const SHELL_URL: &str = "https://agenciavirtual.sefin.ro.gov.br/";
/// Endpoint do `_search` (GET) sobre a classe de conteúdo (API em `sydleone.…`, app `servicedesk-embedded`).
const SEARCH_URL: &str =
    "https://sydleone.sefin.ro.gov.br/api/1/servicedesk-embedded/_classId/5cd32901df14eb3d461160f0/_search";
/// `_id` do catálogo "Serviços" (o `parent._id` dos serviços). "Temas"/"Conteúdos" NÃO entram.
const CATALOGO_ID: &str = "662c1875ee982159b7b199c9";
/// `size` do `_search`: teto de itens numa GET (headroom sobre os ~194 atuais). Se o catálogo crescer
/// além disso, `validar` reprova (`únicos < total`). É `_search` do ES: `size` grande NÃO reduz o total.
const SIZE: u32 = 500;

/// Público único do RO (cenário A — sem faceta de público/tema resolvível).
const PUBLICO_NOME: &str = "Serviços";
const PUBLICO_SLUG: &str = "servicos-gerais";
/// Classe única (a `classification` não é resolvível anonimamente).
const CLASSE_GERAL: &str = "Geral";
/// Órgão de origem.
const ORGAO: &str = "SEFIN-RO";

/// Guard (princípio D-RJ5): mínimo de serviços. Folga abaixo dos 194 observados; rejeita catálogo
/// capado / falha de auth (que devolveria 0).
const MIN_SERVICOS: usize = 170;

/// A resposta ES de `_search`.
#[derive(Debug, Deserialize)]
struct SearchResp {
    hits: Hits,
}

#[derive(Debug, Deserialize)]
struct Hits {
    /// Total de matches. Inteiro puro (não o objeto `{value,…}` do ES7+).
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

/// Campos do serviço que usamos (`_source`); serde ignora o resto. `name`/`description`/`identifier`
/// podem vir `null` em alguns docs, então são `Option` (null → ausente).
#[derive(Debug, Deserialize)]
struct Source {
    #[serde(rename = "_id", default)]
    id: String,
    #[serde(default)]
    identifier: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
}

/// Raspa o catálogo e devolve `(items, publicos_ordem)` prontos para o snapshot v3.
pub fn scrape(
    data_dir: &str,
    use_cache: bool,
) -> Result<(Vec<ServicoRaw>, Vec<Publico>), Box<dyn std::error::Error>> {
    let agent = auli_scraper_kit::build_agent(USER_AGENT, Some(Duration::from_secs(30)));

    // Chave de cache lógica e CURTA (`SEARCH_URL#<catálogo>`): a URL real carrega o `_body` ES
    // url-encoded, longo demais para virar nome de arquivo.
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

    println!("RO: {} serviços (total ES={}, dedup por _id)", items.len(), resp.hits.total);
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
        &GetOpts { log_prefix: "RO", ..Default::default() },
    )?;
    parse_token(&shell)
}

/// GET `_search`; devolve o corpo JSON cru. Retenta com backoff (via kit).
fn fetch_services(agent: &ureq::Agent, token: &str, url: &str) -> Result<String> {
    println!("GET _search (catálogo Serviços): {}", SEARCH_URL);
    let body = auli_scraper_kit::http::get_string(
        agent,
        url,
        &GetOpts {
            log_prefix: "RO",
            accept: Some("application/json"),
            headers: &[("Authorization", token)],
            ..Default::default()
        },
    )?;
    // Defesa: um erro/negação vem como HTML ou `{generalMessages:[…]}`; o corpo bom é o envelope ES.
    if !body.trim_start().starts_with('{') {
        bail!("resposta inesperada do _search (não-JSON) — erro? primeiros bytes: {:?}",
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

/// Percent-encode para valor de query string (preserva "unreserved", escapa o resto). Sem crate.
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

/// Extrai `Bearer …` do primeiro `"Authorization":"…"` do shell. Sem regex. Idêntico ao CE/PI.
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
            link: link(s.identifier.as_deref().unwrap_or_default(), &s.id),
            orgao: ORGAO.to_string(),
            ocorrencias: vec![Ocorrencia {
                publico: PUBLICO_NOME.to_string(),
                classe: CLASSE_GERAL.to_string(),
            }],
        });
    }
    out
}

/// Link canônico Sydle: `…/catalogo-servicos+<identifier>+<_id>`. Sem `identifier`, cai na forma
/// curta `…/catalogo-servicos+<_id>` (evita o malformado `catalogo-servicos++<_id>`).
fn link(identifier: &str, id: &str) -> String {
    let slug = identifier.trim();
    if slug.is_empty() {
        format!("{}+{}", LINK_BASE, id)
    } else {
        format!("{}+{}+{}", LINK_BASE, slug, id)
    }
}

/// Guard (princípio D-RJ5): reprova subcobertura (`total > coletados` → aumentar SIZE/paginar) e
/// catálogo capado (abaixo do mínimo).
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
            "catálogo capado? só {} serviço(s) (mínimo {}). Se veio do cache, limpe data/ro/raw/cache/ \
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
          {"_source": {"_id":"663da0ecee982159b74f8b96","identifier":"requisitar-csc",
            "name":"Requisitar CSC","description":"Requisitar Código de Segurança do Contribuinte."}},
          {"_source": {"_id":"aaa111","identifier":"emissao-de-dua",
            "name":"  Emissão de  DUA ","description":"Emite o DUA."}},
          {"_source": {"_id":"bbb222","identifier":null,"name":"SAC","description":null}}
        ]
      }
    }"#;

    #[test]
    fn parse_le_total_e_hits() {
        let r = parse(RESP_JSON).unwrap();
        assert_eq!(r.hits.total, 3);
        assert_eq!(r.hits.hits.len(), 3);
        assert_eq!(r.hits.hits[0].source.identifier.as_deref(), Some("requisitar-csc"));
    }

    #[test]
    fn build_mapeia_campos_e_link() {
        let r = parse(RESP_JSON).unwrap();
        let items = build_servicos(&r.hits.hits);
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].titulo, "Requisitar CSC");
        assert_eq!(
            items[0].link,
            "https://agenciavirtual.sefin.ro.gov.br/catalogo-servicos+requisitar-csc+663da0ecee982159b74f8b96"
        );
        assert_eq!(items[0].orgao, "SEFIN-RO");
        assert_eq!(items[0].ocorrencias.len(), 1);
        assert_eq!(items[0].ocorrencias[0].publico, "Serviços");
        assert_eq!(items[0].ocorrencias[0].classe, "Geral");
        // clean() comprime os espaços do título do segundo item.
        assert_eq!(items[1].titulo, "Emissão de DUA");
        // identifier vazio (null) -> forma curta do link; description null -> vazia.
        assert_eq!(items[2].link, "https://agenciavirtual.sefin.ro.gov.br/catalogo-servicos+bbb222");
        assert_eq!(items[2].descricao, "");
    }

    #[test]
    fn dedup_por_id() {
        let json = r#"{"hits":{"total":2,"hits":[
          {"_source":{"_id":"x1","identifier":"a","name":"A","description":"d"}},
          {"_source":{"_id":"x1","identifier":"a","name":"A dup","description":"d"}}
        ]}}"#;
        let r = parse(json).unwrap();
        assert_eq!(build_servicos(&r.hits.hits).len(), 1);
    }

    #[test]
    fn pct_encode_escapa_json() {
        assert_eq!(pct_encode(r#"{"a":["b"]}"#), "%7B%22a%22%3A%5B%22b%22%5D%7D");
        assert_eq!(pct_encode("Az9-_.~"), "Az9-_.~");
    }

    #[test]
    fn search_url_carrega_catalogo_e_size() {
        let u = search_url();
        assert!(u.starts_with(SEARCH_URL));
        assert!(u.contains(CATALOGO_ID), "url deve filtrar pelo catálogo Serviços");
    }

    #[test]
    fn parse_token_extrai_bearer() {
        let shell = r#"...window.SYDLE.config = {"ui-api":{"REQUEST_PARAMS":{"headers":{"Authorization":"Bearer eyJabc.def.ghi_LONG_TOKEN"}}}}..."#;
        assert_eq!(parse_token(shell).unwrap(), "Bearer eyJabc.def.ghi_LONG_TOKEN");
    }

    #[test]
    fn validar_reprova_subcobertura() {
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
}
