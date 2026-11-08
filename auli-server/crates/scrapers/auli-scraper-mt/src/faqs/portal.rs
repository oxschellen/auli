// As perguntas da SEFAZ na FAQ do portal do Estado (`portal.mt.gov.br/perguntas-frequentes`, X-Via) — D-MTFAQ-10 e
// -12 (docs/faqs/ESTUDO-MT-Faqs.md §9). Duas chamadas, sem sessão: o órgão (`GET /v1/department/<slug>`, de onde vem o id —
// não fixo no código) e as perguntas dele (`POST /v1/faq {"uo": [<id>]}`), cada uma com a página própria
// (`/perguntas-frequentes/<id>`), a resposta em HTML, a categoria e o status.

use std::collections::BTreeMap;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use auli_scraper_kit::clean;
use auli_scraper_kit::html::{Opcoes, html_para_texto};

use super::{Grupo, Pergunta, Rede};

pub(super) const TITULO: &str = "Portal MT";
/// A página da FAQ; a de cada pergunta é `<FAQ>/<id>`.
pub(super) const FAQ: &str = "https://portal.mt.gov.br/perguntas-frequentes";
const PORTAL: &str = "https://portal.mt.gov.br";
const ORGAO: &str = "https://portal.mt.gov.br/v1/department/secretaria-de-estado-de-fazenda";
const API: &str = "https://portal.mt.gov.br/v1/faq";

#[derive(Deserialize)]
struct Orgao {
    id: String,
}

#[derive(Deserialize)]
struct Item {
    id: String,
    pergunta: String,
    resposta: String,
    status: String,
    /// "Serviços" ou "Institucional": o grupo de quem não tem categoria.
    tipo: String,
    categoria: Option<Nome>,
    uo: Option<Orgao>,
}

#[derive(Deserialize)]
struct Nome {
    nome: String,
}

/// Um grupo por categoria (sem categoria, o tipo), na ordem do nome; dentro dele, na do id (o ObjectId cresce com a
/// criação). A API não garante ordem: ordenado, o snapshot só muda quando a FAQ muda.
pub(super) fn coletar(rede: &Rede) -> Result<Vec<Grupo>> {
    let orgao: Orgao = serde_json::from_str(&rede.get(ORGAO, "application/json")?)
        .context("o órgão fora do formato {id, …}")?;
    let corpo = serde_json::json!({ "uo": [orgao.id] });
    let itens: Vec<Item> = serde_json::from_str(&rede.post(API, &corpo, PORTAL)?)
        .context("a FAQ fora do formato [{id, pergunta, resposta, …}]")?;
    agrupar(itens, &orgao.id)
}

fn agrupar(itens: Vec<Item>, orgao: &str) -> Result<Vec<Grupo>> {
    let mut grupos: BTreeMap<String, Vec<Item>> = BTreeMap::new();
    let mut inativas = 0;
    for i in itens {
        // O filtro é do servidor: pergunta de outro órgão é a API que mudou.
        if i.uo.as_ref().map(|u| u.id.as_str()) != Some(orgao) {
            bail!(
                "a pergunta {} não é da SEFAZ — o filtro por órgão mudou?",
                i.id
            );
        }
        if i.status != "Ativo" {
            inativas += 1;
            continue;
        }
        let grupo = i.categoria.as_ref().map_or(&i.tipo, |c| &c.nome).clone();
        grupos.entry(clean(&grupo)).or_default().push(i);
    }
    if inativas > 0 {
        eprintln!("⚠️  MT FAQs: {inativas} pergunta(s) inativa(s) do Portal MT fora do acervo");
    }
    if grupos.is_empty() {
        bail!("nenhuma pergunta da SEFAZ no Portal MT — a API mudou?");
    }
    Ok(grupos
        .into_iter()
        .map(|(nome, mut itens)| {
            itens.sort_by(|a, b| a.id.cmp(&b.id));
            Grupo {
                nome,
                url: FAQ.to_string(),
                perguntas: itens
                    .into_iter()
                    .map(|i| Pergunta {
                        pergunta: clean(&i.pergunta),
                        resposta: html_para_texto(
                            &i.resposta,
                            Opcoes {
                                base: Some(PORTAL),
                                quebras_do_fonte: false,
                            },
                        ),
                        url: Some(format!("{FAQ}/{}", i.id)),
                    })
                    .collect(),
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::super::rede_de_teste;
    use super::*;

    // As duas respostas reais de 07/11/2026: o órgão (o id da SEFAZ) e as 11 perguntas dele — 5 de Finanças e
    // Tributação (duas com espaço no começo da pergunta), 5 de Comunicação e Transparência e 1 sem categoria, do tipo
    // "Institucional". Na ordem da API, que não é a do id.
    const ORGAO_JSON: &str = include_str!("../../tests/fixtures/mt-faqs-portal-orgao.json");
    const FAQ_JSON: &str = include_str!("../../tests/fixtures/mt-faqs-portal-faq.json");
    const SEFAZ: &str = "6b95c1ea-6693-4644-9bc0-26b6093b9428";

    fn itens() -> Vec<Item> {
        serde_json::from_str(FAQ_JSON).unwrap()
    }

    fn erro<T>(r: Result<T>) -> String {
        match r {
            Ok(_) => panic!("esperava Err"),
            Err(e) => format!("{e:#}"),
        }
    }

    #[test]
    fn agrupa_pela_categoria_ou_pelo_tipo_na_ordem_do_nome() {
        let grupos = agrupar(itens(), SEFAZ).unwrap();
        let resumo: Vec<(&str, usize)> = grupos
            .iter()
            .map(|g| (g.nome.as_str(), g.perguntas.len()))
            .collect();
        assert_eq!(
            resumo,
            [
                ("Comunicação e Transparência", 5),
                ("Finanças e Tributação", 5),
                ("Institucional", 1)
            ]
        );
        assert!(grupos.iter().all(|g| g.url == FAQ));
    }

    #[test]
    fn cada_pergunta_tem_a_propria_pagina_na_ordem_do_id() {
        // Do avesso: a API já manda na ordem do id, e é o `agrupar` que tem de garanti-la.
        let mut i = itens();
        i.reverse();
        let grupos = agrupar(i, SEFAZ).unwrap();
        let financas = &grupos[1].perguntas;
        assert_eq!(
            financas[0].url.as_deref(),
            Some("https://portal.mt.gov.br/perguntas-frequentes/699c48affc237a4a88888cac")
        );
        assert_eq!(
            financas[0].pergunta,
            "Quem é considerado contribuinte do ICMS em MT?"
        );
        assert_eq!(
            financas[3].pergunta,
            "Onde consultar a situação cadastral de uma empresa estabelecida no Estado Mato Grosso?"
        );
        let ids: Vec<&str> = financas.iter().map(|p| p.url.as_deref().unwrap()).collect();
        let mut ordenados = ids.clone();
        ordenados.sort();
        assert_eq!(ids, ordenados);
        let institucional = &grupos[2].perguntas[0];
        assert!(
            institucional
                .resposta
                .starts_with("É o acesso restrito, de uso do contribuinte"),
            "{}",
            institucional.resposta
        );
        assert!(
            grupos
                .iter()
                .flat_map(|g| &g.perguntas)
                .all(|p| !p.resposta.contains('<') && !p.resposta.is_empty())
        );
    }

    #[test]
    fn pergunta_de_outro_orgao_e_erro() {
        let mut i = itens();
        i[4].uo = None;
        assert!(erro(agrupar(i, SEFAZ)).contains("não é da SEFAZ"));
        let e = erro(agrupar(itens(), "outro-orgao"));
        assert!(
            e.contains("a pergunta 699c48affc237a4a88888cac não é da SEFAZ"),
            "{e}"
        );
    }

    #[test]
    fn pergunta_inativa_fica_fora() {
        let mut i = itens();
        let alvo = i
            .iter_mut()
            .find(|i| i.id == "699c4e923f1079070914b3b5")
            .unwrap();
        alvo.status = "Inativo".into();
        let grupos = agrupar(i, SEFAZ).unwrap();
        assert_eq!(grupos.len(), 2);
        assert!(grupos.iter().all(|g| g.nome != "Institucional"));
    }

    #[test]
    fn sem_pergunta_e_erro() {
        assert!(erro(agrupar(Vec::new(), SEFAZ)).contains("nenhuma pergunta da SEFAZ"));
    }

    #[test]
    fn coletar_pede_as_perguntas_pelo_id_do_orgao() {
        let corpo = serde_json::json!({ "uo": [SEFAZ] });
        let post = format!("{API} {corpo}");
        let rede = rede_de_teste("portal", &[(ORGAO, ORGAO_JSON), (&post, FAQ_JSON)]);
        let grupos = coletar(&rede).unwrap();
        assert_eq!(grupos.iter().map(|g| g.perguntas.len()).sum::<usize>(), 11);
    }

    #[test]
    fn orgao_ou_faq_fora_do_formato_e_erro() {
        let rede = rede_de_teste("portal-orgao", &[(ORGAO, r#"{"erro":"x"}"#)]);
        assert!(erro(coletar(&rede)).contains("o órgão fora do formato"));
        let corpo = serde_json::json!({ "uo": [SEFAZ] });
        let post = format!("{API} {corpo}");
        let rede = rede_de_teste("portal-faq", &[(ORGAO, ORGAO_JSON), (&post, "{}")]);
        assert!(erro(coletar(&rede)).contains("a FAQ fora do formato"));
    }
}
