mod derive_faqs;
mod domain;
mod errors;
mod process;
mod servicos;

use domain::entities::get_entity;

fn main() -> errors::Result<()> {
    // CLI: `auli-collections [--usecache] <entity> [process|servicos]`
    //   <entity>     entity id (ex.: `rs`); vazio/omitido -> entidade padrão.
    //   process      (padrão) deriva os artefatos do snapshot, offline.
    //   servicos     raspa os serviços do SC (temporário) e então deriva.
    //   O scraper de faqs/serviços do RS agora é o binário `auli-scraper-rs`.
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let mut positional = raw.iter().filter(|a| !a.starts_with("--"));
    let entity_arg = positional.next().cloned();
    let collection = positional.next().cloned().unwrap_or_else(|| "process".to_string());

    let entity = get_entity(entity_arg.as_deref())?;
    println!("🏛️  Entidade: {} ({})", entity.id, entity.name);

    match collection.as_str() {
        // OFFLINE: deriva contrato, prints, index e per-público do snapshot já gravado.
        "process" => process::run(entity)?,
        "faqs" | "servicos" => {
            return Err(
                "a coleta agora é feita pelos binários `auli-scraper-rs` / `auli-scraper-sc`; \
                 o auli-collections só deriva (rode `auli-collections <entity>`)"
                    .into(),
            );
        }
        other => {
            return Err(format!("subcomando desconhecido: '{}'. Use: process (padrão)", other).into());
        }
    }

    Ok(())
}
