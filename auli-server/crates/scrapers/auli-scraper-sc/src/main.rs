// auli-scraper-sc — scraper da SEF-SC (serviços e FAQs, API JSON Next.js). Sem headless Chrome.
//
// Conhece UMA entidade ("sc"); não lê o registry. Grava os snapshots de serviços (v3) e de FAQs; a
// derivação dos artefatos é o `auli-collections sc`.

mod faqs;
mod pareceres;
mod sc;

/// A entidade que este scraper conhece (D-F2.1 — um crate binário por entidade).
pub const ENTITY: &str = "sc";
const DATA_DIR: &str = "../data/sc/raw";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // CLI: auli-scraper-sc [--usecache] servicos|faqs|all|pareceres   (omitido -> servicos)
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let use_cache = raw.iter().any(|a| a == "--usecache");
    let cmd = raw
        .iter()
        .find(|a| !a.starts_with("--"))
        .map(String::as_str)
        .unwrap_or("servicos");

    println!("🏛️  Scraper SC (SEF-SC) — coleção: {}", cmd);
    if use_cache {
        println!("📦 --usecache: usando apenas páginas em cache (sem rede).");
    }

    match cmd {
        "servicos" => run_servicos(use_cache)?,
        "faqs" => faqs::run(DATA_DIR, use_cache)?,
        // As duas coleções de atendimento. `pareceres` fica fora, como no RS (D-SCFAQ-6).
        "all" => {
            run_servicos(use_cache)?;
            faqs::run(DATA_DIR, use_cache)?;
        }
        // Pareceres (Consultas da COPAT): grava o intermediário `ref/sc-pareceres-temp.txt` e
        // retorna (sem snapshot — a ingestão dos pareceres é estágio posterior, como no RS).
        "pareceres" => {
            pareceres::run(use_cache)?;
            return Ok(());
        }
        other => {
            return Err(format!(
                "coleção desconhecida: '{}'. Use: servicos | faqs | all | pareceres",
                other
            )
            .into());
        }
    }

    println!(
        "✅ Snapshot atualizado. Rode `auli-collections {}` para derivar os artefatos.",
        ENTITY
    );
    Ok(())
}

fn run_servicos(use_cache: bool) -> Result<(), Box<dyn std::error::Error>> {
    let (inputs, publicos_ordem) = sc::scrape(DATA_DIR, use_cache)?;
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
