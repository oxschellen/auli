use thiserror::Error;

pub type Result<T> = core::result::Result<T, Error>;

/// Top-layer error. Wraps the two crates below plus the externals the server touches (HTTP/LLM,
/// I/O, JSON). `Display` is user-facing (it can reach the `answer` field via `exec_all_question`).
#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Custom(String),

    #[error(transparent)]
    Core(#[from] auli_core::Error),

    #[error(transparent)]
    VectorStore(#[from] vector_store::Error),

    #[error("Erro de I/O: {0}")]
    Io(#[from] std::io::Error),

    #[error("Erro de serialização JSON: {0}")]
    SerdeJson(#[from] serde_json::Error),

    #[error(transparent)]
    Llm(#[from] auli_llm::Error),
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
