// auli-scraper-ma — scraper da SEFAZ-MA (serviços, portal-sgc.sefaz.ma.gov.br — Angular + Spring API).
// Sem headless: catálogo JSON com token anônimo (login público) + detalhe rico via conteudos/{id}.
//
// As FAQs vêm do mesmo portal e do mesmo token, a página "Perguntas Frequentes" (API JSON) — `faqs.rs`.
//
// Conhece UMA entidade ("ma"); não lê o registry. Grava os snapshots de serviços (v3) e de FAQs; a
// derivação dos artefatos é o `auli-collections ma`.

mod faqs;
mod ma;

/// A entidade que este scraper conhece (um crate binário por entidade).
pub const ENTITY: &str = "ma";
const DATA_DIR: &str = "../data/ma/raw";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // CLI: auli-scraper-ma [--usecache] servicos|faqs|all   (omitido -> servicos)
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let use_cache = raw.iter().any(|a| a == "--usecache");
    let cmd = raw
        .iter()
        .find(|a| !a.starts_with("--"))
        .map(String::as_str)
        .unwrap_or("servicos");

    println!("🏛️  Scraper MA (SEFAZ-MA) — coleção: {}", cmd);
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
    // Montagem direta de `ServicoRaw` (padrão MT): identidade = id; N ocorrências (público × classe).
    let (items, publicos_ordem) = ma::scrape(DATA_DIR, use_cache)?;
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
