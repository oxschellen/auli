// auli-scraper-pi — scraper da SEFAZ-PI (serviços, "Carta de Serviços" — SPA Sydle ONE).
// Sem headless, sem HTML: a fonte é a API JSON `_search` (GET) sobre o catálogo "Carta de Serviços".
//
// As FAQs são a seção "Perguntas Frequentes" das páginas de Documentos Eletrônicos e o PDF da FAQ da EFD — `faqs.rs`.
//
// Conhece UMA entidade ("pi"); não lê o registry. Grava o snapshot de serviços (v3); a derivação
// dos artefatos é o `auli-collections pi`.

mod faqs;
mod pi;

/// A entidade que este scraper conhece (um crate binário por entidade).
pub const ENTITY: &str = "pi";
const DATA_DIR: &str = "../data/pi/raw";
/// A identidade da frota (D-FROTA-UA) nas FAQs — D-SYDFAQ-7. Os serviços seguem com o UA de navegador por legado
/// (pendência §11).
pub(crate) const UA: &str =
    "AuliBot/0.1 (+https://github.com/oxschellen/auli; carlos.schellenberger@gmail.com)";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // CLI: auli-scraper-pi [--usecache] servicos|faqs|all   (omitido -> servicos)
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let use_cache = raw.iter().any(|a| a == "--usecache");
    let cmd = raw
        .iter()
        .find(|a| !a.starts_with("--"))
        .map(String::as_str)
        .unwrap_or("servicos");

    println!("🏛️  Scraper PI (SEFAZ-PI) — coleção: {}", cmd);
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
    // Montagem direta de `ServicoRaw` (padrão CE/RJ): a identidade é o `_id` do documento.
    let (items, publicos_ordem) = pi::scrape(DATA_DIR, use_cache)?;
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
