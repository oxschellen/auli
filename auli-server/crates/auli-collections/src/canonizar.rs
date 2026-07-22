//! Subcomando `canonizar` — chave canônica de dispositivos (TAREFA-CANONIZADOR, C1–C4).
//!
//! Segundo incremento do knowledge graph: **determinístico, sem rede, sem LLM**. Lê a saída literal
//! do `extrair` (`data/<id>/extracao/extracao.jsonl`), parseia cada `dispositivo.texto` numa chave
//! canônica hierárquica (`ricms-rs:lI:art27:incX`) e colapsa variantes que só diferem por
//! pontuação/grafia (`RICMS`≡`Regulamento do ICMS`, `nº`≡`n.º`, `§ 2º`≡`§ 2.º`).
//!
//! O literal NUNCA é destruído: cada ocorrência sai com o `texto` original ao lado da chave, e a
//! cauda dura (anáfora `referido artigo 31`, despejo de texto `Art. 32 - ...`, sem norma
//! identificável) vira `canonizavel: false` — o grafo omite o nó, nada é chutado (K2).
//!
//! Saídas irmãs em `data/<id>/extracao/`: `dispositivos.jsonl` (1 linha por ocorrência) e
//! `dispositivos-index.json` (semente do grafo: `canon_key → {display, ocorrencias, variantes,
//! pareceres}`). Derivação pura e idempotente: re-rodar produz bytes idênticos (K8).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::domain::entities::EntityConfig;
use crate::errors::Result;
use crate::extracao::{Dispositivo, extracao_dir};

/// Uma linha lida do `extracao.jsonl` — só os campos que o canonizar usa (ncm/temas ignorados).
#[derive(Deserialize)]
struct EntradaLinha {
    numero: String,
    extracao: EntradaExtracao,
}

#[derive(Deserialize)]
struct EntradaExtracao {
    dispositivos: Vec<Dispositivo>,
}

/// Uma linha do `dispositivos.jsonl` (1 por ocorrência). `texto` = literal preservado (auditoria).
#[derive(Serialize)]
struct SaidaLinha {
    numero: String,
    texto: String,
    canon_key: Option<String>,
    canon_display: Option<String>,
    canonizavel: bool,
}

/// Um nó do `dispositivos-index.json` (semente do grafo).
#[derive(Serialize)]
struct IndiceEntrada {
    display: String,
    ocorrencias: usize,
    variantes: Vec<String>,
    pareceres: Vec<String>,
}

/// Acumulador do índice (conjuntos garantem variantes/pareceres únicos e ordenados = determinismo).
struct Acc {
    display: String,
    ocorrencias: usize,
    variantes: BTreeSet<String>,
    pareceres: BTreeSet<String>,
}

/// Uma citação canonizada: a chave (identidade no grafo) + uma forma humana única por chave.
struct Canon {
    key: String,
    display: String,
}

// ── Regexes (compiladas 1×; reusadas em todas as citações) ────────────────────────────────────

/// Anáfora: a citação aponta para algo dito antes (sem o corpo, não dá para resolver) → cauda (K2).
static ANAFORA: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(referid[oa]|dess[ae]|dest[ae]|seu|sua|mesm[oa]|citad[oa])\b").unwrap()
});
/// Despejo de texto de artigo (`Art. 32 - Assegura-se...`): o modelo copiou o corpo, não a citação.
static TEXTO_ARTIGO: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^art\.?\s*\d+\s*[-–—]\s").unwrap());

static LIVRO: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\blivro\s+([ivxlcdm]+)\b").unwrap());
static APENDICE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bap[êe]ndice\s+([ivxlcdm]+)\b").unwrap());
// Captura o número + sufixo opcional (`38-A`, `1º-K`), tolerando o ordinal entre eles; a
// normalização (strip de `º°.`) mora no `canonizar_texto`.
static ARTIGO: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bart(?:igo|\.)?\s*(\d+(?:[º°.]*-[a-z])?)").unwrap());
static PARAGRAFO: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(?:§|par[áa]grafo)\s*(\d+|único|unico)").unwrap());
static INCISO: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\binciso\s+([ivxlcdm]+)\b").unwrap());
static ALINEA: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?i)\bal[íi]nea\s+["“']?([a-z])"#).unwrap());
static ITEM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(?:sub)?item\s+(\d+(?:\.\d+)*)").unwrap());
// Título/Capítulo/Seção estruturam as Instruções Normativas e os Apêndices do RICMS — sem eles, as
// citações colapsavam TODAS na norma (over-merge, viola K5). Seção pode ser romana ou decimal (`2.0`).
static TITULO: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bt[íi]tulo\s+([ivxlcdm]+)\b").unwrap());
static CAPITULO: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bcap[íi]tulo\s+([ivxlcdm]+)\b").unwrap());
static SECAO: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bse[çc][ãa]o\s+(\d+(?:\.\d+)*|[ivxlcdm]+)\b").unwrap());

/// Normas com identificador `número/ano` — cada uma captura (número, ano). Ordem de tentativa em
/// `detectar_norma` (LC antes de Lei; EC antes de tudo com "constitucional").
static EC: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)emenda constitucional\s+n?[º.°]*\s*(\d+)\s*/\s*(\d+)").unwrap()
});
static LC: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)lei complementar\s+n?[º.°]*\s*(\d+)\s*/\s*(\d+)").unwrap());
static CONVENIO: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)conv[êe]nio\s+icms\s+n?[º.°]*\s*(\d+)\s*/\s*(\d+)").unwrap());
static IN_DRP: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:instru[çc][ãa]o normativa|\bin)\s+drp\s+n?[º.°]*\s*(\d+)\s*/\s*(\d+)")
        .unwrap()
});
/// Decreto (estadual/federal): como a Lei, aceita `nº 52.095/14` OU `nº 56.086, de 13.09.21`.
static DECRETO: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)decreto\s+(?:estadual\s+|federal\s+)?n?[º.°]*\s*([\d.]+)(?:\s*/\s*(\d+)|,?\s*de\s+[\d.]+[./](\d+))").unwrap()
});
/// Lei (federal/estadual): dois formatos — `nº 8.820/89` OU `nº 11.442, de 05.01.07` (ano no fim da data).
static LEI: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\blei\s+(?:federal|estadual)?\s*n?[º.°]*\s*([\d.]+)(?:\s*/\s*(\d+)|,?\s*de\s+[\d.]+[./](\d+))").unwrap()
});
static RESOLUCAO_SF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)resolu[çc][ãa]o do senado federal\s+n?[º.°]*\s*(\d+)\s*/\s*(\d+)").unwrap()
});
static PROTOCOLO: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)protocolo\s+icms\s+n?[º.°]*\s*(\d+)\s*/\s*(\d+)").unwrap());
static PORTARIA: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)portaria\s+n?[º.°]*\s*(\d+)\s*/\s*(\d+)").unwrap());
/// Instrução Normativa genérica (sem DRP): captura o subtipo opcional `RE` (`in-re`) ou nada (`in`).
static IN_GENERIC: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)instru[çc][ãa]o normativa\s+(re\s+)?n?[º.°]*\s*(\d+)\s*/\s*(\d+)").unwrap()
});

/// Colapsa espaços/quebras num espaço só (os literais podem trazer quebras do corpo).
fn colapsa(t: &str) -> String {
    t.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Detecta a norma e devolve `(key, display)`. `None` = sem norma identificável (→ cauda, K5).
/// Ordem importa: RICMS primeiro (56%); LC antes de Lei; normas com número extraem `número/ano`
/// (dígitos verbatim, separadores normalizados). RICMS/RS é hardcode do RS-first (K3).
fn detectar_norma(t: &str) -> Option<(String, String)> {
    let low = t.to_lowercase();
    if low.contains("ricms") || low.contains("regulamento do icms") {
        return Some(("ricms-rs".into(), "RICMS/RS".into()));
    }
    if low.contains("constituição federal") || low.contains("constituicao federal") {
        return Some(("cf".into(), "Constituição Federal".into()));
    }
    if low.contains("código tributário nacional")
        || low.contains("codigo tributario nacional")
        || Regex::new(r"(?i)\bctn\b").unwrap().is_match(t)
    {
        return Some(("ctn".into(), "CTN".into()));
    }
    if let Some(c) = EC.captures(t) {
        let (n, a) = (&c[1], &c[2]);
        return Some((format!("ec:{n}/{a}"), format!("EC nº {n}/{a}")));
    }
    if let Some(c) = LC.captures(t) {
        let (n, a) = (&c[1], &c[2]);
        return Some((format!("lc:{n}/{a}"), format!("LC nº {n}/{a}")));
    }
    if let Some(c) = CONVENIO.captures(t) {
        let (n, a) = (&c[1], &c[2]);
        return Some((
            format!("convenio:{n}/{a}"),
            format!("Convênio ICMS {n}/{a}"),
        ));
    }
    if let Some(c) = PROTOCOLO.captures(t) {
        let (n, a) = (&c[1], &c[2]);
        return Some((
            format!("protocolo:{n}/{a}"),
            format!("Protocolo ICMS {n}/{a}"),
        ));
    }
    if let Some(c) = RESOLUCAO_SF.captures(t) {
        let (n, a) = (&c[1], &c[2]);
        return Some((
            format!("resolucao-sf:{n}/{a}"),
            format!("Resolução do Senado Federal {n}/{a}"),
        ));
    }
    if let Some(c) = PORTARIA.captures(t) {
        let (n, a) = (&c[1], &c[2]);
        return Some((format!("portaria:{n}/{a}"), format!("Portaria nº {n}/{a}")));
    }
    if let Some(c) = IN_DRP.captures(t) {
        let (n, a) = (&c[1], &c[2]);
        return Some((format!("in-drp:{n}/{a}"), format!("IN DRP {n}/{a}")));
    }
    if let Some(c) = IN_GENERIC.captures(t) {
        let (re, n, a) = (c.get(1).is_some(), &c[2], &c[3]);
        let (k, d) = if re { ("in-re", "IN RE") } else { ("in", "IN") };
        return Some((format!("{k}:{n}/{a}"), format!("{d} {n}/{a}")));
    }
    if let Some(c) = DECRETO.captures(t) {
        let n = c[1].replace('.', "");
        let a = c
            .get(2)
            .or_else(|| c.get(3))
            .map(|m| m.as_str())
            .unwrap_or("?");
        return Some((format!("decreto:{n}/{a}"), format!("Decreto nº {n}/{a}")));
    }
    if let Some(c) = LEI.captures(t) {
        let n = c[1].replace('.', "");
        let ano = c
            .get(2)
            .or_else(|| c.get(3))
            .map(|m| m.as_str())
            .unwrap_or("?");
        return Some((format!("lei:{n}/{ano}"), format!("Lei nº {n}/{ano}")));
    }
    None
}

/// Parseia UM literal na sua chave canônica (pura, testável). `None` = cauda dura (K2): anáfora,
/// despejo de texto de artigo, resíduo longo sem estrutura, ou sem norma identificável.
fn canonizar_texto(texto: &str) -> Option<Canon> {
    let t = colapsa(texto);
    if ANAFORA.is_match(&t) || TEXTO_ARTIGO.is_match(&t) || t.chars().count() > 120 {
        return None;
    }
    let (norma_key, norma_disp) = detectar_norma(&t)?;

    // Componentes hierárquicos (todos opcionais). `cap` extrai o grupo 1 e normaliza.
    let cap = |re: &Regex| re.captures(&t).map(|c| c[1].to_string());
    let livro = cap(&LIVRO).map(|s| s.to_uppercase());
    let apendice = cap(&APENDICE).map(|s| s.to_uppercase());
    let titulo = cap(&TITULO).map(|s| s.to_uppercase());
    let capitulo = cap(&CAPITULO).map(|s| s.to_uppercase());
    let secao = cap(&SECAO).map(|s| s.to_uppercase());
    // Normaliza o artigo: remove ordinais (`1º-K`→`1-K`) e sobe a caixa do sufixo.
    let artigo = cap(&ARTIGO).map(|s| {
        s.chars()
            .filter(|c| !matches!(c, 'º' | '°' | '.'))
            .collect::<String>()
            .to_uppercase()
    });
    let paragrafo = cap(&PARAGRAFO).map(|s| if s == "único" { "unico".into() } else { s });
    let inciso = cap(&INCISO).map(|s| s.to_uppercase());
    let alinea = cap(&ALINEA).map(|s| s.to_lowercase());
    let item = cap(&ITEM);

    // Chave (ordem hierárquica de K4): norma:livro:apêndice:título:capítulo:seção:artigo:§:inciso:alínea:item.
    let mut key = norma_key;
    if let Some(v) = &livro {
        key.push_str(&format!(":l{v}"));
    }
    if let Some(v) = &apendice {
        key.push_str(&format!(":ap{v}"));
    }
    if let Some(v) = &titulo {
        key.push_str(&format!(":t{v}"));
    }
    if let Some(v) = &capitulo {
        key.push_str(&format!(":cap{v}"));
    }
    if let Some(v) = &secao {
        key.push_str(&format!(":sec{v}"));
    }
    if let Some(v) = &artigo {
        key.push_str(&format!(":art{v}"));
    }
    if let Some(v) = &paragrafo {
        key.push_str(&format!(":§{v}"));
    }
    if let Some(v) = &inciso {
        key.push_str(&format!(":inc{v}"));
    }
    if let Some(v) = &alinea {
        key.push_str(&format!(":al{v}"));
    }
    if let Some(v) = &item {
        key.push_str(&format!(":it{v}"));
    }

    // Display: forma humana única por chave, reconstruída dos componentes (determinística — não
    // depende de frequência). Ex.: "inc. X, art. 27, Livro I do RICMS/RS".
    let mut partes: Vec<String> = Vec::new();
    if let Some(v) = &inciso {
        partes.push(format!("inc. {v}"));
    }
    if let Some(v) = &alinea {
        partes.push(format!("al. {v}"));
    }
    if let Some(v) = &paragrafo {
        partes.push(if v == "unico" {
            "§ único".into()
        } else {
            format!("§ {v}º")
        });
    }
    if let Some(v) = &item {
        partes.push(format!("item {v}"));
    }
    if let Some(v) = &artigo {
        partes.push(format!("art. {v}"));
    }
    if let Some(v) = &livro {
        partes.push(format!("Livro {v}"));
    }
    if let Some(v) = &apendice {
        partes.push(format!("Apêndice {v}"));
    }
    if let Some(v) = &secao {
        partes.push(format!("Seção {v}"));
    }
    if let Some(v) = &capitulo {
        partes.push(format!("Capítulo {v}"));
    }
    if let Some(v) = &titulo {
        partes.push(format!("Título {v}"));
    }
    let display = if partes.is_empty() {
        norma_disp
    } else {
        format!("{} do {norma_disp}", partes.join(", "))
    };

    Some(Canon { key, display })
}

/// Escrita atômica (`.tmp` + rename): a saída é derivada e reescrita inteira a cada rodada (K8).
fn escrever_atomico(path: &Path, conteudo: &str) -> Result<()> {
    let tmp = PathBuf::from(format!("{}.tmp", path.display()));
    std::fs::write(&tmp, conteudo)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

pub fn run(entity: &EntityConfig) -> Result<()> {
    let dir = extracao_dir(entity)?;
    let in_path = dir.join("extracao.jsonl");
    if !in_path.exists() {
        return Err(format!(
            "saída da extração ausente: {} — rode `auli-collections {} extrair` antes.",
            in_path.display(),
            entity.id
        )
        .into());
    }

    let texto = std::fs::read_to_string(&in_path)?;
    let mut saidas: Vec<SaidaLinha> = Vec::new();
    let mut index: BTreeMap<String, Acc> = BTreeMap::new();

    for (i, l) in texto.lines().enumerate() {
        let l = l.trim();
        if l.is_empty() {
            continue;
        }
        let entrada: EntradaLinha = serde_json::from_str(l)
            .map_err(|e| format!("{}: linha {} malformada ({e})", in_path.display(), i + 1))?;
        for disp in entrada.extracao.dispositivos {
            let canon = canonizar_texto(&disp.texto);
            if let Some(c) = &canon {
                let acc = index.entry(c.key.clone()).or_insert_with(|| Acc {
                    display: c.display.clone(),
                    ocorrencias: 0,
                    variantes: BTreeSet::new(),
                    pareceres: BTreeSet::new(),
                });
                acc.ocorrencias += 1;
                acc.variantes.insert(disp.texto.clone());
                acc.pareceres.insert(entrada.numero.clone());
            }
            saidas.push(SaidaLinha {
                numero: entrada.numero.clone(),
                canon_key: canon.as_ref().map(|c| c.key.clone()),
                canon_display: canon.as_ref().map(|c| c.display.clone()),
                canonizavel: canon.is_some(),
                texto: disp.texto,
            });
        }
    }

    // Ordem estável (K6): por `numero`, depois por `texto` — determinístico e idempotente.
    saidas.sort_by(|a, b| a.numero.cmp(&b.numero).then_with(|| a.texto.cmp(&b.texto)));

    let mut buf = String::new();
    for s in &saidas {
        buf.push_str(&serde_json::to_string(s).map_err(|e| format!("serializando saída: {e}"))?);
        buf.push('\n');
    }
    escrever_atomico(&dir.join("dispositivos.jsonl"), &buf)?;

    let index_out: BTreeMap<String, IndiceEntrada> = index
        .into_iter()
        .map(|(k, a)| {
            (
                k,
                IndiceEntrada {
                    display: a.display,
                    ocorrencias: a.ocorrencias,
                    variantes: a.variantes.into_iter().collect(),
                    pareceres: a.pareceres.into_iter().collect(),
                },
            )
        })
        .collect();
    let json = serde_json::to_string_pretty(&index_out)
        .map_err(|e| format!("serializando índice: {e}"))?;
    escrever_atomico(&dir.join("dispositivos-index.json"), &format!("{json}\n"))?;

    let total = saidas.len();
    let canon = saidas.iter().filter(|s| s.canonizavel).count();
    let cauda = total - canon;
    let chaves = index_out.len();
    println!(
        "🔗 {}: ocorrências {total} | canonizáveis {canon} → {chaves} chaves | cauda {cauda}",
        entity.id
    );
    println!(
        "📄 saída: {} + {}",
        dir.join("dispositivos.jsonl").display(),
        dir.join("dispositivos-index.json").display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(t: &str) -> Option<String> {
        canonizar_texto(t).map(|c| c.key)
    }

    #[test]
    fn colapsa_variantes_ricms_no_mesmo_key() {
        // RICMS ≡ Regulamento do ICMS ≡ (RICMS) — a poeira de alias some.
        let a = key("inciso X do artigo 27 do Livro I do RICMS");
        let b = key("inciso X do artigo 27 do Livro I do Regulamento do ICMS");
        let c = key("inciso X do artigo 27 do Livro I do Regulamento do ICMS (RICMS)");
        assert_eq!(a.as_deref(), Some("ricms-rs:lI:art27:incX"));
        assert_eq!(a, b);
        assert_eq!(a, c);
    }

    #[test]
    fn colapsa_ordinais_e_pontuacao() {
        // § 4º ≡ § 4.º ; nº ≡ n.º
        assert_eq!(
            key("§ 4.º do artigo 46 do Livro I do RICMS"),
            key("§ 4º do artigo 46 do Livro I do RICMS")
        );
        assert_eq!(
            key("inciso VII do § 2º do artigo 155 da Constituição Federal"),
            key("inciso VII do § 2.º do artigo 155 da Constituição Federal")
        );
        assert_eq!(
            key("Emenda Constitucional nº 87/15"),
            key("Emenda Constitucional n.º 87/15")
        );
        assert_eq!(
            key("Emenda Constitucional nº 87/15").as_deref(),
            Some("ec:87/15")
        );
    }

    #[test]
    fn ordem_dos_componentes_na_chave() {
        // cf › artigo › § › inciso (K4).
        assert_eq!(
            key("inciso VII do § 2º do artigo 155 da Constituição Federal").as_deref(),
            Some("cf:art155:§2:incVII")
        );
    }

    #[test]
    fn normas_diversas() {
        assert_eq!(
            key("artigo 166 do Código Tributário Nacional").as_deref(),
            Some("ctn:art166")
        );
        assert_eq!(
            key("inciso I do artigo 106 do CTN").as_deref(),
            Some("ctn:art106:incI")
        );
        assert_eq!(
            key("Lei Complementar nº 123/06").as_deref(),
            Some("lc:123/06")
        );
        // Sufixo de artigo preservado.
        assert_eq!(
            key("artigo 1º-K do Livro III do RICMS").as_deref(),
            Some("ricms-rs:lIII:art1-K")
        );
    }

    #[test]
    fn cauda_dura_vira_none() {
        // Anáfora: sem o corpo não dá para saber a que artigo/norma se refere.
        assert!(key("§ 4º do referido artigo 31").is_none());
        assert!(key("§ 6º do seu Apêndice II").is_none());
        // Despejo de texto de artigo (o modelo copiou o corpo).
        assert!(key("Art. 32 - Assegura-se direito a crédito fiscal presumido:").is_none());
        // Sem norma identificável.
        assert!(key("inciso II do artigo 25-B").is_none());
    }

    #[test]
    fn parseia_lei_nos_dois_formatos() {
        assert_eq!(
            key("artigo 4.º da Lei nº 8.820/89").as_deref(),
            Some("lei:8820/89:art4")
        );
        assert_eq!(
            key("artigo 2.º da Lei Federal nº 11.442, de 05.01.07").as_deref(),
            Some("lei:11442/07:art2")
        );
    }

    #[test]
    fn titulo_capitulo_secao_evitam_over_merge() {
        // Provisões DIFERENTES da MESMA IN não podem colapsar (era o bug do over-merge no RS).
        let a = key("Capítulo LX do Título I da Instrução Normativa DRP nº 45/98");
        let b = key("Seção 2.0 do Capítulo V do Título I da Instrução Normativa DRP n.º 45/98");
        assert_eq!(a.as_deref(), Some("in-drp:45/98:tI:capLX"));
        assert_eq!(b.as_deref(), Some("in-drp:45/98:tI:capV:sec2.0"));
        assert_ne!(
            a, b,
            "capítulos/seções distintos são dispositivos distintos"
        );
    }

    #[test]
    fn normas_ampliadas_saem_da_cauda() {
        assert_eq!(
            key("Protocolo ICMS nº 41/08").as_deref(),
            Some("protocolo:41/08")
        );
        assert_eq!(
            key("Resolução do Senado Federal n.º 13/12").as_deref(),
            Some("resolucao-sf:13/12")
        );
        assert_eq!(
            key("Instrução Normativa nº 045/98").as_deref(),
            Some("in:045/98")
        );
        assert_eq!(
            key("Instrução Normativa RE nº 019/19").as_deref(),
            Some("in-re:019/19")
        );
        // Decreto no formato "de DD.MM.YY" (ano no fim da data).
        assert_eq!(
            key("Decreto nº 56.086, de 13.09.21").as_deref(),
            Some("decreto:56086/21")
        );
    }

    #[test]
    fn run_gera_saidas_deterministicas_e_idempotentes() {
        let base = std::env::temp_dir().join(format!("auli_canon_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("raw")).unwrap();
        std::fs::create_dir_all(base.join("extracao")).unwrap();
        let entity = EntityConfig {
            id: "xx".into(),
            name: "Teste".into(),
            system_prompt: String::new(),
            data_dir: base.join("raw").to_string_lossy().into_owned(),
        };
        // Duas linhas com variantes do MESMO dispositivo + uma cauda dura.
        let l1 = r#"{"numero":"P 1","link":"x","prompt_versao":1,"modelo":"m","gerada_em":"t","extracao":{"dispositivos":[{"texto":"inciso X do artigo 27 do Livro I do RICMS"}],"ncm":[],"temas":[]}}"#;
        let l2 = r#"{"numero":"P 2","link":"x","prompt_versao":1,"modelo":"m","gerada_em":"t","extracao":{"dispositivos":[{"texto":"inciso X do artigo 27 do Livro I do Regulamento do ICMS"},{"texto":"§ 4º do referido artigo 31"}],"ncm":[],"temas":[]}}"#;
        std::fs::write(
            base.join("extracao").join("extracao.jsonl"),
            format!("{l1}\n{l2}\n"),
        )
        .unwrap();

        run(&entity).unwrap();
        let disp =
            std::fs::read_to_string(base.join("extracao").join("dispositivos.jsonl")).unwrap();
        let idx =
            std::fs::read_to_string(base.join("extracao").join("dispositivos-index.json")).unwrap();

        // 3 ocorrências (2 canonizáveis colapsam numa chave + 1 cauda).
        assert_eq!(disp.lines().filter(|l| !l.trim().is_empty()).count(), 3);
        assert!(idx.contains("ricms-rs:lI:art27:incX"));
        assert!(
            idx.contains("\"ocorrencias\": 2"),
            "as duas variantes colapsam: {idx}"
        );
        assert!(
            idx.contains("P 1") && idx.contains("P 2"),
            "os dois pareceres na chave"
        );

        // Idempotência (K8): re-rodar produz bytes idênticos.
        run(&entity).unwrap();
        let disp2 =
            std::fs::read_to_string(base.join("extracao").join("dispositivos.jsonl")).unwrap();
        assert_eq!(disp, disp2, "saída não-determinística");
    }
}
