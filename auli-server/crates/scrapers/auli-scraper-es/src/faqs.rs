// SEFAZ-ES FAQs — o portal Receita Orienta (`s1-internet.sefaz.es.gov.br/ReceitaOrienta`), ASP.NET com o HTML
// servido pronto. Estudo, medições e decisões: docs/faqs/ESTUDO-ES-Faqs.md (D-ESFAQ-*).
//
// A fonte é a navegação (D-ESFAQ-1, -2). A barra de menu de uma página de categoria lista as categorias, cada
// categoria lista os seus assuntos, e cada assunto é UMA página com o acordeão inteiro. A pergunta fica direto no
// assunto (`heading{ID}_faq`) ou dentro de um subassunto (`heading{SID}_sa` → `heading{ID}_saf`). A resposta é o
// `.card-body.user-content` do `collapse` correspondente, e o anexo, quando há, fica no `.card-footer`. `{ID}` é o
// id da pergunta no banco, único no portal. A busca `BuscaV2` não entra: devolve também o que o portal tirou do ar.

use std::collections::{HashMap, HashSet};
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

const HOST: &str = "https://s1-internet.sefaz.es.gov.br";
const PORTAL: &str = "https://s1-internet.sefaz.es.gov.br/ReceitaOrienta/";
/// Base das páginas de categoria (`…/categoria/{id}`) e dos links de assunto, que a categoria escreve relativos
/// (`Assunto/{id}/{Categoria}/{catId}`).
const CATEGORIA: &str = "https://s1-internet.sefaz.es.gov.br/ReceitaOrienta/categoria/";
/// A categoria por onde a coleta começa (D-ESFAQ-7): ICMS. A barra de menu de qualquer categoria lista todas, e a
/// home, que também lista, tem 17,8 MB de imagens embutidas.
const SEMENTE: u32 = 61;
/// O que a página de um assunto despublicado diz ("Erro: Você não deveria estar aqui."). No HTML as letras
/// acentuadas vêm como entidades; este trecho não tem acento.
const DESPUBLICADO: &str = "deveria estar aqui";
/// Cortesia entre os GETs (D-ESFAQ-8).
const CORTESIA: Duration = Duration::from_secs(1);
/// Piso do acervo: ~90 % das 1.003 perguntas de 07/11/2026.
const PISO: usize = 900;

/// Coleta as FAQs e grava a árvore do frontend (`<id>-faqs-tree.json`) e o snapshot. Tudo é coletado e conferido
/// antes de qualquer gravação: rodada que falha deixa os arquivos anteriores intactos.
pub fn run(data_dir: &str, use_cache: bool) -> Result<()> {
    let agent = auli_scraper_kit::build_agent(crate::UA, Some(Duration::from_secs(60)));
    let pausa = || {
        if !use_cache {
            sleep(CORTESIA);
        }
    };
    let semente = buscar(
        &agent,
        data_dir,
        &format!("{CATEGORIA}{SEMENTE}"),
        use_cache,
    )?;
    let mut categorias = Vec::new();
    for (id, nome) in menu(&semente).context("ES FAQs: menu de categorias da semente")? {
        let html = if id == SEMENTE {
            semente.clone()
        } else {
            pausa();
            buscar(&agent, data_dir, &format!("{CATEGORIA}{id}"), use_cache)?
        };
        let links = assuntos_da_categoria(&html)
            .with_context(|| format!("ES FAQs: categoria {nome} ({id})"))?;
        let mut assuntos = Vec::new();
        for (nome_assunto, url) in links {
            pausa();
            let html = buscar(&agent, data_dir, &url, use_cache)?;
            let assunto = extrair(&html, &nome_assunto, &url)
                .with_context(|| format!("ES FAQs: assunto {nome_assunto} ({url})"))?;
            println!(
                "ES FAQs: {nome} | {nome_assunto}: {} perguntas",
                assunto.perguntas.len()
            );
            assuntos.push(assunto);
        }
        categorias.push(Categoria { id, nome, assuntos });
    }
    let faqs = consolidar(&categorias)?;
    piso(faqs.len())?;

    // Atômico pelo mesmo motivo das outras entidades: a árvore não tem outra fonte (o snapshot achatado não a
    // reconstrói) e é o único artefato de coleta que o frontend lê direto.
    let tree_path = format!("{data_dir}/{}-faqs-tree.json", crate::ENTITY);
    auli_contract::arquivo::escrever_atomico(
        Path::new(&tree_path),
        &serde_json::to_string_pretty(&montar_arvore(&categorias))?,
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

/// Rede sempre, gravando o cache; com `--usecache`, só o cache, e miss é erro (D-ESFAQ-8).
fn buscar(agent: &Agent, data_dir: &str, url: &str, use_cache: bool) -> Result<String> {
    if use_cache {
        return auli_scraper_kit::cache::read(data_dir, "faqs", url)
            .ok_or_else(|| anyhow!("cache miss para {url} (modo --usecache, sem rede)"));
    }
    let html = auli_scraper_kit::http::get_string(
        agent,
        url,
        &GetOpts {
            log_prefix: "ES FAQs",
            ..Default::default()
        },
    )?;
    auli_scraper_kit::cache::write(data_dir, "faqs", url, &html);
    Ok(html)
}

struct Categoria {
    id: u32,
    nome: String,
    assuntos: Vec<Assunto>,
}

struct Assunto {
    nome: String,
    url: String,
    /// Os subassuntos, na ordem da página.
    subs: Vec<Sub>,
    /// As perguntas, na ordem da página (as diretas e as dos subassuntos misturadas, como o acordeão as mostra).
    perguntas: Vec<Pergunta>,
}

struct Sub {
    id: u64,
    nome: String,
}

struct Pergunta {
    id: u64,
    titulo: String,
    resposta: String,
    /// Índice em `Assunto::subs`; `None` na pergunta direto no assunto.
    sub: Option<usize>,
    /// A página do assunto com a âncora do cabeçalho da pergunta (D-ESFAQ-4).
    link: String,
}

static MENU: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("#navbarNav a.nav-link[href]").unwrap());
static MENU_HREF: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^/ReceitaOrienta/categoria/(\d+)$").unwrap());
static CARD_ASSUNTO: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(".card-categoria a[href]").unwrap());
static ACORDEAO: LazyLock<Selector> = LazyLock::new(|| Selector::parse("#accordion").unwrap());
static CABECALHO: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(r#"div[id^="heading"]"#).unwrap());
static CABECALHO_ID: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^heading(\d+)_(faq|sa|saf)$").unwrap());
static COLLAPSE_SA: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^collapse(\d+)_sa$").unwrap());
static TITULO: LazyLock<Selector> = LazyLock::new(|| Selector::parse(".pPergunta").unwrap());
static CORPO: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(".card-body.user-content").unwrap());
static ANEXO: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(".card-footer a[href]").unwrap());

/// A barra de menu → `(id, nome)` das categorias, na ordem do menu. O nome é o texto do próprio link: o `<p> |</p>`
/// de separação, que vem dentro dele, fica de fora.
fn menu(html: &str) -> Result<Vec<(u32, String)>> {
    let doc = Html::parse_document(html);
    let mut vistos = HashSet::new();
    let mut categorias = Vec::new();
    for a in doc.select(&MENU) {
        let Some(c) = a
            .value()
            .attr("href")
            .and_then(|h| MENU_HREF.captures(h.trim()))
        else {
            continue;
        };
        let id: u32 = c[1].parse()?;
        let nome = clean(
            &a.children()
                .filter_map(|n| match n.value() {
                    Node::Text(t) => Some(&**t),
                    _ => None,
                })
                .collect::<String>(),
        );
        if nome.is_empty() {
            bail!("a categoria {id} do menu não tem nome");
        }
        if vistos.insert(id) {
            categorias.push((id, nome));
        }
    }
    if categorias.is_empty() {
        bail!("a barra de menu não lista categoria nenhuma — o layout mudou?");
    }
    Ok(categorias)
}

/// A página de uma categoria → `(nome, url)` dos assuntos, na ordem da página.
fn assuntos_da_categoria(html: &str) -> Result<Vec<(String, String)>> {
    let doc = Html::parse_document(html);
    let assuntos: Vec<(String, String)> = doc
        .select(&CARD_ASSUNTO)
        .filter_map(|a| {
            let href = a.value().attr("href")?.trim();
            href.starts_with("Assunto/").then(|| {
                (
                    clean(&a.text().collect::<String>()),
                    format!("{CATEGORIA}{href}"),
                )
            })
        })
        .collect();
    if assuntos.is_empty() {
        bail!("a categoria não lista assunto nenhum — o layout mudou?");
    }
    Ok(assuntos)
}

/// A página de um assunto → os subassuntos e as perguntas, na ordem do acordeão.
fn extrair(html: &str, nome: &str, url: &str) -> Result<Assunto> {
    if html.contains(DESPUBLICADO) {
        bail!("a página responde \"Você não deveria estar aqui\" — o assunto foi despublicado?");
    }
    let doc = Html::parse_document(html);
    let acordeao = doc
        .select(&ACORDEAO)
        .next()
        .ok_or_else(|| anyhow!("a página não tem o acordeão (#accordion) — o layout mudou?"))?;
    let mut subs: Vec<Sub> = Vec::new();
    let mut sub_por_id: HashMap<u64, usize> = HashMap::new();
    let mut perguntas = Vec::new();
    for cabecalho in acordeao.select(&CABECALHO) {
        let Some(c) = cabecalho
            .value()
            .id()
            .and_then(|i| CABECALHO_ID.captures(i))
        else {
            continue;
        };
        let id: u64 = c[1].parse()?;
        let tipo = c[2].to_string();
        let titulo = cabecalho
            .select(&TITULO)
            .next()
            .map(|t| clean(&t.text().collect::<String>()))
            .unwrap_or_default();
        if titulo.is_empty() {
            bail!("o cabeçalho heading{id}_{tipo} não tem título");
        }
        if tipo == "sa" {
            sub_por_id.insert(id, subs.len());
            subs.push(Sub { id, nome: titulo });
            continue;
        }
        let sub = if tipo == "saf" {
            let sid = subassunto_de(cabecalho).ok_or_else(|| {
                anyhow!("a pergunta {id} ({titulo:?}) está fora de um subassunto")
            })?;
            Some(*sub_por_id.get(&sid).ok_or_else(|| {
                anyhow!("a pergunta {id} ({titulo:?}) está num subassunto ({sid}) sem cabeçalho")
            })?)
        } else {
            None
        };
        let sel = Selector::parse(&format!("#collapse{id}_{tipo}"))
            .map_err(|e| anyhow!("seletor do collapse {id}: {e:?}"))?;
        let collapse = doc
            .select(&sel)
            .next()
            .ok_or_else(|| anyhow!("a pergunta {id} ({titulo:?}) não tem o bloco da resposta"))?;
        let corpo = collapse.select(&CORPO).next().ok_or_else(|| {
            anyhow!("a pergunta {id} ({titulo:?}) não tem o corpo da resposta (.user-content)")
        })?;
        let mut resposta = html_para_texto(&corpo.inner_html(), HOST);
        if resposta.is_empty() {
            bail!("a pergunta {id} ({titulo:?}) ficou sem resposta");
        }
        // O anexo fica fora do corpo, no rodapé do card; vira uma linha da resposta (D-ESFAQ-6).
        for a in collapse.select(&ANEXO) {
            if let Some(u) = a
                .value()
                .attr("href")
                .and_then(|h| auli_scraper_kit::html::endereco(h.trim(), Some(HOST)))
            {
                resposta.push_str(&format!("\nAnexo: {u}"));
            }
        }
        perguntas.push(Pergunta {
            id,
            titulo,
            resposta,
            sub,
            link: format!("{url}#heading{id}_{tipo}"),
        });
    }
    if perguntas.is_empty() {
        bail!("nenhuma pergunta no acordeão");
    }
    for (i, s) in subs.iter().enumerate() {
        if !perguntas.iter().any(|p| p.sub == Some(i)) {
            bail!("o subassunto {:?} ({}) não tem pergunta", s.nome, s.id);
        }
    }
    Ok(Assunto {
        nome: nome.to_string(),
        url: url.to_string(),
        subs,
        perguntas,
    })
}

/// O `{SID}` do `collapse{SID}_sa` que contém o cabeçalho de uma pergunta de subassunto.
fn subassunto_de(cabecalho: ElementRef) -> Option<u64> {
    cabecalho
        .ancestors()
        .filter_map(ElementRef::wrap)
        .filter_map(|e| e.value().id())
        .find_map(|i| COLLAPSE_SA.captures(i).and_then(|c| c[1].parse().ok()))
}

/// A categoria some da trilha e da árvore quando o único assunto dela tem o mesmo nome (IPVA, ITCMD, Nota Premiada
/// Capixaba, Reforma Tributária, Agendar Atendimento) — D-ESFAQ-3.
fn colapsa(c: &Categoria) -> bool {
    matches!(c.assuntos.as_slice(), [a] if a.nome == c.nome)
}

/// `Categoria | Assunto [| Subassunto]`, sem a categoria quando ela colapsa (D-ESFAQ-3).
fn trilha(c: &Categoria, a: &Assunto, sub: Option<usize>) -> String {
    let mut partes: Vec<&str> = Vec::new();
    if !colapsa(c) {
        partes.push(&c.nome);
    }
    partes.push(&a.nome);
    if let Some(i) = sub {
        partes.push(&a.subs[i].nome);
    }
    partes.join(" | ")
}

/// O acervo, com a guarda que olha todas as páginas: o id é único no portal, então um id repetido é o mesmo
/// assunto listado duas vezes (ou o acordeão mudou de forma) — os `.md` sairiam em dobro.
fn consolidar(categorias: &[Categoria]) -> Result<Vec<FaqRaw>> {
    let mut vistos = HashSet::new();
    let mut faqs = Vec::new();
    for c in categorias {
        for a in &c.assuntos {
            for p in &a.perguntas {
                if !vistos.insert(p.id) {
                    bail!(
                        "a pergunta {} ({:?}) aparece duas vezes (de novo em {})",
                        p.id,
                        p.titulo,
                        a.url
                    );
                }
                faqs.push(FaqRaw {
                    pergunta: p.titulo.clone(),
                    resposta: p.resposta.clone(),
                    origin: trilha(c, a, p.sub),
                    url: p.link.clone(),
                });
            }
        }
    }
    let assuntos: usize = categorias.iter().map(|c| c.assuntos.len()).sum();
    println!(
        "ES FAQs: {} perguntas em {assuntos} assuntos de {} categorias",
        faqs.len(),
        categorias.len()
    );
    Ok(faqs)
}

fn piso(n: usize) -> Result<()> {
    if n < PISO {
        bail!("só {n} perguntas, abaixo do piso de {PISO} — a navegação encolheu?");
    }
    Ok(())
}

/// A árvore da aba de FAQs: raiz → categoria → assunto → subassunto, sem o nó da categoria quando ela colapsa.
/// Assunto com subassuntos é `Menu` com as perguntas diretas como itens (molde MA); sem, é `Faq`.
fn montar_arvore(categorias: &[Categoria]) -> FaqNode {
    let itens = |a: &Assunto, sub: Option<usize>| -> Vec<FaqItem> {
        a.perguntas
            .iter()
            .filter(|p| p.sub == sub)
            .map(|p| FaqItem {
                pergunta: p.titulo.clone(),
                resposta: p.resposta.clone(),
                url: Some(p.link.clone()),
            })
            .collect()
    };
    let no_assunto = |c: &Categoria, a: &Assunto| -> FaqNode {
        let subs: Vec<FaqNode> = a
            .subs
            .iter()
            .enumerate()
            .map(|(i, s)| FaqNode {
                title: s.nome.clone(),
                url: format!("{}#heading{}_sa", a.url, s.id),
                page_type: PageType::Faq,
                origin: trilha(c, a, Some(i)),
                children: Vec::new(),
                faq_items: itens(a, Some(i)),
            })
            .collect();
        FaqNode {
            title: a.nome.clone(),
            url: a.url.clone(),
            page_type: if subs.is_empty() {
                PageType::Faq
            } else {
                PageType::Menu
            },
            origin: trilha(c, a, None),
            children: subs,
            faq_items: itens(a, None),
        }
    };
    FaqNode {
        title: "Perguntas frequentes".to_string(),
        url: PORTAL.to_string(),
        page_type: PageType::Menu,
        origin: String::new(),
        children: categorias
            .iter()
            .map(|c| {
                if colapsa(c) {
                    no_assunto(c, &c.assuntos[0])
                } else {
                    FaqNode {
                        title: c.nome.clone(),
                        url: format!("{CATEGORIA}{}", c.id),
                        page_type: PageType::Menu,
                        origin: c.nome.clone(),
                        children: c.assuntos.iter().map(|a| no_assunto(c, a)).collect(),
                        faq_items: Vec::new(),
                    }
                }
            })
            .collect(),
        faq_items: Vec::new(),
    }
}

/// HTML → texto pelo conversor da frota (`auli_scraper_kit::html`, D-KITHTML-1): links da raiz com o host do
/// Receita Orienta.
fn html_para_texto(s: &str, base: &str) -> String {
    auli_scraper_kit::html::html_para_texto(
        s,
        auli_scraper_kit::html::Opcoes {
            base: Some(base),
            quebras_do_fonte: false,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // Páginas reais de 07/11/2026, sem o `<head>` e os `<script>`: a categoria ICMS (a semente, 41 assuntos) e a
    // IPVA (assunto único, mesmo nome); os assuntos Cooperação Fiscal (perguntas diretas e subassuntos misturados),
    // AC- Aviso de Cobrança (títulos repetidos em subassuntos diferentes) e Isenção - aquisição de veículo novo
    // (anexos no rodapé); e a página de um assunto fora da navegação (CT-e).
    const CAT_ICMS: &str = include_str!("../tests/fixtures/es-faqs-cat61.html");
    const CAT_IPVA: &str = include_str!("../tests/fixtures/es-faqs-cat81.html");
    const COOPERACAO: &str = include_str!("../tests/fixtures/es-faqs-581.html");
    const AVISO: &str = include_str!("../tests/fixtures/es-faqs-641.html");
    const ISENCAO: &str = include_str!("../tests/fixtures/es-faqs-141.html");
    const DESPUBLICADA: &str = include_str!("../tests/fixtures/es-faqs-921.html");
    const URL_COOPERACAO: &str =
        "https://s1-internet.sefaz.es.gov.br/ReceitaOrienta/categoria/Assunto/581/ICMS/61";
    const URL_AVISO: &str =
        "https://s1-internet.sefaz.es.gov.br/ReceitaOrienta/categoria/Assunto/641/ICMS/61";
    const URL_ISENCAO: &str =
        "https://s1-internet.sefaz.es.gov.br/ReceitaOrienta/categoria/Assunto/141/ICMS/61";

    fn erro<T>(r: Result<T>) -> String {
        match r {
            Ok(_) => panic!("esperava Err"),
            Err(e) => format!("{e:#}"),
        }
    }

    fn assunto(html: &str, nome: &str, url: &str) -> Assunto {
        extrair(html, nome, url).unwrap()
    }

    /// Um acordeão mínimo no formato do portal: `itens` são `(id, tipo, título, corpo)`; os `saf` vão dentro do
    /// último `sa`.
    fn pagina(itens: &[(u64, &str, &str, &str)]) -> String {
        let mut s = String::from(r#"<html><body><div id="accordion">"#);
        let mut aberto = false;
        let fecha = |s: &mut String, aberto: &mut bool| {
            if *aberto {
                s.push_str("</div></div></div>");
                *aberto = false;
            }
        };
        for (id, tipo, titulo, corpo) in itens {
            match *tipo {
                "sa" => {
                    fecha(&mut s, &mut aberto);
                    s.push_str(&format!(
                        r#"<div><div id=heading{id}_sa><div class="pPergunta">{titulo}</div></div><div id=collapse{id}_sa class="collapse"><div id="accordion_faq">"#
                    ));
                    aberto = true;
                }
                "faq" => {
                    fecha(&mut s, &mut aberto);
                    s.push_str(&format!(
                        r#"<div><div id=heading{id}_faq><div class="pPergunta">{titulo}</div></div><div id=collapse{id}_faq class="collapse"><div class="card"><div class="card-body user-content">{corpo}</div></div></div></div>"#
                    ));
                }
                _ => s.push_str(&format!(
                    r#"<div><div id=heading{id}_saf><div class="pPergunta">{titulo}</div></div><div id=collapse{id}_saf class="collapse"><div class="card"><div class="card-body user-content">{corpo}</div></div></div></div>"#
                )),
            }
        }
        fecha(&mut s, &mut aberto);
        s.push_str("</div></body></html>");
        s
    }

    #[test]
    fn menu_lista_as_seis_categorias_na_ordem_e_sem_o_separador() {
        let m = menu(CAT_ICMS).unwrap();
        let esperado: Vec<(u32, String)> = [
            (82, "Agendar Atendimento"),
            (61, "ICMS"),
            (81, "IPVA"),
            (62, "ITCMD"),
            (83, "Nota Premiada Capixaba"),
            (63, "Reforma Tributária"),
        ]
        .into_iter()
        .map(|(i, n)| (i, n.to_string()))
        .collect();
        assert_eq!(m, esperado);
    }

    #[test]
    fn menu_vazio_e_erro() {
        let e = erro(menu("<html><body><p>Manutenção</p></body></html>"));
        assert!(e.contains("não lista categoria nenhuma"), "{e}");
    }

    #[test]
    fn categoria_lista_os_assuntos_com_url_absoluta() {
        let icms = assuntos_da_categoria(CAT_ICMS).unwrap();
        assert_eq!(icms.len(), 41);
        assert_eq!(
            icms[0],
            ("AC- Aviso de Cobrança".to_string(), URL_AVISO.to_string())
        );
        let ipva = assuntos_da_categoria(CAT_IPVA).unwrap();
        assert_eq!(
            ipva,
            vec![(
                "IPVA".to_string(),
                "https://s1-internet.sefaz.es.gov.br/ReceitaOrienta/categoria/Assunto/9/IPVA/81"
                    .to_string()
            )]
        );
    }

    #[test]
    fn categoria_sem_assunto_e_erro() {
        let e = erro(assuntos_da_categoria("<html><body></body></html>"));
        assert!(e.contains("não lista assunto nenhum"), "{e}");
    }

    #[test]
    fn perguntas_diretas_e_de_subassunto_na_ordem_da_pagina() {
        let a = assunto(COOPERACAO, "Cooperação Fiscal", URL_COOPERACAO);
        assert_eq!(a.perguntas.len(), 23);
        assert_eq!(
            a.subs.iter().map(|s| s.nome.as_str()).collect::<Vec<_>>(),
            [
                "04. O que são as inconsistências ?",
                "08. O que significa \"status da inconsistência\"?"
            ]
        );
        assert_eq!(a.perguntas.iter().filter(|p| p.sub.is_none()).count(), 13);
        let p0 = &a.perguntas[0];
        assert_eq!(
            (p0.titulo.as_str(), p0.sub, p0.link.as_str()),
            (
                "01. O que é Cooperação Fiscal?",
                None,
                &*format!("{URL_COOPERACAO}#heading3932_faq")
            )
        );
        // A 4ª é a 1ª do subassunto "04.", e a seguinte ao subassunto volta a ser direta.
        let p3 = &a.perguntas[3];
        assert_eq!((p3.id, p3.sub), (6380, Some(0)));
        assert_eq!(p3.link, format!("{URL_COOPERACAO}#heading6380_saf"));
        assert_eq!((a.perguntas[9].id, a.perguntas[9].sub), (3935, None));
    }

    #[test]
    fn titulo_repetido_em_subassuntos_tem_links_distintos() {
        let a = assunto(AVISO, "AC- Aviso de Cobrança", URL_AVISO);
        assert_eq!((a.perguntas.len(), a.subs.len()), (26, 2));
        let iguais: Vec<&Pergunta> = a
            .perguntas
            .iter()
            .filter(|p| {
                p.titulo == "01. O que é um crédito tributário de natureza não contenciosa?"
            })
            .collect();
        assert_eq!(iguais.len(), 2);
        assert_ne!(iguais[0].link, iguais[1].link);
        assert_eq!((iguais[0].sub, iguais[1].sub), (Some(0), Some(1)));
    }

    #[test]
    fn anexo_do_rodape_vira_linha_da_resposta() {
        let a = assunto(ISENCAO, "Isenção - aquisição de veículo novo", URL_ISENCAO);
        let com_anexo: Vec<u64> = a
            .perguntas
            .iter()
            .filter(|p| p.resposta.contains("\nAnexo: "))
            .map(|p| p.id)
            .collect();
        assert_eq!(com_anexo, [8256, 8258, 8272, 8273, 8274]);
        let p = a.perguntas.iter().find(|p| p.id == 8273).unwrap();
        assert_eq!(p.titulo, "h.3) Laudo para pessoa com Síndrome de Down");
        assert!(
            p.resposta.ends_with(
                "\nAnexo: https://s1-internet.sefaz.es.gov.br/ReceitaOrienta/Categoria/DownloadAnexo/8273"
            ),
            "{}",
            p.resposta
        );
    }

    #[test]
    fn resposta_em_linhas_com_links_e_a_data_de_atualizacao() {
        for (html, nome, url) in [
            (COOPERACAO, "Cooperação Fiscal", URL_COOPERACAO),
            (AVISO, "AC- Aviso de Cobrança", URL_AVISO),
            (ISENCAO, "Isenção - aquisição de veículo novo", URL_ISENCAO),
        ] {
            for p in assunto(html, nome, url).perguntas {
                let t = format!("{}\n{}", p.titulo, p.resposta);
                assert!(
                    !t.contains('<') && !t.contains("&nbsp;") && !t.contains("&#x"),
                    "{t}"
                );
                assert!(!t.contains('\u{a0}') && !t.contains('\u{200b}'), "{t}");
            }
        }
        // A data fica, na sua linha: é a única que a resposta mostra.
        let a = assunto(ISENCAO, "Isenção - aquisição de veículo novo", URL_ISENCAO);
        let p = a.perguntas.iter().find(|p| p.id == 8253).unwrap();
        assert_eq!(p.titulo, "a) Quem pode requerer a isenção?");
        assert!(
            p.resposta.lines().any(|l| l.starts_with("Atualizado em ")),
            "{}",
            p.resposta
        );
        // O `<hr>` entre os parágrafos vira quebra: a resposta não fica numa linha só.
        assert!(p.resposta.lines().count() > 1, "{}", p.resposta);
    }

    #[test]
    fn link_cujo_texto_e_a_url_sai_uma_vez_e_o_relativo_ganha_o_host() {
        let html = r#"<div>Acesse <a target="_blank" target='_blank' href="http://app.sefaz.es.gov.br/agv/">http://app.sefaz.es.gov.br/agv/</a>.</div><hr><div>Veja a <a href="/ReceitaOrienta/Aviso">página de avisos</a>.</div><div>Mais <a href="javascript:void(0)">aqui</a>.</div>"#;
        assert_eq!(
            html_para_texto(html, HOST),
            "Acesse \"http://app.sefaz.es.gov.br/agv/\".\nVeja a página de avisos \"https://s1-internet.sefaz.es.gov.br/ReceitaOrienta/Aviso\".\nMais aqui."
        );
    }

    #[test]
    fn assunto_despublicado_e_erro() {
        let e = erro(extrair(DESPUBLICADA, "CT-e", "x"));
        assert!(e.contains("despublicado"), "{e}");
    }

    #[test]
    fn pagina_sem_acordeao_ou_sem_pergunta_e_erro() {
        let e = erro(extrair("<html><body><p>Oi</p></body></html>", "X", "x"));
        assert!(e.contains("não tem o acordeão"), "{e}");
        let e = erro(extrair(
            r#"<html><body><div id="accordion"></div></body></html>"#,
            "X",
            "x",
        ));
        assert!(e.contains("nenhuma pergunta"), "{e}");
    }

    #[test]
    fn resposta_vazia_e_erro() {
        let html = pagina(&[(1, "faq", "01. Oi?", "<div><br></div>")]);
        let e = erro(extrair(&html, "X", "x"));
        assert!(e.contains("ficou sem resposta"), "{e}");
    }

    #[test]
    fn subassunto_sem_pergunta_e_erro() {
        let html = pagina(&[(1, "faq", "01. Oi?", "Sim."), (2, "sa", "02. Vazio", "")]);
        let e = erro(extrair(&html, "X", "x"));
        assert!(e.contains("\"02. Vazio\" (2) não tem pergunta"), "{e}");
    }

    #[test]
    fn pergunta_de_subassunto_fora_dele_e_erro() {
        // Um `saf` direto no `#accordion`, sem `collapse…_sa` em volta.
        let html = pagina(&[(1, "faq", "01. Oi?", "Sim.")]).replace(
            "</div></body></html>",
            r#"<div id=heading5_saf><div class="pPergunta">a) Solta?</div></div><div id=collapse5_saf><div class="card-body user-content">Não.</div></div></div></body></html>"#,
        );
        let e = erro(extrair(&html, "X", "x"));
        assert!(e.contains("está fora de um subassunto"), "{e}");
    }

    #[test]
    fn pergunta_sem_bloco_de_resposta_e_erro() {
        let html = COOPERACAO.replacen("id=collapse3932_faq", "id=outro3932", 1);
        assert_ne!(html, COOPERACAO);
        let e = erro(extrair(&html, "Cooperação Fiscal", URL_COOPERACAO));
        assert!(
            e.contains("3932") && e.contains("não tem o bloco da resposta"),
            "{e}"
        );
    }

    fn categorias() -> Vec<Categoria> {
        vec![
            Categoria {
                id: 61,
                nome: "ICMS".into(),
                assuntos: vec![
                    assunto(AVISO, "AC- Aviso de Cobrança", URL_AVISO),
                    assunto(COOPERACAO, "Cooperação Fiscal", URL_COOPERACAO),
                ],
            },
            Categoria {
                id: 81,
                nome: "IPVA".into(),
                assuntos: vec![assunto(ISENCAO, "IPVA", URL_ISENCAO)],
            },
        ]
    }

    #[test]
    fn trilha_com_subassunto_e_sem_a_categoria_que_colapsa() {
        let faqs = consolidar(&categorias()).unwrap();
        assert_eq!(faqs.len(), 26 + 23 + 23);
        assert_eq!(
            faqs[0].origin,
            "ICMS | AC- Aviso de Cobrança | 1- Aviso de Cobrança \"ICMS declarado e não recolhido\""
        );
        assert_eq!(faqs[26].origin, "ICMS | Cooperação Fiscal");
        // A categoria IPVA tem um assunto só, de mesmo nome: a trilha começa no assunto.
        assert_eq!(
            faqs[49].origin,
            "IPVA | 01. Da isenção para pessoas com deficiência ou autismo"
        );
        assert_eq!(faqs[0].url, format!("{URL_AVISO}#heading4174_saf"));
        let urls: HashSet<&str> = faqs.iter().map(|f| f.url.as_str()).collect();
        assert_eq!(urls.len(), faqs.len());
    }

    #[test]
    fn categoria_de_varios_assuntos_nao_colapsa_mesmo_com_um_de_mesmo_nome() {
        let mut cs = categorias();
        cs[0].assuntos[1].nome = "ICMS".into();
        let faqs = consolidar(&cs).unwrap();
        assert_eq!(faqs[26].origin, "ICMS | ICMS");
    }

    #[test]
    fn id_repetido_e_erro() {
        let mut cs = categorias();
        cs[1].assuntos.push(assunto(AVISO, "AC de novo", URL_AVISO));
        let e = erro(consolidar(&cs));
        assert!(e.contains("aparece duas vezes"), "{e}");
    }

    #[test]
    fn abaixo_do_piso_e_erro() {
        assert!(piso(PISO).is_ok());
        let e = erro(piso(PISO - 1));
        assert!(e.contains("abaixo do piso"), "{e}");
    }

    #[test]
    fn arvore_categoria_assunto_subassunto() {
        let t = montar_arvore(&categorias());
        assert_eq!((t.page_type, t.children.len()), (PageType::Menu, 2));
        let icms = &t.children[0];
        assert_eq!(
            (icms.title.as_str(), icms.page_type, icms.children.len()),
            ("ICMS", PageType::Menu, 2)
        );
        let coop = &icms.children[1];
        assert_eq!(
            (coop.page_type, coop.faq_items.len(), coop.children.len()),
            (PageType::Menu, 13, 2)
        );
        let sub = &coop.children[0];
        assert_eq!(sub.url, format!("{URL_COOPERACAO}#heading982_sa"));
        assert_eq!(sub.faq_items.len(), 6);
        assert_eq!(
            sub.faq_items[0].url.as_deref(),
            Some(&*format!("{URL_COOPERACAO}#heading6380_saf"))
        );
        // A categoria IPVA colapsa: o nó é o próprio assunto.
        let ipva = &t.children[1];
        assert_eq!(
            (ipva.title.as_str(), ipva.url.as_str(), ipva.children.len()),
            ("IPVA", URL_ISENCAO, 4)
        );
    }
}
