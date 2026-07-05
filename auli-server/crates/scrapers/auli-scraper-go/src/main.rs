// auli-scraper-go — scraper da SEFAZ-GO (Secretaria de Estado da Economia de Goiás), serviços via
// API do Portal Expresso (WSO2, client_credentials anônimo). Escopo = órgão Economia (id 20).
// Público único "Serviços" (Cenário A); classe = categoria; identidade = idServico.
//
// Conhece UMA entidade ("go"); não lê o registry. Grava o snapshot de serviços (v3); a derivação
// dos artefatos é o `auli-collections go`.
//
// Sobre o WAF/JA3 de api.go.gov.br e o uso de curl-subprocess na coleta, ver o header do go.rs.

mod go;

/// A entidade que este scraper conhece (D-F2.1 — um crate binário por entidade).
pub const ENTITY: &str = "go";
const DATA_DIR: &str = "../data/go/raw";

/// Identidade deste scraper, gravada como metadado no snapshot.
pub(crate) fn scraper_info() -> auli_contract::ScraperInfo {
    auli_contract::ScraperInfo::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // CLI: auli-scraper-go [--usecache] servicos   (omitido -> servicos)
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let use_cache = raw.iter().any(|a| a == "--usecache");
    let cmd = raw.iter().find(|a| !a.starts_with("--")).map(String::as_str).unwrap_or("servicos");

    println!("🏛️  Scraper GO (SEFAZ-GO / Economia) — coleção: {}", cmd);
    if use_cache {
        println!("📦 --usecache: usando apenas páginas em cache (sem rede).");
    }

    match cmd {
        "servicos" => run_servicos(use_cache)?,
        other => return Err(format!("coleção desconhecida: '{}'. Use: servicos", other).into()),
    }

    println!("✅ Snapshot atualizado. Rode `auli-collections {}` para derivar os artefatos.", ENTITY);
    Ok(())
}

fn run_servicos(use_cache: bool) -> Result<(), Box<dyn std::error::Error>> {
    // ServicoRaw direto (padrão CE/RJ): identidade = idServico (D-GO2/5), ocorrências por categoria.
    let (items, publicos_ordem) = go::scrape(DATA_DIR, use_cache)?;
    auli_contract::snapshot::write_servicos(ENTITY, DATA_DIR, &scraper_info(), publicos_ordem, items)?;
    println!("🎉 Coleta de serviços gravada no snapshot.");
    Ok(())
}
