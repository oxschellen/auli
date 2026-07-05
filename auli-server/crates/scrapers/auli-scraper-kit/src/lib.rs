//! `auli-scraper-kit` — apoio compartilhado dos scrapers por entidade: o **como raspar**.
//!
//! - [`cache`]: cache de páginas em disco, por URL lógica;
//! - [`build_agent`]: builder do agent `ureq` (User-Agent + timeout);
//! - [`aggregate_servicos`] (+ [`descricao_body`], [`PerPublicoServicos`]): agrega os serviços
//!   per-público em `ServicoRaw` para o snapshot.
//!
//! A **fronteira** (tipos, versão, caminho e I/O do snapshot — incluindo o shape per-público
//! `ServicoPerPublico`) mora no `auli-contract` (D-C1): produtor e consumidor usam as mesmas
//! funções de lá. Este kit é exclusivo dos scrapers — nada fora de `crates/scrapers/` depende
//! dele, e ele nunca depende de fastembed/ort/vector-store.

pub mod cache;

mod aggregate;
mod agent;

pub use aggregate::{PerPublicoServicos, aggregate_servicos, descricao_body};
pub use agent::build_agent;
