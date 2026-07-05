// Coleta do catálogo SharePoint da SEFAZ-SP via REST `_api` (JSON verbose, anônimo). Duas listas:
//   - 'Homes 360'  -> ID -> Assunto (a classe/tema).
//   - 'Serviços'   -> Title, Descricao (card), URL, facetas Cidadao/Empresa/Servidor/Tributo (público),
//                     Acesso (forma de acesso), Home360.ID (-> Assunto). Filtro `Indicador eq 1`.
// Um serviço em N facetas vira N `Ocorrencia`s (schema v2). `descricao` = card (D-SP4), não o subsite.

use std::collections::HashMap;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Result, anyhow};
use serde::Deserialize;
use ureq::Agent;

use auli_contract::{Ocorrencia, Publico, ServicoRaw};
use auli_scraper_kit::clean;
use auli_scraper_kit::http::GetOpts;

const BASE: &str = "https://portal.fazenda.sp.gov.br";
const API: &str = "https://portal.fazenda.sp.gov.br/servicos/_api/web/lists";
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
    let agent = auli_scraper_kit::build_agent(auli_scraper_kit::USER_AGENT, Some(Duration::from_secs(30)));

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
    let mut sem_link = 0;
    for s in &svcs {
        match build_servico(s, &assunto, &pubs) {
            Some(sr) => {
                if sr.link.is_empty() {
                    sem_link += 1;
                }
                items.push(sr);
            }
            None => sem_publico += 1,
        }
    }
    if sem_publico > 0 {
        eprintln!("⚠️  SP: {} serviço(s) sem nenhuma faceta de público — fora do catálogo.", sem_publico);
    }
    if sem_link > 0 {
        eprintln!("⚠️  SP: {} serviço(s) sem URL — link vazio no contrato (a linha ainda é a identidade).", sem_link);
    }
    println!("SP: {} serviços com público (de {} no catálogo)", items.len(), svcs.len());

    let publicos_ordem = pubs
        .iter()
        .map(|(_, nome, slug)| Publico { nome: nome.to_string(), slug: slug.to_string() })
        .collect();
    Ok((items, publicos_ordem))
}

/// Monta o `ServicoRaw` de um serviço: uma `Ocorrencia` por faceta de público preenchida (todas com
/// a mesma `classe` = Assunto do Home360). `None` quando o serviço não tem nenhuma faceta (fora do
/// catálogo). A linha é a identidade — a URL não é única no SP, então link vazio ainda vira registro.
fn build_servico(
    s: &SvcRow,
    assunto: &HashMap<i64, String>,
    pubs: &[(&str, &str, &str)],
) -> Option<ServicoRaw> {
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
        return None;
    }
    Some(ServicoRaw {
        titulo: clean(&s.title),
        descricao: build_corpo(s),
        link: canonical(s.url.as_deref().unwrap_or_default()),
        orgao: "SEFAZ-SP".to_string(),
        ocorrencias,
    })
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
    if let Some(cached) = auli_scraper_kit::cache::read_or_bail(data_dir, url, use_cache)? {
        return Ok(cached);
    }
    let body = auli_scraper_kit::http::get_string(
        agent,
        url,
        &GetOpts { log_prefix: "SP", accept: Some("application/json;odata=verbose"), ..Default::default() },
    )?;
    auli_scraper_kit::cache::write(data_dir, url, &body);
    sleep(COURTESY);
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_normaliza_nbsp_zerowidth_e_espacos() {
        assert_eq!(clean("a\u{00a0}b   c"), "a b c");
        assert_eq!(clean("x\u{200b}y"), "xy");
        assert_eq!(clean("  vários   espaços  "), "vários espaços");
    }

    #[test]
    fn canonical_cobre_os_formatos() {
        assert_eq!(canonical("https://x.sp.gov.br/a"), "https://x.sp.gov.br/a");
        assert_eq!(canonical(" https://y.sp.gov.br "), "https://y.sp.gov.br"); // trim
        assert_eq!(canonical("/Pages/Consulta.aspx"), format!("{}/Pages/Consulta.aspx", BASE));
        assert_eq!(canonical("relativo-sem-barra"), "relativo-sem-barra"); // fica como está
        assert_eq!(canonical(""), "");
    }

    // Fixture fiel ao shape verbose do SharePoint (`{ "d": { "results": [...], "__next": ... } }`),
    // com as facetas como `{ "results": [...] }`.
    const SVC_JSON: &str = r#"{
      "Title":"Consulta Pública de Cadastro",
      "Descricao":"Permite ver a ficha cadastral.",
      "URL":"https://www.cadesp.fazenda.sp.gov.br/x.aspx",
      "Home360":{"ID":4},
      "Cidadao":{"results":[]},
      "Empresa":{"results":["Contribuinte de ICMS","Simples Nacional"]},
      "Servidor":{"results":["Servidor da Secretaria da Fazenda"]},
      "Tributo":{"results":["ICMS"]},
      "Acesso":{"results":["Público"]}
    }"#;

    #[test]
    fn parse_verbose_e_facet() {
        let s: SvcRow = serde_json::from_str(SVC_JSON).unwrap();
        assert_eq!(s.title, "Consulta Pública de Cadastro");
        assert_eq!(s.home360.as_ref().unwrap().id, 4);
        assert!(s.facet("Cidadao").is_empty());
        assert_eq!(s.facet("Empresa").len(), 2);
        assert_eq!(s.facet("Servidor"), &["Servidor da Secretaria da Fazenda"]);
        assert!(s.facet("DesconhecidO").is_empty());
    }

    #[test]
    fn parse_resp_segue_results_e_next() {
        let json = format!(r#"{{"d":{{"results":[{}],"__next":"https://x/next"}}}}"#, SVC_JSON);
        let r: Resp<SvcRow> = serde_json::from_str(&json).unwrap();
        assert_eq!(r.d.results.len(), 1);
        assert_eq!(r.d.next.as_deref(), Some("https://x/next"));
        // sem __next -> None
        let json2 = format!(r#"{{"d":{{"results":[{}]}}}}"#, SVC_JSON);
        let r2: Resp<SvcRow> = serde_json::from_str(&json2).unwrap();
        assert!(r2.d.next.is_none());
    }

    #[test]
    fn build_corpo_junta_descricao_e_formas_de_acesso() {
        let s: SvcRow = serde_json::from_str(SVC_JSON).unwrap();
        assert_eq!(build_corpo(&s), "Permite ver a ficha cadastral.\nFormas de acesso: Público");

        // Descrição vazia -> só as formas de acesso.
        let sem_desc: SvcRow =
            serde_json::from_str(r#"{"Descricao":"","Acesso":{"results":["Público","Restrito"]}}"#)
                .unwrap();
        assert_eq!(build_corpo(&sem_desc), "Formas de acesso: Público, Restrito");
    }

    #[test]
    fn build_servico_uma_ocorrencia_por_faceta_com_classe_do_assunto() {
        let s: SvcRow = serde_json::from_str(SVC_JSON).unwrap();
        let assunto: HashMap<i64, String> = [(4i64, "Cadastro".to_string())].into_iter().collect();
        let pubs = publicos();
        let sr = build_servico(&s, &assunto, &pubs).unwrap();
        // Cidadão vazio é pulado; Empresa/Servidor/Tributo preenchidos -> 3 ocorrências.
        let ocs: Vec<(&str, &str)> =
            sr.ocorrencias.iter().map(|o| (o.publico.as_str(), o.classe.as_str())).collect();
        assert_eq!(
            ocs,
            vec![("Empresa", "Cadastro"), ("Servidor Público", "Cadastro"), ("Tributos", "Cadastro")]
        );
        assert_eq!(sr.titulo, "Consulta Pública de Cadastro");
        assert_eq!(sr.orgao, "SEFAZ-SP");
    }

    #[test]
    fn build_servico_sem_faceta_e_none() {
        // Nenhuma faceta de público preenchida -> fora do catálogo (None), mesmo com Acesso.
        let s: SvcRow = serde_json::from_str(
            r#"{"Title":"Interno","Cidadao":{"results":[]},"Acesso":{"results":["Público"]}}"#,
        )
        .unwrap();
        let pubs = publicos();
        assert!(build_servico(&s, &HashMap::new(), &pubs).is_none());
    }

    #[test]
    fn build_servico_sem_home360_classe_vazia() {
        let s: SvcRow =
            serde_json::from_str(r#"{"Title":"X","Cidadao":{"results":["Todos"]}}"#).unwrap();
        let pubs = publicos();
        let sr = build_servico(&s, &HashMap::new(), &pubs).unwrap();
        assert_eq!(sr.ocorrencias[0].publico, "Cidadão");
        assert_eq!(sr.ocorrencias[0].classe, "", "sem Home360 -> classe vazia");
        assert_eq!(sr.link, "", "sem URL -> link vazio (a linha ainda é a identidade)");
    }
}
