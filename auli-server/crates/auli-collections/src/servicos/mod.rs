// servicos collection scraper.
//
// Two backends share the same output shape (per-público `<filename>.json`, the contract
// `<id>-servicos.json` (`Table<Servico>`), a `servicos-index.json` tab manifest, and the
// `portal-servicos.txt` knowledge file — all under `data/<id>/`):
//   - `rs` (SEFAZ-RS): headless Chrome renders the audience listing pages, then ureq fetches each
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

use types::TipoServicos;

/// Scrape an entity's services and write the per-público JSON, the contract `<id>-servicos.json`, the
/// `servicos-index.json` tab manifest, and `portal-servicos.txt` under `data_dir`. Fetched pages are
/// cached under `<data_dir>/cache/servicos/`. Dispatches on `entity_id` (`rs` | `sc`).
pub fn run(entity_id: &str, data_dir: &str, use_cache: bool) -> Result<(), Box<dyn std::error::Error>> {
    match entity_id {
        "rs" => {
            let tipos = utils::get_tipo_servicos();
            let failed = extrair_descricoes::extrair_descricoes_json(data_dir, use_cache)?;
            finish(entity_id, data_dir, &tipos)?;
            report_failed_detail_urls(&failed);
        }
        "sc" => {
            let tipos = sc::scrape(data_dir, use_cache)?;
            finish(entity_id, data_dir, &tipos)?;
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

/// Shared post-scrape steps: write the print txt, the contract `Table<Servico>`, and the tab manifest.
fn finish(entity_id: &str, data_dir: &str, tipos: &[TipoServicos]) -> Result<(), Box<dyn std::error::Error>> {
    gerar_portal_servicos::gerar_portal_services_txt(data_dir, tipos)?;
    write_servicos_contract(entity_id, data_dir, tipos)?;
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

/// Builds the contract `Table<Servico>` from the per-tipo JSON files (deduplicated by `link`, since
/// the same service can appear under several audiences/públicos) and writes it to
/// `<data_dir>/<id>-servicos.json`. This is now the single structured output for services; the old
/// aggregated flat `servicos.json` is no longer written (it was not read by the frontend).
///
/// Each record carries `descricao` = the description BODY (the `tipo/classe/titulo` header lines are
/// dropped, exactly like `portal-servicos.txt`) so `Servico::stored_repr` reproduces the print block;
/// and a materialized `text_to_embed` (D2).
fn write_servicos_contract(
    entity_id: &str,
    data_dir: &str,
    tipos: &[TipoServicos],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut items: Vec<auli_contract::Servico> = Vec::new();
    let mut seen_links: HashSet<String> = HashSet::new();
    let mut next_id: usize = 1;

    for tipo in tipos {
        let path = format!("{}/{}.json", data_dir, tipo.filename);
        if !std::path::Path::new(&path).exists() {
            continue;
        }
        for service in utils::load_servicos_from_json(&path)? {
            // One record per link; first occurrence wins (matches the print/dedup ordering).
            if !seen_links.insert(service.link.clone()) {
                continue;
            }
            let body = gerar_portal_servicos::descricao_body(&service.descricao);
            let text_to_embed =
                servico_text_to_embed(&service.tipo, &service.classe, &service.titulo, &body);
            items.push(auli_contract::Servico {
                id: next_id,
                tipo: service.tipo,
                classe: service.classe,
                orgao: service.orgao,
                link: service.link,
                titulo: service.titulo,
                descricao: body,
                text_to_embed,
            });
            next_id += 1;
        }
    }

    let table = auli_contract::Table::new(entity_id, "servicos", items);
    let out = format!("{}/{}-servicos.json", data_dir, entity_id);
    let json = serde_json::to_string_pretty(&table)?;
    std::fs::write(&out, json)?;
    println!("Wrote {} ({} serviços únicos)", out, table.len());
    Ok(())
}

/// Rebuild `<id>-servicos.json` (contract) **offline**, from the already-scraped per-tipo files
/// listed in `servicos-index.json` — no network. Used to regenerate packs after a `STRATEGY_VERSION`
/// bump without re-scraping. No-op if the index is absent.
pub fn rebuild_contract_from_raw(
    entity_id: &str,
    data_dir: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let index_path = format!("{}/servicos-index.json", data_dir);
    if !std::path::Path::new(&index_path).exists() {
        println!("⏭️  {} ausente — pulando servicos", index_path);
        return Ok(());
    }
    #[derive(serde::Deserialize)]
    struct IdxEntry {
        tipo: String,
        filename: String,
    }
    let bytes = std::fs::read(&index_path)?;
    let idx: Vec<IdxEntry> = serde_json::from_slice(&bytes)?;
    // `url` não é usado por write_servicos_contract (só `filename`); valor vazio basta no rebuild.
    let tipos: Vec<TipoServicos> = idx
        .into_iter()
        .map(|e| TipoServicos { tipo: e.tipo, filename: e.filename, url: String::new() })
        .collect();
    println!("rebuild offline de servicos ({}): {} tipos", entity_id, tipos.len());
    write_servicos_contract(entity_id, data_dir, &tipos)
}

/// `text_to_embed` for a service (D2): the breadcrumb `tipo | classe`, the title, and the start of
/// the description body. Provisional formula — the PLANO leaves the exact `servicos` key as a pending
/// item; re-vectorization is expected (the goal is retrieval equivalence, not bit-parity).
fn servico_text_to_embed(tipo: &str, classe: &str, titulo: &str, body: &str) -> String {
    let snippet: String = body.chars().take(300).collect();
    format!("{} | {}\n{}\n{}", tipo, classe, titulo, snippet.trim())
        .trim()
        .to_string()
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

#[cfg(test)]
mod tests {
    use super::*;
    use auli_contract::Embeddable;

    #[test]
    fn text_to_embed_is_breadcrumb_title_and_body_start() {
        let key = servico_text_to_embed("Empresas", "ICMS", "Emitir guia", "Passos para emitir.");
        assert_eq!(key, "Empresas | ICMS\nEmitir guia\nPassos para emitir.");
    }

    #[test]
    fn contract_servico_stored_repr_matches_print_block() {
        // descricao já é o CORPO (sem o header tipo/classe/titulo), como gravado no contrato.
        let s = auli_contract::Servico {
            id: 1,
            tipo: "Empresas".into(),
            classe: "ICMS".into(),
            orgao: "SEFAZ".into(),
            link: "https://x/svc/1".into(),
            titulo: "Emitir guia".into(),
            descricao: "Passos para emitir a guia.".into(),
            text_to_embed: "Empresas | ICMS\nEmitir guia\nPassos para emitir a guia.".into(),
        };
        // Mesmo conteúdo do bloco de portal-servicos.txt (sem o `// N.` e a newline final).
        let expected = "## pergunta\nEmpresas | ICMS\nEmitir guia\n\n## resposta\nPassos para emitir a guia.\nLink: https://x/svc/1";
        assert_eq!(s.stored_repr(), expected);
    }
}
