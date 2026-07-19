// Coleta da SEFAZ-BA: Carta de Serviços ao Cidadão — listagem única (`ul#search_list`) + fichas
// de detalhe (`index.asp?id=<slug>`). ASP clássico server-side (ureq + `scraper`, sem headless).

use std::collections::HashMap;
use std::sync::LazyLock;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use regex::Regex;
use scraper::{ElementRef, Html, Selector};
use ureq::Agent;
use ureq::tls::{TlsConfig, TlsProvider};

use auli_contract::Publico;
use auli_scraper_kit::PerPublicoServicos;
use auli_scraper_kit::{clean_decoded, decode_entities};
use auli_contract::ServicoPerPublico as Servico;

const BASE: &str = "https://portal.sefaz.ba.gov.br";
const SEED_URL: &str = "https://portal.sefaz.ba.gov.br/scripts/cartadeservicos/index.asp";
// D-BA4: UA de navegador (o `kit::USER_AGENT` da frota — robots.txt restritivo a crawlers
// genéricos; coleta de baixíssima frequência, ~207 GETs por rodada, com cache).
// Cortesia entre fetches de ficha.
const COURTESY: Duration = Duration::from_millis(500);
/// D-BA2: ficha sem subtítulo `<small>` (classe do portal) recebe esta classe.
const CLASSE_GERAL: &str = "Geral";
/// D-BA3: ficha que falhar no fetch/parse degrada para este público, com warning.
const PUBLICO_FALLBACK: &str = "Cidadãos";
const ORGAO: &str = "SEFAZ-BA";

/// Rótulos de público conhecidos das fichas (`panel-title`) -> (nome canônico, slug per-público).
/// Rótulo fora do mapa vira nome/slug derivados, com warning (D-BA1).
fn publico_conhecido(rotulo: &str) -> Option<(&'static str, &'static str)> {
    match rotulo {
        "Serviços aos Cidadãos" => Some(("Cidadãos", "servicos-ao-cidadao")),
        "Serviços às Empresas" => Some(("Empresas", "servicos-a-empresas")),
        "Serviços aos Municípios" => Some(("Municípios", "servicos-a-municipios")),
        _ => None,
    }
}

/// Um item da listagem: `(titulo, link canônico)` — a identidade vem daqui; a ficha fornece
/// público, classe e corpo.
struct ListItem {
    titulo: String,
    link: String,
}

/// O que uma ficha de detalhe fornece.
struct Ficha {
    publico_rotulo: String,
    classe: String,
    corpo: String,
}

/// Agent `ureq` com backend **native-tls (OpenSSL)**, não o rustls padrão: o servidor da SEFAZ-BA
/// só negocia ciphers TLS 1.2 CBC (sem AEAD/GCM), que o rustls não suporta — ver nota no Cargo.toml.
fn build_agent_native_tls(user_agent: &str, timeout: Duration) -> Agent {
    Agent::config_builder()
        .user_agent(user_agent)
        .timeout_global(Some(timeout))
        .tls_config(TlsConfig::builder().provider(TlsProvider::NativeTls).build())
        .build()
        .into()
}

/// Raspa a Carta de Serviços da BA e devolve os per-público (na ordem de primeira aparição) + a
/// ordem dos públicos.
pub fn scrape(data_dir: &str, use_cache: bool) -> Result<(PerPublicoServicos, Vec<Publico>)> {
    let agent = build_agent_native_tls(auli_scraper_kit::USER_AGENT, Duration::from_secs(30));

    // 1. Listagem única.
    let seed = fetch(&agent, data_dir, SEED_URL, use_cache)?;
    let itens = parse_listagem(&Html::parse_document(&seed))?;
    println!("BA: listagem -> {} serviços", itens.len());

    // 2. Ficha de cada serviço (cache + cortesia).
    let mut fichas: HashMap<String, Ficha> = HashMap::new();
    for (i, item) in itens.iter().enumerate() {
        match fetch(&agent, data_dir, &item.link, use_cache)
            .and_then(|html| parse_ficha(&Html::parse_document(&html)))
        {
            Ok(f) => {
                fichas.insert(item.link.clone(), f);
            }
            Err(e) => eprintln!("⚠️  BA: ficha falhou para {}: {} (D-BA3)", item.link, e),
        }
        if (i + 1) % 25 == 0 {
            println!("BA: {}/{} fichas", i + 1, itens.len());
        }
    }

    // 3. Agrupa por público na ordem de primeira aparição (D-BA1); classe/corpo da ficha.
    let mut ordem: Vec<(String, String)> = Vec::new(); // (nome, slug)
    let mut per_pub: Vec<(String, Vec<Servico>)> = Vec::new();
    for item in &itens {
        let (publico_nome, publico_slug, classe, corpo) = match fichas.get(&item.link) {
            Some(f) => {
                let (nome, slug) = match publico_conhecido(&f.publico_rotulo) {
                    Some((n, s)) => (n.to_string(), s.to_string()),
                    None => {
                        eprintln!(
                            "⚠️  BA: público desconhecido '{}' em {} — usando rótulo cru (D-BA1)",
                            f.publico_rotulo, item.link
                        );
                        (f.publico_rotulo.clone(), slugify(&f.publico_rotulo))
                    }
                };
                (nome, slug, f.classe.clone(), f.corpo.clone())
            }
            None => (
                PUBLICO_FALLBACK.to_string(),
                "servicos-ao-cidadao".to_string(),
                CLASSE_GERAL.to_string(),
                String::new(),
            ),
        };

        let idx = match ordem.iter().position(|(n, _)| *n == publico_nome) {
            Some(i) => i,
            None => {
                ordem.push((publico_nome.clone(), publico_slug));
                per_pub.push((publico_nome.clone(), Vec::new()));
                ordem.len() - 1
            }
        };
        // Header de 3 linhas `tipo/classe/titulo` que o aggregate_servicos do kit remove.
        let descricao = format!("{}\n{}\n{}\n{}", publico_nome, classe, item.titulo, corpo);
        per_pub[idx].1.push(Servico {
            id: 0,
            tipo: publico_nome,
            classe,
            orgao: ORGAO.to_string(),
            link: item.link.clone(),
            titulo: item.titulo.clone(),
            descricao,
        });
    }

    for (nome, servicos) in &per_pub {
        println!("BA: público '{}' -> {} serviços", nome, servicos.len());
    }
    let publicos_ordem =
        ordem.into_iter().map(|(nome, slug)| Publico { nome, slug }).collect();
    Ok((per_pub, publicos_ordem))
}

/// Extrai a listagem: `ul#search_list li.list-group-item > a[href="index.asp?id=..."]`. Os
/// separadores de letra (`li.list-group-title`) têm `<a name=...>` sem href e se filtram sozinhos.
fn parse_listagem(doc: &Html) -> Result<Vec<ListItem>> {
    let ul = doc
        .select(&sel("ul#search_list"))
        .next()
        .ok_or_else(|| anyhow!("listagem 'ul#search_list' ausente em {} — layout mudou?", SEED_URL))?;

    let mut out = Vec::new();
    let mut vistos: std::collections::HashSet<String> = std::collections::HashSet::new();
    for a in ul.select(&sel("li.list-group-item a[href]")) {
        let Some(href) = a.value().attr("href") else { continue };
        let Some(link) = canonical(href) else { continue };
        // Só fichas da própria Carta entram na listagem (padrão único observado: index.asp?id=).
        if !link.contains("index.asp?id=") {
            continue;
        }
        let titulo = text(&a);
        if titulo.is_empty() || !vistos.insert(link.clone()) {
            continue;
        }
        out.push(ListItem { titulo, link });
    }
    if out.is_empty() {
        bail!("listagem vazia em {} — estrutura mudou?", SEED_URL);
    }
    Ok(out)
}

/// Extrai uma ficha: público do `panel-title`, classe do `<small>` do título (D-BA2), corpo =
/// introdução + blocos `media-service` como `Heading:\nconteúdo` (links normalizados).
fn parse_ficha(doc: &Html) -> Result<Ficha> {
    let content = doc
        .select(&sel("section#content"))
        .next()
        .ok_or_else(|| anyhow!("ficha sem 'section#content' — layout mudou?"))?;

    let publico_rotulo = content
        .select(&sel(".panel-cs .panel-title"))
        .next()
        .map(|el| text(&el))
        .unwrap_or_default();
    if publico_rotulo.is_empty() {
        bail!("ficha sem público ('.panel-cs .panel-title')");
    }

    let classe = content
        .select(&sel(".title-page h3 small"))
        .next()
        .map(|el| text(&el))
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| CLASSE_GERAL.to_string());

    // Introdução: <p> soltos do conteúdo (fora dos blocos media-service e do título).
    let mut partes: Vec<String> = Vec::new();
    for p in content.select(&sel("p")) {
        if has_ancestor_class(&p, "media-content") || has_ancestor_class(&p, "title-page") {
            continue;
        }
        let t = html_block_to_text(&p.html());
        if !t.is_empty() {
            partes.push(t);
        }
    }

    // Seções: h4.media-heading + div.media-content.
    for bloco in content.select(&sel("div.media-service")) {
        let heading =
            bloco.select(&sel("h4.media-heading")).next().map(|el| text(&el)).unwrap_or_default();
        let corpo = bloco
            .select(&sel("div.media-content"))
            .next()
            .map(|el| html_block_to_text(&el.inner_html()))
            .unwrap_or_default();
        if heading.is_empty() && corpo.is_empty() {
            continue;
        }
        partes.push(format!("{}:\n{}", heading, corpo));
    }

    Ok(Ficha { publico_rotulo, classe, corpo: partes.join("\n") })
}

/// `true` se algum ancestral-elemento de `el` tem a classe `class`.
fn has_ancestor_class(el: &ElementRef, class: &str) -> bool {
    let mut node = el.parent();
    while let Some(n) = node {
        if let Some(e) = ElementRef::wrap(n)
            && e.value().classes().any(|k| k == class)
        {
            return true;
        }
        node = n.parent();
    }
    false
}

// --- HTML -> texto (headers em linha própria + links `anchor "url"`) — helpers do padrão PR ---

static LINK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?is)<a[^>]*href=["']([^"']+)["'][^>]*>(.*?)</a>"#).unwrap());
static BLOCK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)</?(h[1-6]|p|li|ul|ol|div|br|tr|table)[^>]*>").unwrap());
static TAG_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]+>").unwrap());

/// `<a href="url">texto</a>` -> `texto "url"` (âncora vazia -> só `"url"`); ignora `#`/`javascript:`.
fn normalize_body_links(html: &str) -> String {
    LINK_RE
        .replace_all(html, |c: &regex::Captures| {
            let href = c[1].trim();
            let texto = clean_decoded(&strip_tags(&c[2]));
            if href.starts_with('#') || href.starts_with("javascript:") {
                return texto;
            }
            let url = canonical_any(href);
            if texto.is_empty() {
                format!("\"{}\"", url)
            } else {
                format!("{} \"{}\"", texto, url)
            }
        })
        .into_owned()
}

/// HTML de bloco -> texto: normaliza links, quebra linha nos blocos, tira tags e limpa.
fn html_block_to_text(html: &str) -> String {
    let with_links = normalize_body_links(html);
    let with_breaks = BLOCK_RE.replace_all(&with_links, "\n");
    clean_text(&strip_tags(&with_breaks))
}

fn strip_tags(html: &str) -> String {
    TAG_RE.replace_all(html, "").into_owned()
}

/// Normaliza por linha (comprime espaços, decodifica entidades) e descarta linhas vazias. Line-based
/// (preserva quebras) — semântica própria do formato, fica local; usa `kit::decode_entities`.
fn clean_text(s: &str) -> String {
    decode_entities(s)
        .lines()
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn text(el: &ElementRef) -> String {
    clean_decoded(&el.text().collect::<String>())
}

/// Slug simples para rótulos de público fora do mapa (D-BA1): minúsculas, [a-z0-9-].
fn slugify(s: &str) -> String {
    let mut out = String::new();
    for c in s.to_lowercase().chars() {
        let mapped = match c {
            'á' | 'à' | 'â' | 'ã' => 'a',
            'é' | 'ê' => 'e',
            'í' => 'i',
            'ó' | 'ô' | 'õ' => 'o',
            'ú' | 'ü' => 'u',
            'ç' => 'c',
            _ => c,
        };
        if mapped.is_ascii_alphanumeric() {
            out.push(mapped);
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

/// URL canônica de um href da Carta: absolutiza relativos (`index.asp?id=...` ancora no diretório
/// da Carta; `/scripts/...` no host), preserva absolutos, remove fragmento; descarta
/// `javascript:`/`#`/`mailto:`/vazio.
fn canonical(href: &str) -> Option<String> {
    // Alguns slugs `id=` da listagem trazem espaço literal (ex.: `..._ mensal_...`) — inválido numa
    // URI. Encoda para `%20` (o ureq recusa o caractere cru; navegador/curl encodam por conta).
    let h = href.split('#').next().unwrap_or(href).trim().replace(' ', "%20");
    let h = h.as_str();
    if h.is_empty() || h.starts_with("javascript:") || h.starts_with("mailto:") {
        return None;
    }
    if h.starts_with("http://") || h.starts_with("https://") {
        return Some(h.to_string());
    }
    if let Some(stripped) = h.strip_prefix('/') {
        return Some(format!("{}/{}", BASE, stripped));
    }
    // Relativo ao diretório da Carta (caso da listagem: `index.asp?id=...`).
    Some(format!("{}/scripts/cartadeservicos/{}", BASE, h))
}

/// Absolutiza qualquer href (para os links do corpo): relativo -> host do portal; externo -> como está.
fn canonical_any(href: &str) -> String {
    if href.starts_with("http://") || href.starts_with("https://") || href.starts_with("mailto:") {
        href.to_string()
    } else if let Some(stripped) = href.strip_prefix('/') {
        format!("{}/{}", BASE, stripped)
    } else {
        format!("{}/scripts/cartadeservicos/{}", BASE, href)
    }
}

fn sel(s: &str) -> Selector {
    Selector::parse(s).expect("seletor CSS inválido")
}

/// Busca (ou lê do cache) a página `url`. Em `--usecache` um miss é erro (sem rede). Cortesia entre
/// fetches de rede; retry com backoff. Guarda de charset: o portal declara UTF-8, mas ASP clássico
/// pode servir bytes latin-1 — bytes inválidos são substituídos com aviso, nunca derrubam a coleta.
// NB: o retry fica local (não usa `kit::http::get_string`) porque a resposta é lida como bytes e
// passa por `decode_charset` (o ASP é latin1, não UTF-8) — trabalho real de charset por-portal.
fn fetch(agent: &Agent, data_dir: &str, url: &str, use_cache: bool) -> Result<String> {
    if let Some(cached) = auli_scraper_kit::cache::read_or_bail(data_dir, "servicos", url, use_cache)? {
        return Ok(cached);
    }

    let max_attempts = 3;
    let mut delay = Duration::from_millis(800);
    let mut last = anyhow!("sem tentativa");
    for attempt in 1..=max_attempts {
        match agent.get(url).call() {
            Ok(mut resp) => match resp.body_mut().read_to_vec() {
                Ok(bytes) => {
                    let body = decode_charset(&bytes, url);
                    if !body.trim().is_empty() {
                        auli_scraper_kit::cache::write(data_dir, "servicos", url, &body);
                        sleep(COURTESY);
                        return Ok(body);
                    }
                    last = anyhow!("resposta vazia");
                }
                Err(e) => last = anyhow!(e.to_string()),
            },
            Err(e) => last = anyhow!(e.to_string()),
        }
        if attempt < max_attempts {
            eprintln!("BA: falha em {} (tentativa {}/{}): {}. Retentando...", url, attempt, max_attempts, last);
            sleep(delay);
            delay = delay.saturating_mul(2);
        }
    }
    Err(anyhow!("falha ao buscar {}: {}", url, last))
}

/// Decodifica o corpo: UTF-8 válido passa direto; inválido cai para latin-1 (windows-1252 básico),
/// com aviso — nunca derruba a coleta por charset (ASP clássico é imprevisível).
fn decode_charset(bytes: &[u8], url: &str) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => {
            eprintln!("⚠️  BA: {} não é UTF-8 válido — decodificando como latin-1", url);
            bytes.iter().map(|&b| b as char).collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fixtures REAIS capturadas via view-source em 2026-07-04: a listagem completa e a ficha
    /// `?id=senha`.
    const LISTAGEM: &str = include_str!("../tests/fixtures/ba-listagem.html");
    const FICHA_SENHA: &str = include_str!("../tests/fixtures/ba-ficha-senha.html");

    #[test]
    fn listagem_tem_204_servicos_ativos_unicos_com_links_canonicos() {
        let itens = parse_listagem(&Html::parse_document(LISTAGEM)).unwrap();
        // O fonte tem 206 hrefs `?id=`, mas 2 estão COMENTADOS (`<!--li ...-->`): serviços
        // desativados pelo portal (prorrogação de prazo p/ exportação; red. BC diesel). O parser
        // HTML os ignora corretamente — 204 ativos.
        assert_eq!(itens.len(), 204);
        let links: std::collections::HashSet<&str> =
            itens.iter().map(|i| i.link.as_str()).collect();
        assert_eq!(links.len(), 204, "links devem ser únicos");
        assert!(itens.iter().all(|i| i
            .link
            .starts_with("https://portal.sefaz.ba.gov.br/scripts/cartadeservicos/index.asp?id=")));
        assert_eq!(
            itens[0].titulo,
            "AIDF - Cancelamento de Autorização para Impressão de Documentos Fiscais"
        );
        assert_eq!(
            itens.last().unwrap().titulo,
            "Verificação de Autenticidade - Certidão de Débitos Tributários"
        );
        // Separadores de letra (li.list-group-title, <a name=...> sem href) não viram itens.
        assert!(!itens.iter().any(|i| i.titulo.len() == 1));
    }

    #[test]
    fn ficha_senha_extrai_publico_classe_e_secoes() {
        let f = parse_ficha(&Html::parse_document(FICHA_SENHA)).unwrap();
        assert_eq!(f.publico_rotulo, "Serviços aos Cidadãos");
        assert_eq!(f.classe, "Requerimento");
        // Introdução presente.
        assert!(f.corpo.starts_with("Este serviço permite ao contribuinte"));
        // As 5 seções como `Heading:`.
        for h in
            ["Documentos Necessários:", "Como Fazer:", "Canal:", "Tempo Médio:", "Base Legal:"]
        {
            assert!(f.corpo.contains(h), "seção ausente: {}", h);
        }
        // Link interno do corpo absolutizado no formato `texto "url"`.
        assert!(f.corpo.contains(
            "clique aqui \"https://portal.sefaz.ba.gov.br/scripts/cartadeservicos/index.asp?id=senha_cancelamento\""
        ));
        // Conteúdo pontual das seções curtas.
        assert!(f.corpo.contains("Canal:\nInternet."));
        assert!(f.corpo.contains("Base Legal:\nArt. 1º da Portaria nº 582/00."));
    }

    #[test]
    fn publico_conhecido_e_slugify() {
        assert_eq!(
            publico_conhecido("Serviços aos Cidadãos"),
            Some(("Cidadãos", "servicos-ao-cidadao"))
        );
        assert_eq!(publico_conhecido("Outra Coisa"), None);
        assert_eq!(slugify("Serviços à População"), "servicos-a-populacao");
    }

    #[test]
    fn canonical_rules() {
        assert_eq!(
            canonical("index.asp?id=senha").as_deref(),
            Some("https://portal.sefaz.ba.gov.br/scripts/cartadeservicos/index.asp?id=senha")
        );
        assert_eq!(
            canonical("/scripts/cartadeservicos/index.asp?id=x").as_deref(),
            Some("https://portal.sefaz.ba.gov.br/scripts/cartadeservicos/index.asp?id=x")
        );
        assert_eq!(canonical("javascript:;"), None);
        assert_eq!(canonical("mailto:x@y.z"), None);
        assert_eq!(
            canonical("https://www.sefaz.ba.gov.br/carta-de-servicos/").as_deref(),
            Some("https://www.sefaz.ba.gov.br/carta-de-servicos/")
        );
    }

    #[test]
    fn decode_charset_fallback_latin1() {
        assert_eq!(decode_charset("ação".as_bytes(), "u"), "ação");
        // "ação" em latin-1: 0x61 0xE7 0xE3 0x6F.
        assert_eq!(decode_charset(&[0x61, 0xE7, 0xE3, 0x6F], "u"), "ação");
    }
}
