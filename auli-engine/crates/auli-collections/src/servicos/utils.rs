use serde_json::to_string_pretty;
use std::fs::File;
use std::io::Write;

use super::types::{Servico, TipoServicos};

pub fn get_tipo_servicos() -> Vec<TipoServicos> {
    vec![
        TipoServicos {
            tipo: "Cidadãos".to_string(),
            filename: "rs-servicos-ao-cidadao".to_string(),
            url: "https://www.fazenda.rs.gov.br/servicos-ao-cidadao".to_string(),
        },
        TipoServicos {
            tipo: "Empresas".to_string(),
            filename: "rs-servicos-a-empresas".to_string(),
            url: "https://www.fazenda.rs.gov.br/servicos-a-empresas".to_string(),
        },
        TipoServicos {
            tipo: "Fornecedores".to_string(),
            filename: "rs-servicos-a-fornecedores".to_string(),
            url: "https://www.fazenda.rs.gov.br/servicos-a-fornecedores".to_string(),
        },
        TipoServicos {
            tipo: "Agentes".to_string(),
            filename: "rs-servicos-a-agentes-publicos".to_string(),
            url: "https://www.fazenda.rs.gov.br/servicos-a-agentes-publicos".to_string(),
        },
        TipoServicos {
            tipo: "Servidores".to_string(),
            filename: "rs-servicos-a-servidores-publicos".to_string(),
            url: "https://www.fazenda.rs.gov.br/servicos-a-servidores-publicos".to_string(),
        },
    ]
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
