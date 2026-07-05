// auli-scraper-es — scraper da SEFAZ-ES (serviços, portal.es.gov.br — SPA React sobre X-Via).
// Sem headless, sem token: a listagem por órgão vem da API pública `POST /v1/search` (anônima).
//
// Conhece UMA entidade ("es"); não lê o registry. Grava o snapshot de serviços (v3); a derivação
// dos artefatos é o `auli-collections es`.

mod es;

/// A entidade que este scraper conhece (um crate binário por entidade).
pub const ENTITY: &str = "es";
const DATA_DIR: &str = "../data/es/raw";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // CLI: auli-scraper-es [--usecache] servicos   (omitido -> servicos)
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let use_cache = raw.iter().any(|a| a == "--usecache");
    let cmd = raw.iter().find(|a| !a.starts_with("--")).map(String::as_str).unwrap_or("servicos");

    println!("🏛️  Scraper ES (SEFAZ-ES) — coleção: {}", cmd);
    if use_cache {
        println!("📦 --usecache: usando apenas o cache (sem rede).");
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
    // Montagem direta de `ServicoRaw` (padrão MT): identidade = slug; N ocorrências (público × classe).
    let (items, publicos_ordem) = es::scrape(DATA_DIR, use_cache)?;
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
