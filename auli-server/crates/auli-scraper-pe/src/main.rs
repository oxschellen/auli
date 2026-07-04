// auli-scraper-pe — scraper da SEFAZ-PE (serviços; SharePoint 2013 on-prem, HTML server-side).
//
// Conhece UMA entidade ("pe"); não lê o registry. Sem headless Chrome (classe do SC): o portal é
// SharePoint 2013 clássico (WebForms) e renderiza tudo no servidor. O menu global `#menu_servicos`
// traz o catálogo curado em 3 blocos (público: cidadãos/empresas/municípios) × subgrupos opcionais
// (classe); um mesmo link aparece sob vários públicos (ex.: e-Fisco nos 3) — o schema v2+
// (`Ocorrencia`) comporta isso nativamente.
//
// D-PE1 (fase 1): coleta SÓ o menu (1 requisição na home). As páginas de detalhe
// (`/Servicos/...`, corpo em `div.ms-rtestate-field`) ficam para uma fase 2, se o RAG precisar.
// D-PE4 (etiqueta): o robots.txt do portal é restritivo a crawlers genéricos; esta coleta é de
// baixíssima frequência e volume mínimo (1 GET por rodada na fase 1), com cache em disco.

mod scrape;

/// A entidade que este scraper conhece (D-F2.1 — um crate binário por entidade).
pub const ENTITY: &str = "pe";
const DATA_DIR: &str = "../data/pe/raw";

/// Identidade deste scraper, gravada como metadado no snapshot.
pub(crate) fn scraper_info() -> auli_contract::ScraperInfo {
    auli_contract::ScraperInfo {
        nome: env!("CARGO_PKG_NAME").to_string(),
        versao: env!("CARGO_PKG_VERSION").to_string(),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // CLI: auli-scraper-pe [--usecache] servicos   (omitido -> servicos)
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let use_cache = raw.iter().any(|a| a == "--usecache");
    let cmd = raw.iter().find(|a| !a.starts_with("--")).map(String::as_str).unwrap_or("servicos");

    println!("🏛️  Scraper PE (SEFAZ-PE) — coleção: {}", cmd);
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
