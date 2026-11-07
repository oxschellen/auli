//! Coleta dos serviços da SEFA-PA (D-PASERV-1, opção C): o catálogo `pis` do site novo (`pis.rs`) e os serviços
//! que só o catálogo estadual "paradigital" tem.
//!
//! O paradigital (API Prodepa/Spring, `para-digital.sistemas.pa.gov.br/para-digital-service/portal`, GET anônimo) é
//! multi-tenant por órgão (a SEFA é o 48). Dos 34 serviços da SEFA lá, 31 estão no `pis` com o mesmo nome ou outro;
//! os 3 que não estão vêm de `GET /servico/{id}`, com o conteúdo de lá: `finalidade`, `etapaServicos[]`,
//! `requisitoServicos[]` e `linkAcesso`.
//!
//! Público único "Serviços" (D-PASERV-2): o `pis` não diz para quem é o serviço. `orgao` = "SEFA-PA".
//!
//! **D-PA-ROBOTS / D-PASERV-4:** UA AuliBot, ≥1s entre GETs, sempre à rede; o cache só é lido com `--usecache` e só
//! é gravado depois das guardas.

use std::thread::sleep;
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use auli_contract::{Ocorrencia, Publico, ServicoRaw};
use auli_scraper_kit::clean;
use auli_scraper_kit::http::GetOpts;
use serde::Deserialize;

use crate::pis;

/// Base da API Prodepa (Spring). `GET` anônimo, respostas JSON.
const API_BASE: &str = "https://para-digital.sistemas.pa.gov.br/para-digital-service/portal";
/// Base do link cidadão (página do serviço na SPA paradigital — rota válida `…/servico/{id}`).
const LINK_BASE: &str = "https://www.paradigital.pa.gov.br/servico";
/// Órgão de origem (política da frota: separador sigla–UF com `-`).
pub(crate) const ORGAO: &str = "SEFA-PA";
/// O público único (D-PASERV-2). O slug não pode ser `servicos`: `pa-servicos.json` é o agregado.
const PUBLICO_NOME: &str = "Serviços";
const PUBLICO_SLUG: &str = "servicos-gerais";
/// Os serviços que só o paradigital tem (07/11/2026): `(id, classe)`. A classe é o tema do `pis` em que estariam.
const SO_PARADIGITAL: [(&str, &str); 3] = [
    ("1034", "IPVA"), // Dispensa de pagamento IPVA
    ("1115", "IPVA"), // Pedido de Ressarcimento do IPVA
    ("1116", "IPVA"), // Registro de veículo novo
];
/// Cortesia entre GETs (mitigação D-PA-ROBOTS: ≥1s, sem paralelismo).
const COURTESY: Duration = Duration::from_millis(1100);

/// Detalhe do serviço (`GET /servico/{id}`). Só os campos que usamos; serde ignora o resto.
#[derive(Debug, Deserialize)]
struct Detail {
    #[serde(default)]
    nome: Option<String>,
    #[serde(default)]
    finalidade: Option<String>,
    #[serde(default = "default_true")]
    ativo: bool,
    #[serde(rename = "linkAcesso", default)]
    link_acesso: Option<String>,
    #[serde(rename = "etapaServicos", default)]
    etapas: Vec<Texto>,
    #[serde(rename = "requisitoServicos", default)]
    requisitos: Vec<Texto>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
struct Texto {
    #[serde(default)]
    descricao: Option<String>,
}

/// Raspa o `pis` e os serviços só do paradigital e devolve `(items, publicos_ordem)` prontos para o snapshot v3.
pub fn scrape(
    data_dir: &str,
    use_cache: bool,
) -> Result<(Vec<ServicoRaw>, Vec<Publico>), Box<dyn std::error::Error>> {
    let agent = auli_scraper_kit::build_agent(crate::UA, Some(Duration::from_secs(30)));

    // Respostas de rede que só entram no cache DEPOIS das guardas (D-RJ5).
    let mut pending: Vec<(String, String)> = Vec::new();

    // 1) O catálogo `pis`: os temas e as páginas.
    let cats = pis::categorias(&load(
        &agent,
        data_dir,
        &pis::url_categorias(),
        use_cache,
        &mut pending,
    )?)?;
    let paginas = pis::paginas(|n| {
        load(
            &agent,
            data_dir,
            &pis::url_pagina(n),
            use_cache,
            &mut pending,
        )
    })?;
    let mut items = pis::servicos(&paginas, &cats, PUBLICO_NOME)?;
    pis::validar(&items)?;
    println!(
        "PA: {} páginas no pis, {} serviços distintos",
        paginas.len(),
        items.len()
    );

    // 2) Os que só o paradigital tem.
    for (id, classe) in SO_PARADIGITAL {
        let url = format!("{}/servico/{}", API_BASE, id);
        let raw = load(&agent, data_dir, &url, use_cache, &mut pending)?;
        let det: Detail = serde_json::from_str(&raw)
            .map_err(|e| anyhow!("JSON de /servico/{} inválido: {}", id, e))?;
        let Some(s) = build_servico(id, &det, classe) else {
            eprintln!(
                "ℹ️  PA: paradigital {id} inativo ou sem nome — fica de fora (SO_PARADIGITAL)."
            );
            continue;
        };
        if items
            .iter()
            .any(|x| pis::chave(&x.titulo) == pis::chave(&s.titulo))
        {
            eprintln!(
                "ℹ️  PA: \"{}\" (paradigital {id}) já está no pis — fica o do pis (SO_PARADIGITAL).",
                s.titulo
            );
            continue;
        }
        items.push(s);
    }

    // Cache só DEPOIS das guardas.
    for (url, raw) in &pending {
        auli_scraper_kit::cache::write(data_dir, "servicos", url, raw);
    }

    let ocorr: usize = items.iter().map(|s| s.ocorrencias.len()).sum();
    println!("PA: {} serviços ({} ocorrências)", items.len(), ocorr);
    let publicos_ordem = vec![Publico {
        nome: PUBLICO_NOME.to_string(),
        slug: PUBLICO_SLUG.to_string(),
    }];
    Ok((items, publicos_ordem))
}

/// GET JSON. `--usecache` lê só do cache (miss = erro, nunca rede); sem ele, sempre à rede, com a resposta em
/// `pending` (cache após as guardas) e cortesia ≥1s.
fn load(
    agent: &ureq::Agent,
    data_dir: &str,
    url: &str,
    use_cache: bool,
    pending: &mut Vec<(String, String)>,
) -> Result<String> {
    if use_cache {
        return auli_scraper_kit::cache::read(data_dir, "servicos", url).ok_or_else(|| {
            anyhow!(
                "cache vazio para {} (--usecache, sem rede). Rode uma coleta com rede primeiro.",
                url
            )
        });
    }
    let body = auli_scraper_kit::http::get_string(
        agent,
        url,
        &GetOpts {
            log_prefix: "PA",
            accept: Some("application/json"),
            ..Default::default()
        },
    )?;
    if !body.trim_start().starts_with(['[', '{']) {
        bail!(
            "resposta não-JSON de {} — erro/HTML? primeiros bytes: {:?}",
            url,
            body.chars().take(60).collect::<String>()
        );
    }
    pending.push((url.to_string(), body.clone()));
    sleep(COURTESY);
    Ok(body)
}

/// Monta um `ServicoRaw` do detalhe do paradigital, com o público único e a `classe` dada. `None` se inativo ou sem
/// título.
fn build_servico(id: &str, d: &Detail, classe: &str) -> Option<ServicoRaw> {
    if !d.ativo {
        return None;
    }
    let titulo = clean(d.nome.as_deref().unwrap_or_default());
    if titulo.is_empty() {
        return None;
    }
    Some(ServicoRaw {
        descricao: montar_descricao(d),
        link: format!("{}/{}", LINK_BASE, id),
        orgao: ORGAO.to_string(),
        ocorrencias: vec![Ocorrencia {
            publico: PUBLICO_NOME.to_string(),
            classe: classe.to_string(),
        }],
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
    fn build_publico_unico_e_a_classe_dada() {
        let s = build_servico("458", &det(DET_JSON), "IPVA").unwrap();
        assert_eq!(s.titulo, "Alteração de Dados Cadastrais"); // clean comprime espaços
        assert_eq!(s.link, "https://www.paradigital.pa.gov.br/servico/458");
        assert_eq!(s.orgao, "SEFA-PA");
        // As flags cidadao/empresa e o tema do paradigital não contam: um público, a classe do SO_PARADIGITAL.
        assert_eq!(s.ocorrencias.len(), 1);
        assert_eq!(s.ocorrencias[0].publico, "Serviços");
        assert_eq!(s.ocorrencias[0].classe, "IPVA");
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
        assert!(build_servico("458", &det(&json), "IPVA").is_none());
    }

    #[test]
    fn campos_null_nao_quebram() {
        // finalidade/linkAcesso/nome de sub-itens podem vir null.
        let json = r#"{"nome":"Serviço Y","finalidade":null,"linkAcesso":null,"cidadao":true,"ativo":true,
          "etapaServicos":[{"idEtapaServico":1,"descricao":null}]}"#;
        let s = build_servico("2", &det(json), "IPVA").unwrap();
        assert_eq!(s.titulo, "Serviço Y");
        assert_eq!(s.descricao, ""); // finalidade null, etapa null, sem acesso
    }
}
