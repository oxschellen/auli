use std::sync::Arc;

use auli_anon::Anonimizador;
use auli_retrieval::Engine;

/// Shared server state. The vector collections are **read-only** (`ReadStore`), eager-loaded at
/// boot — the server holds no writer and cannot mutate them. The only thing it "produces" is the
/// ephemeral query vector, in memory.
#[derive(Clone)]
pub struct AppState {
    /// O motor de recuperação: embedder + coleções + árvore `docs/`. Compartilhado pelas três
    /// faces (chat, `/v1/retrieve`, MCP). Somente-leitura por construção. Ver D-MCP-8: os campos
    /// `collections`/`embedder`/`docs_root` migraram para dentro dele — uma fonte de verdade.
    pub engine: Arc<Engine>,
    /// Anonimizador de PII compartilhado (regexes compilam uma vez, no boot). Usado para mascarar
    /// a pergunta antes de gravá-la no log/stdout. Ver `auli-anon`.
    pub anonimizador: Arc<Anonimizador>,
}
