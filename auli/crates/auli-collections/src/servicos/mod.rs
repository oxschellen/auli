// servicos collection scraper.
//
// Two backends share the same output shape (per-tipo `servicos-<tipo>.json`, an aggregated flat
// `servicos.json`, a `servicos-index.json` tab manifest, and the `portal-servicos.txt` knowledge
// file — all under `data/<id>/`):
//   - `rs` (SEFAZ-RS): headless Chrome renders the audience listing pages, then reqwest fetches each
//     service's detail page and scrapes the description (`extrair_descricoes` / `utils`).
//   - `sc` (SEF-SC): a clean Next.js JSON API — no browser, no HTML parsing (`sc`).
//
// `run` dispatches on `entity_id`. The per-tipo files differ per entity (RS audiences vs SC
// públicos), so the downstream txt/json aggregation is driven by a tipo list the backend reports.

mod cache;
mod extrair_descricoes;
mod gerar_portal_servicos;
mod sc;
mod types;
mod utils;

use std::collections::HashSet;

use serde::Serialize;

use types::{Servico, TipoServicos};

/// Scrape an entity's services and write the per-tipo JSON, the aggregated `servicos.json`, the
/// `servicos-index.json` tab manifest, and `portal-servicos.txt` under `data_dir`. Fetched pages are
/// cached under `<data_dir>/cache/servicos/`. Dispatches on `entity_id` (`rs` | `sc`).
pub fn run(entity_id: &str, data_dir: &str, use_cache: bool) -> Result<(), Box<dyn std::error::Error>> {
    match entity_id {
        "rs" => {
            let tipos = utils::get_tipo_servicos();
            let failed = extrair_descricoes::extrair_descricoes_json(data_dir, use_cache)?;
            finish(data_dir, &tipos)?;
            report_failed_detail_urls(&failed);
        }
        "sc" => {
            let tipos = sc::scrape(data_dir, use_cache)?;
            finish(data_dir, &tipos)?;
        }
        other => {
            return Err(format!(
                "scraper de servicos ainda não configurado para a entidade '{}'",
                other
            )
            .into());
        }
    }

    println!("🎉 Processing complete!");
    Ok(())
}

/// Shared post-scrape steps: write the portal txt, the aggregated json, and the tab manifest.
fn finish(data_dir: &str, tipos: &[TipoServicos]) -> Result<(), Box<dyn std::error::Error>> {
    gerar_portal_servicos::gerar_portal_services_txt(data_dir, tipos)?;
    write_servicos_json(data_dir, tipos)?;
    write_servicos_index(data_dir, tipos)?;
    Ok(())
}

/// Prints a summary of the detail-page URLs that failed to load during the scrape.
fn report_failed_detail_urls(failed: &[String]) {
    if failed.is_empty() {
        println!("✅ Todas as páginas de detalhe carregaram com sucesso.");
        return;
    }

    eprintln!(
        "\n⚠️  {} página(s) de detalhe falharam ao carregar:",
        failed.len()
    );
    for url in failed {
        eprintln!("  - {}", url);
    }
}

/// Aggregates the per-tipo JSON files into a single flat `servicos.json`, deduplicated by `link`
/// (the same service can appear under several audiences/públicos — see `gerar_portal_servicos`).
fn write_servicos_json(data_dir: &str, tipos: &[TipoServicos]) -> Result<(), Box<dyn std::error::Error>> {
    let mut all: Vec<Servico> = Vec::new();
    let mut seen_links: HashSet<String> = HashSet::new();

    for tipo in tipos {
        let path = format!("{}/{}.json", data_dir, tipo.filename);
        if !std::path::Path::new(&path).exists() {
            continue;
        }
        for service in utils::load_servicos_from_json(&path)? {
            if seen_links.insert(service.link.clone()) {
                all.push(service);
            }
        }
    }

    let json = serde_json::to_string_pretty(&all)?;
    let out = format!("{}/servicos.json", data_dir);
    std::fs::write(&out, json)?;
    println!("Wrote {} ({} serviços únicos)", out, all.len());
    Ok(())
}

/// One entry of `servicos-index.json` — drives the frontend's audience tabs.
#[derive(Serialize)]
struct ServicoIndexEntry {
    tipo: String,
    filename: String,
}

/// Writes `servicos-index.json`: the list of `{ tipo, filename }` tabs for this entity, so the
/// frontend can render the right audience tabs (and load the right files) without hardcoding them.
fn write_servicos_index(data_dir: &str, tipos: &[TipoServicos]) -> Result<(), Box<dyn std::error::Error>> {
    let entries: Vec<ServicoIndexEntry> = tipos
        .iter()
        .map(|t| ServicoIndexEntry {
            tipo: t.tipo.clone(),
            filename: t.filename.clone(),
        })
        .collect();

    let json = serde_json::to_string_pretty(&entries)?;
    let out = format!("{}/servicos-index.json", data_dir);
    std::fs::write(&out, json)?;
    println!("Wrote {} ({} tabs)", out, entries.len());
    Ok(())
}
