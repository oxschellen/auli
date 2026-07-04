//! Coleta do catálogo "Nossos Serviços" da SEFAZ-RJ.
//!
//! A fonte é UMA página WordPress server-rendered (D-RJ1): um menu com âncoras dá a ordem e os
//! nomes (em caixa mista) das ~14 categorias temáticas; cada âncora aponta para a seção com o
//! `<ul>` de links do serviço. Não há descrições (D-RJ3) nem eixo de público — modelamos um
//! público único "Serviços" (D-RJ4) com `classe` = categoria.
//!
//! O link NÃO é único no catálogo (CISC 2×, DARJ/ITD na mesma URL…): a identidade é
//! **`(link, titulo)`** (D-RJ2) e o mesmo par em mais de uma categoria vira UM `ServicoRaw` com
//! uma `Ocorrencia` por categoria. Por isso montamos `ServicoRaw` direto (padrão SP), sem o
//! `aggregate_servicos` do kit.
//!
//! Guards D-RJ5: a raspagem falha alto (nunca degrada) se vier página capada — na descoberta o
//! portal devolveu 1× a página vazia. Pela mesma razão, o cache só é gravado DEPOIS dos guards.

use std::collections::{HashMap, HashSet};
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use auli_contract::{Ocorrencia, Publico, ServicoRaw};
use scraper::{ElementRef, Html, Selector};

/// A página única do catálogo (D-RJ1).
pub const CATALOGO_URL: &str = "https://portal2.fazenda.rj.gov.br/nossos-servicos/";
/// Host para absolutizar links relativos.
const BASE: &str = "https://portal2.fazenda.rj.gov.br";
const USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64; rv:124.0) Gecko/20100101 Firefox/124.0";

/// O público único do RJ (D-RJ4).
const PUBLICO_NOME: &str = "Serviços";
const PUBLICO_SLUG: &str = "nossos-servicos";

/// Guards D-RJ5 — mínimos observados na descoberta (14 categorias, ~90 ocorrências) com folga
/// para flutuação editorial, mas apertados o bastante para rejeitar página capada.
const MIN_CATEGORIAS_COM_ITENS: usize = 12;
const MIN_OCORRENCIAS: usize = 60;

/// Resultado do parse da página, antes dos guards. Separado de [`scrape`] para os testes
/// exercitarem o parser com fixtures (que são menores que os mínimos dos guards).
struct Catalogo {
    /// Um `ServicoRaw` por `(link, titulo)`, na ordem de descoberta.
    items: Vec<ServicoRaw>,
    /// `(nome, nº de itens)` por categoria, na ordem do menu.
    categorias: Vec<(String, usize)>,
    /// Links descartados pela regra de exclusão (href não-http após absolutização).
    excluidos: usize,
}

/// Raspa o catálogo e devolve `(items, publicos_ordem)` prontos para o snapshot v3.
pub fn scrape(
    data_dir: &str,
    use_cache: bool,
) -> Result<(Vec<ServicoRaw>, Vec<Publico>), Box<dyn std::error::Error>> {
    let (html, veio_do_cache) = fetch_catalogo(data_dir, use_cache)?;

    let catalogo = parse_catalogo(&html)?;
    for (nome, n) in &catalogo.categorias {
        println!("RJ: {} — {} itens", nome, n);
    }
    if catalogo.excluidos > 0 {
        eprintln!(
            "⚠️  RJ: {} link(s) descartado(s) pela regra de exclusão (href não-http).",
            catalogo.excluidos
        );
    }

    validar(&catalogo)?;

    // Cache só DEPOIS dos guards (D-RJ5): uma página capada nunca envenena o cache. (No hit de
    // cache os guards rodam de novo de graça — se um cache antigo reprovar, o erro manda limpar.)
    if !veio_do_cache {
        auli_scraper_kit::cache::write(data_dir, CATALOGO_URL, &html);
    }

    let ocorrencias: usize = catalogo.items.iter().map(|s| s.ocorrencias.len()).sum();
    println!(
        "RJ: {} serviços ({} ocorrências) em {} categorias",
        catalogo.items.len(),
        ocorrencias,
        catalogo.categorias.len()
    );

    let publicos_ordem =
        vec![Publico { nome: PUBLICO_NOME.to_string(), slug: PUBLICO_SLUG.to_string() }];
    Ok((catalogo.items, publicos_ordem))
}

/// Busca (ou lê do cache) a página do catálogo. Diferente do padrão dos outros scrapers, o fetch
/// NÃO grava o cache — quem grava é o [`scrape`], depois dos guards (D-RJ5). Retorna
/// `(html, veio_do_cache)`.
fn fetch_catalogo(data_dir: &str, use_cache: bool) -> Result<(String, bool)> {
    if let Some(cached) = auli_scraper_kit::cache::read(data_dir, CATALOGO_URL) {
        println!("Cache hit (catálogo): {}", CATALOGO_URL);
        return Ok((cached, true));
    }
    if use_cache {
        bail!("cache miss para {} (modo --usecache, sem rede)", CATALOGO_URL);
    }

    let agent = auli_scraper_kit::build_agent(USER_AGENT, Some(Duration::from_secs(30)));

    println!("Fetching: {}", CATALOGO_URL);
    let max_attempts = 3;
    let mut delay = Duration::from_millis(800);
    let mut last = anyhow!("sem tentativa");
    for attempt in 1..=max_attempts {
        match agent.get(CATALOGO_URL).call() {
            Ok(mut resp) => match resp.body_mut().read_to_string() {
                Ok(body) if !body.trim().is_empty() => return Ok((body, false)),
                Ok(_) => last = anyhow!("resposta vazia"),
                Err(e) => last = anyhow!(e.to_string()),
            },
            Err(e) => last = anyhow!(e.to_string()),
        }
        if attempt < max_attempts {
            eprintln!("⚠️  RJ: tentativa {} falhou ({}); retentando…", attempt, last);
            sleep(delay);
            delay *= 2;
        }
    }
    Err(anyhow!("falha ao buscar {} após {} tentativas: {}", CATALOGO_URL, max_attempts, last))
}

/// Faz o parse da página inteira: menu → categorias na ordem; seção de cada categoria → itens;
/// dobra por `(link, titulo)` acumulando ocorrências (D-RJ2). Falha se uma âncora do menu não
/// tiver seção correspondente (guard de estrutura, parte do D-RJ5).
fn parse_catalogo(html: &str) -> Result<Catalogo> {
    let doc = Html::parse_document(html);

    // Todos os elementos com id, para validar âncoras e localizar seções (primeiro id vence).
    let sel_com_id = Selector::parse("[id]").unwrap();
    let mut por_id: HashMap<String, ElementRef> = HashMap::new();
    for el in doc.select(&sel_com_id) {
        if let Some(id) = el.value().attr("id") {
            por_id.entry(id.to_string()).or_insert(el);
        }
    }

    let menu = extrair_menu(&doc, &por_id)?;
    let ids_menu: HashSet<String> = menu.iter().map(|(id, _)| id.clone()).collect();

    let mut items: Vec<ServicoRaw> = Vec::new();
    let mut indice: HashMap<(String, String), usize> = HashMap::new();
    let mut categorias: Vec<(String, usize)> = Vec::new();
    let mut excluidos = 0usize;

    for (id, nome) in &menu {
        let alvo = por_id.get(id).ok_or_else(|| {
            anyhow!(
                "âncora '#{}' do menu ('{}') sem seção correspondente — markup mudou?",
                id,
                nome
            )
        })?;

        let mut n_itens = 0usize;
        for (titulo, href) in links_da_categoria(*alvo, &ids_menu) {
            let Some(link) = canonical(&href) else {
                excluidos += 1;
                continue;
            };
            let titulo = clean(&titulo);
            if titulo.is_empty() {
                excluidos += 1;
                continue;
            }
            n_itens += 1;

            let chave = (link.clone(), titulo.clone());
            let idx = *indice.entry(chave).or_insert_with(|| {
                items.push(ServicoRaw {
                    titulo,
                    descricao: String::new(), // D-RJ3: a página não tem corpo; v1 sem descrição.
                    link,
                    orgao: "SEFAZ-RJ".to_string(),
                    ocorrencias: Vec::new(),
                });
                items.len() - 1
            });
            let oc = &mut items[idx].ocorrencias;
            if !oc.iter().any(|o| o.classe == *nome) {
                oc.push(Ocorrencia { publico: PUBLICO_NOME.to_string(), classe: nome.clone() });
            }
        }
        categorias.push((nome.clone(), n_itens));
    }

    Ok(Catalogo { items, categorias, excluidos })
}

/// Extrai do documento a lista ordenada `(id_da_âncora, nome_em_caixa_mista)` das categorias.
///
/// Heurística sem dependência de classes CSS do tema: candidatos são todos os `<a href="#…">`
/// cujo alvo existe no documento; o menu é o MAIOR grupo de candidatos sob o mesmo contêiner
/// (`ul`/`ol`/`nav` ancestral — a página tem poucas âncoras fora dele, como "voltar ao topo").
fn extrair_menu(doc: &Html, por_id: &HashMap<String, ElementRef>) -> Result<Vec<(String, String)>> {
    let sel_ancora = Selector::parse(r##"a[href^="#"]"##).unwrap();

    // (chave do contêiner, id alvo, nome), em ordem de documento.
    let mut candidatos: Vec<(usize, String, String)> = Vec::new();
    let mut chaves: HashMap<_, usize> = HashMap::new();

    for a in doc.select(&sel_ancora) {
        let Some(href) = a.value().attr("href") else { continue };
        let alvo = href.trim_start_matches('#');
        if alvo.is_empty() || !por_id.contains_key(alvo) {
            continue;
        }
        let nome = clean(&a.text().collect::<String>());
        if nome.is_empty() {
            continue;
        }
        // Chave de agrupamento: o `ul`/`ol`/`nav` ancestral mais próximo (o `<a>` vive dentro de
        // um `<li>`, então o pai direto não agruparia nada). Fallback: o pai direto. O tipo
        // (`NodeId` do ego_tree) fica inferido para não depender do re-export do `scraper`.
        let node_chave = {
            let mut chave = a.parent().map(|p| p.id()).unwrap_or_else(|| a.id());
            for anc in a.ancestors() {
                if let Some(el) = ElementRef::wrap(anc) {
                    let tag = el.value().name();
                    if tag == "ul" || tag == "ol" || tag == "nav" {
                        chave = anc.id();
                        break;
                    }
                }
            }
            chave
        };
        let proximo = chaves.len();
        let chave = *chaves.entry(node_chave).or_insert(proximo);
        candidatos.push((chave, alvo.to_string(), nome));
    }

    if candidatos.is_empty() {
        bail!("menu de categorias não encontrado (nenhuma âncora interna válida) — markup mudou?");
    }

    // O maior grupo é o menu; empate resolve pela primeira aparição no documento.
    let mut contagem: HashMap<usize, usize> = HashMap::new();
    for (chave, ..) in &candidatos {
        *contagem.entry(*chave).or_insert(0) += 1;
    }
    let melhor = candidatos
        .iter()
        .map(|(chave, ..)| *chave)
        .max_by_key(|chave| (contagem[chave], usize::MAX - *chave))
        .unwrap();

    let mut vistos = HashSet::new();
    let menu: Vec<(String, String)> = candidatos
        .into_iter()
        .filter(|(chave, id, _)| *chave == melhor && vistos.insert(id.clone()))
        .map(|(_, id, nome)| (id, nome))
        .collect();

    Ok(menu)
}

/// Coleta os `(titulo, href)` da seção de uma categoria, dado o elemento-alvo da âncora.
///
/// Cobre os formatos usuais de âncora do WordPress:
/// 1. id num contêiner que envolve a lista → os links estão DENTRO do alvo;
/// 2. id no heading (ou num span/anchor vazio) → os links estão nos IRMÃOS seguintes, até
///    aparecer a próxima seção (um irmão que é/contém outro id do menu);
/// 3. id num filho do heading → idem, subindo um nível (irmãos do pai).
fn links_da_categoria(alvo: ElementRef, ids_menu: &HashSet<String>) -> Vec<(String, String)> {
    let sel_link = Selector::parse("li a[href]").unwrap();

    // 1. Dentro do próprio alvo.
    let dentro: Vec<(String, String)> = colher_links(alvo, &sel_link);
    if !dentro.is_empty() {
        return dentro;
    }

    // 2. Irmãos seguintes do alvo, até a próxima seção.
    let nos_irmaos = colher_ate_proxima_secao(alvo, ids_menu, &sel_link);
    if !nos_irmaos.is_empty() {
        return nos_irmaos;
    }

    // 3. Irmãos seguintes do PAI do alvo (id em filho do heading).
    if let Some(pai) = alvo.parent().and_then(ElementRef::wrap) {
        return colher_ate_proxima_secao(pai, ids_menu, &sel_link);
    }
    Vec::new()
}

/// Links `li > a[href]` dentro de um elemento.
fn colher_links(el: ElementRef, sel_link: &Selector) -> Vec<(String, String)> {
    el.select(sel_link)
        .filter_map(|a| {
            let href = a.value().attr("href")?.to_string();
            Some((a.text().collect::<String>(), href))
        })
        .collect()
}

/// Percorre os irmãos seguintes de `inicio` colhendo links, parando ANTES do irmão que é (ou
/// contém) outra âncora do menu — a próxima seção.
fn colher_ate_proxima_secao(
    inicio: ElementRef,
    ids_menu: &HashSet<String>,
    sel_link: &Selector,
) -> Vec<(String, String)> {
    let sel_com_id = Selector::parse("[id]").unwrap();
    let mut out = Vec::new();
    for irmao in inicio.next_siblings() {
        let Some(el) = ElementRef::wrap(irmao) else { continue };
        let proprio = el.value().attr("id").map(|id| ids_menu.contains(id)).unwrap_or(false);
        let contem = el
            .select(&sel_com_id)
            .any(|d| d.value().attr("id").map(|id| ids_menu.contains(id)).unwrap_or(false));
        if proprio || contem {
            break;
        }
        out.extend(colher_links(el, sel_link));
    }
    out
}

/// Guards D-RJ5 — reprovam página capada com erro descritivo.
fn validar(catalogo: &Catalogo) -> Result<()> {
    let com_itens = catalogo.categorias.iter().filter(|(_, n)| *n > 0).count();
    if com_itens < MIN_CATEGORIAS_COM_ITENS {
        bail!(
            "página capada? só {} categoria(s) com itens (mínimo {}). Se veio do cache, limpe \
             data/rj/raw/cache/ e re-raspe.",
            com_itens,
            MIN_CATEGORIAS_COM_ITENS
        );
    }
    let ocorrencias: usize = catalogo.items.iter().map(|s| s.ocorrencias.len()).sum();
    if ocorrencias < MIN_OCORRENCIAS {
        bail!(
            "página capada? só {} ocorrência(s) no total (mínimo {}). Se veio do cache, limpe \
             data/rj/raw/cache/ e re-raspe.",
            ocorrencias,
            MIN_OCORRENCIAS
        );
    }
    Ok(())
}

/// Normaliza texto: tira zero-width/nbsp e comprime espaços (padrão SP).
fn clean(s: &str) -> String {
    s.replace('\u{200b}', "").replace('\u{00a0}', " ").split_whitespace().collect::<Vec<_>>().join(" ")
}

/// URL canônica do item: trim; absoluto fica; protocol-relative ganha `https:`; relativo `/…`
/// ganha o host do portal. Devolve `None` para o que a regra de exclusão descarta (âncoras,
/// `javascript:`, `mailto:`, `tel:` e qualquer resultado não-http).
fn canonical(url: &str) -> Option<String> {
    let u = url.trim();
    let abs = if u.starts_with("http://") || u.starts_with("https://") {
        u.to_string()
    } else if let Some(rest) = u.strip_prefix("//") {
        format!("https://{}", rest)
    } else if u.starts_with('/') {
        format!("{}{}", BASE, u)
    } else {
        return None; // '#…', 'javascript:…', 'mailto:…', 'tel:…', relativo sem barra…
    };
    Some(abs)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fixture sintética: menu (ul de âncoras) + seções passadas pelo chamador.
    fn pagina(menu: &str, secoes: &str) -> String {
        format!(
            r##"<html><body>
            <a href="#conteudo">Ir para o conteúdo</a>
            <nav><ul>{menu}</ul></nav>
            <div id="conteudo">{secoes}</div>
            </body></html>"##
        )
    }

    const MENU_2: &str = r##"
        <li><a href="#Atendimento">Atendimento</a></li>
        <li><a href="#Cadastros">Cadastro</a></li>"##;

    #[test]
    fn menu_da_ordem_e_nomes_e_secao_por_wrapper() {
        // Caso 1 de links_da_categoria: id num wrapper que contém a lista.
        let html = pagina(
            MENU_2,
            r##"
            <div id="Atendimento"><h2>ATENDIMENTO</h2>
              <ul><li><a href="https://a.rj.gov.br/x">Agendamento</a></li></ul></div>
            <div id="Cadastros"><h2>CADASTROS</h2>
              <ul><li><a href="https://b.rj.gov.br/y">CISC</a></li></ul></div>"##,
        );
        let c = parse_catalogo(&html).unwrap();
        assert_eq!(
            c.categorias,
            vec![("Atendimento".to_string(), 1), ("Cadastro".to_string(), 1)],
            "ordem e nomes vêm do MENU (caixa mista), não dos headings"
        );
        assert_eq!(c.items.len(), 2);
        assert_eq!(c.items[0].ocorrencias[0].classe, "Atendimento");
        assert_eq!(c.items[0].ocorrencias[0].publico, PUBLICO_NOME);
    }

    #[test]
    fn secao_por_heading_com_id_e_irmaos() {
        // Caso 2: id no heading, lista como irmã — colhe até a próxima seção e para nela.
        let html = pagina(
            MENU_2,
            r##"
            <h2 id="Atendimento">ATENDIMENTO</h2>
            <ul><li><a href="https://a.rj.gov.br/1">Um</a></li>
                <li><a href="https://a.rj.gov.br/2">Dois</a></li></ul>
            <h2 id="Cadastros">CADASTROS</h2>
            <ul><li><a href="https://b.rj.gov.br/3">Três</a></li></ul>"##,
        );
        let c = parse_catalogo(&html).unwrap();
        assert_eq!(c.categorias, vec![("Atendimento".into(), 2), ("Cadastro".into(), 1)]);
        assert!(c.items.iter().all(|s| s.ocorrencias.len() == 1));
    }

    #[test]
    fn mesmo_link_e_titulo_em_duas_categorias_vira_um_servico_com_duas_ocorrencias() {
        // D-RJ2, lado "fiel depositário".
        let html = pagina(
            MENU_2,
            r##"
            <div id="Atendimento"><ul><li><a href="/scdi">SCDI Fiel Depositário</a></li></ul></div>
            <div id="Cadastros"><ul><li><a href="/scdi">SCDI Fiel Depositário</a></li></ul></div>"##,
        );
        let c = parse_catalogo(&html).unwrap();
        assert_eq!(c.items.len(), 1);
        assert_eq!(c.items[0].link, format!("{}/scdi", BASE), "relativo absolutizado");
        let classes: Vec<_> = c.items[0].ocorrencias.iter().map(|o| o.classe.as_str()).collect();
        assert_eq!(classes, vec!["Atendimento", "Cadastro"]);
    }

    #[test]
    fn mesmo_link_com_titulos_diferentes_sao_dois_servicos() {
        // D-RJ2, lado "DARJ/ITD": a identidade é (link, titulo).
        let html = pagina(
            MENU_2,
            r##"
            <div id="Atendimento"><ul>
              <li><a href="https://faz.rj.gov.br/pagamento">Emissão DARJ</a></li>
              <li><a href="https://faz.rj.gov.br/pagamento">Emitir pagamento de ITD</a></li>
            </ul></div>
            <div id="Cadastros"><ul><li><a href="https://b.rj/x">Outro</a></li></ul></div>"##,
        );
        let c = parse_catalogo(&html).unwrap();
        assert_eq!(c.items.len(), 3);
        assert_eq!(c.categorias[0].1, 2);
    }

    #[test]
    fn exclui_ancoras_javascript_e_titulo_vazio() {
        let html = pagina(
            MENU_2,
            r##"
            <div id="Atendimento"><ul>
              <li><a href="#topo">Voltar ao topo</a></li>
              <li><a href="javascript:void(0)">Clique</a></li>
              <li><a href="https://ok.rj.gov.br/s">Serviço válido</a></li>
            </ul></div>
            <div id="Cadastros"><ul><li><a href="https://b.rj/x">Outro</a></li></ul></div>"##,
        );
        let c = parse_catalogo(&html).unwrap();
        assert_eq!(c.items.len(), 2);
        assert_eq!(c.excluidos, 2);
        assert_eq!(c.categorias[0].1, 1);
    }

    #[test]
    fn ancora_sem_alvo_no_documento_nao_vira_categoria() {
        let html = pagina(
            r##"<li><a href="#Atendimento">Atendimento</a></li>
                <li><a href="#NaoExiste">Fantasma</a></li>"##,
            r##"<div id="Atendimento"><ul><li><a href="https://a.rj/1">Um</a></li></ul></div>"##,
        );
        // "#NaoExiste" não tem alvo no documento: nem entra como candidato de menu — e aí o
        // fantasma simplesmente não vira categoria. O erro de "âncora sem seção" cobre o caso de
        // o alvo sumir DEPOIS (id existe em outro lugar mas markup quebrado) — aqui validamos o
        // comportamento de não inventar categoria.
        let c = parse_catalogo(&html).unwrap();
        assert_eq!(c.categorias.len(), 1);
    }

    #[test]
    fn guards_reprovam_pagina_capada() {
        let html = pagina(
            MENU_2,
            r##"
            <div id="Atendimento"><ul><li><a href="https://a.rj/1">Um</a></li></ul></div>
            <div id="Cadastros"><ul><li><a href="https://b.rj/2">Dois</a></li></ul></div>"##,
        );
        let c = parse_catalogo(&html).unwrap();
        let err = validar(&c).unwrap_err().to_string();
        assert!(err.contains("categoria"), "esperava reprovação por categorias, veio: {err}");
    }

    #[test]
    fn canonical_cobre_os_formatos() {
        assert_eq!(canonical(" https://x.gov.br/a "), Some("https://x.gov.br/a".into()));
        assert_eq!(canonical("/darj"), Some(format!("{}/darj", BASE)));
        assert_eq!(canonical("//gnre.pe.gov.br/x"), Some("https://gnre.pe.gov.br/x".into()));
        assert_eq!(canonical("#Cadastros"), None);
        assert_eq!(canonical("javascript:void(0)"), None);
        assert_eq!(canonical("mailto:x@y.br"), None);
    }
}
