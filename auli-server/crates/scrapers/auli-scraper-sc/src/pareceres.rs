//! pareceres — Consultas da **COPAT** (Comissão Permanente de Assuntos Tributários) da SEF-SC,
//! no Portal de Legislação (`legislacao.sef.sc.gov.br`, ASP.NET WebForms / IIS).
//!
//! FONTE (molde **DF** — 1 GET pega o índice inteiro, sem paginação/sessão):
//! - Índice `Copat.aspx`: UMA resposta (~6.8 MB) com TODAS as consultas num acordeão por ano.
//!   Cada linha do painel do ano é
//!   `<tr><td><a href="DocumentoLegalViewer.ashx?id=GUID">NNNN/YY</a></td><td>EMENTA</td></tr>`
//!   → numero + guid + ementa. (Há também um "índice analítico" por assunto, cujas linhas têm o
//!   texto do assunto no 1º `<td>` e os links no 2º; nós só pegamos linhas cujo 1º `<td>` é o número
//!   linkado, o que naturalmente ignora o índice analítico e dá 1 linha por consulta.)
//! - Detalhe `DocumentoLegalViewer.ashx?id=GUID`: HTML (Word-export) com o parecer inteiro
//!   (Ementa / Da Consulta / Legislação / Fundamentação / Resposta + Nº Processo).
//!
//! ESCOPO: consultas de **2011+** (era do formato publicado na íntegra, cf. aviso do portal —
//! LC 313/05 art. 32 §2º). `ementa` = assunto (o campo que será vetorizado); `corpo` = texto
//! integral do detalhe (armazenado com todos os detalhes). Grava só o intermediário
//! `../data/sc/ref/sc-pareceres-temp.txt` (numero/assunto/corpo/link, **sem `resumo`** — o resumo
//! é estágio posterior). Não toca contrato/snapshot/collections.
//!
//! ROBOTS/ACESSO: o site **não tem robots.txt** (404) e a página é `/Publico/`; a coleta destas
//! consultas **públicas** foi solicitada por auditor da NAVI. UA institucional AuliBot + cortesia
//! (1 s) entre requisições (portal de outro ente — cautela).

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::LazyLock;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Result, bail};
use regex::Regex;

use auli_scraper_kit::{
    build_agent, clean,
    http::{GetOpts, get_string},
};

// Regexes compiladas uma única vez (evita recompilar em loop).
static RE_INDEX_ROW: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?s)<td[^>]*>\s*<a[^>]*href="[^"]*DocumentoLegalViewer\.ashx\?id=([0-9A-Fa-f-]+)"[^>]*>\s*(\d{3,4}/\d{2})\s*</a>\s*</td>\s*<td[^>]*>(.*?)</td>"#,
    )
    .expect("regex do índice inválida")
});
static RE_TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)<[^>]+>").unwrap());
static RE_HEAD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?is)<head[^>]*>.*?</head>").unwrap());
static RE_SCRIPT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?is)<script[^>]*>.*?</script>").unwrap());
static RE_STYLE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?is)<style[^>]*>.*?</style>").unwrap());
static RE_COMMENT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)<!--.*?-->").unwrap());
static RE_BLOCK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)</(p|div|tr|h[1-6]|li)>|<br\s*/?>").unwrap());
static RE_WS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[ \t\u{00a0}]+").unwrap());
static RE_DEC: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"&#(\d+);").unwrap());
static RE_HEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"&#[xX]([0-9A-Fa-f]+);").unwrap());

const INDEX_URL: &str = "https://legislacao.sef.sc.gov.br/Consulta/Views/Publico/Copat.aspx";
const DETAIL_BASE: &str =
    "https://legislacao.sef.sc.gov.br/Consulta/Views/Publico/DocumentoLegalViewer.ashx?id=";
/// Árvore de documentos (G5): um `.md` por consulta inédita. Fonte a partir da G5b.
const DOCS_DIR: &str = "../data/sc/docs/pareceres";
const CACHE_DIR: &str = "../data/sc/raw/cache/pareceres";
const UA: &str = "AuliBot/0.1 (+https://github.com/oxschellen/auli; carlos.schellenberger@gmail.com)";
const COURTESY: Duration = Duration::from_millis(1000);
const MIN_YEAR: i32 = 2011;
const MIN_PARECERES: usize = 1500; // guarda contra coleta truncada (esperado ~1744 em 2011+)

/// Uma linha do índice: número humano, GUID do detalhe e a ementa (assunto).
struct Row {
    numero: String,
    guid: String,
    ementa: String,
}

/// Um parecer coletado (intermediário; sem `resumo` autorado).
struct Parecer {
    numero: String,
    assunto: String,
    corpo: String,
    link: String,
}

pub fn run(use_cache: bool) -> Result<()> {
    let agent = build_agent(UA, Some(Duration::from_secs(90)));

    // 1) Índice: 1 único GET (ou cache) devolve todas as consultas.
    let index_html = fetch(&agent, INDEX_URL, use_cache)?;
    let rows = parse_index(&index_html);
    println!(
        "📇 Índice: {} consultas de {}+ (após dedup por GUID e filtro de ano).",
        rows.len(),
        MIN_YEAR
    );
    if rows.len() < MIN_PARECERES {
        bail!(
            "índice devolveu {} consultas (< MIN {MIN_PARECERES}); abortando para não truncar.",
            rows.len()
        );
    }

    // 2) Detalhe por consulta: ementa (do índice) = assunto; corpo = texto integral do detalhe.
    let total = rows.len();
    let mut items = Vec::with_capacity(total);
    for (i, row) in rows.iter().enumerate() {
        let url = format!("{DETAIL_BASE}{}", row.guid);
        let html = fetch(&agent, &url, use_cache)?;
        let corpo = parse_corpo(&html);
        items.push(Parecer {
            numero: row.numero.clone(),
            assunto: row.ementa.clone(),
            corpo,
            link: url,
        });
        if (i + 1) % 50 == 0 || i + 1 == total {
            println!("  … {} / {}", i + 1, total);
        }
        if !use_cache {
            sleep(COURTESY);
        }
    }

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
    Ok(())
}

/// Cache local (o do kit é hardcoded para `servicos`): lê/grava por URL sanitizada em [`CACHE_DIR`].
fn cache_path(url: &str) -> PathBuf {
    let name: String = url
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') { c } else { '_' })
        .collect();
    PathBuf::from(CACHE_DIR).join(format!("{name}.html"))
}

/// GET com cache local. `use_cache` exige hit (sem rede); senão vai à rede e grava o cache.
fn fetch(agent: &ureq::Agent, url: &str, use_cache: bool) -> Result<String> {
    let path = cache_path(url);
    if let Ok(s) = std::fs::read_to_string(&path)
        && !s.trim().is_empty()
    {
        return Ok(s);
    }
    if use_cache {
        bail!("cache miss para {url} (modo --usecache, sem rede)");
    }
    let opts = GetOpts { log_prefix: "SC-par", ..GetOpts::default() };
    let body = get_string(agent, url, &opts)?;
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&path, &body);
    Ok(body)
}

/// Extrai as linhas do índice: cada `<td>` inicial que é um único link de detalhe com número
/// `NNNN/YY`, seguido do `<td>` da ementa. Dedup por GUID; filtra para `>= MIN_YEAR`.
fn parse_index(html: &str) -> Vec<Row> {
    // 1º <td> = SÓ o link numerado (painel do ano); 2º <td> = ementa. As linhas do índice
    // analítico têm texto no 1º <td> (sem link) e não casam.
    let mut seen = HashSet::new();
    let mut rows = Vec::new();
    for c in RE_INDEX_ROW.captures_iter(html) {
        let guid = c[1].to_uppercase();
        let num_raw = c[2].to_string();
        if !seen.insert(guid.clone()) {
            continue; // consulta já vista (aparece também no índice analítico)
        }
        if year_of(&num_raw) < MIN_YEAR {
            continue;
        }
        let ementa = clean(&decode_html(&strip_tags(&c[3])));
        if ementa.is_empty() {
            continue;
        }
        rows.push(Row {
            numero: format!("CONSULTA COPAT nº {num_raw}"),
            guid,
            ementa,
        });
    }
    rows
}

/// Ano de um número `NNNN/YY` (2000+YY para YY<80; senão 1900+YY).
fn year_of(numero: &str) -> i32 {
    numero
        .split('/')
        .nth(1)
        .and_then(|yy| yy.parse::<i32>().ok())
        .map(|yy| if yy < 80 { 2000 + yy } else { 1900 + yy })
        .unwrap_or(0)
}

/// Texto integral do detalhe: remove head/script/style/comentários, transforma blocos em quebras
/// de linha, tira tags, decodifica entidades e normaliza o espaçamento — preservando parágrafos.
fn parse_corpo(html: &str) -> String {
    let mut s = html.to_string();
    // Fora head/script/style e comentários condicionais do Word.
    for re in [&*RE_HEAD, &*RE_SCRIPT, &*RE_STYLE, &*RE_COMMENT] {
        s = re.replace_all(&s, " ").into_owned();
    }
    // Blocos viram quebra de linha para preservar a legibilidade das seções.
    s = RE_BLOCK.replace_all(&s, "\n").into_owned();
    let text = decode_html(&strip_tags(&s));
    normalize_lines(&text)
}

/// Remove todas as tags HTML.
fn strip_tags(s: &str) -> String {
    RE_TAG.replace_all(s, " ").into_owned()
}

/// Colapsa espaços por linha e limita a no máximo uma linha em branco entre parágrafos.
fn normalize_lines(s: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut blank = false;
    for raw in s.split('\n') {
        let line = RE_WS.replace_all(raw.trim(), " ").trim().to_string();
        if line.is_empty() {
            if !blank && !out.is_empty() {
                out.push(String::new());
            }
            blank = true;
        } else {
            out.push(line);
            blank = false;
        }
    }
    while out.last().is_some_and(|l| l.is_empty()) {
        out.pop();
    }
    out.join("\n")
}

/// Decodifica entidades HTML (nomeadas comuns pt-BR + numéricas decimais e hexadecimais).
fn decode_html(s: &str) -> String {
    // Numéricas: &#211; e &#xD3;
    let s = RE_DEC.replace_all(s, |c: &regex::Captures| {
        c[1].parse::<u32>().ok().and_then(char::from_u32).map(String::from).unwrap_or_default()
    });
    let s = RE_HEX.replace_all(&s, |c: &regex::Captures| {
        u32::from_str_radix(&c[1], 16).ok().and_then(char::from_u32).map(String::from).unwrap_or_default()
    });
    let mut s = s.into_owned();
    for (ent, ch) in NAMED {
        if s.contains(ent) {
            s = s.replace(ent, ch);
        }
    }
    s
}

/// Entidades nomeadas relevantes (Latin-1/pt-BR + pontuação). `&amp;` por último ao aplicar.
const NAMED: &[(&str, &str)] = &[
    ("&nbsp;", " "), ("&aacute;", "á"), ("&Aacute;", "Á"), ("&agrave;", "à"), ("&Agrave;", "À"),
    ("&acirc;", "â"), ("&Acirc;", "Â"), ("&atilde;", "ã"), ("&Atilde;", "Ã"), ("&auml;", "ä"),
    ("&eacute;", "é"), ("&Eacute;", "É"), ("&ecirc;", "ê"), ("&Ecirc;", "Ê"), ("&egrave;", "è"),
    ("&iacute;", "í"), ("&Iacute;", "Í"), ("&icirc;", "î"), ("&oacute;", "ó"), ("&Oacute;", "Ó"),
    ("&ocirc;", "ô"), ("&Ocirc;", "Ô"), ("&otilde;", "õ"), ("&Otilde;", "Õ"), ("&ouml;", "ö"),
    ("&uacute;", "ú"), ("&Uacute;", "Ú"), ("&ucirc;", "û"), ("&uuml;", "ü"), ("&Uuml;", "Ü"),
    ("&ccedil;", "ç"), ("&Ccedil;", "Ç"), ("&ntilde;", "ñ"), ("&Ntilde;", "Ñ"),
    ("&ordf;", "ª"), ("&ordm;", "º"), ("&deg;", "°"), ("&sect;", "§"), ("&middot;", "·"),
    ("&hellip;", "…"), ("&ndash;", "–"), ("&mdash;", "—"), ("&laquo;", "«"), ("&raquo;", "»"),
    ("&lsquo;", "‘"), ("&rsquo;", "’"), ("&ldquo;", "“"), ("&rdquo;", "”"),
    ("&quot;", "\""), ("&apos;", "'"), ("&#39;", "'"), ("&lt;", "<"), ("&gt;", ">"),
    ("&amp;", "&"),
];


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn year_of_maps_suffix_to_full_year() {
        assert_eq!(year_of("0037/26"), 2026);
        assert_eq!(year_of("0071/06"), 2006);
        assert_eq!(year_of("0012/99"), 1999);
    }

    #[test]
    fn parse_index_takes_year_panel_rows_and_skips_analytic_index() {
        // Linha do painel do ano (número linkado no 1º td) — deve entrar.
        // Linha do índice analítico (assunto no 1º td, link no 2º) — deve ser ignorada.
        let html = r#"
          <tr><td><a href="DocumentoLegalViewer.ashx?id=AAAA-1">0037/26</a></td>
              <td>ICMS. SUBSTITUI&Ccedil;&Atilde;O TRIBUT&Aacute;RIA.</td></tr>
          <tr><td>Al&iacute;quotas seletivas</td>
              <td><a href="DocumentoLegalViewer.ashx?id=BBBB-2">0071/06</a></td></tr>
          <tr><td><a href="DocumentoLegalViewer.ashx?id=CCCC-3">0005/10</a></td>
              <td>Antes de 2011 — filtrada.</td></tr>
        "#;
        let rows = parse_index(html);
        assert_eq!(rows.len(), 1, "só a linha 2026 do painel do ano");
        assert_eq!(rows[0].numero, "CONSULTA COPAT nº 0037/26");
        assert_eq!(rows[0].guid, "AAAA-1");
        assert_eq!(rows[0].assunto_or_ementa(), "ICMS. SUBSTITUIÇÃO TRIBUTÁRIA.");
    }

    // helper de teste (o campo é `ementa`)
    impl Row {
        fn assunto_or_ementa(&self) -> &str {
            &self.ementa
        }
    }

    #[test]
    fn parse_corpo_strips_word_html_and_keeps_sections() {
        let html = r#"<html><head><style>p{mso-x:1}</style></head><body>
          <!--[if gte mso 9]><xml>junk</xml><![endif]-->
          <p>Ementa</p><p>ICMS. CR&Eacute;DITO PRESUMIDO.</p>
          <p>Da Consulta</p><p>A consulente &eacute; ind&uacute;stria.</p></body></html>"#;
        let corpo = parse_corpo(html);
        assert!(corpo.contains("Ementa"));
        assert!(corpo.contains("ICMS. CRÉDITO PRESUMIDO."));
        assert!(corpo.contains("A consulente é indústria."));
        assert!(!corpo.contains("mso-x"));
        assert!(!corpo.contains("junk"));
    }

    #[test]
    fn decode_html_handles_named_and_numeric() {
        assert_eq!(decode_html("A&ccedil;&atilde;o"), "Ação");
        assert_eq!(decode_html("ACESS&#211;RIOS"), "ACESSÓRIOS");
        assert_eq!(decode_html("&#xD3;timo"), "Ótimo");
        assert_eq!(decode_html("a &amp; b"), "a & b");
    }
}
