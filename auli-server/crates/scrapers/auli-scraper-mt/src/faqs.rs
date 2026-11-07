// SEFAZ-MT FAQs — a FAQ do programa Nota MT (`www.sefaz.mt.gov.br/notamt/duvida`), aplicação Angular
// sobre uma API REST pública, à parte do X-Via dos serviços (`mt.rs`). Estudo, medições e decisões:
// docs/ESTUDO-MT-Faqs.md (D-MTFAQ-*).
//
// Uma chamada só: `GET /notamt/api/duvidas` (sem sessão) devolve todas as perguntas, com a resposta
// inteira, o grupo ("3 - PRÊMIOS E ENTIDADES"), a ordem dentro do grupo e o status. A aplicação agrupa e
// mostra num acordeão; não tem rota nem âncora por pergunta.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::Duration;

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

/// A identidade das FAQs da frota (D-MTFAQ-9); o host não tem `robots.txt` alcançável (D-MTFAQ-3).
const UA: &str =
    "AuliBot/0.1 (+https://github.com/oxschellen/auli; carlos.schellenberger@gmail.com)";

/// Coleta as FAQs e grava a árvore do frontend (`<id>-faqs-tree.json`) e o snapshot. As guardas
/// (D-MTFAQ-4) rodam antes de qualquer gravação: rodada que falha deixa os arquivos anteriores intactos.
pub fn run(data_dir: &str, use_cache: bool) -> Result<()> {
    let duvidas: Vec<Duvida> = serde_json::from_str(&buscar(data_dir, use_cache)?)
        .context("resposta da API fora do formato [{id, descDuvida, descResposta, …}]")?;
    let grupos = agrupar(duvidas)?;
    for g in &grupos {
        println!("MT FAQs: {}: {} perguntas", g.nome, g.perguntas.len());
    }
    let faqs = consolidar(&grupos)?;

    // Atômico pelo mesmo motivo das outras entidades: a árvore não tem outra fonte (o snapshot achatado
    // não a reconstrói) e é o único artefato de coleta que o frontend lê direto.
    let tree_path = format!("{data_dir}/{}-faqs-tree.json", crate::ENTITY);
    auli_contract::arquivo::escrever_atomico(
        Path::new(&tree_path),
        &serde_json::to_string_pretty(&montar_arvore(&grupos))?,
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

/// A chamada à API (D-MTFAQ-5): rede sempre, gravando o cache; com `--usecache`, só o cache, e miss é
/// erro.
fn buscar(data_dir: &str, use_cache: bool) -> Result<String> {
    if use_cache {
        return auli_scraper_kit::cache::read(data_dir, "faqs", API)
            .ok_or_else(|| anyhow!("cache miss para {API} (modo --usecache, sem rede)"));
    }
    let agent = auli_scraper_kit::build_agent(UA, Some(Duration::from_secs(60)));
    let resposta = auli_scraper_kit::http::get_string(
        &agent,
        API,
        &GetOpts {
            log_prefix: "MT FAQs",
            accept: Some("application/json"),
            ..Default::default()
        },
    )?;
    auli_scraper_kit::cache::write(data_dir, "faqs", API, &resposta);
    Ok(resposta)
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
}

struct Grupo {
    nome: String,
    perguntas: Vec<Pergunta>,
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
                perguntas: duvidas
                    .iter()
                    .map(|d| Pergunta {
                        pergunta: clean(&d.desc_duvida),
                        resposta: html_para_texto(&d.desc_resposta),
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

/// O acervo, com as guardas que olham todos os grupos (D-MTFAQ-4).
fn consolidar(grupos: &[Grupo]) -> Result<Vec<FaqRaw>> {
    let faqs: Vec<FaqRaw> = grupos
        .iter()
        .flat_map(|g| {
            g.perguntas.iter().map(move |p| FaqRaw {
                pergunta: p.pergunta.clone(),
                resposta: p.resposta.clone(),
                origin: trilha(g),
                url: FAQ.to_string(),
            })
        })
        .collect();
    // Sem pergunta não há o que coletar: é a API que mudou, não a FAQ que acabou.
    if faqs.is_empty() {
        bail!("nenhuma pergunta ativa na FAQ — a API mudou?");
    }
    if let Some(f) = faqs.iter().find(|f| f.pergunta.is_empty()) {
        bail!("pergunta sem texto em {}", f.origin);
    }
    // O link é o mesmo para todas, então o nome do `.md` (pergunta + hash de link e pergunta) sai só da
    // pergunta: repetida, um arquivo sobrescreveria o outro em silêncio.
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
        eprintln!("⚠️  MT FAQs: {sem_resposta} pergunta(s) com resposta vazia depois da conversão");
    }
    println!(
        "MT FAQs: {} perguntas em {} grupos",
        faqs.len(),
        grupos.len()
    );
    Ok(faqs)
}

fn trilha(g: &Grupo) -> String {
    format!("{PROGRAMA} | {}", g.nome)
}

/// A árvore da aba de FAQs: raiz → o programa → um `Faq` por grupo. A interface começa pelos filhos da
/// raiz, e o nó do programa dá contexto aos grupos. Tudo aponta para a mesma página — a aplicação não tem
/// rota por grupo nem por pergunta —, e os itens ficam sem `url` (a interface usa a do nó).
fn montar_arvore(grupos: &[Grupo]) -> FaqNode {
    let menu = |title: &str, children| FaqNode {
        title: title.to_string(),
        url: FAQ.to_string(),
        page_type: PageType::Menu,
        origin: String::new(),
        children,
        faq_items: Vec::new(),
    };
    let grupos = grupos
        .iter()
        .map(|g| FaqNode {
            title: g.nome.clone(),
            url: FAQ.to_string(),
            page_type: PageType::Faq,
            origin: trilha(g),
            children: Vec::new(),
            faq_items: g
                .perguntas
                .iter()
                .map(|p| FaqItem {
                    pergunta: p.pergunta.clone(),
                    resposta: p.resposta.clone(),
                    url: None,
                })
                .collect(),
        })
        .collect();
    menu("Perguntas frequentes", vec![menu(PROGRAMA, grupos)])
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
        let faqs = consolidar(&agrupar(duvidas()).unwrap()).unwrap();
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
        let e = erro(consolidar(&agrupar(ds).unwrap()));
        assert!(
            e.contains("aparece duas vezes") && e.contains("Nota MT | PRÊMIOS E ENTIDADES"),
            "{e}"
        );
    }

    #[test]
    fn acervo_vazio_e_erro() {
        let e = erro(consolidar(&agrupar(Vec::new()).unwrap()));
        assert!(e.contains("nenhuma pergunta"), "{e}");
    }

    #[test]
    fn pergunta_sem_texto_e_erro() {
        let mut ds = duvidas();
        ds.iter_mut().find(|d| d.id == 540).unwrap().desc_duvida = "  ".into();
        let e = erro(consolidar(&agrupar(ds).unwrap()));
        assert!(
            e.contains("pergunta sem texto em Nota MT | DESCONTO IPVA"),
            "{e}"
        );
    }

    #[test]
    fn arvore_tem_programa_grupos_e_perguntas() {
        let raiz = montar_arvore(&agrupar(duvidas()).unwrap());
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
