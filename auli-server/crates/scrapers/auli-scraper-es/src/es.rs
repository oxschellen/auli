//! Coleta dos serviços da SEFAZ-ES a partir da API do X-Via (portal.es.gov.br) — **molde MT**.
//!
//! O ecossistema antigo (`conectacidadao`/`guiadeservicos`) migrou: tudo redireciona para
//! `portal.es.gov.br`, uma SPA React sobre a **X-Via Suite** (o MESMO stack do MT). A listagem por
//! órgão vem do método público **`POST /v1/search`** com corpo
//! `{query:"", groups:["CATALOG"], departmentSlug:<slug>, from, size}` → um **array JSON** de serviços.
//! É **anônimo** (sem token/sessão). A SEFAZ é `departmentSlug = "secretaria-de-estado-da-fazenda"`.
//!
//! Cada item já traz o conteúdo COMPLETO inline (não há chamada de detalhe): `title`, `description`
//! (resumo), **`serviceLetterContent`** (a carta de serviços, HTML → `html_to_text`), `category` (a
//! classe) e `targets` (o eixo de público — Cenário B). Modelagem (molde MT):
//! - **identidade = `slug`** (único); `link` = `…/servico/<slug>` (rota canônica da SPA);
//! - **públicos = `targets` NORMALIZADOS** (o dado publicado traz `cidadao` E `Cidadão` — mesma
//!   pessoa; normalizamos para não duplicar a ocorrência); `classe` = `category`;
//! - `ocorrencias` = públicos (dedup) × classe.
//!
//! Invariante (lição CE/MS/MT): a API devolve o próprio total em `resultTotal` (por item) — guard
//! duro `únicos == resultTotal` + piso estático. Sem paginação (um `size` alto traz o catálogo do
//! órgão inteiro). Cache só grava DEPOIS dos guards (D-RJ5).
//!
//! **D-PA-ROBOTS (ES = 2º caso):** UA institucional AuliBot, cortesia ≥1s, cache agressivo, nunca
//! autenticar (o Acesso Cidadão fica intocado).

use std::collections::HashSet;
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use auli_contract::{Ocorrencia, Publico, ServicoRaw};
use auli_scraper_kit::clean;
use auli_scraper_kit::http::GetOpts;
use scraper::Html;
use serde::Deserialize;

/// UA institucional do projeto (mitigação D-PA-ROBOTS): nunca UA de browser falso.
const USER_AGENT: &str =
    "AuliBot/0.1 (+https://github.com/oxschellen/auli; carlos.schellenberger@gmail.com)";

/// Endpoint público de busca do catálogo (same-origin do portal X-Via).
const SEARCH_URL: &str = "https://portal.es.gov.br/v1/search";
/// Órgão SEFAZ no X-Via (o slug de `/v1/department`). Escopo = SEFAZ.
const DEPARTMENT_SLUG: &str = "secretaria-de-estado-da-fazenda";
/// Base para as URLs canônicas de detalhe (`…/servico/<slug>`).
const SERVICO_BASE: &str = "https://portal.es.gov.br/servico";
/// `size` alto o bastante para trazer o catálogo do órgão numa chamada (45 hoje). Se o catálogo
/// passar disso, o invariante `únicos == resultTotal` reprova (pede aumentar).
const PAGE_SIZE: u32 = 500;

/// Fallback para serviço sem `targets` (público) ou sem `category` (classe). 0 hoje; defensivo.
const GERAL: &str = "Geral";
/// Órgão de origem.
const ORGAO: &str = "SEFAZ-ES";
/// Piso estático (o invariante principal é o dinâmico `únicos == resultTotal`; ~45 observados).
const MIN_SERVICOS: usize = 40;

/// Um item de serviço do catálogo. Só os campos que usamos; serde ignora o resto.
#[derive(Debug, Deserialize)]
struct Item {
    #[serde(default)]
    title: String,
    #[serde(default)]
    slug: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(rename = "serviceLetterContent", default)]
    letter: Option<String>,
    #[serde(default)]
    category: String,
    #[serde(default)]
    targets: Vec<String>,
    /// Total anunciado pela API para a consulta (igual em todos os itens) — base do invariante.
    #[serde(rename = "resultTotal", default)]
    result_total: i64,
}

/// Raspa o catálogo do órgão e devolve `(items, publicos_ordem)` prontos para o snapshot v3.
pub fn scrape(
    data_dir: &str,
    use_cache: bool,
) -> Result<(Vec<ServicoRaw>, Vec<Publico>), Box<dyn std::error::Error>> {
    let agent = auli_scraper_kit::build_agent(USER_AGENT, Some(Duration::from_secs(30)));

    // Chave de cache = o endpoint + o órgão. O cache só grava DEPOIS dos guards (D-RJ5).
    let logical = format!("{}#dept={}", SEARCH_URL, DEPARTMENT_SLUG);
    let (json, from_cache) = match auli_scraper_kit::cache::read_or_bail(data_dir, &logical, use_cache)? {
        Some(cached) => (cached, true),
        None => (fetch_search(&agent)?, false),
    };

    let raw: Vec<Item> = parse(&json)?;
    let result_total = raw.iter().map(|i| i.result_total).max().unwrap_or(0);
    println!("ES: recebidos {} itens (API anuncia resultTotal={})", raw.len(), result_total);

    let (items, publicos_ordem) = build_servicos(&raw);

    validar(&items, result_total)?;
    if !from_cache {
        auli_scraper_kit::cache::write(data_dir, &logical, &json);
    }

    let ocorrencias: usize = items.iter().map(|s| s.ocorrencias.len()).sum();
    println!(
        "ES: {} serviços ({} ocorrências) em {} público(s)",
        items.len(),
        ocorrencias,
        publicos_ordem.len()
    );
    Ok((items, publicos_ordem))
}

/// POST `/v1/search` (anônimo). Retenta com backoff. Devolve o corpo JSON cru.
fn fetch_search(agent: &ureq::Agent) -> Result<String> {
    let body = serde_json::json!({
        "query": "",
        "groups": ["CATALOG"],
        "departmentSlug": DEPARTMENT_SLUG,
        "from": 0,
        "size": PAGE_SIZE,
    });
    auli_scraper_kit::http::post_json(
        agent,
        SEARCH_URL,
        &[("Origin", "https://portal.es.gov.br")],
        &body,
        &GetOpts { log_prefix: "ES", accept: Some("application/json"), ..Default::default() },
    )
}

/// Parseia o array JSON de serviços.
fn parse(json: &str) -> Result<Vec<Item>> {
    serde_json::from_str::<Vec<Item>>(json)
        .map_err(|e| anyhow!("JSON de /v1/search inválido: {}", e))
}

/// Monta os `ServicoRaw` (dedup por `slug`) e a ordem dos públicos. `ocorrencias` = públicos
/// (normalizados+dedup) × classe; órfãos caem no fallback "Geral".
fn build_servicos(raw: &[Item]) -> (Vec<ServicoRaw>, Vec<Publico>) {
    let mut vistos: HashSet<String> = HashSet::new();
    let mut publicos_ordem: Vec<String> = Vec::new();
    let mut out: Vec<ServicoRaw> = Vec::new();
    let mut orfaos_publico = 0usize;
    let mut orfaos_classe = 0usize;

    for it in raw {
        if it.slug.is_empty() || !vistos.insert(it.slug.clone()) {
            continue;
        }
        let titulo = clean(&it.title);
        if titulo.is_empty() {
            continue;
        }

        let classe = {
            let c = clean(&it.category);
            if c.is_empty() {
                orfaos_classe += 1;
                GERAL.to_string()
            } else {
                c
            }
        };

        // Públicos normalizados e deduplicados (o dado traz `cidadao` E `Cidadão`).
        let mut publicos: Vec<String> = Vec::new();
        for t in &it.targets {
            let p = normaliza_publico(t);
            if !p.is_empty() && !publicos.contains(&p) {
                publicos.push(p);
            }
        }
        if publicos.is_empty() {
            publicos.push(GERAL.to_string());
            orfaos_publico += 1;
        }

        let mut ocorrencias = Vec::with_capacity(publicos.len());
        for p in &publicos {
            if !publicos_ordem.contains(p) {
                publicos_ordem.push(p.clone());
            }
            ocorrencias.push(Ocorrencia { publico: p.clone(), classe: classe.clone() });
        }

        out.push(ServicoRaw {
            titulo,
            descricao: montar_descricao(it),
            link: format!("{}/{}", SERVICO_BASE, it.slug),
            orgao: ORGAO.to_string(),
            ocorrencias,
        });
    }

    if orfaos_publico + orfaos_classe > 0 {
        eprintln!(
            "⚠️  ES: órfãos com fallback '{}': {} sem público (target), {} sem classe (category).",
            GERAL, orfaos_publico, orfaos_classe
        );
    }

    let publicos = publicos_ordem
        .into_iter()
        .map(|nome| Publico { slug: slug_publico(&nome), nome })
        .collect();
    (out, publicos)
}

/// Descrição = resumo (`description`) + a carta rica (`serviceLetterContent`, HTML→texto). Une só as
/// partes não vazias.
fn montar_descricao(it: &Item) -> String {
    let resumo = clean(it.description.as_deref().unwrap_or_default());
    let carta = html_to_text(it.letter.as_deref().unwrap_or_default());
    match (resumo.is_empty(), carta.is_empty()) {
        (false, false) => format!("{}\n\n{}", resumo, carta),
        (false, true) => resumo,
        (true, false) => carta,
        (true, true) => String::new(),
    }
}

/// Normaliza um `target` (público): as variações `cidadao`/`Cidadão` viram um único "Cidadão";
/// `empresa`→"Empresa". Demais valores ficam limpos como estão.
fn normaliza_publico(t: &str) -> String {
    let c = clean(t);
    match c.to_lowercase().as_str() {
        "cidadao" | "cidadão" => "Cidadão".to_string(),
        "empresa" => "Empresa".to_string(),
        _ => c,
    }
}

/// `serviceLetterContent` é HTML. Tags viram espaço (separa parágrafos), depois o html5ever
/// decodifica TODAS as entidades (fora da tabela fixa do kit), e o `kit::clean` comprime espaços.
fn html_to_text(html: &str) -> String {
    if html.trim().is_empty() {
        return String::new();
    }
    let mut spaced = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                spaced.push(' ');
            }
            _ if !in_tag => spaced.push(c),
            _ => {}
        }
    }
    let decoded: String = Html::parse_fragment(&spaced).root_element().text().collect();
    clean(&decoded)
}

/// Slug do arquivo per-público (ex.: `Cidadão` -> `servicos-cidadao`).
fn slug_publico(nome: &str) -> String {
    format!("servicos-{}", slugify(nome))
}

/// ASCII-fold pt-BR + kebab.
fn slugify(s: &str) -> String {
    let mut buf = String::with_capacity(s.len());
    for c in s.chars() {
        let m = match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' | 'Á' | 'À' | 'Â' | 'Ã' | 'Ä' => 'a',
            'é' | 'ê' | 'è' | 'ë' | 'É' | 'Ê' | 'È' | 'Ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' => 'i',
            'ó' | 'ô' | 'õ' | 'ò' | 'ö' | 'Ó' | 'Ô' | 'Õ' | 'Ò' | 'Ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Ü' => 'u',
            'ç' | 'Ç' => 'c',
            c if c.is_ascii_alphanumeric() => c.to_ascii_lowercase(),
            _ => '-',
        };
        buf.push(m);
    }
    buf.split('-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("-")
}

/// Guard: invariante dinâmico `únicos == resultTotal` + piso estático.
fn validar(items: &[ServicoRaw], result_total: i64) -> Result<()> {
    let unicos = items.len();
    if result_total > 0 && unicos as i64 != result_total {
        bail!(
            "catálogo incompleto/divergente: API anuncia resultTotal={} e coletamos {} único(s) \
             (aumente PAGE_SIZE se {} > {}). Se veio do cache, limpe data/es/raw/cache/ e re-raspe.",
            result_total,
            unicos,
            result_total,
            PAGE_SIZE
        );
    }
    if unicos < MIN_SERVICOS {
        bail!(
            "catálogo capado/vazio? só {} serviço(s) (mínimo {}). Se veio do cache, limpe \
             data/es/raw/cache/ e re-raspe.",
            unicos,
            MIN_SERVICOS
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ARR_JSON: &str = r#"[
      {"title":"Emissão de DUA","slug":"emissao-de-dua","description":"Emite o DUA.",
       "serviceLetterContent":"<p>O <strong>DUA</strong> &eacute; o documento &uacute;nico.</p>",
       "category":"IMPOSTOS E MULTAS","targets":["cidadao","Cidadão"],"resultTotal":2},
      {"title":"  Inscrição  Estadual ","slug":"inscricao-estadual","description":null,
       "serviceLetterContent":null,"category":"EMPRESAS","targets":["Empresa"],"resultTotal":2}
    ]"#;

    #[test]
    fn parse_le_itens() {
        let v = parse(ARR_JSON).unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].result_total, 2);
    }

    #[test]
    fn build_normaliza_publico_e_dedup() {
        let (items, publicos) = build_servicos(&parse(ARR_JSON).unwrap());
        assert_eq!(items.len(), 2);
        // "cidadao"+"Cidadão" -> um único público "Cidadão" -> 1 ocorrência.
        let dua = items.iter().find(|s| s.titulo == "Emissão de DUA").unwrap();
        assert_eq!(dua.ocorrencias.len(), 1);
        assert_eq!(dua.ocorrencias[0].publico, "Cidadão");
        assert_eq!(dua.ocorrencias[0].classe, "IMPOSTOS E MULTAS");
        assert_eq!(dua.link, "https://portal.es.gov.br/servico/emissao-de-dua");
        // ordem dos públicos: Cidadão (1º serviço), Empresa (2º).
        assert_eq!(publicos.iter().map(|p| p.nome.as_str()).collect::<Vec<_>>(), ["Cidadão", "Empresa"]);
        assert_eq!(publicos[0].slug, "servicos-cidadao");
    }

    #[test]
    fn descricao_junta_resumo_e_carta_html() {
        let it = &parse(ARR_JSON).unwrap()[0];
        let d = montar_descricao(it);
        // resumo + carta com entidades decodificadas e tags removidas.
        assert!(d.starts_with("Emite o DUA."));
        assert!(d.contains("O DUA é o documento único."), "veio: {d}");
        assert!(!d.contains('<'));
    }

    #[test]
    fn descricao_so_resumo_quando_carta_nula() {
        let it = &parse(ARR_JSON).unwrap()[1];
        // description null + letter null -> vazio; titulo comprimido.
        assert_eq!(montar_descricao(it), "");
    }

    #[test]
    fn titulo_comprime_espacos() {
        let items = build_servicos(&parse(ARR_JSON).unwrap()).0;
        assert!(items.iter().any(|s| s.titulo == "Inscrição Estadual"));
    }

    #[test]
    fn validar_reprova_divergencia_de_total() {
        let dummy = |i: usize| ServicoRaw {
            titulo: format!("s{i}"),
            descricao: String::new(),
            link: format!("l{i}"),
            orgao: ORGAO.into(),
            ocorrencias: vec![],
        };
        let items: Vec<ServicoRaw> = (0..MIN_SERVICOS).map(dummy).collect();
        // resultTotal maior que o coletado -> catálogo capado (page size pequeno).
        let err = validar(&items, MIN_SERVICOS as i64 + 5).unwrap_err().to_string();
        assert!(err.contains("incompleto/divergente"), "veio: {err}");
    }

    #[test]
    fn validar_reprova_capado_pelo_minimo() {
        let poucos = vec![ServicoRaw {
            titulo: "x".into(),
            descricao: String::new(),
            link: "l".into(),
            orgao: ORGAO.into(),
            ocorrencias: vec![],
        }];
        assert!(validar(&poucos, 1).unwrap_err().to_string().contains("capado"));
    }

    #[test]
    fn html_to_text_vazio() {
        assert_eq!(html_to_text(""), "");
        assert_eq!(html_to_text("   "), "");
    }
}
