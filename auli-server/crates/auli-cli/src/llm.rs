//! Wrapper do cliente compartilhado `auli-llm` com a config do servidor (chat do RAG).

use std::time::Duration;

use crate::config::config;
use crate::error::Result;

/// Chama o LLM do chat do RAG. Os parâmetros são decisão do servidor:
/// - `temperature 0.1`, sem `top_p`: RAG tributário exige fidelidade ao contexto recuperado, não
///   diversidade — sampling controlado só pela temperature (top_p omitido = 1.0).
/// - `timeout 30 s` por chamada: ver o comentário completo do invariante em `auli-llm`.
/// - `max_completion_tokens 8192` (era 4096) e `reasoning_effort` do `.env` (D-SF-6): nos modelos de
///   raciocínio, os tokens de saída incluem o raciocínio. Com esforço alto, 4096 arriscaria o
///   modelo gastar a cota pensando e devolver conteúdo vazio — o mesmo defeito que o `Low` do lote
///   de sinopses existe para quebrar. Risco previsto, não medido.
pub async fn chat(system_prompt: &str, user_message: &str) -> Result<String> {
    let params = auli_llm::LlmParams {
        api_url: config().llm_api_url.clone(),
        api_key: config().llm_api_key.clone(),
        model: config().llm_api_model.clone(),
        temperature: 0.1,
        max_completion_tokens: 8192,
        timeout: Duration::from_secs(30),
        reasoning_effort: config().llm_reasoning_effort,
    };
    // O chat do RAG não usa o headroom de rate-limit (isso é do lote de sinopses offline).
    Ok(auli_llm::chat(&params, system_prompt, user_message)
        .await?
        .text)
}
