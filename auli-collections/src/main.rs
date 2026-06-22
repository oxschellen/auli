mod domain;
mod errors;
mod faqs;
mod servicos;

use domain::entities::{EntityConfig, get_entity};

fn main() -> errors::Result<()> {
    // CLI: `cargo run [--usecache] <entity> <collection>`
    //   <entity>     entity id (e.g. `rs`); empty/omitted -> default entity.
    //   <collection> collection kind to scrape (`faqs` | `servicos`); omitted -> `faqs`.
    //   --usecache   run offline: use only cached pages, never hit the network (a cache miss is an error).
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let use_cache = raw.iter().any(|a| a == "--usecache");
    let mut positional = raw.iter().filter(|a| !a.starts_with("--"));
    let entity_arg = positional.next().cloned();
    let collection = positional
        .next()
        .cloned()
        .unwrap_or_else(|| "faqs".to_string());

    // Resolve (and validate) the entity; unknown ids produce a friendly Portuguese error.
    let entity = get_entity(entity_arg.as_deref())?;
    println!("🏛️  Entidade: {} ({})", entity.id, entity.name);
    println!("📚 Coleção: {}", collection);
    if use_cache {
        println!("📦 --usecache: usando apenas páginas em cache (sem rede).");
    }

    match collection.as_str() {
        "faqs" => run_faqs(entity, use_cache)?,
        "servicos" => {
            // servicos dispatches on the entity: `rs` uses headless Chrome + HTML scraping, `sc`
            // uses the Next.js JSON API. Output/cache paths derive from the entity's data_dir.
            servicos::run(&entity.id, &entity.data_dir, use_cache).map_err(|e| e.to_string())?;
        }
        other => {
            return Err(format!("coleção desconhecida: '{}'. Use: faqs | servicos", other).into());
        }
    }

    Ok(())
}

fn run_faqs(entity: &EntityConfig, use_cache: bool) -> errors::Result<()> {
    faqs::run(&faq_source_for(entity, use_cache)?)
}

/// Builds the FAQ portal config for an entity. Output/cache paths derive from the entity's
/// `data_dir`; the portal URLs are still per-entity constants (only `rs` is configured today).
fn faq_source_for(entity: &EntityConfig, use_cache: bool) -> errors::Result<faqs::FaqSource> {
    match entity.id.as_str() {
        "rs" => Ok(faqs::FaqSource {
            base_url: "https://atendimento.receita.rs.gov.br".to_string(),
            root_url: "https://atendimento.receita.rs.gov.br/perguntas-frequentes".to_string(),
            root_title: "Perguntas Frequentes".to_string(),
            collection: "faqs".to_string(),
            data_dir: entity.data_dir.clone(),
            cache_dir: format!("{}/cache/faqs", entity.data_dir),
            use_cache,
        }),
        "sc" => Ok(faqs::FaqSource {
            // NOTE(sc): SEF-SC runs a different portal platform than RS (Next.js, slug/id URLs,
            // categories like /perguntas/<id>). These URLs are correct, but the faqs scraper's
            // HTML parsing + page classification (keyed off RS's `data-matriz-source-uri`) will
            // NOT work unchanged — the SC walk/parse logic still needs to be written.
            base_url: "https://www.sef.sc.gov.br".to_string(),
            root_url: "https://www.sef.sc.gov.br/perguntas".to_string(),
            root_title: "Perguntas Frequentes".to_string(),
            collection: "faqs".to_string(),
            data_dir: entity.data_dir.clone(),
            cache_dir: format!("{}/cache/faqs", entity.data_dir),
            use_cache,
        }),
        other => Err(format!(
            "scraper de faqs ainda não configurado para a entidade '{}'",
            other
        )
        .into()),
    }
}
