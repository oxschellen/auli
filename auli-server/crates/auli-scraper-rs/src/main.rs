// auli-scraper-rs — scraper da SEFAZ-RS (faqs + serviços).
//
// Conhece UMA entidade ("rs"); não lê o registry.toml (isso é assunto do collections/engine).
// Grava o snapshot v2; a derivação dos artefatos é o `auli-collections rs`.

mod errors;
mod faqs;
mod servicos;

/// A entidade que este scraper conhece (D-F2.1 — um crate binário por entidade).
pub const ENTITY: &str = "rs";
/// Onde as saídas geradas vivem (o snapshot é gravado em `../data/rs/`, irmão de `raw/`).
const DATA_DIR: &str = "../data/rs/raw";

/// Identidade deste scraper, gravada como metadado no snapshot.
pub(crate) fn scraper_info() -> auli_contract::ScraperInfo {
    auli_contract::ScraperInfo {
        nome: env!("CARGO_PKG_NAME").to_string(),
        versao: env!("CARGO_PKG_VERSION").to_string(),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // CLI: auli-scraper-rs [--usecache] faqs|servicos|all   (omitido -> all)
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let use_cache = raw.iter().any(|a| a == "--usecache");
    let cmd = raw.iter().find(|a| !a.starts_with("--")).map(String::as_str).unwrap_or("all");

    println!("🏛️  Scraper RS (SEFAZ-RS) — coleção: {}", cmd);
    if use_cache {
        println!("📦 --usecache: usando apenas páginas em cache (sem rede).");
    }

    match cmd {
        "faqs" => run_faqs(use_cache)?,
        "servicos" => servicos::run(DATA_DIR, use_cache)?,
        "all" => {
            run_faqs(use_cache)?;
            servicos::run(DATA_DIR, use_cache)?;
        }
        other => {
            return Err(format!("coleção desconhecida: '{}'. Use: faqs | servicos | all", other).into());
        }
    }

    println!("✅ Snapshot atualizado. Rode `auli-collections {}` para derivar os artefatos.", ENTITY);
    Ok(())
}

fn run_faqs(use_cache: bool) -> errors::Result<()> {
    faqs::run(&faqs::FaqSource {
        id: ENTITY.to_string(),
        base_url: "https://atendimento.receita.rs.gov.br".to_string(),
        root_url: "https://atendimento.receita.rs.gov.br/perguntas-frequentes".to_string(),
        root_title: "Perguntas Frequentes".to_string(),
        data_dir: DATA_DIR.to_string(),
        cache_dir: format!("{}/cache/faqs", DATA_DIR),
        use_cache,
    })
}
