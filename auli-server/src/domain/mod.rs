// Domain layer — core types & registries (no HTTP, no external I/O).
//
// - entities    — multi-tenant registry (EntityConfig, ENTITIES, get_entity)
// - collections — generic content-kind registry (Collection, parsing, prepare_documents)

pub mod collections;
pub mod entities;
