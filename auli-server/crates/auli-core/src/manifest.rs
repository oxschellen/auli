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
///
/// **3 (TAREFA-TETO, ago/2026) — a key ganhou teto de 6.000 caracteres**
/// (`auli_contract::TETO_TEXT_TO_EMBED`), cortando pelo fim no compose único. Aqui os VETORES mudam
/// de verdade, e só os dos documentos cortados: 1,8% do TARF e 0,7% das FAQs; nenhum parecer.
///
/// O motivo é **densidade**, não memória: a medição (`RELATORIO-TETO.md`) mostrou cosseno
/// 0,978 entre o vetor do documento inteiro e o dos seus primeiros 6.000 caracteres na faixa de
/// 6–8 mil, ou seja, a cauda quase não move o vetor e não compra representação. A queda de memória
/// — pico de regeneração de 17,7 GiB para ~2 — é consequência, porque o custo é quadrático no maior
/// texto da coleção.
pub const STRATEGY_VERSION: u32 = 3;

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

/// Mapa `subdiretório de 1º nível de docs/` → hash FNV-1a 64 hex dos arquivos dele (D-DH-1).
///
/// `BTreeMap` de propósito: ordem determinística no JSON do manifesto, então dois manifestos do
/// mesmo estado diferem no máximo pelo `built_at`.
pub type DocsHashes = std::collections::BTreeMap<String, String>;

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
    /// Hash da árvore `docs/` POR SUBDIRETÓRIO de primeiro nível (D-DH-1), chaveado pelo DISCO —
    /// nunca pela lista de coleções do manifesto. É o chaveamento pelo disco que preserva a
    /// cobertura por omissão: um diretório ganha entrada por EXISTIR, e a próxima pasta que
    /// aparecer também — nada depende de alguém ter declarado. (Hoje as 27 entidades só têm
    /// diretórios que são coleções; a diferença aparece no dia em que uma delas não for.)
    ///
    /// **Regra central (D-DH-3):** cada entrada significa "o pack desta coleção veio deste estado
    /// da árvore". Numa rodada parcial (`auli update --kind`), só os kinds reconstruídos são
    /// recarimbados; os demais viajam verbatim do manifesto anterior — se alguém mexeu em
    /// `docs/faqs/` e rodou `--kind tarf`, o hash preservado de `faqs` deixa de bater e o boot
    /// recusa NOMEANDO `faqs`. Recarimbar tudo a cada rodada reproduziria o defeito do agregado,
    /// só que particionado.
    ///
    /// Ausente ⇒ entidade sem árvore, OU manifesto anterior à migração. A validação distingue os
    /// dois pelo disco (D-DH-2): mapa ausente COM árvore presente é erro alto — devolver `Ok` ali
    /// desligaria a guarda em silêncio nas 27 entidades.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub docs_hashes: Option<DocsHashes>,
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

/// Um passo do FNV-1a 64 sobre uma fatia, encadeando o acumulador. Fatorado porque agora há dois
/// campos por arquivo (caminho e conteúdo) e a duplicação do laço era convite a divergirem.
fn fnv_acc(mut acc: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        acc ^= b as u64;
        acc = acc.wrapping_mul(0x100000001b3);
    }
    acc
}

/// Hash da árvore `docs/` por subdiretório de 1º nível, ou `None` se `docs/` não existe (entidade
/// sem árvore).
///
/// **Particiona a VARREDURA de sempre, não a lista de documentos** que o `preparar` enxerga: os
/// mesmos bytes do antigo agregado, agora agrupados por diretório — um `.tmp` esquecido dentro de
/// `docs/tarf/` continua sendo detectado (era o requisito que descartou hashear a lista de
/// documentos). O caminho relativo entra no hash A PARTIR de `docs/` (com o nome do diretório,
/// portanto): renomear sem mudar conteúdo muda o hash, como antes.
///
/// **Arquivo solto direto em `docs/` é erro** (D-DH-6): não pertenceria a nenhuma chave do mapa e
/// ficaria fora da guarda — o único buraco silencioso que a partição poderia abrir. Medido antes
/// de virar erro (F0 da TAREFA): nenhuma das 27 entidades tem arquivo solto.
///
/// Sem delimitador entre caminho e conteúdo — a mesma escolha, com a mesma justificativa, do
/// agregado que este mapa substitui: guarda de integridade contra deriva acidental, não hash
/// resistente a adversário; se um dia virar defesa contra adversário, o FNV-1a inteiro sai junto.
pub fn hash_docs_map(docs_dir: impl AsRef<Path>) -> Result<Option<DocsHashes>> {
    let dir = docs_dir.as_ref();
    if !dir.exists() {
        return Ok(None);
    }
    let mut mapa = DocsHashes::new();
    for entry in std::fs::read_dir(dir)? {
        let sub = entry?.path();
        if !sub.is_dir() {
            return Err(Error::from(format!(
                "arquivo solto direto em `{}`: `{}`. A árvore contém diretórios (um por coleção); \
                 um arquivo fora deles não pertence a nenhuma entrada do mapa de hashes e ficaria \
                 fora da guarda de boot. Mova-o para o subdiretório certo ou remova-o.",
                dir.display(),
                sub.display()
            )));
        }
        let nome = sub
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let mut arquivos: Vec<std::path::PathBuf> = Vec::new();
        coletar_arquivos(&sub, &mut arquivos)?;
        arquivos.sort();
        let mut acc: u64 = 0xcbf29ce484222325;
        for p in &arquivos {
            let rel = p.strip_prefix(dir).unwrap_or(p).to_string_lossy();
            acc = fnv_acc(acc, rel.as_bytes());
            acc = fnv_acc(acc, &std::fs::read(p)?);
        }
        mapa.insert(nome, format!("{acc:016x}"));
    }
    Ok(Some(mapa))
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

/// Confere a árvore em disco contra o mapa `docs_hashes` do manifesto, listando TODAS as
/// divergências — não só a primeira (D-DH-5): o remédio de "uma árvore mudou E outra apareceu" é
/// diferente do de "só uma mudou", e parar no primeiro erro esconderia o segundo.
///
/// A matriz do mapa inteiro é o D-DH-2. O braço `(None, Some)` é o que força a migração: um
/// manifesto anterior ao mapa, com árvore no disco, é recusado alto — devolver `Ok` ali (a
/// tentação óbvia de "compatibilidade") desligaria a guarda em silêncio nas 27 entidades.
pub fn validate_docs_hashes(docs_dir: impl AsRef<Path>, manifest: &Manifest) -> Result<()> {
    let disco = hash_docs_map(&docs_dir)?;
    let (esperado, obtido) = match (&manifest.docs_hashes, disco) {
        // Entidade sem árvore, manifesto coerente com isso — o `None` de sempre.
        (None, None) => return Ok(()),
        (None, Some(_)) => {
            return Err(Error::from(format!(
                "'{}' tem árvore docs/ em disco, mas o manifesto não traz o mapa `docs_hashes` \
                 (manifesto anterior à migração). Re-gere com `auli update` — a rodada completa \
                 recarimba o mapa; a jurisprudência é reaproveitada pelo key_hash.",
                manifest.entity
            )));
        }
        (Some(_), None) => {
            return Err(Error::from(format!(
                "Árvore de documentos ausente para '{}' ({}), mas o manifesto exige `docs_hashes`. \
                 Re-gere os pacotes com `auli update`.",
                manifest.entity,
                docs_dir.as_ref().display()
            )));
        }
        (Some(e), Some(o)) => (e, o),
    };

    let mut problemas: Vec<String> = Vec::new();
    for (dir, h) in &obtido {
        match esperado.get(dir) {
            Some(e) if e == h => {}
            Some(e) => problemas.push(format!(
                "docs/{dir}/ mudou: manifesto tem {e}, disco tem {h}"
            )),
            None => problemas.push(format!(
                "docs/{dir}/ existe em disco e não está no manifesto"
            )),
        }
    }
    for dir in esperado.keys() {
        if !obtido.contains_key(dir) {
            problemas.push(format!("docs/{dir}/ está no manifesto e sumiu do disco"));
        }
    }
    if problemas.is_empty() {
        return Ok(());
    }
    Err(Error::from(format!(
        "Árvore de documentos divergente para '{}':\n  - {}\n\
         Re-gere com `auli update --kind <colecao>` para cada diretório nomeado (ou `auli update` \
         completo — ele relê tudo e recarimba o mapa).",
        manifest.entity,
        problemas.join("\n  - ")
    )))
}

/// Escreve o manifesto **atomicamente**: `.tmp` irmão + `rename`, a mesma disciplina que o
/// `write_collection_file` do `vector-store` já aplicava ao pack (D-INC-5). O temporário é irmão de
/// propósito — `rename` só é atômico dentro do mesmo sistema de arquivos.
///
/// A assimetria anterior (pack atômico, manifesto não) não era teórica: ela se manifestou numa
/// rodada de medição isolada por *hardlinks*, onde os packs sobreviveram pelo `rename` e o manifesto
/// de produção foi escrito **através** do link. O boot depende dos dois arquivos; só um estava
/// protegido. O que muda com isto:
///
/// - fecha a janela do "manifesto pela metade", que o doc do `run_remover` só podia mencionar como
///   possibilidade;
/// - a não-atomicidade que resta — a que existe **entre** pack e manifesto, dois arquivos sem
///   transação — passa a ser uma janela entre **dois estados válidos**, e não entre um estado válido
///   e um arquivo truncado. É o que faz a recuperação por `auli update` ser confiável;
/// - torna a isolação por hardlink uma técnica segura para o diretório de packs inteiro.
pub fn write_manifest(path: impl AsRef<Path>, manifest: &Manifest) -> Result<()> {
    let path = path.as_ref();
    let bytes = serde_json::to_vec_pretty(manifest)?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)?;
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
             migre o pacote ou rode `auli update` para reconstruir do zero.",
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
            docs_hashes: None,
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
    fn mapa_por_diretorio_e_deterministico_e_isola_mudancas() {
        let d = temp_docs("mapa");
        std::fs::create_dir_all(d.join("tarf")).unwrap();
        std::fs::write(d.join("pareceres/a.md"), "A").unwrap();
        std::fs::write(d.join("tarf/b.md"), "B").unwrap();
        let m1 = hash_docs_map(&d).unwrap().unwrap();
        assert_eq!(m1.len(), 2);

        // Mexer em UMA árvore muda SÓ a entrada dela — é a propriedade que o `--kind` compra.
        std::fs::write(d.join("tarf/b.md"), "B'").unwrap();
        let m2 = hash_docs_map(&d).unwrap().unwrap();
        assert_eq!(m1["pareceres"], m2["pareceres"], "pareceres não foi tocado");
        assert_ne!(m1["tarf"], m2["tarf"], "tarf mudou");

        // Renomear sem mudar conteúdo também muda (o caminho entra no hash), e desfazer volta:
        // o mapa é determinístico.
        std::fs::rename(d.join("tarf/b.md"), d.join("tarf/c.md")).unwrap();
        let m3 = hash_docs_map(&d).unwrap().unwrap();
        assert_ne!(m2["tarf"], m3["tarf"]);
        std::fs::rename(d.join("tarf/c.md"), d.join("tarf/b.md")).unwrap();
        assert_eq!(hash_docs_map(&d).unwrap().unwrap()["tarf"], m2["tarf"]);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn mapa_ausente_quando_nao_ha_arvore() {
        let inexistente =
            std::env::temp_dir().join(format!("auli-sem-arvore-{}", std::process::id()));
        assert_eq!(hash_docs_map(&inexistente).unwrap(), None);
    }

    #[test]
    fn arquivo_solto_em_docs_e_erro_nomeado() {
        // D-DH-6: fora de qualquer subdiretório, o arquivo ficaria fora de qualquer hash do mapa
        // — o único buraco que a partição poderia abrir. O erro NOMEIA o arquivo.
        let d = temp_docs("solto");
        std::fs::write(d.join("perdido.md"), "x").unwrap();
        let e = hash_docs_map(&d).unwrap_err().to_string();
        assert!(e.contains("perdido.md"), "erro nomeia o arquivo: {e}");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn validate_lista_todas_as_divergencias_nas_tres_classes() {
        // D-DH-5: as três classes de uma vez, numa mensagem só. Parar na primeira esconderia o
        // remédio das outras duas — repor esse defeito mata os dois últimos asserts.
        let d = temp_docs("classes");
        std::fs::create_dir_all(d.join("faqs")).unwrap();
        std::fs::write(d.join("pareceres/x.md"), "x").unwrap();
        std::fs::write(d.join("faqs/y.md"), "y").unwrap();
        let mut m = manifesto("rs");
        m.docs_hashes = hash_docs_map(&d).unwrap();
        assert!(
            validate_docs_hashes(&d, &m).is_ok(),
            "árvore idêntica passa"
        );

        std::fs::write(d.join("pareceres/x.md"), "x ADULTERADO").unwrap();
        std::fs::remove_dir_all(d.join("faqs")).unwrap();
        std::fs::create_dir_all(d.join("servicos")).unwrap();
        std::fs::write(d.join("servicos/n.md"), "n").unwrap();
        let e = validate_docs_hashes(&d, &m).unwrap_err().to_string();
        assert!(e.contains("docs/pareceres/ mudou"), "erro: {e}");
        assert!(
            e.contains("docs/faqs/ está no manifesto e sumiu"),
            "erro: {e}"
        );
        assert!(e.contains("docs/servicos/ existe em disco"), "erro: {e}");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn mapa_ausente_com_arvore_em_disco_e_erro_alto() {
        // D-DH-2, o braço que força a migração. "Simplificar" (None, Some) para Ok desligaria a
        // guarda em silêncio nas 27 entidades — este teste morre com essa simplificação.
        let d = temp_docs("migracao");
        std::fs::write(d.join("pareceres/x.md"), "x").unwrap();
        let m = manifesto("rs"); // docs_hashes: None
        let e = validate_docs_hashes(&d, &m).unwrap_err().to_string();
        assert!(e.contains("docs_hashes"), "erro: {e}");
        assert!(e.contains("auli update"), "o remédio está na mensagem: {e}");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn sem_arvore_e_sem_mapa_nada_a_validar() {
        let inexistente =
            std::env::temp_dir().join(format!("auli-sem-arvore-v-{}", std::process::id()));
        assert!(validate_docs_hashes(&inexistente, &manifesto("rs")).is_ok());
    }

    #[test]
    fn manifesto_antigo_com_docs_hash_escalar_ainda_desserializa() {
        // O campo antigo é IGNORADO pelo serde (campo desconhecido) e o mapa fica `None` — que a
        // validação recusa quando há árvore (teste acima). É a RECUSA que força a migração, não a
        // desserialização: um manifesto velho nunca derruba o parse, ele derruba o boot com o
        // remédio na mensagem.
        let json = r#"{"entity":"rs","version":"1","built_at":"now","embed_model_id":"m",
                       "embed_dim":1024,"strategy_version":3,"pack_format":2,"collections":[],
                       "docs_hash":"0123456789abcdef"}"#;
        let m: Manifest = serde_json::from_str(json).unwrap();
        assert_eq!(m.docs_hashes, None);
    }

    #[test]
    fn manifesto_sem_arvore_nao_emite_o_campo() {
        // JSON continua enxuto para entidade sem árvore.
        assert!(
            !serde_json::to_string(&manifesto("rs"))
                .unwrap()
                .contains("docs_hashes")
        );
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
            docs_hashes: None,
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

    /// Um manifesto qualquer, válido, para os testes de escrita.
    fn manifesto(entity: &str) -> Manifest {
        Manifest {
            entity: entity.into(),
            version: "1".into(),
            built_at: "now".into(),
            embed_model_id: EMBED_MODEL_ID.into(),
            embed_dim: EMBED_DIM,
            strategy_version: STRATEGY_VERSION,
            pack_format: PACK_FORMAT,
            collections: vec![],
            docs_hashes: None,
        }
    }

    #[test]
    fn escrita_do_manifesto_e_atomica_e_nao_deixa_tmp_para_tras() {
        // Espelha o `escrita_e_atomica_e_nao_deixa_tmp_para_tras` do `vector-store`: o arquivo final
        // aparece por `rename`, e o temporário não sobrevive à escrita.
        let dir = std::env::temp_dir().join(format!("auli-mtmp-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = manifest_path(&dir, "rs");

        write_manifest(&path, &manifesto("rs")).unwrap();

        assert!(path.exists());
        assert!(
            !dir.join("rs.manifest.json.tmp").exists(),
            "o temporário tem de sumir no rename"
        );
        assert_eq!(read_manifest(&path).unwrap().entity, "rs");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn escrever_atraves_de_hardlink_nao_altera_o_arquivo_original() {
        // O teste que descreve o DEFEITO, e não a implementação: com `fs::write` direto, escrever no
        // caminho B de um hardlink escreve no inode compartilhado e altera o A junto. Foi assim que
        // uma rodada de medição isolada por hardlinks alcançou o manifesto de produção.
        let dir = std::env::temp_dir().join(format!("auli-mlink-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("copia")).unwrap();

        let original = manifest_path(&dir, "rs");
        write_manifest(&original, &manifesto("rs")).unwrap();
        let antes = std::fs::read(&original).unwrap();

        // A cópia barata: mesmo inode, dois nomes.
        let link = manifest_path(dir.join("copia"), "rs");
        std::fs::hard_link(&original, &link).unwrap();

        // Uma rodada qualquer escrevendo no NOME da cópia.
        write_manifest(&link, &manifesto("OUTRA")).unwrap();

        assert_eq!(
            std::fs::read(&original).unwrap(),
            antes,
            "escrever pelo nome da cópia NÃO pode alcançar o original"
        );
        assert_eq!(read_manifest(&link).unwrap().entity, "OUTRA");
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
}
