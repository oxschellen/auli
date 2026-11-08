// SEFIN-RO FAQs — as perguntas das fichas da Agência Virtual (Sydle ONE conecta-360). Não há página de FAQ: depois das
// 6 seções do modelo da ficha (O que é?, Legislação, Etapas e prazos…), muitas fichas trazem seções cujo título é uma
// pergunta. Estudo, medições e decisões: docs/faqs/ESTUDO-RO-PI-Faqs.md (D-SYDFAQ-*).
//
// Um `_search` traz os artigos inteiros (o `contentMarkdown`, no molde das FAQs do CE — `kit::sydle`) e os catálogos (o
// identificador de cada um entra no link). Pergunta = seção cujo título termina em "?". As seções que agrupam perguntas
// ("… Perguntas e respostas") são segmentadas por dentro: pelos rótulos "Pergunta:"/"Resposta:" quando os há, senão
// pelo `kit::linhas` (D-SYDFAQ-1, opção B).

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::LazyLock;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use regex::Regex;
use scraper::Html;
use serde::Deserialize;

use auli_contract::FaqRaw;
use auli_scraper_kit::faqtree::{FaqItem, FaqNode, PageType};
use auli_scraper_kit::html::{Opcoes, html_para_texto};
use auli_scraper_kit::http::GetOpts;
use auli_scraper_kit::linhas::{linhas, segmentar};
use auli_scraper_kit::{clean, sydle};

use crate::ro;

/// A Agência Virtual: base dos links (`<catálogo>+<identifier>+<_id>`) e dos caminhos da raiz no texto.
const SITE: &str = "https://agenciavirtual.sefin.ro.gov.br";
/// As classes do `_search`: os artigos (as fichas e os conteúdos) e os catálogos.
const CLASSE_ARTIGO: &str = "5d66ed07e7c98a556a499a36";
const CLASSE_CATALOGO: &str = "5ca3bca7d18bdb280c9c6c2c";
/// Teto de documentos numa GET (229 em 07/11/2026: 224 artigos e 5 catálogos).
const SIZE: usize = 500;
const CORTESIA: Duration = Duration::from_secs(1);
/// O corpo do `_search`: 21 MB em 07/11/2026 (as imagens dos artigos vêm em base64); o `ureq` para em 10 MB.
const LIMITE: u64 = 64 * 1024 * 1024;
/// As seções do modelo da ficha; "O que é?" é a única que termina em "?".
const FICHA: [&str; 6] = [
    "O que é?",
    "Legislação",
    "Etapas e prazos",
    "Onde e como solicitar",
    "Documentação",
    "Taxa",
];
/// Piso: ~85 % das 213 perguntas de 07/11/2026.
const PISO: usize = 180;

/// A seção que agrupa perguntas ("DFe-Download – Perguntas e respostas", "Perguntas e Respostas").
static GRUPO_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\bperguntas\b").unwrap());
/// Os rótulos da Substituição Tributária: "1. Pergunta:" e "Resposta:" (com o texto na mesma linha ou não).
static ROTULO_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^\d+\s*[.)–-]?\s*Pergunta\s*:\s*$").unwrap());
static RESPOSTA_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^Resposta\s*:\s*(.*)$").unwrap());

/// Coleta as FAQs e grava a árvore do frontend (`<id>-faqs-tree.json`) e o snapshot. Tudo é conferido antes de
/// qualquer gravação: rodada que falha deixa os arquivos anteriores intactos.
pub fn run(data_dir: &str, use_cache: bool) -> Result<()> {
    let chave = format!("{}#faqs", ro::SEARCH_URL);
    let json = if use_cache {
        auli_scraper_kit::cache::read(data_dir, "faqs", &chave)
            .ok_or_else(|| anyhow!("cache miss para {chave} (modo --usecache, sem rede)"))?
    } else {
        let agent = auli_scraper_kit::build_agent(ro::USER_AGENT, Some(Duration::from_secs(120)));
        let token = ro::fetch_token(&agent)?;
        sleep(CORTESIA);
        let corpo = buscar(&agent, &token)?;
        auli_scraper_kit::cache::write(data_dir, "faqs", &chave, &corpo);
        corpo
    };
    let docs = documentos(&json)?;
    let perguntas = perguntas(&docs)?;
    let lidas = perguntas.len();
    let perguntas = sem_copias(perguntas);
    piso(perguntas.len())?;
    sem_colisao(&perguntas)?;
    let artigos = perguntas
        .iter()
        .map(|p| &p.id)
        .collect::<HashSet<_>>()
        .len();
    println!(
        "RO FAQs: {} perguntas de {artigos} fichas — {lidas} lidas, {} cópias saíram",
        perguntas.len(),
        lidas - perguntas.len()
    );

    // Atômico pelo mesmo motivo das outras entidades: a árvore não tem outra fonte (o snapshot achatado não a
    // reconstrói) e é o único artefato de coleta que o frontend lê direto.
    let tree_path = format!("{data_dir}/{}-faqs-tree.json", crate::ENTITY);
    auli_contract::arquivo::escrever_atomico(
        Path::new(&tree_path),
        &serde_json::to_string_pretty(&montar_arvore(&perguntas))?,
    )?;
    println!("Wrote {tree_path} (árvore de FAQ p/ o frontend)");

    auli_contract::snapshot::write_faqs(
        crate::ENTITY,
        data_dir,
        &auli_contract::ScraperInfo::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
        consolidar(&perguntas),
    )?;
    println!("🎉 Coleta de FAQs gravada no snapshot.");
    Ok(())
}

/// Coleta capada (o `_search` ou o molde que mudou) é `Err`, não snapshot menor.
fn piso(n: usize) -> Result<()> {
    if n < PISO {
        bail!("RO FAQs: {n} perguntas (piso {PISO}) — o portal mudou?");
    }
    Ok(())
}

/// O `_search` dos artigos e catálogos ativos (GET, o corpo ES em `?_body=`).
fn buscar(agent: &ureq::Agent, token: &str) -> Result<String> {
    let corpo = serde_json::json!({
        "query": { "bool": {
            "must": [{ "term": { "active": true } }],
            "should": [
                { "term": { "_class._id": CLASSE_ARTIGO } },
                { "term": { "_class._id": CLASSE_CATALOGO } }
            ],
            "minimum_should_match": 1
        } },
        "size": SIZE
    })
    .to_string();
    let url = format!("{}?_body={}", ro::SEARCH_URL, ro::pct_encode(&corpo));
    println!("GET _search (artigos e catálogos): {}", ro::SEARCH_URL);
    auli_scraper_kit::http::get_string(
        agent,
        &url,
        &GetOpts {
            log_prefix: "RO FAQs",
            accept: Some("application/json"),
            headers: &[("Authorization", token)],
            limite_bytes: Some(LIMITE),
            ..Default::default()
        },
    )
}

#[derive(Deserialize)]
struct Resposta {
    hits: Hits,
}

#[derive(Deserialize)]
struct Hits {
    total: usize,
    hits: Vec<Hit>,
}

#[derive(Deserialize)]
struct Hit {
    #[serde(rename = "_source")]
    doc: Doc,
}

/// Um documento do `_search`. Só os campos que usamos.
#[derive(Deserialize)]
struct Doc {
    #[serde(rename = "_id")]
    id: String,
    #[serde(rename = "_class")]
    classe: Ref,
    #[serde(default)]
    identifier: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    parent: Option<Ref>,
    #[serde(rename = "contentFormat", default)]
    formato: Option<String>,
    #[serde(rename = "contentMarkdown", default)]
    markdown: Option<String>,
}

#[derive(Deserialize)]
struct Ref {
    #[serde(rename = "_id")]
    id: String,
}

/// Os documentos, com a guarda de cobertura: o `_search` trouxe todos os que achou, abaixo do teto.
fn documentos(json: &str) -> Result<Vec<Doc>> {
    let r: Resposta = serde_json::from_str(json).context("JSON do _search inválido")?;
    let n = r.hits.hits.len();
    if r.hits.total != n || n >= SIZE {
        bail!(
            "o _search achou {} documentos e trouxe {n} (teto {SIZE}) — aumente o SIZE",
            r.hits.total
        );
    }
    Ok(r.hits.hits.into_iter().map(|h| h.doc).collect())
}

struct Pergunta {
    /// O `_id` da ficha (as cópias ficam na de menor `_id`).
    id: String,
    pergunta: String,
    resposta: String,
    /// O nome da ficha — a trilha.
    artigo: String,
    url_artigo: String,
    /// A ficha com a âncora da seção.
    url: String,
}

/// As perguntas de todas as fichas, em ordem alfabética das fichas e, dentro delas, na ordem das seções.
fn perguntas(docs: &[Doc]) -> Result<Vec<Pergunta>> {
    let catalogos: HashMap<&str, &str> = docs
        .iter()
        .filter(|d| d.classe.id == CLASSE_CATALOGO)
        .filter_map(|d| Some((d.id.as_str(), d.identifier.as_deref()?)))
        .collect();
    let mut artigos: Vec<&Doc> = docs
        .iter()
        .filter(|d| {
            d.classe.id == CLASSE_ARTIGO
                && d.formato.as_deref() == Some("MARKDOWN")
                && d.markdown.as_deref().unwrap_or("").contains(sydle::BLOCO)
        })
        .collect();
    let nome = |d: &Doc| clean(d.name.as_deref().unwrap_or(""));
    artigos.sort_by(|a, b| (nome(a), &a.id).cmp(&(nome(b), &b.id)));

    let mut out = Vec::new();
    for a in artigos {
        let artigo = nome(a);
        let ps = do_artigo(a, &artigo, &catalogos)
            .with_context(|| format!("ficha {} ({artigo})", a.id))?;
        out.extend(ps);
    }
    Ok(out)
}

/// As perguntas de uma ficha: as seções-pergunta e as de dentro das seções que agrupam perguntas.
fn do_artigo(a: &Doc, artigo: &str, catalogos: &HashMap<&str, &str>) -> Result<Vec<Pergunta>> {
    let secoes = sydle::secoes(a.markdown.as_deref().unwrap_or(""), SITE)?;
    let mut out = Vec::new();
    let mut url_artigo = None;
    for s in secoes {
        if FICHA.contains(&s.titulo.as_str()) {
            continue;
        }
        let pares = if s.titulo.ends_with('?') {
            if s.texto.is_empty() {
                bail!("a pergunta {:?} não tem resposta", s.titulo);
            }
            vec![(s.titulo.clone(), s.texto.clone())]
        } else if GRUPO_RE.is_match(&s.titulo) {
            internas(&s).with_context(|| format!("seção {:?}", s.titulo))?
        } else {
            continue;
        };
        if url_artigo.is_none() {
            url_artigo = Some(link(a, catalogos)?);
        }
        let url_artigo = url_artigo.clone().unwrap_or_default();
        for (pergunta, resposta) in pares {
            out.push(Pergunta {
                id: a.id.clone(),
                url: format!("{url_artigo}#section-{}", s.slug),
                pergunta,
                resposta,
                artigo: artigo.to_string(),
                url_artigo: url_artigo.clone(),
            });
        }
    }
    Ok(out)
}

/// A ficha na Agência Virtual: `<catálogo>+<identifier>+<_id>`, como os links dos serviços e do índice A-Z.
fn link(a: &Doc, catalogos: &HashMap<&str, &str>) -> Result<String> {
    let pai = a.parent.as_ref().map(|p| p.id.as_str()).unwrap_or("");
    let catalogo = catalogos
        .get(pai)
        .ok_or_else(|| anyhow!("a ficha está fora de um catálogo conhecido (parent {pai:?})"))?;
    Ok(match a.identifier.as_deref().map(str::trim) {
        Some(i) if !i.is_empty() => format!("{SITE}/{catalogo}+{i}+{}", a.id),
        _ => format!("{SITE}/{catalogo}+{}", a.id),
    })
}

/// As perguntas de dentro de uma seção: pelos rótulos "Pergunta:"/"Resposta:" quando os há, senão pelo
/// `kit::linhas` (pergunta = linha em destaque terminada em "?"). Seção sem pergunta nenhuma (o "Perguntas e
/// Respostas" da Ouvidoria só manda para o SAC) dá nenhuma.
fn internas(s: &sydle::Secao) -> Result<Vec<(String, String)>> {
    if s.texto.lines().any(|l| ROTULO_RE.is_match(l)) {
        return rotulos(&s.texto);
    }
    let doc = Html::parse_fragment(&s.html);
    let mut out = Vec::new();
    for b in segmentar(&linhas(doc.root_element())) {
        let resposta = html_para_texto(
            &b.resposta_html,
            Opcoes {
                base: Some(SITE),
                quebras_do_fonte: false,
            },
        );
        if resposta.is_empty() {
            bail!("a pergunta {:?} ficou sem resposta", b.titulo);
        }
        out.push((b.titulo, resposta));
    }
    Ok(out)
}

/// "1. Pergunta:", a pergunta, "Resposta:" e a resposta, até o próximo rótulo. O que vem antes do 1º rótulo (a
/// introdução) sai.
fn rotulos(texto: &str) -> Result<Vec<(String, String)>> {
    let mut pares: Vec<(Vec<&str>, Vec<&str>, bool)> = Vec::new();
    for l in texto.lines() {
        if ROTULO_RE.is_match(l) {
            pares.push((Vec::new(), Vec::new(), false));
            continue;
        }
        let Some((p, r, na_resposta)) = pares.last_mut() else {
            continue;
        };
        if *na_resposta {
            r.push(l);
        } else if let Some(c) = RESPOSTA_RE.captures(l) {
            *na_resposta = true;
            let resto = c.get(1).map_or("", |m| m.as_str()).trim();
            if !resto.is_empty() {
                r.push(resto);
            }
        } else {
            p.push(l);
        }
    }
    pares
        .into_iter()
        .map(|(p, r, _)| {
            let (p, r) = (p.join(" "), r.join("\n"));
            if p.is_empty() || r.is_empty() {
                bail!("rótulo \"Pergunta:\" sem pergunta ou sem resposta ({p:?})");
            }
            Ok((p, r))
        })
        .collect()
}

/// O mesmo par em mais de uma ficha (as irmãs repetem perguntas: 19 cópias em 07/11/2026) fica só na de menor `_id`,
/// na primeira vez (D-SYDFAQ-3, como o D-CEFAQ-5). Quebras de linha e espaços não contam.
fn sem_copias(perguntas: Vec<Pergunta>) -> Vec<Pergunta> {
    let chave = |p: &Pergunta| (normal(&p.pergunta), normal(&p.resposta));
    let mut menor: HashMap<(String, String), String> = HashMap::new();
    for p in &perguntas {
        let id = menor.entry(chave(p)).or_insert_with(|| p.id.clone());
        if p.id < *id {
            *id = p.id.clone();
        }
    }
    let mut vistas = HashSet::new();
    perguntas
        .into_iter()
        .filter(|p| menor[&chave(p)] == p.id && vistas.insert(chave(p)))
        .collect()
}

fn normal(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// O nome do `.md` sai de url + pergunta: a mesma pergunta na mesma âncora, com respostas diferentes, faria um
/// sobrescrever o outro.
fn sem_colisao(perguntas: &[Pergunta]) -> Result<()> {
    let mut vistas = HashSet::new();
    for p in perguntas {
        if !vistas.insert((p.url.as_str(), p.pergunta.as_str())) {
            bail!(
                "{} traz {:?} duas vezes, com respostas diferentes",
                p.url,
                p.pergunta
            );
        }
    }
    Ok(())
}

fn consolidar(perguntas: &[Pergunta]) -> Vec<FaqRaw> {
    perguntas
        .iter()
        .map(|p| FaqRaw {
            pergunta: p.pergunta.clone(),
            resposta: p.resposta.clone(),
            origin: p.artigo.clone(),
            url: p.url.clone(),
        })
        .collect()
}

/// A árvore da aba de FAQs: raiz → fichas → perguntas. A interface começa pelos filhos da raiz. O nó da ficha aponta
/// para a ficha; cada pergunta, para a sua seção.
fn montar_arvore(perguntas: &[Pergunta]) -> FaqNode {
    let mut fichas: Vec<Vec<&Pergunta>> = Vec::new();
    for p in perguntas {
        match fichas.last_mut() {
            Some(grupo) if grupo[0].id == p.id => grupo.push(p),
            _ => fichas.push(vec![p]),
        }
    }
    FaqNode {
        title: "Perguntas frequentes".to_string(),
        url: format!("{SITE}/"),
        page_type: PageType::Menu,
        origin: String::new(),
        children: fichas
            .iter()
            .map(|grupo| FaqNode {
                title: grupo[0].artigo.clone(),
                url: grupo[0].url_artigo.clone(),
                page_type: PageType::Faq,
                origin: grupo[0].artigo.clone(),
                children: Vec::new(),
                faq_items: grupo
                    .iter()
                    .map(|p| FaqItem {
                        pergunta: p.pergunta.clone(),
                        resposta: p.resposta.clone(),
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

    // O `_search` real de 07/11/2026, só com os campos que o scraper lê e as imagens base64 esvaziadas: os catálogos
    // Serviços e Conteúdos; as fichas Petição ao núcleo de cadastro (15 seções-pergunta), Arquivos XML (o grupo
    // "DFe-Download – Perguntas e respostas", 10 em `<h3>`), Substituição Tributária (o grupo com os rótulos
    // "Pergunta:"/"Resposta:", 10), as duas Emissões de DARE (a mesma pergunta nas duas), as Isenções do Taxista e
    // da PcD ("Quando o benefício pode ser utilizado?" com respostas diferentes), Ouvidoria (um "Perguntas e
    // Respostas" sem pergunta), Temas e Categorias do SAC (no catálogo Conteúdos) e dois artigos em HTML (a UPF com
    // blocos sem título).
    const BUSCA: &str = include_str!("../tests/fixtures/ro-faqs-search.json");
    const DARE_PARCIAL: &str = "663da03eee982159b74e4c25";
    const OUVIDORIA: &str = "673cafa761811c105c8e6c96";
    const SERVICOS: &str = "662c1875ee982159b7b199c9";

    fn erro<T>(r: Result<T>) -> String {
        match r {
            Ok(_) => panic!("esperava Err"),
            Err(e) => format!("{e:#}"),
        }
    }

    fn fixture() -> Vec<Pergunta> {
        sem_copias(perguntas(&documentos(BUSCA).unwrap()).unwrap())
    }

    fn da_ficha<'a>(ps: &'a [Pergunta], prefixo: &str) -> Vec<&'a Pergunta> {
        ps.iter()
            .filter(|p| p.artigo.starts_with(prefixo))
            .collect()
    }

    /// Uma ficha sintética no catálogo Serviços.
    fn ficha(md: &str) -> Doc {
        Doc {
            id: "f1".into(),
            classe: Ref {
                id: CLASSE_ARTIGO.into(),
            },
            identifier: Some("ficha".into()),
            name: Some("Ficha".into()),
            parent: Some(Ref {
                id: SERVICOS.into(),
            }),
            formato: Some("MARKDOWN".into()),
            markdown: Some(md.into()),
        }
    }

    fn bloco(slug: &str, titulo: &str, corpo: &str) -> String {
        format!(
            "{}\n<div class=\"heading\" id=\"section-{slug}\"><h2>{titulo}</h2></div>\n\
             <sy-collapse class=\"hydrated collapse service-section\" trigger-by=\"#section-{slug}\">\n\
             <div class=\"content\">{corpo}</div>\n</div>\n",
            sydle::BLOCO
        )
    }

    fn catalogos() -> HashMap<&'static str, &'static str> {
        HashMap::from([(SERVICOS, "catalogo-servicos")])
    }

    #[test]
    fn secoes_pergunta_das_fichas_sem_as_da_ficha() {
        let ps = fixture();
        assert_eq!(ps.len(), 53);
        assert_eq!(da_ficha(&ps, "Petição ao núcleo").len(), 15);
        assert_eq!(da_ficha(&ps, "Isenção do ICMS na aquisição").len(), 11);
        assert_eq!(da_ficha(&ps, "Isenção do ICMS/IPVA para Taxista").len(), 5);
        assert!(ps.iter().all(|p| p.pergunta != "O que é?"));
        // O artigo em HTML fica fora, e o "Perguntas e Respostas" da Ouvidoria não tem pergunta.
        assert!(ps.iter().all(|p| p.id != OUVIDORIA));
        assert!(ps.iter().all(|p| !p.artigo.starts_with("Cronograma")));
    }

    #[test]
    fn grupo_com_rotulos_pergunta_e_resposta() {
        let ps = fixture();
        let st = da_ficha(&ps, "Substituição Tributária");
        assert_eq!(st.len(), 10);
        assert_eq!(st[0].pergunta, "O que é a substituição tributária?");
        assert!(
            st[0]
                .resposta
                .starts_with("A substituição tributária é um regime")
        );
        assert!(st[0].url.ends_with("#section-perguntas-e-respostas"));
        // A introdução (o caminho do SAC) não vira resposta.
        assert!(
            st.iter()
                .all(|p| !p.resposta.contains("Serviço de Atendimento ao Cidadão"))
        );
    }

    #[test]
    fn grupo_pelo_kit_linhas() {
        let ps = fixture();
        let dfe = da_ficha(&ps, "Solicitação de Arquivos XML");
        assert_eq!(dfe.len(), 10);
        assert_eq!(dfe[0].pergunta, "1. O que mudou no DFe-Download?");
        assert!(
            dfe[0]
                .resposta
                .starts_with("Foi implantado um novo mecanismo")
        );
    }

    #[test]
    fn copia_fica_na_ficha_de_menor_id_e_a_outra_resposta_fica() {
        let ps = fixture();
        let dare: Vec<_> = ps
            .iter()
            .filter(|p| p.pergunta == "O que é um DARE e para que serve?")
            .collect();
        assert_eq!(dare.len(), 1);
        assert_eq!(dare[0].id, DARE_PARCIAL);
        let quando: Vec<_> = ps
            .iter()
            .filter(|p| p.pergunta == "Quando o benefício pode ser utilizado?")
            .collect();
        assert_eq!(quando.len(), 2);
        assert_ne!(quando[0].resposta, quando[1].resposta);
    }

    #[test]
    fn link_pelo_catalogo_com_a_ancora_da_secao() {
        let ps = fixture();
        let sac = da_ficha(&ps, "Temas e Categorias do SAC");
        assert_eq!(sac.len(), 1);
        assert!(sac[0].url.starts_with(&format!(
            "{SITE}/catalogo-conteudos+temas-e-categorias-do-sac+"
        )));
        assert!(sac[0].url.contains("#section-existe-algum-suporte-tecnico"));
        let p = &da_ficha(&ps, "Petição ao núcleo")[0];
        assert!(p.url_artigo.starts_with(&format!(
            "{SITE}/catalogo-servicos+peticao-ao-nucleo-de-cadastro+"
        )));
    }

    #[test]
    fn arvore_raiz_fichas_perguntas() {
        let ps = fixture();
        let t = montar_arvore(&ps);
        assert_eq!(t.children.len(), 7);
        let f = &t.children[0];
        assert_eq!(f.title, ps[0].artigo);
        assert_eq!(f.url, ps[0].url_artigo);
        assert_eq!(f.faq_items[0].url.as_deref(), Some(ps[0].url.as_str()));
        assert_eq!(
            t.children.iter().map(|c| c.faq_items.len()).sum::<usize>(),
            53
        );
    }

    #[test]
    fn rotulo_sem_resposta_e_erro() {
        assert!(erro(rotulos("Intro\n1. Pergunta:\nX?\n")).contains("sem resposta"));
        let ok =
            rotulos("1. Pergunta:\nX?\nResposta: Sim.\n2. Pergunta:\nY?\nResposta:\nNão.").unwrap();
        assert_eq!(
            ok,
            vec![("X?".into(), "Sim.".into()), ("Y?".into(), "Não.".into())]
        );
    }

    #[test]
    fn pergunta_sem_resposta_e_erro() {
        let a = ficha(&bloco("p", "Pode?", "&nbsp;"));
        assert!(erro(do_artigo(&a, "Ficha", &catalogos())).contains("não tem resposta"));
    }

    #[test]
    fn ficha_fora_de_catalogo_conhecido_e_erro() {
        let a = ficha(&bloco("p", "Pode?", "Sim."));
        assert!(erro(do_artigo(&a, "Ficha", &HashMap::new())).contains("fora de um catálogo"));
        // Sem pergunta, o catálogo nem é consultado.
        let b = ficha(&bloco("t", "Taxa", "Não há."));
        assert!(do_artigo(&b, "Ficha", &HashMap::new()).unwrap().is_empty());
    }

    #[test]
    fn artigo_em_html_fica_fora_mesmo_com_blocos() {
        // A UPF é HTML com blocos sem título: lida, derrubaria a coleta.
        let docs = documentos(BUSCA).unwrap();
        let upf = docs
            .iter()
            .find(|d| d.id == "669e6b43fe9b6953faf6969e")
            .unwrap();
        assert_eq!(upf.formato.as_deref(), Some("HTML"));
        assert!(upf.markdown.as_deref().unwrap().contains(sydle::BLOCO));
        assert!(do_artigo(upf, "UPF", &catalogos()).is_err());
        assert!(perguntas(&docs).is_ok());
    }

    #[test]
    fn copia_ignora_espacos_e_quebras() {
        let p = |id: &str, r: &str| Pergunta {
            id: id.into(),
            pergunta: "P?".into(),
            resposta: r.into(),
            artigo: id.into(),
            url_artigo: id.into(),
            url: format!("{id}#s"),
        };
        let ps = sem_copias(vec![
            p("b", "Uma  frase\ncortada."),
            p("a", "Uma frase cortada."),
        ]);
        assert_eq!(ps.len(), 1);
        assert_eq!(ps[0].id, "a");
    }

    #[test]
    fn pergunta_de_grupo_sem_resposta_e_erro() {
        let a = ficha(&bloco(
            "g",
            "Perguntas e respostas",
            "<h3>1. Primeira?</h3><p>R1.</p><h3>2. Segunda?</h3>",
        ));
        assert!(erro(do_artigo(&a, "Ficha", &catalogos())).contains("sem resposta"));
    }

    #[test]
    fn busca_incompleta_e_erro() {
        assert!(erro(documentos(r#"{"hits":{"total":3,"hits":[]}}"#)).contains("achou 3"));
    }

    #[test]
    fn colisao_e_piso() {
        let p = |r: &str| Pergunta {
            id: "a".into(),
            pergunta: "P?".into(),
            resposta: r.into(),
            artigo: "A".into(),
            url_artigo: "u".into(),
            url: "u#s".into(),
        };
        assert!(sem_colisao(&[p("R1")]).is_ok());
        assert!(erro(sem_colisao(&[p("R1"), p("R2")])).contains("duas vezes"));
        assert!(piso(PISO).is_ok());
        assert!(erro(piso(PISO - 1)).contains("piso"));
    }
}
