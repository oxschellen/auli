mod derive_faqs;
mod derive_pareceres;
mod domain;
mod errors;
mod process;
mod servicos;

use domain::entities::get_entity;

fn main() -> errors::Result<()> {
    // CLI: `auli-collections <entity> [process]`
    //   <entity>   entity id (ex.: `rs`); vazio/omitido -> entidade padrão.
    //   process    (padrão e único subcomando) deriva os artefatos do snapshot, offline.
    //   A coleta é dos binários `auli-scraper-rs` / `auli-scraper-sc` (fase 2).
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(flag) = args.iter().find(|a| a.starts_with("--")) {
        return Err(format!("flag desconhecida: '{}'. O auli-collections não aceita flags.", flag).into());
    }
    let mut positional = args.iter();
    let entity_arg = positional.next().cloned();
    let collection = positional.next().cloned().unwrap_or_else(|| "process".to_string());

    let entity = get_entity(entity_arg.as_deref())?;
    println!("🏛️  Entidade: {} ({})", entity.id, entity.name);

    match collection.as_str() {
        // OFFLINE: deriva contrato, prints, index e per-público do snapshot já gravado.
        "process" => process::run(entity)?,
        // OFFLINE: ingere pareceres do `.txt` autorado em `ref/` -> `Table<Consulta>` no `raw/`.
        // Passo incremental até haver scraper de pareceres.
        "pareceres" => derive_pareceres::run(entity)?,
        "faqs" | "servicos" => {
            return Err(
                "a coleta agora é feita pelos binários `auli-scraper-rs` / `auli-scraper-sc`; \
                 o auli-collections só deriva (rode `auli-collections <entity>`)"
                    .into(),
            );
        }
        other => {
            return Err(
                format!("subcomando desconhecido: '{}'. Use: process (padrão) | pareceres", other).into(),
            );
        }
    }

    Ok(())
}
