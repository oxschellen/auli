use serde::{Deserialize, Serialize};

// O registro de serviço raspado (e o shape dos JSONs per-público) é contrato — mora no
// `auli-contract` como `ServicoPerPublico` (D-C1), compartilhado com os scrapers e com o `process`.
pub use auli_contract::ServicoPerPublico as Servico;

#[derive(Serialize, Deserialize, Debug)]
pub struct TipoServicos {
    pub tipo: String,
    pub filename: String,
    pub url: String,
}
