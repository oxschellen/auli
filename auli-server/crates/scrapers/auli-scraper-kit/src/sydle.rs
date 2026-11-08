//! Os artigos do Sydle ONE (os portais de serviço do CE e do RO): o `contentMarkdown` é uma sequência de blocos
//! `<div class="section-wrapper">`, um por seção, cada um com o título no cabeçalho (`<div class="heading"
//! id="section-<slug>">`) e o corpo num `<sy-collapse … trigger-by="#section-<slug>">`. Dois moldes: o de 2026 (título em
//! `<h2>`, corpo em HTML com entidades) e o de 2024 (título numa linha `## `, corpo em markdown, onde a quebra de linha
//! separa os itens das listas). Estudos: `docs/faqs/ESTUDO-CE-Faqs.md` e `docs/faqs/ESTUDO-RO-PI-Faqs.md` (D-SYDFAQ-6).

use std::sync::LazyLock;

use anyhow::{Result, anyhow, bail};
use regex::Regex;

use crate::clean;
use crate::html::{Opcoes, html_para_texto};

/// O início de cada seção. O que vem antes do 1º bloco é o `<style>` e o botão "Expandir todas".
pub const BLOCO: &str = "<div class=\"section-wrapper\">";

static SLUG_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"id="section-([^"]*)""#).unwrap());
static H2_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)<h2\b[^>]*>(.*?)</h2>").unwrap());
static MD_H2_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^##[ \t]+(.+?)[ \t]*$").unwrap());

/// Uma seção, já em texto.
#[derive(Debug, Clone, PartialEq)]
pub struct Secao {
    /// O `<slug>` da âncora `#section-<slug>`.
    pub slug: String,
    /// O título, numa linha.
    pub titulo: String,
    /// O corpo: tudo o que vem depois da tag de abertura do `<sy-collapse>`, até o fim do bloco. Pode ser vazio.
    pub texto: String,
    /// O mesmo corpo, cru — para quem precisa segmentá-lo (as perguntas dentro de uma seção, no RO).
    pub html: String,
}

/// As seções de um `contentMarkdown`, na ordem. Sem bloco, nenhuma. `base` completa o caminho da raiz dos links.
/// Erro quando o molde mudou: âncoras e blocos em número diferente, bloco sem âncora, sem o `<sy-collapse>` dele ou
/// sem título.
pub fn secoes(md: &str, base: &str) -> Result<Vec<Secao>> {
    let blocos: Vec<&str> = md.split(BLOCO).skip(1).collect();
    let ancoras = md.matches("id=\"section-").count();
    if ancoras != blocos.len() {
        bail!(
            "{ancoras} âncora(s) id=\"section-\" para {} bloco(s) — o template mudou?",
            blocos.len()
        );
    }
    blocos.into_iter().map(|b| secao(b, base)).collect()
}

/// Uma seção. O título mora no cabeçalho, antes do `<sy-collapse>`: em `<h2>` (2026) ou na linha `## ` (2024).
fn secao(bloco: &str, base: &str) -> Result<Secao> {
    let slug = SLUG_RE
        .captures(bloco)
        .map(|c| c[1].to_string())
        .ok_or_else(|| anyhow!("bloco sem id=\"section-…\""))?;
    let gatilho = format!("trigger-by=\"#section-{slug}\"");
    let i = bloco
        .find(&gatilho)
        .ok_or_else(|| anyhow!("a seção {slug} não tem o <sy-collapse> dela"))?;
    let inicio_linha = bloco[..i].rfind('\n').map_or(0, |j| j + 1);
    if !bloco[inicio_linha..i].contains("<sy-collapse") {
        bail!("o gatilho da seção {slug} não está numa tag <sy-collapse>");
    }
    let cabecalho = &bloco[..inicio_linha];
    let (titulo, markdown) = match H2_RE.captures(cabecalho) {
        Some(c) => (c[1].to_string(), false),
        None => (
            MD_H2_RE
                .captures(cabecalho)
                .map(|c| c[1].to_string())
                .ok_or_else(|| anyhow!("a seção {slug} não tem título (nem <h2> nem ##)"))?,
            true,
        ),
    };
    // A tag de abertura termina no fim da linha, não no primeiro `>`: a do molde de 2024 traz
    // `ref="(r) => handleState(r)"`, um `>` dentro do atributo.
    let corpo = bloco[i..]
        .split_once('\n')
        .map(|(_, resto)| resto)
        .ok_or_else(|| anyhow!("a seção {slug} acaba na tag <sy-collapse>"))?;
    Ok(Secao {
        titulo: clean(&texto(&titulo, base, false).replace('\n', " ")),
        texto: texto(corpo, base, markdown),
        html: corpo.to_string(),
        slug,
    })
}

/// HTML → texto pelo conversor da frota: no molde markdown, a quebra do fonte é quebra.
fn texto(s: &str, base: &str, markdown: bool) -> String {
    html_para_texto(
        s,
        Opcoes {
            base: Some(base),
            quebras_do_fonte: markdown,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = "https://portal.exemplo.gov.br";

    fn bloco_2026(slug: &str, titulo: &str, corpo: &str) -> String {
        format!(
            "{BLOCO}\n    <div class=\"heading\" id=\"section-{slug}\"><h2>{titulo}</h2></div>\n    \
             <sy-collapse class=\"hydrated collapse service-section\" trigger-by=\"#section-{slug}\">\n    \
             <div class=\"content\">{corpo}</div>\n</div>\n"
        )
    }

    fn bloco_2024(slug: &str, titulo: &str, corpo: &str) -> String {
        format!(
            "{BLOCO}\n<div class=\"heading\" id=\"section-{slug}\">\n\n## {titulo}\n</div>\n\
             <sy-collapse ref=\"(r) => handleState(r)\" class=\"collapsed\" trigger-by=\"#section-{slug}\" opened> \n\
             {corpo}\n</div>\n"
        )
    }

    #[test]
    fn os_dois_moldes() {
        let md = format!(
            "<style>.x{{}}</style>{}{}",
            bloco_2026("a", "O que &eacute;?", "<p>Um <a href=\"/x\">link</a>.</p>"),
            bloco_2024("b", "Quem pode?", "- item 1\n- item 2")
        );
        let s = secoes(&md, BASE).unwrap();
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].slug, "a");
        assert_eq!(s[0].titulo, "O que é?");
        assert_eq!(s[0].texto, "Um link \"https://portal.exemplo.gov.br/x\".");
        // No molde markdown a quebra do fonte fica.
        assert_eq!(s[1].titulo, "Quem pode?");
        assert_eq!(s[1].texto, "- item 1\n- item 2");
        assert_eq!(s[1].html, "- item 1\n- item 2\n</div>\n");
    }

    #[test]
    fn links_e_listas_viram_texto() {
        let html = r#"<p>Veja <a href="https://x.ce.gov.br/a">o manual</a> e <a href="https://y">https://y</a>.</p><ul><li>um</li><li>dois &amp; tr&ecirc;s</li></ul><a href="javascript:void(0)">nada</a>"#;
        assert_eq!(
            texto(html, BASE, false),
            "Veja o manual \"https://x.ce.gov.br/a\" e \"https://y\".\num\ndois & três\nnada"
        );
        // No HTML a quebra do fonte é espaço; no markdown, quebra.
        assert_eq!(texto("a\nb", BASE, false), "a b");
        assert_eq!(texto("a\n- b", BASE, true), "a\n- b");
    }

    #[test]
    fn no_molde_2026_a_quebra_do_fonte_e_espaco_e_o_titulo_fica_numa_linha() {
        let s = secoes(&bloco_2026("a", "O que\n é?", "<p>uma\nfrase</p>"), BASE).unwrap();
        assert_eq!(s[0].titulo, "O que é?");
        assert_eq!(s[0].texto, "uma frase");
    }

    #[test]
    fn sem_bloco_nenhuma_secao() {
        assert!(secoes("<p>-</p>", BASE).unwrap().is_empty());
    }

    #[test]
    fn corpo_vazio_e_do_chamador() {
        let s = secoes(&bloco_2026("a", "T", "&nbsp;"), BASE).unwrap();
        assert_eq!(s[0].texto, "");
    }

    #[test]
    fn molde_quebrado_e_erro() {
        let bom = bloco_2026("s", "P?", "R.");
        let a_mais = format!("{bom}<p id=\"section-x\"></p>");
        assert!(secoes(&a_mais, BASE).is_err());
        let sem_collapse = bom.replace("trigger-by", "data-x");
        assert!(secoes(&sem_collapse, BASE).is_err());
        let sem_titulo = bom.replace("<h2>P?</h2>", "");
        assert!(secoes(&sem_titulo, BASE).is_err());
        let gatilho_fora = bom.replace("<sy-collapse", "<div");
        assert!(secoes(&gatilho_fora, BASE).is_err());
    }
}
