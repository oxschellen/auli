//! Reconhecedores customizados da Auli, plugados no scanner do cloakrs via
//! `ScannerBuilder::recognizer(...)`. Cada um cobre uma entidade que os reconhecedores
//! nativos (locale BR) não pegam. Ver §3 do plano de implementação.

mod cnpj_alfanumerico;

pub use cnpj_alfanumerico::CnpjAlfanumericoRecognizer;
