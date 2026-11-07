// SEFAZ-DF FAQs — duas fontes oficiais, ambas HTML servido pelo órgão, sem navegador. Estudo, medições e
// decisões: docs/ESTUDO-DF-Faqs.md (D-DFFAQ-*).
//
// (A) Receita — as "Perguntas e Respostas - FAQ" do Portal da Receita (`aplicacoes/perguntasfrequentes/`,
//     ColdFusion, o mesmo host da Carta de Serviços). A página é uma casca com os temas; as perguntas de um tema
//     chegam por POST de formulário, inteiras (`tipo=0`) ou por tipo de atendimento. O host tem o WAF JA3 dos
//     serviços: tudo pelo curl, com o AuliBot (D-DFFAQ-2/8).
// (B) ISS/NFS-e — as Perguntas Frequentes do sistema do ISS (`iss.fazenda.df.gov.br`, outro host, sem
//     `robots.txt`): uma página só, com as perguntas agrupadas por seção. O `ureq` passa.

use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::sync::LazyLock;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use regex::Regex;
use scraper::{ElementRef, Html, Selector};

use auli_contract::FaqRaw;
use auli_scraper_kit::clean;
use auli_scraper_kit::faqtree::{FaqItem, FaqNode, PageType};
use auli_scraper_kit::http::GetOpts;

const RECEITA: &str = "https://www.receita.fazenda.df.gov.br/aplicacoes/perguntasfrequentes";
const ISS: &str = "https://iss.fazenda.df.gov.br/online/Default/PerguntasFrequentes.aspx";
/// O primeiro nível da trilha de cada fonte.
const FONTE_RECEITA: &str = "Receita-DF";
const FONTE_ISS: &str = "ISS/NFS-e";

const COURTESY: Duration = Duration::from_millis(1000);

/// Pisos (estudo §4, revistos na F0): 22 temas, 900 perguntas da Receita e 95 do ISS em 02/11/2026.
const MIN_TEMAS: usize = 18;
const MIN_RECEITA: usize = 800;
const MIN_ISS: usize = 80;

/// Coleta as duas fontes e grava a árvore do frontend (`<id>-faqs-tree.json`) e o snapshot. As guardas rodam
/// antes de qualquer gravação: rodada que falha deixa os arquivos anteriores intactos.
pub fn run(data_dir: &str, use_cache: bool) -> Result<()> {
    let temas = ler_casca(&buscar_receita(data_dir, None, use_cache)?)?;
    piso(temas.len(), MIN_TEMAS, "tema(s) na casca")?;
    let mut perguntas = Vec::new();
    for tema in &temas {
        let cards = ler_cards(&buscar_receita(data_dir, Some((tema.cod, 0)), use_cache)?)
            .with_context(|| format!("tema {} ({})", tema.nome, tema.cod))?;
        let mut por_tipo = Vec::new();
        for (cod, rotulo) in &tema.tipos {
            let ids = ler_cards(&buscar_receita(
                data_dir,
                Some((tema.cod, *cod)),
                use_cache,
            )?)
            .with_context(|| format!("tema {} ({}), tipo {cod}", tema.nome, tema.cod))?
            .iter()
            .map(|c| c.id)
            .collect();
            por_tipo.push((rotulo.as_str(), ids));
        }
        perguntas.extend(classificar(tema, cards, &por_tipo)?);
    }
    ids_unicos(&perguntas)?;
    piso(perguntas.len(), MIN_RECEITA, "pergunta(s) da Receita")?;
    let da_receita = perguntas.len();

    let agent = auli_scraper_kit::build_agent(crate::UA, Some(Duration::from_secs(60)));
    let iss = com_cache(data_dir, ISS, use_cache, || {
        auli_scraper_kit::http::get_string(
            &agent,
            ISS,
            &GetOpts {
                log_prefix: "DF FAQs",
                ..Default::default()
            },
        )
    })?;
    let iss = ler_iss(&iss)?;
    piso(iss.len(), MIN_ISS, "pergunta(s) do ISS")?;
    perguntas.extend(iss);
    println!(
        "DF FAQs: {da_receita} da Receita em {} temas + {} do ISS",
        temas.len(),
        perguntas.len() - da_receita
    );

    let faqs = consolidar(&perguntas);
    // Atômico pelo mesmo motivo das outras entidades: a árvore não tem outra fonte (o snapshot achatado
    // não a reconstrói) e é o único artefato de coleta que o frontend lê direto.
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

/// A casca (`None`) ou as perguntas de um tema, inteiras (`tipo` 0) ou de um tipo de atendimento — o POST que
/// a página faz quando o visitante escolhe o tipo. A chave do cache leva os campos do formulário.
fn buscar_receita(data_dir: &str, consulta: Option<(u32, u32)>, use_cache: bool) -> Result<String> {
    let opts = GetOpts {
        log_prefix: "DF FAQs",
        ..Default::default()
    };
    let casca = format!("{RECEITA}/perguntasfrequentes.cfm");
    let busca = format!("{RECEITA}/perguntasFreqBusca_div.cfm");
    let corpo = match consulta {
        None => com_cache(data_dir, &casca, use_cache, || {
            auli_scraper_kit::http::get_via_curl(&casca, crate::UA, &opts)
        })?,
        Some((tema, tipo)) => {
            let chave = format!("{busca}?codAssunto={tema}&selectTipoAtendimento={tipo}");
            com_cache(data_dir, &chave, use_cache, || {
                auli_scraper_kit::http::post_form_via_curl(
                    &busca,
                    crate::UA,
                    &[
                        ("selectTipoAtendimento", &tipo.to_string()),
                        ("codAssunto", &tema.to_string()),
                        ("buscaPergunta", ""),
                    ],
                    &opts,
                )
            })?
        }
    };
    if corpo.contains("Error Occurred While Processing Request") {
        bail!("erro do ColdFusion na consulta {consulta:?} (parâmetros inválidos / app mudou?)");
    }
    Ok(corpo)
}

/// Uma página (D-DFFAQ-8): rede sempre, gravando o cache; com `--usecache`, só o cache, e miss é erro.
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

struct Tema {
    cod: u32,
    nome: String,
    /// `(código, rótulo)` dos tipos de atendimento, na ordem do `select`.
    tipos: Vec<(u32, String)>,
}

fn seletor(css: &str) -> Selector {
    Selector::parse(css).unwrap()
}

/// Os temas da casca: um formulário por tema (`formBusca<cod>`, com `codAssunto` e o `select` dos tipos) e o
/// título no botão que abre o tema. O formulário da busca geral não tem `codAssunto` e fica de fora.
fn ler_casca(html: &str) -> Result<Vec<Tema>> {
    let doc = Html::parse_document(html);
    let mut temas = Vec::new();
    let mut vistos = BTreeSet::new();
    for form in doc.select(&seletor(r#"form[id^="formBusca"]"#)) {
        let Some(cod) = form
            .select(&seletor(r#"input[name="codAssunto"]"#))
            .next()
            .and_then(|i| i.value().attr("value"))
        else {
            continue;
        };
        let cod: u32 = cod
            .trim()
            .parse()
            .with_context(|| format!("codAssunto {cod:?} não é número"))?;
        if !vistos.insert(cod) {
            bail!("o tema {cod} aparece duas vezes na casca");
        }
        let nome = doc
            .select(&seletor(&format!(
                r##"button[data-target="#collapseOne{cod}"] h5"##
            )))
            .next()
            .map(|h| clean(&h.text().collect::<String>()))
            .filter(|n| !n.is_empty())
            .ok_or_else(|| anyhow!("o tema {cod} não tem título na casca"))?;
        let mut tipos = Vec::new();
        for opt in form.select(&seletor(r#"select[name="selectTipoAtendimento"] option"#)) {
            let valor = opt.value().attr("value").unwrap_or("").trim();
            if valor == "0" {
                continue;
            }
            let cod_tipo: u32 = valor
                .parse()
                .with_context(|| format!("tipo {valor:?} do tema {cod} não é número"))?;
            tipos.push((cod_tipo, rotulo_do_tipo(&opt.text().collect::<String>())));
        }
        temas.push(Tema { cod, nome, tipos });
    }
    Ok(temas)
}

/// 49 dos 81 rótulos terminam em " - serviço" (02/11/2026), sem informação na trilha.
fn rotulo_do_tipo(s: &str) -> String {
    let r = clean(s);
    match r.strip_suffix(" - serviço") {
        Some(sem) => sem.to_string(),
        None => r,
    }
}

struct Card {
    id: u32,
    pergunta_html: String,
    resposta_html: String,
}

/// O fragmento da busca: um `div.card` por pergunta, com o link que abre a resposta
/// (`data-target="#collapse<id>"`) e a resposta no `div#collapse<id>`. O `id` é a identidade.
///
/// Cada pergunta vem embrulhada no seu `div#collapsePerguntasFrequentes`, e o fragmento é cortado nesses
/// embrulhos antes do parse: há respostas coladas do Word que o banco do portal trunca em ~64 KB no meio de
/// um comentário condicional (`<!--` sem `-->`, 3 em 02/11/2026). Num parse só, o comentário aberto engole as
/// perguntas seguintes — no navegador, o tema ITCD inteiro mostra 13 das 28. Cortado, ele fica na resposta
/// dele, e o texto útil vem antes do corte.
fn ler_cards(html: &str) -> Result<Vec<Card>> {
    let sel_card = seletor("div.card");
    let mut cards = Vec::new();
    for pedaco in html.split(EMBRULHO).skip(1) {
        let doc = Html::parse_fragment(pedaco);
        let mut achados = doc.select(&sel_card);
        let (Some(card), None) = (achados.next(), achados.next()) else {
            bail!("um embrulho de pergunta sem exatamente um div.card — o fragmento mudou?");
        };
        let link = card
            .select(&seletor(
                r##"a[data-toggle="collapse"][data-target^="#collapse"]"##,
            ))
            .next()
            .ok_or_else(|| anyhow!("um div.card sem o link da pergunta — o fragmento mudou?"))?;
        let alvo = link.value().attr("data-target").unwrap_or("");
        let id: u32 = alvo
            .trim_start_matches("#collapse")
            .parse()
            .with_context(|| format!("data-target {alvo:?} sem id numérico"))?;
        let resposta = card
            .select(&seletor(&format!(r#"div[id="collapse{id}"]"#)))
            .next()
            .ok_or_else(|| anyhow!("a pergunta {id} não tem o div da resposta"))?;
        cards.push(Card {
            id,
            pergunta_html: link.inner_html(),
            resposta_html: resposta.inner_html(),
        });
    }
    Ok(cards)
}

/// O começo do embrulho de cada pergunta no fragmento da busca.
const EMBRULHO: &str = r#"<div id="collapsePerguntasFrequentes""#;

struct Pergunta {
    pergunta: String,
    resposta: String,
    /// A trilha, da fonte ao nível mais fundo.
    caminho: Vec<String>,
    link: String,
    /// O id da Receita (o ISS não tem identidade estável: os ids da página mudam a cada carga).
    id: Option<u32>,
}

/// As perguntas de um tema, na ordem do portal, com o tipo de atendimento como 3º nível da trilha
/// (D-DFFAQ-3). Os tipos repartem o tema (02/11/2026: nenhum id fora do tema, nenhum em dois tipos, 1 sem
/// tipo): id de tipo fora do tema é `Err`; id sem tipo fica no tema; id em dois tipos fica no primeiro.
fn classificar(
    tema: &Tema,
    cards: Vec<Card>,
    por_tipo: &[(&str, Vec<u32>)],
) -> Result<Vec<Pergunta>> {
    let do_tema: BTreeSet<u32> = cards.iter().map(|c| c.id).collect();
    let mut tipo_de: HashMap<u32, &str> = HashMap::new();
    let mut em_dois = 0;
    for (rotulo, ids) in por_tipo {
        for id in ids {
            if !do_tema.contains(id) {
                bail!(
                    "o tipo {rotulo:?} traz a pergunta {id}, que não está no tema {}",
                    tema.nome
                );
            }
            match tipo_de.entry(*id) {
                std::collections::hash_map::Entry::Occupied(_) => em_dois += 1,
                std::collections::hash_map::Entry::Vacant(v) => {
                    v.insert(rotulo);
                }
            }
        }
    }
    if em_dois > 0 {
        eprintln!(
            "⚠️  DF FAQs: {em_dois} pergunta(s) do tema {} em dois tipos — fica o primeiro",
            tema.nome
        );
    }
    if cards.is_empty() {
        eprintln!("⚠️  DF FAQs: o tema {} veio sem perguntas", tema.nome);
    }
    let mut perguntas = Vec::new();
    for card in cards {
        let pergunta = uma_linha(&card.pergunta_html, RECEITA);
        if pergunta.is_empty() {
            bail!("a pergunta {} do tema {} veio vazia", card.id, tema.nome);
        }
        let mut caminho = vec![FONTE_RECEITA.to_string(), tema.nome.clone()];
        if let Some(rotulo) = tipo_de.get(&card.id) {
            caminho.push(rotulo.to_string());
        }
        perguntas.push(Pergunta {
            pergunta,
            resposta: sem_prefixo_r(&html_para_texto(&card.resposta_html, RECEITA)),
            caminho,
            link: link_da_receita(card.id),
            id: Some(card.id),
        });
    }
    Ok(perguntas)
}

fn ids_unicos(perguntas: &[Pergunta]) -> Result<()> {
    let mut ids = BTreeSet::new();
    for id in perguntas.iter().filter_map(|p| p.id) {
        if !ids.insert(id) {
            bail!("a pergunta {id} da Receita aparece em dois temas");
        }
    }
    Ok(())
}

/// Coleta capada (página ou busca que mudou) é `Err`, não snapshot menor.
fn piso(n: usize, minimo: usize, oque: &str) -> Result<()> {
    if n < minimo {
        bail!("{n} {oque} (mínimo {minimo}) — a página mudou?");
    }
    Ok(())
}

/// A casca com o id da pergunta no fragmento (D-DFFAQ-4): a app não tem endereço por pergunta, e 50 textos de
/// pergunta se repetem (02/11/2026) — o nome do `.md` sai de link + pergunta. O `#collapse<id>` é o id do
/// portal; o navegador abre a casca.
fn link_da_receita(id: u32) -> String {
    format!("{RECEITA}/perguntasfrequentes.cfm#collapse{id}")
}

static PREFIXO_R_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(?:R\s*[.:]+\s*)+").unwrap());

/// As respostas da Receita abrem com "R." (723 de 900): às vezes "R:", "R .", "R.:", "R." colado, "R." duas
/// vezes (um parágrafo só com "R." antes do primeiro) ou "R" sem pontuação antes da maiúscula. Sai só no
/// começo.
fn sem_prefixo_r(s: &str) -> String {
    let s = PREFIXO_R_RE.replace(s, "");
    match s.strip_prefix("R ") {
        Some(resto) if resto.starts_with(char::is_uppercase) => resto.to_string(),
        _ => s.into_owned(),
    }
}

/// As perguntas do ISS: cada `<h5>` abre uma seção, e os pares `div.accordionHeader` (pergunta) /
/// `div.accordionContent` (resposta) seguintes são dela. A seção do rodapé ("Não encontrou o que procurava?")
/// não tem pares. Pergunta repetida na mesma seção com a mesma resposta sai uma vez (2 em 02/11/2026; uma
/// delas difere só nas quebras de linha — a comparação ignora o espaço); com resposta diferente é `Err` — o
/// link é o da seção, e um `.md` sobrescreveria o outro.
fn ler_iss(html: &str) -> Result<Vec<Pergunta>> {
    let doc = Html::parse_document(html);
    let mut secao: Option<String> = None;
    let mut pendente: Option<String> = None;
    // (seção, pergunta) → a resposta com o espaço normalizado.
    let mut vistas: HashMap<(String, String), String> = HashMap::new();
    let (mut repetidas, mut vazias) = (0, 0);
    let mut perguntas = Vec::new();
    for e in doc.select(&seletor("h5, div.accordionHeader, div.accordionContent")) {
        if e.value().name() == "h5" {
            if !dentro_de_resposta(e) {
                secao = Some(clean(&e.text().collect::<String>()));
            }
        } else if e
            .value()
            .has_class("accordionHeader", scraper::CaseSensitivity::CaseSensitive)
        {
            pendente = Some(uma_linha(&e.inner_html(), ISS));
        } else {
            let pergunta = pendente
                .take()
                .ok_or_else(|| anyhow!("ISS: resposta sem pergunta antes — a página mudou?"))?;
            let secao = secao.clone().ok_or_else(|| {
                anyhow!("ISS: pergunta antes da primeira seção — a página mudou?")
            })?;
            if pergunta.is_empty() {
                bail!("ISS: pergunta vazia na seção {secao:?}");
            }
            let resposta = html_para_texto(&e.inner_html(), ISS);
            if resposta.is_empty() {
                vazias += 1;
                continue;
            }
            let chave = (secao.clone(), pergunta.to_lowercase());
            let comparavel = resposta.split_whitespace().collect::<Vec<_>>().join(" ");
            if let Some(anterior) = vistas.get(&chave) {
                if *anterior != comparavel {
                    bail!(
                        "ISS: a seção {secao:?} traz {pergunta:?} duas vezes, com respostas diferentes"
                    );
                }
                repetidas += 1;
                continue;
            }
            vistas.insert(chave, comparavel);
            perguntas.push(Pergunta {
                link: format!("{ISS}#{}", auli_contract::mddoc::slug(&secao)),
                caminho: vec![FONTE_ISS.to_string(), secao],
                pergunta,
                resposta,
                id: None,
            });
        }
    }
    if repetidas > 0 {
        eprintln!(
            "⚠️  DF FAQs: {repetidas} pergunta(s) do ISS repetidas na mesma seção — saem uma vez"
        );
    }
    if vazias > 0 {
        eprintln!("⚠️  DF FAQs: {vazias} pergunta(s) do ISS sem resposta — descartadas");
    }
    Ok(perguntas)
}

fn dentro_de_resposta(e: ElementRef) -> bool {
    e.ancestors().filter_map(ElementRef::wrap).any(|a| {
        a.value()
            .has_class("accordionContent", scraper::CaseSensitivity::CaseSensitive)
    })
}

fn consolidar(perguntas: &[Pergunta]) -> Vec<FaqRaw> {
    let sem_resposta = perguntas.iter().filter(|p| p.resposta.is_empty()).count();
    if sem_resposta > 0 {
        eprintln!("⚠️  DF FAQs: {sem_resposta} pergunta(s) com resposta vazia depois da conversão");
    }
    perguntas
        .iter()
        .map(|p| FaqRaw {
            pergunta: p.pergunta.clone(),
            resposta: p.resposta.clone(),
            origin: p.caminho.join(" | "),
            url: p.link.clone(),
        })
        .collect()
}

/// Um nível da árvore, com os filhos na ordem em que aparecem (a do portal).
#[derive(Default)]
struct Ramo<'a> {
    filhos: Vec<(&'a str, Ramo<'a>)>,
    perguntas: Vec<&'a Pergunta>,
}

/// A árvore da aba de FAQs: raiz → as duas fontes → temas e tipos (Receita) ou seções (ISS). A interface
/// começa pelos filhos da raiz. Os nós apontam para a página da fonte; cada pergunta tem o seu link.
fn montar_arvore(perguntas: &[Pergunta]) -> FaqNode {
    let mut raiz = Ramo::default();
    for p in perguntas {
        let mut ramo = &mut raiz;
        for nivel in &p.caminho {
            let i = match ramo.filhos.iter().position(|(t, _)| *t == nivel.as_str()) {
                Some(i) => i,
                None => {
                    ramo.filhos.push((nivel.as_str(), Ramo::default()));
                    ramo.filhos.len() - 1
                }
            };
            ramo = &mut ramo.filhos[i].1;
        }
        ramo.perguntas.push(p);
    }
    FaqNode {
        title: "Perguntas frequentes".to_string(),
        url: pagina_da_fonte(FONTE_RECEITA),
        page_type: PageType::Menu,
        origin: String::new(),
        children: raiz
            .filhos
            .iter()
            .map(|(fonte, r)| no(fonte, fonte, &pagina_da_fonte(fonte), r))
            .collect(),
        faq_items: Vec::new(),
    }
}

fn pagina_da_fonte(fonte: &str) -> String {
    if fonte == FONTE_ISS {
        ISS.to_string()
    } else {
        format!("{RECEITA}/perguntasfrequentes.cfm")
    }
}

fn no(titulo: &str, trilha: &str, url: &str, ramo: &Ramo) -> FaqNode {
    FaqNode {
        title: titulo.to_string(),
        url: url.to_string(),
        page_type: if ramo.filhos.is_empty() {
            PageType::Faq
        } else {
            PageType::Menu
        },
        origin: if ramo.perguntas.is_empty() {
            String::new()
        } else {
            trilha.to_string()
        },
        children: ramo
            .filhos
            .iter()
            .map(|(t, r)| no(t, &format!("{trilha} | {t}"), url, r))
            .collect(),
        faq_items: ramo
            .perguntas
            .iter()
            .map(|p| FaqItem {
                pergunta: p.pergunta.clone(),
                resposta: p.resposta.clone(),
                url: Some(p.link.clone()),
            })
            .collect(),
    }
}

/// HTML → texto pelo conversor da frota (`auli_scraper_kit::html`, D-KITHTML-1): links da raiz com o host da página
/// (Receita ou ISS).
fn html_para_texto(s: &str, base: &str) -> String {
    auli_scraper_kit::html::html_para_texto(
        s,
        auli_scraper_kit::html::Opcoes {
            base: Some(base),
            quebras_do_fonte: false,
        },
    )
}

/// Uma pergunta numa linha só.
fn uma_linha(s: &str, base: &str) -> String {
    clean(&html_para_texto(s, base).replace('\n', " "))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Recortes da coleta de 02/11/2026: a casca com o formulário da busca geral e dois temas (24, com 2 tipos;
    // 4250, com o rótulo " - serviço"); o tema 24 inteiro (18 perguntas) e os seus dois tipos (8 + 10); a
    // pergunta 408, colada do Word (comentários condicionais e CSS `mso-`); e duas seções do ISS (10 + 7 pares)
    // com o começo do rodapé.
    const CASCA: &str = include_str!("../tests/fixtures/df-faqs-casca.html");
    const TEMA_24: &str = include_str!("../tests/fixtures/df-faqs-24-0.html");
    const TIPO_80: &str = include_str!("../tests/fixtures/df-faqs-24-80.html");
    const TIPO_187: &str = include_str!("../tests/fixtures/df-faqs-24-187.html");
    const WORD_408: &str = include_str!("../tests/fixtures/df-faqs-word-408.html");
    const ISS_RECORTE: &str = include_str!("../tests/fixtures/df-faqs-iss.html");

    fn tema(cod: u32, nome: &str, tipos: &[(u32, &str)]) -> Tema {
        Tema {
            cod,
            nome: nome.to_string(),
            tipos: tipos.iter().map(|(c, r)| (*c, r.to_string())).collect(),
        }
    }

    fn ids(html: &str) -> Vec<u32> {
        ler_cards(html).unwrap().iter().map(|c| c.id).collect()
    }

    /// Um card no formato do fragmento da busca.
    fn card(id: u32, pergunta: &str, resposta: &str) -> String {
        format!(
            r##"<div id="collapsePerguntasFrequentes" class="collapse show"><div class="card-body p-0 border-0" id="child1">
            <div class="card mb-1"><div class="card-header bg-light border">
            <a href="#" data-toggle="collapse" data-target="#collapse{id}">{pergunta}</a></div>
            <div class="card-body collapse border" data-parent="#child1" id="collapse{id}">{resposta}</div>
            </div></div></div>"##
        )
    }

    #[test]
    fn casca_da_os_temas_com_codigo_titulo_e_tipos_sem_a_busca_geral() {
        let temas = ler_casca(CASCA).unwrap();
        assert_eq!(temas.len(), 2);
        assert_eq!((temas[0].cod, temas[0].nome.as_str()), (24, "Agênci@Net"));
        assert_eq!(
            temas[0].tipos,
            [
                (80, "Obter Outras Informações sobre Agenci@Net".to_string()),
                (
                    187,
                    "Procuração Eletrônica - Obter Informações e Verificar Erros".to_string()
                )
            ]
        );
        assert_eq!(
            temas[1].tipos,
            [(
                17002,
                "Restituir ou Compensar Tributos Diretos Pessoa Física".to_string()
            )]
        );
    }

    #[test]
    fn tema_repetido_na_casca_e_erro() {
        let dobrada = CASCA.replace(r#"value="4250""#, r#"value="24""#);
        assert!(ler_casca(&dobrada).is_err());
    }

    #[test]
    fn fragmento_do_tema_da_uma_pergunta_por_card_com_o_id_do_data_target() {
        let cards = ler_cards(TEMA_24).unwrap();
        assert_eq!(cards.len(), 18);
        assert_eq!(cards[0].id, 209);
        assert_eq!(
            uma_linha(&cards[0].pergunta_html, RECEITA),
            "O que é Agênci@net?"
        );
        let mut todos = ids(TIPO_80);
        todos.extend(ids(TIPO_187));
        todos.sort();
        let mut do_tema = ids(TEMA_24);
        do_tema.sort();
        assert_eq!(todos, do_tema, "os dois tipos repartem o tema");
    }

    #[test]
    fn comentario_do_word_truncado_nao_engole_as_perguntas_seguintes() {
        let truncada =
            "<p>R. Haverá incidência do ITCD.</p><!-- [if gte mso 9]><xml><w:LatentStyles>";
        let fragmento =
            card(363, "Incide ITCD?", truncada) + &card(379, "E a seguinte?", "<p>R. Sim.</p>");
        let cards = ler_cards(&fragmento).unwrap();
        assert_eq!(cards.iter().map(|c| c.id).collect::<Vec<_>>(), [363, 379]);
        assert_eq!(
            sem_prefixo_r(&html_para_texto(&cards[0].resposta_html, RECEITA)),
            "Haverá incidência do ITCD."
        );
    }

    #[test]
    fn tipos_viram_o_terceiro_nivel_da_trilha_e_o_link_leva_o_id() {
        let t = tema(24, "Agênci@Net", &[(80, "Outras"), (187, "Procuração")]);
        let perguntas = classificar(
            &t,
            ler_cards(TEMA_24).unwrap(),
            &[("Outras", ids(TIPO_80)), ("Procuração", ids(TIPO_187))],
        )
        .unwrap();
        assert_eq!(perguntas.len(), 18);
        assert_eq!(perguntas[0].caminho, ["Receita-DF", "Agênci@Net", "Outras"]);
        assert_eq!(
            perguntas[0].link,
            "https://www.receita.fazenda.df.gov.br/aplicacoes/perguntasfrequentes/perguntasfrequentes.cfm#collapse209"
        );
        assert!(
            perguntas[0]
                .resposta
                .starts_with("É uma agência na internet")
        );
        let procuracao = perguntas
            .iter()
            .filter(|p| p.caminho.last().map(String::as_str) == Some("Procuração"))
            .count();
        assert_eq!(procuracao, 10);
    }

    #[test]
    fn pergunta_sem_tipo_fica_no_tema_e_a_que_esta_em_dois_tipos_fica_no_primeiro() {
        let t = tema(1, "Programa Nota Legal", &[]);
        let cards = ler_cards(&(card(1, "Um?", "a") + &card(2, "Dois?", "b"))).unwrap();
        let perguntas = classificar(&t, cards, &[("A", vec![2]), ("B", vec![2])]).unwrap();
        assert_eq!(perguntas[0].caminho, ["Receita-DF", "Programa Nota Legal"]);
        assert_eq!(
            perguntas[1].caminho,
            ["Receita-DF", "Programa Nota Legal", "A"]
        );
    }

    #[test]
    fn tipo_com_pergunta_fora_do_tema_e_erro() {
        let t = tema(24, "Agênci@Net", &[]);
        let r = classificar(
            &t,
            ler_cards(TEMA_24).unwrap(),
            &[("Outras", vec![209, 999_999])],
        );
        assert!(r.is_err());
    }

    #[test]
    fn mesma_pergunta_em_dois_temas_e_erro() {
        let t = tema(24, "Agênci@Net", &[]);
        let mut perguntas = classificar(&t, ler_cards(TEMA_24).unwrap(), &[]).unwrap();
        perguntas
            .extend(classificar(&t, ler_cards(&card(209, "De novo?", "x")).unwrap(), &[]).unwrap());
        assert!(ids_unicos(&perguntas).is_err());
    }

    #[test]
    fn resposta_colada_do_word_sai_sem_css_nem_comentario_condicional() {
        let cards = ler_cards(WORD_408).unwrap();
        let texto = sem_prefixo_r(&html_para_texto(&cards[0].resposta_html, RECEITA));
        assert!(texto.starts_with("Sim. Mas, quando há comprometimento do sigilo fiscal"));
        for lixo in [
            "mso-",
            "Normal 0",
            "Style Definitions",
            "X-NONE",
            "MsoNormalTable",
            "<",
        ] {
            assert!(!texto.contains(lixo), "vazou {lixo:?}: {texto}");
        }
    }

    #[test]
    fn prefixo_r_sai_so_no_comeco_em_todas_as_variantes() {
        for (entrada, saida) in [
            ("R. É uma agência", "É uma agência"),
            ("R: Para obter", "Para obter"),
            ("R . Quando", "Quando"),
            ("R.: Não!", "Não!"),
            ("R.Basta efetuar", "Basta efetuar"),
            ("R.\nR. É assegurado", "É assegurado"),
            ("R A solicitação", "A solicitação"),
            ("R$ 10,00 por mês", "R$ 10,00 por mês"),
            ("Resposta direta", "Resposta direta"),
            ("Sim. R. no meio fica", "Sim. R. no meio fica"),
        ] {
            assert_eq!(sem_prefixo_r(entrada), saida, "{entrada:?}");
        }
    }

    #[test]
    fn so_endereco_http_e_mailto_entra_no_texto() {
        let html = r#"<p>Veja <a href="https://www2.agencianet.fazenda.df.gov.br/x">aqui</a>, escreva para
            <a href="mailto:a@b.df.gov.br">o suporte</a>, leia <a href="/aplicacoes/y.cfm">a norma</a>,
            sem <a href="file:///C:/Users/fulano/AppData/doc.docx">o anexo</a>, <a href="javascript:__doPostBack('x','')">o botão</a>
            e <a href="O%20aumento da área">o parágrafo</a>. Manual: <a href="https://x.br/m.pdf">https://x.br/m.pdf</a></p>"#;
        assert_eq!(
            html_para_texto(html, RECEITA),
            "Veja aqui \"https://www2.agencianet.fazenda.df.gov.br/x\", escreva para o suporte \"mailto:a@b.df.gov.br\", \
             leia a norma \"https://www.receita.fazenda.df.gov.br/aplicacoes/y.cfm\", sem o anexo, o botão e o parágrafo. \
             Manual: \"https://x.br/m.pdf\""
        );
    }

    #[test]
    fn iss_da_as_perguntas_por_secao_com_o_link_da_secao_e_ignora_o_rodape() {
        let perguntas = ler_iss(ISS_RECORTE).unwrap();
        assert_eq!(perguntas.len(), 17);
        assert_eq!(perguntas[0].caminho, ["ISS/NFS-e", "Tomador de Serviços"]);
        assert_eq!(perguntas[10].caminho, ["ISS/NFS-e", "Módulos Especiais"]);
        assert_eq!(
            perguntas[0].link,
            "https://iss.fazenda.df.gov.br/online/Default/PerguntasFrequentes.aspx#tomador-de-servicos"
        );
        assert!(
            perguntas
                .iter()
                .all(|p| !p.pergunta.is_empty() && !p.resposta.is_empty())
        );
    }

    fn iss(pares: &[(&str, &str)]) -> String {
        let corpo: String = pares
            .iter()
            .map(|(p, r)| {
                format!(
                    r#"<div class="accordionHeader">{p}<i class="nc-plus pull-right"></i></div><div class="accordionContent">{r}</div>"#
                )
            })
            .collect();
        format!("<html><body><h5>Prestador de Servi&#231;os</h5><div>{corpo}</div></body></html>")
    }

    #[test]
    fn iss_repetida_na_secao_com_a_mesma_resposta_sai_uma_vez_mesmo_com_outras_quebras() {
        let html = iss(&[
            ("Como emitir?", "<p>Campo:<br>• a diária;<br>• a taxa.</p>"),
            ("Outra?", "<p>Sim.</p>"),
            ("Como emitir?", "<p>Campo: • a diária; • a taxa.</p>"),
        ]);
        let perguntas = ler_iss(&html).unwrap();
        assert_eq!(perguntas.len(), 2);
        assert_eq!(perguntas[0].resposta, "Campo:\n• a diária;\n• a taxa.");
    }

    #[test]
    fn iss_repetida_na_secao_com_respostas_diferentes_e_erro() {
        let html = iss(&[
            ("Como emitir?", "<p>Assim.</p>"),
            ("Como emitir?", "<p>De outro jeito.</p>"),
        ]);
        assert!(ler_iss(&html).is_err());
    }

    #[test]
    fn arvore_tem_uma_raiz_com_as_duas_fontes_na_ordem_e_a_pergunta_sem_tipo_no_tema() {
        let p = |caminho: &[&str], link: &str| Pergunta {
            pergunta: "P?".to_string(),
            resposta: "R".to_string(),
            caminho: caminho.iter().map(|s| s.to_string()).collect(),
            link: link.to_string(),
            id: None,
        };
        let perguntas = [
            p(&["Receita-DF", "IPVA", "Isenção"], "a#collapse1"),
            p(&["Receita-DF", "IPVA"], "a#collapse2"),
            p(&["ISS/NFS-e", "Tomador de Serviços"], "b#tomador"),
        ];
        let raiz = montar_arvore(&perguntas);
        let fontes: Vec<&str> = raiz.children.iter().map(|n| n.title.as_str()).collect();
        assert_eq!(fontes, ["Receita-DF", "ISS/NFS-e"]);
        let ipva = &raiz.children[0].children[0];
        assert!(matches!(ipva.page_type, PageType::Menu));
        assert_eq!(ipva.faq_items.len(), 1);
        assert_eq!(ipva.origin, "Receita-DF | IPVA");
        assert_eq!(ipva.children[0].origin, "Receita-DF | IPVA | Isenção");
        assert_eq!(raiz.children[1].url, ISS);
        assert_eq!(
            raiz.children[1].children[0].faq_items[0].url.as_deref(),
            Some("b#tomador")
        );
    }
}
