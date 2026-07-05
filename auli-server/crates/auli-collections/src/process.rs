//! O subcomando `process` — deriva os artefatos dos snapshots, **offline**.
//!
//! Lê os snapshots por coleção (`../data/<id>/<id>-servicos-snapshot.json` e, se houver,
//! `<id>-faqs-snapshot.json`) e, para cada um presente, chama a derivação de
//! [`crate::derive_faqs::process`] / [`crate::servicos::process`]. Coleção ausente é pulada (não é
//! erro). A validação de `schema_version`/`entidade` mora no `load` do kit (header-first); aqui só
//! orquestramos.

use auli_contract::{ColetaFaqs, ColetaServicos};

use crate::domain::entities::EntityConfig;
use crate::errors::Result;

/// Carrega os snapshots por coleção da entidade e deriva os artefatos dos que existirem.
pub fn run(entity: &EntityConfig) -> Result<()> {
    let id = &entity.id;
    let dir = &entity.data_dir;

    let faqs = auli_contract::snapshot::load::<ColetaFaqs>(id, dir, "faqs")?;
    let servicos = auli_contract::snapshot::load::<ColetaServicos>(id, dir, "servicos")?;

    if faqs.is_none() && servicos.is_none() {
        return Err(format!(
            "nenhum snapshot para '{}' — rode `auli-scraper-{} faqs` e/ou `auli-scraper-{} servicos` antes do process.",
            id, id, id
        )
        .into());
    }

    match &faqs {
        Some(snap) => crate::derive_faqs::process(id, dir, &snap.coleta)?,
        None => println!("⏭️  sem snapshot de faqs — pulando"),
    }
    match &servicos {
        Some(snap) => crate::servicos::process(id, dir, &snap.coleta)?,
        None => println!("⏭️  sem snapshot de serviços — pulando"),
    }

    Ok(())
}
