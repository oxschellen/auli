//! Wrapper do cliente compartilhado `auli-llm` com a config do servidor (chat do RAG).

use std::time::Duration;

use crate::config::config;
use crate::error::Result;

/// Chama o LLM do chat do RAG. Os parâmetros são decisão do servidor:
/// - `temperature 0.1`, sem `top_p`: RAG tributário exige fidelidade ao contexto recuperado, não
///   diversidade — sampling controlado só pela temperature (top_p omitido = 1.0).
/// - `timeout 30 s` (20 s na triagem): ver o comentário completo do invariante em `auli-llm`.
/// - `max_completion_tokens 8192` (era 4096) e `reasoning_effort` do `.env` (D-SF-6): nos modelos de
///   raciocínio, os tokens de saída incluem o raciocínio. Com esforço alto, 4096 arriscaria o
///   modelo gastar a cota pensando e devolver conteúdo vazio — o mesmo defeito que o `Low` do lote
///   de sinopses existe para quebrar. Risco previsto, não medido.
pub async fn chat(system_prompt: &str, user_message: &str) -> Result<String> {
    chamar(system_prompt, user_message, Duration::from_secs(30)).await
}

/// A chamada da TRIAGEM de serviços+faqs (D-SF-9): mesmos parâmetros do [`chat`], teto menor.
///
/// O orçamento é o das duas chamadas somadas: 20 s aqui + 30 s na resposta ficam abaixo dos 70 s
/// do frontend (`callServerAPI.ts`). A triagem devolve um JSON curto, e se estourar o tempo a
/// consulta segue sem ela (fail-open — ver `triagem.rs`), então o teto mais apertado é dela.
///
/// Os 20 s são o teto do passo INTEIRO, não só de cada tentativa: o `auli-llm` repete até três
/// vezes em timeout, e sem o `tokio::time::timeout` por fora a triagem poderia consumir 60 s antes
/// de a resposta começar. Na triagem, desistir cedo é o certo — o contexto integral está à mão.
pub async fn triagem(system_prompt: &str, user_message: &str) -> Result<String> {
    let teto = Duration::from_secs(20);
    match tokio::time::timeout(teto, chamar(system_prompt, user_message, teto)).await {
        Ok(r) => r,
        Err(_) => Err(format!("excedeu o teto de {} s", teto.as_secs()).into()),
    }
}

async fn chamar(system_prompt: &str, user_message: &str, timeout: Duration) -> Result<String> {
    let params = auli_llm::LlmParams {
        api_url: config().llm_api_url.clone(),
        api_key: config().llm_api_key.clone(),
        model: config().llm_api_model.clone(),
        temperature: 0.1,
        max_completion_tokens: 8192,
        timeout,
        reasoning_effort: config().llm_reasoning_effort,
    };
    // O chat do RAG não usa o headroom de rate-limit (isso é do lote de sinopses offline).
    Ok(auli_llm::chat(&params, system_prompt, user_message)
        .await?
        .text)
}
