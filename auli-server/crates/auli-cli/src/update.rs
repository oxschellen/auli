//! `auli update` — the vectorizer / pack builder (the only writer).
//!
//! For each content kind it reads the `.md` tree in `<out>/../docs/<kind>/`, embeds each record's
//! `text_to_embed()` and stores its `stored_repr()` — via `auli-core::embed`, the SAME encoder the
//! server uses on the query — identifies each record by its `doc_path`, and applies the batch into
//! `<out>/<entity>-<kind>.json`. Finally writes `<out>/<entity>.manifest.json` stamping the
//! embedding identity, so the server can validate it at boot.
//!
//! **A jurisprudência não é re-vetorizada por inteiro a cada rodada** (Fase 3 da
//! TAREFA-UPDATE-INCREMENTAL): cada record guarda o `key_hash` da key que o produziu, e o documento
//! cuja key não mudou tem o vetor **reaproveitado** do pack anterior. Serviços e faqs seguem em
//! reescrita total, por doutrina (D-INC-8) — quem decide é o `Kind`, não uma flag.
//!
//! O cache inteiro é descartado quando a identidade de embedding muda (Fase 5): a key não sabe qual
//! modelo a vetorizou, então sem essa guarda o pack misturaria dois espaços vetoriais em silêncio.
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

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use auli_contract::{Documento, Embeddable, Kind};
use auli_core::embed::{EMBED_DIM, EMBED_MAX_TOKENS, Embedder};
use auli_core::manifest::{self, CollectionEntry, Manifest};
use vector_store::{CollectionData, Record, Writer};

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
    // FASE 5 — a guarda que passa por cima de todas as outras (D-INC-2 + Fase 5 da TAREFA).
    //
    // O cache do incremental casa por `key_hash`, que é hash do `text_to_embed`. Trocar de MODELO
    // não muda o `text_to_embed`: a key bate, o hash bate, e o cache serviria vetores de dois
    // espaços vetoriais misturados no mesmo pack. A `DimensionMismatch` da store não pega isso se a
    // dimensão coincidir — e modelos diferentes com 1024 dimensões são o caso comum, não a exceção.
    // É o único defeito deste desenho que nasce silencioso e contamina o acervo inteiro.
    //
    // Daí a comparação ser do TRIPLO de identidade completo, e não só do modelo: `STRATEGY_VERSION`
    // muda o que entra no vetor, e um pack embedado sob outra estratégia está igualmente errado.
    let cache_valido = manifest::read_manifest(manifest::manifest_path(&out, &entity))
        .map(|anterior| anterior.embed_identity() == manifest::identity())
        .unwrap_or(false); // sem manifesto anterior (ou ilegível) não há cache em que confiar
    if !cache_valido {
        println!(
            "🧊 Cache de vetores DESLIGADO — não há manifesto anterior, ou a identidade de \
             embedding mudou. Tudo será re-vetorizado."
        );
    }

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
        entries.push(ingest_items(
            &embedder,
            &writer,
            &entity,
            kind,
            &docs,
            &out,
            cache_valido,
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
        pack_format: manifest::PACK_FORMAT,
        collections: entries,
        docs_hash,
    };
    let mpath = manifest::manifest_path(&out, &entity);
    manifest::write_manifest(&mpath, &manifest)?;
    println!("📝 Manifesto escrito em {:?}", mpath);

    Ok(())
}

/// O que o caminho de ingestão usa do embedder — e **nada além disso**.
///
/// A costura existe por um motivo só: sem ela o [`ingest_items`] só roda com o `Embedder` real, e o
/// teste que decide a Fase 3 — "o incremental e a reescrita total produzem o MESMO arquivo, byte a
/// byte" — exigiria carregar 2 GB de modelo num teste unitário, ou seja, não existiria. A
/// propriedade passava a valer só por construção, que é precisamente o que um teste defende contra
/// regressão futura.
///
/// **Não é uma abstração de embedder**: é o recorte mínimo que torna este caminho testável, com os
/// dois métodos que ele chama e mais nenhum. Por isso mora aqui, ao lado de quem o usa, e não em
/// `auli-core` — lá viraria interface pública de um conceito que só tem uma implementação real.
pub(crate) trait Vetorizador {
    fn embed_dense(&self, textos: Vec<String>) -> Result<Vec<Vec<f32>>>;
    fn conta_tokens(&self, texto: &str) -> Result<usize>;
}

impl Vetorizador for Embedder {
    fn embed_dense(&self, textos: Vec<String>) -> Result<Vec<Vec<f32>>> {
        Ok(Embedder::embed_dense(self, textos)?)
    }

    fn conta_tokens(&self, texto: &str) -> Result<usize> {
        Ok(Embedder::conta_tokens(self, texto)?)
    }
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
fn avisar_truncados(embedder: &dyn Vetorizador, kind: Kind, docs: &[&Documento]) -> Result<()> {
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
/// A ordem é estável de propósito, e o motivo mudou na Fase 1: o id não é mais posicional, é o
/// `doc_path`, então trocar dois documentos de lugar aqui não renomeia registro nenhum. O que a
/// ordenação sustenta hoje é a **ordem canônica de escrita** (D-INC-4) — o pack é gravado ordenado
/// por `id`, e como o `id` É o nome do arquivo, ler a árvore em ordem de nome é o que faz aquela
/// ordenação ser um no-op em vez de um reembaralhamento por rodada. Fora do pack, é o que mantém
/// determinístico o que o `update` imprime e a ordem em que os documentos são embedados.
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
pub(crate) fn preparar(
    entity: &str,
    docs_dir: &Path,
    kind: Kind,
) -> Result<Option<Vec<Documento>>> {
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
pub(crate) fn documento_de(
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

/// O hash da key de um documento — a chave do cache de vetores.
///
/// FNV-1a sobre a KEY, não sobre o arquivo: o `.md` carrega o `resumo_gerada_em`, que muda a cada
/// regeneração de sinopse sem mudar uma vírgula do que é embedado. Hashear o arquivo invalidaria o
/// cache inteiro dos pareceres a cada rodada de sinopse.
fn key_hash(doc: &Documento) -> String {
    manifest::hash_hex(doc.text_to_embed().as_bytes())
}

/// Casa cada documento com o vetor dele e monta o record que vai ao pack.
///
/// Função à parte, e não um `map` embutido no [`ingest_items`], porque é aqui que moram as duas
/// afirmações da Fase 1 — o id é o `doc_path`, e o `key_hash` é da KEY — e testá-las não pode exigir
/// carregar o modelo de 2 GB. Os comprimentos já foram conferidos pelo chamador.
///
/// **O payload é SEMPRE reconstruído da árvore** (D-INC-3), mesmo quando o vetor vem do cache. Há
/// campos no payload que não entram na key — o `link` da jurisprudência é o caso concreto —, então
/// um portal que reorganize URLs sem mudar conteúdo faria o `key_hash` bater com um `link` morto ao
/// lado. Reaproveitar payload seria uma classe inteira de staleness silenciosa; só o VETOR é cache.
fn montar_records(items: &[Documento], embeddings: Vec<Vec<f32>>) -> Vec<Record<String>> {
    items
        .iter()
        .zip(embeddings)
        .map(|(doc, embedding)| Record {
            id: doc.doc_path(),
            embedding,
            payload: doc.stored_repr(),
            key_hash: Some(key_hash(doc)),
        })
        .collect()
}

/// Os vetores do pack atual que ainda servem: `doc_path → embedding` para os documentos cuja key
/// não mudou desde a última rodada.
///
/// Devolve vazio quando o cache está desligado (Fase 5) ou a coleção é de política total (D-INC-8);
/// nesses casos o chamador re-embeda tudo, sem nenhum outro ramo de código.
///
/// Casa por `(id, key_hash)`: o id diz que é o mesmo documento, o `key_hash` diz que a key dele não
/// mudou. Record sem `key_hash` — o pack do formato 1 — nunca casa, então um pack antigo degrada
/// para reescrita total em vez de quebrar.
///
/// **O payload é DESCARTADO na desserialização** (`IgnoredAny`): esta função usa só
/// `(id, key_hash, embedding)`, e no TARF o payload é o `stored_repr` com a fundamentação inteira do
/// acórdão — até 27 mil caracteres por registro. Materializá-lo aqui é o pack inteiro em memória
/// para nada.
///
/// **Não é daqui que vem o pico de RSS do `update`**, e a medição do RS diz isso: 10,72 GiB antes e
/// 10,72 GiB depois desta mudança. O pico é do `apply` — que precisa do payload — somado à árvore
/// inteira viva em `items`; o que esta linha corta é lixo transitório, liberado bem antes. Numa
/// REGENERAÇÃO ela nem chega a ler o arquivo: com o cache desligado a função retorna na primeira
/// linha. A dívida de memória do RS é outra, e é assunto de outra TAREFA.
fn vetores_reaproveitaveis(
    writer: &Writer,
    name: &str,
    items: &[Documento],
    habilitado: bool,
) -> Result<HashMap<String, Vec<f32>>> {
    if !habilitado {
        return Ok(HashMap::new());
    }
    type SemPayload = serde::de::IgnoredAny;
    let atual: CollectionData<SemPayload> =
        vector_store::read_collection_file(writer.path_for(name))?;
    let mut por_id: HashMap<&str, &Record<SemPayload>> =
        HashMap::with_capacity(atual.records.len());
    for r in &atual.records {
        por_id.insert(r.id.as_str(), r);
    }
    let mut reaproveitaveis = HashMap::new();
    for doc in items {
        let id = doc.doc_path();
        if let Some(r) = por_id.get(id.as_str())
            && r.key_hash.as_deref() == Some(key_hash(doc).as_str())
        {
            reaproveitaveis.insert(id, r.embedding.clone());
        }
    }
    Ok(reaproveitaveis)
}

/// Os **órfãos** de uma coleção: registros no pack sem `.md` correspondente na árvore.
///
/// Devolve `(doc_path, titulo, link)` por órfão, em ordem de `doc_path`. O título e o link saem do
/// payload do próprio pack — é a única fonte que ainda descreve um documento que já sumiu da árvore.
/// Payload ilegível não derruba a rodada: entra no log com os campos vazios, porque a informação que
/// importa (a chave) está no `id`, e recusar aqui seria bloquear a limpeza por causa do lixo que ela
/// vai limpar.
pub(crate) fn orfaos_do_pack(
    caminho_do_pack: &Path,
    ids_na_arvore: &std::collections::HashSet<String>,
) -> Result<Vec<(String, String, String)>> {
    let atual: CollectionData<String> = vector_store::read_collection_file(caminho_do_pack)?;
    let mut orfaos: Vec<(String, String, String)> = atual
        .records
        .iter()
        .filter(|r| !ids_na_arvore.contains(&r.id))
        .map(|r| {
            let pack: Option<auli_contract::DocumentoPack> = serde_json::from_str(&r.payload).ok();
            match pack {
                Some(p) => (r.id.clone(), p.titulo, p.link),
                None => (r.id.clone(), String::new(), String::new()),
            }
        })
        .collect();
    orfaos.sort();
    Ok(orfaos)
}

/// Caminho do log de remoções pendentes de uma coleção. Vive ao lado do pack que ele descreve.
pub(crate) fn caminho_log_remocoes(out: &Path, name: &str) -> PathBuf {
    out.join(format!("{name}-remocoes-pendentes.tsv"))
}

/// Regenera o log de remoções pendentes a partir do diff da rodada (D-INC-9).
///
/// **Regenerado, não acumulado**: documento que reaparecer na árvore sai do log sozinho, sem
/// ninguém para lembrar de tirá-lo. Sem órfãos, o arquivo é APAGADO — log velho que sobrevive ao
/// problema é pior que log nenhum, porque alguém vai aprová-lo depois achando que está atual.
///
/// TSV com cabeçalho `#`, no mesmo idioma do `tarf-anomalias.txt`: legível por máquina e por gente,
/// e editável à mão — é o que habilita a aprovação seletiva por linha do D-INC-10.
///
/// Escrito com a mesma disciplina do pack (D-INC-5): `.tmp` + `rename`. O log não é lido no boot,
/// então um truncamento não derrubaria o servidor — mas este é o arquivo que AUTORIZA apagar
/// documento, e meia linha nele é confusão exatamente onde ela custa mais caro.
pub(crate) fn escrever_log_remocoes(
    out: &Path,
    name: &str,
    entity: &str,
    orfaos: &[(String, String, String)],
) -> Result<()> {
    let caminho = caminho_log_remocoes(out, name);
    if orfaos.is_empty() {
        if caminho.exists() {
            std::fs::remove_file(&caminho)?;
            println!("🧹 {name}: nenhum órfão — log de remoções pendentes apagado");
        }
        return Ok(());
    }
    let mut texto = format!(
        "# doc_path\ttitulo\tlink — ÓRFÃOS de `{name}`: estão no pack e não estão mais na árvore.\n\
         # NADA foi removido. Gerado por `auli update` a cada rodada; a decisão é humana.\n\
         # Para autorizar: APAGUE as linhas que NÃO quer remover e rode\n\
         #   auli remover --entity {entity} --out <dir-dos-packs>\n\
         # O `remover` CONSOME este arquivo. Linha não aprovada volta no próximo `auli update` —\n\
         # não aprovar hoje é \"não agora\", não uma decisão permanente.\n"
    );
    for (doc_path, titulo, link) in orfaos {
        texto.push_str(&format!("{doc_path}\t{titulo}\t{link}\n"));
    }
    let tmp = caminho.with_extension("tsv.tmp");
    std::fs::write(&tmp, texto)?;
    std::fs::rename(&tmp, &caminho)?;
    println!(
        "📋 {name}: {} órfão(s) no pack sem `.md` na árvore — NADA foi removido. Revise {}",
        orfaos.len(),
        caminho.display()
    );
    Ok(())
}

/// Carimba uma coleção no manifesto: contagem, tamanho e hash de integridade do arquivo escrito.
///
/// Extraída do `run_update` porque o subcomando de remoção (D-INC-11) escreve o mesmo pack e precisa
/// recarimbar do mesmo jeito. Duas cópias divergentes de um carimbo de integridade é exatamente o
/// tipo de coisa que só aparece quando o boot recusa e ninguém sabe por quê.
pub(crate) fn carimbar_colecao(
    out: &Path,
    kind: Kind,
    name: &str,
    total: u64,
) -> Result<CollectionEntry> {
    let file_name = format!("{name}.json");
    let written = std::fs::read(out.join(&file_name))?;
    Ok(CollectionEntry {
        kind: kind.as_str().to_string(),
        count: total as usize,
        dim: EMBED_DIM,
        file: file_name,
        bytes: written.len() as u64,
        hash: manifest::hash_hex(&written),
    })
}

/// Ingest one collection into the entity's `<entity>-<kind>` vector collection.
///
/// Núcleo do ingest: embeda `text_to_embed`, guarda `stored_repr` e devolve a entrada de manifesto.
///
/// **A identidade do registro é o `doc_path`** (D-INC-1), não mais o `id-N` posicional. O id
/// posicional tornava o upsert incremental impossível por construção: inserir um documento no meio
/// da ordem alfabética deslocava a identidade de todos os seguintes, e por isso o `reset` era
/// obrigatório. O `doc_path` é único nas quatro coleções, derivado e nunca digitado — e É o nome do
/// arquivo na árvore, então o pack passa a ser navegável contra ela a olho nu.
///
/// **Cada record carrega o `key_hash`** (D-INC-2): o hash da key que produziu aquele vetor. É o que
/// permite, na rodada seguinte, saber que um documento não mudou **sem re-embeddá-lo** — a única
/// forma de responder essa pergunta antes desta fase era pagar o embedding para descobrir.
///
/// **Duas políticas, um formato** (D-INC-8): a jurisprudência reaproveita vetor por `key_hash`;
/// serviços e faqs são reescritos por inteiro, sempre. Quem decide é o `Kind`, não uma flag.
fn ingest_items(
    embedder: &dyn Vetorizador,
    writer: &Writer,
    entity: &str,
    kind: Kind,
    items: &[Documento],
    out: &Path,
    cache_valido: bool,
) -> Result<CollectionEntry> {
    let name = format!("{}-{}", entity, kind.as_str());
    let incremental = kind.pack_incremental() && cache_valido;
    let reaproveitados = vetores_reaproveitaveis(writer, &name, items, incremental)?;

    // Os documentos que sobraram — os que mudaram, os inéditos, e todos quando não há cache.
    let a_embedar: Vec<&Documento> = items
        .iter()
        .filter(|d| !reaproveitados.contains_key(&d.doc_path()))
        .collect();

    println!(
        "🔢 {}: {} registros — {} reaproveitados, {} a vetorizar{}",
        kind,
        items.len(),
        reaproveitados.len(),
        a_embedar.len(),
        if incremental {
            ""
        } else {
            " (reescrita total)"
        }
    );
    // Só o DELTA é tokenizado: o aviso de truncamento custa uma passada de tokenizer por documento,
    // e no TARF eram 22,5 mil por rodada para avisar sobre os poucos que mudaram.
    avisar_truncados(embedder, kind, &a_embedar)?;

    let embeddings = embedder.embed_dense(
        a_embedar
            .iter()
            .map(|d| d.text_to_embed().to_string())
            .collect(),
    )?;
    // O `zip` abaixo casaria com o MENOR dos dois e descartaria o resto em silêncio. Este é o ponto
    // onde o descasamento pode nascer — o embedder é quem devolve a outra ponta —, e é por isso que
    // a conferência mora aqui e não na store: lá, `Vec<Record>` já torna o descasamento
    // inexpressável (D-INC-14).
    if embeddings.len() != a_embedar.len() {
        return Err(crate::error::Error::Custom(format!(
            "embedder devolveu {} vetores para {} documentos em '{}' — abortado antes de gravar, \
             porque casar os dois truncaria o acervo em silêncio",
            embeddings.len(),
            a_embedar.len(),
            name
        )));
    }
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

    // Recompõe a ordem da árvore: cada documento pega o vetor do cache ou o recém-calculado. O
    // `novos` é consumido na ordem em que foi embedado, que é a de `a_embedar`.
    let mut novos = embeddings.into_iter();
    let vetores: Vec<Vec<f32>> = items
        .iter()
        .map(|d| match reaproveitados.get(&d.doc_path()) {
            Some(v) => v.clone(),
            None => novos.next().expect("um vetor por documento a embedar"),
        })
        .collect();
    let records = montar_records(items, vetores);

    // O log de órfãos é lido do pack ANTES da escrita, porque depois dela os documentos da árvore já
    // teriam sido aplicados por cima. Só no caminho incremental: na reescrita total o `reset` abaixo
    // apaga tudo o que não está na árvore, e aí não há órfão a relatar — é rebuild, não diff.
    if incremental {
        let na_arvore: std::collections::HashSet<String> =
            items.iter().map(|d| d.doc_path()).collect();
        let orfaos = orfaos_do_pack(&writer.path_for(&name), &na_arvore)?;
        escrever_log_remocoes(out, &name, entity, &orfaos)?;
    }

    // Reescrita total continua sendo `reset` + aplicação; no incremental o `reset` sairia caro e
    // erraria: apagaria os órfãos, que por doutrina (D-INC-9) só saem por decisão humana.
    if !incremental {
        writer.reset::<String>(&name)?;
    }
    let total = writer.apply(&name, records, &[])?;

    println!("✅ {} → {} registros", name, total);
    carimbar_colecao(out, kind, &name, total)
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
            escrever_doc(&dir, numero, *sinopse);
        }
        base.join("docs")
    }

    /// Escreve (ou sobrescreve) o `.md` de um documento de jurisprudência na árvore `dir`.
    fn escrever_doc(dir: &Path, numero: &str, sinopse: Option<&str>) {
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
        std::fs::write(
            dir.join(format!("{}.md", mddoc::slug(numero))),
            mddoc::render_doc(&header, sinopse, &format!("corpo de {numero}")),
        )
        .unwrap();
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
        // A ordem casa com a canônica do pack (D-INC-4); ela não pode depender do read_dir do SO.
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
        // Gravados fora de ordem alfabética: a leitura tem que reordenar, para casar com a ordem
        // canônica do pack (D-INC-4) e não depender do `read_dir` do SO.
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

    /// **O coração da Fase 1**: o id do record é o `doc_path`, que É o nome do `.md` na árvore.
    ///
    /// O que isto impede é o defeito que tornava o incremental impossível: com `id-N` posicional,
    /// inserir um documento no meio da ordem alfabética renomeava a identidade de todos os
    /// seguintes, e o pack só podia ser reescrito inteiro.
    #[test]
    fn o_id_do_record_e_o_nome_do_arquivo_na_arvore() {
        let docs = arvore(
            "ids",
            &[("A 1", Some("s")), ("B 2", Some("s")), ("C 3", Some("s"))],
        );
        let items = preparar("xx", &docs, Kind::Pareceres).unwrap().unwrap();
        let embeddings = vec![vec![1.0]; items.len()];
        let records = montar_records(&items, embeddings);

        let ids: Vec<&str> = records.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(
            ids,
            vec![
                "docs/pareceres/a-1.md",
                "docs/pareceres/b-2.md",
                "docs/pareceres/c-3.md"
            ]
        );
        // E o nome no id existe MESMO na árvore — o pack é navegável contra ela.
        for r in &records {
            let nome = r.id.rsplit('/').next().unwrap();
            assert!(
                docs.join("pareceres").join(nome).exists(),
                "id `{}` deveria nomear um arquivo da árvore",
                r.id
            );
        }
        let _ = std::fs::remove_dir_all(docs.parent().unwrap());
    }

    #[test]
    fn o_key_hash_e_da_key_e_nao_do_arquivo() {
        // A distinção que decide se o cache da Fase 3 funciona: o `.md` guarda `resumo_gerada_em`,
        // que muda a cada regeneração de sinopse sem mudar o que é embedado. Hash do ARQUIVO
        // invalidaria o acervo inteiro dos pareceres a cada rodada; hash da KEY, nenhum registro.
        let docs = arvore("hash", &[("A 1", Some("sinopse"))]);
        let items = preparar("xx", &docs, Kind::Pareceres).unwrap().unwrap();
        let records = montar_records(&items, vec![vec![1.0]]);

        assert_eq!(
            records[0].key_hash.as_deref(),
            Some(manifest::hash_hex(items[0].text_to_embed().as_bytes())).as_deref()
        );

        // Mesma key ⇒ mesmo hash (é o que permite reaproveitar o vetor).
        let iguais = montar_records(&items, vec![vec![9.0]]);
        assert_eq!(records[0].key_hash, iguais[0].key_hash);

        // Sinopse diferente ⇒ key diferente ⇒ hash diferente (é o que força o re-embed).
        let outros = arvore("hash2", &[("A 1", Some("OUTRA sinopse"))]);
        let mudados = preparar("xx", &outros, Kind::Pareceres).unwrap().unwrap();
        let mudados = montar_records(&mudados, vec![vec![1.0]]);
        assert_ne!(records[0].key_hash, mudados[0].key_hash);
        assert_eq!(records[0].id, mudados[0].id, "mas a identidade é a mesma");

        let _ = std::fs::remove_dir_all(docs.parent().unwrap());
        let _ = std::fs::remove_dir_all(outros.parent().unwrap());
    }

    /// Escreve um pack no formato 2 direto em disco, para exercitar o cache sem carregar o modelo.
    fn pack_com(dir: &Path, name: &str, entradas: &[(&Documento, &str, f32)]) -> Writer {
        let w = Writer::new(dir);
        let records: Vec<Record<String>> = entradas
            .iter()
            .map(|(doc, kh, v)| Record {
                id: doc.doc_path(),
                embedding: vec![*v],
                payload: doc.stored_repr(),
                key_hash: Some((*kh).to_string()),
            })
            .collect();
        vector_store::write_collection_file(w.path_for(name), &CollectionData { records }).unwrap();
        w
    }

    #[test]
    fn o_cache_so_reaproveita_quem_tem_a_mesma_key() {
        let docs = arvore("cache", &[("A 1", Some("s")), ("B 2", Some("s"))]);
        let items = preparar("xx", &docs, Kind::Pareceres).unwrap().unwrap();
        let dir = std::env::temp_dir().join(format!("auli-cache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);

        // O primeiro tem a key de hoje; o segundo tem a de uma rodada anterior.
        let w = pack_com(
            &dir,
            "xx-pareceres",
            &[
                (&items[0], &key_hash(&items[0]), 1.0),
                (&items[1], "hash-de-ontem", 2.0),
            ],
        );

        let cache = vetores_reaproveitaveis(&w, "xx-pareceres", &items, true).unwrap();
        assert_eq!(cache.len(), 1, "só o de key inalterada entra");
        assert_eq!(cache.get(&items[0].doc_path()), Some(&vec![1.0]));
        assert!(
            !cache.contains_key(&items[1].doc_path()),
            "key mudou ⇒ o vetor tem de ser recalculado, não reaproveitado"
        );

        // Desligado (Fase 5, ou coleção de política total) ⇒ nada é reaproveitado, sem outro ramo.
        assert!(
            vetores_reaproveitaveis(&w, "xx-pareceres", &items, false)
                .unwrap()
                .is_empty()
        );
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(docs.parent().unwrap());
    }

    #[test]
    fn pack_do_formato_1_nunca_casa_e_degrada_para_reescrita_total() {
        // Record sem `key_hash` é o do formato 1. Ele não pode casar por acidente: o vetor dele pode
        // ter vindo de outra estratégia, e reaproveitá-lo seria o defeito silencioso da Fase 5.
        let docs = arvore("fmt1", &[("A 1", Some("s"))]);
        let items = preparar("xx", &docs, Kind::Pareceres).unwrap().unwrap();
        let dir = std::env::temp_dir().join(format!("auli-fmt1-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let w = Writer::new(&dir);
        vector_store::write_collection_file(
            w.path_for("xx-pareceres"),
            &CollectionData {
                records: vec![Record {
                    id: items[0].doc_path(),
                    embedding: vec![9.0],
                    payload: items[0].stored_repr(),
                    key_hash: None, // formato 1
                }],
            },
        )
        .unwrap();

        assert!(
            vetores_reaproveitaveis(&w, "xx-pareceres", &items, true)
                .unwrap()
                .is_empty()
        );
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(docs.parent().unwrap());
    }

    #[test]
    fn a_politica_de_pack_e_por_familia_de_colecao() {
        // D-INC-8: serviços e faqs são reescritos por inteiro, sempre — quem decide é o `Kind`.
        assert!(Kind::Pareceres.pack_incremental());
        assert!(Kind::Tarf.pack_incremental());
        assert!(!Kind::Servicos.pack_incremental());
        assert!(!Kind::Faqs.pack_incremental());
    }

    /// Embedder determinístico: o vetor é função pura do texto. É essa pureza que permite exigir
    /// igualdade byte a byte entre os dois caminhos — no real ela vem do modelo, que é exatamente o
    /// que não cabe num teste unitário.
    struct EmbedderFalso;

    impl Vetorizador for EmbedderFalso {
        fn embed_dense(&self, textos: Vec<String>) -> Result<Vec<Vec<f32>>> {
            Ok(textos
                .iter()
                .map(|t| vec![(manifest::fnv1a64(t.as_bytes()) % 10_000) as f32 / 1e4; EMBED_DIM])
                .collect())
        }

        fn conta_tokens(&self, texto: &str) -> Result<usize> {
            Ok(texto.chars().count())
        }
    }

    /// Uma rodada de ingestão de `pareceres`: lê a árvore `docs` e escreve o pack em `out`.
    fn ingerir(out: &Path, docs: &Path, cache_valido: bool) {
        let items = preparar("xx", docs, Kind::Pareceres).unwrap().unwrap();
        ingest_items(
            &EmbedderFalso,
            &Writer::new(out),
            "xx",
            Kind::Pareceres,
            &items,
            out,
            cache_valido,
        )
        .unwrap();
    }

    fn bytes_do_pack(out: &Path) -> Vec<u8> {
        std::fs::read(out.join("xx-pareceres.json")).unwrap()
    }

    /// A divergência entre dois packs, se houver — em vez de um `assert_eq` que despejaria dezenas
    /// de milhares de bytes de JSON no console e não diria nada.
    fn diferenca(a: &[u8], b: &[u8]) -> Option<String> {
        if a == b {
            return None;
        }
        let i = a
            .iter()
            .zip(b)
            .position(|(x, y)| x != y)
            .unwrap_or(a.len().min(b.len()));
        Some(format!(
            "{} vs {} bytes, primeira divergência no byte {i}",
            a.len(),
            b.len()
        ))
    }

    fn ids_do_pack(out: &Path) -> Vec<String> {
        let d: CollectionData<String> =
            vector_store::read_collection_file(out.join("xx-pareceres.json")).unwrap();
        d.records.into_iter().map(|r| r.id).collect()
    }

    /// **O teste que decide a Fase 3**: chegar a um estado por rodadas incrementais produz o MESMO
    /// arquivo que reescrevê-lo do zero — byte a byte, não "equivalente".
    ///
    /// A propriedade vale por construção (a escrita ordena por `id`, o conjunto de records é o
    /// mesmo, e o vetor do cache é bit-idêntico ao que seria recalculado), e "por construção" é
    /// precisamente o que um teste defende contra regressão futura: qualquer coisa que passe a
    /// entrar no pack só num dos caminhos quebra aqui.
    #[test]
    fn incremental_e_reescrita_total_produzem_o_mesmo_arquivo() {
        /// O que acontece com a árvore entre o pack de ontem e a rodada de hoje.
        type Mutacao = fn(&Path);
        let cenarios: [(&str, Mutacao); 4] = [
            ("nada-muda", |_| {}),
            ("doc-novo", |dir| escrever_doc(dir, "C 3", Some("s"))),
            ("resumo-novo", |dir| {
                escrever_doc(dir, "B 2", Some("OUTRA sinopse"))
            }),
            ("os-dois", |dir| {
                escrever_doc(dir, "C 3", Some("s"));
                escrever_doc(dir, "B 2", Some("OUTRA sinopse"));
            }),
        ];

        for (tag, mudar_a_arvore) in cenarios {
            let docs = arvore(
                &format!("equiv-{tag}"),
                &[("A 1", Some("s")), ("B 2", Some("s"))],
            );
            let base = docs.parent().unwrap().to_path_buf();
            let (inc, zero) = (base.join("packs-inc"), base.join("packs-zero"));

            ingerir(&inc, &docs, false); // o pack de ontem
            mudar_a_arvore(&docs.join("pareceres"));
            ingerir(&inc, &docs, true); // a rodada incremental de hoje
            ingerir(&zero, &docs, false); // o mesmo estado, reescrito do zero

            if let Some(d) = diferenca(&bytes_do_pack(&inc), &bytes_do_pack(&zero)) {
                panic!("cenário `{tag}`: os dois caminhos têm de convergir byte a byte — {d}");
            }
            let _ = std::fs::remove_dir_all(&base);
        }
    }

    /// O caso de borda que a igualdade acima **não** cobre, e que é comportamento correto: com
    /// órfão os dois caminhos DIVERGEM.
    ///
    /// A reescrita total apaga o que sumiu da árvore; o incremental o preserva e escreve o log para
    /// um humano decidir (D-INC-9). Nomear a divergência aqui é o que impede alguém de "consertar"
    /// a assimetria depois, achando que o incremental está deixando lixo para trás.
    #[test]
    fn com_orfao_os_dois_caminhos_divergem_e_isso_e_correto() {
        let docs = arvore("orfao", &[("A 1", Some("s")), ("B 2", Some("s"))]);
        let base = docs.parent().unwrap().to_path_buf();
        let (inc, zero) = (base.join("packs-inc"), base.join("packs-zero"));

        ingerir(&inc, &docs, false);
        std::fs::remove_file(docs.join("pareceres/b-2.md")).unwrap(); // B sai da árvore
        ingerir(&inc, &docs, true);
        ingerir(&zero, &docs, false);

        assert!(
            diferenca(&bytes_do_pack(&inc), &bytes_do_pack(&zero)).is_some(),
            "com órfão a igualdade byte a byte NÃO vale — e não deve valer"
        );
        assert_eq!(
            ids_do_pack(&inc),
            vec!["docs/pareceres/a-1.md", "docs/pareceres/b-2.md"],
            "o incremental PRESERVA o órfão: só um humano pode removê-lo"
        );
        assert_eq!(
            ids_do_pack(&zero),
            vec!["docs/pareceres/a-1.md"],
            "a reescrita total apaga o que não está na árvore — é rebuild, não diff"
        );
        // E a divergência não fica muda: o incremental deixa o órfão registrado para revisão.
        let log = std::fs::read_to_string(caminho_log_remocoes(&inc, "xx-pareceres")).unwrap();
        assert!(log.contains("docs/pareceres/b-2.md"), "log: {log}");
        assert!(
            !caminho_log_remocoes(&zero, "xx-pareceres").exists(),
            "na reescrita total não há órfão a relatar"
        );

        let _ = std::fs::remove_dir_all(&base);
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
