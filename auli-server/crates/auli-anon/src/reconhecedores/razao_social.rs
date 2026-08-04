//! Reconhecedor de **razão social** para o pipeline de anonimização da Auli.
//!
//! Porteiro único: o **sufixo societário** (`Ltda`, `S.A.`, `S/A`, `EIRELI`, `EPP`, `ME`, `MEI`,
//! `Cia`, `Companhia`). Sem sufixo não há match — é o que mantém a fixture de controle limpa e o
//! que deixa nomes de órgão capitalizados (`Receita Estadual`) de fora, de propósito.
//!
//! Sufixos **fracos** (`ME`, `SA`, `EPP`, `MEI`) colidem com sigla tributária e termo
//! institucional, então pedem duas travas além da contagem de tokens:
//! 1. **Titlecase** — os 2 tokens anteriores precisam ter minúscula depois da inicial, o que
//!    separa `Comercial Andrade ME` de `PARCELAMENTO ICMS ME` (sigla em caixa alta);
//! 2. **stoplist** institucional (`comum::tem_stop`), sem a qual `Simples Nacional ME` viraria
//!    falso positivo — ambos medidos antes de existirem as travas.
//!
//! A janela de 1+4 tokens antes do sufixo evita capturar uma frase capitalizada longa inteira.

use std::sync::LazyLock;

use cloakrs_core::{Confidence, EntityType, Locale, PiiEntity, Recognizer, Span};
use regex::Regex;

use super::comum;

const RECOGNIZER_ID: &str = "auli_razao_social_v1";
const LOCALES_BR: &[Locale] = &[Locale::BR];

/// Sufixos inequívocos: 1 token capitalizado antes já basta.
const SUFIXOS_FORTES: &[&str] = &["Ltda", "Ltda.", "EIRELI", "Cia", "Cia.", "Companhia"];

/// Grupo `razao`: a razão social completa, sufixo incluído (é o span emitido).
/// Grupo `sufixo`: só o sufixo, para escolher a regra de validação.
///
/// A guarda final consome **um caractere não alfanumérico** em vez de usar `\b`: com `\b`, um
/// sufixo terminado em ponto (`S.A.`, `Ltda.`) perderia o ponto, porque não há fronteira de
/// palavra entre `.` e o espaço seguinte e o motor desiste do `\.?` opcional. Como o span sai do
/// grupo `razao`, o caractere consumido pela guarda não entra no que é mascarado.
static RE_RAZAO: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)
        (?P<razao>
            \p{Lu}[\p{L}\d&'’.\-]*                         # 1º token capitalizado
            (?:                                            # + 0..4 tokens
                \s+
                (?: (?:d[aeo]s?|e|&) \s+ )?                # conector opcional
                \p{Lu}[\p{L}\d&'’.\-]*
            ){0,4}
            \s+
            (?P<sufixo>
                Ltda\.? | S\.?/A\.? | S\.?A\.? | SA |
                EIRELI | EPP | MEI | ME |
                Cia\.? | Companhia
            )
        )
        (?: [^\p{L}\p{N}] | $ )
        ",
    )
    .expect("regex de razão social é válida")
});

/// Reconhecedor de razão social. Sem estado — construir com [`Self::novo`].
pub struct RazaoSocialRecognizer;

impl RazaoSocialRecognizer {
    #[must_use]
    pub fn novo() -> Self {
        Self
    }

    /// Sufixo forte exige ≥ 1 token capitalizado antes; fraco exige ≥ 2 **Titlecase**.
    /// A stoplist vale para os dois.
    fn valida(razao: &str, sufixo: &str) -> bool {
        if comum::tem_stop(razao) {
            return false;
        }
        let prefixo = razao.strip_suffix(sufixo).unwrap_or(razao).trim_end();
        let tokens = prefixo.split_whitespace();
        if SUFIXOS_FORTES.contains(&sufixo) {
            tokens.filter(|t| comum::comeca_maiuscula(t)).count() >= 1
        } else {
            tokens.filter(|t| comum::titlecase(t)).count() >= 2
        }
    }
}

impl Recognizer for RazaoSocialRecognizer {
    fn id(&self) -> &str {
        RECOGNIZER_ID
    }

    fn entity_type(&self) -> EntityType {
        EntityType::Custom("razao_social".into())
    }

    fn supported_locales(&self) -> &[Locale] {
        LOCALES_BR
    }

    fn scan(&self, text: &str) -> Vec<PiiEntity> {
        let mut achados = Vec::new();

        for c in RE_RAZAO.captures_iter(text) {
            let razao = c.name("razao").expect("grupo `razao` sempre casa");
            let sufixo = c.name("sufixo").expect("grupo `sufixo` sempre casa");
            if !comum::limites_ok(text, razao.start(), razao.end()) {
                continue;
            }
            if !Self::valida(razao.as_str(), sufixo.as_str()) {
                continue;
            }
            achados.push(PiiEntity {
                entity_type: self.entity_type(),
                span: Span::new(razao.start(), razao.end()),
                text: razao.as_str().to_string(),
                confidence: Confidence::new(0.7).expect("confiança válida"),
                recognizer_id: RECOGNIZER_ID.to_string(),
            });
        }

        achados
    }

    fn validate(&self, candidate: &str) -> bool {
        RE_RAZAO.captures(candidate).is_some_and(|c| {
            let razao = c.name("razao").expect("grupo `razao` sempre casa");
            let sufixo = c.name("sufixo").expect("grupo `sufixo` sempre casa");
            razao.as_str() == candidate && Self::valida(razao.as_str(), sufixo.as_str())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn achados(text: &str) -> Vec<String> {
        RazaoSocialRecognizer::novo()
            .scan(text)
            .into_iter()
            .map(|e| e.text)
            .collect()
    }

    #[test]
    fn positivos() {
        assert_eq!(
            achados("A empresa Anderle Transportes Ltda pediu"),
            ["Anderle Transportes Ltda"]
        );
        assert_eq!(
            achados("junto à Silva & Souza Cia."),
            ["Silva & Souza Cia."]
        );
        assert_eq!(
            achados("cadastro da Padaria do Vale EIRELI"),
            ["Padaria do Vale EIRELI"]
        );
        // Sufixo fraco com 2 tokens Titlecase: casa.
        assert_eq!(
            achados("a Comercial Andrade ME informou"),
            ["Comercial Andrade ME"]
        );
    }

    #[test]
    fn sufixo_com_ponto_mantem_o_ponto() {
        // A guarda consumida (e não `\b`) é o que preserva o ponto final de `S.A.`.
        assert_eq!(
            achados("da Metalúrgica Gaúcha S.A. em"),
            ["Metalúrgica Gaúcha S.A."]
        );
    }

    #[test]
    fn sufixo_fraco_com_um_token_nao_casa() {
        assert!(achados("o José ME respondeu").is_empty());
        assert!(achados("cadastrada como Andrade SA?").is_empty());
    }

    #[test]
    fn sufixo_fraco_com_sigla_em_caixa_alta_nao_casa() {
        // Sem a regra de Titlecase, os três viravam razão social.
        assert!(achados("PARCELAMENTO ICMS ME").is_empty());
        assert!(achados("BAIXA DA IE ME INFORME").is_empty());
    }

    #[test]
    fn termo_institucional_nao_casa() {
        // Titlecase sozinho não resolveria: quem barra é a stoplist.
        assert!(achados("O regime do Simples Nacional ME impede o crédito?").is_empty());
    }

    #[test]
    fn sem_sufixo_ou_em_minusculas_nao_casa() {
        assert!(achados("Você me disse que sim").is_empty());
        assert!(achados("consultar a Receita Estadual do RS").is_empty());
        assert!(achados("Veja a Instrução Normativa RE 45/2019 e o Decreto 37.699/97.").is_empty());
    }

    #[test]
    fn janela_limita_a_captura() {
        // Frase capitalizada longa: só os <= 5 tokens antes do sufixo entram.
        // (Sem termo da stoplist no meio — quem está sob teste aqui é a janela, não a stoplist.)
        let v =
            achados("Grupo Industrial Comercial Importadora Exportadora Atlântica Serrana Ltda");
        assert_eq!(v.len(), 1);
        assert!(
            v[0].split_whitespace().count() <= 6,
            "capturou demais: {v:?}"
        );
        assert!(v[0].ends_with("Ltda"));
    }

    #[test]
    fn respeita_boundaries() {
        assert!(achados("xAnderle Transportes Ltda").is_empty());
    }

    #[test]
    fn tipo_e_confianca_corretos() {
        let e = &RazaoSocialRecognizer::novo().scan("a Anderle Transportes Ltda pediu")[0];
        assert_eq!(e.entity_type, EntityType::Custom("razao_social".into()));
        assert_eq!(e.confidence.value(), 0.7);
        assert_eq!(e.recognizer_id, RECOGNIZER_ID);
    }
}
