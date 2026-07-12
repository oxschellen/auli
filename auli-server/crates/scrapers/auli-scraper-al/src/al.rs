//! Coleta dos serviços da SEFAZ-AL a partir do **Portal Alagoas Digital** (`alagoasdigital.al.gov.br`).
//!
//! A SEFAZ-AL não tem portal próprio: seus serviços vivem no catálogo estadual, exposto por uma **API
//! REST pública "Dados Abertos"** (sem auth). Coleta: `organs.json` (deriva o UUID da SEFAZ) →
//! `services.json?organ_id={UUID}` (stubs) → `services/{id}.json` (detalhe rico). Sem headless.
//!
//! Modelagem (validada em descobertas.md#al): `titulo` = `name`; `descricao` = `description` + prazo +
//! etapas (com canais) + requisitos + outras informações (tudo **HTML+entidades** → `html_to_text`);
//! **público** = `audiences[]` (vocab controlado — NÃO `applicants[].type`, que é texto livre);
//! `classe` = `categories[]`; `link` = `url`. Guardas dinâmicas: derivar o UUID (não hardcodar),
//! ler o tamanho da lista em runtime, bail se vazia, exigir `organ == UUID` em todo stub.

use std::collections::HashSet;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use auli_contract::{Ocorrencia, Publico, ServicoRaw};
use auli_scraper_kit::clean;
use auli_scraper_kit::http::GetOpts;
use scraper::Html;
use serde_json::Value;
use ureq::Agent;

const USER_AGENT: &str =
    "AuliBot/0.1 (+https://github.com/oxschellen/auli; carlos.schellenberger@gmail.com)";

const API: &str = "https://alagoasdigital.al.gov.br/api/v1";
const SITE: &str = "https://www.alagoasdigital.al.gov.br";
const ORGAO: &str = "SEFAZ-AL";
const CLASSE_FALLBACK: &str = "Geral";
/// Público fallback p/ serviço sem `audiences`. NÃO usar "Serviços" — seu slug `servicos` colidiria
/// com o arquivo agregado `al-servicos.json` (reservado). "Contribuinte" (slug `contribuinte`) é
/// semântico p/ a SEFAZ e não colide.
const PUBLICO_FALLBACK: &str = "Contribuinte";
/// Cortesia entre GETs (API cívica; a lista + ~60 detalhes).
const COURTESY: Duration = Duration::from_millis(500);
/// Guard: piso de serviços (a SEFAZ tem ~60). Não é o contrato — só detecta coleta capada.
const MIN_SERVICOS: usize = 40;

/// Raspa a API e devolve `(items, publicos_ordem)` prontos para o snapshot v3.
pub fn scrape(
    data_dir: &str,
    use_cache: bool,
) -> Result<(Vec<ServicoRaw>, Vec<Publico>), Box<dyn std::error::Error>> {
    let agent = auli_scraper_kit::build_agent(USER_AGENT, Some(Duration::from_secs(30)));
    let mut pending: Vec<(String, String)> = Vec::new();

    // 1) Deriva o UUID da SEFAZ (não hardcodado).
    let organs = load(&agent, data_dir, &format!("{}/organs.json", API), use_cache, &mut pending)?;
    let uuid = derive_sefaz_uuid(&organs)?;
    println!("AL: SEFAZ organ_id = {}", uuid);

    // 2) Lista filtrada por órgão + guardas de coerência.
    let lista_url = format!("{}/services.json?organ_id={}", API, uuid);
    let lista = load(&agent, data_dir, &lista_url, use_cache, &mut pending)?;
    let stubs = parse_stubs(&lista, &uuid)?;
    println!("AL: {} serviços na lista da SEFAZ", stubs.len());

    // 3) Detalhe de cada serviço.
    let mut items: Vec<ServicoRaw> = Vec::new();
    let mut vistos: HashSet<String> = HashSet::new();
    for id in &stubs {
        let url = format!("{}/services/{}.json", API, id);
        let det = load(&agent, data_dir, &url, use_cache, &mut pending)?;
        let d: Value = serde_json::from_str(&det).map_err(|e| format!("detalhe {id} inválido: {e}"))?;
        let servico = build_servico(&d);
        if servico.titulo.is_empty() || !vistos.insert(servico.link.clone()) {
            continue;
        }
        items.push(servico);
    }

    validar(&items)?;

    for (url, raw) in &pending {
        auli_scraper_kit::cache::write(data_dir, url, raw);
    }

    let publicos_ordem = publicos_ordem(&items);
    let ocorrencias: usize = items.iter().map(|s| s.ocorrencias.len()).sum();
    println!(
        "AL: {} serviços ({} ocorrências) em {} público(s)",
        items.len(),
        ocorrencias,
        publicos_ordem.len()
    );
    Ok((items, publicos_ordem))
}

/// GET (JSON) com cache. Miss + `--usecache` = erro. Rede -> `pending` + cortesia.
fn load(
    agent: &Agent,
    data_dir: &str,
    url: &str,
    use_cache: bool,
    pending: &mut Vec<(String, String)>,
) -> Result<String> {
    if let Some(cached) = auli_scraper_kit::cache::read(data_dir, url) {
        return Ok(cached);
    }
    if use_cache {
        bail!("cache vazio para {} (--usecache, sem rede). Rode uma coleta com rede primeiro.", url);
    }
    let body = auli_scraper_kit::http::get_string(
        agent,
        url,
        &GetOpts { log_prefix: "AL", accept: Some("application/json"), ..Default::default() },
    )?;
    pending.push((url.to_string(), body.clone()));
    sleep(COURTESY);
    Ok(body)
}

/// Deriva o `organ_id` da SEFAZ de `organs.json`: `acronym == SEFAZ` e `nature == Estadual`. Exige
/// exatamente 1 match (0 = fonte quebrada; >1 = ambíguo) — não hardcodar o UUID (lição CE).
fn derive_sefaz_uuid(organs_json: &str) -> Result<String> {
    let v: Value = serde_json::from_str(organs_json).map_err(|e| anyhow!("organs.json inválido: {e}"))?;
    let arr = as_list(&v).ok_or_else(|| anyhow!("organs.json não é uma lista"))?;
    let matches: Vec<&Value> = arr
        .iter()
        .filter(|o| eq_ci(str_field(o, "acronym"), "SEFAZ") && eq_ci(str_field(o, "nature"), "Estadual"))
        .collect();
    match matches.as_slice() {
        [o] => {
            let id = str_field(o, "id");
            if id.is_empty() {
                bail!("SEFAZ encontrada mas sem `id` em organs.json");
            }
            Ok(id.to_string())
        }
        [] => bail!("SEFAZ não encontrada em organs.json (acronym=SEFAZ, nature=Estadual)"),
        m => bail!("{} órgãos batem SEFAZ/Estadual — ambíguo, não dá para derivar o UUID", m.len()),
    }
}

/// Parseia os stubs da lista filtrada. Guarda de coerência: todo item deve ter `organ == uuid`
/// (senão o filtro falhou silenciosamente). Bail se a lista vier vazia.
fn parse_stubs(lista_json: &str, uuid: &str) -> Result<Vec<String>> {
    let v: Value = serde_json::from_str(lista_json).map_err(|e| anyhow!("services.json inválido: {e}"))?;
    let arr = as_list(&v).ok_or_else(|| anyhow!("services.json não é uma lista"))?;
    if arr.is_empty() {
        bail!("lista da SEFAZ veio vazia — fonte quebrada ou UUID mudou (não 'SEFAZ sem serviços')");
    }
    let mut ids = Vec::new();
    for s in arr {
        let organ = str_field(s, "organ");
        if organ != uuid {
            bail!("filtro incoerente: serviço com organ={} != SEFAZ {} — filtro falhou", organ, uuid);
        }
        let id = str_field(s, "id");
        if !id.is_empty() {
            ids.push(id.to_string());
        }
    }
    Ok(ids)
}

/// Monta o `ServicoRaw` de um detalhe. `ocorrencias` = `audiences[]` (público) × `categories[]`
/// (classe), dedup; fallbacks se algum eixo vier vazio.
fn build_servico(d: &Value) -> ServicoRaw {
    let titulo = html_to_text(str_field(d, "name"));
    let link = {
        let url = str_field(d, "url");
        if url.starts_with("http") {
            url.to_string()
        } else {
            format!("{}{}", SITE, str_field(d, "url_relativa"))
        }
    };
    let descricao = build_descricao(d);

    let mut publicos = nomes(d, "audiences");
    if publicos.is_empty() {
        publicos.push(PUBLICO_FALLBACK.to_string());
    }
    let mut classes = nomes(d, "categories");
    if classes.is_empty() {
        classes.push(CLASSE_FALLBACK.to_string());
    }
    let mut ocorrencias = Vec::new();
    for publico in &publicos {
        for classe in &classes {
            ocorrencias.push(Ocorrencia { publico: publico.clone(), classe: classe.clone() });
        }
    }

    ServicoRaw { titulo, descricao, link, orgao: ORGAO.to_string(), ocorrencias }
}

/// Descrição rica: `description` + prazo (`estimated_time`) + etapas (`steps`, com canais) +
/// requisitos (`applicants[].requirements` não-vazios) + outras informações. Tudo HTML → texto.
fn build_descricao(d: &Value) -> String {
    let mut linhas: Vec<String> = Vec::new();

    let desc = html_to_text(str_field(d, "description"));
    if !desc.is_empty() {
        linhas.push(desc);
    }

    if let Some(et) = d.get("estimated_time") {
        let prazo = html_to_text(str_field(et, "description"));
        if !prazo.is_empty() {
            linhas.push(format!("Prazo: {}", prazo));
        }
    }

    if let Some(steps) = d.get("steps").and_then(Value::as_array) {
        for st in steps {
            let titulo = html_to_text(str_field(st, "title"));
            let corpo = html_to_text(str_field(st, "description"));
            let mut linha = match (titulo.is_empty(), corpo.is_empty()) {
                (false, false) => format!("- {}: {}", titulo, corpo),
                (false, true) => format!("- {}", titulo),
                (true, false) => format!("- {}", corpo),
                (true, true) => continue,
            };
            let canais: Vec<String> = st
                .get("providing_channels")
                .and_then(Value::as_array)
                .map(|cs| {
                    cs.iter()
                        .map(|c| {
                            let ty = str_field(c, "type");
                            let cd = html_to_text(str_field(c, "description"));
                            if cd.is_empty() { ty.to_string() } else { format!("{} {}", ty, cd) }
                        })
                        .filter(|s| !s.trim().is_empty())
                        .collect()
                })
                .unwrap_or_default();
            if !canais.is_empty() {
                linha.push_str(&format!(" (Canais: {})", canais.join("; ")));
            }
            linhas.push(linha);
        }
    }

    let reqs: Vec<String> = d
        .get("applicants")
        .and_then(Value::as_array)
        .map(|aps| {
            aps.iter()
                .filter_map(|a| {
                    let r = html_to_text(str_field(a, "requirements"));
                    if r.is_empty() {
                        return None;
                    }
                    let ty = html_to_text(str_field(a, "type"));
                    Some(if ty.is_empty() { r } else { format!("{}: {}", ty, r) })
                })
                .collect()
        })
        .unwrap_or_default();
    if !reqs.is_empty() {
        linhas.push(format!("Requisitos: {}", reqs.join(" | ")));
    }

    let outras = html_to_text(str_field(d, "other_informations"));
    if !outras.is_empty() {
        linhas.push(format!("Informações adicionais: {}", outras));
    }

    linhas.join("\n")
}

/// Nomes (`name`) de um array de taxonomia (`audiences`/`categories`), decodificados, dedup na ordem.
fn nomes(d: &Value, campo: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    if let Some(arr) = d.get(campo).and_then(Value::as_array) {
        for x in arr {
            let nome = html_to_text(x.get("name").and_then(Value::as_str).unwrap_or(""));
            if !nome.is_empty() && !out.contains(&nome) {
                out.push(nome);
            }
        }
    }
    out
}

/// Ordem dos públicos = nomes distintos em primeira ocorrência; slug via [`slugify`].
fn publicos_ordem(items: &[ServicoRaw]) -> Vec<Publico> {
    let mut seen: Vec<String> = Vec::new();
    for s in items {
        for o in &s.ocorrencias {
            if !seen.contains(&o.publico) {
                seen.push(o.publico.clone());
            }
        }
    }
    seen.into_iter().map(|nome| Publico { slug: slugify(&nome), nome }).collect()
}

/// Slug de arquivo a partir do nome do público (deacentua, minúsculo, não-alfanumérico → `-`).
fn slugify(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        let d = deaccent(c);
        if d.is_ascii_alphanumeric() {
            out.extend(d.to_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

/// Deacentua as vogais/ç comuns do português; demais caracteres passam inalterados.
fn deaccent(c: char) -> char {
    match c {
        'á' | 'à' | 'â' | 'ã' | 'ä' | 'Á' | 'À' | 'Â' | 'Ã' | 'Ä' => 'a',
        'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' | 'Î' | 'Ï' => 'i',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' | 'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' => 'o',
        'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Û' | 'Ü' => 'u',
        'ç' | 'Ç' => 'c',
        other => other,
    }
}

/// String em uma chave (`""` se ausente/não-string).
fn str_field<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(Value::as_str).unwrap_or("")
}

/// Igualdade case-insensitive (ASCII) já aparando espaços.
fn eq_ci(a: &str, b: &str) -> bool {
    a.trim().eq_ignore_ascii_case(b.trim())
}

/// A lista de um JSON que pode ser um array no topo ou um objeto embrulhando um array.
fn as_list(v: &Value) -> Option<&Vec<Value>> {
    v.as_array()
        .or_else(|| v.as_object().and_then(|m| m.values().find_map(Value::as_array)))
}

/// HTML -> texto: tags viram espaço, entidades decodificadas (html5ever), clean. Faz o strip de tags
/// **duas vezes** — antes e depois da decodificação — porque a fonte às vezes traz tags
/// entity-encodadas (`&lt;b&gt;`) DENTRO do HTML: o html5ever as revela como `<b>` literal, que o
/// segundo strip remove.
fn html_to_text(html: &str) -> String {
    let once = strip_tags(html);
    let decoded: String = Html::parse_fragment(&once).root_element().text().collect();
    clean(&strip_tags(&decoded))
}

/// Remove `<…>` (tag → espaço), preservando o texto fora das tags.
fn strip_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

/// Guard (princípio D-RJ5): reprova coleta capada (fonte/filtro mudou).
fn validar(items: &[ServicoRaw]) -> Result<()> {
    if items.len() < MIN_SERVICOS {
        bail!(
            "catálogo capado? só {} serviço(s) (mínimo {}). A API/filtro pode ter mudado; se veio do \
             cache, limpe data/al/raw/cache/ e re-raspe.",
            items.len(),
            MIN_SERVICOS
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ORGANS: &str = r#"[
      {"id":"e1799779-d21d-411e-8387-03cbc106c6c1","name":"Secretaria da Fazenda","active":"true","acronym":"SEFAZ","nature":"Estadual"},
      {"id":"596e172e8c36c7000","name":"Agência Reguladora","active":"true","acronym":"ARSAL","nature":"Estadual"},
      {"id":"outro","name":"Sefaz Municipal","active":"false","acronym":"SEFAZ","nature":"Municipal"}
    ]"#;

    const DETALHE: &str = r#"{
      "id":"588fba6f",
      "name":"Regulariza&ccedil;&atilde;o Simples Nacional - SEFAZ",
      "active":true,
      "url":"http://www.alagoasdigital.al.gov.br/servico/588fba6f/regularizacao",
      "description":"<p>Permite regularizar pend&ecirc;ncias.</p>",
      "estimated_time":{"min":null,"max":null,"description":"<p>Imediato</p>","unit":"","type":"until"},
      "steps":[
        {"title":"Acessar o portal","description":"<p>Fa&ccedil;a login.</p>","cost":{"value":"Gratuito"},
         "providing_channels":[{"description":"https://contribuinte.sefaz.al.gov.br","type":"WEB"},{"description":"","type":"TELEFONE"}]}
      ],
      "applicants":[{"type":"Contribuinte","requirements":"<p>login e senha</p>"},{"type":"Procurador","requirements":""}],
      "audiences":[{"id":"1","name":"Empresa"},{"id":"2","name":"Cidad&atilde;o"}],
      "categories":[{"id":"9","name":"Economia e Finan&ccedil;as"}],
      "other_informations":"<p>Lei n&ordm; 5900.</p>"
    }"#;

    #[test]
    fn derive_sefaz_uuid_exige_estadual() {
        // Ignora o ARSAL e a SEFAZ Municipal; pega a Estadual.
        assert_eq!(derive_sefaz_uuid(ORGANS).unwrap(), "e1799779-d21d-411e-8387-03cbc106c6c1");
        assert!(derive_sefaz_uuid("[]").unwrap_err().to_string().contains("não encontrada"));
    }

    #[test]
    fn parse_stubs_exige_coerencia_de_organ() {
        let ok = r#"[{"id":"a","organ":"U"},{"id":"b","organ":"U"}]"#;
        assert_eq!(parse_stubs(ok, "U").unwrap(), vec!["a", "b"]);
        // um stub fora do órgão -> bail (filtro falhou)
        let bad = r#"[{"id":"a","organ":"U"},{"id":"b","organ":"X"}]"#;
        assert!(parse_stubs(bad, "U").unwrap_err().to_string().contains("incoerente"));
        assert!(parse_stubs("[]", "U").unwrap_err().to_string().contains("vazia"));
    }

    #[test]
    fn build_servico_monta_campos_e_ocorrencias() {
        let d: Value = serde_json::from_str(DETALHE).unwrap();
        let s = build_servico(&d);
        assert_eq!(s.titulo, "Regularização Simples Nacional - SEFAZ");
        assert!(s.link.starts_with("http://www.alagoasdigital.al.gov.br/servico/588fba6f/"));
        // descrição rica, com entidades decodificadas e sem tags
        assert!(s.descricao.contains("Permite regularizar pendências."));
        assert!(s.descricao.contains("Prazo: Imediato"));
        assert!(s.descricao.contains("- Acessar o portal: Faça login."));
        assert!(s.descricao.contains("Canais: WEB https://contribuinte.sefaz.al.gov.br; TELEFONE"));
        assert!(s.descricao.contains("Requisitos: Contribuinte: login e senha"));
        assert!(s.descricao.contains("Informações adicionais: Lei nº 5900."));
        assert!(!s.descricao.contains('<') && !s.descricao.contains("&ccedil;"));
        // público de audiences (não de applicants.type), classe de categories, cartesiano
        let pares: Vec<_> =
            s.ocorrencias.iter().map(|o| (o.publico.as_str(), o.classe.as_str())).collect();
        assert_eq!(pares, [("Empresa", "Economia e Finanças"), ("Cidadão", "Economia e Finanças")]);
    }

    #[test]
    fn html_to_text_remove_tags_entity_encodadas() {
        // Tag real (<p>) + tag entity-encodada (&lt;b&gt;) dentro do HTML → ambas somem (a tag vira
        // espaço, como no restante da frota — daí o espaço antes da vírgula).
        let d = html_to_text("<p>Guia pode ser &lt;b&gt;manuscrita&lt;/b&gt;, sem carimbo.</p>");
        assert_eq!(d, "Guia pode ser manuscrita , sem carimbo.");
        assert!(!d.contains('<') && !d.contains("&lt;"));
    }

    #[test]
    fn slugify_deacentua() {
        assert_eq!(slugify("Cidadão"), "cidadao");
        assert_eq!(slugify("Produtor Rural"), "produtor-rural");
        assert_eq!(slugify("Setor Público"), "setor-publico");
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
