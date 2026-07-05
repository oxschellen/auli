// auli-scraper-ba — scraper da SEFAZ-BA (Carta de Serviços ao Cidadão; ASP clássico, HTML
// server-side).
//
// Conhece UMA entidade ("ba"); não lê o registry. Sem headless Chrome (classe do SC): a Carta é
// ASP clássico + Bootstrap 3, tudo renderizado no servidor. A listagem é uma página única
// (`ul#search_list`, ~206 serviços, sem paginação); cada ficha (`index.asp?id=<slug>`) declara o
// público (`panel-title`), a classe (subtítulo `<small>`) e o conteúdo em blocos `media-service`
// (Documentos Necessários / Como Fazer / Canal / Tempo Médio / Base Legal) — ouro para o RAG.
//
// D-BA4 (etiqueta): o robots.txt do portal é restritivo a crawlers genéricos; coleta de
// baixíssima frequência, ~207 GETs por rodada com cortesia de 500ms e cache em disco.

mod scrape;

/// A entidade que este scraper conhece (D-F2.1 — um crate binário por entidade).
pub const ENTITY: &str = "ba";
const DATA_DIR: &str = "../data/ba/raw";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // CLI: auli-scraper-ba [--usecache] servicos   (omitido -> servicos)
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let use_cache = raw.iter().any(|a| a == "--usecache");
    let cmd = raw.iter().find(|a| !a.starts_with("--")).map(String::as_str).unwrap_or("servicos");

    println!("🏛️  Scraper BA (SEFAZ-BA) — coleção: {}", cmd);
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
