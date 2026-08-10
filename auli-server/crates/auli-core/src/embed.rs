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

/// Quantos textos vão por chamada ao `fastembed` em [`Embedder::embed_dense`]. **Teto de memória,
/// não de lote**: o `batch_size` do modelo continua 1, e fatiar não altera nenhum vetor.
///
/// O que a fatia limita é o `colbert`/`sparse` que o fastembed acumula e nós descartamos — ~1,4 MB
/// por documento no acervo do TARF. Com 256, o transitório fica em ~360 MB em vez dos ~31 GB de uma
/// coleção inteira de uma vez.
///
/// O valor não é crítico: qualquer coisa entre dezenas e alguns milhares troca memória por um número
/// irrelevante de chamadas a mais. 256 dá folga confortável mesmo se um lote inteiro bater no teto de
/// 8.192 tokens (256 × 8.192 × 1024 × 4 B ≈ 8,6 GB no pior caso teórico, que o corpus real não tem).
const FATIA: usize = 256;

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
        Ok(Self {
            inner: Mutex::new(model),
        })
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
    ///
    /// **A entrada é fatiada em [`FATIA`] textos por chamada — por MEMÓRIA, não por velocidade.**
    /// O BGE-M3 é multi-vetor: cada passada devolve `dense`, `sparse` **e** `colbert`, e o fastembed
    /// acumula os três ao longo de toda a lista que recebe. Usamos só o `dense`, mas o descarte só
    /// acontece no fim — então uma chamada com a coleção inteira constrói tudo antes de jogar fora.
    ///
    /// O `colbert` não é um vetor por documento: é **um vetor de 1024 floats por token**
    /// (`[batch, seq_len-1, 1024]`). Medido no acervo do TARF — 22.476 acórdãos, ~337 tokens de média
    /// — dá ~1,4 MB por documento e **~31 GB** no total, contra 0,09 GB do `dense`. É o que explicava
    /// o pico de 42,5 GB de RSS do `auli update` (auli_pendencias.md §31.1).
    ///
    /// Fatiar por fora é **numericamente inócuo**: o `batch_size = 1` interno já processa um texto
    /// por vez, então o tamanho da fatia não toca em nada que a doutrina acima protege. É o que o
    /// teste `fatiar_a_entrada_nao_muda_um_unico_vetor` fixa.
    pub fn embed_dense(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let mut model = self
            .inner
            .lock()
            .map_err(|_| Error::from("embedder mutex poisoned"))?;
        let mut dense = Vec::with_capacity(texts.len());
        for fatia in texts.chunks(FATIA) {
            // `out` morre no fim de cada volta, e com ele o `sparse` e o `colbert` da fatia.
            let out = model.embed(fatia, Some(1))?;
            dense.extend(out.dense);
        }
        Ok(dense)
    }

    /// Quantos tokens deste texto o encoder REALMENTE consome — pelo tokenizer do próprio modelo,
    /// já com a truncagem em [`EMBED_MAX_TOKENS`] aplicada. Por isso o resultado nunca passa do
    /// teto, e **igual ao teto significa que o texto bateu nele** (o excedente foi descartado).
    ///
    /// É o que permite ao `update` avisar quais itens foram cortados sem estimar por chars — a
    /// razão chars/token varia demais entre idiomas para servir de guarda.
    pub fn conta_tokens(&self, texto: &str) -> Result<usize> {
        let model = self
            .inner
            .lock()
            .map_err(|_| Error::from("embedder mutex poisoned"))?;
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

    /// INVARIANTE do fatiamento: partir a entrada em pedaços não pode mudar NADA.
    ///
    /// O `embed_dense` fatia em [`FATIA`] textos por chamada para não deixar o fastembed acumular o
    /// `colbert` da coleção inteira (~31 GB no TARF). Isso é seguro porque o `batch_size = 1` interno
    /// já processa um texto por vez — mas "é seguro porque eu raciocinei" não é garantia, e esta é a
    /// função onde um engano custa re-vetorizar tudo. Aqui a igualdade é medida.
    ///
    /// O teste força fatias de tamanho 1, 2 e 3 sobre 5 textos comparando com uma chamada só, e
    /// exige igualdade BIT A BIT — não cosseno. Se um dia o fatiamento passar a interferir, a
    /// primeira coisa a checar é se o `Some(1)` ainda está lá.
    #[test]
    #[ignore = "carrega o modelo BGE-M3 (lento); rode com --ignored"]
    fn fatiar_a_entrada_nao_muda_um_unico_vetor() {
        let cache = std::env::var("EMBED_CACHE_DIR").unwrap_or_else(|_| "../models".into());
        let e = Embedder::new(cache.into(), 4).expect("embedder");
        // Comprimentos deliberadamente desiguais: é a desigualdade que fazia o padding vazar.
        let textos: Vec<String> = vec![
            "ICMS".into(),
            "PARECER Nº 42 — crédito de energia elétrica no processo produtivo".into(),
            "a".repeat(3000),
            "IPVA – isenção para pessoa com deficiência".into(),
            "ACÓRDÃO 118/22 — redução da base de cálculo, café torrado e moído".into(),
        ];

        let inteiro = e.embed_dense(textos.clone()).unwrap();

        for fatia in [1usize, 2, 3] {
            let mut picado: Vec<Vec<f32>> = Vec::new();
            for pedaco in textos.chunks(fatia) {
                picado.extend(e.embed_dense(pedaco.to_vec()).unwrap());
            }
            assert_eq!(picado.len(), inteiro.len(), "fatia {fatia}: contagem mudou");
            for (i, (a, b)) in inteiro.iter().zip(&picado).enumerate() {
                assert_eq!(
                    a, b,
                    "fatia {fatia}, texto {i}: o vetor mudou ao fatiar a entrada. \
                     O `batch_size = 1` do `embed_dense` ainda está lá?"
                );
            }
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
