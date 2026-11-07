//! `auli-scraper-kit` — apoio compartilhado dos scrapers por entidade: o **como raspar**.
//!
//! - [`cache`]: cache de páginas em disco, por URL lógica (+ [`cache::read_or_bail`]);
//! - [`http`]: GET/POST com retry/backoff compartilhado ([`http::get_string`]/[`http::post_json`]);
//! - [`build_agent`] + [`USER_AGENT`]: builder do agent `ureq` e o UA de navegador (a exceção da
//!   frota — o padrão é o AuliBot, D-FROTA-UA);
//! - [`clean`]/[`clean_decoded`]/[`decode_entities`]: normalização de texto compartilhada;
//! - [`aggregate_servicos`] (+ [`descricao_body`], [`PerPublicoServicos`]): agrega os serviços
//!   per-público em `ServicoRaw` para o snapshot;
//! - [`faqtree`]: a árvore `<id>-faqs-tree.json` que a aba de FAQs do frontend lê.
//! - [`html`]: o HTML das respostas de FAQ em texto ([`html::html_para_texto`]), uma regra só para a frota.
//! - [`linhas`]: FAQ em linhas — a pergunta é a linha toda em negrito terminada em "?" (os Joomla da PB e do PA).
//! - [`sydle`]: as seções (`section-wrapper`) dos artigos do Sydle ONE (as FAQs do CE e do RO).
//! - [`pdf`]: o texto e a data de criação de um PDF pelo poppler (as FAQs do PI e da BA).
//!
//! A **fronteira** (tipos, versão, caminho e I/O do snapshot — incluindo o shape per-público
//! `ServicoPerPublico`) mora no `auli-contract` (D-C1): produtor e consumidor usam as mesmas
//! funções de lá. Este kit é exclusivo dos scrapers — nada fora de `crates/scrapers/` depende
//! dele, e ele nunca depende de fastembed/ort/vector-store.

pub mod cache;
pub mod docs;
pub mod faqtree;
pub mod html;
pub mod http;
pub mod linhas;
pub mod pdf;
pub mod sydle;

mod agent;
mod aggregate;
mod text;

pub use agent::build_agent;
pub use aggregate::{PerPublicoServicos, aggregate_servicos, descricao_body};
pub use text::{clean, clean_decoded, decode_entities};

/// UA de navegador (Firefox/124 em Linux) — a **exceção** da frota, não o padrão (D-FROTA-UA,
/// 02/11/2026). O padrão é o UA institucional `AuliBot/x.y (+url; contato)`, declarado em cada crate.
/// Este só onde o portal exigir, com o **motivo comentado no call site**; os usos anteriores à regra,
/// nunca testados com o AuliBot, dizem isso no comentário (lista em `docs/auli_scraper_pendências.md`
/// §11).
pub const USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64; rv:124.0) Gecko/20100101 Firefox/124.0";
