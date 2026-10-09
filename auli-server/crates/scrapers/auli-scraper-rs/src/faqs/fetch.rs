// HTTP fetching for the faqs scraper, with an on-disk cache.
//
// Each page is fetched once and its (pretty-printed) HTML cached under the scrape's cache dir;
// subsequent runs read from disk, so re-runs don't re-hit the portal. The portal serves FAQ/menu
// content through an AJAX endpoint that returns JSON with the rendered markup in a `body` field.

use std::fs;
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

use auli_scraper_kit::http::GetOpts;
use ureq::Agent;

use crate::errors::Result;
use crate::faqs::html::format_html;

const ACCEPT: &str = "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8";
const ACCEPT_LANGUAGE: &str = "pt-BR,pt;q=0.9,en-US;q=0.8,en;q=0.7";
// O F5 na frente de `_service/*` (desde out/2026) rejeita o ureq pela impressão do cliente e devolve
// um desafio JS, com 200, ao AJAX que pede `Accept: text/html`. Passa o curl com o `Accept` de um
// `$.getJSON` — daí o AJAX ir por `get_via_curl` e as páginas seguirem no ureq.
const ACCEPT_JSON: &str = "application/json, text/javascript, */*; q=0.01";

/// Number of attempts for a network operation before giving up.
const MAX_ATTEMPTS: u32 = 3;
/// Initial backoff between attempts (doubles each retry).
const RETRY_BASE_DELAY: Duration = Duration::from_millis(800);

/// Runs `op`, retrying transient failures (e.g. connection reset) with exponential backoff.
fn retry<T>(label: &str, mut op: impl FnMut() -> Result<T>) -> Result<T> {
    let mut delay = RETRY_BASE_DELAY;
    let mut last_err = None;

    for attempt in 1..=MAX_ATTEMPTS {
        match op() {
            Ok(value) => return Ok(value),
            Err(e) => {
                eprintln!(
                    "  fetch attempt {}/{} failed for {}: {}",
                    attempt, MAX_ATTEMPTS, label, e
                );
                last_err = Some(e);
                if attempt < MAX_ATTEMPTS {
                    sleep(delay);
                    delay *= 2;
                }
            }
        }
    }

    Err(last_err.expect("at least one attempt runs before returning an error"))
}

/// Builds a ureq agent with a browser-like User-Agent. (Accept headers are set per request.)
pub fn build_agent() -> Agent {
    auli_scraper_kit::build_agent(auli_scraper_kit::USER_AGENT, None)
}

/// Fetches (or reads from cache) the rendered HTML of `url`.
/// In `use_cache` (offline) mode a cache miss is an error instead of a network fetch.
pub fn get_web_page_html(
    agent: &Agent,
    url: &str,
    cache_path: &Path,
    use_cache: bool,
) -> Result<String> {
    if cache_path.exists() {
        return Ok(fs::read_to_string(cache_path)?);
    }
    if use_cache {
        return Err(cache_miss(url, cache_path));
    }

    let raw = retry(url, || {
        let mut resp = agent
            .get(url)
            .header("Accept", ACCEPT)
            .header("Accept-Language", ACCEPT_LANGUAGE)
            .call()?;
        Ok(resp.body_mut().read_to_string()?)
    })?;
    // Vem com 200, então o `retry` não a vê; gravada, a próxima rodada a leria do cache como página.
    if crate::f5::bloqueio_f5(&raw) {
        return Err(format!("{}: o F5 devolveu desafio/bloqueio no lugar da página", url).into());
    }
    let html = format_html(&raw);
    save(cache_path, &html)?;
    Ok(html)
}

/// Fetches (or reads from cache) the `body` markup returned by the portal's AJAX list endpoint.
pub fn get_web_page_ajax_body_html(
    url: &str,
    cache_path: &Path,
    ajax_url: &str,
    use_cache: bool,
) -> Result<String> {
    if cache_path.exists() {
        return Ok(fs::read_to_string(cache_path)?);
    }
    if use_cache {
        return Err(cache_miss(url, cache_path));
    }

    let raw_body = retry(url, || {
        let raw = auli_scraper_kit::http::get_via_curl(
            ajax_url,
            &GetOpts {
                log_prefix: "RS",
                headers: &[
                    ("X-Requested-With", "XMLHttpRequest"),
                    ("Referer", url),
                    ("Accept", ACCEPT_JSON),
                    ("Accept-Language", ACCEPT_LANGUAGE),
                ],
                attempts: 1, // o `retry` em volta já repete — e repete também o parse.
                ..Default::default()
            },
        )
        .map_err(|e| format!("AJAX request failed for {}: {}", url, e))?;

        let value: serde_json::Value = serde_json::from_str(&raw)
            .map_err(|e| format!("AJAX parse failed for {}: {}", url, e))?;

        let body = value["body"]
            .as_str()
            .ok_or_else(|| format!("Missing 'body' field in AJAX response for {}", url))?
            .to_string();
        Ok(body)
    })?;

    let body_html = format_html(&raw_body);
    save(cache_path, &body_html)?;
    Ok(body_html)
}

/// Error returned in `--usecache` mode when a page has no cache file to read.
fn cache_miss(url: &str, cache_path: &Path) -> crate::errors::Error {
    format!(
        "cache miss para {} (sem {}): modo --usecache, sem acesso à rede",
        url,
        cache_path.display()
    )
    .into()
}

fn save(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::f5::teste::{DESAFIO, arquivos, dir_vazio, servidor_local};

    #[test]
    fn get_web_page_html_nao_grava_no_cache_a_pagina_do_f5() {
        let dir = dir_vazio("faq-html-f5");
        let url = format!("{}/perguntas-frequentes", servidor_local(DESAFIO));
        let r = get_web_page_html(&build_agent(), &url, &dir.join("pagina.html"), false);
        let n = arquivos(&dir);
        let _ = std::fs::remove_dir_all(&dir);
        assert!(r.is_err(), "a página do F5 tem de ser erro, não conteúdo");
        assert_eq!(n, 0, "a página do F5 não pode entrar no cache");
    }

    #[test]
    fn get_web_page_html_grava_no_cache_a_pagina_legitima() {
        // Controle: o mesmo caminho grava quando o conteúdo é bom — o "0 arquivos" acima não é vácuo.
        let dir = dir_vazio("faq-html-ok");
        let url = format!("{}/perguntas-frequentes", servidor_local("<html>ok</html>"));
        let r = get_web_page_html(&build_agent(), &url, &dir.join("pagina.html"), false);
        let n = arquivos(&dir);
        let _ = std::fs::remove_dir_all(&dir);
        assert!(r.is_ok());
        assert_eq!(n, 1);
    }
}
