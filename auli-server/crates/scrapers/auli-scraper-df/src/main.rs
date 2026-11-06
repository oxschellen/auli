// auli-scraper-df — scraper da SEFAZ-DF (serviços, receita.fazenda.df.gov.br — Carta de Serviços).
// Sem headless: a listagem ColdFusion embute a árvore inteira do catálogo (~476 serviços) como um
// objeto JS; cada `servico.cfm` traz a descrição rica num accordion.
//
// As FAQs vêm de duas fontes — as Perguntas e Respostas do Portal da Receita (mesmo host) e as do sistema
// do ISS/NFS-e — `faqs.rs`.
//
// Conhece UMA entidade ("df"); não lê o registry. Grava o snapshot de serviços (v3); a derivação
// dos artefatos é o `auli-collections df`.

mod df;
mod faqs;

/// A entidade que este scraper conhece (um crate binário por entidade).
pub const ENTITY: &str = "df";
const DATA_DIR: &str = "../data/df/raw";
/// A identidade da frota (D-FROTA-UA), nas duas coleções; os dois hosts a aceitam (FAQs: sondagem de
/// 02/11/2026; serviços: recoleta verificada de 06/11/2026).
pub(crate) const UA: &str =
    "AuliBot/0.1 (+https://github.com/oxschellen/auli; carlos.schellenberger@gmail.com)";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // CLI: auli-scraper-df [--usecache] servicos|faqs|all   (omitido -> servicos)
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let use_cache = raw.iter().any(|a| a == "--usecache");
    let cmd = raw
        .iter()
        .find(|a| !a.starts_with("--"))
        .map(String::as_str)
        .unwrap_or("servicos");

    println!("🏛️  Scraper DF (SEFAZ-DF) — coleção: {}", cmd);
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
    // Montagem direta de `ServicoRaw` (padrão CE/AC): identidade = o serviço (link = servico.cfm).
    let (items, publicos_ordem) = df::scrape(DATA_DIR, use_cache)?;
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
