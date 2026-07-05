//! Coleta dos serviços da SEFAZ-AM a partir do "Portfólio de Serviços" (Next.js **App Router**).
//!
//! O portal é uma SPA Next.js App Router — NÃO há `__NEXT_DATA__` nem rota `/_next/data/{buildId}`
//! (isso é Pages Router). A listagem inteira vem **server-rendered no flight RSC**, obtido com o
//! header **`RSC: 1`** na própria URL da página (`Content-Type: text/x-component`). Não há chamada
//! lazy: expandir os accordions do detalhe não dispara XHR (verificado na descoberta). Portanto a
//! coleta é `ureq` GET + parse do flight — sem navegador.
//!
//! No flight, o componente de listagem é `["$","$L8",null,{"items":[…]}]`. `items` é a **árvore pura
//! em JSON**: categoria → (subcategoria opcional) → serviço-folha. Extraímos o array `items`
//! (âncora única `{"items":[`) por balanceamento de colchetes e o desserializamos.
//!
//! Modelagem (padrão MS — `ServicoRaw` direto, N ocorrências por serviço):
//! - **classe** = a categoria de topo da árvore (19 categorias).
//! - **público** = PF/PJ/Órgãos, deduzido por pertencimento às 3 **rotas de perfil**
//!   (`/portfolio-servicos/{pessoa-fisica,pessoa-juridica,orgaos-publicos}`); um serviço pode servir
//!   a vários públicos (sobreposição real). `ocorrencias` = {público × classe} para cada público.
//! - identidade = `id` (inteiro); `link` = `…/portfolio-servicos/detalhes/{id}` (interno) ou a URL
//!   externa/submenu servida no próprio nó. `descricao` = `description` da listagem (resumo curto —
//!   o conteúdo rico do detalhe ficou fora por decisão de escopo).
//! - **agendável** NÃO é público (a rota `/agendaveis` devolve tudo); é atributo — ignorado como
//!   faceta. **Duplicatas** publicadas (nome igual, `id` distinto) são mantidas (fidelidade).

use std::collections::HashSet;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use auli_contract::{Ocorrencia, Publico, ServicoRaw};
use auli_scraper_kit::clean;
use auli_scraper_kit::http::GetOpts;
use serde::Deserialize;

const BASE: &str = "https://www.sefaz.am.gov.br";
/// Listagem completa (sem paginação): a árvore de 278 serviços.
const LISTA_URL: &str = "https://www.sefaz.am.gov.br/portfolio-servicos/todos";
/// Rotas de perfil: (segmento da URL, nome do público, slug). A ordem define `publicos_ordem`.
const PERFIS: [(&str, &str, &str); 3] = [
    ("pessoa-fisica", "Pessoa Física", "pessoa-fisica"),
    ("pessoa-juridica", "Pessoa Jurídica", "pessoa-juridica"),
    ("orgaos-publicos", "Órgãos Públicos", "orgaos-publicos"),
];
/// Órgão de origem.
const ORGAO: &str = "SEFAZ-AM";
/// Classe de fallback (um serviço-folha sem categoria de topo — não observado, mas defensivo).
const CLASSE_FALLBACK: &str = "Geral";
/// Cortesia entre GETs.
const COURTESY: Duration = Duration::from_millis(300);
/// Guard (princípio D-RJ5): mínimo de serviços. Folga sob os 278 observados; rejeita catálogo capado.
const MIN_SERVICOS: usize = 250;

/// Um nó da árvore `items` (categoria/subcategoria/serviço). Só os campos que usamos.
#[derive(Debug, Deserialize)]
struct Node {
    #[serde(default)]
    id: Option<i64>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    children: Vec<Node>,
}

/// Um serviço-folha achatado da árvore (categoria de topo já resolvida).
#[derive(Debug, Clone)]
struct Leaf {
    id: i64,
    name: String,
    description: String,
    url: String,
    categoria: String,
}

/// Raspa o portfólio e devolve `(items, publicos_ordem)` prontos para o snapshot v3.
pub fn scrape(
    data_dir: &str,
    use_cache: bool,
) -> Result<(Vec<ServicoRaw>, Vec<Publico>), Box<dyn std::error::Error>> {
    let agent =
        auli_scraper_kit::build_agent(auli_scraper_kit::USER_AGENT, Some(Duration::from_secs(30)));

    // Flights buscados da rede que só entram no cache DEPOIS dos guards (D-RJ5).
    let mut pending: Vec<(String, String)> = Vec::new();

    // 1) Listagem completa -> serviços-folha (com categoria de topo).
    let todos_raw = load_flight(&agent, data_dir, LISTA_URL, use_cache, &mut pending)?;
    let todos_leaves = leaves(&parse_items(&todos_raw)?);

    // 2) Rotas de perfil -> conjuntos de ids por público.
    let mut perfil_sets: Vec<(&str, HashSet<i64>)> = Vec::new();
    for (rota, nome, _slug) in PERFIS {
        let url = format!("{}/portfolio-servicos/{}", BASE, rota);
        let raw = load_flight(&agent, data_dir, &url, use_cache, &mut pending)?;
        let set: HashSet<i64> = leaves(&parse_items(&raw)?).into_iter().map(|l| l.id).collect();
        perfil_sets.push((nome, set));
    }

    // 3) Montagem: cada serviço-folha vira um ServicoRaw com N ocorrências (público × classe).
    let items = build_servicos(&todos_leaves, &perfil_sets);
    validar(&items)?;

    // Cache só DEPOIS dos guards.
    for (url, raw) in &pending {
        auli_scraper_kit::cache::write(data_dir, url, raw);
    }

    let ocorr: usize = items.iter().map(|s| s.ocorrencias.len()).sum();
    println!("AM: {} serviços ({} ocorrências, dedup por id)", items.len(), ocorr);
    let publicos_ordem = PERFIS
        .iter()
        .map(|(_, nome, slug)| Publico { nome: nome.to_string(), slug: slug.to_string() })
        .collect();
    Ok((items, publicos_ordem))
}

/// Busca um flight RSC (com cache). Miss + `--usecache` = erro (nunca fallback). Rede -> vai para
/// `pending` (cacheado só após os guards).
fn load_flight(
    agent: &ureq::Agent,
    data_dir: &str,
    url: &str,
    use_cache: bool,
    pending: &mut Vec<(String, String)>,
) -> Result<String> {
    if let Some(cached) = auli_scraper_kit::cache::read(data_dir, url) {
        println!("Cache hit: {}", url);
        return Ok(cached);
    }
    if use_cache {
        bail!("cache vazio para {} (--usecache, sem rede). Rode uma coleta com rede primeiro.", url);
    }
    let body = fetch_flight(agent, url)?;
    pending.push((url.to_string(), body.clone()));
    sleep(COURTESY);
    Ok(body)
}

/// GET com header `RSC: 1` -> o flight cru. Retenta com backoff (via kit).
fn fetch_flight(agent: &ureq::Agent, url: &str) -> Result<String> {
    println!("GET (RSC): {}", url);
    let body = auli_scraper_kit::http::get_string(
        agent,
        url,
        &GetOpts { log_prefix: "AM", headers: &[("RSC", "1")], ..Default::default() },
    )?;
    // Defesa: o flight bom carrega a âncora da listagem. Sem ela = página de erro/markup mudou.
    if !body.contains("\"items\":[") {
        bail!("flight sem a âncora \"items\" ({}) — erro/HTML? primeiros bytes: {:?}",
            url, body.chars().take(60).collect::<String>());
    }
    Ok(body)
}

/// Extrai e desserializa o array `items` do flight (âncora única `{"items":[`).
fn parse_items(flight: &str) -> Result<Vec<Node>> {
    const KEY: &str = "{\"items\":[";
    let i = flight
        .find(KEY)
        .ok_or_else(|| anyhow!("âncora \"items\" não encontrada no flight (markup RSC mudou?)"))?;
    let arr_start = i + KEY.len() - 1; // posição do '['
    let slice = balanced(&flight[arr_start..])?;
    serde_json::from_str::<Vec<Node>>(slice).map_err(|e| anyhow!("JSON de items inválido: {}", e))
}

/// Devolve o prefixo balanceado de `s` (que começa em `[` ou `{`), respeitando strings/escapes.
fn balanced(s: &str) -> Result<&str> {
    let mut depth: i32 = 0;
    let mut instr = false;
    let mut esc = false;
    for (i, &b) in s.as_bytes().iter().enumerate() {
        if esc {
            esc = false;
            continue;
        }
        match b {
            b'\\' if instr => esc = true,
            b'"' => instr = !instr,
            b'[' | b'{' if !instr => depth += 1,
            b']' | b'}' if !instr => {
                depth -= 1;
                if depth == 0 {
                    return Ok(&s[..=i]); // b é ASCII (']'/'}'), logo i+1 é fronteira UTF-8 válida
                }
            }
            _ => {}
        }
    }
    bail!("estrutura não balanceada no flight")
}

/// Achata a árvore em serviços-folha, propagando a **categoria de topo** (nome do nó de nível 0).
fn leaves(nodes: &[Node]) -> Vec<Leaf> {
    fn walk(nodes: &[Node], top: Option<&str>, out: &mut Vec<Leaf>) {
        for n in nodes {
            if !n.children.is_empty() {
                // No nível 0, `top` é None -> a categoria passa a ser o nome deste nó.
                let t = top.or(n.name.as_deref());
                walk(&n.children, t, out);
            } else if let Some(id) = n.id {
                out.push(Leaf {
                    id,
                    name: n.name.clone().unwrap_or_default(),
                    description: n.description.clone().unwrap_or_default(),
                    url: n.url.clone().unwrap_or_default(),
                    categoria: top.unwrap_or(CLASSE_FALLBACK).to_string(),
                });
            }
        }
    }
    let mut out = Vec::new();
    walk(nodes, None, &mut out);
    out
}

/// Monta os `ServicoRaw` (dedup por `id`), com `ocorrencias` = {público × classe} por pertencimento.
fn build_servicos(todos: &[Leaf], perfil_sets: &[(&str, HashSet<i64>)]) -> Vec<ServicoRaw> {
    let mut vistos: HashSet<i64> = HashSet::new();
    let mut out: Vec<ServicoRaw> = Vec::new();
    for leaf in todos {
        if !vistos.insert(leaf.id) {
            continue;
        }
        let titulo = clean(&leaf.name);
        if titulo.is_empty() {
            continue;
        }
        let ocorrencias: Vec<Ocorrencia> = perfil_sets
            .iter()
            .filter(|(_, set)| set.contains(&leaf.id))
            .map(|(nome, _)| Ocorrencia {
                publico: (*nome).to_string(),
                classe: leaf.categoria.clone(),
            })
            .collect();
        if ocorrencias.is_empty() {
            // Não observado (todo serviço de `todos` está em ≥1 perfil); avisa se o portal mudar.
            eprintln!("⚠️  AM: serviço {} ({}) sem público em nenhuma rota de perfil — pulando.",
                leaf.id, titulo);
            continue;
        }
        out.push(ServicoRaw {
            titulo,
            descricao: clean(&leaf.description),
            link: link(&leaf.url),
            orgao: ORGAO.to_string(),
            ocorrencias,
        });
    }
    out
}

/// Link canônico: absolutiza relativos (`/…` -> `BASE/…`) e tira o filtro `?profile=…`. URLs externas
/// (sistemas/online.sefaz, etc.) ficam como servidas.
fn link(url: &str) -> String {
    let u = url.split("?profile=").next().unwrap_or(url).trim();
    if let Some(rest) = u.strip_prefix('/') {
        format!("{}/{}", BASE, rest)
    } else {
        u.to_string()
    }
}

/// Guard (princípio D-RJ5): reprova catálogo capado (abaixo do mínimo — falha de rota/parse).
fn validar(items: &[ServicoRaw]) -> Result<()> {
    if items.len() < MIN_SERVICOS {
        bail!(
            "catálogo capado? só {} serviço(s) (mínimo {}). Se veio do cache, limpe data/am/raw/cache/ \
             e re-raspe.",
            items.len(),
            MIN_SERVICOS
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Flight mínimo: 2 categorias, 3 serviços-folha (um em subcategoria).
    const FLIGHT_TODOS: &str = r#"garbage...["$","$L8",null,{"items":[
      {"name":"IPVA e Veículos","actions":[],"children":[
        {"id":63,"name":"Isentar IPVA (Roubo/Furto)","description":"Pedido de isenção.","url":"/portfolio-servicos/detalhes/63?profile=todos","children":[]},
        {"id":882,"name":"  Pedir Inscrição  Estadual ","description":"Obtenção de IE.","url":"/portfolio-servicos/detalhes/882?profile=todos","children":[]}
      ]},
      {"name":"Consulta Pública","actions":[],"children":[
        {"name":"Preços","children":[
          {"id":900,"name":"Busca Preço","description":"Consulta.","url":"https://buscapreco.sefaz.am.gov.br/home","children":[]}
        ]}
      ]}
    ]}]...trailing"#;

    fn ids(json: &str) -> HashSet<i64> {
        leaves(&parse_items(json).unwrap()).into_iter().map(|l| l.id).collect()
    }

    #[test]
    fn parse_items_extrai_arvore() {
        let nodes = parse_items(FLIGHT_TODOS).unwrap();
        assert_eq!(nodes.len(), 2, "duas categorias de topo");
    }

    #[test]
    fn leaves_achata_e_propaga_categoria() {
        let lv = leaves(&parse_items(FLIGHT_TODOS).unwrap());
        assert_eq!(lv.len(), 3);
        // categoria de topo propagada mesmo através de subcategoria.
        let busca = lv.iter().find(|l| l.id == 900).unwrap();
        assert_eq!(busca.categoria, "Consulta Pública");
        assert_eq!(lv.iter().find(|l| l.id == 63).unwrap().categoria, "IPVA e Veículos");
    }

    #[test]
    fn build_ocorrencias_por_pertencimento() {
        let todos = leaves(&parse_items(FLIGHT_TODOS).unwrap());
        // 63 em PF+PJ; 882 só PJ; 900 só PF.
        let perfis: Vec<(&str, HashSet<i64>)> = vec![
            ("Pessoa Física", HashSet::from([63, 900])),
            ("Pessoa Jurídica", HashSet::from([63, 882])),
            ("Órgãos Públicos", HashSet::from([])),
        ];
        let items = build_servicos(&todos, &perfis);
        assert_eq!(items.len(), 3);
        let s63 = items.iter().find(|s| s.titulo.contains("Isentar")).unwrap();
        assert_eq!(s63.ocorrencias.len(), 2, "63 serve PF e PJ");
        assert!(s63.ocorrencias.iter().all(|o| o.classe == "IPVA e Veículos"));
        let s882 = items.iter().find(|s| s.titulo.contains("Inscrição")).unwrap();
        assert_eq!(s882.ocorrencias.len(), 1);
        assert_eq!(s882.ocorrencias[0].publico, "Pessoa Jurídica");
        // clean() comprime os espaços do título.
        assert_eq!(s882.titulo, "Pedir Inscrição Estadual");
    }

    #[test]
    fn build_pula_servico_sem_publico() {
        let todos = leaves(&parse_items(FLIGHT_TODOS).unwrap());
        let perfis: Vec<(&str, HashSet<i64>)> =
            vec![("Pessoa Física", HashSet::from([63]))]; // 882 e 900 sem público
        let items = build_servicos(&todos, &perfis);
        assert_eq!(items.len(), 1, "só o 63 tem público");
        assert_eq!(items[0].ocorrencias.len(), 1);
    }

    #[test]
    fn dedup_por_id() {
        // O mesmo id repetido na árvore vira um único serviço.
        let todos = vec![
            Leaf { id: 5, name: "A".into(), description: "d".into(), url: "/x".into(), categoria: "C".into() },
            Leaf { id: 5, name: "A dup".into(), description: "d".into(), url: "/x".into(), categoria: "C".into() },
        ];
        let perfis: Vec<(&str, HashSet<i64>)> = vec![("Pessoa Física", HashSet::from([5]))];
        assert_eq!(build_servicos(&todos, &perfis).len(), 1);
    }

    #[test]
    fn link_absolutiza_e_tira_profile() {
        assert_eq!(
            link("/portfolio-servicos/detalhes/882?profile=todos"),
            "https://www.sefaz.am.gov.br/portfolio-servicos/detalhes/882"
        );
        // externo fica como está.
        assert_eq!(link("https://buscapreco.sefaz.am.gov.br/home"), "https://buscapreco.sefaz.am.gov.br/home");
        // submenu relativo é absolutizado.
        assert_eq!(link("/submenu/554"), "https://www.sefaz.am.gov.br/submenu/554");
    }

    #[test]
    fn balanced_respeita_strings() {
        // colchete dentro de string não desbalanceia.
        let s = r#"[{"a":"]["}] resto"#;
        assert_eq!(balanced(s).unwrap(), r#"[{"a":"]["}]"#);
    }

    #[test]
    fn ids_por_rota() {
        assert_eq!(ids(FLIGHT_TODOS), HashSet::from([63, 882, 900]));
    }

    #[test]
    fn validar_reprova_capado() {
        let poucos = vec![ServicoRaw {
            titulo: "x".into(),
            descricao: String::new(),
            link: "l".into(),
            orgao: ORGAO.into(),
            ocorrencias: vec![],
        }];
        assert!(validar(&poucos).unwrap_err().to_string().contains("capado"));
    }
}
