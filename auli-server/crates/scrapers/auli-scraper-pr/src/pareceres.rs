//! pareceres — **Consultas Tributárias** do Setor Consultivo da SEFA-PR.
//!
//! FONTE (diferente de SC/SP — é PDF, não HTML): a página `_l_DownloadLegislacao2.asp?eTpDoc=16`
//! lista UM PDF por ano (2007→…), cada um uma **compilação anual** com TODAS as consultas daquele
//! ano (ex.: 2007 = 613 páginas, 137 consultas). Os PDFs moram em
//! `sefanet.pr.gov.br/dados/SEFADOCUMENTOS/<code>.pdf` (nome irregular — extraído da página, não
//! construído).
//!
//! ESTRATÉGIA: baixa cada PDF anual → `pdftotext -layout` (subprocesso poppler) → tira a mobília de
//! página que se repete (cabeçalho SEFA/SETOR CONSULTIVO, réguas `___`, números de página) → divide
//! no marcador `CONSULTA Nº: NN, de <data>`. Por consulta: `assunto` = a SÚMULA (ementa, em CAIXA
//! ALTA); `corpo` = o texto integral da consulta; `link` = o PDF anual (não há link por consulta —
//! elas vêm agregadas). Dedup por (ano, número).
//!
//! ESCOPO: grava só o intermediário `../data/pr/ref/pr-pareceres-temp.txt` (numero/assunto/corpo/link,
//! sem `resumo`). Não toca contrato/snapshot/collections. ROBOTS: sem robots.txt (IIS 404); documentos
//! públicos. UA AuliBot + cortesia 1 s entre PDFs. Cache local do TEXTO por ano (re-runs instantâneos).

use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Command;
use std::sync::LazyLock;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use regex::Regex;

use auli_scraper_kit::{
    build_agent, clean,
    http::{GetOpts, get_string},
};

const INDEX_URL: &str = "https://www.arinternet.pr.gov.br/portalsefa/_l_DownloadLegislacao2.asp?eTpDoc=16&eTpPer=9&eDtPublicacaoIni=&eDtPublicacaoFim=&eNrDocumento=&eAnoDocumento=&eTpMod=1";
const OUT_PATH: &str = "../data/pr/ref/pr-pareceres-temp.txt";
const CACHE_DIR: &str = "../data/pr/raw/cache/pareceres";
const UA: &str = "AuliBot/0.1 (+https://github.com/oxschellen/auli; carlos.schellenberger@gmail.com)";
const COURTESY: Duration = Duration::from_millis(1000);
const MIN_PARECERES: usize = 1500; // guarda contra coleta truncada (20 anos × ~130)

/// Link para um PDF anual: `.pdf` (grupo 1). Extraímos da página; não construímos.
static RE_PDF: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"href="([^"]+\.pdf)""#).unwrap());
/// Marcador de consulta no texto do PDF: `CONSULTA Nº: NN, de <data>` (âncora de linha + `, de`).
static RE_MARK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*CONSULTA\s+N[ºo°]\s*:?\s*(\d+)\s*,\s*de\b").unwrap()
});
static RE_WS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[ \t]+").unwrap());

/// Uma consulta coletada (intermediário; sem `resumo`).
struct Parecer {
    numero: String,
    assunto: String,
    corpo: String,
    link: String,
}

pub fn run(use_cache: bool) -> Result<()> {
    let agent = build_agent(UA, Some(Duration::from_secs(60)));

    // 1) Índice: 1 GET da página ASP → (ano, url do PDF).
    let index_html = {
        let opts = GetOpts { log_prefix: "PR-par", ..GetOpts::default() };
        get_string(&agent, INDEX_URL, &opts)?
    };
    let anos = parse_index(&index_html);
    println!("📇 {} PDFs anuais no índice.", anos.len());
    if anos.is_empty() {
        bail!("nenhum PDF anual no índice — a página mudou?");
    }

    // 2) Por ano: baixa o PDF, extrai texto, divide em consultas.
    let mut seen: HashSet<(i32, u32)> = HashSet::new();
    let mut items: Vec<Parecer> = Vec::new();
    for (ano, url) in &anos {
        let text = fetch_year_text(url, *ano, use_cache)?;
        let antes = items.len();
        for p in split_consultas(&text, *ano, url) {
            // dedup por (ano, número)
            let n: u32 = p.numero_n;
            if seen.insert((*ano, n)) {
                items.push(p.parecer);
            }
        }
        println!("  {ano}: +{} consultas ({} no total)", items.len() - antes, items.len());
        if !use_cache {
            sleep(COURTESY);
        }
    }

    if items.len() < MIN_PARECERES {
        bail!(
            "coleta devolveu {} consultas (< MIN {MIN_PARECERES}); abortando para não truncar.",
            items.len()
        );
    }
    write_temp(&items)?;
    println!(
        "✅ Escrito {OUT_PATH} ({} consultas). O estágio de resumo autorado é posterior.",
        items.len()
    );
    Ok(())
}

/// Extrai (ano, url_https) dos links `.pdf` do índice. Ano vem do nome do arquivo (…AAAA????.pdf) ou
/// do número `0NNNN/AAAA` na mesma linha; usamos os 4 dígitos de ano presentes no href.
fn parse_index(html: &str) -> Vec<(i32, String)> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for c in RE_PDF.captures_iter(html) {
        let href = &c[1];
        // ano = últimos 4 dígitos antes de ".pdf" (o nome termina em …AAAA.pdf).
        let digits: String = href.chars().filter(|c| c.is_ascii_digit()).collect();
        let ano = digits
            .get(digits.len().saturating_sub(4)..)
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(0);
        if !(1990..=2100).contains(&ano) {
            continue;
        }
        if !seen.insert(ano) {
            continue;
        }
        let url = if href.starts_with("http://") {
            href.replacen("http://", "https://", 1)
        } else {
            href.to_string()
        };
        out.push((ano, url));
    }
    out.sort_by_key(|(a, _)| *a);
    out
}

/// Baixa o PDF anual (curl) e extrai o texto (`pdftotext -layout`), com cache do TEXTO por ano.
fn fetch_year_text(url: &str, ano: i32, use_cache: bool) -> Result<String> {
    let txt_path = PathBuf::from(CACHE_DIR).join(format!("{ano}.txt"));
    if let Ok(s) = std::fs::read_to_string(&txt_path)
        && !s.trim().is_empty()
    {
        println!("Cache hit: {ano}");
        return Ok(s);
    }
    if use_cache {
        bail!("cache miss para {ano} (modo --usecache, sem rede)");
    }
    std::fs::create_dir_all(CACHE_DIR).ok();
    let pdf_path = PathBuf::from(CACHE_DIR).join(format!("{ano}.pdf"));

    // Download via curl (segue redirects; PDFs grandes).
    println!("Baixando PDF {ano}: {url}");
    let dl = Command::new("curl")
        .args([
            "-sSL", "--fail", "--max-time", "180", "-A", UA, "-o",
            pdf_path.to_str().unwrap(), url,
        ])
        .status()
        .map_err(|e| anyhow!("curl falhou ({ano}): {e}"))?;
    if !dl.success() {
        bail!("curl não baixou o PDF de {ano} (status {dl})");
    }

    // pdftotext -layout <pdf> - (para stdout).
    let out = Command::new("pdftotext")
        .args(["-layout", "-enc", "UTF-8", pdf_path.to_str().unwrap(), "-"])
        .output()
        .map_err(|e| anyhow!("pdftotext falhou ({ano}) — poppler instalado? {e}"))?;
    if !out.status.success() {
        bail!("pdftotext erro ({ano}): {}", String::from_utf8_lossy(&out.stderr));
    }
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    std::fs::write(&txt_path, &text).ok();
    Ok(text)
}

/// `true` se a linha é mobília de página repetida (cabeçalho SEFA, régua, número de página solto).
fn is_furniture(line: &str) -> bool {
    let t = line.trim();
    t.is_empty()
        || t.contains("SECRETARIA DE ESTADO DA FAZENDA")
        || t.contains("SETOR CONSULTIVO")
        || t.chars().all(|c| c == '_')
        || t.chars().all(|c| c.is_ascii_digit()) // número de página solto
}

/// Divide o texto do PDF anual em consultas pelo marcador `CONSULTA Nº: NN, de …`.
fn split_consultas(text: &str, ano: i32, link: &str) -> Vec<ParecerN> {
    // Linhas úteis (sem mobília).
    let lines: Vec<&str> = text.lines().filter(|l| !is_furniture(l)).collect();
    let joined = lines.join("\n");

    // Posições dos marcadores no texto sem mobília.
    let marks: Vec<(usize, u32)> = RE_MARK
        .captures_iter(&joined)
        .filter_map(|c| {
            let n: u32 = c[1].parse().ok()?;
            Some((c.get(0).unwrap().start(), n))
        })
        .collect();

    let mut out = Vec::new();
    for i in 0..marks.len() {
        let start = marks[i].0;
        let end = marks.get(i + 1).map(|m| m.0).unwrap_or(joined.len());
        let block = &joined[start..end];
        let numero_n = marks[i].1;
        let assunto = extract_sumula(block);
        let corpo = normalize(block);
        if corpo.is_empty() {
            continue;
        }
        out.push(ParecerN {
            numero_n,
            parecer: Parecer {
                numero: format!("CONSULTA Nº {numero_n}/{ano}"),
                assunto,
                corpo,
                link: link.to_string(),
            },
        });
    }
    out
}

/// Rótulo da ementa em CAIXA ALTA no topo da consulta. Consultas recentes usam `SÚMULA:`; as antigas
/// (2007–2008) usam `ASSUNTO:`; raras usam `EMENTA:`. Casamos no INÍCIO da linha (já aparada) para não
/// pegar a palavra no meio da prosa.
fn is_ementa_label(l: &str) -> bool {
    let u = l.to_uppercase();
    u.starts_with("SÚMULA")
        || u.starts_with("SUMULA")
        || u.starts_with("ASSUNTO")
        || u.starts_with("EMENTA")
}

/// Linha de METADADO em CAIXA ALTA que segue a ementa (relator, consulente, protocolo) e NÃO faz parte
/// dela — encerra a continuação em maiúsculas para não poluir o `assunto`.
fn is_metadata_label(l: &str) -> bool {
    let u = l.to_uppercase();
    u.starts_with("RELATOR")
        || u.starts_with("CONSULENTE")
        || u.starts_with("INTERESSAD")
        || u.starts_with("PROTOCOLO")
        || u.starts_with("SID ")
        || u.starts_with("SID N")
}

/// Extrai a ementa (em CAIXA ALTA) de um bloco de consulta: do rótulo (`SÚMULA:`/`ASSUNTO:`/`EMENTA:`)
/// enquanto as linhas forem predominantemente maiúsculas; para na primeira linha de prosa (corpo).
/// Consultas antigas republicadas / canceladas não têm rótulo — nesse caso, cai na 1ª frase da narrativa.
fn extract_sumula(block: &str) -> String {
    let lines: Vec<&str> = block.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    if let Some(s) = lines.iter().position(|l| is_ementa_label(l)) {
        let mut parts: Vec<String> = Vec::new();
        if let Some((_, rest)) = lines[s].split_once(':')
            && !rest.trim().is_empty()
        {
            parts.push(rest.trim().to_string());
        }
        for l in &lines[s + 1..] {
            if is_metadata_label(l) {
                break;
            }
            if maiuscula(l) {
                parts.push((*l).to_string());
            } else {
                break;
            }
        }
        // Remove tokens de 1 letra soltos no fim (ex.: o "A" que abre o corpo "A Consulente…").
        let mut joined = clean(&parts.join(" "));
        while joined.ends_with(|c: char| c.is_uppercase()) {
            let last = joined.rsplit(' ').next().unwrap_or("");
            if last.chars().count() == 1 {
                joined = joined[..joined.len() - last.len()].trim_end().to_string();
            } else {
                break;
            }
        }
        if !joined.is_empty() {
            return joined;
        }
    }
    fallback_first_sentence(&lines)
}

/// Ementa sintética para consultas sem rótulo: a 1ª frase da narrativa (pula o cabeçalho `CONSULTA Nº:`
/// e linhas não-descritivas — protocolo, relator, marcadores `(...)`). Junta linhas até o 1º ponto
/// final, com teto de 240 caracteres. Vazio só para stubs cancelados sem narrativa.
fn fallback_first_sentence(lines: &[&str]) -> String {
    let mut buf = String::new();
    for l in lines.iter().skip(1) {
        let t = l.trim_start_matches(['"', '(', ')', '.', ' ']);
        let u = t.to_uppercase();
        if t.is_empty()
            || u.starts_with("SID")
            || u.starts_with("PROTOCOLO")
            || u.starts_with("RESPOSTA")
            || u.starts_with("RELATOR")
            || u.starts_with("CONSULTA N")
        {
            continue;
        }
        if !buf.is_empty() {
            buf.push(' ');
        }
        buf.push_str(t);
        if buf.contains('.') || buf.chars().count() > 240 {
            break;
        }
    }
    let buf = clean(&buf);
    let cut = buf.char_indices().find(|(_, c)| *c == '.').map(|(i, _)| i + 1).unwrap_or(buf.len());
    let mut s = buf[..cut].trim().to_string();
    if s.chars().count() > 240 {
        s = s.chars().take(240).collect::<String>().trim_end().to_string();
    }
    s
}

/// Linha "de súmula": entre suas letras, menos de 15% são minúsculas (ementa em CAIXA ALTA).
fn maiuscula(l: &str) -> bool {
    let letters: Vec<char> = l.chars().filter(|c| c.is_alphabetic()).collect();
    if letters.is_empty() {
        return false;
    }
    let lower = letters.iter().filter(|c| c.is_lowercase()).count();
    (lower as f32) / (letters.len() as f32) < 0.15
}

/// Colapsa espaços por linha e limita linhas em branco; preserva parágrafos.
fn normalize(block: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut blank = false;
    for raw in block.lines() {
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

/// `Parecer` + o número inteiro (para dedup por (ano, número)).
struct ParecerN {
    numero_n: u32,
    parecer: Parecer,
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
    fn parse_index_extracts_year_and_https() {
        let html = r#"<a href="http://www.sefanet.pr.gov.br/dados/SEFADOCUMENTOS/116200702007.pdf">x</a>
                      <a href="http://x/16200802008.pdf">y</a>"#;
        let a = parse_index(html);
        assert_eq!(a.len(), 2);
        assert_eq!(a[0].0, 2007);
        assert!(a[0].1.starts_with("https://"));
        assert_eq!(a[1].0, 2008);
    }

    #[test]
    fn split_and_sumula_on_sample() {
        let text = "\
SECRETARIA DE ESTADO DA FAZENDA DO PARANÁ - SEFA
SETOR CONSULTIVO
_______________
1
CONSULTA Nº: 01, de 25 de janeiro de 2007
SÚMULA: ICMS. CRÉDITO. PRESTAÇÃO DE SERVIÇO DE
TRANSPORTE. IMPOSSIBILIDADE DE CREDITAMENTO.
A
Consulente informa que tem atividade de comércio.
Expõe que adquire materiais.
CONSULTA Nº: 02, de 05 de fevereiro de 2007
SÚMULA: ICMS. DIFERIMENTO. ENCERRAMENTO.
O Consulente questiona o diferimento.";
        let v = split_consultas(text, 2007, "https://x/2007.pdf");
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].parecer.numero, "CONSULTA Nº 1/2007");
        assert_eq!(
            v[0].parecer.assunto,
            "ICMS. CRÉDITO. PRESTAÇÃO DE SERVIÇO DE TRANSPORTE. IMPOSSIBILIDADE DE CREDITAMENTO."
        );
        assert!(v[0].parecer.corpo.contains("Consulente informa que tem atividade"));
        assert!(!v[0].parecer.corpo.contains("SECRETARIA DE ESTADO")); // mobília fora
        assert_eq!(v[1].parecer.numero, "CONSULTA Nº 2/2007");
        assert_eq!(v[1].parecer.assunto, "ICMS. DIFERIMENTO. ENCERRAMENTO.");
    }

    #[test]
    fn ementa_sob_rotulo_assunto() {
        // Consultas antigas (2007–2008) rotulam a ementa como ASSUNTO:, não SÚMULA:.
        let text = "\
CONSULTA Nº: 11, de 12 de fevereiro de 2007
ASSUNTO: ICMS. PRAZO DE RECOLHIMENTO. CARVÃO
VEGETAL.
A consulente informa que importa carvão do Paraguai.";
        let v = split_consultas(text, 2007, "https://x/2007.pdf");
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].parecer.assunto, "ICMS. PRAZO DE RECOLHIMENTO. CARVÃO VEGETAL.");
    }

    #[test]
    fn ementa_para_antes_de_metadados() {
        // Relator/consulente/protocolo em CAIXA ALTA seguem a ementa mas NÃO fazem parte dela.
        let text = "\
CONSULTA Nº: 130, de 20 de dezembro de 2007
SÚMULA: ICMS. IMPORTAÇÃO. CÁLCULO DO CRÉDITO PRESUMIDO.
RELATORA: MAYSA CRISTINA DO PRADO
A consulente informa que importa mercadorias.";
        let v = split_consultas(text, 2007, "https://x/2007.pdf");
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].parecer.assunto, "ICMS. IMPORTAÇÃO. CÁLCULO DO CRÉDITO PRESUMIDO.");
    }

    #[test]
    fn fallback_para_consulta_sem_rotulo() {
        // Consulta antiga republicada, sem SÚMULA/ASSUNTO: usa a 1ª frase da narrativa.
        let text = "\
CONSULTA Nº: 140, de 17 de outubro de 2006
A consulente, estabelecimento que tem como atividade o comércio atacadista, importa mercadorias.
Expõe que efetua o desembaraço no Porto de Paranaguá.";
        let v = split_consultas(text, 2008, "https://x/2008.pdf");
        assert_eq!(v.len(), 1);
        assert_eq!(
            v[0].parecer.assunto,
            "A consulente, estabelecimento que tem como atividade o comércio atacadista, importa mercadorias."
        );
    }

    #[test]
    fn maiuscula_distingue_sumula_de_prosa() {
        assert!(maiuscula("ICMS. CRÉDITO. IMPOSSIBILIDADE."));
        assert!(!maiuscula("O Consulente informa que adquire materiais."));
    }
}
