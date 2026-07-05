// Coleta da SEFAZ-PE: menu global `#menu_servicos` (3 blocos de público × subgrupos opcionais).
// Portal SharePoint 2013 on-prem server-side — o HTML já vem pronto (ureq + `scraper`, sem
// headless). Fase 1 (D-PE1): só o menu — UMA requisição por rodada.

use std::thread::sleep;
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use scraper::{ElementRef, Html, Selector};
use ureq::Agent;

use auli_contract::Publico;
use auli_scraper_kit::PerPublicoServicos;
use auli_scraper_kit::http::GetOpts;
use auli_contract::ServicoPerPublico as Servico;

const BASE: &str = "https://www.sefaz.pe.gov.br";
// A home renderiza o menu global completo (verificado também em páginas internas — o menu vem da
// masterpage `master2013`, presente em todas).
const SEED_URL: &str = "https://www.sefaz.pe.gov.br/";
// D-PE4: UA de navegador (o `kit::USER_AGENT` da frota — o robots.txt do portal é restritivo a
// crawlers genéricos; a coleta é de baixíssima frequência e volume mínimo — 1 GET por rodada na
// fase 1).
// Cortesia entre fetches de rede (irrelevante na fase 1 com 1 GET; vale para uma futura fase 2).
const COURTESY: Duration = Duration::from_millis(400);
/// D-PE2: itens de topo (sem subgrupo) recebem esta classe; itens sob subgrupo recebem o texto do
/// header do subgrupo (ex.: "Tributos").
const CLASSE_GERAL: &str = "Geral";
const ORGAO: &str = "SEFAZ-PE";

/// Os 3 públicos do menu, na ordem de exibição.
/// `(título no portal, nome canônico do público, slug do arquivo per-público)`.
fn publicos() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("Para cidadãos", "Cidadãos", "servicos-ao-cidadao"),
        ("Para empresas", "Empresas", "servicos-a-empresas"),
        ("Para municípios", "Municípios", "servicos-a-municipios"),
    ]
}

/// Um item do menu: um serviço listado sob um bloco (público) e um subgrupo opcional (classe).
struct MenuItem {
    titulo: String,
    link: String,
    classe: String,
}

/// Raspa os serviços do PE e devolve os per-público (na ordem do menu) + a ordem dos públicos.
pub fn scrape(data_dir: &str, use_cache: bool) -> Result<(PerPublicoServicos, Vec<Publico>)> {
    let agent = auli_scraper_kit::build_agent(auli_scraper_kit::USER_AGENT, Some(Duration::from_secs(30)));

    // 1. Seed: a home, com o menu global.
    let seed = fetch(&agent, data_dir, SEED_URL, use_cache)?;
    let doc = Html::parse_document(&seed);

    // 2. Parse do menu: 3 públicos -> itens (titulo, link, classe).
    let blocks = parse_menu(&doc)?;

    // 3. Casa os blocos com os públicos esperados (pelo título do portal). Bloco esperado ausente é
    //    erro duro: o catálogo sairia sem um público inteiro.
    let pubs = publicos();
    let mut inputs: PerPublicoServicos = Vec::new();
    for (titulo_portal, nome, _) in &pubs {
        let Some((_, items)) = blocks.iter().find(|(t, _)| t == titulo_portal) else {
            bail!("bloco '{}' ausente em #menu_servicos — layout mudou?", titulo_portal);
        };
        println!("PE: bloco '{}' -> {} ocorrências", nome, items.len());
        if items.is_empty() {
            eprintln!("⚠️  PE: bloco '{}' veio vazio — estrutura do menu mudou?", nome);
        }
        let servicos = items
            .iter()
            .map(|it| {
                // Header de 3 linhas `tipo/classe/titulo` que o aggregate_servicos do kit remove;
                // fase 1 não coleta corpo (descricao final vazia).
                let descricao = format!("{}\n{}\n{}\n", nome, it.classe, it.titulo);
                Servico {
                    id: 0,
                    tipo: nome.to_string(),
                    classe: it.classe.clone(),
                    orgao: ORGAO.to_string(),
                    link: it.link.clone(),
                    titulo: it.titulo.clone(),
                    descricao,
                }
            })
            .collect();
        inputs.push((nome.to_string(), servicos));
    }

    let publicos_ordem = pubs
        .iter()
        .map(|(_, nome, slug)| Publico { nome: nome.to_string(), slug: slug.to_string() })
        .collect();
    Ok((inputs, publicos_ordem))
}

/// Extrai os blocos de `#menu_servicos`: cada `div.submenu_block` vira `(título do bloco, itens)`.
/// Dentro do bloco, itera as `<ul>` de topo de `div.submenu_itens`; um `<li>` com `<ul>` aninhada é
/// um subgrupo (header vira classe dos filhos; header com href real também vira item — D-PE3, caso
/// "Tributos Transferências Constitucionais" em municípios, que é uma página real).
fn parse_menu(doc: &Html) -> Result<Vec<(String, Vec<MenuItem>)>> {
    let menu = doc
        .select(&sel("#menu_servicos"))
        .next()
        .ok_or_else(|| anyhow!("menu '#menu_servicos' ausente no seed {} — layout mudou?", SEED_URL))?;

    let mut out = Vec::new();
    for block in menu.select(&sel("div.submenu_block")) {
        let titulo_bloco = block
            .select(&sel("div.submenu_title"))
            .next()
            .map(|t| text(&t))
            .unwrap_or_default();
        let mut items = Vec::new();
        // `>` garante só as ULs de topo (as aninhadas são filhas de <li>).
        for ul in block.select(&sel("div.submenu_itens > ul")) {
            collect_ul(&ul, &mut items);
        }
        out.push((titulo_bloco, items));
    }
    Ok(out)
}

/// Coleta os itens de uma `<ul>` de topo: cada `<li>` direto tem um `<a>` header e, opcionalmente,
/// uma `<ul>` aninhada (subgrupo).
fn collect_ul(ul: &ElementRef, out: &mut Vec<MenuItem>) {
    for li in direct_children(ul, "li") {
        let header = direct_children(&li, "a").into_iter().next();
        let nested = direct_children(&li, "ul").into_iter().next();
        let (header_titulo, header_link) = match &header {
            Some(a) => (text(a), a.value().attr("href").and_then(canonical)),
            None => (String::new(), None),
        };

        // O header vira item quando aponta para uma página real (top-level ou subgrupo D-PE3).
        if let Some(link) = header_link {
            if !header_titulo.is_empty() {
                out.push(MenuItem {
                    titulo: header_titulo.clone(),
                    link,
                    classe: CLASSE_GERAL.to_string(),
                });
            }
        }

        // Filhos do subgrupo: classe = texto do header.
        if let Some(nested_ul) = nested {
            for a in nested_ul.select(&sel("a")) {
                let Some(href) = a.value().attr("href") else { continue };
                let Some(link) = canonical(href) else { continue };
                let titulo = text(&a);
                if titulo.is_empty() {
                    continue;
                }
                out.push(MenuItem { titulo, link, classe: header_titulo.clone() });
            }
        }
    }
}

/// Filhos-elemento diretos de `el` com a tag `name` (o crate `scraper` não suporta `:scope`).
fn direct_children<'a>(el: &ElementRef<'a>, name: &str) -> Vec<ElementRef<'a>> {
    el.children()
        .filter_map(ElementRef::wrap)
        .filter(|e| e.value().name() == name)
        .collect()
}

/// URL canônica de um item do menu: absolutiza relativos contra o host da SEFAZ-PE, preserva
/// absolutos (inclusive externos: efisco, gnre — são serviços válidos), remove fragmento; descarta
/// `javascript:`/`#`/vazio (headers de subgrupo sem página própria).
fn canonical(href: &str) -> Option<String> {
    let h = href.split('#').next().unwrap_or(href).trim();
    if h.is_empty() || h.starts_with("javascript:") {
        return None;
    }
    if h.starts_with("http://") || h.starts_with("https://") {
        return Some(h.to_string());
    }
    if let Some(stripped) = h.strip_prefix('/') {
        return Some(format!("{}/{}", BASE, stripped));
    }
    Some(format!("{}/{}", BASE, h))
}

/// Texto de um elemento com whitespace colapsado (o HTML do portal tem dezenas de `\n` dentro dos
/// `<a>`) e entidades comuns decodificadas pelo próprio parser. O squeeze é o `kit::clean`.
fn text(el: &ElementRef) -> String {
    auli_scraper_kit::clean(&el.text().collect::<String>())
}

fn sel(s: &str) -> Selector {
    Selector::parse(s).expect("seletor CSS inválido")
}

/// Busca (ou lê do cache) a página `url`. Em `--usecache` um miss é erro (sem rede). O retry/backoff
/// é o `kit::http::get_string`; o wrapper mantém o cache-write e a cortesia entre fetches de rede.
fn fetch(agent: &Agent, data_dir: &str, url: &str, use_cache: bool) -> Result<String> {
    if let Some(cached) = auli_scraper_kit::cache::read_or_bail(data_dir, url, use_cache)? {
        return Ok(cached);
    }
    let body = auli_scraper_kit::http::get_string(
        agent,
        url,
        &GetOpts { log_prefix: "PE", ..Default::default() },
    )?;
    auli_scraper_kit::cache::write(data_dir, url, &body);
    sleep(COURTESY);
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fixture estrutural derivada do HTML real do portal (masterpage `master2013`, capturada em
    /// 2026-07-04 de uma página de notícia — o menu vem da masterpage e é idêntico em toda página).
    /// Substituir por uma captura integral da home quando o real-scrape rodar no desktop.
    const FIXTURE: &str = include_str!("../tests/fixtures/pe-menu.html");

    fn parsed() -> Vec<(String, Vec<MenuItem>)> {
        parse_menu(&Html::parse_document(FIXTURE)).unwrap()
    }

    #[test]
    fn parses_three_publicos_in_order_scoped_to_menu_servicos() {
        let blocks = parsed();
        // O decoy `#menu_pulicacoes` da fixture não pode vazar para cá.
        let titulos: Vec<&str> = blocks.iter().map(|(t, _)| t.as_str()).collect();
        assert_eq!(titulos, ["Para cidadãos", "Para empresas", "Para municípios"]);
    }

    #[test]
    fn cidadaos_has_top_items_and_tributos_subgroup() {
        let blocks = parsed();
        let (_, items) = &blocks[0];
        // 5 de topo + 4 sob "Tributos" (header do subgrupo é javascript:; e não vira item).
        assert_eq!(items.len(), 9);
        assert!(items.iter().all(|i| !i.titulo.is_empty() && i.link.starts_with("http")));
        let ipva = items.iter().find(|i| i.titulo.starts_with("IPVA")).unwrap();
        // Whitespace interno do <a> colapsado.
        assert_eq!(ipva.titulo, "IPVA - e demais serviços para Veículos");
        assert_eq!(ipva.classe, "Tributos");
        // Relativo absolutizado.
        assert_eq!(ipva.link, "https://www.sefaz.pe.gov.br/Servicos/IPVA");
        // Header javascript:; do subgrupo não vira item.
        assert!(!items.iter().any(|i| i.titulo == "Tributos"));
    }

    #[test]
    fn empresas_spans_both_column_uls() {
        let blocks = parsed();
        let (_, items) = &blocks[1];
        // 12 na primeira coluna + 13 na segunda (duas <ul class="col45"> no mesmo bloco).
        assert_eq!(items.len(), 25);
        assert!(items.iter().any(|i| i.titulo == "SEF I"));
        assert!(items.iter().any(|i| i.titulo == "Programa de Conformidade e Autorregularização - Coopera"));
        // URL com percent-encoding preservada como está no href.
        let lib = items.iter().find(|i| i.titulo == "Liberação de Mercadoria").unwrap();
        assert_eq!(
            lib.link,
            "https://www.sefaz.pe.gov.br/Servicos/Paginas/Libera%C3%A7%C3%A3o-de-Mercadorias.aspx"
        );
    }

    #[test]
    fn municipios_subgroup_header_with_real_href_becomes_item_too() {
        let blocks = parsed();
        let (_, items) = &blocks[2];
        // 4 de topo + header do subgrupo (href real, D-PE3) + 3 filhos + 1 de topo = 9.
        assert_eq!(items.len(), 9);
        let header = items
            .iter()
            .find(|i| i.titulo == "Tributos Transferências Constitucionais")
            .expect("header do subgrupo com href real deve virar item");
        assert_eq!(header.classe, CLASSE_GERAL);
        let icms_ipi = items.iter().find(|i| i.titulo == "ICMS e IPI").unwrap();
        assert_eq!(icms_ipi.classe, "Tributos Transferências Constitucionais");
        // Filho com link externo absoluto preservado.
        let ipva = items.iter().find(|i| i.titulo == "IPVA").unwrap();
        assert_eq!(ipva.link, "https://arevirtualws.sefaz.pe.gov.br/sfar/sfar_municipio_periodo.php");
    }

    #[test]
    fn efisco_and_dae10_dedupe_into_multi_ocorrencias_via_kit_aggregate() {
        let blocks = parsed();
        let inputs: PerPublicoServicos = blocks
            .into_iter()
            .map(|(titulo_portal, items)| {
                let nome = publicos()
                    .into_iter()
                    .find(|(t, _, _)| *t == titulo_portal)
                    .map(|(_, n, _)| n.to_string())
                    .unwrap();
                let servicos = items
                    .into_iter()
                    .map(|it| Servico {
                        id: 0,
                        tipo: nome.clone(),
                        classe: it.classe.clone(),
                        orgao: ORGAO.to_string(),
                        link: it.link,
                        titulo: it.titulo,
                        descricao: format!("{}\n{}\n{}\n", nome, it.classe, "t"),
                    })
                    .collect();
                (nome, servicos)
            })
            .collect();

        let out = auli_scraper_kit::aggregate_servicos(&inputs);

        let efisco = out.iter().find(|s| s.link == "https://efisco.sefaz.pe.gov.br/").unwrap();
        let pubs: Vec<&str> = efisco.ocorrencias.iter().map(|o| o.publico.as_str()).collect();
        assert_eq!(pubs, ["Cidadãos", "Empresas", "Municípios"]);

        let dae10 = out
            .iter()
            .find(|s| s.link == "https://www.sefaz.pe.gov.br/Servicos/Paginas/DAE-10.aspx")
            .unwrap();
        assert_eq!(dae10.ocorrencias.len(), 2);

        // Fase 1: sem corpo — descricao final vazia (header de 3 linhas removido pelo kit).
        assert!(out.iter().all(|s| s.descricao.is_empty()));
    }

    #[test]
    fn canonical_rules() {
        assert_eq!(canonical("javascript:;"), None);
        assert_eq!(canonical("#topo"), None);
        assert_eq!(canonical(""), None);
        assert_eq!(
            canonical("/Servicos/ICMS#secao").as_deref(),
            Some("https://www.sefaz.pe.gov.br/Servicos/ICMS")
        );
        assert_eq!(
            canonical("https://www.gnre.pe.gov.br:444/gnre/portal/GNRE_Principal.jsp").as_deref(),
            Some("https://www.gnre.pe.gov.br:444/gnre/portal/GNRE_Principal.jsp")
        );
    }
}
