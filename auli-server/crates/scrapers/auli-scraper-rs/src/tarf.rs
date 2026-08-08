//! tarf — Acórdãos do **TARF** (Tribunal Administrativo de Recursos Fiscais) do RS, no portal
//! `tarf.sefaz.rs.gov.br` (Angular SPA "Legislacao 3.0"). Coleção irmã de `pareceres` dentro da
//! entidade rs; materializa a árvore `data/rs/docs/tarf/*.md` no formato dos pareceres.
//!
//! ⚠️ ROBOTS/ACESSO: em 08/08/2026 o `https://tarf.sefaz.rs.gov.br/robots.txt` responde
//! (HTTP 200) exatamente:
//!
//! ```text
//! User-Agent: *
//! Disallow: /
//!
//! User-Agent: MS Search 6.0 Robot
//! Disallow: /
//! ```
//!
//! É o mesmo `Disallow: /` do portal de legislação da SEFAZ-RS, cuja coleta (ver
//! [`crate::pareceres`]) foi autorizada pelo mantenedor — a própria SEFAZ-RS, de quem o TARF é
//! órgão julgador e a quem este scraper serve. Os acórdãos são **documentos públicos** publicados
//! no portal, e a coleta atende demanda institucional do NAVI. UA institucional AuliBot +
//! cortesia de 1 s entre requisições + cache agressivo (uma re-coleta não repete rede).
//!
//! FONTE — API JSON, sem headless e sem HTML server-rendered na listagem (D-TARF-2):
//! - Listagem `GET /api/PesquisaDocumentos?TamanhoPagina=500&NumeroPagina=N&grupos=14543`
//!   (`grupos` minúsculo; `NumeroPagina` começa em 1). Envelope `{ totalItens, resultado[] }`;
//!   o campo `ementa` do item **vem sempre vazio** — a ementa real está no corpo.
//! - Corpo `GET /api/documentos/{id}/DocumentoHtml?NumeroPagina=N&TamanhoPagina=4000`, que
//!   **pagina dispositivos** (mesmo envelope). Não é seguro assumir 1 dispositivo: o 475/19 tem
//!   dois, e o primeiro é um stub de 14 caracteres (D-TARF-5).
//!
//! DOIS LAYOUTS DE CABEÇALHO (medido em amostra estratificada por ano, 103 documentos):
//! - **moderno** (~34%, 2015+): a `<tr>` rotulada `EMENTA:` traz a ementa sozinha na célula
//!   (máximo 1 parágrafo em toda a amostra) e a fundamentação vem nas `<tr>` SEGUINTES, de rótulo
//!   vazio.
//! - **antigo** (~66%, 2005–2014 e 2019): export do Word (`MsoNormalTable`), em que a primeira
//!   `<table>` é só um cabeçalho `RECURSO Nº | ACÓRDÃO Nº` e a `EMENTA:` mora numa tabela
//!   posterior, com ementa e fundamentação empacotadas na MESMA célula (mediana 5 parágrafos).
//!
//! Daí a regra única de [`extrair`], que atende os dois sem ramificar por layout: varre **todas**
//! as tabelas até achar a linha rotulada `EMENTA`; a ementa é o **primeiro parágrafo** da célula e
//! a fundamentação é o resto da célula mais as linhas seguintes de rótulo vazio. No layout moderno
//! "primeiro parágrafo" e "célula inteira" coincidem (a célula só tem um); no antigo é o que separa
//! um `assunto` telegráfico (mediana 144 caracteres) de um que arrastaria a fundamentação inteira
//! (mediana 764, máximo 21.658).

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::LazyLock;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use auli_contract::mddoc::{self, DocHeader};
use auli_scraper_kit::{
    build_agent, cache, clean,
    http::{GetOpts, get_string},
};
use regex::Regex;
use serde::Deserialize;

const API_BASE: &str = "https://tarf.sefaz.rs.gov.br/api";
/// Deep link do documento na SPA — concatenação pura, sem requisição extra (D-TARF-4).
const DEEP_LINK_BASE: &str = "https://tarf.sefaz.rs.gov.br/documento/";
/// Único grupo no escopo: "ACÓRDÃOS A PARTIR DE 2005" (D-TARF-2).
const GRUPO_ACORDAOS: &str = "14543";
/// Árvore de documentos (G5): um `.md` por acórdão inédito.
const DOCS_DIR: &str = "../data/rs/docs/tarf";
/// Namespace de cache da frota: `<CACHE_BASE>/cache/<CACHE_KIND>`, irmão de `cache/pareceres`.
const CACHE_BASE: &str = "../data/rs/raw";
const CACHE_KIND: &str = "tarf";
/// Onde os documentos não emitidos são registrados para decisão humana (D-TARF-6).
const ANOMALIAS: &str = "../data/rs/raw/tarf-anomalias.txt";
const UA: &str =
    "AuliBot/0.1 (+https://github.com/oxschellen/auli; carlos.schellenberger@gmail.com)";
const COURTESY: Duration = Duration::from_millis(1000);
/// Aceito pelo portal (46 páginas para os ~22,6k acórdãos).
const TAM_PAGINA_LISTAGEM: usize = 500;
/// Teto validado pelo servidor para o `DocumentoHtml` (aceita 10–4000).
const TAM_PAGINA_DISPOSITIVOS: usize = 4000;
/// De quantos em quantos documentos a árvore é gravada (~3 min de coleta por lote).
const LOTE: usize = 200;
/// Passadas pela listagem antes de desistir de fechar o `totalItens` (ver [`listar`]).
const TENTATIVAS_LISTAGEM: usize = 3;
/// Corpos que podem falhar EM SEQUÊNCIA antes de a coleta desistir: isolado é defeito do
/// documento, sequência é portal fora do ar.
const FALHAS_SEGUIDAS_MAX: usize = 5;

// ---------------------------------------------------------------------------
// Shape da API
// ---------------------------------------------------------------------------

/// Envelope paginado, compartilhado pela listagem e pelo `DocumentoHtml`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Envelope<T> {
    total_itens: usize,
    resultado: Vec<T>,
}

/// Item da listagem. Campos não usados (grupo, área, numero achatado, ementa vazia…) são ignorados.
#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Item {
    id: String,
    titulo: String,
    /// Usada só para desempatar revisões do mesmo acórdão (ver [`dedup_por_slug`]).
    #[serde(default)]
    data_documento: String,
}

impl Item {
    /// `numero` do frontmatter: o título da listagem, normalizado como todo texto raspado da frota.
    /// O colapso de espaço não é cosmético — sem ele `"ACÓRDÃO  061/25"` e `"ACÓRDÃO 061/25"` são
    /// `numero` distintos que disputam o mesmo `.md`, o que o contrato trata como colisão.
    fn numero(&self) -> String {
        clean(&self.titulo)
    }

    fn link(&self) -> String {
        format!("{DEEP_LINK_BASE}{}", self.id)
    }
}

/// Um dispositivo do documento. `texto` é o HTML limpo; o irmão `textoFormatado` traz o mesmo
/// conteúdo embrulhado no `<style>`/`<span>` da classe `tarfHack` do portal — não usamos.
#[derive(Deserialize)]
struct Dispositivo {
    texto: String,
}

// ---------------------------------------------------------------------------
// Coleta
// ---------------------------------------------------------------------------

pub fn run(use_cache: bool) -> Result<()> {
    let agent = build_agent(UA, Some(Duration::from_secs(90)));

    // 1) Listagem inteira (barata: 46 GETs) — é re-percorrida a cada rodada, sempre.
    let itens = listar(&agent, use_cache)?;

    // 2) Um `.md` por slug: revisões do mesmo acórdão não podem disputar o arquivo (D-TARF-3).
    let (itens, descartados) = dedup_por_slug(itens);
    if !descartados.is_empty() {
        println!(
            "🔁 {} documento(s) descartado(s) por slug repetido (revisões do mesmo acórdão); \
             registrados em {ANOMALIAS}.",
            descartados.len()
        );
    }

    // 3) Incremental (D-TARF-8): o corpo só é buscado para quem ainda não tem `.md` na árvore.
    let dir = Path::new(DOCS_DIR);
    let ineditos: Vec<Item> = itens
        .iter()
        .filter(|i| {
            !dir.join(format!("{}.md", mddoc::slug(&i.numero())))
                .exists()
        })
        .cloned()
        .collect();
    println!(
        "📇 {} acórdãos na listagem; {} inéditos a coletar ({} já na árvore).",
        itens.len(),
        ineditos.len(),
        itens.len() - ineditos.len()
    );

    // 4) Corpo + extração, emitindo a cada LOTE documentos.
    //
    // A emissão é incremental de propósito. A carga inicial são ~22,6 mil corpos a 1 req/s (~6 h):
    // guardar tudo para gravar no fim significaria perder a rodada inteira numa queda às 5 h — e
    // segurar os corpos todos em memória. Gravando por lote, o "existe ⇒ pula" da rodada seguinte
    // retoma de onde parou, que é o resume prometido em D-TARF-8.
    let total = ineditos.len();
    let mut lote: Vec<(DocHeader, Option<String>, String)> = Vec::with_capacity(LOTE);
    let (mut criados, mut pulados, mut fallbacks, mut sem_ementa) =
        (0usize, 0usize, 0usize, 0usize);
    let (mut falhas, mut falhas_seguidas) = (0usize, 0usize);
    reiniciar_anomalias(&descartados)?;
    for (i, item) in ineditos.iter().enumerate() {
        let numero = item.numero();
        // Um documento cujo corpo o portal não entrega não derruba a coleta: vira anomalia, como
        // o sem-ementa. Observado no `001afa1b-9166-4d11-8431-08dc11d4a7ca`, cujo `DocumentoHtml`
        // pendura a conexão até o timeout enquanto o resto do portal responde em ~1 s. Com o `?`
        // aqui, esse único documento mataria as 6 h de coleta — e mataria de novo a cada rodada,
        // porque o "existe ⇒ pula" o mantém inédito para sempre.
        let bruto = match corpo_bruto(&agent, &item.id, use_cache) {
            Ok(bruto) => {
                falhas_seguidas = 0;
                bruto
            }
            Err(e) => {
                falhas_seguidas += 1;
                falhas += 1;
                eprintln!("⚠️  corpo indisponível: {numero} ({}) — {e}", item.id);
                anexar_anomalias(&[format!(
                    "FALHA-CORPO\t{numero}\t{}\t{}",
                    item.id,
                    item.link()
                )])?;
                // Falhas isoladas são do documento; uma sequência delas é do portal — e aí
                // continuar só produziria uma árvore com buracos e um relatório otimista.
                if falhas_seguidas >= FALHAS_SEGUIDAS_MAX {
                    gravar(dir, &mut lote, &mut criados, &mut pulados)?;
                    bail!(
                        "{FALHAS_SEGUIDAS_MAX} corpos seguidos falharam (último: {}) — portal \
                         indisponível? {criados} documento(s) já gravado(s); a próxima rodada \
                         retoma de onde parou.",
                        item.id
                    );
                }
                continue;
            }
        };
        let Some(extraido) = extrair(&bruto) else {
            eprintln!("⚠️  sem linha EMENTA: {numero} ({})", item.id);
            sem_ementa += 1;
            anexar_anomalias(&[format!(
                "SEM-EMENTA\t{numero}\t{}\t{}",
                item.id,
                item.link()
            )])?;
            continue;
        };
        let sinopse = if extraido.fundamentacao.is_empty() {
            fallbacks += 1;
            extraido.ementa.clone() // D-TARF-6: sem fundamentação, a sinopse é a própria ementa
        } else {
            extraido.fundamentacao.join("\n\n")
        };
        lote.push((
            DocHeader {
                numero,
                assunto: extraido.ementa,
                link: item.link(),
                // `sinopse_info` registra proveniência LLM, que aqui não existe: a sinopse é do
                // próprio acórdão. E `docs/tarf/` está fora do alcance do passo
                // `auli-collections <id> sinopse` (que varre só `docs/pareceres/`), então o
                // documento nunca é confundido com um pendente.
                sinopse_info: None,
            },
            Some(sinopse),
            html_para_texto(&bruto),
        ));
        if lote.len() >= LOTE {
            gravar(dir, &mut lote, &mut criados, &mut pulados)?;
            println!("   {}/{total} — {criados} gravados.", i + 1);
        }
    }
    // O resto do lote, fora do laço: se o último documento for anomalia, o `continue` acima pula
    // o flush, e sem isto o último lote parcial se perderia.
    gravar(dir, &mut lote, &mut criados, &mut pulados)?;

    // 5) Relatório final, no padrão da frota.
    println!(
        "✅ árvore {}: {criados} novo(s), {pulados} já existente(s) (pulados). \
         {fallbacks} com sinopse = ementa (sem fundamentação no cabeçalho); \
         {sem_ementa} sem ementa, {falhas} com corpo indisponível e {} descartado(s) por slug \
         repetido — todos em {ANOMALIAS}.",
        dir.display(),
        descartados.len()
    );
    Ok(())
}

/// Esvazia o lote na árvore, acumulando os contadores. No-op se o lote está vazio.
fn gravar(
    dir: &Path,
    lote: &mut Vec<(DocHeader, Option<String>, String)>,
    criados: &mut usize,
    pulados: &mut usize,
) -> Result<()> {
    if lote.is_empty() {
        return Ok(());
    }
    let (c, p) = mddoc::escrever_lote_se_ausente_com_sinopse(dir, lote)?;
    *criados += c;
    *pulados += p;
    lote.clear();
    Ok(())
}

/// Percorre a listagem inteira do grupo, acumulando por GUID.
///
/// Guarda primária (D-TARF-3), sempre dinâmica: os GUIDs únicos coletados têm que bater com o
/// `totalItens` que o próprio portal informa — nunca um total hardcoded.
///
/// A paginação do portal **não é estável**: a listagem vem ordenada por atualização, então um
/// acórdão editado durante os ~46 s da varredura salta para a primeira página e empurra os demais,
/// fazendo um item de fronteira ser visto duas vezes e outro nenhuma (observado: 22.588 de 22.590).
/// Por isso a varredura é repetida enquanto faltar item, acumulando no mesmo mapa — a repetição
/// converge porque cada passada tem um recorte diferente. O `bail!` continua lá para quando nem
/// assim fechar: faltar documento em silêncio é o que não pode.
fn listar(agent: &ureq::Agent, use_cache: bool) -> Result<Vec<Item>> {
    let mut vistos: HashSet<String> = HashSet::new();
    let mut itens: Vec<Item> = Vec::new();
    let mut total_itens = 0usize;

    for tentativa in 1..=TENTATIVAS_LISTAGEM {
        total_itens = percorrer(agent, use_cache, &mut vistos, &mut itens)?;
        if itens.len() >= total_itens {
            break;
        }
        eprintln!(
            "⚠️  listagem: {} de {total_itens} após a passada {tentativa}/{TENTATIVAS_LISTAGEM} \
             (paginação instável); repassando.",
            itens.len()
        );
    }

    if itens.len() < total_itens {
        bail!(
            "listagem incompleta: {} GUIDs únicos coletados em {TENTATIVAS_LISTAGEM} passadas, \
             mas o portal informa totalItens={total_itens}",
            itens.len()
        );
    }
    if itens.len() > total_itens {
        // Acórdão publicado no meio da varredura: superset, não truncamento — segue o jogo.
        println!(
            "ℹ️  {} acórdãos vistos para um totalItens={total_itens}: o acervo cresceu durante a \
             varredura.",
            itens.len()
        );
    }
    Ok(itens)
}

/// Uma passada completa pela paginação, acrescentando ao acumulador o que ainda não foi visto.
/// Devolve o `totalItens` informado na última página lida.
fn percorrer(
    agent: &ureq::Agent,
    use_cache: bool,
    vistos: &mut HashSet<String>,
    itens: &mut Vec<Item>,
) -> Result<usize> {
    let mut pagina = 1usize;
    // Sem valor inicial de propósito: quem manda é o `totalItens` que o portal devolve na 1ª
    // página, e toda saída do laço acontece depois de lê-lo.
    let mut total_itens;

    loop {
        let url = format!(
            "{API_BASE}/PesquisaDocumentos?TamanhoPagina={TAM_PAGINA_LISTAGEM}\
             &NumeroPagina={pagina}&grupos={GRUPO_ACORDAOS}"
        );
        // Live: sempre fresca. A listagem é o que traz acórdãos novos — servi-la do cache
        // congelaria o acervo na primeira coleta e o incremental nunca mais acharia nada.
        let corpo = buscar(agent, &url, use_cache, Cache::Ignorar)?;
        let envelope: Envelope<Item> = serde_json::from_str(&corpo)
            .with_context(|| format!("listagem: JSON inesperado na página {pagina}"))?;
        total_itens = envelope.total_itens;
        if envelope.resultado.is_empty() {
            break;
        }
        for item in envelope.resultado {
            if vistos.insert(item.id.clone()) {
                itens.push(item);
            }
        }
        println!(
            "   página {pagina}: {} acumulados de {total_itens}.",
            itens.len()
        );
        if itens.len() >= total_itens {
            break;
        }
        pagina += 1;
    }
    Ok(total_itens)
}

/// Garante **um item por slug**, mantendo o de `dataDocumento` mais recente (desempate pelo GUID,
/// para a escolha não depender da ordem em que o portal devolveu a listagem).
///
/// O título do acórdão não é identidade: 60 números aparecem em 2+ GUIDs, e conferindo o conteúdo
/// são revisões/republicações do mesmo acórdão (num caso, byte a byte idênticas). Sem este passo o
/// lote ou aborta (dois `numero` textualmente distintos gerando o mesmo `.md`) ou perde documentos
/// em silêncio. Devolve também a linha de registro de cada descartado, para o arquivo de anomalias:
/// a decisão sobre eles é humana.
fn dedup_por_slug(itens: Vec<Item>) -> (Vec<Item>, Vec<String>) {
    let mut melhor: HashMap<String, Item> = HashMap::new();
    let mut descartados: Vec<String> = Vec::new();
    let mut registrar = |perdedor: &Item| {
        descartados.push(format!(
            "SLUG-DUPLICADO\t{}\t{}\t{}",
            perdedor.numero(),
            perdedor.id,
            perdedor.link()
        ));
    };
    for item in itens {
        let slug = mddoc::slug(&item.numero());
        match melhor.get(&slug) {
            None => {
                melhor.insert(slug, item);
            }
            Some(atual) => {
                let chave = |i: &Item| (i.data_documento.clone(), i.id.clone());
                if chave(&item) > chave(atual) {
                    registrar(atual);
                    melhor.insert(slug, item);
                } else {
                    registrar(&item);
                }
            }
        }
    }
    let mut itens: Vec<Item> = melhor.into_values().collect();
    itens.sort_by(|a, b| a.id.cmp(&b.id)); // ordem estável, independente do HashMap
    (itens, descartados)
}

/// HTML bruto do documento: todos os dispositivos, na ordem devolvida, concatenados.
///
/// D-TARF-5: `totalItens` do envelope manda. Truncar em silêncio é proibido — o 475/19 mostra por
/// quê: seu primeiro dispositivo é um stub de 14 caracteres e o acórdão inteiro está no segundo.
fn corpo_bruto(agent: &ureq::Agent, id: &str, use_cache: bool) -> Result<String> {
    let mut partes: Vec<String> = Vec::new();
    let mut pagina = 1usize;
    loop {
        let url = format!(
            "{API_BASE}/documentos/{id}/DocumentoHtml\
             ?NumeroPagina={pagina}&TamanhoPagina={TAM_PAGINA_DISPOSITIVOS}"
        );
        let corpo = buscar(agent, &url, use_cache, Cache::Preferir)?;
        let envelope: Envelope<Dispositivo> = serde_json::from_str(&corpo)
            .with_context(|| format!("corpo de {id}: JSON inesperado na página {pagina}"))?;
        let nesta = envelope.resultado.len();
        partes.extend(envelope.resultado.into_iter().map(|d| d.texto));
        if partes.len() >= envelope.total_itens || nesta == 0 {
            if partes.len() < envelope.total_itens {
                bail!(
                    "documento {id}: portal informa {} dispositivos mas devolveu {}",
                    envelope.total_itens,
                    partes.len()
                );
            }
            break;
        }
        pagina += 1;
    }
    Ok(partes.join("\n"))
}

/// Como uma requisição trata o cache no modo live. Sob `--usecache` a distinção some: ali tudo vem
/// do cache, ou a coleta erra.
#[derive(Clone, Copy, PartialEq)]
enum Cache {
    /// Corpo do documento: imutável para nós (D-TARF-8), então o cache manda.
    Preferir,
    /// Listagem: muda a cada publicação, então vai à rede — mas ainda grava o cache, para o
    /// `--usecache` conseguir reproduzir a coleta inteira offline.
    Ignorar,
}

/// GET com cache. Cortesia só em requisição real: cache hit não dorme (como o resto da frota).
fn buscar(agent: &ureq::Agent, url: &str, use_cache: bool, modo: Cache) -> Result<String> {
    if (modo == Cache::Preferir || use_cache)
        && let Some(cached) = cache::read_or_bail(CACHE_BASE, CACHE_KIND, url, use_cache)?
    {
        return Ok(cached);
    }
    let opts = GetOpts {
        log_prefix: "RS-tarf",
        accept: Some("application/json"),
        ..GetOpts::default()
    };
    let corpo = get_string(agent, url, &opts)?;
    cache::write(CACHE_BASE, CACHE_KIND, url, &corpo);
    sleep(COURTESY);
    Ok(corpo)
}

/// Recomeça o arquivo de anomalias com os descartes por slug, que já são conhecidos antes de
/// qualquer requisição de corpo.
///
/// É reescrita, não acréscimo: o conteúdo é inteiramente derivado da rodada — os descartes vêm da
/// listagem e os "sem ementa" nunca viram `.md`, então seguem inéditos e são recalculados sempre.
/// Acrescentar só empilharia as mesmas linhas a cada re-execução.
fn reiniciar_anomalias(descartes: &[String]) -> Result<()> {
    if let Some(pai) = Path::new(ANOMALIAS).parent() {
        std::fs::create_dir_all(pai)?;
    }
    let cabecalho =
        "# tag\tnumero\tid\tlink — gerado por `auli-scraper-rs tarf`; decisão humana.\n";
    std::fs::write(ANOMALIAS, format!("{cabecalho}{}", linhas(descartes)))?;
    Ok(())
}

/// Acrescenta anomalias encontradas durante a coleta, para que uma rodada interrompida ainda deixe
/// registro do que já achou.
fn anexar_anomalias(novas: &[String]) -> Result<()> {
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new().append(true).open(ANOMALIAS)?;
    f.write_all(linhas(novas).as_bytes())?;
    Ok(())
}

fn linhas(ls: &[String]) -> String {
    ls.iter().map(|l| format!("{l}\n")).collect()
}

// ---------------------------------------------------------------------------
// Extração do cabeçalho (D-TARF-6)
// ---------------------------------------------------------------------------

static RE_TABELA: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)<table[^>]*>(.*?)</table>").unwrap());
static RE_LINHA: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)<tr[^>]*>(.*?)</tr>").unwrap());
static RE_CELULA: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)<t[dh][^>]*>(.*?)</t[dh]>").unwrap());
static RE_FIM_PARAGRAFO: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?is)</p\s*>").unwrap());
static RE_ROTULO_EMENTA: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^ementa\s*:?$").unwrap());

/// O que o cabeçalho do acórdão fornece ao documento: a ementa (vira `assunto`) e a fundamentação
/// resumida (vira `## sinopse`).
struct Extraido {
    ementa: String,
    fundamentacao: Vec<String>,
}

/// Extrai ementa e fundamentação do cabeçalho tabelado, sobre o HTML BRUTO — a estrutura
/// `<table>/<tr>/<td>` é a âncora, não o texto. Ver a nota de layouts no topo do módulo.
///
/// `None` = nenhuma linha rotulada `EMENTA` em nenhuma tabela ⇒ anomalia; o chamador não emite
/// `.md` (documento sem `assunto` não existe) e registra o caso.
fn extrair(html: &str) -> Option<Extraido> {
    for tabela in RE_TABELA.captures_iter(html) {
        let linhas: Vec<&str> = RE_LINHA
            .captures_iter(&tabela[1])
            .map(|c| c.get(1).unwrap().as_str())
            .collect();
        for (i, linha) in linhas.iter().enumerate() {
            let cels = celulas(linha);
            let Some(rotulo) = cels.first() else {
                continue;
            };
            if !RE_ROTULO_EMENTA.is_match(&texto_de(rotulo)) {
                continue;
            }
            let mut ps: Vec<String> = cels[1..].iter().flat_map(|c| paragrafos(c)).collect();
            if ps.is_empty() {
                return None; // rótulo EMENTA com célula vazia — tão inútil quanto não ter o rótulo
            }
            let ementa = ps.remove(0);
            let mut fundamentacao = ps;
            // Layout moderno: a fundamentação continua nas linhas seguintes de rótulo vazio, até
            // uma linha com rótulo ou o fim da tabela.
            for seguinte in &linhas[i + 1..] {
                let cels = celulas(seguinte);
                let Some(rotulo) = cels.first() else {
                    continue;
                };
                if !texto_de(rotulo).is_empty() {
                    break;
                }
                for c in &cels[1..] {
                    fundamentacao.extend(paragrafos(c));
                }
            }
            return Some(Extraido {
                ementa,
                fundamentacao,
            });
        }
    }
    None
}

/// HTML de cada célula de uma linha.
fn celulas(linha: &str) -> Vec<&str> {
    RE_CELULA
        .captures_iter(linha)
        .map(|c| c.get(1).unwrap().as_str())
        .collect()
}

/// Texto de um fragmento, em linha única: tags fora, entidades decodificadas, espaços colapsados.
fn texto_de(html: &str) -> String {
    clean(&decode_entidades(&sem_tags(html)))
}

/// Parágrafos não-vazios de uma célula. Sem `<p>`, a célula inteira conta como um parágrafo.
fn paragrafos(celula: &str) -> Vec<String> {
    RE_FIM_PARAGRAFO
        .split(celula)
        .map(texto_de)
        .filter(|p| !p.is_empty())
        .collect()
}

// ---------------------------------------------------------------------------
// HTML → texto (D-TARF-7)
// ---------------------------------------------------------------------------

static RE_STYLE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)<style[^>]*>.*?</style>").unwrap());
static RE_SCRIPT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)<script[^>]*>.*?</script>").unwrap());
static RE_COMENTARIO: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)<!--.*?-->").unwrap());
static RE_BLOCO: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?is)</(p|div|tr|table|h[1-6]|li|blockquote)>|<br\s*/?>").unwrap()
});
static RE_TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)<[^>]+>").unwrap());
static RE_ENTIDADE_NUM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"&#(x?)([0-9A-Fa-f]+);").unwrap());

/// Corpo integral em texto, cabeçalho tabelado incluído: recorrentes, processo e auto de lançamento
/// viram texto pesquisável, sem frontmatter novo (D-TARF-4).
///
/// O `<style>`/`<script>` são removidos por garantia: no campo `texto` que consumimos eles não
/// aparecem (o CSS `tarfHack` do portal vive só no irmão `textoFormatado`), mas a remoção custa duas
/// regexes e impede que uma mudança de campo despeje CSS dentro do `.md`. As imagens embutidas em
/// `data:` URI (o 141/26 tem três, e sozinhas respondem por ~900 KB do documento) saem junto com as
/// tags: base64 não contém `>`, então `<img …>` é sempre uma tag só.
fn html_para_texto(html: &str) -> String {
    let mut s = html.to_string();
    for re in [&*RE_STYLE, &*RE_SCRIPT, &*RE_COMENTARIO] {
        s = re.replace_all(&s, " ").into_owned();
    }
    s = RE_BLOCO.replace_all(&s, "\n").into_owned();
    let texto = decode_entidades(&sem_tags(&s));
    normalizar_linhas(&texto)
}

fn sem_tags(s: &str) -> String {
    RE_TAG.replace_all(s, " ").into_owned()
}

/// Entidades numéricas (`&#160;`, `&#xA0;`) e o punhado de nomeadas que o portal emite — quase toda
/// a pontuação tipográfica do Word. Numéricas primeiro; `&amp;` por último, para não re-decodificar
/// o que já foi decodificado.
fn decode_entidades(s: &str) -> String {
    let s = RE_ENTIDADE_NUM.replace_all(s, |c: &regex::Captures| {
        let base = if &c[1] == "x" { 16 } else { 10 };
        u32::from_str_radix(&c[2], base)
            .ok()
            .and_then(char::from_u32)
            .map(String::from)
            .unwrap_or_else(|| c[0].to_string())
    });
    let mut s = s.into_owned();
    for (ent, ch) in NOMEADAS {
        if s.contains(ent) {
            s = s.replace(ent, ch);
        }
    }
    s
}

const NOMEADAS: &[(&str, &str)] = &[
    ("&nbsp;", " "),
    ("&ldquo;", "“"),
    ("&rdquo;", "”"),
    ("&lsquo;", "‘"),
    ("&rsquo;", "’"),
    ("&ndash;", "–"),
    ("&mdash;", "—"),
    ("&hellip;", "…"),
    ("&sect;", "§"),
    ("&ordm;", "º"),
    ("&ordf;", "ª"),
    ("&deg;", "°"),
    ("&quot;", "\""),
    ("&apos;", "'"),
    ("&lt;", "<"),
    ("&gt;", ">"),
    ("&amp;", "&"),
];

/// Colapsa espaços por linha e limita a uma linha em branco entre parágrafos.
fn normalizar_linhas(s: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut vazia = false;
    for bruta in s.split('\n') {
        let linha = clean(bruta);
        if linha.is_empty() {
            if !vazia && !out.is_empty() {
                out.push(String::new());
            }
            vazia = true;
        } else {
            out.push(linha);
            vazia = false;
        }
    }
    while out.last().is_some_and(|l| l.is_empty()) {
        out.pop();
    }
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 510/24 — layout moderno, aparado depois do início do RELATÓRIO.
    const MODERNO: &str = include_str!("../tests/fixtures/tarf-510-24.html");
    /// 141/26 — moderno, com as variações reais: link para o PDF antes da tabela, três recorrentes,
    /// "RECORRIDo(s): os mesmos", recursos ex officio + voluntário e uma linha de rótulo vazio
    /// ANTES da linha da ementa.
    const MODERNO_VARIANTE: &str = include_str!("../tests/fixtures/tarf-141-26.html");
    /// 1389/12 — layout antigo (Word/MsoNormalTable), íntegro: a primeira tabela é só
    /// `RECURSO Nº | ACÓRDÃO Nº` e a ementa mora numa tabela posterior, junto da fundamentação.
    const ANTIGO: &str = include_str!("../tests/fixtures/tarf-1389-12.html");

    #[test]
    fn moderno_extrai_ementa_e_fundamentacao() {
        let e = extrair(MODERNO).expect("510/24 tem linha EMENTA");
        assert_eq!(
            e.ementa,
            "ICMS. INDUSTRIALIZAÇÃO POR ENCOMENDA. ESTABELECIMENTO SIMULADO. DIFERIMENTO \
             INAPLICÁVEL. OPERAÇÃO INTERESTADUAL TRIBUTADA. SIMULAÇÃO. RECURSO VOLUNTÁRIO. \
             DESPROVIMENTO."
        );
        assert_eq!(
            e.fundamentacao.len(),
            6,
            "5 parágrafos + o dispositivo final"
        );
        assert!(e.fundamentacao[0].starts_with("Para fins da legislação tributária"));
        assert!(
            e.fundamentacao
                .last()
                .unwrap()
                .starts_with("Por unanimidade, não conheceram")
        );
    }

    #[test]
    fn moderno_corpo_tem_cabecalho_e_relatorio_e_nao_vaza_css() {
        let corpo = html_para_texto(MODERNO);
        assert!(
            corpo.contains("RECORRENTE"),
            "cabeçalho vira texto pesquisável"
        );
        assert!(corpo.contains("MOINHOS CONSOLATA LTDA."));
        assert!(corpo.contains("23/1404-0025519-8"), "número do processo");
        assert!(corpo.contains("RELATÓRIO"));
        assert!(!corpo.contains("tarfHack"), "nenhum resíduo de CSS");
        assert!(!corpo.contains('{'), "nenhum resíduo de CSS");
    }

    #[test]
    fn moderno_variante_com_pdf_e_multiplos_recorrentes() {
        let e = extrair(MODERNO_VARIANTE).expect("141/26 tem linha EMENTA");
        assert!(
            e.ementa
                .starts_with("RECURSOs ex officio e VOLUNTÁRIO. ITCD.")
        );
        assert_eq!(e.fundamentacao.len(), 9);
        // A linha de rótulo vazio que aparece ANTES da ementa não é confundida com fundamentação.
        assert!(e.fundamentacao[0].starts_with("A transferência de quotas sociais"));
        let corpo = html_para_texto(MODERNO_VARIANTE);
        assert!(corpo.contains("EPIFANIO GENELCI LOSS"));
        assert!(corpo.contains("os mesmos"), "RECORRIDo(s) sem nome próprio");
    }

    #[test]
    fn antigo_separa_ementa_da_fundamentacao_na_mesma_celula() {
        // O caso que a regra do "primeiro parágrafo" existe para atender: no export do Word a
        // célula da ementa carrega também a fundamentação inteira.
        let e = extrair(ANTIGO).expect("1389/12 tem linha EMENTA (na 2ª tabela)");
        assert_eq!(
            e.ementa,
            "ICMS. TRÂNSITO DE MERCADORIA. DOCUMENTO INIDÔNEO. CARGA. CONFERÊNCIA. DIFERENÇAS \
             ENCONTRADAS A MAIOR E A MENOR. IRREGULARIDADES. CONFIGURAM INFRAÇÃO DE NATUREZA \
             MATERIAL QUALIFICADA prevista no art. 8º, I, \"d\" da Lei Nº 6.537/73 e alterações."
        );
        assert!(
            e.ementa.len() < 300,
            "a ementa não pode arrastar a fundamentação: {} chars",
            e.ementa.len()
        );
        assert_eq!(e.fundamentacao.len(), 6);
        assert!(e.fundamentacao[0].starts_with("O trânsito de mercadorias"));
        assert_eq!(e.fundamentacao.last().unwrap(), "Unânime.");
        // A primeira tabela (RECURSO Nº | ACÓRDÃO Nº) não tem rótulo EMENTA e é ignorada sem drama.
        let corpo = html_para_texto(ANTIGO);
        assert!(corpo.contains("RECURSO Nº 171/12") && corpo.contains("ACÓRDÃO Nº 1389/12"));
    }

    #[test]
    fn sem_fundamentacao_devolve_lista_vazia_para_o_fallback() {
        let html = r#"<table><tr><td><p>EMENTA:</p></td>
            <td><p>TAXA DE SERVIÇOS. RECURSO VOLUNTÁRIO DESPROVIDO.</p></td></tr></table>"#;
        let e = extrair(html).unwrap();
        assert_eq!(e.ementa, "TAXA DE SERVIÇOS. RECURSO VOLUNTÁRIO DESPROVIDO.");
        assert!(
            e.fundamentacao.is_empty(),
            "o chamador usa a ementa como sinopse"
        );
    }

    #[test]
    fn sem_linha_ementa_e_anomalia() {
        let html = r#"<table><tr><td><p>RECURSO Nº 762/07</p></td>
            <td><p>ACÓRDÃO Nº 872/07</p></td></tr></table><p>Corpo sem ementa.</p>"#;
        assert!(extrair(html).is_none());
    }

    #[test]
    fn linha_com_rotulo_encerra_a_fundamentacao() {
        let html = r#"<table>
            <tr><td><p>EMENTA:</p></td><td><p>A EMENTA.</p></td></tr>
            <tr><td><p>&nbsp;</p></td><td><p>Fundamento um.</p></td></tr>
            <tr><td><p>OBSERVAÇÃO:</p></td><td><p>Não é fundamentação.</p></td></tr>
            <tr><td><p>&nbsp;</p></td><td><p>Nem isto.</p></td></tr>
        </table>"#;
        let e = extrair(html).unwrap();
        assert_eq!(e.ementa, "A EMENTA.");
        assert_eq!(e.fundamentacao, vec!["Fundamento um."]);
    }

    #[test]
    fn dedup_mantem_a_revisao_mais_recente_e_registra_as_demais() {
        let item = |id: &str, titulo: &str, data: &str| Item {
            id: id.into(),
            titulo: titulo.into(),
            data_documento: data.into(),
        };
        let itens = vec![
            item("aaa", "ACÓRDÃO 051/24", "2024-01-31T00:00:00"),
            item("bbb", "ACÓRDÃO 051/24", "2024-12-10T00:00:00"),
            // Só o espaço duplo difere: `clean` no numero evita que virem uma colisão de slug.
            item("ccc", "ACÓRDÃO  061/25", "2025-05-13T00:00:00"),
            item("ddd", "ACÓRDÃO 061/25", "2025-06-11T00:00:00"),
            item("eee", "ACÓRDÃO 007/26", "2026-02-02T00:00:00"),
        ];
        let (mantidos, descartados) = dedup_por_slug(itens);
        let ids: Vec<&str> = mantidos.iter().map(|i| i.id.as_str()).collect();
        assert_eq!(ids, vec!["bbb", "ddd", "eee"]);
        assert_eq!(descartados.len(), 2);
        assert!(
            descartados
                .iter()
                .all(|l| l.starts_with("SLUG-DUPLICADO\t"))
        );
        assert!(descartados.iter().any(|l| l.contains("aaa")));
        assert!(descartados.iter().any(|l| l.contains("ccc")));
        // E o lote resultante passa na guarda do contrato (nenhum slug repetido).
        assert!(!mantidos.is_empty());
    }

    #[test]
    fn numero_colapsa_espaco_do_titulo() {
        let i = Item {
            id: "x".into(),
            titulo: "ACÓRDÃO  061/25 ".into(),
            data_documento: String::new(),
        };
        assert_eq!(i.numero(), "ACÓRDÃO 061/25");
        assert_eq!(i.link(), "https://tarf.sefaz.rs.gov.br/documento/x");
    }

    #[test]
    fn style_e_imagem_embutida_nao_vazam_para_o_corpo() {
        // O `textoFormatado` do portal embrulha o mesmo conteúdo num <style> da classe tarfHack.
        let html = concat!(
            "<style>.tarfHack p { text-align:justify; } </style>",
            "<span class=\"tarfHack\"><p>Texto do acórdão.</p>",
            "<img src=\"data:image/png;base64,iVBORw0KGgoAAAANSUhEUg\" width=\"29\" />",
            "<p>Fim.</p></span>"
        );
        let corpo = html_para_texto(html);
        assert_eq!(corpo, "Texto do acórdão.\nFim.");
        assert!(!corpo.contains("tarfHack") && !corpo.contains('{'));
        assert!(!corpo.contains("base64"), "data: URI sai junto com a tag");
    }

    #[test]
    fn entidades_numericas_e_tipograficas() {
        // As que o portal emite de fato (nenhuma entidade de letra acentuada apareceu nos 137
        // documentos amostrados — os acentos vêm em UTF-8 direto; só a pontuação do Word e as
        // numéricas são escapadas).
        assert_eq!(texto_de("a&#160;b"), "a b");
        assert_eq!(
            texto_de("&ldquo;cita&#231;&atilde;o&rdquo;"),
            "“citaç&atilde;o”"
        );
        assert_eq!(texto_de("art. 8&ordm; &ndash; inciso"), "art. 8º – inciso");
        assert_eq!(texto_de("&#x2044; e &#8260;"), "⁄ e ⁄");
        assert_eq!(texto_de("a &amp; b"), "a & b");
    }

    #[test]
    fn documento_emitido_faz_round_trip_com_sinopse() {
        // Fecha o ciclo: o que este módulo produz é parseável pelo contrato e nasce COM sinopse —
        // o que satisfaz por construção a guarda `recusar_pareceres_sem_sinopse` do update.
        let e = extrair(MODERNO).unwrap();
        let header = DocHeader {
            numero: "ACÓRDÃO 510/24".into(),
            assunto: e.ementa,
            link: "https://tarf.sefaz.rs.gov.br/documento/ee87f4bc".into(),
            sinopse_info: None,
        };
        let sinopse = e.fundamentacao.join("\n\n");
        let texto = mddoc::render_doc(&header, Some(&sinopse), &html_para_texto(MODERNO));
        let (h, s, corpo) = mddoc::parse_doc(&texto).unwrap();
        assert_eq!(h, header);
        assert!(s.is_some_and(|s| !s.trim().is_empty()));
        assert!(corpo.contains("RELATÓRIO"));
        assert_eq!(mddoc::slug(&h.numero), "acordao-510-24");
    }
}
