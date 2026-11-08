// SEFAZ-PI FAQs — a seção "Perguntas Frequentes" das 8 páginas de Documentos Eletrônicos do portal (Sydle ONE) e o PDF
// "Perguntas Frequentes EFD ICMS IPI" do repositório de documentos. A árvore de temas "Perguntas Frequentes" do portal
// está vazia. Estudo, medições e decisões: docs/faqs/ESTUDO-RO-PI-Faqs.md (D-SYDFAQ-*).
//
// Páginas: um `_search` (os artigos ligados à pasta "Documentos Eletrônicos"); a pergunta é a linha em negrito
// terminada em "?", pelo `kit::linhas`, depois de dois reparos (D-SYDFAQ-5). PDF: o método `obterPastaComItens` lista a
// pasta "4. Perguntas Frequentes" (exatamente um PDF); o texto sai do `pdftotext -layout` (poppler) e as perguntas, do
// sumário (D-SYDFAQ-8).

use std::collections::HashSet;
use std::path::Path;
use std::sync::LazyLock;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use regex::Regex;
use scraper::{Html, Selector};
use serde::Deserialize;

use auli_contract::FaqRaw;
use auli_scraper_kit::faqtree::{FaqItem, FaqNode, PageType};
use auli_scraper_kit::html::{Opcoes, html_para_texto};
use auli_scraper_kit::http::GetOpts;
use auli_scraper_kit::linhas::{linhas, segmentar};

use crate::pi;

/// A pasta "Conteúdos - Institucional → Documentos Eletrônicos": as 8 páginas se ligam a ela (`linkedParents`).
const PASTA_DOCUMENTOS_ELETRONICOS: &str = "69d6979dc41c7a09017fa749";
/// As pastas de FAQ vazias do portal (07/11/2026): o tema "Informações sobre a legislação tributária | Perguntas
/// Frequentes", os 7 subtemas dele e as "Dúvidas gerais" do Nota Piauiense e do IPVA. Artigo nelas é aviso: hora de
/// rever o escopo.
const PASTAS_FAQ_VAZIAS: [&str; 10] = [
    "69b9aa26054de51bfeaf9f02",
    "69bc4656dc1cbf3207c3501f",
    "69bc4efddc1cbf3207c6fc3e",
    "69bc4820dc1cbf3207e0a859",
    "69bc4375dc1cbf3207aa5f86",
    "69bc49f0dc1cbf32071f27b2",
    "69bc503bdc1cbf3207f23f4d",
    "69bc461edc1cbf3207c1b9ef",
    "69bc478fdc1cbf3207cdb6f1",
    "69bc47a4dc1cbf3207ce3cce",
];
/// A classe dos artigos (as pastas são de outra).
const CLASSE_ARTIGO: &str = "5d66ed07e7c98a556a499a36";
/// O método do repositório de documentos que a página `/documentos/<pasta>` chama (GET anônimo).
const API_PASTA: &str = "https://portal.sefaz.pi.gov.br/api/1/govPiPortalInstitucional/_classId/68e40169df43e345a988b6dd/obterPastaComItens";
/// "Legislação e Documentos EFD → 4. Perguntas Frequentes".
const PASTA_FAQ_EFD: &str = "legislacao-e-documentos-efd-4-perguntas-frequentes";
const CORTESIA: Duration = Duration::from_secs(1);
/// O PDF tem 425 KB (07/11/2026).
const LIMITE_PDF: u64 = 20 * 1024 * 1024;
/// Pisos: 99 perguntas nas páginas e 35 no PDF em 07/11/2026.
const PISO_PAGINAS: usize = 90;
const PISO_PDF: usize = 30;

/// O link para o ambiente de homologação (`portal-h`) que o editor deixou nas perguntas do CT-e e da DeSTDA: sai, e o
/// texto fica em destaque (a pergunta do CT-e é só o link, sem negrito).
static PORTAL_H_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)<a\b[^>]*href="https?://portal-h\.sefaz\.pi\.gov\.br[^"]*"[^>]*>(.*?)</a>"#)
        .unwrap()
});
/// O texto-modelo da sanfona do Elementor que sobrou em duas perguntas: partido em volta da pergunta ("Título da
/// sanfo" + pergunta + "na") ou inteiro depois dela.
static SANFONA_PARTIDA_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"T(?:í|&iacute;)tulo da sanfo([^<]*?\?)na").unwrap());
static SANFONA_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"T(?:í|&iacute;)tulo da sanfona").unwrap());
static SECAO: LazyLock<Selector> = LazyLock::new(|| Selector::parse("section.secao").unwrap());
static H3: LazyLock<Selector> = LazyLock::new(|| Selector::parse("h3").unwrap());
static CONTEUDO: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("div.conteudo-html").unwrap());

/// O rodapé de cada página do PDF.
static RODAPE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*Secretaria da Fazenda do Estado do Piauí\s+\d+\s*$").unwrap()
});
/// Uma entrada do sumário começa pelo número ("1", "1.5", "1.5.2") e termina na linha com o pontilhado e a página.
static NUMERO_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(\d+(?:\.\d+)*)\s+(\S.*)$").unwrap());
/// O pontilhado longo, que marca as páginas do sumário.
static PONTILHADO_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\s*\.{4,}\s*\d+\s*$").unwrap());
/// O fim de uma entrada do sumário: o pontilhado e a página. O pontilhado longo pode vir colado ("CADASTRO……. 4"); o
/// curto ("fazer? . 5", "(bloco H)?.. 7") vem depois de espaço, "?" ou ")" — "Versão 1.0" não fecha entrada.
static FIM_ENTRADA_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:\.{4,}|(?:\s|[?)])\.+)\s*\d+\s*$").unwrap());
static PAGINA_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s*\.+\s*\d+\s*$").unwrap());
/// A data da capa ("Novembro de 2021").
static DATA_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(janeiro|fevereiro|março|abril|maio|junho|julho|agosto|setembro|outubro|novembro|dezembro) de \d{4}$")
        .unwrap()
});

/// Coleta as FAQs e grava a árvore do frontend (`<id>-faqs-tree.json`) e o snapshot. Tudo é conferido antes de
/// qualquer gravação: rodada que falha deixa os arquivos anteriores intactos.
pub fn run(data_dir: &str, use_cache: bool) -> Result<()> {
    let agent = auli_scraper_kit::build_agent(crate::UA, Some(Duration::from_secs(60)));
    // O token anônimo do HTML inicial, efêmero: um por rodada. Com `--usecache` não há rede.
    let token = if use_cache {
        String::new()
    } else {
        pi::fetch_token(&agent)?
    };
    let rede = Rede {
        agent: &agent,
        token: &token,
        data_dir,
        use_cache,
    };

    // 1) As 8 páginas de Documentos Eletrônicos.
    let busca = rede.json(
        &format!("{}#faqs-documentos-eletronicos", pi::SEARCH_URL),
        &busca_url(),
    )?;
    let mut paginas = Vec::new();
    for d in documentos(&busca)? {
        let perguntas = da_pagina(d.html.as_deref().unwrap_or(""))
            .with_context(|| format!("PI FAQs: página {}", d.nome()))?;
        println!("PI FAQs: {}: {} perguntas", d.nome(), perguntas.len());
        paginas.push(Grupo {
            titulo: d.nome(),
            url: d.link(),
            capitulos: vec![Capitulo {
                titulo: String::new(),
                url: d.link(),
                perguntas: perguntas
                    .into_iter()
                    .map(|(pergunta, resposta)| Pergunta {
                        pergunta,
                        resposta,
                        trilha: d.nome(),
                        url: d.link(),
                    })
                    .collect(),
            }],
        });
    }
    paginas.sort_by(|a, b| a.titulo.cmp(&b.titulo));
    piso(total(&paginas), PISO_PAGINAS, "nas páginas")?;

    // 2) O PDF da FAQ da EFD.
    let listagem = rede.json(&format!("{API_PASTA}#{PASTA_FAQ_EFD}"), &pasta_url())?;
    let arquivo = arquivo_da_pasta(&listagem)?;
    let texto = rede.pdf_texto(&arquivo.url)?;
    let pdf = do_pdf(&texto, &arquivo)?;
    println!("PI FAQs: {}: {} perguntas", pdf.titulo, total(&[&pdf]));
    piso(total(&[&pdf]), PISO_PDF, "no PDF")?;

    // 3) As pastas de FAQ que hoje estão vazias.
    let vigia = rede.json(
        &format!("{}#faqs-pastas-vazias", pi::SEARCH_URL),
        &vigia_url(),
    )?;
    if let Some(aviso) = aviso_pastas(&vigia)? {
        println!("{aviso}");
    }

    let mut grupos = paginas;
    grupos.push(pdf);
    sem_colisao(&grupos)?;
    println!("PI FAQs: {} perguntas", total(&grupos));

    // Atômico pelo mesmo motivo das outras entidades: a árvore não tem outra fonte (o snapshot achatado não a
    // reconstrói) e é o único artefato de coleta que o frontend lê direto.
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
        consolidar(&grupos),
    )?;
    println!("🎉 Coleta de FAQs gravada no snapshot.");
    Ok(())
}

/// As chamadas de rede, cada uma com o cache (D-SYDFAQ-7): rede sempre, gravando o cache; com `--usecache`, só o
/// cache, e miss é erro.
struct Rede<'a> {
    agent: &'a ureq::Agent,
    token: &'a str,
    data_dir: &'a str,
    use_cache: bool,
}

impl Rede<'_> {
    fn cache(&self, chave: &str) -> Result<String> {
        auli_scraper_kit::cache::read(self.data_dir, "faqs", chave)
            .ok_or_else(|| anyhow!("cache miss para {chave} (modo --usecache, sem rede)"))
    }

    /// Um GET da API (o corpo ES ou os argumentos do método em `?_body=`).
    fn json(&self, chave: &str, url: &str) -> Result<String> {
        if self.use_cache {
            return self.cache(chave);
        }
        sleep(CORTESIA);
        let corpo = auli_scraper_kit::http::get_string(
            self.agent,
            url,
            &GetOpts {
                log_prefix: "PI FAQs",
                accept: Some("application/json"),
                headers: &[("Authorization", self.token)],
                ..Default::default()
            },
        )?;
        auli_scraper_kit::cache::write(self.data_dir, "faqs", chave, &corpo);
        Ok(corpo)
    }

    /// O texto do PDF. O cache guarda o texto já extraído: o `--usecache` não precisa do poppler.
    fn pdf_texto(&self, url: &str) -> Result<String> {
        let chave = format!("{url}#pdftotext");
        if self.use_cache {
            return self.cache(&chave);
        }
        sleep(CORTESIA);
        println!("Fetching: {url}");
        let pdf = self
            .agent
            .get(url)
            .call()
            .with_context(|| format!("GET {url}"))?
            .body_mut()
            .with_config()
            .limit(LIMITE_PDF)
            .read_to_vec()
            .with_context(|| format!("corpo de {url}"))?;
        let texto = auli_scraper_kit::pdf::extrair(&pdf)?.texto;
        auli_scraper_kit::cache::write(self.data_dir, "faqs", &chave, &texto);
        Ok(texto)
    }
}

/// O `_search` dos artigos ativos ligados à pasta "Documentos Eletrônicos" (`linkedParents` é `nested`).
fn busca_url() -> String {
    let corpo = serde_json::json!({
        "query": { "bool": { "must": [
            { "term": { "active": true } },
            { "nested": { "path": "linkedParents", "query": {
                "term": { "linkedParents.parent._id": PASTA_DOCUMENTOS_ELETRONICOS }
            } } }
        ] } },
        "size": 50
    })
    .to_string();
    format!("{}?_body={}", pi::SEARCH_URL, pi::pct_encode(&corpo))
}

/// Quantos artigos ativos estão nas pastas de FAQ vazias (filhos ou ligados). `size` 0: só a contagem.
fn vigia_url() -> String {
    let corpo = serde_json::json!({
        "query": { "bool": {
            "must": [
                { "term": { "active": true } },
                { "term": { "_class._id": CLASSE_ARTIGO } }
            ],
            "should": [
                { "terms": { "parent._id": PASTAS_FAQ_VAZIAS } },
                { "nested": { "path": "linkedParents", "query": {
                    "terms": { "linkedParents.parent._id": PASTAS_FAQ_VAZIAS }
                } } }
            ],
            "minimum_should_match": 1
        } },
        "size": 0
    })
    .to_string();
    format!("{}?_body={}", pi::SEARCH_URL, pi::pct_encode(&corpo))
}

/// O aviso, quando as pastas de FAQ vazias ganharam artigo.
fn aviso_pastas(json: &str) -> Result<Option<String>> {
    let b: Busca =
        serde_json::from_str(json).context("JSON do _search das pastas de FAQ inválido")?;
    Ok((b.hits.total > 0).then(|| {
        format!(
            "⚠️  PI FAQs: {} artigo(s) nas pastas de FAQ do portal, vazias em 07/11/2026 — reveja o escopo \
             (docs/faqs/ESTUDO-RO-PI-Faqs.md §1)",
            b.hits.total
        )
    }))
}

fn pasta_url() -> String {
    let corpo = serde_json::json!({ "folderIdentifier": PASTA_FAQ_EFD }).to_string();
    format!("{API_PASTA}?_body={}", pi::pct_encode(&corpo))
}

#[derive(Deserialize)]
struct Busca {
    hits: Hits,
}

#[derive(Deserialize)]
struct Hits {
    total: usize,
    hits: Vec<Hit>,
}

#[derive(Deserialize)]
struct Hit {
    #[serde(rename = "_source")]
    doc: Doc,
}

#[derive(Deserialize)]
struct Doc {
    #[serde(rename = "_id")]
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(rename = "friendlyUrl", default)]
    friendly_url: Option<String>,
    #[serde(rename = "contentHtml", default)]
    html: Option<String>,
}

impl Doc {
    fn nome(&self) -> String {
        auli_scraper_kit::clean(self.name.as_deref().unwrap_or(""))
    }

    /// A página no portal (`/<friendlyUrl>`, como os serviços).
    fn link(&self) -> String {
        match self.friendly_url.as_deref().map(str::trim) {
            Some(f) if !f.is_empty() => format!("{}/{f}", pi::BASE),
            _ => format!("{}/{}", pi::BASE, self.id),
        }
    }
}

/// As páginas, com a guarda de cobertura.
fn documentos(json: &str) -> Result<Vec<Doc>> {
    let b: Busca = serde_json::from_str(json).context("JSON do _search inválido")?;
    if b.hits.total != b.hits.hits.len() {
        bail!(
            "o _search achou {} páginas e trouxe {}",
            b.hits.total,
            b.hits.hits.len()
        );
    }
    Ok(b.hits.hits.into_iter().map(|h| h.doc).collect())
}

/// Os reparos de D-SYDFAQ-5, no HTML: o link de homologação vira destaque e o texto-modelo da sanfona sai.
fn reparos(html: &str) -> String {
    let html = PORTAL_H_RE.replace_all(html, "<strong>$1</strong>");
    let html = SANFONA_PARTIDA_RE.replace_all(&html, "$1");
    SANFONA_RE.replace_all(&html, "").into_owned()
}

/// As perguntas da seção "Perguntas Frequentes …" de uma página.
fn da_pagina(html: &str) -> Result<Vec<(String, String)>> {
    let doc = Html::parse_fragment(&reparos(html));
    let secoes: Vec<_> = doc
        .select(&SECAO)
        .filter(|s| {
            s.select(&H3).next().is_some_and(|h| {
                h.text()
                    .collect::<String>()
                    .to_lowercase()
                    .contains("perguntas")
            })
        })
        .collect();
    let [secao] = secoes.as_slice() else {
        bail!(
            "{} seções \"Perguntas Frequentes\" (esperava 1) — a página mudou?",
            secoes.len()
        );
    };
    let conteudo = secao
        .select(&CONTEUDO)
        .next()
        .ok_or_else(|| anyhow!("a seção de perguntas não tem o div.conteudo-html"))?;
    let mut out = Vec::new();
    for b in segmentar(&linhas(conteudo)) {
        let resposta = html_para_texto(
            &b.resposta_html,
            Opcoes {
                base: Some(pi::BASE),
                quebras_do_fonte: false,
            },
        );
        if resposta.is_empty() {
            bail!("a pergunta {:?} ficou sem resposta", b.titulo);
        }
        for t in [&b.titulo, &resposta] {
            if t.contains("sanfo") || t.contains("portal-h") {
                bail!(
                    "sobrou um resto do editor em {:?} — os reparos não pegaram",
                    b.titulo
                );
            }
        }
        out.push((b.titulo, resposta));
    }
    if out.is_empty() {
        bail!("a seção \"Perguntas Frequentes\" não tem pergunta");
    }
    Ok(out)
}

/// O arquivo da pasta do repositório.
#[derive(Debug, PartialEq)]
struct Arquivo {
    /// O título no repositório, sem o ".pdf".
    titulo: String,
    url: String,
}

#[derive(Deserialize)]
struct Pasta {
    items: Vec<ItemPasta>,
}

#[derive(Deserialize)]
struct ItemPasta {
    #[serde(rename = "isFile", default)]
    arquivo: bool,
    #[serde(default)]
    title: String,
    #[serde(rename = "fileName", default)]
    nome: String,
    #[serde(rename = "fileUrl", default)]
    url: Option<String>,
}

/// O único PDF da pasta. Mais de um é erro: a SEFAZ costuma deixar as versões antigas ao lado (o Guia Prático tem 7),
/// e uma versão nova pede revisão.
fn arquivo_da_pasta(json: &str) -> Result<Arquivo> {
    let p: Pasta = serde_json::from_str(json).context("JSON de obterPastaComItens inválido")?;
    let pdfs: Vec<&ItemPasta> = p
        .items
        .iter()
        .filter(|i| i.arquivo && i.nome.to_lowercase().ends_with(".pdf"))
        .collect();
    let [pdf] = pdfs.as_slice() else {
        bail!(
            "a pasta {PASTA_FAQ_EFD} tem {} PDFs (esperava 1) — versão nova? Revise o escopo (D-SYDFAQ-8)",
            pdfs.len()
        );
    };
    let url = pdf
        .url
        .clone()
        .filter(|u| !u.is_empty())
        .ok_or_else(|| anyhow!("o PDF {:?} veio sem fileUrl", pdf.title))?;
    let titulo = pdf.title.trim();
    Ok(Arquivo {
        titulo: titulo
            .strip_suffix(".pdf")
            .unwrap_or(titulo)
            .trim()
            .to_string(),
        url,
    })
}

/// Uma entrada do sumário do PDF.
#[derive(Debug)]
struct Entrada {
    numero: String,
    titulo: String,
}

/// Um nó de 1º nível da árvore: uma página, ou o PDF.
struct Grupo {
    titulo: String,
    url: String,
    /// As páginas têm um capítulo só, sem título; o PDF, um por capítulo do sumário.
    capitulos: Vec<Capitulo>,
}

struct Capitulo {
    titulo: String,
    url: String,
    perguntas: Vec<Pergunta>,
}

struct Pergunta {
    pergunta: String,
    resposta: String,
    trilha: String,
    url: String,
}

fn total(grupos: &[impl std::borrow::Borrow<Grupo>]) -> usize {
    grupos
        .iter()
        .flat_map(|g| &g.borrow().capitulos)
        .map(|c| c.perguntas.len())
        .sum()
}

/// O texto do PDF → as perguntas, pelo sumário: ele dá a lista e a ordem (o número e se é pergunta — termina em "?"
/// — ou capítulo/subgrupo); o corpo dá o texto. Guardas: a numeração do sumário não pula, e cada entrada dele está
/// no corpo, na ordem.
fn do_pdf(texto: &str, arquivo: &Arquivo) -> Result<Grupo> {
    let paginas: Vec<&str> = texto.split('\u{c}').collect();
    let data = paginas
        .first()
        .and_then(|capa| capa.lines().map(str::trim).find(|l| DATA_RE.is_match(l)))
        .ok_or_else(|| anyhow!("a capa do PDF não tem a data (\"Novembro de 2021\")"))?;
    let titulo = format!("{} ({data})", arquivo.titulo);

    // O sumário são as páginas com pontilhado; o corpo, as que vêm depois da última delas.
    let ultima_do_sumario = paginas
        .iter()
        .rposition(|p| p.lines().any(|l| PONTILHADO_RE.is_match(l)))
        .ok_or_else(|| anyhow!("o PDF não tem sumário"))?;
    let entradas = sumario(&paginas[..=ultima_do_sumario])?;
    let corpo = corpo(&paginas, ultima_do_sumario + 1);

    // As posições das entradas no corpo, em ordem.
    let mut posicoes = Vec::with_capacity(entradas.len());
    let mut desde = 0;
    for e in &entradas {
        let i = (desde..corpo.len())
            .find(|&i| {
                NUMERO_RE
                    .captures(corpo[i].1)
                    .is_some_and(|c| c[1] == e.numero && comeca_igual(&c[2], &e.titulo))
            })
            .ok_or_else(|| {
                anyhow!(
                    "a entrada {} do sumário ({:?}) não está no corpo do PDF",
                    e.numero,
                    e.titulo
                )
            })?;
        posicoes.push(i);
        desde = i + 1;
    }

    let mut capitulos: Vec<Capitulo> = Vec::new();
    let mut subgrupo: Option<String> = None;
    for (k, e) in entradas.iter().enumerate() {
        let inicio = posicoes[k];
        let fim = posicoes.get(k + 1).copied().unwrap_or(corpo.len());
        let pagina = corpo[inicio].0;
        let url = format!("{}#page={pagina}", arquivo.url);
        let nivel = e.numero.matches('.').count();
        if nivel == 0 {
            capitulos.push(Capitulo {
                titulo: e.titulo.clone(),
                url,
                perguntas: Vec::new(),
            });
            subgrupo = None;
            continue;
        }
        // Um item de 2º nível abre um ramo novo: o subgrupo anterior ("1.5 Certificado Digital") fica para trás.
        if nivel == 1 {
            subgrupo = None;
        }
        if !e.titulo.ends_with('?') {
            subgrupo = Some(e.titulo.clone());
            continue;
        }
        // A pergunta no corpo vai até a linha que fecha com "?"; o resto até a próxima entrada é a resposta.
        let mut linhas = corpo[inicio..fim].iter().map(|(_, l)| *l);
        let mut pergunta = Vec::new();
        for l in linhas.by_ref() {
            pergunta.push(l.trim());
            if l.trim_end().ends_with('?') {
                break;
            }
        }
        let pergunta = pergunta
            .join(" ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let resposta = paragrafos(linhas);
        if resposta.is_empty() {
            bail!("a pergunta {pergunta:?} do PDF ficou sem resposta");
        }
        let Some(cap) = capitulos.last_mut() else {
            bail!("a pergunta {} vem antes do 1º capítulo", e.numero);
        };
        let trilha = std::iter::once(titulo.as_str())
            .chain(std::iter::once(cap.titulo.as_str()))
            .chain(subgrupo.as_deref())
            .collect::<Vec<_>>()
            .join(" | ");
        cap.perguntas.push(Pergunta {
            pergunta,
            resposta,
            trilha,
            url,
        });
    }
    Ok(Grupo {
        titulo,
        url: arquivo.url.clone(),
        capitulos,
    })
}

/// A linha do corpo começa como o título do sumário (os 20 primeiros caracteres, sem contar espaços): uma linha de
/// resposta que começa por número ("2 dias…") não é o cabeçalho do capítulo 2.
fn comeca_igual(linha: &str, titulo: &str) -> bool {
    let norm = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
    let (l, t) = (norm(linha), norm(titulo));
    let n = l.chars().count().min(t.chars().count()).min(20);
    n > 0 && l.chars().take(n).eq(t.chars().take(n))
}

/// As entradas do sumário: do número até a linha com o pontilhado.
fn sumario(paginas: &[&str]) -> Result<Vec<Entrada>> {
    let mut out = Vec::new();
    let mut atual: Option<(String, Vec<String>)> = None;
    for l in paginas.iter().flat_map(|p| p.lines()) {
        if RODAPE_RE.is_match(l) {
            continue;
        }
        if atual.is_none() {
            if let Some(c) = NUMERO_RE.captures(l) {
                atual = Some((c[1].to_string(), vec![c[2].to_string()]));
            } else {
                continue;
            }
        } else if let Some((_, partes)) = atual.as_mut() {
            partes.push(l.trim().to_string());
        }
        if FIM_ENTRADA_RE.is_match(l)
            && let Some((numero, partes)) = atual.take()
        {
            let titulo = PAGINA_RE.replace(&partes.join(" "), "").into_owned();
            out.push(Entrada {
                numero,
                titulo: titulo.split_whitespace().collect::<Vec<_>>().join(" "),
            });
        }
    }
    if out.is_empty() {
        bail!("o sumário do PDF está vazio");
    }
    numeracao_continua(&out)?;
    Ok(out)
}

/// A numeração do sumário não pula: depois de "1.4" vem "1.5", "1.4.1" ou "2"; o primeiro filho é ".1". Uma entrada
/// que engoliu a seguinte (o pontilhado que o parser não reconheceu) quebra a sequência.
fn numeracao_continua(entradas: &[Entrada]) -> Result<()> {
    let mut anterior: Vec<u32> = Vec::new();
    for e in entradas {
        let n: Vec<u32> = e
            .numero
            .split('.')
            .map(|p| {
                p.parse()
                    .map_err(|_| anyhow!("número {:?} no sumário", e.numero))
            })
            .collect::<Result<_>>()?;
        let k = n.len() - 1;
        let esperado = if k < anterior.len() {
            anterior[k] + 1
        } else {
            1
        };
        if k > anterior.len() || n[..k] != anterior[..k] || n[k] != esperado {
            bail!(
                "o sumário do PDF pula de {:?} para {} ({:?}) — uma entrada engoliu outra?",
                anterior,
                e.numero,
                e.titulo
            );
        }
        anterior = n;
    }
    Ok(())
}

/// As linhas do corpo, com a página de cada uma (1-based), sem o rodapé e sem as linhas vazias das bordas de cada
/// página — assim o parágrafo que atravessa a página se junta.
fn corpo<'a>(paginas: &[&'a str], desde: usize) -> Vec<(usize, &'a str)> {
    let mut out = Vec::new();
    for (i, p) in paginas.iter().enumerate().skip(desde) {
        let linhas: Vec<&str> = p.lines().filter(|l| !RODAPE_RE.is_match(l)).collect();
        let ini = linhas.iter().position(|l| !l.trim().is_empty());
        let fim = linhas.iter().rposition(|l| !l.trim().is_empty());
        if let (Some(a), Some(b)) = (ini, fim) {
            out.extend(linhas[a..=b].iter().map(|l| (i + 1, *l)));
        }
    }
    out
}

/// As linhas da resposta em parágrafos: no `-layout`, o parágrafo começa recuado (ou com o marcador "▪") e continua
/// na margem; linha vazia fecha o parágrafo.
fn paragrafos<'a>(linhas: impl Iterator<Item = &'a str>) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut aberto = false;
    for l in linhas {
        let t = l.trim();
        if t.is_empty() {
            aberto = false;
            continue;
        }
        let novo = !aberto || l.starts_with(char::is_whitespace) || t.starts_with(['▪', '•', '●']);
        if novo {
            out.push(t.to_string());
        } else if let Some(ultimo) = out.last_mut() {
            ultimo.push(' ');
            ultimo.push_str(t);
        }
        aberto = true;
    }
    out.iter()
        .map(|p| p.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Coleta capada é `Err`, não snapshot menor.
fn piso(n: usize, minimo: usize, onde: &str) -> Result<()> {
    if n < minimo {
        bail!("PI FAQs: {n} perguntas {onde} (piso {minimo}) — o portal mudou?");
    }
    Ok(())
}

/// O nome do `.md` sai de url + pergunta: a mesma pergunta na mesma página, com respostas diferentes, faria um
/// sobrescrever o outro.
fn sem_colisao(grupos: &[Grupo]) -> Result<()> {
    let mut vistas = HashSet::new();
    for p in grupos
        .iter()
        .flat_map(|g| &g.capitulos)
        .flat_map(|c| &c.perguntas)
    {
        if !vistas.insert((p.url.as_str(), p.pergunta.as_str())) {
            bail!("{} traz {:?} duas vezes", p.url, p.pergunta);
        }
    }
    Ok(())
}

fn consolidar(grupos: &[Grupo]) -> Vec<FaqRaw> {
    grupos
        .iter()
        .flat_map(|g| &g.capitulos)
        .flat_map(|c| &c.perguntas)
        .map(|p| FaqRaw {
            pergunta: p.pergunta.clone(),
            resposta: p.resposta.clone(),
            origin: p.trilha.clone(),
            url: p.url.clone(),
        })
        .collect()
}

fn itens(perguntas: &[Pergunta]) -> Vec<FaqItem> {
    perguntas
        .iter()
        .map(|p| FaqItem {
            pergunta: p.pergunta.clone(),
            resposta: p.resposta.clone(),
            url: Some(p.url.clone()),
        })
        .collect()
}

/// A árvore da aba de FAQs: raiz → as páginas (com as perguntas) e o PDF (→ capítulos → perguntas).
fn montar_arvore(grupos: &[Grupo]) -> FaqNode {
    FaqNode {
        title: "Perguntas frequentes".to_string(),
        url: format!("{}/", pi::BASE),
        page_type: PageType::Menu,
        origin: String::new(),
        children: grupos
            .iter()
            .map(|g| match g.capitulos.as_slice() {
                [c] if c.titulo.is_empty() => FaqNode {
                    title: g.titulo.clone(),
                    url: g.url.clone(),
                    page_type: PageType::Faq,
                    origin: g.titulo.clone(),
                    children: Vec::new(),
                    faq_items: itens(&c.perguntas),
                },
                caps => FaqNode {
                    title: g.titulo.clone(),
                    url: g.url.clone(),
                    page_type: PageType::Menu,
                    origin: g.titulo.clone(),
                    children: caps
                        .iter()
                        .map(|c| FaqNode {
                            title: c.titulo.clone(),
                            url: c.url.clone(),
                            page_type: PageType::Faq,
                            origin: format!("{} | {}", g.titulo, c.titulo),
                            children: Vec::new(),
                            faq_items: itens(&c.perguntas),
                        })
                        .collect(),
                    faq_items: Vec::new(),
                },
            })
            .collect(),
        faq_items: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Da coleta de 07/11/2026, só com os campos que o scraper lê e as imagens base64 esvaziadas: 4 das 8 páginas de
    // Documentos Eletrônicos — CT-e (a pergunta é um link para o `portal-h`, uma com o texto-modelo partido em volta),
    // DeSTDA (`<h4>` do Elementor, uma com o texto-modelo depois), NFC-e (linhas em negrito sem "?" na resposta) e
    // NF-e —, a listagem da pasta "4. Perguntas Frequentes" e o texto do PDF pelo `pdftotext -layout`.
    const BUSCA: &str = include_str!("../tests/fixtures/pi-faqs-busca.json");
    const PASTA: &str = include_str!("../tests/fixtures/pi-faqs-pasta.json");
    const EFD: &str = include_str!("../tests/fixtures/pi-faqs-efd.txt");

    fn erro<T>(r: Result<T>) -> String {
        match r {
            Ok(_) => panic!("esperava Err"),
            Err(e) => format!("{e:#}"),
        }
    }

    fn pagina(prefixo: &str) -> Vec<(String, String)> {
        let docs = documentos(BUSCA).unwrap();
        let d = docs.iter().find(|d| d.nome().starts_with(prefixo)).unwrap();
        da_pagina(d.html.as_deref().unwrap()).unwrap()
    }

    fn arquivo() -> Arquivo {
        arquivo_da_pasta(PASTA).unwrap()
    }

    fn pdf() -> Grupo {
        do_pdf(EFD, &arquivo()).unwrap()
    }

    fn perguntas_do_pdf(g: &Grupo) -> Vec<&Pergunta> {
        g.capitulos.iter().flat_map(|c| &c.perguntas).collect()
    }

    #[test]
    fn link_de_homologacao_vira_destaque_e_o_texto_modelo_sai() {
        let ct = pagina("CT-e");
        assert_eq!(ct.len(), 10);
        assert_eq!(ct[0].0, "O que é o CT-e?");
        assert!(
            ct.iter()
                .any(|(p, _)| p == "O que é a inutilização de número do CT-e?")
        );
        let destda = pagina("DeSTDA");
        assert_eq!(destda.len(), 10);
        assert!(
            destda
                .iter()
                .any(|(p, _)| p == "A declaração pode ser retificada?")
        );
        for (p, r) in ct.iter().chain(&destda) {
            assert!(!p.contains("sanf") && !r.contains("sanf"), "{p}");
            assert!(!r.contains("portal-h"), "{p}");
        }
    }

    #[test]
    fn negrito_sem_interrogacao_fica_na_resposta() {
        let nfce = pagina("NFC-e");
        assert_eq!(nfce.len(), 23);
        let (_, r) = nfce
            .iter()
            .find(|(p, _)| {
                p == "Quais os tipos de documentos fiscais em papel que a NFC-e substitui?"
            })
            .unwrap();
        assert!(r.starts_with("A NFC-e substitui:\nA nota fiscal de venda a consumidor"));
        assert_eq!(pagina("NF-e").len(), 20);
    }

    #[test]
    fn reparos_no_html() {
        assert_eq!(
            reparos(r#"<p><a href="https://portal-h.sefaz.pi.gov.br/x?q=1">O que é?</a></p>"#),
            "<p><strong>O que é?</strong></p>"
        );
        assert_eq!(
            reparos("T&iacute;tulo da sanfoO que &eacute; X?na"),
            "O que &eacute; X?"
        );
        assert_eq!(reparos("Pode?Título da sanfona"), "Pode?");
        // Outro link fica.
        let fora = r#"<a href="https://www.sefaz.pi.gov.br/">site</a>"#;
        assert_eq!(reparos(fora), fora);
    }

    #[test]
    fn pagina_sem_secao_ou_sem_pergunta_e_erro() {
        assert!(
            erro(da_pagina(
                "<section class=\"secao\"><h3>Apresentação</h3></section>"
            ))
            .contains("0 seções")
        );
        let vazia = r#"<section class="secao"><h3>Perguntas Frequentes X</h3><div class="conteudo-html"><p>Nada.</p></div></section>"#;
        assert!(erro(da_pagina(vazia)).contains("não tem pergunta"));
        let sem_resposta = r#"<section class="secao"><h3>Perguntas Frequentes X</h3><div class="conteudo-html"><p><strong>Pode?</strong></p></div></section>"#;
        assert!(erro(da_pagina(sem_resposta)).contains("sem resposta"));
        let duas = format!("{vazia}{vazia}");
        assert!(erro(da_pagina(&duas)).contains("2 seções"));
    }

    #[test]
    fn link_da_pagina_e_o_friendly_url() {
        let docs = documentos(BUSCA).unwrap();
        let nfe = docs.iter().find(|d| d.nome().starts_with("NF-e")).unwrap();
        assert_eq!(
            nfe.link(),
            "https://portal.sefaz.pi.gov.br/nf-e-nota-fiscal-eletronica"
        );
        assert!(erro(documentos(r#"{"hits":{"total":9,"hits":[]}}"#)).contains("achou 9"));
    }

    #[test]
    fn um_pdf_so_na_pasta() {
        let a = arquivo();
        assert_eq!(a.titulo, "Perguntas Frequentes EFD ICMS IPI – Versão 1.0");
        assert_eq!(
            a.url,
            "https://portal.sefaz.pi.gov.br/api/1/main/_ecm/_defaultFile/www/zu5a3chxym/"
        );
        let item = r#"{"isFile":true,"title":"V.pdf","fileName":"v.pdf","fileUrl":"https://x/"}"#;
        assert!(
            erro(arquivo_da_pasta(&format!(r#"{{"items":[{item},{item}]}}"#))).contains("2 PDFs")
        );
        assert!(erro(arquivo_da_pasta(r#"{"items":[]}"#)).contains("0 PDFs"));
    }

    #[test]
    fn pdf_pelo_sumario() {
        let g = pdf();
        assert_eq!(
            g.titulo,
            "Perguntas Frequentes EFD ICMS IPI – Versão 1.0 (Novembro de 2021)"
        );
        let ps = perguntas_do_pdf(&g);
        assert_eq!(ps.len(), 35);
        assert_eq!(g.capitulos.len(), 6);
        assert_eq!(g.capitulos[0].titulo, "CADASTRO");
        assert_eq!(
            g.capitulos
                .iter()
                .map(|c| c.perguntas.len())
                .collect::<Vec<_>>(),
            [8, 11, 2, 9, 3, 2]
        );
        // As do pontilhado curto ("fazer? . 5", "(bloco H)?.. 7") e as que começam no topo da página.
        for n in ["1.4 ", "1.5.4 ", "2.1 ", "2.2 ", "1.5.2 ", "2.7 ", "4.9 "] {
            assert!(ps.iter().any(|p| p.pergunta.starts_with(n)), "{n}");
        }
        assert!(ps.iter().all(|p| p.pergunta.ends_with('?')));
    }

    #[test]
    fn trilha_com_o_subgrupo_e_link_com_a_pagina() {
        let g = pdf();
        let ps = perguntas_do_pdf(&g);
        let p = |n: &str| ps.iter().find(|p| p.pergunta.starts_with(n)).unwrap();
        assert_eq!(
            p("1.5.2 ").trilha,
            "Perguntas Frequentes EFD ICMS IPI – Versão 1.0 (Novembro de 2021) | CADASTRO | Certificado Digital"
        );
        assert_eq!(
            p("2.1 ").trilha,
            "Perguntas Frequentes EFD ICMS IPI – Versão 1.0 (Novembro de 2021) | REGISTROS EFD"
        );
        assert!(p("1.1 ").url.ends_with("/zu5a3chxym/#page=4"));
        assert!(p("1.5.2 ").url.ends_with("#page=6"));
    }

    #[test]
    fn paragrafos_do_layout() {
        let g = pdf();
        let ps = perguntas_do_pdf(&g);
        let p = |n: &str| ps.iter().find(|p| p.pergunta.starts_with(n)).unwrap();
        // O parágrafo que atravessa a página se junta; o rodapé sai.
        assert!(
            p("1.1 ")
                .resposta
                .contains("atualização cadastral; Cópia do pagamento")
        );
        assert!(ps.iter().all(|p| {
            !p.resposta
                .contains("Secretaria da Fazenda do Estado do Piauí")
        }));
        // O marcador abre linha.
        assert!(
            p("2.2 ")
                .resposta
                .contains("da cobrança:\n▪ Se a CVI se refere a Antecipação Parcial")
        );
        assert_eq!(
            paragrafos(
                [
                    "       Um parágrafo",
                    "que continua.",
                    "▪   Item",
                    "dois.",
                    "",
                    "Novo."
                ]
                .into_iter()
            ),
            "Um parágrafo que continua.\n▪ Item dois.\nNovo."
        );
    }

    #[test]
    fn sumario_que_pula_ou_capa_sem_data_e_erro() {
        let a = arquivo();
        // Sem o subgrupo "1.5", o "1.5.1" vem depois do "1.4".
        let sem_15 = EFD.replacen("1.5       Certificado Digital", "Certificado Digital", 1);
        assert!(erro(do_pdf(&sem_15, &a)).contains("pula"));
        let sem_data = EFD.replacen("Novembro de 2021", "", 1);
        assert!(erro(do_pdf(&sem_data, &a)).contains("data"));
        // A pergunta do sumário que não está no corpo.
        let sem_corpo = EFD.replacen("6.2 Possuía saldo credor", "Possuía saldo credor", 1);
        assert!(erro(do_pdf(&sem_corpo, &a)).contains("não está no corpo"));
    }

    /// Um PDF mínimo no formato do `-layout`: capa, sumário (com pontilhado) e corpo, separados por `\f`.
    fn pdf_sintetico(corpo: &str) -> String {
        format!(
            "Capa\n   Novembro de 2021\n\u{c}Sumário\n1  A........ 2\n  1.1  Grupo ........ 2\n    \
             1.1.1  Primeira? ........ 2\n  1.2  Segunda? ........ 2\n\u{c}{corpo}"
        )
    }

    fn sintetico() -> Arquivo {
        Arquivo {
            titulo: "T".into(),
            url: "u".into(),
        }
    }

    #[test]
    fn subgrupo_vale_ate_o_proximo_item_do_2o_nivel() {
        let g = do_pdf(
            &pdf_sintetico(
                "1 A\n  1.1 Grupo\n    1.1.1 Primeira?\n       Um.\n  1.2 Segunda?\n       Dois.\n",
            ),
            &sintetico(),
        )
        .unwrap();
        let ps = perguntas_do_pdf(&g);
        assert_eq!(ps[0].trilha, "T (Novembro de 2021) | A | Grupo");
        assert_eq!(ps[1].trilha, "T (Novembro de 2021) | A");
        assert_eq!(ps[1].url, "u#page=3");
    }

    #[test]
    fn linha_de_resposta_que_comeca_pelo_numero_seguinte_nao_e_cabecalho() {
        let g = do_pdf(
            &pdf_sintetico(
                "1 A\n  1.1 Grupo\n    1.1.1 Primeira?\n       Vale\n1.2 vezes ao ano.\n  1.2 Segunda?\n       Dois.\n",
            ),
            &sintetico(),
        )
        .unwrap();
        let ps = perguntas_do_pdf(&g);
        assert_eq!(ps[0].resposta, "Vale 1.2 vezes ao ano.");
        assert_eq!(ps[1].pergunta, "1.2 Segunda?");
    }

    #[test]
    fn pergunta_do_pdf_sem_resposta_e_erro() {
        let r = do_pdf(
            &pdf_sintetico("1 A\n  1.1 Grupo\n    1.1.1 Primeira?\n  1.2 Segunda?\n       Dois.\n"),
            &sintetico(),
        );
        assert!(erro(r).contains("sem resposta"));
    }

    #[test]
    fn numeracao_que_pula_um_irmao_e_erro() {
        let e = |n: &str| Entrada {
            numero: n.into(),
            titulo: n.into(),
        };
        assert!(numeracao_continua(&[e("1"), e("1.1"), e("1.2"), e("2"), e("2.1")]).is_ok());
        assert!(erro(numeracao_continua(&[e("1"), e("1.1"), e("1.3")])).contains("pula"));
        assert!(erro(numeracao_continua(&[e("1"), e("3")])).contains("pula"));
    }

    #[test]
    fn resto_do_editor_que_escapa_aos_reparos_e_erro() {
        let html = r#"<section class="secao"><h3>Perguntas Frequentes X</h3><div class="conteudo-html"><p><strong>Pode?</strong></p><p>Sim. Título da sanfoX</p></div></section>"#;
        assert!(erro(da_pagina(html)).contains("resto do editor"));
    }

    #[test]
    fn artigo_nas_pastas_de_faq_vazias_e_aviso() {
        assert_eq!(
            aviso_pastas(r#"{"hits":{"total":0,"hits":[]}}"#).unwrap(),
            None
        );
        assert!(
            aviso_pastas(r#"{"hits":{"total":2,"hits":[]}}"#)
                .unwrap()
                .unwrap()
                .contains("2 artigo(s)")
        );
    }

    #[test]
    fn cabecalho_so_com_o_titulo_do_sumario() {
        assert!(comeca_igual("CADASTRO", "CADASTRO"));
        assert!(comeca_igual(
            "A minha Inscrição Estadual foi",
            "A minha Inscrição Estadual foi suspensa"
        ));
        assert!(!comeca_igual("dias úteis para", "REGISTROS EFD"));
    }

    #[test]
    fn arvore_paginas_e_o_pdf_por_capitulo() {
        let docs = documentos(BUSCA).unwrap();
        let nfe = docs.iter().find(|d| d.nome().starts_with("NF-e")).unwrap();
        let pagina = Grupo {
            titulo: nfe.nome(),
            url: nfe.link(),
            capitulos: vec![Capitulo {
                titulo: String::new(),
                url: nfe.link(),
                perguntas: Vec::new(),
            }],
        };
        let t = montar_arvore(&[pagina, pdf()]);
        assert_eq!(t.children.len(), 2);
        assert!(t.children[0].children.is_empty());
        assert_eq!(t.children[1].children.len(), 6);
        assert_eq!(t.children[1].children[0].faq_items.len(), 8);
        assert!(t.children[1].children[0].url.ends_with("#page=4"));
    }

    #[test]
    fn colisao_e_piso() {
        let g = pdf();
        assert!(sem_colisao(std::slice::from_ref(&g)).is_ok());
        let dobro = [pdf(), pdf()];
        assert!(erro(sem_colisao(&dobro)).contains("duas vezes"));
        assert!(piso(PISO_PDF, PISO_PDF, "no PDF").is_ok());
        assert!(erro(piso(PISO_PDF - 1, PISO_PDF, "no PDF")).contains("piso"));
    }
}
