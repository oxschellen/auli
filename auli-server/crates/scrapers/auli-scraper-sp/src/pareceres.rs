//! pareceres — **Respostas de Consultas (RC)** da SEFAZ-SP, no Portal da Legislação
//! (`legislacao.fazenda.sp.gov.br`, SharePoint).
//!
//! FONTE (doutrina "API JSON > HTML" — como o catálogo de serviços do SP):
//! as RCs moram na biblioteca **Páginas** (lista SharePoint `{F286D0D1-…}`, ~27 k itens). A REST
//! `_api` anônima devolve, por item e como colunas, TUDO o que precisamos — sem buscar página a
//! página:
//!
//! - `FileLeafRef` → nome do arquivo `RC{numero}_{ano}.aspx` (numero/ano = chave natural)
//! - `Ementa` → a ementa (dentro de `<div class='sefazElement-EmentaRC'>`)
//! - `PublishingPageContent` → o corpo integral (Relato + Interpretação)
//!
//! A biblioteca tem também páginas de sistema (`Atos.aspx`, `Search.aspx`…); filtramos as RCs pelo
//! padrão do `FileLeafRef` no cliente.
//!
//! PAGINAÇÃO: o filtro por `FileLeafRef` (campo não indexado, >5000 itens) estoura o *list view
//! threshold* do SharePoint; então paginamos por `$orderby=Id` + `$skiptoken` (via `odata.nextLink`),
//! que é seguro, e filtramos as RCs no cliente. ~136 páginas de 200 → poucos minutos.
//!
//! ESCOPO: RCs de 2012+ (início do acervo do portal). `Ementa` = assunto (vetorizado); corpo =
//! `PublishingPageContent` (armazenado com todos os detalhes). Grava só o intermediário
//! `../data/sp/ref/sp-pareceres-temp.txt` (numero/assunto/corpo/link, **sem `resumo`**). Não toca
//! contrato/snapshot/collections.
//!
//! ACESSO: sem robots.txt (SharePoint 404), páginas `/Paginas/` públicas; coleta pública. UA
//! institucional AuliBot + cortesia 1 s. Cache local por página (re-runs instantâneos).

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::LazyLock;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use regex::Regex;
use serde::Deserialize;

use auli_scraper_kit::{
    build_agent, clean,
    http::{GetOpts, get_string},
};

const HOST: &str = "https://legislacao.fazenda.sp.gov.br";
const LIST_ID: &str = "F286D0D1-5624-47DA-A856-A8571296EB7F";
const OUT_PATH: &str = "../data/sp/ref/sp-pareceres-temp.txt";
const CACHE_DIR: &str = "../data/sp/raw/cache/pareceres";
const UA: &str = "AuliBot/0.1 (+https://github.com/oxschellen/auli; carlos.schellenberger@gmail.com)";
const COURTESY: Duration = Duration::from_millis(1000);
const PAGE_SIZE: usize = 200;
const MIN_YEAR: i32 = 2012;
const MIN_PARECERES: usize = 10000; // guarda contra coleta truncada (esperado ~15–20 k)

static RE_RC: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^RC(\d+)_(\d{4})\.aspx$").expect("regex RC inválida"));
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

/// Um item da biblioteca Páginas (só as colunas que selecionamos).
#[derive(Deserialize)]
struct Item {
    #[serde(rename = "FileLeafRef")]
    file: Option<String>,
    #[serde(rename = "Ementa")]
    ementa: Option<String>,
    #[serde(rename = "PublishingPageContent")]
    corpo: Option<String>,
}

/// Uma página de resultados da REST (odata=nometadata).
#[derive(Deserialize)]
struct Page {
    value: Vec<Item>,
    #[serde(rename = "odata.nextLink")]
    next: Option<String>,
}

/// Uma RC coletada (intermediário; sem `resumo` autorado).
struct Parecer {
    numero: String,
    assunto: String,
    corpo: String,
    link: String,
}

pub fn run(use_cache: bool) -> Result<()> {
    let agent = build_agent(UA, Some(Duration::from_secs(90)));
    let mut url = format!(
        "{HOST}/_api/web/lists/getbyid('{LIST_ID}')/items\
         ?$select=FileLeafRef,Ementa,PublishingPageContent&$top={PAGE_SIZE}&$orderby=Id"
    );

    let mut seen: HashSet<String> = HashSet::new();
    let mut items: Vec<Parecer> = Vec::new();
    let mut pagina = 0usize;

    loop {
        let body = fetch(&agent, &url, pagina, use_cache)?;
        let page: Page = serde_json::from_str(&body)
            .map_err(|e| anyhow!("JSON inválido na página {pagina}: {e}"))?;

        for it in page.value {
            let Some(file) = it.file else { continue };
            let Some(caps) = RE_RC.captures(&file) else { continue }; // só RC{n}_{ano}.aspx
            let ano: i32 = caps[2].parse().unwrap_or(0);
            if ano < MIN_YEAR {
                continue;
            }
            if !seen.insert(file.clone()) {
                continue;
            }
            let corpo = parse_corpo(it.corpo.as_deref().unwrap_or(""));
            if corpo.is_empty() {
                continue;
            }
            let assunto = clean(&decode_html(&strip_tags(it.ementa.as_deref().unwrap_or(""))));
            items.push(Parecer {
                numero: format!("RESPOSTA À CONSULTA TRIBUTÁRIA {}/{}", &caps[1], ano),
                assunto,
                corpo,
                link: format!("{HOST}/Paginas/{file}"),
            });
        }

        pagina += 1;
        if pagina.is_multiple_of(10) {
            println!("  … página {pagina}, {} RCs até agora", items.len());
        }
        match page.next {
            Some(n) if !n.is_empty() => {
                url = if n.starts_with("http") { n } else { format!("{HOST}/{}", n.trim_start_matches('/')) };
            }
            _ => break,
        }
        if !use_cache {
            sleep(COURTESY);
        }
    }

    println!("📇 {} RCs de {}+ ({pagina} páginas da REST).", items.len(), MIN_YEAR);
    if items.len() < MIN_PARECERES {
        bail!(
            "coleta devolveu {} RCs (< MIN {MIN_PARECERES}); abortando para não truncar.",
            items.len()
        );
    }

    write_temp(&items)?;
    println!(
        "✅ Escrito {OUT_PATH} ({} RCs). O estágio de resumo autorado é posterior.",
        items.len()
    );
    Ok(())
}

/// Cache local por página (a nextLink/`$skiptoken` é determinística por Id → cache por índice de página).
fn cache_path(pagina: usize) -> PathBuf {
    PathBuf::from(CACHE_DIR).join(format!("page-{pagina:04}.json"))
}

fn fetch(agent: &ureq::Agent, url: &str, pagina: usize, use_cache: bool) -> Result<String> {
    let path = cache_path(pagina);
    if let Ok(s) = std::fs::read_to_string(&path)
        && !s.trim().is_empty()
    {
        return Ok(s);
    }
    if use_cache {
        bail!("cache miss para página {pagina} (modo --usecache, sem rede)");
    }
    let opts = GetOpts {
        log_prefix: "SP-RC",
        accept: Some("application/json;odata=nometadata"),
        ..GetOpts::default()
    };
    let body = get_string(agent, url, &opts)?;
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&path, &body);
    Ok(body)
}

/// Remove todas as tags HTML.
fn strip_tags(s: &str) -> String {
    RE_TAG.replace_all(s, " ").into_owned()
}

/// HTML (Word/Office export) → texto integral legível: fora head/script/style/comentários, blocos
/// viram quebra de linha, tira tags, decodifica entidades, normaliza o espaçamento.
fn parse_corpo(html: &str) -> String {
    let mut s = html.to_string();
    for re in [&*RE_HEAD, &*RE_SCRIPT, &*RE_STYLE, &*RE_COMMENT] {
        s = re.replace_all(&s, " ").into_owned();
    }
    s = RE_BLOCK.replace_all(&s, "\n").into_owned();
    normalize_lines(&decode_html(&strip_tags(&s)))
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

/// Entidades nomeadas relevantes (Latin-1/pt-BR + pontuação). `&amp;` por último.
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

fn write_temp(items: &[Parecer]) -> Result<()> {
    let mut out = String::new();
    for (i, p) in items.iter().enumerate() {
        out.push_str(&format!("// {}\n", i + 1));
        out.push_str("## pergunta:\n");
        out.push_str(&format!("descricao: {}\n", p.numero));
        out.push_str(&format!("assunto  : {}\n", p.assunto));
        out.push_str(&format!("link: {}\n", p.link));
        out.push_str("## resposta:\n");
        out.push_str(p.corpo.trim());
        out.push_str("\n\n");
    }
    if let Some(parent) = std::path::Path::new(OUT_PATH).parent() {
        std::fs::create_dir_all(parent).map_err(|e| anyhow!("criar dir de {OUT_PATH}: {e}"))?;
    }
    std::fs::write(OUT_PATH, out).map_err(|e| anyhow!("gravar {OUT_PATH}: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rc_filename_matches_and_extracts() {
        let c = RE_RC.captures("RC29588_2024.aspx").unwrap();
        assert_eq!(&c[1], "29588");
        assert_eq!(&c[2], "2024");
        assert!(RE_RC.captures("Atos.aspx").is_none());
        assert!(RE_RC.captures("Search.aspx").is_none());
        assert!(RE_RC.captures("RC_2024.aspx").is_none());
    }

    #[test]
    fn ementa_strips_div_and_decodes() {
        let raw = "<div class='sefazElement-EmentaRC'><p>ICMS &#8211; substitui&ccedil;&atilde;o.</p></div>";
        assert_eq!(clean(&decode_html(&strip_tags(raw))), "ICMS – substituição.");
    }

    #[test]
    fn corpo_strips_style_and_keeps_sections() {
        // O PublishingPageContent traz CSS inline do Word (<style> p.MsoNormal {…}) — não pode vazar.
        let raw = "<style>p.MsoNormal { margin-top:0cm; font-size:11.0pt; }</style>\
                   <p>RESPOSTA À CONSULTA 1/2024</p><p>Relato</p><p>A consulente &eacute; ind&uacute;stria.</p>";
        let corpo = parse_corpo(raw);
        assert!(!corpo.contains("MsoNormal"), "CSS vazou: {corpo}");
        assert!(!corpo.contains("margin-top"));
        assert!(corpo.starts_with("RESPOSTA À CONSULTA 1/2024"));
        assert!(corpo.contains("Relato"));
        assert!(corpo.contains("A consulente é indústria."));
    }

    #[test]
    fn page_json_parses_value_and_nextlink() {
        let j = r#"{"value":[{"FileLeafRef":"RC1_2024.aspx","Ementa":"<p>x</p>","PublishingPageContent":"<p>corpo</p>"}],"odata.nextLink":"https://h/_api/next"}"#;
        let p: Page = serde_json::from_str(j).unwrap();
        assert_eq!(p.value.len(), 1);
        assert_eq!(p.value[0].file.as_deref(), Some("RC1_2024.aspx"));
        assert_eq!(p.next.as_deref(), Some("https://h/_api/next"));
    }
}
