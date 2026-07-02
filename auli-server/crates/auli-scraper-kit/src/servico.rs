use serde::{Deserialize, Serialize};

/// Um serviço raspado de um público (a entrada de [`crate::aggregate_servicos`]) — e também o shape
/// dos JSONs per-público que o `process` grava. `descricao` carrega o header `tipo/classe/titulo`
/// que [`crate::descricao_body`] remove ao materializar o corpo limpo do snapshot.
#[derive(Serialize, Deserialize, Debug)]
pub struct Servico {
    /// Id sequencial por arquivo (começa em 1). Não é globalmente único — use `link` para isso.
    pub id: usize,
    /// Público/categoria (ex.: `"Cidadãos"`, `"Empresas"`).
    pub tipo: String,
    /// Classe/grupo do serviço (do título do card).
    pub classe: String,
    /// Órgão de origem.
    pub orgao: String,
    /// URL do serviço.
    pub link: String,
    /// Título legível.
    pub titulo: String,
    /// Descrição do serviço (corpo da página de detalhe, com o header `tipo/classe/titulo`).
    pub descricao: String,
}
