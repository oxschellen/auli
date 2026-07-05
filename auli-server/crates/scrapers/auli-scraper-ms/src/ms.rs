//! Coleta do catálogo de serviços da SEFAZ-MS (https://www.sefaz.ms.gov.br/servicos/).
//!
//! A fonte é o catálogo próprio da SEFAZ, WordPress **server-rendered** (D-MS1) — o Portal Único
//! ms.gov.br (SPA) entra só como destino dos links canônicos e como fonte futura de descrições
//! (Fase 2). A listagem expõe a grade inteira: filtros `?usuario=<perfil>` (públicos) e
//! `?categoria=<slug>` (classes) e paginação **"load more" cumulativa** via `?pp=` (pp=N devolve os
//! N primeiros; um `pp` alto traz o catálogo inteiro em 1 GET por filtro).
//!
//! Estratégia (variante B do plano): 1 GET "Todos" (ground truth + descoberta dos filtros na
//! própria página — nada de taxonomia hardcoded) + 1 GET por perfil + 1 GET por categoria, todos
//! com `pp` alto. Perfis e categorias são taxonomias WordPress **independentes**: cada serviço tem
//! um conjunto P(s) e um conjunto C(s), e as `ocorrencias` são o produto P(s) × C(s) (D-MS3) —
//! exatamente o que o portal renderiza ao combinar filtros. Fallback "Geral" para órfãos.
//!
//! Identidade = `link` (URL canônica do Portal Único, id numérico embutido — única) (D-MS2).
//! Sem descrição na v1 (a listagem é título+link; o detalhe é SPA) (D-MS4). Rótulos fiéis ao
//! publicado, typo de slug incluído (D-MS6).
//!
//! Guards (D-MS5 corrigido, lição do CE). O portal **não** renderiza contador no HTML (o
//! "Mostrando X de N" é montado por JS), então o invariante é ancorado em sinais que existem e se
//! cruzam: **`N` = âncoras de serviço distintas do "Todos"** (dinâmico, ~276 observados) e o
//! **cross-check `união(filtros) ⊆ Todos`** — um link que aparece num filtro mas não no "Todos"
//! denuncia um "Todos" capado. Cap-detect adicional por `pp` no fetch. Cache só grava DEPOIS dos
//! guards (princípio D-RJ5).

use std::collections::HashSet;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use auli_contract::{Ocorrencia, Publico, ServicoRaw};
use scraper::{Html, Selector};

/// A listagem do catálogo (D-MS1).
const LISTA_URL: &str = "https://www.sefaz.ms.gov.br/servicos/";
const USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64; rv:124.0) Gecko/20100101 Firefox/124.0";

/// `pp` inicial ("load more" cumulativo). Folgado sobre o catálogo (~276). Se um GET vier com
/// EXATAMENTE `pp` itens distintos, pode estar capado nesse teto → refazemos UMA vez com `pp*4`.
const PP_INICIAL: u32 = 500;
/// Cortesia entre GETs de rede (~26 por raspagem completa).
const COURTESY: Duration = Duration::from_millis(400);

/// Fallback para serviços órfãos de perfil/categoria (D-MS3). Se usado, o público "Geral" entra
/// ao FIM de `publicos_ordem`.
const GERAL: &str = "Geral";
/// Órgão de origem.
const ORGAO: &str = "SEFAZ-MS";

/// Guard estático de folga por baixo (o invariante principal é o cross-check dinâmico D-MS5;
/// ~276 serviços observados em 2026-07).
const MIN_SERVICOS: usize = 240;

/// Um filtro descoberto na página "Todos": slug (valor do query param) + rótulo como exibido.
type Filtro = (String, String);

/// Raspa o catálogo e devolve `(items, publicos_ordem)` prontos para o snapshot v3.
pub fn scrape(
    data_dir: &str,
    use_cache: bool,
) -> Result<(Vec<ServicoRaw>, Vec<Publico>), Box<dyn std::error::Error>> {
    let agent = auli_scraper_kit::build_agent(USER_AGENT, Some(Duration::from_secs(30)));
    // Páginas cruas na ordem (url_lógica, html) — o cache só grava depois dos guards (D-RJ5).
    let mut raw: Vec<(String, String)> = Vec::new();

    // 1. "Todos": ground truth (conjunto completo) e descoberta dos filtros na própria página.
    let (todos_html, todos) = fetch_lista(&agent, data_dir, use_cache, "", &mut raw)?;
    let perfis = extrair_filtros(&todos_html, "usuario=")?;
    let categorias = extrair_filtros(&todos_html, "categoria=")?;
    println!(
        "MS: Todos -> {} serviços; {} perfis, {} categorias descobertos",
        todos.len(),
        perfis.len(),
        categorias.len()
    );

    // 2. Um GET por perfil e por categoria: pertencimento por conjunto de links.
    let mut por_perfil: Vec<(String, HashSet<String>)> = Vec::new();
    for (slug, nome) in &perfis {
        let (_, itens) =
            fetch_lista(&agent, data_dir, use_cache, &format!("usuario={}&", slug), &mut raw)?;
        println!("MS: perfil {} -> {} itens", nome, itens.len());
        if itens.is_empty() {
            eprintln!("⚠️  MS: perfil '{}' sem itens — suspeito (markup/filtro mudou?).", nome);
        }
        por_perfil.push((nome.clone(), itens.into_iter().map(|(_, l)| l).collect()));
    }
    let mut por_categoria: Vec<(String, HashSet<String>)> = Vec::new();
    for (slug, nome) in &categorias {
        let (_, itens) =
            fetch_lista(&agent, data_dir, use_cache, &format!("categoria={}&", slug), &mut raw)?;
        if itens.is_empty() {
            println!("MS: categoria {} -> 0 itens", nome);
        }
        por_categoria.push((nome.clone(), itens.into_iter().map(|(_, l)| l).collect()));
    }

    // 3. Fold: ocorrências = P(s) × C(s) (D-MS3), na ordem de descoberta dos filtros.
    let (items, usou_geral) = montar_servicos(&todos, &por_perfil, &por_categoria);

    // 4. Guards (D-MS5) — antes de qualquer escrita de cache.
    let uperfis: HashSet<&String> = por_perfil.iter().flat_map(|(_, s)| s.iter()).collect();
    let ucats: HashSet<&String> = por_categoria.iter().flat_map(|(_, s)| s.iter()).collect();
    validar(&items, &uperfis, &ucats)?;
    for (logical, html) in &raw {
        auli_scraper_kit::cache::write(data_dir, logical, html);
    }

    let ocorrencias: usize = items.iter().map(|s| s.ocorrencias.len()).sum();
    println!(
        "MS: {} serviços ({} ocorrências) em {} perfis × {} categorias",
        items.len(),
        ocorrencias,
        perfis.len(),
        categorias.len()
    );

    let mut publicos_ordem: Vec<Publico> =
        perfis.iter().map(|(slug, nome)| Publico { nome: nome.clone(), slug: slug_publico(slug) }).collect();
    if usou_geral {
        publicos_ordem.push(Publico { nome: GERAL.into(), slug: "servicos-gerais".into() });
    }
    Ok((items, publicos_ordem))
}

/// Slug do arquivo per-público a partir do slug do filtro (ex.: `cidadao` -> `servicos-cidadao`).
fn slug_publico(filtro: &str) -> String {
    format!("servicos-{}", filtro)
}

/// Busca (ou lê do cache) UMA listagem — "Todos" (`filtro_qs` vazio) ou filtrada
/// (`"usuario=x&"`/`"categoria=y&"`) — e devolve `(html, itens)` com os itens `(titulo, link)` já
/// dedup por link. Cap-detect: se a página vier com EXATAMENTE `pp` itens (pode estar capada no
/// teto), refaz uma vez com `pp*4`; cheia de novo = erro. Acumula a página crua em `raw` para o
/// cache pós-guards. A chave de cache é a LISTAGEM (o filtro), sem o `pp` — o pp é mecanismo de
/// fetch, não identidade, então `--usecache` acha a página mesmo se a coleta precisou subir o pp.
fn fetch_lista(
    agent: &ureq::Agent,
    data_dir: &str,
    use_cache: bool,
    filtro_qs: &str,
    raw: &mut Vec<(String, String)>,
) -> Result<(String, Vec<(String, String)>)> {
    let logical = format!("{}?{}ordem=AZ", LISTA_URL, filtro_qs);
    if let Some(cached) = auli_scraper_kit::cache::read(data_dir, &logical) {
        println!("Cache hit: {}", logical);
        let itens = parse_lista(&cached);
        return Ok((cached, itens));
    }
    if use_cache {
        bail!("cache miss para {} (modo --usecache, sem rede)", logical);
    }

    let mut pp = PP_INICIAL;
    for tentativa in 0..2 {
        let url = format!("{}?{}ordem=AZ&pp={}", LISTA_URL, filtro_qs, pp);
        let html = get_string(agent, &url)?;
        sleep(COURTESY);
        let itens = parse_lista(&html);
        // Página não-cheia (< pp): o `pp` cobriu tudo — resultado confiável.
        if itens.len() < pp as usize {
            raw.push((logical, html.clone()));
            return Ok((html, itens));
        }
        // Exatamente `pp` itens: pode estar capada nesse teto. Uma segunda tentativa com pp maior.
        if tentativa == 0 {
            eprintln!(
                "⚠️  MS: {} itens == pp={} (possível teto); refazendo com pp={}.",
                itens.len(),
                pp,
                pp * 4
            );
            pp *= 4;
        } else {
            bail!(
                "listagem ainda cheia com pp={} ({} itens) — catálogo maior que o teto ou capado ({}).",
                pp,
                itens.len(),
                logical
            );
        }
    }
    unreachable!("o loop retorna ou dá bail em 2 tentativas");
}

/// GET com retentativas e backoff (padrão da frota).
fn get_string(agent: &ureq::Agent, url: &str) -> Result<String> {
    let max_attempts = 3;
    let mut delay = Duration::from_millis(800);
    let mut last = anyhow!("sem tentativa");
    println!("Fetching: {}", url);
    for attempt in 1..=max_attempts {
        match agent.get(url).call() {
            Ok(mut resp) => match resp.body_mut().read_to_string() {
                Ok(s) if !s.trim().is_empty() => return Ok(s),
                Ok(_) => last = anyhow!("resposta vazia"),
                Err(e) => last = anyhow!(e.to_string()),
            },
            Err(e) => last = anyhow!(e.to_string()),
        }
        if attempt < max_attempts {
            eprintln!("⚠️  MS: tentativa {} falhou ({}); retentando…", attempt, last);
            sleep(delay);
            delay *= 2;
        }
    }
    Err(anyhow!("falha ao buscar {} após {} tentativas: {}", url, max_attempts, last))
}

/// Parseia uma página de listagem: os itens de serviço `(titulo, link)`, dedup por link.
///
/// O item de serviço é reconhecido pela **assinatura do link canônico** do Portal Único —
/// `https://www.ms.gov.br/<tema>/<slug><id-numérico>/` (último segmento termina em dígito) —
/// em vez de classes CSS do tema: sobrevive a restyling e exclui a navegação por construção
/// (ex.: `/orgao/...87/servicos` termina em "servicos", não em dígito).
fn parse_lista(html: &str) -> Vec<(String, String)> {
    let doc = Html::parse_document(html);
    let sel_a = Selector::parse("a[href]").unwrap();
    let mut itens: Vec<(String, String)> = Vec::new();
    let mut vistos: HashSet<String> = HashSet::new();
    for a in doc.select(&sel_a) {
        let Some(href) = a.value().attr("href") else { continue };
        let href = href.trim();
        if !eh_link_de_servico(href) {
            continue;
        }
        let titulo = clean(&a.text().collect::<String>());
        if titulo.is_empty() || !vistos.insert(href.to_string()) {
            continue;
        }
        itens.push((titulo, href.to_string()));
    }
    itens
}

/// Assinatura do link canônico de serviço no Portal Único: host `www.ms.gov.br`, path com ≥ 2
/// segmentos e o último terminando em dígito (o id numérico colado ao slug).
fn eh_link_de_servico(href: &str) -> bool {
    let Some(path) = href.strip_prefix("https://www.ms.gov.br/") else { return false };
    if path.contains('?') || path.contains('#') {
        return false;
    }
    let segs: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if segs.len() < 2 {
        return false;
    }
    segs.last().unwrap().chars().last().is_some_and(|c| c.is_ascii_digit())
}

/// Fold: para cada serviço do "Todos" (ordem AZ do portal), P(s) e C(s) por pertencimento aos
/// conjuntos filtrados; `ocorrencias` = produto P(s) × C(s) na ordem de descoberta dos filtros;
/// órfãos caem no fallback "Geral" (D-MS3). Devolve também se o fallback de público foi usado.
fn montar_servicos(
    todos: &[(String, String)],
    por_perfil: &[(String, HashSet<String>)],
    por_categoria: &[(String, HashSet<String>)],
) -> (Vec<ServicoRaw>, bool) {
    let mut usou_geral_publico = false;
    let mut orfaos_perfil = 0usize;
    let mut orfaos_categoria = 0usize;

    let items: Vec<ServicoRaw> = todos
        .iter()
        .map(|(titulo, link)| {
            let mut ps: Vec<&str> =
                por_perfil.iter().filter(|(_, set)| set.contains(link)).map(|(n, _)| n.as_str()).collect();
            let mut cs: Vec<&str> = por_categoria
                .iter()
                .filter(|(_, set)| set.contains(link))
                .map(|(n, _)| n.as_str())
                .collect();
            if ps.is_empty() {
                ps.push(GERAL);
                usou_geral_publico = true;
                orfaos_perfil += 1;
            }
            if cs.is_empty() {
                cs.push(GERAL);
                orfaos_categoria += 1;
            }
            let ocorrencias = ps
                .iter()
                .flat_map(|p| {
                    cs.iter().map(move |c| Ocorrencia { publico: (*p).into(), classe: (*c).into() })
                })
                .collect();
            ServicoRaw {
                titulo: titulo.clone(),
                descricao: String::new(), // D-MS4: v1 sem descrição (detalhe é a SPA do Portal Único).
                link: link.clone(),
                orgao: ORGAO.to_string(),
                ocorrencias,
            }
        })
        .collect();

    if orfaos_perfil + orfaos_categoria > 0 {
        eprintln!(
            "⚠️  MS: órfãos com fallback '{}': {} sem perfil, {} sem categoria.",
            GERAL, orfaos_perfil, orfaos_categoria
        );
    }
    (items, usou_geral_publico)
}

/// Filtros descobertos na página "Todos": âncoras cujo href carrega `usuario=`/`categoria=`.
/// Ordem de aparição preservada (vira a ordem de abas/grupos); dedup por slug (primeiro vence).
fn extrair_filtros(html: &str, param: &str) -> Result<Vec<Filtro>> {
    let doc = Html::parse_document(html);
    let sel_a = Selector::parse("a[href]").unwrap();
    let mut out: Vec<Filtro> = Vec::new();
    let mut vistos: HashSet<String> = HashSet::new();
    for a in doc.select(&sel_a) {
        let Some(href) = a.value().attr("href") else { continue };
        let Some(pos) = href.find(param) else { continue };
        let valor = href[pos + param.len()..]
            .split(['&', '#'])
            .next()
            .unwrap_or("")
            .trim();
        let nome = clean(&a.text().collect::<String>());
        if valor.is_empty() || nome.is_empty() || !vistos.insert(valor.to_string()) {
            continue;
        }
        out.push((valor.to_string(), nome));
    }
    if out.is_empty() {
        bail!("nenhum filtro '{}' encontrado na página — markup mudou?", param);
    }
    Ok(out)
}

/// Guards D-MS5 (sem contador — ele é JS-only). O sinal de completude é **cruzado**: todo link
/// visto num filtro (perfil/categoria) tem que estar na listagem "Todos"; um link de filtro fora
/// do "Todos" denuncia um "Todos" capado/incompleto. Depois, o piso estático de folga.
fn validar(
    items: &[ServicoRaw],
    uperfis: &HashSet<&String>,
    ucats: &HashSet<&String>,
) -> Result<()> {
    let todos: HashSet<&str> = items.iter().map(|s| s.link.as_str()).collect();
    if let Some(fora) = uperfis
        .iter()
        .chain(ucats.iter())
        .find(|l| !todos.contains(l.as_str()))
    {
        bail!(
            "Todos incompleto/capado: link aparece num filtro mas não na listagem geral — ex.: {}. \
             Se veio do cache, limpe data/ms/raw/cache/ e re-raspe.",
            fora
        );
    }
    if items.len() < MIN_SERVICOS {
        bail!(
            "catálogo capado? só {} serviço(s) (mínimo {}). Se veio do cache, limpe \
             data/ms/raw/cache/ e re-raspe.",
            items.len(),
            MIN_SERVICOS
        );
    }
    Ok(())
}

/// Normaliza texto: tira zero-width/nbsp e comprime espaços (padrão da frota).
fn clean(s: &str) -> String {
    s.replace('\u{200b}', "").replace('\u{00a0}', " ").split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn itens(pares: &[(&str, &str)]) -> Vec<(String, String)> {
        pares.iter().map(|(t, l)| (t.to_string(), l.to_string())).collect()
    }

    fn set(links: &[&str]) -> HashSet<String> {
        links.iter().map(|s| s.to_string()).collect()
    }

    fn svc(link: &str) -> ServicoRaw {
        ServicoRaw {
            titulo: "t".into(),
            descricao: String::new(),
            link: link.into(),
            orgao: ORGAO.into(),
            ocorrencias: vec![],
        }
    }

    const PAGINA: &str = r#"<html><body>
      <nav>
        <a href="/servicos/?usuario=cidadao&ordem=AZ">Cidadão</a>
        <a href="/servicos/?usuario=produtor-rural&ordem=AZ">Produtor Rural</a>
        <a href="/servicos/?categoria=ipva">IPVA</a>
        <a href="/servicos/?categoria=comunicacao-e-transparencia">comunicacao-e-transparencia</a>
        <a href="https://www.ms.gov.br/orgao/sefaz-ms87/servicos">SEFAZ no Portal</a>
      </nav>
      <main>
        <ul>
          <li><a href="https://www.ms.gov.br/financas-e-impostos/emitir-guia-ipva1234/">Emitir guia de IPVA</a></li>
          <li><a href="https://www.ms.gov.br/empresa-industria-e-comercio/inscricao-estadual567/">Inscrição estadual</a></li>
          <li><a href="https://www.ms.gov.br/financas-e-impostos/emitir-guia-ipva1234/">Emitir guia de IPVA</a></li>
        </ul>
      </main>
    </body></html>"#;

    #[test]
    fn parse_lista_extrai_servicos_e_dedup_por_link() {
        let l = parse_lista(PAGINA);
        assert_eq!(l.len(), 2, "dedup por link; navegação (/orgao/…) excluída pela assinatura");
        assert_eq!(l[0].0, "Emitir guia de IPVA");
        assert_eq!(l[1].0, "Inscrição estadual");
    }

    #[test]
    fn assinatura_do_link_exclui_navegacao() {
        assert!(eh_link_de_servico("https://www.ms.gov.br/financas-e-impostos/emitir-guia-ipva1234/"));
        assert!(eh_link_de_servico("https://www.ms.gov.br/tema/slug99")); // sem barra final
        assert!(!eh_link_de_servico("https://www.ms.gov.br/orgao/sefaz-ms87/servicos"), "não termina em dígito");
        assert!(!eh_link_de_servico("https://www.ms.gov.br/somente-um-seg9/"), "path com 1 segmento");
        assert!(!eh_link_de_servico("https://www.sefaz.ms.gov.br/servicos/?pp=30"), "host errado");
        assert!(!eh_link_de_servico("https://www.ms.gov.br/a/b1/?x=1"), "querystring não é detalhe");
    }

    #[test]
    fn extrai_filtros_na_ordem_com_rotulo_do_portal() {
        let perfis = extrair_filtros(PAGINA, "usuario=").unwrap();
        assert_eq!(perfis, vec![
            ("cidadao".to_string(), "Cidadão".to_string()),
            ("produtor-rural".to_string(), "Produtor Rural".to_string()),
        ]);
        let cats = extrair_filtros(PAGINA, "categoria=").unwrap();
        assert_eq!(cats[1].1, "comunicacao-e-transparencia", "fidelidade ao rótulo publicado (D-MS6)");
    }

    #[test]
    fn fold_faz_produto_perfis_x_categorias_na_ordem() {
        let todos = itens(&[("A", "https://www.ms.gov.br/t/a1/"), ("B", "https://www.ms.gov.br/t/b2/")]);
        let por_perfil = vec![
            ("Cidadão".to_string(), set(&["https://www.ms.gov.br/t/a1/", "https://www.ms.gov.br/t/b2/"])),
            ("Empresa".to_string(), set(&["https://www.ms.gov.br/t/a1/"])),
        ];
        let por_categoria = vec![
            ("IPVA".to_string(), set(&["https://www.ms.gov.br/t/a1/"])),
            ("Certidões".to_string(), set(&["https://www.ms.gov.br/t/a1/"])),
        ];
        let (items, usou_geral) = montar_servicos(&todos, &por_perfil, &por_categoria);
        // A: 2 perfis × 2 categorias = 4 ocorrências, na ordem (Cidadão,IPVA), (Cidadão,Certidões)…
        assert_eq!(items[0].ocorrencias.len(), 4);
        assert_eq!(items[0].ocorrencias[0].publico, "Cidadão");
        assert_eq!(items[0].ocorrencias[0].classe, "IPVA");
        assert_eq!(items[0].ocorrencias[1].classe, "Certidões");
        assert_eq!(items[0].ocorrencias[2].publico, "Empresa");
        // B: perfil Cidadão, sem categoria -> classe Geral (fallback só na categoria).
        assert_eq!(items[1].ocorrencias.len(), 1);
        assert_eq!(items[1].ocorrencias[0].publico, "Cidadão");
        assert_eq!(items[1].ocorrencias[0].classe, GERAL);
        assert!(!usou_geral, "fallback de PÚBLICO não foi usado (só o de categoria)");
    }

    #[test]
    fn orfao_de_perfil_cai_no_publico_geral() {
        let todos = itens(&[("X", "https://www.ms.gov.br/t/x9/")]);
        let (items, usou_geral) = montar_servicos(&todos, &[], &[]);
        assert!(usou_geral);
        assert_eq!(items[0].ocorrencias[0].publico, GERAL);
        assert_eq!(items[0].ocorrencias[0].classe, GERAL);
    }

    #[test]
    fn validar_reprova_link_de_filtro_fora_do_todos() {
        // Um link visto num perfil mas ausente do "Todos" => Todos capado/incompleto.
        let items = vec![svc("https://www.ms.gov.br/t/a1/")];
        let b2 = "https://www.ms.gov.br/t/b2/".to_string();
        let uperfis: HashSet<&String> = [&b2].into_iter().collect();
        let err = validar(&items, &uperfis, &HashSet::new()).unwrap_err().to_string();
        assert!(err.contains("incompleto") || err.contains("capado"), "esperava erro de completude: {err}");
    }

    #[test]
    fn validar_reprova_abaixo_do_minimo() {
        // Uniões coerentes (⊆ Todos), mas poucos itens => piso estático reprova.
        let a1 = "https://www.ms.gov.br/t/a1/".to_string();
        let items = vec![svc(&a1)];
        let uperfis: HashSet<&String> = [&a1].into_iter().collect();
        let err = validar(&items, &uperfis, &HashSet::new()).unwrap_err().to_string();
        assert!(err.contains("mínimo") || err.contains("capado"), "esperava erro de piso: {err}");
    }
}
