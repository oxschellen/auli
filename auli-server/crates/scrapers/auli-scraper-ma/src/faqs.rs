// SEFAZ-MA FAQs — a página "Perguntas Frequentes" do Portal SGC (`/portal/perguntas-frequentes`), o mesmo SPA
// Angular sobre a API Spring dos serviços (`ma.rs`). Estudo, medições e decisões: docs/ESTUDO-MA-Faqs.md
// (D-MAFAQ-*).
//
// Uma chamada só, com o token anônimo do portal: `GET /sgc/api/faq/perguntas/faq?publicado=true` devolve todas
// as perguntas publicadas, cada uma com a resposta inteira (HTML) e a categoria — e a categoria traz a mãe
// (`categoriaPai`). A página agrupa por categoria num acordeão de dois níveis; não tem rota nem âncora por
// pergunta.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;
use std::sync::LazyLock;

use anyhow::{Context, Result, anyhow, bail};
use serde::Deserialize;

use auli_contract::FaqRaw;
use auli_scraper_kit::clean;
use auli_scraper_kit::faqtree::{FaqItem, FaqNode, PageType};
use auli_scraper_kit::http::GetOpts;

use crate::ma::{API_BASE, FRONT_BASE};

/// O filtro é o da página, e só ele separa o que aparece: sem `publicado=true` vêm também as 17 perguntas da
/// categoria inativa "CANAL VERDE", que dizem `publicado: true` no próprio item (06/11/2026).
static API: LazyLock<String> =
    LazyLock::new(|| format!("{API_BASE}/faq/perguntas/faq?publicado=true"));
/// A página das perguntas frequentes: o link de toda pergunta e de todo nó da árvore.
static FAQ: LazyLock<String> =
    LazyLock::new(|| format!("{FRONT_BASE}/portal/perguntas-frequentes"));

/// Coleta as FAQs e grava a árvore do frontend (`<id>-faqs-tree.json`) e o snapshot. As guardas rodam antes de
/// qualquer gravação: rodada que falha deixa os arquivos anteriores intactos.
pub fn run(data_dir: &str, use_cache: bool) -> Result<()> {
    let perguntas: Vec<Pergunta> = serde_json::from_str(&buscar(data_dir, use_cache)?)
        .context("resposta da API fora do formato [{id, pergunta, resposta, categoria}, …]")?;
    let mut raizes = agrupar(perguntas)?;
    desambiguar(&mut raizes);
    for r in &raizes {
        println!("MA FAQs: {}: {} perguntas", r.nome, r.total());
    }
    let faqs = consolidar(&raizes)?;

    // Atômico pelo mesmo motivo das outras entidades: a árvore não tem outra fonte (o snapshot achatado não a
    // reconstrói) e é o único artefato de coleta que o frontend lê direto.
    let tree_path = format!("{data_dir}/{}-faqs-tree.json", crate::ENTITY);
    auli_contract::arquivo::escrever_atomico(
        Path::new(&tree_path),
        &serde_json::to_string_pretty(&montar_arvore(&raizes))?,
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

/// A chamada à API: rede sempre (login anônimo + 1 GET), gravando o cache; com `--usecache`, só o cache, e
/// miss é erro.
fn buscar(data_dir: &str, use_cache: bool) -> Result<String> {
    if use_cache {
        return auli_scraper_kit::cache::read(data_dir, "faqs", &API)
            .ok_or_else(|| anyhow!("cache miss para {} (modo --usecache, sem rede)", *API));
    }
    let agent = crate::ma::build_agent();
    let token = crate::ma::login(&agent)?;
    let resposta = auli_scraper_kit::http::get_string(
        &agent,
        &API,
        &GetOpts {
            log_prefix: "MA FAQs",
            accept: Some("application/json"),
            headers: &[("AuthorizationPortal", &token)],
            ..Default::default()
        },
    )?;
    auli_scraper_kit::cache::write(data_dir, "faqs", &API, &resposta);
    Ok(resposta)
}

#[derive(Deserialize)]
struct Pergunta {
    id: u64,
    pergunta: String,
    resposta: String,
    categoria: Categoria,
}

/// A categoria como a API manda. A mãe (`categoriaPai`) vem rasa — `ordem: null` e `ativo: false` em todas,
/// valores de omissão —, então nome e ordem de uma raiz só valem lidos da categoria da pergunta pendurada
/// direto nela, que é de onde a página também os tira.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Categoria {
    id: u64,
    nome: String,
    ordem: Option<i64>,
    categoria_pai: Option<Box<Categoria>>,
}

struct Item {
    id: u64,
    pergunta: String,
    resposta: String,
}

struct Sub {
    nome: String,
    itens: Vec<Item>,
}

struct Raiz {
    nome: String,
    /// As perguntas penduradas direto na raiz. Numa raiz com subcategorias é só a de apresentação ("Sobre o
    /// IPVA") — e é ela que faz a raiz existir na página.
    itens: Vec<Item>,
    subs: Vec<Sub>,
}

impl Raiz {
    fn total(&self) -> usize {
        self.itens.len() + self.subs.iter().map(|s| s.itens.len()).sum::<usize>()
    }
}

/// As perguntas na árvore da página: raiz → subcategoria. Raízes pela `ordem` (sem ordem vão para o fim; o
/// nome desempata), subcategorias pelo `id` (como a página), perguntas pelo texto (como a página; o `id`
/// desempata). Ordenado, o snapshot só muda quando a FAQ muda.
fn agrupar(perguntas: Vec<Pergunta>) -> Result<Vec<Raiz>> {
    let mut ids = BTreeSet::new();
    // Raiz → (nome, ordem), lidos das perguntas diretas; raiz → subcategoria → itens.
    let mut cabecas: HashMap<u64, (String, Option<i64>)> = HashMap::new();
    let mut diretas: HashMap<u64, Vec<Item>> = HashMap::new();
    let mut subs: HashMap<u64, BTreeMap<u64, Sub>> = HashMap::new();
    for p in perguntas {
        if !ids.insert(p.id) {
            bail!("a pergunta {} veio duas vezes — a API mudou?", p.id);
        }
        let nome = nome_da_categoria(&p.categoria)?;
        let item = Item {
            id: p.id,
            pergunta: clean(&p.pergunta),
            resposta: html_para_texto(&p.resposta),
        };
        match &p.categoria.categoria_pai {
            None => {
                cabecas.insert(p.categoria.id, (nome, p.categoria.ordem));
                diretas.entry(p.categoria.id).or_default().push(item);
            }
            Some(mae) if mae.categoria_pai.is_none() => subs
                .entry(mae.id)
                .or_default()
                .entry(p.categoria.id)
                .or_insert_with(|| Sub {
                    nome,
                    itens: Vec::new(),
                })
                .itens
                .push(item),
            Some(_) => bail!(
                "a pergunta {} está numa categoria de três níveis ({}) — a coleta só conhece dois",
                p.id,
                p.categoria.id
            ),
        }
    }
    // A página só desenha um grupo que tem pergunta própria: subcategoria cuja raiz não tem nenhuma não aparece.
    if let Some((raiz, s)) = subs.iter().find(|(r, _)| !cabecas.contains_key(*r)) {
        let s = s
            .values()
            .next()
            .map(|s| s.nome.as_str())
            .unwrap_or_default();
        bail!(
            "a subcategoria {s:?} pende da categoria {raiz}, que não tem pergunta própria — a página não a mostra"
        );
    }

    let mut raizes: Vec<(Option<i64>, u64, Raiz)> = cabecas
        .into_iter()
        .map(|(id, (nome, ordem))| {
            let mut itens = diretas.remove(&id).unwrap_or_default();
            ordenar(&mut itens);
            let subs = subs
                .remove(&id)
                .unwrap_or_default()
                .into_values()
                .map(|mut s| {
                    ordenar(&mut s.itens);
                    s
                })
                .collect();
            (ordem, id, Raiz { nome, itens, subs })
        })
        .collect();
    raizes.sort_by(|(oa, ia, a), (ob, ib, b)| {
        (oa.is_none(), oa, &a.nome, ia).cmp(&(ob.is_none(), ob, &b.nome, ib))
    });
    Ok(raizes.into_iter().map(|(_, _, r)| r).collect())
}

fn ordenar(itens: &mut [Item]) {
    itens.sort_by(|a, b| (&a.pergunta, a.id).cmp(&(&b.pergunta, b.id)));
}

/// O nome com os espaços normalizados (`" REFIS - …"`, `"NOVO SISTEMA DE  DOWNLOAD …"`); vazio é erro.
fn nome_da_categoria(c: &Categoria) -> Result<String> {
    let nome = clean(&c.nome);
    if nome.is_empty() {
        bail!("a categoria {} veio sem nome", c.id);
    }
    Ok(nome)
}

/// D-MAFAQ-2: o link é o mesmo para todas, então o nome do `.md` (pergunta + hash de link e pergunta) sai só da
/// pergunta. Pergunta que aparece em mais de uma raiz ganha o nome da raiz entre parênteses — hoje as duas
/// "Sobre a seção" (RTC e Ativação de Novas Empresas). Repetida dentro da mesma raiz continua repetida, e a
/// guarda do `consolidar` para a coleta.
fn desambiguar(raizes: &mut [Raiz]) {
    let mut contagem: HashMap<String, usize> = HashMap::new();
    for r in raizes.iter() {
        for i in itens_da_raiz(r) {
            *contagem.entry(i.pergunta.clone()).or_default() += 1;
        }
    }
    for r in raizes.iter_mut() {
        let nome = r.nome.clone();
        let itens = r
            .itens
            .iter_mut()
            .chain(r.subs.iter_mut().flat_map(|s| s.itens.iter_mut()));
        for i in itens {
            if contagem[&i.pergunta] > 1 {
                i.pergunta = format!("{} ({nome})", i.pergunta);
            }
        }
    }
}

fn itens_da_raiz(r: &Raiz) -> impl Iterator<Item = &Item> {
    r.itens.iter().chain(r.subs.iter().flat_map(|s| &s.itens))
}

/// D-MAFAQ-3: `<raiz>` para as perguntas penduradas direto na raiz, `<raiz> | <subcategoria>` para as outras.
fn trilha(raiz: &Raiz, sub: Option<&Sub>) -> String {
    match sub {
        None => raiz.nome.clone(),
        Some(s) => format!("{} | {}", raiz.nome, s.nome),
    }
}

/// O acervo, com as guardas que olham todas as raízes.
fn consolidar(raizes: &[Raiz]) -> Result<Vec<FaqRaw>> {
    let faq = |i: &Item, origin: String| FaqRaw {
        pergunta: i.pergunta.clone(),
        resposta: i.resposta.clone(),
        origin,
        url: FAQ.to_string(),
    };
    let faqs: Vec<FaqRaw> = raizes
        .iter()
        .flat_map(|r| {
            let diretas = r.itens.iter().map(move |i| faq(i, trilha(r, None)));
            let das_subs = r
                .subs
                .iter()
                .flat_map(move |s| s.itens.iter().map(move |i| faq(i, trilha(r, Some(s)))));
            diretas.chain(das_subs)
        })
        .collect();
    // Sem pergunta não há o que coletar: é a API que mudou, não a FAQ que acabou.
    if faqs.is_empty() {
        bail!("nenhuma pergunta publicada na FAQ — a API mudou?");
    }
    if let Some(f) = faqs.iter().find(|f| f.pergunta.is_empty()) {
        bail!("pergunta sem texto em {}", f.origin);
    }
    let mut vistas = BTreeSet::new();
    if let Some(f) = faqs.iter().find(|f| !vistas.insert(f.pergunta.as_str())) {
        bail!(
            "a pergunta {:?} aparece duas vezes (a 2ª em {}) — com o mesmo link, os .md teriam o mesmo nome",
            f.pergunta,
            f.origin
        );
    }
    let sem_resposta = faqs.iter().filter(|f| f.resposta.is_empty()).count();
    if sem_resposta > 0 {
        eprintln!("⚠️  MA FAQs: {sem_resposta} pergunta(s) com resposta vazia depois da conversão");
    }
    println!(
        "MA FAQs: {} perguntas em {} raízes e {} subcategorias",
        faqs.len(),
        raizes.len(),
        raizes.iter().map(|r| r.subs.len()).sum::<usize>()
    );
    Ok(faqs)
}

/// A árvore da aba de FAQs: raiz → uma raiz sem subcategoria é `Faq`; com subcategorias é `Menu` com um `Faq` por
/// subcategoria e, pendurada no próprio `Menu`, a pergunta de apresentação (como as raízes com pergunta da SC).
/// Tudo aponta para a mesma página — o SPA não tem rota por categoria nem por pergunta —, e os itens ficam sem
/// `url` (a interface usa a do nó).
fn montar_arvore(raizes: &[Raiz]) -> FaqNode {
    let itens = |itens: &[Item]| -> Vec<FaqItem> {
        itens
            .iter()
            .map(|i| FaqItem {
                pergunta: i.pergunta.clone(),
                resposta: i.resposta.clone(),
                url: None,
            })
            .collect()
    };
    let no = |title: &str, page_type, origin, children, faq_items| FaqNode {
        title: title.to_string(),
        url: FAQ.to_string(),
        page_type,
        origin,
        children,
        faq_items,
    };
    let filhos = raizes
        .iter()
        .map(|r| {
            let subs: Vec<FaqNode> = r
                .subs
                .iter()
                .map(|s| {
                    no(
                        &s.nome,
                        PageType::Faq,
                        trilha(r, Some(s)),
                        Vec::new(),
                        itens(&s.itens),
                    )
                })
                .collect();
            let tipo = if subs.is_empty() {
                PageType::Faq
            } else {
                PageType::Menu
            };
            no(&r.nome, tipo, trilha(r, None), subs, itens(&r.itens))
        })
        .collect();
    no(
        "Perguntas frequentes",
        PageType::Menu,
        String::new(),
        filhos,
        Vec::new(),
    )
}

/// HTML → texto pelo conversor da frota (`auli_scraper_kit::html`, D-KITHTML-1): host do portal SGC.
fn html_para_texto(s: &str) -> String {
    auli_scraper_kit::html::html_para_texto(
        s,
        auli_scraper_kit::html::Opcoes {
            base: Some(FAQ.as_str()),
            quebras_do_fonte: false,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // Recorte da resposta de 06/11/2026: 14 perguntas de 7 raízes, embaralhadas. As duas "Sobre a seção" (698
    // RTC, 682 Ativação); raízes com subcategoria (RTC, Ativação, IPVA, os dois REFIS) e sem (Substituição
    // Tributária, Comércio Exterior); raiz com espaço à esquerda (" REFIS … ICMS", ordem 0); a 212 com espaço
    // duplo na pergunta; a 663 com U+FEFF; a 742 com links e `&gt;`; a 299, ponteiro só de links.
    const PERGUNTAS: &str = include_str!("../tests/fixtures/ma-faqs-perguntas.json");

    fn perguntas() -> Vec<Pergunta> {
        serde_json::from_str(PERGUNTAS).unwrap()
    }

    fn raizes() -> Vec<Raiz> {
        let mut r = agrupar(perguntas()).unwrap();
        desambiguar(&mut r);
        r
    }

    fn erro<T>(r: Result<T>) -> String {
        match r {
            Ok(_) => panic!("esperava Err"),
            Err(e) => format!("{e:#}"),
        }
    }

    fn resposta(id: u64) -> String {
        let p = perguntas().into_iter().find(|p| p.id == id).unwrap();
        html_para_texto(&p.resposta)
    }

    fn pergunta_mut(ps: &mut [Pergunta], id: u64) -> &mut Pergunta {
        ps.iter_mut().find(|p| p.id == id).unwrap()
    }

    #[test]
    fn raizes_pela_ordem_da_propria_categoria() {
        // A mãe aninhada vem com `ordem: null`: lida dali, as raízes com subcategoria iriam todas para o fim.
        let nomes: Vec<String> = raizes().into_iter().map(|r| r.nome).collect();
        assert_eq!(
            nomes,
            [
                "REFIS - PROGRAMA DE PAGAMENTO E PARCELAMENTO DE DÉBITOS DE ICMS",
                "REFIS - PROGRAMA DE PAGAMENTO E PARCELAMENTO DE DÉBITOS DE IPVA",
                "ATIVAÇÃO DE NOVAS EMPRESAS – PORTARIA nº 212/2026",
                "REFORMA TRIBUTÁRIA DO CONSUMO - RTC",
                "IPVA",
                "COMÉRCIO EXTERIOR",
                "SUBSTITUIÇÃO TRIBUTÁRIA",
            ]
        );
    }

    #[test]
    fn raiz_sem_ordem_vai_para_o_fim() {
        let mut ps = perguntas();
        pergunta_mut(&mut ps, 482).categoria.ordem = None;
        let nomes: Vec<String> = agrupar(ps).unwrap().into_iter().map(|r| r.nome).collect();
        assert_eq!(
            nomes.last().unwrap(),
            "REFIS - PROGRAMA DE PAGAMENTO E PARCELAMENTO DE DÉBITOS DE ICMS"
        );
    }

    #[test]
    fn perguntas_pelo_texto_e_subcategorias_na_raiz() {
        let rs = raizes();
        let st: Vec<&str> = rs[6].itens.iter().map(|i| i.pergunta.as_str()).collect();
        assert!(st[0].starts_with("b) Como corrigir") && st[1].starts_with("f) Como tratar"));
        assert!(rs[6].subs.is_empty());
        let rtc = &rs[3];
        assert_eq!(rtc.itens.len(), 1);
        assert_eq!(rtc.subs.len(), 1);
        let sub: Vec<u64> = rtc.subs[0].itens.iter().map(|i| i.id).collect();
        assert_eq!(sub, [733, 742]);
    }

    #[test]
    fn pergunta_sai_com_espacos_normalizados() {
        let rs = raizes();
        assert_eq!(
            rs[4].subs[0].itens[0].pergunta,
            "IP.01.001 - Qual a periodicidade do IPVA?"
        );
    }

    #[test]
    fn pergunta_repetida_em_duas_raizes_ganha_o_nome_da_raiz() {
        let rs = raizes();
        assert_eq!(
            rs[2].itens[0].pergunta,
            "Sobre a seção (ATIVAÇÃO DE NOVAS EMPRESAS – PORTARIA nº 212/2026)"
        );
        assert_eq!(
            rs[3].itens[0].pergunta,
            "Sobre a seção (REFORMA TRIBUTÁRIA DO CONSUMO - RTC)"
        );
        // As outras ficam como vieram.
        assert_eq!(rs[4].itens[0].pergunta, "Sobre o IPVA");
    }

    #[test]
    fn pergunta_repetida_na_mesma_raiz_e_erro() {
        let mut ps = perguntas();
        pergunta_mut(&mut ps, 733).pergunta = perguntas()
            .into_iter()
            .find(|p| p.id == 742)
            .unwrap()
            .pergunta;
        let mut rs = agrupar(ps).unwrap();
        desambiguar(&mut rs);
        let e = erro(consolidar(&rs));
        assert!(
            e.contains("aparece duas vezes") && e.contains("RTC.03 – Opção pelo Simples Nacional"),
            "{e}"
        );
    }

    #[test]
    fn trilha_e_link_de_todas() {
        let faqs = consolidar(&raizes()).unwrap();
        assert_eq!(faqs.len(), 14);
        assert!(
            faqs.iter()
                .all(|f| f.url == "https://portal-sgc.sefaz.ma.gov.br/portal/perguntas-frequentes")
        );
        assert_eq!(
            faqs[0].origin,
            "REFIS - PROGRAMA DE PAGAMENTO E PARCELAMENTO DE DÉBITOS DE ICMS"
        );
        assert_eq!(
            faqs[1].origin,
            "REFIS - PROGRAMA DE PAGAMENTO E PARCELAMENTO DE DÉBITOS DE ICMS | REF.07 – Manutenção do Benefício"
        );
        assert_eq!(faqs[13].origin, "SUBSTITUIÇÃO TRIBUTÁRIA");
    }

    #[test]
    fn html_para_texto_mantem_o_link_e_decodifica() {
        let t = resposta(742);
        let linhas: Vec<&str> = t.lines().collect();
        assert_eq!(linhas[0], "Explicação:");
        assert_eq!(linhas[1], "Para empresas contribuinte do Estado:");
        assert!(
            t.contains(
                "agências, clique aqui. \"https://portal-sgc.sefaz.ma.gov.br/portal/contato\""
            ),
            "{t}"
        );
        assert!(t.contains("o menu SEFAZNET > DTE.05 – Consulta"), "{t}");
        assert!(!t.contains('<') && !t.contains("&gt;"), "{t}");
    }

    #[test]
    fn html_para_texto_tira_o_feff() {
        let p = perguntas().into_iter().find(|p| p.id == 663).unwrap();
        assert!(p.resposta.contains('\u{feff}'));
        assert!(!html_para_texto(&p.resposta).contains('\u{feff}'));
    }

    #[test]
    fn ponteiro_so_de_links_fica_com_as_urls() {
        let t = resposta(299);
        assert_eq!(t.lines().count(), 3, "{t}");
        assert!(
            t.lines().all(|l| l.ends_with('"') && l.contains(" \"http")),
            "{t}"
        );
    }

    #[test]
    fn id_repetido_e_erro() {
        let mut ps = perguntas();
        pergunta_mut(&mut ps, 71).id = 70;
        let e = erro(agrupar(ps));
        assert!(e.contains("a pergunta 70 veio duas vezes"), "{e}");
    }

    #[test]
    fn categoria_de_tres_niveis_e_erro() {
        let mut ps = perguntas();
        let avo = serde_json::from_str::<Categoria>(r#"{"id":1,"nome":"X","ordem":0}"#).unwrap();
        pergunta_mut(&mut ps, 742)
            .categoria
            .categoria_pai
            .as_mut()
            .unwrap()
            .categoria_pai = Some(Box::new(avo));
        let e = erro(agrupar(ps));
        assert!(e.contains("três níveis"), "{e}");
    }

    #[test]
    fn subcategoria_sem_raiz_com_pergunta_e_erro() {
        // Sem a "Sobre o IPVA" (211), a raiz IPVA não tem pergunta própria e a página esconde a subcategoria.
        let ps: Vec<Pergunta> = perguntas().into_iter().filter(|p| p.id != 211).collect();
        let e = erro(agrupar(ps));
        assert!(
            e.contains("IP.01 - Conceitos Gerais") && e.contains("não tem pergunta própria"),
            "{e}"
        );
    }

    #[test]
    fn categoria_sem_nome_e_erro() {
        let mut ps = perguntas();
        pergunta_mut(&mut ps, 299).categoria.nome = " ".into();
        let e = erro(agrupar(ps));
        assert!(e.contains("veio sem nome"), "{e}");
    }

    #[test]
    fn resposta_fora_do_formato_e_erro() {
        assert!(serde_json::from_str::<Vec<Pergunta>>(r#"{"erro":"indisponível"}"#).is_err());
        let sem_categoria =
            r#"[{"id":1,"pergunta":"P?","resposta":"R","ativo":true,"publicado":true}]"#;
        assert!(serde_json::from_str::<Vec<Pergunta>>(sem_categoria).is_err());
    }

    #[test]
    fn acervo_vazio_e_erro() {
        let e = erro(consolidar(&agrupar(Vec::new()).unwrap()));
        assert!(e.contains("nenhuma pergunta"), "{e}");
    }

    #[test]
    fn pergunta_sem_texto_e_erro() {
        let mut ps = perguntas();
        pergunta_mut(&mut ps, 70).pergunta = "  ".into();
        let e = erro(consolidar(&agrupar(ps).unwrap()));
        assert!(
            e.contains("pergunta sem texto em SUBSTITUIÇÃO TRIBUTÁRIA"),
            "{e}"
        );
    }

    #[test]
    fn arvore_com_menu_para_raiz_com_subcategorias() {
        let t = montar_arvore(&raizes());
        assert_eq!((t.page_type, t.children.len()), (PageType::Menu, 7));
        let ipva = &t.children[4];
        assert_eq!(
            (ipva.title.as_str(), ipva.page_type),
            ("IPVA", PageType::Menu)
        );
        assert_eq!(ipva.origin, "IPVA");
        assert_eq!(ipva.faq_items[0].pergunta, "Sobre o IPVA");
        assert_eq!(ipva.children[0].page_type, PageType::Faq);
        assert_eq!(ipva.children[0].origin, "IPVA | IP.01 - Conceitos Gerais");
        let st = &t.children[6];
        assert_eq!(
            (st.page_type, st.children.len(), st.faq_items.len()),
            (PageType::Faq, 0, 2)
        );
        let todos = |n: &FaqNode| n.faq_items.iter().all(|i| i.url.is_none()) && n.url == *FAQ;
        assert!(t.children.iter().all(todos));
    }
}
