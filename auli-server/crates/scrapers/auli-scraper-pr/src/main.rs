// auli-scraper-pr — scraper da SEFA-PR (serviços; portal Drupal server-side, HTML pronto).
//
// Conhece UMA entidade ("pr"); não lê o registry. Sem headless Chrome (classe do SC). O mega-menu
// "Serviços para você!" traz o catálogo curado em 7 abas (público) × grupos (classe); um mesmo link
// aparece sob várias abas/classes — o schema v2 (`Ocorrencia`) comporta isso nativamente.

mod scrape;

/// A entidade que este scraper conhece (D-F2.1 — um crate binário por entidade).
pub const ENTITY: &str = "pr";
const DATA_DIR: &str = "../data/pr/raw";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // CLI: auli-scraper-pr [--usecache] servicos   (omitido -> servicos)
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let use_cache = raw.iter().any(|a| a == "--usecache");
    let cmd = raw.iter().find(|a| !a.starts_with("--")).map(String::as_str).unwrap_or("servicos");

    println!("🏛️  Scraper PR (SEFA-PR) — coleção: {}", cmd);
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
    let (inputs, publicos_ordem) = scrape::scrape(DATA_DIR, use_cache)?;
    let items = auli_scraper_kit::aggregate_servicos(&inputs);
    auli_contract::snapshot::write_servicos(
        ENTITY,
        DATA_DIR,
        &auli_contract::ScraperInfo::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
        publicos_ordem,
        items,
    )?;
    println!("🎉 Coleta de serviços gravada no snapshot.");
    Ok(())
}
