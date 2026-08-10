//! Write face of the store. Only `auli update` links this; the server cannot construct it.

use std::path::PathBuf;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::{CollectionData, Error, Record, Result, read_collection_file, write_collection_file};

/// Writes `<name>.json` collection files under a base directory. Each operation reads the current
/// file, applies the change, and rewrites it — the clean-reload pattern (`reset` then `upsert`)
/// used by ingestion. Single-writer by design: there is exactly one `auli update` process.
pub struct Writer {
    base_path: PathBuf,
}

impl Writer {
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
        }
    }

    /// On-disk path for a `<entity>-<kind>` collection name.
    pub fn path_for(&self, name: &str) -> PathBuf {
        self.base_path.join(format!("{name}.json"))
    }

    /// Drop every record in a collection (write an empty file). Used for clean full reloads so
    /// re-ingesting fewer blocks than before leaves no orphan `id-(N+1)..` records.
    pub fn reset<P: Serialize>(&self, name: &str) -> Result<()> {
        write_collection_file(self.path_for(name), &CollectionData::<P>::default())
    }

    /// Upsert whole records (replace by id, else append) and persist. Returns the total record
    /// count after the write.
    ///
    /// **Takes `Record`s, not parallel slices.** It used to take `(ids, embeddings, payloads)` as
    /// three positional slices, guarded by an arity check because a length mismatch would `zip` to
    /// the shortest and drop records in silence. Adding a fourth slice for `key_hash` would widen
    /// exactly the surface that guard existed to cover; carrying assembled records makes the
    /// mismatch **unexpressable** instead of merely detectable. The check doesn't vanish — it moves
    /// to where a mismatch can still be born, in the caller that zips the embedder's output with
    /// its input.
    ///
    /// **Dimension is fixed on first insert.** The collection's width is whatever its first record
    /// established (or, for an empty collection, the first embedding in this batch); any embedding
    /// of a different width is rejected with [`Error::DimensionMismatch`]. This turns the old
    /// silent degrade (a wrong-width vector that `cosine_distance` would later score as max-distance)
    /// into a loud write-time error, so `cosine_distance`'s `2.0` fallback is left to cover only
    /// *legitimate* anti-correlation at query time. It stays a runtime check because it is about
    /// **width**, which no type here constrains.
    pub fn upsert<P>(&self, name: &str, records: Vec<Record<P>>) -> Result<u64>
    where
        P: Serialize + DeserializeOwned,
    {
        let path = self.path_for(name);
        let mut data: CollectionData<P> = read_collection_file(&path)?;

        // Establish the collection's dimension (existing records win; else this batch's first
        // embedding) and reject any divergent width before writing anything.
        if let Some(expected) = data
            .records
            .first()
            .map(|r| r.embedding.len())
            .or_else(|| records.first().map(|r| r.embedding.len()))
            && let Some(bad) = records.iter().find(|r| r.embedding.len() != expected)
        {
            return Err(Error::DimensionMismatch {
                expected,
                got: bad.embedding.len(),
            });
        }

        for rec in records {
            match data.records.iter_mut().find(|r| r.id == rec.id) {
                Some(existing) => *existing = rec,
                None => data.records.push(rec),
            }
        }

        // Ordem canônica por `id` (D-INC-4). Sem isto, uma reescrita total (que chega na ordem
        // alfabética da árvore) e um caminho incremental (que chega na ordem de aplicação)
        // produziriam arquivos DIFERENTES com conteúdo logicamente idêntico — e o teste que a Fase 3
        // depende, "incremental == do zero, byte a byte", não teria como exigir igualdade.
        // Hoje isto é um no-op: `id` é o `doc_path`, que É o nome do arquivo, e a árvore já é lida
        // em ordem de nome. Ordenar aqui é o que mantém isso verdadeiro quando a ordem de chegada
        // deixar de ser essa.
        data.records.sort_by(|a, b| a.id.cmp(&b.id));

        write_collection_file(&path, &data)?;
        Ok(data.records.len() as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Um record de teste, com `key_hash` derivado do payload (o papel que ele tem na vida real:
    /// hash da key que produziu o vetor).
    fn rec(id: &str, embedding: Vec<f32>, payload: &str) -> Record<String> {
        Record {
            id: id.into(),
            embedding,
            payload: payload.into(),
            key_hash: Some(format!("h-{payload}")),
        }
    }

    #[test]
    fn reset_then_upsert_writes_records() {
        let dir = std::env::temp_dir().join(format!("vs-writer-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let w = Writer::new(&dir);

        w.reset::<String>("rs-faqs").unwrap();
        let total = w
            .upsert(
                "rs-faqs",
                vec![
                    rec("a.md", vec![0.1, 0.2], "alpha"),
                    rec("b.md", vec![0.3, 0.4], "beta"),
                ],
            )
            .unwrap();
        assert_eq!(total, 2);

        // Re-load and confirm the file round-trips with the `document` key.
        let data: CollectionData<String> = read_collection_file(w.path_for("rs-faqs")).unwrap();
        assert_eq!(data.records.len(), 2);
        assert_eq!(data.records[0].id, "a.md");
        assert_eq!(data.records[1].payload, "beta");
        assert_eq!(data.records[1].key_hash.as_deref(), Some("h-beta"));

        // Upsert by existing id replaces in place (no duplicate, count stable).
        let total = w
            .upsert("rs-faqs", vec![rec("a.md", vec![9.0, 9.0], "ALPHA")])
            .unwrap();
        assert_eq!(total, 2);
        let data: CollectionData<String> = read_collection_file(w.path_for("rs-faqs")).unwrap();
        assert_eq!(data.records[0].payload, "ALPHA");
        assert_eq!(data.records[0].key_hash.as_deref(), Some("h-ALPHA"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn escrita_ordena_por_id_independente_da_ordem_de_chegada() {
        // D-INC-4: a ordem do arquivo é canônica, não a ordem em que os records foram aplicados —
        // é o que permite exigir "incremental == do zero, byte a byte" na Fase 3.
        let dir = std::env::temp_dir().join(format!("vs-ordem-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let w = Writer::new(&dir);
        w.reset::<String>("c").unwrap();

        w.upsert(
            "c",
            vec![
                rec("c.md", vec![1.0], "c"),
                rec("a.md", vec![1.0], "a"),
                rec("b.md", vec![1.0], "b"),
            ],
        )
        .unwrap();
        let data: CollectionData<String> = read_collection_file(w.path_for("c")).unwrap();
        let ids: Vec<&str> = data.records.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, vec!["a.md", "b.md", "c.md"]);

        // Um record novo inserido depois entra no LUGAR dele, não no fim.
        w.upsert("c", vec![rec("ab.md", vec![1.0], "ab")]).unwrap();
        let data: CollectionData<String> = read_collection_file(w.path_for("c")).unwrap();
        let ids: Vec<&str> = data.records.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, vec!["a.md", "ab.md", "b.md", "c.md"]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn escrita_e_atomica_e_nao_deixa_tmp_para_tras() {
        // D-INC-5: o arquivo final aparece por `rename`, e o `.tmp` não sobrevive à escrita.
        let dir = std::env::temp_dir().join(format!("vs-atomica-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let w = Writer::new(&dir);
        w.reset::<String>("c").unwrap();
        w.upsert("c", vec![rec("a.md", vec![1.0], "a")]).unwrap();

        assert!(w.path_for("c").exists());
        assert!(
            !dir.join("c.json.tmp").exists(),
            "o temporário tem de sumir no rename"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pack_antigo_sem_key_hash_ainda_carrega() {
        // D-INC-2: `key_hash` é opcional para que um pack do formato 1 desserialize como `None` —
        // que o chamador lê como "não sei ⇒ re-embeda", degradando para o comportamento de hoje.
        let json = r#"{"records":[{"id":"id-1","embedding":[0.1],"document":"velho"}]}"#;
        let data: CollectionData<String> = serde_json::from_str(json).unwrap();
        assert_eq!(data.records[0].key_hash, None);
        // E um record sem hash não emite o campo — o JSON não engorda por causa dele.
        assert!(!serde_json::to_string(&data).unwrap().contains("key_hash"));
    }

    #[test]
    fn upsert_rejects_mismatched_dimension() {
        let dir = std::env::temp_dir().join(format!("vs-dim-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let w = Writer::new(&dir);
        w.reset::<String>("c").unwrap();

        // First insert fixes width = 3.
        w.upsert("c", vec![rec("a.md", vec![1.0, 2.0, 3.0], "a")])
            .unwrap();

        // A later width-2 vector is rejected, and nothing is written.
        let err = w
            .upsert("c", vec![rec("b.md", vec![1.0, 2.0], "b")])
            .unwrap_err();
        assert!(matches!(
            err,
            Error::DimensionMismatch {
                expected: 3,
                got: 2
            }
        ));
        let data: CollectionData<String> = read_collection_file(w.path_for("c")).unwrap();
        assert_eq!(data.records.len(), 1, "rejected batch must not persist");

        // A mismatched width WITHIN the first batch (into an empty collection) is also caught.
        w.reset::<String>("d").unwrap();
        let err = w
            .upsert(
                "d",
                vec![
                    rec("a.md", vec![1.0, 2.0], "a"),
                    rec("b.md", vec![1.0, 2.0, 3.0], "b"),
                ],
            )
            .unwrap_err();
        assert!(matches!(
            err,
            Error::DimensionMismatch {
                expected: 2,
                got: 3
            }
        ));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
