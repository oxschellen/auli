//! Coleta dos serviços da SEFAZ-MT a partir da API JSON do "Catálogo de Serviços do X-Via Portal".
//!
//! O portal é uma SPA React (X-Via, front do X-Road de MT): não há HTML server-rendered (D-MT1).
//! A listagem por órgão vem do método público `POST /v1/search/department` com corpo
//! `{groups:["CATALOG"], departmentSlug:<slug>}` → um **array JSON** de serviços. É **anônimo**: sem
//! Keycloak, sem token (o `#error=login_required` do shell é ruído do silent-SSO `prompt=none`, não
//! do catálogo). D-MT3 satisfeito.
//!
//! Cada item já traz `title`, `description` (rica, inline → NÃO há chamada de detalhe), `category`
//! (a classe) e `targets` (o eixo de público — Cenário B). Modelagem (D-MT2/4):
//! - **identidade = `slug`** (único no catálogo do órgão); `link` = URL canônica de detalhe
//!   `…/app/catalog/<categorySlug>/<slug>`;
//! - **públicos = `targets`** (ex.: Cidadão, Empresa), descobertos na ordem de aparição;
//! - **`classe` = `category`** (uma por serviço); `ocorrencias` = targets × category.
//!
//! Invariante (D-MT5, lição CE/MS): a API devolve o próprio total em `resultTotal` (por item) — o
//! guard duro é `únicos == resultTotal`, mais um piso estático de folga por baixo. Sem paginação
//! (a resposta traz o catálogo inteiro do órgão numa chamada). Cache só grava DEPOIS dos guards
//! (D-RJ5).

use std::collections::HashSet;
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use auli_contract::{Ocorrencia, Publico, ServicoRaw};
use serde::Deserialize;

/// Endpoint público da listagem por órgão (same-origin do portal — `window.BACKEND_ENDPOINT`).
const DEPARTMENT_URL: &str = "https://portal.mt.gov.br/v1/search/department";
/// O órgão SEFAZ no X-Via (o slug da URL `/app/catalog/orgao/<slug>`) — D-MT1: escopo = SEFAZ.
const DEPARTMENT_SLUG: &str = "secretaria-de-estado-de-fazenda";
/// Base para as URLs canônicas de detalhe.
const CATALOG_BASE: &str = "https://portal.mt.gov.br/app/catalog";

const USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64; rv:124.0) Gecko/20100101 Firefox/124.0";

/// Fallback para serviço sem `targets` (público) ou sem `category` (classe). 0 hoje; defensivo.
const GERAL: &str = "Geral";
/// Órgão de origem.
const ORGAO: &str = "SEFAZ-MT";

/// Piso estático de folga (o invariante principal é o dinâmico `únicos == resultTotal`; ~27
/// serviços observados em 2026-07). Baixo de propósito: só um backstop contra resposta capada/vazia.
const MIN_SERVICOS: usize = 15;

/// Um item de serviço do catálogo. Só os campos que usamos; serde ignora o resto.
#[derive(Debug, Deserialize)]
struct Item {
    #[serde(default)]
    title: String,
    #[serde(default)]
    slug: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    category: String,
    #[serde(rename = "categorySlug", default)]
    category_slug: String,
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
    let logical = format!("{}#dept={}", DEPARTMENT_URL, DEPARTMENT_SLUG);
    let (json, from_cache) = match auli_scraper_kit::cache::read(data_dir, &logical) {
        Some(cached) => {
            println!("Cache hit: {}", logical);
            (cached, true)
        }
        None => {
            if use_cache {
                return Err(anyhow!(
                    "cache vazio para o catálogo de MT (modo --usecache, sem rede). Rode uma \
                     coleta com rede primeiro."
                )
                .into());
            }
            (fetch_department(&agent)?, false)
        }
    };

    let raw: Vec<Item> = parse(&json)?;
    let result_total = raw.iter().map(|i| i.result_total).max().unwrap_or(0);
    println!("MT: recebidos {} itens (API anuncia resultTotal={})", raw.len(), result_total);

    let (items, publicos_ordem) = build_servicos(&raw);

    // Guards (D-MT5) antes de qualquer escrita de cache.
    validar(&items, result_total)?;
    if !from_cache {
        auli_scraper_kit::cache::write(data_dir, &logical, &json);
    }

    let ocorrencias: usize = items.iter().map(|s| s.ocorrencias.len()).sum();
    println!(
        "MT: {} serviços ({} ocorrências) em {} público(s)",
        items.len(),
        ocorrencias,
        publicos_ordem.len()
    );
    Ok((items, publicos_ordem))
}

/// POST `search/department` (anônimo). Retenta com backoff. Devolve o corpo JSON cru.
fn fetch_department(agent: &ureq::Agent) -> Result<String> {
    let body = serde_json::json!({
        "groups": ["CATALOG"],
        "departmentSlug": DEPARTMENT_SLUG,
    });
    let max_attempts = 3;
    let mut delay = Duration::from_millis(800);
    let mut last = anyhow!("sem tentativa");
    println!("POST {} (departmentSlug={})", DEPARTMENT_URL, DEPARTMENT_SLUG);
    for attempt in 1..=max_attempts {
        let sent = agent
            .post(DEPARTMENT_URL)
            .header("Accept", "application/json")
            .header("Origin", "https://portal.mt.gov.br")
            .send_json(&body);
        match sent {
            Ok(mut resp) => match resp.body_mut().read_to_string() {
                Ok(s) if !s.trim().is_empty() => return Ok(s),
                Ok(_) => last = anyhow!("resposta vazia"),
                Err(e) => last = anyhow!(e.to_string()),
            },
            Err(e) => last = anyhow!(e.to_string()),
        }
        if attempt < max_attempts {
            eprintln!("⚠️  MT: tentativa {} falhou ({}); retentando…", attempt, last);
            std::thread::sleep(delay);
            delay *= 2;
        }
    }
    Err(anyhow!("falha ao buscar o catálogo após {} tentativas: {}", max_attempts, last))
}

/// Parseia o array JSON de serviços.
fn parse(json: &str) -> Result<Vec<Item>> {
    serde_json::from_str::<Vec<Item>>(json)
        .map_err(|e| anyhow!("JSON de search/department inválido: {}", e))
}

/// Monta os `ServicoRaw` (dedup por `slug`, ordem de descoberta) e a ordem dos públicos.
/// `ocorrencias` = `targets` × `category`; órfãos caem no fallback "Geral" (D-MT4).
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

        let mut publicos: Vec<String> =
            it.targets.iter().map(|t| clean(t)).filter(|t| !t.is_empty()).collect();
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
            descricao: clean(&it.description),
            link: canonical(&it.category_slug, &it.slug),
            orgao: ORGAO.to_string(),
            ocorrencias,
        });
    }

    if orfaos_publico + orfaos_classe > 0 {
        eprintln!(
            "⚠️  MT: órfãos com fallback '{}': {} sem público (target), {} sem classe (category).",
            GERAL, orfaos_publico, orfaos_classe
        );
    }

    let publicos = publicos_ordem
        .into_iter()
        .map(|nome| Publico { slug: slug_publico(&nome), nome })
        .collect();
    (out, publicos)
}

/// URL canônica de detalhe: `…/app/catalog/<categorySlug>/<slug>`.
fn canonical(category_slug: &str, slug: &str) -> String {
    format!("{}/{}/{}", CATALOG_BASE, category_slug, slug)
}

/// Slug do arquivo per-público a partir do nome do público (ex.: `Cidadão` -> `servicos-cidadao`).
fn slug_publico(nome: &str) -> String {
    format!("servicos-{}", slugify(nome))
}

/// ASCII-fold pt-BR + kebab: `Poder Público` -> `poder-publico`, `Cidadão` -> `cidadao`.
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

/// Normaliza texto: tira zero-width/nbsp e comprime espaços (padrão da frota).
fn clean(s: &str) -> String {
    s.replace('\u{200b}', "").replace('\u{00a0}', " ").split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Guard D-MT5: invariante dinâmico `únicos == resultTotal` (a API dá o próprio total), depois o
/// piso estático de folga. Uma resposta capada (menos itens que o total anunciado) reprova alto.
fn validar(items: &[ServicoRaw], result_total: i64) -> Result<()> {
    let unicos = items.len();
    if result_total > 0 && unicos as i64 != result_total {
        bail!(
            "catálogo incompleto/divergente: API anuncia resultTotal={} e coletamos {} único(s). \
             Se veio do cache, limpe data/mt/raw/cache/ e re-raspe.",
            result_total,
            unicos
        );
    }
    if unicos < MIN_SERVICOS {
        bail!(
            "catálogo capado/vazio? só {} serviço(s) (mínimo {}). Se veio do cache, limpe \
             data/mt/raw/cache/ e re-raspe.",
            unicos,
            MIN_SERVICOS
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Fixture derivada de itens reais do `search/department` (campos que usamos).
    const JSON: &str = r#"[
      {"title":"Abrir Inscrição Estadual MEI","slug":"abrir-ie-mei",
       "description":"  Abertura de IE do MEI. ","category":"Finanças e Tributação",
       "categorySlug":"financas-e-tributacao","targets":["Cidadão","Empresa"],"resultTotal":3},
      {"title":"Alterar Representação de Contabilista","slug":"alterar-rep-contabilista",
       "description":"Troca de contador.","category":"Finanças e Tributação",
       "categorySlug":"financas-e-tributacao","targets":["Empresa","Cidadão"],"resultTotal":3},
      {"title":"Ouvidoria da SEFAZ","slug":"ouvidoria","description":"Fale com a SEFAZ.",
       "category":"Comunicação e Transparência","categorySlug":"comunicacao-e-transparencia",
       "targets":["Cidadão"],"resultTotal":3}
    ]"#;

    fn parsed() -> Vec<Item> {
        parse(JSON).unwrap()
    }

    #[test]
    fn parse_le_itens_e_result_total() {
        let raw = parsed();
        assert_eq!(raw.len(), 3);
        assert_eq!(raw[0].result_total, 3);
        assert_eq!(raw[0].targets, vec!["Cidadão", "Empresa"]);
    }

    #[test]
    fn build_mapeia_campos_link_e_ocorrencias() {
        let (items, publicos) = build_servicos(&parsed());
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].titulo, "Abrir Inscrição Estadual MEI");
        assert_eq!(items[0].descricao, "Abertura de IE do MEI.", "clean() comprime espaços");
        assert_eq!(items[0].orgao, "SEFAZ-MT");
        assert_eq!(
            items[0].link,
            "https://portal.mt.gov.br/app/catalog/financas-e-tributacao/abrir-ie-mei"
        );
        // ocorrências = targets × category (category única): 2 targets -> 2 ocorrências, na ordem.
        assert_eq!(items[0].ocorrencias.len(), 2);
        assert_eq!(items[0].ocorrencias[0].publico, "Cidadão");
        assert_eq!(items[0].ocorrencias[0].classe, "Finanças e Tributação");
        assert_eq!(items[0].ocorrencias[1].publico, "Empresa");
        // públicos na ordem de descoberta (1o item: Cidadão, Empresa); slug ascii-fold.
        assert_eq!(publicos.iter().map(|p| p.nome.as_str()).collect::<Vec<_>>(), vec!["Cidadão", "Empresa"]);
        assert_eq!(publicos[0].slug, "servicos-cidadao");
        assert_eq!(publicos[1].slug, "servicos-empresa");
    }

    #[test]
    fn dedup_por_slug() {
        let mut raw = parsed();
        raw.push(parse(JSON).unwrap().remove(0)); // repete o slug "abrir-ie-mei"
        let (items, _) = build_servicos(&raw);
        assert_eq!(items.len(), 3, "slug repetido não duplica");
    }

    #[test]
    fn orfao_sem_target_cai_no_publico_geral() {
        let json = r#"[{"title":"X","slug":"x","description":"d","category":"C","categorySlug":"c","targets":[],"resultTotal":1}]"#;
        let (items, publicos) = build_servicos(&parse(json).unwrap());
        assert_eq!(items[0].ocorrencias[0].publico, GERAL);
        assert_eq!(items[0].ocorrencias[0].classe, "C");
        assert_eq!(publicos[0].nome, GERAL);
    }

    #[test]
    fn orfao_sem_category_cai_na_classe_geral() {
        let json = r#"[{"title":"X","slug":"x","description":"d","category":"","categorySlug":"","targets":["Cidadão"],"resultTotal":1}]"#;
        let (items, _) = build_servicos(&parse(json).unwrap());
        assert_eq!(items[0].ocorrencias[0].classe, GERAL);
    }

    #[test]
    fn validar_reprova_divergencia_do_result_total() {
        let items = vec![svc("a"), svc("b")];
        let err = validar(&items, 5).unwrap_err().to_string();
        assert!(err.contains("incompleto") || err.contains("divergente"), "veio: {err}");
    }

    #[test]
    fn validar_reprova_abaixo_do_minimo() {
        let items = vec![svc("a")]; // 1 == resultTotal 1, mas < MIN_SERVICOS
        let err = validar(&items, 1).unwrap_err().to_string();
        assert!(err.contains("capado") || err.contains("vazio"), "veio: {err}");
    }

    #[test]
    fn slugify_ascii_fold() {
        assert_eq!(slugify("Cidadão"), "cidadao");
        assert_eq!(slugify("Poder Público"), "poder-publico");
        assert_eq!(slugify("Comunicação e Transparência"), "comunicacao-e-transparencia");
    }

    fn svc(slug: &str) -> ServicoRaw {
        ServicoRaw {
            titulo: "t".into(),
            descricao: String::new(),
            link: slug.into(),
            orgao: ORGAO.into(),
            ocorrencias: vec![],
        }
    }
}
