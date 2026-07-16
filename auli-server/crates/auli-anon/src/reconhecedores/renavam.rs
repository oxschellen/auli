//! Reconhecedor de RENAVAM para o pipeline de anonimização da Auli.
//!
//! Via única (corrida): 9–11 dígitos com contexto obrigatório ("renavam").
//! Renavams antigos têm 9 dígitos; os atuais, 11 — os de 9 são completados
//! com zeros à esquerda para a validação.
//!
//! DV (mod-11) como reforço, não porteiro — mesmo padrão da IE:
//! DV válido → confiança 0.9; DV inválido com contexto explícito → 0.7 e
//! mascara mesmo assim (digitação errada não pode virar vazamento).
//!
//! Precedência: 11 dígitos que validam como CPF ficam com o CpfRecognizer —
//! o número é mascarado de qualquer forma, apenas com o rótulo [CPF_n].

use cloakrs_core::{Confidence, EntityType, Locale, PiiEntity, Recognizer, Span};
use regex::Regex;

const LOCALES_BR: &[Locale] = &[Locale::BR];

/// Reconhecedor de RENAVAM. Compila as regexes uma vez em [`Self::novo`] e reutiliza.
pub struct RenavamRecognizer {
    corrido: Regex,
    contexto: Regex,
}

impl RenavamRecognizer {
    #[must_use]
    pub fn novo() -> Self {
        Self {
            corrido: Regex::new(r"\d{9,11}").expect("regex RENAVAM inválida"),
            contexto: Regex::new(r"(?i)\brenavam\b").expect("regex de contexto RENAVAM inválida"),
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
}

impl Recognizer for RenavamRecognizer {
    fn id(&self) -> &str {
        "auli_renavam_v1"
    }

    fn entity_type(&self) -> EntityType {
        EntityType::Custom("renavam".into())
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
            let d: String = m.as_str().chars().filter(char::is_ascii_digit).collect();
            if d.len() == 11 && cpf_cnpj::cpf::validate(&d) {
                continue; // CPF com DV válido: o CpfRecognizer assume o span
            }
            let confianca = if dv_renavam(&d) { 0.9 } else { 0.7 };
            achados.push(PiiEntity {
                entity_type: self.entity_type(),
                span: Span::new(m.start(), m.end()),
                text: m.as_str().to_string(),
                confidence: Confidence::new(confianca).expect("confiança válida"),
                recognizer_id: self.id().to_string(),
            });
        }

        achados
    }

    fn validate(&self, candidate: &str) -> bool {
        let d: String = candidate.chars().filter(char::is_ascii_digit).collect();
        (9..=11).contains(&d.len())
    }
}

/// DV do RENAVAM: completa com zeros à esquerda até 11 dígitos; sobre os 10 primeiros,
/// pesos 3,2,9,8,7,6,5,4,3,2; DV = 11 − (soma % 11); resultado 10 ou 11 → 0.
fn dv_renavam(digitos: &str) -> bool {
    if !(9..=11).contains(&digitos.len()) {
        return false;
    }
    let cheio = format!("{digitos:0>11}");
    let d: Vec<u32> = cheio.chars().filter_map(|c| c.to_digit(10)).collect();
    const PESOS: [u32; 10] = [3, 2, 9, 8, 7, 6, 5, 4, 3, 2];
    let soma: u32 = d[..10].iter().zip(PESOS.iter()).map(|(x, p)| x * p).sum();
    let dv = match 11 - (soma % 11) {
        10 | 11 => 0,
        v => v,
    };
    d[10] == dv
}

#[cfg(test)]
mod tests {
    use super::*;

    fn achados(text: &str) -> Vec<String> {
        RenavamRecognizer::novo()
            .scan(text)
            .into_iter()
            .map(|e| e.text)
            .collect()
    }

    #[test]
    fn dv_renavam_valido_e_invalido() {
        assert!(dv_renavam("12345678900")); // soma 231, resto 0, DV 11→0
        assert!(!dv_renavam("12345678901")); // DV errado
    }

    #[test]
    fn mascara_com_contexto_mesmo_com_dv_invalido() {
        // 12345678901 tem DV inválido, mas o contexto "RENAVAM" garante o mascaramento.
        assert_eq!(achados("O veículo RENAVAM 12345678901 tem IPVA em aberto."), ["12345678901"]);
    }

    #[test]
    fn exige_contexto() {
        assert!(achados("O código 12345678900 consta no lote.").is_empty());
    }

    #[test]
    fn cede_a_cpf_valido() {
        // 52998224725 é CPF válido → o span é do CpfRecognizer.
        assert!(achados("renavam informado é 52998224725 no sistema.").is_empty());
    }

    #[test]
    fn respeita_boundaries() {
        // Dígito extra colado → 12 dígitos, boundary recusa a borda.
        assert!(achados("renavam 123456789012").is_empty());
    }
}
