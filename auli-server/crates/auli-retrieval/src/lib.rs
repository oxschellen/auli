//! `auli-retrieval` — o motor de recuperação do Auli.
//!
//! Une o que uma consulta semântica precisa: o embedder (BGE-M3, in-process), os `ReadStore`s
//! por `<entidade>-<kind>` e o estreitamento por proximidade. É a peça compartilhada pelas três
//! faces do servidor: o chat (`/v1/question`), o retrieval HTTP (`/v1/retrieve`) e o MCP (`/mcp`).
//!
//! ## Fronteira (D-MCP-2)
//! Depende só de `auli-core`, `vector-store` e `auli-contract`. NUNCA de axum/rmcp/auli-llm/
//! auli-anon. Só enxerga `ReadStore` — é somente-leitura por construção, como o servidor.
//!
//! ## Sincronia (D-MCP-3)
//! Todos os métodos são blocking (o embed é CPU-bound). Chamadores async envelopam em
//! `tokio::task::spawn_blocking`, exatamente como o `rag.rs` já fazia.
//!
//! ## Forma (D-MCP-4)
//! O núcleo são **funções livres** sobre `&Collections`/`&Path`; os métodos de [`Engine`] são
//! delegações de uma linha. Assim os testes exercitam o motor inteiro sem nunca construir um
//! `Embedder` (que carregaria o BGE-M3).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use auli_contract::{DocumentoPack, Kind, bloco, conteudo_indisponivel, mddoc};
use auli_core::embed::Embedder;
use tracing::debug;
use vector_store::ReadStore;

/// Todas as coleções carregadas, chaveadas por `<entidade>-<kind>` (ex.: `rs-faqs`).
/// (Tipo movido de `auli_cli::packs` — o carregamento continua lá; o motor só consome.)
pub type Collections = HashMap<String, Arc<ReadStore<String>>>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("falha ao gerar embedding: {0}")]
    Embed(String),
    #[error("coleção '{0}' não carregada")]
    ColecaoAusente(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Um documento recuperado. `score` é DISTÂNCIA cosseno — menor é mais próximo (contrato do
/// `vector-store`).
#[derive(Debug, Clone)]
pub struct Hit {
    pub payload: String,
    pub score: f32,
}

/// Um parecer recuperado, decodificado do payload leve. `corpo` é opcional: a busca devolve só
/// metadados; `parecer_por_numero` preenche sob demanda lendo a árvore `docs/`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ParecerHit {
    pub numero: String,
    pub assunto: String,
    pub resumo: String,
    pub link: String,
    /// Distância cosseno da busca; `None` quando o parecer foi obtido por número (sem busca).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub corpo: Option<String>,
}

/// O motor. Construir UMA vez no boot (o embedder é pesado) e compartilhar via `Arc`.
pub struct Engine {
    collections: Collections,
    embedder: Arc<Embedder>,
    /// Raiz dos packs: a árvore `docs/` de cada entidade é irmã, em `<docs_root>/<id>/docs/`.
    docs_root: PathBuf,
}

impl Engine {
    pub fn new(collections: Collections, embedder: Arc<Embedder>, docs_root: PathBuf) -> Self {
        Self {
            collections,
            embedder,
            docs_root,
        }
    }

    /// Acesso direto a um store (usado pelo `list_handler` do server). `None` = coleção fora do
    /// mapa (entidade não registrada) — ver a nota de semântica em [`search_embedded`].
    pub fn store(&self, name: &str) -> Option<&Arc<ReadStore<String>>> {
        self.collections.get(name)
    }

    /// Embeda um texto (a pergunta). Blocking — CPU-bound.
    pub fn embed(&self, texto: &str) -> Result<Vec<f32>> {
        self.embedder
            .embed_dense(vec![texto.to_string()])
            .map_err(|e| Error::Embed(e.to_string()))?
            .into_iter()
            .next()
            .ok_or_else(|| Error::Embed("embedder devolveu lote vazio".into()))
    }

    /// Busca com o vetor JÁ pronto. Delegação (D-MCP-4).
    pub fn search_embedded(
        &self,
        collection: &str,
        embedding: &[f32],
        ceiling: usize,
        floor: usize,
        band: f32,
    ) -> Result<Vec<Hit>> {
        search_embedded(
            &self.collections,
            collection,
            embedding,
            ceiling,
            floor,
            band,
        )
    }

    /// Embeda a pergunta e busca. Conveniência para as faces que fazem UMA busca (retrieve/MCP);
    /// o chat continua embedando uma vez e reusando o vetor em duas coleções.
    pub fn search(
        &self,
        collection: &str,
        texto: &str,
        ceiling: usize,
        floor: usize,
        band: f32,
    ) -> Result<Vec<Hit>> {
        let embedding = self.embed(texto)?;
        self.search_embedded(collection, &embedding, ceiling, floor, band)
    }

    /// Busca em `<id>-<kind>` (uma coleção de JURISPRUDÊNCIA) devolvendo hits DECODIFICADOS
    /// (payload leve → campos), sem corpo. Payload que não desserializa não derruba a busca: vira um
    /// hit degradado com o JSON cru no `assunto` (mesma filosofia do `bloco_parecer` de sempre —
    /// nunca propagar erro na query).
    pub fn search_jurisprudencia(
        &self,
        entity_id: &str,
        kind: Kind,
        texto: &str,
        ceiling: usize,
        floor: usize,
        band: f32,
    ) -> Result<Vec<ParecerHit>> {
        debug_assert!(
            kind.exige_resumo(),
            "só jurisprudência tem payload de consulta"
        );
        let collection = format!("{entity_id}-{}", kind.as_str());
        Ok(self
            .search(&collection, texto, ceiling, floor, band)?
            .into_iter()
            .map(|hit| decode_parecer(&hit.payload, Some(hit.score)))
            .collect())
    }

    /// Delegação (D-MCP-4).
    pub fn documento_por_numero(
        &self,
        entity_id: &str,
        kind: Kind,
        numero: &str,
    ) -> Result<Option<ParecerHit>> {
        documento_por_numero(&self.collections, &self.docs_root, entity_id, kind, numero)
    }

    /// Delegação (D-MCP-4).
    pub fn ler_corpo(
        &self,
        entity_id: &str,
        doc_path: &str,
    ) -> std::result::Result<String, String> {
        ler_corpo(&self.docs_root, entity_id, doc_path)
    }

    /// Raiz dos packs/árvore docs (o chat precisa dela para o `bloco_parecer`).
    pub fn docs_root(&self) -> &Path {
        &self.docs_root
    }

    /// Delegação (D-MCP-4).
    pub fn entidades_com(&self, kind: &str) -> Vec<String> {
        entidades_com(&self.collections, kind)
    }

    /// Delegação: pareceres relacionados por co-citação de dispositivos (expansão por grafo do RAG).
    pub fn pareceres_relacionados(
        &self,
        entity_id: &str,
        seeds: &[String],
        max: usize,
        min_shared: usize,
    ) -> Vec<String> {
        pareceres_relacionados(&self.docs_root, entity_id, seeds, max, min_shared)
    }

    /// Delegação: bloco de contexto RAG de um parecer pelo número (mesma renderização do chat).
    pub fn bloco_por_numero(&self, entity_id: &str, numero: &str) -> Result<Option<String>> {
        bloco_por_numero(&self.collections, &self.docs_root, entity_id, numero)
    }
}

// ============================ NÚCLEO PURO (funções livres) ============================
// O coração testável do motor (D-MCP-4): funções sobre `&Collections`/`&Path`, sem Embedder e sem
// estado. Os métodos de `Engine` acima são delegações de uma linha para cá.

/// Busca com vetor pronto.
///
/// SEMÂNTICA DO ERRO: com o `load_all` do `auli-cli`, TODA entidade registrada tem os quatro kinds
/// no mapa — arquivo ausente vira store VAZIO —, então `ColecaoAusente` só dispara para coleção
/// realmente fora do mapa (entidade não registrada). Store vazio é SUCESSO com zero hits; quem
/// precisa distinguir "tem acervo de verdade" usa [`entidades_com`].
pub fn search_embedded(
    collections: &Collections,
    collection: &str,
    embedding: &[f32],
    ceiling: usize,
    floor: usize,
    band: f32,
) -> Result<Vec<Hit>> {
    let store = collections
        .get(collection)
        .ok_or_else(|| Error::ColecaoAusente(collection.to_string()))?;
    let scored = store.query_scored(embedding, ceiling);
    debug!(
        "{collection} scores: {:?}",
        scored.iter().map(|(_, s)| *s).collect::<Vec<_>>()
    );
    Ok(select_by_proximity(scored, floor, band)
        .into_iter()
        .map(|(payload, score)| Hit { payload, score })
        .collect())
}

/// Ids de entidade com a coleção `<id>-<kind>` carregada e NÃO-vazia, ordenados. É o teste de
/// "tem acervo de verdade" (store vazio não conta) — a ferramenta MCP valida a UF por aqui.
pub fn entidades_com(collections: &Collections, kind: &str) -> Vec<String> {
    let sufixo = format!("-{kind}");
    let mut ids: Vec<String> = collections
        .iter()
        .filter(|(name, store)| name.ends_with(&sufixo) && !store.is_empty())
        .map(|(name, _)| name[..name.len() - sufixo.len()].to_string())
        .collect();
    ids.sort_unstable();
    ids
}

/// Localiza um parecer pelo `numero` exato (comparação insensível a caixa), varrendo a lista da
/// coleção. O(n) sobre milhares de registros — irrelevante ao lado do custo de rede do cliente.
/// Preenche o `corpo` lendo a árvore `docs/` (degradação graciosa: corpo ausente vira aviso).
pub fn documento_por_numero(
    collections: &Collections,
    docs_root: &Path,
    entity_id: &str,
    kind: Kind,
    numero: &str,
) -> Result<Option<ParecerHit>> {
    debug_assert!(
        kind.exige_resumo(),
        "só jurisprudência tem payload de consulta"
    );
    let collection = format!("{entity_id}-{}", kind.as_str());
    let store = collections
        .get(&collection)
        .ok_or(Error::ColecaoAusente(collection))?;
    let alvo = numero.trim().to_lowercase();
    for payload_json in store.list() {
        let Ok(payload) = serde_json::from_str::<DocumentoPack>(&payload_json) else {
            continue; // pack incompatível: pula, não derruba
        };
        if payload.titulo.trim().to_lowercase() == alvo {
            let corpo = ler_corpo(docs_root, entity_id, &payload.doc_path)
                .unwrap_or_else(|_| conteudo_indisponivel(&payload));
            return Ok(Some(ParecerHit {
                numero: payload.titulo,
                assunto: payload.ementa,
                resumo: payload.resumo,
                link: payload.link,
                score: None,
                corpo: Some(corpo),
            }));
        }
    }
    Ok(None)
}

/// Bloco de contexto RAG de UM parecer pelo `numero` exato — a MESMA renderização do chat
/// ([`auli_contract::bloco`]), lendo o corpo da árvore `docs/`. `None` se o número não está na
/// coleção (ex.: parecer citado no grafo mas ainda não vetorizado). Usado pela expansão por grafo
/// para incluir pareceres relacionados no mesmo formato dos recuperados por vetor.
pub fn bloco_por_numero(
    collections: &Collections,
    docs_root: &Path,
    entity_id: &str,
    numero: &str,
) -> Result<Option<String>> {
    let collection = format!("{entity_id}-pareceres");
    let store = collections
        .get(&collection)
        .ok_or(Error::ColecaoAusente(collection))?;
    let alvo = numero.trim().to_lowercase();
    for payload_json in store.list() {
        let Ok(payload) = serde_json::from_str::<DocumentoPack>(&payload_json) else {
            continue;
        };
        if payload.titulo.trim().to_lowercase() == alvo {
            let corpo = ler_corpo(docs_root, entity_id, &payload.doc_path)
                .unwrap_or_else(|_| conteudo_indisponivel(&payload));
            return Ok(Some(bloco(&payload, &corpo)));
        }
    }
    Ok(None)
}

/// Uma entrada do `dispositivos-index.json` (do `canonizar`) — só a lista de pareceres interessa.
#[derive(serde::Deserialize)]
struct IdxEntry {
    pareceres: Vec<String>,
}

/// Pareceres relacionados aos `seeds` por **co-citação de dispositivos**: dado o conjunto de
/// pareceres já recuperados (por vetor), devolve outros que citam os MESMOS dispositivos legais,
/// ranqueados por quantos compartilham com a base dos seeds. Sinal complementar ao da similaridade
/// textual — acha pareceres juridicamente conexos que a busca semântica erra.
///
/// Lê `<docs_root>/<id>/extracao/dispositivos-index.json` (saída do `canonizar`). **Ausente ou
/// ilegível → vazio**: a expansão é opcional, então entidade sem grafo se comporta como hoje.
/// Determinístico (empate por número). Exclui os próprios seeds.
pub fn pareceres_relacionados(
    docs_root: &Path,
    entity_id: &str,
    seeds: &[String],
    max: usize,
    min_shared: usize,
) -> Vec<String> {
    if max == 0 || seeds.is_empty() {
        return Vec::new();
    }
    let path = docs_root
        .join(entity_id)
        .join("extracao")
        .join("dispositivos-index.json");
    let Ok(texto) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    let Ok(idx) = serde_json::from_str::<BTreeMap<String, IdxEntry>>(&texto) else {
        return Vec::new();
    };

    // parecer -> dispositivos (invertendo o índice canon_key -> pareceres).
    let mut par2disp: HashMap<&str, HashSet<&str>> = HashMap::new();
    for (k, e) in &idx {
        for p in &e.pareceres {
            par2disp.entry(p.as_str()).or_default().insert(k.as_str());
        }
    }

    let seed_set: HashSet<&str> = seeds.iter().map(String::as_str).collect();
    // União dos dispositivos da base recuperada.
    let mut seed_disp: HashSet<&str> = HashSet::new();
    for s in seeds {
        if let Some(ds) = par2disp.get(s.as_str()) {
            seed_disp.extend(ds.iter().copied());
        }
    }
    if seed_disp.is_empty() {
        return Vec::new();
    }

    // Pontua candidatos (não-seed) por dispositivos compartilhados com a base; ordena e corta.
    let mut scored: Vec<(&str, usize)> = par2disp
        .iter()
        .filter(|(p, _)| !seed_set.contains(**p))
        .map(|(p, ds)| (*p, ds.iter().filter(|d| seed_disp.contains(**d)).count()))
        .filter(|(_, n)| *n >= min_shared)
        .collect();
    scored.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    scored
        .into_iter()
        .take(max)
        .map(|(p, _)| p.to_string())
        .collect()
}

/// Lê o `.md` da árvore da entidade e extrai a seção `## corpo` (parser do contrato).
///
/// **A guarda do caminho (D-MCP-10).** Esta é a única junção de caminho do serving, e os dois
/// segmentos juntados vêm de fora: `entity_id` do chamador e `doc_path` do payload do pack. Ambos
/// são confiáveis hoje — o `doc_path` é DERIVADO (`Documento::doc_path()` sempre produz
/// `docs/{kind}/…`, e o `conferir_doc_path` do `auli update` recusa quem divergir, D-LEG-4) e o
/// `entity_id` é lista-branca em todo chamador. A guarda não corrige buraco aberto: ela para de
/// depender disso. Sem ela, um `doc_path` adulterado com `..` sairia da árvore em silêncio, e um
/// com barra inicial faria o `join` DESCARTAR a raiz (semântica do `Path::join`).
///
/// Lista branca de componentes, não caça a `".."`: um predicado só rejeita `ParentDir`, `RootDir`,
/// prefixo de disco e `CurDir`, sem depender de separador de plataforma nem de codificação. É
/// sintático de propósito — nada de `canonicalize`, que custaria um syscall por documento de toda
/// consulta para decidir uma questão que não depende do disco.
///
/// Rejeição é `Err`, nunca pânico: os dois chamadores (`rag::bloco_documento` e
/// `documento_por_numero`) já degradam para `conteudo_indisponivel` e registram o erro, então a
/// tentativa fica no log e a resposta continua de pé.
pub fn ler_corpo(
    docs_root: &Path,
    entity_id: &str,
    doc_path: &str,
) -> std::result::Result<String, String> {
    let relativo = Path::new(entity_id).join(doc_path);
    if relativo
        .components()
        .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(format!(
            "caminho fora da árvore da entidade: {entity_id:?}/{doc_path:?}"
        ));
    }
    let caminho = docs_root.join(relativo);
    let texto = std::fs::read_to_string(&caminho).map_err(|e| e.to_string())?;
    let (_header, _sinopse, corpo) = mddoc::parse_doc(&texto).map_err(|e| e.to_string())?;
    Ok(corpo)
}

/// Decodifica o payload do pack para o hit de saída. JSON inválido degrada para um hit com o cru
/// no `assunto` — visível, mas sem derrubar a resposta. Pública: o handler `/v1/retrieve` usa.
///
/// Os nomes `numero`/`assunto` do [`ParecerHit`] são **contrato de fio** com os clientes MCP e com
/// o `/v1/retrieve`, e por isso NÃO acompanharam o vocabulário interno: renomeá-los quebraria quem
/// já consome. A tradução acontece aqui, num lugar só.
pub fn decode_parecer(payload_json: &str, score: Option<f32>) -> ParecerHit {
    match serde_json::from_str::<DocumentoPack>(payload_json) {
        Ok(p) => ParecerHit {
            numero: p.titulo,
            assunto: p.ementa,
            resumo: p.resumo,
            link: p.link,
            score,
            corpo: None,
        },
        Err(_) => ParecerHit {
            numero: String::new(),
            assunto: payload_json.to_string(),
            resumo: String::new(),
            link: String::new(),
            score,
            corpo: None,
        },
    }
}

/// Mantém os `floor` primeiros sempre; além disso, mantém docs a até `band` do melhor score.
/// Para cedo assim que um doc cai fora da banda, já que tudo depois dele está mais longe ainda.
///
/// CONTRATO: `scored` PRECISA vir ordenado por score ascendente (melhor/mais próximo primeiro). O
/// `break` depende dessa monotonicidade — em entrada desordenada ele descarta silenciosamente docs
/// dentro da banda. `query_scored` garante a ordem; o `debug_assert!` pega qualquer chamador futuro
/// que não garanta.
pub fn select_by_proximity(
    scored: Vec<(String, f32)>,
    floor: usize,
    band: f32,
) -> Vec<(String, f32)> {
    debug_assert!(
        scored.windows(2).all(|w| w[0].1 <= w[1].1),
        "select_by_proximity requires input sorted by ascending score (best-first)"
    );
    let Some(&(_, best)) = scored.first() else {
        return vec![];
    };
    let mut out = Vec::new();
    for (i, (doc, score)) in scored.into_iter().enumerate() {
        if i < floor || (score - best) <= band {
            out.push((doc, score));
        } else {
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use vector_store::Record;

    /// Testes SEM modelo: usam as FUNÇÕES LIVRES (D-MCP-4) com stores sintéticos.
    /// Nenhum teste deste módulo constrói `Engine` nem `Embedder`.
    fn store_de(v: Vec<(&str, Vec<f32>)>) -> Arc<ReadStore<String>> {
        let records = v
            .into_iter()
            .enumerate()
            .map(|(i, (payload, embedding))| Record {
                id: format!("id-{i}"),
                embedding,
                payload: payload.to_string(),
                key_hash: None,
            })
            .collect();
        Arc::new(ReadStore::from_records(records))
    }

    fn scored(pairs: &[(&str, f32)]) -> Vec<(String, f32)> {
        pairs.iter().map(|(d, s)| (d.to_string(), *s)).collect()
    }

    /// Só os documentos, para os asserts transplantados do `rag.rs` (que comparavam `Vec<String>`).
    fn docs(v: Vec<(String, f32)>) -> Vec<String> {
        v.into_iter().map(|(d, _)| d).collect()
    }

    /// Diretório temporário exclusivo deste teste (nome + pid), removido no fim.
    fn temp_dir(nome: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("auli-retrieval-{nome}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn payload_json(numero: &str, doc_path: &str) -> String {
        serde_json::to_string(&DocumentoPack {
            kind: Kind::Pareceres,
            trilha: String::new(),
            titulo: numero.into(),
            ementa: "ICMS – crédito".into(),
            resumo: "Resumo do parecer.".into(),
            link: "http://x/1".into(),
            doc_path: doc_path.into(),
        })
        .unwrap()
    }

    #[test]
    fn search_embedded_ordena_e_estreita() {
        let mut cols: Collections = Collections::new();
        cols.insert(
            "sc-pareceres".into(),
            store_de(vec![
                ("perto", vec![1.0, 0.0]),
                ("ortogonal", vec![0.0, 1.0]),
                ("oposto", vec![-1.0, 0.0]),
            ]),
        );
        // banda 0.5: só o "perto" (dist 0) entra; "ortogonal" (dist 1) fica fora.
        let hits = search_embedded(&cols, "sc-pareceres", &[1.0, 0.0], 10, 0, 0.5).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].payload, "perto");
        assert!(hits[0].score < 1e-6);
    }

    #[test]
    fn colecao_ausente_e_erro_tipado() {
        let cols = Collections::new();
        let e = search_embedded(&cols, "xx-pareceres", &[1.0], 10, 0, f32::INFINITY).unwrap_err();
        assert!(matches!(e, Error::ColecaoAusente(_)));
    }

    #[test]
    fn store_vazio_e_sucesso_com_zero_hits_nao_erro() {
        // A distinção que a semântica do motor promete: fora do mapa = erro; no mapa e vazio = Ok
        // com zero hits. É o caso REAL do `load_all` (arquivo ausente vira store vazio).
        let mut cols: Collections = Collections::new();
        cols.insert(
            "mg-pareceres".into(),
            Arc::new(ReadStore::from_records(vec![])),
        );
        let hits = search_embedded(&cols, "mg-pareceres", &[1.0], 10, 0, f32::INFINITY).unwrap();
        assert!(hits.is_empty());
    }

    #[test]
    fn decode_parecer_json_valido_e_invalido() {
        let ok = decode_parecer(
            &payload_json("PARECER Nº 1", "docs/pareceres/p1.md"),
            Some(0.1),
        );
        assert_eq!(ok.numero, "PARECER Nº 1");
        assert_eq!(ok.assunto, "ICMS – crédito");
        assert_eq!(ok.score, Some(0.1));
        assert!(ok.corpo.is_none());

        let ruim = decode_parecer("isto não é json", None);
        assert_eq!(ruim.assunto, "isto não é json"); // degradação visível, sem pânico
        assert!(ruim.numero.is_empty());
    }

    #[test]
    fn parecer_por_numero_acha_e_le_o_corpo_da_arvore() {
        // Espelha `bloco_parecer_le_corpo_da_arvore_e_monta_o_bloco` do rag.rs, adaptado à raiz
        // resolvida pelo motor: <docs_root>/<entity_id>/<doc_path>.
        let root = temp_dir("por-numero");
        let pdir = root.join("sc").join("docs/pareceres");
        std::fs::create_dir_all(&pdir).unwrap();
        let header =
            mddoc::cabecalho_jurisprudencia("PARECER Nº 1", "ICMS – crédito", "http://x/1");
        std::fs::write(
            pdir.join("parecer-no-1.md"),
            mddoc::render_doc(&header, None, "É o corpo integral."),
        )
        .unwrap();

        let mut cols: Collections = Collections::new();
        cols.insert(
            "sc-pareceres".into(),
            store_de(vec![(
                payload_json("PARECER Nº 1", "docs/pareceres/parecer-no-1.md").as_str(),
                vec![1.0],
            )]),
        );

        // Caixa DIFERENTE da gravada: a busca é insensível a caixa e a espaços nas bordas.
        let achado = documento_por_numero(&cols, &root, "sc", Kind::Pareceres, "  parecer nº 1  ")
            .unwrap()
            .unwrap();
        assert_eq!(achado.numero, "PARECER Nº 1");
        assert_eq!(achado.corpo.as_deref(), Some("É o corpo integral."));
        assert_eq!(achado.link, "http://x/1");
        assert!(achado.score.is_none(), "obtido por número, não por busca");

        // Miss: número que não existe na coleção.
        assert!(
            documento_por_numero(&cols, &root, "sc", Kind::Pareceres, "PARECER Nº 999")
                .unwrap()
                .is_none()
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn parecer_por_numero_degrada_quando_o_corpo_falta() {
        // Sem árvore materializada: acha o parecer e degrada o corpo para o resumo, nunca erra.
        let root = temp_dir("corpo-ausente");
        let mut cols: Collections = Collections::new();
        cols.insert(
            "sc-pareceres".into(),
            store_de(vec![(
                payload_json("PARECER Nº 1", "docs/pareceres/sumiu.md").as_str(),
                vec![1.0],
            )]),
        );

        let achado = documento_por_numero(&cols, &root, "sc", Kind::Pareceres, "PARECER Nº 1")
            .unwrap()
            .unwrap();
        let corpo = achado.corpo.unwrap();
        assert!(
            corpo.contains("[corpo indisponível — ver link]"),
            "corpo: {corpo}"
        );
        assert!(
            corpo.contains("Resumo do parecer."),
            "usa o resumo no lugar do corpo"
        );
    }

    #[test]
    fn parecer_por_numero_em_colecao_fora_do_mapa_e_erro_tipado() {
        let cols = Collections::new();
        let e = documento_por_numero(
            &cols,
            Path::new("/inexistente"),
            "xx",
            Kind::Pareceres,
            "PARECER Nº 1",
        )
        .unwrap_err();
        assert!(matches!(e, Error::ColecaoAusente(_)));
    }

    /// A trava da D-A6: o `kind` escolhe o STORE, e as duas coleções não se enxergam.
    ///
    /// Sem ela, o modo de falha é mudo nos dois sentidos: pedir um acórdão sem `colecao` devolveria
    /// "não achado" (buscou nos pareceres), e um número que por acaso exista nas duas devolveria o
    /// documento errado, com link e corpo de outro acervo.
    #[test]
    fn o_kind_escolhe_o_store_e_as_colecoes_nao_se_enxergam() {
        let root = temp_dir("kind-escolhe-store");
        let mut cols: Collections = Collections::new();
        cols.insert(
            "rs-pareceres".into(),
            store_de(vec![(
                payload_json("PARECER Nº 1", "docs/pareceres/parecer-no-1.md").as_str(),
                vec![1.0],
            )]),
        );
        cols.insert(
            "rs-tarf".into(),
            store_de(vec![(
                payload_json("ACÓRDÃO 510/24", "docs/tarf/acordao-510-24.md").as_str(),
                vec![1.0],
            )]),
        );

        let achar = |kind, numero| {
            documento_por_numero(&cols, &root, "rs", kind, numero)
                .unwrap()
                .map(|h| h.numero)
        };
        assert_eq!(
            achar(Kind::Tarf, "ACÓRDÃO 510/24"),
            Some("ACÓRDÃO 510/24".to_string())
        );
        assert_eq!(
            achar(Kind::Pareceres, "PARECER Nº 1"),
            Some("PARECER Nº 1".to_string())
        );
        // Cruzado: cada número só existe no próprio acervo.
        assert_eq!(achar(Kind::Pareceres, "ACÓRDÃO 510/24"), None);
        assert_eq!(achar(Kind::Tarf, "PARECER Nº 1"), None);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn ler_corpo_falha_em_arquivo_inexistente() {
        assert!(ler_corpo(Path::new("/nao/existe"), "sc", "docs/pareceres/x.md").is_err());
    }

    /// D-MCP-10: caminho que sai da árvore é recusado ANTES de tocar o disco.
    ///
    /// O alvo plantado fora é um `.md` VÁLIDO — se não fosse, o teste passaria pelo motivo errado
    /// (o `parse_doc` recusaria o formato) e sobreviveria à remoção da guarda. Assim ele morre por
    /// mutação: sem a guarda, o `..` sobe um nível, o arquivo existe, parseia, e o `Ok` mata os
    /// asserts de rejeição.
    #[test]
    fn ler_corpo_recusa_caminho_que_sai_da_arvore() {
        let root = temp_dir("traversal");
        let pdir = root.join("sc").join("docs/pareceres");
        std::fs::create_dir_all(&pdir).unwrap();
        let header = mddoc::cabecalho_jurisprudencia("PARECER Nº 1", "ICMS", "http://x/1");
        let doc = mddoc::render_doc(&header, None, "É o corpo integral.");
        std::fs::write(pdir.join("parecer-no-1.md"), &doc).unwrap();
        // Vizinho legítimo de OUTRA entidade: o alvo clássico do `..`, e um arquivo que parseia.
        std::fs::create_dir_all(root.join("rs")).unwrap();
        std::fs::write(root.join("rs").join("segredo.md"), &doc).unwrap();

        // Caminho feliz: continua lendo.
        assert_eq!(
            ler_corpo(&root, "sc", "docs/pareceres/parecer-no-1.md").unwrap(),
            "É o corpo integral."
        );

        // `..` no doc_path — o arquivo do vizinho EXISTE e parseia; só a guarda o impede.
        let e = ler_corpo(&root, "sc", "../rs/segredo.md").unwrap_err();
        assert!(e.contains("fora da árvore"), "erro inesperado: {e}");

        // `..` no entity_id — o mesmo `join`, o outro segmento.
        assert!(
            ler_corpo(&root, "sc/../rs", "segredo.md")
                .unwrap_err()
                .contains("fora da árvore")
        );

        // Caminho ABSOLUTO: o `join` descartaria a raiz e leria o arquivo apontado.
        let absoluto = root.join("rs").join("segredo.md");
        assert!(
            ler_corpo(&root, "sc", absoluto.to_str().unwrap())
                .unwrap_err()
                .contains("fora da árvore")
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    /// A guarda é sintática e NÃO pode recusar nome legítimo. O caso que mais se parece com um
    /// falso positivo no acervo real é a subpasta da legislação, que tem espaço, ponto e travessão
    /// no nome (`Lei 6.537-1973 — Processo Tributário Administrativo`).
    #[test]
    fn ler_corpo_aceita_o_doc_path_da_legislacao_com_subpasta() {
        let root = temp_dir("subpasta");
        let lei = "Lei 6.537-1973 — Processo Tributário Administrativo";
        let dir = root.join("rs").join("docs/legislacao").join(lei);
        std::fs::create_dir_all(&dir).unwrap();
        let header = mddoc::cabecalho_jurisprudencia("Art. 1º", "PTA", "http://x/1");
        std::fs::write(
            dir.join("par-01.md"),
            mddoc::render_doc(&header, None, "É a resposta."),
        )
        .unwrap();

        assert_eq!(
            ler_corpo(&root, "rs", &format!("docs/legislacao/{lei}/par-01.md")).unwrap(),
            "É a resposta."
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn relacionados_ranqueia_por_dispositivos_compartilhados_e_exclui_seeds() {
        let root = temp_dir("relac");
        let exdir = root.join("rs").join("extracao");
        std::fs::create_dir_all(&exdir).unwrap();
        // d1: P1,P2,P3 | d2: P1,P2 | d3: P3,P4 (campos extras devem ser ignorados).
        let idx = r#"{
          "d1": {"display":"art. 1","ocorrencias":3,"variantes":[],"pareceres":["P1","P2","P3"]},
          "d2": {"display":"art. 2","ocorrencias":2,"variantes":[],"pareceres":["P1","P2"]},
          "d3": {"display":"art. 3","ocorrencias":2,"variantes":[],"pareceres":["P3","P4"]}
        }"#;
        std::fs::write(exdir.join("dispositivos-index.json"), idx).unwrap();

        // seed P1 (dispositivos d1,d2). P2 compartilha 2 (d1,d2); P3 compartilha 1 (d1); P4 zero.
        let seeds = vec!["P1".to_string()];
        let r = pareceres_relacionados(&root, "rs", &seeds, 5, 1);
        assert_eq!(
            r,
            vec!["P2", "P3"],
            "ranqueado por nº compartilhado, seed P1 excluído"
        );
        // min_shared=2 corta P3.
        assert_eq!(
            pareceres_relacionados(&root, "rs", &seeds, 5, 2),
            vec!["P2"]
        );
        // max limita.
        assert_eq!(
            pareceres_relacionados(&root, "rs", &seeds, 1, 1),
            vec!["P2"]
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn relacionados_sem_indice_ou_sem_seeds_e_vazio() {
        // Sem arquivo de índice: expansão é opt-in, comporta-se como hoje (vazio).
        assert!(
            pareceres_relacionados(Path::new("/nao/existe"), "rs", &["P1".into()], 5, 1).is_empty()
        );
        // Sem seeds: nada a expandir.
        let root = temp_dir("relac-vazio");
        std::fs::create_dir_all(root.join("rs").join("extracao")).unwrap();
        std::fs::write(
            root.join("rs")
                .join("extracao")
                .join("dispositivos-index.json"),
            "{}",
        )
        .unwrap();
        assert!(pareceres_relacionados(&root, "rs", &[], 5, 1).is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn entidades_com_ignora_stores_vazios_e_ordena() {
        let mut cols = Collections::new();
        cols.insert("sp-pareceres".into(), store_de(vec![("x", vec![1.0])]));
        cols.insert("rs-pareceres".into(), store_de(vec![("y", vec![1.0])]));
        cols.insert(
            "mg-pareceres".into(),
            Arc::new(ReadStore::from_records(vec![])),
        ); // vazio
        cols.insert("rs-servicos".into(), store_de(vec![("z", vec![1.0])])); // outro kind
        assert_eq!(entidades_com(&cols, "pareceres"), vec!["rs", "sp"]);
        assert_eq!(entidades_com(&cols, "servicos"), vec!["rs"]);
        assert!(entidades_com(&cols, "faqs").is_empty());
    }

    // ---- Transplantados do `rag.rs`, com o assert adaptado ao retorno com score ----

    #[test]
    fn empty_input_yields_nothing() {
        assert!(select_by_proximity(vec![], 0, f32::INFINITY).is_empty());
    }

    #[test]
    fn default_band_keeps_everything() {
        let d = scored(&[("a", 0.0), ("b", 0.3), ("c", 1.7)]);
        assert_eq!(
            docs(select_by_proximity(d, 0, f32::INFINITY)),
            vec!["a", "b", "c"]
        );
    }

    #[test]
    fn finite_band_narrows_to_proximity_of_best() {
        let d = scored(&[("a", 0.10), ("b", 0.12), ("c", 0.90)]);
        assert_eq!(docs(select_by_proximity(d, 0, 0.05)), vec!["a", "b"]);
    }

    #[test]
    fn floor_overrides_band() {
        let d = scored(&[("a", 0.10), ("b", 0.12), ("c", 0.90)]);
        assert_eq!(
            docs(select_by_proximity(d.clone(), 3, 0.05)),
            vec!["a", "b", "c"]
        );
        assert_eq!(select_by_proximity(d, 10, 0.05).len(), 3);
    }

    #[test]
    fn band_zero_keeps_only_ties_with_best() {
        let d = scored(&[("a", 0.20), ("b", 0.20), ("c", 0.21)]);
        assert_eq!(docs(select_by_proximity(d, 0, 0.0)), vec!["a", "b"]);
    }

    #[test]
    fn select_by_proximity_preserva_o_score() {
        // A mudança de assinatura em relação ao rag.rs: o score sobrevive para as faces novas.
        let d = scored(&[("a", 0.10), ("b", 0.12)]);
        let out = select_by_proximity(d, 0, f32::INFINITY);
        assert_eq!(out[0].1, 0.10);
        assert_eq!(out[1].1, 0.12);
    }
}
