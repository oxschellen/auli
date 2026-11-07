// SEFAZ-RN FAQs — a FAQ do programa Nota Potiguar (`np.sefaz.rn.gov.br/portal`). Estudo, medições e decisões:
// docs/ESTUDO-RN-Faqs.md (D-RNFAQ-*).
//
// O site da SEFAZ-RN (WordPress) não tem FAQ tributária; o Nota Potiguar, o programa de cidadania fiscal, tem duas
// telas de FAQ num portal AngularJS. O HTML delas vem escrito no próprio bundle, nos templates do `$templateCache`
// (`js/templates.js`): `$templateCache.put("<tela>", "…" + "…" + …)` — 1 GET, molde AP. A tela "O Aplicativo" é a
// versão completa (D-RNFAQ-1); a "Comunicação" tem as mesmas perguntas em redação mais velha e só serve de
// conferência.

use std::collections::{BTreeSet, HashSet};
use std::path::Path;
use std::sync::LazyLock;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use scraper::node::Node;
use scraper::{ElementRef, Html, Selector};
use ureq::Agent;

use auli_contract::FaqRaw;
use auli_scraper_kit::clean;
use auli_scraper_kit::faqtree::{FaqItem, FaqNode, PageType};
use auli_scraper_kit::http::GetOpts;

const HOST: &str = "https://np.sefaz.rn.gov.br";
const PORTAL: &str = "https://np.sefaz.rn.gov.br/portal/";
const TEMPLATES_URL: &str = "https://np.sefaz.rn.gov.br/portal/js/templates.js";
/// A tela da v1 e o link oficial dela: o app usa `hashPrefix("")`, então a rota vem depois de `#`.
const TELA: &str = "components/oaplicativo/views/faq.html";
const LINK: &str = "https://np.sefaz.rn.gov.br/portal/#/o-aplicativo/faq";
/// A outra tela de FAQ do portal. Pergunta dela que falte na da v1 é aviso: as duas telas divergiram.
const TELA_CONFERENCIA: &str = "components/comunicacao/views/faq.html";
const TRILHA: &str = "Nota Potiguar";
/// Piso contra uma tela esvaziada (23 perguntas em 06/11/2026).
const PISO: usize = 15;

/// Coleta a FAQ e grava a árvore do frontend (`<id>-faqs-tree.json`) e o snapshot. Tudo é conferido antes de
/// qualquer gravação: rodada que falha deixa os arquivos anteriores intactos.
pub fn run(data_dir: &str, use_cache: bool) -> Result<()> {
    let agent = auli_scraper_kit::build_agent(crate::UA, Some(Duration::from_secs(60)));
    let js = buscar(&agent, data_dir, TEMPLATES_URL, use_cache)?;
    let perguntas = template(&js, TELA)
        .and_then(|html| extrair(&html))
        .with_context(|| format!("RN FAQs: tela {TELA} do {TEMPLATES_URL}"))?;
    let faqs = acervo(&perguntas)?;
    match so_na_conferencia(&js, &perguntas) {
        Ok(fora) => {
            for p in fora {
                eprintln!(
                    "⚠️  RN FAQs: pergunta só na tela {TELA_CONFERENCIA}, não na da v1: {p:?}"
                );
            }
        }
        Err(e) => eprintln!("⚠️  RN FAQs: a tela de conferência não deu para ler: {e:#}"),
    }

    // Atômico pelo mesmo motivo das outras entidades: a árvore não tem outra fonte (o snapshot achatado não a
    // reconstrói) e é o único artefato de coleta que o frontend lê direto.
    let tree_path = format!("{data_dir}/{}-faqs-tree.json", crate::ENTITY);
    auli_contract::arquivo::escrever_atomico(
        Path::new(&tree_path),
        &serde_json::to_string_pretty(&montar_arvore(&perguntas))?,
    )?;
    println!("Wrote {tree_path} (árvore de FAQ p/ o frontend)");

    auli_contract::snapshot::write_faqs(
        crate::ENTITY,
        data_dir,
        &auli_contract::ScraperInfo::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
        faqs,
    )?;
    println!("🎉 Coleta de FAQs gravada no snapshot.");
    Ok(())
}

/// Rede sempre, gravando o cache; com `--usecache`, só o cache, e miss é erro (como as FAQs da frota).
fn buscar(agent: &Agent, data_dir: &str, url: &str, use_cache: bool) -> Result<String> {
    if use_cache {
        return auli_scraper_kit::cache::read(data_dir, "faqs", url)
            .ok_or_else(|| anyhow!("cache miss para {url} (modo --usecache, sem rede)"));
    }
    let js = auli_scraper_kit::http::get_string(
        agent,
        url,
        &GetOpts {
            log_prefix: "RN FAQs",
            ..Default::default()
        },
    )?;
    auli_scraper_kit::cache::write(data_dir, "faqs", url, &js);
    Ok(js)
}

struct Pergunta {
    titulo: String,
    resposta: String,
}

/// O HTML de uma tela: o 2º argumento de `$templateCache.put("<tela>", "…" + "…" + …)`, com os literais de string
/// juntados e desescapados (`\"`, `\n`, `\uXXXX`…).
fn template(js: &str, tela: &str) -> Result<String> {
    let marca = format!("$templateCache.put(\"{tela}\",");
    let ini = js
        .find(&marca)
        .ok_or_else(|| anyhow!("o templates.js não tem a tela {tela}"))?
        + marca.len();
    let mut c = js[ini..].chars().peekable();
    let mut html = String::new();
    loop {
        while c.next_if(|x| x.is_whitespace()).is_some() {}
        match c.next() {
            Some('"') => loop {
                match c.next() {
                    Some('"') => break,
                    Some('\\') => match c.next() {
                        Some('n') => html.push('\n'),
                        Some('t') => html.push('\t'),
                        Some('r') => html.push('\r'),
                        Some('u') => {
                            let hex: String = (0..4).filter_map(|_| c.next()).collect();
                            let ch = u32::from_str_radix(&hex, 16)
                                .ok()
                                .and_then(char::from_u32)
                                .ok_or_else(|| {
                                    anyhow!("escape \\u{hex} inválido na tela {tela}")
                                })?;
                            html.push(ch);
                        }
                        // `\"`, `\\`, `\'`, `\/`: o caractere em si.
                        Some(x) => html.push(x),
                        None => bail!("a tela {tela} termina no meio de um escape"),
                    },
                    Some(x) => html.push(x),
                    None => bail!("a tela {tela} termina no meio de um literal de string"),
                }
            },
            outro => bail!("a tela {tela}: esperava um literal de string, veio {outro:?}"),
        }
        while c.next_if(|x| x.is_whitespace()).is_some() {}
        match c.next() {
            Some('+') => continue,
            Some(')') => return Ok(html),
            outro => bail!("a tela {tela}: esperava `+` ou `)` depois do literal, veio {outro:?}"),
        }
    }
}

static H3: LazyLock<Selector> = LazyLock::new(|| Selector::parse("h3").unwrap());

/// O HTML da tela → as perguntas, na ordem da tela: cada `<h3>` é uma pergunta, e a resposta é o que vem depois
/// dele, até o próximo `<h3>`.
fn extrair(html: &str) -> Result<Vec<Pergunta>> {
    let doc = Html::parse_fragment(html);
    let mut perguntas = Vec::new();
    for h3 in doc.select(&H3) {
        let titulo = clean(&h3.text().collect::<String>());
        if titulo.is_empty() {
            bail!("um <h3> da tela está vazio");
        }
        let mut e = Escritor::default();
        for irmao in h3.next_siblings() {
            match irmao.value() {
                Node::Text(t) => e.atual.push_str(t),
                Node::Element(el) if el.name() == "h3" => break,
                Node::Element(_) => {
                    if let Some(el) = ElementRef::wrap(irmao) {
                        andar(el, &mut e);
                    }
                }
                _ => {}
            }
        }
        e.quebra();
        if e.linhas.is_empty() {
            bail!("a pergunta {titulo:?} ficou sem resposta");
        }
        perguntas.push(Pergunta {
            titulo,
            resposta: e.linhas.join("\n"),
        });
    }
    if perguntas.is_empty() {
        bail!("a tela não tem nenhuma pergunta (<h3>) — o layout mudou?");
    }
    Ok(perguntas)
}

#[derive(Default)]
struct Escritor {
    linhas: Vec<String>,
    atual: String,
}

impl Escritor {
    fn quebra(&mut self) {
        let t = clean(&self.atual);
        if !t.is_empty() {
            self.linhas.push(t);
        }
        self.atual.clear();
    }
}

/// HTML → linhas: bloco e `<br>` viram quebra, link vira `texto "url"` (relativo absolutizado). O html5ever já
/// decodificou as entidades; nbsp e zero-width saem no `clean`.
fn andar(el: ElementRef, e: &mut Escritor) {
    let tag = el.value().name();
    match tag {
        "script" | "style" => return,
        "br" => return e.quebra(),
        "a" => {
            if let Some(href) =
                el.value().attr("href").map(str::trim).filter(|h| {
                    !h.is_empty() && !h.starts_with('#') && !h.starts_with("javascript:")
                })
            {
                let texto = clean(&el.text().collect::<String>());
                let url = absoluto(href);
                if texto.is_empty() {
                    e.atual.push_str(&format!(" \"{url}\" "));
                } else if texto == href || texto == url {
                    e.atual.push_str(&format!(" {texto} "));
                } else {
                    e.atual.push_str(&format!(" {texto} \"{url}\" "));
                }
                return;
            }
        }
        _ => {}
    }
    let bloco = matches!(
        tag,
        "p" | "div"
            | "li"
            | "ul"
            | "ol"
            | "section"
            | "table"
            | "tr"
            | "h1"
            | "h2"
            | "h4"
            | "h5"
            | "h6"
    );
    if bloco {
        e.quebra();
    }
    for filho in el.children() {
        match filho.value() {
            Node::Text(t) => e.atual.push_str(t),
            Node::Element(_) => {
                if let Some(f) = ElementRef::wrap(filho) {
                    andar(f, e);
                }
            }
            _ => {}
        }
    }
    if bloco {
        e.quebra();
    }
}

fn absoluto(href: &str) -> String {
    if href.starts_with("http://") || href.starts_with("https://") {
        href.to_string()
    } else if href.starts_with('/') {
        format!("{HOST}{href}")
    } else {
        format!("{PORTAL}{href}")
    }
}

/// As perguntas → o acervo. Todas têm o link da tela, então o nome do `.md` (pergunta + hash de link e pergunta)
/// depende só da pergunta: repetida é `Err`.
fn acervo(perguntas: &[Pergunta]) -> Result<Vec<FaqRaw>> {
    if perguntas.len() < PISO {
        bail!(
            "a tela {TELA} tem {} perguntas, menos que o piso de {PISO} — esvaziou?",
            perguntas.len()
        );
    }
    let mut vistas = BTreeSet::new();
    if let Some(p) = perguntas.iter().find(|p| !vistas.insert(p.titulo.as_str())) {
        bail!(
            "a pergunta {:?} aparece duas vezes na tela — os .md teriam o mesmo nome",
            p.titulo
        );
    }
    println!("RN FAQs: {} perguntas ({TRILHA})", perguntas.len());
    Ok(perguntas
        .iter()
        .map(|p| FaqRaw {
            pergunta: p.titulo.clone(),
            resposta: p.resposta.clone(),
            origin: TRILHA.to_string(),
            url: LINK.to_string(),
        })
        .collect())
}

/// As perguntas da tela de conferência que não estão na da v1.
fn so_na_conferencia(js: &str, perguntas: &[Pergunta]) -> Result<Vec<String>> {
    let nossas: HashSet<&str> = perguntas.iter().map(|p| p.titulo.as_str()).collect();
    Ok(extrair(&template(js, TELA_CONFERENCIA)?)?
        .into_iter()
        .map(|p| p.titulo)
        .filter(|t| !nossas.contains(t.as_str()))
        .collect())
}

fn montar_arvore(perguntas: &[Pergunta]) -> FaqNode {
    FaqNode {
        title: "Perguntas frequentes".to_string(),
        url: PORTAL.to_string(),
        page_type: PageType::Menu,
        origin: String::new(),
        children: vec![FaqNode {
            title: TRILHA.to_string(),
            url: LINK.to_string(),
            page_type: PageType::Faq,
            origin: TRILHA.to_string(),
            children: Vec::new(),
            faq_items: perguntas
                .iter()
                .map(|p| FaqItem {
                    pergunta: p.titulo.clone(),
                    resposta: p.resposta.clone(),
                    url: None,
                })
                .collect(),
        }],
        faq_items: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Recorte real do `js/templates.js` (06/11/2026): o cabeçalho e 4 módulos — as duas telas de FAQ e o
    /// módulo antes de cada uma.
    const RECORTE: &str = include_str!("../tests/fixtures/rn-faqs-templates-recorte.js");

    fn tela_v1() -> Vec<Pergunta> {
        extrair(&template(RECORTE, TELA).unwrap()).unwrap()
    }

    #[test]
    fn template_junta_e_desescapa_os_literais() {
        let js = r#"$templateCache.put("t.html",
    "<p class=\"x\">a\n" +
    "b \u00e9 \'c\' \/ \\</p>");
}]);"#;
        assert_eq!(
            template(js, "t.html").unwrap(),
            "<p class=\"x\">a\nb é 'c' / \\</p>"
        );
    }

    #[test]
    fn template_le_a_tela_certa_no_recorte_real() {
        let html = template(RECORTE, TELA).unwrap();
        assert!(html.starts_with("<section"), "{}", &html[..80]);
        assert!(html.contains("FAQ (Perguntas Frequentes)"));
        assert!(!html.contains("\\\""), "escape que sobrou");
        let outra = template(RECORTE, TELA_CONFERENCIA).unwrap();
        assert!(outra.contains("<h1 class=\"titulo-pagina-h1\">Perguntas Frequentes</h1>"));
    }

    #[test]
    fn tela_sem_template_ou_literal_quebrado_e_erro() {
        assert!(template(RECORTE, "components/nao/existe.html").is_err());
        assert!(template(r#"$templateCache.put("t.html", "abc"#, "t.html").is_err());
        assert!(template(r#"$templateCache.put("t.html", x + "a");"#, "t.html").is_err());
    }

    #[test]
    fn as_23_perguntas_da_tela_o_aplicativo() {
        let p = tela_v1();
        assert_eq!(p.len(), 23);
        assert_eq!(p[0].titulo, "O que é o aplicativo \"Nota Potiguar\"?");
        assert_eq!(p[4].titulo, "Como funciona o desconto no IPVA?");
        assert_eq!(p[22].titulo, "Como faço para baixar o aplicativo?");
        assert!(
            p.iter()
                .all(|q| !q.resposta.is_empty() && !q.resposta.contains('<'))
        );
    }

    #[test]
    fn br_vira_quebra_de_linha_na_resposta() {
        let baixar = tela_v1().pop().unwrap();
        let linhas: Vec<&str> = baixar.resposta.lines().collect();
        assert_eq!(linhas.len(), 6, "{}", baixar.resposta);
        assert!(linhas[0].starts_with("I. Na loja Google Play"));
        assert!(linhas[5].starts_with("VI. Ao confirmar o cadastro"));
    }

    #[test]
    fn resposta_para_no_proximo_h3_e_link_vira_texto_url() {
        let html = r#"<h1>FAQ</h1>
            <h3>Primeira?</h3><p>Um <a href="downloads/manuais/faq-np.pdf">manual</a>.</p><ul><li>a</li><li>b</li></ul>
            <h3>Segunda?</h3><p>Ver <a href="/portal/">o portal</a> e <a ui-sref="x">ali</a>.</p>"#;
        let p = extrair(html).unwrap();
        assert_eq!(p.len(), 2);
        assert_eq!(
            p[0].resposta,
            "Um manual \"https://np.sefaz.rn.gov.br/portal/downloads/manuais/faq-np.pdf\" .\na\nb"
        );
        assert_eq!(
            p[1].resposta,
            "Ver o portal \"https://np.sefaz.rn.gov.br/portal/\" e ali."
        );
    }

    #[test]
    fn pergunta_sem_resposta_ou_tela_sem_pergunta_e_erro() {
        assert!(extrair("<h3>Sozinha?</h3><h3>Outra?</h3><p>r</p>").is_err());
        assert!(extrair("<h1>FAQ</h1><p>sem perguntas</p>").is_err());
        assert!(extrair("<h3> </h3><p>r</p>").is_err());
    }

    #[test]
    fn acervo_com_link_da_tela_e_trilha() {
        let faqs = acervo(&tela_v1()).unwrap();
        assert_eq!(faqs.len(), 23);
        assert!(
            faqs.iter()
                .all(|f| f.url == LINK && f.origin == "Nota Potiguar")
        );
    }

    #[test]
    fn acervo_abaixo_do_piso_ou_com_pergunta_repetida_e_erro() {
        let mut p = tela_v1();
        p.truncate(PISO - 1);
        assert!(acervo(&p).is_err());
        let mut p = tela_v1();
        let copia = Pergunta {
            titulo: p[0].titulo.clone(),
            resposta: "outra".to_string(),
        };
        p.push(copia);
        assert!(acervo(&p).is_err());
    }

    #[test]
    fn conferencia_aponta_so_a_pergunta_que_falta_na_v1() {
        let p = tela_v1();
        assert_eq!(
            so_na_conferencia(RECORTE, &p).unwrap(),
            Vec::<String>::new()
        );
        let sem_a_primeira: Vec<Pergunta> = tela_v1().into_iter().skip(1).collect();
        assert_eq!(
            so_na_conferencia(RECORTE, &sem_a_primeira).unwrap(),
            vec!["O que é o aplicativo \"Nota Potiguar\"?".to_string()]
        );
    }

    #[test]
    fn arvore_com_um_no_nota_potiguar() {
        let a = montar_arvore(&tela_v1());
        assert_eq!(a.page_type, PageType::Menu);
        assert_eq!(a.children.len(), 1);
        let n = &a.children[0];
        assert_eq!(
            (n.title.as_str(), n.url.as_str(), n.page_type.clone()),
            ("Nota Potiguar", LINK, PageType::Faq)
        );
        assert_eq!(n.faq_items.len(), 23);
        assert!(n.faq_items.iter().all(|i| i.url.is_none()));
    }
}
