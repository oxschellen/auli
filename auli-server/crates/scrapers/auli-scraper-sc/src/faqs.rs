// SEF-SC FAQs — "Perguntas frequentes" (a Base de Conhecimento da CAF), servidas pelo mesmo portal
// Next.js do `sc.rs`. Estudo, medições e decisões: docs/ESTUDO-SC-Faqs.md (D-SCFAQ-*).
//
// A fonte é `/_next/data/<buildId>/perguntas/buscar.json?assunto=<raiz>`, uma chamada por raiz da
// árvore de assuntos (`perguntas.json` → `assuntosBusca`). Três propriedades da fonte moldam o código:
//   - `assunto=<nó intermediário>` devolve cada descendente em dobro → dedup por
//     `BaseConhecimentoID` dentro de cada raiz (`consolidar`);
//   - a árvore do JSON tem só 2 níveis; o 3º só aparece no `Assunto` de cada pergunta → o mapa de
//     assuntos é a árvore completada pelas perguntas (`completar_mapa`);
//   - `Questao`/`Resposta` vêm ora em HTML, ora em texto puro com `\n` → `html_para_texto`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::LazyLock;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use regex::Regex;
use serde::Deserialize;
use ureq::Agent;

use auli_contract::FaqRaw;
use auli_scraper_kit::faqtree::{FaqItem, FaqNode, PageType};
use auli_scraper_kit::http::GetOpts;

use crate::pareceres::{COURTESY, UA};
use crate::sc::NextData;

const BASE: &str = "https://www.sef.sc.gov.br";

/// Coleta as FAQs e grava a árvore do frontend (`<id>-faqs-tree.json`) e o snapshot. As guardas do
/// `consolidar` rodam antes de qualquer gravação: rodada que falha deixa os arquivos anteriores intactos.
pub fn run(data_dir: &str, use_cache: bool) -> Result<()> {
    let agent = auli_scraper_kit::build_agent(UA, Some(Duration::from_secs(60)));

    // O HTML só serve para o `buildId` (muda a cada deploy do portal).
    let pagina = format!("{BASE}/perguntas");
    let html = buscar(data_dir, &agent, &pagina, &pagina, use_cache)?;
    let build_id = crate::sc::parse_build_id(&html).map_err(|e| anyhow!("{e}"))?;
    println!("SC FAQs: buildId {build_id}");

    let arvore: NextData<ArvoreApi> = serde_json::from_str(&buscar(
        data_dir,
        &agent,
        &format!("{BASE}/perguntas.json"),
        &format!("{BASE}/_next/data/{build_id}/perguntas.json"),
        use_cache,
    )?)?;
    let raizes = arvore.page_props.resposta_api.assuntos_busca;
    println!("SC FAQs: {} raízes na árvore de assuntos", raizes.len());

    let mut buscas = Vec::new();
    for raiz in &raizes {
        let corpo = buscar(
            data_dir,
            &agent,
            &format!("{BASE}/perguntas/buscar?assunto={}", raiz.id),
            &format!(
                "{BASE}/_next/data/{build_id}/perguntas/buscar.json?assunto={}",
                raiz.id
            ),
            use_cache,
        )?;
        let busca: NextData<BuscaApi> = serde_json::from_str(&corpo)?;
        buscas.push((raiz.id, busca.page_props.resposta_api));
    }

    let consolidado = consolidar(&raizes, buscas)?;
    if consolidado.fora_da_visibilidade > 0 {
        eprintln!(
            "⚠️  SC FAQs: {} pergunta(s) com Visibilidade ≠ 2 — pode ser conteúdo interno; revise \
             antes de publicar (D-SCFAQ-3e)",
            consolidado.fora_da_visibilidade
        );
    }
    let faqs: Vec<FaqRaw> = consolidado
        .itens
        .iter()
        .map(|(p, t)| montar_faq_raw(p, t))
        .collect();

    // Atômico pelo mesmo motivo do RS: a árvore não tem outra fonte (o snapshot achatado não a
    // reconstrói) e é o único artefato de coleta que o frontend lê direto.
    let tree_path = format!("{data_dir}/{}-faqs-tree.json", crate::ENTITY);
    auli_contract::arquivo::escrever_atomico(
        Path::new(&tree_path),
        &serde_json::to_string_pretty(&montar_arvore(&raizes, &consolidado))?,
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

/// Uma requisição (D-SCFAQ-4): rede sempre, gravando o cache sob a URL lógica (sem `buildId`); com
/// `--usecache`, só o cache, e miss é erro. Não é o `fetch_cached` do `sc.rs`, que é cache-first: com
/// 33 requisições não há o que economizar, e um cache velho reproduziria a coleta antiga em silêncio.
fn buscar(
    data_dir: &str,
    agent: &Agent,
    logical: &str,
    url: &str,
    use_cache: bool,
) -> Result<String> {
    if use_cache {
        return auli_scraper_kit::cache::read(data_dir, "faqs", logical)
            .ok_or_else(|| anyhow!("cache miss para {logical} (modo --usecache, sem rede)"));
    }
    sleep(COURTESY);
    let corpo = auli_scraper_kit::http::get_string(
        agent,
        url,
        &GetOpts {
            log_prefix: "SC FAQs",
            ..Default::default()
        },
    )?;
    auli_scraper_kit::cache::write(data_dir, "faqs", logical, &corpo);
    Ok(corpo)
}

// --- API response shapes (só os campos usados: `IconeBase64` traz um PNG inteiro e os `Exige*` são
//     da abertura de chamado da CAF — ficam de fora de propósito) ---

/// `perguntas.json`: a árvore de assuntos.
#[derive(Deserialize)]
struct ArvoreApi {
    #[serde(rename = "assuntosBusca")]
    assuntos_busca: Vec<AssuntoNo>,
}

#[derive(Deserialize)]
struct AssuntoNo {
    #[serde(rename = "AssuntoID")]
    id: i64,
    #[serde(rename = "Nome")]
    nome: String,
    #[serde(rename = "Subassuntos", default)]
    subassuntos: Vec<AssuntoNo>,
}

/// `perguntas/buscar.json?assunto=<id>`: as perguntas da subárvore do assunto.
#[derive(Deserialize)]
struct BuscaApi {
    #[serde(default)]
    perguntas: Vec<Pergunta>,
    #[serde(default)]
    buscar: bool,
    #[serde(rename = "errorCode", default)]
    error_code: Option<serde_json::Value>,
}

#[derive(Deserialize, Clone)]
struct Pergunta {
    #[serde(rename = "BaseConhecimentoID")]
    id: i64,
    #[serde(rename = "Questao", default)]
    questao: Option<String>,
    #[serde(rename = "Resposta", default)]
    resposta: Option<String>,
    #[serde(rename = "Visibilidade", default)]
    visibilidade: i64,
    #[serde(rename = "Assunto")]
    assunto: AssuntoRef,
}

#[derive(Deserialize, Clone)]
struct AssuntoRef {
    #[serde(rename = "AssuntoID")]
    id: i64,
    #[serde(rename = "Nome")]
    nome: String,
    #[serde(rename = "SuperiorHierarquicoID", default)]
    pai: Option<i64>,
}

/// AssuntoID → (Nome, pai). A árvore dá 2 níveis; o 3º entra pelas perguntas (`completar_mapa`).
type Mapa = BTreeMap<i64, (String, Option<i64>)>;

fn mapa_da_arvore(raizes: &[AssuntoNo]) -> Mapa {
    fn visitar(no: &AssuntoNo, pai: Option<i64>, mapa: &mut Mapa) {
        mapa.insert(no.id, (no.nome.clone(), pai));
        for filho in &no.subassuntos {
            visitar(filho, Some(no.id), mapa);
        }
    }
    let mut mapa = Mapa::new();
    for raiz in raizes {
        visitar(raiz, None, &mut mapa);
    }
    mapa
}

/// Acrescenta ao mapa os assuntos que só as perguntas conhecem (o 3º nível). Assunto que já está na
/// árvore fica como a árvore diz.
fn completar_mapa(mapa: &mut Mapa, perguntas: &[Pergunta]) {
    for p in perguntas {
        mapa.entry(p.assunto.id)
            .or_insert_with(|| (p.assunto.nome.clone(), p.assunto.pai));
    }
}

/// Trilha raiz → assunto (`A | B | C`), subindo por `SuperiorHierarquicoID`. Elo faltando é `Err`
/// (D-SCFAQ-3c): a pergunta ficaria sem `origin`.
fn trilha(mapa: &Mapa, id: i64) -> Result<String> {
    let mut nomes = Vec::new();
    let mut atual = Some(id);
    while let Some(i) = atual {
        let Some((nome, pai)) = mapa.get(&i) else {
            bail!("assunto {i} fora do mapa de assuntos — a cadeia de pais de {id} não resolve");
        };
        if nomes.len() > mapa.len() {
            bail!("ciclo na cadeia de pais do assunto {id}");
        }
        nomes.push(nome.trim());
        atual = *pai;
    }
    nomes.reverse();
    Ok(nomes.join(" | "))
}

/// `assunto=<nó intermediário>` devolve cada descendente em dobro: fica a 1ª ocorrência de cada
/// `BaseConhecimentoID`, na ordem da fonte.
fn dedup(perguntas: Vec<Pergunta>) -> Vec<Pergunta> {
    let mut vistas = BTreeSet::new();
    perguntas
        .into_iter()
        .filter(|p| vistas.insert(p.id))
        .collect()
}

/// O acervo consolidado: as perguntas únicas na ordem das raízes, cada uma com a sua trilha, e o mapa
/// de assuntos completo (para a árvore do frontend).
struct Consolidado {
    itens: Vec<(Pergunta, String)>,
    mapa: Mapa,
    /// Perguntas com `Visibilidade != 2` — aviso, não erro (D-SCFAQ-3e).
    fora_da_visibilidade: usize,
}

/// Junta as respostas do `buscar` (uma por raiz, na ordem de `raizes`) e aplica as guardas D-SCFAQ-3.
fn consolidar(raizes: &[AssuntoNo], buscas: Vec<(i64, BuscaApi)>) -> Result<Consolidado> {
    let mut mapa = mapa_da_arvore(raizes);

    // (a) `buscar:false` é parâmetro não reconhecido e `errorCode` é backend que falhou — nunca "vazio".
    let mut por_raiz = Vec::new();
    for (raiz, busca) in buscas {
        if !busca.buscar || busca.error_code.is_some() {
            bail!(
                "assunto={raiz}: buscar={}, errorCode={:?} — parâmetro não reconhecido ou backend falhou",
                busca.buscar,
                busca.error_code
            );
        }
        let linhas = busca.perguntas.len();
        let unicas = dedup(busca.perguntas);
        let nome = mapa.get(&raiz).map_or("?", |(n, _)| n.trim());
        println!(
            "SC FAQs: raiz {raiz} ({nome}): {linhas} linhas → {} únicas",
            unicas.len()
        );
        por_raiz.push((raiz, unicas));
    }

    // (b) Uma pergunta tem um assunto e a árvore não repete id: ela pertence a UMA raiz. Se aparecer em
    //     duas, a soma por raiz deixa de bater com o total único.
    let mut raiz_de = BTreeMap::new();
    let mut repetidas = Vec::new();
    for (raiz, perguntas) in &por_raiz {
        for p in perguntas {
            if let Some(outra) = raiz_de.insert(p.id, *raiz) {
                repetidas.push(format!("{} (raízes {outra} e {raiz})", p.id));
            }
        }
    }
    if !repetidas.is_empty() {
        bail!(
            "Σ únicas por raiz ≠ total único: {} pergunta(s) em mais de uma raiz — {}",
            repetidas.len(),
            repetidas.join(", ")
        );
    }

    // (d) Zero no total é a coleta que caiu, não o portal vazio.
    let perguntas: Vec<Pergunta> = por_raiz.into_iter().flat_map(|(_, ps)| ps).collect();
    if perguntas.is_empty() {
        bail!(
            "nenhuma pergunta nas {} raízes — a coleta caiu (o portal não está vazio)",
            raizes.len()
        );
    }

    // (c) Trilha de cada pergunta, contra a árvore completada pelos assuntos das próprias perguntas.
    completar_mapa(&mut mapa, &perguntas);
    let itens = perguntas
        .into_iter()
        .map(|p| {
            let t = trilha(&mapa, p.assunto.id)?;
            Ok((p, t))
        })
        .collect::<Result<Vec<_>>>()?;

    // (f) Pergunta sem texto vira `.md` sem título e vetor sem conteúdo — inútil para a busca, então
    //     é erro, não aviso como a (e): o operador olha a fonte. Resposta vazia não entra (título e
    //     link ainda são documento).
    let sem_texto: Vec<String> = itens
        .iter()
        .filter(|(p, _)| texto_da_pergunta(p).is_empty())
        .map(|(p, _)| p.id.to_string())
        .collect();
    if !sem_texto.is_empty() {
        bail!(
            "{} pergunta(s) sem texto de Questao — {}",
            sem_texto.len(),
            sem_texto.join(", ")
        );
    }

    // (e) Até 01/11/2026 todas vieram com Visibilidade 2; outro valor pode ser conteúdo interno. Só
    //     conta — quem avisa é o `run`.
    let fora_da_visibilidade = itens.iter().filter(|(p, _)| p.visibilidade != 2).count();
    println!(
        "SC FAQs: {} perguntas únicas em {} raízes",
        itens.len(),
        raizes.len()
    );

    Ok(Consolidado {
        itens,
        mapa,
        fora_da_visibilidade,
    })
}

static LINK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)<a\b[^>]*?\shref\s*=\s*["']([^"']*)["'][^>]*>(.*?)</a>"#).unwrap()
});
static BLOCO_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)<\s*/?\s*(p|br|div|li|ul|ol|tr|table|h[1-6])\b[^>]*>").unwrap()
});
static TAG_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]+>").unwrap());

/// HTML (ou texto puro) → texto (D-SCFAQ-7): `<a href="url">texto</a>` vira `texto "url"` (a
/// convenção do `normalize_links`), tag de bloco vira quebra de linha, as demais tags saem, entidades
/// são decodificadas e linhas vazias descartadas. Texto puro só passa pelo `clean` de cada linha.
fn html_para_texto(s: &str) -> String {
    let com_links = LINK_RE.replace_all(s, |c: &regex::Captures| {
        let href = auli_scraper_kit::decode_entities(c[1].trim());
        let texto = auli_scraper_kit::clean(&auli_scraper_kit::decode_entities(
            &TAG_RE.replace_all(&c[2], ""),
        ));
        if href.is_empty() || href.starts_with('#') || href.starts_with("javascript:") {
            texto
        } else if texto.is_empty() {
            format!("\"{href}\"")
        } else {
            format!("{texto} \"{href}\"")
        }
    });
    let com_quebras = BLOCO_RE.replace_all(&com_links, "\n");
    let sem_tags = TAG_RE.replace_all(&com_quebras, "");
    auli_scraper_kit::decode_entities(&sem_tags)
        .lines()
        .map(auli_scraper_kit::clean)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Link oficial da pergunta: a busca por número devolve exatamente ela.
fn link_da_pergunta(id: i64) -> String {
    format!("{BASE}/perguntas/buscar?numero={id}")
}

/// A pergunta numa linha só.
fn texto_da_pergunta(p: &Pergunta) -> String {
    auli_scraper_kit::clean(&html_para_texto(p.questao.as_deref().unwrap_or("")))
}

fn texto_da_resposta(p: &Pergunta) -> String {
    html_para_texto(p.resposta.as_deref().unwrap_or(""))
}

fn montar_faq_raw(p: &Pergunta, trilha: &str) -> FaqRaw {
    FaqRaw {
        pergunta: texto_da_pergunta(p),
        resposta: texto_da_resposta(p),
        origin: trilha.to_string(),
        url: link_da_pergunta(p.id),
    }
}

/// A árvore da aba de FAQs: o mapa de assuntos com cada pergunta pendurada no seu assunto. Os filhos
/// de um nó são os da árvore, na ordem da fonte, seguidos dos de 3º nível, por nome. Nó sem pergunta
/// em toda a subárvore fica de fora.
fn montar_arvore(raizes: &[AssuntoNo], c: &Consolidado) -> FaqNode {
    fn da_arvore(no: &AssuntoNo, filhos: &mut BTreeMap<i64, Vec<i64>>) {
        filhos.insert(no.id, no.subassuntos.iter().map(|f| f.id).collect());
        for f in &no.subassuntos {
            da_arvore(f, filhos);
        }
    }
    let mut filhos = BTreeMap::new();
    for raiz in raizes {
        da_arvore(raiz, &mut filhos);
    }
    let mut so_nas_perguntas: Vec<(i64, &str, i64)> = c
        .mapa
        .iter()
        .filter(|(id, _)| !filhos.contains_key(*id))
        .filter_map(|(id, (nome, pai))| pai.map(|pai| (pai, nome.trim(), *id)))
        .collect();
    so_nas_perguntas.sort();
    for (pai, _, id) in so_nas_perguntas {
        filhos.entry(pai).or_default().push(id);
    }

    // assunto → (trilha, perguntas penduradas nele)
    let mut itens: BTreeMap<i64, (String, Vec<FaqItem>)> = BTreeMap::new();
    for (p, t) in &c.itens {
        let (_, faq_items) = itens
            .entry(p.assunto.id)
            .or_insert_with(|| (t.clone(), Vec::new()));
        faq_items.push(FaqItem {
            pergunta: texto_da_pergunta(p),
            resposta: texto_da_resposta(p),
            url: Some(link_da_pergunta(p.id)),
        });
    }

    fn no(
        id: i64,
        mapa: &Mapa,
        filhos: &BTreeMap<i64, Vec<i64>>,
        itens: &BTreeMap<i64, (String, Vec<FaqItem>)>,
    ) -> Option<FaqNode> {
        let children: Vec<FaqNode> = filhos
            .get(&id)
            .into_iter()
            .flatten()
            .filter_map(|f| no(*f, mapa, filhos, itens))
            .collect();
        let (origin, faq_items) = itens.get(&id).cloned().unwrap_or_default();
        if children.is_empty() && faq_items.is_empty() {
            return None;
        }
        Some(FaqNode {
            title: mapa.get(&id).map_or("", |(n, _)| n.trim()).to_string(),
            url: format!("{BASE}/perguntas/buscar?assunto={id}"),
            page_type: if children.is_empty() {
                PageType::Faq
            } else {
                PageType::Menu
            },
            origin,
            children,
            faq_items,
        })
    }

    FaqNode {
        title: "Perguntas frequentes".to_string(),
        url: format!("{BASE}/perguntas"),
        page_type: PageType::Menu,
        origin: String::new(),
        children: raizes
            .iter()
            .filter_map(|r| no(r.id, &c.mapa, &filhos, &itens))
            .collect(),
        faq_items: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Fixtures fiéis ao shape real (`_next/data/<buildId>/perguntas.json` e `…/perguntas/buscar.json`,
    // 01/11/2026), recortadas. Os campos que o código ignora (`IconeBase64`, `Exige*`,
    // `AtendimentoID`) estão aqui de propósito. A árvore tem só 2 níveis, como a real: o assunto 166
    // (ASPECTOS GERAIS, filho de 75) só aparece no `Assunto` da pergunta 1211.
    const ARVORE_JSON: &str = r#"{"pageProps":{"respostaApi":{"noticias":[],"assuntosBusca":[
      {"AssuntoID":2955,"Nome":"BENEFÍCIOS FISCAIS","ExigeCPF":true,"ExigeCNPJ":true,"ExigeIE":true,
       "IconeBase64":"data:image/png;base64,iVBORw0KGgo=","Subassuntos":[
        {"AssuntoID":2969,"Nome":"PIE- PROGRAMA DE INCENTIVO AO ESPORTE","IconeBase64":null,"Subassuntos":[]},
        {"AssuntoID":75,"Nome":"TTD/FUNDOS","IconeBase64":null,"Subassuntos":[]}]},
      {"AssuntoID":91,"Nome":"BP-E- BILHETE DE PASSAGEM ELETRÔNICO","IconeBase64":null,"Subassuntos":[]},
      {"AssuntoID":72,"Nome":"DTEC - DOMICÍLIO TRIBUTÁRIO ELETRÔNICO DO CONTRIBUINTE","IconeBase64":null,
       "Subassuntos":[]}
    ]}}}"#;

    // `assunto=2955`: cada pergunta vem em dobro, como na fonte. A 1300 (pendurada direto no nó
    // intermediário 75) é sintética — o caso que a fonte permite e a árvore do frontend tem de cobrir.
    const BUSCA_2955: &str = r#"{"pageProps":{"respostaApi":{"perguntas":[
      {"BaseConhecimentoID":1211,"Questao":"Como efetuar um pedido de TTD?",
       "Resposta":"Os pedidos de TTD devem ser realizados pelo SAT, através da aplicação “TTD - Efetuar um\nPedido de Tratamento Tributário Diferenciado”.\nO requerente deve estar logado no SAT.",
       "Visibilidade":2,"Assunto":{"AssuntoID":166,"Nome":"ASPECTOS GERAIS","SuperiorHierarquicoID":75,
       "ExigeCPF":false,"ExigeCNPJ":true,"ExigeIE":true},"AtendimentoID":null},
      {"BaseConhecimentoID":1558,"Questao":"<p>O que é PIE?</p>",
       "Resposta":"<p>O Programa de Incentivo ao Esporte (PIE) é um programa&nbsp;de fomento.</p><p><br></p><p>Veja <a href=\"http://www.sef.sc.gov.br/nfce\" target=\"_blank\" data-saferedirecturl=\"https://www.google.com/url?q=http://www.sef.sc.gov.br/nfce&amp;source=gmail\" style=\"color: rgb(17, 85, 204);\">www.sef.sc.gov.br/nfce</a> e <strong>R&amp;D</strong>.</p>",
       "Visibilidade":2,"Assunto":{"AssuntoID":2969,"Nome":"PIE- PROGRAMA DE INCENTIVO AO ESPORTE",
       "SuperiorHierarquicoID":2955},"AtendimentoID":null},
      {"BaseConhecimentoID":1300,"Questao":"Pergunta do nó intermediário?","Resposta":"Resposta.",
       "Visibilidade":2,"Assunto":{"AssuntoID":75,"Nome":"TTD/FUNDOS","SuperiorHierarquicoID":2955},
       "AtendimentoID":null},
      {"BaseConhecimentoID":1211,"Questao":"Como efetuar um pedido de TTD?",
       "Resposta":"Os pedidos de TTD devem ser realizados pelo SAT, através da aplicação “TTD - Efetuar um\nPedido de Tratamento Tributário Diferenciado”.\nO requerente deve estar logado no SAT.",
       "Visibilidade":2,"Assunto":{"AssuntoID":166,"Nome":"ASPECTOS GERAIS","SuperiorHierarquicoID":75},
       "AtendimentoID":null},
      {"BaseConhecimentoID":1558,"Questao":"<p>O que é PIE?</p>","Resposta":"<p>O Programa de Incentivo ao Esporte (PIE) é um programa&nbsp;de fomento.</p>",
       "Visibilidade":2,"Assunto":{"AssuntoID":2969,"Nome":"PIE- PROGRAMA DE INCENTIVO AO ESPORTE",
       "SuperiorHierarquicoID":2955},"AtendimentoID":null},
      {"BaseConhecimentoID":1300,"Questao":"Pergunta do nó intermediário?","Resposta":"Resposta.",
       "Visibilidade":2,"Assunto":{"AssuntoID":75,"Nome":"TTD/FUNDOS","SuperiorHierarquicoID":2955},
       "AtendimentoID":null}
    ],"buscar":true,"errorCode":null}}}"#;

    const BUSCA_91: &str =
        r#"{"pageProps":{"respostaApi":{"perguntas":[],"buscar":true,"errorCode":null}}}"#;

    const BUSCA_72: &str = r#"{"pageProps":{"respostaApi":{"perguntas":[
      {"BaseConhecimentoID":1170,"Questao":"Pode acontecer que dois CNPJs de uma mesma “holding” tenham dois contratos?",
       "Resposta":"Sim.","Visibilidade":2,
       "Assunto":{"AssuntoID":72,"Nome":"DTEC - DOMICÍLIO TRIBUTÁRIO ELETRÔNICO DO CONTRIBUINTE",
       "SuperiorHierarquicoID":null},"AtendimentoID":null}
    ],"buscar":true,"errorCode":null}}}"#;

    fn arvore() -> Vec<AssuntoNo> {
        let a: NextData<ArvoreApi> = serde_json::from_str(ARVORE_JSON).unwrap();
        a.page_props.resposta_api.assuntos_busca
    }

    fn busca(json: &str) -> BuscaApi {
        let b: NextData<BuscaApi> = serde_json::from_str(json).unwrap();
        b.page_props.resposta_api
    }

    fn buscas() -> Vec<(i64, BuscaApi)> {
        vec![
            (2955, busca(BUSCA_2955)),
            (91, busca(BUSCA_91)),
            (72, busca(BUSCA_72)),
        ]
    }

    fn erro(r: Result<Consolidado>) -> String {
        match r {
            Ok(_) => panic!("esperava Err"),
            Err(e) => e.to_string(),
        }
    }

    #[test]
    fn consolidar_tira_o_dobro_e_resolve_a_trilha_do_terceiro_nivel() {
        let c = consolidar(&arvore(), buscas()).unwrap();
        let ids: Vec<i64> = c.itens.iter().map(|(p, _)| p.id).collect();
        // 6 linhas da raiz 2955 → 3 únicas, na ordem da fonte; a raiz 91 vazia não é erro.
        assert_eq!(ids, vec![1211, 1558, 1300, 1170]);
        let trilha_de = |id| c.itens.iter().find(|(p, _)| p.id == id).unwrap().1.as_str();
        // 166 não está na árvore: a trilha sai do `Assunto` da pergunta + o pai 75, que está.
        assert_eq!(
            trilha_de(1211),
            "BENEFÍCIOS FISCAIS | TTD/FUNDOS | ASPECTOS GERAIS"
        );
        assert_eq!(
            trilha_de(1558),
            "BENEFÍCIOS FISCAIS | PIE- PROGRAMA DE INCENTIVO AO ESPORTE"
        );
        assert_eq!(
            trilha_de(1170),
            "DTEC - DOMICÍLIO TRIBUTÁRIO ELETRÔNICO DO CONTRIBUINTE"
        );
        assert_eq!(c.fora_da_visibilidade, 0);
    }

    #[test]
    fn buscar_false_e_erro_e_nao_vazio() {
        let mut b = buscas();
        b[1].1 = busca(
            r#"{"pageProps":{"respostaApi":{"perguntas":[],"buscar":false,"errorCode":null}}}"#,
        );
        assert!(erro(consolidar(&arvore(), b)).contains("assunto=91: buscar=false"));
    }

    #[test]
    fn error_code_preenchido_e_erro() {
        let mut b = buscas();
        b[2].1 = busca(
            r#"{"pageProps":{"respostaApi":{"perguntas":[],"buscar":true,"errorCode":500}}}"#,
        );
        assert!(erro(consolidar(&arvore(), b)).contains("assunto=72"));
    }

    #[test]
    fn pergunta_em_duas_raizes_e_erro() {
        // A 1170 (raiz 72) aparece também na resposta da raiz 2955.
        let mut b = buscas();
        let intrusa = b[2].1.perguntas[0].clone();
        b[0].1.perguntas.push(intrusa);
        let e = erro(consolidar(&arvore(), b));
        assert!(e.contains("1170 (raízes 2955 e 72)"), "{e}");
    }

    #[test]
    fn pai_fora_do_mapa_e_erro() {
        // Um 4º nível: o assunto 999 não está na árvore e o pai dele (998) não aparece em lugar nenhum.
        let mut b = buscas();
        let mut orfa = b[2].1.perguntas[0].clone();
        orfa.id = 4242;
        orfa.assunto = AssuntoRef {
            id: 999,
            nome: "NETO".to_string(),
            pai: Some(998),
        };
        b[2].1.perguntas.push(orfa);
        let e = erro(consolidar(&arvore(), b));
        assert!(e.contains("assunto 998 fora do mapa"), "{e}");
    }

    #[test]
    fn zero_perguntas_e_erro() {
        let b = vec![(2955, busca(BUSCA_91)), (91, busca(BUSCA_91))];
        assert!(erro(consolidar(&arvore(), b)).contains("nenhuma pergunta"));
    }

    #[test]
    fn pergunta_sem_texto_e_erro() {
        // HTML que vira vazio (o caminho pelo `html_para_texto`, não só o `null`) e um `null`. A 1558
        // vem em dobro na raiz 2955: as duas linhas mudam, senão o dedup ficaria com a intacta.
        let mut b = buscas();
        for p in b[0].1.perguntas.iter_mut().filter(|p| p.id == 1558) {
            p.questao = Some("<p> </p>".to_string());
        }
        b[2].1.perguntas[0].questao = None;
        let e = erro(consolidar(&arvore(), b));
        assert!(e.contains("2 pergunta(s) sem texto de Questao"), "{e}");
        assert!(e.contains("1558") && e.contains("1170"), "{e}");
    }

    #[test]
    fn visibilidade_diferente_de_2_so_avisa() {
        let mut b = buscas();
        b[2].1.perguntas[0].visibilidade = 1;
        let c = consolidar(&arvore(), b).unwrap();
        assert_eq!(c.fora_da_visibilidade, 1);
        assert_eq!(c.itens.len(), 4);
    }

    #[test]
    fn html_para_texto_converte_paragrafos_links_e_entidades() {
        let p = &busca(BUSCA_2955).perguntas[1];
        assert_eq!(
            html_para_texto(p.resposta.as_deref().unwrap()),
            "O Programa de Incentivo ao Esporte (PIE) é um programa de fomento.\n\
             Veja www.sef.sc.gov.br/nfce \"http://www.sef.sc.gov.br/nfce\" e R&D."
        );
        // Âncora vazia vira só a url; `#` e `javascript:` ficam só com o texto.
        assert_eq!(
            html_para_texto(r#"<a href="https://x.sc.gov.br/a"></a>"#),
            "\"https://x.sc.gov.br/a\""
        );
        assert_eq!(html_para_texto(r##"<a href="#topo">Topo</a>"##), "Topo");
        assert_eq!(html_para_texto("a<br>b<BR/>c"), "a\nb\nc");
    }

    #[test]
    fn html_para_texto_deixa_o_texto_puro_como_veio() {
        let p = &busca(BUSCA_2955).perguntas[0];
        let r = p.resposta.as_deref().unwrap();
        assert_eq!(html_para_texto(r), r);
    }

    #[test]
    fn faq_raw_tem_pergunta_em_texto_trilha_e_link_oficial() {
        let c = consolidar(&arvore(), buscas()).unwrap();
        let (p, t) = &c.itens[1];
        let f = montar_faq_raw(p, t);
        assert_eq!(f.pergunta, "O que é PIE?");
        assert!(f.resposta.starts_with("O Programa de Incentivo"));
        assert_eq!(
            f.origin,
            "BENEFÍCIOS FISCAIS | PIE- PROGRAMA DE INCENTIVO AO ESPORTE"
        );
        assert_eq!(
            f.url,
            "https://www.sef.sc.gov.br/perguntas/buscar?numero=1558"
        );
    }

    #[test]
    fn arvore_do_frontend_tem_o_terceiro_nivel_e_omite_o_vazio() {
        let raizes = arvore();
        let c = consolidar(&raizes, buscas()).unwrap();
        let t = montar_arvore(&raizes, &c);
        assert_eq!(t.title, "Perguntas frequentes");
        assert_eq!(t.page_type, PageType::Menu);
        // A raiz 91 não tem pergunta: fica de fora.
        let raiz: Vec<&str> = t.children.iter().map(|n| n.title.as_str()).collect();
        assert_eq!(
            raiz,
            vec![
                "BENEFÍCIOS FISCAIS",
                "DTEC - DOMICÍLIO TRIBUTÁRIO ELETRÔNICO DO CONTRIBUINTE"
            ]
        );
        let beneficios = &t.children[0];
        assert_eq!(beneficios.page_type, PageType::Menu);
        assert_eq!(
            beneficios.url,
            "https://www.sef.sc.gov.br/perguntas/buscar?assunto=2955"
        );
        let ttd = &beneficios.children[1];
        // TTD/FUNDOS é Menu (tem o 166 como filho) E tem a pergunta pendurada nele.
        assert_eq!(ttd.title, "TTD/FUNDOS");
        assert_eq!(ttd.page_type, PageType::Menu);
        assert_eq!(ttd.faq_items.len(), 1);
        assert_eq!(ttd.children.len(), 1);
        let aspectos = &ttd.children[0];
        assert_eq!(aspectos.page_type, PageType::Faq);
        assert_eq!(
            aspectos.origin,
            "BENEFÍCIOS FISCAIS | TTD/FUNDOS | ASPECTOS GERAIS"
        );
        assert_eq!(
            aspectos.faq_items[0].pergunta,
            "Como efetuar um pedido de TTD?"
        );
        // Cada pergunta leva o próprio link, não só o do assunto.
        assert_eq!(
            aspectos.faq_items[0].url.as_deref(),
            Some("https://www.sef.sc.gov.br/perguntas/buscar?numero=1211")
        );
        // O fio que o `parseFaqs.ts` lê: folha sem `children`, page_type como string.
        let json = serde_json::to_value(aspectos).unwrap();
        assert_eq!(json["page_type"], "Faq");
        assert!(json.get("children").is_none());
    }
}
