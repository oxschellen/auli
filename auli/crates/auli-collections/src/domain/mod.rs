// Domain layer — core types & registries (no HTTP, no external I/O).
//
// - entities — multi-tenant registry (EntityConfig, ENTITIES, get_entity)

// Some registry fields are read by serde/tests but not by the scraper pipeline; suppress dead-code
// noise for those.
#![allow(dead_code)]

pub mod entities;
