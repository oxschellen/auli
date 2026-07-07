//! Coleta dos serviços da SEFAZ-RR a partir do array `apps` embutido no `script.js` da home.
//!
//! O portal (`www.sefaz.rr.gov.br`) é um site custom sem catálogo server-rendered — os serviços são
//! apps GeneXus/SIATE em `portalweb.sefaz.rr.gov.br`. Mas o `script.js` traz um array
//! `const apps = [{category, title, description, href}, …]` (molde AP): catálogo estruturado com
//! descrição curta. 20 entradas, 16 hrefs distintos (4 serviços em `cidadao` E `empresa`).
//!
//! Modelagem: `titulo`=`title`; `descricao`=`description`; **público**=`category` (Cidadão/Empresa);
//! `classe`="Serviços" (sem eixo de tema); `link`=`href` (identidade/dedup); um serviço em 2
//! categorias vira 2 ocorrências.

use std::collections::HashMap;

use anyhow::{Result, bail};
use auli_contract::{Ocorrencia, Publico, ServicoRaw};
use auli_scraper_kit::clean;
use auli_scraper_kit::http::GetOpts;
use regex::Regex;
use std::time::Duration;

const USER_AGENT: &str =
    "AuliBot/0.1 (+https://github.com/oxschellen/auli; carlos.schellenberger@gmail.com)";

const SCRIPT_URL: &str = "https://www.sefaz.rr.gov.br/script.js";
const ORGAO: &str = "SEFAZ-RR";
const CLASSE: &str = "Serviços";
const PUB_CIDADAO: &str = "Cidadão";
const PUB_EMPRESA: &str = "Empresa";
/// Guard: piso de serviços (o array tem 16 hrefs distintos).
const MIN_SERVICOS: usize = 12;

/// Raspa o `script.js` e devolve `(items, publicos_ordem)` prontos para o snapshot v3.
pub fn scrape(
    data_dir: &str,
    use_cache: bool,
) -> Result<(Vec<ServicoRaw>, Vec<Publico>), Box<dyn std::error::Error>> {
    let (js, da_rede) = load(data_dir, use_cache)?;
    let items = parse(&js);
    validar(&items)?;
    if da_rede {
        // Cache só depois de o parse+guard passarem (D-RJ5).
        auli_scraper_kit::cache::write(data_dir, SCRIPT_URL, &js);
    }

    let publicos_ordem = publicos_ordem(&items);
    let ocorrencias: usize = items.iter().map(|s| s.ocorrencias.len()).sum();
    println!(
        "RR: {} serviços ({} ocorrências) em {} público(s)",
        items.len(),
        ocorrencias,
        publicos_ordem.len()
    );
    Ok((items, publicos_ordem))
}

/// GET (JS) com cache. Devolve `(corpo, veio_da_rede)`. Miss + `--usecache` = erro.
fn load(data_dir: &str, use_cache: bool) -> Result<(String, bool), Box<dyn std::error::Error>> {
    if let Some(cached) = auli_scraper_kit::cache::read(data_dir, SCRIPT_URL) {
        return Ok((cached, false));
    }
    if use_cache {
        return Err(format!(
            "cache vazio para {} (--usecache, sem rede). Rode uma coleta com rede primeiro.",
            SCRIPT_URL
        )
        .into());
    }
    let agent = auli_scraper_kit::build_agent(USER_AGENT, Some(Duration::from_secs(30)));
    let body = auli_scraper_kit::http::get_string(
        &agent,
        SCRIPT_URL,
        &GetOpts { log_prefix: "RR", ..Default::default() },
    )?;
    if !body.contains("const apps") {
        return Err(format!("`const apps` sumiu de {} (bundle mudou?)", SCRIPT_URL).into());
    }
    Ok((body, true))
}

/// Parseia o array `apps` (objetos `{category, title, description, href}`), dedup por `href`.
fn parse(js: &str) -> Vec<ServicoRaw> {
    let re = Regex::new(
        r#"(?s)\{\s*category:\s*"([^"]*)",\s*title:\s*"([^"]*)",\s*description:\s*"([^"]*)",\s*href:\s*"([^"]*)""#,
    )
    .unwrap();

    let mut items: Vec<ServicoRaw> = Vec::new();
    let mut pos: HashMap<String, usize> = HashMap::new();
    for cap in re.captures_iter(js) {
        let publico = publico_de(&cap[1]).to_string();
        let titulo = clean(&cap[2]);
        let descricao = clean(&cap[3]);
        let link = cap[4].trim().to_string();
        if titulo.is_empty() || link.is_empty() {
            continue;
        }
        let ocorrencia = Ocorrencia { publico, classe: CLASSE.to_string() };
        if let Some(&i) = pos.get(&link) {
            // href já visto (serviço em 2 categorias) -> acumula o outro público.
            if !items[i].ocorrencias.iter().any(|o| o.publico == ocorrencia.publico) {
                items[i].ocorrencias.push(ocorrencia);
            }
            continue;
        }
        pos.insert(link.clone(), items.len());
        items.push(ServicoRaw {
            titulo,
            descricao,
            link,
            orgao: ORGAO.to_string(),
            ocorrencias: vec![ocorrencia],
        });
    }
    items
}

/// `category` -> rótulo de público da frota.
fn publico_de(category: &str) -> &'static str {
    match category.trim().to_ascii_lowercase().as_str() {
        "cidadao" => PUB_CIDADAO,
        "empresa" => PUB_EMPRESA,
        _ => CLASSE, // fallback improvável ("Serviços")
    }
}

/// Ordem dos públicos = nomes distintos em primeira ocorrência; slug fixo dos 2 públicos conhecidos.
fn publicos_ordem(items: &[ServicoRaw]) -> Vec<Publico> {
    let mut seen: Vec<String> = Vec::new();
    for s in items {
        for o in &s.ocorrencias {
            if !seen.contains(&o.publico) {
                seen.push(o.publico.clone());
            }
        }
    }
    seen.into_iter()
        .map(|nome| {
            let slug = match nome.as_str() {
                PUB_CIDADAO => "servicos-ao-cidadao",
                PUB_EMPRESA => "servicos-a-empresa",
                _ => "servicos-gerais",
            };
            Publico { nome, slug: slug.to_string() }
        })
        .collect()
}

/// Guard (princípio D-RJ5): reprova coleta capada (o array `apps` mudou/sumiu).
fn validar(items: &[ServicoRaw]) -> Result<()> {
    if items.len() < MIN_SERVICOS {
        bail!(
            "catálogo capado? só {} serviço(s) (mínimo {}). O array `apps` do script.js pode ter \
             mudado; se veio do cache, limpe data/rr/raw/cache/ e re-raspe.",
            items.len(),
            MIN_SERVICOS
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const JS: &str = r#"
      function foo() {}
      const apps = [
        { category: "cidadao", title: "Certidão Negativa",
          description: "Emite a certidão negativa de débitos estaduais.",
          href: "https://portalweb.sefaz.rr.gov.br/cnd/servlet/wp_siate_emitir" },
        { category: "empresa", title: "Certidão Negativa de Débitos",
          description: "Emite a certidão negativa de débitos estaduais.",
          href: "https://portalweb.sefaz.rr.gov.br/cnd/servlet/wp_siate_emitir" },
        { category: "empresa", title: "SINTEGRA",
          description: "Consulta pública do cadastro estadual.",
          href: "https://portalweb.sefaz.rr.gov.br/sintegra/servlet/wp_siate_consultasintegra" }
      ];
    "#;

    #[test]
    fn parse_dedup_por_href_acumula_publicos() {
        let items = parse(JS);
        // 2 hrefs distintos: o CND (cidadão + empresa) e o SINTEGRA (empresa).
        assert_eq!(items.len(), 2);
        let cnd = items.iter().find(|s| s.link.ends_with("wp_siate_emitir")).unwrap();
        assert_eq!(cnd.titulo, "Certidão Negativa"); // 1º título vence
        let pubs: Vec<_> = cnd.ocorrencias.iter().map(|o| o.publico.as_str()).collect();
        assert_eq!(pubs, ["Cidadão", "Empresa"]);
        assert_eq!(cnd.ocorrencias[0].classe, "Serviços");
        assert!(cnd.descricao.contains("Emite a certidão negativa"));

        let sint = items.iter().find(|s| s.link.contains("sintegra")).unwrap();
        assert_eq!(sint.ocorrencias.len(), 1);
        assert_eq!(sint.ocorrencias[0].publico, "Empresa");
    }

    #[test]
    fn publicos_ordem_mapeia_slugs() {
        let po = publicos_ordem(&parse(JS));
        assert_eq!(
            po.iter().map(|p| (p.nome.as_str(), p.slug.as_str())).collect::<Vec<_>>(),
            [("Cidadão", "servicos-ao-cidadao"), ("Empresa", "servicos-a-empresa")]
        );
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
