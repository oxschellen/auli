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
/// even if the model matches — `validate_manifest` exige IGUALDADE, então qualquer divergência fecha
/// a porta no boot.
///
/// **A numeração foi REINICIADA em jul/2026 (TAREFA-FAQ-PR): esta é a estratégia 1 da nova série.**
/// O que mudou: a key das faqs passou a incluir a RESPOSTA (assunto + `P:` + `R:`, D-FAQPR-1), e o
/// encoder subiu de `max_length` 512 para o teto do modelo (8192) — com 512 a resposta seria cortada
/// em silêncio. Os dois efeitos mudam vetores, e o reset veio casado com rescrape completo: recomeço
/// limpo de código e de dados juntos. **Pré-condição do reset (declarada e verificada): nenhum pack
/// da série antiga carimbado `1` sobrevive em lugar nenhum** — um pack v1 ORIGINAL validaria em
/// silêncio contra este `1` novo. Bumps futuros seguem daqui: 2, 3, …
///
/// Regra da sinopse (F5): regenerar sinopses em massa (mudança de `SINOPSE_PROMPT_VERSION` ou do
/// modelo da sinopse + re-geração) muda os textos embedados de `pareceres` ⇒ bump obrigatório aqui.
/// Sinopses novas convivendo com antigas (append-only, sem re-geração) NÃO exigem bump — o embedder
/// é o mesmo.
///
/// ### Nota histórica — a numeração aposentada (v1–v4)
///
/// Continua sendo o registro canônico do que cada carimbo antigo significa, caso apareça um pack
/// velho: **v2** — a fonte passou a ser o `auli-contract` tipado (era o parsing de `portal-*.txt`).
/// **v3 (G3)** — o pack de `pareceres` passou a guardar o payload LEVE (JSON sem corpo) no lugar do
/// bloco pré-renderizado; servidor G3 lendo pack v2 renderizaria lixo. **v4** — correção do
/// vazamento de padding: o `update` embedava em lote e o fastembed fazia padding ao maior do lote,
/// contaminando o vetor agrupado (o mesmo texto saía com cosseno ~0,98 conforme a companhia — ver
/// `embed::testes_ordem`), enquanto a query, embedada sozinha, vivia noutro regime; `batch_size = 1`
/// pôs os dois no mesmo. Nos três casos o efeito prático era o desta série: re-ingestão obrigatória.
///
/// ### Série nova
///
/// **2 (B4, ago/2026) — pack v2: o payload leve para TODAS as coleções.** Serviços e FAQs guardavam
/// o bloco de contexto inteiro, pré-renderizado, dentro do pack — o texto integral de cada documento
/// duplicado, uma cópia na árvore e outra ao lado do vetor. Agora as quatro coleções guardam o
/// `DocumentoPack` (JSON com trilha/titulo/ementa/resumo/link/doc_path) e o servidor lê o conteúdo
/// tarde, pelo `doc_path`, só para os k documentos escolhidos. Servidor novo lendo pack 1
/// serviria JSON cru como texto do documento; servidor antigo lendo pack 2, idem ao contrário.
///
/// **Os VETORES não mudam neste bump.** Nenhuma fórmula de `text_to_embed` foi tocada — o que mudou
/// é só o que viaja ao lado do vetor. O bump existe porque pack e servidor mudam em lockstep, não
/// porque o espaço vetorial mudou; a re-ingestão é obrigatória do mesmo jeito.
pub const STRATEGY_VERSION: u32 = 2;

/// Versão do **FORMATO do arquivo de pack** — deliberadamente separada do [`STRATEGY_VERSION`]
/// (D-INC-7). A fronteira entre os dois é o que impede um reembed desnecessário:
///
/// - **estratégia** = "o que entra no vetor mudou" ⇒ os vetores estão errados ⇒ **reembed
///   obrigatório**;
/// - **formato** = "como o arquivo é escrito mudou" ⇒ os vetores continuam certos ⇒ **migração,
///   nunca reembed**.
///
/// Fundi-los custaria 72 minutos de GPU para reescrever um campo de texto, e apagaria a distinção
/// da qual a §34.2 depende.
///
/// **1** — ids posicionais `id-1..id-N`, sem `key_hash`. **2** (Fase 1 da TAREFA-UPDATE-INCREMENTAL)
/// — o id é o `doc_path` do documento, e cada record carrega o `key_hash` da key que o vetorizou.
/// Manifesto sem o campo **é** formato 1: foi escrito antes de ele existir.
pub const PACK_FORMAT: u32 = 2;

/// O formato de quem não declara. Ver [`PACK_FORMAT`].
fn pack_format_padrao() -> u32 {
    1
}

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
    /// Formato do arquivo de pack (D-INC-7). Ver [`PACK_FORMAT`]. Ausente ⇒ 1.
    #[serde(default = "pack_format_padrao")]
    pub pack_format: u32,
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
///
/// **Sem delimitador entre caminho e conteúdo — aceito, não esquecido.** O fluxo é
/// `caminho‖conteúdo‖caminho‖conteúdo…`, então em tese dois layouts distintos poderiam produzir a
/// mesma sequência de bytes (mover um sufixo do nome para o início do corpo). Irrelevante aqui: é
/// guarda de INTEGRIDADE contra deriva acidental — pack e árvore fora de sincronia —, não hash
/// resistente a colisão adversarial; e a árvore é escrita só por nós, com nomes gerados por
/// `slug*`. Se algum dia virar defesa contra adversário, este é o primeiro ponto a consertar (um
/// `\0` entre os campos resolve) — mas aí o FNV-1a inteiro teria de sair junto.
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
///
/// O **formato do pack** é conferido com a mesma igualdade, e por contrato explícito (D-INC-7).
/// Um servidor novo lendo um pack formato 1 desserializaria sem erro — os ids `id-N` são `String`
/// como qualquer outra e o `key_hash` ausente vira `None` —, então a incompatibilidade passaria em
/// silêncio até alguém notar retrieval estranho. Depender da tolerância do serde funcionaria por
/// acidente; esta comparação é o contrato.
pub fn validate_manifest(path: impl AsRef<Path>, expected: &EmbedIdentity) -> Result<Manifest> {
    let manifest = read_manifest(path)?;
    if manifest.pack_format != PACK_FORMAT {
        return Err(Error::from(format!(
            "Formato de pack incompatível para '{}': pacote é formato {}, servidor espera {}. \
             O ESPAÇO VETORIAL não mudou — só o arquivo —, então isto é migração, não re-embedding: \
             rode a migração da TAREFA-UPDATE-INCREMENTAL (ou `auli update` para reconstruir do zero).",
            manifest.entity, manifest.pack_format, PACK_FORMAT,
        )));
    }
    let got = manifest.embed_identity();
    if &got != expected {
        return Err(Error::from(format!(
            "Manifest incompatível: pacote tem (modelo={}, dim={}, strategy={}), servidor espera (modelo={}, dim={}, strategy={}). Re-gere os pacotes com `auli update`.",
            got.embed_model_id,
            got.embed_dim,
            got.strategy_version,
            expected.embed_model_id,
            expected.embed_dim,
            expected.strategy_version,
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
            pack_format: PACK_FORMAT,
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
        let inexistente =
            std::env::temp_dir().join(format!("auli-sem-arvore-{}", std::process::id()));
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
        assert_eq!(
            hash_docs_tree(&d).unwrap().unwrap(),
            h3,
            "hash é determinístico"
        );

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
            pack_format: PACK_FORMAT,
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
    fn validate_recusa_formato_de_pack_antigo() {
        // D-INC-7: um pack formato 1 DESSERIALIZA sem erro num servidor novo (os `id-N` são String
        // como qualquer outra, o `key_hash` ausente vira `None`), então sem esta comparação a
        // incompatibilidade passaria em silêncio.
        let dir = std::env::temp_dir().join(format!("auli-packfmt-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = manifest_path(&dir, "rs");
        let mut m = Manifest {
            entity: "rs".into(),
            version: "1".into(),
            built_at: "now".into(),
            embed_model_id: EMBED_MODEL_ID.into(),
            embed_dim: EMBED_DIM,
            strategy_version: STRATEGY_VERSION,
            pack_format: 1, // formato antigo, identidade de embedding CERTA
            collections: vec![],
            docs_hash: None,
        };
        write_manifest(&path, &m).unwrap();
        let e = validate_manifest(&path, &identity())
            .unwrap_err()
            .to_string();
        assert!(e.contains("Formato de pack"), "erro: {e}");
        // A mensagem tem de mandar MIGRAR, não re-embeddar: os vetores estão certos.
        assert!(e.contains("migração"), "erro: {e}");

        m.pack_format = PACK_FORMAT;
        write_manifest(&path, &m).unwrap();
        assert!(validate_manifest(&path, &identity()).is_ok());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn manifesto_sem_pack_format_e_formato_1() {
        // Quem não declara foi escrito antes do campo existir — logo, é o formato 1.
        let json = r#"{"entity":"rs","version":"1","built_at":"now","embed_model_id":"m",
                       "embed_dim":1024,"strategy_version":2,"collections":[]}"#;
        let m: Manifest = serde_json::from_str(json).unwrap();
        assert_eq!(m.pack_format, 1);
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
