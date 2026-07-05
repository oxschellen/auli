// auli-scraper-go — scraper da SEFAZ-GO (serviços; o órgão fazendário de GO é a Secretaria de
// Estado da Economia — D-GO2). Fonte: API do Portal Expresso (WSO2), catálogo por órgão. Sem
// headless, sem HTML server-rendered: JSON direto, auth client_credentials anônima.
//
// Conhece UMA entidade ("go"); não lê o registry. Grava o snapshot de serviços (v3); a derivação
// dos artefatos é o `auli-collections go`.

mod go;

/// A entidade que este scraper conhece (um crate binário por entidade).
pub const ENTITY: &str = "go";
const DATA_DIR: &str = "../data/go/raw";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // CLI: auli-scraper-go [--usecache] servicos   (omitido -> servicos)
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let use_cache = raw.iter().any(|a| a == "--usecache");
    let cmd = raw.iter().find(|a| !a.starts_with("--")).map(String::as_str).unwrap_or("servicos");

    println!("🏛️  Scraper GO (SEFAZ-GO / Economia) — coleção: {}", cmd);
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
    // Montagem direta de `ServicoRaw` (padrão CE): identidade = `idServico` (D-GO); o
    // `aggregate_servicos` (dedup por link) não se aplica.
    let (items, publicos_ordem) = go::scrape(DATA_DIR, use_cache)?;
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
