// servicos scraper (SEFAZ-RS).
//
// Headless Chrome renderiza as páginas de listagem por público; o ureq busca cada página de detalhe
// e raspa a descrição (`extrair_descricoes` / `utils`). Os arquivos per-tipo são gravados durante o
// scrape (recuperação de falha) e agregados em `Vec<ServicoRaw>` no snapshot. A derivação dos
// artefatos (contrato, prints, per-público, index) é o `auli-collections process`.

mod extrair_descricoes;
mod types;
mod utils;

use auli_scraper_kit::PerPublicoServicos;
use types::TipoServicos;

/// Raspa os serviços do RS e grava a coleta no snapshot (`colecoes.servicos`).
pub fn run(data_dir: &str, use_cache: bool) -> Result<(), Box<dyn std::error::Error>> {
    let tipos = utils::get_tipo_servicos();
    let failed = extrair_descricoes::extrair_descricoes_json(data_dir, use_cache)?;
    let inputs = load_per_tipo(data_dir, &tipos)?;
    write_servicos_snapshot(data_dir, &inputs, publicos_ordem_from(&tipos))?;
    report_failed_detail_urls(&failed);
    println!("🎉 Coleta de serviços gravada no snapshot.");
    Ok(())
}

/// Agrega os per-público em memória em `Vec<ServicoRaw>` e grava a coleta no snapshot.
fn write_servicos_snapshot(
    data_dir: &str,
    inputs: &PerPublicoServicos,
    publicos_ordem: Vec<auli_contract::Publico>,
) -> Result<(), Box<dyn std::error::Error>> {
    let items = auli_scraper_kit::aggregate_servicos(inputs);
    auli_scraper_kit::snapshot::write_servicos(
        crate::ENTITY,
        data_dir,
        &crate::scraper_info(),
        publicos_ordem,
        items,
    )?;
    Ok(())
}

/// Lê os arquivos per-tipo (na ordem de `tipos`) para a agregação — o scrape os grava
/// incrementalmente como recuperação de falha. Arquivo ausente é ignorado.
fn load_per_tipo(
    data_dir: &str,
    tipos: &[TipoServicos],
) -> Result<PerPublicoServicos, Box<dyn std::error::Error>> {
    let mut loaded = Vec::new();
    for tipo in tipos {
        let path = format!("{}/{}.json", data_dir, tipo.filename);
        if !std::path::Path::new(&path).exists() {
            continue;
        }
        loaded.push((tipo.tipo.clone(), utils::load_servicos_from_json(&path)?));
    }
    Ok(loaded)
}

/// `publicos_ordem` do snapshot a partir da lista de tipos (`tipo` -> `nome`, `filename` -> `slug`).
fn publicos_ordem_from(tipos: &[TipoServicos]) -> Vec<auli_contract::Publico> {
    tipos
        .iter()
        .map(|t| auli_contract::Publico { nome: t.tipo.clone(), slug: t.filename.clone() })
        .collect()
}

/// Prints a summary of the detail-page URLs that failed to load during the scrape.
fn report_failed_detail_urls(failed: &[String]) {
    if failed.is_empty() {
        println!("✅ Todas as páginas de detalhe carregaram com sucesso.");
        return;
    }

    eprintln!("\n⚠️  {} página(s) de detalhe falharam ao carregar:", failed.len());
    for url in failed {
        eprintln!("  - {}", url);
    }
}
