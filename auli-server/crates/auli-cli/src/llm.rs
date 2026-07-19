//! Wrapper do cliente compartilhado `auli-llm` com a config do servidor (chat do RAG).

use std::time::Duration;

use crate::config::config;
use crate::error::Result;

/// Chama o LLM do chat do RAG. Os parâmetros são decisão do servidor:
/// - `temperature 0.1`, sem `top_p`: RAG tributário exige fidelidade ao contexto recuperado, não
///   diversidade — sampling controlado só pela temperature (top_p omitido = 1.0).
/// - `timeout 30 s`: mantido abaixo do budget de 35 s do frontend (callServerAPI.ts) — ver o
///   comentário completo do invariante em `auli-llm`.
pub async fn chat(system_prompt: &str, user_message: &str) -> Result<String> {
    let params = auli_llm::LlmParams {
        api_url: config().llm_api_url.clone(),
        api_key: config().llm_api_key.clone(),
        model: config().llm_api_model.clone(),
        temperature: 0.1,
        max_completion_tokens: 4096,
        timeout: Duration::from_secs(30),
    };
    // O chat do RAG não usa o headroom de rate-limit (isso é do lote de sinopses offline).
    Ok(auli_llm::chat(&params, system_prompt, user_message).await?.text)
}
