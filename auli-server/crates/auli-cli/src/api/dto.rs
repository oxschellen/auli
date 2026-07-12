// Types for interaction with the Web UI
use derive_more::Display;
use serde::{Deserialize, Serialize};

// Struct for input Questions
#[derive(Debug, Display, Serialize, Deserialize)]
#[display("question: {}", question)]
pub struct Question {
    pub question: String,
    // Target entity (state) id. Missing/empty -> default entity ("rs").
    #[serde(default)]
    pub entity: Option<String>,
    // Query type sent by the UI: 1 = Serviços+FAQs (default), 2 = Pareceres. Missing/unknown ->
    // default (see `QueryType::from_code`). `type` is a Rust keyword, hence the rename.
    #[serde(default, rename = "type")]
    pub query_type: Option<u8>,
}

// Query string for GET data-management routes, e.g. `?entity=rs`.
#[derive(Debug, Deserialize)]
pub struct EntityQuery {
    #[serde(default)]
    pub entity: Option<String>,
}

// Struct for output of Answers
#[derive(Debug, Display, Serialize, Deserialize)]
#[display("question: {}\nanswer: {}", question, answer)]
pub struct Answer {
    pub question: String,
    pub answer: String,
}

#[cfg(test)]
mod tests {
    use super::Question;

    #[test]
    fn question_reads_the_type_field() {
        let q: Question =
            serde_json::from_str(r#"{"question":"x","entity":"rs","type":2}"#).unwrap();
        assert_eq!(q.query_type, Some(2));
    }

    #[test]
    fn question_without_type_or_entity_defaults_to_none() {
        let q: Question = serde_json::from_str(r#"{"question":"x"}"#).unwrap();
        assert_eq!(q.query_type, None);
        assert_eq!(q.entity, None);
    }
}
