//! Reconhecedor de protocolo eletrônico para o pipeline de anonimização da Auli.
//!
//! Duas vias, no molde do telefone:
//! 1. **Formatado** (`AAAA/NNNNNN...`, ex.: `2026/000123456`) — ano seguido de barra e
//!    sequência de 6–12 dígitos. Dispara sem contexto: referências de legislação têm a
//!    forma invertida (`número/ano`, ex.: `IN 2.229/2024`, `Convênio 190/2017`) e não
//!    casam, pois exigimos 6+ dígitos após a barra.
//! 2. **Corrido** (9–15 dígitos) — só com contexto ("protocolo", "processo", "proc.")
//!    e com precedência para CPF/CNPJ: se os dígitos validam pelo DV, o span é deles.

use cloakrs_core::{Confidence, EntityType, Locale, PiiEntity, Recognizer, Span};
use regex::Regex;

const LOCALES_BR: &[Locale] = &[Locale::BR];

/// Reconhecedor de protocolo. Compila as regexes uma vez em [`Self::novo`] e reutiliza.
pub struct ProtocoloRecognizer {
    formatado: Regex,
    corrido: Regex,
    contexto: Regex,
}

impl ProtocoloRecognizer {
    #[must_use]
    pub fn novo() -> Self {
        Self {
            formatado: Regex::new(r"\d{4}/\d{6,12}").expect("regex protocolo formatado inválida"),
            corrido: Regex::new(r"\d{9,15}").expect("regex protocolo corrido inválida"),
            // "proc" cobre processo/proc./procedimento; \b evita casar dentro de
            // "reciprocidade" (sem \b, "proc" apareceria lá dentro).
            contexto: Regex::new(r"(?i)\bprotocolo\b|\bproc\w*\b")
                .expect("regex de contexto protocolo inválida"),
        }
    }

    fn limites_ok(text: &str, start: usize, end: usize) -> bool {
        let antes = text[..start].chars().next_back();
        let depois = text[end..].chars().next();
        let livre = |c: Option<char>| c.is_none_or(|c| !c.is_ascii_alphanumeric());
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

    fn digitos(candidate: &str) -> String {
        candidate.chars().filter(char::is_ascii_digit).collect()
    }
}

impl Recognizer for ProtocoloRecognizer {
    fn id(&self) -> &str {
        "auli_protocolo_v1"
    }

    fn entity_type(&self) -> EntityType {
        EntityType::Custom("protocolo".into())
    }

    fn supported_locales(&self) -> &[Locale] {
        LOCALES_BR
    }

    fn scan(&self, text: &str) -> Vec<PiiEntity> {
        let mut achados: Vec<PiiEntity> = Vec::new();

        // 1) Formatado AAAA/NNNNNN...: dispara livre, contexto só reforça.
        for m in self.formatado.find_iter(text) {
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

        // 2) Corrido: contexto obrigatório + precedência de CPF/CNPJ por DV.
        for m in self.corrido.find_iter(text) {
            if !Self::limites_ok(text, m.start(), m.end()) {
                continue;
            }
            if !self.tem_contexto(text, m.start()) {
                continue;
            }
            let d = Self::digitos(m.as_str());
            if d.len() == 11 && cpf_cnpj::cpf::validate(&d) {
                continue; // CPF com DV válido: o CpfRecognizer assume
            }
            if d.len() == 14 && cpf_cnpj::cnpj::validate(&d) {
                continue; // CNPJ com DV válido: o CnpjRecognizer assume
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
        let d = Self::digitos(candidate);
        (9..=16).contains(&d.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn achados(text: &str) -> Vec<String> {
        ProtocoloRecognizer::novo()
            .scan(text)
            .into_iter()
            .map(|e| e.text)
            .collect()
    }

    #[test]
    fn formatado_dispensa_contexto() {
        assert_eq!(achados("Veja o 2026/000123456 no sistema."), ["2026/000123456"]);
    }

    #[test]
    fn corrido_exige_contexto() {
        assert!(achados("O número 123456789012 consta no lote.").is_empty());
        assert_eq!(achados("O protocolo 123456789012 foi aberto."), ["123456789012"]);
    }

    #[test]
    fn corrido_cede_a_cpf_e_cnpj_validos() {
        // 52998224725 é CPF válido; 11222333000181 é CNPJ válido → o span é deles.
        assert!(achados("processo do CPF 52998224725 aberto.").is_empty());
        assert!(achados("processo do CNPJ 11222333000181 aberto.").is_empty());
    }

    #[test]
    fn referencia_de_legislacao_nao_casa() {
        // número/ano invertido, com menos de 6 dígitos após a barra.
        assert!(achados("Conforme a IN 2.229/2024 e o Convênio 190/2017.").is_empty());
    }

    #[test]
    fn respeita_boundaries() {
        assert!(achados("X2026/000123456").is_empty()); // letra colada antes
    }
}
