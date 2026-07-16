//! Reconhecedores customizados da Auli, plugados no scanner do cloakrs via
//! `ScannerBuilder::recognizer(...)`. Cada um cobre uma entidade que os reconhecedores
//! nativos (locale BR) não pegam. Ver §3 do plano de implementação.

mod cep;
mod cnpj_alfanumerico;
mod ga;
mod ie;
mod protocolo;
mod renavam;
mod telefone;

pub use cep::CepRecognizer;
pub use cnpj_alfanumerico::CnpjAlfanumericoRecognizer;
pub use ga::GaRecognizer;
pub use ie::InscricaoEstadualRecognizer;
pub use protocolo::ProtocoloRecognizer;
pub use renavam::RenavamRecognizer;
pub use telefone::TelefoneBrRecognizer;
