//! HTTP com retry compartilhado — o miolo de fetch que os scrapers repetiam (retry 3× + backoff
//! ×2, corpo vazio = falha). Semântica idêntica às cópias por-entidade que ele substitui; a
//! identidade de rede (User-Agent) e a cortesia entre requisições ficam no call site (agent +
//! `sleep`), que é onde variam por portal.

use std::process::Command;
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
    /// Headers extra do GET (ex.: `Accept-Language`, `X-Portal`), além do `Accept`. Default vazio.
    pub headers: &'a [(&'a str, &'a str)],
    /// Número de tentativas.
    pub attempts: u32,
    /// Atraso inicial do backoff (dobra a cada tentativa falha).
    pub base_delay: Duration,
}

impl Default for GetOpts<'_> {
    fn default() -> Self {
        Self {
            log_prefix: "",
            accept: None,
            headers: &[],
            attempts: 3,
            base_delay: Duration::from_millis(800),
        }
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
        for (k, v) in opts.headers {
            req = req.header(*k, *v);
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
            eprintln!(
                "⚠️  {}: tentativa {} falhou ({}); retentando…",
                opts.log_prefix, attempt, last
            );
            sleep(delay);
            delay *= 2;
        }
    }
    Err(anyhow!(
        "falha ao buscar {} após {} tentativas: {}",
        url,
        opts.attempts,
        last
    ))
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
            eprintln!(
                "⚠️  {}: tentativa {} falhou ({}); retentando…",
                opts.log_prefix, attempt, last
            );
            sleep(delay);
            delay *= 2;
        }
    }
    Err(anyhow!(
        "falha no POST {} após {} tentativas: {}",
        url,
        opts.attempts,
        last
    ))
}

/// GET via **subprocess `curl`**, para hosts atrás de WAF que faz *allowlist por fingerprint TLS
/// (JA3)*: o ClientHello do `ureq` (rustls ou native-tls) difere do curl nas **extensões** (falta
/// ALPN, sobra `session_ticket`), e o `TlsConfig` do ureq 3 não expõe ALPN/cipher-list para
/// alinhar. O curl usa o OpenSSL do sistema, cujo JA3 está na allowlist. Ver a nota de WAF no
/// scraper que chama isto (GO). **Exceção documentada** — como o backend native-tls do BA — usada
/// só onde o `ureq` é barrado; o resto da coleta segue em [`get_string`].
///
/// Segurança: argumentos **separados** (`Command::arg`), nunca via shell — a URL/headers jamais
/// passam por um interpretador. `--fail-with-body` faz o curl retornar status ≠ 0 em resposta
/// HTTP ≥ 400 (senão um "Acesso Negado" 200 ou um 5xx viraria "sucesso" com corpo HTML). Requer o
/// binário `curl` no PATH (dependência de runtime — registrada na doc do scraper); ausência vira
/// erro descritivo, não panic.
///
/// Aproxima o comportamento de [`get_string`]: honra `opts.accept`, `opts.headers`, `opts.attempts`
/// e `opts.base_delay` (mesmo backoff ×2). `opts.log_prefix` prefixa o log de retry.
///
/// O User-Agent vem de quem chama (D-FROTA-UA): o call site diz qual e por quê — o AuliBot por padrão,
/// o [`crate::USER_AGENT`] de navegador só com o motivo comentado.
pub fn get_via_curl(url: &str, user_agent: &str, opts: &GetOpts) -> Result<String> {
    ensure_curl_available()?;
    println!("Fetching (curl): {}", url);
    com_retry_curl(
        "ao buscar",
        url,
        opts,
        &args_curl(url, user_agent, &[], opts),
    )
}

/// POST de formulário (`application/x-www-form-urlencoded`) via **subprocess `curl`** — o mesmo caminho
/// e as mesmas garantias do [`get_via_curl`], para os hosts com WAF JA3 que recebem formulário (as FAQs
/// do DF). Cada par vai num `--data-urlencode nome=valor`: o curl codifica o valor e faz o POST.
pub fn post_form_via_curl(
    url: &str,
    user_agent: &str,
    form: &[(&str, &str)],
    opts: &GetOpts,
) -> Result<String> {
    ensure_curl_available()?;
    println!("Posting (curl): {}", url);
    com_retry_curl(
        "no POST",
        url,
        opts,
        &args_curl(url, user_agent, form, opts),
    )
}

/// Retry/backoff do caminho curl. Corpo vazio conta como falha.
fn com_retry_curl(oque: &str, url: &str, opts: &GetOpts, args: &[String]) -> Result<String> {
    let mut delay = opts.base_delay;
    let mut last = anyhow!("sem tentativa");
    for attempt in 1..=opts.attempts {
        match run_curl_once(args) {
            Ok(body) if !body.trim().is_empty() => return Ok(body),
            Ok(_) => last = anyhow!("resposta vazia"),
            Err(e) => last = e,
        }
        if attempt < opts.attempts {
            eprintln!(
                "⚠️  {}: tentativa {} (curl) falhou ({}); retentando…",
                opts.log_prefix, attempt, last
            );
            sleep(delay);
            delay *= 2;
        }
    }
    Err(anyhow!(
        "falha {} {} via curl após {} tentativas: {}",
        oque,
        url,
        opts.attempts,
        last
    ))
}

/// Os argumentos do curl. `-sS` (silencioso, mas mostra erro); `--fail-with-body` (status ≠ 0 em
/// HTTP ≥ 400, ainda entregando o corpo no stdout p/ diagnóstico); o UA de quem chama; `Accept` e
/// headers extra do `GetOpts`; os campos do formulário, se houver. Args separados — sem shell.
fn args_curl(url: &str, user_agent: &str, form: &[(&str, &str)], opts: &GetOpts) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "-sS".into(),
        "--fail-with-body".into(),
        "-A".into(),
        user_agent.into(),
    ];
    if let Some(a) = opts.accept {
        args.extend(["-H".into(), format!("Accept: {}", a)]);
    }
    for (k, v) in opts.headers {
        args.extend(["-H".into(), format!("{}: {}", k, v)]);
    }
    for (k, v) in form {
        args.extend(["--data-urlencode".into(), format!("{}={}", k, v)]);
    }
    args.extend(["--".into(), url.into()]); // `--` encerra as flags: a URL nunca é confundida com opção.
    args
}

/// Uma execução do curl com os argumentos de [`args_curl`].
fn run_curl_once(args: &[String]) -> Result<String> {
    let out = Command::new("curl")
        .args(args)
        .output()
        .map_err(|e| anyhow!("falha ao executar curl: {}", e))?;
    if !out.status.success() {
        let code = out
            .status
            .code()
            .map(|c| c.to_string())
            .unwrap_or_else(|| "sinal".into());
        let stderr = String::from_utf8_lossy(&out.stderr);
        let body_head: String = String::from_utf8_lossy(&out.stdout)
            .chars()
            .take(200)
            .collect();
        return Err(anyhow!(
            "curl exit {} ({}); corpo[..200]: {}",
            code,
            stderr.trim(),
            body_head
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Preflight único: `curl --version`. Erro descritivo (não panic) se o binário faltar — a coleta
/// do GO depende dele por causa do WAF; a mensagem aponta o porquê.
fn ensure_curl_available() -> Result<()> {
    match Command::new("curl").arg("--version").output() {
        Ok(o) if o.status.success() => Ok(()),
        Ok(_) => Err(anyhow!(
            "`curl` respondeu com erro ao --version; verifique a instalação"
        )),
        Err(_) => Err(anyhow!(
            "este scraper requer o binário `curl` no PATH (o host da API está atrás de WAF que \
             só aceita o fingerprint TLS do curl — ver nota de WAF no scraper). Instale curl."
        )),
    }
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
        assert!(o.headers.is_empty());
        assert_eq!(o.log_prefix, "");
    }

    #[test]
    fn args_curl_do_get_levam_o_ua_de_quem_chama_e_a_url_depois_do_separador() {
        let opts = GetOpts {
            accept: Some("application/json"),
            headers: &[("Referer", "https://x/")],
            ..Default::default()
        };
        assert_eq!(
            args_curl("https://h/p", "AuliBot/0.1", &[], &opts),
            [
                "-sS",
                "--fail-with-body",
                "-A",
                "AuliBot/0.1",
                "-H",
                "Accept: application/json",
                "-H",
                "Referer: https://x/",
                "--",
                "https://h/p"
            ]
        );
    }

    #[test]
    fn args_curl_do_post_mandam_cada_campo_num_data_urlencode() {
        let args = args_curl(
            "https://h/busca.cfm",
            "AuliBot/0.1",
            &[("codAssunto", "24"), ("buscaPergunta", "")],
            &GetOpts::default(),
        );
        assert_eq!(
            args,
            [
                "-sS",
                "--fail-with-body",
                "-A",
                "AuliBot/0.1",
                "--data-urlencode",
                "codAssunto=24",
                "--data-urlencode",
                "buscaPergunta=",
                "--",
                "https://h/busca.cfm"
            ]
        );
    }

    #[test]
    fn ensure_curl_available_ok_ou_erro_descritivo() {
        // CI pode não ter curl; aceita o Ok OU a mensagem didática (nunca panic).
        match ensure_curl_available() {
            Ok(()) => {}
            Err(e) => assert!(e.to_string().contains("curl"), "erro deve citar curl: {e}"),
        }
    }
}
