//! In-process embeddings via `fastembed` (BGE-M3, ONNX) — the single encoder used by BOTH the
//! `update` mode (documents) and the `server` mode (the user question). Same code, same model,
//! same `max_length`, so the cosine space is shared by construction.
//!
//! The model is held behind a `Mutex` because `TextEmbedding::embed` takes `&mut self`. All
//! methods are **blocking** and CPU-bound; async callers wrap each call in
//! `tokio::task::spawn_blocking`. Only the dense vector is used (`Vec<Vec<f32>>`).
//!
//! **O modelo é o BGE-M3 em FP32** (`BAAI/bge-m3`, vetor denso por pooling CLS, normalizado), e não mais o
//! INT8 (`gpahal/bge-m3-onnx-int8`) usado até 11/2026 — decisão de 07/11/2026,
//! `docs/Estudo-fastembed-update-v2.md` §Resultado. O INT8 mudava de vetor com a versão do ONNX Runtime
//! (cosseno ~0,986 entre o 1.24 e o 1.31 para o mesmo texto); o FP32 dá cosseno 1,0000000 entre os dois, e
//! na busca é igual ou um pouco melhor (acerto em 1º de 91,2% contra 90,6% nas 2.056 perguntas das FAQs do
//! RS). Custa ~2,3× o tempo de vetorização, ~+0,9 GB de memória e ~2,3 GB de modelo.
//!
//! Embedding strategy: we embed a *key* — não o documento inteiro —, mas a key deixou de ser
//! sempre curta: desde a TAREFA-FAQ-PR a das FAQs carrega a RESPOSTA junto (assunto + `P:` + `R:`).
//! Por isso o `max_length` é o teto do próprio modelo ([`EMBED_MAX_TOKENS`]), e não um valor
//! dimensionado à key: cortar por baixo dos panos é o que não pode acontecer. Desde a TAREFA-TETO
//! ninguém chega perto dele: a key é cortada em `TETO_TEXT_TO_EMBED` caracteres antes de chegar
//! aqui, e a mais longa das 25.381 do `rs` conta 2.769 tokens.

use std::path::PathBuf;
use std::sync::Mutex;

use fastembed::{AutoDevicePolicy, EmbeddingModel, TextEmbedding, TextInitOptions};

use crate::error::{Error, Result};

/// BGE-M3 dense width. A convenience for callers/asserts — the vector store itself is
/// dimensionless and stores whatever width the embedder emits.
pub const EMBED_DIM: usize = 1024;

/// Teto de tokens do BGE-M3 — e o `max_length` com que o encoder é construído. Texto acima disto é
/// truncado PELO TOKENIZER (o excedente não chega ao modelo). Era 512 até a TAREFA-FAQ-PR, quando a
/// key das FAQs passou a incluir a resposta — com 512 a maioria das respostas seria cortada em
/// silêncio. Desde a TAREFA-TETO este caminho é inalcançável por construção: o corte em caracteres
/// acontece antes, e o teto que vale na prática é o `TETO_TEXT_TO_EMBED`, não este.
pub const EMBED_MAX_TOKENS: usize = 8192;

/// Quantos textos vão por chamada ao `fastembed` em [`Embedder::embed_dense`]. **Teto de memória,
/// não de lote**: o `batch_size` do modelo continua 1, e fatiar não altera nenhum vetor.
///
/// O que a fatia limita é a saída bruta que o fastembed guarda até o fim da chamada, antes do pooling: o
/// `last_hidden_state`, 1024 floats **por token** de cada texto (no INT8 de antes, multi-vetor, era o
/// `colbert`, da mesma escala — ~1,4 MB por documento no acervo do TARF, ~31 GB numa coleção inteira de uma
/// vez). Com 256, o transitório fica na casa de centenas de MB.
///
/// O valor não é crítico: qualquer coisa entre dezenas e alguns milhares troca memória por um número
/// irrelevante de chamadas a mais. 256 dá folga confortável mesmo se um lote inteiro bater no teto de
/// 8.192 tokens (256 × 8.192 × 1024 × 4 B ≈ 8,6 GB no pior caso teórico, que o corpus real não tem).
const FATIA: usize = 256;

pub struct Embedder {
    inner: Mutex<TextEmbedding>,
}

impl Embedder {
    /// Build once at startup. Blocking and slow: loads (and on first run downloads from Hugging
    /// Face into `cache_dir`) the BGE-M3 FP32 ONNX model (~2,3 GB).
    ///
    /// **Sempre na CPU (`AutoDevicePolicy::PreferCPU`) — contrato de reprodutibilidade.** Sem a política, o
    /// ONNX Runtime escolhe o dispositivo sozinho (`MaxEfficiency`: prefere NPU e, no macOS, registra sempre o
    /// CoreML), e o mesmo texto sairia com outro vetor numa máquina com outro hardware. Documentos e pergunta têm
    /// de sair do mesmo cálculo — a mesma razão do `batch_size = 1` e do lockfile único. Rodar em GPU/NPU é uma
    /// mudança de propósito, e exige revetorizar o acervo inteiro.
    pub fn new(cache_dir: PathBuf, threads: usize) -> Result<Self> {
        let opts = TextInitOptions::new(EmbeddingModel::BGEM3)
            .with_max_length(EMBED_MAX_TOKENS) // o teto do modelo — ver a const
            .with_intra_threads(threads)
            .with_cache_dir(cache_dir)
            .with_auto_device(AutoDevicePolicy::PreferCPU);
        let model = TextEmbedding::try_new(opts)?; // fastembed::Error -> crate::Error via #[from]
        Ok(Self {
            inner: Mutex::new(model),
        })
    }

    /// Dense vectors for a batch of texts (Phase 1). Blocking — call via `spawn_blocking`.
    ///
    /// **`batch_size = 1`.** No INT8 de antes ele era obrigatório: com o lote automático, o fastembed faz
    /// padding ao maior texto do lote, e a quantização dinâmica deixava o padding VAZAR para o vetor — o mesmo
    /// texto em lotes diferentes saía com cosseno ~0,98 em vez de 1,00 (medido — ver `testes_ordem`). No FP32 o
    /// lote não muda o vetor (07/11/2026: 23 textos em lote = um por um, bit a bit), mas o `Some(1)` fica: é mais
    /// rápido com textos de comprimento desigual (11 s contra 21 s nos mesmos 23) e mantém a garantia sem
    /// depender do modelo. Sem ela, duas consequências graves:
    ///
    /// 1. O servidor embeda a **pergunta sozinha** e o `update` embeda os **documentos em lote** —
    ///    com padding dinâmico, query e documentos sairiam de regimes diferentes do mesmo modelo.
    /// 2. Acrescentar um documento mudaria o vetor dos outros, tornando o índice instável a cada
    ///    coleta incremental.
    ///
    /// Com `Some(1)` cada texto é padded ao próprio tamanho: o mesmo vetor entre execuções, em qualquer ordem
    /// ou composição.
    ///
    /// **A entrada é fatiada em [`FATIA`] textos por chamada — por MEMÓRIA, não por velocidade.**
    /// O fastembed roda o modelo sobre toda a lista que recebe e só no fim faz o pooling: até lá, guarda a
    /// saída bruta de cada texto — o `last_hidden_state`, **um vetor de 1024 floats por token**. Numa chamada
    /// com a coleção inteira, é a coleção inteira em memória antes de virar um vetor por texto. No INT8 de
    /// antes, o que se acumulava era o `colbert`, da mesma escala: medido no acervo do TARF — 22.476
    /// acórdãos, ~337 tokens de média —, ~1,4 MB por documento e **~31 GB** no total, contra 0,09 GB do
    /// `dense`. Era o que explicava o pico de 42,5 GB de RSS do `auli update` (auli_pendencias.md §31.1).
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
            // A saída bruta da fatia morre no fim de cada volta; só o vetor de cada texto fica.
            dense.extend(model.embed(fatia, Some(1))?);
        }
        Ok(dense)
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
    /// O `embed_dense` fatia em [`FATIA`] textos por chamada para não deixar o fastembed acumular a saída
    /// bruta da coleção inteira (o `last_hidden_state`; no INT8 de antes, o `colbert`, ~31 GB no TARF). Isso é seguro porque o `batch_size = 1` interno
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
