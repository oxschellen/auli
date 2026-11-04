// auli-scraper-ce — scraper da SEFAZ-CE (serviços, "Portal de Serviços" — SPA Sydle ONE).
// Sem headless, sem HTML: a fonte é a API JSON `getChildren` (POST) do catálogo `servico-geral`.
//
// As FAQs são os artigos de perguntas e respostas do catálogo irmão `conteudo-geral`, pela mesma API — `faqs.rs`.
//
// Conhece UMA entidade ("ce"); não lê o registry. Grava o snapshot de serviços (v3); a derivação
// dos artefatos é o `auli-collections ce`.

mod ce;
mod faqs;

/// A entidade que este scraper conhece (um crate binário por entidade).
pub const ENTITY: &str = "ce";
const DATA_DIR: &str = "../data/ce/raw";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // CLI: auli-scraper-ce [--usecache] servicos|faqs|all   (omitido -> servicos)
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let use_cache = raw.iter().any(|a| a == "--usecache");
    let cmd = raw
        .iter()
        .find(|a| !a.starts_with("--"))
        .map(String::as_str)
        .unwrap_or("servicos");

    println!("🏛️  Scraper CE (SEFAZ-CE) — coleção: {}", cmd);
    if use_cache {
        println!("📦 --usecache: usando apenas páginas em cache (sem rede).");
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
    // Montagem direta de `ServicoRaw` (padrão SP/RJ): a identidade é o `_id` do documento e o
    // `aggregate_servicos` (que deduplica por link) não se aplica.
    let (items, publicos_ordem) = ce::scrape(DATA_DIR, use_cache)?;
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
