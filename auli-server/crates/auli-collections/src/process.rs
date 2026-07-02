//! O subcomando `process` — deriva os artefatos do snapshot, **offline**.
//!
//! Lê `../data/<id>/<id>-snapshot.json`, valida `schema_version`/`entidade`, e para cada coleta
//! presente chama as derivações de [`crate::faqs::process`] / [`crate::servicos::process`]. Coleção
//! ausente é pulada (não é erro). É aqui que mora a validação amigável do schema — o `auli-contract`
//! segue só serde, sem domínio (mesmo precedente da `Table<P>`: quem lê é que reclama).

use auli_contract::SNAPSHOT_SCHEMA_VERSION;

use crate::domain::entities::EntityConfig;
use crate::errors::Result;

/// Carrega o snapshot da entidade e deriva os artefatos das coleções presentes.
pub fn run(entity: &EntityConfig) -> Result<()> {
    let snapshot = auli_scraper_kit::snapshot::load(&entity.id, &entity.data_dir)?.ok_or_else(|| {
        format!(
            "snapshot ausente para '{}' — rode `{} faqs` e/ou `{} servicos` antes do process.",
            entity.id, entity.id, entity.id
        )
    })?;

    if snapshot.schema_version != SNAPSHOT_SCHEMA_VERSION {
        return Err(format!(
            "snapshot de '{}' está na versão de schema v{} (esperado v{}). Re-raspe a entidade — o \
             snapshot é regenerável do cache, não há migração.",
            entity.id, snapshot.schema_version, SNAPSHOT_SCHEMA_VERSION
        )
        .into());
    }
    if snapshot.entidade != entity.id {
        return Err(format!(
            "entidade do snapshot ('{}') não bate com a pedida ('{}').",
            snapshot.entidade, entity.id
        )
        .into());
    }

    match &snapshot.colecoes.faqs {
        Some(coleta) => crate::derive_faqs::process(&entity.id, &entity.data_dir, coleta)?,
        None => println!("⏭️  sem coleção de faqs no snapshot — pulando"),
    }
    match &snapshot.colecoes.servicos {
        Some(coleta) => crate::servicos::process(&entity.id, &entity.data_dir, coleta)
            .map_err(|e| e.to_string())?,
        None => println!("⏭️  sem coleção de serviços no snapshot — pulando"),
    }

    Ok(())
}
