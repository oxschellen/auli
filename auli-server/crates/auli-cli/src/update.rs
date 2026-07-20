//! `auli update` — the vectorizer / pack builder (the only writer).
//!
//! For each content kind it reads the scraper's contract table `<source>/<entity>-<table>.json`
//! (an `auli_contract::Table<P>`), embeds each record's `text_to_embed()` and stores its
//! `stored_repr()` — via `auli-core::embed`, the SAME encoder the server uses on the query —
//! assigns sequential `id-1..id-N`, and `Writer::reset` + `upsert` into
//! `<out>/<entity>-<kind>.json`. Finally writes `<out>/<entity>.manifest.json` stamping the
//! embedding identity, so the server can validate it at boot.
//!
//! `servicos` is one vocabulary end-to-end: the source contract is `<entity>-servicos.json` and the
//! pack/kind is `<entity>-servicos` too. `notas` has no struct source yet and is simply absent until
//! modeled as a contract.
//!
//! **`pareceres` é a exceção (G5b): a fonte é a árvore `docs/pareceres/*.md`, não um JSON.** Os
//! produtores (scrapers) criam um `.md` por consulta inédita; o passo `auli-collections <id> sinopse`
//! preenche a `## sinopse`; aqui a árvore é lida em ordem de nome e vetorizada. A guarda recusa a
//! vetorização se algum documento estiver sem sinopse — embedar sem ela é índice cego —, mas a
//! árvore em si segue válida (pendência é estado legal dela).
//!
//! Does NOT use the server `Config` (no LLM vars needed for ingestion) — only the embedder
//! settings, read directly from the environment with defaults.

use std::path::{Path, PathBuf};

use auli_contract::{Embeddable, Table};
use auli_core::embed::{Embedder, EMBED_DIM};
use auli_core::manifest::{self, CollectionEntry, Manifest};
use serde::de::DeserializeOwned;
use vector_store::Writer;

use crate::error::Result;

pub fn run_update(entity: String, source: PathBuf, out: PathBuf, version: Option<String>) -> Result<()> {
    dotenvy::dotenv().ok();
    let cache_dir = std::env::var("EMBED_CACHE_DIR").unwrap_or_else(|_| "./models".to_string());
    let threads: usize = std::env::var("EMBED_THREADS").ok().and_then(|v| v.parse().ok()).unwrap_or(16);

    println!("🧠 Carregando embedder (BGE-M3) de '{}'...", cache_dir);
    let embedder = Embedder::new(cache_dir.into(), threads)?;
    let writer = Writer::new(&out);
    let mut entries: Vec<CollectionEntry> = Vec::new();

    // faqs: contract Table<Faq> in <source>/<entity>-faqs.json -> pack <entity>-faqs.
    if let Some(entry) = ingest::<auli_contract::Faq>(
        &embedder, &writer, &entity, "faqs", &format!("{}-faqs.json", entity), &source, &out,
    )? {
        entries.push(entry);
    }
    // servicos: <entity>-servicos.json (contract) -> pack <entity>-servicos.
    if let Some(entry) = ingest::<auli_contract::Servico>(
        &embedder, &writer, &entity, "servicos", &format!("{}-servicos.json", entity), &source, &out,
    )? {
        entries.push(entry);
    }
    // pareceres: a FONTE é a árvore `docs/pareceres/*.md` (G5b) — o JSON saiu do caminho. Os
    // produtores (scrapers) criam um `.md` por consulta inédita; o passo
    // `auli-collections <entity> sinopse` preenche a `## sinopse`; aqui só lemos e vetorizamos.
    // Entidade sem árvore -> pulada.
    let docs_dir = out
        .parent()
        .ok_or_else(|| format!("diretório de packs sem pai: {}", out.display()))?
        .join("docs");
    let mut docs_hash = None;
    if let Some((hash, consultas)) = preparar_pareceres(&entity, &docs_dir)? {
        docs_hash = hash;
        println!("🔢 pareceres: {} registros → vetorizando...", consultas.len());
        entries.push(ingest_items(&embedder, &writer, &entity, "pareceres", &consultas, &out)?);
    }
    // notas: sem fonte struct por ora (autoradas) — ausentes até serem modeladas.

    let manifest = Manifest {
        entity: entity.clone(),
        version: version.unwrap_or_else(|| "1".to_string()),
        built_at: chrono::Utc::now().to_rfc3339(),
        embed_model_id: manifest::EMBED_MODEL_ID.to_string(),
        embed_dim: EMBED_DIM,
        strategy_version: manifest::STRATEGY_VERSION,
        collections: entries,
        docs_hash,
    };
    let mpath = manifest::manifest_path(&out, &entity);
    manifest::write_manifest(&mpath, &manifest)?;
    println!("📝 Manifesto escrito em {:?}", mpath);

    Ok(())
}

/// Lê a árvore `docs/pareceres/*.md` — **a fonte** (G5b) — e devolve `(docs_hash, consultas prontas
/// para vetorizar)`, ou `None` se a entidade não tem árvore.
///
/// O JSON saiu do caminho: cada `.md` carrega tudo que o índice precisa (frontmatter dá
/// `numero`/`assunto`/`link`, a seção `## sinopse` dá o `resumo`, `## corpo` dá o corpo). O
/// `text_to_embed` é recomposto aqui pelo ponto único (`compose_text_to_embed`), então todos os
/// registros seguem a mesma fórmula, independentemente de quando foram produzidos.
///
/// Ordem estável: os arquivos são lidos em ordem de nome, para o pack ser reproduzível — os `id-N`
/// da coleção não podem dançar entre rodadas.
fn preparar_pareceres(
    entity: &str,
    docs_dir: &Path,
) -> Result<Option<(Option<String>, Vec<auli_contract::Consulta>)>> {
    let dir = docs_dir.join("pareceres");
    if !dir.exists() {
        return Ok(None); // entidade sem pareceres — sem árvore, sem hash
    }
    let mut caminhos: Vec<std::path::PathBuf> = std::fs::read_dir(&dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "md"))
        .collect();
    caminhos.sort();
    if caminhos.is_empty() {
        return Ok(None);
    }

    let mut consultas = Vec::with_capacity(caminhos.len());
    let mut pendentes: Vec<String> = Vec::new();
    for caminho in &caminhos {
        let texto = std::fs::read_to_string(caminho)?;
        let (header, sinopse, corpo) = auli_contract::mddoc::parse_doc(&texto)
            .map_err(|e| format!("`{}` não parseia ({e})", caminho.display()))?;
        let resumo = sinopse.unwrap_or_default();
        if resumo.trim().is_empty() {
            pendentes.push(header.numero.clone());
        }
        consultas.push(auli_contract::Consulta {
            text_to_embed: auli_contract::compose_text_to_embed(&header.numero, &header.assunto, &resumo),
            numero: header.numero,
            assunto: header.assunto,
            link: header.link,
            sinopse_info: header.sinopse_info,
            resumo,
            corpo,
        });
    }

    println!("📄 docs: {} pareceres lidos de {}", consultas.len(), dir.display());
    recusar_pareceres_sem_sinopse(entity, &pendentes)?;
    let hash = manifest::hash_docs_tree(docs_dir)?;
    Ok(Some((hash, consultas)))
}

/// Recusa vetorizar quando algum `.md` da árvore está **sem a seção `## sinopse`**: embedar sem ela
/// é regressão silenciosa (o `text_to_embed` fica cego para o corpo — é o que o passo `sinopse`
/// resolve). Recebe os números já apurados pela leitura da árvore.
fn recusar_pareceres_sem_sinopse(entity: &str, pendentes: &[String]) -> Result<()> {
    if pendentes.is_empty() {
        return Ok(());
    }
    let amostra = pendentes.iter().take(5).cloned().collect::<Vec<_>>().join(", ");
    let reticencias = if pendentes.len() > 5 { ", ..." } else { "" };
    // O manifesto NÃO é gravado nesta recusa (o `Err` corta antes do `write_manifest`). Para uma
    // entidade já vetorizada, a árvore no disco fica À FRENTE do manifesto antigo — e o boot
    // recusaria por `docs_hash`, corretamente. Daí o aviso de reinício.
    Err(format!(
        "{} documento(s) da árvore sem a seção `## sinopse` ({amostra}{reticencias}).\n\
         A VETORIZAÇÃO foi recusada para não indexar às cegas — a árvore em si está válida \
         (pendência é estado legal dela).\n\
         Remédio: rode `auli-collections {entity} sinopse` (ele preenche os `.md`) e depois \
         `auli update` de novo.\n\
         Atenção: se esta entidade já estava vetorizada, NÃO reinicie o servidor antes do novo \
         `auli update` — a árvore no disco ficou à frente do manifesto e o boot recusaria.",
        pendentes.len()
    )
    .into())
}

/// Ingest one contract table into the entity's `<entity>-<kind>` vector collection.
///
/// `source_file` is the scraper's contract JSON (e.g. `rs-servicos.json`); `kind` is the engine
/// vector kind (e.g. `servicos`). Embeds `P::text_to_embed`, stores `P::stored_repr`. Returns the
/// manifest entry, or `None` if the source is absent (a kind with no struct source yet — skipped).
fn ingest<P>(
    embedder: &Embedder,
    writer: &Writer,
    entity: &str,
    kind: &str,
    source_file: &str,
    source_dir: &Path,
    out: &Path,
) -> Result<Option<CollectionEntry>>
where
    P: Embeddable + DeserializeOwned,
{
    let src = source_dir.join(source_file);
    if !src.exists() {
        println!("⏭️  {} ausente ({:?}) — pulando", kind, src);
        return Ok(None);
    }

    let bytes = std::fs::read(&src)?;
    let table: Table<P> = serde_json::from_slice(&bytes)?;
    println!("🔢 {}: {} registros → vetorizando...", kind, table.items.len());
    Ok(Some(ingest_items(embedder, writer, entity, kind, &table.items, out)?))
}

/// Núcleo do ingest: embeda `text_to_embed`, guarda `stored_repr` e devolve a entrada de manifesto.
/// Separado de [`ingest`] porque pareceres não vêm mais direto do arquivo — passam pela hidratação
/// da árvore (G4) e chegam aqui em memória.
fn ingest_items<P>(
    embedder: &Embedder,
    writer: &Writer,
    entity: &str,
    kind: &str,
    items: &[P],
    out: &Path,
) -> Result<CollectionEntry>
where
    P: Embeddable,
{
    let to_embed: Vec<String> = items.iter().map(|it| it.text_to_embed().to_string()).collect();
    let stored: Vec<String> = items.iter().map(|it| it.stored_repr()).collect();

    let name = format!("{}-{}", entity, kind);
    let embeddings = embedder.embed_dense(to_embed)?;
    // The manifest stamps `dim = EMBED_DIM` (a constant). If the model ever produces a different
    // real width without an `EMBED_MODEL_ID` bump, the manifest would lie and boot validation
    // (which checks the identity triple, not the file width) wouldn't catch it. Fail loudly here.
    if let Some(first) = embeddings.first()
        && first.len() != EMBED_DIM
    {
        return Err(crate::error::Error::Custom(format!(
            "embedder produziu dim {} ≠ EMBED_DIM {} para '{}' — faça bump de EMBED_MODEL_ID e re-gere os packs",
            first.len(),
            EMBED_DIM,
            name
        )));
    }
    let ids: Vec<String> = (1..=stored.len()).map(|i| format!("id-{}", i)).collect();

    writer.reset::<String>(&name)?; // clean reload: no orphan id-(N+1)..
    let total = writer.upsert(&name, &ids, embeddings, &stored)?;

    // Stamp this collection in the manifest (count + integrity hash of the written file).
    let file_name = format!("{}.json", name);
    let written = std::fs::read(out.join(&file_name))?;
    println!("✅ {} → {} registros", name, total);
    Ok(CollectionEntry {
        kind: kind.to_string(),
        count: total as usize,
        dim: EMBED_DIM,
        file: file_name,
        bytes: written.len() as u64,
        hash: manifest::hash_hex(&written),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use auli_contract::{SinopseInfo, mddoc};

    /// Monta uma árvore de teste e devolve `docs_dir` (o pai de `pareceres/`).
    fn arvore(tag: &str, docs: &[(&str, Option<&str>)]) -> PathBuf {
        let base = std::env::temp_dir().join(format!("auli-update-g5b-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let dir = base.join("docs").join("pareceres");
        std::fs::create_dir_all(&dir).unwrap();
        for (numero, sinopse) in docs {
            let header = mddoc::DocHeader {
                numero: (*numero).into(),
                assunto: format!("assunto de {numero}"),
                link: format!("http://x/{numero}"),
                sinopse_info: sinopse.map(|_| SinopseInfo {
                    modelo: "m".into(),
                    prompt_versao: 1,
                    gerada_em: "2026-07-20T00:00:00Z".into(),
                }),
            };
            let corpo = format!("corpo de {numero}");
            std::fs::write(
                dir.join(format!("{}.md", mddoc::slug(numero))),
                mddoc::render_doc(&header, *sinopse, &corpo),
            )
            .unwrap();
        }
        base.join("docs")
    }

    #[test]
    fn le_a_arvore_como_fonte_sem_json_nenhum() {
        // O CORAÇÃO DA G5b: não existe JSON no cenário — tudo vem dos `.md`.
        let docs = arvore("fonte", &[("CONSULTA Nº 1/26", Some("SINOPSE UM"))]);
        let (hash, consultas) = preparar_pareceres("xx", &docs).unwrap().unwrap();
        assert_eq!(consultas.len(), 1);
        let c = &consultas[0];
        assert_eq!(c.numero, "CONSULTA Nº 1/26");
        assert_eq!(c.assunto, "assunto de CONSULTA Nº 1/26");
        assert_eq!(c.link, "http://x/CONSULTA Nº 1/26");
        assert_eq!(c.resumo, "SINOPSE UM");
        assert_eq!(c.corpo, "corpo de CONSULTA Nº 1/26");
        assert_eq!(c.sinopse_info.as_ref().unwrap().modelo, "m");
        // `text_to_embed` recomposto pelo ponto único: numero + assunto + sinopse.
        assert_eq!(
            c.text_to_embed,
            auli_contract::compose_text_to_embed(&c.numero, &c.assunto, &c.resumo)
        );
        assert_eq!(hash, manifest::hash_docs_tree(&docs).unwrap());
    }

    #[test]
    fn documento_sem_sinopse_recusa_a_vetorizacao() {
        let docs = arvore("pend", &[("A 1", Some("tem")), ("B 2", None)]);
        let err = preparar_pareceres("xx", &docs).unwrap_err().to_string();
        assert!(err.contains("1 documento(s)"), "erro: {err}");
        assert!(err.contains("B 2"), "deve nomear o pendente: {err}");
        assert!(err.contains("auli-collections xx sinopse"), "deve dar o remédio: {err}");
        assert!(err.contains("NÃO reinicie o servidor"), "deve avisar da janela: {err}");
    }

    #[test]
    fn entidade_sem_arvore_e_pulada() {
        let base = std::env::temp_dir().join(format!("auli-update-g5b-vazio-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        assert!(preparar_pareceres("xx", &base.join("docs")).unwrap().is_none());
    }

    #[test]
    fn arvore_vazia_e_pulada_em_vez_de_gerar_pack_vazio() {
        let docs = arvore("semdocs", &[]);
        assert!(preparar_pareceres("xx", &docs).unwrap().is_none());
    }

    #[test]
    fn ordem_e_estavel_por_nome_de_arquivo() {
        // Os `id-N` do pack derivam da ordem; ela não pode depender do read_dir do SO.
        let docs = arvore("ordem", &[("C 3", Some("s")), ("A 1", Some("s")), ("B 2", Some("s"))]);
        let (_, consultas) = preparar_pareceres("xx", &docs).unwrap().unwrap();
        let numeros: Vec<&str> = consultas.iter().map(|c| c.numero.as_str()).collect();
        assert_eq!(numeros, vec!["A 1", "B 2", "C 3"], "ordem deve ser a dos slugs, ordenados");
    }

    #[test]
    fn documento_ilegivel_e_erro_alto() {
        let docs = arvore("ruim", &[("A 1", Some("s"))]);
        std::fs::write(docs.join("pareceres/ruim.md"), "sem frontmatter").unwrap();
        let err = preparar_pareceres("xx", &docs).unwrap_err().to_string();
        assert!(err.contains("não parseia"), "erro: {err}");
    }
}
