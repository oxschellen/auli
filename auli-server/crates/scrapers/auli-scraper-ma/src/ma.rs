//! Coleta dos serviços da SEFAZ-MA a partir do Portal SGC (`portal-sgc.sefaz.ma.gov.br`).
//!
//! O portal é uma **SPA Angular** sobre uma **API REST Spring Boot** (`/sgc/api`). SEM headless:
//! - **Auth (anônima):** o front loga com **credenciais PÚBLICAS baked no bundle** (servidas a todo
//!   visitante) → `POST /sgc/api/login` body `{id_cliente, senha, portal:true}` → `{authtoken}`. O
//!   token (JWT efêmero) vai no header **`AuthorizationPortal`** (NÃO `Authorization`). Re-logamos a
//!   cada rodada.
//! - **Catálogo:** `GET /sgc/api/portal/servicos` com filtros obrigatórios (`flgPublicado=true`,
//!   `flgLocal=PORTAL`, `notOutros=false`, `page`/`pageSize`) → `{items, total}`. O `total` é o guard.
//! - **Descrição rica:** `GET /sgc/api/portal/conteudos/{idConteudo}` → `{descricao}` (HTML →
//!   `html_to_text`). 27/38 têm `idConteudo`; os 11 sem são link-only (`linkExterno`).
//!
//! **⚠️ Gotcha TLS:** o servidor manda a cadeia **incompleta** (só a folha; falta o intermediário
//! GlobalSign). curl/ureq/rustls rejeitam; o browser passa via AIA. Solução: embutir o intermediário
//! como **trust anchor** no rustls (`RootCerts::new_with_certs`) — não precisa de native-tls (o cipher
//! é moderno, TLS 1.3 AEAD). PEM embutido abaixo.
//!
//! Modelagem (padrão MT): `titulo`=`nomeServico`; **público**=`flgTipoServico` mapeado; `classe`=
//! "Geral" (o portal não usa categoria: `idServicoCategoria`=0 em todos); `link`=`linkExterno` ou a
//! página de conteúdo; identidade=`id`.

use std::collections::HashSet;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use auli_contract::{Ocorrencia, Publico, ServicoRaw};
use auli_scraper_kit::clean;
use auli_scraper_kit::http::GetOpts;
use scraper::Html;
use serde::Deserialize;
use ureq::Agent;
use ureq::tls::{Certificate, RootCerts, TlsConfig};

const USER_AGENT: &str =
    "AuliBot/0.1 (+https://github.com/oxschellen/auli; carlos.schellenberger@gmail.com)";

const FRONT_BASE: &str = "https://portal-sgc.sefaz.ma.gov.br";
const API_BASE: &str = "https://portal-sgc.sefaz.ma.gov.br/sgc/api";
/// Filtros obrigatórios do catálogo (sem eles → 500). `pageSize` alto traz o catálogo todo numa GET.
const SERVICOS_QUERY: &str = "nomeServico=&flgTipoServico=&flgDestaqueNovo=&flgPublicado=true\
    &flgPaginaPrincipal=&flgLocal=PORTAL&notOutros=false&page=0&pageSize=1000&sortOrder=&sortField=";

/// Credenciais do **cliente anônimo PÚBLICO** do portal (id + "senha" bcrypt), extraídas do bundle
/// Angular (`environment`) servido a QUALQUER visitante. **NÃO são segredo** — é o login anônimo que o
/// próprio front faz para navegar o catálogo público. (Comentário para scanners de segredo — lição GO.)
const ID_CLIENTE: &str = "41";
const SENHA_PUBLICA: &str = "$2a$12$jJ8IAazstO0PpE3exBAA9OBx9IrPs.jA8pjAhyQKZwjldpps/Y3Q2";

/// Órgão de origem.
const ORGAO: &str = "SEFAZ-MA";
/// Classe única (o portal não expõe categoria por serviço).
const CLASSE_GERAL: &str = "Geral";
/// Cortesia entre GETs (gentileza; D-PA-ROBOTS).
const COURTESY: Duration = Duration::from_millis(300);
/// Piso estático (o guard duro é `únicos == total`; ~38 hoje).
const MIN_SERVICOS: usize = 30;

/// Intermediário **GlobalSign GCC R3 DV TLS CA 2020** (emitido pela GlobalSign Root R3, que está no
/// store do sistema). O servidor da SEFAZ-MA NÃO o envia na cadeia — embutimos como trust anchor para
/// a verificação TLS fechar (ver header do módulo). Baixado do AIA do certificado do servidor.
const GLOBALSIGN_INTERMEDIATE_PEM: &str = "-----BEGIN CERTIFICATE-----
MIIEsDCCA5igAwIBAgIQd70OB0LV2enQSdd00CpvmjANBgkqhkiG9w0BAQsFADBM
MSAwHgYDVQQLExdHbG9iYWxTaWduIFJvb3QgQ0EgLSBSMzETMBEGA1UEChMKR2xv
YmFsU2lnbjETMBEGA1UEAxMKR2xvYmFsU2lnbjAeFw0yMDA3MjgwMDAwMDBaFw0y
OTAzMTgwMDAwMDBaMFMxCzAJBgNVBAYTAkJFMRkwFwYDVQQKExBHbG9iYWxTaWdu
IG52LXNhMSkwJwYDVQQDEyBHbG9iYWxTaWduIEdDQyBSMyBEViBUTFMgQ0EgMjAy
MDCCASIwDQYJKoZIhvcNAQEBBQADggEPADCCAQoCggEBAKxnlJV/de+OpwyvCXAJ
IcxPCqkFPh1lttW2oljS3oUqPKq8qX6m7K0OVKaKG3GXi4CJ4fHVUgZYE6HRdjqj
hhnuHY6EBCBegcUFgPG0scB12Wi8BHm9zKjWxo3Y2bwhO8Fvr8R42pW0eINc6OTb
QXC0VWFCMVzpcqgz6X49KMZowAMFV6XqtItcG0cMS//9dOJs4oBlpuqX9INxMTGp
6EASAF9cnlAGy/RXkVS9nOLCCa7pCYV+WgDKLTF+OK2Vxw3RUJ/p8009lQeUARv2
UCcNNPCifYX1xIspvarkdjzLwzOdLahDdQbJON58zN4V+lMj0msg+c0KnywPIRp3
BMkCAwEAAaOCAYUwggGBMA4GA1UdDwEB/wQEAwIBhjAdBgNVHSUEFjAUBggrBgEF
BQcDAQYIKwYBBQUHAwIwEgYDVR0TAQH/BAgwBgEB/wIBADAdBgNVHQ4EFgQUDZjA
c3+rvb3ZR0tJrQpKDKw+x3wwHwYDVR0jBBgwFoAUj/BLf6guRSSuTVD6Y5qL3uLd
G7wwewYIKwYBBQUHAQEEbzBtMC4GCCsGAQUFBzABhiJodHRwOi8vb2NzcDIuZ2xv
YmFsc2lnbi5jb20vcm9vdHIzMDsGCCsGAQUFBzAChi9odHRwOi8vc2VjdXJlLmds
b2JhbHNpZ24uY29tL2NhY2VydC9yb290LXIzLmNydDA2BgNVHR8ELzAtMCugKaAn
hiVodHRwOi8vY3JsLmdsb2JhbHNpZ24uY29tL3Jvb3QtcjMuY3JsMEcGA1UdIARA
MD4wPAYEVR0gADA0MDIGCCsGAQUFBwIBFiZodHRwczovL3d3dy5nbG9iYWxzaWdu
LmNvbS9yZXBvc2l0b3J5LzANBgkqhkiG9w0BAQsFAAOCAQEAy8j/c550ea86oCkf
r2W+ptTCYe6iVzvo7H0V1vUEADJOWelTv07Obf+YkEatdN1Jg09ctgSNv2h+LMTk
KRZdAXmsE3N5ve+z1Oa9kuiu7284LjeS09zHJQB4DJJJkvtIbjL/ylMK1fbMHhAW
i0O194TWvH3XWZGXZ6ByxTUIv1+kAIql/Mt29PmKraTT5jrzcVzQ5A9jw16yysuR
XRrLODlkS1hyBjsfyTNZrmL1h117IFgntBA5SQNVl9ckedq5r4RSAU85jV8XK5UL
REjRZt2I6M9Po9QL7guFLu4sPFJpwR1sPJvubS2THeo7SxYoNDtdyBHs7euaGcMa
D/fayQ==
-----END CERTIFICATE-----
";

/// Resposta do login.
#[derive(Debug, Deserialize)]
struct LoginResp {
    authtoken: String,
}

/// Resposta do catálogo.
#[derive(Debug, Deserialize)]
struct ServicosResp {
    #[serde(default)]
    items: Vec<Item>,
    #[serde(default)]
    total: i64,
}

/// Um item do catálogo (só o que usamos).
#[derive(Debug, Deserialize)]
struct Item {
    #[serde(default)]
    id: i64,
    #[serde(rename = "nomeServico", default)]
    nome: Option<String>,
    #[serde(rename = "flgTipoServico", default)]
    tipo: Option<String>,
    #[serde(rename = "idConteudo", default)]
    id_conteudo: Option<i64>,
    #[serde(rename = "linkExterno", default)]
    link_externo: Option<String>,
}

/// Resposta do conteúdo (descrição rica, HTML).
#[derive(Debug, Deserialize)]
struct Conteudo {
    #[serde(default)]
    descricao: Option<String>,
}

/// Raspa o portal e devolve `(items, publicos_ordem)` prontos para o snapshot v3.
pub fn scrape(
    data_dir: &str,
    use_cache: bool,
) -> Result<(Vec<ServicoRaw>, Vec<Publico>), Box<dyn std::error::Error>> {
    let agent = build_agent();
    let mut token: Option<String> = None; // login preguiçoso (só se houver fetch de rede).
    let mut pending: Vec<(String, String)> = Vec::new();

    // 1) Catálogo (chave de cache lógica e curta).
    let servicos_url = format!("{}/portal/servicos?{}", API_BASE, SERVICOS_QUERY);
    let lista_json = load(&agent, data_dir, &servicos_url, "servicos#todos", use_cache, &mut token, &mut pending)?;
    let resp: ServicosResp = serde_json::from_str(&lista_json)
        .map_err(|e| anyhow!("JSON do catálogo inválido: {}", e))?;
    println!("MA: {} itens (total={})", resp.items.len(), resp.total);

    // 2) Detalhe rico + montagem.
    let mut publicos_ordem: Vec<String> = Vec::new();
    let mut items: Vec<ServicoRaw> = Vec::new();
    let mut vistos: HashSet<i64> = HashSet::new();
    for it in &resp.items {
        if it.id == 0 || !vistos.insert(it.id) {
            continue;
        }
        let titulo = clean(it.nome.as_deref().unwrap_or_default());
        if titulo.is_empty() {
            continue;
        }
        let descricao = match it.id_conteudo {
            Some(idc) if idc > 0 => {
                let url = format!("{}/portal/conteudos/{}", API_BASE, idc);
                let cj = load(&agent, data_dir, &url, &url, use_cache, &mut token, &mut pending)?;
                let c: Conteudo = serde_json::from_str(&cj)
                    .map_err(|e| anyhow!("JSON do conteúdo {} inválido: {}", idc, e))?;
                html_to_text(c.descricao.as_deref().unwrap_or_default())
            }
            _ => String::new(),
        };

        let publico = map_publico(it.tipo.as_deref().unwrap_or_default());
        if !publicos_ordem.iter().any(|p| p == publico) {
            publicos_ordem.push(publico.to_string());
        }
        items.push(ServicoRaw {
            titulo,
            descricao,
            link: link(it),
            orgao: ORGAO.to_string(),
            ocorrencias: vec![Ocorrencia {
                publico: publico.to_string(),
                classe: CLASSE_GERAL.to_string(),
            }],
        });
    }

    validar(&items, resp.total)?;

    // Cache só DEPOIS dos guards.
    for (key, raw) in &pending {
        auli_scraper_kit::cache::write(data_dir, key, raw);
    }

    let ocorr: usize = items.iter().map(|s| s.ocorrencias.len()).sum();
    println!("MA: {} serviços ({} ocorrências) em {} público(s)", items.len(), ocorr, publicos_ordem.len());
    let publicos = publicos_ordem
        .into_iter()
        .map(|nome| Publico { slug: slug_publico(&nome), nome })
        .collect();
    Ok((items, publicos))
}

/// Agent `ureq` com o intermediário GlobalSign embutido como trust anchor (rustls padrão). Resolve a
/// cadeia incompleta do servidor sem desabilitar a verificação nem usar native-tls.
fn build_agent() -> Agent {
    let cert =
        Certificate::from_pem(GLOBALSIGN_INTERMEDIATE_PEM.as_bytes()).expect("intermediário PEM válido");
    let roots = RootCerts::new_with_certs(&[cert]);
    Agent::config_builder()
        .user_agent(USER_AGENT)
        .timeout_global(Some(Duration::from_secs(30)))
        .tls_config(TlsConfig::builder().root_certs(roots).build())
        .build()
        .into()
}

/// GET (JSON, UTF-8) com cache e o header `AuthorizationPortal`. Miss + `--usecache` = erro. Rede ->
/// login preguiçoso + `pending` + cortesia. `cache_key` é curto/lógico; `url` é o alvo real.
fn load(
    agent: &Agent,
    data_dir: &str,
    url: &str,
    cache_key: &str,
    use_cache: bool,
    token: &mut Option<String>,
    pending: &mut Vec<(String, String)>,
) -> Result<String> {
    if let Some(cached) = auli_scraper_kit::cache::read(data_dir, cache_key) {
        return Ok(cached);
    }
    if use_cache {
        bail!("cache vazio para {} (--usecache, sem rede). Rode uma coleta com rede primeiro.", cache_key);
    }
    if token.is_none() {
        *token = Some(login(agent)?);
    }
    let body = auli_scraper_kit::http::get_string(
        agent,
        url,
        &GetOpts {
            log_prefix: "MA",
            accept: Some("application/json"),
            headers: &[("AuthorizationPortal", token.as_ref().unwrap())],
            ..Default::default()
        },
    )?;
    if !body.trim_start().starts_with(['{', '[']) {
        bail!("resposta não-JSON de {} (erro/auth?): {:?}", url, body.chars().take(60).collect::<String>());
    }
    pending.push((cache_key.to_string(), body.clone()));
    sleep(COURTESY);
    Ok(body)
}

/// Login anônimo (credenciais públicas do bundle) → devolve o `authtoken` (já com "Bearer ").
fn login(agent: &Agent) -> Result<String> {
    println!("MA: login anônimo (cliente público {})", ID_CLIENTE);
    let body = serde_json::json!({ "id_cliente": ID_CLIENTE, "senha": SENHA_PUBLICA, "portal": true });
    let resp = auli_scraper_kit::http::post_json(
        agent,
        &format!("{}/login", API_BASE),
        &[],
        &body,
        &GetOpts { log_prefix: "MA", accept: Some("application/json"), ..Default::default() },
    )?;
    let parsed: LoginResp =
        serde_json::from_str(&resp).map_err(|e| anyhow!("JSON do login inválido: {}", e))?;
    if parsed.authtoken.trim().is_empty() {
        bail!("login sem authtoken");
    }
    Ok(parsed.authtoken)
}

/// Mapeia `flgTipoServico` para o nome de público exibido.
fn map_publico(flg: &str) -> &'static str {
    match flg {
        "CITIZEN" => "Cidadão",
        "COMPANY" => "Empresa",
        "PUBLIC_AGENCY" => "Órgão Público",
        "CERTIFICATE" => "Certidões",
        _ => "Serviços",
    }
}

/// Link canônico: `linkExterno` quando houver; senão a página de conteúdo; senão o catálogo.
fn link(it: &Item) -> String {
    if let Some(l) = it.link_externo.as_deref() {
        let l = l.trim();
        if !l.is_empty() {
            return l.to_string();
        }
    }
    match it.id_conteudo {
        Some(idc) if idc > 0 => format!("{}/portal/conteudo/{}", FRONT_BASE, idc),
        _ => format!("{}/portal/servicos", FRONT_BASE),
    }
}

/// HTML da `descricao` do conteúdo -> texto (tags viram espaço; html5ever decodifica entidades; clean).
fn html_to_text(html: &str) -> String {
    if html.trim().is_empty() {
        return String::new();
    }
    let mut spaced = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                spaced.push(' ');
            }
            _ if !in_tag => spaced.push(c),
            _ => {}
        }
    }
    let decoded: String = Html::parse_fragment(&spaced).root_element().text().collect();
    clean(&decoded)
}

fn slug_publico(nome: &str) -> String {
    format!("servicos-{}", slugify(nome))
}

fn slugify(s: &str) -> String {
    let mut buf = String::with_capacity(s.len());
    for c in s.chars() {
        let m = match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' | 'Á' | 'À' | 'Â' | 'Ã' | 'Ä' => 'a',
            'é' | 'ê' | 'è' | 'ë' | 'É' | 'Ê' | 'È' | 'Ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' => 'i',
            'ó' | 'ô' | 'õ' | 'ò' | 'ö' | 'Ó' | 'Ô' | 'Õ' | 'Ò' | 'Ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Ü' => 'u',
            'ç' | 'Ç' => 'c',
            c if c.is_ascii_alphanumeric() => c.to_ascii_lowercase(),
            _ => '-',
        };
        buf.push(m);
    }
    buf.split('-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("-")
}

/// Guard: invariante `únicos == total` (a API dá o próprio total) + piso estático.
fn validar(items: &[ServicoRaw], total: i64) -> Result<()> {
    let unicos = items.len();
    if total > 0 && (unicos as i64) < total {
        bail!(
            "coletados {} < total {} anunciado pela API — catálogo capado/paginação. Se veio do cache, \
             limpe data/ma/raw/cache/ e re-raspe.",
            unicos,
            total
        );
    }
    if unicos < MIN_SERVICOS {
        bail!("catálogo capado/vazio? só {} serviço(s) (mínimo {}).", unicos, MIN_SERVICOS);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: i64, nome: &str, tipo: &str, idc: Option<i64>, link: Option<&str>) -> Item {
        Item {
            id,
            nome: Some(nome.into()),
            tipo: Some(tipo.into()),
            id_conteudo: idc,
            link_externo: link.map(|s| s.into()),
        }
    }

    #[test]
    fn map_publico_mapeia_tipos() {
        assert_eq!(map_publico("CITIZEN"), "Cidadão");
        assert_eq!(map_publico("COMPANY"), "Empresa");
        assert_eq!(map_publico("PUBLIC_AGENCY"), "Órgão Público");
        assert_eq!(map_publico("CERTIFICATE"), "Certidões");
        assert_eq!(map_publico("OUTRO"), "Serviços");
    }

    #[test]
    fn link_prioriza_externo_depois_conteudo_depois_catalogo() {
        assert_eq!(link(&item(1, "x", "COMPANY", Some(9), Some("https://ext/y"))), "https://ext/y");
        assert_eq!(
            link(&item(2, "x", "COMPANY", Some(3171), None)),
            "https://portal-sgc.sefaz.ma.gov.br/portal/conteudo/3171"
        );
        assert_eq!(link(&item(3, "x", "COMPANY", None, Some("  "))), "https://portal-sgc.sefaz.ma.gov.br/portal/servicos");
    }

    #[test]
    fn html_to_text_decodifica_e_remove_tags() {
        // Tags viram espaço (fronteira de tag), entidades são decodificadas, espaços comprimidos.
        let t = html_to_text("<h2>Ouvidoria</h2><p>Baseada nos princ&iacute;pios <b>constitucionais</b>.</p>");
        assert!(t.starts_with("Ouvidoria Baseada nos princípios constitucionais"));
        assert!(!t.contains('<') && !t.contains("&iacute;"));
        assert!(!t.contains("  "), "sem espaços duplos: {t:?}");
        assert_eq!(html_to_text(""), "");
    }

    #[test]
    fn parse_login_e_catalogo() {
        let l: LoginResp = serde_json::from_str(r#"{"authtoken":"Bearer eyJ.x.y","refreshtoken":"r"}"#).unwrap();
        assert_eq!(l.authtoken, "Bearer eyJ.x.y");
        let s: ServicosResp = serde_json::from_str(
            r#"{"items":[{"id":485,"nomeServico":"SEFAZNET","flgTipoServico":"COMPANY","idConteudo":null,"linkExterno":"https://e"}],"total":38}"#,
        ).unwrap();
        assert_eq!(s.total, 38);
        assert_eq!(s.items[0].id, 485);
        assert_eq!(s.items[0].tipo.as_deref(), Some("COMPANY"));
    }

    #[test]
    fn agent_com_intermediario_constroi() {
        // o PEM embutido é válido (não entra em pânico).
        let _ = build_agent();
    }

    #[test]
    fn validar_reprova_subcobertura_e_capado() {
        let dummy = |i: usize| ServicoRaw {
            titulo: format!("s{i}"),
            descricao: String::new(),
            link: format!("l{i}"),
            orgao: ORGAO.into(),
            ocorrencias: vec![],
        };
        let items: Vec<ServicoRaw> = (0..MIN_SERVICOS).map(dummy).collect();
        assert!(validar(&items, MIN_SERVICOS as i64 + 5).unwrap_err().to_string().contains("capad"));
        assert!(validar(&items[..1], 0).unwrap_err().to_string().contains("capado"));
        assert!(validar(&items, MIN_SERVICOS as i64).is_ok());
    }
}
