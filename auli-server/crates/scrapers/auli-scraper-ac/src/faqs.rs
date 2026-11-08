// SEFAZ-AC FAQs — a aba "PERGUNTAS FREQUENTES" do post "Isenção de ICMS para PCD" (`?p=9727`), a única FAQ da SEFAZ-AC.
// O post também é uma ficha da Carta de Serviços: o texto está no acervo de serviços, e aqui cada pergunta vira um
// documento. Estudo, medições e decisões: docs/faqs/ESTUDO-AC-Faqs.md (D-ACFAQ-*).
//
// O post é um acordeão do Elementor (`.elementor-toggle-item`); a pergunta é a linha em negrito terminada em "?", pelo
// `kit::linhas` (o molde da PB e da PA). Só a aba de perguntas é lida: segmentado inteiro, o post daria à última resposta
// as abas que viessem depois.

use std::collections::HashSet;
use std::path::Path;
use std::sync::LazyLock;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use regex::Regex;
use scraper::{Html, Selector};
use ureq::Agent;

use auli_contract::FaqRaw;
use auli_scraper_kit::faqtree::{FaqItem, FaqNode, PageType};
use auli_scraper_kit::html::{Opcoes, html_para_texto};
use auli_scraper_kit::http::GetOpts;
use auli_scraper_kit::linhas::{linhas, segmentar};

use crate::ac;

/// As FAQs (D-ACFAQ-1, opção A): `(caminho, nome)`. O nome é a trilha — o título do post.
const FONTES: &[(&str, &str)] = &[("/?p=9727", "Isenção de ICMS para PCD")];
const CORTESIA: Duration = Duration::from_secs(1);
/// Piso: 17 perguntas em 07/11/2026.
const PISO: usize = 15;

/// A pergunta partida por um `<br>` entre dois negritos ("…mas não</strong><br /><strong>utilizei, posso solicitar
/// novamente?"): o `kit::linhas` quebra a linha no `<br>`, e a 1ª metade iria para a resposta anterior. Os dois viram um.
static NEGRITO_PARTIDO_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"</strong>\s*<br\s*/?>\s*<strong>").unwrap());
static ABA: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(".elementor-toggle-item").unwrap());
static TITULO_ABA: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(".elementor-toggle-title").unwrap());
static CONTEUDO_ABA: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(".elementor-tab-content").unwrap());

/// Coleta as FAQs e grava a árvore do frontend (`<id>-faqs-tree.json`) e o snapshot. Tudo é conferido antes de
/// qualquer gravação: rodada que falha deixa os arquivos anteriores intactos.
pub fn run(data_dir: &str, use_cache: bool) -> Result<()> {
    let agent = ac::build_agent();
    let mut paginas = Vec::new();
    for (i, (caminho, nome)) in FONTES.iter().enumerate() {
        if i > 0 && !use_cache {
            sleep(CORTESIA);
        }
        let url = format!("{}{caminho}", ac::BASE);
        let html = buscar(&agent, data_dir, &url, use_cache)?;
        let perguntas = extrair(&html).with_context(|| format!("AC FAQs: {nome} ({url})"))?;
        println!("AC FAQs: {nome}: {} perguntas", perguntas.len());
        paginas.push(Pagina {
            nome,
            url,
            perguntas,
        });
    }
    let faqs = consolidar(&paginas)?;
    piso(faqs.len())?;

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

/// Rede sempre, gravando o cache; com `--usecache`, só o cache, e miss é erro (D-ACFAQ-2).
fn buscar(agent: &Agent, data_dir: &str, url: &str, use_cache: bool) -> Result<String> {
    if use_cache {
        return auli_scraper_kit::cache::read(data_dir, "faqs", url)
            .ok_or_else(|| anyhow!("cache miss para {url} (modo --usecache, sem rede)"));
    }
    let html = auli_scraper_kit::http::get_string(
        agent,
        url,
        &GetOpts {
            log_prefix: "AC FAQs",
            ..Default::default()
        },
    )?;
    auli_scraper_kit::cache::write(data_dir, "faqs", url, &html);
    Ok(html)
}

struct Pagina {
    nome: &'static str,
    url: String,
    perguntas: Vec<Pergunta>,
}

struct Pergunta {
    titulo: String,
    resposta: String,
}

/// O post → as perguntas da aba "PERGUNTAS FREQUENTES", na ordem. Os negritos partidos por `<br>` se juntam antes.
fn extrair(html: &str) -> Result<Vec<Pergunta>> {
    let doc = Html::parse_document(&NEGRITO_PARTIDO_RE.replace_all(html, " "));
    let abas: Vec<_> = doc
        .select(&ABA)
        .filter(|a| {
            a.select(&TITULO_ABA).next().is_some_and(|t| {
                t.text()
                    .collect::<String>()
                    .to_uppercase()
                    .contains("PERGUNTAS")
            })
        })
        .collect();
    let [aba] = abas.as_slice() else {
        bail!(
            "{} abas \"PERGUNTAS FREQUENTES\" no post (esperava 1) — o layout mudou?",
            abas.len()
        );
    };
    let conteudo = aba
        .select(&CONTEUDO_ABA)
        .next()
        .ok_or_else(|| anyhow!("a aba de perguntas não tem o .elementor-tab-content"))?;
    let mut out = Vec::new();
    for b in segmentar(&linhas(conteudo)) {
        let resposta = html_para_texto(
            &b.resposta_html,
            Opcoes {
                base: Some(ac::BASE),
                quebras_do_fonte: false,
            },
        );
        if resposta.is_empty() {
            bail!("a pergunta {:?} ficou sem resposta", b.titulo);
        }
        out.push(Pergunta {
            titulo: b.titulo,
            resposta,
        });
    }
    if out.is_empty() {
        bail!("a aba \"PERGUNTAS FREQUENTES\" não tem pergunta");
    }
    Ok(out)
}

/// As páginas → `FaqRaw` (trilha = a página, link = a página). O par (link, pergunta) repetido é erro: o nome do `.md`
/// sai dele.
fn consolidar(paginas: &[Pagina]) -> Result<Vec<FaqRaw>> {
    let mut vistas = HashSet::new();
    let mut out = Vec::new();
    for p in paginas {
        for q in &p.perguntas {
            if !vistas.insert((p.url.as_str(), q.titulo.as_str())) {
                bail!("{} traz {:?} duas vezes", p.url, q.titulo);
            }
            out.push(FaqRaw {
                pergunta: q.titulo.clone(),
                resposta: q.resposta.clone(),
                origin: p.nome.to_string(),
                url: p.url.clone(),
            });
        }
    }
    Ok(out)
}

/// Coleta capada é `Err`, não snapshot menor.
fn piso(n: usize) -> Result<()> {
    if n < PISO {
        bail!("AC FAQs: {n} perguntas (piso {PISO}) — o portal mudou?");
    }
    Ok(())
}

/// A árvore da aba de FAQs: raiz → uma página por fonte → as perguntas.
fn montar_arvore(paginas: &[Pagina]) -> FaqNode {
    FaqNode {
        title: "Perguntas frequentes".to_string(),
        url: format!("{}/", ac::BASE),
        page_type: PageType::Menu,
        origin: String::new(),
        children: paginas
            .iter()
            .map(|p| FaqNode {
                title: p.nome.to_string(),
                url: p.url.clone(),
                page_type: PageType::Faq,
                origin: p.nome.to_string(),
                children: Vec::new(),
                faq_items: p
                    .perguntas
                    .iter()
                    .map(|q| FaqItem {
                        pergunta: q.titulo.clone(),
                        resposta: q.resposta.clone(),
                        url: Some(p.url.clone()),
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

    // O post real de 07/11/2026 sem o `<head>`, os scripts, os estilos e os SVGs: o acordeão com as 4 abas (a de
    // perguntas é a última; uma pergunta partida por `<br>` entre dois negritos).
    const POST: &str = include_str!("../tests/fixtures/ac-faqs-post-9727.html");

    fn erro<T>(r: Result<T>) -> String {
        match r {
            Ok(_) => panic!("esperava Err"),
            Err(e) => format!("{e:#}"),
        }
    }

    /// Um acordeão do Elementor com as abas `(título, conteúdo)`.
    fn acordeao(abas: &[(&str, &str)]) -> String {
        let itens: String = abas
            .iter()
            .map(|(t, c)| {
                format!(
                    r#"<div class="elementor-toggle-item"><div class="elementor-tab-title"><a class="elementor-toggle-title">{t}</a></div><div class="elementor-tab-content">{c}</div></div>"#
                )
            })
            .collect();
        format!(r#"<html><body><div class="elementor-toggle">{itens}</div></body></html>"#)
    }

    #[test]
    fn as_17_da_aba_de_perguntas() {
        let ps = extrair(POST).unwrap();
        assert_eq!(ps.len(), 17);
        assert_eq!(
            ps[0].titulo,
            "Quem tem direito a solicitar veículo com isenção de ICMS?"
        );
        assert!(ps[0].resposta.starts_with("Pessoas com deficiência física"));
        assert_eq!(ps[16].titulo, "Qual o valor da taxa?");
        // O "Perguntas frequentes:" que abre a aba não vira resposta de ninguém.
        assert!(
            ps.iter()
                .all(|p| !p.resposta.contains("Perguntas frequentes"))
        );
    }

    #[test]
    fn negrito_partido_por_br_e_uma_pergunta_so() {
        let ps = extrair(POST).unwrap();
        assert_eq!(
            ps[14].titulo,
            "Obtive a autorização para compra do veículo com isenção mas não utilizei, posso solicitar novamente?"
        );
        assert!(!ps[13].resposta.contains("Obtive"));
    }

    #[test]
    fn so_a_aba_de_perguntas() {
        let html = acordeao(&[
            (
                "ISENÇÃO",
                "<p><strong>Quem pode pedir?</strong></p><p>Todos.</p>",
            ),
            (
                "PERGUNTAS FREQUENTES",
                "<p><strong>Quanto custa?</strong></p><p>R$ 13.</p>",
            ),
            ("ANEXO", "<p>Formulário.</p>"),
        ]);
        let ps = extrair(&html).unwrap();
        assert_eq!(ps.len(), 1);
        assert_eq!(ps[0].titulo, "Quanto custa?");
        assert_eq!(ps[0].resposta, "R$ 13.");
    }

    #[test]
    fn aba_ausente_repetida_vazia_ou_pergunta_sem_resposta_e_erro() {
        assert!(erro(extrair(&acordeao(&[("ANEXO", "<p>x</p>")]))).contains("0 abas"));
        let faq = (
            "PERGUNTAS FREQUENTES",
            "<p><strong>P?</strong></p><p>R.</p>",
        );
        assert!(erro(extrair(&acordeao(&[faq, faq]))).contains("2 abas"));
        let vazia = acordeao(&[("PERGUNTAS FREQUENTES", "<p>Nada.</p>")]);
        assert!(erro(extrair(&vazia)).contains("não tem pergunta"));
        let sem = acordeao(&[("PERGUNTAS FREQUENTES", "<p><strong>P?</strong></p>")]);
        assert!(erro(extrair(&sem)).contains("sem resposta"));
    }

    fn pagina(perguntas: &[(&str, &str)]) -> Pagina {
        Pagina {
            nome: "Isenção de ICMS para PCD",
            url: "https://sefaz.ac.gov.br/2021/?p=9727".into(),
            perguntas: perguntas
                .iter()
                .map(|(t, r)| Pergunta {
                    titulo: t.to_string(),
                    resposta: r.to_string(),
                })
                .collect(),
        }
    }

    #[test]
    fn trilha_e_link_do_post_e_repetida_e_erro() {
        let faqs = consolidar(&[pagina(&[("P?", "R.")])]).unwrap();
        assert_eq!(faqs[0].origin, "Isenção de ICMS para PCD");
        assert_eq!(faqs[0].url, "https://sefaz.ac.gov.br/2021/?p=9727");
        assert!(erro(consolidar(&[pagina(&[("P?", "R."), ("P?", "S.")])])).contains("duas vezes"));
    }

    #[test]
    fn piso() {
        assert!(super::piso(PISO).is_ok());
        assert!(erro(super::piso(PISO - 1)).contains("piso"));
    }

    #[test]
    fn arvore_raiz_post_perguntas() {
        let t = montar_arvore(&[pagina(&[("P?", "R."), ("Q?", "S.")])]);
        assert_eq!(t.children.len(), 1);
        assert_eq!(t.children[0].title, "Isenção de ICMS para PCD");
        assert_eq!(t.children[0].faq_items.len(), 2);
        assert_eq!(
            t.children[0].faq_items[0].url.as_deref(),
            Some("https://sefaz.ac.gov.br/2021/?p=9727")
        );
    }
}
