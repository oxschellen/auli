//! A/B de retrieval entre DOIS packs de faqs — o gate de promoção da TAREFA-FAQ-PR (P5).
//!
//! Lê os packs **direto pelo `vector-store`**, sem passar pela validação de manifesto do boot: os
//! dois carimbam estratégias diferentes por construção (é essa a mudança sendo medida), e a
//! validação — que existe para impedir servir pack de outra era — recusaria o lado "antes".
//!
//! A query é embedada UMA vez e comparada contra os dois packs. Isso é correto mesmo com o
//! `max_length` tendo mudado (512 → 8192): o teto só trunca, e nenhuma consulta real chega perto
//! dele — o vetor da query é idêntico nos dois regimes.
//!
//! Consultas em `data/eval/faq_pr_consultas.txt`, uma por linha, campos separados por TAB:
//!   `consulta`                     → só entra na impressão lado a lado (julgamento humano)
//!   `consulta<TAB>frag[<TAB>frag]` → também entra no hit@1 / hit@5
//!
//! **Vários fragmentos = várias respostas ACEITÁVEIS** (basta uma casar). Uma pergunta de
//! contribuinte costuma ter mais de um item que a responde bem; rotular com um só produz falso
//! negativo — foi o que aconteceu na primeira rodada, em que o pack novo trouxe em 1º um item
//! correto que o rótulo não previa e apareceu como regressão.
//!
//! Rodar:
//!   AULI_AB_ANTES=/tmp/faqpr/rs-faqs-ANTES.json \
//!   AULI_AB_NOVO=../../data/rs/packs/rs-faqs.json \
//!   AULI_AB_CONSULTAS=../../data/eval/faq_pr_consultas.txt \
//!   EMBED_CACHE_DIR=../../models \
//!   cargo test -p auli-cli --release --test ab_faq_pr -- --nocapture --ignored

use auli_core::embed::Embedder;
use vector_store::{cosine_distance, read_collection_file};

const TOP: usize = 5;

/// Uma consulta do arquivo de avaliação; `fragmentos` vazio ⇒ só julgamento humano.
struct Consulta {
    texto: String,
    fragmentos: Vec<String>,
}

fn ler_consultas(caminho: &str) -> Vec<Consulta> {
    std::fs::read_to_string(caminho)
        .unwrap_or_else(|e| panic!("consultas em `{caminho}`: {e}"))
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let mut campos = l.split('\t').map(str::trim).filter(|c| !c.is_empty());
            Consulta {
                texto: campos.next().unwrap_or_default().to_string(),
                fragmentos: campos.map(str::to_string).collect(),
            }
        })
        .collect()
}

/// Os TOP documentos mais próximos da query, do mais perto ao mais longe.
fn top(pack: &[vector_store::Record<String>], q: &[f32]) -> Vec<(f32, String)> {
    let mut d: Vec<(f32, String)> = pack
        .iter()
        .map(|r| (cosine_distance(&r.embedding, q), r.payload.clone()))
        .collect();
    d.sort_by(|a, b| a.0.total_cmp(&b.0));
    d.truncate(TOP);
    d
}

/// Posição (1-based) do primeiro documento que contenha QUALQUER um dos fragmentos aceitáveis.
fn posicao_do_acerto(hits: &[(f32, String)], fragmentos: &[String]) -> Option<usize> {
    let alvos: Vec<String> = fragmentos.iter().map(|f| f.to_lowercase()).collect();
    hits.iter()
        .position(|(_, doc)| {
            let d = doc.to_lowercase();
            alvos.iter().any(|a| d.contains(a))
        })
        .map(|i| i + 1)
}

/// Uma linha de resumo por pack: quantos hit@1 e hit@5 entre as consultas rotuladas.
#[derive(Default)]
struct Placar {
    hit1: usize,
    hit5: usize,
}

#[test]
#[ignore = "precisa dos dois packs + modelo; rode explicitamente com AULI_AB_* setados"]
fn ab_faq_pr() {
    let (Ok(antes_p), Ok(novo_p)) = (
        std::env::var("AULI_AB_ANTES"),
        std::env::var("AULI_AB_NOVO"),
    ) else {
        eprintln!("AULI_AB_ANTES/AULI_AB_NOVO não setados — pulando");
        return;
    };
    let consultas_p = std::env::var("AULI_AB_CONSULTAS")
        .unwrap_or_else(|_| "../../data/eval/faq_pr_consultas.txt".into());
    let cache = std::env::var("EMBED_CACHE_DIR").unwrap_or_else(|_| "../../models".into());

    let antes = read_collection_file::<String>(&antes_p)
        .expect("pack ANTES")
        .records;
    let novo = read_collection_file::<String>(&novo_p)
        .expect("pack NOVO")
        .records;
    assert!(!antes.is_empty() && !novo.is_empty(), "pack vazio");
    let consultas = ler_consultas(&consultas_p);
    println!(
        "\nA/B — ANTES: {} registros ({antes_p})\n      NOVO : {} registros ({novo_p})\n      {} consultas ({} rotuladas)\n",
        antes.len(),
        novo.len(),
        consultas.len(),
        consultas
            .iter()
            .filter(|c| !c.fragmentos.is_empty())
            .count()
    );

    let embedder = Embedder::new(cache.into(), 8).expect("embedder");
    let (mut pa, mut pn) = (Placar::default(), Placar::default());

    for c in &consultas {
        let q = embedder.embed_dense(vec![c.texto.clone()]).expect("embed")[0].clone();
        let (ta, tn) = (top(&antes, &q), top(&novo, &q));

        println!("═══ {}", c.texto);
        for (rotulo, hits) in [("ANTES", &ta), ("NOVO ", &tn)] {
            for (i, (dist, doc)) in hits.iter().enumerate() {
                // Pack v2 (B4): o payload é o `DocumentoPack` em JSON. `trilha / titulo` é o que
                // identifica o item aqui — o mesmo par que a 2ª e a 3ª linha do bloco antigo
                // mostravam, agora lido do campo em vez de fatiado do texto.
                let titulo: String =
                    match serde_json::from_str::<auli_contract::DocumentoPack>(doc) {
                        Ok(p) => format!("{} / {}", p.trilha, p.titulo),
                        Err(_) => doc.to_string(),
                    }
                    .chars()
                    .take(120)
                    .collect();
                println!("  {rotulo} {}. {:.4}  {titulo}", i + 1, dist);
            }
        }
        if !c.fragmentos.is_empty() {
            let (a, n) = (
                posicao_do_acerto(&ta, &c.fragmentos),
                posicao_do_acerto(&tn, &c.fragmentos),
            );
            for (pos, placar) in [(a, &mut pa), (n, &mut pn)] {
                if let Some(p) = pos {
                    placar.hit5 += 1;
                    if p == 1 {
                        placar.hit1 += 1;
                    }
                }
            }
            println!(
                "  ▸ alvo {:?} → ANTES: {}  NOVO: {}",
                c.fragmentos,
                a.map_or("fora do top-5".into(), |p| format!("posição {p}")),
                n.map_or("fora do top-5".into(), |p| format!("posição {p}"))
            );
        }
        println!();
    }

    let rotuladas = consultas
        .iter()
        .filter(|c| !c.fragmentos.is_empty())
        .count();
    println!("──────────────── PLACAR (sobre {rotuladas} consultas rotuladas)");
    println!(
        "  ANTES  hit@1 {}/{rotuladas}   hit@5 {}/{rotuladas}",
        pa.hit1, pa.hit5
    );
    println!(
        "  NOVO   hit@1 {}/{rotuladas}   hit@5 {}/{rotuladas}",
        pn.hit1, pn.hit5
    );
    println!("  Gate (D-FAQPR / P5): promover só se NOVO ≥ ANTES nos dois. As consultas sem");
    println!("  fragmento acima são julgamento humano — leia os top-5 lado a lado.\n");
}
