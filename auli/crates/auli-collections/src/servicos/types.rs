use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct TipoServicos {
    pub tipo: String,
    pub filename: String,
    pub url: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Servico {
    /// Sequential ID for reference (starts from 1)
    pub id: usize,
    /// Category type (e.g. "Cidadãos", "Empresas")
    pub tipo: String,
    /// Service class/group from the card title
    pub classe: String,
    /// Originating organ label from the card
    pub orgao: String,
    /// URL link for the service
    pub link: String,
    /// Human-readable title
    pub titulo: String,
    /// Service description from the detail page (a tipo/classe/titulo header + the description body)
    pub descricao: String,
}
