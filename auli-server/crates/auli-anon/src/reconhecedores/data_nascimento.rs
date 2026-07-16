//! Reconhecedor de data de nascimento para o pipeline de anonimização da Auli.
//!
//! Via única: `\d{2}/\d{2}/\d{4}` **somente** com contexto de nascimento ("nasc"
//! cobre nasceu/nascido/nascida/nascimento/"data de nascimento") nos ~40 caracteres
//! anteriores.
//!
//! **Cuidado central do domínio:** datas genéricas (vencimentos, fatos geradores,
//! prazos) são onipresentes em pergunta tributária e NÃO podem ser mascaradas — só a
//! data acompanhada de contexto explícito de nascimento é PII. Um leve teste de
//! plausibilidade (dia 1–31, mês 1–12) descarta sequências como `99/99/9999`.

use cloakrs_core::{Confidence, EntityType, Locale, PiiEntity, Recognizer, Span};
use regex::Regex;

const LOCALES_BR: &[Locale] = &[Locale::BR];

/// Reconhecedor de data de nascimento. Compila as regexes uma vez em [`Self::novo`].
pub struct DataNascimentoRecognizer {
    data: Regex,
    contexto: Regex,
}

impl DataNascimentoRecognizer {
    #[must_use]
    pub fn novo() -> Self {
        Self {
            data: Regex::new(r"\d{2}/\d{2}/\d{4}").expect("regex de data inválida"),
            contexto: Regex::new(r"(?i)nasc").expect("regex de contexto de nascimento inválida"),
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

/// Descarta datas impossíveis (dia 1–31, mês 1–12). O ano fica livre — o contexto de
/// nascimento é o porteiro; a plausibilidade só barra ruído como `99/99/9999`.
fn data_plausivel(candidate: &str) -> bool {
    let mut partes = candidate.split('/');
    let dia = partes.next().and_then(|s| s.parse::<u32>().ok());
    let mes = partes.next().and_then(|s| s.parse::<u32>().ok());
    matches!((dia, mes), (Some(d), Some(m)) if (1..=31).contains(&d) && (1..=12).contains(&m))
}

impl Recognizer for DataNascimentoRecognizer {
    fn id(&self) -> &str {
        "auli_data_nascimento_v1"
    }

    fn entity_type(&self) -> EntityType {
        EntityType::Custom("data_nasc".into())
    }

    fn supported_locales(&self) -> &[Locale] {
        LOCALES_BR
    }

    fn scan(&self, text: &str) -> Vec<PiiEntity> {
        let mut achados: Vec<PiiEntity> = Vec::new();

        for m in self.data.find_iter(text) {
            if !Self::limites_ok(text, m.start(), m.end()) {
                continue;
            }
            if !self.tem_contexto(text, m.start()) {
                continue; // sem contexto de nascimento, é data genérica → não mascara
            }
            if !data_plausivel(m.as_str()) {
                continue;
            }
            achados.push(PiiEntity {
                entity_type: self.entity_type(),
                span: Span::new(m.start(), m.end()),
                text: m.as_str().to_string(),
                confidence: Confidence::new(0.9).expect("confiança válida"),
                recognizer_id: self.id().to_string(),
            });
        }

        achados
    }

    fn validate(&self, candidate: &str) -> bool {
        self.data.is_match(candidate) && data_plausivel(candidate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn achados(text: &str) -> Vec<String> {
        DataNascimentoRecognizer::novo()
            .scan(text)
            .into_iter()
            .map(|e| e.text)
            .collect()
    }

    #[test]
    fn mascara_com_contexto_de_nascimento() {
        assert_eq!(
            achados("O dependente nasceu em 14/03/1998 e consta no ITCD."),
            ["14/03/1998"]
        );
        assert_eq!(achados("Data de nascimento: 01/12/2000, confirmar."), ["01/12/2000"]);
    }

    #[test]
    fn data_generica_sem_contexto_nao_mascara() {
        // Vencimento e fato gerador NÃO podem ser mascarados.
        assert!(achados("O vencimento da guia é 20/05/2026 para todos.").is_empty());
        assert!(achados("O fato gerador ocorreu em 10/01/2024, apure o imposto.").is_empty());
    }

    #[test]
    fn data_implausivel_nao_mascara() {
        assert!(achados("nasceu em 99/99/1998 conforme cadastro.").is_empty());
    }
}
