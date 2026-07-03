// auli-scraper-mg — scraper da SEF-MG (serviços, portal ServiceNow CSM, API JSON). Sem headless
// Chrome.
//
// Conhece UMA entidade ("mg"); não lê o registry. Grava o snapshot v2; a derivação dos artefatos é
// o `auli-collections mg`.

mod mg;

/// A entidade que este scraper conhece (D-F2.1 — um crate binário por entidade).
pub const ENTITY: &str = "mg";
const DATA_DIR: &str = "../data/mg/raw";

/// Identidade deste scraper, gravada como metadado no snapshot.
pub(crate) fn scraper_info() -> auli_contract::ScraperInfo {
    auli_contract::ScraperInfo {
        nome: env!("CARGO_PKG_NAME").to_string(),
        versao: env!("CARGO_PKG_VERSION").to_string(),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // CLI: auli-scraper-mg [--usecache] servicos   (omitido -> servicos)
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let use_cache = raw.iter().any(|a| a == "--usecache");
    let cmd = raw.iter().find(|a| !a.starts_with("--")).map(String::as_str).unwrap_or("servicos");

    println!("🏛️  Scraper MG (SEF-MG) — coleção: {}", cmd);
    if use_cache {
        println!("📦 --usecache: usando apenas páginas em cache (sem rede).");
    }

    match cmd {
        "servicos" => run_servicos(use_cache)?,
        other => {
            return Err(format!("coleção desconhecida: '{}'. Use: servicos", other).into());
        }
    }

    println!("✅ Snapshot atualizado. Rode `auli-collections {}` para derivar os artefatos.", ENTITY);
    Ok(())
}

fn run_servicos(use_cache: bool) -> Result<(), Box<dyn std::error::Error>> {
    let (inputs, publicos_ordem) = mg::scrape(DATA_DIR, use_cache)?;
    let items = auli_scraper_kit::aggregate_servicos(&inputs);
    auli_scraper_kit::snapshot::write_servicos(
        ENTITY,
        DATA_DIR,
        &scraper_info(),
        publicos_ordem,
        items,
    )?;
    println!("🎉 Coleta de serviços gravada no snapshot.");
    Ok(())
}
