use serde_json::to_string_pretty;
use std::fs::File;
use std::io::Write;

use super::types::{Servico, TipoServicos};

pub fn get_tipo_servicos() -> Vec<TipoServicos> {
    vec![
        TipoServicos {
            tipo: "Cidadãos".to_string(),
            filename: "servicos-ao-cidadao".to_string(),
            url: "https://www.fazenda.rs.gov.br/servicos-ao-cidadao".to_string(),
        },
        TipoServicos {
            tipo: "Empresas".to_string(),
            filename: "servicos-a-empresas".to_string(),
            url: "https://www.fazenda.rs.gov.br/servicos-a-empresas".to_string(),
        },
        TipoServicos {
            tipo: "Fornecedores".to_string(),
            filename: "servicos-a-fornecedores".to_string(),
            url: "https://www.fazenda.rs.gov.br/servicos-a-fornecedores".to_string(),
        },
        TipoServicos {
            tipo: "Agentes".to_string(),
            filename: "servicos-a-agentes-publicos".to_string(),
            url: "https://www.fazenda.rs.gov.br/servicos-a-agentes-publicos".to_string(),
        },
        TipoServicos {
            tipo: "Servidores".to_string(),
            filename: "servicos-a-servidores-publicos".to_string(),
            url: "https://www.fazenda.rs.gov.br/servicos-a-servidores-publicos".to_string(),
        },
    ]
}

/// Caminho do arquivo de recuperação incremental per-tipo do scrape, num subdiretório próprio
/// (`raw/scrape/`) para NÃO colidir com os JSONs per-público que o `auli-collections process` grava
/// em `raw/<slug>.json` (os slugs do RS são idênticos aos `filename`). Gravado por
/// `extrair_descricoes` a cada serviço e relido por `load_per_tipo` para agregar no snapshot.
pub fn scrape_recovery_path(data_dir: &str, filename: &str) -> String {
    format!("{}/scrape/{}.json", data_dir, filename)
}

pub fn load_servicos_from_json(path: &str) -> Result<Vec<Servico>, Box<dyn std::error::Error>> {
    let content = if path.starts_with("http://") || path.starts_with("https://") {
        ureq::get(path).call()?.body_mut().read_to_string()?
    } else {
        std::fs::read_to_string(path)?
    };

    let services: Vec<Servico> = serde_json::from_str(&content)?;
    Ok(services)
}

// Saves the `Vec<Service>` vector to a pretty-printed JSON file.
pub fn save_servicos_to_json(
    services: &Vec<Servico>,
    filename: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Saving {} services to {}", services.len(), filename);

    let json_content = to_string_pretty(services)?;

    let mut file = File::create(filename)?;
    file.write_all(json_content.as_bytes())?;

    println!("Successfully saved services to {}", filename);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovery_path_is_namespaced_under_scrape() {
        // Must NOT be `<data_dir>/<slug>.json` (that path is where `process` writes the per-público
        // files — the collision this subdir avoids).
        let p = scrape_recovery_path("../data/rs/raw", "servicos-ao-cidadao");
        assert_eq!(p, "../data/rs/raw/scrape/servicos-ao-cidadao.json");
    }

    #[test]
    fn recovery_file_round_trips_through_scrape_dir() {
        let base = std::env::temp_dir().join(format!("rs-recov-{}/raw", std::process::id()));
        let dir = base.to_str().unwrap();
        std::fs::create_dir_all(format!("{}/scrape", dir)).unwrap();

        let svc = Servico {
            id: 1,
            tipo: "Cidadãos".into(),
            classe: "IPVA".into(),
            orgao: "SEFAZ-RS".into(),
            link: "https://x".into(),
            titulo: "Emitir guia".into(),
            descricao: "corpo".into(),
        };
        let path = scrape_recovery_path(dir, "servicos-ao-cidadao");
        save_servicos_to_json(&vec![svc], &path).unwrap();

        // The reader (`load_per_tipo`) reads exactly this path.
        let back = load_servicos_from_json(&path).unwrap();
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].link, "https://x");

        let _ = std::fs::remove_dir_all(base.parent().unwrap());
    }
}
