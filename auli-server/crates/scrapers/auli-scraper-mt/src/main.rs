// auli-scraper-mt — scraper da SEFAZ-MT (serviços, "Catálogo de Serviços do X-Via Portal" — SPA
// React). Sem headless, sem HTML server-rendered, sem token: a fonte é a API pública
// `POST /v1/search/department` (anônima), filtrada pelo órgão SEFAZ.
//
// Conhece UMA entidade ("mt"); não lê o registry. Grava o snapshot de serviços (v3); a derivação
// dos artefatos é o `auli-collections mt`.

mod mt;

/// A entidade que este scraper conhece (um crate binário por entidade).
pub const ENTITY: &str = "mt";
const DATA_DIR: &str = "../data/mt/raw";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // CLI: auli-scraper-mt [--usecache] servicos   (omitido -> servicos)
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let use_cache = raw.iter().any(|a| a == "--usecache");
    let cmd = raw.iter().find(|a| !a.starts_with("--")).map(String::as_str).unwrap_or("servicos");

    println!("🏛️  Scraper MT (SEFAZ-MT) — coleção: {}", cmd);
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
    // Montagem direta de `ServicoRaw` (padrão CE/RJ): identidade = `slug` (D-MT2), ocorrências =
    // targets × category (D-MT4) — o `aggregate_servicos` não se aplica.
    let (items, publicos_ordem) = mt::scrape(DATA_DIR, use_cache)?;
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
