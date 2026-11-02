// SEFAZ-SP FAQs — as "Perguntas Frequentes" do portal, uma lista SharePoint por subsite de serviço,
// lidas pela mesma `_api` anônima do `scrape.rs`. Estudo, medições e decisões: docs/ESTUDO-SP-Faqs.md
// (D-SPFAQ-*).
//
// A página central `/Paginas/perguntas-frequentes.aspx` é um plugin JS: lista os subsites pela `Homes 360`
// e, para o escolhido, lê `<URLHome>/_api/web/lists/getByTitle('Perguntas Frequentes')/items`. O scraper
// faz o mesmo para todos. Três propriedades da fonte moldam o código:
//   - `URLHome` vem de quatro jeitos (relativo com barra, sem barra, ausente, absoluto para outro
//     domínio) → `caminho_do_home` normaliza o que o plugin consegue usar e descarta o resto;
//   - `Resposta` é HTML do editor SharePoint, com entidades numéricas (`&#58;`), espaço cru, `<style>`,
//     `<svg>`, tabelas e `href`/`src` relativos → `html_para_texto`;
//   - o `Id` é por lista, e a pergunta pode ter vários `Assunto` → identidade (subsite, Id) e o link do
//     botão "copiar link" do portal (`link_da_pergunta`), que traz os dois.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::LazyLock;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use regex::Regex;
use serde::Deserialize;
use ureq::Agent;

use auli_contract::FaqRaw;
use auli_scraper_kit::clean;
use auli_scraper_kit::faqtree::{FaqItem, FaqNode, PageType};
use auli_scraper_kit::http::GetOpts;

use crate::pareceres::{COURTESY, UA, decode_html};
use crate::scrape::{Facet, Resp};

const BASE: &str = "https://portal.fazenda.sp.gov.br";

/// A chamada do plugin (`ObterLinksPesquisaServico`), só com as colunas usadas.
fn url_dos_homes() -> String {
    format!(
        "{BASE}/servicos/_api/web/lists/getByTitle('Homes%20360')/items?$select=Id,Title,URLHome&$orderby=Title%20asc&$top=200"
    )
}

/// A chamada do plugin (`ObterLinksPesquisa`) para um subsite. `caminho` tem barra no fim.
fn url_da_lista(caminho: &str) -> String {
    format!(
        "{BASE}{caminho}_api/web/lists/getByTitle('Perguntas%20Frequentes')/items?$select=Id,Title,Resposta,Assunto&$orderby=Title&$top=1000"
    )
}

/// Coleta as FAQs e grava a árvore do frontend (`<id>-faqs-tree.json`) e o snapshot. As guardas
/// (D-SPFAQ-4) rodam antes de qualquer gravação: rodada que falha deixa os arquivos anteriores intactos.
pub fn run(data_dir: &str, use_cache: bool) -> Result<()> {
    let agent = auli_scraper_kit::build_agent(UA, Some(Duration::from_secs(60)));

    let homes: Vec<Home> = buscar_todas(data_dir, &agent, &url_dos_homes(), use_cache)?;
    println!("SP FAQs: {} Homes na `Homes 360`", homes.len());

    let mut subsites = Vec::new();
    let mut sem_url = 0;
    for h in homes {
        let titulo = clean(&h.title);
        let Some(caminho) = caminho_do_home(h.url_home.as_deref()) else {
            match h.url_home.as_deref().map(str::trim) {
                Some(u) if !u.is_empty() => eprintln!(
                    "⚠️  SP FAQs: {titulo}: URLHome fora do portal ({u}) — o plugin também não o lê; pulado"
                ),
                _ => sem_url += 1,
            }
            continue;
        };
        let itens: Vec<Item> = buscar_todas(data_dir, &agent, &url_da_lista(&caminho), use_cache)?;
        let s = montar_subsite(titulo, caminho, itens)?;
        println!("SP FAQs: {}: {} perguntas", s.titulo, s.perguntas.len());
        subsites.push(s);
    }
    println!(
        "SP FAQs: {} subsites lidos, {} Homes sem URLHome, {} listas vazias",
        subsites.len(),
        sem_url,
        subsites.iter().filter(|s| s.perguntas.is_empty()).count()
    );
    let faqs = consolidar(&subsites)?;

    // Atômico pelo mesmo motivo do RS, do SC e do PR: a árvore não tem outra fonte (o snapshot achatado
    // não a reconstrói) e é o único artefato de coleta que o frontend lê direto.
    let tree_path = format!("{data_dir}/{}-faqs-tree.json", crate::ENTITY);
    auli_contract::arquivo::escrever_atomico(
        Path::new(&tree_path),
        &serde_json::to_string_pretty(&montar_arvore(&subsites))?,
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

/// Todas as páginas de uma lista, seguindo `d.__next` (nenhuma lista passa de 1.000 em 01/11/2026, mas
/// o `$top` não é garantia).
fn buscar_todas<T: for<'de> Deserialize<'de>>(
    data_dir: &str,
    agent: &Agent,
    url: &str,
    use_cache: bool,
) -> Result<Vec<T>> {
    let mut out = Vec::new();
    let mut proxima = Some(url.to_string());
    while let Some(u) = proxima {
        let corpo = buscar(data_dir, agent, &u, use_cache)?;
        let r: Resp<T> =
            serde_json::from_str(&corpo).map_err(|e| anyhow!("JSON inválido de {u}: {e}"))?;
        out.extend(r.d.results);
        proxima = r.d.next;
    }
    Ok(out)
}

/// Uma requisição (D-SPFAQ-8, a lição da D-SCFAQ-4): rede sempre, gravando o cache sob a url; com
/// `--usecache`, só o cache, e miss é erro. Não é o `fetch` do `scrape.rs`, que é cache-first: um cache
/// velho reproduziria a coleta antiga em silêncio.
fn buscar(data_dir: &str, agent: &Agent, url: &str, use_cache: bool) -> Result<String> {
    if use_cache {
        return auli_scraper_kit::cache::read(data_dir, "faqs", url)
            .ok_or_else(|| anyhow!("cache miss para {url} (modo --usecache, sem rede)"));
    }
    sleep(COURTESY);
    let corpo = auli_scraper_kit::http::get_string(
        agent,
        url,
        &GetOpts {
            log_prefix: "SP FAQs",
            accept: Some("application/json;odata=verbose"),
            ..Default::default()
        },
    )?;
    auli_scraper_kit::cache::write(data_dir, "faqs", url, &corpo);
    Ok(corpo)
}

// --- Shapes da API (só as colunas usadas) ---

/// Um item da `Homes 360`: um subsite de serviço. O `Assunto` dela é a categoria do Home, não o da FAQ.
#[derive(Deserialize)]
struct Home {
    #[serde(rename = "Title", default)]
    title: String,
    #[serde(rename = "URLHome", default)]
    url_home: Option<String>,
}

/// Um item da lista `Perguntas Frequentes` de um subsite. `Assunto` é MultiChoice.
#[derive(Deserialize)]
struct Item {
    #[serde(rename = "Id")]
    id: i64,
    #[serde(rename = "Title", default)]
    title: Option<String>,
    #[serde(rename = "Resposta", default)]
    resposta: Option<String>,
    #[serde(rename = "Assunto", default)]
    assunto: Facet,
}

/// Um subsite com a sua lista, já em texto. `caminho` é o `URLHome` normalizado (barra no fim).
struct Subsite {
    titulo: String,
    caminho: String,
    perguntas: Vec<Pergunta>,
}

struct Pergunta {
    id: i64,
    pergunta: String,
    resposta: String,
    /// Os `Assunto` da pergunta, na ordem da fonte (14% das perguntas têm mais de um em 01/11/2026).
    assuntos: Vec<String>,
}

/// O caminho do subsite, com barra no fim, como o plugin o concatena (`/servicos/isencao-icms-veiculos`
/// vem sem a barra, e o botão do próprio portal monta um link quebrado com ele). `None` para o que o
/// plugin não consegue usar: sem `URLHome` (3 em 01/11/2026) ou absoluto, para outro domínio (1).
fn caminho_do_home(url_home: Option<&str>) -> Option<String> {
    let u = url_home?.trim();
    if !u.starts_with('/') {
        return None;
    }
    Some(if u.ends_with('/') {
        u.to_string()
    } else {
        format!("{u}/")
    })
}

/// Uma lista em texto, com as guardas (c) e (d) da D-SPFAQ-4, que são da lista.
fn montar_subsite(titulo: String, caminho: String, itens: Vec<Item>) -> Result<Subsite> {
    // (c) O `Id` é a chave do item na lista do SharePoint: repetido, a paginação duplicou algo.
    let mut vistos = BTreeSet::new();
    let repetidos: Vec<String> = itens
        .iter()
        .filter(|i| !vistos.insert(i.id))
        .map(|i| i.id.to_string())
        .collect();
    if !repetidos.is_empty() {
        bail!(
            "{titulo}: Id repetido na lista — {} (a paginação duplicou?)",
            repetidos.join(", ")
        );
    }
    let perguntas: Vec<Pergunta> = itens
        .into_iter()
        .map(|i| Pergunta {
            id: i.id,
            // `Title` é texto puro (a "(<IM>)" do SAT é texto): só o `clean`, sem tirar tag.
            pergunta: clean(i.title.as_deref().unwrap_or("")),
            resposta: html_para_texto(i.resposta.as_deref().unwrap_or("")),
            assuntos: i
                .assunto
                .results
                .iter()
                .map(|a| clean(a))
                .filter(|a| !a.is_empty())
                .collect(),
        })
        .collect();
    // (d) Pergunta sem texto vira `.md` sem título e vetor sem conteúdo (a D-SCFAQ-3f).
    let sem_texto: Vec<String> = perguntas
        .iter()
        .filter(|p| p.pergunta.is_empty())
        .map(|p| p.id.to_string())
        .collect();
    if !sem_texto.is_empty() {
        bail!(
            "{titulo}: {} pergunta(s) sem texto — {}",
            sem_texto.len(),
            sem_texto.join(", ")
        );
    }
    Ok(Subsite {
        titulo,
        caminho,
        perguntas,
    })
}

/// O acervo, com as guardas que olham todos os subsites: (a) nenhum subsite lido e (b) zero perguntas
/// no total são `Err`; resposta vazia só avisa.
fn consolidar(subsites: &[Subsite]) -> Result<Vec<FaqRaw>> {
    // (a) Sem subsite com `URLHome` não há o que coletar: é a `Homes 360` que mudou, não o portal vazio.
    if subsites.is_empty() {
        bail!("nenhum Home da `Homes 360` com URLHome do portal — a lista mudou?");
    }
    let faqs: Vec<FaqRaw> = subsites
        .iter()
        .flat_map(|s| s.perguntas.iter().map(move |p| montar_faq_raw(s, p)))
        .collect();
    // (b) Zero no total é a coleta que caiu, não o portal vazio.
    if faqs.is_empty() {
        bail!(
            "nenhuma pergunta nos {} subsites — a coleta caiu (o portal não está vazio)",
            subsites.len()
        );
    }
    // Resposta vazia só avisa: título e link ainda são documento (0 em 01/11/2026).
    let sem_resposta: Vec<String> = subsites
        .iter()
        .flat_map(|s| {
            s.perguntas
                .iter()
                .filter(|p| p.resposta.is_empty())
                .map(move |p| format!("{} #{}", s.caminho, p.id))
        })
        .collect();
    if !sem_resposta.is_empty() {
        eprintln!(
            "⚠️  SP FAQs: {} pergunta(s) com resposta vazia — {}",
            sem_resposta.len(),
            sem_resposta.join(", ")
        );
    }
    println!(
        "SP FAQs: {} perguntas em {} subsites",
        faqs.len(),
        subsites.len()
    );
    Ok(faqs)
}

static OCULTO_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?is)<style\b.*?</style>|<script\b.*?</script>|<svg\b.*?</svg>|<!--.*?-->")
        .unwrap()
});
static ESPACO_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());
static IMG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?is)<img\b[^>]*?\ssrc\s*=\s*["']([^"']*)["'][^>]*>"#).unwrap());
static LINK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)<a\b[^>]*?\shref\s*=\s*["']([^"']*)["'][^>]*>(.*?)</a>"#).unwrap()
});
static BLOCO_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)</?(?:p|div|br|hr|li|ul|ol|tr|table|tbody|thead|h[1-6]|blockquote)\b[^>]*>")
        .unwrap()
});
static CELULA_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)</?t[dh]\b[^>]*>").unwrap());
static TAG_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]+>").unwrap());

/// HTML do editor SharePoint → texto, na convenção do SC e do PR: `<a href="url">texto</a>` vira
/// `texto "url"`, `<img src>` vira `"url"`, tag de bloco vira quebra de linha. Diferenças medidas nas
/// 2.665 respostas de 01/11/2026:
///   - o editor escapa o `<` do texto (`&lt;escrituras&gt;`), então TODA tag sai — e as entidades só são
///     decodificadas depois, para o `&lt;` não virar tag;
///   - o HTML traz quebras de linha cruas, que no navegador são espaço: o espaço é colapsado antes de
///     os blocos virarem linha, senão a frase se parte no meio;
///   - `<style>`, `<script>`, `<svg>` e comentários saem com o conteúdo;
///   - `href` e `src` vêm com entidades (`https&#58;//`) e às vezes relativos (`/servicos/…`);
///   - célula de tabela vira espaço e linha de tabela vira linha: "CST_ICMS 53". Não `|`: as respostas
///     citam registros SPED (`|C197|SP90090278|…`).
fn html_para_texto(s: &str) -> String {
    let visivel = OCULTO_RE.replace_all(s, "");
    let corrido = ESPACO_RE.replace_all(&visivel, " ");
    let com_imagens = IMG_RE.replace_all(&corrido, |c: &regex::Captures| {
        format!("\"{}\"", absoluta(&decode_html(c[1].trim())))
    });
    let com_links = LINK_RE.replace_all(&com_imagens, |c: &regex::Captures| {
        let href = decode_html(c[1].trim());
        let texto = clean(&decode_html(&TAG_RE.replace_all(&c[2], "")));
        if href.is_empty() || href.starts_with('#') || href.starts_with("javascript:") {
            texto
        } else if texto.is_empty() {
            format!("\"{}\"", absoluta(&href))
        } else {
            format!("{texto} \"{}\"", absoluta(&href))
        }
    });
    let com_quebras = BLOCO_RE.replace_all(&com_links, "\n");
    let com_celulas = CELULA_RE.replace_all(&com_quebras, " ");
    let sem_tags = TAG_RE.replace_all(&com_celulas, "");
    decode_html(&sem_tags)
        .lines()
        .map(clean)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// `/servicos/…` → `https://portal.fazenda.sp.gov.br/servicos/…`; o resto como veio.
fn absoluta(url: &str) -> String {
    if url.starts_with('/') {
        format!("{BASE}{url}")
    } else {
        url.to_string()
    }
}

/// A página de FAQs do subsite, onde o portal abre a lista daquele Home.
fn pagina_do_subsite(s: &Subsite) -> String {
    format!("{BASE}{}Paginas/perguntas-frequentes.aspx", s.caminho)
}

/// O que o botão "copiar link" do portal copia (`TemplatePerguntasFrequentesGeral.html`):
/// `webAbsoluteUrl + URLHome + 'Paginas/perguntas-frequentes.aspx#ID=' + Id` (D-SPFAQ-2). O `#ID=`
/// separa as perguntas de mesmo texto no mesmo subsite (32 em 01/11/2026) — sem ele, o nome do `.md`
/// colidiria.
fn link_da_pergunta(s: &Subsite, p: &Pergunta) -> String {
    format!("{}#ID={}", pagina_do_subsite(s), p.id)
}

/// `<Home> | <assunto>, <assunto>` (D-SPFAQ-3); sem assunto, só o Home.
fn trilha(s: &Subsite, p: &Pergunta) -> String {
    if p.assuntos.is_empty() {
        s.titulo.clone()
    } else {
        format!("{} | {}", s.titulo, p.assuntos.join(", "))
    }
}

fn montar_faq_raw(s: &Subsite, p: &Pergunta) -> FaqRaw {
    FaqRaw {
        pergunta: p.pergunta.clone(),
        resposta: p.resposta.clone(),
        origin: trilha(s, p),
        url: link_da_pergunta(s, p),
    }
}

/// A árvore da aba de FAQs: raiz → um `Menu` por subsite (ordem da `Homes 360`) → um `Faq` por assunto
/// (ordem da primeira aparição na lista). A pergunta com vários assuntos aparece em cada um, como o
/// portal a mostra; sem assunto, vai num "Geral". Subsite sem pergunta fica de fora.
fn montar_arvore(subsites: &[Subsite]) -> FaqNode {
    let children = subsites
        .iter()
        .filter(|s| !s.perguntas.is_empty())
        .map(|s| {
            let mut assuntos: Vec<(String, Vec<FaqItem>)> = Vec::new();
            for p in &s.perguntas {
                let nomes = if p.assuntos.is_empty() {
                    vec!["Geral".to_string()]
                } else {
                    p.assuntos.clone()
                };
                for nome in nomes {
                    let item = FaqItem {
                        pergunta: p.pergunta.clone(),
                        resposta: p.resposta.clone(),
                        url: Some(link_da_pergunta(s, p)),
                    };
                    match assuntos.iter_mut().find(|(n, _)| *n == nome) {
                        Some((_, itens)) => itens.push(item),
                        None => assuntos.push((nome, vec![item])),
                    }
                }
            }
            FaqNode {
                title: s.titulo.clone(),
                url: pagina_do_subsite(s),
                page_type: PageType::Menu,
                origin: String::new(),
                children: assuntos
                    .into_iter()
                    .map(|(nome, faq_items)| FaqNode {
                        origin: format!("{} | {nome}", s.titulo),
                        title: nome,
                        url: pagina_do_subsite(s),
                        page_type: PageType::Faq,
                        children: Vec::new(),
                        faq_items,
                    })
                    .collect(),
                faq_items: Vec::new(),
            }
        })
        .collect();

    FaqNode {
        title: "Perguntas frequentes".to_string(),
        url: format!("{BASE}/Paginas/perguntas-frequentes.aspx"),
        page_type: PageType::Menu,
        origin: String::new(),
        children,
        faq_items: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Recortes do JSON real da `_api` (01/11/2026), no shape verbose. Os Homes cobrem os quatro jeitos
    // de `URLHome` (GIA normal, Pauta do Café sem, Recursos Humanos absoluto em outro domínio, Isenção
    // ICMS - Veículos sem a barra final). A lista da GIA tem uma pergunta de cada caso difícil:
    // dois assuntos (82), imagem relativa (64), lista e registro SPED (71), href relativo (80), tabela (98).
    const HOMES: &str = include_str!("../tests/fixtures/sp-faqs-homes.json");
    const LISTA_GIA: &str = include_str!("../tests/fixtures/sp-faqs-lista-gia.json");

    fn homes() -> Vec<Home> {
        serde_json::from_str::<Resp<Home>>(HOMES).unwrap().d.results
    }

    fn itens_gia() -> Vec<Item> {
        serde_json::from_str::<Resp<Item>>(LISTA_GIA)
            .unwrap()
            .d
            .results
    }

    fn gia() -> Subsite {
        montar_subsite(
            "GIA - Guia de Informação e Apuração do ICMS".to_string(),
            "/servicos/gia/".to_string(),
            itens_gia(),
        )
        .unwrap()
    }

    fn erro<T>(r: Result<T>) -> String {
        match r {
            Ok(_) => panic!("esperava Err"),
            Err(e) => e.to_string(),
        }
    }

    fn resposta(id: i64) -> String {
        gia()
            .perguntas
            .into_iter()
            .find(|p| p.id == id)
            .unwrap()
            .resposta
    }

    #[test]
    fn caminho_do_home_normaliza_a_barra_e_descarta_o_que_o_plugin_nao_le() {
        let caminhos: Vec<Option<String>> = homes()
            .iter()
            .map(|h| caminho_do_home(h.url_home.as_deref()))
            .collect();
        assert_eq!(
            caminhos,
            vec![
                Some("/servicos/gia/".to_string()),
                None, // Pauta do Café: sem URLHome
                None, // Recursos Humanos: https://www.sfp.sp.gov.br/…
                Some("/servicos/isencao-icms-veiculos/".to_string()),
            ]
        );
        // O Title vem com espaço sobrando ("GIA - … do ICMS "): o `clean` do `run` tira.
        assert_eq!(
            clean(&homes()[0].title),
            "GIA - Guia de Informação e Apuração do ICMS"
        );
    }

    #[test]
    fn faq_raw_tem_trilha_com_todos_os_assuntos_e_o_link_do_portal() {
        let s = gia();
        let faqs = consolidar(std::slice::from_ref(&s)).unwrap();
        assert_eq!(faqs.len(), 5);
        let multi = &faqs[0];
        assert!(
            multi
                .pergunta
                .starts_with("\"Erro ao migrar o banco\" ao abrir")
        );
        assert_eq!(
            multi.origin,
            "GIA - Guia de Informação e Apuração do ICMS | Erros, Uso do Programa"
        );
        assert_eq!(
            multi.url,
            "https://portal.fazenda.sp.gov.br/servicos/gia/Paginas/perguntas-frequentes.aspx#ID=82"
        );
    }

    #[test]
    fn id_repetido_na_lista_e_erro() {
        let mut itens = itens_gia();
        itens[1].id = itens[0].id;
        let e = erro(montar_subsite(
            "GIA".to_string(),
            "/servicos/gia/".to_string(),
            itens,
        ));
        assert!(e.contains("GIA: Id repetido na lista — 82"), "{e}");
    }

    #[test]
    fn pergunta_sem_texto_e_erro() {
        // Só o zero-width que o editor põe no começo do texto: o `clean` esvazia.
        let mut itens = itens_gia();
        itens[2].title = Some("\u{200b} ".to_string());
        let id = itens[2].id;
        let e = erro(montar_subsite(
            "GIA".to_string(),
            "/servicos/gia/".to_string(),
            itens,
        ));
        assert!(
            e.contains(&format!("GIA: 1 pergunta(s) sem texto — {id}")),
            "{e}"
        );
    }

    #[test]
    fn nenhum_subsite_e_zero_perguntas_sao_erro() {
        assert!(erro(consolidar(&[])).contains("nenhum Home"));
        let vazio =
            montar_subsite("GIA".to_string(), "/servicos/gia/".to_string(), vec![]).unwrap();
        assert!(erro(consolidar(&[vazio])).contains("nenhuma pergunta nos 1 subsites"));
    }

    #[test]
    fn resposta_vazia_so_avisa() {
        let mut s = gia();
        s.perguntas[0].resposta.clear();
        assert_eq!(consolidar(&[s]).unwrap().len(), 5);
    }

    #[test]
    fn titulo_e_texto_puro_e_nao_perde_o_que_parece_tag() {
        // A pergunta 530 do SAT: "(<IM>)" é texto, não tag.
        let mut itens = itens_gia();
        itens[0].title = Some(
            "Preenchimento do campo relativo à informação de Inscrição Municipal (<IM>)"
                .to_string(),
        );
        let s = montar_subsite("GIA".to_string(), "/servicos/gia/".to_string(), itens).unwrap();
        assert!(
            s.perguntas[0]
                .pergunta
                .ends_with("Inscrição Municipal (<IM>)")
        );
    }

    #[test]
    fn entidades_numericas_e_zero_width_saem() {
        // 82: `&quot;`, `&#160;`, `&#58;` e o U+200B do editor.
        assert_eq!(
            resposta(82),
            "Se, ao abrir o programa GIA, é exibida uma mensagem do tipo \"Erro ao migrar o banco\" ou no \
             rodapé do programa existe mensagem indicando que o banco de dados está inoperante, é \
             necessário instalar o Microsoft SQL Server Compact 4.0 SP1 disponível em \
             https://www.microsoft.com/pt-br/download/details.aspx?id=30709."
        );
    }

    #[test]
    fn imagem_relativa_vira_url_absoluta_e_quebra_crua_e_espaco() {
        // 64: `<img>` relativo, e o HTML tem "\n      " entre o `<p>` e o `<img>`.
        assert_eq!(
            resposta(64),
            "Quando um contribuinte adquire ou vende uma mercadoria na condição de substituído \
             tributário, ele deve escriturar registros C197 conforme exemplo abaixo:\n\
             \"https://portal.fazenda.sp.gov.br/servicos/gia/PublishingImages/4-Substituído%201.JPG\"\n\
             \"https://portal.fazenda.sp.gov.br/servicos/gia/PublishingImages/4-Substituído%202.JPG\""
        );
    }

    #[test]
    fn lista_vira_linhas_e_o_registro_sped_fica_intacto() {
        // 71: `<ul><li>` e as linhas `|C195|…|` do leiaute SPED.
        assert_eq!(
            resposta(71),
            "Devem ser lançados dois ajustes:\n\
             C197 com o ajuste SP90090278 informando o valor do ICMS Substituído no campo VL_ICMS\n\
             C197 com o ajuste SP90090104 informando no campo VL_OUTROS o valor da coluna Outras \
             (Valor total da nota – IPI – valor do ICMS Substituído)\n\
             Exemplo:\n\
             |C195|000001||\n\
             |C197|SP90090278|CFOP||||valor do ICMS Substituído||\n\
             |C195|000002||\n\
             |C197|SP90090104|CFOP|||1|0|Valor total da nota – IPI – valor do ICMS Substituído|"
        );
    }

    #[test]
    fn href_relativo_vira_url_absoluta() {
        // 80: `<a href="/servicos/gia/Paginas/…">`.
        assert_eq!(
            resposta(80),
            "Para saber quando ocorrem e como corrigir as principais divergências e inconsistências, \
             acesse a página Eliminação da GIA - Principais Divergências e Inconsistências. \
             \"https://portal.fazenda.sp.gov.br/servicos/gia/Paginas/GIAEFD-divergencias-inconsistencias.aspx\""
        );
    }

    #[test]
    fn tabela_junta_as_celulas_da_linha() {
        // 98: tabela de dados do editor (`ms-rteTable-default`), com href `https&#58;//`.
        let r = resposta(98);
        assert!(
            r.contains("\nC190 (SPED)\nCST_ICMS 53\nCFOP 5652\nALIQ_ICMS 1,0635\nVL_OPR 100\n"),
            "{r}"
        );
        assert!(
            r.contains("Nota Orientativa 01/2023 v 1.4 - ICMS Monofásico \"http://sped.rfb.gov.br/arquivo/show/7265\""),
            "{r}"
        );
    }

    #[test]
    fn menor_que_escapado_fica_como_texto() {
        // FAQ 29 dos Cartórios: `&lt;escrituras&gt;` é texto — decodificar depois de tirar as tags.
        let r = r#"<div class="ExternalClassC72A011E98BD4371B4189F3566E771B9"><p>Devido a alterações meramente técnicas no Schema (.XSD) de validação do arquivo XML utilizado para transmissão em lote, está publicado e atualizado na subpágina de <a href="/servicos/cartorios/Paginas/Downloads.aspx">&quot;Downloads</a>&quot;, clicando em &quot;Transmissão em lote&quot;. </p><p>Sendo o caso, corrija logo no início de seu XML, na tag &lt;escrituras&gt; o atributo xsi&#58;noNamespaceSchemaLocation, do seguinte modo&#58;</p></div>"#;
        assert_eq!(
            html_para_texto(r),
            "Devido a alterações meramente técnicas no Schema (.XSD) de validação do arquivo XML \
             utilizado para transmissão em lote, está publicado e atualizado na subpágina de \
             \"Downloads \"https://portal.fazenda.sp.gov.br/servicos/cartorios/Paginas/Downloads.aspx\"\", \
             clicando em \"Transmissão em lote\".\n\
             Sendo o caso, corrija logo no início de seu XML, na tag <escrituras> o atributo \
             xsi:noNamespaceSchemaLocation, do seguinte modo:"
        );
    }

    #[test]
    fn style_svg_e_comentario_saem_com_o_conteudo() {
        // FAQ 12 do Nos Conformes (`<style>` com CSS) e 6 da Carteira Predial (`<svg>` no link).
        let r = "<div><style>\r\n.x{color:red}</style>  \n<div>\n   <p>O período de avaliação\n   vai de maio de 2018.</p><!-- nota --><p>Veja <strong>aqui<span aria-hidden=\"true\"><svg xmlns=\"http://www.w3.org/2000/svg\" width=\"20\" height=\"20\"><title>ícone</title></svg></span></strong>.</p></div></div>";
        assert_eq!(
            html_para_texto(r),
            "O período de avaliação vai de maio de 2018.\nVeja aqui."
        );
    }

    #[test]
    fn arvore_tem_subsite_assunto_e_a_pergunta_multi_assunto_em_cada_um() {
        let t = montar_arvore(&[gia()]);
        assert_eq!(t.title, "Perguntas frequentes");
        assert_eq!(
            t.url,
            "https://portal.fazenda.sp.gov.br/Paginas/perguntas-frequentes.aspx"
        );
        let g = &t.children[0];
        assert_eq!(g.page_type, PageType::Menu);
        assert_eq!(
            g.url,
            "https://portal.fazenda.sp.gov.br/servicos/gia/Paginas/perguntas-frequentes.aspx"
        );
        let assuntos: Vec<(&str, usize)> = g
            .children
            .iter()
            .map(|n| (n.title.as_str(), n.faq_items.len()))
            .collect();
        assert_eq!(
            assuntos,
            vec![("Erros", 1), ("Uso do Programa", 1), ("GIA da EFD", 4)]
        );
        let erros = &g.children[0];
        assert_eq!(erros.page_type, PageType::Faq);
        assert_eq!(
            erros.origin,
            "GIA - Guia de Informação e Apuração do ICMS | Erros"
        );
        // A 82 está em "Erros" e em "Uso do Programa", com o mesmo link.
        assert_eq!(g.children[1].faq_items[0].url, erros.faq_items[0].url);
        assert_eq!(
            erros.faq_items[0].url.as_deref(),
            Some(
                "https://portal.fazenda.sp.gov.br/servicos/gia/Paginas/perguntas-frequentes.aspx#ID=82"
            )
        );
        // O fio que o `parseFaqs.ts` lê: folha sem `children`, page_type como string.
        let json = serde_json::to_value(erros).unwrap();
        assert_eq!(json["page_type"], "Faq");
        assert!(json.get("children").is_none());
    }
}
