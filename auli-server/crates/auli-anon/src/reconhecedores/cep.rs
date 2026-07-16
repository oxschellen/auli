//! Reconhecedor de CEP para o pipeline de anonimização da Auli.
//!
//! Duas vias, no molde do telefone:
//! 1. **Com hífen** (`90010-150`) — formato inequívoco, dispara sem contexto.
//! 2. **Corrido** (8 dígitos) — só com palavra de contexto ("CEP", "endereço",
//!    "código postal") nos ~40 caracteres anteriores.
//!
//! Armadilha de contexto: "cep" solto casa dentro de "recepção" — o contexto
//! usa fronteira de palavra (\b), como na IE.

use cloakrs_core::{Confidence, EntityType, Locale, PiiEntity, Recognizer, Span};
use regex::Regex;

const LOCALES_BR: &[Locale] = &[Locale::BR];

/// Reconhecedor de CEP. Compila as regexes uma vez em [`Self::novo`] e reutiliza.
pub struct CepRecognizer {
    com_hifen: Regex,
    corrido: Regex,
    contexto: Regex,
}

impl CepRecognizer {
    #[must_use]
    pub fn novo() -> Self {
        Self {
            com_hifen: Regex::new(r"\d{5}-\d{3}").expect("regex CEP com hífen inválida"),
            corrido: Regex::new(r"\d{8}").expect("regex CEP corrido inválida"),
            contexto: Regex::new(r"(?i)\bcep\b|endere[çc]o|c[óo]digo\s+postal")
                .expect("regex de contexto CEP inválida"),
        }
    }

    fn limites_ok(text: &str, start: usize, end: usize) -> bool {
        let antes = text[..start].chars().next_back();
        let depois = text[end..].chars().next();
        let livre = |c: Option<char>| c.is_none_or(|c| !c.is_ascii_alphanumeric());
        // Recusa separador colado a dígito nas bordas — evita fatiar CPF
        // (529.982.247-25 contém "982.247-25"? não, mas "24725"-like corridos sim),
        // CNPJ, GA e protocolos pelo meio.
        let sem_estrutura_colada = |lado: Option<char>, vizinho: Option<char>| {
            !(matches!(lado, Some('/') | Some('.') | Some('-'))
                && vizinho.is_some_and(|c| c.is_ascii_digit()))
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

impl Recognizer for CepRecognizer {
    fn id(&self) -> &str {
        "auli_cep_v1"
    }

    fn entity_type(&self) -> EntityType {
        EntityType::Custom("cep".into())
    }

    fn supported_locales(&self) -> &[Locale] {
        LOCALES_BR
    }

    fn scan(&self, text: &str) -> Vec<PiiEntity> {
        let mut achados: Vec<PiiEntity> = Vec::new();

        // 1) Com hífen: dispara livre; contexto só reforça a confiança.
        for m in self.com_hifen.find_iter(text) {
            if !Self::limites_ok(text, m.start(), m.end()) {
                continue;
            }
            let boost = if self.tem_contexto(text, m.start()) { 0.15 } else { 0.0 };
            achados.push(PiiEntity {
                entity_type: self.entity_type(),
                span: Span::new(m.start(), m.end()),
                text: m.as_str().to_string(),
                confidence: Confidence::new(0.75 + boost).expect("confiança válida"),
                recognizer_id: self.id().to_string(),
            });
        }

        // 2) Corrido: contexto obrigatório.
        for m in self.corrido.find_iter(text) {
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
                continue;
            }
            achados.push(PiiEntity {
                entity_type: self.entity_type(),
                span: Span::new(m.start(), m.end()),
                text: m.as_str().to_string(),
                confidence: Confidence::new(0.7).expect("confiança válida"),
                recognizer_id: self.id().to_string(),
            });
        }

        achados
    }

    fn validate(&self, candidate: &str) -> bool {
        let d: String = candidate.chars().filter(char::is_ascii_digit).collect();
        d.len() == 8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn achados(text: &str) -> Vec<String> {
        CepRecognizer::novo()
            .scan(text)
            .into_iter()
            .map(|e| e.text)
            .collect()
    }

    #[test]
    fn com_hifen_dispensa_contexto() {
        assert_eq!(achados("A sede mudou para 90010-150 recentemente."), ["90010-150"]);
    }

    #[test]
    fn corrido_exige_contexto() {
        // 8 dígitos corridos sem palavra de contexto → não dispara.
        assert!(achados("O lote 90010150 foi arrematado.").is_empty());
        // Com "endereço" por perto → dispara.
        assert_eq!(achados("O endereço tem CEP 90010150 atualizado."), ["90010150"]);
    }

    #[test]
    fn recepcao_nao_e_contexto() {
        // "recepção" contém "cep", mas \bcep\b não casa sem fronteira de palavra.
        assert!(achados("A recepção anotou o 90010150 do lote.").is_empty());
    }

    #[test]
    fn respeita_boundaries() {
        assert!(achados("cep X90010-150").is_empty()); // letra colada antes
        assert!(achados("cep 90010-1500").is_empty()); // dígito extra depois
    }
}
