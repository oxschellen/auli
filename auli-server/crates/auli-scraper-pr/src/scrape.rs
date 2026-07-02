// Coleta da SEFA-PR: mega-menu "Serviços para você!" (7 abas × grupos) + páginas de detalhe (Carta
// de Serviços). Portal Drupal server-side — o HTML já vem pronto (ureq + `scraper`, sem headless).

use std::collections::HashMap;
use std::sync::LazyLock;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use regex::Regex;
use scraper::{Html, Selector};
use ureq::Agent;

use auli_contract::Publico;
use auli_scraper_kit::{PerPublicoServicos, Servico};

const BASE: &str = "https://www.fazenda.pr.gov.br";
// Página interna estável com o mega-menu completo (a raiz pode cair numa splash de campanha).
const SEED_URL: &str = "https://www.fazenda.pr.gov.br/Pagina/Carta-de-servicos";
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64; rv:124.0) Gecko/20100101 Firefox/124.0";
// Cortesia entre fetches de detalhe (portal marca noindex; coleta de baixa frequência — D-PR6).
const COURTESY: Duration = Duration::from_millis(400);

/// Os 7 públicos, na ordem de exibição (D-PR3). "Mais buscados" é excluída por completo (só curadoria).
/// `(id do painel no HTML, nome do público, slug do arquivo per-público)`.
fn publicos() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("servicos-tema-cidado", "Cidadão", "pr-servicos-ao-cidadao"),
        ("servicos-tema-empresa", "Empresa", "pr-servicos-a-empresas"),
        ("servicos-tema-municpio", "Município", "pr-servicos-a-municipios"),
        ("servicos-tema-produtor-rural", "Produtor rural", "pr-servicos-a-produtores-rurais"),
        ("servicos-tema-receitapr", "Receita/PR", "pr-servicos-receita-pr"),
        ("servicos-tema-programas", "Programas", "pr-servicos-programas"),
        ("servicos-tema-legislao", "Legislação", "pr-servicos-legislacao"),
    ]
}

/// Um item do menu: um serviço listado sob uma aba (público) e um grupo (classe).
struct MenuItem {
    titulo: String,
    link: String,
    classe: String,
}

/// Raspa os serviços do PR e devolve os per-público (na ordem do menu) + a ordem dos públicos.
pub fn scrape(data_dir: &str, use_cache: bool) -> Result<(PerPublicoServicos, Vec<Publico>)> {
    let agent = auli_scraper_kit::build_agent(USER_AGENT, Some(Duration::from_secs(30)));

    // 1. Seed com o mega-menu.
    let seed = fetch(&agent, data_dir, SEED_URL, use_cache)?;
    let doc = Html::parse_document(&seed);
    if doc.select(&sel("#block-governodigitalmenuservicosagrupamento")).next().is_none() {
        bail!("mega-menu 'Serviços para você' ausente no seed {} — layout mudou?", SEED_URL);
    }

    // 2. Parse do menu: 7 públicos -> itens (titulo, link, classe).
    let pubs = publicos();
    let mut per_pub: Vec<(String, Vec<MenuItem>)> = Vec::new();
    for (panel_id, nome, _) in &pubs {
        let items = parse_panel(&doc, panel_id);
        println!("PR: aba '{}' -> {} ocorrências", nome, items.len());
        per_pub.push((nome.to_string(), items));
    }

    // 3. Guarda D-PR3: links exclusivos de "Mais buscados" (sumiriam do catálogo).
    orphan_check(&doc, &per_pub);

    // 4. Detalhe de cada link único (cache + cortesia).
    let mut unique: Vec<String> = Vec::new();
    for (_, items) in &per_pub {
        for it in items {
            if !unique.contains(&it.link) {
                unique.push(it.link.clone());
            }
        }
    }
    println!("PR: {} links únicos — buscando detalhes...", unique.len());
    let mut detail: HashMap<String, Detail> = HashMap::new();
    for (i, link) in unique.iter().enumerate() {
        match fetch_detail(&agent, data_dir, link, use_cache) {
            Ok(d) => {
                detail.insert(link.clone(), d);
            }
            Err(e) => eprintln!("⚠️  PR: detalhe falhou para {}: {}", link, e),
        }
        if (i + 1) % 25 == 0 {
            println!("PR: {}/{} detalhes", i + 1, unique.len());
        }
    }

    // 5. Monta o PerPublicoServicos (uma ocorrência por item do menu; descricao com header de 3
    //    linhas que o `aggregate_servicos` do kit remove).
    let mut inputs: PerPublicoServicos = Vec::new();
    for (nome, items) in per_pub {
        let mut servicos = Vec::new();
        for it in items {
            let (orgao, corpo) = match detail.get(&it.link) {
                Some(d) => (d.orgao.clone(), d.corpo.clone()),
                None => ("SEFA".to_string(), String::new()),
            };
            let descricao = format!("{}\n{}\n{}\n{}", nome, it.classe, it.titulo, corpo);
            servicos.push(Servico {
                id: 0,
                tipo: nome.clone(),
                classe: it.classe,
                orgao,
                link: it.link,
                titulo: it.titulo,
                descricao,
            });
        }
        inputs.push((nome, servicos));
    }

    let publicos_ordem = pubs
        .iter()
        .map(|(_, nome, slug)| Publico { nome: nome.to_string(), slug: slug.to_string() })
        .collect();
    Ok((inputs, publicos_ordem))
}

/// Extrai os itens de um painel/aba: `li.agrupador` -> header `<a>` (classe) + `ul.lista-sub-agrupadores`
/// -> `a[href*="/servicos/"]` (titulo, link canônico).
fn parse_panel(doc: &Html, panel_id: &str) -> Vec<MenuItem> {
    let mut out = Vec::new();
    let grupo_sel = sel(&format!("#{} li.agrupador", panel_id));
    let header_sel = sel("a");
    let item_sel = sel("ul.lista-sub-agrupadores a");
    for grupo in doc.select(&grupo_sel) {
        let classe = grupo
            .select(&header_sel)
            .next()
            .map(|a| text(&a))
            .unwrap_or_default();
        for a in grupo.select(&item_sel) {
            let Some(href) = a.value().attr("href") else { continue };
            let Some(link) = canonical(href) else { continue };
            out.push(MenuItem { titulo: text(&a), link, classe: classe.clone() });
        }
    }
    out
}

/// D-PR3: avisa (sem falhar) se algum link só existe em "Mais buscados" — sumiria do catálogo.
fn orphan_check(doc: &Html, per_pub: &[(String, Vec<MenuItem>)]) {
    let coletados: std::collections::HashSet<&str> =
        per_pub.iter().flat_map(|(_, its)| its.iter().map(|i| i.link.as_str())).collect();
    let mut orfaos = Vec::new();
    for it in parse_panel(doc, "servicos-tema-mais-buscados") {
        if !coletados.contains(it.link.as_str()) && !orfaos.contains(&it.link) {
            orfaos.push(it.link.clone());
        }
    }
    if orfaos.is_empty() {
        println!("✅ PR: nenhum serviço exclusivo de 'Mais buscados' (D-PR3 ok).");
    } else {
        eprintln!("⚠️  PR: {} link(s) só em 'Mais buscados' (decisão manual):", orfaos.len());
        for l in &orfaos {
            eprintln!("  - {}", l);
        }
    }
}

/// Corpo limpo + órgão de uma página de detalhe.
struct Detail {
    corpo: String,
    orgao: String,
}

static ORGAO_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"Secretaria de Estado da Fazenda[^<\n]*").unwrap());

/// Extrai a Carta de Serviços (D-PR4): a coluna principal (`div.col-md-8.col-lg-9` — seções
/// "O que é" -> "O que diz a lei" + botão de ação, links normalizados) + "Forma de atendimento" e
/// "Quanto custa" da lateral; `orgao` do bloco "Órgão Responsável" (fallback "SEFA").
fn fetch_detail(agent: &Agent, data_dir: &str, url: &str, use_cache: bool) -> Result<Detail> {
    let html = fetch(agent, data_dir, url, use_cache)?;
    let doc = Html::parse_document(&html);

    // Coluna principal sem o `header-titulo` (breadcrumb + título repetido = resíduo de template);
    // mantém `servico-acao` (botão de ação) e as seções `h3`.
    let mut corpo = doc
        .select(&sel("div.col-md-8.col-lg-9"))
        .next()
        .map(|el| {
            let html: String = el
                .children()
                .filter_map(scraper::ElementRef::wrap)
                .filter(|c| !c.value().classes().any(|k| k == "header-titulo"))
                .map(|c| c.html())
                .collect();
            html_block_to_text(&html)
        })
        .unwrap_or_default();

    // Lateral: Forma de atendimento / Quanto custa (blocos `servico-info-quadro`).
    let lateral: String = doc
        .select(&sel("div.col-md-4.col-lg-3 div.servico-info-quadro"))
        .map(|el| clean_text(&strip_tags(&normalize_body_links(&el.inner_html()))))
        .filter(|t| {
            t.contains("Forma de atendimento") || t.contains("Quanto custa")
        })
        .collect::<Vec<_>>()
        .join("\n");
    if !lateral.trim().is_empty() {
        corpo.push('\n');
        corpo.push_str(lateral.trim());
    }

    let orgao = ORGAO_RE
        .find(&html)
        .map(|m| m.as_str().trim().to_string())
        .unwrap_or_else(|| "SEFA".to_string());

    Ok(Detail { corpo, orgao })
}

// --- HTML -> texto (headers em linha própria + links `anchor "url"`) ---

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
            let texto = clean_inline(&strip_tags(&c[2]));
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

/// Decodifica as entidades comuns e comprime espaços numa linha só.
fn clean_inline(s: &str) -> String {
    decode_entities(s).split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Normaliza por linha (comprime espaços, decodifica entidades) e descarta linhas vazias.
fn clean_text(s: &str) -> String {
    decode_entities(s)
        .lines()
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn decode_entities(s: &str) -> String {
    s.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&aacute;", "á")
}

fn text(el: &scraper::ElementRef) -> String {
    clean_inline(&el.text().collect::<String>())
}

/// URL canônica de um serviço: só `/servicos/...` viram serviço (D-PR5); host `www.fazenda.pr.gov.br`,
/// https, sem fragmento. `None` se não for um link de serviço.
fn canonical(href: &str) -> Option<String> {
    let h = href.split('#').next().unwrap_or(href);
    let pos = h.find("/servicos/")?;
    Some(format!("{}{}", BASE, &h[pos..]))
}

/// Absolutiza qualquer href (para os links do corpo): relativo -> host da SEFA; externo -> como está.
fn canonical_any(href: &str) -> String {
    if href.starts_with("http://") || href.starts_with("https://") {
        href.to_string()
    } else if let Some(stripped) = href.strip_prefix('/') {
        format!("{}/{}", BASE, stripped)
    } else {
        href.to_string()
    }
}

fn sel(s: &str) -> Selector {
    Selector::parse(s).expect("seletor CSS inválido")
}

/// Busca (ou lê do cache) a página `url`. Em `--usecache` um miss é erro (sem rede). Cortesia entre
/// fetches de rede.
fn fetch(agent: &Agent, data_dir: &str, url: &str, use_cache: bool) -> Result<String> {
    if let Some(cached) = auli_scraper_kit::cache::read(data_dir, url) {
        return Ok(cached);
    }
    if use_cache {
        bail!("cache miss para {} (modo --usecache, sem rede)", url);
    }

    let max_attempts = 3;
    let mut delay = Duration::from_millis(800);
    let mut last = anyhow!("sem tentativa");
    for attempt in 1..=max_attempts {
        match agent.get(url).call() {
            Ok(mut resp) => match resp.body_mut().read_to_string() {
                Ok(body) if !body.trim().is_empty() => {
                    auli_scraper_kit::cache::write(data_dir, url, &body);
                    sleep(COURTESY);
                    return Ok(body);
                }
                Ok(_) => last = anyhow!("resposta vazia"),
                Err(e) => last = anyhow!(e.to_string()),
            },
            Err(e) => last = anyhow!(e.to_string()),
        }
        if attempt < max_attempts {
            eprintln!("PR: falha em {} (tentativa {}/{}): {}. Retentando...", url, attempt, max_attempts, last);
            sleep(delay);
            delay = delay.saturating_mul(2);
        }
    }
    Err(anyhow!("falha ao buscar {}: {}", url, last))
}
