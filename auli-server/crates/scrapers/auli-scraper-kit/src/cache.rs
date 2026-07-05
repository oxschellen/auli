// On-disk page cache for the servicos scraper, mirroring the faqs cache.
//
// Each fetched page is stored under `<data_dir>/cache/servicos/` keyed by a sanitized URL, so
// re-runs read from disk instead of re-hitting the portal. This matters most for the expensive
// per-tipo listing pages but also spares the many service detail pages. (This kit is generic —
// its consumers fetch plain HTTP; the RS-only headless-Chrome path lives in auli-scraper-rs.)
// Only successful fetches are cached; failed requests are left uncached so they retry next run.

use std::path::PathBuf;

/// Directory where fetched servicos pages are cached, derived from the entity's `data_dir`.
fn cache_dir(data_dir: &str) -> PathBuf {
    PathBuf::from(format!("{}/cache/servicos", data_dir))
}

/// Cache file path for a URL (non `[A-Za-z0-9-_.]` chars become `_`).
fn cache_path(data_dir: &str, url: &str) -> PathBuf {
    cache_dir(data_dir).join(format!("{}.html", url_to_filename(url)))
}

/// Returns the cached page for `url` if one exists and is non-empty.
pub fn read(data_dir: &str, url: &str) -> Option<String> {
    match std::fs::read_to_string(cache_path(data_dir, url)) {
        Ok(content) if !content.trim().is_empty() => Some(content),
        _ => None,
    }
}

/// Writes `content` to the cache for `url`, creating the directory as needed.
/// Best-effort: failures are logged, not propagated, so caching never breaks a scrape.
pub fn write(data_dir: &str, url: &str, content: &str) {
    let path = cache_path(data_dir, url);
    if let Some(parent) = path.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        eprintln!(
            "⚠️  cache servicos: não foi possível criar {:?}: {}",
            parent, e
        );
        return;
    }
    if let Err(e) = std::fs::write(&path, content) {
        eprintln!(
            "⚠️  cache servicos: não foi possível gravar {:?}: {}",
            path, e
        );
    }
}

/// Leitura do cache com a convenção da frota: `Some` = hit (imprime o `Cache hit:` padrão),
/// `None` = "vá à rede", `Err` = miss em modo `--usecache` (mensagem canônica). Cobre só a leitura;
/// a ordem cache-pós-guards (rj/ms/ce/mt) fica no call site, que decide quando gravar.
pub fn read_or_bail(data_dir: &str, url: &str, use_cache: bool) -> anyhow::Result<Option<String>> {
    if let Some(cached) = read(data_dir, url) {
        println!("Cache hit: {}", url);
        return Ok(Some(cached));
    }
    if use_cache {
        anyhow::bail!("cache miss para {} (modo --usecache, sem rede)", url);
    }
    Ok(None)
}

/// Turns a URL into a safe cache filename (non `[A-Za-z0-9-_.]` chars become `_`).
fn url_to_filename(url: &str) -> String {
    url.chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' => c,
            _ => '_',
        })
        .collect()
}
