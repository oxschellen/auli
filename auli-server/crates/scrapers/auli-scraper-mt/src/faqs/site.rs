// As "Perguntas frequentes" das áreas de serviço do site da SEFAZ (`www5.sefaz.mt.gov.br`, Liferay) — D-MTFAQ-10,
// -11 e -13 (docs/ESTUDO-MT-Faqs.md §9).
//
// A enumeração é a do próprio site, nunca uma lista no código: a página "Todos os Serviços" lista as áreas; o menu
// de cada área (`div.mosaic-relateds-list`) tem, ou não, o item "Perguntas frequentes"; a página dele traz as
// perguntas no `div.mosaic-content`, em negrito e terminadas em "?" (`kit::linhas`). Em 07/11/2026, 5 das 24 áreas
// têm o item: o Contencioso, as Declarações Fiscais (DIMP) e os Documentos Fiscais (NFA-e) com perguntas; a
// Autorregularização com a página vazia; o ITCD apontando para o Portal do Conhecimento, que não as tem mais. Página
// sem pergunta é aviso. A cortesia entre as chamadas é o `Crawl-delay: 30` do site (`Rede`).

use std::collections::HashSet;
use std::sync::LazyLock;

use anyhow::{Context, Result, bail};
use scraper::{Html, Selector};

use auli_scraper_kit::html::{Opcoes, html_para_texto};
use auli_scraper_kit::linhas::{linhas, segmentar};

use super::{Grupo, Pergunta, Rede};

pub(super) const TITULO: &str = "SEFAZ-MT";
/// "Todos os Serviços": as áreas.
pub(super) const TODOS: &str = "https://www5.sefaz.mt.gov.br/servicos";
const SITE: &str = "https://www5.sefaz.mt.gov.br";
/// 24 em 07/11/2026: menos que isto é o layout que mudou, não áreas que sumiram.
const MIN_AREAS: usize = 15;

static AREA: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(r#"a[href*="/servicos?c="]"#).unwrap());
static MENU: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("div.mosaic-relateds-list a").unwrap());
static CONTEUDO: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("div.mosaic-content").unwrap());

/// Um grupo por área com perguntas, na ordem de "Todos os Serviços".
pub(super) fn coletar(rede: &Rede) -> Result<Vec<Grupo>> {
    let areas = areas(&rede.get(TODOS, "text/html")?);
    if areas.len() < MIN_AREAS {
        bail!(
            "\"Todos os Serviços\" com {} áreas (mínimo {MIN_AREAS}) — o layout mudou?",
            areas.len()
        );
    }
    let mut vistas = HashSet::new();
    let mut grupos = Vec::new();
    for (nome, url) in areas {
        let Some(faq) = link_faq(&rede.get(&url, "text/html")?) else {
            continue;
        };
        if !vistas.insert(faq.clone()) {
            continue;
        }
        let perguntas =
            perguntas(&rede.get(&faq, "text/html")?).with_context(|| format!("{nome}: {faq}"))?;
        if perguntas.is_empty() {
            println!(
                "⚠️  MT FAQs: {nome}: a página de perguntas frequentes não tem pergunta ({faq})"
            );
            continue;
        }
        grupos.push(Grupo {
            nome,
            url: faq,
            perguntas,
        });
    }
    if grupos.is_empty() {
        bail!("nenhuma área com perguntas frequentes — o layout mudou?");
    }
    Ok(grupos)
}

/// As áreas de "Todos os Serviços": `(nome, url)`, sem repetir o link.
fn areas(html: &str) -> Vec<(String, String)> {
    let doc = Html::parse_document(html);
    let mut vistos = HashSet::new();
    doc.select(&AREA)
        .filter_map(|a| {
            let nome = texto(&a.text().collect::<String>());
            let url = absoluto(a.value().attr("href")?);
            (!nome.is_empty() && vistos.insert(url.clone())).then_some((nome, url))
        })
        .collect()
}

/// O item "Perguntas frequentes" do menu da área, se houver.
fn link_faq(html: &str) -> Option<String> {
    Html::parse_document(html)
        .select(&MENU)
        .find(|a| texto(&a.text().collect::<String>()).eq_ignore_ascii_case("perguntas frequentes"))
        .and_then(|a| a.value().attr("href").map(absoluto))
}

/// As perguntas do conteúdo da página (exatamente um `div.mosaic-content`). A resposta perde o "Resposta:" do começo
/// (o Contencioso o põe em todas).
fn perguntas(html: &str) -> Result<Vec<Pergunta>> {
    let doc = Html::parse_document(html);
    let conteudos: Vec<_> = doc.select(&CONTEUDO).collect();
    let [conteudo] = conteudos[..] else {
        bail!(
            "{} blocos de conteúdo (div.mosaic-content) — o layout mudou?",
            conteudos.len()
        );
    };
    segmentar(&linhas(conteudo))
        .into_iter()
        .map(|b| {
            let resposta = html_para_texto(
                &b.resposta_html,
                Opcoes {
                    base: Some(SITE),
                    quebras_do_fonte: false,
                },
            );
            let resposta = resposta
                .strip_prefix("Resposta:")
                .map_or(resposta.as_str(), str::trim_start)
                .to_string();
            if resposta.is_empty() {
                bail!("a pergunta {:?} ficou sem resposta", b.titulo);
            }
            Ok(Pergunta {
                pergunta: b.titulo,
                resposta,
                url: None,
            })
        })
        .collect()
}

fn absoluto(href: &str) -> String {
    if href.starts_with("http") {
        href.to_string()
    } else {
        format!("{SITE}/{}", href.trim_start_matches('/'))
    }
}

fn texto(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::super::rede_de_teste;
    use super::*;

    // Páginas reais de 07/11/2026, sem `<script>`, `<style>` e comentários: "Todos os Serviços"; as áreas Declarações
    // Fiscais (com o item "Perguntas frequentes" no menu) e Calculadora do Difal (sem); as perguntas da NFA-e (16, a 6ª
    // partida em dois negritos, "…da NFA-" + "e?"), as do Contencioso (21, toda resposta começando em "Resposta:") e
    // a página vazia da Autorregularização.
    const SERVICOS: &str = include_str!("../../tests/fixtures/mt-faqs-www5-servicos.html");
    const AREA_COM: &str = include_str!("../../tests/fixtures/mt-faqs-www5-area-declaracoes.html");
    const AREA_SEM: &str = include_str!("../../tests/fixtures/mt-faqs-www5-area-difal.html");
    const NFAE: &str = include_str!("../../tests/fixtures/mt-faqs-www5-nfae.html");
    const CONTENCIOSO: &str = include_str!("../../tests/fixtures/mt-faqs-www5-contencioso.html");
    const VAZIA: &str = include_str!("../../tests/fixtures/mt-faqs-www5-autorregularizacao.html");

    fn erro<T>(r: Result<T>) -> String {
        match r {
            Ok(_) => panic!("esperava Err"),
            Err(e) => format!("{e:#}"),
        }
    }

    #[test]
    fn areas_de_todos_os_servicos() {
        let a = areas(SERVICOS);
        assert_eq!(a.len(), 24);
        assert_eq!(
            a[0],
            (
                "Autorregularização".to_string(),
                format!("{TODOS}?c=6227552&e=6347163")
            )
        );
        assert_eq!(a[23].0, "USC");
        assert!(a.contains(&(
            "Documentos Fiscais".to_string(),
            format!("{TODOS}?c=16773297&e=74921623")
        )));
        // O link de ícone (sem texto) não é área, e o mesmo link repetido (num menu) não conta duas vezes.
        let cartao = r#"<a href="/servicos?c=1&e=1"><img src="i.png"></a><a href="/servicos?c=1&e=1">IPVA</a>
            <a href="/servicos?c=1&e=1">IPVA</a>"#;
        assert_eq!(
            areas(cartao),
            [("IPVA".to_string(), format!("{TODOS}?c=1&e=1"))]
        );
    }

    #[test]
    fn link_faq_so_no_menu_da_area() {
        assert_eq!(
            link_faq(AREA_COM).as_deref(),
            Some("https://www5.sefaz.mt.gov.br/servicos?c=75259024&e=75467035&s=75468011")
        );
        assert_eq!(link_faq(AREA_SEM), None);
        // O texto fora do menu não conta.
        let fora = r#"<p><a href="/servicos?c=1&e=2">Perguntas frequentes</a></p>
            <div class="mosaic-relateds-list"><a href="/servicos?c=1&e=3">Legislação</a></div>"#;
        assert_eq!(link_faq(fora), None);
    }

    #[test]
    fn perguntas_da_nfae() {
        let p = perguntas(NFAE).unwrap();
        assert_eq!(p.len(), 16);
        assert_eq!(p[0].pergunta, "O que é a NFA-e?");
        assert_eq!(
            p[5].pergunta,
            "Quais são os requisitos para cancelamento extemporâneo da NFA-e?"
        );
        assert!(
            p[5].resposta
                .starts_with("Para cancelamento extemporâneo da NFA-e, o usuário deve atender"),
            "{}",
            p[5].resposta
        );
        assert_eq!(
            p[15].pergunta,
            "Quais são as consequências do registro de eventos da NFA-e?"
        );
        assert!(
            p.iter()
                .all(|p| p.url.is_none() && !p.resposta.contains('<'))
        );
    }

    #[test]
    fn contencioso_perde_o_resposta_do_comeco() {
        let p = perguntas(CONTENCIOSO).unwrap();
        assert_eq!(p.len(), 21);
        assert_eq!(
            p[0].pergunta,
            "1. O que é o Contencioso Administrativo Tributário?"
        );
        assert!(
            p[0].resposta
                .starts_with("O Contencioso Administrativo Tributário (PAT) é o litígio"),
            "{}",
            p[0].resposta
        );
        assert!(p.iter().all(|p| !p.resposta.starts_with("Resposta")));
    }

    #[test]
    fn pagina_sem_pergunta_e_vazia_nao_erro() {
        assert!(perguntas(VAZIA).unwrap().is_empty());
    }

    #[test]
    fn conteudo_ausente_ou_repetido_e_erro() {
        let e = erro(perguntas("<html><body><p>nada</p></body></html>"));
        assert!(e.contains("0 blocos de conteúdo"), "{e}");
        let dois = r#"<div class="mosaic-content"></div><div class="mosaic-content"></div>"#;
        assert!(erro(perguntas(dois)).contains("2 blocos de conteúdo"));
    }

    #[test]
    fn pergunta_so_com_o_rotulo_de_resposta_e_erro() {
        let html =
            r#"<div class="mosaic-content"><p><strong>Pode?</strong></p><p>Resposta:</p></div>"#;
        let e = erro(perguntas(html));
        assert!(e.contains("\"Pode?\" ficou sem resposta"), "{e}");
    }

    /// Um site de mentira com 15 áreas: a 1 e a 2 apontam para a mesma página de perguntas, a 3 para uma página vazia,
    /// a 4 para outra por link absoluto, com o item escrito de outro jeito; as demais não têm o item.
    fn site(n_areas: usize) -> Vec<(String, String)> {
        let area = |i: usize| format!("{TODOS}?c={i}&e={i}");
        let menu = |href: &str, texto: &str| {
            format!(r#"<div class="mosaic-relateds-list"><a href="{href}">{texto}</a></div>"#)
        };
        let faq = |corpo: &str| format!(r#"<div class="mosaic-content">{corpo}</div>"#);
        let todos: String = (1..=n_areas)
            .map(|i| format!(r#"<a href="/servicos?c={i}&e={i}">Área {i}</a>"#))
            .collect();
        let mut r = vec![(TODOS.to_string(), todos)];
        for i in 1..=n_areas {
            let pagina = match i {
                1 => menu("/servicos?c=1&e=100", "Perguntas frequentes"),
                2 => menu("/servicos?c=1&e=100", "Perguntas frequentes"),
                3 => menu("/servicos?c=3&e=300", "Perguntas frequentes"),
                4 => menu(&format!("{TODOS}?c=4&e=400"), " Perguntas \n Frequentes "),
                _ => menu("/servicos?c=9&e=9", "Legislação"),
            };
            r.push((area(i), pagina));
        }
        r.push((
            format!("{TODOS}?c=1&e=100"),
            faq("<p><strong>Um?</strong></p><p>Resposta: primeira.</p><p><strong>Dois?</strong></p><p>segunda.</p>"),
        ));
        r.push((format!("{TODOS}?c=3&e=300"), faq("<p>Em breve.</p>")));
        r.push((
            format!("{TODOS}?c=4&e=400"),
            faq("<p><strong>Três?</strong></p><p>terceira.</p>"),
        ));
        r
    }

    fn rede(nome: &str, n_areas: usize) -> Rede<'static> {
        let s = site(n_areas);
        let r: Vec<(&str, &str)> = s.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        rede_de_teste(nome, &r)
    }

    #[test]
    fn coletar_segue_o_menu_de_cada_area_sem_repetir_a_pagina() {
        let grupos = coletar(&rede("site", 15)).unwrap();
        let resumo: Vec<(&str, &str, usize)> = grupos
            .iter()
            .map(|g| (g.nome.as_str(), g.url.as_str(), g.perguntas.len()))
            .collect();
        assert_eq!(
            resumo,
            [
                (
                    "Área 1",
                    "https://www5.sefaz.mt.gov.br/servicos?c=1&e=100",
                    2
                ),
                (
                    "Área 4",
                    "https://www5.sefaz.mt.gov.br/servicos?c=4&e=400",
                    1
                ),
            ]
        );
        let um = &grupos[0].perguntas;
        assert_eq!(
            (um[0].pergunta.as_str(), um[0].resposta.as_str()),
            ("Um?", "primeira.")
        );
        assert_eq!(um[1].resposta, "segunda.");
    }

    #[test]
    fn poucas_areas_e_erro() {
        let e = erro(coletar(&rede("site-poucas", 14)));
        assert!(e.contains("com 14 áreas (mínimo 15)"), "{e}");
    }

    #[test]
    fn nenhuma_area_com_perguntas_e_erro() {
        let mut s = site(15);
        s.retain(|(k, _)| !k.ends_with("e=100") && !k.ends_with("e=400"));
        s.push((
            format!("{TODOS}?c=1&e=100"),
            r#"<div class="mosaic-content"></div>"#.into(),
        ));
        s.push((
            format!("{TODOS}?c=4&e=400"),
            r#"<div class="mosaic-content"></div>"#.into(),
        ));
        let r: Vec<(&str, &str)> = s.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        let e = erro(coletar(&rede_de_teste("site-nenhuma", &r)));
        assert!(e.contains("nenhuma área com perguntas frequentes"), "{e}");
    }
}
