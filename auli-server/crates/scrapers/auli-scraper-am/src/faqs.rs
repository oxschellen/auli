// SEFAZ-AM FAQs — as "Perguntas Frequentes" que o Portfólio de Serviços publica dentro do detalhe de cada
// serviço: uma seção do `serviceDetails` no mesmo flight RSC (`RSC: 1`) que os serviços já leem. Sem navegador.
// Estudo, medições e decisões: docs/faqs/ESTUDO-AM-Faqs.md (D-AMFAQ-*).
//
// O escopo é o do `servicos`: a árvore `items` do portfólio — os serviços e as categorias que têm página de
// detalhe. O flight da listagem traz também os links do Portal do Servidor Fazendário (o RH dos servidores da
// SEFAZ-AM), que ficam fora dos dois (D-AMFAQ-10).

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::LazyLock;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use regex::Regex;
use serde::de::DeserializeOwned;

use auli_contract::FaqRaw;
use auli_scraper_kit::clean;
use auli_scraper_kit::faqtree::{FaqItem, FaqNode, PageType};

use crate::am;

/// A identidade das FAQs da frota (D-FROTA-UA); o portal a aceita (sondagem de 02/11/2026).
const UA: &str =
    "AuliBot/0.1 (+https://github.com/oxschellen/auli; carlos.schellenberger@gmail.com)";
const COURTESY: Duration = Duration::from_millis(1000);

const DETALHE: &str = "/portfolio-servicos/detalhes/";
/// A âncora do flight do detalhe: a lista das seções que a página mostra.
const ANCORA_DETALHE: &str = "\"visibleSections\":[";
const SECAO: &str = "PERGUNTAS_FREQUENTES";
const CAMPO: &str = "perguntasRespostasHtml";

/// Pisos (estudo §4, revistos na F0): 306 páginas de detalhe na árvore, 47 com perguntas e 176 perguntas depois
/// das cópias em 02/11/2026.
const MIN_PAGINAS: usize = 280;
const MIN_COM_FAQ: usize = 40;
const MIN_PERGUNTAS: usize = 150;

/// Lê a árvore do portfólio e o detalhe de cada página e grava a árvore do frontend (`<id>-faqs-tree.json`) e
/// o snapshot. As guardas rodam antes de qualquer gravação: rodada que falha deixa os arquivos anteriores
/// intactos.
pub fn run(data_dir: &str, use_cache: bool) -> Result<()> {
    let agent = auli_scraper_kit::build_agent(UA, Some(Duration::from_secs(60)));
    let lista = com_cache(data_dir, am::LISTA_URL, use_cache, || {
        am::fetch_flight(&agent, am::LISTA_URL, am::ANCORA_LISTA)
    })?;
    let paginas = paginas(&am::parse_items(&lista)?);
    piso(paginas.len(), MIN_PAGINAS, "página(s) de detalhe na árvore")?;
    let mut perguntas = Vec::new();
    let mut com_faq = 0;
    for pagina in &paginas {
        let flight = com_cache(data_dir, &pagina.link, use_cache, || {
            am::fetch_flight(&agent, &pagina.link, ANCORA_DETALHE)
        })?;
        let Some(html) = secao(&flight).with_context(|| format!("detalhe {}", pagina.link))? else {
            continue;
        };
        com_faq += 1;
        perguntas.extend(
            parear(&html_para_texto(&html, am::BASE), pagina)
                .with_context(|| format!("detalhe {}", pagina.link))?,
        );
    }
    piso(com_faq, MIN_COM_FAQ, "página(s) com perguntas")?;
    let lidas = perguntas.len();
    let perguntas = sem_copias(perguntas);
    piso(perguntas.len(), MIN_PERGUNTAS, "pergunta(s)")?;
    sem_colisao(&perguntas)?;
    println!(
        "AM FAQs: {} pergunta(s) de {com_faq} página(s) em {} — {} cópia(s) saíram",
        perguntas.len(),
        paginas.len(),
        lidas - perguntas.len()
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

/// Uma página (D-AMFAQ-7): rede sempre, gravando o cache; com `--usecache`, só o cache, e miss é erro.
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

/// Coleta capada (página ou markup que mudou) é `Err`, não snapshot menor.
fn piso(n: usize, minimo: usize, oque: &str) -> Result<()> {
    if n < minimo {
        bail!("{n} {oque} (mínimo {minimo}) — a página mudou?");
    }
    Ok(())
}

struct Pagina {
    id: i64,
    nome: String,
    /// A categoria de topo e o nome — a classe dos serviços (D-AMFAQ-3); a página de uma categoria de topo fica
    /// só com o nome.
    caminho: Vec<String>,
    /// O detalhe sem a query, como o `link` dos serviços.
    link: String,
}

/// As páginas de detalhe da árvore, na ordem do portfólio: os serviços e as categorias que têm página (63 das
/// 64 em 02/11/2026; a 302 tem perguntas). Os externos e os submenus não têm detalhe.
fn paginas(nodes: &[am::Node]) -> Vec<Pagina> {
    fn walk(
        nodes: &[am::Node],
        topo: Option<&str>,
        vistas: &mut HashSet<String>,
        out: &mut Vec<Pagina>,
    ) {
        for n in nodes {
            let nome = clean(n.name.as_deref().unwrap_or(""));
            let caminho_url = n.url.as_deref().and_then(|u| u.split('?').next());
            if let (Some(id), Some(caminho_url)) = (n.id, caminho_url)
                && caminho_url.starts_with(DETALHE)
                && !nome.is_empty()
            {
                let link = format!("{}{caminho_url}", am::BASE);
                if vistas.insert(link.clone()) {
                    let caminho = match topo {
                        Some(t) => vec![t.to_string(), nome.clone()],
                        None => vec![nome.clone()],
                    };
                    out.push(Pagina {
                        id,
                        nome: nome.clone(),
                        caminho,
                        link,
                    });
                }
            }
            walk(&n.children, Some(topo.unwrap_or(&nome)), vistas, out);
        }
    }
    let mut out = Vec::new();
    walk(nodes, None, &mut HashSet::new(), &mut out);
    out
}

/// O HTML da seção de perguntas do detalhe, ou `None` quando a página não a mostra. A seção oculta pode trazer
/// marcação vazia no campo (a 1143, em 02/11/2026): o que vale é `visibleSections`. O campo é o HTML inline ou
/// a referência (`"$c"`) a um bloco de texto do mesmo flight.
fn secao(flight: &str) -> Result<Option<String>> {
    let visiveis: Vec<String> = valor_unico(flight, "visibleSections")?;
    if !visiveis.iter().any(|s| s == SECAO) {
        return Ok(None);
    }
    let campo: Option<String> = valor_unico(flight, CAMPO)?;
    let campo = campo.ok_or_else(|| anyhow!("a seção está visível e o campo {CAMPO} é null"))?;
    let html = match campo.strip_prefix('$') {
        None => campo,
        Some(r) if !r.is_empty() && r.bytes().all(|b| b.is_ascii_hexdigit()) => {
            bloco_de_texto(flight, r)
                .ok_or_else(|| anyhow!("a referência ${r} não leva a um bloco de texto do flight"))?
                .to_string()
        }
        Some(_) => bail!("valor RSC {campo:?} no campo {CAMPO} — referência de tipo desconhecido"),
    };
    if html_para_texto(&html, am::BASE).is_empty() {
        bail!("a seção está visível e o HTML não tem texto");
    }
    Ok(Some(html))
}

/// O valor JSON de `"chave":` no flight — uma vez só em cada detalhe (02/11/2026); mais de uma é markup novo.
fn valor_unico<T: DeserializeOwned>(flight: &str, chave: &str) -> Result<T> {
    let agulha = format!("\"{chave}\":");
    let mut achados = flight.match_indices(&agulha);
    let (Some((i, _)), None) = (achados.next(), achados.next()) else {
        bail!("o flight não traz {agulha} exatamente uma vez — o markup RSC mudou?");
    };
    serde_json::Deserializer::from_str(&flight[i + agulha.len()..])
        .into_iter::<T>()
        .next()
        .ok_or_else(|| anyhow!("{agulha} sem valor no flight"))?
        .with_context(|| format!("o valor de {agulha} no flight"))
}

/// O texto do bloco `<id>:T<tamanho em hex>,<texto>` do flight. O tamanho é em bytes, e o bloco não termina em
/// quebra de linha: o seguinte vem colado (`…</p>b:T6a1,<p>…`). Por isso o flight é lido de linha em linha desde
/// o começo — buscar `c:T` direto acharia também o `1c:T`.
fn bloco_de_texto<'a>(flight: &'a str, id: &str) -> Option<&'a str> {
    let mut resto = flight;
    while !resto.is_empty() {
        let (linha, depois) = resto.split_once(':')?;
        if let Some(t) = depois.strip_prefix('T') {
            let (hex, corpo) = t.split_once(',')?;
            let n = usize::from_str_radix(hex, 16).ok()?;
            let texto = corpo.get(..n)?;
            if linha == id {
                return Some(texto);
            }
            resto = &corpo[n..];
        } else {
            resto = depois.split_once('\n').map_or("", |(_, r)| r);
        }
    }
    None
}

struct Pergunta {
    /// O id da página (as cópias ficam na de menor id).
    id: i64,
    pergunta: String,
    resposta: String,
    caminho: Vec<String>,
    link: String,
}

static NUMERO_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\d+)\s*[).\-–]").unwrap());
static PREFIXO_RESPOSTA_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:Resposta\s*:|R\s*[.:])\s*").unwrap());

/// `1)`, `1.`, `1 -`, `1 –` no começo da linha → o número e o resto. `2.1)` não: é subtítulo dentro da resposta
/// (REDAR).
fn numero(linha: &str) -> Option<(u32, &str)> {
    let c = NUMERO_RE.captures(linha)?;
    let resto = &linha[c.get(0)?.end()..];
    if resto.starts_with(|ch: char| ch.is_ascii_digit()) {
        return None;
    }
    Some((c[1].parse().ok()?, resto.trim_start()))
}

/// Os pares de uma página (estudo §3). As linhas são as do texto da seção: parágrafo, item, linha de tabela e
/// `<br>` quebram. Abre pergunta a linha numerada quando ao menos metade das numeradas tem "?" (o REDAR tem 2
/// tópicos sem "?" em 5; um passo a passo numerado sem "?" fica texto); sem isso, a linha com "?". A pergunta
/// vai até o primeiro "?", e o que sobra na linha abre a resposta (o Refis cola as duas). Sem nenhuma
/// pergunta, a seção vira uma só, com o nome da página (D-AMFAQ-4).
fn parear(texto: &str, pagina: &Pagina) -> Result<Vec<Pergunta>> {
    let linhas: Vec<&str> = texto.lines().collect();
    let numeradas: Vec<(u32, &str)> = linhas.iter().filter_map(|l| numero(l)).collect();
    let com_interrogacao = numeradas.iter().filter(|(_, l)| l.contains('?')).count();
    let por_numero = !numeradas.is_empty() && 2 * com_interrogacao >= numeradas.len();
    let abre = |l: &'_ str| -> Option<String> {
        if por_numero {
            numero(l).map(|(_, resto)| resto.to_string())
        } else {
            l.contains('?').then(|| l.to_string())
        }
    };
    if !linhas.iter().any(|l| abre(l).is_some()) {
        return Ok(vec![pergunta_de(
            pagina,
            format!("Dúvidas frequentes — {}", pagina.nome),
            texto.to_string(),
        )?]);
    }
    if por_numero {
        let ordem: Vec<u32> = numeradas.iter().map(|(n, _)| *n).collect();
        if ordem.windows(2).any(|w| w[1] != w[0] + 1) {
            eprintln!(
                "⚠️  AM FAQs: numeração fora de ordem em {} — {ordem:?}",
                pagina.link
            );
        }
    }
    let mut pares: Vec<(String, Vec<String>)> = Vec::new();
    let mut antes = 0;
    for l in &linhas {
        if let Some(texto) = abre(l) {
            let (pergunta, resto) = match texto.find('?') {
                Some(i) => (&texto[..=i], texto[i + 1..].trim()),
                None => (texto.as_str(), ""),
            };
            let resposta = if resto.is_empty() {
                Vec::new()
            } else {
                vec![resto.to_string()]
            };
            pares.push((pergunta.to_string(), resposta));
        } else if let Some((_, resposta)) = pares.last_mut() {
            resposta.push(l.to_string());
        } else {
            antes += 1;
        }
    }
    if antes > 0 {
        eprintln!(
            "⚠️  AM FAQs: {antes} linha(s) antes da primeira pergunta em {} — descartadas",
            pagina.link
        );
    }
    pares
        .into_iter()
        .map(|(pergunta, resposta)| pergunta_de(pagina, pergunta, resposta.join("\n")))
        .collect()
}

fn pergunta_de(pagina: &Pagina, pergunta: String, resposta: String) -> Result<Pergunta> {
    let pergunta = clean(&pergunta);
    let resposta = PREFIXO_RESPOSTA_RE.replace(&resposta, "").into_owned();
    if resposta.is_empty() {
        bail!("a pergunta {pergunta:?} não tem resposta");
    }
    Ok(Pergunta {
        id: pagina.id,
        pergunta,
        resposta,
        caminho: pagina.caminho.clone(),
        link: pagina.link.clone(),
    })
}

/// Par idêntico em páginas diferentes (o Refis em 5, parcelamentos, REDAR, certidões — D-AMFAQ-5) fica só na de
/// menor id; a ordem é a do portfólio.
fn sem_copias(perguntas: Vec<Pergunta>) -> Vec<Pergunta> {
    let chave = |p: &Pergunta| (p.pergunta.clone(), p.resposta.clone());
    let mut menor: HashMap<(String, String), i64> = HashMap::new();
    for p in &perguntas {
        let id = menor.entry(chave(p)).or_insert(p.id);
        *id = (*id).min(p.id);
    }
    let mut vistas = HashSet::new();
    perguntas
        .into_iter()
        .filter(|p| menor[&chave(p)] == p.id && vistas.insert(chave(p)))
        .collect()
}

/// O nome do `.md` sai de link + pergunta: a mesma pergunta duas vezes na página, com respostas diferentes,
/// faria um sobrescrever o outro.
fn sem_colisao(perguntas: &[Pergunta]) -> Result<()> {
    let mut vistas = HashSet::new();
    for p in perguntas {
        if !vistas.insert((p.link.as_str(), p.pergunta.as_str())) {
            bail!(
                "{} traz {:?} duas vezes, com respostas diferentes",
                p.link,
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
            origin: p.caminho.join(" | "),
            url: p.link.clone(),
        })
        .collect()
}

/// Um nível da árvore, com os filhos na ordem em que aparecem (a do portfólio).
#[derive(Default)]
struct Ramo<'a> {
    filhos: Vec<(&'a str, Ramo<'a>)>,
    perguntas: Vec<&'a Pergunta>,
}

/// A árvore da aba de FAQs: raiz → categorias de topo → páginas. A interface começa pelos filhos da raiz. O nó
/// de uma página aponta para o detalhe; a categoria sem perguntas próprias, para a listagem.
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
        url: am::LISTA_URL.to_string(),
        page_type: PageType::Menu,
        origin: String::new(),
        children: raiz.filhos.iter().map(|(t, r)| no(t, t, r)).collect(),
        faq_items: Vec::new(),
    }
}

fn no(titulo: &str, trilha: &str, ramo: &Ramo) -> FaqNode {
    FaqNode {
        title: titulo.to_string(),
        url: ramo
            .perguntas
            .first()
            .map_or(am::LISTA_URL.to_string(), |p| p.link.clone()),
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
            .map(|(t, r)| no(t, &format!("{trilha} | {t}"), r))
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

/// HTML → texto pelo conversor da frota (`auli_scraper_kit::html`, D-KITHTML-1): links da raiz com o host do
/// `am::BASE`.
fn html_para_texto(s: &str, base: &str) -> String {
    auli_scraper_kit::html::html_para_texto(
        s,
        auli_scraper_kit::html::Opcoes {
            base: Some(base),
            quebras_do_fonte: false,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // Recortes da coleta de 02/11/2026. Flights: o 882 (a seção é a referência `$c`, com os três blocos de texto
    // colados), o 1143 (seção oculta, com marcação vazia no campo) e o 2923 (o HTML inline no campo), do primeiro
    // bloco de texto até a linha do `serviceDetails`. HTML da seção: 502 (pergunta e resposta no mesmo `<p>`,
    // com `<br>`), 4302 (Refis: `N.` e a resposta colada no "?"), 1202 (REDAR: tópicos sem "?" e subtítulos
    // `2.1)`), 3162 (`<div>` aninhado e "Resposta:"), 1082 (17 perguntas, com `<span><strong>`), 446 (uma
    // pergunta sem número) e 302 (texto corrido, sem pergunta).
    const D882: &str = include_str!("../tests/fixtures/am-faqs-d882-recorte.rsc");
    const D1143: &str = include_str!("../tests/fixtures/am-faqs-d1143-recorte.rsc");
    const D2923: &str = include_str!("../tests/fixtures/am-faqs-d2923-recorte.rsc");
    const D502: &str = include_str!("../tests/fixtures/am-faqs-d502.html");
    const D4302: &str = include_str!("../tests/fixtures/am-faqs-d4302.html");
    const D1202: &str = include_str!("../tests/fixtures/am-faqs-d1202.html");
    const D3162: &str = include_str!("../tests/fixtures/am-faqs-d3162.html");
    const D1082: &str = include_str!("../tests/fixtures/am-faqs-d1082.html");
    const D446: &str = include_str!("../tests/fixtures/am-faqs-d446.html");
    const D302: &str = include_str!("../tests/fixtures/am-faqs-d302.html");

    fn pagina(id: i64, nome: &str) -> Pagina {
        Pagina {
            id,
            nome: nome.to_string(),
            caminho: vec!["Categoria".to_string(), nome.to_string()],
            link: format!("{}{DETALHE}{id}", am::BASE),
        }
    }

    fn pares(html: &str) -> Vec<Pergunta> {
        parear(&html_para_texto(html, am::BASE), &pagina(1, "Serviço")).unwrap()
    }

    fn pares_de(texto: &str) -> Vec<(String, String)> {
        parear(texto, &pagina(1, "Serviço"))
            .unwrap()
            .into_iter()
            .map(|p| (p.pergunta, p.resposta))
            .collect()
    }

    #[test]
    fn referencia_leva_ao_bloco_de_texto_colado_aos_outros() {
        let html = secao(D882).unwrap().unwrap();
        assert!(
            html.starts_with("<p>1) O que &eacute; REDESIM?</p>"),
            "{html:.80}"
        );
        assert_eq!(html.len(), 0x6a1);
        let perguntas = pares(&html);
        assert_eq!(perguntas[0].pergunta, "O que é REDESIM?");
        assert!(
            perguntas[0]
                .resposta
                .starts_with("A Rede Nacional para a Simplificação")
        );
    }

    #[test]
    fn campo_inline_e_o_proprio_html() {
        let html = secao(D2923).unwrap().unwrap();
        let perguntas = pares(&html);
        assert_eq!(perguntas.len(), 1);
        assert_eq!(perguntas[0].resposta, "Não.");
    }

    #[test]
    fn secao_oculta_nao_conta_mesmo_com_marcacao_no_campo() {
        assert!(D1143.contains(r#""perguntasRespostasHtml":"$e""#));
        assert!(secao(D1143).unwrap().is_none());
    }

    #[test]
    fn secao_visivel_sem_texto_null_ou_repetida_e_erro() {
        let visivel = r#"2:["$","x",null,{"visibleSections":["RESUMO","PERGUNTAS_FREQUENTES"],"perguntasRespostasHtml":"#;
        assert!(secao(&format!("{visivel}\"<p>&nbsp;</p>\"}}]\n")).is_err());
        assert!(secao(&format!("{visivel}null}}]\n")).is_err());
        assert!(
            secao(&format!("{visivel}\"$9\"}}]\n")).is_err(),
            "referência sem bloco"
        );
        let dobrado = format!("{visivel}\"<p>a?</p><p>b</p>\"}}]\n{visivel}\"<p>c</p>\"}}]\n");
        assert!(secao(&dobrado).is_err());
    }

    #[test]
    fn bloco_de_texto_le_bytes_e_nao_confunde_1c_com_c() {
        // `1c` vem antes de `c`, e o tamanho de `c` é em bytes: "ç" vale 2, e os 8 caracteres são 9 bytes.
        let flight = "1:I[\"x\"]\n1c:T3,abcc:T9,<p>ç</p>2:[\"$c\"]\n";
        assert_eq!(bloco_de_texto(flight, "c"), Some("<p>ç</p>"));
        assert_eq!(bloco_de_texto(flight, "1c"), Some("abc"));
        assert_eq!(bloco_de_texto(flight, "d"), None);
    }

    #[test]
    fn numero_reconhece_as_marcas_e_nao_o_subtitulo() {
        for (linha, esperado) in [
            ("1) O que é?", Some((1, "O que é?"))),
            ("2. Quais?", Some((2, "Quais?"))),
            ("3 - Quem?", Some((3, "Quem?"))),
            ("4 – Onde?", Some((4, "Onde?"))),
            ("10)Quando?", Some((10, "Quando?"))),
            ("2.1) Pagamento em duplicidade", None),
            ("1.000 reais", None),
            ("10/04/2025", None),
            ("2 a 10 parcelas: 90%;", None),
        ] {
            assert_eq!(numero(linha), esperado, "{linha:?}");
        }
    }

    #[test]
    fn contagem_exata_por_formato() {
        for (html, n) in [
            (D502, 9),
            (D4302, 10),
            (D1202, 5),
            (D3162, 5),
            (D1082, 17),
            (D446, 1),
            (D302, 1),
        ] {
            let perguntas = pares(html);
            assert_eq!(perguntas.len(), n);
            assert!(perguntas.iter().all(|p| !p.resposta.is_empty()));
        }
    }

    #[test]
    fn pergunta_e_resposta_no_mesmo_paragrafo_separadas_por_br() {
        let perguntas = pares(D502);
        assert_eq!(perguntas[0].pergunta, "Quando posso parcelar meu IPVA?");
        assert_eq!(
            perguntas[0].resposta,
            "Após o vencimento, desde que os débitos sejam anteriores ao ano atual."
        );
    }

    #[test]
    fn br_quebra_a_linha_dentro_da_resposta() {
        let perguntas = pares("<p>1) A?</p><p>Juros;<br>multa.</p>");
        assert_eq!(perguntas[0].resposta, "Juros;\nmulta.");
    }

    #[test]
    fn texto_depois_do_ponto_de_interrogacao_abre_a_resposta() {
        // O Refis separa com `<br data-start=…>`; o mesmo texto sem a quebra dá a mesma pergunta.
        let perguntas = pares(D4302);
        assert_eq!(
            perguntas[0].pergunta,
            "Quais débitos podem ser incluídos no Refis 2025?"
        );
        assert!(
            perguntas[0]
                .resposta
                .starts_with("Podem aderir ao programa")
        );
        assert_eq!(
            pares_de("1) A? Sim, até 60 parcelas.\n2) B?\nNão."),
            [
                ("A?".to_string(), "Sim, até 60 parcelas.".to_string()),
                ("B?".to_string(), "Não.".to_string())
            ]
        );
    }

    #[test]
    fn topico_sem_interrogacao_e_pergunta_e_subtitulo_numerado_fica_na_resposta() {
        let perguntas = pares(D1202);
        assert_eq!(
            perguntas[1].pergunta,
            "Situações que podem ser objeto de REDAR"
        );
        assert!(perguntas[1].resposta.contains("\n2.1) "));
    }

    #[test]
    fn prefixo_da_resposta_sai_so_no_comeco() {
        let perguntas = pares(D3162);
        assert!(
            perguntas[0]
                .resposta
                .starts_with("A Retificação de Pagamento (REDAR)")
        );
        for (texto, resposta) in [
            ("1) A?\nResposta: Sim.", "Sim."),
            ("1) A?\nR. A Rede", "A Rede"),
            ("1) A?\nRespostas são enviadas", "Respostas são enviadas"),
            ("1) A?\nR$ 10,00", "R$ 10,00"),
        ] {
            assert_eq!(pares_de(texto)[0].1, resposta, "{texto:?}");
        }
    }

    #[test]
    fn sem_numero_a_linha_com_interrogacao_abre_e_sem_nenhuma_vira_uma_so() {
        let perguntas = pares(D446);
        assert!(
            perguntas[0]
                .pergunta
                .starts_with("Quais empresas que NÃO podem")
        );
        assert!(
            perguntas[0]
                .resposta
                .starts_with("Empresas optantes do Simples")
        );
        let corrido = pares(D302);
        assert_eq!(corrido[0].pergunta, "Dúvidas frequentes — Serviço");
        assert!(
            corrido[0]
                .resposta
                .starts_with("COMO SOLICITAR REGIME ESPECIAL")
        );
    }

    #[test]
    fn passo_a_passo_numerado_sem_interrogacao_nao_vira_pergunta() {
        let texto = "Como pedir o bloqueio?\n1 - Registre o B.O.;\n2 - Fale com o banco;\n3 - Peça o bloqueio.";
        let pares = pares_de(texto);
        assert_eq!(pares.len(), 1);
        assert_eq!(pares[0].0, "Como pedir o bloqueio?");
        assert!(pares[0].1.ends_with("3 - Peça o bloqueio."));
    }

    #[test]
    fn pergunta_sem_resposta_e_erro() {
        assert!(parear("1) A?\n2) B?\nSim.", &pagina(1, "Serviço")).is_err());
    }

    #[test]
    fn copias_ficam_na_pagina_de_menor_id_na_ordem_do_portfolio() {
        let p = |id, pergunta: &str, resposta: &str| Pergunta {
            id,
            pergunta: pergunta.to_string(),
            resposta: resposta.to_string(),
            caminho: Vec::new(),
            link: format!("l{id}"),
        };
        let perguntas = sem_copias(vec![
            p(4324, "Refis?", "Sim."),
            p(4302, "Outra?", "Não."),
            p(4302, "Refis?", "Sim."),
            p(4325, "Refis?", "Sim, com outra resposta."),
        ]);
        let ids: Vec<(i64, &str)> = perguntas
            .iter()
            .map(|p| (p.id, p.pergunta.as_str()))
            .collect();
        assert_eq!(ids, [(4302, "Outra?"), (4302, "Refis?"), (4325, "Refis?")]);
    }

    #[test]
    fn mesma_pergunta_duas_vezes_na_pagina_e_erro() {
        let perguntas = parear("1) A?\nSim.\n2) A?\nNão.", &pagina(1, "Serviço")).unwrap();
        assert!(sem_colisao(&sem_copias(perguntas)).is_err());
    }

    #[test]
    fn paginas_da_arvore_com_categoria_de_topo_sem_query_e_sem_externos() {
        let json = r#"[{"id":302,"name":"Isenções","url":"/portfolio-servicos/detalhes/302?profile=todos","children":[
            {"id":446,"name":"Redução de alíquota","url":"/portfolio-servicos/detalhes/446?profile=todos","children":[]},
            {"id":9,"name":"Sub","url":null,"children":[
                {"id":882,"name":"Inscrição","url":"/portfolio-servicos/detalhes/882?gerarTaxa=true&profile=todos","children":[]},
                {"id":882,"name":"Inscrição","url":"/portfolio-servicos/detalhes/882?profile=todos","children":[]}]},
            {"id":7,"name":"Sistema externo","url":"https://online.sefaz.am.gov.br/x","children":[]}]}]"#;
        let nodes: Vec<am::Node> = serde_json::from_str(json).unwrap();
        let paginas = paginas(&nodes);
        let vistas: Vec<(&str, &[String])> = paginas
            .iter()
            .map(|p| (p.link.as_str(), p.caminho.as_slice()))
            .collect();
        assert_eq!(
            vistas,
            [
                (
                    "https://www.sefaz.am.gov.br/portfolio-servicos/detalhes/302",
                    &["Isenções".to_string()][..]
                ),
                (
                    "https://www.sefaz.am.gov.br/portfolio-servicos/detalhes/446",
                    &["Isenções".to_string(), "Redução de alíquota".to_string()][..]
                ),
                (
                    "https://www.sefaz.am.gov.br/portfolio-servicos/detalhes/882",
                    &["Isenções".to_string(), "Inscrição".to_string()][..]
                ),
            ]
        );
    }

    #[test]
    fn arvore_poe_a_pergunta_da_categoria_no_proprio_no_e_aponta_para_o_detalhe() {
        let p = |caminho: &[&str], link: &str| Pergunta {
            id: 1,
            pergunta: "P?".to_string(),
            resposta: "R".to_string(),
            caminho: caminho.iter().map(|s| s.to_string()).collect(),
            link: link.to_string(),
        };
        let perguntas = [
            p(&["Isenções"], "d302"),
            p(&["Isenções", "Redução"], "d446"),
            p(&["IPVA", "Parcelamento"], "d502"),
        ];
        let raiz = montar_arvore(&perguntas);
        let topo: Vec<&str> = raiz.children.iter().map(|n| n.title.as_str()).collect();
        assert_eq!(topo, ["Isenções", "IPVA"]);
        let isencoes = &raiz.children[0];
        assert!(matches!(isencoes.page_type, PageType::Menu));
        assert_eq!(
            (isencoes.url.as_str(), isencoes.faq_items.len()),
            ("d302", 1)
        );
        assert_eq!(isencoes.children[0].origin, "Isenções | Redução");
        assert_eq!(raiz.children[1].url, am::LISTA_URL);
        assert_eq!(raiz.children[1].children[0].url, "d502");
    }

    #[test]
    fn links_da_resposta_entram_no_texto() {
        let html = r#"<p>Veja a <a href="https://www.sefaz.am.gov.br/cartilha.pdf">cartilha</a> e <a href="/portfolio-servicos/todos">o portfólio</a>; escreva para <a href="mailto:a@sefaz.am.gov.br">a@sefaz.am.gov.br</a>.</p>"#;
        assert_eq!(
            html_para_texto(html, am::BASE),
            "Veja a cartilha \"https://www.sefaz.am.gov.br/cartilha.pdf\" e o portfólio \
             \"https://www.sefaz.am.gov.br/portfolio-servicos/todos\"; escreva para a@sefaz.am.gov.br \"mailto:a@sefaz.am.gov.br\"."
        );
    }
}
