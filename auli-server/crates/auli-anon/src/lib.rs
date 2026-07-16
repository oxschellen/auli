//! `auli-anon` — anonimização de PII das perguntas do Chat da Auli.
//!
//! Substitui identificadores estruturados (CPF, CNPJ, e-mail, …) por placeholders
//! numerados (`[CPF_1]`) **antes** de a pergunta (a) ser gravada no log de
//! pergunta/resposta e (b) sair do processo rumo ao LLM externo. Determinístico,
//! sem rede, in-process.
//!
//! ## Garantias e não-garantias
//! - O `mapping` devolvido em [`Anonimizado`] contém os **valores originais**: vive apenas
//!   em memória, no escopo da requisição, e **nunca** deve ser serializado em disco.
//! - Falha do sanitizer deve ser tratada pelo chamador como **fail-closed** — descartar o
//!   texto cru e persistir/enviar [`TEXTO_FALLBACK_ERRO`], nunca o original.
//! - O embedding da pergunta continua sendo calculado sobre o **texto original** (é local,
//!   in-process — não há vazamento) para não degradar a busca vetorial.
//! - Nome de pessoa, razão social e endereço livre **ainda não** são cobertos (Fase 4).
//!
//! ## Estado (Fase 0)
//! Apenas os reconhecedores nativos do cloakrs (locale BR): **CPF, CNPJ numérico e e-mail**.
//! Os demais (telefone, IE, protocolo, GA, RENAVAM, placa, CEP, data de nascimento e o CNPJ
//! alfanumérico de 2026) chegam na Fase 1 como reconhecedores customizados, plugados via
//! `ScannerBuilder::recognizer(...)` no construtor.

use cloakrs_core::{Locale, PromptMapping, PromptSanitizer};
use cloakrs_locales::default_registry;

mod reconhecedores;

/// Placeholder gravado no lugar da pergunta quando a anonimização falha. A biblioteca nunca
/// decide sozinha deixar o texto cru passar; o chamador aplica esta política *fail-closed*.
pub const TEXTO_FALLBACK_ERRO: &str = "[ERRO DE ANONIMIZAÇÃO — pergunta descartada do log]";

/// Resultado de [`Anonimizador::anonimizar`].
pub struct Anonimizado {
    /// Texto com a PII substituída por placeholders (`[CPF_1]`, `[EMAIL_1]`, …).
    pub texto: String,
    /// Mapeamento placeholder → valor original, para [`Anonimizador::restaurar`].
    /// **Nunca persistir**: contém os valores originais.
    pub mapping: PromptMapping,
}

/// Erros de anonimização.
#[derive(Debug, thiserror::Error)]
pub enum AnonError {
    /// Falha ao montar o scanner (registro de reconhecedores / locale).
    #[error("falha ao construir o anonimizador: {0}")]
    Construcao(String),
    /// Falha ao sanitizar um texto.
    #[error("falha ao anonimizar o texto: {0}")]
    Sanitizacao(String),
}

/// Anonimizador reutilizável. Construir **uma vez** na inicialização do server e compartilhar
/// (os reconhecedores compilam suas regexes no construtor).
pub struct Anonimizador {
    sanitizer: PromptSanitizer,
}

impl Anonimizador {
    /// Monta o scanner com o locale BR e todos os reconhecedores registrados.
    pub fn novo() -> Result<Self, AnonError> {
        let scanner = default_registry()
            .into_scanner_builder()
            .locale(Locale::BR)
            .recognizer(reconhecedores::CnpjAlfanumericoRecognizer::novo())
            .recognizer(reconhecedores::TelefoneBrRecognizer::novo())
            .recognizer(reconhecedores::InscricaoEstadualRecognizer::novo())
            .recognizer(reconhecedores::CepRecognizer::novo())
            .recognizer(reconhecedores::ProtocoloRecognizer::novo())
            .recognizer(reconhecedores::GaRecognizer::novo())
            .recognizer(reconhecedores::RenavamRecognizer::novo())
            .recognizer(reconhecedores::PlacaRecognizer::novo())
            // Fase 1 (a seguir): data.
            .build()
            .map_err(|e| AnonError::Construcao(e.to_string()))?;
        Ok(Self {
            sanitizer: PromptSanitizer::new(scanner),
        })
    }

    /// Anonimiza `texto`. Determinístico, sem rede, in-process.
    pub fn anonimizar(&self, texto: &str) -> Result<Anonimizado, AnonError> {
        let (texto, mapping) = self
            .sanitizer
            .sanitize(texto)
            .map_err(|e| AnonError::Sanitizacao(e.to_string()))?;
        Ok(Anonimizado { texto, mapping })
    }

    /// Restaura os placeholders de uma resposta do LLM usando o `mapping` da pergunta.
    pub fn restaurar(&self, texto: &str, mapping: &PromptMapping) -> String {
        mapping.restore(texto)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constroi_e_anonimiza_cpf() {
        let anon = Anonimizador::novo().expect("construir");
        let r = anon
            .anonimizar("O contribuinte de CPF 529.982.247-25 pediu certidão.")
            .expect("anonimizar");
        assert!(!r.texto.contains("529.982.247-25"), "CPF vazou: {}", r.texto);
        assert!(!r.mapping.entries.is_empty(), "nenhuma entidade detectada");
    }

    #[test]
    fn ciclo_restore_recupera_o_original() {
        let anon = Anonimizador::novo().expect("construir");
        let r = anon
            .anonimizar("A empresa de CNPJ 11.222.333/0001-81 pode compensar?")
            .expect("anonimizar");
        // O LLM ecoa o placeholder na resposta; o restore devolve o valor original.
        let restaurado = anon.restaurar(&r.texto, &r.mapping);
        assert!(restaurado.contains("11.222.333/0001-81"), "restore falhou: {restaurado}");
    }

    #[test]
    fn controle_sem_pii_nao_detecta_nada() {
        let anon = Anonimizador::novo().expect("construir");
        let r = anon
            .anonimizar("Qual o período de inadimplência para cancelar um parcelamento?")
            .expect("anonimizar");
        assert!(r.mapping.entries.is_empty(), "falso positivo: {:?}", r.texto);
    }
}
