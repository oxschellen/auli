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
//! pack/kind is `<entity>-servicos` too. `pareceres` is authored, not scraped: its contract
//! (`<entity>-pareceres.json`) is derived from the reference `.txt` by
//! `auli-collections <entity> pareceres`, then vectorized here like any other kind. `notas` has no
//! struct source yet and is simply absent until modeled as a contract.
//!
//! Ordem no fluxo de pareceres: a árvore `docs/` é materializada ANTES da guarda de sinopse, porque
//! pendência é estado legal da árvore e é dela que o passo `sinopse` parte. A guarda protege só a
//! vetorização — embedar sem sinopse é índice cego.
//!
//! Does NOT use the server `Config` (no LLM vars needed for ingestion) — only the embedder
//! settings, read directly from the environment with defaults.

use std::path::{Path, PathBuf};

use auli_contract::{Embeddable, Table};
use auli_core::embed::{Embedder, EMBED_DIM};
use auli_core::manifest::{self, CollectionEntry, Manifest};
use serde::de::DeserializeOwned;
use vector_store::Writer;

use crate::docs;
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
    // pareceres: <entity>-pareceres.json (contract) -> pack <entity>-pareceres. A fonte passa pelo
    // passo `auli-collections <entity> sinopse` (gera a sinopse que vira `resumo`/`text_to_embed`);
    // ingerida do `.txt` de referência por `auli-collections <entity> pareceres` (sem scraper por ora).
    // Ausente para entidades sem esse arquivo -> pulado.
    let docs_dir = out
        .parent()
        .ok_or_else(|| format!("diretório de packs sem pai: {}", out.display()))?
        .join("docs");
    let mut docs_hash = None;
    if let Some((hash, consultas)) = preparar_pareceres(&entity, &source, &docs_dir)? {
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

/// Materializa a árvore, **hidrata as sinopses dela** e só então aplica a guarda; devolve
/// `(docs_hash, consultas prontas para vetorizar)` — ou `None` para entidade sem pareceres.
///
/// G4 — propriedade dividida: a ÁRVORE é dona da sinopse (o passo `auli-collections <id> sinopse`
/// escreve nos `.md`); o JSON é dono de `numero`/`assunto`/`link`/`corpo` e do rol. Por isso o
/// `resumo` que vai ao índice vem do `.md`, não do JSON — e o `text_to_embed` é recomposto aqui pelo
/// ponto único (`compose_text_to_embed`), para reaproveitados e recém-gerados seguirem a mesma
/// fórmula.
///
/// A ordem vem da G3.5: materializar ANTES da guarda, porque pendência é estado **legal** da árvore
/// e é dela que o passo `sinopse` parte. A guarda protege só a vetorização.
fn preparar_pareceres(
    entity: &str,
    source: &Path,
    docs_dir: &Path,
) -> Result<Option<(Option<String>, Vec<auli_contract::Consulta>)>> {
    let Some(n) = docs::materializar_pareceres(entity, source, docs_dir)? else {
        return Ok(None); // entidade sem pareceres — sem árvore, sem hash
    };
    println!("📄 docs: {n} pareceres materializados em {}", docs_dir.join("pareceres").display());
    let hash = manifest::hash_docs_tree(docs_dir)?;

    let text = std::fs::read_to_string(source.join(format!("{entity}-pareceres.json")))?;
    let table: Table<auli_contract::Consulta> = serde_json::from_str(&text)?;
    let mut consultas = table.items;
    hidratar_da_arvore(&mut consultas, docs_dir)?;
    recusar_pareceres_sem_sinopse(entity, &consultas)?;
    Ok(Some((hash, consultas)))
}

/// Preenche `resumo`/`sinopse_info` de cada consulta a partir do seu `.md` (fonte da sinopse na G4)
/// e recompõe o `text_to_embed`. Arquivo ausente/ilegível é erro: a materialização acabou de rodar,
/// então isso só acontece se algo o removeu no meio — melhor falhar alto que indexar às cegas.
fn hidratar_da_arvore(consultas: &mut [auli_contract::Consulta], docs_dir: &Path) -> Result<()> {
    let dir = docs_dir.join("pareceres");
    for c in consultas.iter_mut() {
        let caminho = dir.join(format!("{}.md", auli_contract::mddoc::slug(&c.numero)));
        let texto = std::fs::read_to_string(&caminho)
            .map_err(|e| format!("`{}` ilegível ({e}) — re-materialize a árvore", caminho.display()))?;
        let (header, sinopse, _corpo) = auli_contract::mddoc::parse_doc(&texto)
            .map_err(|e| format!("`{}` não parseia ({e})", caminho.display()))?;
        c.resumo = sinopse.unwrap_or_default();
        c.sinopse_info = header.sinopse_info;
        c.text_to_embed = auli_contract::compose_text_to_embed(&c.numero, &c.assunto, &c.resumo);
    }
    Ok(())
}

/// Recusa vetorizar consulta sem sinopse **na árvore**: embedar sem ela é regressão silenciosa (o
/// `text_to_embed` fica cego para o corpo — é o que o passo `sinopse` resolve).
fn recusar_pareceres_sem_sinopse(entity: &str, consultas: &[auli_contract::Consulta]) -> Result<()> {
    let vazios: Vec<&str> = consultas
        .iter()
        .filter(|c| c.resumo.trim().is_empty())
        .map(|c| c.numero.as_str())
        .collect();
    if !vazios.is_empty() {
        let amostra = vazios.iter().take(5).copied().collect::<Vec<_>>().join(", ");
        let reticencias = if vazios.len() > 5 { ", ..." } else { "" };
        // A árvore já foi materializada quando chegamos aqui; o manifesto, não (este `Err` corta o
        // fluxo antes de `write_manifest`). Daí o aviso de reinício: para entidade já vetorizada, a
        // árvore no disco fica À FRENTE do manifesto antigo e o boot recusaria por `docs_hash`.
        return Err(format!(
            "{} consultas sem seção `## sinopse` na árvore ({amostra}{reticencias}).\n\
             A árvore docs/ foi materializada (com pendências — estado válido); a VETORIZAÇÃO foi \
             recusada para não indexar às cegas.\n\
             Remédio: rode `auli-collections {entity} sinopse` (ele preenche os `.md`) e depois \
             `auli update` de novo.\n\
             Atenção: se esta entidade já estava vetorizada, NÃO reinicie o servidor antes do novo \
             `auli update` — a árvore no disco ficou à frente do manifesto e o boot recusaria.",
            vazios.len()
        )
        .into());
    }
    Ok(())
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
    use auli_contract::{Consulta, SinopseInfo};

    fn consulta(numero: &str, resumo: &str) -> Consulta {
        Consulta {
            numero: numero.into(),
            assunto: "ICMS. ASSUNTO".into(),
            resumo: resumo.into(),
            corpo: "Corpo integral.".into(),
            link: "http://x/1".into(),
            text_to_embed: "irrelevante aqui".into(),
            sinopse_info: (!resumo.is_empty()).then(|| SinopseInfo {
                modelo: "m".into(),
                prompt_versao: 1,
                gerada_em: "2026-07-19T00:00:00Z".into(),
            }),
        }
    }

    /// Devolve `(source, docs)` — `source` já com o contrato `xx-pareceres.json` escrito.
    fn cenario(tag: &str, items: Vec<Consulta>) -> (PathBuf, PathBuf) {
        let base = std::env::temp_dir().join(format!("auli-update-g35-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let source = base.join("raw");
        std::fs::create_dir_all(&source).unwrap();
        let t = Table::new("xx", "pareceres", items);
        std::fs::write(source.join("xx-pareceres.json"), serde_json::to_vec(&t).unwrap()).unwrap();
        (source, base.join("docs"))
    }

    #[test]
    fn pendentes_materializam_a_arvore_mas_recusam_a_vetorizacao() {
        let (source, docs) = cenario(
            "pend",
            vec![
                consulta("CONSULTA Nº 1/26", "### Descrição Resumida do Assunto\nx"),
                consulta("CONSULTA Nº 2/26", ""),
            ],
        );
        let err = preparar_pareceres("xx", &source, &docs).unwrap_err();

        // A guarda recusou...
        assert!(err.to_string().contains("1 consultas sem seção"), "erro inesperado: {err}");
        // ...mas a árvore existe, com o pendente inclusive.
        let pronto = docs.join("pareceres/consulta-no-1-26.md");
        let pendente = docs.join("pareceres/consulta-no-2-26.md");
        assert!(pronto.exists(), "consulta com sinopse não foi materializada");
        assert!(pendente.exists(), "consulta PENDENTE não foi materializada — é estado legal da árvore");
        // Pendência na árvore = arquivo sem a seção `## sinopse` (é assim que a G4 a reconhece).
        let texto = std::fs::read_to_string(&pendente).unwrap();
        assert!(!texto.contains("## sinopse"), "pendente não deveria ter seção de sinopse");
        assert!(texto.contains("## corpo"), "todo doc materializado tem corpo");
        // Nenhum manifesto gravado: o `Err` corta o fluxo antes do `write_manifest` (por construção,
        // `preparar_pareceres` nunca escreve manifesto — o teste ancora a expectativa).
        let base = source.parent().unwrap();
        assert!(!base.join("xx.manifest.json").exists(), "manifesto não deve ser gravado na recusa");
    }

    #[test]
    fn sem_pendentes_devolve_o_hash_da_arvore() {
        let (source, docs) = cenario(
            "ok",
            vec![consulta("CONSULTA Nº 1/26", "### Descrição Resumida do Assunto\nx")],
        );
        let (hash, consultas) = preparar_pareceres("xx", &source, &docs).unwrap().unwrap();
        assert!(hash.is_some(), "árvore materializada deve produzir docs_hash");
        assert_eq!(hash, manifest::hash_docs_tree(&docs).unwrap(), "hash deve ser o da árvore no disco");
        assert_eq!(consultas.len(), 1);
    }

    #[test]
    fn a_sinopse_indexada_vem_da_arvore_e_nao_do_json() {
        // O CORAÇÃO DA G4: o passo `sinopse` escreve no `.md`; o JSON fica sem `resumo`. O que vai ao
        // índice tem de vir da árvore — senão a vetorização sai cega.
        let (source, docs) = cenario("g4hidrata", vec![consulta("CONSULTA Nº 1/26", "")]);
        // 1ª passada: materializa pendente e a guarda recusa (sem sinopse em lugar nenhum).
        assert!(preparar_pareceres("xx", &source, &docs).is_err());

        // O passo `sinopse` (simulado) preenche a ÁRVORE — o JSON continua sem resumo.
        let p = docs.join("pareceres/consulta-no-1-26.md");
        let (mut h, _s, corpo) = auli_contract::mddoc::parse_doc(&std::fs::read_to_string(&p).unwrap()).unwrap();
        h.sinopse_info = Some(auli_contract::SinopseInfo {
            modelo: "m".into(),
            prompt_versao: 1,
            gerada_em: "2026-07-20T00:00:00Z".into(),
        });
        std::fs::write(&p, auli_contract::mddoc::render_doc(&h, Some("SINOPSE SO NA ARVORE"), &corpo)).unwrap();

        // 2ª passada: guarda passa e o registro sai hidratado da árvore.
        let (_hash, consultas) = preparar_pareceres("xx", &source, &docs).unwrap().unwrap();
        assert_eq!(consultas[0].resumo, "SINOPSE SO NA ARVORE", "resumo tem de vir do .md");
        assert_eq!(consultas[0].sinopse_info.as_ref().unwrap().modelo, "m", "proveniência vem do .md");
        assert!(
            consultas[0].text_to_embed.contains("SINOPSE SO NA ARVORE"),
            "text_to_embed recomposto com a sinopse da árvore: {:?}",
            consultas[0].text_to_embed
        );
    }

    #[test]
    fn entidade_sem_pareceres_nao_tem_arvore_nem_hash() {
        let base = std::env::temp_dir().join(format!("auli-update-g35-vazio-{}", std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        let docs = base.join("docs");
        assert_eq!(preparar_pareceres("xx", &base, &docs).unwrap(), None);
        assert!(!docs.exists(), "entidade sem pareceres não deve criar docs/");
    }

    #[test]
    fn mensagem_da_guarda_ensina_o_remedio() {
        let (source, docs) = cenario("msg", vec![consulta("CONSULTA Nº 7/26", "")]);
        let msg = preparar_pareceres("xx", &source, &docs).unwrap_err().to_string();
        assert!(msg.contains("CONSULTA Nº 7/26"), "deve nomear os pendentes: {msg}");
        assert!(msg.contains("auli-collections xx sinopse"), "deve dar o comando remédio: {msg}");
        assert!(msg.contains("NÃO reinicie o servidor"), "deve avisar da janela de reinício: {msg}");
    }
}
