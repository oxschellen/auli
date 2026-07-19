// On-disk page cache for scrapers, mirroring the faqs cache.
//
// Each fetched page is stored under `<data_dir>/cache/<kind>/` keyed by a sanitized URL, so
// re-runs read from disk instead of re-hitting the portal. `kind` is the cache namespace: most
// scrapers use `"servicos"` (the catálogo); a scraper that fetches a distinct content type under the
// same `data_dir` (e.g. RS, que também coleta `"pareceres"`) passa outro namespace, e cada tipo
// ganha seu próprio `cache/<kind>/` — sem colidir nem aninhar. This matters most for the expensive
// listing pages but also spares the many detail pages. (This kit is generic — its consumers fetch
// plain HTTP; the RS-only headless-Chrome path lives in auli-scraper-rs.)
// Only successful fetches are cached; failed requests are left uncached so they retry next run.

use std::path::PathBuf;

/// Directory where fetched pages for `kind` are cached, derived from the entity's `data_dir`.
fn cache_dir(data_dir: &str, kind: &str) -> PathBuf {
    PathBuf::from(format!("{}/cache/{}", data_dir, kind))
}

/// Cache file path for a URL (non `[A-Za-z0-9-_.]` chars become `_`).
fn cache_path(data_dir: &str, kind: &str, url: &str) -> PathBuf {
    cache_dir(data_dir, kind).join(format!("{}.html", url_to_filename(url)))
}

/// Returns the cached page for `url` under `kind` if one exists and is non-empty.
pub fn read(data_dir: &str, kind: &str, url: &str) -> Option<String> {
    match std::fs::read_to_string(cache_path(data_dir, kind, url)) {
        Ok(content) if !content.trim().is_empty() => Some(content),
        _ => None,
    }
}

/// Writes `content` to the `kind` cache for `url`, creating the directory as needed.
/// Best-effort: failures are logged, not propagated, so caching never breaks a scrape.
pub fn write(data_dir: &str, kind: &str, url: &str, content: &str) {
    let path = cache_path(data_dir, kind, url);
    if let Some(parent) = path.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        eprintln!(
            "⚠️  cache {kind}: não foi possível criar {:?}: {}",
            parent, e
        );
        return;
    }
    if let Err(e) = std::fs::write(&path, content) {
        eprintln!(
            "⚠️  cache {kind}: não foi possível gravar {:?}: {}",
            path, e
        );
    }
}

/// Leitura do cache com a convenção da frota: `Some` = hit (imprime o `Cache hit:` padrão),
/// `None` = "vá à rede", `Err` = miss em modo `--usecache` (mensagem canônica). Cobre só a leitura;
/// a ordem cache-pós-guards (rj/ms/ce/mt) fica no call site, que decide quando gravar.
pub fn read_or_bail(
    data_dir: &str,
    kind: &str,
    url: &str,
    use_cache: bool,
) -> anyhow::Result<Option<String>> {
    if let Some(cached) = read(data_dir, kind, url) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cada_kind_grava_no_seu_diretorio() {
        let base = std::env::temp_dir().join(format!("auli-cache-test-{}", std::process::id()));
        let base = base.to_str().unwrap();
        assert_eq!(cache_dir(base, "servicos"), PathBuf::from(format!("{base}/cache/servicos")));
        assert_eq!(cache_dir(base, "pareceres"), PathBuf::from(format!("{base}/cache/pareceres")));
    }

    #[test]
    fn mesma_url_em_kinds_diferentes_nao_colide() {
        let dir = std::env::temp_dir().join(format!("auli-cache-kind-{}", std::process::id()));
        let base = dir.to_str().unwrap();
        let url = "http://exemplo/doc?id=1";
        write(base, "servicos", url, "conteudo-servicos");
        write(base, "pareceres", url, "conteudo-pareceres");
        assert_eq!(read(base, "servicos", url).as_deref(), Some("conteudo-servicos"));
        assert_eq!(read(base, "pareceres", url).as_deref(), Some("conteudo-pareceres"));
        // Um kind sem gravação não vê o do outro.
        assert_eq!(read(base, "faqs", url), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
