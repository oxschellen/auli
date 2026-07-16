//! Reconhecedor de placa veicular para o pipeline de anonimização da Auli.
//!
//! Duas vias:
//! 1. **Mercosul** (`LLLNLNN`, ex.: `IVW4D21`) — o intercalado letra-dígito-letra é
//!    inequívoco (nenhuma sigla ou código tributário tem essa forma): dispara livre.
//! 2. **Formato antigo** (`LLL-NNNN` / `LLLNNNN`, ex.: `ABC-1234`) — colide com
//!    sigla + código (ex.: `CST0102`), então exige contexto ("placa", "veículo",
//!    "carro", "caminhão", "moto").
//!
//! Case-insensitive nas duas vias: atendente escreve "ivw4d21" no chat.

use cloakrs_core::{Confidence, EntityType, Locale, PiiEntity, Recognizer, Span};
use regex::Regex;

const LOCALES_BR: &[Locale] = &[Locale::BR];

/// Reconhecedor de placa. Compila as regexes uma vez em [`Self::novo`] e reutiliza.
pub struct PlacaRecognizer {
    mercosul: Regex,
    antiga: Regex,
    contexto: Regex,
}

impl PlacaRecognizer {
    #[must_use]
    pub fn novo() -> Self {
        Self {
            mercosul: Regex::new(r"(?i)[a-z]{3}\d[a-z]\d{2}").expect("regex placa Mercosul inválida"),
            antiga: Regex::new(r"(?i)[a-z]{3}-?\d{4}").expect("regex placa antiga inválida"),
            contexto: Regex::new(r"(?i)\bplacas?\b|ve[íi]culo|\bcarro\b|caminh[ãa]o|\bmoto\b")
                .expect("regex de contexto placa inválida"),
        }
    }

    fn limites_ok(text: &str, start: usize, end: usize) -> bool {
        let antes = text[..start].chars().next_back();
        let depois = text[end..].chars().next();
        let livre = |c: Option<char>| c.is_none_or(|c| !c.is_ascii_alphanumeric());
        let sem_estrutura_colada = |lado: Option<char>, vizinho: Option<char>| {
            !(matches!(lado, Some('/') | Some('.') | Some('-'))
                && vizinho.is_some_and(|c| c.is_ascii_alphanumeric()))
        };
        let antes2 = start
            .checked_sub(2)
            .and_then(|i| text.get(i..).and_then(|s| s.chars().next()));
        let depois2 = text.get(end + 1..).and_then(|s| s.chars().next());
        livre(antes)
            && livre(depois)
            && sem_estrutura_colada(antes, antes2)
            && sem_estrutura_colada(depois, depois2)
    }

    fn tem_contexto(&self, text: &str, start: usize) -> bool {
        let mut ini = start.saturating_sub(40);
        while ini > 0 && !text.is_char_boundary(ini) {
            ini -= 1;
        }
        self.contexto.is_match(&text[ini..start])
    }
}

impl Recognizer for PlacaRecognizer {
    fn id(&self) -> &str {
        "auli_placa_v1"
    }

    fn entity_type(&self) -> EntityType {
        EntityType::Custom("placa".into())
    }

    fn supported_locales(&self) -> &[Locale] {
        LOCALES_BR
    }

    fn scan(&self, text: &str) -> Vec<PiiEntity> {
        let mut achados: Vec<PiiEntity> = Vec::new();

        // 1) Mercosul: formato inequívoco, dispara livre.
        for m in self.mercosul.find_iter(text) {
            if !Self::limites_ok(text, m.start(), m.end()) {
                continue;
            }
            let boost = if self.tem_contexto(text, m.start()) { 0.15 } else { 0.0 };
            achados.push(PiiEntity {
                entity_type: self.entity_type(),
                span: Span::new(m.start(), m.end()),
                text: m.as_str().to_string(),
                confidence: Confidence::new(0.8 + boost).expect("confiança válida"),
                recognizer_id: self.id().to_string(),
            });
        }

        // 2) Formato antigo: contexto obrigatório.
        for m in self.antiga.find_iter(text) {
            if !Self::limites_ok(text, m.start(), m.end()) {
                continue;
            }
            if !self.tem_contexto(text, m.start()) {
                continue;
            }
            if achados
                .iter()
                .any(|a| a.span.start < m.end() && m.start() < a.span.end)
            {
                continue; // já coberto pela via Mercosul
            }
            achados.push(PiiEntity {
                entity_type: self.entity_type(),
                span: Span::new(m.start(), m.end()),
                text: m.as_str().to_string(),
                confidence: Confidence::new(0.75).expect("confiança válida"),
                recognizer_id: self.id().to_string(),
            });
        }

        achados
    }

    fn validate(&self, candidate: &str) -> bool {
        self.mercosul.is_match(candidate) || self.antiga.is_match(candidate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn achados(text: &str) -> Vec<String> {
        PlacaRecognizer::novo()
            .scan(text)
            .into_iter()
            .map(|e| e.text)
            .collect()
    }

    #[test]
    fn mercosul_dispensa_contexto_e_e_case_insensitive() {
        assert_eq!(achados("Vi o IVW4D21 estacionado na rua."), ["IVW4D21"]);
        assert_eq!(achados("registrei ivw4d21 no sistema."), ["ivw4d21"]);
    }

    #[test]
    fn antiga_exige_contexto() {
        // Sem contexto, ABC1234 poderia ser sigla+código → não dispara.
        assert!(achados("O código ABC1234 consta na tabela.").is_empty());
        // Com contexto de veículo, dispara.
        assert_eq!(achados("A placa ABC-1234 do carro venceu."), ["ABC-1234"]);
    }

    #[test]
    fn sigla_mais_codigo_sem_contexto_nao_casa() {
        // Formato antigo colide com CST0102 etc.; sem contexto de veículo, fica de fora.
        assert!(achados("A situação CST0102 aplica-se aqui.").is_empty());
    }

    #[test]
    fn respeita_boundaries() {
        assert!(achados("XIVW4D21").is_empty()); // letra colada antes
    }
}
