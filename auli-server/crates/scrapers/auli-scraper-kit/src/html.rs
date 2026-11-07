//! HTML → texto das respostas de FAQ, na convenção da frota: bloco vira linha, link vira `texto "url"`, entidades
//! decodificadas uma vez. Uma função para os 12 crates de FAQ, que tinham 10 variantes próprias (estudo e
//! decisões D-KITHTML-1..8: `docs/ESTUDO-kit-html-para-texto.md`).
//!
//! As regras, na ordem:
//!
//! 1. `<style>`, `<script>`, `<svg>` e comentários saem inteiros (o Word cola CSS e comentários condicionais).
//! 2. Com [`Opcoes::quebras_do_fonte`], a quebra de linha do fonte vira `<br>` — só fora das tags.
//! 3. O espaço do fonte é compactado.
//! 4. `<img src>` vira `"url"`; `<a href>texto</a>` vira `texto "url"`, ou só `"url"` quando o texto é a própria URL,
//!    ou só o texto quando o `href` não é endereço ([`endereco`]).
//! 5. Tag de bloco vira quebra de linha; célula de tabela vira espaço; as outras tags HTML saem.
//! 6. O `<` que sobrou é texto (`<Consulta Pendencias>`, `(0) < Soma`) e fica; as entidades são decodificadas pelo
//!    html5ever, uma vez; U+FEFF sai; cada linha passa pelo [`clean`] e as vazias saem.

use std::sync::LazyLock;

use regex::Regex;
use scraper::Html;

use crate::clean;

/// O que muda de uma entidade para outra.
#[derive(Clone, Copy, Debug, Default)]
pub struct Opcoes<'a> {
    /// Uma URL do portal: o host dela completa o caminho da raiz (`/x`) de link e imagem. `None`: o caminho da raiz
    /// não é endereço, e o link fica só com o texto.
    pub base: Option<&'a str>,
    /// A quebra de linha do fonte é estrutura (alíneas "a)", "b)" em texto corrido) e vira quebra (D-KITHTML-4):
    /// SC, MT, MS, PR e o modo `markdown` do CE. Sem ela, o espaço do fonte é compactado.
    pub quebras_do_fonte: bool,
}

static OCULTO_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?is)<style\b.*?</style>|<script\b.*?</script>|<svg\b.*?</svg>|<!--.*?-->")
        .unwrap()
});
/// Uma tag inteira, um trecho de texto, ou um `<` solto — para trocar a quebra do fonte só no texto.
static TAG_OU_TEXTO_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]*>|[^<]+|<").unwrap());
static ESPACO_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());
static IMG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?is)<img\b[^>]*?\ssrc\s*=\s*["']([^"']*)["'][^>]*>"#).unwrap());
static LINK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)<a\b[^>]*?\shref\s*=\s*["']([^"']*)["'][^>]*>(.*?)</a>"#).unwrap()
});
static BLOCO_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)</?(?:p|div|br|hr|li|ul|ol|tr|table|tbody|thead|h[1-6]|blockquote|center)\b[^>]*>",
    )
    .unwrap()
});
static CELULA_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)</?t[dh]\b[^>]*>").unwrap());
/// Só tag de nome HTML conhecido, ou com prefixo (`<o:p>` do Word), sai (D-KITHTML-5). `<Consulta Pendencias>` e
/// `< Alteração` são texto do portal.
static TAG_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"(?i)</?(?:[a-z][a-z0-9]*:[a-z][a-z0-9]*|",
        "a|abbr|address|article|aside|audio|b|base|bdi|bdo|big|blockquote|body|br|button|caption|",
        "center|cite|code|col|colgroup|dd|del|details|dfn|div|dl|dt|em|embed|fieldset|figcaption|",
        "figure|font|footer|form|h[1-6]|head|header|hr|html|i|iframe|img|input|ins|kbd|label|",
        "legend|li|link|main|mark|meta|nav|nobr|noscript|object|ol|optgroup|option|p|param|",
        "picture|pre|q|s|samp|section|select|small|source|span|strike|strong|sub|summary|sup|",
        "table|tbody|td|textarea|tfoot|th|thead|time|title|tr|tt|u|ul|var|video|wbr",
        r")(?:\s[^>]*|/)?>"
    ))
    .unwrap()
});

/// HTML → texto (as regras estão no topo do módulo).
pub fn html_para_texto(s: &str, o: Opcoes) -> String {
    let visivel = OCULTO_RE.replace_all(s, "");
    // Dentro de uma tag (`<span\nstyle=…>`), a troca quebraria a tag.
    let visivel = if o.quebras_do_fonte {
        TAG_OU_TEXTO_RE
            .replace_all(&visivel, |c: &regex::Captures| {
                let trecho = &c[0];
                if trecho.starts_with('<') {
                    trecho.to_string()
                } else {
                    trecho.replace('\n', "<br>")
                }
            })
            .into_owned()
    } else {
        visivel.into_owned()
    };
    let corrido = ESPACO_RE.replace_all(&visivel, " ");
    let com_imagens = IMG_RE.replace_all(&corrido, |c: &regex::Captures| {
        match endereco(c[1].trim(), o.base) {
            Some(url) => format!("\"{url}\""),
            None => String::new(),
        }
    });
    // O texto é comparado com a URL já decodificado; o que sai é o original, que o passo final decodifica uma vez.
    let com_links = LINK_RE.replace_all(&com_imagens, |c: &regex::Captures| {
        let texto = clean(&TAG_RE.replace_all(&c[2], ""));
        match endereco(c[1].trim(), o.base) {
            Some(url) if texto.is_empty() || clean(&decodificar(&texto)) == decodificar(&url) => {
                format!("\"{url}\"")
            }
            Some(url) => format!("{texto} \"{url}\""),
            None => texto,
        }
    });
    let com_quebras = BLOCO_RE.replace_all(&com_links, "\n");
    let com_celulas = CELULA_RE.replace_all(&com_quebras, " ");
    let sem_tags = TAG_RE.replace_all(&com_celulas, "");
    // Sem o escape, o html5ever leria `<Consulta Pendencias>` como um elemento e o descartaria.
    decodificar(&sem_tags.replace('<', "&lt;"))
        .replace('\u{feff}', "")
        .lines()
        .map(clean)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// O endereço de um `href` (ou `src`) que vale pôr no texto: `http(s)`, `ftp` e `mailto` como vieram; o caminho da
/// raiz com o host da `base`; o resto (`#`, `javascript:`, `file:`, `www.x` sem esquema, texto solto) não. O esquema
/// é lido no `href` decodificado — o SharePoint escreve `https&#58;//` —, mas o que volta é o original, para a
/// decodificação acontecer uma vez só (`&amp;region=` decodificado duas vezes viraria `®ion=`).
pub fn endereco(href: &str, base: Option<&str>) -> Option<String> {
    let minusculo = decodificar(href).to_ascii_lowercase();
    if ["http://", "https://", "ftp://", "mailto:"]
        .iter()
        .any(|p| minusculo.starts_with(p))
    {
        Some(href.to_string())
    } else if href.starts_with('/') && !href.starts_with("//") {
        let host = base?.splitn(4, '/').take(3).collect::<Vec<_>>().join("/");
        Some(format!("{host}{href}"))
    } else {
        None
    }
}

fn decodificar(s: &str) -> String {
    Html::parse_fragment(s).root_element().text().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = "https://portal.exemplo.gov.br/faq/pagina";

    fn t(s: &str) -> String {
        html_para_texto(
            s,
            Opcoes {
                base: Some(BASE),
                ..Default::default()
            },
        )
    }

    fn com_quebras(s: &str) -> String {
        html_para_texto(
            s,
            Opcoes {
                base: Some(BASE),
                quebras_do_fonte: true,
            },
        )
    }

    #[test]
    fn style_script_svg_e_comentarios_saem() {
        assert_eq!(
            t(
                "<style>p{x:1}</style>a<script>x()</script><svg><t>s</t></svg><!--[if gte mso 9]>w<![endif]-->b"
            ),
            "ab"
        );
    }

    #[test]
    fn espaco_do_fonte_e_compactado_sem_a_opcao() {
        assert_eq!(t("a\n  b\tc"), "a b c");
    }

    #[test]
    fn quebra_do_fonte_vira_quebra_com_a_opcao() {
        assert_eq!(
            com_quebras("Exportadoras:\na) matéria-prima;\nb) bens"),
            "Exportadoras:\na) matéria-prima;\nb) bens"
        );
    }

    #[test]
    fn quebra_dentro_da_tag_nao_quebra_a_tag() {
        // O caso do SC: com `<p<br>style=…`, a regex de bloco comia o `<p<br>` e `normal">` vazava para o texto.
        assert_eq!(
            com_quebras("<p\nstyle=\"font-weight:\nnormal\">Não, somente</p>"),
            "Não, somente"
        );
    }

    #[test]
    fn blocos_quebram_e_celulas_separam() {
        assert_eq!(
            t("<p>a</p><div>b</div>c<br>d<hr>e<ul><li>f</li></ul><center>g</center><h2>h</h2>"),
            "a\nb\nc\nd\ne\nf\ng\nh"
        );
        assert_eq!(t("a<center>b</center>c"), "a\nb\nc");
        assert_eq!(
            t("<table><tr><td>Até 1</td><td>2%</td></tr></table>"),
            "Até 1 2%"
        );
    }

    #[test]
    fn link_vira_texto_e_url() {
        assert_eq!(
            t(r#"Veja <a target="_blank" href="https://x.gov.br/a">a <b>lei</b></a>."#),
            "Veja a lei \"https://x.gov.br/a\"."
        );
    }

    #[test]
    fn url_sai_uma_vez_quando_o_texto_e_a_url() {
        assert_eq!(
            t(r#"<a href="http://www.fazenda.pr.gov.br">http://www.fazenda.pr.gov.br</a>"#),
            "\"http://www.fazenda.pr.gov.br\""
        );
        // A comparação é com o texto decodificado.
        assert_eq!(
            t(r#"<a href="https://x.gov.br/?a=1&amp;b=2">https://x.gov.br/?a=1&amp;b=2</a>"#),
            "\"https://x.gov.br/?a=1&b=2\""
        );
        assert_eq!(
            t(r#"<a href="https://x.gov.br/"></a>"#),
            "\"https://x.gov.br/\""
        );
        // O `href` com o `:` codificado e o texto sem: é a mesma URL.
        assert_eq!(
            t(r#"<a href="https&#58;//www.x.gov.br/a">https://www.x.gov.br/a</a>"#),
            "\"https://www.x.gov.br/a\""
        );
    }

    #[test]
    fn esquema_com_entidade_e_endereco_e_sai_decodificado_uma_vez() {
        // O SharePoint de SP e de GO escreve o `:` como `&#58;`.
        assert_eq!(
            t(
                r#"pelo link <a href="https&#58;//www3.fazenda.sp.gov.br/CAWEB/Account/Login.aspx">https&#58;//www3.fazenda.sp.gov.br/CAWEB/A/Login.aspx</a>"#
            ),
            "pelo link https://www3.fazenda.sp.gov.br/CAWEB/A/Login.aspx \"https://www3.fazenda.sp.gov.br/CAWEB/Account/Login.aspx\""
        );
        // Uma vez só: `&amp;region=` vira `&region=`, não `®ion=`.
        assert_eq!(
            t(r#"<a href="https://x.gov.br/?a=1&amp;region=2">mapa</a>"#),
            "mapa \"https://x.gov.br/?a=1&region=2\""
        );
    }

    #[test]
    fn href_que_nao_e_endereco_fica_so_com_o_texto() {
        for href in [
            "#topo",
            "javascript:void(0)",
            "",
            "www.nfe.fazenda.gov.br",
            "Lei nº 7.799/2002,",
            "file:///C:/x.doc",
        ] {
            assert_eq!(
                t(&format!(r#"<a href="{href}">texto</a>"#)),
                "texto",
                "href {href:?}"
            );
        }
    }

    #[test]
    fn ftp_e_mailto_sao_endereco() {
        assert_eq!(
            t(r#"<a href="ftp://ftp.sefaz.go.gov.br/D_09108.htm">Decreto nº 9.108</a>"#),
            "Decreto nº 9.108 \"ftp://ftp.sefaz.go.gov.br/D_09108.htm\""
        );
        assert_eq!(
            t(r#"<a href="mailto:a@x.gov.br">a@x.gov.br</a>"#),
            "a@x.gov.br \"mailto:a@x.gov.br\""
        );
    }

    #[test]
    fn caminho_da_raiz_ganha_o_host_da_base_ou_fica_so_o_texto() {
        assert_eq!(
            t(r#"<a href="/Legislacao/lei.htm">lei</a> <img src="/img/a.png">"#),
            "lei \"https://portal.exemplo.gov.br/Legislacao/lei.htm\" \"https://portal.exemplo.gov.br/img/a.png\""
        );
        assert_eq!(
            html_para_texto(
                r#"<a href="/Legislacao/lei.htm">lei</a><img src="/a.png">"#,
                Opcoes::default()
            ),
            "lei"
        );
    }

    #[test]
    fn lt_que_e_texto_fica() {
        // SC: a regex `<[^>]+>` apagava até o próximo `>`.
        assert_eq!(
            t(
                "Qtde de entrada (0) < Soma Qtde de Saídas (100) + Estoque final (0). Neste caso, <b>é</b> obrigatória"
            ),
            "Qtde de entrada (0) < Soma Qtde de Saídas (100) + Estoque final (0). Neste caso, é obrigatória"
        );
        // PR e CE: o html5ever lia `<Palavra>` como elemento e o descartava.
        assert_eq!(
            t(
                "menu Cadastro de Contribuintes < Alteração Cadastral < Dados Cadastrais. <Consulta Pendencias>, basta"
            ),
            "menu Cadastro de Contribuintes < Alteração Cadastral < Dados Cadastrais. <Consulta Pendencias>, basta"
        );
        assert_eq!(
            t("<p>CRT - <CRT> Código de Regime Tributário</p>"),
            "CRT - <CRT> Código de Regime Tributário"
        );
    }

    #[test]
    fn tag_com_prefixo_do_word_sai() {
        assert_eq!(t("<p class=MsoNormal>Sim.<o:p></o:p></p>"), "Sim.");
    }

    #[test]
    fn entidades_decodificadas_uma_vez() {
        assert_eq!(
            t("a&nbsp;b &#58; c &quot;d&quot; &amp;lt;e&amp;gt;"),
            "a b : c \"d\" &lt;e&gt;"
        );
    }

    #[test]
    fn feff_zero_width_e_linhas_vazias_saem() {
        assert_eq!(
            t("<p>\u{feff}a\u{200b}b</p><p><br></p><p>&nbsp;</p><p>c</p>"),
            "ab\nc"
        );
    }
}
