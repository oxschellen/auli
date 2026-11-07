// auli-scraper-pa — scraper da SEFA-PA. Serviços: o catálogo `pis` do site novo (WordPress REST, `pis.rs`) e os 3
// que só o catálogo estadual "paradigital" tem (API Prodepa, `pa.rs`). Sem headless, sem login: JSON anônimo.
//
// Conhece UMA entidade ("pa"); não lê o registry. Grava o snapshot de serviços (v3); a derivação
// dos artefatos é o `auli-collections pa`.
//
// As FAQs vêm do site antigo da SEFA (`antigo.sefa.pa.gov.br`, Joomla) — `faqs.rs`.

mod faqs;
mod pa;
mod pis;

/// A entidade que este scraper conhece (um crate binário por entidade).
pub const ENTITY: &str = "pa";
const DATA_DIR: &str = "../data/pa/raw";
/// A identidade da frota (D-FROTA-UA): FAQs (D-PAFAQ-5) e serviços (D-PASERV-4).
pub(crate) const UA: &str =
    "AuliBot/0.1 (+https://github.com/oxschellen/auli; carlos.schellenberger@gmail.com)";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // CLI: auli-scraper-pa [--usecache] servicos|faqs|all   (omitido -> servicos)
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let use_cache = raw.iter().any(|a| a == "--usecache");
    let cmd = raw
        .iter()
        .find(|a| !a.starts_with("--"))
        .map(String::as_str)
        .unwrap_or("servicos");

    println!("🏛️  Scraper PA (SEFA-PA) — coleção: {}", cmd);
    if use_cache {
        println!("📦 --usecache: usando apenas o cache (sem rede).");
    }

    match cmd {
        "servicos" => run_servicos(use_cache)?,
        "faqs" => faqs::run(DATA_DIR, use_cache)?,
        "all" => {
            run_servicos(use_cache)?;
            faqs::run(DATA_DIR, use_cache)?;
        }
        other => {
            return Err(format!(
                "coleção desconhecida: '{}'. Use: servicos | faqs | all",
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
    // Montagem direta de `ServicoRaw` (padrão MS): identidade = id do serviço; N ocorrências
    // (público × classe).
    let (items, publicos_ordem) = pa::scrape(DATA_DIR, use_cache)?;
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
