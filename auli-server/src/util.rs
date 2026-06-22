//! Small cross-cutting helpers.

use crate::errors::Result;

/// Run a blocking, fallible closure on Tokio's blocking pool and flatten the result: a
/// `JoinError` (panic/cancel) and the closure's own `Err` both collapse into one `crate::Error`.
/// Lets callers write `run_blocking(..).await?` instead of
/// `spawn_blocking(..).await.map_err(|e| e.to_string())??`.
pub async fn run_blocking<F, T>(f: F) -> Result<T>
where
    F: FnOnce() -> Result<T> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(f).await.map_err(|e| e.to_string())?
}
