//! Reconhecedor de **endereço** (logradouro + nome + número) para a anonimização da Auli.
//!
//! **Dois porteiros**, ambos obrigatórios: a palavra de logradouro no início (`Rua`, `Av.`,
//! `Travessa`, `Rodovia`, …) e o número no fim. Só um dos dois não basta — é a combinação que
//! separa endereço de menção genérica (`"informe rua e número no cadastro"`).
//!
//! A captura para **no número, inclusive**: bairro, cidade e UF que venham depois ficam de fora
//! (D-ANON-4.4), porque são ambíguos demais e o ganho de mascará-los é pequeno perto do risco.
//!
//! Entre o nome do logradouro e o número é exigido **separador real** (vírgula ou espaço). É o que
//! mantém `Rodovia BR-116 km 23` fora: ali o "número" está colado dentro do próprio token.
//!
//! Case-sensitive de propósito: `rua` em minúscula, no meio de uma frase, é substantivo comum.

use std::sync::LazyLock;

use cloakrs_core::{Confidence, EntityType, Locale, PiiEntity, Recognizer, Span};
use regex::Regex;

use super::comum;

const RECOGNIZER_ID: &str = "auli_endereco_v1";
const LOCALES_BR: &[Locale] = &[Locale::BR];

/// Grupo `endereco`: do logradouro até o número, inclusive (é o span emitido).
///
/// O conector é opcional **antes do 1º token** e entre os demais: sem isso, `Travessa dos
/// Cataventos, nº 22` e `Rua dos Andradas, 736` — a forma mais comum de endereço brasileiro —
/// não casariam, porque o token seguinte ao logradouro seria o conector em minúscula.
static RE_ENDERECO: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)
        (?P<endereco>
            (?: Rua | Av\.? | Avenida | Travessa | Trav\. | Pra[çc]a |
                Rodovia | Rod\. | Estrada | Alameda | Al\. | Largo )
            \s+
            (?: (?:d[aeo]s?|e) \s+ )?                      # conector antes do 1º token
            \p{Lu}[\p{L}\d'’.\-]*                          # 1º token do logradouro
            (?:                                            # + 0..4 tokens
                \s+
                (?: (?:d[aeo]s?|e) \s+ )?
                \p{Lu}[\p{L}\d'’.\-]*
            ){0,4}
            (?: \s*,\s* | \s+ )                            # separador REAL antes do número
            (?: n[ºo°]?\.?\s* )?                           # prefixo nº opcional
            \d{1,6}
        )
        \b
        ",
    )
    .expect("regex de endereço é válida")
});

/// Reconhecedor de endereço. Sem estado — construir com [`Self::novo`].
pub struct EnderecoRecognizer;

impl EnderecoRecognizer {
    #[must_use]
    pub fn novo() -> Self {
        Self
    }
}

impl Recognizer for EnderecoRecognizer {
    fn id(&self) -> &str {
        RECOGNIZER_ID
    }

    fn entity_type(&self) -> EntityType {
        EntityType::Custom("endereco".into())
    }

    fn supported_locales(&self) -> &[Locale] {
        LOCALES_BR
    }

    fn scan(&self, text: &str) -> Vec<PiiEntity> {
        let mut achados = Vec::new();

        for c in RE_ENDERECO.captures_iter(text) {
            let m = c.name("endereco").expect("grupo `endereco` sempre casa");
            if !comum::limites_ok(text, m.start(), m.end()) {
                continue;
            }
            achados.push(PiiEntity {
                entity_type: self.entity_type(),
                span: Span::new(m.start(), m.end()),
                text: m.as_str().to_string(),
                confidence: Confidence::new(0.7).expect("confiança válida"),
                recognizer_id: RECOGNIZER_ID.to_string(),
            });
        }

        achados
    }

    fn validate(&self, candidate: &str) -> bool {
        RE_ENDERECO
            .captures(candidate)
            .is_some_and(|c| c.name("endereco").is_some_and(|m| m.as_str() == candidate))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn achados(text: &str) -> Vec<String> {
        EnderecoRecognizer::novo()
            .scan(text)
            .into_iter()
            .map(|e| e.text)
            .collect()
    }

    #[test]
    fn positivos() {
        assert_eq!(
            achados("estabelecida na Av. Mauá, 1155, Centro"),
            ["Av. Mauá, 1155"]
        );
        assert_eq!(
            achados("Rua Sete de Setembro, 666 em Porto Alegre"),
            ["Rua Sete de Setembro, 666"]
        );
        assert_eq!(
            achados("na Avenida Ipiranga 6681"),
            ["Avenida Ipiranga 6681"]
        );
    }

    #[test]
    fn conector_antes_do_primeiro_token() {
        // A forma mais comum de endereço brasileiro; sem o conector opcional, nenhuma das duas casa.
        assert_eq!(
            achados("Travessa dos Cataventos, nº 22"),
            ["Travessa dos Cataventos, nº 22"]
        );
        assert_eq!(achados("Rua dos Andradas, 736"), ["Rua dos Andradas, 736"]);
    }

    #[test]
    fn captura_para_no_numero() {
        // D-ANON-4.4: bairro/cidade/UF não entram.
        assert_eq!(
            achados("Av. Borges de Medeiros, 1501, Praia de Belas, Porto Alegre/RS"),
            ["Av. Borges de Medeiros, 1501"]
        );
    }

    #[test]
    fn negativos() {
        // Rodovia com km: o dígito está colado no token, sem separador real antes.
        assert!(achados("acidente na Rodovia BR-116 km 23").is_empty());
        // Logradouro sem número: falta o segundo porteiro.
        assert!(achados("fica na Rua da Praia, sem número").is_empty());
        // "Al" sem ponto não é abreviatura de Alameda.
        assert!(achados("Al Pacino 1974").is_empty());
        // Menção genérica, em minúsculas.
        assert!(achados("informe rua e número no cadastro").is_empty());
    }

    #[test]
    fn respeita_boundaries() {
        assert!(achados("xRua dos Andradas, 736").is_empty());
    }

    #[test]
    fn tipo_e_confianca_corretos() {
        let e = &EnderecoRecognizer::novo().scan("na Av. Mauá, 1155 fica")[0];
        assert_eq!(e.entity_type, EntityType::Custom("endereco".into()));
        assert_eq!(e.confidence.value(), 0.7);
        assert_eq!(e.recognizer_id, RECOGNIZER_ID);
    }
}
