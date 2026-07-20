//! POST /v1/retrieve — recuperação semântica PURA: embeda a pergunta (local), varre a coleção e
//! devolve os documentos com score. NÃO chama o LLM externo; a pergunta nunca sai do processo
//! (D-MCP-5), então não passa pelo anonimizador e o log registra só metadados.

use std::sync::Arc;
use std::time::Instant;

use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use tracing::{error, info};

use auli_core::corpus;

use crate::api::dto::{RetrieveHit, RetrieveRequest, RetrieveResponse};
use crate::entities::{EntityConfig, get_entity};
use crate::state::AppState;
use crate::util::run_blocking;

/// Teto duro de top_k, acima do n_results padrão dos kinds — protege o embedder e o payload.
const MAX_TOP_K: usize = 20;

/// Kind assumido quando o cliente não manda nenhum: o caso do auditor.
const KIND_PADRAO: &str = "pareceres";

/// Resolve kind + top_k. Puro E independente do registry de entidades — `corpus::from_kind` é um
/// vocabulário estático. Separado de propósito: é o que torna estes contratos testáveis sem
/// `data/registry.toml` (que não existe no ambiente de teste) e sem `Engine` (que carregaria o
/// BGE-M3).
fn validar_kind_top_k(req: &RetrieveRequest) -> Result<(&'static str, usize), (StatusCode, String)> {
    let kind = req.kind.as_deref().unwrap_or(KIND_PADRAO);
    let collection = corpus::from_kind(kind).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let top_k = req.top_k.unwrap_or(collection.n_results).clamp(1, MAX_TOP_K);
    Ok((collection.kind, top_k))
}

/// Validação completa da requisição. O kind vem ANTES da entidade de propósito: é vocabulário
/// estático, então um kind inválido merece 400 mesmo quando a entidade também está errada — erro
/// mais acionável para o cliente do que um 404 sobre a entidade.
fn validar_retrieve(
    req: &RetrieveRequest,
) -> Result<(&'static EntityConfig, &'static str, usize), (StatusCode, String)> {
    let (kind, top_k) = validar_kind_top_k(req)?;
    let cfg = get_entity(req.entity.as_deref()).map_err(|e| (StatusCode::NOT_FOUND, e))?;
    Ok((cfg, kind, top_k))
}

pub async fn retrieve_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RetrieveRequest>,
) -> impl IntoResponse {
    let started = Instant::now();

    let (cfg, kind, top_k) = match validar_retrieve(&req) {
        Ok(v) => v,
        Err((status, message)) => return erro(status, message),
    };

    let engine = state.engine.clone();
    let collection = cfg.collection(kind);
    let question = req.question;

    // Blocking: embed + scan são CPU-bound (mesma disciplina do chat).
    let resultado = run_blocking(move || {
        engine.search(&collection, &question, top_k, 0, f32::INFINITY).map_err(|e| e.to_string().into())
    })
    .await;

    let hits = match resultado {
        Ok(h) => h,
        Err(e) => {
            // Erro interno NUNCA vira corpo de resposta (lição do question_handler): texto fixo
            // amigável para o cliente, detalhe só no log.
            error!(entity = %cfg.id, kind = %kind, "falha no retrieve: {e}");
            return erro(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Não foi possível completar a busca. Tente novamente.".into(),
            );
        }
    };

    // D-MCP-5: log SÓ de metadados — nunca o texto da pergunta.
    info!(
        entity = %cfg.id, kind = %kind, top_k, hits = hits.len(),
        ms = started.elapsed().as_millis() as u64,
        "retrieve concluído"
    );

    // Exatamente UM dos vetores é preenchido, conforme o kind; o outro vai vazio (e serializa
    // vazio de propósito — ver o doc de `RetrieveResponse`).
    let body = if kind == KIND_PADRAO {
        RetrieveResponse {
            entity: cfg.id.clone(),
            kind: kind.into(),
            hits: vec![],
            pareceres: hits
                .into_iter()
                .map(|h| auli_retrieval::decode_parecer(&h.payload, Some(h.score)))
                .collect(),
        }
    } else {
        RetrieveResponse {
            entity: cfg.id.clone(),
            kind: kind.into(),
            hits: hits.into_iter().map(|h| RetrieveHit { score: h.score, texto: h.payload }).collect(),
            pareceres: vec![],
        }
    };
    (StatusCode::OK, Json(body)).into_response()
}

fn erro(status: StatusCode, message: String) -> axum::response::Response {
    (status, Json(serde_json::json!({ "status": "Erro", "message": message }))).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(entity: Option<&str>, kind: Option<&str>, top_k: Option<usize>) -> RetrieveRequest {
        RetrieveRequest {
            question: "crédito de energia".into(),
            entity: entity.map(String::from),
            kind: kind.map(String::from),
            top_k,
        }
    }

    // NOTA: estes testes exercitam `validar_kind_top_k`, não `validar_retrieve`, porque a
    // resolução de entidade depende de `data/registry.toml` — ausente no ambiente de `cargo test`
    // (o `ENTITIES` é um LazyLock sobre o CWD). Foi exatamente por isso que a validação foi
    // partida em duas: o pedaço independente de runtime fica coberto.

    #[test]
    fn kind_desconhecido_e_400() {
        let (status, msg) = validar_kind_top_k(&req(None, Some("planilhas"), None)).unwrap_err();
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(msg.contains("Tipo de coleção desconhecido"), "msg: {msg}");
    }

    #[test]
    fn kind_invalido_ganha_de_entidade_invalida() {
        // A ordem importa: kind (vocabulário estático) é checado antes da entidade, então um
        // pedido errado nos dois eixos recebe o 400 mais acionável, não o 404.
        let (status, _) = validar_retrieve(&req(Some("zz"), Some("planilhas"), None)).unwrap_err();
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn entidade_desconhecida_e_404() {
        // Com kind válido, a falha restante é de entidade. (No ambiente de teste o registry está
        // vazio, então QUALQUER id cai aqui — o que este teste fixa é o status, não a distinção
        // entre "id inexistente" e "registry ausente".)
        let (status, msg) = validar_retrieve(&req(Some("zz"), Some("pareceres"), None)).unwrap_err();
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(msg.contains("Entidade desconhecida"), "msg: {msg}");
    }

    #[test]
    fn kind_ausente_cai_em_pareceres() {
        let (kind, _top_k) = validar_kind_top_k(&req(None, None, None)).unwrap();
        assert_eq!(kind, KIND_PADRAO);
        assert_eq!(kind, "pareceres");
    }

    #[test]
    fn top_k_ausente_usa_o_n_results_do_kind() {
        let (_, top_k) = validar_kind_top_k(&req(None, Some("pareceres"), None)).unwrap();
        assert_eq!(top_k, corpus::PARECERES.n_results);
    }

    #[test]
    fn top_k_e_limitado_ao_teto_e_ao_piso() {
        // Acima do teto: cai em MAX_TOP_K (protege o embedder e o payload).
        let (_, alto) = validar_kind_top_k(&req(None, Some("pareceres"), Some(9999))).unwrap();
        assert_eq!(alto, MAX_TOP_K);
        // Zero não faz sentido numa busca: vira 1, em vez de devolver sempre vazio.
        let (_, zero) = validar_kind_top_k(&req(None, Some("pareceres"), Some(0))).unwrap();
        assert_eq!(zero, 1);
    }

    #[test]
    fn kind_valido_resolve_pelo_vocabulario_do_corpus() {
        for k in ["servicos", "faqs", "pareceres", "notas"] {
            let (kind, _) = validar_kind_top_k(&req(None, Some(k), None)).unwrap();
            assert_eq!(kind, k);
        }
    }
}
