// SEFAZ-MS FAQs — a FAQ da Fale Conosco MS (`www.faleconosco.ms.gov.br/faq`), aplicação AngularJS sobre
// uma API REST pública, à parte do site WordPress dos serviços (`ms.rs`). Estudo, medições e decisões:
// docs/faqs/ESTUDO-MS-Faqs.md (D-MSFAQ-*).
//
// A API (POST `recursos/faq/<ação>`, corpo `[{key, value}]`, sem sessão) tem os três níveis da navegação:
// assuntos (`listassunto`, com a contagem `qtdeFaq` do próprio portal) → subtemas
// (`listdetalhamentoassunto`) → perguntas (`listfaqpordetalhamentoassunto`). A lista de perguntas traz só
// uma PRÉVIA da resposta (~100 caracteres, cortada no meio da tag): a resposta inteira vem de
// `getrespostafaq`, uma chamada por pergunta — o que a aplicação faz quando se clica na pergunta.

use std::collections::BTreeSet;
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use scraper::Html;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::json;
use ureq::Agent;

use auli_contract::FaqRaw;
use auli_scraper_kit::clean;
use auli_scraper_kit::faqtree::{FaqItem, FaqNode, PageType};
use auli_scraper_kit::http::GetOpts;

const API: &str = "https://www.faleconosco.ms.gov.br/faq/recursos/faq";
const FAQ: &str = "https://www.faleconosco.ms.gov.br/faq/";

/// A identidade das FAQs da frota (D-MSFAQ-4): o portal não barra robô e o `robots.txt` não existe.
const UA: &str =
    "AuliBot/0.1 (+https://github.com/oxschellen/auli; carlos.schellenberger@gmail.com)";
const COURTESY: Duration = Duration::from_millis(1000);

/// Coleta as FAQs e grava a árvore do frontend (`<id>-faqs-tree.json`) e o snapshot. As guardas
/// (D-MSFAQ-3) rodam antes de qualquer gravação: rodada que falha deixa os arquivos anteriores intactos.
pub fn run(data_dir: &str, use_cache: bool) -> Result<()> {
    let agent = auli_scraper_kit::build_agent(UA, Some(Duration::from_secs(60)));
    let mut assuntos: Vec<Assunto> =
        lista(&buscar(data_dir, &agent, "listassunto", &[], use_cache)?)?;
    if assuntos.is_empty() {
        bail!("a FAQ não listou nenhum assunto — a API mudou?");
    }
    // A API devolve os assuntos numa ordem própria; pelo nome, o snapshot só muda quando a FAQ muda.
    assuntos.sort_by_key(|a| a.nome.to_lowercase());
    println!("MS FAQs: {} assuntos", assuntos.len());

    let mut coletados = Vec::new();
    for a in assuntos {
        let id_assunto = a.id.to_string();
        let mut subtemas: Vec<Subtema> = lista(&buscar(
            data_dir,
            &agent,
            "listdetalhamentoassunto",
            &[("idAssunto", &id_assunto)],
            use_cache,
        )?)?;
        subtemas.sort_by_key(|s| s.descricao.to_lowercase());

        let mut com_perguntas = Vec::new();
        for s in subtemas {
            let id_subtema = s.id.to_string();
            let mut previas: Vec<Previa> = lista(&buscar(
                data_dir,
                &agent,
                "listfaqpordetalhamentoassunto",
                &[
                    ("idAssunto", &id_assunto),
                    ("idDetalhamentoAssunto", &id_subtema),
                ],
                use_cache,
            )?)?;
            previas.sort_by_key(|p| p.id);
            let mut perguntas = Vec::new();
            for p in previas {
                let respostas: Vec<String> = lista(&buscar(
                    data_dir,
                    &agent,
                    "getrespostafaq",
                    &[("idFaq", &p.id.to_string())],
                    use_cache,
                )?)?;
                perguntas.push(Pergunta {
                    id: p.id,
                    pergunta: uma_linha(&p.pergunta),
                    resposta: html_para_texto(respostas.first().map_or("", String::as_str)),
                });
            }
            if perguntas.is_empty() {
                eprintln!(
                    "⚠️  MS FAQs: {} | {}: subtema sem perguntas",
                    clean(&a.nome),
                    clean(&s.descricao)
                );
                continue;
            }
            com_perguntas.push(SubtemaFaq {
                id: s.id,
                nome: clean(&s.descricao),
                perguntas,
            });
        }

        let assunto = AssuntoFaq {
            id: a.id,
            nome: clean(&a.nome),
            subtemas: com_perguntas,
        };
        conferir_contagem(&assunto, a.qtde_faq.unwrap_or(0))?;
        println!(
            "MS FAQs: {}: {} perguntas em {} subtemas",
            assunto.nome,
            assunto.perguntas().count(),
            assunto.subtemas.len()
        );
        if !assunto.subtemas.is_empty() {
            coletados.push(assunto);
        }
    }
    let faqs = consolidar(&coletados)?;

    // Atômico pelo mesmo motivo das outras entidades: a árvore não tem outra fonte (o snapshot achatado
    // não a reconstrói) e é o único artefato de coleta que o frontend lê direto.
    let tree_path = format!("{data_dir}/{}-faqs-tree.json", crate::ENTITY);
    auli_contract::arquivo::escrever_atomico(
        Path::new(&tree_path),
        &serde_json::to_string_pretty(&montar_arvore(&coletados))?,
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

/// Uma chamada à API (D-MSFAQ-4): rede sempre, gravando o cache sob uma url com os parâmetros na query
/// (o corpo do POST não entra na chave do kit); com `--usecache`, só o cache, e miss é erro.
fn buscar(
    data_dir: &str,
    agent: &Agent,
    acao: &str,
    params: &[(&str, &str)],
    use_cache: bool,
) -> Result<String> {
    let query: Vec<String> = params.iter().map(|(k, v)| format!("{k}={v}")).collect();
    let chave = format!("{API}/{acao}?{}", query.join("&"));
    if use_cache {
        return auli_scraper_kit::cache::read(data_dir, "faqs", &chave)
            .ok_or_else(|| anyhow!("cache miss para {chave} (modo --usecache, sem rede)"));
    }
    sleep(COURTESY);
    let corpo: Vec<_> = params
        .iter()
        .map(|(k, v)| json!({ "key": k, "value": v }))
        .collect();
    let resposta = auli_scraper_kit::http::post_json(
        agent,
        &format!("{API}/{acao}"),
        &[],
        &json!(corpo),
        &GetOpts {
            log_prefix: "MS FAQs",
            accept: Some("application/json"),
            ..Default::default()
        },
    )?;
    auli_scraper_kit::cache::write(data_dir, "faqs", &chave, &resposta);
    Ok(resposta)
}

/// Toda resposta da API é `{"data": [...]}`.
fn lista<T: DeserializeOwned>(json: &str) -> Result<Vec<T>> {
    #[derive(Deserialize)]
    struct Envelope<T> {
        data: Vec<T>,
    }
    Ok(serde_json::from_str::<Envelope<T>>(json)
        .context("resposta da API fora do formato {\"data\": [...]}")?
        .data)
}

#[derive(Deserialize)]
struct Assunto {
    id: u32,
    nome: String,
    #[serde(rename = "qtdeFaq")]
    qtde_faq: Option<usize>,
}

#[derive(Deserialize)]
struct Subtema {
    id: u32,
    descricao: String,
}

/// Um item da lista de perguntas: a resposta que vem junto é a prévia cortada e não é lida.
#[derive(Deserialize)]
struct Previa {
    id: u64,
    pergunta: String,
}

struct Pergunta {
    id: u64,
    pergunta: String,
    resposta: String,
}

struct SubtemaFaq {
    id: u32,
    nome: String,
    perguntas: Vec<Pergunta>,
}

struct AssuntoFaq {
    id: u32,
    nome: String,
    subtemas: Vec<SubtemaFaq>,
}

impl AssuntoFaq {
    fn perguntas(&self) -> impl Iterator<Item = (&SubtemaFaq, &Pergunta)> {
        self.subtemas
            .iter()
            .flat_map(|s| s.perguntas.iter().map(move |p| (s, p)))
    }
}

/// (b) A contagem é do próprio portal: perguntas únicas do assunto == `qtdeFaq` (1.200 de 1.200 em
/// 02/11/2026, em todos os 71 assuntos). Menos é subtema que a coleta não alcançou.
fn conferir_contagem(a: &AssuntoFaq, qtde_faq: usize) -> Result<()> {
    let unicas: BTreeSet<u64> = a.perguntas().map(|(_, p)| p.id).collect();
    if unicas.len() != qtde_faq {
        bail!(
            "{}: {} perguntas coletadas, o portal conta {qtde_faq} — subtema que a coleta não alcançou?",
            a.nome,
            unicas.len()
        );
    }
    Ok(())
}

/// O acervo, com as guardas que olham todos os assuntos (D-MSFAQ-3).
fn consolidar(assuntos: &[AssuntoFaq]) -> Result<Vec<FaqRaw>> {
    let faqs: Vec<FaqRaw> = assuntos
        .iter()
        .flat_map(|a| a.perguntas().map(move |(s, p)| (a, s, p)))
        .map(|(a, s, p)| FaqRaw {
            pergunta: p.pergunta.clone(),
            resposta: p.resposta.clone(),
            origin: trilha(a, s),
            url: link_da_pergunta(a, s, p),
        })
        .collect();
    // (c) Sem pergunta não há o que coletar: é a API que mudou, não a FAQ que acabou.
    if faqs.is_empty() {
        bail!("nenhuma pergunta na FAQ — a API mudou?");
    }
    // (d) Pergunta sem texto não é documento; resposta vazia só avisa (título e link ainda servem).
    if let Some(f) = faqs.iter().find(|f| f.pergunta.is_empty()) {
        bail!("pergunta sem texto em {} ({})", f.origin, f.url);
    }
    let sem_resposta = faqs.iter().filter(|f| f.resposta.is_empty()).count();
    if sem_resposta > 0 {
        eprintln!("⚠️  MS FAQs: {sem_resposta} pergunta(s) com resposta vazia depois da conversão");
    }
    println!(
        "MS FAQs: {} perguntas em {} assuntos",
        faqs.len(),
        assuntos.len()
    );
    Ok(faqs)
}

/// A página do subtema: a aplicação não tem rota por pergunta — ela abre com um clique.
fn link_do_subtema(a: &AssuntoFaq, s: &SubtemaFaq) -> String {
    format!("{FAQ}#/detalhamentoAssunto/{}/{}", a.id, s.id)
}

/// A página do subtema com o `id` da pergunta (D-MSFAQ-2). A aplicação (AngularJS 1.2) casa a rota só
/// pelo caminho e ignora o `?idFaq=`, então o link abre o subtema; o parâmetro dá a cada pergunta um link
/// próprio. Precisa: o portal tem a mesma pergunta duas vezes no mesmo subtema, em versões com respostas
/// diferentes (2 casos em 02/11/2026), e o nome do `.md` sai de link + pergunta.
fn link_da_pergunta(a: &AssuntoFaq, s: &SubtemaFaq, p: &Pergunta) -> String {
    format!("{}?idFaq={}", link_do_subtema(a, s), p.id)
}

fn trilha(a: &AssuntoFaq, s: &SubtemaFaq) -> String {
    format!("{} | {}", a.nome, s.nome)
}

/// A árvore da aba de FAQs: raiz → um `Menu` por assunto → um `Faq` por subtema.
fn montar_arvore(assuntos: &[AssuntoFaq]) -> FaqNode {
    FaqNode {
        title: "Perguntas frequentes".to_string(),
        url: format!("{FAQ}#/home"),
        page_type: PageType::Menu,
        origin: String::new(),
        children: assuntos
            .iter()
            .map(|a| FaqNode {
                title: a.nome.clone(),
                url: format!("{FAQ}#/assunto/{}", a.id),
                page_type: PageType::Menu,
                origin: String::new(),
                children: a
                    .subtemas
                    .iter()
                    .map(|s| FaqNode {
                        title: s.nome.clone(),
                        url: link_do_subtema(a, s),
                        page_type: PageType::Faq,
                        origin: trilha(a, s),
                        children: Vec::new(),
                        faq_items: s
                            .perguntas
                            .iter()
                            .map(|p| FaqItem {
                                pergunta: p.pergunta.clone(),
                                resposta: p.resposta.clone(),
                                url: Some(link_da_pergunta(a, s, p)),
                            })
                            .collect(),
                    })
                    .collect(),
                faq_items: Vec::new(),
            })
            .collect(),
        faq_items: Vec::new(),
    }
}

/// HTML → texto pelo conversor da frota (`auli_scraper_kit::html`, D-KITHTML-1): a quebra do fonte é estrutura nas
/// respostas do Fale Conosco.
fn html_para_texto(s: &str) -> String {
    auli_scraper_kit::html::html_para_texto(
        s,
        auli_scraper_kit::html::Opcoes {
            base: Some(FAQ),
            quebras_do_fonte: true,
        },
    )
}

/// Um trecho curto (pergunta, texto de link) numa linha só, sem tag.
fn uma_linha(s: &str) -> String {
    clean(&texto_do_html(s))
}

fn texto_do_html(s: &str) -> String {
    Html::parse_fragment(s).root_element().text().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Recortes da coleta de 02/11/2026: a lista de assuntos (CTe, IPVA, NFGas), os subtemas do CTe, a
    // lista do subtema "Credenciamento" do CTe — com a mesma pergunta duas vezes (ids 49 e 10154) —, a
    // resposta 49 (links e entidades nomeadas) e a 11285 (o começo, com os comentários condicionais do
    // Word que trazem `<xml>` e `<style>`).
    const ASSUNTOS: &str = include_str!("../tests/fixtures/ms-faqs-assuntos.json");
    const SUBTEMAS: &str = include_str!("../tests/fixtures/ms-faqs-subtemas-cte.json");
    const PERGUNTAS: &str = include_str!("../tests/fixtures/ms-faqs-perguntas-cte-30.json");
    const RESPOSTA_49: &str = include_str!("../tests/fixtures/ms-faqs-resposta-49.json");
    const RESPOSTA_WORD: &str = include_str!("../tests/fixtures/ms-faqs-resposta-word.json");

    fn erro<T>(r: Result<T>) -> String {
        match r {
            Ok(_) => panic!("esperava Err"),
            Err(e) => format!("{e:#}"),
        }
    }

    fn resposta(json: &str) -> String {
        html_para_texto(&lista::<String>(json).unwrap()[0])
    }

    /// O assunto CTe com o subtema "Credenciamento" e as três perguntas do recorte.
    fn cte() -> AssuntoFaq {
        let perguntas = lista::<Previa>(PERGUNTAS)
            .unwrap()
            .into_iter()
            .map(|p| Pergunta {
                id: p.id,
                pergunta: uma_linha(&p.pergunta),
                resposta: resposta(RESPOSTA_49),
            })
            .collect();
        AssuntoFaq {
            id: 2,
            nome: "CTe".into(),
            subtemas: vec![SubtemaFaq {
                id: 30,
                nome: "Credenciamento".into(),
                perguntas,
            }],
        }
    }

    #[test]
    fn lista_le_assuntos_subtemas_perguntas_e_resposta() {
        let a: Vec<Assunto> = lista(ASSUNTOS).unwrap();
        let resumo: Vec<(u32, &str, Option<usize>)> = a
            .iter()
            .map(|x| (x.id, x.nome.as_str(), x.qtde_faq))
            .collect();
        assert_eq!(
            resumo,
            [
                (2, "CTe", Some(106)),
                (13, "IPVA", Some(20)),
                (86, "NFGas", Some(1))
            ]
        );
        let s: Vec<Subtema> = lista(SUBTEMAS).unwrap();
        assert_eq!((s[0].id, s[0].descricao.as_str()), (30, "Credenciamento"));
        let p: Vec<Previa> = lista(PERGUNTAS).unwrap();
        assert_eq!(
            p.iter().map(|x| x.id).collect::<Vec<_>>(),
            [49, 10154, 10157]
        );
        assert!(lista::<String>(RESPOSTA_49).unwrap()[0].starts_with("<p>"));
    }

    #[test]
    fn resposta_fora_do_formato_e_erro() {
        let e = erro(lista::<Assunto>(r#"{"erro":"sessão expirada"}"#));
        assert!(e.contains("fora do formato"), "{e}");
    }

    #[test]
    fn html_para_texto_decodifica_as_entidades_nomeadas_e_mantem_os_links() {
        let t = resposta(RESPOSTA_49);
        assert!(
            t.starts_with("Desde 08/10/2013, o contribuinte deverá realizar"),
            "{t}"
        );
        assert!(
            t.contains(
                "banner Credenciamento CT-e \"https://ww2.dfe.ms.gov.br/cte-credenciamento/certificado-cte/index.jsf\""
            ),
            "{t}"
        );
        assert!(
            t.contains("\n· Requisitos para ser emitente de CT-e:"),
            "{t}"
        );
        assert!(!t.contains('&') && !t.contains('<'), "{t}");
    }

    #[test]
    fn html_para_texto_descarta_os_comentarios_do_word() {
        let t = resposta(RESPOSTA_WORD);
        assert!(
            t.starts_with("Para assinar o DTE, você precisa ter uma conta gov.br"),
            "{t}"
        );
        assert!(
            t.contains("\nClique no card \"Cadastrar DTE\" ou \"Atualizar DTE\"."),
            "{t}"
        );
        for lixo in ["mso-", "MsoNormalTable", "OfficeDocumentSettings", "{"] {
            assert!(!t.contains(lixo), "sobrou {lixo}: {t}");
        }
    }

    #[test]
    fn contagem_abaixo_da_do_portal_e_erro() {
        let a = cte();
        conferir_contagem(&a, 3).unwrap();
        let e = erro(conferir_contagem(&a, 4));
        assert!(e.contains("3 perguntas coletadas, o portal conta 4"), "{e}");
    }

    #[test]
    fn a_mesma_pergunta_duas_vezes_no_subtema_ganha_dois_links() {
        let faqs = consolidar(&[cte()]).unwrap();
        let links: Vec<&str> = faqs.iter().map(|f| f.url.as_str()).collect();
        assert_eq!(
            links,
            [
                "https://www.faleconosco.ms.gov.br/faq/#/detalhamentoAssunto/2/30?idFaq=49",
                "https://www.faleconosco.ms.gov.br/faq/#/detalhamentoAssunto/2/30?idFaq=10154",
                "https://www.faleconosco.ms.gov.br/faq/#/detalhamentoAssunto/2/30?idFaq=10157",
            ]
        );
        assert_eq!(faqs[0].pergunta, faqs[1].pergunta);
        assert!(faqs.iter().all(|f| f.origin == "CTe | Credenciamento"));
    }

    #[test]
    fn acervo_vazio_e_erro() {
        let e = erro(consolidar(&[]));
        assert!(e.contains("nenhuma pergunta"), "{e}");
    }

    #[test]
    fn pergunta_sem_texto_e_erro() {
        let mut a = cte();
        a.subtemas[0].perguntas[2].pergunta = String::new();
        let e = erro(consolidar(&[a]));
        assert!(
            e.contains("pergunta sem texto") && e.contains("idFaq=10157"),
            "{e}"
        );
    }

    #[test]
    fn arvore_tem_assunto_subtema_e_perguntas() {
        let raiz = montar_arvore(&[cte()]);
        assert_eq!(raiz.url, "https://www.faleconosco.ms.gov.br/faq/#/home");
        let assunto = &raiz.children[0];
        assert_eq!(
            (assunto.title.as_str(), assunto.url.as_str()),
            ("CTe", "https://www.faleconosco.ms.gov.br/faq/#/assunto/2")
        );
        let subtema = &assunto.children[0];
        assert_eq!(subtema.page_type, PageType::Faq);
        assert_eq!(
            subtema.url,
            "https://www.faleconosco.ms.gov.br/faq/#/detalhamentoAssunto/2/30"
        );
        assert_eq!(subtema.origin, "CTe | Credenciamento");
        assert_eq!(subtema.faq_items.len(), 3);
        assert_eq!(
            subtema.faq_items[1].url.as_deref(),
            Some("https://www.faleconosco.ms.gov.br/faq/#/detalhamentoAssunto/2/30?idFaq=10154")
        );
    }
}
