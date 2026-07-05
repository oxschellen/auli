// HTTP fetching for the faqs scraper, with an on-disk cache.
//
// Each page is fetched once and its (pretty-printed) HTML cached under the scrape's cache dir;
// subsequent runs read from disk, so re-runs don't re-hit the portal. The portal serves FAQ/menu
// content through an AJAX endpoint that returns JSON with the rendered markup in a `body` field.

use std::fs;
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

use ureq::Agent;

use crate::errors::Result;
use crate::faqs::html::format_html;

const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64; rv:124.0) Gecko/20100101 Firefox/124.0";
const ACCEPT: &str = "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8";
const ACCEPT_LANGUAGE: &str = "pt-BR,pt;q=0.9,en-US;q=0.8,en;q=0.7";

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
    auli_scraper_kit::build_agent(USER_AGENT, None)
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
    let html = format_html(&raw);
    save(cache_path, &html)?;
    Ok(html)
}

/// Fetches (or reads from cache) the `body` markup returned by the portal's AJAX list endpoint.
pub fn get_web_page_ajax_body_html(
    agent: &Agent,
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
        let mut resp = agent
            .get(ajax_url)
            .header("X-Requested-With", "XMLHttpRequest")
            .header("Referer", url)
            .header("Accept", ACCEPT)
            .header("Accept-Language", ACCEPT_LANGUAGE)
            .call()
            .map_err(|e| format!("AJAX request failed for {}: {}", url, e))?;

        let value: serde_json::Value = resp
            .body_mut()
            .read_json()
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
