//! Coleta dos serviços da SEFAZ-GO (Secretaria de Estado da Economia) da API do Portal Expresso.
//!
//! O portal (`go.gov.br/servicos`) é uma SPA Angular: sem HTML server-rendered (D-GO1). A listagem
//! por órgão vem da API WSO2 **`GET /expresso/2.0.0/servicosOrgaos/{id}`** — um array JSON com os
//! serviços do órgão, descrição (`infoServico`, HTML) e categorias inline. Sem paginação.
//!
//! Auth (D-GO3): a API exige `Authorization: Bearer <token>`; o token é obtido por
//! **client_credentials ANÔNIMO** (`POST /oauth2/token`, sem login de usuário). Efêmero → re-obtido
//! a cada rodada. As credenciais do cliente estão embutidas no bundle Angular servido a QUALQUER
//! visitante — são de um cliente público/anônimo, **não são segredo** (ver `AUTH_BASIC`).
//!
//! Modelagem (D-GO2/4/5): identidade = `idServico`; `link` = `…/servicos/servico/<descUrlAmigavel>`
//! (cru). Cenário A (o dado não traz eixo de público): público único "Serviços". `classe` =
//! `nomeCategoriaServico` (mapeado do `/categorias` pelo `idCategoriaServico`; multi por serviço),
//! fallback "Geral". `descricao` = `infoServico` limpo (HTML → texto). `orgao` = "SEFAZ-GO".
//!
//! Guards: invariante dinâmico `únicos == qtdeServicosPublicados(órgão)` (do `/orgaos`) + piso
//! estático. Cache pós-guards (D-RJ5).

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use auli_contract::{Ocorrencia, Publico, ServicoRaw};
use auli_scraper_kit::clean;
use auli_scraper_kit::http::GetOpts;
use regex::Regex;
use scraper::Html;
use serde::Deserialize;
use ureq::Agent;
use ureq::tls::{TlsConfig, TlsProvider};

const TOKEN_URL: &str = "https://sso.go.gov.br/oauth2/token";
/// Órgão fazendário de GO = Secretaria de Estado da Economia (`idPrestadorServico`).
const ORGAO_ID: u32 = 20;
const SERVICOS_URL: &str = "https://api.go.gov.br/expresso/2.0.0/servicosOrgaos/20";
const ORGAOS_URL: &str = "https://api.go.gov.br/expresso/2.0.0/orgaos";
const CATEGORIAS_URL: &str = "https://api.go.gov.br/expresso/2.0.0/categorias";
/// Base para os links canônicos de detalhe (a SPA).
const CATALOG_BASE: &str = "https://www.go.gov.br/servicos/servico";

/// `Basic base64("<client_secret>:<client_pass>")` do cliente **público anônimo** do Expresso.
/// NÃO é segredo: estas credenciais vêm do bundle Angular (`main.<hash>.js`) servido a qualquer
/// visitante de go.gov.br; qualquer scanner de secrets pode ignorar. Se rotacionarem (401 no
/// token), re-extrair do bundle: `grep -oE 'client_(secret|pass):"[^"]+"'`.
const AUTH_BASIC: &str =
    "Basic ak1Rb3lIX1QyR3BXWHdCbEg2Z29XZkJCZHIwYTprOEJPc0lIVEY2c0FSZkhxNHFCUHN2YVlqZjRh";

/// Público único do GO (Cenário A — o dado não tem eixo de público).
const PUBLICO_NOME: &str = "Serviços";
const PUBLICO_SLUG: &str = "servicos-gerais";
/// Fallback de classe para serviço sem categoria (D-GO).
const GERAL: &str = "Geral";
const ORGAO: &str = "SEFAZ-GO";
/// Piso estático de folga (o invariante principal é `únicos == qtdeServicosPublicados`; 94 hoje).
const MIN_SERVICOS: usize = 70;

/// Tags HTML no `infoServico` — trocadas por espaço (separa parágrafos) antes de decodificar.
static TAG_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]+>").unwrap());

/// Um serviço do `servicosOrgaos`. Só os campos que usamos; serde ignora o resto.
#[derive(Debug, Deserialize)]
struct ServicoItem {
    #[serde(rename = "idServico")]
    id: i64,
    #[serde(rename = "nomeServico", default)]
    nome: String,
    #[serde(rename = "descUrlAmigavel", default)]
    slug: String,
    #[serde(rename = "infoServico", default)]
    info: String,
    #[serde(rename = "categoriaServico", default)]
    categorias: Vec<CatRef>,
}

#[derive(Debug, Deserialize)]
struct CatRef {
    #[serde(rename = "idCategoriaServico")]
    id: i64,
}

#[derive(Debug, Deserialize)]
struct Categoria {
    #[serde(rename = "idCategoriaServico")]
    id: i64,
    #[serde(rename = "nomeCategoriaServico", default)]
    nome: String,
}

#[derive(Debug, Deserialize)]
struct Orgao {
    #[serde(rename = "idPrestadorServico")]
    id: i64,
    #[serde(rename = "qtdeServicosPublicados", default)]
    qtde: i64,
}

/// Raspa o catálogo do órgão e devolve `(items, publicos_ordem)` prontos para o snapshot v3.
pub fn scrape(
    data_dir: &str,
    use_cache: bool,
) -> Result<(Vec<ServicoRaw>, Vec<Publico>), Box<dyn std::error::Error>> {
    // native-tls (não o kit::build_agent): o WAF da api.go.gov.br bloqueia o fingerprint TLS do
    // rustls (ver Cargo.toml). O provider OpenSSL passa.
    let agent = build_agent_native_tls(auli_scraper_kit::USER_AGENT, Duration::from_secs(30));
    let mut token: Option<String> = None;
    // Páginas cruas (url_lógica = url, sem paginação) — o cache só grava DEPOIS dos guards (D-RJ5).
    let mut raw: Vec<(String, String)> = Vec::new();

    // Categorias (id -> nome legível da classe), o total do órgão (invariante) e os serviços.
    let cats_json = get_json(&agent, data_dir, CATEGORIAS_URL, use_cache, &mut token, &mut raw)?;
    let cat_map = parse_categorias(&cats_json)?;

    let orgaos_json = get_json(&agent, data_dir, ORGAOS_URL, use_cache, &mut token, &mut raw)?;
    let total = total_orgao(&orgaos_json, ORGAO_ID)?;

    let svc_json = get_json(&agent, data_dir, SERVICOS_URL, use_cache, &mut token, &mut raw)?;
    let items = parse_servicos(&svc_json)?;

    let (servicos, orfaos) = build_servicos(items, &cat_map);

    // Guards (antes de qualquer escrita de cache).
    validar(&servicos, total)?;
    for (logical, json) in &raw {
        auli_scraper_kit::cache::write(data_dir, logical, json);
    }

    if orfaos > 0 {
        eprintln!("⚠️  GO: {} serviço(s) sem categoria → classe '{}'.", orfaos, GERAL);
    }
    let ocorrencias: usize = servicos.iter().map(|s| s.ocorrencias.len()).sum();
    println!(
        "GO: {} serviços ({} ocorrências); órgão anuncia {} publicados",
        servicos.len(),
        ocorrencias,
        total
    );
    Ok((servicos, vec![Publico { nome: PUBLICO_NOME.to_string(), slug: PUBLICO_SLUG.to_string() }]))
}

/// GET JSON com cache (chave = a própria URL) + Bearer. O token é buscado de forma preguiçosa (só
/// no 1º miss de rede). O cache só grava DEPOIS dos guards, então aqui só acumulamos em `raw`.
fn get_json(
    agent: &ureq::Agent,
    data_dir: &str,
    url: &str,
    use_cache: bool,
    token: &mut Option<String>,
    raw: &mut Vec<(String, String)>,
) -> Result<String> {
    if let Some(cached) = auli_scraper_kit::cache::read_or_bail(data_dir, url, use_cache)? {
        return Ok(cached);
    }
    if token.is_none() {
        *token = Some(fetch_token(agent)?);
    }
    let bearer = format!("Bearer {}", token.as_ref().unwrap());
    let json = auli_scraper_kit::http::get_string(
        agent,
        url,
        &GetOpts {
            log_prefix: "GO",
            accept: Some("application/json"),
            headers: &[("Authorization", bearer.as_str())],
            ..Default::default()
        },
    )?;
    raw.push((url.to_string(), json.clone()));
    Ok(json)
}

/// Agente `ureq` com o provider native-tls (OpenSSL) — o fingerprint que o WAF da api.go.gov.br
/// aceita (o rustls do `kit::build_agent` leva "Acesso Negado"; ver Cargo.toml).
fn build_agent_native_tls(user_agent: &str, timeout: Duration) -> Agent {
    Agent::config_builder()
        .user_agent(user_agent)
        .timeout_global(Some(timeout))
        .tls_config(TlsConfig::builder().provider(TlsProvider::NativeTls).build())
        .build()
        .into()
}

/// POST `client_credentials` (anônimo) → `access_token`. Form-urlencoded (não JSON), então é local,
/// não usa `kit::http::post_json`. Retry com backoff.
fn fetch_token(agent: &ureq::Agent) -> Result<String> {
    let max_attempts = 3;
    let mut delay = Duration::from_millis(800);
    let mut last = anyhow!("sem tentativa");
    println!("POST token (client_credentials): {}", TOKEN_URL);
    for attempt in 1..=max_attempts {
        let sent = agent
            .post(TOKEN_URL)
            .header("Authorization", AUTH_BASIC)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Accept", "application/json")
            .send("grant_type=client_credentials");
        match sent {
            Ok(mut resp) => match resp.body_mut().read_to_string() {
                Ok(s) if !s.trim().is_empty() => return parse_token(&s),
                Ok(_) => last = anyhow!("resposta vazia"),
                Err(e) => last = anyhow!(e.to_string()),
            },
            Err(e) => last = anyhow!(e.to_string()),
        }
        if attempt < max_attempts {
            eprintln!("⚠️  GO: token tentativa {} falhou ({}); retentando…", attempt, last);
            sleep(delay);
            delay *= 2;
        }
    }
    Err(anyhow!("falha ao obter token após {} tentativas: {}", max_attempts, last))
}

/// Extrai `access_token` da resposta do endpoint de token.
fn parse_token(json: &str) -> Result<String> {
    let v: serde_json::Value =
        serde_json::from_str(json).map_err(|e| anyhow!("JSON do token inválido: {}", e))?;
    v["access_token"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| anyhow!("access_token ausente na resposta do token"))
}

/// `idCategoriaServico` -> `nomeCategoriaServico` (a classe legível).
fn parse_categorias(json: &str) -> Result<HashMap<i64, String>> {
    let cats: Vec<Categoria> =
        serde_json::from_str(json).map_err(|e| anyhow!("JSON de /categorias inválido: {}", e))?;
    Ok(cats.into_iter().filter(|c| !c.nome.is_empty()).map(|c| (c.id, clean(&c.nome))).collect())
}

/// `qtdeServicosPublicados` do órgão alvo (base do invariante).
fn total_orgao(json: &str, orgao_id: u32) -> Result<i64> {
    let orgaos: Vec<Orgao> =
        serde_json::from_str(json).map_err(|e| anyhow!("JSON de /orgaos inválido: {}", e))?;
    orgaos
        .iter()
        .find(|o| o.id == orgao_id as i64)
        .map(|o| o.qtde)
        .ok_or_else(|| anyhow!("órgão {} ausente em /orgaos", orgao_id))
}

/// Parseia o array de serviços do `servicosOrgaos`.
fn parse_servicos(json: &str) -> Result<Vec<ServicoItem>> {
    serde_json::from_str(json).map_err(|e| anyhow!("JSON de servicosOrgaos inválido: {}", e))
}

/// Monta os `ServicoRaw` (dedup por `idServico`, ordem de descoberta). `classe` = nome da categoria
/// (mapeado); `ocorrencias` = uma por categoria sob o público único; sem categoria → "Geral".
/// Devolve também quantos serviços caíram no fallback.
fn build_servicos(items: Vec<ServicoItem>, cat_map: &HashMap<i64, String>) -> (Vec<ServicoRaw>, usize) {
    let mut vistos: HashSet<i64> = HashSet::new();
    let mut out: Vec<ServicoRaw> = Vec::new();
    let mut orfaos = 0usize;

    for it in items {
        if !vistos.insert(it.id) {
            continue;
        }
        let titulo = clean(&it.nome);
        if titulo.is_empty() || it.slug.is_empty() {
            continue;
        }

        let mut classes: Vec<String> =
            it.categorias.iter().filter_map(|c| cat_map.get(&c.id).cloned()).collect();
        if classes.is_empty() {
            classes.push(GERAL.to_string());
            orfaos += 1;
        }

        let ocorrencias = classes
            .into_iter()
            .map(|classe| Ocorrencia { publico: PUBLICO_NOME.to_string(), classe })
            .collect();

        out.push(ServicoRaw {
            titulo,
            descricao: html_to_text(&it.info),
            link: format!("{}/{}", CATALOG_BASE, it.slug),
            orgao: ORGAO.to_string(),
            ocorrencias,
        });
    }
    (out, orfaos)
}

/// `infoServico` (HTML) → texto: tags viram espaço (separa parágrafos), o parser html5ever
/// decodifica TODAS as entidades (`&ccedil;`, `&atilde;`, … — além da tabela fixa do kit), e o
/// `kit::clean` comprime espaços.
fn html_to_text(html: &str) -> String {
    let spaced = TAG_RE.replace_all(html, " ");
    let frag = Html::parse_fragment(&spaced);
    let text: String = frag.root_element().text().collect();
    clean(&text)
}

/// Guard: invariante dinâmico `únicos == qtdeServicosPublicados` (o órgão anuncia o total), depois
/// o piso estático de folga.
fn validar(items: &[ServicoRaw], total: i64) -> Result<()> {
    let unicos = items.len();
    if total > 0 && unicos as i64 != total {
        bail!(
            "catálogo incompleto/divergente: órgão anuncia {} publicados e coletamos {} único(s). \
             Se veio do cache, limpe data/go/raw/cache/ e re-raspe.",
            total,
            unicos
        );
    }
    if unicos < MIN_SERVICOS {
        bail!(
            "catálogo capado/vazio? só {} serviço(s) (mínimo {}). Se veio do cache, limpe \
             data/go/raw/cache/ e re-raspe.",
            unicos,
            MIN_SERVICOS
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SVC_JSON: &str = r#"[
      {"idServico":947,"nomeServico":"Agendar Atendimento","descUrlAmigavel":"agendar-atendimento",
       "infoServico":"<p>Servi&ccedil;o de agendamento.</p><p>Segunda etapa.</p>",
       "categoriaServico":[{"idCategoriaServico":21},{"idCategoriaServico":3}]},
      {"idServico":1000,"nomeServico":"  Emitir DARE  ","descUrlAmigavel":"emitir-dare",
       "infoServico":"Sem tags, s&oacute; texto.","categoriaServico":[]},
      {"idServico":947,"nomeServico":"Dup","descUrlAmigavel":"dup","infoServico":"x",
       "categoriaServico":[{"idCategoriaServico":21}]}
    ]"#;

    fn cat_map() -> HashMap<i64, String> {
        [(21i64, "Finanças e Impostos".to_string()), (3i64, "Agricultura e Pecuária".to_string())]
            .into_iter()
            .collect()
    }

    #[test]
    fn parse_e_dedup_por_id() {
        let items = parse_servicos(SVC_JSON).unwrap();
        assert_eq!(items.len(), 3, "3 no JSON (um é dup de id)");
        let (out, _) = build_servicos(items, &cat_map());
        assert_eq!(out.len(), 2, "dedup por idServico: 947 aparece uma vez");
    }

    #[test]
    fn ocorrencias_por_categoria_e_link_e_publico_unico() {
        let (out, orfaos) = build_servicos(parse_servicos(SVC_JSON).unwrap(), &cat_map());
        let s = &out[0];
        assert_eq!(s.titulo, "Agendar Atendimento");
        assert_eq!(s.link, "https://www.go.gov.br/servicos/servico/agendar-atendimento");
        assert_eq!(s.orgao, "SEFAZ-GO");
        // 2 categorias -> 2 ocorrências, todas sob o público único "Serviços".
        assert_eq!(s.ocorrencias.len(), 2);
        assert!(s.ocorrencias.iter().all(|o| o.publico == "Serviços"));
        let classes: Vec<&str> = s.ocorrencias.iter().map(|o| o.classe.as_str()).collect();
        assert_eq!(classes, vec!["Finanças e Impostos", "Agricultura e Pecuária"]);
        // O 2º item não tem categoria -> fallback "Geral".
        assert_eq!(out[1].ocorrencias[0].classe, GERAL);
        assert_eq!(orfaos, 1);
    }

    #[test]
    fn html_to_text_decodifica_entidades_e_separa_paragrafos() {
        // `&ccedil;` (fora da tabela do kit) decodificado; parágrafos NÃO fundem as palavras.
        let t = html_to_text("<p>Servi&ccedil;o de agendamento.</p><p>Segunda etapa.</p>");
        assert_eq!(t, "Serviço de agendamento. Segunda etapa.");
        assert_eq!(html_to_text("Sem tags, s&oacute; texto."), "Sem tags, só texto.");
    }

    #[test]
    fn total_orgao_acha_o_alvo() {
        let json = r#"[{"idPrestadorServico":13,"qtdeServicosPublicados":6},
                       {"idPrestadorServico":20,"qtdeServicosPublicados":94}]"#;
        assert_eq!(total_orgao(json, 20).unwrap(), 94);
        assert!(total_orgao(json, 99).is_err());
    }

    #[test]
    fn parse_categorias_mapeia_id_para_nome() {
        let json = r#"[{"idCategoriaServico":21,"nomeCategoriaServico":"Finanças e Impostos"},
                       {"idCategoriaServico":9,"nomeCategoriaServico":""}]"#;
        let m = parse_categorias(json).unwrap();
        assert_eq!(m.get(&21).map(String::as_str), Some("Finanças e Impostos"));
        assert!(!m.contains_key(&9), "categoria sem nome é descartada do mapa");
    }

    #[test]
    fn parse_token_extrai_access_token() {
        assert_eq!(parse_token(r#"{"access_token":"abc123","token_type":"Bearer"}"#).unwrap(), "abc123");
        assert!(parse_token(r#"{"error":"invalid_client"}"#).is_err());
    }

    #[test]
    fn validar_reprova_divergencia_e_minimo() {
        let svc = |i: usize| ServicoRaw {
            titulo: format!("s{i}"),
            descricao: String::new(),
            link: format!("l{i}"),
            orgao: ORGAO.into(),
            ocorrencias: vec![],
        };
        let n: Vec<ServicoRaw> = (0..94).map(svc).collect();
        assert!(validar(&n, 94).is_ok());
        assert!(validar(&n, 90).is_err(), "divergência do total anunciado reprova");
        let poucos: Vec<ServicoRaw> = (0..10).map(svc).collect();
        assert!(validar(&poucos, 10).is_err(), "abaixo do piso reprova");
    }
}
