use thiserror::Error;

pub type Result<T> = core::result::Result<T, Error>;

/// Errors from the vector store. Deliberately small and domain-free — the store knows nothing
/// about embeddings, models, or tributação; it only reads/writes `(id, vector, payload)` records.
#[derive(Debug, Error)]
pub enum Error {
    #[error("Erro de I/O do vector store: {0}")]
    Io(#[from] std::io::Error),

    #[error("Erro de (de)serialização do vector store: {0}")]
    Serde(#[from] serde_json::Error),

    /// Reserved for Phase 5 dimension enforcement (reject a vector whose width disagrees with the
    /// collection's established dimension). Carried here so the public error surface is stable.
    #[error("Dimensão incompatível: esperado {expected}, recebido {got}")]
    DimensionMismatch { expected: usize, got: usize },
}
