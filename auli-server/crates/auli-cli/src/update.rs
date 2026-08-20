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
use auli_core::embed::{EMBED_DIM, Embedder};
use auli_core::manifest::{self, CollectionEntry, Manifest};
use vector_store::{CollectionData, Record, Writer};

use crate::error::Result;

pub fn run_update(
    entity: String,
    out: PathBuf,
    version: Option<String>,
    kinds: Vec<String>,
) -> Result<()> {
    dotenvy::dotenv().ok();

    // Resolve o filtro ANTES de carregar o modelo: nome de coleção errado é erro de chamada e tem
    // de falhar em milissegundos, não depois de 2 GB de embedder na memória.
    let filtro = resolver_kinds(&kinds)?;

    // A árvore `docs/` é a fonte das CINCO coleções, por isso o caminho é montado antes dos blocos.
    let docs_dir = out
        .parent()
        .ok_or_else(|| format!("diretório de packs sem pai: {}", out.display()))?
        .join("docs");

    // O manifesto anterior serve a TRÊS leitores: o `cache_valido` (Fase 5), as guardas do parcial
    // (D-DH-4) e a mesclagem (D-DH-3). Lido uma vez.
    let anterior = manifest::read_manifest(manifest::manifest_path(&out, &entity)).ok();

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
    let cache_valido = anterior
        .as_ref()
        .map(|a| a.embed_identity() == manifest::identity())
        .unwrap_or(false); // sem manifesto anterior (ou ilegível) não há cache em que confiar

    // Guardas da rodada parcial (D-DH-4) — todas ANTES do embedder, para falhar barato.
    if let Some(sel) = &filtro {
        let Some(anterior) = anterior.as_ref() else {
            return Err(crate::error::Error::Custom(format!(
                "--kind exige um manifesto anterior de '{entity}' e não há nenhum legível em {} — \
                 a rodada parcial PRESERVA o que não rodou, e sem manifesto não existe o que \
                 preservar. Rode o `auli update` completo primeiro.",
                out.display()
            )));
        };
        if anterior.docs_hashes.is_none() {
            return Err(crate::error::Error::Custom(format!(
                "--kind exige o mapa `docs_hashes` no manifesto anterior de '{entity}', e ele não \
                 está lá (manifesto anterior à migração). Rode o `auli update` completo uma vez — \
                 ele recarimba o mapa; a jurisprudência é reaproveitada pelo key_hash."
            )));
        }
        if !cache_valido {
            return Err(crate::error::Error::Custom(format!(
                "--kind com identidade de embedding divergente do manifesto anterior de \
                 '{entity}'. A rodada parcial carimbaria a identidade NOVA num manifesto cujos \
                 outros packs são do espaço velho — e o boot os abençoaria, que é exatamente o \
                 defeito que a guarda existe para impedir. Mudança de identidade exige o `auli \
                 update` completo."
            )));
        }
        println!(
            "🎯 Rodada parcial: {}",
            sel.iter()
                .map(|k| k.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    } else if !cache_valido {
        println!(
            "🧊 Cache de vetores DESLIGADO — não há manifesto anterior, ou a identidade de \
             embedding mudou. Tudo será re-vetorizado."
        );
    }

    let cache_dir = std::env::var("EMBED_CACHE_DIR").unwrap_or_else(|_| "./models".to_string());
    let threads: usize = std::env::var("EMBED_THREADS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(16);

    println!("🧠 Carregando embedder (BGE-M3) de '{}'...", cache_dir);
    let embedder = Embedder::new(cache_dir.into(), threads)?;
    let writer = Writer::new(&out);
    let mut entries: Vec<CollectionEntry> = Vec::new();

    // **UM caminho de código para as cinco coleções** (D-B2). A fonte é sempre a árvore
    // `docs/<kind>/*.md`, o registro é sempre um `Documento`, e o que difere — como o `.md` vira
    // struct, e se há guarda de resumo — está dentro do `preparar`. Coleção sem árvore é pulada —
    // EXCETO quando foi pedida por `--kind`: pedido explícito de árvore inexistente é quase sempre
    // engano, e prosseguir escreveria um manifesto "parcial de nada" que só confundiria.
    //
    // **Um pack POR coleção** (D-A2): `<id>-pareceres` e `<id>-tarf` nunca se misturam. São acervos
    // de natureza diferente (interpretação da lei × julgamento de recurso) e o usuário escolhe qual
    // consultar; fundi-los tiraria essa escolha dele e diluiria as duas buscas.
    for kind in Kind::TODOS {
        if filtro.as_ref().is_some_and(|sel| !sel.contains(&kind)) {
            continue;
        }
        let Some(docs) = preparar(&entity, &docs_dir, kind)? else {
            if filtro.is_some() {
                return Err(crate::error::Error::Custom(format!(
                    "--kind {kind}: a entidade '{entity}' não tem árvore em {} — nada a vetorizar.",
                    docs_dir.join(kind.as_str()).display()
                )));
            }
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

    // O mapa do DISCO, uma varredura só, com dois usos: fornecer o hash novo dos kinds que rodaram
    // (a regra central diz que só ELES são recarimbados) e, na rodada parcial, avisar sobre
    // árvores que andaram fora do filtro. `None` = `docs/` não existe (entidade sem árvore).
    let disco = manifest::hash_docs_map(&docs_dir)?;

    let manifest = match &filtro {
        None => Manifest {
            entity: entity.clone(),
            version: version.unwrap_or_else(|| "1".to_string()),
            built_at: chrono::Utc::now().to_rfc3339(),
            embed_model_id: manifest::EMBED_MODEL_ID.to_string(),
            embed_dim: EMBED_DIM,
            strategy_version: manifest::STRATEGY_VERSION,
            pack_format: manifest::PACK_FORMAT,
            collections: entries,
            // Rodada COMPLETA sincronizou cada pack com sua árvore nesta passada, então recarimbar
            // o mapa inteiro do disco é exatamente a regra central aplicada a todos de uma vez.
            docs_hashes: disco,
        },
        Some(sel) => {
            let anterior =
                anterior.expect("as guardas do parcial exigiram o manifesto — ver início");
            // `docs/` existe: o `preparar` acabou de ler dela para cada kind do filtro.
            let disco = disco.unwrap_or_default();
            let m = mesclar_parcial(anterior, entries, sel, &disco, version);
            avisar_arvores_fora_do_filtro(&m, &disco, sel);
            m
        }
    };

    let mpath = manifest::manifest_path(&out, &entity);
    manifest::write_manifest(&mpath, &manifest)?;
    println!("📝 Manifesto escrito em {:?}", mpath);

    Ok(())
}

/// `--kind` textual → `Kind`, com erro alto para nome desconhecido (D-DH-7) e dedup preservando a
/// ordem de chegada. Vec vazio ⇒ `None` ⇒ rodada completa, o comportamento de sempre.
fn resolver_kinds(kinds: &[String]) -> Result<Option<Vec<Kind>>> {
    if kinds.is_empty() {
        return Ok(None);
    }
    let mut sel: Vec<Kind> = Vec::new();
    for nome in kinds {
        let Some(kind) = Kind::parse(nome.trim()) else {
            return Err(crate::error::Error::Custom(format!(
                "--kind '{nome}': coleção desconhecida. As válidas: {}.",
                Kind::TODOS
                    .iter()
                    .map(|k| k.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        };
        if !sel.contains(&kind) {
            sel.push(kind);
        }
    }
    Ok(Some(sel))
}

/// Monta o manifesto de uma rodada PARCIAL (D-DH-3): parte do anterior, substitui as entradas e os
/// hashes SÓ dos kinds que rodaram, e preserva verbatim todo o resto — inclusive diretórios que não
/// são coleção e hashes que já não batem com o disco. É a preservação que faz o boot recusar
/// NOMEANDO a árvore que andou fora do filtro; copiar o disco aqui reproduziria o defeito do
/// agregado, só que particionado.
///
/// A identidade de embedding sai das constantes locais, não do anterior — a guarda D-DH-4 já provou
/// que são iguais, e carimbar a local mantém a regra "quem escreve carimba o que É".
///
/// Pura (sem I/O, sem embedder) de propósito: é ela que os testes de mutação exercitam.
fn mesclar_parcial(
    anterior: Manifest,
    novas: Vec<CollectionEntry>,
    rodados: &[Kind],
    disco: &manifest::DocsHashes,
    version: Option<String>,
) -> Manifest {
    let mut collections = anterior.collections;
    for entrada in novas {
        match collections.iter_mut().find(|c| c.kind == entrada.kind) {
            Some(c) => *c = entrada,
            None => collections.push(entrada),
        }
    }
    let mut docs_hashes = anterior.docs_hashes.unwrap_or_default();
    for kind in rodados {
        match disco.get(kind.as_str()) {
            Some(h) => {
                docs_hashes.insert(kind.as_str().to_string(), h.clone());
            }
            // Inalcançável no fluxo (o `preparar` errou alto se a árvore pedida não existe), mas o
            // braço honesto é remover a chave: não existe "pack reconstruído de árvore que sumiu".
            None => {
                docs_hashes.remove(kind.as_str());
            }
        }
    }
    Manifest {
        entity: anterior.entity,
        // Sem `--version`, a versão anterior é MANTIDA — a rodada parcial não é uma release nova
        // por si; quem quiser marcar uma, passa a flag.
        version: version.unwrap_or(anterior.version),
        built_at: chrono::Utc::now().to_rfc3339(),
        embed_model_id: manifest::EMBED_MODEL_ID.to_string(),
        embed_dim: EMBED_DIM,
        strategy_version: manifest::STRATEGY_VERSION,
        pack_format: manifest::PACK_FORMAT,
        collections,
        docs_hashes: Some(docs_hashes),
    }
}

/// A cortesia da rodada parcial: compara o que foi PRESERVADO com o disco e avisa o que o boot vai
/// recusar. Aviso, nunca erro — o fluxo legítimo de "mexi em duas árvores" é exatamente rodar dois
/// `--kind` em sequência, e o primeiro não pode falhar por causa do segundo.
fn avisar_arvores_fora_do_filtro(
    manifesto: &Manifest,
    disco: &manifest::DocsHashes,
    rodados: &[Kind],
) {
    let rodados: std::collections::HashSet<&str> = rodados.iter().map(|k| k.as_str()).collect();
    let vazio = manifest::DocsHashes::new();
    let preservado = manifesto.docs_hashes.as_ref().unwrap_or(&vazio);
    let mut avisos: Vec<String> = Vec::new();
    for (dir, h) in disco {
        if rodados.contains(dir.as_str()) {
            continue;
        }
        match preservado.get(dir) {
            Some(p) if p == h => {}
            Some(_) => avisos.push(format!("docs/{dir}/ mudou desde o carimbo preservado")),
            None => avisos.push(format!("docs/{dir}/ existe e não está no manifesto")),
        }
    }
    for dir in preservado.keys() {
        if !rodados.contains(dir.as_str()) && !disco.contains_key(dir) {
            avisos.push(format!("docs/{dir}/ está no manifesto e sumiu do disco"));
        }
    }
    if avisos.is_empty() {
        return;
    }
    println!("⚠️  Árvores fora do filtro divergentes — o boot vai recusar até serem recarimbadas:");
    for a in &avisos {
        println!("   - {a}");
    }
    println!("   Rode `auli update --kind <colecao>` para cada uma (ou o update completo).");
}

/// O que o caminho de ingestão usa do embedder — e **nada além disso**.
///
/// A costura existe por um motivo só: sem ela o [`ingest_items`] só roda com o `Embedder` real, e o
/// teste que decide a Fase 3 — "o incremental e a reescrita total produzem o MESMO arquivo, byte a
/// byte" — exigiria carregar 2 GB de modelo num teste unitário, ou seja, não existiria. A
/// propriedade passava a valer só por construção, que é precisamente o que um teste defende contra
/// regressão futura.
///
/// **Não é uma abstração de embedder**: é o recorte mínimo que torna este caminho testável, com o
/// único método que ele chama. Por isso mora aqui, ao lado de quem o usa, e não em `auli-core` — lá
/// viraria interface pública de um conceito que só tem uma implementação real.
///
/// (Tinha dois métodos até a TAREFA-TETO: o `conta_tokens` servia ao aviso de truncamento do
/// encoder, que deixou de ser alcançável quando a key ganhou teto no contrato.)
pub(crate) trait Vetorizador {
    fn embed_dense(&self, textos: Vec<String>) -> Result<Vec<Vec<f32>>>;
}

impl Vetorizador for Embedder {
    fn embed_dense(&self, textos: Vec<String>) -> Result<Vec<Vec<f32>>> {
        Ok(Embedder::embed_dense(self, textos)?)
    }
}

/// Quantas keys da coleção bateram no teto do contrato — **estatística da rodada, não alarme**.
///
/// Até a TAREFA-TETO isto avisava quem passava dos `EMBED_MAX_TOKENS` do encoder: evento raro, e por
/// isso o aviso significava "aconteceu algo excepcional". Com o teto de
/// [`TETO_TEXT_TO_EMBED`](auli_contract::TETO_TEXT_TO_EMBED) caracteres no contrato, cortar é o
/// caminho **normal** de ~2% do TARF a cada rodada — um alarme ali viraria ruído que ninguém lê.
///
/// E o aviso antigo ficou **impossível por construção**: 6.000 caracteres são ~1.500 a 2.000 tokens,
/// bem abaixo dos 8.192 do modelo, então o encoder não trunca mais nada. Alarme que não pode
/// disparar é código morto (CLAUDE.md §3), e por isso a checagem saiu junto com o tokenizer.
///
/// Cobre TODOS os documentos da coleção, não só o delta a vetorizar: sem tokenizer, contar é de
/// graça, e o número que interessa é o da coleção.
fn relatar_teto(kind: Kind, items: &[Documento]) {
    let cortadas = items
        .iter()
        .filter(|d| d.text_to_embed().chars().count() >= auli_contract::TETO_TEXT_TO_EMBED)
        .count();
    if cortadas > 0 {
        println!(
            "✂️  {kind}: {cortadas} de {} keys no teto de {} chars — a cauda não entra no vetor",
            items.len(),
            auli_contract::TETO_TEXT_TO_EMBED
        );
    }
}

/// Os `.md` de um diretório, em ordem de CAMINHO. `None` se o diretório não existe ou está vazio —
/// nos dois casos a coleção é pulada, o que é melhor que gerar um pack vazio.
///
/// `com_subpastas` (de [`Kind::arvore_com_subpastas`], D-LEG-3) liga a descida de **um** nível, que
/// só a legislação usa: lá a subpasta é uma lei. Segundo nível é erro alto, não arquivo ignorado —
/// um `.md` esquecido dois níveis abaixo sumiria da coleção sem nada na tela.
///
/// A ordem é estável de propósito, e o motivo mudou na Fase 1: o id não é mais posicional, é o
/// `doc_path`, então trocar dois documentos de lugar aqui não renomeia registro nenhum. O que a
/// ordenação sustenta hoje é a **ordem canônica de escrita** (D-INC-4) — o pack é gravado ordenado
/// por `id`, e como o `id` É o caminho relativo do arquivo, ler a árvore nessa ordem é o que faz
/// aquela ordenação ser um no-op em vez de um reembaralhamento por rodada. Fora do pack, é o que
/// mantém determinístico o que o `update` imprime e a ordem em que os documentos são embedados.
///
/// **A ordenação é pela STRING do caminho relativo, não pelo `Path`.** Com árvore plana as duas
/// coincidem; com subpasta, não: `Path` compara componente a componente, e o pack compara o `id`
/// como string (`write.rs`, `a.id.cmp(&b.id)`). Em `"Lei A"` vs `"Lei A-B"` as duas ordens se
/// invertem — `/` (0x2F) é maior que `-` (0x2D) —, e aí a ordenação do pack deixaria de ser no-op.
fn arquivos_md(dir: &Path, com_subpastas: bool) -> Result<Option<Vec<PathBuf>>> {
    if !dir.exists() {
        return Ok(None);
    }
    let mut achados: Vec<(String, PathBuf)> = Vec::new();
    for entrada in std::fs::read_dir(dir)? {
        let caminho = entrada?.path();
        if !caminho.is_dir() {
            empilhar_md(dir, caminho, &mut achados);
            continue;
        }
        if !com_subpastas {
            continue;
        }
        for entrada in std::fs::read_dir(&caminho)? {
            let neto = entrada?.path();
            if neto.is_dir() {
                return Err(format!(
                    "`{}` é uma subpasta de segundo nível, e a árvore desta coleção só tem um \
                     nível (uma pasta por lei). Achate a árvore — um `.md` mais fundo que isso \
                     não seria lido por ninguém.",
                    neto.display()
                )
                .into());
            }
            empilhar_md(dir, neto, &mut achados);
        }
    }
    achados.sort_by(|(a, _), (b, _)| a.cmp(b));
    let caminhos: Vec<PathBuf> = achados.into_iter().map(|(_, p)| p).collect();
    Ok((!caminhos.is_empty()).then_some(caminhos))
}

/// Guarda o `.md` junto da chave de ordenação: o caminho relativo a `dir`, como string.
fn empilhar_md(dir: &Path, caminho: PathBuf, out: &mut Vec<(String, PathBuf)>) {
    if caminho.extension().is_some_and(|e| e == "md") {
        let chave = caminho
            .strip_prefix(dir)
            .unwrap_or(&caminho)
            .to_string_lossy()
            .into_owned();
        out.push((chave, caminho));
    }
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
    let Some(caminhos) = arquivos_md(&dir, kind.arvore_com_subpastas())? else {
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
        let doc = documento_de(kind, header, resumo, corpo);
        conferir_doc_path(docs_dir, caminho, &doc)?;
        docs.push(doc);
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

/// **A guarda do invariante do `doc_path`** (D-LEG-4): o caminho DERIVADO do documento tem que ser
/// o caminho de onde ele foi REALMENTE lido.
///
/// O `doc_path` é o id do registro no pack e é por ele que o serving lê o corpo, tarde, na query.
/// Quando os dois divergem nada quebra no build nem no boot: o pack fica com um caminho que não
/// existe, e o sintoma aparece só na tela do usuário, como corpo indisponível — o mesmo defeito
/// silencioso que o `Documento::doc_path` existe para evitar, mas agora do outro lado, no dado.
///
/// Vale para as CINCO coleções, não só para a legislação. Nas outras quatro o nome sai de campos
/// que o próprio arquivo carrega (título, link), então o invariante é praticamente tautológico —
/// medido na adoção: 48.408 arquivos das 28 entidades, zero divergências. Na legislação ele deixa
/// de ser tautológico, porque a subpasta sai da TRILHA: uma trilha com o 1º segmento errado escreve
/// um `doc_path` que aponta para outra lei. É por isso que a guarda nasce aqui, mas é por valer
/// para todas que ela também protege um `.md` renomeado à mão nas outras.
fn conferir_doc_path(docs_dir: &Path, lido: &Path, doc: &Documento) -> Result<()> {
    let real = format!(
        "docs/{}",
        lido.strip_prefix(docs_dir).unwrap_or(lido).display()
    );
    let derivado = doc.doc_path();
    if derivado != real {
        return Err(format!(
            "`{}` está em conflito com o próprio conteúdo: o caminho derivado do frontmatter é \
             `{derivado}`, mas o arquivo foi lido de `{real}`. Corrija o DADO — renomeie o arquivo, \
             ou ajuste o campo de onde o caminho sai (na legislação, o 1º segmento da `trilha` é o \
             nome da subpasta). O `doc_path` é o id no pack e o caminho por onde o corpo é lido na \
             query: divergente, ele vira corpo indisponível sem erro nenhum.",
            lido.display()
        )
        .into());
    }
    Ok(())
}

/// Um `.md` lido vira [`Documento`]. A única coisa que difere por coleção é a fórmula da key — o
/// resto do mapeamento é o vocabulário unificado caindo 1:1.
///
/// As quatro fórmulas continuam sendo as do contrato, congeladas: mudar qualquer uma re-vetoriza
/// os packs daquela coleção.
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
        // Legislação é a FAQ mais o dispositivo no slot da ementa (D-LEG-7): a pergunta é o
        // `titulo`, a resposta é o `## corpo`, e a `ementa` traz o "Art. 21, I".
        Kind::Legislacao => auli_contract::compose_legislacao_text_to_embed(
            &header.trilha,
            &header.titulo,
            &header.ementa,
            &corpo,
        ),
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
    // recusaria pelo `docs_hashes`, corretamente. Daí o aviso de reinício.
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
    relatar_teto(kind, items);

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
    fn arvore_faqs(tag: &str, docs: &[(&str, &str)]) -> PathBuf {
        let base =
            std::env::temp_dir().join(format!("auli-update-faq-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let dir = base.join("docs").join("faqs");
        std::fs::create_dir_all(&dir).unwrap();
        for (pergunta, origin) in docs {
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
                dir.join(format!("{}.md", mddoc::nome_faq_de(pergunta, &header.link))),
                mddoc::render_doc(&header, None, &corpo),
            )
            .unwrap();
        }
        base.join("docs")
    }

    #[test]
    fn preparar_faqs_le_arvore_em_ordem() {
        // Gravadas fora de ordem alfabética: a leitura reordena, para o pack ser reproduzível.
        let docs = arvore_faqs("ordem", &[("Segunda", ""), ("Primeira", "Inicial | FAQ")]);
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
    fn arvore_servicos(tag: &str, docs: &[&str]) -> PathBuf {
        let base =
            std::env::temp_dir().join(format!("auli-update-svc-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let dir = base.join("docs").join("servicos");
        std::fs::create_dir_all(&dir).unwrap();
        for titulo in docs {
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
                dir.join(format!(
                    "{}.md",
                    mddoc::nome_servico_de(titulo, &header.link)
                )),
                mddoc::render_doc(&header, None, &corpo),
            )
            .unwrap();
        }
        base.join("docs")
    }

    #[test]
    fn preparar_servicos_le_arvore_em_ordem() {
        // Gravados fora de ordem alfabética: a leitura tem que reordenar, para casar com a ordem
        // canônica do pack (D-INC-4) e não depender do `read_dir` do SO. Os nomes de arquivo saem
        // das MESMAS funções que materializam a árvore — desde a guarda do `doc_path`, uma árvore
        // com nome arbitrário não é mais um cenário possível, e sim erro alto.
        let docs = arvore_servicos("ordem", &["Segundo", "Primeiro"]);
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
    fn mapa_cobre_a_arvore_de_servicos_sozinha() {
        // D11, na forma do mapa: entidade que só tem serviços também ganha entrada — a guarda de
        // boot a protege de graça, como antes.
        let docs = arvore_servicos("hash", &["Um"]);
        assert!(
            preparar("xx", &docs, Kind::Pareceres).unwrap().is_none(),
            "sem pareceres"
        );
        let mapa = manifest::hash_docs_map(&docs).unwrap().unwrap();
        assert!(mapa.contains_key("servicos"), "mas com entrada no mapa");
        let _ = std::fs::remove_dir_all(docs.parent().unwrap());
    }

    #[test]
    fn resolver_kinds_vazio_e_rodada_completa() {
        assert_eq!(resolver_kinds(&[]).unwrap(), None);
    }

    #[test]
    fn resolver_kinds_recusa_nome_desconhecido_e_lista_os_validos() {
        let e = resolver_kinds(&["faq".into()]).unwrap_err().to_string();
        assert!(e.contains("'faq'"), "erro nomeia o pedido: {e}");
        assert!(e.contains("faqs"), "e lista as válidas: {e}");
    }

    #[test]
    fn resolver_kinds_deduplica_preservando_a_ordem() {
        let sel = resolver_kinds(&["tarf".into(), "faqs".into(), "tarf".into()])
            .unwrap()
            .unwrap();
        assert_eq!(sel, vec![Kind::Tarf, Kind::Faqs]);
    }

    /// Manifesto anterior de brinquedo: duas coleções carimbadas e um diretório SEM pack, que é o
    /// caso que a cobertura por omissão protege — ele tem hash e nenhuma `CollectionEntry`.
    fn manifesto_anterior() -> Manifest {
        let entrada = |kind: &str| CollectionEntry {
            kind: kind.into(),
            count: 1,
            dim: EMBED_DIM,
            file: format!("xx-{kind}.json"),
            bytes: 10,
            hash: format!("hash-antigo-{kind}"),
        };
        let mut mapa = manifest::DocsHashes::new();
        mapa.insert("faqs".into(), "arvore-antiga-faqs".into());
        mapa.insert("tarf".into(), "arvore-antiga-tarf".into());
        mapa.insert("sem-pack".into(), "arvore-antiga-sem-pack".into());
        Manifest {
            entity: "xx".into(),
            version: "7".into(),
            built_at: "antes".into(),
            embed_model_id: manifest::EMBED_MODEL_ID.into(),
            embed_dim: EMBED_DIM,
            strategy_version: manifest::STRATEGY_VERSION,
            pack_format: manifest::PACK_FORMAT,
            collections: vec![entrada("faqs"), entrada("tarf")],
            docs_hashes: Some(mapa),
        }
    }

    #[test]
    fn parcial_substitui_so_o_que_rodou_e_preserva_o_resto_verbatim() {
        let nova = CollectionEntry {
            kind: "faqs".into(),
            count: 2,
            dim: EMBED_DIM,
            file: "xx-faqs.json".into(),
            bytes: 20,
            hash: "hash-novo-faqs".into(),
        };
        let mut disco = manifest::DocsHashes::new();
        disco.insert("faqs".into(), "arvore-nova-faqs".into());
        // O disco também tem as outras DIFERENTES do carimbo — a regra central (D-DH-3) é NÃO
        // copiá-las: é a preservação que faz o boot recusá-las nomeando-as.
        disco.insert("tarf".into(), "arvore-nova-tarf".into());
        disco.insert("sem-pack".into(), "arvore-nova-sem-pack".into());

        let m = mesclar_parcial(
            manifesto_anterior(),
            vec![nova],
            &[Kind::Faqs],
            &disco,
            None,
        );

        let mapa = m.docs_hashes.unwrap();
        assert_eq!(
            mapa["faqs"], "arvore-nova-faqs",
            "o que rodou é recarimbado do disco"
        );
        // Verificado por mutação: trocar a preservação por cópia do disco mata estes dois asserts
        // — e é exatamente o defeito do agregado, particionado.
        assert_eq!(
            mapa["tarf"], "arvore-antiga-tarf",
            "fora do filtro viaja verbatim"
        );
        assert_eq!(
            mapa["sem-pack"], "arvore-antiga-sem-pack",
            "diretório sem pack também"
        );

        let faq = m.collections.iter().find(|c| c.kind == "faqs").unwrap();
        assert_eq!(faq.hash, "hash-novo-faqs");
        let tarf = m.collections.iter().find(|c| c.kind == "tarf").unwrap();
        assert_eq!(
            tarf.hash, "hash-antigo-tarf",
            "entrada fora do filtro intacta"
        );
        assert_eq!(m.version, "7", "sem --version, a versão anterior é mantida");
    }

    #[test]
    fn parcial_de_colecao_nova_acrescenta_entrada_e_chave() {
        // Primeira vez de uma coleção numa entidade (ex.: `legislacao` estreando): a entrada é
        // ACRESCENTADA e a chave do mapa idem — sem exigir rodada completa.
        let nova = CollectionEntry {
            kind: "legislacao".into(),
            count: 509,
            dim: EMBED_DIM,
            file: "xx-legislacao.json".into(),
            bytes: 30,
            hash: "hash-legislacao".into(),
        };
        let mut disco = manifest::DocsHashes::new();
        disco.insert("legislacao".into(), "arvore-legislacao".into());

        let m = mesclar_parcial(
            manifesto_anterior(),
            vec![nova],
            &[Kind::Legislacao],
            &disco,
            Some("8".into()),
        );
        assert!(m.collections.iter().any(|c| c.kind == "legislacao"));
        assert_eq!(m.docs_hashes.unwrap()["legislacao"], "arvore-legislacao");
        assert_eq!(m.version, "8", "--version explícita vence");
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

    /// Escreve um par P/R de legislação em `docs/legislacao/<pasta>/`. `pasta` é dada à parte de
    /// propósito: é ela que os testes da guarda põem em conflito com a trilha.
    fn arvore_legislacao(tag: &str, pasta: &str, trilha: &str, perguntas: &[&str]) -> PathBuf {
        let base =
            std::env::temp_dir().join(format!("auli-update-leg-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let dir = base
            .join("docs")
            .join(Kind::Legislacao.as_str())
            .join(pasta);
        std::fs::create_dir_all(&dir).unwrap();
        for p in perguntas {
            let header = mddoc::DocHeader {
                titulo: (*p).into(),
                trilha: trilha.into(),
                ementa: "Art. 1º".into(),
                link: "http://x/lei".into(),
                orgao: None,
                resumo_info: None,
            };
            std::fs::write(
                dir.join(format!("{}.md", mddoc::nome_faq_de(p, &header.link))),
                mddoc::render_doc(&header, None, &format!("resposta de {p}")),
            )
            .unwrap();
        }
        base.join("docs")
    }

    /// A leitura recursiva (D-LEG-3) e o `doc_path` com subpasta, no caminho feliz.
    #[test]
    fn legislacao_le_a_arvore_com_subpasta_e_grava_o_doc_path_dela() {
        let lei = "Lei 6.537-1973 — PTA";
        let trilha = format!("{lei} | Título I | Capítulo I");
        let docs = arvore_legislacao("feliz", lei, &trilha, &["Qual o prazo?", "Cabe recurso?"]);

        let leg = preparar("xx", &docs, Kind::Legislacao).unwrap().unwrap();
        assert_eq!(leg.len(), 2, "os dois arquivos da subpasta foram lidos");
        for d in &leg {
            assert!(
                d.doc_path().starts_with(&format!("docs/legislacao/{lei}/")),
                "doc_path: {}",
                d.doc_path()
            );
            // A key é a da legislação, com o dispositivo no slot da ementa.
            assert_eq!(
                d.text_to_embed,
                auli_contract::compose_legislacao_text_to_embed(
                    &d.trilha, &d.titulo, &d.ementa, &d.corpo
                )
            );
        }
        // Ordem determinística pelo caminho relativo completo (D-INC-4).
        let mut caminhos: Vec<String> = leg.iter().map(|d| d.doc_path()).collect();
        let ordenados = {
            let mut c = caminhos.clone();
            c.sort();
            c
        };
        assert_eq!(caminhos, ordenados, "a árvore é lida em ordem de caminho");
        caminhos.clear();

        let _ = std::fs::remove_dir_all(docs.parent().unwrap());
    }

    /// A ordem de leitura tem que ser a MESMA do `id` no pack (D-INC-4), e o pack ordena o `id`
    /// como STRING (`write.rs`). Este par de leis é o caso em que ordenar `Path` — componente a
    /// componente — dá resposta diferente: `/` (0x2F) > `-` (0x2D), então na string `"Lei A-B/…"`
    /// vem ANTES de `"Lei A/…"`, e por componente vem depois. Com a ordenação errada a escrita do
    /// pack deixa de ser no-op e reembaralha registro a cada rodada.
    #[test]
    fn a_ordem_de_leitura_e_a_do_id_no_pack_mesmo_com_leis_de_nome_prefixo() {
        let base =
            std::env::temp_dir().join(format!("auli-update-leg-{}-ordem", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let raiz = base.join("docs").join(Kind::Legislacao.as_str());
        for lei in ["Lei A", "Lei A-B"] {
            let dir = raiz.join(lei);
            std::fs::create_dir_all(&dir).unwrap();
            let titulo = format!("Pergunta de {lei}?");
            let header = mddoc::DocHeader {
                titulo: titulo.clone(),
                trilha: format!("{lei} | Título I"),
                ementa: "Art. 1º".into(),
                link: "http://x/lei".into(),
                orgao: None,
                resumo_info: None,
            };
            std::fs::write(
                dir.join(format!("{}.md", mddoc::nome_faq_de(&titulo, &header.link))),
                mddoc::render_doc(&header, None, "resposta"),
            )
            .unwrap();
        }

        let docs = base.join("docs");
        let lidos = preparar("xx", &docs, Kind::Legislacao).unwrap().unwrap();
        let caminhos: Vec<String> = lidos.iter().map(|d| d.doc_path()).collect();
        let mut como_o_pack_ordena = caminhos.clone();
        como_o_pack_ordena.sort(); // String::cmp — o mesmo do `a.id.cmp(&b.id)`
        assert_eq!(caminhos, como_o_pack_ordena);
        assert!(
            caminhos[0].contains("Lei A-B/"),
            "a lei de nome mais longo vem primeiro na ordem de string: {caminhos:?}"
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    /// A guarda do D-LEG-4, no defeito que ela existe para pegar: a trilha diz uma lei e o arquivo
    /// está na pasta de outra. Sem a guarda isso compila, vetoriza e só some na query.
    #[test]
    fn trilha_que_nao_bate_com_a_pasta_e_erro_alto_com_os_dois_caminhos() {
        let docs = arvore_legislacao(
            "conflito",
            "Lei 6.537-1973 — PTA",
            "Lei 8.820-1989 — ICMS | Título I", // ← 1º segmento aponta para OUTRA lei
            &["Qual o prazo?"],
        );
        let err = preparar("xx", &docs, Kind::Legislacao)
            .unwrap_err()
            .to_string();

        // Os dois caminhos na mensagem — sem eles o operador não sabe qual lado corrigir.
        assert!(
            err.contains("docs/legislacao/Lei 8.820-1989 — ICMS/"),
            "{err}"
        );
        assert!(
            err.contains("docs/legislacao/Lei 6.537-1973 — PTA/"),
            "{err}"
        );

        let _ = std::fs::remove_dir_all(docs.parent().unwrap());
    }

    /// A mesma guarda nas coleções planas: um `.md` renomeado à mão deixa de bater com o nome
    /// derivado do título. Aqui o invariante é quase tautológico — e é justamente por isso que a
    /// única forma de quebrá-lo é mexer no dado por fora.
    #[test]
    fn arquivo_renomeado_a_mao_tambem_e_pego_nas_colecoes_planas() {
        let docs = arvore("renomeado", &[("CONSULTA Nº 1/26", Some("s"))]);
        let dir = docs.join("pareceres");
        std::fs::rename(
            dir.join("consulta-no-1-26.md"),
            dir.join("consulta-outro-nome.md"),
        )
        .unwrap();

        let err = preparar("xx", &docs, Kind::Pareceres)
            .unwrap_err()
            .to_string();
        assert!(err.contains("docs/pareceres/consulta-no-1-26.md"), "{err}");
        assert!(
            err.contains("docs/pareceres/consulta-outro-nome.md"),
            "{err}"
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
