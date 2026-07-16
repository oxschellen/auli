//! Reconhecedor de telefones brasileiros para o pipeline de anonimização da Auli.
//!
//! Duas estratégias:
//! 1. **Formatado** — número com separadores/DDD entre parênteses/+55: dispara sem contexto,
//!    pois o formato é inequívoco. Ex.: `(51) 99876-5432`, `+55 51 3214-5678`.
//! 2. **Corrido** — 10–11 dígitos sem separadores: só dispara com palavra de contexto
//!    (telefone, celular, fone, whatsapp, contato...) nos ~40 caracteres anteriores,
//!    e somente se NÃO for um CPF com dígito verificador válido (precedência do CPF).

use cloakrs_core::{Confidence, EntityType, Locale, PiiEntity, Recognizer, Span};
use regex::Regex;

const LOCALES_BR: &[Locale] = &[Locale::BR];

const PALAVRAS_CONTEXTO: &[&str] = &[
    "telefone", "celular", "fone", "whatsapp", "zap", "contato", "ligar", "ligue", "tel",
];

/// Reconhecedor de telefone BR. Compila as regexes uma vez em [`Self::novo`] e reutiliza.
pub struct TelefoneBrRecognizer {
    formatado: Regex,
    corrido: Regex,
}

impl TelefoneBrRecognizer {
    #[must_use]
    pub fn novo() -> Self {
        Self {
            // +55 opcional · DDD com ou sem parênteses · 9 opcional · bloco de 4 · separador · bloco de 4
            // Exige ao menos um separador entre os blocos finais para contar como "formatado".
            formatado: Regex::new(r"(?:\+55[\s.\-]?)?\(?\d{2}\)?[\s.\-]?9?\d{4}[\s.\-]\d{4}")
                .expect("regex de telefone formatado inválida"),
            // 10 ou 11 dígitos corridos (11 exige o 9 de celular na 3ª posição).
            corrido: Regex::new(r"(?:\d{2}9\d{8}|\d{10})")
                .expect("regex de telefone corrido inválida"),
        }
    }

    fn limites_ok(text: &str, start: usize, end: usize) -> bool {
        let antes = text[..start].chars().next_back();
        let depois = text[end..].chars().next();
        let livre = |c: Option<char>| c.is_none_or(|c| !c.is_ascii_alphanumeric());
        // Evita casar dentro de sequências maiores (GA de 16 dígitos, CNPJ corrido etc.)
        // e dentro de estruturas com separador colado a dígito (ex.: 224/3210012).
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

    fn tem_contexto(text: &str, start: usize) -> bool {
        // Recuar até uma fronteira de char válida (texto pt-BR tem acentos multibyte).
        let mut ini = start.saturating_sub(40);
        while ini > 0 && !text.is_char_boundary(ini) {
            ini -= 1;
        }
        let janela = text[ini..start].to_lowercase();
        PALAVRAS_CONTEXTO.iter().any(|p| janela.contains(p))
    }

    fn digitos(candidate: &str) -> String {
        candidate.chars().filter(char::is_ascii_digit).collect()
    }
}

impl Recognizer for TelefoneBrRecognizer {
    fn id(&self) -> &str {
        "auli_telefone_br_v1"
    }

    fn entity_type(&self) -> EntityType {
        EntityType::Custom("telefone".into())
    }

    fn supported_locales(&self) -> &[Locale] {
        LOCALES_BR
    }

    fn scan(&self, text: &str) -> Vec<PiiEntity> {
        let mut achados = Vec::new();

        // 1) Formatado: sem exigência de contexto.
        for m in self.formatado.find_iter(text) {
            if !Self::limites_ok(text, m.start(), m.end()) {
                continue;
            }
            if !self.validate(m.as_str()) {
                continue;
            }
            let boost = if Self::tem_contexto(text, m.start()) { 0.2 } else { 0.0 };
            achados.push(PiiEntity {
                entity_type: self.entity_type(),
                span: Span::new(m.start(), m.end()),
                text: m.as_str().to_string(),
                confidence: Confidence::new(0.7 + boost).expect("confiança válida"),
                recognizer_id: self.id().to_string(),
            });
        }

        // 2) Corrido: contexto obrigatório + não pode ser CPF válido (precedência do CPF).
        for m in self.corrido.find_iter(text) {
            if !Self::limites_ok(text, m.start(), m.end()) {
                continue;
            }
            if !Self::tem_contexto(text, m.start()) {
                continue;
            }
            let d = Self::digitos(m.as_str());
            if d.len() == 11 && cpf_cnpj::cpf::validate(&d) {
                continue; // CPF com DV válido: deixa para o CpfRecognizer
            }
            // Já coberto pelo formatado? (spans sobrepostos)
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
                confidence: Confidence::new(0.75).expect("confiança válida"),
                recognizer_id: self.id().to_string(),
            });
        }

        achados
    }

    fn validate(&self, candidate: &str) -> bool {
        let d = Self::digitos(candidate);
        let n = if d.starts_with("55") && d.len() > 11 { &d[2..] } else { &d[..] };
        match n.len() {
            10 => true,                    // fixo (ou celular antigo)
            11 => n.as_bytes()[2] == b'9', // celular novo exige o 9
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn achados(text: &str) -> Vec<String> {
        TelefoneBrRecognizer::novo()
            .scan(text)
            .into_iter()
            .map(|e| e.text)
            .collect()
    }

    #[test]
    fn formatado_dispensa_contexto() {
        // Celular e fixo formatados, sem palavra de contexto por perto.
        assert_eq!(achados("Atende no (51) 3214-5678 comercial."), ["(51) 3214-5678"]);
        assert_eq!(achados("Ocorreu no (51) 99876-5432 hoje."), ["(51) 99876-5432"]);
        assert_eq!(achados("Ligações de +55 51 99876-5432 entram."), ["+55 51 99876-5432"]);
    }

    #[test]
    fn corrido_exige_contexto() {
        // Sem palavra de contexto: 10–11 dígitos corridos NÃO disparam.
        assert!(achados("O código 5133214567 refere-se ao setor.").is_empty());
        // Com contexto: dispara.
        assert_eq!(achados("Contato pelo whatsapp 51998765432 por favor.").len(), 1);
    }

    #[test]
    fn corrido_que_e_cpf_valido_cede_ao_cpf() {
        // 52998224725 é CPF de DV válido: mesmo com contexto, não vira telefone.
        assert!(achados("Telefone 52998224725 do contribuinte.").is_empty());
    }

    #[test]
    fn celular_sem_o_nono_digito_e_invalido() {
        // 11 dígitos mas sem o 9 na 3ª posição → validate() rejeita o formatado.
        assert!(achados("Ligar para (51) 18876-5432 depois.").is_empty());
    }

    #[test]
    fn respeita_estrutura_colada_da_ie() {
        // 224/3210012 (IE) não pode ser confundida com telefone corrido.
        assert!(achados("A IE 224/3210012 consta baixada.").is_empty());
    }
}
