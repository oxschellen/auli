// External LLM client — Groq-compatible chat completions.

use std::time::Instant;

use reqwest::Client;
use serde_json::{json, Value};

use crate::config::config;
use crate::errors::{Error, Result};

// Send a system + user message to the chat-completions endpoint and return the assistant text.
// Retries up to 3 times on connect/timeout errors. An API-level error is returned as a
// human-readable message rather than an Err.
pub async fn chat(system_prompt: &str, user_message: &str) -> Result<String> {
    let start = Instant::now();
    println!("LLM_API_MODEL: {:?}", config().llm_api_model);

    const AUTH_HEADER: &str = "Authorization";
    const CONTENT_TYPE_HEADER: &str = "Content-Type";

    let client = Client::new();
    let auth_token = format!("Bearer {}", config().llm_api_key);
    let request_body = json!({
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": user_message }
        ],
        "model": config().llm_api_model.as_str(),
        "stream": false,
        "temperature": 0.5,
        "max_completion_tokens": 1024,
        "top_p": 0.5,
        "stop": null,
    });

    let response_text = {
        let mut result: Result<String> = Err(Error::Custom("unreachable".into()));
        for attempt in 1u32..=3 {
            match client
                .post(config().llm_api_url.as_str())
                .header(AUTH_HEADER, &auth_token)
                .header(CONTENT_TYPE_HEADER, "application/json")
                .json(&request_body)
                .send()
                .await
            {
                Ok(resp) => {
                    result = resp.text().await.map_err(Into::into);
                    break;
                }
                Err(e) if (e.is_connect() || e.is_timeout()) && attempt < 3 => {
                    println!("Erro de conexão (tentativa {}/3): {e}. Tentando novamente...", attempt);
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    result = Err(e.into());
                }
                Err(e) => {
                    result = Err(e.into());
                    break;
                }
            }
        }
        result?
    };

    let data: Value = serde_json::from_str(&response_text)?;

    let answer = if let Some(err) = data.get("error") {
        format!("Erro na chamada da API do modelo AI: {}!", err)
    } else {
        data["choices"][0]["message"]["content"].as_str().unwrap_or_default().to_string()
    };

    let elapsed = start.elapsed().as_millis();
    println!("Tempo de chamada do LLM API : {:6} millisegundos", elapsed);

    Ok(answer)
}
