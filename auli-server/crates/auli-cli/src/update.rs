//! `auli update` — the vectorizer / pack builder (the only writer).
//!
//! For each content kind it reads the `.md` tree in `<out>/../docs/<kind>/`, embeds each record's
//! `text_to_embed()` and stores its `stored_repr()` — via `auli-core::embed`, the SAME encoder the
//! server uses on the query — assigns sequential `id-1..id-N`, and `Writer::reset` + `upsert` into
//! `<out>/<entity>-<kind>.json`. Finally writes `<out>/<entity>.manifest.json` stamping the
//! embedding identity, so the server can validate it at boot.
//!
//! `servicos` is one vocabulary end-to-end: the pack/kind is `<entity>-servicos`. **A fonte é a
//! árvore `docs/servicos/*.md` (TAREFA-SERVICOS-MD)**, regenerada a cada
//! `auli-collections <id> process`; o mesmo vale para `faqs` (TAREFA-FAQS-MD). Entidade sem a
//! árvore é pulada. `notas` has no struct source yet and is simply absent until modeled as a
//! contract.
//!
//! **`pareceres` é a exceção (G5b): a fonte é a árvore `docs/pareceres/*.md`, não um JSON.** Os
//! produtores (scrapers) criam um `.md` por consulta inédita; o passo `auli-collections <id> sinopse`
//! preenche o `## resumo`; aqui a árvore é lida em ordem de nome e vetorizada. A guarda recusa a
//! vetorização se algum documento estiver sem resumo — embedar sem ela é índice cego —, mas a
//! árvore em si segue válida (pendência é estado legal dela).
//!
//! Does NOT use the server `Config` (no LLM vars needed for ingestion) — only the embedder
//! settings, read directly from the environment with defaults.

use std::path::{Path, PathBuf};

use auli_contract::{Documento, Embeddable, Kind};
use auli_core::embed::{EMBED_DIM, EMBED_MAX_TOKENS, Embedder};
use auli_core::manifest::{self, CollectionEntry, Manifest};
use vector_store::Writer;

use crate::error::Result;

pub fn run_update(entity: String, out: PathBuf, version: Option<String>) -> Result<()> {
    dotenvy::dotenv().ok();
    let cache_dir = std::env::var("EMBED_CACHE_DIR").unwrap_or_else(|_| "./models".to_string());
    let threads: usize = std::env::var("EMBED_THREADS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(16);

    println!("🧠 Carregando embedder (BGE-M3) de '{}'...", cache_dir);
    let embedder = Embedder::new(cache_dir.into(), threads)?;
    let writer = Writer::new(&out);
    let mut entries: Vec<CollectionEntry> = Vec::new();

    // A árvore `docs/` é a fonte das QUATRO coleções — faqs (TAREFA-FAQS-MD), serviços
    // (TAREFA-SERVICOS-MD) e a jurisprudência, pareceres e TARF (G5b) —, por isso o caminho é
    // montado antes dos blocos.
    let docs_dir = out
        .parent()
        .ok_or_else(|| format!("diretório de packs sem pai: {}", out.display()))?
        .join("docs");
    // **UM caminho de código para as quatro coleções** (D-B2). A fonte é sempre a árvore
    // `docs/<kind>/*.md`, o registro é sempre um `Documento`, e o que difere — como o `.md` vira
    // struct, e se há guarda de resumo — está dentro do `preparar`. Coleção sem árvore é pulada.
    //
    // **Um pack POR coleção** (D-A2): `<id>-pareceres` e `<id>-tarf` nunca se misturam. São acervos
    // de natureza diferente (interpretação da lei × julgamento de recurso) e o usuário escolhe qual
    // consultar; fundi-los tiraria essa escolha dele e diluiria as duas buscas.
    for kind in Kind::TODOS {
        let Some(docs) = preparar(&entity, &docs_dir, kind)? else {
            continue;
        };
        avisar_truncados(&embedder, kind, &docs)?;
        println!("🔢 {}: {} registros → vetorizando...", kind, docs.len());
        entries.push(ingest_items(
            &embedder, &writer, &entity, kind, &docs, &out,
        )?);
    }
    // notas: sem fonte struct por ora (autoradas) — ausentes até serem modeladas.

    // Hash da árvore `docs/` INTEIRA (pareceres e serviços): ela é fonte de conteúdo vetorizado e
    // servido, então divergir dela é tão grave quanto pack corrompido. Fica aqui, e não dentro de um
    // dos preparadores, porque a árvore não pertence a uma coleção só — entidade que só tem serviços
    // também ganha hash, e o `validate_docs_hash` do boot passa a protegê-la de graça.
    // `hash_docs_tree` devolve `None` se `docs/` não existe.
    let docs_hash = manifest::hash_docs_tree(&docs_dir)?;

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

/// Avisa quais documentos batem no teto de tokens do encoder e portanto vão ao índice **truncados**.
///
/// Aviso, não erro (D-FAQPR-5 revisada): o item ainda vetoriza, com o excedente descartado pelo
/// tokenizer. O que não pode é isso acontecer em silêncio — daí a contagem pelo tokenizer REAL e o
/// nome do `.md` de cada vítima. Amostra de até 5, no padrão dos outros relatórios do update.
///
/// Vale para as QUATRO coleções desde a B4 (antes só as faqs eram conferidas). O TARF é quem mais
/// pede: a `## resumo` dele é a fundamentação inteira do acórdão, que chega a 27 mil caracteres —
/// duas ordens de grandeza acima da sinopse curta de um parecer.
fn avisar_truncados(embedder: &Embedder, kind: Kind, docs: &[Documento]) -> Result<()> {
    let mut truncados: Vec<String> = Vec::new();
    for d in docs {
        if embedder.conta_tokens(&d.text_to_embed)? >= EMBED_MAX_TOKENS {
            // O `doc_path` já É o nome do arquivo pela regra da coleção — nada a recomputar.
            truncados.push(d.doc_path());
        }
    }
    if !truncados.is_empty() {
        let amostra = truncados
            .iter()
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        let reticencias = if truncados.len() > 5 { ", …" } else { "" };
        println!(
            "⚠️  {} de {} acima de {EMBED_MAX_TOKENS} tokens — o excedente NÃO entra no vetor: {amostra}{reticencias}",
            truncados.len(),
            kind
        );
    }
    Ok(())
}

/// Os `.md` de um diretório, em ordem de NOME. `None` se o diretório não existe ou está vazio —
/// nos dois casos a coleção é pulada, o que é melhor que gerar um pack vazio.
///
/// A ordem é estável de propósito: os `id-N` do pack derivam dela e não podem depender do
/// `read_dir` do sistema de arquivos, ou o pack deixa de ser reproduzível entre rodadas.
fn arquivos_md(dir: &Path) -> Result<Option<Vec<PathBuf>>> {
    if !dir.exists() {
        return Ok(None);
    }
    let mut caminhos: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "md"))
        .collect();
    caminhos.sort();
    Ok((!caminhos.is_empty()).then_some(caminhos))
}

/// Lê a árvore `docs/<kind>/*.md` — **a FONTE** — e devolve os documentos prontos para vetorizar,
/// ou `None` se a entidade não tem essa coleção.
///
/// **Um caminho para as quatro** (D-B2). O que muda entre elas é só qual fórmula compõe a key, e
/// isso vive em [`documento_de`]. O `text_to_embed` é sempre RECOMPOSTO aqui, pelo ponto único do
/// contrato: assim todos os registros seguem a mesma fórmula, independentemente de quando o `.md`
/// foi produzido.
fn preparar(entity: &str, docs_dir: &Path, kind: Kind) -> Result<Option<Vec<Documento>>> {
    let dir = docs_dir.join(kind.as_str());
    let Some(caminhos) = arquivos_md(&dir)? else {
        return Ok(None);
    };

    let mut docs = Vec::with_capacity(caminhos.len());
    let mut pendentes: Vec<String> = Vec::new();
    for caminho in &caminhos {
        let texto = std::fs::read_to_string(caminho)?;
        let (header, resumo, corpo) = auli_contract::mddoc::parse_doc(&texto)
            .map_err(|e| format!("`{}` não parseia ({e})", caminho.display()))?;
        let resumo = resumo.unwrap_or_default();
        if kind.exige_resumo() && resumo.trim().is_empty() {
            pendentes.push(header.titulo.clone());
        }
        docs.push(documento_de(kind, header, resumo, corpo));
    }

    println!(
        "📄 docs: {} {} lidos de {}",
        docs.len(),
        kind.plural(),
        dir.display()
    );
    recusar_jurisprudencia_sem_resumo(entity, kind, &pendentes)?;
    Ok(Some(docs))
}

/// Um `.md` lido vira [`Documento`]. A única coisa que difere por coleção é a fórmula da key — o
/// resto do mapeamento é o vocabulário unificado caindo 1:1.
///
/// As três fórmulas continuam sendo as do contrato, congeladas: mudar qualquer uma re-vetoriza os
/// packs daquela coleção.
fn documento_de(
    kind: Kind,
    header: auli_contract::mddoc::DocHeader,
    resumo: String,
    corpo: String,
) -> Documento {
    let text_to_embed = match kind {
        Kind::Faqs => {
            auli_contract::compose_faq_text_to_embed(&header.trilha, &header.titulo, &corpo)
        }
        Kind::Servicos => {
            // A trilha vem pré-composta da árvore; `tipo`/`classe` são as partes dela.
            let (tipo, classe) = auli_contract::partes_trilha_servico(&header.trilha);
            auli_contract::compose_servico_text_to_embed(tipo, classe, &header.titulo, &corpo)
        }
        Kind::Pareceres | Kind::Tarf => {
            auli_contract::compose_text_to_embed(&header.titulo, &header.ementa, &resumo)
        }
    };
    Documento {
        kind,
        text_to_embed,
        trilha: header.trilha,
        titulo: header.titulo,
        ementa: header.ementa,
        link: header.link,
        resumo_info: header.resumo_info,
        resumo,
        corpo,
    }
}

/// Recusa vetorizar quando algum `.md` de **jurisprudência** está sem a seção `## resumo`: embedar
/// sem ela é regressão silenciosa (o `text_to_embed` fica cego para o corpo — é o que o passo
/// `sinopse` resolve). Recebe os títulos já apurados pela leitura da árvore.
///
/// A exigência é **por coleção, não por formato** (D-FMT-7): serviços e faqs compartilham o mesmo
/// vocabulário e nunca têm `## resumo`, e isso é estado normal deles — só a jurisprudência promete
/// um resumo.
fn recusar_jurisprudencia_sem_resumo(
    entity: &str,
    kind: auli_contract::Kind,
    pendentes: &[String],
) -> Result<()> {
    if pendentes.is_empty() {
        return Ok(());
    }
    let amostra = pendentes
        .iter()
        .take(5)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    let reticencias = if pendentes.len() > 5 { ", ..." } else { "" };
    // O remédio difere por coleção, e mandar o errado é pior que não mandar nenhum: o passo
    // `sinopse` varre só `docs/pareceres/`, então sugeri-lo para um acórdão pendente manda o
    // operador para um beco que roda, não faz nada e não explica por quê.
    let remedio = match kind {
        auli_contract::Kind::Pareceres => format!(
            "Remédio: rode `auli-collections {entity} sinopse` (ele preenche os `.md`) e depois \
             `auli update` de novo."
        ),
        _ => String::from(
            "Remédio: o resumo do acórdão vem da COLETA (a fundamentação do cabeçalho, com a \
             ementa de fallback) — não há passo de sinopse para esta coleção. Investigue o `.md` \
             citado: ou o parser deixou passar um layout novo, ou o arquivo foi editado à mão.",
        ),
    };
    // O manifesto NÃO é gravado nesta recusa (o `Err` corta antes do `write_manifest`). Para uma
    // entidade já vetorizada, a árvore no disco fica À FRENTE do manifesto antigo — e o boot
    // recusaria por `docs_hash`, corretamente. Daí o aviso de reinício.
    Err(format!(
        "{} documento(s) de `{kind}` sem a seção `## resumo` ({amostra}{reticencias}).\n\
         A VETORIZAÇÃO foi recusada para não indexar às cegas — a árvore em si está válida \
         (pendência é estado legal dela).\n\
         {remedio}\n\
         Atenção: se esta entidade já estava vetorizada, NÃO reinicie o servidor antes do novo \
         `auli update` — a árvore no disco ficou à frente do manifesto e o boot recusaria.",
        pendentes.len()
    )
    .into())
}

/// Ingest one contract table into the entity's `<entity>-<kind>` vector collection.
///
/// Núcleo do ingest: embeda `text_to_embed`, guarda `stored_repr` e devolve a entrada de manifesto.
/// Separado de [`ingest`] porque pareceres não vêm mais direto do arquivo — passam pela hidratação
/// da árvore (G4) e chegam aqui em memória.
fn ingest_items<P>(
    embedder: &Embedder,
    writer: &Writer,
    entity: &str,
    kind: Kind,
    items: &[P],
    out: &Path,
) -> Result<CollectionEntry>
where
    P: Embeddable,
{
    let to_embed: Vec<String> = items
        .iter()
        .map(|it| it.text_to_embed().to_string())
        .collect();
    let stored: Vec<String> = items.iter().map(|it| it.stored_repr()).collect();

    let name = format!("{}-{}", entity, kind.as_str());
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
        kind: kind.as_str().to_string(),
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
    use auli_contract::{ResumoInfo, mddoc};

    /// Monta uma árvore de jurisprudência de teste e devolve `docs_dir` (o pai de `<kind>/`).
    fn arvore_de(kind: Kind, tag: &str, docs: &[(&str, Option<&str>)]) -> PathBuf {
        let base =
            std::env::temp_dir().join(format!("auli-update-g5b-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let dir = base.join("docs").join(kind.as_str());
        std::fs::create_dir_all(&dir).unwrap();
        for (numero, sinopse) in docs {
            let mut header = mddoc::cabecalho_jurisprudencia(
                numero,
                &format!("assunto de {numero}"),
                &format!("http://x/{numero}"),
            );
            header.resumo_info = sinopse.map(|_| ResumoInfo {
                modelo: "m".into(),
                prompt_versao: 1,
                gerada_em: "2026-07-20T00:00:00Z".into(),
            });
            let corpo = format!("corpo de {numero}");
            std::fs::write(
                dir.join(format!("{}.md", mddoc::slug(numero))),
                mddoc::render_doc(&header, *sinopse, &corpo),
            )
            .unwrap();
        }
        base.join("docs")
    }

    /// Atalho para a coleção que a maioria dos testes exercita.
    fn arvore(tag: &str, docs: &[(&str, Option<&str>)]) -> PathBuf {
        arvore_de(Kind::Pareceres, tag, docs)
    }

    #[test]
    fn le_a_arvore_como_fonte_sem_json_nenhum() {
        // O CORAÇÃO DA G5b: não existe JSON no cenário — tudo vem dos `.md`.
        let docs = arvore("fonte", &[("CONSULTA Nº 1/26", Some("SINOPSE UM"))]);
        let consultas = preparar("xx", &docs, Kind::Pareceres).unwrap().unwrap();
        assert_eq!(consultas.len(), 1);
        let c = &consultas[0];
        assert_eq!(c.titulo, "CONSULTA Nº 1/26");
        assert_eq!(c.ementa, "assunto de CONSULTA Nº 1/26");
        assert_eq!(c.link, "http://x/CONSULTA Nº 1/26");
        assert_eq!(c.resumo, "SINOPSE UM");
        assert_eq!(c.corpo, "corpo de CONSULTA Nº 1/26");
        assert_eq!(c.resumo_info.as_ref().unwrap().modelo, "m");
        // `text_to_embed` recomposto pelo ponto único: numero + assunto + sinopse.
        assert_eq!(
            c.text_to_embed,
            auli_contract::compose_text_to_embed(&c.titulo, &c.ementa, &c.resumo)
        );
    }

    /// **O coração da A2**: o TARF entra pelo MESMO caminho dos pareceres e sai num pack SEPARADO,
    /// com o `doc_path` apontando para a própria árvore.
    ///
    /// O defeito que este teste impede é silencioso: um acórdão cujo `doc_path` diga
    /// `docs/pareceres/...` vetoriza normalmente, entra no índice, é recuperado — e só na hora de
    /// montar o contexto RAG o corpo não é achado, degradando para o resumo sem nenhum erro visível.
    #[test]
    fn tarf_le_a_propria_arvore_e_grava_o_doc_path_dela() {
        use auli_contract::{DocumentoPack, Embeddable};

        let docs = arvore_de(
            Kind::Tarf,
            "tarf",
            &[("ACÓRDÃO 510/24", Some("FUNDAMENTAÇÃO"))],
        );
        // A árvore montada é a do TARF: pedir `pareceres` no mesmo `docs_dir` não acha nada.
        assert!(
            preparar("rs", &docs, Kind::Pareceres).unwrap().is_none(),
            "cada coleção lê SÓ o próprio diretório"
        );

        let acordaos = preparar("rs", &docs, Kind::Tarf).unwrap().unwrap();
        assert_eq!(acordaos.len(), 1);
        let a = &acordaos[0];
        assert_eq!(a.kind, Kind::Tarf);
        assert_eq!(a.titulo, "ACÓRDÃO 510/24");
        assert_eq!(a.resumo, "FUNDAMENTAÇÃO");

        // O payload que vai ao pack aponta para `docs/tarf/`, não para `docs/pareceres/`.
        let payload: DocumentoPack = serde_json::from_str(&a.stored_repr()).unwrap();
        assert_eq!(payload.doc_path, "docs/tarf/acordao-510-24.md");
        // E o `text_to_embed` é o mesmo compose de sempre — nenhuma fórmula nova nesta coleção.
        assert_eq!(
            a.text_to_embed,
            auli_contract::compose_text_to_embed(&a.titulo, &a.ementa, &a.resumo)
        );
        let _ = std::fs::remove_dir_all(docs.parent().unwrap());
    }

    #[test]
    fn acordao_sem_resumo_nao_manda_o_operador_para_o_passo_sinopse() {
        // O passo `sinopse` varre só `docs/pareceres/`: sugeri-lo aqui manda o operador para um
        // comando que roda, não faz nada e não explica por quê.
        let docs = arvore_de(Kind::Tarf, "tarf-pend", &[("ACÓRDÃO 1/24", None)]);
        let err = preparar("rs", &docs, Kind::Tarf).unwrap_err().to_string();
        assert!(err.contains("`tarf`"), "nomeia a coleção: {err}");
        // A palavra "sinopse" APARECE — dizendo que aquele passo não existe aqui, que é a informação
        // útil. O que não pode aparecer é o COMANDO, que é o que o operador copiaria e colaria.
        assert!(
            !err.contains("auli-collections rs sinopse"),
            "não manda para o beco: {err}"
        );
        assert!(err.contains("COLETA"), "aponta a origem certa: {err}");
        let _ = std::fs::remove_dir_all(docs.parent().unwrap());
    }

    #[test]
    fn documento_sem_sinopse_recusa_a_vetorizacao() {
        let docs = arvore("pend", &[("A 1", Some("tem")), ("B 2", None)]);
        let err = preparar("xx", &docs, Kind::Pareceres)
            .unwrap_err()
            .to_string();
        assert!(err.contains("1 documento(s)"), "erro: {err}");
        assert!(err.contains("B 2"), "deve nomear o pendente: {err}");
        assert!(
            err.contains("auli-collections xx sinopse"),
            "deve dar o remédio: {err}"
        );
        assert!(
            err.contains("NÃO reinicie o servidor"),
            "deve avisar da janela: {err}"
        );
    }

    #[test]
    fn entidade_sem_arvore_e_pulada() {
        let base =
            std::env::temp_dir().join(format!("auli-update-g5b-vazio-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        assert!(
            preparar("xx", &base.join("docs"), Kind::Pareceres)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn arvore_vazia_e_pulada_em_vez_de_gerar_pack_vazio() {
        let docs = arvore("semdocs", &[]);
        assert!(preparar("xx", &docs, Kind::Pareceres).unwrap().is_none());
    }

    #[test]
    fn ordem_e_estavel_por_nome_de_arquivo() {
        // Os `id-N` do pack derivam da ordem; ela não pode depender do read_dir do SO.
        let docs = arvore(
            "ordem",
            &[("C 3", Some("s")), ("A 1", Some("s")), ("B 2", Some("s"))],
        );
        let consultas = preparar("xx", &docs, Kind::Pareceres).unwrap().unwrap();
        let numeros: Vec<&str> = consultas.iter().map(|c| c.titulo.as_str()).collect();
        assert_eq!(
            numeros,
            vec!["A 1", "B 2", "C 3"],
            "ordem deve ser a dos slugs, ordenados"
        );
    }

    /// Monta uma árvore de FAQS de teste e devolve `docs_dir` (o pai de `faqs/`). Os nomes dos
    /// arquivos são dados de fora para o teste de ordem poder escolhê-los.
    fn arvore_faqs(tag: &str, docs: &[(&str, &str, &str)]) -> PathBuf {
        let base =
            std::env::temp_dir().join(format!("auli-update-faq-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let dir = base.join("docs").join("faqs");
        std::fs::create_dir_all(&dir).unwrap();
        for (nome_arquivo, pergunta, origin) in docs {
            let header = mddoc::DocHeader {
                titulo: (*pergunta).into(),
                trilha: (*origin).into(),
                ementa: String::new(),
                link: format!("https://x/{pergunta}"),
                orgao: None,
                resumo_info: None,
            };
            let corpo = format!("resposta de {pergunta}");
            std::fs::write(
                dir.join(format!("{nome_arquivo}.md")),
                mddoc::render_doc(&header, None, &corpo),
            )
            .unwrap();
        }
        base.join("docs")
    }

    #[test]
    fn preparar_faqs_le_arvore_em_ordem() {
        // Gravadas fora de ordem alfabética: a leitura reordena, para o pack ser reproduzível.
        let docs = arvore_faqs(
            "ordem",
            &[
                ("b-faq", "Segunda", ""),
                ("a-faq", "Primeira", "Inicial | FAQ"),
            ],
        );
        let faqs = preparar("xx", &docs, Kind::Faqs).unwrap().unwrap();
        assert_eq!(faqs.len(), 2);
        let perguntas: Vec<&str> = faqs.iter().map(|f| f.titulo.as_str()).collect();
        assert_eq!(
            perguntas,
            vec!["Primeira", "Segunda"],
            "ordem = nome de arquivo"
        );
        let f = &faqs[0];
        assert_eq!(f.trilha, "Inicial | FAQ");
        assert_eq!(f.link, "https://x/Primeira");
        assert_eq!(
            f.corpo, "resposta de Primeira",
            "resposta == corpo do `.md`"
        );
        // `text_to_embed` recomposto pelo ponto único do contrato — com a RESPOSTA (D-FAQPR-1).
        assert_eq!(
            f.text_to_embed,
            "Inicial | FAQ\nP: Primeira\nR: resposta de Primeira"
        );
        assert_eq!(
            faqs[1].trilha, "",
            "origin vazio sobrevive ao round-trip pela árvore"
        );
        assert_eq!(faqs[1].text_to_embed, "P: Segunda\nR: resposta de Segunda");
        let _ = std::fs::remove_dir_all(docs.parent().unwrap());
    }

    #[test]
    fn preparar_faqs_none_sem_arvore() {
        // Sem árvore, a coleção é pulada pelo chamador.
        let base =
            std::env::temp_dir().join(format!("auli-update-faq-vazio-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        assert!(
            preparar("xx", &base.join("docs"), Kind::Faqs)
                .unwrap()
                .is_none()
        );
        // Árvore existente porém vazia também é `None` — melhor que gerar um pack vazio.
        let docs = arvore_faqs("semdocs", &[]);
        assert!(preparar("xx", &docs, Kind::Faqs).unwrap().is_none());
        let _ = std::fs::remove_dir_all(&base);
    }

    /// Monta uma árvore de SERVIÇOS de teste e devolve `docs_dir` (o pai de `servicos/`). Os nomes
    /// dos arquivos são dados de fora para o teste de ordem poder escolhê-los.
    fn arvore_servicos(tag: &str, docs: &[(&str, &str)]) -> PathBuf {
        let base =
            std::env::temp_dir().join(format!("auli-update-svc-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let dir = base.join("docs").join("servicos");
        std::fs::create_dir_all(&dir).unwrap();
        for (nome_arquivo, titulo) in docs {
            let header = mddoc::DocHeader {
                titulo: (*titulo).into(),
                trilha: auli_contract::trilha_servico("Cidadãos", "IPVA"),
                ementa: String::new(),
                link: format!("https://x/{titulo}"),
                orgao: Some("Receita Estadual".into()),
                resumo_info: None,
            };
            let corpo = format!("descrição de {titulo}");
            std::fs::write(
                dir.join(format!("{nome_arquivo}.md")),
                mddoc::render_doc(&header, None, &corpo),
            )
            .unwrap();
        }
        base.join("docs")
    }

    #[test]
    fn preparar_servicos_le_arvore_em_ordem() {
        // Gravados fora de ordem alfabética: a leitura tem que reordenar, porque os `id-N` do pack
        // derivam dela e não podem depender do `read_dir` do SO.
        let docs = arvore_servicos("ordem", &[("b-svc", "Segundo"), ("a-svc", "Primeiro")]);
        let servicos = preparar("xx", &docs, Kind::Servicos).unwrap().unwrap();
        assert_eq!(servicos.len(), 2);
        let titulos: Vec<&str> = servicos.iter().map(|s| s.titulo.as_str()).collect();
        assert_eq!(
            titulos,
            vec!["Primeiro", "Segundo"],
            "ordem = nome de arquivo"
        );
        let s = &servicos[0];
        // A trilha vem PRÉ-COMPOSTA da árvore. `id` e `orgao` não atravessam para o `Documento`:
        // são campos de borda (D-B2), e nem a key nem o bloco os usam — quem precisa deles olha o
        // `Servico`, no `process`.
        assert_eq!(s.trilha, "Cidadãos | IPVA");
        assert_eq!(s.link, "https://x/Primeiro");
        assert_eq!(
            s.corpo, "descrição de Primeiro",
            "corpo == `## corpo` do `.md`"
        );
        assert_eq!(s.kind, Kind::Servicos);
        // `text_to_embed` recomposto pelo ponto único do contrato, com as partes da trilha.
        let (tipo, classe) = auli_contract::partes_trilha_servico(&s.trilha);
        assert_eq!(
            s.text_to_embed,
            auli_contract::compose_servico_text_to_embed(tipo, classe, &s.titulo, &s.corpo)
        );
        let _ = std::fs::remove_dir_all(docs.parent().unwrap());
    }

    #[test]
    fn preparar_servicos_none_sem_arvore() {
        // Sem árvore, a coleção é pulada pelo chamador.
        let base =
            std::env::temp_dir().join(format!("auli-update-svc-vazio-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        assert!(
            preparar("xx", &base.join("docs"), Kind::Servicos)
                .unwrap()
                .is_none()
        );
        // Árvore existente porém vazia também é `None` — melhor que gerar um pack vazio.
        let docs = arvore_servicos("semdocs", &[]);
        assert!(preparar("xx", &docs, Kind::Servicos).unwrap().is_none());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn docs_hash_cobre_a_arvore_de_servicos_sozinha() {
        // D11: entidade que só tem serviços também ganha `docs_hash` — antes ele só existia se
        // houvesse pareceres, e a árvore de serviços ficaria fora da guarda de boot.
        let docs = arvore_servicos("hash", &[("a-svc", "Um")]);
        assert!(
            preparar("xx", &docs, Kind::Pareceres).unwrap().is_none(),
            "sem pareceres"
        );
        assert!(
            manifest::hash_docs_tree(&docs).unwrap().is_some(),
            "mas com hash"
        );
        let _ = std::fs::remove_dir_all(docs.parent().unwrap());
    }

    #[test]
    fn documento_ilegivel_e_erro_alto() {
        let docs = arvore("ruim", &[("A 1", Some("s"))]);
        std::fs::write(docs.join("pareceres/ruim.md"), "sem frontmatter").unwrap();
        let err = preparar("xx", &docs, Kind::Pareceres)
            .unwrap_err()
            .to_string();
        assert!(err.contains("não parseia"), "erro: {err}");
    }
}
