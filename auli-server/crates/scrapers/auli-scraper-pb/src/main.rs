// auli-scraper-pb — scraper da SEFAZ-PB (serviços, cartaservico.sefaz.pb.gov.br — Carta em PHP).
// Sem headless: `servicos.php` lista os serviços; cada `saibamais.php?id=N` traz a ficha rica.
//
// Conhece UMA entidade ("pb"); não lê o registry. Grava o snapshot de serviços (v3); a derivação
// dos artefatos é o `auli-collections pb`.
//
// As FAQs vêm do portal institucional (`www.sefaz.pb.gov.br`, Joomla) — `faqs.rs`.

mod faqs;
mod pb;

/// A entidade que este scraper conhece (um crate binário por entidade).
pub const ENTITY: &str = "pb";
const DATA_DIR: &str = "../data/pb/raw";
/// A identidade da frota (D-FROTA-UA) nas FAQs — D-PBFAQ-8. Os serviços declaram a mesma em `pb.rs`.
pub(crate) const UA: &str =
    "AuliBot/0.1 (+https://github.com/oxschellen/auli; carlos.schellenberger@gmail.com)";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // CLI: auli-scraper-pb [--usecache] servicos|faqs|all   (omitido -> servicos)
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let use_cache = raw.iter().any(|a| a == "--usecache");
    let cmd = raw
        .iter()
        .find(|a| !a.starts_with("--"))
        .map(String::as_str)
        .unwrap_or("servicos");

    println!("🏛️  Scraper PB (SEFAZ-PB) — coleção: {}", cmd);
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
    // Montagem direta de `ServicoRaw` (padrão TO/DF): identidade = a ficha (saibamais.php?id=N).
    let (items, publicos_ordem) = pb::scrape(DATA_DIR, use_cache)?;
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
