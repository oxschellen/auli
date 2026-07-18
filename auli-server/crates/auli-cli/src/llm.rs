// External LLM client — Groq-compatible chat completions.

use std::time::{Duration, Instant};

use reqwest::{Client, StatusCode};
use serde_json::{json, Value};

use crate::config::config;
use crate::error::{Error, Result};

// Send a system + user message to the chat-completions endpoint and return the assistant text.
// Retries up to 3 times on connect/timeout errors (send OR body read). An API-level error — a JSON
// `error` field, a non-JSON body, or a hung request that hits the timeout — is returned as a
// human-readable message rather than an Err, so the handler never leaks a raw serde/reqwest error.
pub async fn chat(system_prompt: &str, user_message: &str) -> Result<String> {
    let start = Instant::now();

    const AUTH_HEADER: &str = "Authorization";
    const CONTENT_TYPE_HEADER: &str = "Content-Type";

    // Timeout kept below the frontend's 35 s budget (callServerAPI.ts): a hung LLM surfaces here as
    // a retryable timeout instead of the client giving up while the handler keeps the paid call open.
    let client = Client::builder().timeout(Duration::from_secs(30)).build()?;
    let auth_token = format!("Bearer {}", config().llm_api_key);
    let request_body = json!({
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": user_message }
        ],
        "model": config().llm_api_model.as_str(),
        "stream": false,
        "temperature": 0.5,
        "max_completion_tokens": 4096,
        "top_p": 0.5,
        "stop": null,
    });

    // (status, body) from the first round-trip that both connects and reads. Retries cover
    // connect/timeout on send AND on body read — both transient.
    let (status, response_text): (StatusCode, String) = {
        let mut result: Result<(StatusCode, String)> = Err(Error::Custom("unreachable".into()));
        for attempt in 1u32..=3 {
            let outcome = async {
                let resp = client
                    .post(config().llm_api_url.as_str())
                    .header(AUTH_HEADER, &auth_token)
                    .header(CONTENT_TYPE_HEADER, "application/json")
                    .json(&request_body)
                    .send()
                    .await?;
                let status = resp.status();
                let text = resp.text().await?;
                Ok::<(StatusCode, String), reqwest::Error>((status, text))
            }
            .await;

            match outcome {
                Ok(pair) => {
                    result = Ok(pair);
                    break;
                }
                Err(e) if (e.is_connect() || e.is_timeout()) && attempt < 3 => {
                    println!("Erro de conexão (tentativa {}/3): {e}. Tentando novamente...", attempt);
                    tokio::time::sleep(Duration::from_millis(500)).await;
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

    let answer = match serde_json::from_str::<Value>(&response_text) {
        Ok(data) => {
            if let Some(err) = data.get("error") {
                format!("Erro na chamada da API do modelo AI: {}!", err)
            } else {
                data["choices"][0]["message"]["content"].as_str().unwrap_or_default().to_string()
            }
        }
        // Non-JSON body (e.g. an HTML 5xx from a proxy) — surface the status, not a raw serde error.
        Err(_) => format!(
            "Erro na chamada da API do modelo AI (HTTP {}): resposta não-JSON do provedor.",
            status
        ),
    };

    let elapsed = start.elapsed().as_millis();
    println!("Tempo de chamada do LLM API : {:6} millisegundos", elapsed);

    Ok(answer)
}
