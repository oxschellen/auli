//! O componente **esparso (lexical) do BGE-M3** — por enquanto, só para a Fase 0 do
//! `docs/Estudo-rag-dense.md` (medição). Nenhum caminho de produção chama este módulo: o `update`, o
//! motor e o servidor seguem só com o denso do [`Embedder`](crate::embed::Embedder).
//!
//! É um tipo à parte do `Embedder` de propósito (D-HIB-1(a) do estudo): uma segunda sessão do
//! `fastembed`, `SparseTextEmbedding` com `SparseModel::BGEM3`, sobre **os mesmos arquivos** do denso
//! (`BAAI/bge-m3`, `onnx/model.onnx`) e com **as mesmas opções**: `max_length` no teto do modelo, CPU,
//! `batch_size = 1` e o mesmo fatiamento. Os padrões do `fastembed` para o esparso são outros (512
//! tokens e lote de 256) — por isso cada opção é repetida aqui, e não herdada.
//!
//! O que o `fastembed 7.6.1` faz por baixo: roda o modelo, pega a **primeira saída** do ONNX (no FP32
//! é `token_embeddings`, um vetor por token; a segunda é `sentence_embedding`, de onde o denso sai) e,
//! por token que não seja especial (ids 0–3), calcula `max(0, h · w + b)` com a camada
//! `sparse_linear` do BGE-M3. Por id de token fica o maior peso.

use std::path::PathBuf;
use std::sync::Mutex;

use fastembed::{AutoDevicePolicy, SparseInitOptions, SparseModel, SparseTextEmbedding};
use serde::{Deserialize, Serialize};

use crate::embed::{EMBED_MAX_TOKENS, FATIA};
use crate::error::{Error, Result};

/// Um vetor esparso: ids de token do vocabulário do modelo, em ordem crescente, e o peso de cada um.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VetorEsparso {
    pub indices: Vec<u32>,
    pub valores: Vec<f32>,
}

impl VetorEsparso {
    /// A pontuação do próprio BGE-M3: a soma, sobre os ids de token em comum, de
    /// `peso_daqui × peso_dali`. **Maior é mais próximo** — o contrário da distância cosseno do denso.
    ///
    /// Percorre os dois em ordem crescente de id (os dois chegam ordenados do `fastembed`), então a
    /// ordem da soma é fixa: o mesmo par dá o mesmo número bit a bit.
    pub fn pontuar(&self, outro: &VetorEsparso) -> f32 {
        let (mut i, mut j, mut soma) = (0, 0, 0.0f32);
        while i < self.indices.len() && j < outro.indices.len() {
            match self.indices[i].cmp(&outro.indices[j]) {
                std::cmp::Ordering::Less => i += 1,
                std::cmp::Ordering::Greater => j += 1,
                std::cmp::Ordering::Equal => {
                    soma += self.valores[i] * outro.valores[j];
                    i += 1;
                    j += 1;
                }
            }
        }
        soma
    }
}

pub struct EmbedderEsparso {
    inner: Mutex<SparseTextEmbedding>,
}

impl EmbedderEsparso {
    /// Carrega a segunda sessão. Mesmo `cache_dir` do denso: o modelo já está lá, não há download.
    pub fn new(cache_dir: PathBuf, threads: usize) -> Result<Self> {
        let opts = SparseInitOptions::new(SparseModel::BGEM3)
            .with_max_length(EMBED_MAX_TOKENS)
            .with_intra_threads(threads)
            .with_cache_dir(cache_dir)
            .with_auto_device(AutoDevicePolicy::PreferCPU);
        let model = SparseTextEmbedding::try_new(opts)?;
        Ok(Self {
            inner: Mutex::new(model),
        })
    }

    /// Os vetores esparsos de uma lista de textos, com `batch_size = 1` e fatiada em [`FATIA`], como o
    /// `embed_dense`. Blocking.
    pub fn embed_sparse(&self, texts: Vec<String>) -> Result<Vec<VetorEsparso>> {
        let mut model = self
            .inner
            .lock()
            .map_err(|_| Error::from("embedder esparso: mutex envenenado"))?;
        let mut saida = Vec::with_capacity(texts.len());
        for fatia in texts.chunks(FATIA) {
            for e in model.embed(fatia, Some(1))? {
                saida.push(VetorEsparso {
                    indices: e.indices.into_iter().map(|i| i as u32).collect(),
                    valores: e.values,
                });
            }
        }
        Ok(saida)
    }

    /// Como o tokenizer do modelo parte um texto: `(id, peça)` por token, sem os especiais. É o que
    /// mostra se caixa e acento mudam os ids (item M4 do estudo).
    pub fn tokens(&self, texto: &str) -> Result<Vec<(u32, String)>> {
        let model = self
            .inner
            .lock()
            .map_err(|_| Error::from("embedder esparso: mutex envenenado"))?;
        let enc = model
            .tokenizer
            .encode(texto, false)
            .map_err(|e| Error::from(format!("tokenizer: {e}")))?;
        Ok(enc
            .get_ids()
            .iter()
            .copied()
            .zip(enc.get_tokens().iter().cloned())
            .collect())
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn v(indices: &[u32], valores: &[f32]) -> VetorEsparso {
        VetorEsparso {
            indices: indices.to_vec(),
            valores: valores.to_vec(),
        }
    }

    #[test]
    fn pontuar_soma_so_os_tokens_em_comum() {
        let a = v(&[1, 5, 9], &[0.5, 0.25, 2.0]);
        let b = v(&[5, 7, 9, 12], &[4.0, 3.0, 0.5, 1.0]);
        // 5: 0,25 × 4 = 1; 9: 2 × 0,5 = 1.
        assert_eq!(a.pontuar(&b), 2.0);
        assert_eq!(b.pontuar(&a), 2.0);
        assert_eq!(a.pontuar(&v(&[2, 3], &[1.0, 1.0])), 0.0);
        assert_eq!(a.pontuar(&v(&[], &[])), 0.0);
    }

    fn esparso() -> EmbedderEsparso {
        let cache = std::env::var("EMBED_CACHE_DIR").unwrap_or_else(|_| "../models".into());
        EmbedderEsparso::new(cache.into(), 4).expect("embedder esparso")
    }

    /// M6 do estudo, o par de `embedding_independe_da_ordem_e_da_composicao_do_lote`: o vetor de um
    /// texto não depende de quem mais está na chamada, nem da posição. Igualdade bit a bit.
    #[test]
    #[ignore = "carrega o modelo BGE-M3 (lento); rode com --ignored"]
    fn esparso_independe_da_ordem_e_da_composicao_do_lote() {
        let e = esparso();
        let a = "PARECER Nº 1\nICMS – crédito de energia elétrica";
        let b = "PARECER Nº 2\nICMS – substituição tributária de medicamentos";
        let c = "PARECER Nº 3\nIPVA – isenção para pessoa com deficiência";

        let v_ab = e.embed_sparse(vec![a.into(), b.into()]).unwrap();
        let v_ba = e.embed_sparse(vec![b.into(), a.into()]).unwrap();
        let v_abc = e.embed_sparse(vec![a.into(), b.into(), c.into()]).unwrap();
        let v_so_a = e.embed_sparse(vec![a.into()]).unwrap();

        assert!(
            !v_ab[0].indices.is_empty(),
            "esparso vazio para um texto real"
        );
        assert_eq!(v_ab[0], v_ba[1], "posição na chamada mudou o vetor");
        assert_eq!(v_ab[0], v_abc[0], "chamada maior mudou o vetor");
        assert_eq!(v_ab[0], v_so_a[0], "sozinho × acompanhado mudou o vetor");
    }

    /// M6: fatiar a entrada não muda nada — o par de `fatiar_a_entrada_nao_muda_um_unico_vetor`.
    #[test]
    #[ignore = "carrega o modelo BGE-M3 (lento); rode com --ignored"]
    fn fatiar_a_entrada_nao_muda_um_unico_esparso() {
        let e = esparso();
        let textos: Vec<String> = vec![
            "ICMS".into(),
            "PARECER Nº 42 — crédito de energia elétrica no processo produtivo".into(),
            "a".repeat(3000),
            "IPVA – isenção para pessoa com deficiência".into(),
            "ACÓRDÃO 118/22 — redução da base de cálculo, café torrado e moído".into(),
        ];
        let inteiro = e.embed_sparse(textos.clone()).unwrap();
        for fatia in [1usize, 2, 3] {
            let mut picado = Vec::new();
            for pedaco in textos.chunks(fatia) {
                picado.extend(e.embed_sparse(pedaco.to_vec()).unwrap());
            }
            assert_eq!(picado, inteiro, "fatia {fatia}: o esparso mudou ao fatiar");
        }
    }

    /// M6: a mesma entrada, duas vezes, dá o mesmo esparso bit a bit.
    #[test]
    #[ignore = "carrega o modelo BGE-M3 (lento); rode com --ignored"]
    fn esparso_e_deterministico_entre_chamadas() {
        let e = esparso();
        let t = vec!["PARECER Nº 7\nICMS – diferencial de alíquota".to_string()];
        assert_eq!(
            e.embed_sparse(t.clone()).unwrap(),
            e.embed_sparse(t).unwrap()
        );
    }
}
