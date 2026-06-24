use std::{
    collections::HashSet,
    io::{self, Write},
    path::Path,
};

use super::types::{Servico, TipoServicos};
use super::utils::load_servicos_from_json;

/// Builds `portal-servicos.txt` from the per-tipo JSON files listed in `tipos`.
///
/// Services are deduplicated by `link`: an entity may list the same service under more than one
/// audience (e.g. SC's `publicos` is a list, so one service can appear in several per-público files),
/// and the RAG knowledge file must contain each service exactly once — otherwise the vector DB ends
/// up with duplicate blocks. The first occurrence wins; later duplicates are skipped. (RS ids restart
/// at 1 per file, so `link` — not `id` — is the only globally unique key.)
pub fn gerar_portal_services_txt(
    data_dir: &str,
    tipos: &[TipoServicos],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut formatted_results = String::new();
    let mut seen_links: HashSet<String> = HashSet::new();
    let mut index: usize = 1;

    for tipo_servicos in tipos {
        let filename_json = format!("{}/{}.json", data_dir, tipo_servicos.filename);
        if !Path::new(&filename_json).exists() {
            eprintln!("⚠️  arquivo de tipo ausente, ignorando: {}", filename_json);
            continue;
        }

        let vec_services: Vec<Servico> = load_servicos_from_json(&filename_json)?;
        for service in &vec_services {
            // Skip a service already emitted under another audience — keep one block per link.
            if !seen_links.insert(service.link.clone()) {
                continue;
            }

            // Match the portal-faqs.txt block shape: a `// N.` comment line, a `## pergunta` block
            // (breadcrumb `tipo | classe` + the service title), then a `## resposta` block with the
            // description body and the link.
            let breadcrumb = format!("{} | {}", service.tipo, service.classe);
            let body = descricao_body(&service.descricao);

            let formatted_output = format!(
                "// {}.\n## pergunta\n{}\n{}\n\n## resposta\n{}\nLink: {}\n\n",
                index, breadcrumb, service.titulo, body, service.link
            );

            index += 1;
            formatted_results.push_str(&formatted_output);
        }
    }

    let out = format!("{}/portal-servicos.txt", data_dir);
    save_to_file(&formatted_results, &out)?;
    println!("Wrote {} ({} serviços únicos)", out, index - 1);

    Ok(())
}

/// The service description without its leading `tipo / classe / titulo` header lines, which
/// `build_descricao` (in extrair_descricoes.rs) prepends. Those three fields are emitted in the
/// `## pergunta` block instead, so dropping them here avoids duplicating them in `## resposta`.
/// An empty/missing description yields an empty body.
pub(super) fn descricao_body(descricao: &str) -> String {
    descricao
        .lines()
        .skip(3)
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

pub fn save_to_file(content: &str, filename: &str) -> io::Result<()> {
    let mut file = std::fs::File::create(filename)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}
