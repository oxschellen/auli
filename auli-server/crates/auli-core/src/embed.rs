//! In-process embeddings via `fastembed` (BGE-M3, ONNX) — the single encoder used by BOTH the
//! `update` mode (documents) and the `server` mode (the user question). Same code, same model,
//! same `max_length`, so the cosine space is shared by construction.
//!
//! The model is held behind a `Mutex` because `Bgem3Embedding::embed` takes `&mut self`. All
//! methods are **blocking** and CPU-bound; async callers wrap each call in
//! `tokio::task::spawn_blocking`. Phase 1 uses only the dense output, preserving the old
//! `Vec<Vec<f32>>` contract.
//!
//! Embedding strategy: we embed a *key* — não o documento inteiro —, mas a key deixou de ser
//! sempre curta: desde a TAREFA-FAQ-PR a das FAQs carrega a RESPOSTA junto (assunto + `P:` + `R:`).
//! Por isso o `max_length` é o teto do próprio modelo ([`EMBED_MAX_TOKENS`]), e não um valor
//! dimensionado à key: cortar por baixo dos panos é o que não pode acontecer. Quem estoura o teto é
//! detectável por [`Embedder::conta_tokens`] — o `update` avisa, item a item.

use std::path::PathBuf;
use std::sync::Mutex;

use fastembed::{Bgem3Embedding, Bgem3InitOptions, Bgem3Model};

use crate::error::{Error, Result};

/// BGE-M3 dense width. A convenience for callers/asserts — the vector store itself is
/// dimensionless and stores whatever width the embedder emits.
pub const EMBED_DIM: usize = 1024;

/// Teto de tokens do BGE-M3 — e o `max_length` com que o encoder é construído. Texto acima disto é
/// truncado PELO TOKENIZER (o excedente não chega ao modelo): é o comportamento aceito, com aviso no
/// `update`, não um erro. Era 512 até a TAREFA-FAQ-PR, quando a key das FAQs passou a incluir a
/// resposta — com 512 a maioria das respostas seria cortada em silêncio.
pub const EMBED_MAX_TOKENS: usize = 8192;

pub struct Embedder {
    inner: Mutex<Bgem3Embedding>,
}

impl Embedder {
    /// Build once at startup. Blocking and slow: loads (and on first run downloads from Hugging
    /// Face into `cache_dir`) the BGE-M3 INT8 ONNX model.
    pub fn new(cache_dir: PathBuf, threads: usize) -> Result<Self> {
        let opts = Bgem3InitOptions::new(Bgem3Model::BGEM3Q)
            .with_max_length(EMBED_MAX_TOKENS) // o teto do modelo — ver a const
            .with_intra_threads(threads)
            .with_cache_dir(cache_dir);
        let model = Bgem3Embedding::try_new(opts)?; // anyhow::Error -> crate::Error via #[from]
        Ok(Self { inner: Mutex::new(model) })
    }

    /// Dense vectors for a batch of texts (Phase 1). Blocking — call via `spawn_blocking`.
    ///
    /// **`batch_size = 1` é obrigatório, não uma escolha de performance.** Com o lote automático, o
    /// fastembed faz padding ao maior texto do lote e o padding VAZA para o vetor agrupado: o mesmo
    /// texto embedado em lotes diferentes sai com cosseno ~0,98 em vez de 1,00 (medido — ver
    /// `testes_ordem`). Duas consequências graves:
    ///
    /// 1. O servidor embeda a **pergunta sozinha** e o `update` embeda os **documentos em lote** —
    ///    com padding dinâmico, query e documentos sairiam de regimes diferentes do mesmo modelo.
    /// 2. Acrescentar um documento mudaria o vetor dos outros, tornando o índice instável a cada
    ///    coleta incremental.
    ///
    /// Com `Some(1)` cada texto é padded ao próprio tamanho: cosseno 1,000000 entre execuções, em
    /// qualquer ordem ou composição. E sai **~2,9× mais rápido** na prática, porque o padding ao maior
    /// do lote desperdiçava compute com textos de comprimento muito desigual.
    pub fn embed_dense(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let mut model = self.inner.lock().map_err(|_| Error::from("embedder mutex poisoned"))?;
        let out = model.embed(&texts, Some(1))?;
        Ok(out.dense)
    }

    /// Quantos tokens deste texto o encoder REALMENTE consome — pelo tokenizer do próprio modelo,
    /// já com a truncagem em [`EMBED_MAX_TOKENS`] aplicada. Por isso o resultado nunca passa do
    /// teto, e **igual ao teto significa que o texto bateu nele** (o excedente foi descartado).
    ///
    /// É o que permite ao `update` avisar quais itens foram cortados sem estimar por chars — a
    /// razão chars/token varia demais entre idiomas para servir de guarda.
    pub fn conta_tokens(&self, texto: &str) -> Result<usize> {
        let model = self.inner.lock().map_err(|_| Error::from("embedder mutex poisoned"))?;
        let enc = model
            .tokenizer
            .encode(texto, true)
            .map_err(|e| Error::from(format!("tokenizer: {e}")))?;
        Ok(enc.len())
    }
}

#[cfg(test)]
mod testes_ordem {
    use super::*;

    fn cos(x: &[f32], y: &[f32]) -> f32 {
        let (mut d, mut nx, mut ny) = (0.0f32, 0.0f32, 0.0f32);
        for (p, q) in x.iter().zip(y) {
            d += p * q;
            nx += p * p;
            ny += q * q;
        }
        d / (nx.sqrt() * ny.sqrt())
    }

    /// INVARIANTE: o vetor de um texto não pode depender de quem mais está no lote, nem da posição.
    ///
    /// Antes do `batch_size = 1` isto FALHAVA: o mesmo texto saía com cosseno 0,978 entre lotes
    /// diferentes (para escala, dois textos distintos ficam em ~0,65 — ou seja, o ruído do lote
    /// comia ~2% da faixa angular útil e trocava documentos na fronteira do top-k).
    #[test]
    #[ignore = "carrega o modelo BGE-M3 (lento); rode com --ignored"]
    fn embedding_independe_da_ordem_e_da_composicao_do_lote() {
        let cache = std::env::var("EMBED_CACHE_DIR").unwrap_or_else(|_| "../models".into());
        let e = Embedder::new(cache.into(), 4).expect("embedder");
        let a = "PARECER Nº 1\nICMS – crédito de energia elétrica";
        let b = "PARECER Nº 2\nICMS – substituição tributária de medicamentos";
        let c = "PARECER Nº 3\nIPVA – isenção para pessoa com deficiência";

        let v_ab = e.embed_dense(vec![a.into(), b.into()]).unwrap();
        let v_ba = e.embed_dense(vec![b.into(), a.into()]).unwrap();
        let v_abc = e.embed_dense(vec![a.into(), b.into(), c.into()]).unwrap();
        let v_so_a = e.embed_dense(vec![a.into()]).unwrap();

        // Tolerância apertada de propósito: queremos identidade, não "parecido".
        const TOL: f32 = 1e-6;
        for (rotulo, outro) in [
            ("posição no lote", &v_ba[1]),
            ("lote maior", &v_abc[0]),
            ("sozinho vs em lote", &v_so_a[0]),
        ] {
            let s = cos(&v_ab[0], outro);
            assert!(
                (1.0 - s).abs() < TOL,
                "{rotulo}: cosseno {s:.6} ≠ 1 — o vetor mudou com a composição do lote. \
                 Se `embed_dense` deixar de usar batch_size=1, o padding volta a vazar."
            );
        }
    }

    /// O detector de truncamento: `conta_tokens` bate no teto exatamente quando o texto passa dele.
    /// É o que sustenta o aviso do `update` — se um dia o `max_length` deixar de ser aplicado ao
    /// tokenizer, o aviso viraria silêncio e este teste falha.
    #[test]
    #[ignore = "carrega o modelo BGE-M3 (lento); rode com --ignored"]
    fn conta_tokens_bate_no_teto_so_quando_o_texto_estoura() {
        let cache = std::env::var("EMBED_CACHE_DIR").unwrap_or_else(|_| "../models".into());
        let e = Embedder::new(cache.into(), 4).expect("embedder");
        let curto = e.conta_tokens("Como emitir a guia de IPVA?").unwrap();
        assert!(curto > 0 && curto < 32, "texto curto: {curto} tokens");
        // Bem acima do teto: o excedente é descartado, então a contagem PARA no teto.
        let gigante = "palavra ".repeat(EMBED_MAX_TOKENS);
        assert_eq!(e.conta_tokens(&gigante).unwrap(), EMBED_MAX_TOKENS);
    }

    /// Guarda o outro lado do invariante: a mesma entrada, embedada duas vezes, dá o MESMO vetor
    /// bit a bit. É o que torna os packs reprodutíveis.
    #[test]
    #[ignore = "carrega o modelo BGE-M3 (lento); rode com --ignored"]
    fn embedding_e_deterministico_entre_chamadas() {
        let cache = std::env::var("EMBED_CACHE_DIR").unwrap_or_else(|_| "../models".into());
        let e = Embedder::new(cache.into(), 4).expect("embedder");
        let t = vec!["PARECER Nº 7\nICMS – diferencial de alíquota".to_string()];
        assert_eq!(e.embed_dense(t.clone()).unwrap(), e.embed_dense(t).unwrap());
    }
}
