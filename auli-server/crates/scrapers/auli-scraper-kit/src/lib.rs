//! `auli-scraper-kit` — apoio compartilhado dos scrapers por entidade: o **como raspar**.
//!
//! - [`cache`]: cache de páginas em disco, por URL lógica (+ [`cache::read_or_bail`]);
//! - [`http`]: GET/POST com retry/backoff compartilhado ([`http::get_string`]/[`http::post_json`]);
//! - [`build_agent`] + [`USER_AGENT`]: builder do agent `ureq` e a identidade de rede padrão;
//! - [`clean`]/[`clean_decoded`]/[`decode_entities`]: normalização de texto compartilhada;
//! - [`aggregate_servicos`] (+ [`descricao_body`], [`PerPublicoServicos`]): agrega os serviços
//!   per-público em `ServicoRaw` para o snapshot.
//!
//! A **fronteira** (tipos, versão, caminho e I/O do snapshot — incluindo o shape per-público
//! `ServicoPerPublico`) mora no `auli-contract` (D-C1): produtor e consumidor usam as mesmas
//! funções de lá. Este kit é exclusivo dos scrapers — nada fora de `crates/scrapers/` depende
//! dele, e ele nunca depende de fastembed/ort/vector-store.

pub mod cache;
pub mod docs;
pub mod http;

mod aggregate;
mod agent;
mod text;

pub use aggregate::{PerPublicoServicos, aggregate_servicos, descricao_body};
pub use agent::build_agent;
pub use text::{clean, clean_decoded, decode_entities};

/// Identidade de rede padrão da frota (Firefox/124 em Linux, usado por 8 dos 11 scrapers). Uma
/// divergência local (ex.: um portal que exija outro UA) deve vir com **comentário do motivo** —
/// senão é drift. (Pendência de projeto: avaliar um UA identificável `AuliBot/x.y (+url)`.)
pub const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64; rv:124.0) Gecko/20100101 Firefox/124.0";
