//! Monta o POOL de candidatos para rotular as consultas do A/B (TAREFA-FAQ-PR, P5).
//!
//! Rotular olhando o resultado de UM dos packs enviesaria o gate a favor dele. O pool é a UNIÃO dos
//! top-k dos dois packs: todo item que qualquer um dos lados considerou plausível entra na lista, e
//! o julgamento humano decide quais respondem. É a mecânica de pooling do TREC, na escala daqui.
//!
//! Emite JSON no stdout: `[{consulta, candidatos: [{pergunta, resposta, origem: [antes|novo]}]}]`.
//!
//! Rodar:
//!   AULI_AB_ANTES=/tmp/faqpr/rs-faqs-ANTES.json AULI_AB_NOVO=/tmp/faqpr/rs-faqs-NOVO.json \
//!   AULI_AB_CONSULTAS=.../consultas.txt EMBED_CACHE_DIR=../../models \
//!   cargo test -p auli-cli --release --test ab_pool -- --nocapture --ignored > pool.json

use auli_core::embed::Embedder;
use vector_store::{cosine_distance, read_collection_file};

const POOL_K: usize = 8;

#[test]
#[ignore = "ferramenta de rotulagem; rode explicitamente com AULI_AB_* setados"]
fn dump_pool() {
    let (Ok(antes_p), Ok(novo_p)) = (std::env::var("AULI_AB_ANTES"), std::env::var("AULI_AB_NOVO"))
    else {
        eprintln!("AULI_AB_ANTES/AULI_AB_NOVO não setados — pulando");
        return;
    };
    let consultas_p = std::env::var("AULI_AB_CONSULTAS").expect("AULI_AB_CONSULTAS");
    let cache = std::env::var("EMBED_CACHE_DIR").unwrap_or_else(|_| "../../models".into());

    let antes = read_collection_file::<String>(&antes_p).expect("ANTES").records;
    let novo = read_collection_file::<String>(&novo_p).expect("NOVO").records;
    let consultas: Vec<String> = std::fs::read_to_string(&consultas_p)
        .expect("consultas")
        .lines()
        .map(|l| l.split('\t').next().unwrap_or("").trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    let embedder = Embedder::new(cache.into(), 8).expect("embedder");
    let mut saida = Vec::new();

    for c in &consultas {
        let q = embedder.embed_dense(vec![c.clone()]).expect("embed")[0].clone();
        let topo = |pack: &[vector_store::Record<String>]| -> Vec<String> {
            let mut d: Vec<(f32, String)> = pack
                .iter()
                .map(|r| (cosine_distance(&r.embedding, &q), r.payload.clone()))
                .collect();
            d.sort_by(|a, b| a.0.total_cmp(&b.0));
            d.into_iter().take(POOL_K).map(|(_, doc)| doc).collect()
        };
        let (ta, tn) = (topo(&antes), topo(&novo));
        let mut candidatos = Vec::new();
        let mut vistos = std::collections::HashSet::new();
        for (origem, docs) in [("antes", &ta), ("novo", &tn)] {
            for doc in docs {
                if vistos.insert(doc.clone()) {
                    candidatos.push(serde_json::json!({ "origem": origem, "doc": doc }));
                }
            }
        }
        saida.push(serde_json::json!({ "consulta": c, "candidatos": candidatos }));
    }
    println!("{}", serde_json::to_string(&saida).unwrap());
}
