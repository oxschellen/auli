// SEFAZ-PE FAQs — as páginas "Perguntas e Respostas" do IPVA e do ITCMD, páginas de publicação do mesmo
// SharePoint 2013 dos serviços (`scrape.rs`). Estudo, medições e decisões: docs/faqs/ESTUDO-PE-Faqs.md (D-PEFAQ-*).
//
// Sem API (a `_api` e o `listdata.svc` dão 403): uma lista curada de duas páginas, HTML server-side. O corpo de
// cada uma é um único campo rico (`div[id$="_RichHtmlField"]`), em que as perguntas são parágrafos numerados
// ("1. ", "5.1 - ") e a resposta é tudo até a próxima — parágrafos, listas, citações de lei e tabelas.

use std::collections::{BTreeSet, HashSet};
use std::path::Path;
use std::sync::LazyLock;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use regex::Regex;
use scraper::node::Node;
use scraper::{ElementRef, Html, Selector};
use ureq::Agent;

use auli_contract::FaqRaw;
use auli_scraper_kit::clean;
use auli_scraper_kit::faqtree::{FaqItem, FaqNode, PageType};
use auli_scraper_kit::http::GetOpts;

const BASE: &str = "https://www.sefaz.pe.gov.br";
/// As FAQs da v1 (D-PEFAQ-1): `(tributo, caminho)`. O tributo é a trilha e o nó da árvore.
const PAGINAS: &[(&str, &str)] = &[
    ("IPVA", "/Servicos/IPVA/Paginas/Perguntas-e-Respostas.aspx"),
    ("ITCMD", "/Servicos/ICD/Paginas/Perguntas-e-Respostas.aspx"),
];

/// Cortesia entre as duas páginas.
const CORTESIA: Duration = Duration::from_secs(1);

/// Coleta as FAQs e grava a árvore do frontend (`<id>-faqs-tree.json`) e o snapshot. As duas páginas são
/// coletadas e conferidas antes de qualquer gravação: rodada que falha deixa os arquivos anteriores intactos.
pub fn run(data_dir: &str, use_cache: bool) -> Result<()> {
    let agent = auli_scraper_kit::build_agent(crate::UA, Some(Duration::from_secs(60)));
    let mut paginas = Vec::new();
    for (i, (tributo, caminho)) in PAGINAS.iter().enumerate() {
        if i > 0 && !use_cache {
            sleep(CORTESIA);
        }
        let url = format!("{BASE}{caminho}");
        let html = buscar(&agent, data_dir, &url, use_cache)?;
        let perguntas =
            extrair(&html, &url).with_context(|| format!("PE FAQs: {tributo} ({url})"))?;
        println!("PE FAQs: {tributo}: {} perguntas", perguntas.len());
        paginas.push(Pagina {
            tributo,
            url,
            perguntas,
        });
    }
    let faqs = consolidar(&paginas)?;

    // Atômico pelo mesmo motivo das outras entidades: a árvore não tem outra fonte (o snapshot achatado não a
    // reconstrói) e é o único artefato de coleta que o frontend lê direto.
    let tree_path = format!("{data_dir}/{}-faqs-tree.json", crate::ENTITY);
    auli_contract::arquivo::escrever_atomico(
        Path::new(&tree_path),
        &serde_json::to_string_pretty(&montar_arvore(&paginas))?,
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

/// Rede sempre, gravando o cache; com `--usecache`, só o cache, e miss é erro (D-PEFAQ-6).
fn buscar(agent: &Agent, data_dir: &str, url: &str, use_cache: bool) -> Result<String> {
    if use_cache {
        return auli_scraper_kit::cache::read(data_dir, "faqs", url)
            .ok_or_else(|| anyhow!("cache miss para {url} (modo --usecache, sem rede)"));
    }
    let html = auli_scraper_kit::http::get_string(
        agent,
        url,
        &GetOpts {
            log_prefix: "PE FAQs",
            ..Default::default()
        },
    )?;
    auli_scraper_kit::cache::write(data_dir, "faqs", url, &html);
    Ok(html)
}

struct Pagina {
    tributo: &'static str,
    url: String,
    perguntas: Vec<Pergunta>,
}

struct Pergunta {
    titulo: String,
    resposta: String,
    /// `página#N` quando a página tem a âncora `<a name="N">` da pergunta; senão, a página.
    link: String,
    /// `Some` só com âncora: é o link próprio da pergunta na árvore.
    ancora: Option<String>,
}

static CAMPO: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(r#"div.ms-rtestate-field[id$="_RichHtmlField"]"#).unwrap());
static ANCORA: LazyLock<Selector> = LazyLock::new(|| Selector::parse("a[name]").unwrap());
/// Pergunta de 1º nível: "29. Como fica…". O espaço depois do ponto é obrigatório — na tabela de alíquotas do
/// ITCMD, "19.600,00" casaria "19.".
static NIVEL1: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\d+)\.\s+\S").unwrap());
/// Subitem: "5.1 - Quais…", "11.2 - É possível…".
static SUBITEM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\d+)\.(\d+)\s*[-–.]\s+\S").unwrap());

/// O HTML da página → as perguntas, na ordem da página.
fn extrair(html: &str, url: &str) -> Result<Vec<Pergunta>> {
    let doc = Html::parse_document(html);
    let campos: Vec<ElementRef> = doc.select(&CAMPO).collect();
    let campo = match campos.as_slice() {
        [c] => *c,
        [] => bail!("a página não tem o campo de conteúdo (div …_RichHtmlField) — o layout mudou?"),
        _ => bail!(
            "a página tem {} campos de conteúdo (div …_RichHtmlField) — o layout mudou?",
            campos.len()
        ),
    };
    let ancoras: HashSet<String> = campo
        .select(&ANCORA)
        .filter_map(|a| a.value().attr("name"))
        .map(|n| n.trim().to_string())
        .collect();
    let mut e = Escritor::default();
    andar(campo, &mut e);
    e.quebra();
    let brutas = segmentar(&e.linhas)?;
    if brutas.is_empty() {
        bail!("nenhuma pergunta numerada na página");
    }
    brutas
        .into_iter()
        .map(|b| {
            if b.resposta.is_empty() {
                bail!(
                    "a pergunta {:?} ficou sem resposta — a seguinte veio colada?",
                    b.titulo
                );
            }
            let ancora = b
                .nivel1
                .map(|n| n.to_string())
                .filter(|n| ancoras.contains(n))
                .map(|n| format!("{url}#{n}"));
            Ok(Pergunta {
                titulo: b.titulo,
                resposta: b.resposta.join("\n"),
                link: ancora.clone().unwrap_or_else(|| url.to_string()),
                ancora,
            })
        })
        .collect()
}

#[derive(Debug, PartialEq)]
enum Linha {
    Texto(String),
    /// Um `<hr>`: o IPVA fecha com um, seguido do rodapé da Gerência de IPVA, que não é resposta.
    Corte,
}

#[derive(Default)]
struct Escritor {
    linhas: Vec<Linha>,
    atual: String,
}

impl Escritor {
    fn quebra(&mut self) {
        let t = clean(&self.atual);
        if !t.is_empty() {
            self.linhas.push(Linha::Texto(t));
        }
        self.atual.clear();
    }
}

/// HTML → linhas: bloco vira quebra, `<hr>` vira corte, link vira `texto "url"` (relativo absolutizado com o
/// host). Linha de tabela com mais de uma célula vira UMA linha, células separadas por " | " (as células embrulham
/// o texto em `<p>`, que ali não quebra); a de uma célula só — as caixas "IMPORTANTE" — segue como bloco. O
/// html5ever já decodificou as entidades (`&#58;`, `&#160;`…); zero-width e nbsp saem no `clean`.
fn andar(el: ElementRef, e: &mut Escritor) {
    let tag = el.value().name();
    match tag {
        "script" | "style" => return,
        "br" => return e.quebra(),
        "hr" => {
            e.quebra();
            e.linhas.push(Linha::Corte);
            return;
        }
        "tr" => {
            let celulas: Vec<ElementRef> = el
                .children()
                .filter_map(ElementRef::wrap)
                .filter(|c| matches!(c.value().name(), "td" | "th"))
                .collect();
            if celulas.len() > 1 {
                e.quebra();
                let linha = celulas
                    .into_iter()
                    .map(|c| {
                        let mut s = Escritor::default();
                        andar(c, &mut s);
                        s.quebra();
                        let partes: Vec<String> = s
                            .linhas
                            .into_iter()
                            .filter_map(|l| match l {
                                Linha::Texto(t) => Some(t),
                                Linha::Corte => None,
                            })
                            .collect();
                        partes.join(" ")
                    })
                    .filter(|c| !c.is_empty())
                    .collect::<Vec<_>>()
                    .join(" | ");
                if !linha.is_empty() {
                    e.linhas.push(Linha::Texto(linha));
                }
                return;
            }
        }
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
                    // O texto já é a URL ("https://atendimento.sefaz.pe.gov.br/"): uma vez basta.
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
            | "blockquote"
            | "table"
            | "tbody"
            | "thead"
            | "tr"
            | "h1"
            | "h2"
            | "h3"
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
    if href.starts_with('/') {
        format!("{BASE}{href}")
    } else {
        href.to_string()
    }
}

struct Bruta {
    titulo: String,
    /// O número da pergunta de 1º nível (para a âncora); `None` nos subitens.
    nivel1: Option<u32>,
    resposta: Vec<String>,
}

/// As linhas → perguntas. Uma linha abre pergunta quando o número dela continua a sequência: `N. ` com N = a
/// anterior + 1, ou `N.M - ` com N = a atual, M = o subitem anterior + 1 e texto terminado em "?" (os "4.1 - Na
/// capital…" do ITCMD são partes da resposta 4). As listas numeradas das respostas recomeçam em 1 e ficam na
/// resposta. O que vem antes da 1ª pergunta e depois de um `<hr>` sai.
fn segmentar(linhas: &[Linha]) -> Result<Vec<Bruta>> {
    let mut perguntas: Vec<Bruta> = Vec::new();
    let (mut n1, mut sub) = (0u32, 0u32);
    let mut cortado = false;
    let mut fora = 0;
    for linha in linhas {
        let t = match linha {
            Linha::Corte => {
                cortado = true;
                continue;
            }
            Linha::Texto(t) => t,
        };
        if let Some(c) = NIVEL1.captures(t) {
            let n: u32 = c[1].parse()?;
            if n == n1 + 1 {
                (n1, sub, cortado) = (n, 0, false);
                perguntas.push(Bruta {
                    titulo: t.clone(),
                    nivel1: Some(n),
                    resposta: Vec::new(),
                });
                continue;
            }
            if n > n1 + 1 {
                bail!("a numeração pulou de {n1} para {n}: {t:?}");
            }
            // Uma lista da resposta que chegasse ao número da pergunta seguinte a "roubaria", e a verdadeira
            // apareceria depois com o número repetido.
            if n == n1 && n1 > 0 && t.ends_with('?') {
                bail!("a pergunta {n} aparece duas vezes: {t:?}");
            }
        } else if let Some(c) = SUBITEM.captures(t) {
            let (n, m): (u32, u32) = (c[1].parse()?, c[2].parse()?);
            if n == n1 && m == sub + 1 && t.ends_with('?') {
                (sub, cortado) = (m, false);
                perguntas.push(Bruta {
                    titulo: t.clone(),
                    nivel1: None,
                    resposta: Vec::new(),
                });
                continue;
            }
        }
        match perguntas.last_mut() {
            Some(p) if !cortado => p.resposta.push(t.clone()),
            _ => fora += 1,
        }
    }
    if fora > 0 {
        println!("PE FAQs: {fora} linha(s) fora de pergunta (antes da 1ª ou depois de um <hr>)");
    }
    Ok(perguntas)
}

/// O acervo, com a guarda que olha as duas páginas.
fn consolidar(paginas: &[Pagina]) -> Result<Vec<FaqRaw>> {
    let faqs: Vec<FaqRaw> = paginas
        .iter()
        .flat_map(|p| {
            p.perguntas.iter().map(move |q| FaqRaw {
                pergunta: q.titulo.clone(),
                resposta: q.resposta.clone(),
                origin: p.tributo.to_string(),
                url: q.link.clone(),
            })
        })
        .collect();
    // O nome do `.md` sai de link + pergunta: o mesmo par duas vezes, um arquivo sobrescreveria o outro.
    let mut vistos = BTreeSet::new();
    if let Some(f) = faqs
        .iter()
        .find(|f| !vistos.insert((f.url.as_str(), f.pergunta.as_str())))
    {
        bail!(
            "a pergunta {:?} aparece duas vezes em {} — os .md teriam o mesmo nome",
            f.pergunta,
            f.origin
        );
    }
    println!(
        "PE FAQs: {} perguntas em {} páginas",
        faqs.len(),
        paginas.len()
    );
    Ok(faqs)
}

/// A árvore da aba de FAQs: raiz → um `Faq` por página (o tributo). A pergunta com âncora leva o próprio link;
/// as outras usam o da página.
fn montar_arvore(paginas: &[Pagina]) -> FaqNode {
    FaqNode {
        title: "Perguntas frequentes".to_string(),
        url: BASE.to_string(),
        page_type: PageType::Menu,
        origin: String::new(),
        children: paginas
            .iter()
            .map(|p| FaqNode {
                title: p.tributo.to_string(),
                url: p.url.clone(),
                page_type: PageType::Faq,
                origin: p.tributo.to_string(),
                children: Vec::new(),
                faq_items: p
                    .perguntas
                    .iter()
                    .map(|q| FaqItem {
                        pergunta: q.titulo.clone(),
                        resposta: q.resposta.clone(),
                        url: q.ancora.clone(),
                    })
                    .collect(),
            })
            .collect(),
        faq_items: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Corpo real das páginas de 06/11/2026, dentro do `div` do campo rico. ITCMD: a página inteira (as 19
    // perguntas, os 4.1/4.2 que são parte da resposta 4, as tabelas de alíquotas com "19.600,00", a nota antes da
    // 1ª pergunta, nenhuma âncora). IPVA: as perguntas 1 a 6 (com o 5.1, as âncoras 1–4, o calendário em tabela, as
    // caixas "IMPORTANTE", listas numeradas nas respostas e links relativos), seguidas do `<hr>` e do rodapé da
    // Gerência de IPVA que fecham a página.
    const ICD: &str = include_str!("../tests/fixtures/pe-faqs-itcmd.html");
    const IPVA: &str = include_str!("../tests/fixtures/pe-faqs-ipva-recorte.html");
    const URL_ICD: &str =
        "https://www.sefaz.pe.gov.br/Servicos/ICD/Paginas/Perguntas-e-Respostas.aspx";
    const URL_IPVA: &str =
        "https://www.sefaz.pe.gov.br/Servicos/IPVA/Paginas/Perguntas-e-Respostas.aspx";

    fn erro<T>(r: Result<T>) -> String {
        match r {
            Ok(_) => panic!("esperava Err"),
            Err(e) => format!("{e:#}"),
        }
    }

    fn titulos(ps: &[Pergunta]) -> Vec<&str> {
        ps.iter().map(|p| p.titulo.as_str()).collect()
    }

    #[test]
    fn itcmd_tem_17_perguntas_e_os_subitens_com_interrogacao() {
        let ps = extrair(ICD, URL_ICD).unwrap();
        assert_eq!(ps.len(), 19);
        let t = titulos(&ps);
        assert_eq!(t[0], "1. O QUE É O ITCMD?");
        assert_eq!(t[11], "11.1 - Qual o prazo para entrar com o processo?");
        assert_eq!(t[12], "11.2 - É possível parcelar débitos de ITCMD?");
        assert_eq!(t[18], "17. COMO FAZER A DOAÇÃO DE UM IMÓVEL?");
    }

    #[test]
    fn hr_corta_so_ate_a_proxima_pergunta() {
        let campo = r#"<div id="x_RichHtmlField" class="ms-rtestate-field"><p>1. Primeira?</p><p>R1.</p>
            <hr><p>Aviso solto.</p><p>2. Segunda?</p><p>R2.</p></div>"#;
        let ps = extrair(campo, URL_IPVA).unwrap();
        assert_eq!(
            (ps[0].resposta.as_str(), ps[1].resposta.as_str()),
            ("R1.", "R2.")
        );
    }

    #[test]
    fn subitem_sem_interrogacao_fica_na_resposta() {
        let ps = extrair(ICD, URL_ICD).unwrap();
        let q4 = &ps[3];
        assert_eq!(q4.titulo, "4. COMO PAGAR O ITCMD?");
        assert!(
            q4.resposta
                .contains("4.1 - Na capital e nos municípios da região metropolitana:"),
            "{}",
            q4.resposta
        );
        assert!(
            q4.resposta
                .contains("4.2 - Nas demais localidades do Estado")
        );
    }

    #[test]
    fn tabela_vira_linhas_com_celulas_separadas() {
        let ps = extrair(ICD, URL_ICD).unwrap();
        let q10 = &ps[9];
        assert_eq!(q10.titulo, "10. QUAL A ALÍQUOTA DO ITCMD?");
        assert!(
            q10.resposta
                .contains("VALOR DO QUINHÃO, LEGADO OU DOAÇÃO | ALÍQUOTA | PARCELA A DEDUZIR"),
            "{}",
            q10.resposta
        );
        assert!(
            q10.resposta
                .lines()
                .any(|l| l.starts_with("até R$ 80.000,00 | "))
        );
        // "19.600,00" não abre pergunta: a 11 vem depois da tabela.
        assert!(q10.resposta.contains("19.600,00"));
    }

    #[test]
    fn nota_antes_da_primeira_pergunta_sai() {
        let ps = extrair(ICD, URL_ICD).unwrap();
        assert!(
            ps.iter()
                .all(|p| !p.resposta.contains("Alterações trazidas no ANEXO 6"))
        );
    }

    #[test]
    fn sem_ancora_o_link_e_a_pagina() {
        let ps = extrair(ICD, URL_ICD).unwrap();
        assert!(ps.iter().all(|p| p.link == URL_ICD && p.ancora.is_none()));
    }

    #[test]
    fn ipva_com_subitem_e_ancoras() {
        let ps = extrair(IPVA, URL_IPVA).unwrap();
        let t = titulos(&ps);
        assert_eq!(t.len(), 7);
        assert_eq!(t[0], "1. Que significa IPVA?");
        assert_eq!(
            t[5],
            "5.1 - Quais as situações em que o IPVA somente pode ser recolhido em cota única?"
        );
        assert_eq!(ps[1].link, format!("{URL_IPVA}#2"));
        assert_eq!(ps[3].ancora.as_deref(), Some(&*format!("{URL_IPVA}#4")));
        // A 5 não tem âncora, e o subitem nunca tem.
        assert_eq!(ps[4].link, URL_IPVA);
        assert_eq!(ps[5].link, URL_IPVA);
    }

    #[test]
    fn lista_numerada_da_resposta_fica_na_resposta() {
        let ps = extrair(IPVA, URL_IPVA).unwrap();
        let q3 = &ps[2];
        assert_eq!(q3.titulo, "3. Quem deve pagar IPVA ?");
        assert!(
            q3.resposta
                .contains("1. O IPVA já pago é vinculado ao veículo")
        );
        assert!(
            q3.resposta
                .contains("3. Nos casos de veículo de propriedade de empresa")
        );
    }

    #[test]
    fn link_relativo_fica_absoluto() {
        let ps = extrair(IPVA, URL_IPVA).unwrap();
        let todas = ps.iter().map(|p| p.resposta.as_str()).collect::<String>();
        assert!(
            todas.contains("\"https://www.sefaz.pe.gov.br/Legislacao/Tributaria/Documents/Legislacao/Leis_Tributarias/1992/Lei10849_92.htm\""),
            "{todas}"
        );
        assert!(!todas.contains("\"/Legislacao"));
    }

    #[test]
    fn link_cujo_texto_e_a_url_sai_uma_vez() {
        let campo = r#"<div id="x_RichHtmlField" class="ms-rtestate-field"><p>1. Onde?</p>
            <p>No <a href="https://atendimento.sefaz.pe.gov.br/">https://atendimento.sefaz.pe.gov.br/</a> ou na
            <a href="/Servicos/IPVA">página do IPVA</a>.</p></div>"#;
        let r = &extrair(campo, URL_IPVA).unwrap()[0].resposta;
        assert_eq!(
            r,
            "No https://atendimento.sefaz.pe.gov.br/ ou na página do IPVA \"https://www.sefaz.pe.gov.br/Servicos/IPVA\" ."
        );
    }

    #[test]
    fn rodape_depois_do_hr_sai() {
        let ps = extrair(IPVA, URL_IPVA).unwrap();
        let ultima = &ps.last().unwrap().resposta;
        assert!(!ultima.contains("O órgão que normatiza"), "{ultima}");
        assert!(!ultima.contains("Estamos prontos a lhe prestar serviços"));
    }

    #[test]
    fn texto_sem_tags_entidades_nem_zero_width() {
        for (html, url) in [(ICD, URL_ICD), (IPVA, URL_IPVA)] {
            for p in extrair(html, url).unwrap() {
                let t = format!("{}\n{}", p.titulo, p.resposta);
                assert!(
                    !t.contains('<') && !t.contains("&#58;") && !t.contains("&#160;"),
                    "{t}"
                );
                assert!(!t.contains('\u{200b}') && !t.contains('\u{a0}'), "{t}");
            }
        }
    }

    #[test]
    fn numeracao_que_pula_e_erro() {
        let html = ICD.replacen("5. QUAL A DOCUMENTAÇÃO", "6. QUAL A DOCUMENTAÇÃO", 1);
        assert_ne!(html, ICD);
        let e = erro(extrair(&html, URL_ICD));
        assert!(e.contains("a numeração pulou de 4 para 6"), "{e}");
    }

    #[test]
    fn numero_repetido_e_erro() {
        // O HTML quebra o texto em linhas ("7. QUAL É O\nFATO GERADOR…"): troca só o começo.
        let html = ICD.replacen("7. QUAL É O", "6. QUAL É O", 1);
        assert_ne!(html, ICD);
        let e = erro(extrair(&html, URL_ICD));
        assert!(e.contains("a pergunta 6 aparece duas vezes"), "{e}");
    }

    #[test]
    fn pergunta_sem_resposta_e_erro() {
        // A 17 é a última: sem o corpo dela, fica sem resposta.
        let i = ICD.find(">17.").unwrap() + 1;
        let html = format!(
            "{}17. COMO FAZER A DOAÇÃO DE UM IMÓVEL?</strong></p></div></body></html>",
            &ICD[..i]
        );
        let e = erro(extrair(&html, URL_ICD));
        assert!(e.contains("ficou sem resposta"), "{e}");
    }

    #[test]
    fn pagina_sem_campo_ou_com_dois_e_erro() {
        let e = erro(extrair("<html><body><p>1. Oi?</p></body></html>", URL_ICD));
        assert!(e.contains("não tem o campo de conteúdo"), "{e}");
        let dois = format!("{ICD}{ICD}");
        let e = erro(extrair(&dois, URL_ICD));
        assert!(e.contains("2 campos de conteúdo"), "{e}");
    }

    #[test]
    fn pagina_sem_pergunta_e_erro() {
        let html = r#"<div id="x_RichHtmlField" class="ms-rtestate-field"><p>Em breve.</p></div>"#;
        let e = erro(extrair(html, URL_ICD));
        assert!(e.contains("nenhuma pergunta"), "{e}");
    }

    fn paginas() -> Vec<Pagina> {
        vec![
            Pagina {
                tributo: "IPVA",
                url: URL_IPVA.into(),
                perguntas: extrair(IPVA, URL_IPVA).unwrap(),
            },
            Pagina {
                tributo: "ITCMD",
                url: URL_ICD.into(),
                perguntas: extrair(ICD, URL_ICD).unwrap(),
            },
        ]
    }

    #[test]
    fn acervo_com_a_trilha_do_tributo() {
        let faqs = consolidar(&paginas()).unwrap();
        assert_eq!(faqs.len(), 26);
        assert_eq!(
            (faqs[0].origin.as_str(), faqs[0].url.as_str()),
            ("IPVA", &*format!("{URL_IPVA}#1"))
        );
        assert_eq!(faqs[25].origin, "ITCMD");
    }

    #[test]
    fn pergunta_repetida_e_erro() {
        let mut ps = paginas();
        let t = ps[1].perguntas[0].titulo.clone();
        ps[1].perguntas[1].titulo = t;
        let e = erro(consolidar(&ps));
        assert!(e.contains("aparece duas vezes em ITCMD"), "{e}");
    }

    #[test]
    fn arvore_com_um_no_por_tributo() {
        let t = montar_arvore(&paginas());
        assert_eq!((t.page_type, t.children.len()), (PageType::Menu, 2));
        let ipva = &t.children[0];
        assert_eq!(
            (ipva.title.as_str(), ipva.page_type, ipva.url.as_str()),
            ("IPVA", PageType::Faq, URL_IPVA)
        );
        assert_eq!(
            ipva.faq_items[0].url.as_deref(),
            Some(&*format!("{URL_IPVA}#1"))
        );
        assert_eq!(ipva.faq_items[4].url, None);
        assert_eq!(t.children[1].faq_items.len(), 19);
    }
}
