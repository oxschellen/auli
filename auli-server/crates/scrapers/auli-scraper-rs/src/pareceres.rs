//! pareceres — coleta dos **Pareceres** da SEFAZ-RS no Portal de Legislação
//! (`legislacao.sefaz.rs.gov.br`, ASP.NET WebForms / IIS, windows-1252). Molde BA/TO (HTML
//! server-rendered, sem headless).
//!
//! ⚠️ ROBOTS/ACESSO: o portal declara `robots.txt: Disallow: /` e a busca é rotulada
//! "Acesso Restrito". A coleta destes pareceres **públicos** foi autorizada pelo mantenedor (a
//! própria SEFAZ-RS). UA institucional AuliBot + cortesia (500 ms) entre requisições.
//!
//! ⚠️ TRANSPORTE — **curl subprocess** (como GO/DF/SE): com o `ureq` a paginação por postback
//! sempre volta a página 1 (uma camada de cache/WAF na frente do IIS serve resposta degradada ao
//! ClientHello/headers do ureq). O `curl` — com os MESMOS cookie+viewstate — devolve a página certa.
//! Então listagem e detalhe vão por `curl`, com um **cookie jar** que mantém a sessão
//! `ASP.NET_SessionId` (exigida pelo postback: sem ela o servidor pendura a conexão).
//!
//! Fonte:
//! - **Listagem** `Search.aspx?CodArea=3&CodGroup=159`: cada resultado é
//!   `<h5><a href="javascript:goDocument(<inpKey>,'')">PARECER Nº N</a></h5><p>ASSUNTO</p>`.
//!   Paginação por postback: POST `__EVENTTARGET=LinkToPage` / `__EVENTARGUMENT=<pág>` carregando o
//!   `__VIEWSTATE`/`__VIEWSTATEGENERATOR` **da resposta anterior**. ~20/pág, ~17 págs → ~331 chaves.
//! - **Detalhe** `DocumentView.aspx?inpKey=N` (público/anônimo): corpo em `#DOCContent .content`.
//!
//! ESCOPO (incremental): grava só o intermediário `../data/rs/ref/rs-pareceres-temp.txt`
//! (numero/assunto/corpo/link, **sem `resumo`** — o resumo autorado é um estágio posterior que
//! preserva o resumo já existente). Não toca contrato/snapshot/collections.

use std::collections::HashSet;
use std::io::Write;
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::Duration;

use regex::Regex;
use scraper::{Html, Selector};

use auli_scraper_kit::{cache, clean};

use crate::errors::{Error, Result};

const LISTING_URL: &str = "http://www.legislacao.sefaz.rs.gov.br/Site/Search.aspx?CodArea=3&CodGroup=159";
const DETAIL_BASE: &str = "http://www.legislacao.sefaz.rs.gov.br/Site/DocumentView.aspx?inpKey=";
const UA: &str = "AuliBot/0.1 (+https://github.com/oxschellen/auli; carlos.schellenberger@gmail.com)";
const OUT_PATH: &str = "../data/rs/ref/rs-pareceres-temp.txt";
/// Árvore de documentos (G5): um `.md` por consulta inédita. Fonte a partir da G5b.
const DOCS_DIR: &str = "../data/rs/docs/pareceres";
// Cache namespace da frota: `<CACHE_BASE>/cache/<CACHE_KIND>` = `../data/rs/raw/cache/pareceres`,
// irmão de `cache/servicos` e `cache/faqs` (mesma estrutura dos demais estados). `CACHE_DIR` é esse
// mesmo diretório, usado para o cookie jar e o `create_dir_all`.
const CACHE_BASE: &str = "../data/rs/raw";
const CACHE_KIND: &str = "pareceres";
const CACHE_DIR: &str = "../data/rs/raw/cache/pareceres";
const COURTESY: Duration = Duration::from_millis(500);
const MAX_PAGES: usize = 40; // safety cap (real ≈ 17)
const MIN_PARECERES: usize = 250; // guarda contra coleta truncada (o .txt autorado tem 331)

struct Row {
    inp_key: String,
    numero: String,
    assunto: String,
}

struct Parecer {
    numero: String,
    assunto: String,
    corpo: String,
    link: String,
}

pub fn run(use_cache: bool) -> Result<()> {
    std::fs::create_dir_all(CACHE_DIR)?;
    // Cookie jar do curl: mantém a sessão ASP.NET entre o GET da página 1 e os POSTs de paginação.
    let jar = format!("{CACHE_DIR}/.session-cookies.txt");
    let _ = std::fs::remove_file(&jar); // sessão nova a cada coleta

    println!("🔎 Enumerando documentos (CodArea=3 / CodGroup=159)…");
    let all_rows = enumerate(&jar, use_cache)?;
    let total_docs = all_rows.len();
    let rows: Vec<Row> = all_rows.into_iter().filter(|r| is_consulta_formal(&r.numero)).collect();
    println!("   {} consultas formais (pareceres+informações) de {total_docs} documentos.", rows.len());
    if rows.len() < MIN_PARECERES {
        return Err(Error::Custom(format!(
            "coleta devolveu {} pareceres (< MIN {MIN_PARECERES}); abortando para não truncar.",
            rows.len()
        )));
    }

    let total = rows.len();
    let mut items: Vec<Parecer> = Vec::with_capacity(total);
    for (i, row) in rows.into_iter().enumerate() {
        let url = format!("{DETAIL_BASE}{}", row.inp_key);
        let html = fetch_detail(&url, use_cache)?;
        let corpo = parse_corpo(&html);
        if corpo.is_empty() {
            eprintln!("⚠️  corpo vazio: inpKey={} ({})", row.inp_key, row.numero);
        }
        items.push(Parecer { numero: row.numero, assunto: row.assunto, corpo, link: url });
        if (i + 1) % 25 == 0 {
            println!("   detalhe {}/{total}", i + 1);
        }
        sleep(COURTESY);
    }

    write_temp(&items)?;
    // G5: emite a árvore `.md` (um arquivo por consulta INÉDITA; existente é pulado — é o
    // incremental, e protege sinopses já geradas). O `.txt` acima segue até a G5b aposentar o JSON.
    let docs: Vec<auli_scraper_kit::docs::DocParaEmitir<'_>> = items
        .iter()
        .map(|p| auli_scraper_kit::docs::DocParaEmitir {
            numero: &p.numero,
            assunto: &p.assunto,
            link: &p.link,
            corpo: &p.corpo,
        })
        .collect();
    let dir = std::path::Path::new(DOCS_DIR);
    let (criados, pulados) = auli_scraper_kit::docs::emitir_pareceres(dir, &docs)?;
    auli_scraper_kit::docs::relatar(dir, criados, pulados);
    println!("✅ Escrito {OUT_PATH} ({} pareceres). O estágio de resumo autorado é posterior.", items.len());
    Ok(())
}

/// Enumera todas as chaves (inpKey + numero + assunto) percorrendo a paginação por postback.
/// Para quando uma página não traz nenhuma chave nova (fim do conjunto) ou no teto MAX_PAGES.
fn enumerate(jar: &str, use_cache: bool) -> Result<Vec<Row>> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut rows: Vec<Row> = Vec::new();

    // Página 1 = GET (semeia o cookie de sessão no jar); demais = POST `LinkToPage`. As páginas da
    // listagem são específicas da sessão (o VIEWSTATE muda por sessão e o postback exige o cookie),
    // então NO MODO LIVE são sempre buscadas frescas — nunca do cache — para tudo vir da MESMA sessão.
    // Só o `--usecache` (offline) lê a listagem do cache.
    let cache_key1 = format!("{LISTING_URL}#page1");
    let mut html = if use_cache {
        cache::read_or_bail(CACHE_BASE, CACHE_KIND, &cache_key1, true)?
            .ok_or_else(|| Error::Custom("página 1 ausente do cache (--usecache)".into()))?
    } else {
        let html = decode_charset(&curl_get(LISTING_URL, Some(jar))?);
        cache::write(CACHE_BASE, CACHE_KIND, &cache_key1, &html);
        html
    };
    let mut page = 1usize;

    loop {
        let mut novos = 0usize;
        for r in parse_rows(&html) {
            if seen.insert(r.inp_key.clone()) {
                rows.push(r);
                novos += 1;
            }
        }
        println!("   página {page}: {novos} novos (total {}).", rows.len());
        if novos == 0 {
            break;
        }
        if page >= MAX_PAGES {
            eprintln!("⚠️  atingiu MAX_PAGES={MAX_PAGES}; parando a paginação.");
            break;
        }
        page += 1;
        sleep(COURTESY);
        let next = post_listing_page(page, &html, jar, use_cache)?;
        html = next;
    }

    Ok(rows)
}

/// POST `LinkToPage=<page>` reenviando o **formulário inteiro** da página anterior. Só o VIEWSTATE
/// NÃO basta: o filtro "Consultas Formais Respondidas" (checkbox marcado na busca) se perde e o
/// servidor volta à busca padrão (todos os tipos) — o browser reenvia todos os campos, nós também.
/// Cacheável por chave sintética (`<url>#page<N>`) para `--usecache` reproduzir a coleta offline.
fn post_listing_page(page: usize, prev_html: &str, jar: &str, use_cache: bool) -> Result<String> {
    let cache_key = format!("{LISTING_URL}#page{page}");
    if use_cache {
        return cache::read_or_bail(CACHE_BASE, CACHE_KIND, &cache_key, true)?
            .ok_or_else(|| Error::Custom(format!("página {page} ausente do cache (--usecache)")));
    }
    let doc = Html::parse_document(prev_html);
    let mut fields = collect_form_fields(&doc);
    fields.retain(|(k, _)| k != "__EVENTTARGET" && k != "__EVENTARGUMENT");
    let mut all: Vec<(String, String)> = vec![
        ("__EVENTTARGET".to_string(), "LinkToPage".to_string()),
        ("__EVENTARGUMENT".to_string(), page.to_string()),
    ];
    all.extend(fields);
    let refs: Vec<(&str, &str)> = all.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let body = form_urlencoded(&refs);
    let html = decode_charset(&curl_post_form(LISTING_URL, &body, jar)?);
    cache::write(CACHE_BASE, CACHE_KIND, &cache_key, &html);
    Ok(html)
}

/// Coleta todos os campos submissíveis do `FormBuscaAvancada`: inputs (hidden/text sempre;
/// checkbox/radio só se `checked`) e o valor selecionado de cada `select`. Botões são ignorados.
fn collect_form_fields(doc: &Html) -> Vec<(String, String)> {
    let mut fields: Vec<(String, String)> = Vec::new();

    let input_sel = Selector::parse("input[name]").expect("seletor válido");
    for el in doc.select(&input_sel) {
        let v = el.value();
        let name = v.attr("name").unwrap_or("");
        if name.is_empty() {
            continue;
        }
        let typ = v.attr("type").unwrap_or("text").to_ascii_lowercase();
        let val = v.attr("value").unwrap_or("");
        match typ.as_str() {
            "checkbox" | "radio" => {
                if v.attr("checked").is_some() {
                    fields.push((name.to_string(), if val.is_empty() { "on".into() } else { val.to_string() }));
                }
            }
            "submit" | "button" | "image" | "reset" | "file" => {}
            _ => fields.push((name.to_string(), val.to_string())),
        }
    }

    let sel_sel = Selector::parse("select[name]").expect("seletor válido");
    let opt_sel = Selector::parse("option").expect("seletor válido");
    for sel in doc.select(&sel_sel) {
        let name = sel.value().attr("name").unwrap_or("");
        if name.is_empty() {
            continue;
        }
        let mut first: Option<String> = None;
        let mut chosen: Option<String> = None;
        for opt in sel.select(&opt_sel) {
            let val = opt.value().attr("value").unwrap_or("").to_string();
            if first.is_none() {
                first = Some(val.clone());
            }
            if opt.value().attr("selected").is_some() {
                chosen = Some(val);
                break;
            }
        }
        fields.push((name.to_string(), chosen.or(first).unwrap_or_default()));
    }

    fields
}

/// GET de um detalhe (`DocumentView.aspx?inpKey=N`), com cache. Público — não precisa da sessão.
fn fetch_detail(url: &str, use_cache: bool) -> Result<String> {
    if let Some(cached) = cache::read_or_bail(CACHE_BASE, CACHE_KIND, url, use_cache)? {
        return Ok(cached);
    }
    let html = decode_charset(&curl_get(url, None)?);
    cache::write(CACHE_BASE, CACHE_KIND, url, &html);
    Ok(html)
}

// ---------------------------------------------------------------------------
// HTTP via curl subprocess (args separados, nunca via shell; `--fail-with-body`).
// ---------------------------------------------------------------------------

/// Nº de tentativas por requisição (o portal às vezes pendura a conexão → timeout transitório).
const ATTEMPTS: u32 = 4;

/// Retenta `f` até `ATTEMPTS` vezes com backoff linear; propaga o último erro se todas falharem.
fn with_retry(label: &str, mut f: impl FnMut() -> Result<Vec<u8>>) -> Result<Vec<u8>> {
    let mut last = String::new();
    for attempt in 1..=ATTEMPTS {
        match f() {
            Ok(bytes) => return Ok(bytes),
            Err(e) => {
                last = e.to_string();
                if attempt < ATTEMPTS {
                    eprintln!("⚠️  {label}: tentativa {attempt}/{ATTEMPTS} falhou ({last}); retentando…");
                    sleep(Duration::from_secs(attempt as u64));
                }
            }
        }
    }
    Err(Error::Custom(format!("{label} falhou após {ATTEMPTS} tentativas: {last}")))
}

/// GET via `curl`. Com `jar`, escreve/lê os cookies (`-c`/`-b`) para manter a sessão.
fn curl_get(url: &str, jar: Option<&str>) -> Result<Vec<u8>> {
    with_retry(&format!("curl GET {url}"), || {
        let mut args: Vec<&str> =
            vec!["-sS", "--fail-with-body", "--max-time", "40", "-A", UA, "-H", "Accept: */*"];
        if let Some(j) = jar {
            args.extend_from_slice(&["-c", j, "-b", j]);
        }
        args.extend_from_slice(&["-o", "-", url]);
        let out = Command::new("curl")
            .args(&args)
            .output()
            .map_err(|e| Error::Custom(format!("curl indisponível no PATH? {e}")))?;
        if !out.status.success() {
            return Err(Error::Custom(format!(
                "status {} — {}",
                out.status,
                String::from_utf8_lossy(&out.stderr).trim()
            )));
        }
        Ok(out.stdout)
    })
}

/// POST `application/x-www-form-urlencoded` via `curl`, corpo pela stdin (`--data-binary @-`, sem
/// re-encode), cookies pelo `jar`.
fn curl_post_form(url: &str, body: &str, jar: &str) -> Result<Vec<u8>> {
    with_retry(&format!("curl POST {url}"), || {
        let mut child = Command::new("curl")
            .args([
                "-sS", "--fail-with-body", "--max-time", "40",
                "-A", UA, "-H", "Accept: */*",
                "-H", "Content-Type: application/x-www-form-urlencoded",
                "-b", jar, "-c", jar,
                "--data-binary", "@-",
                "-o", "-", url,
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| Error::Custom(format!("curl indisponível no PATH? {e}")))?;
        child
            .stdin
            .take()
            .ok_or_else(|| Error::Custom("stdin do curl indisponível".into()))?
            .write_all(body.as_bytes())?;
        let out = child.wait_with_output()?;
        if !out.status.success() {
            return Err(Error::Custom(format!(
                "status {} — {}",
                out.status,
                String::from_utf8_lossy(&out.stderr).trim()
            )));
        }
        Ok(out.stdout)
    })
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Extrai TODAS as linhas de resultado da listagem: `goDocument(<key>,'')">TÍTULO</a></h5><p>ASSUNTO</p>`.
/// O grupo (`CodGroup=159`, "Consultas Formais Respondidas") mistura tipos de documento
/// (PARECER, DECRETO, …); capturamos todos aqui e filtramos por PARECER em [`run`] — se filtrássemos
/// no regex, uma página só de decretos zeraria os "novos" e abortaria a paginação cedo demais.
fn parse_rows(html: &str) -> Vec<Row> {
    let re = Regex::new(
        r#"(?s)goDocument\((\d+),''\)"\s*>\s*([^<]*)</a>\s*</h5>\s*<p>\s*(.*?)\s*</p>"#,
    )
    .expect("regex de listagem válida");
    re.captures_iter(html)
        .map(|c| Row {
            inp_key: c[1].to_string(),
            numero: clean(&decode_html(c[2].trim())),
            assunto: clean(&decode_html(strip_tags(c[3].trim()).trim())),
        })
        .collect()
}

/// Um documento é uma "Consulta Formal Respondida" se o título começa por "PARECER" ou "INFORMAÇÃO"
/// (os dois tipos que compõem a coleção; guarda extra caso o filtro da busca deixe passar outro tipo).
fn is_consulta_formal(titulo: &str) -> bool {
    let t = titulo.trim_start().to_uppercase();
    t.starts_with("PARECER") || t.starts_with("INFORMA")
}

/// Corpo do parecer a partir do detalhe: texto de `#DOCContent`, preservando parágrafos e sem o
/// cabeçalho boilerplate ("Este documento foi gerado em …"), começando no "PARECER Nº".
fn parse_corpo(html: &str) -> String {
    let doc = Html::parse_document(html);
    let sel = Selector::parse("#DOCContent").expect("seletor válido");
    let Some(el) = doc.select(&sel).next() else {
        return String::new();
    };
    let text = html_to_text(&el.inner_html());
    match text.find("PARECER") {
        Some(i) => text[i..].trim().to_string(),
        None => text.trim().to_string(),
    }
}

/// HTML → texto preservando parágrafos: tags de bloco viram quebras, o resto é removido, entidades
/// decodificadas e linhas em branco excessivas colapsadas.
fn html_to_text(html: &str) -> String {
    let with_breaks = html
        .replace("</p>", "\n\n")
        .replace("</div>", "\n")
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n");
    let decoded = decode_html(&strip_tags(&with_breaks));
    let mut out = String::new();
    let mut blanks = 0;
    for l in decoded.lines() {
        let l = l.trim();
        if l.is_empty() {
            blanks += 1;
            if blanks <= 1 {
                out.push('\n');
            }
        } else {
            blanks = 0;
            out.push_str(l);
            out.push('\n');
        }
    }
    out.trim().to_string()
}

fn strip_tags(s: &str) -> String {
    Regex::new(r"(?s)<[^>]*>").expect("regex de tags válida").replace_all(s, " ").into_owned()
}

/// Entidades nomeadas frequentes no texto jurídico (o `kit::decode_entities` só cobre um punhado).
/// `&amp;` fica por último para não re-decodificar entidades duplo-escapadas.
const NAMED_ENTITIES: &[(&str, &str)] = &[
    ("&nbsp;", " "),
    ("&ndash;", "–"), ("&mdash;", "—"),
    ("&ordm;", "º"), ("&ordf;", "ª"), ("&deg;", "°"), ("&sect;", "§"),
    ("&hellip;", "…"), ("&laquo;", "«"), ("&raquo;", "»"),
    ("&aacute;", "á"), ("&agrave;", "à"), ("&acirc;", "â"), ("&atilde;", "ã"), ("&auml;", "ä"),
    ("&eacute;", "é"), ("&ecirc;", "ê"), ("&egrave;", "è"),
    ("&iacute;", "í"), ("&iuml;", "ï"),
    ("&oacute;", "ó"), ("&ocirc;", "ô"), ("&otilde;", "õ"), ("&ouml;", "ö"),
    ("&uacute;", "ú"), ("&uuml;", "ü"), ("&ugrave;", "ù"),
    ("&ccedil;", "ç"), ("&ntilde;", "ñ"),
    ("&Aacute;", "Á"), ("&Agrave;", "À"), ("&Acirc;", "Â"), ("&Atilde;", "Ã"),
    ("&Eacute;", "É"), ("&Ecirc;", "Ê"),
    ("&Iacute;", "Í"),
    ("&Oacute;", "Ó"), ("&Ocirc;", "Ô"), ("&Otilde;", "Õ"),
    ("&Uacute;", "Ú"), ("&Ccedil;", "Ç"),
    ("&quot;", "\""), ("&apos;", "'"), ("&#39;", "'"),
    ("&lt;", "<"), ("&gt;", ">"),
    ("&amp;", "&"),
];

/// Decodifica entidades HTML: numéricas (`&#186;` / `&#xBA;`) primeiro, depois as nomeadas.
fn decode_html(s: &str) -> String {
    let re = Regex::new(r"&#(x?)([0-9A-Fa-f]+);").expect("regex de entidade numérica válida");
    let s = re.replace_all(s, |c: &regex::Captures| {
        let radix = if &c[1] == "x" { 16 } else { 10 };
        u32::from_str_radix(&c[2], radix)
            .ok()
            .and_then(char::from_u32)
            .map(|ch| ch.to_string())
            .unwrap_or_else(|| c[0].to_string())
    });
    let mut out = s.into_owned();
    for (ent, rep) in NAMED_ENTITIES {
        if out.contains(ent) {
            out = out.replace(ent, rep);
        }
    }
    out
}

/// Decodifica o corpo: UTF-8 válido passa direto; inválido cai para latin-1 (o ASP clássico é
/// windows-1252, não UTF-8). As entidades HTML (`&aacute;`, …) são resolvidas depois.
fn decode_charset(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => bytes.iter().map(|&b| b as char).collect(),
    }
}

/// `application/x-www-form-urlencoded` explícito: percent-encode tudo fora do conjunto não-reservado
/// (espaço → `+`). O `__VIEWSTATE` é base64 (`+`/`/`/`=`) e precisa chegar ao ASP.NET exatamente assim.
fn form_urlencoded(pairs: &[(&str, &str)]) -> String {
    fn enc(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        for b in s.bytes() {
            match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
                b' ' => out.push('+'),
                _ => out.push_str(&format!("%{b:02X}")),
            }
        }
        out
    }
    pairs.iter().map(|(k, v)| format!("{}={}", enc(k), enc(v))).collect::<Vec<_>>().join("&")
}

fn write_temp(items: &[Parecer]) -> Result<()> {
    let mut out = String::new();
    for (i, p) in items.iter().enumerate() {
        out.push_str(&format!("// {}\n", i + 1));
        out.push_str("## pergunta:\n");
        out.push_str(&format!("descricao: {}\n", p.numero));
        out.push_str(&format!("assunto  : {}\n", p.assunto));
        out.push_str(&format!("link: {}\n", p.link));
        out.push_str("## resposta:\n");
        out.push_str(&p.corpo);
        out.push_str("\n\n");
    }
    if let Some(parent) = std::path::Path::new(OUT_PATH).parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(OUT_PATH, out)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rows_extracts_key_numero_assunto() {
        let html = r#"<h5> <a href="javascript:goDocument(299748,'')"> PARECER N&ordm; 26164</a></h5> <p> ICMS &ndash; Tratamento tribut&aacute;rio.</p>"#;
        let rows = parse_rows(html);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].inp_key, "299748");
        assert_eq!(rows[0].numero, "PARECER Nº 26164");
        assert_eq!(rows[0].assunto, "ICMS – Tratamento tributário.");
    }

    #[test]
    fn corpo_starts_at_parecer_and_preserves_paragraphs() {
        let html = r#"<div id="DOCContent"><div class="content">Este documento foi gerado em 28/07/2025.<p>PARECER Nº 25148</p><p>É o parecer.</p></div></div>"#;
        let corpo = parse_corpo(html);
        assert!(corpo.starts_with("PARECER Nº 25148"), "corpo: {corpo:?}");
        assert!(corpo.contains("É o parecer."));
    }

    #[test]
    fn decode_charset_falls_back_to_latin1() {
        assert_eq!(decode_charset("ok".as_bytes()), "ok");
        assert_eq!(decode_charset(&[b'N', 0xBA]), "Nº");
    }

    #[test]
    fn form_urlencoded_percent_encodes_base64_viewstate() {
        let body = form_urlencoded(&[("__EVENTARGUMENT", "2"), ("__VIEWSTATE", "AB+/=cd ef")]);
        assert_eq!(body, "__EVENTARGUMENT=2&__VIEWSTATE=AB%2B%2F%3Dcd+ef");
    }

    #[test]
    fn collect_form_fields_takes_hidden_and_checked_and_select() {
        let html = r#"<form>
            <input type="hidden" name="__VIEWSTATE" value="VS" />
            <input type="text" name="TxtKw" value="" />
            <input type="checkbox" name="cbOn" checked="checked" />
            <input type="checkbox" name="cbOff" />
            <input type="submit" name="Btn" value="Buscar" />
            <select name="Ddl"><option value="a">A</option><option value="b" selected="selected">B</option></select>
        </form>"#;
        let doc = Html::parse_document(html);
        let f = collect_form_fields(&doc);
        assert!(f.contains(&("__VIEWSTATE".into(), "VS".into())));
        assert!(f.contains(&("TxtKw".into(), "".into())));
        assert!(f.contains(&("cbOn".into(), "on".into())));
        assert!(!f.iter().any(|(k, _)| k == "cbOff")); // unchecked -> omitido
        assert!(!f.iter().any(|(k, _)| k == "Btn")); // submit -> omitido
        assert!(f.contains(&("Ddl".into(), "b".into()))); // selected
    }
}
