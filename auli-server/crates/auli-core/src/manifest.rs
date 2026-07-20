//! Embedding identity + pack manifest.
//!
//! A manifest pins the *identity* of the embedding space a set of packs was built with: which
//! model, which dimension, and which `STRATEGY_VERSION` (what `auli-contract` / the scraper
//! materialized as `text_to_embed`). `auli update` writes it; `auli server` validates the local identity against
//! it at boot and **refuses to serve** on a mismatch — turning "forgot to re-ingest after changing
//! the strategy" from silent bad retrieval into a loud boot error.
//!
//! This lives in `auli-core`, never in `vector-store`: an agnostic store cannot know what an
//! "embedding model" is. Identity is a domain concept.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::embed::EMBED_DIM;
use crate::error::{Error, Result};

/// Model identifier baked into every manifest. Change this whenever the model or quantization
/// changes (it implies a full re-ingest).
pub const EMBED_MODEL_ID: &str = "bge-m3-q-int8";

/// Bumped whenever what gets embedded changes (the scraper's `text_to_embed` formula / the contract
/// `stored_repr`). A pack built under an old strategy is incompatible with a server running a new one
/// even if the model matches. v2: source is the typed `auli-contract` (was: `portal-*.txt` parsing).
///
/// Regra da sinopse (F5): regenerar sinopses em massa (mudança de `SINOPSE_PROMPT_VERSION` ou do
/// modelo da sinopse + re-geração) muda os textos embedados de `pareceres` ⇒ bump obrigatório aqui.
/// Sinopses novas convivendo com antigas (append-only, sem re-geração) NÃO exigem bump — o embedder
/// é o mesmo.
///
/// v3 (G3): o pack de `pareceres` passou a guardar o payload LEVE (JSON sem corpo) no lugar do bloco
/// pré-renderizado; um servidor G3 lendo pack v2 (bloco gordo) renderizaria lixo. O bump fecha essa
/// porta no boot (`validate_manifest` exige igualdade) — packs pré-G3 são incompatíveis por construção.
///
/// v4: correção do **vazamento de padding** no embedder. Até aqui o `update` embedava os documentos
/// em lote e o fastembed fazia padding ao maior do lote, contaminando o vetor agrupado — o MESMO
/// texto saía com cosseno ~0,98 conforme a companhia (ver `embed::testes_ordem`). A query nunca
/// sofreu disso (o servidor embeda uma pergunta só), então documentos e perguntas viviam em regimes
/// diferentes do mesmo modelo. Com `batch_size = 1` os documentos passam ao regime da query. Os
/// vetores mudam ⇒ **todo pack anterior é incompatível** e precisa ser regerado.
///
/// (Bump aqui, e não em `EMBED_MODEL_ID`: o modelo e a quantização são os mesmos; o que mudou foi
/// COMO ele é invocado. O efeito prático — re-ingestão obrigatória — é idêntico.)
pub const STRATEGY_VERSION: u32 = 4;

/// The triple that must match between the packs and the running server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbedIdentity {
    pub embed_model_id: String,
    pub embed_dim: usize,
    pub strategy_version: u32,
}

/// The local identity of this build. Compared against a pack's manifest at server boot.
pub fn identity() -> EmbedIdentity {
    EmbedIdentity {
        embed_model_id: EMBED_MODEL_ID.to_string(),
        embed_dim: EMBED_DIM,
        strategy_version: STRATEGY_VERSION,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionEntry {
    pub kind: String,
    pub count: usize,
    pub dim: usize,
    pub file: String,
    pub bytes: u64,
    /// FNV-1a 64 of the collection file bytes — catches a half-copied/corrupted pack.
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub entity: String,
    pub version: String,
    pub built_at: String,
    pub embed_model_id: String,
    pub embed_dim: usize,
    pub strategy_version: u32,
    pub collections: Vec<CollectionEntry>,
    /// Hash agregado da árvore `docs/` (fonte `.md` dos pareceres), quando ela existe. Validado no
    /// boot junto dos hashes de pack: a árvore é fonte de conteúdo servido, então divergir dela é
    /// tão grave quanto um pack corrompido. Ausente em manifestos de entidades sem árvore (e nos
    /// antigos, anteriores à árvore) — nesse caso não há o que validar.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub docs_hash: Option<String>,
}

impl Manifest {
    pub fn embed_identity(&self) -> EmbedIdentity {
        EmbedIdentity {
            embed_model_id: self.embed_model_id.clone(),
            embed_dim: self.embed_dim,
            strategy_version: self.strategy_version,
        }
    }
}

/// FNV-1a 64-bit hash of a byte slice. Cheap, dependency-free, plenty for "is this the file I
/// wrote?" integrity — not a cryptographic guarantee.
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Hex string form used in the manifest.
pub fn hash_hex(bytes: &[u8]) -> String {
    format!("{:016x}", fnv1a64(bytes))
}

/// Manifest file name for an entity: `<entity>.manifest.json`.
pub fn manifest_path(packs_dir: impl AsRef<Path>, entity: &str) -> std::path::PathBuf {
    packs_dir.as_ref().join(format!("{entity}.manifest.json"))
}

/// Hash agregado da árvore de documentos em `docs_dir` (recursivo), ou `None` se o diretório não
/// existe (entidade sem árvore).
///
/// Agrega FNV-1a sobre a lista **ordenada** de `caminho_relativo` + bytes de cada arquivo — assim o
/// hash muda se um arquivo mudar, for adicionado, removido ou renomeado. Um único agregado (D2):
/// barato e detecta qualquer alteração; não diz *qual* arquivo mudou — para isso, re-materializar.
pub fn hash_docs_tree(docs_dir: impl AsRef<Path>) -> Result<Option<String>> {
    let dir = docs_dir.as_ref();
    if !dir.exists() {
        return Ok(None);
    }
    let mut arquivos: Vec<std::path::PathBuf> = Vec::new();
    coletar_arquivos(dir, &mut arquivos)?;
    arquivos.sort();

    // Encadeia (caminho relativo, conteúdo) na ordem — o caminho entra no hash para que renomear
    // sem mudar conteúdo também seja detectado.
    let mut acc: u64 = 0xcbf29ce484222325;
    for p in &arquivos {
        let rel = p.strip_prefix(dir).unwrap_or(p).to_string_lossy();
        for b in rel.as_bytes() {
            acc ^= *b as u64;
            acc = acc.wrapping_mul(0x100000001b3);
        }
        for b in std::fs::read(p)? {
            acc ^= b as u64;
            acc = acc.wrapping_mul(0x100000001b3);
        }
    }
    Ok(Some(format!("{acc:016x}")))
}

/// Coleta recursivamente os caminhos de arquivo sob `dir`.
fn coletar_arquivos(dir: &Path, out: &mut Vec<std::path::PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            coletar_arquivos(&path, out)?;
        } else {
            out.push(path);
        }
    }
    Ok(())
}

/// Confere a árvore em disco contra o `docs_hash` do manifesto. Manifesto sem `docs_hash` (entidade
/// sem árvore, ou pacote anterior à árvore) ⇒ nada a validar.
///
/// Divergência é erro duro pelo mesmo motivo do manifesto de pack: o corpo servido ao LLM sai dessa
/// árvore, então servir com ela fora de sincronia é responder a partir de conteúdo que não é o que
/// foi indexado.
pub fn validate_docs_hash(docs_dir: impl AsRef<Path>, manifest: &Manifest) -> Result<()> {
    let Some(esperado) = &manifest.docs_hash else {
        return Ok(());
    };
    let obtido = hash_docs_tree(&docs_dir)?;
    match obtido {
        Some(h) if &h == esperado => Ok(()),
        Some(h) => Err(Error::from(format!(
            "Árvore de documentos divergente para '{}': manifesto tem docs_hash={esperado}, disco tem {h}. \
             Re-gere os pacotes com `auli update` (ele relê a árvore e recarimba o hash).",
            manifest.entity
        ))),
        None => Err(Error::from(format!(
            "Árvore de documentos ausente para '{}' ({}), mas o manifesto exige docs_hash={esperado}. \
             Re-gere os pacotes com `auli update`.",
            manifest.entity,
            docs_dir.as_ref().display()
        ))),
    }
}

pub fn write_manifest(path: impl AsRef<Path>, manifest: &Manifest) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(manifest)?;
    std::fs::write(path, bytes)?;
    Ok(())
}

pub fn read_manifest(path: impl AsRef<Path>) -> Result<Manifest> {
    let bytes = std::fs::read(path)?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// Read the manifest at `path` and check its embedding identity equals `expected`. A divergence in
/// model, dimension, or strategy version is a hard error: the server should refuse to serve rather
/// than answer from a vector space that doesn't match its query encoder.
pub fn validate_manifest(path: impl AsRef<Path>, expected: &EmbedIdentity) -> Result<Manifest> {
    let manifest = read_manifest(path)?;
    let got = manifest.embed_identity();
    if &got != expected {
        return Err(Error::from(format!(
            "Manifest incompatível: pacote tem (modelo={}, dim={}, strategy={}), servidor espera (modelo={}, dim={}, strategy={}). Re-gere os pacotes com `auli update`.",
            got.embed_model_id, got.embed_dim, got.strategy_version,
            expected.embed_model_id, expected.embed_dim, expected.strategy_version,
        )));
    }
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_round_trips_and_matches_self() {
        let id = identity();
        assert_eq!(id.embed_model_id, EMBED_MODEL_ID);
        assert_eq!(id.embed_dim, EMBED_DIM);
        assert_eq!(id, id.clone());
    }

    #[test]
    fn fnv1a_known_vector() {
        // FNV-1a 64 of "" is the offset basis; of "a" is a well-known constant.
        assert_eq!(fnv1a64(b""), 0xcbf29ce484222325);
        assert_eq!(fnv1a64(b"a"), 0xaf63dc4c8601ec8c);
    }

    #[test]
    fn validate_rejects_strategy_mismatch() {
        let dir = std::env::temp_dir().join(format!("auli-manifest-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = manifest_path(&dir, "rs");
        let mut m = Manifest {
            entity: "rs".into(),
            version: "1".into(),
            built_at: "now".into(),
            embed_model_id: EMBED_MODEL_ID.into(),
            embed_dim: EMBED_DIM,
            strategy_version: STRATEGY_VERSION + 1, // wrong
            collections: vec![],
            docs_hash: None,
        };
        write_manifest(&path, &m).unwrap();
        assert!(validate_manifest(&path, &identity()).is_err());

        m.strategy_version = STRATEGY_VERSION; // fixed
        write_manifest(&path, &m).unwrap();
        assert!(validate_manifest(&path, &identity()).is_ok());

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Diretório temporário próprio por teste (evita colisão entre testes paralelos).
    fn temp_docs(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("auli-docs-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("pareceres")).unwrap();
        d
    }

    #[test]
    fn docs_hash_ausente_quando_nao_ha_arvore() {
        let inexistente = std::env::temp_dir().join(format!("auli-sem-arvore-{}", std::process::id()));
        assert_eq!(hash_docs_tree(&inexistente).unwrap(), None);
    }

    #[test]
    fn docs_hash_muda_com_conteudo_nome_e_remocao() {
        let d = temp_docs("mudanca");
        let a = d.join("pareceres/a.md");
        std::fs::write(&a, "corpo A").unwrap();
        let h1 = hash_docs_tree(&d).unwrap().unwrap();

        // Conteúdo diferente ⇒ hash diferente.
        std::fs::write(&a, "corpo A'").unwrap();
        let h2 = hash_docs_tree(&d).unwrap().unwrap();
        assert_ne!(h1, h2, "mudança de conteúdo tem de mudar o hash");

        // Renomear sem mudar conteúdo ⇒ hash diferente (o caminho entra no hash).
        let b = d.join("pareceres/b.md");
        std::fs::rename(&a, &b).unwrap();
        let h3 = hash_docs_tree(&d).unwrap().unwrap();
        assert_ne!(h2, h3, "renomear tem de mudar o hash");

        // Arquivo novo ⇒ hash diferente; remover volta ao anterior (determinístico).
        std::fs::write(d.join("pareceres/c.md"), "c").unwrap();
        let h4 = hash_docs_tree(&d).unwrap().unwrap();
        assert_ne!(h3, h4);
        std::fs::remove_file(d.join("pareceres/c.md")).unwrap();
        assert_eq!(hash_docs_tree(&d).unwrap().unwrap(), h3, "hash é determinístico");

        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn validate_docs_hash_aceita_igual_e_recusa_divergente_ou_ausente() {
        let d = temp_docs("validate");
        std::fs::write(d.join("pareceres/x.md"), "conteudo").unwrap();
        let h = hash_docs_tree(&d).unwrap().unwrap();

        let mut m = Manifest {
            entity: "sc".into(),
            version: "1".into(),
            built_at: "now".into(),
            embed_model_id: EMBED_MODEL_ID.into(),
            embed_dim: EMBED_DIM,
            strategy_version: STRATEGY_VERSION,
            collections: vec![],
            docs_hash: Some(h),
        };
        assert!(validate_docs_hash(&d, &m).is_ok(), "árvore idêntica passa");

        // Árvore mexida por fora ⇒ recusa.
        std::fs::write(d.join("pareceres/x.md"), "conteudo ADULTERADO").unwrap();
        let e = validate_docs_hash(&d, &m).unwrap_err().to_string();
        assert!(e.contains("divergente"), "erro: {e}");

        // Árvore sumida, mas manifesto exige ⇒ recusa.
        let _ = std::fs::remove_dir_all(&d);
        let e = validate_docs_hash(&d, &m).unwrap_err().to_string();
        assert!(e.contains("ausente"), "erro: {e}");

        // Manifesto sem docs_hash (entidade sem árvore / pacote antigo) ⇒ nada a validar.
        m.docs_hash = None;
        assert!(validate_docs_hash(&d, &m).is_ok());
    }

    #[test]
    fn manifesto_antigo_sem_docs_hash_ainda_desserializa() {
        let json = r#"{"entity":"rs","version":"1","built_at":"now","embed_model_id":"m",
                       "embed_dim":1024,"strategy_version":2,"collections":[]}"#;
        let m: Manifest = serde_json::from_str(json).unwrap();
        assert_eq!(m.docs_hash, None);
        // E um manifesto sem árvore não emite o campo (JSON continua enxuto).
        assert!(!serde_json::to_string(&m).unwrap().contains("docs_hash"));
    }
}
