//! Cliente LLM compartilhado (chat completions compatível com Groq).
//! Consumidores: o servidor (`auli-cli`, chat do RAG) e o `auli-collections` (sinopse, F3+).
//!
//! Transplante fiel do antigo `auli-cli/src/llm.rs`: mesmos 3 retries, mesmas mensagens pt-BR,
//! mesmo tratamento de erro-de-API-vira-texto. Duas mudanças estruturais: `Client` estático
//! (pool/TLS reaproveitados entre chamadas) com timeout POR REQUISIÇÃO, e parâmetros vindos de
//! `LlmParams` (este crate NÃO lê env — quem lê é o chamador).

use std::sync::LazyLock;
use std::time::{Duration, Instant};

use reqwest::StatusCode;
use serde_json::{Value, json};

/// Cliente reaproveitado entre chamadas (pool de conexões + TLS). Sem timeout no builder — o
/// timeout é por requisição (`params.timeout`), com a mesma semântica de antes (cobre send+body).
static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(reqwest::Client::new);

/// Esforço de raciocínio dos modelos "reasoning" (ex.: gpt-oss da Groq). Enviado como
/// `reasoning_effort` só quando o chamador o define; ausente = default do provedor. `Low` quebra
/// loops de reasoning em documentos patológicos (o modelo consome todo o `max_completion_tokens`
/// raciocinando e devolve conteúdo vazio) sem sacrificar a qualidade de tarefas literais.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasoningEffort {
    Low,
    Medium,
    High,
}

impl ReasoningEffort {
    /// Valor exato esperado pela API (`reasoning_effort`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

/// Parâmetros de uma chamada. O chamador monta a partir da SUA config — este crate não lê env.
#[derive(Debug, Clone)]
pub struct LlmParams {
    pub api_url: String,
    pub api_key: String,
    pub model: String,
    pub temperature: f64,
    pub max_completion_tokens: u32,
    /// Timeout por tentativa (o retry continua sendo 3 tentativas — comportamento herdado).
    pub timeout: Duration,
    /// Esforço de raciocínio. `None` = não envia o campo (default do provedor). Só afeta modelos
    /// "reasoning"; ignorado pelos demais.
    pub reasoning_effort: Option<ReasoningEffort>,
}

pub type Result<T> = core::result::Result<T, Error>;

/// Resposta do chat + o headroom de rate-limit lido dos headers (quando o provedor os envia).
/// `remaining_requests` = `x-ratelimit-remaining-requests` (Requests Per Day / RPD) — o consumidor
/// usa para parar ANTES de esgotar a cota (zero rejeição). `reset_requests` = `x-ratelimit-reset-
/// requests` bruto (ex.: `"2m59.56s"`), útil para reportar quando a cota volta. Ambos `None` se o
/// header não veio.
#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub text: String,
    pub remaining_requests: Option<u64>,
    pub reset_requests: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Custom(String),
    #[error("Erro de requisição HTTP: {0}")]
    Reqwest(#[from] reqwest::Error),
}

/// Envia system + user ao endpoint de chat-completions e devolve o texto do assistente. Tenta até
/// 3 vezes em erros de conexão/timeout (no send OU na leitura do body). Um erro de nível de API —
/// campo `error` no JSON, body não-JSON, ou requisição pendurada que estoura o timeout — volta como
/// mensagem legível em vez de `Err`, para o chamador nunca vazar um erro cru de serde/reqwest.
pub async fn chat(params: &LlmParams, system_prompt: &str, user_message: &str) -> Result<ChatResponse> {
    let start = Instant::now();

    const AUTH_HEADER: &str = "Authorization";
    const CONTENT_TYPE_HEADER: &str = "Content-Type";

    let auth_token = format!("Bearer {}", params.api_key);
    let mut request_body = json!({
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": user_message }
        ],
        "model": params.model.as_str(),
        "stream": false,
        "temperature": params.temperature,
        "max_completion_tokens": params.max_completion_tokens,
        "stop": null,
    });
    // Só inclui `reasoning_effort` quando o chamador o define — ausente preserva o default do provedor.
    if let Some(effort) = params.reasoning_effort {
        request_body["reasoning_effort"] = json!(effort.as_str());
    }

    // (status, body, headroom) from the first round-trip that both connects and reads. Retries cover
    // connect/timeout on send AND on body read — both transient. Os headers de rate-limit são lidos
    // ANTES de consumir o body (`resp.text()` move `resp`).
    let (status, response_text, remaining_requests, reset_requests): (
        StatusCode,
        String,
        Option<u64>,
        Option<String>,
    ) = {
        let mut result: Result<(StatusCode, String, Option<u64>, Option<String>)> =
            Err(Error::Custom("unreachable".into()));
        for attempt in 1u32..=3 {
            let outcome = async {
                // Timeout POR REQUISIÇÃO, mantido pelo chamador abaixo do seu próprio budget — no
                // servidor, os 35 s do frontend (callServerAPI.ts): um LLM pendurado vira timeout
                // retryável aqui em vez de o cliente desistir com a chamada paga ainda aberta.
                let resp = CLIENT
                    .post(params.api_url.as_str())
                    .header(AUTH_HEADER, &auth_token)
                    .header(CONTENT_TYPE_HEADER, "application/json")
                    .timeout(params.timeout)
                    .json(&request_body)
                    .send()
                    .await?;
                let status = resp.status();
                let remaining = header_u64(&resp, "x-ratelimit-remaining-requests");
                let reset = header_str(&resp, "x-ratelimit-reset-requests");
                let text = resp.text().await?;
                Ok::<(StatusCode, String, Option<u64>, Option<String>), reqwest::Error>((
                    status, text, remaining, reset,
                ))
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

    Ok(ChatResponse { text: answer, remaining_requests, reset_requests })
}

/// Lê um header como `u64` (aparado), ou `None` se ausente/não-numérico.
fn header_u64(resp: &reqwest::Response, name: &str) -> Option<u64> {
    resp.headers().get(name)?.to_str().ok()?.trim().parse().ok()
}

/// Lê um header como `String` (aparado), ou `None` se ausente/não-textual.
fn header_str(resp: &reqwest::Response, name: &str) -> Option<String> {
    Some(resp.headers().get(name)?.to_str().ok()?.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fumaça: a API pública compila e `LlmParams` é `Debug` + `Clone` (contrato estável para os
    /// chamadores). Não testa o parsing da resposta — a função é monolítica e o critério da F2 é a
    /// fidelidade do transplante, não refatorar para testar.
    #[test]
    fn llm_params_e_debug_e_clone() {
        let p = LlmParams {
            api_url: "https://exemplo/v1/chat/completions".into(),
            api_key: "k".into(),
            model: "llama-3.3-70b-versatile".into(),
            temperature: 0.1,
            max_completion_tokens: 4096,
            timeout: Duration::from_secs(30),
            reasoning_effort: None,
        };
        let p2 = p.clone();
        assert_eq!(format!("{p:?}"), format!("{p2:?}"));
    }
}
