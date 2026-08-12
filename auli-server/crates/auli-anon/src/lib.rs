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
//! - Nome de pessoa, razão social e endereço são cobertos por **heurística** (Fase 4), com recall
//!   parcial assumido: sem o gatilho/porteiro esperado, a entidade passa. Ver
//!   `docs/auli-anon_pendencias.md`.
//!
//! ## Estado (Fases 0–4)
//! Reconhecedores nativos do cloakrs (locale BR): **CPF, CNPJ numérico e e-mail**. Os demais são
//! nossos, plugados via `ScannerBuilder::recognizer(...)` no construtor:
//! - **Fase 1, identificador estruturado** (validação por DV ou formato inequívoco): CNPJ
//!   alfanumérico de 2026, telefone, IE, protocolo, GA, RENAVAM, placa, CEP, data de nascimento.
//! - **Fase 4, entidade sem forma canônica** (heurística, confiança 0.7): razão social, endereço
//!   e nome de pessoa.

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
            .recognizer(reconhecedores::DataNascimentoRecognizer::novo())
            .recognizer(reconhecedores::RazaoSocialRecognizer::novo())
            .recognizer(reconhecedores::EnderecoRecognizer::novo())
            .recognizer(reconhecedores::NomePessoaRecognizer::novo())
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
        assert!(
            !r.texto.contains("529.982.247-25"),
            "CPF vazou: {}",
            r.texto
        );
        assert!(!r.mapping.entries.is_empty(), "nenhuma entidade detectada");
    }

    /// O ciclo é **identidade**, não "o valor reaparece em algum lugar".
    ///
    /// A asserção era `contains` até 12/08/2026, e por isso não podia pegar a classe de defeito que
    /// o `tools/anon-tarf` encontrou em 34 documentos reais: quando o `span` de uma detecção cobre
    /// um caractere que o valor guardado não tem (o `credit_card_luhn_v1` do `cloakrs` grava o span
    /// com o separador final e o texto com `trim()`), a restauração devolve o valor **e come o
    /// separador**. Com `contains`, o texto ainda "contém" o valor e o teste passa; com igualdade,
    /// não passa. Um teste mais fraco que a propriedade que ele guarda é um teste que mente.
    #[test]
    fn ciclo_restore_recupera_o_original() {
        let anon = Anonimizador::novo().expect("construir");
        let original = "A empresa de CNPJ 11.222.333/0001-81 pode compensar?";
        let r = anon.anonimizar(original).expect("anonimizar");
        assert_ne!(
            r.texto, original,
            "nada foi mascarado — a fixture perdeu o sentido"
        );
        // O LLM ecoa o placeholder na resposta; o restore devolve o texto EXATO de entrada.
        assert_eq!(
            anon.restaurar(&r.texto, &r.mapping),
            original,
            "o ciclo anonimizar → restaurar não é identidade"
        );
    }

    /// **Reprodução mínima de um defeito de terceiro, VERMELHA de propósito.**
    ///
    /// `cloakrs-patterns 0.3.0`, `src/credit_card.rs:41-45`: o `span` da detecção cobre o separador
    /// final do casamento (o regex `\b(?:\d[ -.]?){13,19}\b` permite terminar em espaço, ponto ou
    /// hífen) e o `text` guardado é o `trim()` dele. A restauração devolve o valor sem o separador
    /// num vão que o tinha, e **come um caractere**, em silêncio:
    ///
    /// ```text
    /// orig : … 300 ML 27101932 6910 12 UN
    /// volta: … 300 ML 27101932 6910 1️2UN
    /// ```
    ///
    /// Encontrado pelo `tools/anon-tarf` em **34 documentos reais** — 18 acórdãos do TARF e 16
    /// pareceres —, sempre onde há corrida de 13 a 19 dígitos: quantidades, números de processo e
    /// tabelas CEST/NCM, todas falsos positivos do Luhn.
    ///
    /// Fica `#[ignore]` porque a correção é upstream e não está no nosso cronograma. **Quando o
    /// `cloakrs` corrigir, tirar o `ignore`**: ela vira a regressão que impede a volta.
    #[test]
    #[ignore = "defeito upstream em cloakrs-patterns 0.3.0 — vermelha até a correção"]
    fn round_trip_quebra_quando_o_span_passa_do_valor() {
        let anon = Anonimizador::novo().expect("construir");
        let original = "Produto: ROST OFF WURTH 300 ML 27101932 6910 12 UN";
        let r = anon.anonimizar(original).expect("anonimizar");
        assert_eq!(anon.restaurar(&r.texto, &r.mapping), original);
    }

    #[test]
    fn controle_sem_pii_nao_detecta_nada() {
        let anon = Anonimizador::novo().expect("construir");
        let r = anon
            .anonimizar("Qual o período de inadimplência para cancelar um parcelamento?")
            .expect("anonimizar");
        assert!(
            r.mapping.entries.is_empty(),
            "falso positivo: {:?}",
            r.texto
        );
    }
}
