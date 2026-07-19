//! Teste de retrieval PURO (sem LLM) do pack de pareceres — valida a key nova (título + assunto +
//! sinopse). Carrega o embedder BGE-M3 e o `ReadStore`, embedda cada pergunta e imprime o top-5 por
//! proximidade. Uso:
//!   EMBED_CACHE_DIR=/abs/models cargo run -p auli-cli --example retrieval_test -- data/sc/packs/sc-pareceres.json

use std::path::PathBuf;

use auli_core::embed::Embedder;
use vector_store::ReadStore;

/// Extrai `numero` (1ª linha após `## pergunta`) e `assunto` (2ª) do `stored_repr` para exibição.
fn resumo_doc(doc: &str) -> (String, String) {
    let linhas: Vec<&str> = doc.lines().collect();
    let idx = linhas.iter().position(|l| l.trim_start().starts_with("## pergunta"));
    match idx {
        Some(i) => {
            let numero = linhas.get(i + 1).map(|s| s.trim()).unwrap_or("").to_string();
            let assunto = linhas.get(i + 2).map(|s| s.trim()).unwrap_or("").to_string();
            (numero, assunto)
        }
        None => (String::new(), linhas.first().unwrap_or(&"").to_string()),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pack = std::env::args().nth(1).expect("uso: ... -- <caminho do pack pareceres.json>");
    let cache = std::env::var("EMBED_CACHE_DIR").expect("EMBED_CACHE_DIR ausente");

    eprintln!("🧠 carregando embedder de {cache} ...");
    let embedder = Embedder::new(PathBuf::from(cache), 4)?;
    eprintln!("📦 carregando pack {pack} ...");
    let store: ReadStore<String> = ReadStore::load(&pack)?;
    eprintln!("   {} registros no pack\n", store.len());

    let perguntas = [
        "Como funciona a substituição tributária de ICMS nas operações com pneus?",
        "Incide ICMS na transferência de mercadorias entre estabelecimentos da mesma empresa?",
        "Diferencial de alíquota do ICMS na venda para consumidor final não contribuinte de outro estado",
        "Aproveitamento de crédito de ICMS na aquisição de energia elétrica pela indústria",
        "Tratamento tributário do ICMS na importação de mercadoria por conta e ordem de terceiro",
    ];

    for q in perguntas {
        let emb = embedder.embed_dense(vec![q.to_string()])?.into_iter().next().unwrap();
        let hits = store.query_scored(&emb, 5);
        println!("❓ {q}");
        for (rank, (doc, score)) in hits.iter().enumerate() {
            let (numero, assunto) = resumo_doc(doc);
            let assunto_curto: String = assunto.chars().take(90).collect();
            println!("   {}. [{:.3}] {}  —  {}", rank + 1, score, numero, assunto_curto);
        }
        println!();
    }
    Ok(())
}
