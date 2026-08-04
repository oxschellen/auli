//! Reconhecedor de **nome de pessoa** por âncora de contexto — a entidade mais ambígua da Fase 4.
//!
//! Nome não tem forma canônica: qualquer sequência capitalizada pode ser um. Por isso o porteiro é
//! o **gatilho** (`Sr.`, `contribuinte`, `requerente`, `sócio`, `titular`, `contador`, …)
//! imediatamente antes da sequência. Sem gatilho não há match — `João da Silva Pereira` solto numa
//! frase passa incólume, e isso é deliberado: recall parcial é aceito, falso positivo não.
//!
//! Duas limitações conhecidas e documentadas (insumo da fase de pesquisa NER):
//! - nome de **1 token** (`Sr. João`) não é capturado — 2 a 4 tokens é o mínimo (D-ANON-4.5);
//! - gatilho distante do nome (`o contribuinte, que já esteve aqui, João Silva`) não casa.
//!
//! O span emitido é **apenas o nome**: o gatilho nunca é mascarado, senão a pergunta perderia o
//! sentido (`o [NOME_1] solicitou` em vez de `o contribuinte [NOME_1] solicitou`).

use std::sync::LazyLock;

use cloakrs_core::{Confidence, EntityType, Locale, PiiEntity, Recognizer, Span};
use regex::Regex;

use super::comum;

const RECOGNIZER_ID: &str = "auli_nome_pessoa_v1";
const LOCALES_BR: &[Locale] = &[Locale::BR];

/// Gatilhos, do mais longo para o mais curto dentro de cada família (`Srta` antes de `Sra` antes
/// de `Sr`), para o motor não parar na alternativa curta.
const GATILHOS: &str = r"Srta\.?|Sra\.?|Sr\.?|[Cc]ontribuinte|[Rr]equerente|[Pp]rodutor(?:a)?\s+rural|[Ss][óo]ci[oa]|[Nn]ome|[Tt]itular|[Rr]espons[áa]vel|[Cc]ontador(?:a)?";

/// Núcleo do nome: 2 a 4 tokens capitalizados, com conectores entre eles. Compartilhado pela regex
/// ancorada no gatilho e pela do [`Recognizer::validate`], que recebe o nome já isolado.
const NUCLEO: &str = r"\p{Lu}\p{L}+(?:\s+(?:d[aeo]s?\s+|e\s+)?\p{Lu}\p{L}+){1,3}";

/// Gatilho + nome. O `\b` inicial é obrigatório: sem ele, o gatilho `nome` casaria dentro de
/// *sobrenome* e `informe o sobrenome Silva Pereira` viraria detecção.
///
/// O conector opcional entre gatilho e nome é o que faz `em nome de Maria Eduarda dos Santos`
/// casar — sem ele o `de` interrompia a sequência e o nome inteiro escapava.
static RE_NOME: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"(?x) \b (?: {GATILHOS} ) \s* [:,]? \s* (?: (?:d[aeo]s?|e) \s+ )? (?P<nome> {NUCLEO} ) \b"
    ))
    .expect("regex de nome de pessoa é válida")
});

/// O núcleo sozinho, ancorado — o `validate` recebe o candidato já sem o gatilho.
static RE_NUCLEO: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(r"^(?:{NUCLEO})$")).expect("regex de núcleo de nome é válida")
});

/// Reconhecedor de nome de pessoa. Sem estado — construir com [`Self::novo`].
pub struct NomePessoaRecognizer;

impl NomePessoaRecognizer {
    #[must_use]
    pub fn novo() -> Self {
        Self
    }
}

impl Recognizer for NomePessoaRecognizer {
    fn id(&self) -> &str {
        RECOGNIZER_ID
    }

    fn entity_type(&self) -> EntityType {
        EntityType::Custom("nome".into())
    }

    fn supported_locales(&self) -> &[Locale] {
        LOCALES_BR
    }

    fn scan(&self, text: &str) -> Vec<PiiEntity> {
        let mut achados = Vec::new();

        for c in RE_NOME.captures_iter(text) {
            // Span do GRUPO, não do match: o gatilho fica de fora do que será mascarado.
            let m = c.name("nome").expect("grupo `nome` sempre casa");
            if !comum::limites_ok(text, m.start(), m.end()) {
                continue;
            }
            // Stoplist: o gatilho é seguido de termo institucional com frequência
            // ("contribuinte Pessoa Física", "Sr. Contribuinte Substituto Tributário").
            if comum::tem_stop(m.as_str()) {
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
        RE_NUCLEO.is_match(candidate) && !comum::tem_stop(candidate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn achados(text: &str) -> Vec<String> {
        NomePessoaRecognizer::novo()
            .scan(text)
            .into_iter()
            .map(|e| e.text)
            .collect()
    }

    #[test]
    fn positivos() {
        assert_eq!(
            achados("o contribuinte João da Silva Pereira solicitou"),
            ["João da Silva Pereira"]
        );
        assert_eq!(
            achados("Sr. Paulo Roberto Antunes compareceu"),
            ["Paulo Roberto Antunes"]
        );
        assert_eq!(achados("o sócio Carlos Alberto informou"), ["Carlos Alberto"]);
        assert_eq!(achados("titular: Ana Beatriz Rocha"), ["Ana Beatriz Rocha"]);
    }

    #[test]
    fn conector_entre_gatilho_e_nome() {
        assert_eq!(
            achados("em nome de Maria Eduarda dos Santos"),
            ["Maria Eduarda dos Santos"]
        );
    }

    #[test]
    fn sem_gatilho_nao_casa() {
        // Limitação assumida: nome solto passa. Precisão vence recall nesta fase.
        assert!(achados("João da Silva Pereira solicitou a baixa").is_empty());
    }

    #[test]
    fn um_token_nao_casa() {
        // D-ANON-4.5.
        assert!(achados("o contribuinte João solicitou").is_empty());
    }

    #[test]
    fn termo_institucional_apos_gatilho_nao_casa() {
        assert!(achados("o contribuinte Pessoa Física deve").is_empty());
        assert!(achados("titular da Inscrição Estadual pode").is_empty());
        assert!(achados("nome da Nota Fiscal emitida").is_empty());
        assert!(achados("responsável na Receita Estadual").is_empty());
        assert!(achados("Sr. Contribuinte Substituto Tributário deve").is_empty());
        assert!(achados("o contribuinte Vossa Senhoria informou").is_empty());
    }

    #[test]
    fn gatilho_dentro_de_outra_palavra_nao_conta() {
        // Sem o \b inicial, o gatilho `nome` casaria dentro de "sobrenome".
        assert!(achados("informe o sobrenome Silva Pereira no cadastro").is_empty());
    }

    #[test]
    fn gatilho_sem_nome_depois_nao_casa() {
        assert!(achados("O contribuinte deve apresentar a documentação.").is_empty());
    }

    #[test]
    fn span_exclui_o_gatilho() {
        let e = &NomePessoaRecognizer::novo().scan("Sr. Paulo Roberto Antunes compareceu")[0];
        assert_eq!(e.text, "Paulo Roberto Antunes");
        assert_eq!(e.span.start, 4, "o span não pode começar no gatilho");
    }

    #[test]
    fn tipo_e_confianca_corretos() {
        let e = &NomePessoaRecognizer::novo().scan("o sócio Carlos Alberto informou")[0];
        assert_eq!(e.entity_type, EntityType::Custom("nome".into()));
        assert_eq!(e.confidence.value(), 0.7);
        assert_eq!(e.recognizer_id, RECOGNIZER_ID);
    }
}
