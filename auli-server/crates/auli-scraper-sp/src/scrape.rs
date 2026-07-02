// Coleta do catálogo SharePoint da SEFAZ-SP via REST `_api` (JSON verbose, anônimo). Duas listas:
//   - 'Homes 360'  -> ID -> Assunto (a classe/tema).
//   - 'Serviços'   -> Title, Descricao (card), URL, facetas Cidadao/Empresa/Servidor/Tributo (público),
//                     Acesso (forma de acesso), Home360.ID (-> Assunto). Filtro `Indicador eq 1`.
// Um serviço em N facetas vira N `Ocorrencia`s (schema v2). `descricao` = card (D-SP4), não o subsite.

use std::collections::HashMap;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use serde::Deserialize;
use ureq::Agent;

use auli_contract::{Ocorrencia, Publico, ServicoRaw};

const BASE: &str = "https://portal.fazenda.sp.gov.br";
const API: &str = "https://portal.fazenda.sp.gov.br/servicos/_api/web/lists";
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64; rv:124.0) Gecko/20100101 Firefox/124.0";
const COURTESY: Duration = Duration::from_millis(300);

/// Os públicos, na ordem de exibição: as 4 abas do catálogo. `(campo da faceta, nome, slug)`.
/// `Tributos` entra como aba (o portal a tabula) para não perder os ~8 serviços sem audiência —
/// a `classe` continua sendo o `Assunto`, uma dimensão separada.
fn publicos() -> [(&'static str, &'static str, &'static str); 4] {
    [
        ("Cidadao", "Cidadão", "servicos-ao-cidadao"),
        ("Empresa", "Empresa", "servicos-a-empresas"),
        ("Servidor", "Servidor Público", "servicos-a-servidores"),
        ("Tributo", "Tributos", "servicos-tributos"),
    ]
}

// --- Shapes da API (verbose: `{ "d": { "results": [...], "__next": "..." } }`) ---

#[derive(Deserialize)]
struct Resp<T> {
    d: DBlock<T>,
}

#[derive(Deserialize)]
struct DBlock<T> {
    results: Vec<T>,
    #[serde(rename = "__next", default)]
    next: Option<String>,
}

#[derive(Deserialize, Default)]
struct Facet {
    #[serde(default)]
    results: Vec<String>,
}

#[derive(Deserialize)]
struct HomeRef {
    #[serde(rename = "ID")]
    id: i64,
}

#[derive(Deserialize)]
struct HomeRow {
    #[serde(rename = "ID")]
    id: i64,
    #[serde(rename = "Assunto", default)]
    assunto: String,
}

#[derive(Deserialize)]
struct SvcRow {
    #[serde(rename = "Title", default)]
    title: String,
    #[serde(rename = "Descricao", default)]
    descricao: Option<String>,
    #[serde(rename = "URL", default)]
    url: Option<String>,
    #[serde(rename = "Cidadao", default)]
    cidadao: Facet,
    #[serde(rename = "Empresa", default)]
    empresa: Facet,
    #[serde(rename = "Servidor", default)]
    servidor: Facet,
    #[serde(rename = "Tributo", default)]
    tributo: Facet,
    #[serde(rename = "Acesso", default)]
    acesso: Facet,
    #[serde(rename = "Home360", default)]
    home360: Option<HomeRef>,
}

impl SvcRow {
    fn facet(&self, campo: &str) -> &[String] {
        match campo {
            "Cidadao" => &self.cidadao.results,
            "Empresa" => &self.empresa.results,
            "Servidor" => &self.servidor.results,
            "Tributo" => &self.tributo.results,
            _ => &[],
        }
    }
}

/// Raspa o catálogo da SEFAZ-SP e devolve os `ServicoRaw` (um por serviço) + a ordem dos públicos.
pub fn scrape(data_dir: &str, use_cache: bool) -> Result<(Vec<ServicoRaw>, Vec<Publico>)> {
    let agent = auli_scraper_kit::build_agent(USER_AGENT, Some(Duration::from_secs(30)));

    // 1. Homes 360: ID -> Assunto (classe).
    let homes_url = format!(
        "{}/getbytitle('Homes%20360')/items?$select=ID,Assunto&$top=1000",
        API
    );
    let homes: Vec<HomeRow> = fetch_all(&agent, data_dir, &homes_url, use_cache)?;
    let assunto: HashMap<i64, String> = homes.into_iter().map(|h| (h.id, h.assunto)).collect();
    println!("SP: {} homes (assuntos)", assunto.len());

    // 2. Serviços (Indicador eq 1).
    let svcs_url = format!(
        "{}/getbytitle('Servi%C3%A7os')/items?$select=ID,Title,Descricao,URL,Cidadao,Empresa,\
         Servidor,Tributo,Acesso,Home360/ID&$expand=Home360/ID&$filter=Indicador%20eq%201&$top=1000",
        API
    );
    let svcs: Vec<SvcRow> = fetch_all(&agent, data_dir, &svcs_url, use_cache)?;
    println!("SP: {} serviços no catálogo", svcs.len());

    // 3. Um `ServicoRaw` por serviço do catálogo (a linha é a identidade — a URL não é única no SP).
    //    Ocorrências = uma por faceta de público preenchida (todas com a mesma `classe` = Assunto).
    let pubs = publicos();
    let mut items: Vec<ServicoRaw> = Vec::new();
    let mut sem_publico = 0;
    for s in &svcs {
        let classe = s
            .home360
            .as_ref()
            .and_then(|h| assunto.get(&h.id))
            .cloned()
            .unwrap_or_default();
        let ocorrencias: Vec<Ocorrencia> = pubs
            .iter()
            .filter(|(campo, ..)| !s.facet(campo).is_empty())
            .map(|(_, nome, _)| Ocorrencia { publico: nome.to_string(), classe: classe.clone() })
            .collect();
        if ocorrencias.is_empty() {
            sem_publico += 1;
            continue;
        }
        items.push(ServicoRaw {
            titulo: clean(&s.title),
            descricao: build_corpo(s),
            link: canonical(s.url.as_deref().unwrap_or_default()),
            orgao: "SEFAZ-SP".to_string(),
            ocorrencias,
        });
    }
    if sem_publico > 0 {
        eprintln!("⚠️  SP: {} serviço(s) sem nenhuma faceta de público — fora do catálogo.", sem_publico);
    }
    println!("SP: {} serviços com público (de {} no catálogo)", items.len(), svcs.len());

    let publicos_ordem = pubs
        .iter()
        .map(|(_, nome, slug)| Publico { nome: nome.to_string(), slug: slug.to_string() })
        .collect();
    Ok((items, publicos_ordem))
}

/// Corpo do card: descrição (limpa) + "Formas de acesso: ...". Sem o conteúdo dos subsites (D-SP4).
fn build_corpo(s: &SvcRow) -> String {
    let mut corpo = clean(s.descricao.as_deref().unwrap_or_default());
    let acesso: Vec<String> = s.acesso.results.iter().map(|a| clean(a)).filter(|a| !a.is_empty()).collect();
    if !acesso.is_empty() {
        if !corpo.is_empty() {
            corpo.push('\n');
        }
        corpo.push_str("Formas de acesso: ");
        corpo.push_str(&acesso.join(", "));
    }
    corpo
}

/// Normaliza texto do SharePoint: tira zero-width/nbsp e comprime espaços.
fn clean(s: &str) -> String {
    s.replace('\u{200b}', "").replace('\u{00a0}', " ").split_whitespace().collect::<Vec<_>>().join(" ")
}

/// URL canônica do serviço (D-SP3): trim; relativo `/...` -> host do portal; externo/absoluto como está.
fn canonical(url: &str) -> String {
    let u = url.trim();
    if u.starts_with("http://") || u.starts_with("https://") {
        u.to_string()
    } else if let Some(rest) = u.strip_prefix('/') {
        format!("{}/{}", BASE, rest)
    } else {
        u.to_string()
    }
}

/// Busca todas as páginas de uma lista, seguindo `d.__next` (paginação SharePoint). Cacheia cada
/// página por URL (kit); em `--usecache` um miss é erro.
fn fetch_all<T: for<'de> Deserialize<'de>>(
    agent: &Agent,
    data_dir: &str,
    first_url: &str,
    use_cache: bool,
) -> Result<Vec<T>> {
    let mut out = Vec::new();
    let mut url = Some(first_url.to_string());
    while let Some(u) = url {
        let body = fetch(agent, data_dir, &u, use_cache)?;
        let parsed: Resp<T> = serde_json::from_str(&body)
            .map_err(|e| anyhow!("JSON inválido de {}: {}", u, e))?;
        out.extend(parsed.d.results);
        url = parsed.d.next;
    }
    Ok(out)
}

/// Busca (ou lê do cache) uma URL do `_api` (Accept verbose). Retenta falhas transitórias; cortesia
/// entre chamadas de rede.
fn fetch(agent: &Agent, data_dir: &str, url: &str, use_cache: bool) -> Result<String> {
    if let Some(cached) = auli_scraper_kit::cache::read(data_dir, url) {
        return Ok(cached);
    }
    if use_cache {
        bail!("cache miss para {} (modo --usecache, sem rede)", url);
    }

    let max_attempts = 3;
    let mut delay = Duration::from_millis(800);
    let mut last = anyhow!("sem tentativa");
    for attempt in 1..=max_attempts {
        match agent.get(url).header("Accept", "application/json;odata=verbose").call() {
            Ok(mut resp) => match resp.body_mut().read_to_string() {
                Ok(body) if !body.trim().is_empty() => {
                    auli_scraper_kit::cache::write(data_dir, url, &body);
                    sleep(COURTESY);
                    return Ok(body);
                }
                Ok(_) => last = anyhow!("resposta vazia"),
                Err(e) => last = anyhow!(e.to_string()),
            },
            Err(e) => last = anyhow!(e.to_string()),
        }
        if attempt < max_attempts {
            eprintln!("SP: falha em {} (tentativa {}/{}): {}. Retentando...", url, attempt, max_attempts, last);
            sleep(delay);
            delay = delay.saturating_mul(2);
        }
    }
    Err(anyhow!("falha ao buscar {}: {}", url, last))
}
