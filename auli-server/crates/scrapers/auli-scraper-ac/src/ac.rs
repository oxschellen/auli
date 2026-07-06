//! Coleta dos serviços da SEFAZ-AC a partir da Carta de Serviços (WordPress + Elementor).
//!
//! O portal é WordPress/Elementor, HTML server-rendered (`wp-json` = 404). A **Carta de Serviços**
//! (`?page_id=6732`) lista os serviços em cards agrupados por categoria; cada card aponta para um
//! **post** (`?p=NNNNN`) com a descrição rica. O scraper: parseia a Carta → (post, título, categoria)
//! → busca cada post → extrai o corpo do serviço do container `.elementor-widget-theme-post-content`.
//!
//! **⚠️ Gotcha TLS:** o servidor envia o intermediário ERRADO (Sectigo RSA OV antigo) faltando o
//! **R36** (emissor real do leaf) → nem o store do sistema nem o Mozilla/rustls fecham a cadeia. Fix:
//! embutir o R36 como trust anchor no rustls (`RootCerts::new_with_certs`), como o MA. PEM abaixo.
//!
//! Modelagem (Cenário A): `titulo` = card da Carta; `descricao` = corpo do post; `classe` = categoria;
//! público único "Serviços"; `link` = `…/?p={post}`; identidade = o post.

use std::collections::HashSet;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Result, bail};
use auli_contract::{Ocorrencia, Publico, ServicoRaw};
use auli_scraper_kit::clean;
use auli_scraper_kit::http::GetOpts;
use regex::Regex;
use scraper::{Html, Selector};
use ureq::Agent;
use ureq::tls::{Certificate, RootCerts, TlsConfig};

const USER_AGENT: &str =
    "AuliBot/0.1 (+https://github.com/oxschellen/auli; carlos.schellenberger@gmail.com)";

const BASE: &str = "https://sefaz.ac.gov.br/2021";
const CARTA_URL: &str = "https://sefaz.ac.gov.br/2021/?page_id=6732";

/// Público único (não há eixo de audiência).
const PUBLICO_NOME: &str = "Serviços";
const PUBLICO_SLUG: &str = "servicos-gerais";
const ORGAO: &str = "SEFAZ-AC";
const CLASSE_FALLBACK: &str = "Geral";
/// Cortesia entre GETs (17 posts + a Carta).
const COURTESY: Duration = Duration::from_millis(300);
/// Guard: piso de serviços (a Carta tem 17).
const MIN_SERVICOS: usize = 15;

/// Intermediário **Sectigo Public Server Authentication CA OV R36** (emissor real do leaf; o servidor
/// manda o intermediário errado). Embutido como trust anchor no rustls (ver header). Baixado do AIA.
const SECTIGO_R36_PEM: &str = "-----BEGIN CERTIFICATE-----
MIIGTDCCBDSgAwIBAgIQLBo8dulD3d3/GRsxiQrtcTANBgkqhkiG9w0BAQwFADBf
MQswCQYDVQQGEwJHQjEYMBYGA1UEChMPU2VjdGlnbyBMaW1pdGVkMTYwNAYDVQQD
Ey1TZWN0aWdvIFB1YmxpYyBTZXJ2ZXIgQXV0aGVudGljYXRpb24gUm9vdCBSNDYw
HhcNMjEwMzIyMDAwMDAwWhcNMzYwMzIxMjM1OTU5WjBgMQswCQYDVQQGEwJHQjEY
MBYGA1UEChMPU2VjdGlnbyBMaW1pdGVkMTcwNQYDVQQDEy5TZWN0aWdvIFB1Ymxp
YyBTZXJ2ZXIgQXV0aGVudGljYXRpb24gQ0EgT1YgUjM2MIIBojANBgkqhkiG9w0B
AQEFAAOCAY8AMIIBigKCAYEApkMtJ3R06jo0fceI0M52B7K+TyMeGcv2BQ5AVc3j
lYt76TvHIu/nNe22W/RJXX9rWUD/2GE6GF5x0V4bsY7K3IeJ8E7+KzG/TGboySfD
u+F52jqQBbY62ofhYjMeiAbLI02+FqwHeM8uIrUtcX8b2RCxF358TB0NHVccAXZc
FYgZndZCeXxjuca7pJJ20LLUnXtgXcjAE1vY4WvbReW0W6mkeZyNGdmpTcFs5Y+s
yy6LtE5Zocji9J9NlNnReox2RWVyEXpA1ChZ4gqN+ZpVSIQ0HBorVFbBKyhdZyEX
gZgNSNtBRwxqwIzJePJhYd4ZUhO1vk+/uP3nwDk0p95q/j7naXNCSvESnrHPypaB
WRK066nKfPRPi9m9kIOhMdYfS8giFRTcdgL24Ycilj7ecAK9Trh0VbjwouJ4WH+x
bt47u68ZFCD/ac55I0DNHkCpaPruj6e9Rmr7K46wZDAYXuEAqB7tGG/jd6JAA+H2
O44CV98NRsU213f1kScIZntNAgMBAAGjggGBMIIBfTAfBgNVHSMEGDAWgBRWc1hk
lfmSGrASKgRieaFAFYghSTAdBgNVHQ4EFgQU42Z0u3BojSxdTg6mSo+bNyKcgpIw
DgYDVR0PAQH/BAQDAgGGMBIGA1UdEwEB/wQIMAYBAf8CAQAwHQYDVR0lBBYwFAYI
KwYBBQUHAwEGCCsGAQUFBwMCMBsGA1UdIAQUMBIwBgYEVR0gADAIBgZngQwBAgIw
VAYDVR0fBE0wSzBJoEegRYZDaHR0cDovL2NybC5zZWN0aWdvLmNvbS9TZWN0aWdv
UHVibGljU2VydmVyQXV0aGVudGljYXRpb25Sb290UjQ2LmNybDCBhAYIKwYBBQUH
AQEEeDB2ME8GCCsGAQUFBzAChkNodHRwOi8vY3J0LnNlY3RpZ28uY29tL1NlY3Rp
Z29QdWJsaWNTZXJ2ZXJBdXRoZW50aWNhdGlvblJvb3RSNDYucDdjMCMGCCsGAQUF
BzABhhdodHRwOi8vb2NzcC5zZWN0aWdvLmNvbTANBgkqhkiG9w0BAQwFAAOCAgEA
BZXWDHWC3cubb/e1I1kzi8lPFiK/ZUoH09ufmVOrc5ObYH/XKkWUexSPqRkwKFKr
7r8OuG+p7VNB8rifX6uopqKAgsvZtZsq7iAFw04To6vNcxeBt1Eush3cQ4b8nbQR
MQLChgEAqwhuXp9P48T4QEBSksYav7+aFjNySsLYlPzNqVM3RNwvBdvp6vgDtGwc
xlKQZVuuNVIaoYyls8swhxDeSHKpRdxRauTLZ+pl+wGvy0pnrLEJGSz9mOEmfbod
e/XopR2NGqaHJ6bIjyxPu6UtyQGI26En7UAEozACrHz06Nx2jTAY9E6NeB6XuobE
wLK025ZRmvglcURG1BrV24tGHHTgxCe8M3oGlpUSMTKQ2dkgljZVYt+gKdFtWELZ
MuRdi+X3XsrR8LFz+aLUiDRfQqhmw3RxjIyVKvvu9UPYY1nsvxYmFnUSeM+2q1z/
iPUry+xDY9MC6+IhleKT094VKdFVp7LXH42+wvU+17lRolQ2mK2N/nBLVBwaIhib
QXw4VYKwB86Bc6eS6iqsc94KEgD/U4VsjmgfhK+Xp4NM+VYzTTa3QeV3p8xOM0cw
q1p8oZFA+OBcz3FYWpDIe5j0NWKlw9hXsTyPY/HeZUV59akskSOSRSmDfe8wJDPX
58uB9/7lud0G3x0pxQAcffP0ayKavNwDTw4UfJ34cEw=
-----END CERTIFICATE-----
";

/// Um serviço da Carta (antes de buscar o corpo).
struct Card {
    post_id: String,
    titulo: String,
    classe: String,
}

/// Raspa a Carta e devolve `(items, publicos_ordem)` prontos para o snapshot v3.
pub fn scrape(
    data_dir: &str,
    use_cache: bool,
) -> Result<(Vec<ServicoRaw>, Vec<Publico>), Box<dyn std::error::Error>> {
    let agent = build_agent();
    let mut pending: Vec<(String, String)> = Vec::new();

    // 1) A Carta -> cards (post, título, categoria).
    let carta = load(&agent, data_dir, CARTA_URL, use_cache, &mut pending)?;
    let cards = parse_carta(&carta);
    println!("AC: {} serviços na Carta", cards.len());

    // 2) Corpo de cada post.
    let mut items: Vec<ServicoRaw> = Vec::new();
    let mut vistos: HashSet<String> = HashSet::new();
    for c in &cards {
        if !vistos.insert(c.post_id.clone()) {
            continue;
        }
        let link = format!("{}/?p={}", BASE, c.post_id);
        let post = load(&agent, data_dir, &link, use_cache, &mut pending)?;
        items.push(ServicoRaw {
            titulo: c.titulo.clone(),
            descricao: extract_descricao(&post),
            link,
            orgao: ORGAO.to_string(),
            ocorrencias: vec![Ocorrencia {
                publico: PUBLICO_NOME.to_string(),
                classe: c.classe.clone(),
            }],
        });
    }

    validar(&items)?;

    for (url, raw) in &pending {
        auli_scraper_kit::cache::write(data_dir, url, raw);
    }

    let classes: HashSet<&str> = items.iter().flat_map(|s| s.ocorrencias.iter()).map(|o| o.classe.as_str()).collect();
    println!("AC: {} serviços em {} classe(s)", items.len(), classes.len());
    let publicos_ordem =
        vec![Publico { nome: PUBLICO_NOME.to_string(), slug: PUBLICO_SLUG.to_string() }];
    Ok((items, publicos_ordem))
}

/// Agent `ureq` com o intermediário Sectigo R36 embutido como trust anchor (rustls). Resolve a cadeia
/// quebrada do servidor sem desabilitar a verificação.
fn build_agent() -> Agent {
    let cert = Certificate::from_pem(SECTIGO_R36_PEM.as_bytes()).expect("R36 PEM válido");
    let roots = RootCerts::new_with_certs(&[cert]);
    Agent::config_builder()
        .user_agent(USER_AGENT)
        .timeout_global(Some(Duration::from_secs(30)))
        .tls_config(TlsConfig::builder().root_certs(roots).build())
        .build()
        .into()
}

/// GET (HTML) com cache. Miss + `--usecache` = erro. Rede -> `pending` + cortesia.
fn load(
    agent: &Agent,
    data_dir: &str,
    url: &str,
    use_cache: bool,
    pending: &mut Vec<(String, String)>,
) -> Result<String> {
    if let Some(cached) = auli_scraper_kit::cache::read(data_dir, url) {
        return Ok(cached);
    }
    if use_cache {
        bail!("cache vazio para {} (--usecache, sem rede). Rode uma coleta com rede primeiro.", url);
    }
    let body = auli_scraper_kit::http::get_string(
        agent,
        url,
        &GetOpts { log_prefix: "AC", ..Default::default() },
    )?;
    if !body.contains("elementor") {
        bail!("HTML inesperado de {} (não é Elementor / markup mudou?)", url);
    }
    pending.push((url.to_string(), body.clone()));
    sleep(COURTESY);
    Ok(body)
}

/// Parseia a Carta: cards `?p=NNNNN` na seção "Lista de Serviços", categoria pelo heading anterior.
fn parse_carta(html: &str) -> Vec<Card> {
    let start = html.find("Lista de Serviços").unwrap_or(0);
    let seg = &html[start..];

    // Headings de categoria (posição, nome sem o prefixo "Serviços ").
    let cat_re = Regex::new(r"(?s)elementor-heading-title[^>]*>\s*(Serviços[^<]{3,70})").unwrap();
    let cats: Vec<(usize, String)> = cat_re
        .captures_iter(seg)
        .map(|c| (c.get(0).unwrap().start(), strip_prefixo(&clean(&c[1]))))
        .collect();

    // Cards: link ?p=<id> com o título; ignora "Acesse a descrição…".
    let card_re = Regex::new(r#"(?s)href="[^"]*\?p=(\d+)"[^>]*>(.*?)</a>"#).unwrap();
    let mut seen: HashSet<String> = HashSet::new();
    let mut out: Vec<Card> = Vec::new();
    for cap in card_re.captures_iter(seg) {
        let pos = cap.get(0).unwrap().start();
        let post_id = cap[1].to_string();
        let titulo = html_to_text(&cap[2]);
        if titulo.len() < 5 || titulo.contains("Acesse a descri") {
            continue;
        }
        if !seen.insert(post_id.clone()) {
            continue;
        }
        let classe = cats
            .iter()
            .rev()
            .find(|(p, _)| *p < pos)
            .map(|(_, c)| c.clone())
            .unwrap_or_else(|| CLASSE_FALLBACK.to_string());
        out.push(Card { post_id, titulo, classe });
    }
    out
}

/// Corpo do serviço: o texto do container do post (`.elementor-widget-theme-post-content`), isolado do
/// header/footer/sidebar. O Elementor injeta CSS inline (`<style>`) DENTRO do container — removemos
/// `<style>`/`<script>` antes de extrair o texto (senão a descrição vem poluída de CSS).
fn extract_descricao(post_html: &str) -> String {
    let doc = Html::parse_document(post_html);
    let sel = Selector::parse(".elementor-widget-theme-post-content").unwrap();
    let inner = match doc.select(&sel).next() {
        Some(e) => e.inner_html(),
        None => return String::new(),
    };
    let sem_css = Regex::new(r"(?is)<(style|script)\b[^>]*>.*?</(style|script)>")
        .unwrap()
        .replace_all(&inner, " ");
    html_to_text(&sem_css)
}

/// HTML curto (texto de âncora) -> texto: tags viram espaço, entidades decodificadas, clean.
fn html_to_text(html: &str) -> String {
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

/// Nome da classe a partir do heading: "Serviços do IPVA" -> "IPVA", "Serviços Cadastros" ->
/// "Cadastros", "Serviços Notas Fiscais…" -> "Notas Fiscais…".
fn strip_prefixo(s: &str) -> String {
    let s = s.trim();
    for p in ["Serviços do ", "Serviços da ", "Serviços de ", "Serviços "] {
        if let Some(r) = s.strip_prefix(p) {
            return r.trim().to_string();
        }
    }
    s.to_string()
}

/// Guard (princípio D-RJ5): reprova coleta capada (Carta/markup mudou).
fn validar(items: &[ServicoRaw]) -> Result<()> {
    if items.len() < MIN_SERVICOS {
        bail!(
            "catálogo capado? só {} serviço(s) (mínimo {}). A Carta/markup pode ter mudado; se veio do \
             cache, limpe data/ac/raw/cache/ e re-raspe.",
            items.len(),
            MIN_SERVICOS
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const CARTA: &str = r#"
      <h2>Lista de Serviços</h2>
      <div class="elementor-widget-heading"><h3 class="elementor-heading-title">Serviços do IPVA</h3></div>
      <li class="eael-feature-list-item">
        <span class="eael-feature-list-title">IPVA – Isenção Táxi</span>
        <a href="https://sefaz.ac.gov.br/2021/?p=20859">IPVA – Isenção Táxi / Mototáxi</a>
        <a href="https://sefaz.ac.gov.br/2021/?p=20859">Acesse a descrição completa do serviço »</a>
      </li>
      <div class="elementor-widget-heading"><h3 class="elementor-heading-title">Serviços Cadastros</h3></div>
      <li class="eael-feature-list-item">
        <a href="https://sefaz.ac.gov.br/2021/?p=438">Cadastro de Contribuintes</a>
      </li>
    "#;

    const POST: &str = r#"<html><body>
      <header>menu topo</header>
      <div class="elementor-element elementor-widget-theme-post-content">
        <style>.elementor-9727 .elementor-element{transition:background 0.3s;}</style>
        <div class="elementor-widget-container">
          <p>São isentos de IPVA os ve&iacute;culos de <b>t&aacute;xi</b>, conforme a Lei 114/2002.</p>
        </div>
      </div>
      <footer>Rua Benjamin Constant, 946 — CNPJ 04.034.484/0001-40</footer>
    </body></html>"#;

    #[test]
    fn parse_carta_extrai_cards_e_categoria() {
        let cards = parse_carta(CARTA);
        assert_eq!(cards.len(), 2, "2 serviços distintos (o 'Acesse…' é ignorado, o ?p dup também)");
        let ipva = cards.iter().find(|c| c.post_id == "20859").unwrap();
        assert_eq!(ipva.titulo, "IPVA – Isenção Táxi / Mototáxi");
        assert_eq!(ipva.classe, "IPVA"); // "Serviços do IPVA" -> "IPVA"
        let cad = cards.iter().find(|c| c.post_id == "438").unwrap();
        assert_eq!(cad.classe, "Cadastros");
    }

    #[test]
    fn extract_descricao_isola_o_corpo_do_post() {
        let d = extract_descricao(POST);
        // pega SÓ o post-content (não o header/footer); entidades decodificadas; SEM o CSS do <style>.
        assert!(d.starts_with("São isentos de IPVA os veículos de táxi"), "veio: {d}");
        assert!(!d.contains("Benjamin Constant"), "não deve pegar o rodapé");
        assert!(!d.contains("menu topo"));
        assert!(!d.contains("transition"), "não deve pegar o CSS do <style>: {d}");
        assert!(!d.contains('<') && !d.contains("&iacute;"));
    }

    #[test]
    fn agent_com_r36_constroi() {
        let _ = build_agent(); // o PEM embutido é válido.
    }

    #[test]
    fn validar_reprova_capado() {
        let poucos = vec![ServicoRaw {
            titulo: "x".into(),
            descricao: String::new(),
            link: "l".into(),
            orgao: ORGAO.into(),
            ocorrencias: vec![],
        }];
        assert!(validar(&poucos).unwrap_err().to_string().contains("capado"));
    }
}
