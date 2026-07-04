//! `auli-scraper-kit` — apoio compartilhado dos scrapers por entidade.
//!
//! Reúne o que `auli-scraper-rs` e `auli-scraper-sc` têm em comum, sem arrastar o engine:
//! - [`snapshot`]: I/O do *snapshot de coleta* (load/merge/save, `coletado_em` UTC);
//! - [`cache`]: cache de páginas em disco, por URL lógica;
//! - [`build_agent`]: builder do agent `ureq` (User-Agent + timeout);
//! - [`Servico`]: o registro de um serviço raspado (entrada da agregação);
//! - [`aggregate_servicos`]: agrega os serviços per-público em `ServicoRaw` (para o snapshot).
//!
//! Depende só de `auli-contract` + rede/serde — nada de fastembed/ort/vector-store.

pub mod cache;
pub mod snapshot;

mod aggregate;
mod agent;
mod servico;

pub use aggregate::{PerPublicoServicos, aggregate_servicos, descricao_body};
pub use agent::build_agent;
pub use servico::Servico;
