# TAREFA-MCP — Motor de retrieval (`auli-retrieval`) + rota `/v1/retrieve` + servidor MCP (`rmcp`)

> **Para o Claude Code.** Seguir o `CLAUDE.md` do repo: incrementos pequenos, mudanças cirúrgicas,
> cada gate com critério de verificação. Trabalhar em `~/Desktop/auli`. **Um PR por gate**
> (G1..G5), na ordem — cada gate deixa `cargo check --workspace` e `cargo test` verdes.

---

## 0. Contexto e objetivo

Hoje o retrieval (embeddar pergunta → varrer `ReadStore` → estreitar por proximidade) vive dentro de
`auli-cli/src/rag.rs`, entrelaçado com o caminho do chat (prompt + LLM + anonimização + log). O
objetivo é separar o **motor de recuperação** num crate próprio e dar a ele **duas faces finas no
mesmo processo**:

```
                        ┌──────────────────────┐
  frontend (browser) ──▶│ HTTP  /v1/question   │──▶ chat: prompt + LLM externo (como hoje)
                        │ HTTP  /v1/retrieve   │──▶ retrieval puro (NOVO — sem LLM)
  IA do auditor ───────▶│ MCP   /mcp (rmcp)    │──▶ retrieval puro (NOVO — sem LLM)
                        └──────────┬───────────┘
                                   │  mesmo processo, mesmo Arc<Engine>
                        ┌──────────▼───────────┐
                        │  auli-retrieval      │  embedder BGE-M3 + ReadStores + docs/
                        └──────────────────────┘
```

**Por que no mesmo processo:** o embedder BGE-M3 in-process é o recurso pesado; um binário MCP
separado teria que carregar o modelo de novo. Montando o serviço MCP no mesmo Axum que já serve
`/v1/question`, os dois protocolos compartilham o mesmo `Arc<Embedder>` e os mesmos `ReadStore`s.

**Versões verificadas (2026-07-20):** `rmcp` **2.2.0** (SDK oficial, crates.io), com features
`server`, `macros`, `transport-streamable-http-server`, `schemars`. O `StreamableHttpService` do
rmcp é um serviço tower que se aninha num `Router` **axum 0.8** via `nest_service` — compatível com
o `axum 0.8.9` que o `auli-cli` já usa. O padrão de código abaixo segue o exemplo oficial
`examples/servers/src/counter_streamhttp.rs` do repositório `modelcontextprotocol/rust-sdk`.

> ✅ **Validado (revisão Claude Code, 2026-07-20), contra o fonte do rmcp 2.2.0:** `ServerInfo`
> (`new`/`with_server_info`/`with_instructions`), `Implementation::from_build_env`,
> `ContentBlock::text`, `CallToolResult::success`, `ErrorData::{internal_error,invalid_params}`,
> `Parameters<T>` e `StreamableHttpService::new(factory, Arc<M>, config)` existem com as
> assinaturas que o esqueleto do G4 assume, e o rmcp pinna `schemars` 1.0 (compatível com o
> planejado). Re-conferir apenas se a versão resolvida subir de 2.2.x.

---

## 1. Decisões de arquitetura (fechadas — não rediscutir durante a implementação)

- **D-MCP-1 — Motor é crate, faces são módulos.** Nasce o crate `auli-server/crates/auli-retrieval`
  (o motor). As duas faces novas (`/v1/retrieve` e `/mcp`) são módulos **dentro do `auli-cli`**, não
  crates — mesmo processo, mudança mínima.
- **D-MCP-2 — Fronteira de dependências do motor.** `auli-retrieval` depende **apenas** de
  `auli-core` (Embedder), `vector-store` (ReadStore), `auli-contract` (payload/mddoc) e
  serde/serde_json. **Nunca** de axum, rmcp, `auli-llm` ou `auli-anon`. O motor é somente-leitura
  por construção (só enxerga `ReadStore`, jamais `Writer`) — preservar o argumento institucional.
- **D-MCP-3 — Motor é síncrono e puro.** Métodos blocking; quem é async (handlers) envelopa em
  `spawn_blocking`, como hoje. Isso mantém o motor testável sem Tokio.
- **D-MCP-4 — Testabilidade sem modelo: funções livres (opção (a), DECIDIDA).** O núcleo do
  motor são funções livres sobre `&Collections` — `search_embedded()`, `entidades_com()`,
  `parecer_por_numero()`, `ler_corpo()`, `decode_parecer()`, `select_by_proximity()` — e os
  métodos do `Engine` são delegações de uma linha. Testes unitários usam as funções livres com
  vetores sintéticos e `ReadStore::from_records`, sem nunca construir um `Embedder`. Testes que
  PRECISAM do modelo seguem a convenção JÁ existente do workspace: `#[ignore = "<motivo>"]` com
  motivo explícito, rodando sob `cargo test -- --ignored` e com `EMBED_CACHE_DIR` apontando para
  os modelos. Quatro ocorrências hoje — `auli-core/src/embed.rs:83,115`
  ("carrega o modelo BGE-M3 (lento); rode com --ignored"),
  `auli-cli/tests/packs_smoke.rs:22` e `auli-collections/src/servicos/mod.rs:215`.

  > **Correção (revisão Claude Code, 2026-07-20).** Uma versão anterior desta decisão afirmava
  > que "o workspace não usa `#[ignore]`". **Está errado** — a afirmação veio de um `grep` meu
  > por `#\[ignore\]` literal, que não casa com a forma `#[ignore = "motivo"]` efetivamente usada.
  > O rascunho original da TAREFA estava certo ao invocar essa convenção; a revisão é que a negou.
  > Vale a convenção existente, com a única correção de que o motivo é obrigatório.
- **D-MCP-5 — Privacidade nas faces novas.** `/v1/retrieve` e as ferramentas MCP **não chamam LLM
  externo**: a pergunta é embedada localmente e nunca sai do processo. Por isso **não** passam pelo
  anonimizador, e o log dessas rotas grava **apenas metadados** (entidade, kind, top_k, nº de hits,
  latência) — nunca o texto da pergunta. (O log completo com pergunta segue existindo só no caminho
  do chat, como hoje.)
- **D-MCP-6 — Rate limit (revisado).** O recurso caro é o embedder, então TODA rota que o toca
  é limitada. `/v1/question` e `/v1/retrieve` COMPARTILHAM um único limiter (1 req/s, burst 2)
  construído UMA vez no `app()` — `question_rate_limiter()` cria um limiter novo a cada chamada
  (`ratelimit.rs:32`), logo instanciar por rota dobraria a cota efetiva por IP sobre o mesmo
  recurso. `/mcp` ganha limiter PRÓPRIO e mais folgado (10 req/s, burst 30) JÁ NA V1: o handshake
  MCP faz várias requisições em sequência e quebraria com 1 req/s — mas quota diferente ≠ sem
  quota; um endpoint público CPU-bound sem limite seria DoS trivial.
- **D-MCP-7 — Ferramentas MCP da v1 (só três):** `listar_entidades`, `buscar_pareceres`,
  `obter_parecer`. `buscar_servicos_faqs` fica para v2 (pendência). Nomes e descrições em pt-BR —
  o consumidor é a IA de um auditor brasileiro.
- **D-MCP-8 — `AppState` passa a carregar o motor.** `AppState { engine: Arc<Engine>, anonimizador }`.
  Os campos `collections`, `embedder` e `docs_root` **migram para dentro do Engine** (o
  `list_handler` passa a ler via `engine.store(...)`). Uma fonte de verdade, sem Arcs duplicados.
- **D-MCP-9 — Sem auth na v1.** O endpoint é conteúdo público somente-leitura, atrás do tunnel
  e com rate limit (D-MCP-6). Token opcional (API key via header) é pendência (v2).

---

## G1 — Crate `auli-retrieval` (o motor)

### G1.1 Workspace

Em `auli-server/Cargo.toml`, adicionar o membro (manter o comentário da fronteira dos scrapers
intocado):

```toml
members = [
    "crates/auli-contract",
    "crates/vector-store",
    "crates/auli-anon",
    "crates/auli-core",
    "crates/auli-llm",
    "crates/auli-retrieval",   # <- NOVO: motor de recuperação (ver D-MCP-2)
    "crates/auli-cli",
    "crates/auli-collections",
    "crates/scrapers/*",
]
```

### G1.2 `crates/auli-retrieval/Cargo.toml`

```toml
[package]
name = "auli-retrieval"
version = "0.1.0"
edition = "2024"
description = "Motor de recuperação do Auli: embedder + ReadStores + estreitamento por proximidade. Somente leitura, sem LLM, sem HTTP."

[dependencies]
auli-contract = { path = "../auli-contract" }
auli-core = { path = "../auli-core" }
vector-store = { path = "../vector-store" }

serde = { version = "1.0.228", features = ["derive"] }
serde_json = "1.0.150"
thiserror = "2.0.18"
tracing = "0.1.44"
```

### G1.3 `crates/auli-retrieval/src/lib.rs` (arquivo completo)

O conteúdo abaixo **transplanta** de `auli-cli/src/rag.rs`: `select_by_proximity` (com o
`debug_assert!` e o contrato de ordenação), `ler_corpo`, e a lógica de decodificação do payload
leve de parecer. Não reescrever a semântica — mover.

```rust
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

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use auli_contract::{mddoc, ConsultaPackPayload};
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

/// Um parecer recuperado, decodificado do payload leve (G3). `corpo` é opcional: a busca devolve
/// só metadados; `obter_parecer`/`corpo_do_parecer` preenchem sob demanda lendo a árvore `docs/`.
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
        Self { collections, embedder, docs_root }
    }

    /// Acesso direto a um store (usado pelo `list_handler` do server). `None` = coleção não
    /// carregada (a entidade não tem esse kind).
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

    /// Busca com o vetor JÁ pronto. Delegação de uma linha (D-MCP-4, opção (a)).
    pub fn search_embedded(
        &self,
        collection: &str,
        embedding: &[f32],
        ceiling: usize,
        floor: usize,
        band: f32,
    ) -> Result<Vec<Hit>> {
        search_embedded(&self.collections, collection, embedding, ceiling, floor, band)
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

    /// Busca em `<id>-pareceres` devolvendo hits DECODIFICADOS (payload leve → campos), sem corpo.
    /// Payload que não desserializa não derruba a busca: vira um hit degradado com o JSON cru no
    /// `assunto` (mesma filosofia do `bloco_parecer` de sempre — nunca propagar erro na query).
    pub fn search_pareceres(
        &self,
        entity_id: &str,
        texto: &str,
        ceiling: usize,
        floor: usize,
        band: f32,
    ) -> Result<Vec<ParecerHit>> {
        let collection = format!("{entity_id}-pareceres");
        Ok(self
            .search(&collection, texto, ceiling, floor, band)?
            .into_iter()
            .map(|hit| decode_parecer(&hit.payload, Some(hit.score)))
            .collect())
    }

    /// Delegação (D-MCP-4, opção (a)).
    pub fn parecer_por_numero(&self, entity_id: &str, numero: &str) -> Result<Option<ParecerHit>> {
        parecer_por_numero(&self.collections, &self.docs_root, entity_id, numero)
    }

    /// Delegação (D-MCP-4, opção (a)).
    pub fn ler_corpo(&self, entity_id: &str, doc_path: &str) -> std::result::Result<String, String> {
        ler_corpo(&self.docs_root, entity_id, doc_path)
    }

    /// Raiz dos packs/árvore docs (o chat precisa dela para o `bloco_parecer`).
    pub fn docs_root(&self) -> &Path {
        &self.docs_root
    }

    /// Delegação (D-MCP-4, opção (a)).
    pub fn entidades_com(&self, kind: &str) -> Vec<String> {
        entidades_com(&self.collections, kind)
    }
}

// ============================ NÚCLEO PURO (funções livres) ============================
// O coração testável do motor (D-MCP-4, opção (a)): funções sobre `&Collections`/`&Path`, sem
// Embedder e sem estado. Os métodos de `Engine` acima são delegações de uma linha para cá.

/// Busca com vetor pronto. SEMÂNTICA DO ERRO (revisão, blocker 1): com o `load_all` atual do
/// auli-cli, TODA entidade registrada tem os quatro kinds no mapa — arquivo ausente vira store
/// VAZIO —, então `ColecaoAusente` só dispara para coleção realmente fora do mapa (entidade não
/// registrada). Store vazio é SUCESSO com zero hits; quem precisa distinguir "tem acervo de
/// verdade" usa `entidades_com`.
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
    debug!("{collection} scores: {:?}", scored.iter().map(|(_, s)| *s).collect::<Vec<_>>());
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
pub fn parecer_por_numero(
    collections: &Collections,
    docs_root: &Path,
    entity_id: &str,
    numero: &str,
) -> Result<Option<ParecerHit>> {
    let collection = format!("{entity_id}-pareceres");
    let store = collections
        .get(&collection)
        .ok_or(Error::ColecaoAusente(collection))?;
    let alvo = numero.trim().to_lowercase();
    for payload_json in store.list() {
        let Ok(payload) = serde_json::from_str::<ConsultaPackPayload>(&payload_json) else {
            continue; // pack incompatível: pula, não derruba
        };
        if payload.numero.trim().to_lowercase() == alvo {
            let corpo = ler_corpo(docs_root, entity_id, &payload.doc_path)
                .unwrap_or_else(|e| format!("[corpo indisponível — ver link] ({e})\n{}", payload.resumo));
            return Ok(Some(ParecerHit {
                numero: payload.numero,
                assunto: payload.assunto,
                resumo: payload.resumo,
                link: payload.link,
                score: None,
                corpo: Some(corpo),
            }));
        }
    }
    Ok(None)
}

/// Lê o `.md` da árvore da entidade e extrai a seção `## corpo` (parser do contrato).
pub fn ler_corpo(docs_root: &Path, entity_id: &str, doc_path: &str) -> std::result::Result<String, String> {
    let caminho = docs_root.join(entity_id).join(doc_path);
    let texto = std::fs::read_to_string(&caminho).map_err(|e| e.to_string())?;
    let (_header, _sinopse, corpo) = mddoc::parse_doc(&texto).map_err(|e| e.to_string())?;
    Ok(corpo)
}

/// Decodifica o payload leve de um parecer. JSON inválido degrada para um hit com o cru no
/// `assunto` — visível, mas sem derrubar a resposta. Pública: o handler /v1/retrieve usa.
pub fn decode_parecer(payload_json: &str, score: Option<f32>) -> ParecerHit {
    match serde_json::from_str::<ConsultaPackPayload>(payload_json) {
        Ok(p) => ParecerHit {
            numero: p.numero,
            assunto: p.assunto,
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
/// (Transplantado de `rag.rs` — MESMO contrato: entrada ordenada ascendente, `debug_assert!`.)
pub fn select_by_proximity(scored: Vec<(String, f32)>, floor: usize, band: f32) -> Vec<(String, f32)> {
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
```

> **Atenção à assinatura:** o `select_by_proximity` do `rag.rs` devolve `Vec<String>`; a versão do
> motor devolve `Vec<(String, f32)>` — o score agora interessa às faces novas. No G2 o `rag.rs`
> descarta o score com um `.map(|(doc, _)| doc)`.

### G1.4 Testes do motor (no mesmo `lib.rs`, `mod tests`)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use vector_store::Record;

    /// Testes SEM modelo: usam as FUNÇÕES LIVRES (D-MCP-4, opção (a)) com stores sintéticos.
    /// Nenhum teste deste módulo constrói `Engine` nem `Embedder`.
    fn store_de(v: Vec<(&str, Vec<f32>)>) -> Arc<ReadStore<String>> {
        let records = v
            .into_iter()
            .enumerate()
            .map(|(i, (payload, embedding))| Record {
                id: format!("id-{i}"),
                embedding,
                payload: payload.to_string(),
            })
            .collect();
        Arc::new(ReadStore::from_records(records))
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
    fn decode_parecer_json_valido_e_invalido() {
        let json = serde_json::json!({
            "numero": "PARECER Nº 1", "assunto": "ICMS – crédito",
            "resumo": "Resumo.", "link": "http://x/1", "doc_path": "docs/pareceres/p1.md"
        })
        .to_string();
        let ok = decode_parecer(&json, Some(0.1));
        assert_eq!(ok.numero, "PARECER Nº 1");
        assert_eq!(ok.score, Some(0.1));
        assert!(ok.corpo.is_none());

        let ruim = decode_parecer("isto não é json", None);
        assert_eq!(ruim.assunto, "isto não é json"); // degradação visível, sem pânico
    }

    #[test]
    fn parecer_por_numero_acha_e_le_o_corpo_da_arvore() {
        // Usa as funções LIVRES `parecer_por_numero`/`ler_corpo` (recebem `&Collections` e `&Path`).
        // Espelha o teste `bloco_parecer_le_corpo_da_arvore_e_monta_o_bloco` do rag.rs:
        // monta docs/<id>/docs/pareceres/p1.md num temp dir, um store com o payload leve,
        // e verifica numero/corpo. Também o caso caixa-diferente ("parecer nº 1") e o miss (None).
        // (Implementar completo — ver o teste original no rag.rs como referência de setup.)
    }

    #[test]
    fn entidades_com_ignora_stores_vazios_e_ordena() {
        let mut cols = Collections::new();
        cols.insert("sp-pareceres".into(), store_de(vec![("x", vec![1.0])]));
        cols.insert("rs-pareceres".into(), store_de(vec![("y", vec![1.0])]));
        cols.insert("mg-pareceres".into(), Arc::new(ReadStore::from_records(vec![]))); // vazio
        cols.insert("rs-servicos".into(), store_de(vec![("z", vec![1.0])]));           // outro kind
        assert_eq!(entidades_com(&cols, "pareceres"), vec!["rs", "sp"]);
    }

    // Transplantar TODOS os testes de select_by_proximity do rag.rs (empty_input, default_band,
    // finite_band, floor_overrides_band, band_zero) adaptando o assert ao novo retorno com score.
}
```

**Verificação do G1:** `cargo test -p auli-retrieval` verde; `cargo check --workspace` verde;
`cargo tree -p auli-retrieval` **não** contém axum, rmcp, reqwest, auli-llm nem auli-anon.
(Ele CONTÉM fastembed/ort via `auli-core` — esperado e correto: a garantia da fronteira é "sem
HTTP, sem LLM, sem anonimizador", não leveza de build.)

---

## G2 — Religar o chat ao motor (paridade estrita)

Objetivo: `rag.rs` passa a consumir o `Engine`; **comportamento byte-idêntico** do `/v1/question`.

1. `auli-cli/Cargo.toml`: adicionar `auli-retrieval = { path = "../auli-retrieval" }`.
2. `packs.rs`: trocar a definição local `pub type Collections = ...` por
   `pub use auli_retrieval::Collections;` (o `load_all` continua aqui — ele depende do registry e
   do manifesto, que são preocupação da aplicação, não do motor).
3. `state.rs`:

```rust
use auli_retrieval::Engine;

#[derive(Clone)]
pub struct AppState {
    /// O motor de recuperação: embedder + coleções + árvore docs/. Compartilhado pelas três
    /// faces (chat, /v1/retrieve, MCP). Somente-leitura por construção.
    pub engine: Arc<Engine>,
    pub anonimizador: Arc<Anonimizador>,
}
```

4. `lib.rs::run_server`: construir o Engine e montar o `AppState` novo. Atenção (revisão, item 8):
   o `docs_root` atual do `AppState` é `Arc<Path>`, não `PathBuf` — na construção, o mais simples:
   `Engine::new(collections, embedder, docs_root.as_ref().to_path_buf())`
   e montar o `AppState` novo. As linhas de boot (prints de pacotes/embedder/anonimizador) não mudam.
5. `rag.rs` — mudanças cirúrgicas, nada além delas:
   - `exec_all_question` passa a receber `engine: Arc<Engine>` no lugar de
     `collections`/`embedder`/`docs_root` (o `anonimizador` continua como está).
   - O embed da pergunta: `run_blocking(move || engine.embed(&q).map_err(|e| e.to_string().into()))`
     — **uma vez**, reusado nas duas buscas, como hoje.
   - `retrieve(...)`: vira um envelope fino sobre `engine.search_embedded`, preservando a semântica
     tolerante do chat (coleção ausente → `warn!` + `vec![]`, nunca erro):

```rust
async fn retrieve(
    engine: Arc<Engine>,
    collection: String,
    label: &'static str,
    embedding: Vec<f32>,
    n_results: usize,
    floor: usize,
    band: f32,
) -> Result<Vec<String>> {
    run_blocking(move || {
        match engine.search_embedded(&collection, &embedding, n_results, floor, band) {
            Ok(hits) => Ok(hits.into_iter().map(|h| h.payload).collect()),
            Err(auli_retrieval::Error::ColecaoAusente(_)) => {
                warn!("coleção '{label}' ausente para esta entidade — ignorando");
                Ok(vec![])
            }
            Err(e) => Err(e.to_string().into()),
        }
    })
    .await
}
```

   - `bloco_parecer`/`ler_corpo`: apagar o `ler_corpo` local; `bloco_parecer` passa a chamar
     `engine.ler_corpo(&cfg.id, &payload.doc_path)` (a raiz mora no engine agora). O
     `render_consulta_block` e a degradação para o resumo ficam exatamente como estão.
   - `select_by_proximity` local: **apagar** (função e testes) — importar do motor onde ainda for
     citado; os testes vivem no motor desde o G1.
6. `api/handlers/question.rs` e `collections.rs`: ajustar às assinaturas novas
   (`state.engine.clone()` no question; `state.engine.store(&collection_name)` no list — o `.list()`
   do store é o mesmo).
7. Constantes `SVC_FLOOR/BAND` etc. **ficam no rag.rs** — são política do chat, não do motor.
8. **Trava automatizada da paridade (pedido da revisão).** Extrair de `exec_all_question` a
   montagem do contexto numa `fn montar_rag(...) -> String` PURA — sem I/O, sem Engine — que
   devolve a string do contexto exatamente como vai ao prompt e ao log. Teste unitário com docs
   sintéticos pina a saída byte a byte. É a única mudança do G2 que ADICIONA código em vez de
   mover — justificada porque o G2 reescreve o caminho vivo do chat e, sem ela, a única
   verificação de paridade seria o diff manual de log.

   **FRONTEIRA (decidir aqui, não na implementação):** `montar_rag` recebe `&[String]` já
   PRONTOS para renderizar — nunca payloads crus. Concretamente:
   - `ServicosFaqs` → `montar_rag_servicos_faqs(svc_docs: &[String], faq_docs: &[String])`,
     os documentos como saem do `retrieve`.
   - `Pareceres` → `montar_rag_pareceres(blocos: &[String])`, onde cada bloco é a saída de
     `bloco_parecer` — quem chama faz o I/O (ler o corpo da árvore `docs/`) ANTES.

   O motivo é que `bloco_parecer` lê disco ([rag.rs](auli-server/crates/auli-cli/src/rag.rs#L134));
   se `montar_rag` recebesse payloads crus, deixaria de ser pura e o teste de paridade precisaria
   de árvore temporária — perdendo exatamente a propriedade que justifica a extração. O `render`
   existente e os formatos (`\n## servico\n{i}\n{doc}\n`, `\n// Resultado: {i}\n{doc}\n`,
   `\n## PARECER\n{i}\n{bloco}\n`, e o `format!("{}\n{}", rag_service, rag_faq)`) movem para
   dentro dessas funções SEM alteração — é o byte-a-byte que o teste pina.

**Verificação do G2 (paridade):**
- `cargo test --workspace` verde (os testes que moveram junto com o código movem-se; nenhum teste
  é enfraquecido).
- `cargo test -p auli-cli` verde incluindo o teste de `montar_rag` (paridade automatizada do
  formato do contexto). Nota: o rótulo do `debug!` de scores muda de `svc`/`faq`/`par` para o
  nome da coleção (`sc-servicos`) — mudança de LOG, fora do escopo da paridade, que é o contexto RAG.
- Manual, no servidor local com os packs reais: fazer a MESMA pergunta antes e depois do gate
  (ex.: uma das perguntas do exemplo `retrieval_test` — atenção: é um EXAMPLE do auli-cli, roda
  com `cargo run -p auli-cli --example retrieval_test`, não um teste — `type=2`, entidade `sc`)
  e **diffar a seção
  `CONTEXTO RAG` dos dois logs** em `./logs` — precisa ser idêntica (mesmos docs, mesma ordem).
  A resposta do LLM pode variar (temperatura); o contexto, não.

---

## G3 — Rota HTTP `POST /v1/retrieve` (retrieval puro, sem LLM)

### G3.1 DTOs — acrescentar em `api/dto.rs`

```rust
/// Corpo do POST /v1/retrieve. `kind` usa o vocabulário único de `corpus::from_kind`.
#[derive(Debug, Deserialize)]
pub struct RetrieveRequest {
    pub question: String,
    #[serde(default)]
    pub entity: Option<String>,
    /// "servicos" | "faqs" | "pareceres" | "notas". Ausente ⇒ "pareceres" (o caso do auditor).
    #[serde(default)]
    pub kind: Option<String>,
    /// Teto de resultados. Ausente ⇒ o `n_results` do kind; sempre limitado a MAX_TOP_K.
    #[serde(default)]
    pub top_k: Option<usize>,
}

/// Um hit genérico (servicos/faqs/notas): o texto indexado + a distância cosseno.
#[derive(Debug, Serialize)]
pub struct RetrieveHit {
    pub score: f32,
    pub texto: String,
}

/// Resposta. Os DOIS vetores SEMPRE serializam, mesmo vazios (revisão, item 5): array vazio no
/// vetor do kind pedido significa "zero resultados" — distinguível de erro ou kind trocado.
/// `pareceres` → vetor `pareceres` (estruturado, SEM corpo); demais kinds → vetor `hits`.
#[derive(Debug, Serialize)]
pub struct RetrieveResponse {
    pub entity: String,
    pub kind: String,
    pub hits: Vec<RetrieveHit>,
    pub pareceres: Vec<auli_retrieval::ParecerHit>,
}
```

### G3.2 Handler — novo arquivo `api/handlers/retrieve.rs`

```rust
//! POST /v1/retrieve — recuperação semântica PURA: embeda a pergunta (local), varre a coleção e
//! devolve os documentos com score. NÃO chama o LLM externo; a pergunta nunca sai do processo
//! (D-MCP-5), então não passa pelo anonimizador e o log registra só metadados.

use std::sync::Arc;
use std::time::Instant;

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use tracing::info;

use auli_core::corpus;

use crate::api::dto::{RetrieveHit, RetrieveRequest, RetrieveResponse};
use crate::entities::get_entity;
use crate::state::AppState;
use crate::util::run_blocking;

/// Teto duro de top_k, acima do n_results padrão dos kinds — protege o embedder e o payload.
const MAX_TOP_K: usize = 20;

pub async fn retrieve_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RetrieveRequest>,
) -> impl IntoResponse {
    let started = Instant::now();

    let cfg = match get_entity(req.entity.as_deref()) {
        Ok(cfg) => cfg,
        Err(e) => return erro(StatusCode::NOT_FOUND, e),
    };
    let kind = req.kind.as_deref().unwrap_or("pareceres");
    let collection = match corpus::from_kind(kind) {
        Ok(c) => c,
        Err(e) => return erro(StatusCode::BAD_REQUEST, e),
    };
    let top_k = req.top_k.unwrap_or(collection.n_results).min(MAX_TOP_K).max(1);

    let engine = state.engine.clone();
    let entity_id = cfg.id.clone();
    let kind_owned = kind.to_string();
    let question = req.question;

    // Blocking: embed + scan são CPU-bound (mesma disciplina do chat).
    let resultado = run_blocking(move || {
        let name = format!("{entity_id}-{kind_owned}");
        engine
            .search(&name, &question, top_k, 0, f32::INFINITY)
            .map_err(|e| e.to_string().into())
    })
    .await;

    let hits = match resultado {
        Ok(h) => h,
        Err(_) => {
            // Erro interno NUNCA vira corpo de resposta (lição do question_handler): texto fixo
            // amigável; o detalhe já está no tracing.
            return erro(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Não foi possível completar a busca. Tente novamente.".into(),
            );
        }
    };

    // D-MCP-5: log SÓ de metadados — nunca o texto da pergunta.
    info!(
        entity = %cfg.id, kind = %kind, top_k, hits = hits.len(),
        ms = started.elapsed().as_millis() as u64,
        "retrieve concluído"
    );

    let body = if kind == "pareceres" {
        RetrieveResponse {
            entity: cfg.id.clone(),
            kind: kind.into(),
            hits: vec![],
            pareceres: hits
                .into_iter()
                .map(|h| auli_retrieval::decode_parecer(&h.payload, Some(h.score)))
                .collect(),
        }
    } else {
        RetrieveResponse {
            entity: cfg.id.clone(),
            kind: kind.into(),
            hits: hits.into_iter().map(|h| RetrieveHit { score: h.score, texto: h.payload }).collect(),
            pareceres: vec![],
        }
    };
    (StatusCode::OK, Json(body)).into_response()
}

fn erro(status: StatusCode, message: String) -> axum::response::Response {
    (status, Json(serde_json::json!({ "status": "Erro", "message": message }))).into_response()
}
```

### G3.3 Rota + limiter — em `api/mod.rs`

```rust
// Rota pública de retrieval puro (sem LLM). CPU-bound no embedder ⇒ divide o MESMO limiter do
// question (D-MCP-6): `question_rate_limiter()` constrói um limiter NOVO a cada chamada
// (ratelimit.rs:32), então ele passa a ser construído UMA vez no app() e injetado nas duas
// rotas — instanciar por rota dobraria a cota efetiva por IP sobre o mesmo recurso.
pub fn retrieve_routes(state: Arc<AppState>, limiter: SharedLimiter) -> Router {
    Router::new()
        .route("/v1/retrieve", post(retrieve_handler))
        .layer(middleware::from_fn_with_state(limiter, ratelimit::rate_limit))
        .with_state(state)
}
```

E em `lib.rs::app()` (o trecho completo está no G4.3):

```rust
let embed_limiter = ratelimit::question_rate_limiter(); // UM limiter para question + retrieve
// ...
    .merge(question_routes(state.clone(), embed_limiter.clone()))
    .merge(retrieve_routes(state.clone(), embed_limiter))
```

`question_routes` ganha o parâmetro (deixa de construir o limiter internamente). Em
`ratelimit.rs`, criar o alias `pub type SharedLimiter = ...` para o tipo `Arc<RateLimiter<...>>`
que `question_rate_limiter()` já devolve. Registrar `retrieve_handler` no `handlers/mod.rs`.

### G3.4 Testes do G3

Em `auli-cli/tests/retrieve_api.rs`:

```rust
//! Testes da rota /v1/retrieve.
//!
//! Os testes de contrato de erro (entidade/kind inválidos) não precisam de modelo: validação
//! pura, antes do embedder. O teste feliz carrega o BGE-M3 e segue a convenção do workspace
//! (D-MCP-4): `#[ignore = "carrega o modelo BGE-M3 (lento); rode com --ignored"]`, executado
//! com `cargo test -- --ignored` e EMBED_CACHE_DIR apontando para os modelos.

// 1) kind desconhecido → 400 com "Tipo de coleção desconhecido"
//    (montar AppState de teste; ver nota abaixo sobre o embedder)
// 2) entidade desconhecida → 404 com "Entidade desconhecida"
// 3) fluxo feliz (#[ignore], carrega o modelo — convenção D-MCP-4): AppState real com um pack sintético mínimo gravado via
//    vector_store::Writer num temp dir + EMBED_CACHE_DIR; POST question="crédito de energia",
//    kind="pareceres" → 200, `pareceres` não-vazio, score presente, corpo AUSENTE no JSON.
//
// NOTA embedder nos testes 1–2: o AppState exige Arc<Engine> que exige Arc<Embedder>, então um
// teste HTTP completo custaria o load do modelo só para exercitar validação. Extrair a validação
// para uma função PURA `fn validar_retrieve(req: &RetrieveRequest) -> Result<(&EntityConfig,
// &str, usize), (StatusCode, String)>` (resolve entidade, kind e top_k) e cobrir os casos 1–2
// chamando-a direto — dez linhas, sem axum, sem Engine. O handler passa a ser
// `validar_retrieve(&req)?` + busca. O teste HTTP end-to-end fica só no item 3.
```

Testes de serialização dos DTOs (em `dto.rs`): `RetrieveRequest` sem `kind` desserializa com
`None`; `RetrieveResponse` serializa os DOIS vetores mesmo vazios (sem `skip_serializing_if`).

**Verificação do G3:** `cargo test -p auli-cli` verde; na máquina com os modelos,
`EMBED_CACHE_DIR=../../../models cargo test -p auli-cli -- --ignored` verde (fluxo feliz, D-MCP-4);
e o smoke manual:

```bash
curl -s localhost:3000/v1/retrieve -H 'Content-Type: application/json' \
  -d '{"question":"crédito de ICMS na aquisição de energia elétrica","entity":"sc","kind":"pareceres","top_k":5}' | python3 -m json.tool
```

Deve devolver até 5 pareceres com `numero/assunto/resumo/link/score`, **sem** `corpo`, e o log do
servidor deve mostrar `retrieve concluído` **sem** o texto da pergunta.

---

## G4 — Servidor MCP (`rmcp`) no mesmo processo

### G4.1 Dependências (`auli-cli/Cargo.toml`)

```toml
# MCP (Model Context Protocol) — face para assistentes de IA (Claude, etc). Streamable HTTP
# aninhado no mesmo Router axum; ver src/mcp.rs.
rmcp = { version = "2.2", features = ["server", "macros", "transport-streamable-http-server", "schemars"] }
schemars = "1.0"
```

### G4.2 Novo módulo `auli-cli/src/mcp.rs` (arquivo completo)

```rust
//! Face MCP do Auli — servidor Model Context Protocol sobre o MESMO motor do chat.
//!
//! Três ferramentas (D-MCP-7), pensadas para a IA de um auditor/analista fiscal:
//!   - `listar_entidades`   — quais UFs têm acervo de pareceres indexado
//!   - `buscar_pareceres`   — busca semântica; devolve metadados + sinopse + link (sem corpo)
//!   - `obter_parecer`      — corpo integral de UM parecer, pelo número exato
//!
//! Privacidade (D-MCP-5): a pergunta é embedada localmente e NUNCA sai do processo; nenhum LLM
//! externo é chamado neste caminho; o tracing registra só metadados.

use std::sync::Arc;

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    schemars, tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler,
};

use auli_retrieval::Engine;

use crate::entities;

/// Teto de top_k das buscas MCP (mesmo racional do /v1/retrieve).
const MAX_TOP_K: usize = 20;
const DEFAULT_TOP_K: usize = 5;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct BuscarPareceresArgs {
    /// Sigla da UF em minúsculas (ex.: "rs", "sc", "sp", "pr"). Use `listar_entidades` para ver
    /// as disponíveis.
    pub uf: String,
    /// Pergunta ou tema em linguagem natural (ex.: "crédito de ICMS na aquisição de energia
    /// elétrica pela indústria").
    pub pergunta: String,
    /// Quantos resultados devolver (1 a 20; padrão 5).
    #[serde(default)]
    pub top_k: Option<usize>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ObterParecerArgs {
    /// Sigla da UF em minúsculas (ex.: "sc").
    pub uf: String,
    /// Número EXATO como devolvido por `buscar_pareceres` (ex.: "PARECER Nº 26164").
    pub numero: String,
}

#[derive(Clone)]
pub struct AuliMcp {
    engine: Arc<Engine>,
    tool_router: ToolRouter<AuliMcp>,
}

#[tool_router]
impl AuliMcp {
    pub fn new(engine: Arc<Engine>) -> Self {
        Self { engine, tool_router: Self::tool_router() }
    }

    #[tool(description = "Lista as UFs (secretarias estaduais de Fazenda) com acervo de pareceres \
        tributários indexado no Auli, com o nome da secretaria e o total de documentos.")]
    fn listar_entidades(&self) -> Result<CallToolResult, McpError> {
        let linhas: Vec<String> = self
            .engine
            .entidades_com("pareceres")
            .into_iter()
            .map(|id| {
                let nome = entities::get_entity(Some(&id))
                    .map(|c| c.name.clone())
                    .unwrap_or_else(|_| id.clone());
                let total = self
                    .engine
                    .store(&format!("{id}-pareceres"))
                    .map(|s| s.len())
                    .unwrap_or(0);
                format!("- {id} ({nome}): {total} pareceres")
            })
            .collect();
        let texto = if linhas.is_empty() {
            "Nenhuma UF com acervo de pareceres carregado.".to_string()
        } else {
            format!("UFs com acervo de pareceres:\n{}", linhas.join("\n"))
        };
        Ok(CallToolResult::success(vec![ContentBlock::text(texto)]))
    }

    #[tool(description = "Busca semântica no acervo de pareceres tributários de uma UF. Devolve \
        para cada resultado: número, assunto (ementa), sinopse com palavras-chave, link oficial e \
        score de proximidade (menor = mais próximo). NÃO devolve o corpo integral — use \
        `obter_parecer` com o número para lê-lo.")]
    async fn buscar_pareceres(
        &self,
        Parameters(args): Parameters<BuscarPareceresArgs>,
    ) -> Result<CallToolResult, McpError> {
        let uf = args.uf.trim().to_lowercase();
        // Guarda ANTES do embed — e pelo teste certo (revisão, blocker 1): com o `load_all`
        // atual, TODA entidade registrada tem store de pareceres (possivelmente VAZIO), então
        // `store().is_some()` não diz nada. `entidades_com` exige store não-vazio = "tem acervo
        // de verdade". De quebra, este caminho de erro é testável sem carregar o modelo.
        if !self.engine.entidades_com("pareceres").contains(&uf) {
            return Err(McpError::invalid_params(
                format!("UF '{uf}' sem acervo de pareceres. Use `listar_entidades`."),
                None,
            ));
        }
        let top_k = args.top_k.unwrap_or(DEFAULT_TOP_K).clamp(1, MAX_TOP_K);
        let engine = self.engine.clone();
        let pergunta = args.pergunta.clone();
        let uf2 = uf.clone();

        // Embed + scan são CPU-bound: fora do runtime async (mesma disciplina das outras faces).
        let hits = tokio::task::spawn_blocking(move || {
            engine.search_pareceres(&uf2, &pergunta, top_k, 0, f32::INFINITY)
        })
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?
        // `ColecaoAusente` é inalcançável aqui (guarda acima); o que restar é interno de verdade.
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        tracing::info!(uf = %uf, top_k, hits = hits.len(), "mcp buscar_pareceres");

        // JSON estruturado no content de texto: é o formato que assistentes consomem melhor.
        let json = serde_json::to_string_pretty(&hits)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![ContentBlock::text(json)]))
    }

    #[tool(description = "Devolve o corpo integral de um parecer tributário de uma UF, dado o \
        número exato (como devolvido por `buscar_pareceres`). Inclui assunto, sinopse e link \
        oficial.")]
    async fn obter_parecer(
        &self,
        Parameters(args): Parameters<ObterParecerArgs>,
    ) -> Result<CallToolResult, McpError> {
        let uf = args.uf.trim().to_lowercase();
        let numero = args.numero.clone();
        let engine = self.engine.clone();
        let uf2 = uf.clone();

        // I/O de disco + varredura da lista: também fora do runtime.
        let achado = tokio::task::spawn_blocking(move || engine.parecer_por_numero(&uf2, &numero))
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?
            .map_err(|e| McpError::invalid_params(e.to_string(), None))?;

        tracing::info!(uf = %uf, achou = achado.is_some(), "mcp obter_parecer");

        match achado {
            Some(p) => {
                let json = serde_json::to_string_pretty(&p)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                Ok(CallToolResult::success(vec![ContentBlock::text(json)]))
            }
            None => Ok(CallToolResult::success(vec![ContentBlock::text(format!(
                "Nenhum parecer com número '{}' na UF '{uf}'. Confira o número exato via \
                 `buscar_pareceres`.",
                args.numero
            ))])),
        }
    }
}

#[tool_handler]
impl ServerHandler for AuliMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::from_build_env())
            .with_instructions(
                "Acervo Auli de pareceres tributários estaduais brasileiros (conteúdo público \
                 das Secretarias da Fazenda, com links oficiais). Fluxo típico: \
                 `listar_entidades` → `buscar_pareceres` (uma busca por UF; para comparar \
                 estados, busque em cada UF) → `obter_parecer` para ler o corpo integral. \
                 Scores são distância cosseno: menor = mais próximo."
                    .to_string(),
            )
    }
}
```

> **Sobre `ServerInfo::new` / `with_server_info` / builder de capabilities:** essa é a superfície
> conferida no exemplo oficial do SDK (jul/2026). Se `ServerInfo::new` não existir na versão
> resolvida, o padrão alternativo do SDK é struct literal
> `ServerInfo { capabilities, server_info, instructions, .. }` — adaptar mantendo
> `enable_tools()` e as `instructions` acima.

### G4.3 Montagem no router — `lib.rs`

**Onde mora a montagem (revisão, item 6):** todo grupo de rotas hoje é uma função do `api/mod.rs` e
o `app()` é uma lista plana de `.merge()`. O MCP segue a MESMA forma — a construção do
`StreamableHttpService` fica em `api/mod.rs`, não inline no `lib.rs` (que assim não precisa importar
`axum::middleware` nem `ratelimit`).

Em `api/mod.rs`:

```rust
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
};

// Face MCP: um serviço tower aninhado em /mcp, servido pelo MESMO processo/porta. O factory roda
// por sessão e só clona Arcs — barato. Limiter PRÓPRIO e mais folgado (D-MCP-6, JÁ NA V1): o
// handshake MCP faz várias requisições em sequência — mas quota diferente ≠ sem quota.
// O layer de CORS do `app()` envolve /mcp também; é inócuo para clientes MCP (não são browsers).
pub fn mcp_routes(state: Arc<AppState>) -> Router {
    let engine = state.engine.clone();
    let service = StreamableHttpService::new(
        move || Ok(crate::mcp::AuliMcp::new(engine.clone())),
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig::default(),
    );
    Router::new()
        .nest_service("/mcp", service)
        .layer(middleware::from_fn_with_state(ratelimit::mcp_rate_limiter(), ratelimit::rate_limit))
}
```

Em `lib.rs` — o `app()` continua plano:

```rust
pub mod mcp;   // junto dos outros mods

use crate::api::{cors_routes, data_routes, mcp_routes, public_routes, question_routes, retrieve_routes};

/// Assemble the full application router. (comentário existente)
pub fn app(state: Arc<AppState>) -> Router {
    // UM limiter para question + retrieve: é o mesmo recurso (o embedder). Ver D-MCP-6.
    let embed_limiter = api::ratelimit::question_rate_limiter();

    Router::new()
        .merge(public_routes())
        .merge(question_routes(state.clone(), embed_limiter.clone()))
        .merge(retrieve_routes(state.clone(), embed_limiter))
        .merge(data_routes(state.clone()))
        .merge(mcp_routes(state))
        .layer(cors_routes())
}
```

Em `ratelimit.rs`, acrescentar `mcp_rate_limiter()` — MESMA construção do `question_rate_limiter()`
(nada de macro `nonzero!`, que não existe neste crate; o estilo do arquivo é `NonZeroU32::new`):

```rust
/// Limiter do `/mcp` (D-MCP-6): 10 req/s, burst 30 — folgado o bastante para o handshake MCP
/// (initialize → initialized → tools/list em sequência), apertado o bastante para não deixar um
/// endpoint público CPU-bound sem teto. Nota: o 429 sai como JSON simples, não JSON-RPC — correto
/// no nível HTTP, e clientes MCP tratam erro de transporte.
pub fn mcp_rate_limiter() -> SharedLimiter {
    let quota = Quota::per_second(NonZeroU32::new(10).unwrap()).allow_burst(NonZeroU32::new(30).unwrap());
    Arc::new(RateLimiter::keyed(quota))
}
```

Boot (`run_server`): acrescentar um print na sequência dos existentes:
`println!("🔌 Servidor MCP em /mcp (rmcp, streamable HTTP)");`

### G4.4 Testes do G4

Em `mcp.rs`, `mod tests`:

```rust
// Nota de setup: os testes 1–2 exercitam a LÓGICA via funções livres do motor (sem Engine) e
// rodam no `cargo test` normal. O teste 4 chama métodos do AuliMcp, que exige Engine real (com
// Embedder): `#[ignore = "carrega o modelo BGE-M3 (lento); rode com --ignored"]` (D-MCP-4), com
// um helper `fn engine_de_teste() -> &'static Arc<Engine>` usando std::sync::OnceLock para pagar
// o load UMA vez no binário de teste. O teste 3 NÃO precisa de modelo: a guarda roda antes do
// embed (ver abaixo), então ele fica sem #[ignore] — é o ganho concreto de ter movido a guarda.
//
// 1) lógica de listar_entidades: `entidades_com` + contagem com Collections sintéticas —
//    UFs não-vazias ordenadas; store vazio não aparece. (função livre; sem Engine)
// 2) lógica de obter_parecer: `parecer_por_numero` com temp dir docs/<id>/docs/pareceres/p1.md
//    + store com payload leve; caixa diferente no numero acha; miss devolve None. (função livre)
// 3) buscar_pareceres em UF sem acervo → McpError::invalid_params citando listar_entidades.
//    Cobrir os DOIS casos que a guarda unifica (revisão, blocker 1): UF fora do registro E UF
//    registrada com store de pareceres VAZIO — o segundo é o que `store().is_some()` deixaria
//    passar. A guarda roda antes do embed, então o teste não faz busca nenhuma.
// 4) fluxo feliz de buscar_pareceres com pack sintético (#[ignore], engine_de_teste; D-MCP-4).
```

**Smoke manual do protocolo** — novo `scripts/mcp-smoke.sh` (colocar no repo):

```bash
#!/usr/bin/env bash
# Smoke do endpoint MCP (streamable HTTP): initialize → initialized → tools/list → tools/call.
# Uso: ./scripts/mcp-smoke.sh [http://localhost:3000/mcp]
set -euo pipefail
URL="${1:-http://localhost:3000/mcp}"
H=(-H 'Content-Type: application/json' -H 'Accept: application/json, text/event-stream')

echo "== initialize =="
INIT=$(curl -s -D /tmp/mcp-headers "${H[@]}" "$URL" -d '{
  "jsonrpc":"2.0","id":1,"method":"initialize",
  "params":{"protocolVersion":"2025-03-26","capabilities":{},
            "clientInfo":{"name":"mcp-smoke","version":"0.1"}}}')
echo "$INIT" | head -c 800; echo
SID=$(grep -i '^mcp-session-id:' /tmp/mcp-headers | tr -d '\r' | awk '{print $2}')
echo "session: $SID"
S=(-H "Mcp-Session-Id: $SID")

echo "== notifications/initialized =="
curl -s "${H[@]}" "${S[@]}" "$URL" -d '{"jsonrpc":"2.0","method":"notifications/initialized"}' >/dev/null

echo "== tools/list =="
curl -s "${H[@]}" "${S[@]}" "$URL" -d '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' | head -c 1500; echo

echo "== tools/call listar_entidades =="
curl -s "${H[@]}" "${S[@]}" "$URL" -d '{
  "jsonrpc":"2.0","id":3,"method":"tools/call",
  "params":{"name":"listar_entidades","arguments":{}}}' | head -c 1500; echo

echo "== tools/call buscar_pareceres (sc) =="
curl -s "${H[@]}" "${S[@]}" "$URL" -d '{
  "jsonrpc":"2.0","id":4,"method":"tools/call",
  "params":{"name":"buscar_pareceres","arguments":{
    "uf":"sc","pergunta":"crédito de ICMS na aquisição de energia elétrica","top_k":3}}}' | head -c 3000; echo
```

> Nota: com streamable HTTP a resposta pode vir como SSE (`event: message` / `data: {...}`) — o
> smoke considera sucesso ver o JSON-RPC `result` dentro do corpo, em qualquer dos dois formatos.
> Se `initialize` responder 406, conferir o header `Accept` duplo acima (obrigatório no spec).

**Verificação do G4 — três degraus, NESTA ordem** (cada um valida uma camada; não pular):

1. **Protocolo (localhost):** `cargo test -p auli-cli` verde e `./scripts/mcp-smoke.sh` mostrando
   as 3 ferramentas no `tools/list` e resultados reais no `tools/call`; o log do servidor mostra
   `mcp buscar_pareceres` **sem** o texto da pergunta. Valida: handshake, sessão, schemas.
2. **Cliente real (localhost, via Claude Code):** na máquina do servidor,
   `claude mcp add --transport http auli-local http://localhost:3000/mcp`, abrir uma sessão,
   conferir com `/mcp` que as 3 ferramentas aparecem e pedir em linguagem natural: *"liste as UFs
   com pareceres e busque em SC sobre crédito de energia elétrica"*. Valida: descoberta de tools
   por um cliente MCP de verdade e a qualidade das descrições (é a IA escolhendo a ferramenta).
   Claude Code fala com localhost direto — é o ÚNICO cliente Claude que testa sem expor o servidor.
3. **Conector remoto (claude.ai/Desktop, via tunnel):** adicionar conector personalizado
   (Customize → Connectors → “+” → Add custom connector) com a URL `https://api.auli.com.br/mcp`
   e repetir o pedido do degrau 2. **Atenção à rede:** o Claude conecta ao servidor a partir da
   NUVEM da Anthropic, não do dispositivo do usuário — vale para claude.ai, Desktop e apps
   móveis. Logo: (a) este degrau NÃO funciona contra localhost; (b) se houver WAF/bot-fight no
   Cloudflare, o caminho `/mcp` precisa liberar os IPs da Anthropic. Valida: alcançabilidade
   pública + o caminho completo que o auditor vai usar.

Se o degrau 3 falhar com o 2 verde, o problema é rede/tunnel, não código — não mexer no `mcp.rs`.

---

## G5 — Documentação e runbook

1. `README.md`: na tabela de componentes, acrescentar `auli-retrieval`; na seção "How it works",
   nota curta das três faces (question / retrieve / mcp) com o diagrama do §0 desta TAREFA.
2. `auli_code.md`: seção nova do crate `auli-retrieval` (fronteira D-MCP-2, API pública) e do
   módulo `mcp.rs`; atualizar o desenho de camadas.
3. `auli_operations.md`: seção "Conectando clientes ao MCP" cobrindo os dois lados: (a) chat —
   claude.ai/Desktop/celular via Customize → Connectors → “+” → Add custom connector com a URL
   `https://api.auli.com.br/mcp` (disponível nos planos free/Pro/Max/Team/Enterprise; free = 1
   conector; em Team/Enterprise um Owner adiciona primeiro em Organization Settings), com o aviso
   de rede: a conexão parte da nuvem da Anthropic, nunca de localhost; (b) CLI — 
   `claude mcp add --transport http auli https://api.auli.com.br/mcp`. Incluir o `mcp-smoke.sh`
   como teste de protocolo e a observação de privacidade (D-MCP-5).
4. **`.mcp.json` na raiz do repo (dogfooding), COMMITADO:**

   ```json
   {
     "mcpServers": {
       "auli": { "type": "http", "url": "https://api.auli.com.br/mcp" }
     }
   }
   ```

   Efeito: toda sessão de Claude Code dentro do repo enxerga o próprio acervo como ferramenta
   (`mcp__auli__buscar_pareceres` etc.) — útil para consultar pareceres durante o desenvolvimento
   e é um teste de regressão permanente do endpoint. O campo `type: "http"` aceita o alias
   `streamable-http`. Nota no README: o Claude Code pede aprovação do `.mcp.json` de projeto na
   primeira sessão — comportamento esperado.
5. `auli_features.md`: item novo "Acervo como serviço (retrieve + MCP)" — 5 linhas, honesto sobre
   ser v1.
6. Registrar pendências (no `auli_pendencias.md`, seção nova "MCP v2"):
   - auth opcional (API key via header `Authorization`) no `/mcp` (D-MCP-9);
   - ferramenta `buscar_servicos_faqs`;
   - `buscar_pareceres` multi-UF numa chamada (hoje: uma UF por chamada — o assistente itera);
   - calibração das bandas (a rota `/v1/retrieve` agora expõe os scores — usar nos ~10 testes SC).

**Verificação do G5:** docs coerentes com o código; `git grep -n "docs_root" auli-server/crates/auli-cli/src/state.rs`
vazio (campo migrou para o Engine); `.mcp.json` presente na raiz e reconhecido por uma sessão de
Claude Code no repo (`/mcp` lista o servidor `auli` com as 3 ferramentas);
`cargo check --workspace && cargo test --workspace` verdes.

---

## Invariantes (valem para TODOS os gates)

1. **Paridade do chat:** o contexto RAG do `/v1/question` é byte-idêntico ao de antes do G2.
2. **Somente leitura:** `auli-retrieval` referencia `ReadStore`, jamais `vector_store::Writer`.
3. **Fronteira do motor:** `cargo tree -p auli-retrieval` sem axum/rmcp/reqwest/auli-llm/auli-anon
   — a garantia é "sem HTTP, sem LLM, sem anonimizador" (fastembed/ort via auli-core estão lá, e
   devem estar).
4. **Privacidade:** nenhum texto de pergunta em log das faces novas; nenhum LLM externo no caminho
   retrieve/MCP; nenhum erro interno cru no corpo de resposta ao cliente.
5. **Fronteira dos scrapers intocada:** o check de CI existente continua passando.
6. **Sem regressão de boot:** entidade sem pareceres continua subindo e respondendo o aviso
   amigável no chat, como hoje.
