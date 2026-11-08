// SEFAZ-MT FAQs — três fontes, cada uma um ramo da árvore. Estudo, medições e decisões: docs/faqs/ESTUDO-MT-Faqs.md
// (D-MTFAQ-*).
//
// - **Nota MT** (`www.sefaz.mt.gov.br/notamt/duvida`, aplicação Angular): uma chamada, `GET /notamt/api/duvidas` (sem
//   sessão), devolve todas as perguntas, com a resposta inteira, o grupo ("3 - PRÊMIOS E ENTIDADES"), a ordem dentro
//   do grupo e o status. A aplicação agrupa e mostra num acordeão; não tem rota nem âncora por pergunta.
// - **Site da SEFAZ** (`www5`, Liferay): as páginas "Perguntas frequentes" das áreas de serviço — `site.rs`
//   (D-MTFAQ-10, -11, -13).
// - **Portal MT** (`portal.mt.gov.br`, X-Via): as perguntas da SEFAZ na FAQ do portal do Estado — `portal.rs`
//   (D-MTFAQ-10, -12).

mod portal;
mod site;

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;
use std::thread::sleep;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, bail};
use serde::Deserialize;

use auli_contract::FaqRaw;
use auli_scraper_kit::clean;
use auli_scraper_kit::faqtree::{FaqItem, FaqNode, PageType};
use auli_scraper_kit::http::GetOpts;

const API: &str = "https://www.sefaz.mt.gov.br/notamt/api/duvidas";
/// A página das dúvidas: o link de toda pergunta e de todo nó da árvore.
const FAQ: &str = "https://www.sefaz.mt.gov.br/notamt/duvida";
/// O programa abre a trilha de cada pergunta: "PROGRAMA" ou "CADASTRO" sozinhos não dizem de quê.
const PROGRAMA: &str = "Nota MT";

/// A identidade das FAQs da frota (D-MTFAQ-9).
const UA: &str =
    "AuliBot/0.1 (+https://github.com/oxschellen/auli; carlos.schellenberger@gmail.com)";
/// O `Crawl-delay` que o `www5` declara no `robots.txt`, honrado (D-MTFAQ-13). Nos outros hosts, 1 s.
const CORTESIA_WWW5: Duration = Duration::from_secs(30);
const CORTESIA: Duration = Duration::from_secs(1);
const WWW5: &str = "www5.sefaz.mt.gov.br";

/// Coleta as FAQs e grava a árvore do frontend (`<id>-faqs-tree.json`) e o snapshot. As guardas
/// (D-MTFAQ-4) rodam antes de qualquer gravação: rodada que falha deixa os arquivos anteriores intactos.
pub fn run(data_dir: &str, use_cache: bool) -> Result<()> {
    let rede = Rede {
        data_dir,
        use_cache,
        agente: auli_scraper_kit::build_agent(UA, Some(Duration::from_secs(60))),
        ultima: RefCell::new(HashMap::new()),
    };
    let duvidas: Vec<Duvida> = serde_json::from_str(&rede.get(API, "application/json")?)
        .context("resposta da API fora do formato [{id, descDuvida, descResposta, …}]")?;
    let fontes = [
        Fonte {
            titulo: PROGRAMA,
            url: FAQ.to_string(),
            grupos: agrupar(duvidas)?,
        },
        Fonte {
            titulo: site::TITULO,
            url: site::TODOS.to_string(),
            grupos: site::coletar(&rede).context("MT FAQs: site da SEFAZ (www5)")?,
        },
        Fonte {
            titulo: portal::TITULO,
            url: portal::FAQ.to_string(),
            grupos: portal::coletar(&rede).context("MT FAQs: Portal MT (X-Via)")?,
        },
    ];
    for f in &fontes {
        for g in &f.grupos {
            println!(
                "MT FAQs: {} | {}: {} perguntas",
                f.titulo,
                g.nome,
                g.perguntas.len()
            );
        }
    }
    let faqs = consolidar(&fontes)?;

    // Atômico pelo mesmo motivo das outras entidades: a árvore não tem outra fonte (o snapshot achatado
    // não a reconstrói) e é o único artefato de coleta que o frontend lê direto.
    let tree_path = format!("{data_dir}/{}-faqs-tree.json", crate::ENTITY);
    auli_contract::arquivo::escrever_atomico(
        Path::new(&tree_path),
        &serde_json::to_string_pretty(&montar_arvore(&fontes))?,
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

/// As chamadas de rede (D-MTFAQ-5): rede sempre, gravando o cache; com `--usecache`, só o cache, e miss é erro. Entre
/// duas chamadas ao mesmo host, a cortesia dele: 30 s no `www5`, 1 s nos outros.
struct Rede<'a> {
    data_dir: &'a str,
    use_cache: bool,
    agente: ureq::Agent,
    ultima: RefCell<HashMap<String, Instant>>,
}

impl Rede<'_> {
    fn get(&self, url: &str, accept: &str) -> Result<String> {
        self.chamar(url, url, || {
            auli_scraper_kit::http::get_string(
                &self.agente,
                url,
                &GetOpts {
                    log_prefix: "MT FAQs",
                    accept: Some(accept),
                    ..Default::default()
                },
            )
        })
    }

    /// POST de JSON; o cache é por URL e corpo.
    fn post(&self, url: &str, corpo: &serde_json::Value, origin: &str) -> Result<String> {
        self.chamar(url, &format!("{url} {corpo}"), || {
            auli_scraper_kit::http::post_json(
                &self.agente,
                url,
                &[("Origin", origin)],
                corpo,
                &GetOpts {
                    log_prefix: "MT FAQs",
                    accept: Some("application/json"),
                    ..Default::default()
                },
            )
        })
    }

    fn chamar(&self, url: &str, chave: &str, f: impl FnOnce() -> Result<String>) -> Result<String> {
        if self.use_cache {
            return auli_scraper_kit::cache::read(self.data_dir, "faqs", chave)
                .ok_or_else(|| anyhow!("cache miss para {chave} (modo --usecache, sem rede)"));
        }
        let host = url.split('/').nth(2).unwrap_or_default().to_string();
        let espera = if host == WWW5 {
            CORTESIA_WWW5
        } else {
            CORTESIA
        };
        if let Some(antes) = self.ultima.borrow().get(&host) {
            sleep(espera.saturating_sub(antes.elapsed()));
        }
        let corpo = f();
        self.ultima.borrow_mut().insert(host, Instant::now());
        let corpo = corpo?;
        auli_scraper_kit::cache::write(self.data_dir, "faqs", chave, &corpo);
        Ok(corpo)
    }
}

/// Uma `Rede` só de cache (`--usecache`), num diretório temporário com as respostas dadas, `(chave, corpo)`: a chave é
/// a URL do GET, ou `"<url> <corpo>"` no POST. O diretório tem o nome do teste e é refeito a cada execução.
#[cfg(test)]
fn rede_de_teste(nome: &str, respostas: &[(&str, &str)]) -> Rede<'static> {
    let dir = std::env::temp_dir().join(format!("auli-mt-faqs-teste-{nome}"));
    let _ = std::fs::remove_dir_all(&dir);
    let dir: &'static str = Box::leak(dir.to_string_lossy().into_owned().into_boxed_str());
    for (chave, corpo) in respostas {
        auli_scraper_kit::cache::write(dir, "faqs", chave, corpo);
    }
    Rede {
        data_dir: dir,
        use_cache: true,
        agente: auli_scraper_kit::build_agent(UA, None),
        ultima: RefCell::new(HashMap::new()),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Duvida {
    id: u64,
    desc_duvida: String,
    desc_resposta: String,
    desc_grupo_duvida: String,
    status_duvida: Status,
    /// A ordem dentro do grupo. Uma pergunta veio sem (02/11/2026): vai para o fim do grupo.
    numr_ordem_duvida: Option<u32>,
}

#[derive(Deserialize)]
struct Status {
    ativo: bool,
}

struct Pergunta {
    pergunta: String,
    resposta: String,
    /// O link da própria pergunta (o Portal MT tem uma página por pergunta); sem ele, vale o do grupo.
    url: Option<String>,
}

struct Grupo {
    nome: String,
    url: String,
    perguntas: Vec<Pergunta>,
}

/// Um ramo da árvore: o programa Nota MT, o site da SEFAZ ou o Portal MT. O título abre a trilha das perguntas.
struct Fonte {
    titulo: &'static str,
    url: String,
    grupos: Vec<Grupo>,
}

/// As dúvidas ativas, por grupo na ordem do número do grupo e, dentro dele, na ordem do portal
/// (`numrOrdemDuvida`; sem ordem vai para o fim; o `id` desempata). A API devolve tudo embaralhado:
/// ordenado, o snapshot só muda quando a FAQ muda.
fn agrupar(duvidas: Vec<Duvida>) -> Result<Vec<Grupo>> {
    let mut ids = BTreeSet::new();
    let mut inativas = 0;
    let mut por_grupo: BTreeMap<(u32, String), Vec<Duvida>> = BTreeMap::new();
    for d in duvidas {
        if !ids.insert(d.id) {
            bail!("a dúvida {} veio duas vezes — a API mudou?", d.id);
        }
        if !d.status_duvida.ativo {
            inativas += 1;
            continue;
        }
        por_grupo
            .entry(numero_e_nome(&d.desc_grupo_duvida)?)
            .or_default()
            .push(d);
    }
    if inativas > 0 {
        eprintln!("⚠️  MT FAQs: {inativas} dúvida(s) inativa(s) fora do acervo");
    }
    Ok(por_grupo
        .into_iter()
        .map(|((_, nome), mut duvidas)| {
            duvidas.sort_by_key(|d| (d.numr_ordem_duvida.is_none(), d.numr_ordem_duvida, d.id));
            Grupo {
                nome,
                url: FAQ.to_string(),
                perguntas: duvidas
                    .iter()
                    .map(|d| Pergunta {
                        pergunta: clean(&d.desc_duvida),
                        resposta: html_para_texto(&d.desc_resposta),
                        url: None,
                    })
                    .collect(),
            }
        })
        .collect())
}

/// `"3 - PRÊMIOS E ENTIDADES"` → `(3, "PRÊMIOS E ENTIDADES")`. O número é a ordem do grupo na página, não
/// parte do nome (D-MTFAQ-7).
fn numero_e_nome(grupo: &str) -> Result<(u32, String)> {
    grupo
        .split_once(" - ")
        .and_then(|(n, nome)| Some((n.trim().parse().ok()?, clean(nome))))
        .filter(|(_, nome)| !nome.is_empty())
        .ok_or_else(|| anyhow!("grupo fora do formato \"N - NOME\": {grupo:?}"))
}

/// O acervo, com as guardas que olham todas as fontes (D-MTFAQ-4).
fn consolidar(fontes: &[Fonte]) -> Result<Vec<FaqRaw>> {
    let faqs: Vec<FaqRaw> = fontes
        .iter()
        .flat_map(|f| {
            f.grupos.iter().flat_map(move |g| {
                g.perguntas.iter().map(move |p| FaqRaw {
                    pergunta: p.pergunta.clone(),
                    resposta: p.resposta.clone(),
                    origin: trilha(f, g),
                    url: p.url.clone().unwrap_or_else(|| g.url.clone()),
                })
            })
        })
        .collect();
    // Sem pergunta não há o que coletar: é a fonte que mudou, não a FAQ que acabou.
    if faqs.is_empty() {
        bail!("nenhuma pergunta ativa na FAQ — a API mudou?");
    }
    if let Some(f) = faqs.iter().find(|f| f.pergunta.is_empty()) {
        bail!("pergunta sem texto em {}", f.origin);
    }
    // O nome do `.md` sai do link e da pergunta: repetidos, um arquivo sobrescreveria o outro em silêncio. No Nota MT
    // o link é o mesmo para todas, então é a pergunta que não pode repetir.
    let mut vistas = BTreeSet::new();
    if let Some(f) = faqs
        .iter()
        .find(|f| !vistas.insert((f.url.as_str(), f.pergunta.as_str())))
    {
        bail!(
            "a pergunta {:?} aparece duas vezes (a 2ª em {}) — com o mesmo link, os .md teriam o mesmo nome",
            f.pergunta,
            f.origin
        );
    }
    let sem_resposta = faqs.iter().filter(|f| f.resposta.is_empty()).count();
    if sem_resposta > 0 {
        eprintln!("⚠️  MT FAQs: {sem_resposta} pergunta(s) com resposta vazia depois da conversão");
    }
    println!(
        "MT FAQs: {} perguntas em {} grupos",
        faqs.len(),
        fontes.iter().map(|f| f.grupos.len()).sum::<usize>()
    );
    Ok(faqs)
}

fn trilha(f: &Fonte, g: &Grupo) -> String {
    format!("{} | {}", f.titulo, g.nome)
}

/// A árvore da aba de FAQs: raiz → um `Menu` por fonte → um `Faq` por grupo. A interface começa pelos filhos da raiz,
/// e o nó da fonte dá contexto aos grupos. O item leva o link próprio quando tem (o Portal MT); sem ele, a interface
/// usa o do nó.
fn montar_arvore(fontes: &[Fonte]) -> FaqNode {
    let menu = |title: &str, url: &str, children| FaqNode {
        title: title.to_string(),
        url: url.to_string(),
        page_type: PageType::Menu,
        origin: String::new(),
        children,
        faq_items: Vec::new(),
    };
    let ramos = fontes
        .iter()
        .map(|f| {
            let grupos = f
                .grupos
                .iter()
                .map(|g| FaqNode {
                    title: g.nome.clone(),
                    url: g.url.clone(),
                    page_type: PageType::Faq,
                    origin: trilha(f, g),
                    children: Vec::new(),
                    faq_items: g
                        .perguntas
                        .iter()
                        .map(|p| FaqItem {
                            pergunta: p.pergunta.clone(),
                            resposta: p.resposta.clone(),
                            url: p.url.clone(),
                        })
                        .collect(),
                })
                .collect();
            menu(f.titulo, &f.url, grupos)
        })
        .collect();
    menu("Perguntas frequentes", FAQ, ramos)
}

/// HTML → texto pelo conversor da frota (`auli_scraper_kit::html`, D-KITHTML-1): a quebra do fonte é estrutura nas
/// respostas do Nota MT.
fn html_para_texto(s: &str) -> String {
    auli_scraper_kit::html::html_para_texto(
        s,
        auli_scraper_kit::html::Opcoes {
            base: Some(FAQ),
            quebras_do_fonte: true,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // Recorte da resposta de 02/11/2026: 7 dúvidas de 5 grupos — no grupo 3, as ordens 1 e 2 e a única
    // sem ordem (136); a 138 com lista malformada, a 540 com link, a 338 com `<b>` e espaço no fim da
    // pergunta, a 193 com espaço no começo e resposta "Sim.". Embaralhado como a API embaralha, e de modo
    // que o grupo 3 chegue de trás para frente (136, 179, 178).
    const DUVIDAS: &str = include_str!("../tests/fixtures/mt-faqs-duvidas.json");

    fn duvidas() -> Vec<Duvida> {
        serde_json::from_str(DUVIDAS).unwrap()
    }

    fn erro<T>(r: Result<T>) -> String {
        match r {
            Ok(_) => panic!("esperava Err"),
            Err(e) => format!("{e:#}"),
        }
    }

    /// O Nota MT como a única fonte.
    fn nota(grupos: Vec<Grupo>) -> [Fonte; 1] {
        [Fonte {
            titulo: PROGRAMA,
            url: FAQ.to_string(),
            grupos,
        }]
    }

    fn resposta(id: u64) -> String {
        let d = duvidas().into_iter().find(|d| d.id == id).unwrap();
        html_para_texto(&d.desc_resposta)
    }

    #[test]
    fn agrupa_pelo_numero_do_grupo_e_ordena_pela_ordem_do_portal() {
        let grupos = agrupar(duvidas()).unwrap();
        let nomes: Vec<&str> = grupos.iter().map(|g| g.nome.as_str()).collect();
        assert_eq!(
            nomes,
            [
                "PROGRAMA",
                "PRÊMIOS E ENTIDADES",
                "PAGAMENTO DOS PRÊMIOS",
                "EMPRESAS",
                "DESCONTO IPVA"
            ]
        );
        let premios: Vec<&str> = grupos[1]
            .perguntas
            .iter()
            .map(|p| p.pergunta.as_str())
            .collect();
        assert_eq!(
            premios,
            [
                "Fui sorteado. Como faço para receber meu prêmio?",
                "Quanto tempo tenho para receber meu prêmio?",
                "Como receber o prêmio?",
            ]
        );
    }

    #[test]
    fn pergunta_sai_sem_espaco_nas_pontas() {
        let grupos = agrupar(duvidas()).unwrap();
        let empresas = &grupos[3].perguntas[0];
        assert_eq!(
            empresas.pergunta,
            "Empresas do Simples Nacional estão obrigadas a aderir?"
        );
        assert_eq!(empresas.resposta, "Sim.");
        assert_eq!(
            grupos[2].perguntas[0].pergunta,
            "Onde informar os meus dados bancários?"
        );
    }

    #[test]
    fn html_para_texto_mantem_o_link_e_tira_as_tags() {
        let t = resposta(540);
        assert!(
            t.ends_with(
                "Para acessar o conteúdo do Portal do Conhecimento clique aqui \"https://sac.sefaz.mt.gov.br/citsmart/pages/knowledgeBasePortal/knowledgeBasePortal.load#/knowledge/19156\"."
            ),
            "{t}"
        );
        let t = resposta(338);
        assert!(
            t.contains("clicar em Salvar.\nAtenção: Antes de Salvar"),
            "{t}"
        );
        // Sem `>` na conta: a 338 tem "Meus Dados => Conta Bancária" no texto.
        for t in [resposta(540), resposta(338), resposta(138)] {
            assert!(!t.contains('<'), "{t}");
        }
    }

    #[test]
    fn html_para_texto_quebra_a_lista_malformada_em_linhas() {
        let t = resposta(138);
        let linhas: Vec<&str> = t.lines().collect();
        assert_eq!(
            linhas[0],
            "Participação em sorteios com prêmios em dinheiro;"
        );
        assert_eq!(linhas.len(), 7, "{t}");
        assert!(
            linhas[5].starts_with("•Menor preço, pesquisa de preços"),
            "{t}"
        );
    }

    #[test]
    fn duvida_inativa_fica_fora() {
        let mut ds = duvidas();
        ds.iter_mut()
            .find(|d| d.id == 179)
            .unwrap()
            .status_duvida
            .ativo = false;
        let grupos = agrupar(ds).unwrap();
        assert_eq!(grupos[1].perguntas.len(), 2);
        assert!(
            grupos[1]
                .perguntas
                .iter()
                .all(|p| p.pergunta != "Quanto tempo tenho para receber meu prêmio?")
        );
    }

    #[test]
    fn id_repetido_e_erro() {
        let mut ds = duvidas();
        ds.iter_mut().find(|d| d.id == 540).unwrap().id = 178;
        let e = erro(agrupar(ds));
        assert!(e.contains("a dúvida 178 veio duas vezes"), "{e}");
    }

    #[test]
    fn grupo_fora_do_formato_e_erro() {
        let mut ds = duvidas();
        ds[0].desc_grupo_duvida = "PRÊMIOS E ENTIDADES".into();
        let e = erro(agrupar(ds));
        assert!(e.contains("grupo fora do formato"), "{e}");
    }

    #[test]
    fn resposta_fora_do_formato_e_erro() {
        let e = serde_json::from_str::<Vec<Duvida>>(r#"{"erro":"indisponível"}"#);
        assert!(e.is_err());
        let sem_status = r#"[{"id":1,"descDuvida":"P?","descResposta":"R","descGrupoDuvida":"1 - G","numrOrdemDuvida":1}]"#;
        assert!(serde_json::from_str::<Vec<Duvida>>(sem_status).is_err());
    }

    #[test]
    fn todas_com_o_mesmo_link_e_a_trilha_do_programa() {
        let faqs = consolidar(&nota(agrupar(duvidas()).unwrap())).unwrap();
        assert_eq!(faqs.len(), 7);
        assert!(faqs.iter().all(|f| f.url == FAQ));
        assert_eq!(faqs[0].origin, "Nota MT | PROGRAMA");
        assert_eq!(faqs[6].origin, "Nota MT | DESCONTO IPVA");
    }

    #[test]
    fn pergunta_repetida_e_erro() {
        let mut ds = duvidas();
        ds.iter_mut().find(|d| d.id == 136).unwrap().desc_duvida =
            " Quanto tempo tenho para receber meu prêmio? ".into();
        let e = erro(consolidar(&nota(agrupar(ds).unwrap())));
        assert!(
            e.contains("aparece duas vezes") && e.contains("Nota MT | PRÊMIOS E ENTIDADES"),
            "{e}"
        );
    }

    #[test]
    fn acervo_vazio_e_erro() {
        let e = erro(consolidar(&nota(agrupar(Vec::new()).unwrap())));
        assert!(e.contains("nenhuma pergunta"), "{e}");
    }

    #[test]
    fn pergunta_sem_texto_e_erro() {
        let mut ds = duvidas();
        ds.iter_mut().find(|d| d.id == 540).unwrap().desc_duvida = "  ".into();
        let e = erro(consolidar(&nota(agrupar(ds).unwrap())));
        assert!(
            e.contains("pergunta sem texto em Nota MT | DESCONTO IPVA"),
            "{e}"
        );
    }

    /// As três fontes: o Nota MT da fixture; um grupo do site, cujas perguntas levam o link da página; e um do Portal
    /// MT, com a página própria da pergunta — que repete o texto de uma do Nota MT, mas em outro link.
    fn tres_fontes() -> [Fonte; 3] {
        let grupo = |nome: &str, url: String, pergunta: &str, propria: Option<String>| Grupo {
            nome: nome.to_string(),
            url,
            perguntas: vec![Pergunta {
                pergunta: pergunta.to_string(),
                resposta: "R.".to_string(),
                url: propria,
            }],
        };
        [
            nota(agrupar(duvidas()).unwrap())
                .into_iter()
                .next()
                .unwrap(),
            Fonte {
                titulo: site::TITULO,
                url: site::TODOS.to_string(),
                grupos: vec![grupo(
                    "Documentos Fiscais",
                    format!("{}?c=1&e=2", site::TODOS),
                    "O que é a NFA-e?",
                    None,
                )],
            },
            Fonte {
                titulo: portal::TITULO,
                url: portal::FAQ.to_string(),
                grupos: vec![grupo(
                    "Institucional",
                    portal::FAQ.to_string(),
                    "Como receber o prêmio?",
                    Some(format!("{}/abc", portal::FAQ)),
                )],
            },
        ]
    }

    #[test]
    fn consolidar_junta_as_tres_fontes_com_a_trilha_e_o_link_de_cada_uma() {
        let faqs = consolidar(&tres_fontes()).unwrap();
        assert_eq!(faqs.len(), 9);
        let fim: Vec<(&str, &str)> = faqs[7..]
            .iter()
            .map(|f| (f.origin.as_str(), f.url.as_str()))
            .collect();
        assert_eq!(
            fim,
            [
                (
                    "SEFAZ-MT | Documentos Fiscais",
                    "https://www5.sefaz.mt.gov.br/servicos?c=1&e=2"
                ),
                (
                    "Portal MT | Institucional",
                    "https://portal.mt.gov.br/perguntas-frequentes/abc"
                ),
            ]
        );
    }

    #[test]
    fn arvore_tem_um_ramo_por_fonte() {
        let raiz = montar_arvore(&tres_fontes());
        let ramos: Vec<(&str, &str, PageType)> = raiz
            .children
            .iter()
            .map(|r| (r.title.as_str(), r.url.as_str(), r.page_type))
            .collect();
        assert_eq!(
            ramos,
            [
                ("Nota MT", FAQ, PageType::Menu),
                ("SEFAZ-MT", site::TODOS, PageType::Menu),
                ("Portal MT", portal::FAQ, PageType::Menu),
            ]
        );
        let site = &raiz.children[1].children[0];
        assert_eq!(
            (site.url.as_str(), site.origin.as_str()),
            (
                "https://www5.sefaz.mt.gov.br/servicos?c=1&e=2",
                "SEFAZ-MT | Documentos Fiscais"
            )
        );
        assert_eq!(site.faq_items[0].url, None);
        let portal = &raiz.children[2].children[0];
        assert_eq!(portal.page_type, PageType::Faq);
        assert_eq!(
            portal.faq_items[0].url.as_deref(),
            Some("https://portal.mt.gov.br/perguntas-frequentes/abc")
        );
    }

    #[test]
    fn arvore_tem_programa_grupos_e_perguntas() {
        let raiz = montar_arvore(&nota(agrupar(duvidas()).unwrap()));
        assert_eq!((raiz.page_type, raiz.children.len()), (PageType::Menu, 1));
        let programa = &raiz.children[0];
        assert_eq!(
            (programa.title.as_str(), programa.page_type),
            ("Nota MT", PageType::Menu)
        );
        let premios = &programa.children[1];
        assert_eq!(premios.page_type, PageType::Faq);
        assert_eq!(premios.url, FAQ);
        assert_eq!(premios.origin, "Nota MT | PRÊMIOS E ENTIDADES");
        assert_eq!(premios.faq_items.len(), 3);
        assert!(premios.faq_items.iter().all(|i| i.url.is_none()));
    }
}
