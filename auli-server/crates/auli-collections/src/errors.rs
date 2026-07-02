use thiserror::Error;

pub type Result<T> = core::result::Result<T, Error>;

/// Unified error type for the crate.
///
/// `Display` produces a human-readable message (surfaced on the CLI / in logs), while `Debug`
/// keeps the full detail. External errors are wrapped with `#[from]` so `?` converts them automatically.
#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Custom(String),

    // -- Externals
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),

    #[error("Erro de I/O: {0}")]
    Io(#[from] std::io::Error),

    #[error("Erro de serialização JSON: {0}")]
    SerdeJson(#[from] serde_json::Error),
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
