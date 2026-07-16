//! Reconhecedor de Inscrição Estadual (IE) para o pipeline de anonimização da Auli.
//!
//! Estruturado como **tabela de padrões por UF**, para crescer junto com a indexação
//! de novos estados (SP e SC na fila). Implementado: RS.
//!
//! Regras:
//! - Contexto obrigatório SEMPRE ("IE", "inscrição estadual", "insc. est.") — números
//!   de 9–12 dígitos são ambíguos demais para disparo livre.
//! - Dígito verificador é reforço de confiança, não porteiro: DV válido → 0.9;
//!   DV inválido mas contexto explícito → 0.7 e mascara mesmo assim
//!   (um erro de digitação do atendente não pode virar vazamento no log).
//!
//! Formatos por UF (referência SINTEGRA):
//! - RS: 10 dígitos, `NNN/NNNNNNN` — DV mod-11, pesos 2,9,8,7,6,5,4,3,2 sobre os 9 primeiros.
//! - SP (futuro): 12 dígitos `NNN.NNN.NNN.NNN`, dois DVs (posições 9 e 12).
//! - SC (futuro): 9 dígitos `NNN.NNN.NNN`, DV mod-11, pesos 9..2 sobre os 8 primeiros.

use cloakrs_core::{Confidence, EntityType, Locale, PiiEntity, Recognizer, Span};
use regex::Regex;

const LOCALES_BR: &[Locale] = &[Locale::BR];

/// Uma entrada da tabela: UF, padrão textual e validador de DV.
struct PadraoUf {
    uf: &'static str,
    regex: Regex,
    valida_dv: fn(&str) -> bool,
}

/// Reconhecedor de IE. Compila os padrões e o contexto uma vez em [`Self::novo`].
pub struct InscricaoEstadualRecognizer {
    padroes: Vec<PadraoUf>,
    contexto: Regex,
}

impl InscricaoEstadualRecognizer {
    #[must_use]
    pub fn novo() -> Self {
        Self {
            padroes: vec![PadraoUf {
                uf: "RS",
                // 3 dígitos + separador opcional (/ ou espaço) + 7 dígitos.
                regex: Regex::new(r"\d{3}[/\s]?\d{7}").expect("regex IE-RS inválida"),
                valida_dv: dv_rs,
            }],
            // Palavras de contexto com fronteira de palavra — "ie" solto casaria
            // dentro de "série", "período" etc., por isso o \b é obrigatório.
            contexto: Regex::new(
                r"(?i)\b(?:i\.?e\.?|inscri[çc][ãa]o\s+estadual|insc\.?\s*est(?:adual|\.)?)\b",
            )
            .expect("regex de contexto IE inválida"),
        }
    }

    fn limites_ok(text: &str, start: usize, end: usize) -> bool {
        let antes = text[..start].chars().next_back();
        let depois = text[end..].chars().next();
        let livre = |c: Option<char>| c.is_none_or(|c| !c.is_ascii_alphanumeric());
        // Recusa separador colado a dígito nas bordas (evita fatiar CNPJ 11.222.333/0001-81
        // ou protocolo 2026/000123456 pelo meio).
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

impl Recognizer for InscricaoEstadualRecognizer {
    fn id(&self) -> &str {
        "auli_ie_v1"
    }

    fn entity_type(&self) -> EntityType {
        EntityType::Custom("ie".into())
    }

    fn supported_locales(&self) -> &[Locale] {
        LOCALES_BR
    }

    fn scan(&self, text: &str) -> Vec<PiiEntity> {
        let mut achados: Vec<PiiEntity> = Vec::new();

        for padrao in &self.padroes {
            for m in padrao.regex.find_iter(text) {
                if !Self::limites_ok(text, m.start(), m.end()) {
                    continue;
                }
                if !self.tem_contexto(text, m.start()) {
                    continue; // contexto é obrigatório para IE, sem exceção
                }
                if achados
                    .iter()
                    .any(|a| a.span.start < m.end() && m.start() < a.span.end)
                {
                    continue; // já coberto por padrão de outra UF
                }
                let confianca = if (padrao.valida_dv)(m.as_str()) { 0.9 } else { 0.7 };
                achados.push(PiiEntity {
                    entity_type: self.entity_type(),
                    span: Span::new(m.start(), m.end()),
                    text: m.as_str().to_string(),
                    confidence: Confidence::new(confianca).expect("confiança válida"),
                    recognizer_id: self.id().to_string(),
                });
                let _ = padrao.uf; // reservado para telemetria futura por UF
            }
        }

        achados
    }

    fn validate(&self, candidate: &str) -> bool {
        // Aceita se algum padrão da tabela reconhece o formato (DV não é porteiro).
        self.padroes.iter().any(|p| p.regex.is_match(candidate))
    }
}

/// DV da IE do RS: mod-11 com pesos 2,9,8,7,6,5,4,3,2 sobre os 9 primeiros dígitos.
/// DV = 11 − (soma % 11); resultado 10 ou 11 → 0.
fn dv_rs(candidate: &str) -> bool {
    let d: Vec<u32> = candidate.chars().filter_map(|c| c.to_digit(10)).collect();
    if d.len() != 10 {
        return false;
    }
    const PESOS: [u32; 9] = [2, 9, 8, 7, 6, 5, 4, 3, 2];
    let soma: u32 = d[..9].iter().zip(PESOS.iter()).map(|(x, p)| x * p).sum();
    let dv = match 11 - (soma % 11) {
        10 | 11 => 0,
        v => v,
    };
    d[9] == dv
}

#[cfg(test)]
mod tests {
    use super::*;

    fn achados(text: &str) -> Vec<String> {
        InscricaoEstadualRecognizer::novo()
            .scan(text)
            .into_iter()
            .map(|e| e.text)
            .collect()
    }

    #[test]
    fn dv_rs_valido_e_invalido() {
        assert!(dv_rs("224/3210015")); // DV correto (soma 94, resto 6, DV 5)
        assert!(!dv_rs("224/3210012")); // DV errado
    }

    #[test]
    fn exige_contexto() {
        // Mesmo formato de IE, sem palavra de contexto → não dispara.
        assert!(achados("O número 224/3210012 apareceu no relatório.").is_empty());
    }

    #[test]
    fn mascara_com_contexto_mesmo_com_dv_invalido() {
        // 224/3210012 tem DV inválido, mas o contexto "IE" garante o mascaramento (0.7).
        assert_eq!(achados("A IE 224/3210012 consta como baixada."), ["224/3210012"]);
        // Forma "inscrição estadual" por extenso também vale como contexto.
        assert_eq!(achados("A inscrição estadual 224/3210012 está ativa."), ["224/3210012"]);
    }

    #[test]
    fn nao_fatia_cnpj_nem_protocolo() {
        // Separador colado a dígito nas bordas → não fatia estruturas maiores.
        assert!(achados("IE do CNPJ 11.222.333/0001-81 pendente.").is_empty());
        assert!(achados("IE e o protocolo 2026/000123456 juntos.").is_empty());
    }
}
