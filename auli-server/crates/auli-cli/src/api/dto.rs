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

/// Corpo do POST /v1/retrieve. `kind` usa o vocabulário único de `corpus::from_kind`.
#[derive(Debug, Deserialize)]
pub struct RetrieveRequest {
    pub question: String,
    #[serde(default)]
    pub entity: Option<String>,
    /// "servicos" | "faqs" | "pareceres" | "notas". Ausente ⇒ "pareceres" (o caso do auditor).
    #[serde(default)]
    pub kind: Option<String>,
    /// Teto de resultados. Ausente ⇒ o `n_results` do kind; sempre limitado a MAX_TOP_K.
    #[serde(default)]
    pub top_k: Option<usize>,
}

/// Um hit genérico (servicos/faqs/notas): o texto indexado + a distância cosseno.
#[derive(Debug, Serialize)]
pub struct RetrieveHit {
    pub score: f32,
    pub texto: String,
}

/// Resposta. Os DOIS vetores SEMPRE serializam, mesmo vazios: array vazio no vetor do kind pedido
/// significa "zero resultados" — distinguível de erro ou de kind trocado, o que um
/// `skip_serializing_if` apagaria (o cliente receberia um objeto sem array nenhum).
/// `pareceres` → vetor `pareceres` (estruturado, SEM corpo); demais kinds → vetor `hits`.
#[derive(Debug, Serialize)]
pub struct RetrieveResponse {
    pub entity: String,
    pub kind: String,
    pub hits: Vec<RetrieveHit>,
    pub pareceres: Vec<auli_retrieval::ParecerHit>,
}

#[cfg(test)]
mod tests {
    use super::{Question, RetrieveHit, RetrieveRequest, RetrieveResponse};

    #[test]
    fn retrieve_request_sem_kind_e_top_k_desserializa_com_none() {
        let r: RetrieveRequest = serde_json::from_str(r#"{"question":"x"}"#).unwrap();
        assert_eq!(r.question, "x");
        assert_eq!(r.entity, None);
        assert_eq!(r.kind, None);
        assert_eq!(r.top_k, None);
    }

    #[test]
    fn retrieve_response_serializa_os_dois_vetores_mesmo_vazios() {
        // O contrato do item 5 da revisão: zero resultados ≠ ausência de campo. Um cliente precisa
        // conseguir distinguir "busquei e não achei" de "kind trocado" / "erro".
        let vazia = RetrieveResponse {
            entity: "sc".into(),
            kind: "pareceres".into(),
            hits: vec![],
            pareceres: vec![],
        };
        let json = serde_json::to_value(&vazia).unwrap();
        assert_eq!(json["hits"], serde_json::json!([]));
        assert_eq!(json["pareceres"], serde_json::json!([]));
    }

    #[test]
    fn retrieve_response_de_kind_generico_carrega_hits() {
        let r = RetrieveResponse {
            entity: "rs".into(),
            kind: "servicos".into(),
            hits: vec![RetrieveHit { score: 0.25, texto: "SERVICO".into() }],
            pareceres: vec![],
        };
        let json = serde_json::to_value(&r).unwrap();
        assert_eq!(json["hits"][0]["texto"], "SERVICO");
        assert_eq!(json["hits"][0]["score"], 0.25);
        assert_eq!(json["pareceres"], serde_json::json!([]));
    }

    #[test]
    fn parecer_hit_omite_corpo_e_score_ausentes() {
        // Contrato do /v1/retrieve: a busca devolve metadados, NUNCA o corpo (que é o que
        // `obter_parecer`/MCP servem sob demanda).
        let r = RetrieveResponse {
            entity: "sc".into(),
            kind: "pareceres".into(),
            hits: vec![],
            pareceres: vec![auli_retrieval::decode_parecer(
                &serde_json::json!({
                    "numero": "PARECER Nº 1", "assunto": "ICMS", "resumo": "R.",
                    "link": "http://x/1", "doc_path": "docs/pareceres/p1.md"
                })
                .to_string(),
                // 0.25 é exatamente representável em binário; um 0.1 sairia como
                // 0.10000000149011612 no JSON (o score é f32 e widening para f64 expõe o ruído).
                Some(0.25),
            )],
        };
        let json = serde_json::to_value(&r).unwrap();
        assert_eq!(json["pareceres"][0]["numero"], "PARECER Nº 1");
        assert_eq!(json["pareceres"][0]["score"], 0.25);
        assert!(json["pareceres"][0].get("corpo").is_none(), "corpo nunca vai na busca");
    }

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
