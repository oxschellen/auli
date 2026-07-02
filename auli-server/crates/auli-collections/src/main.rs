mod derive_faqs;
mod domain;
mod errors;
mod process;
mod servicos;

use domain::entities::get_entity;

/// Identidade gravada como metadado no snapshot quando o collections ainda raspa (SC, até a etapa D).
pub(crate) fn scraper_info() -> auli_contract::ScraperInfo {
    auli_contract::ScraperInfo {
        nome: env!("CARGO_PKG_NAME").to_string(),
        versao: env!("CARGO_PKG_VERSION").to_string(),
    }
}

fn main() -> errors::Result<()> {
    // CLI: `auli-collections [--usecache] <entity> [process|servicos]`
    //   <entity>     entity id (ex.: `rs`); vazio/omitido -> entidade padrão.
    //   process      (padrão) deriva os artefatos do snapshot, offline.
    //   servicos     raspa os serviços do SC (temporário) e então deriva.
    //   O scraper de faqs/serviços do RS agora é o binário `auli-scraper-rs`.
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let use_cache = raw.iter().any(|a| a == "--usecache");
    let mut positional = raw.iter().filter(|a| !a.starts_with("--"));
    let entity_arg = positional.next().cloned();
    let collection = positional.next().cloned().unwrap_or_else(|| "process".to_string());

    let entity = get_entity(entity_arg.as_deref())?;
    println!("🏛️  Entidade: {} ({})", entity.id, entity.name);

    match collection.as_str() {
        // OFFLINE: deriva contrato, prints, index e per-público do snapshot já gravado.
        "process" => process::run(entity)?,
        // SC ainda raspa aqui (migra para `auli-scraper-sc` na etapa D); depois deriva.
        "servicos" => {
            servicos::run(&entity.id, &entity.data_dir, use_cache).map_err(|e| e.to_string())?;
            process::run(entity)?;
        }
        "faqs" => {
            return Err("o scraper de faqs agora é o binário `auli-scraper-rs faqs`".into());
        }
        other => {
            return Err(format!(
                "coleção desconhecida: '{}'. Use: process (padrão) | servicos",
                other
            )
            .into());
        }
    }

    Ok(())
}
