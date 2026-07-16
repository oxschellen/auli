//! Reconhecedor de número de Guia de Arrecadação (GA/GNRE) para a anonimização da Auli.
//!
//! Só existe a via corrida: 10–17 dígitos com contexto obrigatório
//! ("GA", "guia de arrecadação", "GNRE", "guia").
//!
//! **Armadilha central do domínio:** "GA 1118" é CÓDIGO DE RECEITA (3–4 dígitos,
//! ex.: 1118 emolumentos, 221 ICMS), não identificador de guia — aparece o tempo
//! todo em pergunta tributária e NÃO pode ser mascarado. O mínimo de 10 dígitos
//! no padrão é o que protege esse caso.
//!
//! Precedência por DV: 11 dígitos que validam como CPF ou 14 que validam como
//! CNPJ ficam com os reconhecedores nativos.

use cloakrs_core::{Confidence, EntityType, Locale, PiiEntity, Recognizer, Span};
use regex::Regex;

const LOCALES_BR: &[Locale] = &[Locale::BR];

/// Reconhecedor de GA/GNRE. Compila as regexes uma vez em [`Self::novo`] e reutiliza.
pub struct GaRecognizer {
    corrido: Regex,
    contexto: Regex,
}

impl GaRecognizer {
    #[must_use]
    pub fn novo() -> Self {
        Self {
            corrido: Regex::new(r"\d{10,17}").expect("regex GA inválida"),
            // "GA" é curtíssimo: \b obrigatório (sem ele casaria dentro de "chegada",
            // "obrigatória", "pagamento"...). GNRE e "guia" cobrem as variações.
            contexto: Regex::new(r"(?i)\bga\b|\bgnre\b|\bguia\b|guia\s+de\s+arrecada[çc][ãa]o")
                .expect("regex de contexto GA inválida"),
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

impl Recognizer for GaRecognizer {
    fn id(&self) -> &str {
        "auli_ga_v1"
    }

    fn entity_type(&self) -> EntityType {
        EntityType::Custom("ga".into())
    }

    fn supported_locales(&self) -> &[Locale] {
        LOCALES_BR
    }

    fn scan(&self, text: &str) -> Vec<PiiEntity> {
        let mut achados: Vec<PiiEntity> = Vec::new();

        for m in self.corrido.find_iter(text) {
            if !Self::limites_ok(text, m.start(), m.end()) {
                continue;
            }
            if !self.tem_contexto(text, m.start()) {
                continue;
            }
            let d = Self::digitos(m.as_str());
            if d.len() == 11 && cpf_cnpj::cpf::validate(&d) {
                continue;
            }
            if d.len() == 14 && cpf_cnpj::cnpj::validate(&d) {
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
        (10..=17).contains(&d.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn achados(text: &str) -> Vec<String> {
        GaRecognizer::novo()
            .scan(text)
            .into_iter()
            .map(|e| e.text)
            .collect()
    }

    #[test]
    fn mascara_com_contexto() {
        assert_eq!(
            achados("A GA de número 0312026000987654 foi paga."),
            ["0312026000987654"]
        );
    }

    #[test]
    fn codigo_de_receita_nao_mascara() {
        // "GA 1118" (código de receita, 4 dígitos) NÃO pode ser mascarado.
        assert!(achados("Pagou em duplicidade a GA 1118 este mês.").is_empty());
    }

    #[test]
    fn exige_contexto() {
        assert!(achados("O número 0312026000987654 apareceu no lote.").is_empty());
    }

    #[test]
    fn cede_a_cpf_e_cnpj_validos() {
        assert!(achados("guia do CPF 52998224725 emitida.").is_empty());
        assert!(achados("GA do CNPJ 11222333000181 emitida.").is_empty());
    }

    #[test]
    fn ga_dentro_de_pagamento_nao_e_contexto() {
        // "pagamento" contém "ga", mas \bga\b não casa sem fronteira de palavra.
        assert!(achados("O pagamento 0312026000987654 consta pendente.").is_empty());
    }
}
