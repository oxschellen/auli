use serde::{Deserialize, Serialize};

// O registro de serviço raspado (e o shape dos JSONs per-público) mora no kit, compartilhado com os
// scrapers e com o `process`.
pub use auli_scraper_kit::Servico;

#[derive(Serialize, Deserialize, Debug)]
pub struct TipoServicos {
    pub tipo: String,
    pub filename: String,
    pub url: String,
}
