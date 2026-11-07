// auli-scraper-rn — scraper da SEFAZ-RN (serviços, www.sefaz.rn.gov.br — WordPress + React).
// Sem headless: o CPT `servicos` da WP REST são cards de atalho; enriquecemos os que apontam para
// posts (`/postagem/`) com o corpo (ACF `Matéria`).
//
// As FAQs vêm do programa Nota Potiguar (`np.sefaz.rn.gov.br/portal`), não do site da SEFAZ — `faqs.rs`.
//
// Conhece UMA entidade ("rn"); não lê o registry. Grava os snapshots de serviços (v3) e de FAQs; a
// derivação dos artefatos é o `auli-collections rn`.

mod faqs;
mod rn;

/// A entidade que este scraper conhece (um crate binário por entidade).
pub const ENTITY: &str = "rn";
const DATA_DIR: &str = "../data/rn/raw";
/// A identidade da frota (D-FROTA-UA), nas duas coleções e nos dois hosts (`www.sefaz.rn.gov.br`,
/// `np.sefaz.rn.gov.br`).
pub(crate) const UA: &str =
    "AuliBot/0.1 (+https://github.com/oxschellen/auli; carlos.schellenberger@gmail.com)";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // CLI: auli-scraper-rn [--usecache] servicos|faqs|all   (omitido -> servicos)
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let use_cache = raw.iter().any(|a| a == "--usecache");
    let cmd = raw
        .iter()
        .find(|a| !a.starts_with("--"))
        .map(String::as_str)
        .unwrap_or("servicos");

    println!("🏛️  Scraper RN (SEFAZ-RN) — coleção: {}", cmd);
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
    // Montagem direta de `ServicoRaw` (padrão CE/AC): identidade = o card (link).
    let (items, publicos_ordem) = rn::scrape(DATA_DIR, use_cache)?;
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
