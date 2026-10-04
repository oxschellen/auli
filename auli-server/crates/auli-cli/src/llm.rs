//! Wrapper do cliente compartilhado `auli-llm` com a config do servidor (chat do RAG).

use std::time::Duration;

use tracing::warn;

use crate::config::config;
use crate::error::Result;

/// Chama o LLM do chat do RAG. Os parâmetros são decisão do servidor:
/// - `temperature 0.1`, sem `top_p`: RAG tributário exige fidelidade ao contexto recuperado, não
///   diversidade — sampling controlado só pela temperature (top_p omitido = 1.0).
/// - `timeout 30 s` (20 s na triagem): ver o comentário completo do invariante em `auli-llm`.
/// - `max_completion_tokens 8192` (era 4096) e `reasoning_effort` do `.env` (D-SF-6): nos modelos de
///   raciocínio, os tokens de saída incluem o raciocínio. Com esforço alto, 4096 arriscaria o
///   modelo gastar a cota pensando e devolver conteúdo vazio — o mesmo defeito que o `Low` do lote
///   de sinopses existe para quebrar. Medido em 04/10/2026: com `high`, nem 8192 bastam ao
///   gpt-oss-120b (3 de 4 chamadas vazias) — use `medium`; o vazio vira erro em [`texto_ou_aviso`].
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
    let resp = auli_llm::chat(&params, system_prompt, user_message).await?;
    Ok(texto_ou_aviso(resp.text, resp.finish_reason.as_deref()))
}

/// Conteúdo vazio vira texto de erro, no idioma dos erros de API do `auli-llm`.
///
/// Existe porque o vazio passava calado: com `reasoning_effort=high`, o gpt-oss-120b gastou os 8192
/// tokens raciocinando (`finish_reason=length`) em 3 de 4 chamadas, e o usuário recebia uma resposta
/// em branco, sem erro nem aviso no log. Fica aqui, e não no `auli-llm`, porque os lotes offline
/// (sinopse, extração) contam com o vazio para cair na validação e re-tentar com `Low` — um texto de
/// erro de API os faria desistir sem o resgate. Na triagem, o texto não é JSON: ela falha aberta.
fn texto_ou_aviso(texto: String, finish_reason: Option<&str>) -> String {
    if !texto.trim().is_empty() {
        return texto;
    }
    let motivo = finish_reason.unwrap_or("ausente");
    warn!("o LLM devolveu conteúdo vazio (finish_reason={motivo})");
    format!(
        "Erro na chamada da API do modelo AI: o modelo não produziu resposta \
         (finish_reason={motivo}). Tente novamente."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conteudo_vazio_vira_erro_com_o_finish_reason() {
        let t = texto_ou_aviso("  \n".into(), Some("length"));
        assert!(t.starts_with("Erro na chamada da API"), "{t}");
        assert!(t.contains("finish_reason=length"), "{t}");
        assert!(texto_ou_aviso(String::new(), None).contains("finish_reason=ausente"));
    }

    #[test]
    fn conteudo_presente_passa_intacto() {
        assert_eq!(
            texto_ou_aviso("**Resumo**".into(), Some("stop")),
            "**Resumo**"
        );
    }
}
