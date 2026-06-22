// Domain layer — core types & registries (no HTTP, no external I/O).
//
// - entities    — multi-tenant registry (EntityConfig, ENTITIES, get_entity)
// - collections — generic content-kind registry (Collection, parsing, prepare_documents)

// Wired into the crate but not yet consumed by the pipeline; suppress dead-code noise until it is.
#![allow(dead_code)]

pub mod collections;
pub mod entities;
