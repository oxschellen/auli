// SEF-MG FAQs — as "Perguntas Frequentes" do Portal SPED MG (`portalsped.fazenda.mg.gov.br/spedmg`),
// OpenCms com HTML estático, à parte do CSM ServiceNow do `mg.rs`. Estudo, medições e decisões:
// docs/ESTUDO-MG-Faqs.md (D-MGFAQ-*).
//
// Cada módulo do SPED (NF-e, NFC-e, CT-e…) tem um menu lateral; quando ele tem "Perguntas…", a página é
// um índice — seções em romano, perguntas numeradas e sub-tópicos sem número — que linka cada pergunta
// para `respostas_<romano>/index.html#<âncora>`. Três propriedades da fonte moldam o código:
//   - seis módulos têm a entrada no menu, mas só NF-e e NFC-e têm o FAQ em HTML no SPED; CT-e, CT-e OS,
//     EFD e MDF-e apontam para fora (portal nacional, PDF) → `classificar`;
//   - a página de respostas é exportada do Word e muda de seção para seção: âncora `name` (NF-e), `id`
//     (NFC-e), nenhuma (NF-e VIII e IX, e a 1ª da I), pergunta e resposta no mesmo `<p>` (VIII) → a
//     pergunta é localizada pela âncora que o índice anuncia e, sem ela, pelo número (`parse_respostas`);
//   - o índice é a autoridade da estrutura: dele vêm a lista de perguntas, a seção e o sub-tópico, que a
//     página de respostas repete (com "• ") ou omite.
//
// A 2ª fonte, a base "SRE - Dúvidas Frequentes" do CSM, é o `csm.rs`; aqui ela só entra no acervo e na
// árvore, depois dos módulos do SPED.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::LazyLock;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use regex::Regex;
use ureq::Agent;

use auli_contract::FaqRaw;
use auli_scraper_kit::faqtree::{FaqItem, FaqNode, PageType};
use auli_scraper_kit::http::GetOpts;
use auli_scraper_kit::{clean, decode_entities};

const BASE: &str = "https://portalsped.fazenda.mg.gov.br";
const HOME: &str = "https://portalsped.fazenda.mg.gov.br/spedmg/";

/// A identidade das FAQs do SC, do PR e do SP (D-MGFAQ-8). O `mg.rs` segue com o UA do kit: é outro host.
const UA: &str =
    "AuliBot/0.1 (+https://github.com/oxschellen/auli; carlos.schellenberger@gmail.com)";
const COURTESY: Duration = Duration::from_millis(1000);

/// Coleta as FAQs e grava a árvore do frontend (`<id>-faqs-tree.json`) e o snapshot. As guardas
/// (D-MGFAQ-5) rodam antes de qualquer gravação: rodada que falha deixa os arquivos anteriores intactos.
pub fn run(data_dir: &str, use_cache: bool) -> Result<()> {
    let agent = auli_scraper_kit::build_agent(UA, Some(Duration::from_secs(60)));

    let modulos = parse_home(&buscar(data_dir, &agent, HOME, use_cache)?)?;
    println!("MG FAQs: {} módulos na home do SPED", modulos.len());

    let mut com_faq = Vec::new();
    for m in modulos {
        let pagina = buscar(data_dir, &agent, &absoluta(&m.caminho), use_cache)?;
        let Some(caminho) = link_das_faqs(&pagina) else {
            println!("MG FAQs: {}: sem perguntas no menu", m.nome);
            continue;
        };
        let indice = absoluta(&caminho);
        let entradas = match classificar(&indice, &buscar(data_dir, &agent, &indice, use_cache)?)? {
            Faq::Html(entradas) => entradas,
            Faq::Fora(links) => {
                eprintln!(
                    "⚠️  MG FAQs: {}: as perguntas do menu ({indice}) não estão no SPED — apontam para {} — fora (D-MGFAQ-4)",
                    m.nome,
                    links.join(", ")
                );
                continue;
            }
        };
        let mut perguntas = Vec::new();
        for da_pagina in entradas.chunk_by(|a, b| a.pagina == b.pagina) {
            let html = buscar(data_dir, &agent, &absoluta(&da_pagina[0].pagina), use_cache)?;
            perguntas.extend(parse_respostas(&html, da_pagina)?);
        }
        println!(
            "MG FAQs: {}: {} perguntas em {} seções",
            m.nome,
            perguntas.len(),
            perguntas
                .chunk_by(|a, b| a.entrada.secao == b.entrada.secao)
                .count()
        );
        com_faq.push(ModuloFaq {
            nome: m.nome,
            indice,
            perguntas,
        });
    }
    let mut faqs = consolidar(&com_faq)?;
    let csm = crate::csm::coletar(data_dir, use_cache)?;
    faqs.extend(crate::csm::faqs(&csm));
    println!("MG FAQs: {} perguntas no total", faqs.len());

    // Atômico pelo mesmo motivo do RS, do SC, do PR e do SP: a árvore não tem outra fonte (o snapshot
    // achatado não a reconstrói) e é o único artefato de coleta que o frontend lê direto.
    let mut arvore = montar_arvore(&com_faq);
    arvore.children.push(crate::csm::arvore(&csm));
    let tree_path = format!("{data_dir}/{}-faqs-tree.json", crate::ENTITY);
    auli_contract::arquivo::escrever_atomico(
        Path::new(&tree_path),
        &serde_json::to_string_pretty(&arvore)?,
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

/// Uma requisição (D-MGFAQ-10, a lição da D-SCFAQ-4): rede sempre, gravando o cache sob a url; com
/// `--usecache`, só o cache, e miss é erro. Não é o `fetch_page` do `mg.rs`, que é cache-first: um cache
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
            log_prefix: "MG FAQs",
            ..Default::default()
        },
    )?;
    auli_scraper_kit::cache::write(data_dir, "faqs", url, &corpo);
    Ok(corpo)
}

/// Um módulo da barra da home. `nome` junta a sigla e o `title` do link: "NF-e - Nota Fiscal Eletrônica".
struct Modulo {
    caminho: String,
    nome: String,
}

/// Uma pergunta do índice, com o que a página de respostas precisa para localizá-la.
#[derive(Clone)]
struct Entrada {
    secao: String,
    subtopico: Option<String>,
    /// O `N.` da linha do índice. É o que localiza a pergunta na página que não tem a âncora.
    numero: Option<u32>,
    /// Caminho da página de respostas, sem a âncora.
    pagina: String,
    /// Por página, não por módulo: `#1` existe na I e na II do NF-e; a NFC-e usa `#a1`, `#b1`…
    ancora: String,
    /// Como o índice escreve — só para conferir a página, que tem a versão revisada (D-MGFAQ-12).
    texto: String,
}

/// Uma pergunta coletada: o texto da página de respostas, sem o `N.`, e a resposta já em texto.
struct Pergunta {
    entrada: Entrada,
    pergunta: String,
    resposta: String,
}

struct ModuloFaq {
    nome: String,
    indice: String,
    perguntas: Vec<Pergunta>,
}

static MODULO_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"<a class="menu menu-\d+" href="(/spedmg/[^"/]+/)" title="([^"]*)">([^<]*)</a>"#)
        .unwrap()
});

/// Os módulos, na ordem da barra (D-MGFAQ-4): a fonte de verdade, nunca uma lista no código.
fn parse_home(html: &str) -> Result<Vec<Modulo>> {
    let modulos: Vec<Modulo> = MODULO_RE
        .captures_iter(html)
        .map(|c| Modulo {
            caminho: c[1].to_string(),
            nome: format!(
                "{} - {}",
                clean(&decode_entities(&c[3])),
                clean(&decode_entities(&c[2]))
            ),
        })
        .collect();
    if modulos.is_empty() {
        bail!("a home do SPED não lista nenhum módulo — a página mudou?");
    }
    Ok(modulos)
}

const MENU_LATERAL: &str = r#"<div class="col-xs-12 col-sm-3"#;
static CONTEUDO_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"<div\s+id="conteudo"\s*>"#).unwrap());
static LINK_DAS_FAQS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"<a href="([^"]+)"[^>]*>\s*Perguntas\b[^<]*</a>"#).unwrap());

/// O miolo da página, entre o `#conteudo` e o menu lateral — sem o cabeçalho, o modal de aviso e o rodapé.
fn conteudo(html: &str) -> Result<&str> {
    let inicio = CONTEUDO_RE
        .find(html)
        .ok_or_else(|| anyhow!("página sem `#conteudo` — o template do SPED mudou?"))?
        .end();
    let fim = html[inicio..]
        .find(MENU_LATERAL)
        .ok_or_else(|| anyhow!("página sem o menu lateral — o template do SPED mudou?"))?;
    Ok(&html[inicio..inicio + fim])
}

/// O "Perguntas e Respostas" do menu lateral do módulo. Só no menu: o conteúdo pode citar perguntas.
fn link_das_faqs(html: &str) -> Option<String> {
    let (_, menu) = html.split_once(MENU_LATERAL)?;
    LINK_DAS_FAQS_RE.captures(menu).map(|c| c[1].to_string())
}

/// O que a entrada "Perguntas…" do menu leva a ver.
enum Faq {
    /// Um índice de perguntas do próprio SPED (NF-e, NFC-e).
    Html(Vec<Entrada>),
    /// Só links para fora do SPED ou para arquivos (CT-e, CT-e OS e MDF-e → portais nacionais; EFD → PDF).
    Fora(Vec<String>),
}

static HREF_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?i)href\s*=\s*["']([^"']+)["']"#).unwrap());
static ARQUIVO_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\.(?:pdf|docx?|xlsx?|zip)$").unwrap());

/// Índice, ou página que aponta para fora. Uma página que linka outras páginas do SPED e nenhuma de
/// respostas é `Err`: é o índice do NF-e com markup novo, e pular em silêncio perderia o módulo inteiro.
fn classificar(url: &str, html: &str) -> Result<Faq> {
    let entradas = parse_indice(html)?;
    if !entradas.is_empty() {
        return Ok(Faq::Html(entradas));
    }
    let links: Vec<String> = HREF_RE
        .captures_iter(conteudo(html)?)
        .map(|c| decode_entities(&c[1]))
        .collect();
    let internas: Vec<&String> = links
        .iter()
        .filter(|l| (l.starts_with('/') || l.starts_with(BASE)) && !ARQUIVO_RE.is_match(l))
        .collect();
    if !internas.is_empty() {
        bail!(
            "{url}: sem link de `respostas_…`, mas com páginas do próprio SPED ({}) — o markup do índice mudou?",
            internas
                .iter()
                .map(|l| l.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    Ok(Faq::Fora(links))
}

static TABELA_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)<table\b.*?</table>").unwrap());
static LINHA_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?is)<tr\b.*?</tr>").unwrap());
static CELULA_TEXTO_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)<td\b[^>]*>(.*?)</td>").unwrap());
/// Um link de pergunta do índice: a página de respostas, a âncora e o texto.
static PERGUNTA_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r##"(?is)href="([^"#]*/respostas_[ivxlc]+/index\.html)#([^"]+)"[^>]*>(.*?)</a>"##)
        .unwrap()
});
static ROTULO_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\d+)\s*[.)]?$").unwrap());

/// O índice: o título de seção vem antes da tabela dela, em `<p><strong>`; na tabela, a linha com link é
/// pergunta (o número numa célula, o texto no link) e a linha sem link é sub-tópico, que vale até o
/// próximo — no NF-e, os 8 da seção III. Aplica as guardas do índice (D-MGFAQ-5).
fn parse_indice(html: &str) -> Result<Vec<Entrada>> {
    let c = conteudo(html)?;
    let mut entradas: Vec<Entrada> = Vec::new();
    let mut secao: Option<String> = None;
    let mut fim_da_anterior = 0;
    for tabela in TABELA_RE.find_iter(c) {
        // Sem texto entre duas tabelas, a seção segue a mesma.
        if let Some(titulo) = html_para_texto(&c[fim_da_anterior..tabela.start()], BASE)
            .lines()
            .last()
        {
            secao = Some(titulo.to_string());
        }
        fim_da_anterior = tabela.end();
        let mut subtopico = None;
        for linha in LINHA_RE.find_iter(tabela.as_str()) {
            let Some(p) = PERGUNTA_RE.captures(linha.as_str()) else {
                let texto = uma_linha(linha.as_str());
                if !texto.is_empty() {
                    subtopico = Some(texto);
                }
                continue;
            };
            let secao = secao.clone().ok_or_else(|| {
                anyhow!("índice: pergunta antes de qualquer título de seção — o markup mudou?")
            })?;
            let numero = CELULA_TEXTO_RE
                .captures_iter(linha.as_str())
                .find_map(|td| {
                    ROTULO_RE
                        .captures(&uma_linha(&td[1]))
                        .map(|n| n[1].to_string())
                })
                .map(|n| n.parse())
                .transpose()?;
            entradas.push(Entrada {
                secao,
                subtopico: subtopico.clone(),
                numero,
                pagina: p[1].to_string(),
                ancora: decode_entities(&p[2]),
                texto: uma_linha(&p[3]),
            });
        }
    }

    // (a) Link de pergunta fora de linha de tabela escaparia do parse sem erro: a contagem crua bate.
    let no_html = PERGUNTA_RE.find_iter(c).count();
    if no_html != entradas.len() {
        bail!(
            "índice: {no_html} links de pergunta no HTML, {} parseados — o parse perdeu algum",
            entradas.len()
        );
    }
    // (b) A âncora é por página: a mesma duas vezes na mesma página são duas perguntas num link só.
    let mut vistas = BTreeSet::new();
    for e in &entradas {
        if !vistas.insert((e.pagina.as_str(), e.ancora.as_str())) {
            bail!("índice: {}#{} aparece duas vezes", e.pagina, e.ancora);
        }
    }
    Ok(entradas)
}

/// Âncora de pergunta na página de respostas: `<a name="8">` no NF-e, `<a id="a1">` na NFC-e.
static ANCORA_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)<a\b[^>]*?\s(?:name|id)\s*=\s*["']([^"']+)["'][^>]*>"#).unwrap()
});
/// A âncora vira esta marca antes da conversão para texto, que tira as tags: assim a linha da pergunta
/// continua achável. Caracteres de uso privado, que o portal não usa.
static MARCA_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new("\u{E000}[^\u{E001}]*\u{E001}").unwrap());
static NUMERO_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\d+)\s*[.)]\s*").unwrap());

fn marca(ancora: &str) -> String {
    format!("\u{E000}{ancora}\u{E001}")
}

/// Uma página de respostas, com as perguntas que o índice aponta para ela (na ordem do índice). Cada
/// pergunta é a linha da sua âncora ou — na página que não a tem — a primeira linha, depois da pergunta
/// anterior, que começa com o seu `N.`; a resposta vai até a próxima pergunta.
fn parse_respostas(html: &str, entradas: &[Entrada]) -> Result<Vec<Pergunta>> {
    let ancoras: BTreeSet<&str> = entradas.iter().map(|e| e.ancora.as_str()).collect();
    // As âncoras que o índice não anuncia (`emissao`, os `OLE_LINK` do Word) saem com as outras tags.
    let marcado = ANCORA_RE.replace_all(conteudo(html)?, |c: &regex::Captures| {
        if ancoras.contains(&c[1]) {
            marca(&c[1])
        } else {
            c[0].to_string()
        }
    });
    let texto = html_para_texto(&marcado, BASE);
    let linhas: Vec<&str> = texto.lines().collect();
    let sem_marca: Vec<String> = linhas
        .iter()
        .map(|l| clean(&MARCA_RE.replace_all(l, "")))
        .collect();

    let mut posicoes = Vec::with_capacity(entradas.len());
    let mut cursor = 0;
    for e in entradas {
        let pela_ancora = linhas[cursor..]
            .iter()
            .position(|l| l.contains(&marca(&e.ancora)));
        let pelo_numero = || {
            let n = e.numero?.to_string();
            sem_marca[cursor..]
                .iter()
                .position(|l| NUMERO_RE.captures(l).is_some_and(|c| c[1] == n))
        };
        let k = cursor
            + pela_ancora.or_else(pelo_numero).ok_or_else(|| {
                anyhow!(
                    "{}#{}: a pergunta do índice não está na página — «{}»",
                    e.pagina,
                    e.ancora,
                    e.texto
                )
            })?;
        posicoes.push(k);
        cursor = k + 1;
    }

    let subtopicos: BTreeSet<&str> = entradas
        .iter()
        .filter_map(|e| e.subtopico.as_deref())
        .collect();
    let mut perguntas = Vec::with_capacity(entradas.len());
    for (i, e) in entradas.iter().enumerate() {
        let k = posicoes[i];
        let pergunta = NUMERO_RE.replace(&sem_marca[k], "").to_string();
        // (c) A linha achada tem de ser a pergunta que o índice anuncia. As duas versões diferem em 9 de
        //     149 (a página corrige o "ás" e o "IEmpresa" do índice), mas a menor sobreposição medida em
        //     01/11/2026 é 0,91; uma linha errada — um item numerado de resposta — fica perto de zero.
        if sobreposicao(&e.texto, &pergunta) < 0.5 {
            bail!(
                "{}#{}: a linha achada não é a pergunta do índice — índice «{}», página «{}»",
                e.pagina,
                e.ancora,
                e.texto,
                pergunta
            );
        }
        let fim = posicoes.get(i + 1).copied().unwrap_or(sem_marca.len());
        let mut resposta: Vec<&str> = sem_marca[k + 1..fim].iter().map(String::as_str).collect();
        // O título do próximo sub-tópico, que a página repete antes da pergunta (com "• "), não é resposta.
        while resposta
            .last()
            .is_some_and(|l| subtopicos.contains(l.strip_prefix('•').map_or(*l, str::trim_start)))
        {
            resposta.pop();
        }
        perguntas.push(Pergunta {
            entrada: e.clone(),
            pergunta,
            resposta: resposta.join("\n"),
        });
    }
    Ok(perguntas)
}

static PALAVRA_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\w+").unwrap());

/// Fração das palavras do texto do índice que a linha da página também tem.
fn sobreposicao(indice: &str, pagina: &str) -> f64 {
    let palavras = |s: &str| -> BTreeSet<String> {
        PALAVRA_RE
            .find_iter(&s.to_lowercase())
            .map(|m| m.as_str().to_string())
            .collect()
    };
    let do_indice = palavras(indice);
    if do_indice.is_empty() {
        return 0.0;
    }
    do_indice.intersection(&palavras(pagina)).count() as f64 / do_indice.len() as f64
}

/// Todas as perguntas, na ordem da barra de módulos e do índice, com o módulo de cada uma.
fn perguntas(modulos: &[ModuloFaq]) -> impl Iterator<Item = (&ModuloFaq, &Pergunta)> {
    modulos
        .iter()
        .flat_map(|m| m.perguntas.iter().map(move |p| (m, p)))
}

/// O acervo, com as guardas que olham todos os módulos (D-MGFAQ-5).
fn consolidar(modulos: &[ModuloFaq]) -> Result<Vec<FaqRaw>> {
    // (d) Sem pergunta não há o que coletar: é o SPED que mudou, não as FAQs que acabaram.
    let faqs: Vec<FaqRaw> = perguntas(modulos)
        .map(|(m, p)| montar_faq_raw(m, p))
        .collect();
    if faqs.is_empty() {
        bail!("nenhuma pergunta em HTML em nenhum módulo do SPED — a home ou os menus mudaram?");
    }

    // (e) Resposta vazia só avisa: título e link ainda são documento. 0 em 01/11/2026.
    let sem_resposta: Vec<String> = perguntas(modulos)
        .filter(|(_, p)| p.resposta.is_empty())
        .map(|(_, p)| link_da_pergunta(&p.entrada))
        .collect();
    if !sem_resposta.is_empty() {
        eprintln!(
            "⚠️  MG FAQs: {} pergunta(s) com resposta vazia depois da conversão — {}",
            sem_resposta.len(),
            sem_resposta.join(", ")
        );
    }

    println!(
        "MG FAQs: {} perguntas em {} módulos",
        faqs.len(),
        modulos.len()
    );
    Ok(faqs)
}

static LINK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)<a\b[^>]*?\shref\s*=\s*["']([^"']*)["'][^>]*>(.*?)</a>"#).unwrap()
});
static BLOCO_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)</?(?:br|div|h[1-6]|hr|li|ol|p|table|tbody|thead|tr|ul)\b[^>]*>").unwrap()
});
static CELULA_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)</?t[dh]\b[^>]*>").unwrap());
static TAG_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]+>").unwrap());
static NUMERICA_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"&#(\d+);").unwrap());

/// HTML → texto, na convenção da frota: `<a href="url">texto</a>` vira `texto "url"` (relativo ganha o
/// `base`), tag de bloco vira quebra de linha, célula vira espaço, linhas vazias saem. Toda tag sai, como
/// no SP (D-MGFAQ-6): nas 17 páginas de 01/11/2026 não há `<` cru (o `>` do texto vem como `&gt;`), e
/// as entidades só são decodificadas depois.
pub(crate) fn html_para_texto(s: &str, base: &str) -> String {
    let com_links = LINK_RE.replace_all(s, |c: &regex::Captures| {
        let href = decodificar(c[1].trim());
        let texto = uma_linha(&c[2]);
        if href.is_empty() || href.starts_with('#') || href.starts_with("javascript:") {
            texto
        } else if texto.is_empty() {
            format!("\"{}\"", absoluta_em(base, &href))
        } else {
            format!("{texto} \"{}\"", absoluta_em(base, &href))
        }
    });
    let com_quebras = BLOCO_RE.replace_all(&com_links, "\n");
    let com_celulas = CELULA_RE.replace_all(&com_quebras, " ");
    decodificar(&TAG_RE.replace_all(&com_celulas, ""))
        .lines()
        .map(clean)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Um trecho curto (texto de link, linha do índice) numa linha só, sem tag.
pub(crate) fn uma_linha(s: &str) -> String {
    clean(&decodificar(&TAG_RE.replace_all(s, "")))
}

/// As entidades do kit mais as numéricas decimais, que o editor do CSM emite (`&#34;`, `&#61;`, `&#43;`)
/// e o SPED não (0 nas 17 páginas). As numéricas vão antes: nenhum `&amp;#` nos 62 artigos de 02/11/2026.
fn decodificar(s: &str) -> String {
    let numericas = NUMERICA_RE.replace_all(s, |c: &regex::Captures| {
        c[1].parse()
            .ok()
            .and_then(char::from_u32)
            .map_or_else(|| c[0].to_string(), String::from)
    });
    decode_entities(&numericas)
}

fn absoluta(href: &str) -> String {
    absoluta_em(BASE, href)
}

fn absoluta_em(base: &str, href: &str) -> String {
    if href.starts_with('/') {
        format!("{base}{href}")
    } else {
        href.to_string()
    }
}

/// O link do índice, exato (D-MGFAQ-2). Nas 7 perguntas cuja página não tem a âncora, ele abre o topo
/// da página — como no portal.
fn link_da_pergunta(e: &Entrada) -> String {
    format!("{}#{}", absoluta(&e.pagina), e.ancora)
}

/// `<módulo> | <seção> | <sub-tópico>`, ou dois níveis sem sub-tópico (D-MGFAQ-3).
fn trilha(m: &ModuloFaq, e: &Entrada) -> String {
    match &e.subtopico {
        Some(s) => format!("{} | {} | {s}", m.nome, e.secao),
        None => format!("{} | {}", m.nome, e.secao),
    }
}

fn montar_faq_raw(m: &ModuloFaq, p: &Pergunta) -> FaqRaw {
    FaqRaw {
        pergunta: p.pergunta.clone(),
        resposta: p.resposta.clone(),
        origin: trilha(m, &p.entrada),
        url: link_da_pergunta(&p.entrada),
    }
}

fn item(p: &Pergunta) -> FaqItem {
    FaqItem {
        pergunta: p.pergunta.clone(),
        resposta: p.resposta.clone(),
        url: Some(link_da_pergunta(&p.entrada)),
    }
}

/// A árvore da aba de FAQs: raiz → um `Menu` por módulo → a seção, que é `Faq` quando não tem sub-tópico
/// e `Menu` com um `Faq` por sub-tópico quando tem. A pergunta de seção com sub-tópicos que vem antes do
/// primeiro (a III.1 do NF-e) fica nos `faq_items` do `Menu`, como no SC; a aba a lista depois dos filhos.
fn montar_arvore(modulos: &[ModuloFaq]) -> FaqNode {
    let children = modulos
        .iter()
        .map(|m| FaqNode {
            title: m.nome.clone(),
            url: m.indice.clone(),
            page_type: PageType::Menu,
            origin: String::new(),
            children: m
                .perguntas
                .chunk_by(|a, b| a.entrada.secao == b.entrada.secao)
                .map(|secao| montar_secao(m, secao))
                .collect(),
            faq_items: Vec::new(),
        })
        .collect();

    FaqNode {
        title: "Perguntas frequentes".to_string(),
        url: HOME.to_string(),
        page_type: PageType::Menu,
        origin: String::new(),
        children,
        faq_items: Vec::new(),
    }
}

fn montar_secao(m: &ModuloFaq, perguntas: &[Pergunta]) -> FaqNode {
    let e = &perguntas[0].entrada;
    let url = absoluta(&e.pagina);
    let com_sub: Vec<&Pergunta> = perguntas
        .iter()
        .filter(|p| p.entrada.subtopico.is_some())
        .collect();
    if com_sub.is_empty() {
        return FaqNode {
            title: e.secao.clone(),
            url,
            page_type: PageType::Faq,
            origin: trilha(m, e),
            children: Vec::new(),
            faq_items: perguntas.iter().map(item).collect(),
        };
    }
    FaqNode {
        title: e.secao.clone(),
        url: url.clone(),
        page_type: PageType::Menu,
        origin: String::new(),
        children: com_sub
            .chunk_by(|a, b| a.entrada.subtopico == b.entrada.subtopico)
            .map(|sub| FaqNode {
                title: sub[0].entrada.subtopico.clone().unwrap_or_default(),
                url: url.clone(),
                page_type: PageType::Faq,
                origin: trilha(m, &sub[0].entrada),
                children: Vec::new(),
                faq_items: sub.iter().map(|p| item(p)).collect(),
            })
            .collect(),
        faq_items: perguntas
            .iter()
            .filter(|p| p.entrada.subtopico.is_none())
            .map(item)
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Recortes do HTML real (01/11/2026): a barra de módulos da home; o índice do NF-e inteiro (do
    // `#conteudo` ao fim do menu lateral); as páginas de respostas III (âncora `name`, sub-tópicos com
    // "• ", a âncora `emissao` que não é pergunta) e VIII (sem âncora, pergunta e resposta no mesmo `<p>`)
    // do NF-e e a IV da NFC-e (âncora `id`); e a página de FAQ do CT-e, que aponta para o portal nacional.
    const HOME_HTML: &str = include_str!("../tests/fixtures/mg-faqs-home.html");
    const INDICE_NFE: &str = include_str!("../tests/fixtures/mg-faqs-indice-nfe.html");
    const RESPOSTAS_III: &str = include_str!("../tests/fixtures/mg-faqs-respostas-nfe-iii.html");
    const RESPOSTAS_VIII: &str = include_str!("../tests/fixtures/mg-faqs-respostas-nfe-viii.html");
    const RESPOSTAS_NFCE_IV: &str =
        include_str!("../tests/fixtures/mg-faqs-respostas-nfce-iv.html");
    const CTE_FAQ: &str = include_str!("../tests/fixtures/mg-faqs-cte-faq.html");

    const PAGINA_III: &str = "/spedmg/nfe/Perguntas-Frequentes/respostas_iii/index.html";
    const PAGINA_VIII: &str = "/spedmg/nfe/Perguntas-Frequentes/respostas_viii/index.html";

    fn erro<T>(r: Result<T>) -> String {
        match r {
            Ok(_) => panic!("esperava Err"),
            Err(e) => e.to_string(),
        }
    }

    fn da_pagina(pagina: &str) -> Vec<Entrada> {
        parse_indice(INDICE_NFE)
            .unwrap()
            .into_iter()
            .filter(|e| e.pagina == pagina)
            .collect()
    }

    fn nfe(perguntas: Vec<Pergunta>) -> ModuloFaq {
        ModuloFaq {
            nome: "NF-e - Nota Fiscal Eletrônica".to_string(),
            indice: "https://portalsped.fazenda.mg.gov.br/spedmg/nfe/Perguntas-Frequentes/"
                .to_string(),
            perguntas,
        }
    }

    fn iii_e_viii() -> ModuloFaq {
        let mut p = parse_respostas(RESPOSTAS_III, &da_pagina(PAGINA_III)).unwrap();
        p.extend(parse_respostas(RESPOSTAS_VIII, &da_pagina(PAGINA_VIII)).unwrap());
        nfe(p)
    }

    /// A entrada da IV da NFC-e como o índice a escreve (com o "IEmpresa" que a página corrige).
    fn nfce_iv() -> Vec<Entrada> {
        vec![Entrada {
            secao: "IV. Obrigações Acessórias".to_string(),
            subtopico: None,
            numero: Some(1),
            pagina: "/spedmg/nfce/Perguntas-Frequentes/respostas_iv/index.html".to_string(),
            ancora: "d1".to_string(),
            texto: "IEmpresa do simples nacional que aderir emissão da NFC será obrigada a transmitir o SPED FISCAL EFD ou continuará pelo SINTEGRA?".to_string(),
        }]
    }

    #[test]
    fn parse_home_le_os_modulos_na_ordem_da_barra() {
        let m = parse_home(HOME_HTML).unwrap();
        assert_eq!(m.len(), 15);
        assert_eq!(m[0].caminho, "/spedmg/efd/");
        assert_eq!(m[0].nome, "EFD - Escrituração Fiscal Digital");
        let nfe = m.iter().find(|m| m.caminho == "/spedmg/nfe/").unwrap();
        assert_eq!(nfe.nome, "NF-e - Nota Fiscal Eletrônica");
    }

    #[test]
    fn home_sem_modulo_e_erro() {
        assert!(erro(parse_home("<html></html>")).contains("nenhum módulo"));
    }

    #[test]
    fn link_das_faqs_vem_do_menu_lateral() {
        assert_eq!(
            link_das_faqs(INDICE_NFE).as_deref(),
            Some("/spedmg/nfe/Perguntas-Frequentes/")
        );
        assert_eq!(link_das_faqs(CTE_FAQ).as_deref(), Some("/spedmg/cte/faq/"));
        // "Perguntas" no conteúdo não é a entrada do menu.
        let so_no_conteudo = format!(
            r#"<p><a href="/spedmg/x/">Perguntas Frequentes</a></p>{MENU_LATERAL}"><a href="/spedmg/bpe/"> Início</a>"#
        );
        assert_eq!(link_das_faqs(&so_no_conteudo), None);
    }

    #[test]
    fn parse_indice_le_secoes_subtopicos_numeros_e_ancoras() {
        let e = parse_indice(INDICE_NFE).unwrap();
        assert_eq!(e.len(), 87);
        let secoes: Vec<&str> = e
            .chunk_by(|a, b| a.secao == b.secao)
            .map(|s| s[0].secao.as_str())
            .collect();
        assert_eq!(secoes.len(), 9);
        assert_eq!(secoes[0], "I. Conceito, uso e obrigatoriedade da NF-e");
        assert_eq!(secoes[8], "IX. Outras Informações:");
        assert_eq!(e[0].numero, Some(1));
        assert_eq!(e[0].ancora, "1");
        assert_eq!(
            e[0].pagina,
            "/spedmg/nfe/Perguntas-Frequentes/respostas_i/index.html"
        );
        assert_eq!(e[0].texto, "O que é a Nota Fiscal Eletrônica – NF-e?");
        // A âncora é por página: `#1` está na I e na II.
        assert_eq!(e[16].ancora, "1");
        assert!(e[16].pagina.ends_with("respostas_ii/index.html"));
        // Os 8 sub-tópicos da III; a III.1 vem antes do primeiro.
        let iii: Vec<&Entrada> = e.iter().filter(|e| e.pagina == PAGINA_III).collect();
        assert_eq!(iii.len(), 34);
        assert_eq!(iii[0].subtopico, None);
        assert_eq!(
            iii[1].subtopico.as_deref(),
            Some("Emissão e autorização da NF-e")
        );
        assert_eq!(
            iii[10].subtopico.as_deref(),
            Some("Correção, cancelamento e inutilização de NF-e")
        );
        let subtopicos: BTreeSet<&str> =
            iii.iter().filter_map(|e| e.subtopico.as_deref()).collect();
        assert_eq!(subtopicos.len(), 8);
        assert!(
            e.iter()
                .filter(|e| e.pagina != PAGINA_III)
                .all(|e| e.subtopico.is_none())
        );
    }

    #[test]
    fn link_de_pergunta_fora_de_tabela_e_erro() {
        let html = INDICE_NFE.replacen(
            "</table>",
            r#"</table><p><a href="/spedmg/nfe/Perguntas-Frequentes/respostas_i/index.html#99">Solta?</a></p>"#,
            1,
        );
        let e = erro(parse_indice(&html));
        assert!(
            e.contains("88 links de pergunta no HTML, 87 parseados"),
            "{e}"
        );
    }

    #[test]
    fn ancora_repetida_na_pagina_e_erro() {
        let html = INDICE_NFE.replacen(
            "respostas_ii/index.html#2\"",
            "respostas_ii/index.html#1\"",
            1,
        );
        let e = erro(parse_indice(&html));
        assert!(
            e.contains("respostas_ii/index.html#1 aparece duas vezes"),
            "{e}"
        );
    }

    #[test]
    fn pagina_do_cte_aponta_para_fora() {
        let Faq::Fora(links) = classificar("cte", CTE_FAQ).unwrap() else {
            panic!("esperava Fora");
        };
        assert_eq!(
            links,
            vec![
                "http://www.cte.fazenda.gov.br/portal/",
                "http://www.cte.fazenda.gov.br/portal/PerguntasFrequentes.aspx?tipoConteudo=l5imOVlDqPU="
            ]
        );
        assert!(matches!(classificar("nfe", INDICE_NFE).unwrap(), Faq::Html(e) if e.len() == 87));
    }

    #[test]
    fn indice_com_markup_novo_nao_vira_fora() {
        // Sem `respostas_` o parse não acha pergunta, mas a página ainda linka páginas do SPED: pular em
        // silêncio perderia o NF-e inteiro.
        let html = INDICE_NFE.replace("respostas_", "resposta_");
        let e = erro(classificar("nfe", &html));
        assert!(e.contains("nfe: sem link de `respostas_…`"), "{e}");
    }

    #[test]
    fn respostas_pela_ancora_name_sem_os_subtopicos() {
        let entradas = da_pagina(PAGINA_III);
        let p = parse_respostas(RESPOSTAS_III, &entradas).unwrap();
        assert_eq!(p.len(), 34);
        assert_eq!(p[0].pergunta, "Como funciona o modelo operacional da NF-e?");
        assert_eq!(
            p[7].pergunta,
            "É possível o envio por lote de NF-e ou a emissão deve ser feita nota a nota?"
        );
        assert!(
            p[7].resposta.starts_with(
                "A NF-e é um documento autônomo e a sua emissão deve ser feita nota a nota"
            ),
            "{}",
            p[7].resposta
        );
        // O "• <sub-tópico>" que a página põe antes da próxima pergunta sai da resposta anterior.
        let subtopicos: BTreeSet<&str> = entradas
            .iter()
            .filter_map(|e| e.subtopico.as_deref())
            .collect();
        for q in &p {
            let ultima = q.resposta.lines().last().unwrap_or_default();
            assert!(
                !subtopicos.contains(ultima.trim_start_matches("• ")),
                "{}: {ultima}",
                q.entrada.ancora
            );
        }
        assert!(
            p[9].resposta
                .starts_with("Não, o modelo nacional da nota fiscal eletrônica")
        );
    }

    #[test]
    fn pagina_sem_ancora_localiza_pelo_numero() {
        // A VIII não tem âncora, e a pergunta e a resposta estão no mesmo `<p>`, separadas por `<br>`.
        let p = parse_respostas(RESPOSTAS_VIII, &da_pagina(PAGINA_VIII)).unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(
            p[0].pergunta,
            "Como o destinatário poderá saber quais as NF-e foram emitidas para ele e como fazer o download dessas NF-e?"
        );
        let linhas: Vec<&str> = p[0].resposta.lines().collect();
        assert_eq!(linhas.len(), 5);
        assert!(linhas[0].starts_with(
            "► A Nota Técnica 2012/002 definiu o evento de Manifestação do Destinatário"
        ));
        assert_eq!(
            linhas[4],
            "► Consulte mais detalhes na Nota Técnica 2012.002 \"http://www.nfe.fazenda.gov.br/portal/exibirArquivo.aspx?conteudo=bUBJ/PmtKQo=\" - Define as especificações técnicas para a implementação dos eventos da Manifestação do Destinatário."
        );
    }

    #[test]
    fn respostas_pela_ancora_id_com_o_texto_da_pagina() {
        let p = parse_respostas(RESPOSTAS_NFCE_IV, &nfce_iv()).unwrap();
        assert_eq!(
            p[0].pergunta,
            "Empresa do simples nacional que aderir emissão da NFC será obrigada a transmitir o SPED FISCAL EFD ou continuará pelo SINTEGRA?"
        );
        assert_eq!(
            p[0].resposta.lines().nth(1),
            Some(
                "A NFC-e deve ser informada no registro tipo 61 do arquivo Sintegra, conforme prevê o \"Manual de Orientação do Convênio 57/95 \"https://www.confaz.fazenda.gov.br/legislacao/convenios/1995/cv057_95_manual_de_orientacao\"\"."
            )
        );
        // Sem a âncora, o número acha a mesma linha.
        let sem_ancora = RESPOSTAS_NFCE_IV.replace(r#"<a id="d1"></a>"#, "");
        let q = parse_respostas(&sem_ancora, &nfce_iv()).unwrap();
        assert_eq!(q[0].pergunta, p[0].pergunta);
        assert_eq!(q[0].resposta, p[0].resposta);
    }

    #[test]
    fn ancora_id_dentro_de_lista_numerada() {
        // NFC-e, respostas I (01/11/2026): da 2ª em diante o número é o `<ol start>`, não texto.
        let html = format!(
            r#"<div  id="conteudo" ><p><strong>I. Conceito, uso e obrigatoriedade da NFC-e</strong></p>
<p>&nbsp;</p>
<p>&nbsp;&nbsp;&nbsp;&nbsp; <a id="a1"></a>1.<strong> O que é a Nota Fiscal Eletrônica – NFC-e?</strong></p>
<p style="text-align: justify;">A Nota Fiscal de Consumidor Eletrônica - NFC-e -, modelo 65, é o documento digital emitido e armazenado eletronicamente.</p>
<p>&nbsp;</p>
<ol start="2">
<li><strong><a id="a2"></a>Já existe legislação em vigor para regulamentar a NFC-e?</strong></li>
</ol>
<p style="text-align: justify;">Sim. A NFC-e foi instituída pelo <a href="https://www.confaz.fazenda.gov.br/legislacao/ajustes/2016/AJ_019_16" target="_blank" rel="noopener noreferrer">Ajuste SINIEF nº 19/16</a>.</p>
{MENU_LATERAL}">"#
        );
        let entrada = |n: u32, texto: &str| Entrada {
            secao: "I. Conceito, uso e obrigatoriedade da NFC-e".to_string(),
            subtopico: None,
            numero: Some(n),
            pagina: "/spedmg/nfce/Perguntas-Frequentes/respostas_i/index.html".to_string(),
            ancora: format!("a{n}"),
            texto: texto.to_string(),
        };
        let p = parse_respostas(
            &html,
            &[
                entrada(1, "O que é a Nota Fiscal Eletrônica – NFC-e?"),
                entrada(
                    2,
                    "Já existe legislação em vigor para regulamentar a NFC-e?",
                ),
            ],
        )
        .unwrap();
        assert_eq!(p[0].pergunta, "O que é a Nota Fiscal Eletrônica – NFC-e?");
        assert_eq!(
            p[0].resposta,
            "A Nota Fiscal de Consumidor Eletrônica - NFC-e -, modelo 65, é o documento digital emitido e armazenado eletronicamente."
        );
        assert_eq!(
            p[1].pergunta,
            "Já existe legislação em vigor para regulamentar a NFC-e?"
        );
        assert_eq!(
            p[1].resposta,
            "Sim. A NFC-e foi instituída pelo Ajuste SINIEF nº 19/16 \"https://www.confaz.fazenda.gov.br/legislacao/ajustes/2016/AJ_019_16\"."
        );
    }

    #[test]
    fn pergunta_que_a_pagina_nao_tem_e_erro() {
        let html = RESPOSTAS_VIII.replacen("1. Como o destinatário", "Como o destinatário", 1);
        let e = erro(parse_respostas(&html, &da_pagina(PAGINA_VIII)));
        assert!(
            e.contains("respostas_viii/index.html#1: a pergunta do índice não está na página"),
            "{e}"
        );
    }

    #[test]
    fn linha_que_nao_e_a_pergunta_do_indice_e_erro() {
        let mut entradas = da_pagina(PAGINA_III);
        entradas[7].texto = "Qual é o prazo de validade do certificado digital?".to_string();
        let e = erro(parse_respostas(RESPOSTAS_III, &entradas));
        assert!(
            e.contains("respostas_iii/index.html#8: a linha achada não é a pergunta do índice"),
            "{e}"
        );
    }

    #[test]
    fn nenhuma_pergunta_e_erro() {
        assert!(erro(consolidar(&[])).contains("nenhuma pergunta em HTML"));
    }

    #[test]
    fn resposta_vazia_so_avisa() {
        let mut m = iii_e_viii();
        m.perguntas[0].resposta.clear();
        assert_eq!(consolidar(&[m]).unwrap().len(), 35);
    }

    #[test]
    fn faq_raw_tem_trilha_de_dois_ou_tres_niveis_e_link_do_indice() {
        let faqs = consolidar(&[iii_e_viii()]).unwrap();
        assert_eq!(
            faqs[0].origin,
            "NF-e - Nota Fiscal Eletrônica | III. Modelo Operacional - O que muda com a NF-e:"
        );
        assert_eq!(
            faqs[1].origin,
            "NF-e - Nota Fiscal Eletrônica | III. Modelo Operacional - O que muda com a NF-e: | Emissão e autorização da NF-e"
        );
        assert_eq!(
            faqs[1].url,
            "https://portalsped.fazenda.mg.gov.br/spedmg/nfe/Perguntas-Frequentes/respostas_iii/index.html#2"
        );
        assert_eq!(
            faqs[34].url,
            "https://portalsped.fazenda.mg.gov.br/spedmg/nfe/Perguntas-Frequentes/respostas_viii/index.html#1"
        );
    }

    #[test]
    fn arvore_tem_modulo_secao_e_subtopico() {
        let t = montar_arvore(&[iii_e_viii()]);
        assert_eq!(t.title, "Perguntas frequentes");
        assert_eq!(t.url, "https://portalsped.fazenda.mg.gov.br/spedmg/");
        let nfe = &t.children[0];
        assert_eq!(nfe.page_type, PageType::Menu);
        assert_eq!(nfe.title, "NF-e - Nota Fiscal Eletrônica");
        assert_eq!(nfe.children.len(), 2);

        // A III tem sub-tópicos: `Menu` com um `Faq` por sub-tópico, e a III.1 nos itens do próprio `Menu`.
        let iii = &nfe.children[0];
        assert_eq!(iii.page_type, PageType::Menu);
        assert_eq!(iii.url, format!("{BASE}{PAGINA_III}"));
        assert_eq!(iii.children.len(), 8);
        assert_eq!(iii.children[0].title, "Emissão e autorização da NF-e");
        assert_eq!(iii.children[0].page_type, PageType::Faq);
        assert_eq!(
            iii.children[0].faq_items[0].url.as_deref(),
            Some(format!("{BASE}{PAGINA_III}#2").as_str())
        );
        assert_eq!(iii.faq_items.len(), 1);
        assert_eq!(
            iii.faq_items[0].pergunta,
            "Como funciona o modelo operacional da NF-e?"
        );
        let soma: usize = iii.children.iter().map(|c| c.faq_items.len()).sum();
        assert_eq!(soma + iii.faq_items.len(), 34);

        // A VIII não tem: `Faq` direto. O fio que o `parseFaqs.ts` lê: folha sem `children`.
        let viii = &nfe.children[1];
        assert_eq!(viii.page_type, PageType::Faq);
        assert_eq!(viii.title, "VIII. Sobre Download da NF-e");
        let json = serde_json::to_value(viii).unwrap();
        assert_eq!(json["page_type"], "Faq");
        assert!(json.get("children").is_none());
    }

    #[test]
    fn html_para_texto_absolutiza_tira_tags_e_decodifica_depois() {
        assert_eq!(
            html_para_texto(
                r#"<p>Veja o <a href="/spedmg/nfe/Obrigatoriedade/">cronograma</a>.</p><table><tr><td>CST</td><td>53</td></tr></table><p>&lt;b&gt; e &gt;A Sefaz&nbsp;Virtual</p>"#,
                BASE
            ),
            "Veja o cronograma \"https://portalsped.fazenda.mg.gov.br/spedmg/nfe/Obrigatoriedade/\".\nCST 53\n<b> e >A Sefaz Virtual"
        );
    }

    #[test]
    fn html_para_texto_decodifica_as_numericas_do_csm_e_absolutiza_no_base_dado() {
        // Do texto do CSM (02/11/2026): `&#34;` e `&#61;` no corpo e no href.
        assert_eq!(
            html_para_texto(
                r#"<p>A rejeição &#34;690&#34; (indIEDest&#61;2) — <a href="/csm?id&#61;kb_search">busca</a> e <a href="http://x.gov.br/a.aspx?t&#61;r&amp;c&#61;7">consulta</a></p>"#,
                "https://atendimento2.fazenda.mg.gov.br"
            ),
            "A rejeição \"690\" (indIEDest=2) — busca \"https://atendimento2.fazenda.mg.gov.br/csm?id=kb_search\" e consulta \"http://x.gov.br/a.aspx?t=r&c=7\""
        );
    }
}
