// SEFA-PA FAQs — as páginas de perguntas e respostas do site antigo da SEFA (`antigo.sefa.pa.gov.br`, Joomla), para
// onde o menu "Orientações" do site novo aponta. Estudo, medições e decisões: docs/faqs/ESTUDO-PA-Faqs.md (D-PAFAQ-*).
//
// Não há FAQ central (o índice "Perguntas e Respostas" do Manual de Atendimento é uma lista de links, a maioria PDF):
// uma lista curada de 11 páginas (D-PAFAQ-1, opção A). A pergunta é um trecho em negrito no meio do texto, como na
// PB — a segmentação é a do kit (`kit::linhas`, D-PAFAQ-2, -3), que também cobre as páginas cujas perguntas são só
// numeradas, sem negrito (PIX, Parcelamento, Imunidades).

use std::collections::HashSet;
use std::path::Path;
use std::sync::LazyLock;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use scraper::{Html, Selector};
use ureq::Agent;

use auli_contract::FaqRaw;
use auli_scraper_kit::faqtree::{FaqItem, FaqNode, PageType};
use auli_scraper_kit::html::{Opcoes, html_para_texto};
use auli_scraper_kit::http::GetOpts;
use auli_scraper_kit::linhas::{
    CLOAK_TEXTO, linhas, numeracao_irregular, segmentar, sem_cloak, sem_prefixo_r,
};

const HOST: &str = "https://antigo.sefa.pa.gov.br";
/// A página do índice "Perguntas e Respostas", raiz da árvore.
const INDICE: &str = "https://antigo.sefa.pa.gov.br/orientacoes/27-orientacoes/manual-de-atendimento/11578-perguntas-e-respostas";

/// As FAQs da v1 (D-PAFAQ-1, opção A): `(caminho, nome)`. O caminho é o link do título da página (o canônico do
/// Joomla); o nome é a trilha. A Alíquota Modal 19% existe em dois artigos idênticos (20715 e 20716): fica o 20716,
/// o do índice.
const FONTES: &[(&str, &str)] = &[
    (
        "/orientacoes/27-orientacoes/manual-de-atendimento/20716-aliquota-modal",
        "Alíquota Modal 19%",
    ),
    ("/27-orientacoes/manual-de-atendimento/21546-pix", "PIX"),
    (
        "/28-orientacoes/manual-de-atendimento/imunidades-e-isencoes/17525-duvidas-frequentes",
        "Imunidades e Isenções",
    ),
    (
        "/38-orientacoes/18084-perguntas-e-respostas",
        "Selo Fiscal de Controle e Qualidade",
    ),
    (
        "/noticias/13401-perguntas-frequentes-parcelamento-de-debitos",
        "Parcelamento de débitos",
    ),
    (
        "/129-downloads/dief/11586-dief-2016-perguntas-e-respostas",
        "DIEF 2016",
    ),
    (
        "/119-orientacoes/sintegra/1439-duvidas-frequentes",
        "SINTEGRA",
    ),
    (
        "/orientacoes/115-orientacoes/sped/1575-faq-nfe",
        "NF-e - Manifestação do Destinatário",
    ),
    (
        "/orientacoes/121-orientacoes/sped/nfe/1475-perguntas-respostas",
        "NF-e - Denegação",
    ),
    (
        "/orientacoes/116-orientacoes/sped/efd/1427-efd-nfe",
        "EFD - Cruzamento EFD x NF-e",
    ),
    (
        "/orientacoes/116-orientacoes/sped/efd/1426-efd-estadual",
        "EFD Estadual",
    ),
];

/// Cortesia entre as páginas (D-PAFAQ-5).
const CORTESIA: Duration = Duration::from_secs(1);
/// Piso do acervo: ~90 % das 152 perguntas de 07/11/2026.
const PISO: usize = 135;

/// Coleta as FAQs e grava a árvore do frontend (`<id>-faqs-tree.json`) e o snapshot. Tudo é coletado e conferido
/// antes de qualquer gravação: rodada que falha deixa os arquivos anteriores intactos.
pub fn run(data_dir: &str, use_cache: bool) -> Result<()> {
    let agent = auli_scraper_kit::build_agent(crate::UA, Some(Duration::from_secs(60)));
    let mut paginas = Vec::new();
    for (i, (caminho, nome)) in FONTES.iter().enumerate() {
        if i > 0 && !use_cache {
            sleep(CORTESIA);
        }
        let url = format!("{HOST}{caminho}");
        let html = buscar(&agent, data_dir, &url, use_cache)?;
        let perguntas = extrair(&html).with_context(|| format!("PA FAQs: {nome} ({url})"))?;
        println!("PA FAQs: {nome}: {} perguntas", perguntas.len());
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

/// Rede sempre, gravando o cache; com `--usecache`, só o cache, e miss é erro (D-PAFAQ-5).
fn buscar(agent: &Agent, data_dir: &str, url: &str, use_cache: bool) -> Result<String> {
    if use_cache {
        return auli_scraper_kit::cache::read(data_dir, "faqs", url)
            .ok_or_else(|| anyhow!("cache miss para {url} (modo --usecache, sem rede)"));
    }
    let html = auli_scraper_kit::http::get_string(
        agent,
        url,
        &GetOpts {
            log_prefix: "PA FAQs",
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
    /// O subgrupo da página ("… - PERGUNTAS FREQUENTES"); vazio quando não há.
    grupo: String,
}

/// O artigo do Joomla: título, data e corpo. O título e a linha de data vêm antes da 1ª pergunta e saem.
static CORPO: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("div.w3-card-12.internas").unwrap());

/// A página → as perguntas, na ordem da página.
fn extrair(html: &str) -> Result<Vec<Pergunta>> {
    let doc = Html::parse_document(&sem_cloak(html));
    let corpo = doc.select(&CORPO).next().ok_or_else(|| {
        anyhow!("a página não tem o artigo (div.w3-card-12.internas) — o layout mudou?")
    })?;
    let brutas = segmentar(&linhas(corpo));
    for (grupo, nums) in numeracao_irregular(&brutas) {
        println!(
            "PA FAQs: aviso — numeração {nums:?}{}",
            if grupo.is_empty() {
                String::new()
            } else {
                format!(" ({grupo})")
            }
        );
    }
    let mut perguntas = Vec::new();
    for b in brutas {
        let resposta = sem_prefixo_r(&html_para_texto(
            &b.resposta_html,
            Opcoes {
                base: Some(HOST),
                quebras_do_fonte: false,
            },
        ));
        if resposta.is_empty() {
            bail!("a pergunta {:?} ficou sem resposta", b.titulo);
        }
        if b.titulo.contains(CLOAK_TEXTO) || resposta.contains(CLOAK_TEXTO) {
            bail!(
                "sobrou um e-mail protegido do Joomla em {:?} — o plugin mudou?",
                b.titulo
            );
        }
        perguntas.push(Pergunta {
            titulo: b.titulo,
            resposta,
            grupo: b.grupo,
        });
    }
    if perguntas.is_empty() {
        bail!("nenhuma pergunta na página");
    }
    Ok(perguntas)
}

/// `Página [| Grupo]` (D-PAFAQ-4).
fn trilha(p: &Pagina, q: &Pergunta) -> String {
    if q.grupo.is_empty() {
        p.nome.to_string()
    } else {
        format!("{} | {}", p.nome, q.grupo)
    }
}

/// O acervo, com a guarda que olha todas as páginas: o nome do `.md` sai de link + pergunta, e o mesmo par duas vezes
/// sobrescreveria um arquivo.
fn consolidar(paginas: &[Pagina]) -> Result<Vec<FaqRaw>> {
    let mut vistos = HashSet::new();
    let mut faqs = Vec::new();
    for p in paginas {
        for q in &p.perguntas {
            if !vistos.insert((p.url.as_str(), q.titulo.as_str())) {
                bail!("a pergunta {:?} aparece duas vezes em {}", q.titulo, p.url);
            }
            faqs.push(FaqRaw {
                pergunta: q.titulo.clone(),
                resposta: q.resposta.clone(),
                origin: trilha(p, q),
                url: p.url.clone(),
            });
        }
    }
    println!(
        "PA FAQs: {} perguntas em {} páginas",
        faqs.len(),
        paginas.len()
    );
    Ok(faqs)
}

fn piso(n: usize) -> Result<()> {
    if n < PISO {
        bail!("só {n} perguntas, abaixo do piso de {PISO} — alguma página encolheu?");
    }
    Ok(())
}

/// A árvore da aba de FAQs: raiz → um nó por página → um nó por grupo, quando a página tem. As perguntas sem grupo
/// ficam no nó da página.
fn montar_arvore(paginas: &[Pagina]) -> FaqNode {
    let item = |q: &Pergunta| FaqItem {
        pergunta: q.titulo.clone(),
        resposta: q.resposta.clone(),
        url: None,
    };
    let filhos = paginas
        .iter()
        .map(|p| {
            let mut diretos = Vec::new();
            let mut grupos: Vec<(String, Vec<FaqItem>)> = Vec::new();
            for q in &p.perguntas {
                if q.grupo.is_empty() {
                    diretos.push(item(q));
                    continue;
                }
                match grupos.iter_mut().find(|(g, _)| *g == q.grupo) {
                    Some((_, itens)) => itens.push(item(q)),
                    None => grupos.push((q.grupo.clone(), vec![item(q)])),
                }
            }
            let children: Vec<FaqNode> = grupos
                .into_iter()
                .map(|(g, faq_items)| FaqNode {
                    origin: format!("{} | {g}", p.nome),
                    title: g,
                    url: p.url.clone(),
                    page_type: PageType::Faq,
                    children: Vec::new(),
                    faq_items,
                })
                .collect();
            FaqNode {
                title: p.nome.to_string(),
                url: p.url.clone(),
                page_type: if children.is_empty() {
                    PageType::Faq
                } else {
                    PageType::Menu
                },
                origin: p.nome.to_string(),
                children,
                faq_items: diretos,
            }
        })
        .collect();
    FaqNode {
        title: "Perguntas e Respostas".to_string(),
        url: INDICE.to_string(),
        page_type: PageType::Menu,
        origin: String::new(),
        children: filhos,
        faq_items: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // O artigo (`div.w3-card-12.internas`) das páginas reais de 07/11/2026: SINTEGRA (negrito numerado, e-mails
    // protegidos), Selo Fiscal (negrito sem número; a imagem em base64 saiu da fixture), Parcelamento (numeradas
    // "1.1 -" sem negrito), Denegação (a 8 em negrito sem "?"), Cruzamento EFD x NF-e (a 2 com o "?" no meio) e o
    // índice "Perguntas e Respostas" (só links).
    const SINTEGRA: &str = include_str!("../tests/fixtures/pa-faqs-sintegra.html");
    const SELO: &str = include_str!("../tests/fixtures/pa-faqs-selo.html");
    const PARCELAMENTO: &str = include_str!("../tests/fixtures/pa-faqs-parcelamento.html");
    const DENEGACAO: &str = include_str!("../tests/fixtures/pa-faqs-denegacao.html");
    const CRUZAMENTO: &str = include_str!("../tests/fixtures/pa-faqs-cruzamento.html");
    const INDICE_HTML: &str = include_str!("../tests/fixtures/pa-faqs-indice.html");

    fn erro<T>(r: Result<T>) -> String {
        match r {
            Ok(_) => panic!("esperava Err"),
            Err(e) => format!("{e:#}"),
        }
    }

    fn artigo(html: &str) -> String {
        format!(r#"<div class="w3-card-12 internas"><h1>Título</h1>{html}</div>"#)
    }

    #[test]
    fn negrito_numerado_com_email_protegido() {
        let ps = extrair(SINTEGRA).unwrap();
        assert_eq!(ps.len(), 31);
        assert!(
            ps[0].titulo.starts_with(
                "1 - Quais contribuintes são obrigados a fazer a declaração do Sintegra"
            )
        );
        let todas: String = ps.iter().map(|p| p.resposta.as_str()).collect();
        assert!(!todas.contains(CLOAK_TEXTO));
    }

    #[test]
    fn negrito_sem_numero() {
        let ps = extrair(SELO).unwrap();
        assert_eq!(ps.len(), 5);
        assert_eq!(
            ps[0].titulo,
            "O uso do Selo Fiscal de Controle e Qualidade da água é obrigatório?"
        );
    }

    #[test]
    fn numeradas_sem_negrito() {
        let ps = extrair(PARCELAMENTO).unwrap();
        assert_eq!(ps.len(), 29);
        assert!(
            ps[0]
                .titulo
                .starts_with("1.1 - Como faço para parcelar um débito tributário")
        );
    }

    #[test]
    fn negrito_que_continua_a_numeracao_sem_interrogacao_abre_pergunta() {
        let ps = extrair(DENEGACAO).unwrap();
        assert_eq!(ps.len(), 9);
        assert!(ps[7].titulo.starts_with("8. A CONSULTA À SITUAÇÃO ATUAL"));
        // Antes, a 8 ficava dentro da resposta da 7.
        assert!(!ps[6].resposta.contains("A CONSULTA À SITUAÇÃO"));
        let ps = extrair(CRUZAMENTO).unwrap();
        assert_eq!(ps.len(), 8);
        assert_eq!(
            ps[1].titulo,
            "2- Qualquer valor deve ser retificado? Mesmo que a diferença seja de centavos."
        );
    }

    #[test]
    fn prefixo_r_sai_da_resposta() {
        let ps = extrair(&artigo("<p><b>01) Quais?</b></p><p>R. Algumas.</p>")).unwrap();
        assert_eq!(ps[0].resposta, "Algumas.");
    }

    #[test]
    fn indice_so_de_links_pagina_sem_artigo_ou_sem_resposta_e_erro() {
        let e = erro(extrair(INDICE_HTML));
        assert!(e.contains("nenhuma pergunta"), "{e}");
        let e = erro(extrair("<html><body><p>Oi</p></body></html>"));
        assert!(e.contains("não tem o artigo"), "{e}");
        let e = erro(extrair(&artigo(
            "<p><b>1. Primeira?</b></p><p><b>2. Segunda?</b></p><p>R2.</p>",
        )));
        assert!(e.contains("\"1. Primeira?\" ficou sem resposta"), "{e}");
    }

    fn paginas() -> Vec<Pagina> {
        vec![
            Pagina {
                nome: "SINTEGRA",
                url: format!("{HOST}/119-orientacoes/sintegra/1439-duvidas-frequentes"),
                perguntas: extrair(SINTEGRA).unwrap(),
            },
            Pagina {
                nome: "Selo Fiscal de Controle e Qualidade",
                url: format!("{HOST}/38-orientacoes/18084-perguntas-e-respostas"),
                perguntas: extrair(SELO).unwrap(),
            },
        ]
    }

    #[test]
    fn trilha_e_link_da_pagina() {
        let faqs = consolidar(&paginas()).unwrap();
        assert_eq!(faqs.len(), 36);
        assert_eq!(faqs[0].origin, "SINTEGRA");
        assert_eq!(
            faqs[31].url,
            "https://antigo.sefa.pa.gov.br/38-orientacoes/18084-perguntas-e-respostas"
        );
    }

    #[test]
    fn pergunta_repetida_e_erro_e_piso() {
        let mut ps = paginas();
        let t = ps[1].perguntas[0].titulo.clone();
        ps[1].perguntas[1].titulo = t;
        assert!(erro(consolidar(&ps)).contains("aparece duas vezes"));
        assert!(piso(PISO).is_ok());
        assert!(erro(piso(PISO - 1)).contains("abaixo do piso"));
    }

    #[test]
    fn arvore_um_no_por_pagina() {
        let t = montar_arvore(&paginas());
        assert_eq!(
            (t.page_type, t.children.len(), t.url.as_str()),
            (PageType::Menu, 2, INDICE)
        );
        assert_eq!(
            (t.children[0].page_type, t.children[0].faq_items.len()),
            (PageType::Faq, 31)
        );
    }
}
