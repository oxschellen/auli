use thiserror::Error;

pub type Result<T> = core::result::Result<T, Error>;

/// Unified error for the auli domain core. Wraps the layer below (`vector-store`) plus the
/// externals the embedder and corpus parsing touch. `Display` is user-facing (it can reach the
/// `answer` field upstream); `Debug` keeps full detail for logs.
#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Custom(String),

    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),

    #[error("Erro de I/O: {0}")]
    Io(#[from] std::io::Error),

    #[error("Erro de serialização JSON: {0}")]
    SerdeJson(#[from] serde_json::Error),

    #[error(transparent)]
    VectorStore(#[from] vector_store::Error),
}

impl From<String> for Error {
    fn from(val: String) -> Self {
        Self::Custom(val)
    }
}

impl From<&str> for Error {
    fn from(val: &str) -> Self {
        Self::Custom(val.to_string())
    }
}
