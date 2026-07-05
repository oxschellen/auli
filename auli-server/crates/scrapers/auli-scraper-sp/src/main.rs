// auli-scraper-sp — scraper da SEFAZ-SP (serviços; catálogo SharePoint via REST `_api` anônimo).
//
// Conhece UMA entidade ("sp"); não lê o registry. Sem headless Chrome e sem parser HTML: o catálogo
// vem em JSON (duas listas SharePoint — 'Serviços' e 'Homes 360'). Um serviço pertence a várias
// facetas (Cidadão/Empresa/Servidor/Tributo) → múltiplas `Ocorrencia`s (schema v2 nativo).

mod scrape;

/// A entidade que este scraper conhece (D-F2.1 — um crate binário por entidade).
pub const ENTITY: &str = "sp";
const DATA_DIR: &str = "../data/sp/raw";

/// Identidade deste scraper, gravada como metadado no snapshot.
pub(crate) fn scraper_info() -> auli_contract::ScraperInfo {
    auli_contract::ScraperInfo {
        nome: env!("CARGO_PKG_NAME").to_string(),
        versao: env!("CARGO_PKG_VERSION").to_string(),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // CLI: auli-scraper-sp [--usecache] servicos   (omitido -> servicos)
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let use_cache = raw.iter().any(|a| a == "--usecache");
    let cmd = raw.iter().find(|a| !a.starts_with("--")).map(String::as_str).unwrap_or("servicos");

    println!("🏛️  Scraper SP (SEFAZ-SP) — coleção: {}", cmd);
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
    // No SP a URL NÃO é chave única (vários serviços distintos compartilham um login), então o scraper
    // monta os `ServicoRaw` direto (um por serviço do catálogo, com suas ocorrências) em vez de passar
    // pelo `aggregate_servicos` do kit (que dedupa por link).
    let (items, publicos_ordem) = scrape::scrape(DATA_DIR, use_cache)?;
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
