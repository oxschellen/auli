// SEF-MG FAQs, 2ª fonte — a base "SRE - Dúvidas Frequentes" da Base de Conhecimento do CSM
// (`atendimento2.fazenda.mg.gov.br/csm`, o mesmo ServiceNow dos serviços do `mg.rs`). Estudo, medições e
// decisões: docs/ESTUDO-MG-Faqs.md §11 (D-MGFAQ-13 a 15).
//
// ⚠️ ROBOTS/ACESSO: o host declara `robots.txt: Disallow: /`. A coleta desta base pública é decisão do
// mantenedor (02/11/2026, docs/auli_pendencias.md §42) — não há autorização da SEF-MG. Mitigações:
// cortesia de 1 s sem paralelismo, cache, 64 requisições por rodada. Duas exceções à D-PA-ROBOTS, ambas
// impostas pelo portal: o UA do kit (o ServiceNow marca o AuliBot como `isCrawlerBot` e omite o texto do
// artigo — os serviços já usam o mesmo UA neste host) e a sessão anônima abaixo.
//
// Fluxo:
//   1. a lista: a página `kb_search` não traz os resultados — o widget "Knowledge Result" os pede num
//      POST que exige o cookie e o token `g_ck` que a página HTML entrega a qualquer visitante, sem login.
//      Um GET da página abre a sessão; o POST, filtrado pela base, devolve os artigos de uma vez;
//   2. cada artigo pela page API (`kb_article_view`, sem sessão, como no `mg.rs`), widget
//      `edx_article_header`.
// A natureza do artigo vem da categoria (`NATUREZAS`): rejeição de DF-e (o título é a pergunta e a seção
// "Resposta para o Erro" é a resposta), a compilação do PED (uma tabela de pares P./R., uma pergunta por
// par) e as orientações da DOLT/SUTRI (documentos de 4 a 65 mil caracteres — não são FAQ: ficam fora, sem
// baixar).

use std::collections::BTreeSet;
use std::sync::LazyLock;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use regex::Regex;
use serde_json::{Value, json};
use ureq::Agent;

use auli_contract::FaqRaw;
use auli_scraper_kit::clean;
use auli_scraper_kit::faqtree::{FaqItem, FaqNode, PageType};
use auli_scraper_kit::http::GetOpts;

use crate::faqs::{html_para_texto, uma_linha};
use crate::mg::{BASE, PORTAL_ID, widget_data};

/// sys_id da base "SRE - Dúvidas Frequentes". A outra base com artigos, "SRE - Serviços públicos"
/// (`bccaaf011bbec61029f686afe54bcbfc`), é a dos serviços do `mg.rs` — sem o filtro, a busca devolve as duas.
const KB: &str = "f18a2fc11bbec61029f686afe54bcbed";
const NOME_DA_BASE: &str = "SRE - Dúvidas Frequentes";
/// sys_id do widget "Knowledge Result" da página `kb_search`.
const WIDGET_RESULTADOS: &str = "ff26f2b1db12220099f93691f0b8f5ce";
/// Janela do POST. A base tinha 81 artigos em 02/11/2026; passar disso é `Err` em `parse_lista`.
const MAX_ARTIGOS: usize = 100;
const COURTESY: Duration = Duration::from_millis(1000);
const LOG: &str = "MG FAQs (CSM)";

#[derive(Clone, Copy, Debug, PartialEq)]
enum Natureza {
    Rejeicao,
    Compilacao,
    Orientacao,
}

/// As categorias da base, fechadas (D-MGFAQ-14): categoria nova é `Err` — a natureza dela é decisão, não
/// palpite. A ordem é a da árvore.
const NATUREZAS: [(&str, Natureza); 3] = [
    ("DF-e - Correção de Erros / Rejeições", Natureza::Rejeicao),
    ("Sintegra - PED", Natureza::Compilacao),
    ("Orientações Tributárias DOLT/SUTRI", Natureza::Orientacao),
];

/// Um artigo da lista. `categoria` já limpa: o portal escreve "DF-e  - Correção…", com dois espaços.
#[derive(Debug)]
struct Listado {
    sys_id: String,
    titulo: String,
    categoria: String,
}

/// O que a coleta lê da página de um artigo.
struct Artigo {
    numero: String,
    titulo: String,
    /// `kbContentData.data`: as seções do modelo (rejeição) ou o HTML do corpo (texto livre).
    conteudo: Value,
}

pub(crate) struct Pergunta {
    pergunta: String,
    resposta: String,
    url: String,
}

/// Uma categoria coletada, com as perguntas na ordem da árvore.
pub(crate) struct Categoria {
    nome: String,
    perguntas: Vec<Pergunta>,
}

/// Coleta a base. Nada é gravado aqui além do cache: as guardas rodam antes da escrita do `faqs::run`.
pub(crate) fn coletar(data_dir: &str, use_cache: bool) -> Result<Vec<Categoria>> {
    let agent =
        auli_scraper_kit::build_agent(auli_scraper_kit::USER_AGENT, Some(Duration::from_secs(60)));
    let listados = parse_lista(&buscar_lista(data_dir, &agent, use_cache)?)?;
    println!("{LOG}: {} artigos na base {NOME_DA_BASE}", listados.len());

    let mut categorias = Vec::new();
    for (nome, natureza) in NATUREZAS {
        let da_categoria: Vec<&Listado> = listados.iter().filter(|l| l.categoria == nome).collect();
        if natureza == Natureza::Orientacao {
            println!(
                "{LOG}: {nome}: {} artigos fora — orientação tributária, não FAQ (D-MGFAQ-14)",
                da_categoria.len()
            );
            continue;
        }
        let mut artigos = Vec::new();
        for l in &da_categoria {
            artigos.push(parse_artigo(
                &buscar_artigo(data_dir, &agent, &l.sys_id, use_cache)?,
                l,
            )?);
        }
        let perguntas = perguntas_da_categoria(natureza, &artigos)?;
        if perguntas.is_empty() {
            eprintln!("⚠️  {LOG}: {nome}: nenhum artigo na lista — a categoria saiu da base?");
            continue;
        }
        println!(
            "MG FAQs: {NOME_DA_BASE} | {nome}: {} perguntas de {} artigos",
            perguntas.len(),
            da_categoria.len()
        );
        categorias.push(Categoria {
            nome: nome.to_string(),
            perguntas,
        });
    }
    if categorias.is_empty() {
        bail!("nenhuma pergunta na base {NOME_DA_BASE} — a lista ou as categorias mudaram?");
    }
    Ok(categorias)
}

/// As perguntas de uma categoria, na ordem da árvore.
fn perguntas_da_categoria(natureza: Natureza, artigos: &[Artigo]) -> Result<Vec<Pergunta>> {
    let mut perguntas = Vec::new();
    for a in artigos {
        match natureza {
            Natureza::Rejeicao => perguntas.push(rejeicao(a)?),
            Natureza::Compilacao => perguntas.extend(compilacao(a)?),
            Natureza::Orientacao => unreachable!("as orientações não são baixadas"),
        }
    }
    // A lista vem por visualizações, que mudam a cada rodada; pelo título, o snapshot só muda quando a
    // base muda.
    if natureza == Natureza::Rejeicao {
        perguntas.sort_by(|a, b| a.pergunta.cmp(&b.pergunta));
    }
    Ok(perguntas)
}

fn url_da_busca() -> String {
    format!("{BASE}/csm?id=kb_search&kb_knowledge_base={KB}")
}

fn url_da_lista() -> String {
    format!(
        "{BASE}/api/now/sp/widget/{WIDGET_RESULTADOS}?id=kb_search&kb_knowledge_base={KB}&portal_id={PORTAL_ID}"
    )
}

/// O link do artigo pelo número, que sobrevive às versões (o `sys_id` muda a cada uma) — D-MGFAQ-2.
fn link_do_artigo(numero: &str) -> String {
    format!("{BASE}/csm?id=kb_article_view&sysparm_article={numero}")
}

/// A lista (D-MGFAQ-13): rede sempre, gravando o cache sob a url do POST; com `--usecache`, só o cache
/// (sem sessão), e miss é erro — a D-MGFAQ-10 do SPED.
fn buscar_lista(data_dir: &str, agent: &Agent, use_cache: bool) -> Result<String> {
    let url = url_da_lista();
    if use_cache {
        return auli_scraper_kit::cache::read(data_dir, "faqs", &url)
            .ok_or_else(|| anyhow!("cache miss para {url} (modo --usecache, sem rede)"));
    }
    let (cookie, token) = abrir_sessao(agent)?;
    sleep(COURTESY);
    println!("Posting: {url}");
    // O payload é o que o `Knowledge Portal Service` do portal monta para a busca sem palavra-chave.
    let payload = json!({
        "keyword": "",
        "language": "",
        "variables": { "kb_knowledge_base": [KB] },
        "resource": "",
        "kb_query": "",
        "social_query": "",
        "category_as_tree": false,
        "order": "sys_view_count,true",
        "start": 0,
        "end": MAX_ARTIGOS,
        "attachment": true,
        "portal_suffix": "csm",
        "knowledge_fields": ["author", "sys_view_count", "sys_updated_on", "rating", "external"],
        "social_fields": [],
    });
    let corpo = auli_scraper_kit::http::post_json(
        agent,
        &url,
        &[
            ("Cookie", &cookie),
            ("X-UserToken", &token),
            ("X-Portal", PORTAL_ID),
            ("X-Requested-With", "XMLHttpRequest"),
        ],
        &json!({ "payload": payload.to_string() }),
        &GetOpts {
            log_prefix: LOG,
            accept: Some("application/json"),
            ..Default::default()
        },
    )?;
    auli_scraper_kit::cache::write(data_dir, "faqs", &url, &corpo);
    Ok(corpo)
}

static G_CK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"g_ck\s*=\s*['"]([^'"]+)['"]"#).unwrap());

/// A sessão anônima que o portal abre para qualquer visitante: os cookies do GET da página da busca e o
/// token `g_ck` que ela traz no HTML. Sem eles o POST da lista responde `invalid_token`.
fn abrir_sessao(agent: &Agent) -> Result<(String, String)> {
    let url = url_da_busca();
    sleep(COURTESY);
    println!("Fetching: {url}");
    let mut resp = agent
        .get(&url)
        .call()
        .map_err(|e| anyhow!("{LOG}: GET {url}: {e}"))?;
    let cookie = resp
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .filter_map(|c| c.split(';').next())
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .collect::<Vec<_>>()
        .join("; ");
    let html = resp.body_mut().read_to_string()?;
    let token = G_CK_RE
        .captures(&html)
        .map(|c| c[1].to_string())
        .ok_or_else(|| anyhow!("a página {url} não trouxe o token `g_ck` — o portal mudou?"))?;
    Ok((cookie, token))
}

/// Um artigo pela page API, sob a url de navegação (a chave de cache do `mg.rs`).
fn buscar_artigo(data_dir: &str, agent: &Agent, sys_id: &str, use_cache: bool) -> Result<String> {
    let chave = format!("{BASE}/csm?id=kb_article_view&sys_kb_id={sys_id}");
    if use_cache {
        return auli_scraper_kit::cache::read(data_dir, "faqs", &chave)
            .ok_or_else(|| anyhow!("cache miss para {chave} (modo --usecache, sem rede)"));
    }
    sleep(COURTESY);
    let corpo = auli_scraper_kit::http::get_string(
        agent,
        &format!(
            "{BASE}/api/now/sp/page?id=kb_article_view&sys_kb_id={sys_id}&portal_id={PORTAL_ID}"
        ),
        &GetOpts {
            log_prefix: LOG,
            accept: Some("application/json"),
            headers: &[
                ("X-Portal", PORTAL_ID),
                ("X-Requested-With", "XMLHttpRequest"),
            ],
            ..Default::default()
        },
    )?;
    auli_scraper_kit::cache::write(data_dir, "faqs", &chave, &corpo);
    Ok(corpo)
}

/// A resposta do POST, com as guardas da lista (D-MGFAQ-13).
fn parse_lista(json: &str) -> Result<Vec<Listado>> {
    let v: Value = serde_json::from_str(json)?;
    if v["result"]["invalid_token"].as_bool() == Some(true) {
        bail!(
            "o portal recusou a sessão anônima (`invalid_token`) — o token `g_ck` mudou de lugar?"
        );
    }
    let r = &v["result"]["data"]["results"];
    let (Some(total), Some(itens)) = (r["meta"]["count"].as_u64(), r["results"].as_array()) else {
        bail!(
            "a resposta da busca não tem `meta.count` e `results` — o widget de resultados mudou?"
        );
    };
    // (f) A contagem é do próprio portal: menos itens que ela é a janela do POST pequena demais.
    if total == 0 {
        bail!("a busca não achou nenhum artigo na base {NOME_DA_BASE} — o filtro mudou?");
    }
    if itens.len() as u64 != total {
        bail!(
            "a busca trouxe {} de {total} artigos — a base passou de {MAX_ARTIGOS}?",
            itens.len()
        );
    }

    let mut listados = Vec::new();
    let mut vistos = BTreeSet::new();
    for it in itens {
        let titulo = it["title"].as_str().unwrap_or_default();
        let sys_id = it["id"]
            .as_str()
            .and_then(|id| id.strip_prefix("kb_knowledge:"))
            .ok_or_else(|| anyhow!("{titulo}: sem `id` kb_knowledge na lista"))?;
        // (g) Só a base pedida, uma categoria por artigo, e a categoria na lista fechada.
        if it["meta"]["knowledgeBaseId"].as_str() != Some(KB) {
            bail!("{titulo}: artigo de outra base na lista — o filtro do POST não pegou");
        }
        let categoria = match it["meta"]["parentCategories"].as_array().map(Vec::as_slice) {
            Some([c]) => clean(c.as_str().unwrap_or_default()),
            _ => bail!("{titulo}: o artigo não tem exatamente uma categoria"),
        };
        if !NATUREZAS.iter().any(|(n, _)| *n == categoria) {
            bail!(
                "categoria nova na base {NOME_DA_BASE}: \"{categoria}\" ({titulo}) — decidir a natureza dela (D-MGFAQ-14)"
            );
        }
        if !vistos.insert(sys_id.to_string()) {
            bail!("{titulo}: artigo repetido na lista ({sys_id})");
        }
        listados.push(Listado {
            sys_id: sys_id.to_string(),
            titulo: titulo.to_string(),
            categoria,
        });
    }
    Ok(listados)
}

/// A página de um artigo (h): o widget do cabeçalho, com texto — robô não recebe o texto.
fn parse_artigo(json: &str, l: &Listado) -> Result<Artigo> {
    let v: Value = serde_json::from_str(json)?;
    let d = widget_data(&v, "edx_article_header").ok_or_else(|| {
        anyhow!(
            "{}: a página não tem o widget `edx_article_header`",
            l.titulo
        )
    })?;
    if d["isCrawlerBot"].as_bool() != Some(false) {
        bail!(
            "{}: o portal tratou a coleta como robô (`isCrawlerBot`) e omitiu o texto — o UA mudou?",
            l.titulo
        );
    }
    let numero = d["number"].as_str().unwrap_or_default();
    if numero.is_empty() {
        bail!("{}: a página não traz o número do artigo", l.titulo);
    }
    Ok(Artigo {
        numero: numero.to_string(),
        titulo: clean(d["shortDesc"].as_str().unwrap_or_default()),
        conteudo: d["kbContentData"]["data"].clone(),
    })
}

/// Rejeição (D-MGFAQ-15): o título é a pergunta; a resposta é a seção "Resposta para o Erro". A
/// "Descrição do Erro" vem cortada em ~40 caracteres e o "Assunto" (1 artigo) repete o título.
fn rejeicao(a: &Artigo) -> Result<Pergunta> {
    let resposta = a
        .conteudo
        .as_array()
        .and_then(|secoes| {
            secoes
                .iter()
                .find(|s| s["label"].as_str() == Some("Resposta para o Erro"))
        })
        .and_then(|s| s["content"].as_str())
        .map(|h| html_para_texto(h, BASE))
        .unwrap_or_default();
    if a.titulo.is_empty() || resposta.is_empty() {
        bail!(
            "{} ({}): rejeição sem título ou sem \"Resposta para o Erro\" — o modelo do artigo mudou?",
            a.titulo,
            a.numero
        );
    }
    Ok(Pergunta {
        pergunta: a.titulo.clone(),
        resposta,
        url: link_do_artigo(&a.numero),
    })
}

static LINHA_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?is)<tr\b.*?</tr>").unwrap());
static CELULA_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)<td\b[^>]*>(.*?)</td>").unwrap());
static NUMERO_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\d+)\s*[.)]\s*(.*)$").unwrap());

/// Compilação P./R. (D-MGFAQ-15): cada par de linhas da tabela — `P.` | `N) pergunta`, depois `R.` |
/// resposta — é uma pergunta, sem o `N)` (D-MGFAQ-11). A numeração é conferida, 1 a N sem buraco. O
/// artigo não tem âncora: as N perguntas levam o link do artigo.
fn compilacao(a: &Artigo) -> Result<Vec<Pergunta>> {
    let html = a.conteudo.as_str().unwrap_or_default();
    let linhas: Vec<(String, &str)> = LINHA_RE
        .find_iter(html)
        .map(|linha| {
            let celulas: Vec<&str> = CELULA_RE
                .captures_iter(linha.as_str())
                .map(|c| c.get(1).map_or("", |m| m.as_str()))
                .collect();
            match celulas.as_slice() {
                [rotulo, texto] => (uma_linha(rotulo), *texto),
                _ => (String::new(), ""),
            }
        })
        .collect();
    if linhas.is_empty() || !linhas.len().is_multiple_of(2) {
        bail!(
            "{} ({}): a tabela não tem pares de linhas P./R. ({} linhas)",
            a.titulo,
            a.numero,
            linhas.len()
        );
    }

    let mut perguntas = Vec::new();
    for (i, par) in linhas.chunks(2).enumerate() {
        let [(p, pergunta), (r, resposta)] = par else {
            unreachable!()
        };
        let esperado = i + 1;
        let lido = NUMERO_RE.captures(&uma_linha(pergunta)).and_then(|c| {
            let n: usize = c[1].parse().ok()?;
            Some((n, c[2].to_string()))
        });
        let (pergunta, resposta) = match lido {
            Some((n, texto)) if p == "P." && r == "R." && n == esperado => {
                (texto, html_para_texto(resposta, BASE))
            }
            _ => bail!(
                "{} ({}): o par {esperado} da tabela não é `P.` + `{esperado})` seguido de `R.` — a tabela mudou?",
                a.titulo,
                a.numero
            ),
        };
        if pergunta.is_empty() || resposta.is_empty() {
            bail!(
                "{} ({}): a pergunta {esperado} veio sem texto ou sem resposta",
                a.titulo,
                a.numero
            );
        }
        perguntas.push(Pergunta {
            pergunta,
            resposta,
            url: link_do_artigo(&a.numero),
        });
    }
    Ok(perguntas)
}

fn trilha(c: &Categoria) -> String {
    format!("{NOME_DA_BASE} | {}", c.nome)
}

pub(crate) fn faqs(categorias: &[Categoria]) -> Vec<FaqRaw> {
    categorias
        .iter()
        .flat_map(|c| {
            c.perguntas.iter().map(move |p| FaqRaw {
                pergunta: p.pergunta.clone(),
                resposta: p.resposta.clone(),
                origin: trilha(c),
                url: p.url.clone(),
            })
        })
        .collect()
}

/// O ramo da base na aba de FAQs: um `Menu` com um `Faq` por categoria, ao lado dos módulos do SPED.
pub(crate) fn arvore(categorias: &[Categoria]) -> FaqNode {
    FaqNode {
        title: NOME_DA_BASE.to_string(),
        url: url_da_busca(),
        page_type: PageType::Menu,
        origin: String::new(),
        children: categorias
            .iter()
            .map(|c| FaqNode {
                title: c.nome.clone(),
                url: url_da_busca(),
                page_type: PageType::Faq,
                origin: trilha(c),
                children: Vec::new(),
                faq_items: c
                    .perguntas
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

    // Recortes da coleta real (02/11/2026): a resposta do POST com 3 dos 81 artigos (uma orientação, o
    // PED e a rejeição 215), e o widget `edx_article_header` de três artigos — as rejeições 215 e 690 (a
    // única com "Assunto") e o PED com os 3 primeiros dos 100 pares.
    const LISTA: &str = include_str!("../tests/fixtures/mg-faqs-csm-lista.json");
    const REJEICAO_215: &str = include_str!("../tests/fixtures/mg-faqs-csm-rejeicao-215.json");
    const REJEICAO_690: &str = include_str!("../tests/fixtures/mg-faqs-csm-rejeicao-690.json");
    const PED: &str = include_str!("../tests/fixtures/mg-faqs-csm-ped.json");

    fn erro<T>(r: Result<T>) -> String {
        match r {
            Ok(_) => panic!("esperava Err"),
            Err(e) => e.to_string(),
        }
    }

    fn listado(titulo: &str) -> Listado {
        Listado {
            sys_id: "x".into(),
            titulo: titulo.into(),
            categoria: String::new(),
        }
    }

    fn artigo(json: &str) -> Artigo {
        parse_artigo(json, &listado("artigo")).unwrap()
    }

    #[test]
    fn parse_lista_le_os_artigos_com_a_categoria_limpa() {
        let l = parse_lista(LISTA).unwrap();
        let resumo: Vec<(&str, &str)> = l
            .iter()
            .map(|a| (a.titulo.as_str(), a.categoria.as_str()))
            .collect();
        assert_eq!(
            resumo,
            [
                ("ITCD - 002/2006", "Orientações Tributárias DOLT/SUTRI"),
                ("PED - Dúvidas Frequentes", "Sintegra - PED"),
                (
                    "Rejeição 215: Falha no schema XML.",
                    "DF-e - Correção de Erros / Rejeições"
                ),
            ]
        );
        assert_eq!(l[1].sys_id, "7b5db3e8971bead00994f1a6f053af04");
    }

    #[test]
    fn lista_com_a_sessao_recusada_e_erro() {
        let e = erro(parse_lista(
            r#"{"result":{"invalid_token":true,"payload":"{}"}}"#,
        ));
        assert!(e.contains("invalid_token"), "{e}");
    }

    #[test]
    fn lista_menor_que_a_contagem_do_portal_e_erro() {
        let e = erro(parse_lista(&LISTA.replace(r#""count":3"#, r#""count":4"#)));
        assert!(e.contains("trouxe 3 de 4"), "{e}");
    }

    #[test]
    fn lista_vazia_e_erro() {
        let e = erro(parse_lista(
            r#"{"result":{"data":{"results":{"meta":{"count":0},"results":[]}}}}"#,
        ));
        assert!(e.contains("nenhum artigo"), "{e}");
    }

    #[test]
    fn artigo_de_outra_base_e_erro() {
        let e = erro(parse_lista(&LISTA.replacen(
            KB,
            "bccaaf011bbec61029f686afe54bcbfc",
            1,
        )));
        assert!(e.contains("outra base"), "{e}");
    }

    #[test]
    fn artigo_com_duas_categorias_e_erro() {
        let e = erro(parse_lista(&LISTA.replace(
            r#"["Sintegra - PED"]"#,
            r#"["Sintegra - PED","Orientações Tributárias DOLT/SUTRI"]"#,
        )));
        assert!(e.contains("exatamente uma categoria"), "{e}");
    }

    #[test]
    fn categoria_nova_e_erro() {
        let e = erro(parse_lista(
            &LISTA.replace("Sintegra - PED", "Sintegra - Novidades"),
        ));
        assert!(
            e.contains("categoria nova") && e.contains("Sintegra - Novidades"),
            "{e}"
        );
    }

    #[test]
    fn artigo_repetido_na_lista_e_erro() {
        let mut v: Value = serde_json::from_str(LISTA).unwrap();
        let r = &mut v["result"]["data"]["results"];
        let primeiro = r["results"][0].clone();
        r["results"].as_array_mut().unwrap().push(primeiro);
        r["meta"]["count"] = json!(4);
        let e = erro(parse_lista(&v.to_string()));
        assert!(e.contains("repetido"), "{e}");
    }

    #[test]
    fn rejeicao_tem_o_titulo_como_pergunta_e_a_resposta_para_o_erro() {
        let p = rejeicao(&artigo(REJEICAO_215)).unwrap();
        assert_eq!(p.pergunta, "Rejeição 215: Falha no schema XML.");
        assert!(
            p.resposta.starts_with(
                "A rejeição 215 \"Falha no schema XML\" ocorre quando é enviado um arquivo XML"
            ),
            "{}",
            p.resposta
        );
        assert!(
            p.resposta
                .contains("\"https://www.sefaz.rs.gov.br/nfe/NFE-VAL.aspx\""),
            "{}",
            p.resposta
        );
        assert_eq!(
            p.url,
            "https://atendimento2.fazenda.mg.gov.br/csm?id=kb_article_view&sysparm_article=KB0011056"
        );
    }

    #[test]
    fn rejeicao_com_assunto_usa_so_a_resposta_para_o_erro() {
        let p = rejeicao(&artigo(REJEICAO_690)).unwrap();
        assert_eq!(
            p.pergunta,
            "Rejeição 690: Pedido de Cancelamento para NF-e com CT-e ou MDF-e"
        );
        assert!(
            p.resposta
                .starts_with("Quando uma NF-e estiver vinculada a um CT-e ou MDF-e"),
            "{}",
            p.resposta
        );
    }

    #[test]
    fn rejeicao_sem_a_resposta_para_o_erro_e_erro() {
        let a = artigo(&REJEICAO_215.replace("Resposta para o Erro", "Solução"));
        let e = erro(rejeicao(&a));
        assert!(
            e.contains("Resposta para o Erro") && e.contains("KB0011056"),
            "{e}"
        );
    }

    #[test]
    fn artigo_servido_a_robo_e_erro() {
        let json = REJEICAO_215.replace(r#""isCrawlerBot":false"#, r#""isCrawlerBot":true"#);
        let e = erro(parse_artigo(&json, &listado("Rejeição 215")));
        assert!(e.contains("isCrawlerBot"), "{e}");
    }

    #[test]
    fn artigo_sem_numero_e_erro() {
        let json = REJEICAO_215.replace(r#""number":"KB0011056""#, r#""number":"""#);
        let e = erro(parse_artigo(&json, &listado("Rejeição 215")));
        assert!(e.contains("número"), "{e}");
    }

    #[test]
    fn compilacao_vira_uma_pergunta_por_par_sem_o_numero() {
        let ps = compilacao(&artigo(PED)).unwrap();
        let perguntas: Vec<&str> = ps.iter().map(|p| p.pergunta.as_str()).collect();
        assert_eq!(
            perguntas,
            [
                "Qual a legislação pertinente a arquivos eletrônicos e onde encontrá-la?",
                "Quem deve apresentar o arquivo eletrônico?",
                "Quem deve apresentar o arquivo com item de mercadoria?",
            ]
        );
        assert!(
            ps[0]
                .resposta
                .starts_with("Nacionalmente o assunto é tratado pelo Convênio ICMS 57/95")
        );
        assert!(ps.iter().all(|p| p.url
            == "https://atendimento2.fazenda.mg.gov.br/csm?id=kb_article_view&sysparm_article=KB0011140"));
    }

    #[test]
    fn compilacao_com_buraco_na_numeracao_e_erro() {
        let e = erro(compilacao(&artigo(
            &PED.replace("<p>2) Quem deve", "<p>4) Quem deve"),
        )));
        assert!(e.contains("par 2"), "{e}");
    }

    #[test]
    fn compilacao_com_rotulo_trocado_e_erro() {
        let e = erro(compilacao(&artigo(&PED.replacen(
            "<p>R.</p>",
            "<p>P.</p>",
            1,
        ))));
        assert!(e.contains("par 1"), "{e}");
        let e = erro(compilacao(&artigo(&PED.replacen(
            "<p>P.</p>",
            "<p>R.</p>",
            1,
        ))));
        assert!(e.contains("par 1"), "{e}");
    }

    #[test]
    fn compilacao_com_linha_sobrando_e_erro() {
        let sobrando = PED.replace(
            "</tbody></table>",
            "<tr><td><p>P.</p></td><td><p>4) Sem resposta?</p></td></tr></tbody></table>",
        );
        let e = erro(compilacao(&artigo(&sobrando)));
        assert!(e.contains("pares") && e.contains("7 linhas"), "{e}");
    }

    #[test]
    fn compilacao_com_resposta_vazia_e_erro() {
        let sem_resposta = PED.replace(
            "Nacionalmente o assunto é tratado pelo Convênio ICMS 57/95 e alterações. Em Minas Gerais o contribuinte deverá consultar o Decreto 48.633/2023 do Estado de Minas Gerais, e a Portaria SRE 222/2023, na qual constará o layout de todos os registros",
            "",
        );
        let e = erro(compilacao(&artigo(&sem_resposta)));
        assert!(
            e.contains("pergunta 1 veio sem texto ou sem resposta"),
            "{e}"
        );
    }

    #[test]
    fn rejeicoes_saem_pelo_titulo_nao_pelas_visualizacoes() {
        let ps = perguntas_da_categoria(
            Natureza::Rejeicao,
            &[artigo(REJEICAO_690), artigo(REJEICAO_215)],
        )
        .unwrap();
        let ordem: Vec<&str> = ps
            .iter()
            .filter_map(|p| p.pergunta.split(':').next())
            .collect();
        assert_eq!(ordem, ["Rejeição 215", "Rejeição 690"]);
    }

    #[test]
    fn faqs_e_arvore_levam_a_base_e_a_categoria() {
        let categorias = vec![
            Categoria {
                nome: NATUREZAS[0].0.to_string(),
                perguntas: vec![rejeicao(&artigo(REJEICAO_215)).unwrap()],
            },
            Categoria {
                nome: NATUREZAS[1].0.to_string(),
                perguntas: compilacao(&artigo(PED)).unwrap(),
            },
        ];
        let raws = faqs(&categorias);
        assert_eq!(raws.len(), 4);
        assert_eq!(
            raws[0].origin,
            "SRE - Dúvidas Frequentes | DF-e - Correção de Erros / Rejeições"
        );
        assert_eq!(raws[3].origin, "SRE - Dúvidas Frequentes | Sintegra - PED");

        let a = arvore(&categorias);
        assert_eq!(a.title, "SRE - Dúvidas Frequentes");
        assert_eq!(
            a.url,
            "https://atendimento2.fazenda.mg.gov.br/csm?id=kb_search&kb_knowledge_base=f18a2fc11bbec61029f686afe54bcbed"
        );
        let filhos: Vec<(&str, usize)> = a
            .children
            .iter()
            .map(|c| (c.title.as_str(), c.faq_items.len()))
            .collect();
        assert_eq!(
            filhos,
            [
                ("DF-e - Correção de Erros / Rejeições", 1),
                ("Sintegra - PED", 3)
            ]
        );
        assert!(a.children.iter().all(|c| c.page_type == PageType::Faq));
    }
}
