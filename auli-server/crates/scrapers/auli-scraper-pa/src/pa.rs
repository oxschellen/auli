//! Coleta dos serviços da SEFA-PA a partir do catálogo estadual "paradigital" (API Prodepa/Spring).
//!
//! O portal `paradigital` é uma SPA Quasar; a API está em `para-digital.sistemas.pa.gov.br`
//! (`/para-digital-service/portal`). Tudo **GET anônimo** (sem sessão/login). O catálogo é
//! **multi-tenant por órgão**: a SEFA é o **órgão 48**.
//! - `GET /orgao/48` → `[{ id, nome }]` (os serviços da SEFA; a listagem NÃO traz descrição).
//! - `GET /servico/{id}` → o detalhe rico: `finalidade` ("o que é"), `etapaServicos[]` (passo a passo),
//!   `requisitoServicos[]` (requisitos), `contatos[]`, `tema` (categoria), flags `cidadao`/`empresa`/
//!   `estado` (público) e `linkAcesso`. Como a listagem é magra, o detalhe é obrigatório (34 GETs).
//!
//! Modelagem (padrão MS — `ServicoRaw` direto, N ocorrências): identidade = `id`; `descricao` =
//! finalidade + etapas + requisitos + acesso (montada); `classe` = `tema.descricao`; `publico` =
//! {Cidadão/Empresa/Estado} pelas flags (sobrepostos → `ServicoPerPublico`); `link` = a página do
//! serviço no paradigital (`…/servico/{id}`, rota válida da SPA). `orgao` = "SEFA-PA".
//!
//! **D-PA-ROBOTS (mitigações):** UA institucional **AuliBot** (nunca UA de browser falso),
//! **rate-limit ≥1s** entre GETs, cache agressivo, e **nunca autenticar**.

use std::thread::sleep;
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use auli_contract::{Ocorrencia, Publico, ServicoRaw};
use auli_scraper_kit::clean;
use auli_scraper_kit::http::GetOpts;
use serde::Deserialize;

/// UA identificado do projeto (mitigação D-PA-ROBOTS): nunca um UA de browser falso. Contato no UA
/// para que o operador do portal saiba quem somos e como falar conosco.
const USER_AGENT: &str =
    "AuliBot/0.1 (+https://github.com/oxschellen/auli; carlos.schellenberger@gmail.com)";

/// Base da API Prodepa (Spring). `GET` anônimo, respostas JSON.
const API_BASE: &str = "https://para-digital.sistemas.pa.gov.br/para-digital-service/portal";
/// Órgão da SEFA no catálogo estadual (63 órgãos ao todo — ver D-PA-ACERVO).
const ORGAO_ID: &str = "48";
/// Base do link cidadão (página do serviço na SPA paradigital — rota válida `…/servico/{id}`).
const LINK_BASE: &str = "https://www.paradigital.pa.gov.br/servico";
/// Órgão de origem (política da frota: separador sigla–UF com `-`).
const ORGAO: &str = "SEFA-PA";
/// Classe de fallback quando o serviço não traz `tema`.
const CLASSE_FALLBACK: &str = "Geral";
/// Públicos (nome, slug), na ordem de `publicos_ordem`. Mapeiam as flags do detalhe.
const PUBLICOS: [(&str, &str); 3] =
    [("Cidadão", "cidadao"), ("Empresa", "empresa"), ("Estado", "estado")];
/// Cortesia entre GETs (mitigação D-PA-ROBOTS: ≥1s, sem paralelismo).
const COURTESY: Duration = Duration::from_millis(1100);
/// Guard (princípio D-RJ5): mínimo de serviços. Folga sob os 34 observados; rejeita catálogo capado.
const MIN_SERVICOS: usize = 30;

/// Item da listagem por órgão (`GET /orgao/48`): usamos só o `id` (string) — o `nome` e o resto vêm
/// do detalhe. Os demais campos do JSON são ignorados pelo serde.
#[derive(Debug, Deserialize)]
struct OrgaoItem {
    #[serde(default)]
    id: String,
}

/// Detalhe do serviço (`GET /servico/{id}`). Só os campos que usamos; serde ignora o resto.
#[derive(Debug, Deserialize)]
struct Detail {
    #[serde(default)]
    nome: Option<String>,
    #[serde(default)]
    finalidade: Option<String>,
    #[serde(default)]
    cidadao: bool,
    #[serde(default)]
    empresa: bool,
    #[serde(default)]
    estado: bool,
    #[serde(default = "default_true")]
    ativo: bool,
    #[serde(rename = "linkAcesso", default)]
    link_acesso: Option<String>,
    #[serde(default)]
    tema: Option<Tema>,
    #[serde(rename = "etapaServicos", default)]
    etapas: Vec<Texto>,
    #[serde(rename = "requisitoServicos", default)]
    requisitos: Vec<Texto>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
struct Tema {
    #[serde(default)]
    descricao: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Texto {
    #[serde(default)]
    descricao: Option<String>,
}

/// Raspa o catálogo da SEFA e devolve `(items, publicos_ordem)` prontos para o snapshot v3.
pub fn scrape(
    data_dir: &str,
    use_cache: bool,
) -> Result<(Vec<ServicoRaw>, Vec<Publico>), Box<dyn std::error::Error>> {
    let agent = auli_scraper_kit::build_agent(USER_AGENT, Some(Duration::from_secs(30)));

    // Respostas de rede que só entram no cache DEPOIS dos guards (D-RJ5).
    let mut pending: Vec<(String, String)> = Vec::new();

    // 1) Listagem por órgão -> [{id, nome}].
    let lista_url = format!("{}/orgao/{}", API_BASE, ORGAO_ID);
    let lista_raw = load(&agent, data_dir, &lista_url, use_cache, &mut pending)?;
    let lista: Vec<OrgaoItem> = serde_json::from_str(&lista_raw)
        .map_err(|e| anyhow!("JSON da listagem /orgao/{} inválido: {}", ORGAO_ID, e))?;
    println!("PA: {} serviços na listagem do órgão {}", lista.len(), ORGAO_ID);

    // 2) Detalhe rico de cada serviço.
    let mut items: Vec<ServicoRaw> = Vec::new();
    for it in &lista {
        if it.id.trim().is_empty() {
            continue;
        }
        let url = format!("{}/servico/{}", API_BASE, it.id.trim());
        let raw = load(&agent, data_dir, &url, use_cache, &mut pending)?;
        let det: Detail = serde_json::from_str(&raw)
            .map_err(|e| anyhow!("JSON de /servico/{} inválido: {}", it.id, e))?;
        if let Some(s) = build_servico(&it.id, &det) {
            items.push(s);
        }
    }

    validar(&items)?;

    // Cache só DEPOIS dos guards.
    for (url, raw) in &pending {
        auli_scraper_kit::cache::write(data_dir, url, raw);
    }

    let ocorr: usize = items.iter().map(|s| s.ocorrencias.len()).sum();
    println!("PA: {} serviços ({} ocorrências)", items.len(), ocorr);
    let publicos_ordem =
        PUBLICOS.iter().map(|(n, s)| Publico { nome: n.to_string(), slug: s.to_string() }).collect();
    Ok((items, publicos_ordem))
}

/// GET com cache. Miss + `--usecache` = erro (nunca fallback). Rede -> `pending` (cache após guards)
/// e cortesia ≥1s.
fn load(
    agent: &ureq::Agent,
    data_dir: &str,
    url: &str,
    use_cache: bool,
    pending: &mut Vec<(String, String)>,
) -> Result<String> {
    if let Some(cached) = auli_scraper_kit::cache::read(data_dir, url) {
        println!("Cache hit: {}", url);
        return Ok(cached);
    }
    if use_cache {
        bail!("cache vazio para {} (--usecache, sem rede). Rode uma coleta com rede primeiro.", url);
    }
    let body = auli_scraper_kit::http::get_string(
        agent,
        url,
        &GetOpts { log_prefix: "PA", accept: Some("application/json"), ..Default::default() },
    )?;
    if !body.trim_start().starts_with(['[', '{']) {
        bail!("resposta não-JSON de {} — erro/HTML? primeiros bytes: {:?}",
            url, body.chars().take(60).collect::<String>());
    }
    pending.push((url.to_string(), body.clone()));
    sleep(COURTESY);
    Ok(body)
}

/// Monta um `ServicoRaw` do detalhe. `None` se inativo ou sem título.
fn build_servico(id: &str, d: &Detail) -> Option<ServicoRaw> {
    if !d.ativo {
        return None;
    }
    let titulo = clean(d.nome.as_deref().unwrap_or_default());
    if titulo.is_empty() {
        return None;
    }
    let classe = d
        .tema
        .as_ref()
        .and_then(|t| t.descricao.as_deref())
        .map(clean)
        .filter(|c| !c.is_empty())
        .unwrap_or_else(|| CLASSE_FALLBACK.to_string());

    // Público pelas flags; um serviço pode servir a vários (sobreposição).
    let ocorrencias: Vec<Ocorrencia> = PUBLICOS
        .iter()
        .filter(|(nome, _)| match *nome {
            "Cidadão" => d.cidadao,
            "Empresa" => d.empresa,
            "Estado" => d.estado,
            _ => false,
        })
        .map(|(nome, _)| Ocorrencia { publico: (*nome).to_string(), classe: classe.clone() })
        .collect();
    if ocorrencias.is_empty() {
        // Sem flag de público -> ainda é um serviço público; entra como "Cidadão" (não perde o item).
        eprintln!("ℹ️  PA: serviço {} ({}) sem flag de público; assumindo Cidadão.", id, titulo);
        return Some(ServicoRaw {
            descricao: montar_descricao(d),
            link: format!("{}/{}", LINK_BASE, id),
            orgao: ORGAO.to_string(),
            ocorrencias: vec![Ocorrencia { publico: "Cidadão".to_string(), classe }],
            titulo,
        });
    }

    Some(ServicoRaw {
        descricao: montar_descricao(d),
        link: format!("{}/{}", LINK_BASE, id),
        orgao: ORGAO.to_string(),
        ocorrencias,
        titulo,
    })
}

/// Descrição rica: finalidade + "Como proceder" (etapas) + "Requisitos" + "Acesso". Cada peça é
/// limpa isoladamente (single-line) e as quebras são reinseridas aqui — o `clean` comprime espaços.
fn montar_descricao(d: &Detail) -> String {
    let mut partes: Vec<String> = Vec::new();

    let fin = clean(d.finalidade.as_deref().unwrap_or_default());
    if !fin.is_empty() {
        partes.push(fin);
    }
    let etapas = linhas(&d.etapas);
    if !etapas.is_empty() {
        partes.push(format!("Como proceder:\n{}", etapas.join("\n")));
    }
    let reqs = linhas(&d.requisitos);
    if !reqs.is_empty() {
        partes.push(format!("Requisitos:\n{}", reqs.join("\n")));
    }
    if let Some(l) = d.link_acesso.as_deref() {
        let l = l.trim();
        if !l.is_empty() {
            partes.push(format!("Acesso: {}", l));
        }
    }
    partes.join("\n\n")
}

/// Limpa cada `descricao` de uma lista e descarta as vazias.
fn linhas(xs: &[Texto]) -> Vec<String> {
    xs.iter()
        .filter_map(|x| {
            let t = clean(x.descricao.as_deref().unwrap_or_default());
            if t.is_empty() { None } else { Some(t) }
        })
        .collect()
}

/// Guard (princípio D-RJ5): reprova catálogo capado (abaixo do mínimo — falha de API/parse).
fn validar(items: &[ServicoRaw]) -> Result<()> {
    if items.len() < MIN_SERVICOS {
        bail!(
            "catálogo capado? só {} serviço(s) (mínimo {}). Se veio do cache, limpe data/pa/raw/cache/ \
             e re-raspe.",
            items.len(),
            MIN_SERVICOS
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const DET_JSON: &str = r#"{
      "idServico": 458, "nome": "  Alteração de  Dados Cadastrais ",
      "finalidade": "Alterar os dados cadastrais.",
      "cidadao": true, "empresa": true, "estado": false, "ativo": true,
      "linkAcesso": "https://app.sefa.pa.gov.br/x",
      "tema": {"idTema": 60, "descricao": "Tributos e empresas"},
      "etapaServicos": [{"idEtapaServico":1,"descricao":"a) Reunir documentos."},
                        {"idEtapaServico":2,"descricao":"b) Protocolar."}],
      "requisitoServicos": [{"idRequisitoServico":9,"descricao":"Apresentar documentos."}]
    }"#;

    fn det(json: &str) -> Detail {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn build_mapeia_publico_classe_link() {
        let s = build_servico("458", &det(DET_JSON)).unwrap();
        assert_eq!(s.titulo, "Alteração de Dados Cadastrais"); // clean comprime espaços
        assert_eq!(s.link, "https://www.paradigital.pa.gov.br/servico/458");
        assert_eq!(s.orgao, "SEFA-PA");
        // cidadao+empresa (estado=false) -> 2 ocorrências, mesma classe.
        assert_eq!(s.ocorrencias.len(), 2);
        assert_eq!(s.ocorrencias[0].publico, "Cidadão");
        assert_eq!(s.ocorrencias[1].publico, "Empresa");
        assert!(s.ocorrencias.iter().all(|o| o.classe == "Tributos e empresas"));
    }

    #[test]
    fn descricao_monta_secoes() {
        let d = montar_descricao(&det(DET_JSON));
        assert!(d.starts_with("Alterar os dados cadastrais."));
        assert!(d.contains("Como proceder:\na) Reunir documentos.\nb) Protocolar."));
        assert!(d.contains("Requisitos:\nApresentar documentos."));
        assert!(d.contains("Acesso: https://app.sefa.pa.gov.br/x"));
    }

    #[test]
    fn inativo_vira_none() {
        let json = DET_JSON.replace("\"ativo\": true", "\"ativo\": false");
        assert!(build_servico("458", &det(&json)).is_none());
    }

    #[test]
    fn sem_publico_assume_cidadao() {
        let json = DET_JSON
            .replace("\"cidadao\": true", "\"cidadao\": false")
            .replace("\"empresa\": true", "\"empresa\": false");
        let s = build_servico("1", &det(&json)).unwrap();
        assert_eq!(s.ocorrencias.len(), 1);
        assert_eq!(s.ocorrencias[0].publico, "Cidadão");
    }

    #[test]
    fn sem_tema_usa_fallback() {
        let json = r#"{"nome":"X","finalidade":"f","cidadao":true,"ativo":true}"#;
        let s = build_servico("7", &det(json)).unwrap();
        assert_eq!(s.ocorrencias[0].classe, "Geral");
    }

    #[test]
    fn campos_null_nao_quebram() {
        // finalidade/linkAcesso/nome de sub-itens podem vir null.
        let json = r#"{"nome":"Serviço Y","finalidade":null,"linkAcesso":null,"cidadao":true,"ativo":true,
          "etapaServicos":[{"idEtapaServico":1,"descricao":null}]}"#;
        let s = build_servico("2", &det(json)).unwrap();
        assert_eq!(s.titulo, "Serviço Y");
        assert_eq!(s.descricao, ""); // finalidade null, etapa null, sem acesso
    }

    #[test]
    fn lista_parseia_id_string() {
        let arr: Vec<OrgaoItem> =
            serde_json::from_str(r#"[{"nome":"A","id":"458"},{"nome":"B ","id":"1030"}]"#).unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0].id, "458");
    }

    #[test]
    fn validar_reprova_capado() {
        let poucos = vec![ServicoRaw {
            titulo: "x".into(),
            descricao: String::new(),
            link: "l".into(),
            orgao: ORGAO.into(),
            ocorrencias: vec![],
        }];
        assert!(validar(&poucos).unwrap_err().to_string().contains("capado"));
    }
}
