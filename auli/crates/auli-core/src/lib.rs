//! Auli domain core — the middle layer.
//!
//! Depends on `vector-store` (below); knows nothing of RAG, HTTP, or the CLI (above). It owns the
//! pieces that must agree between the two binary modes:
//!
//! - [`embed`] — the BGE-M3 embedder, the single encoder for documents (`update`) and the query
//!   (`server`).
//! - [`corpus`] — `EmbedStrategy`, the `Collection` table, block parsing, and `prepare_documents`:
//!   *what* gets embedded.
//! - [`manifest`] — the embedding identity (model + dim + strategy version) and pack manifest
//!   schema/validation that pins it.

pub mod corpus;
pub mod embed;
pub mod error;
pub mod manifest;

pub use error::{Error, Result};
