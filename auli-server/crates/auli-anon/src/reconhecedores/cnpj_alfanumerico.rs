//! Reconhecedor de **CNPJ alfanumérico** — o novo formato da IN RFB nº 2.229/2024,
//! obrigatório a partir de jul/2026 (ex.: `12.ABC.345/01DE-35`).
//!
//! O reconhecedor nativo do cloakrs só cobre CNPJ **numérico**; este preenche a lacuna.
//! Para não competir com o nativo, dispara **apenas** quando o candidato contém ao menos
//! uma letra — CNPJ puramente numérico continua sendo do reconhecedor nativo.
//!
//! Validação: o dígito verificador é conferido por `cpf_cnpj::cnpj::validate`, que implementa
//! o mod-11 alfanumérico (valor ASCII − 48) e rejeita letra nas posições de DV. Assim, um
//! amontoado qualquer de 14 caracteres não vira falso positivo.

use cloakrs_core::{Confidence, EntityType, Locale, PiiEntity, Recognizer, Span};
use regex::Regex;
use std::sync::LazyLock;

const RECOGNIZER_ID: &str = "auli_cnpj_alfanumerico_v1";
const LOCALES: &[Locale] = &[Locale::BR];

/// 12 caracteres base `[0-9A-Z]` + 2 dígitos de DV, com máscara `. . / -` opcional.
/// `(?i)` para aceitar letras minúsculas (a `validate` normaliza para maiúsculas).
static PADRAO: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)[0-9A-Z]{2}\.?[0-9A-Z]{3}\.?[0-9A-Z]{3}/?[0-9A-Z]{4}-?[0-9]{2}")
        .expect("regex de CNPJ alfanumérico é válida")
});

/// Reconhecedor do CNPJ alfanumérico. Sem estado — construir com [`Self::novo`].
pub struct CnpjAlfanumericoRecognizer;

impl CnpjAlfanumericoRecognizer {
    #[must_use]
    pub fn novo() -> Self {
        Self
    }
}

impl Recognizer for CnpjAlfanumericoRecognizer {
    fn id(&self) -> &str {
        RECOGNIZER_ID
    }

    /// Reutiliza `EntityType::Cnpj` para compartilhar a numeração `[CNPJ_n]` com o nativo.
    fn entity_type(&self) -> EntityType {
        EntityType::Cnpj
    }

    fn supported_locales(&self) -> &[Locale] {
        LOCALES
    }

    fn scan(&self, text: &str) -> Vec<PiiEntity> {
        let bytes = text.as_bytes();
        let mut achados = Vec::new();

        for m in PADRAO.find_iter(text) {
            let (start, end) = (m.start(), m.end());
            let candidato = m.as_str();

            // Só a variante COM letras — o numérico puro é do reconhecedor nativo.
            if !candidato.chars().any(|c| c.is_ascii_alphabetic()) {
                continue;
            }

            // Boundaries: não casar no meio de um token maior (dígito/letra colado antes/depois).
            if start > 0 && bytes[start - 1].is_ascii_alphanumeric() {
                continue;
            }
            if end < bytes.len() && bytes[end].is_ascii_alphanumeric() {
                continue;
            }

            // DV mod-11 alfanumérico (rejeita DV inválido e letra nas posições de DV).
            if !cpf_cnpj::cnpj::validate(candidato) {
                continue;
            }

            achados.push(PiiEntity {
                entity_type: EntityType::Cnpj,
                span: Span::new(start, end),
                text: candidato.to_string(),
                confidence: Confidence::new(0.95).expect("0.95 é confiança válida"),
                recognizer_id: RECOGNIZER_ID.to_string(),
            });
        }

        achados
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn achados(text: &str) -> Vec<String> {
        CnpjAlfanumericoRecognizer::novo()
            .scan(text)
            .into_iter()
            .map(|e| e.text)
            .collect()
    }

    #[test]
    fn casa_mascarado_bare_e_minusculo() {
        assert_eq!(achados("O CNPJ 12.ABC.345/01DE-35 é novo."), ["12.ABC.345/01DE-35"]);
        assert_eq!(achados("Empresa 12ABC34501DE35 aderiu."), ["12ABC34501DE35"]);
        assert_eq!(achados("cnpj 12abc34501de35 ok"), ["12abc34501de35"]);
    }

    #[test]
    fn ignora_cnpj_numerico_puro() {
        // Sem letras → é do reconhecedor nativo, não deste.
        assert!(achados("CNPJ 11.222.333/0001-81 e 11222333000181").is_empty());
    }

    #[test]
    fn rejeita_dv_invalido_e_letra_no_dv() {
        assert!(achados("12ABC34501DE99").is_empty()); // DV errado
        assert!(achados("12.ABC.345/01DE-AB").is_empty()); // letra na posição de DV
    }

    #[test]
    fn respeita_boundaries() {
        // Colado a mais caracteres alfanuméricos → não casa (token maior).
        assert!(achados("X12ABC34501DE35").is_empty());
        assert!(achados("12ABC34501DE3567").is_empty());
    }

    #[test]
    fn confianca_e_tipo_corretos() {
        let e = &CnpjAlfanumericoRecognizer::novo().scan("CNPJ 12.ABC.345/01DE-35.")[0];
        assert_eq!(e.entity_type, EntityType::Cnpj);
        assert_eq!(e.confidence.value(), 0.95);
        assert_eq!(e.recognizer_id, "auli_cnpj_alfanumerico_v1");
    }
}
