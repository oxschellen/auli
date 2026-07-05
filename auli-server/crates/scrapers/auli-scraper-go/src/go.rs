//! Coleta dos serviços da SEFAZ-GO (Secretaria de Estado da Economia de Goiás).
//!
//! Fonte (D-GO1): API do **Portal Expresso** (WSO2 API Manager), `api.go.gov.br/expresso/2.0.0/`,
//! escopo do órgão **Economia (id 20)**. Auth (D-GO3): **client_credentials anônimo** — as
//! credenciais são de um cliente público, extraídas do bundle Angular servido a qualquer visitante
//! (não são segredo); token efêmero, re-obtido a cada run (padrão CE).
//!
//! **WAF / JA3 (D-GO-WAF).** `api.go.gov.br` faz allowlist por fingerprint TLS: o ClientHello do
//! `ureq` (rustls E native-tls) difere do curl nas **extensões** (falta ALPN, sobra
//! `session_ticket`), e o `TlsConfig` do ureq 3 não expõe ALPN/cipher-list para alinhar (medido no
//! spike; native-tls do BA não resolve). Por isso os GETs de catálogo usam
//! `kit::http::get_via_curl` (subprocess curl — exceção documentada, coleta pública e de baixa
//! frequência, ~3 requests/rodada). O **token** sai pelo `ureq` normal: o host de SSO
//! (`sso.go.gov.br`) **não** tem o WAF.
//!
//! Modelagem: público único "Serviços" (Cenário A — o dado não traz eixo de público, D-GO4);
//! `classe` = nome da categoria (o `categoriaServico[]` do serviço só traz `idCategoriaServico`; o
//! nome legível vem do endpoint `/categorias` — multi por serviço → uma `Ocorrencia` por categoria;
//! sem categoria → "Geral", D-GO2). Identidade = `idServico` (estável, único); `link` canônico
//! `…/servicos/servico/<descUrlAmigavel>` cru (braille e tudo, D-GO5). `descricao` = `infoServico`
//! (HTML inline) limpo. `orgao` = "SEFAZ-GO" (D-GO2).
//!
//! Invariante (D-GO): `únicos == qtdeServicosPublicados(órgão 20)` do `/orgaos` (dinâmico) + piso
//! estático. Cache pós-guards (D-RJ5).

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use auli_contract::{Ocorrencia, Publico, ServicoRaw};
use auli_scraper_kit::http::GetOpts;
use scraper::Html;
use serde::Deserialize;

const TOKEN_URL: &str = "https://sso.go.gov.br/oauth2/token";
const API_BASE: &str = "https://api.go.gov.br/expresso/2.0.0";
/// Órgão Economia (SEFAZ-GO) no Expresso.
const ORGAO_ID: u32 = 20;
/// Prefixo do link canônico de detalhe (Portal Expresso).
const LINK_BASE: &str = "https://www.go.gov.br/servicos/servico";

/// Credenciais do cliente **público/anônimo** do Expresso, extraídas do bundle Angular
/// (`www.go.gov.br/main.<hash>.js`) servido a qualquer visitante — **não são segredo**. Se
/// rotacionarem (401 no token), re-extrair do bundle: `grep 'client_secret\|client_pass'`.
const CLIENT_SECRET: &str = "jMQoyH_T2GpWXwBlH6goWfBBdr0a";
const CLIENT_PASS: &str = "k8BOsIHTF6sARfHq4qBPsvaYjf4a";

/// Público único (Cenário A).
const PUBLICO_NOME: &str = "Serviços";
const PUBLICO_SLUG: &str = "servicos-gerais";
/// Classe de fallback para serviço sem categoria (D-GO2).
const CLASSE_GERAL: &str = "Geral";
const ORGAO: &str = "SEFAZ-GO";
/// Piso estático de folga (o invariante dinâmico é o principal; 94 observados na Fase 0).
const MIN_SERVICOS: usize = 70;

/// Cache lógico: uma "URL" por recurso (o token não entra; padrão da frota).
fn cache_key(recurso: &str) -> String {
    format!("{}/{}", API_BASE, recurso)
}

// ---- Shapes da API (só os campos usados; serde ignora o resto) ----

#[derive(Debug, Deserialize)]
struct TokenResp {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct Servico {
    #[serde(rename = "idServico")]
    id_servico: i64,
    #[serde(rename = "nomeServico", default)]
    nome_servico: String,
    #[serde(rename = "descUrlAmigavel", default)]
    desc_url_amigavel: String,
    #[serde(rename = "infoServico", default)]
    info_servico: String,
    /// No `servicosOrgaos` cada categoria só traz o `idServico`->`idCategoriaServico` (o nome vem do
    /// `/categorias`).
    #[serde(rename = "categoriaServico", default)]
    categoria_servico: Vec<CatRef>,
}

#[derive(Debug, Deserialize)]
struct CatRef {
    #[serde(rename = "idCategoriaServico")]
    id: i64,
}

/// Uma categoria do endpoint `/categorias` (id -> nome legível).
#[derive(Debug, Deserialize)]
struct CategoriaDef {
    #[serde(rename = "idCategoriaServico")]
    id: i64,
    #[serde(rename = "nomeCategoriaServico", default)]
    nome: String,
}

#[derive(Debug, Deserialize)]
struct Orgao {
    #[serde(rename = "idPrestadorServico")]
    id: u32,
    #[serde(rename = "qtdeServicosPublicados", default)]
    qtde: i64,
}

/// Raspa o catálogo e devolve `(items, publicos_ordem)` prontos para o snapshot v3.
pub fn scrape(
    data_dir: &str,
    use_cache: bool,
) -> Result<(Vec<ServicoRaw>, Vec<Publico>), Box<dyn std::error::Error>> {
    // Páginas cruas (json) na ordem — cache só grava DEPOIS dos guards (D-RJ5). Token compartilhado
    // (buscado uma vez, no 1º miss de rede).
    let mut raw: Vec<(String, String)> = Vec::new();
    let mut token: Option<String> = None;

    let servicos_json =
        fetch_recurso(data_dir, use_cache, &format!("servicosOrgaos/{}", ORGAO_ID), &mut token, &mut raw)?;
    let orgaos_json = fetch_recurso(data_dir, use_cache, "orgaos", &mut token, &mut raw)?;
    let cats_json = fetch_recurso(data_dir, use_cache, "categorias", &mut token, &mut raw)?;

    let servicos: Vec<Servico> =
        serde_json::from_str(&servicos_json).map_err(|e| anyhow!("JSON de servicosOrgaos inválido: {e}"))?;
    let orgaos: Vec<Orgao> =
        serde_json::from_str(&orgaos_json).map_err(|e| anyhow!("JSON de orgaos inválido: {e}"))?;
    let cats: Vec<CategoriaDef> =
        serde_json::from_str(&cats_json).map_err(|e| anyhow!("JSON de categorias inválido: {e}"))?;

    let cat_map: HashMap<i64, String> = cats
        .into_iter()
        .filter(|c| !c.nome.trim().is_empty())
        .map(|c| (c.id, auli_scraper_kit::clean(&c.nome)))
        .collect();

    let total_orgao = orgaos
        .iter()
        .find(|o| o.id == ORGAO_ID)
        .map(|o| o.qtde as usize)
        .ok_or_else(|| anyhow!("órgão {} não encontrado em /orgaos", ORGAO_ID))?;

    let (items, orfaos) = build_servicos(&servicos, &cat_map);
    if orfaos > 0 {
        eprintln!("⚠️  GO: {} serviço(s) sem categoria -> classe '{}'.", orfaos, CLASSE_GERAL);
    }

    validar(&items, total_orgao)?;

    // Cache só depois dos guards (D-RJ5).
    for (logical, json) in &raw {
        auli_scraper_kit::cache::write(data_dir, logical, json);
    }

    let ocorrencias: usize = items.iter().map(|s| s.ocorrencias.len()).sum();
    println!("GO: {} serviços ({} ocorrências); portal anuncia {}", items.len(), ocorrencias, total_orgao);

    let publicos_ordem = vec![Publico { nome: PUBLICO_NOME.into(), slug: PUBLICO_SLUG.into() }];
    Ok((items, publicos_ordem))
}

/// Busca (ou lê do cache) um recurso da API. Token via `ureq` (SSO sem WAF), compartilhado; o GET do
/// recurso via **curl** (WAF/JA3 — D-GO-WAF). Acumula o json cru em `raw` para o cache pós-guards.
fn fetch_recurso(
    data_dir: &str,
    use_cache: bool,
    recurso: &str,
    token: &mut Option<String>,
    raw: &mut Vec<(String, String)>,
) -> Result<String> {
    let logical = cache_key(recurso);
    if let Some(cached) = auli_scraper_kit::cache::read(data_dir, &logical) {
        println!("Cache hit: {}", logical);
        return Ok(cached);
    }
    if use_cache {
        bail!("cache miss para {} (modo --usecache, sem rede)", logical);
    }

    if token.is_none() {
        *token = Some(fetch_token()?);
    }
    let bearer = format!("Bearer {}", token.as_ref().unwrap());
    let url = format!("{}/{}", API_BASE, recurso);
    // GET via curl: o ureq é barrado pelo WAF (D-GO-WAF). Bearer + Accept nos headers do GetOpts.
    let body = auli_scraper_kit::http::get_via_curl(
        &url,
        &GetOpts {
            log_prefix: "GO",
            accept: Some("application/json"),
            headers: &[("Authorization", &bearer)],
            ..Default::default()
        },
    )?;
    // Defesa extra: se algo não-JSON escapar (ex.: "Acesso Negado" 200 que o curl não pegou como
    // erro), falhar aqui com contexto em vez de propagar um serde-error cru lá em cima.
    if !body.trim_start().starts_with(['[', '{']) {
        let head: String = body.chars().take(200).collect();
        bail!("resposta não-JSON de {} (WAF?): {}", url, head);
    }
    raw.push((logical, body.clone()));
    Ok(body)
}

/// Obtém o token client_credentials anônimo. Via **ureq** (o host de SSO não tem WAF): é um POST
/// `x-www-form-urlencoded` (não JSON), então não usa `kit::http::post_json`; monta o agent do kit.
fn fetch_token() -> Result<String> {
    let agent = auli_scraper_kit::build_agent(auli_scraper_kit::USER_AGENT, Some(Duration::from_secs(30)));
    let basic = basic_auth(CLIENT_SECRET, CLIENT_PASS);

    println!("POST token (SSO, sem WAF)");
    let mut last = anyhow!("sem tentativa");
    let mut delay = Duration::from_millis(800);
    for attempt in 1..=3 {
        let sent = agent
            .post(TOKEN_URL)
            .header("Authorization", &basic)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Accept", "application/json")
            .send("grant_type=client_credentials");
        match sent {
            Ok(mut resp) => match resp.body_mut().read_to_string() {
                Ok(s) if !s.trim().is_empty() => {
                    let t: TokenResp =
                        serde_json::from_str(&s).map_err(|e| anyhow!("JSON do token inválido: {e}"))?;
                    if t.access_token.is_empty() {
                        bail!("token vazio na resposta do SSO");
                    }
                    return Ok(t.access_token);
                }
                Ok(_) => last = anyhow!("resposta vazia"),
                Err(e) => last = anyhow!(e.to_string()),
            },
            Err(e) => last = anyhow!(e.to_string()),
        }
        if attempt < 3 {
            eprintln!("⚠️  GO: token tentativa {} falhou ({}); retentando…", attempt, last);
            std::thread::sleep(delay);
            delay *= 2;
        }
    }
    Err(anyhow!("falha ao obter token após 3 tentativas: {}", last))
}

/// `Basic base64(user:pass)` sem dependência extra (base64 padrão, alfabeto RFC 4648).
fn basic_auth(user: &str, pass: &str) -> String {
    let creds = format!("{}:{}", user, pass);
    format!("Basic {}", b64(creds.as_bytes()))
}

/// Base64 padrão (com padding). Pequeno o bastante para não puxar um crate.
fn b64(input: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | (b[2] as u32);
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 { T[((n >> 6) & 63) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { T[(n & 63) as usize] as char } else { '=' });
    }
    out
}

/// Monta os `ServicoRaw` a partir do array da API, deduplicando por `idServico`. `classe` = nome da
/// categoria (do `cat_map`, pelo `idCategoriaServico`); `ocorrencias` = uma por categoria (todas sob
/// o público único "Serviços"); sem categoria conhecida -> "Geral". Retorna `(items, nº de órfãos)`.
fn build_servicos(servicos: &[Servico], cat_map: &HashMap<i64, String>) -> (Vec<ServicoRaw>, usize) {
    let mut vistos: HashSet<i64> = HashSet::new();
    let mut out: Vec<ServicoRaw> = Vec::new();
    let mut orfaos = 0usize;

    for s in servicos {
        if !vistos.insert(s.id_servico) {
            continue;
        }
        let titulo = auli_scraper_kit::clean(&s.nome_servico);
        if titulo.is_empty() {
            continue;
        }
        let mut classes: Vec<String> =
            s.categoria_servico.iter().filter_map(|c| cat_map.get(&c.id).cloned()).collect();
        classes.dedup();
        if classes.is_empty() {
            classes.push(CLASSE_GERAL.to_string());
            orfaos += 1;
        }
        let ocorrencias = classes
            .into_iter()
            .map(|classe| Ocorrencia { publico: PUBLICO_NOME.into(), classe })
            .collect();

        out.push(ServicoRaw {
            titulo,
            descricao: html_to_text(&s.info_servico),
            link: format!("{}/{}", LINK_BASE, s.desc_url_amigavel),
            orgao: ORGAO.to_string(),
            ocorrencias,
        });
    }
    (out, orfaos)
}

/// `infoServico` é HTML inline. Tags viram espaço (separa parágrafos), depois o parser html5ever
/// decodifica TODAS as entidades (`&ccedil;`, `&atilde;`, … — fora da tabela fixa do kit), e o
/// `kit::clean` comprime espaços.
fn html_to_text(html: &str) -> String {
    let mut spaced = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                spaced.push(' '); // fronteira de tag vira espaço (evita colar palavras)
            }
            _ if !in_tag => spaced.push(c),
            _ => {}
        }
    }
    let decoded: String = Html::parse_fragment(&spaced).root_element().text().collect();
    auli_scraper_kit::clean(&decoded)
}

/// Guards (D-GO): invariante dinâmico `únicos == total do órgão` + piso estático.
fn validar(items: &[ServicoRaw], total_orgao: usize) -> Result<()> {
    let unicos = items.len();
    if unicos != total_orgao {
        bail!(
            "catálogo incompleto/divergente: /orgaos anuncia {} para o órgão {} e coletamos {} \
             único(s). Se veio do cache, limpe data/go/raw/cache/ e re-raspe.",
            total_orgao,
            ORGAO_ID,
            unicos
        );
    }
    if unicos < MIN_SERVICOS {
        bail!("catálogo capado? só {} serviço(s) (mínimo {}).", unicos, MIN_SERVICOS);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const JSON: &str = r#"[
      {"idServico":101,"nomeServico":"Emitir DARE","descUrlAmigavel":"emitir-dare-⠳-difal",
       "infoServico":"<p>Emitir <b>DARE</b>&nbsp;online. Servi&ccedil;o r&aacute;pido.</p>",
       "categoriaServico":[{"idCategoriaServico":21},{"idCategoriaServico":30}]},
      {"idServico":102,"nomeServico":"Consultar débitos","descUrlAmigavel":"consultar-debitos",
       "infoServico":"Consulta simples","categoriaServico":[]},
      {"idServico":101,"nomeServico":"Emitir DARE (dup)","descUrlAmigavel":"x","infoServico":"","categoriaServico":[{"idCategoriaServico":21}]}
    ]"#;

    fn cat_map() -> HashMap<i64, String> {
        [(21i64, "Finanças e Impostos".to_string()), (30i64, "Veículos".to_string())]
            .into_iter()
            .collect()
    }

    #[test]
    fn parse_e_dedup_por_id() {
        let v: Vec<Servico> = serde_json::from_str(JSON).unwrap();
        let (items, orfaos) = build_servicos(&v, &cat_map());
        assert_eq!(items.len(), 2, "idServico 101 repetido entra uma vez");
        assert_eq!(orfaos, 1, "o 102 não tem categoria");
    }

    #[test]
    fn ocorrencias_multi_categoria_sob_publico_unico() {
        let v: Vec<Servico> = serde_json::from_str(JSON).unwrap();
        let (items, _) = build_servicos(&v, &cat_map());
        assert_eq!(items[0].ocorrencias.len(), 2);
        assert!(items[0].ocorrencias.iter().all(|o| o.publico == "Serviços"));
        assert_eq!(items[0].ocorrencias[0].classe, "Finanças e Impostos");
        assert_eq!(items[0].ocorrencias[1].classe, "Veículos");
    }

    #[test]
    fn fallback_geral_para_sem_categoria() {
        let v: Vec<Servico> = serde_json::from_str(JSON).unwrap();
        let (items, _) = build_servicos(&v, &cat_map());
        let s102 = items.iter().find(|s| s.link.ends_with("consultar-debitos")).unwrap();
        assert_eq!(s102.ocorrencias.len(), 1);
        assert_eq!(s102.ocorrencias[0].classe, "Geral");
    }

    #[test]
    fn categoria_desconhecida_no_mapa_vira_geral() {
        // Serviço com id de categoria que não está no /categorias -> sem classe -> "Geral".
        let json = r#"[{"idServico":1,"nomeServico":"X","descUrlAmigavel":"x","infoServico":"y","categoriaServico":[{"idCategoriaServico":999}]}]"#;
        let v: Vec<Servico> = serde_json::from_str(json).unwrap();
        let (items, orfaos) = build_servicos(&v, &cat_map());
        assert_eq!(items[0].ocorrencias[0].classe, "Geral");
        assert_eq!(orfaos, 1);
    }

    #[test]
    fn link_preserva_slug_cru_com_braille() {
        let v: Vec<Servico> = serde_json::from_str(JSON).unwrap();
        let (items, _) = build_servicos(&v, &cat_map());
        assert_eq!(items[0].link, "https://www.go.gov.br/servicos/servico/emitir-dare-⠳-difal");
    }

    #[test]
    fn html_to_text_decodifica_entidades_ricas_e_separa_paragrafos() {
        // `&ccedil;`/`&aacute;` (fora da tabela do kit) decodificados via html5ever; nbsp e squeeze.
        assert_eq!(
            html_to_text("<p>Emitir <b>DARE</b>&nbsp;online. Servi&ccedil;o r&aacute;pido.</p>"),
            "Emitir DARE online. Serviço rápido."
        );
        assert_eq!(html_to_text("Sem tags &amp; com entidade"), "Sem tags & com entidade");
    }

    #[test]
    fn b64_confere() {
        assert_eq!(b64(b"user:pass"), "dXNlcjpwYXNz");
        assert_eq!(b64(b"M"), "TQ==");
        assert_eq!(b64(b"Ma"), "TWE=");
    }

    #[test]
    fn validar_invariante_e_piso() {
        let svc = |id: &str| ServicoRaw {
            titulo: "t".into(), descricao: String::new(), link: id.into(),
            orgao: ORGAO.into(), ocorrencias: vec![],
        };
        let dois = vec![svc("a"), svc("b")];
        assert!(validar(&dois, 5).unwrap_err().to_string().contains('5'), "divergência do total");
        assert!(validar(&dois, 2).unwrap_err().to_string().contains("capado"), "piso estático");
    }
}
