// SEFAZ-CE FAQs — os artigos de perguntas e respostas ("Principais Dúvidas sobre …") do catálogo `conteudo-geral`
// do Portal de Serviços (Sydle ONE), pela mesma API JSON dos serviços: `getChildren` lista o catálogo e `getItem`
// traz o artigo. Sem navegador. Estudo, medições e decisões: docs/ESTUDO-CE-Faqs.md (D-CEFAQ-*).
//
// Cada artigo é um `contentMarkdown` com um bloco `<div class="section-wrapper">` por pergunta, em dois templates:
// o de 2026 (pergunta em `<h2>`, resposta em HTML com entidades) e o de 2024 (pergunta numa linha `## `, resposta
// em markdown, onde a quebra de linha separa os itens das listas). A resposta é tudo o que vem depois da tag de
// abertura do `<sy-collapse … trigger-by="#section-<slug>">`, até o fim do bloco.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::LazyLock;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use regex::Regex;
use serde::Deserialize;

use auli_contract::FaqRaw;
use auli_scraper_kit::clean;
use auli_scraper_kit::faqtree::{FaqItem, FaqNode, PageType};
use auli_scraper_kit::http::GetOpts;

use crate::ce;

/// A identidade das FAQs da frota (D-FROTA-UA, D-CEFAQ-8); as sondagens do estudo rodaram com ela. Os serviços
/// do CE seguem com o UA de navegador (decisão registrada, robots).
const UA: &str =
    "AuliBot/0.1 (+https://github.com/oxschellen/auli; carlos.schellenberger@gmail.com)";
const COURTESY: Duration = Duration::from_millis(400);

/// O catálogo irmão do `servico-geral`: "Conteúdo - Geral" (428 conteúdos em 03/11/2026).
const RAIZ: &str = "648af6fc64778b7336bf45d3";
const RAIZ_PATH: &str = "conteudo-geral";
/// O método que o anônimo pode chamar para ler um artigo (`get`, `getContent`, `getArticle`… dão 403). Mesma
/// classe de API do `getChildren`.
const GETITEM_URL: &str = "https://portalservicos.sefaz.ce.gov.br/api/1/servicedesk-embedded/_classId/5cd5e83d59f8170cd4dc2c43/getItem";
/// Teto da paginação (44 páginas em 03/11/2026, a última vazia).
const MAX_PAGINAS: u32 = 80;

/// O nome que a SEFAZ-CE dá aos artigos de FAQ (D-CEFAQ-2): 96 dos 428 conteúdos em 03/11/2026. O resto são
/// os "COMUNICADO COTRI nº …" e páginas institucionais.
static FAQ_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)d[úu]vidas|pergunta|faq").unwrap());

/// Pisos (estudo §3): 93 artigos com perguntas e 2.706 perguntas depois das cópias em 03/11/2026.
const MIN_ARTIGOS: usize = 80;
const MIN_PERGUNTAS: usize = 2400;
/// Artigos de FAQ sem nenhuma seção (lista de bancos, link para o gov.br, "-"): 3 em 03/11/2026. Acima disto, o
/// template mudou (D-CEFAQ-6).
const MAX_SEM_SECOES: usize = 10;

const BLOCO: &str = "<div class=\"section-wrapper\">";

/// Lê a listagem do `conteudo-geral` e cada artigo de FAQ e grava a árvore do frontend (`<id>-faqs-tree.json`) e o
/// snapshot. As guardas rodam antes de qualquer gravação: rodada que falha deixa os arquivos anteriores intactos.
pub fn run(data_dir: &str, use_cache: bool) -> Result<()> {
    let agent = auli_scraper_kit::build_agent(UA, Some(Duration::from_secs(30)));
    // O token anônimo do shell, efêmero: um por rodada. Com `--usecache` não há rede.
    let token = if use_cache {
        String::new()
    } else {
        ce::fetch_token(&agent)?
    };

    let mut lista: Vec<ce::Item> = Vec::new();
    let mut ids = HashSet::new();
    let mut pagina = 0;
    loop {
        pagina += 1;
        if pagina > MAX_PAGINAS {
            bail!("a listagem passou de {MAX_PAGINAS} páginas — a paginação mudou?");
        }
        let chave = format!("{}#{RAIZ_PATH}&page={pagina}", ce::GETCHILDREN_URL);
        let corpo = com_cache(data_dir, &chave, use_cache, || {
            ce::fetch_page(&agent, &token, RAIZ, RAIZ_PATH, pagina)
        })?;
        let itens = ce::parse_page(&corpo)?.items;
        // Termina na página vazia, como nos serviços (página não-final pode vir curta).
        if itens.is_empty() {
            break;
        }
        for item in itens {
            if !ids.insert(item.id.clone()) {
                bail!(
                    "o _id {} veio duas vezes na listagem — a paginação mudou?",
                    item.id
                );
            }
            lista.push(item);
        }
    }
    let artigos = de_faq(&lista);
    println!(
        "CE FAQs: {} conteúdo(s) em {} página(s) da listagem, {} artigo(s) de FAQ pelo nome",
        lista.len(),
        pagina - 1,
        artigos.len()
    );

    let mut perguntas = Vec::new();
    let mut sem_secoes = Vec::new();
    for item in &artigos {
        let corpo = com_cache(
            data_dir,
            &format!("{GETITEM_URL}#{}", item.id),
            use_cache,
            || fetch_artigo(&agent, &token, &item.id),
        )?;
        let artigo = parse_artigo(&corpo, &item.id)?;
        match secoes(&artigo).with_context(|| format!("artigo {} ({})", artigo.id, artigo.name))? {
            Some(ps) => perguntas.extend(ps),
            None => sem_secoes.push(artigo.name),
        }
    }
    for nome in &sem_secoes {
        println!("ℹ️  CE FAQs: artigo sem seções, fica fora — {nome}");
    }
    if sem_secoes.len() > MAX_SEM_SECOES {
        bail!(
            "{} artigo(s) de FAQ sem seções (máximo {MAX_SEM_SECOES}) — o template mudou?",
            sem_secoes.len()
        );
    }
    let com_secoes = artigos.len() - sem_secoes.len();
    piso(com_secoes, MIN_ARTIGOS, "artigo(s) com perguntas")?;
    let lidas = perguntas.len();
    let perguntas = sem_copias(perguntas);
    piso(perguntas.len(), MIN_PERGUNTAS, "pergunta(s)")?;
    sem_colisao(&perguntas)?;
    println!(
        "CE FAQs: {} pergunta(s) de {com_secoes} artigo(s) — {} lida(s), {} cópia(s) saíram",
        perguntas.len(),
        lidas,
        lidas - perguntas.len()
    );

    let faqs = consolidar(&perguntas);
    // Atômico pelo mesmo motivo das outras entidades: a árvore não tem outra fonte (o snapshot achatado não a
    // reconstrói) e é o único artefato de coleta que o frontend lê direto.
    let tree_path = format!("{data_dir}/{}-faqs-tree.json", crate::ENTITY);
    auli_contract::arquivo::escrever_atomico(
        Path::new(&tree_path),
        &serde_json::to_string_pretty(&montar_arvore(&perguntas))?,
    )?;
    println!("Wrote {tree_path} (árvore de FAQ p/ o frontend)");

    let info = auli_contract::ScraperInfo::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    auli_contract::snapshot::write_faqs(crate::ENTITY, data_dir, &info, faqs)?;
    println!("🎉 Coleta de FAQs gravada no snapshot.");
    Ok(())
}

/// Uma chamada (D-CEFAQ-8): rede sempre, gravando o cache; com `--usecache`, só o cache, e miss é erro.
fn com_cache(
    data_dir: &str,
    chave: &str,
    use_cache: bool,
    rede: impl FnOnce() -> Result<String>,
) -> Result<String> {
    if use_cache {
        return auli_scraper_kit::cache::read(data_dir, "faqs", chave)
            .ok_or_else(|| anyhow!("cache miss para {chave} (modo --usecache, sem rede)"));
    }
    sleep(COURTESY);
    let corpo = rede()?;
    auli_scraper_kit::cache::write(data_dir, "faqs", chave, &corpo);
    Ok(corpo)
}

/// Coleta capada (listagem ou template que mudou) é `Err`, não snapshot menor.
fn piso(n: usize, minimo: usize, oque: &str) -> Result<()> {
    if n < minimo {
        bail!("{n} {oque} (mínimo {minimo}) — o portal mudou?");
    }
    Ok(())
}

/// Os artigos de FAQ da listagem, na ordem dela (D-CEFAQ-2).
fn de_faq(lista: &[ce::Item]) -> Vec<&ce::Item> {
    lista.iter().filter(|i| FAQ_RE.is_match(&i.name)).collect()
}

/// POST `getItem` de um artigo; devolve o corpo JSON cru. Retenta com backoff (via kit).
fn fetch_artigo(agent: &ureq::Agent, token: &str, id: &str) -> Result<String> {
    println!("POST getItem {id}");
    auli_scraper_kit::http::post_json(
        agent,
        GETITEM_URL,
        &[
            ("Authorization", token),
            ("X-Explorer-Account-Token", ce::ORG_HEADER),
        ],
        &serde_json::json!({ "application": ce::APP_IN_BODY, "id": id }),
        &GetOpts {
            log_prefix: "CE",
            accept: Some("application/json"),
            ..Default::default()
        },
    )
}

#[derive(Deserialize)]
struct RespostaItem {
    #[serde(rename = "return")]
    artigo: Artigo,
}

/// Um artigo do `getItem`. Só os campos que usamos; serde ignora o resto.
#[derive(Deserialize)]
struct Artigo {
    #[serde(rename = "_id")]
    id: String,
    #[serde(default)]
    identifier: String,
    #[serde(default)]
    name: String,
    #[serde(rename = "contentFormat")]
    formato: Option<String>,
    #[serde(rename = "contentMarkdown")]
    markdown: Option<String>,
}

/// O artigo pedido, ou `Err` se o corpo não é o envelope do `getItem` ou traz outro `_id`.
fn parse_artigo(corpo: &str, id: &str) -> Result<Artigo> {
    let artigo = serde_json::from_str::<RespostaItem>(corpo)
        .with_context(|| format!("JSON de getItem inválido para {id}"))?
        .artigo;
    if artigo.id != id {
        bail!("getItem {id} devolveu o artigo {}", artigo.id);
    }
    Ok(artigo)
}

struct Pergunta {
    /// O `_id` do artigo (as cópias ficam no de menor `_id`).
    id: String,
    pergunta: String,
    resposta: String,
    /// O nome do artigo, íntegro (D-CEFAQ-4).
    artigo: String,
    /// O artigo no portal — o nó da árvore.
    url_artigo: String,
    /// A pergunta no artigo, `…#section-<slug>`. Sem a âncora, as perguntas que se repetem no mesmo artigo com
    /// outra resposta (59 em 03/11/2026) dariam o mesmo `.md`.
    url: String,
}

static SLUG_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"id="section-([^"]*)""#).unwrap());
static H2_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)<h2\b[^>]*>(.*?)</h2>").unwrap());
static MD_H2_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^##[ \t]+(.+?)[ \t]*$").unwrap());

/// As perguntas de um artigo, ou `None` quando ele não tem seções (`contentFormat` HTML, ou nenhum bloco):
/// o artigo fica fora, com log (D-CEFAQ-6).
fn secoes(a: &Artigo) -> Result<Option<Vec<Pergunta>>> {
    let md = a.markdown.as_deref().unwrap_or("");
    // O que vem antes do 1º bloco é o `<style>` e o botão "Expandir todas".
    let blocos: Vec<&str> = md.split(BLOCO).skip(1).collect();
    if a.formato.as_deref() != Some("MARKDOWN") || blocos.is_empty() {
        return Ok(None);
    }
    let ancoras = md.matches("id=\"section-").count();
    if ancoras != blocos.len() {
        bail!(
            "{ancoras} âncora(s) id=\"section-\" para {} bloco(s) — o template mudou?",
            blocos.len()
        );
    }
    let url_artigo = url_artigo(&a.identifier, &a.id);
    let mut slugs = HashSet::new();
    let mut out = Vec::with_capacity(blocos.len());
    for bloco in blocos {
        let (slug, pergunta, resposta) = secao(bloco)?;
        // O portal repete o slug quando duas perguntas só diferem em pontuação (uma em 03/11/2026, `ICMS/IPI` e
        // `ICMS|IPI`, com respostas diferentes): as duas entram. A colisão de verdade é a do `.md`.
        if !slugs.insert(slug.clone()) {
            eprintln!("⚠️  CE FAQs: a âncora #section-{slug} se repete em {url_artigo}");
        }
        out.push(Pergunta {
            id: a.id.clone(),
            url: format!("{url_artigo}#section-{slug}"),
            pergunta,
            resposta,
            artigo: clean(&a.name),
            url_artigo: url_artigo.clone(),
        });
    }
    Ok(Some(out))
}

/// `(slug, pergunta, resposta)` de um bloco. A pergunta mora no cabeçalho, antes do `<sy-collapse>`: em `<h2>`
/// (template de 2026) ou na linha `## ` (2024, markdown).
fn secao(bloco: &str) -> Result<(String, String, String)> {
    let slug = SLUG_RE
        .captures(bloco)
        .map(|c| c[1].to_string())
        .ok_or_else(|| anyhow!("bloco sem id=\"section-…\""))?;
    let gatilho = format!("trigger-by=\"#section-{slug}\"");
    let i = bloco
        .find(&gatilho)
        .ok_or_else(|| anyhow!("a seção {slug} não tem o <sy-collapse> dela"))?;
    let inicio_linha = bloco[..i].rfind('\n').map_or(0, |j| j + 1);
    if !bloco[inicio_linha..i].contains("<sy-collapse") {
        bail!("o gatilho da seção {slug} não está numa tag <sy-collapse>");
    }
    let cabecalho = &bloco[..inicio_linha];
    let (pergunta, markdown) = match H2_RE.captures(cabecalho) {
        Some(c) => (c[1].to_string(), false),
        None => (
            MD_H2_RE
                .captures(cabecalho)
                .map(|c| c[1].to_string())
                .ok_or_else(|| anyhow!("a seção {slug} não tem pergunta (nem <h2> nem ##)"))?,
            true,
        ),
    };
    // A tag de abertura termina no fim da linha, não no primeiro `>`: a do template de 2024 traz
    // `ref="(r) => handleState(r)"`, um `>` dentro do atributo.
    let corpo = bloco[i..]
        .split_once('\n')
        .map(|(_, resto)| resto)
        .ok_or_else(|| anyhow!("a seção {slug} acaba na tag <sy-collapse>"))?;
    let pergunta = clean(&html_para_texto(&pergunta, false).replace('\n', " "));
    let resposta = html_para_texto(corpo, markdown);
    if pergunta.is_empty() {
        bail!("a seção {slug} tem a pergunta vazia");
    }
    if resposta.is_empty() {
        bail!("a pergunta {pergunta:?} não tem resposta");
    }
    Ok((slug, pergunta, resposta))
}

/// O artigo no portal: `…/<identifier>+<_id>`, como o `link` dos serviços; sem `identifier`, a forma curta.
fn url_artigo(identifier: &str, id: &str) -> String {
    format!("{}/{identifier}+{id}", ce::BASE)
}

/// O mesmo par em mais de um lugar — artigos irmãos ou o mesmo artigo (473 cópias em 03/11/2026; a "Restituição de
/// Taxas" está inteira em outro artigo) — fica só no de menor `_id`, na primeira vez (D-CEFAQ-5). Quebras de linha e
/// espaços não contam: há cópias que só diferem por um `<br>` no meio da frase.
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

/// A árvore da aba de FAQs: raiz → artigos, na ordem da listagem → perguntas. A interface começa pelos filhos da
/// raiz. O nó do artigo aponta para o artigo; cada pergunta, para a sua seção.
fn montar_arvore(perguntas: &[Pergunta]) -> FaqNode {
    let mut artigos: Vec<Vec<&Pergunta>> = Vec::new();
    for p in perguntas {
        match artigos.last_mut() {
            Some(grupo) if grupo[0].id == p.id => grupo.push(p),
            _ => artigos.push(vec![p]),
        }
    }
    FaqNode {
        title: "Perguntas frequentes".to_string(),
        url: format!("{}/", ce::BASE),
        page_type: PageType::Menu,
        origin: String::new(),
        children: artigos
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

/// HTML → texto pelo conversor da frota (`auli_scraper_kit::html`, D-KITHTML-1): no modo `markdown` (o
/// `contentMarkdown` do Sydle) a quebra do fonte é quebra.
fn html_para_texto(s: &str, markdown: bool) -> String {
    auli_scraper_kit::html::html_para_texto(
        s,
        auli_scraper_kit::html::Opcoes {
            base: Some(ce::BASE),
            quebras_do_fonte: markdown,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // Da coleta de 03/11/2026 (pacotes do estudo), só com os campos que o scraper lê. Artigos: CTe (template de
    // 2024, 46 seções, a tag `<sy-collapse>` com `>` no atributo), Diferimento (template de 2026, 17 seções, com
    // entidades e 5 perguntas repetidas no próprio artigo), Bancos Conveniados (`contentFormat` HTML, sem seções)
    // e um recorte da Escrituração (4 blocos: o vizinho, os dois da âncora repetida e o último, com o `<script>`).
    // Listagem: a página 11 (nenhum artigo de FAQ), a 15 (7 de 10) e a 44 (vazia, a terminadora).
    const CTE: &str = include_str!("../tests/fixtures/ce-faqs-item-cte.json");
    const DIFERIMENTO: &str = include_str!("../tests/fixtures/ce-faqs-item-diferimento.json");
    const BANCOS: &str = include_str!("../tests/fixtures/ce-faqs-item-bancos.json");
    const ESCRITURACAO: &str =
        include_str!("../tests/fixtures/ce-faqs-item-escrituracao-recorte.json");
    const CG11: &str = include_str!("../tests/fixtures/ce-faqs-cg-11.json");
    const CG15: &str = include_str!("../tests/fixtures/ce-faqs-cg-15.json");
    const CG44: &str = include_str!("../tests/fixtures/ce-faqs-cg-44.json");

    const ID_CTE: &str = "64da50f7170f5a26dc5e9ea5";
    const ID_DIFERIMENTO: &str = "6504811762f08b64cdd578ba";

    fn perguntas(corpo: &str, id: &str) -> Vec<Pergunta> {
        secoes(&parse_artigo(corpo, id).unwrap()).unwrap().unwrap()
    }

    fn artigo(md: &str) -> Artigo {
        Artigo {
            id: "a".into(),
            identifier: "x".into(),
            name: "Dúvidas sobre X".into(),
            formato: Some("MARKDOWN".into()),
            markdown: Some(md.into()),
        }
    }

    fn bloco_2026(slug: &str, pergunta: &str, resposta: &str) -> String {
        format!(
            "{BLOCO}\n    <div class=\"heading\" id=\"section-{slug}\"><h2>{pergunta}</h2></div>\n    \
             <sy-collapse class=\"hydrated collapse service-section\" trigger-by=\"#section-{slug}\">\n    \
             <div class=\"content\"><p>{resposta}</p></div>\n</div>\n"
        )
    }

    fn pergunta(id: &str, slug: &str, p: &str, r: &str) -> Pergunta {
        Pergunta {
            id: id.into(),
            pergunta: p.into(),
            resposta: r.into(),
            artigo: format!("Artigo {id}"),
            url_artigo: format!("https://h/x+{id}"),
            url: format!("https://h/x+{id}#section-{slug}"),
        }
    }

    #[test]
    fn template_2024_pergunta_na_linha_markdown_e_tag_ate_o_fim_da_linha() {
        let ps = perguntas(CTE, ID_CTE);
        assert_eq!(ps.len(), 46);
        assert_eq!(
            ps[0].pergunta,
            "A consulta da validade, existência e autorização de um CT-e é obrigatória ou facultativa?"
        );
        assert_eq!(
            ps[0].resposta,
            "É obrigatória. Caberá ao tomador do serviço certificar-se da validade do documento, mediante \
             consulta pública."
        );
        assert!(ps.iter().all(|p| !p.resposta.contains("handleState")));
    }

    #[test]
    fn template_2024_mantem_as_quebras_de_linha_do_markdown() {
        let ps = perguntas(CTE, ID_CTE);
        let p = ps
            .iter()
            .find(|p| p.pergunta == "Como emitir o CTe de substituição?")
            .unwrap();
        assert!(
            p.resposta
                .contains("Confira quais são as situações:\n1 - Para valores - É possível"),
            "{:.200}",
            p.resposta
        );
    }

    #[test]
    fn template_2026_pergunta_no_h2_e_entidades_decodificadas() {
        let ps = perguntas(DIFERIMENTO, ID_DIFERIMENTO);
        assert_eq!(ps.len(), 17);
        assert_eq!(
            ps[0].pergunta,
            "Aplica-se o diferimento do ICMS às operações sujeitas ao regime de Substituição Tributária?"
        );
        assert!(
            ps[0].resposta.starts_with(
                "Não. Salvo disposição da legislação em contrário, fica vedada a aplicação do diferimento"
            ),
            "{:.120}",
            ps[0].resposta
        );
        // O último bloco carrega o `<script>` do botão "Expandir todas": sai inteiro.
        let ultima = ps.last().unwrap();
        assert_eq!(
            ultima.resposta,
            "Não. Se a etapa do diferimento termina com uma operação de exportação de mercadorias, o ICMS \
             diferido não será exigido."
        );
    }

    #[test]
    fn url_da_pergunta_e_o_artigo_com_a_ancora_da_secao() {
        let ps = perguntas(CTE, ID_CTE);
        assert_eq!(
            ps[0].url,
            "https://portalservicos.sefaz.ce.gov.br/principais-duvidas-sobre-cte-conhecimento-de-transporte-\
             eletronico+64da50f7170f5a26dc5e9ea5#section-a-consulta-da-validade-existencia-e-autorizacao-de-um-\
             ct-e-e-obrigatoria-ou-facultativa"
        );
        assert_eq!(
            ps[0].url_artigo,
            "https://portalservicos.sefaz.ce.gov.br/principais-duvidas-sobre-cte-conhecimento-de-transporte-\
             eletronico+64da50f7170f5a26dc5e9ea5"
        );
        assert_eq!(
            ps[0].artigo,
            "Principais Dúvidas sobre CTe - Conhecimento de Transporte Eletrônico"
        );
    }

    #[test]
    fn artigo_html_sem_secoes_fica_fora() {
        let a = parse_artigo(BANCOS, "6501e95d3c373727041947fe").unwrap();
        assert_eq!(a.formato.as_deref(), Some("HTML"));
        assert!(secoes(&a).unwrap().is_none());
        assert!(secoes(&artigo("<p>-</p>")).unwrap().is_none());
    }

    #[test]
    fn ancora_repetida_e_aviso_e_as_duas_perguntas_entram() {
        let ps = perguntas(ESCRITURACAO, "64da510a170f5a26dc5f377d");
        assert_eq!(ps.len(), 4);
        assert_eq!(ps[1].url, ps[2].url);
        assert_ne!(ps[1].pergunta, ps[2].pergunta, "ICMS/IPI e ICMS|IPI");
        assert!(
            ps[1]
                .resposta
                .starts_with("Primeiramente, é necessário consultar")
        );
        assert!(ps[2].resposta.starts_with("Será preciso consultar"));
        assert!(sem_colisao(&sem_copias(ps)).is_ok());
    }

    #[test]
    fn selecao_pelo_nome_na_ordem_da_listagem() {
        let pg15 = ce::parse_page(CG15).unwrap().items;
        let nomes: Vec<&str> = de_faq(&pg15).iter().map(|i| i.name.as_str()).collect();
        assert_eq!(nomes.len(), 7, "{nomes:?}");
        assert!(nomes.iter().all(|n| FAQ_RE.is_match(n)));
        assert!(de_faq(&ce::parse_page(CG11).unwrap().items).is_empty());
        assert!(ce::parse_page(CG44).unwrap().items.is_empty());
    }

    #[test]
    fn copias_ficam_no_menor_id_e_ignoram_quebras_de_linha() {
        // As 5 perguntas que o Diferimento repete no próprio artigo saem.
        assert_eq!(sem_copias(perguntas(DIFERIMENTO, ID_DIFERIMENTO)).len(), 12);
        let ps = sem_copias(vec![
            pergunta("b", "s1", "P?", "uma\nresposta"),
            pergunta("a", "s9", "P?", "uma resposta"),
            pergunta("b", "s2", "Q?", "outra"),
        ]);
        let urls: Vec<&str> = ps.iter().map(|p| p.url.as_str()).collect();
        assert_eq!(
            urls,
            ["https://h/x+a#section-s9", "https://h/x+b#section-s2"]
        );
    }

    #[test]
    fn mesma_ancora_e_pergunta_com_outra_resposta_e_colisao() {
        let ps = vec![
            pergunta("a", "s", "P?", "um"),
            pergunta("a", "s", "P?", "dois"),
        ];
        assert!(sem_colisao(&ps).is_err());
        let ps = vec![
            pergunta("a", "s", "P?", "um"),
            pergunta("a", "s-2", "P?", "dois"),
        ];
        assert!(sem_colisao(&ps).is_ok());
    }

    #[test]
    fn template_quebrado_e_erro() {
        let bom = bloco_2026("s", "P?", "R.");
        assert_eq!(secoes(&artigo(&bom)).unwrap().unwrap().len(), 1);
        // Âncora a mais (bloco sem o `section-wrapper`).
        let a_mais = format!("{bom}<div id=\"section-t\">");
        assert!(secoes(&artigo(&a_mais)).is_err());
        assert!(secoes(&artigo(&bloco_2026("s", " ", "R."))).is_err());
        assert!(secoes(&artigo(&bloco_2026("s", "P?", "&nbsp;"))).is_err());
        let sem_collapse = bom.replace("sy-collapse", "div");
        assert!(secoes(&artigo(&sem_collapse)).is_err());
        let sem_pergunta = bom.replace("<h2>P?</h2>", "");
        assert!(secoes(&artigo(&sem_pergunta)).is_err());
    }

    #[test]
    fn get_item_de_outro_artigo_e_erro() {
        assert!(parse_artigo(CTE, ID_CTE).is_ok());
        assert!(parse_artigo(CTE, ID_DIFERIMENTO).is_err());
        assert!(parse_artigo("<html>", ID_CTE).is_err());
    }

    #[test]
    fn links_e_listas_viram_texto() {
        let html = r#"<p>Veja <a href="https://x.ce.gov.br/a">o manual</a> e <a href="https://y">https://y</a>.</p><ul><li>um</li><li>dois &amp; tr&ecirc;s</li></ul><a href="javascript:void(0)">nada</a>"#;
        assert_eq!(
            html_para_texto(html, false),
            "Veja o manual \"https://x.ce.gov.br/a\" e \"https://y\".\num\ndois & três\nnada"
        );
        // No HTML a quebra do fonte é espaço; no markdown, quebra.
        assert_eq!(html_para_texto("a\nb", false), "a b");
        assert_eq!(html_para_texto("a\n- b", true), "a\n- b");
    }

    #[test]
    fn arvore_raiz_artigos_perguntas() {
        let ps = vec![
            pergunta("a", "s1", "P1?", "R1"),
            pergunta("a", "s2", "P2?", "R2"),
            pergunta("b", "s1", "P3?", "R3"),
        ];
        let raiz = montar_arvore(&ps);
        assert_eq!(raiz.page_type, PageType::Menu);
        assert_eq!(raiz.children.len(), 2);
        let a = &raiz.children[0];
        assert_eq!(
            (a.title.as_str(), a.url.as_str()),
            ("Artigo a", "https://h/x+a")
        );
        assert_eq!(a.page_type, PageType::Faq);
        assert_eq!(a.faq_items.len(), 2);
        assert_eq!(
            a.faq_items[1].url.as_deref(),
            Some("https://h/x+a#section-s2")
        );
        assert_eq!(raiz.children[1].faq_items.len(), 1);
    }
}
