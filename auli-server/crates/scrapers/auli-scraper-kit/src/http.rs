//! HTTP com retry compartilhado — o miolo de fetch que os scrapers repetiam (retry 3× + backoff
//! ×2, corpo vazio = falha). Semântica idêntica às cópias por-entidade que ele substitui; a
//! identidade de rede (User-Agent) e a cortesia entre requisições ficam no call site (agent +
//! `sleep`), que é onde variam por portal.

use std::thread::sleep;
use std::time::Duration;

use anyhow::{Result, anyhow};

/// Opções de fetch. `Default` = 3 tentativas, backoff base 800ms (×2 a cada falha), sem `Accept`,
/// sem prefixo de log.
pub struct GetOpts<'a> {
    /// Prefixo do log de retry (ex.: `"PE"`).
    pub log_prefix: &'a str,
    /// Header `Accept`, se o endpoint exigir (ex.: `Some("application/json;odata=verbose")`).
    pub accept: Option<&'a str>,
    /// Número de tentativas.
    pub attempts: u32,
    /// Atraso inicial do backoff (dobra a cada tentativa falha).
    pub base_delay: Duration,
}

impl Default for GetOpts<'_> {
    fn default() -> Self {
        Self { log_prefix: "", accept: None, attempts: 3, base_delay: Duration::from_millis(800) }
    }
}

/// GET que devolve o corpo como String, com retry/backoff. Corpo vazio conta como falha.
pub fn get_string(agent: &ureq::Agent, url: &str, opts: &GetOpts) -> Result<String> {
    println!("Fetching: {}", url);
    let mut delay = opts.base_delay;
    let mut last = anyhow!("sem tentativa");
    for attempt in 1..=opts.attempts {
        let mut req = agent.get(url);
        if let Some(a) = opts.accept {
            req = req.header("Accept", a);
        }
        match req.call() {
            Ok(mut resp) => match resp.body_mut().read_to_string() {
                Ok(s) if !s.trim().is_empty() => return Ok(s),
                Ok(_) => last = anyhow!("resposta vazia"),
                Err(e) => last = anyhow!(e.to_string()),
            },
            Err(e) => last = anyhow!(e.to_string()),
        }
        if attempt < opts.attempts {
            eprintln!("⚠️  {}: tentativa {} falhou ({}); retentando…", opts.log_prefix, attempt, last);
            sleep(delay);
            delay *= 2;
        }
    }
    Err(anyhow!("falha ao buscar {} após {} tentativas: {}", url, opts.attempts, last))
}

/// POST com corpo JSON e headers extra (ex.: `Authorization`, tenant, `Origin`), com retry/backoff.
/// Devolve o corpo cru como String. Corpo vazio conta como falha.
pub fn post_json(
    agent: &ureq::Agent,
    url: &str,
    headers: &[(&str, &str)],
    body: &serde_json::Value,
    opts: &GetOpts,
) -> Result<String> {
    let mut delay = opts.base_delay;
    let mut last = anyhow!("sem tentativa");
    for attempt in 1..=opts.attempts {
        let mut req = agent.post(url);
        if let Some(a) = opts.accept {
            req = req.header("Accept", a);
        }
        for (k, v) in headers {
            req = req.header(*k, *v);
        }
        match req.send_json(body) {
            Ok(mut resp) => match resp.body_mut().read_to_string() {
                Ok(s) if !s.trim().is_empty() => return Ok(s),
                Ok(_) => last = anyhow!("resposta vazia"),
                Err(e) => last = anyhow!(e.to_string()),
            },
            Err(e) => last = anyhow!(e.to_string()),
        }
        if attempt < opts.attempts {
            eprintln!("⚠️  {}: tentativa {} falhou ({}); retentando…", opts.log_prefix, attempt, last);
            sleep(delay);
            delay *= 2;
        }
    }
    Err(anyhow!("falha no POST {} após {} tentativas: {}", url, opts.attempts, last))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_opts_default() {
        let o = GetOpts::default();
        assert_eq!(o.attempts, 3);
        assert_eq!(o.base_delay, Duration::from_millis(800));
        assert!(o.accept.is_none());
        assert_eq!(o.log_prefix, "");
    }
}
