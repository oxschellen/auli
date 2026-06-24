# Auli — Descrição Técnica do Código

Documento técnico auditável do Projeto Auli, baseado na leitura direta do código-fonte
dos três repositórios (`auli-server`, `auli-frontend`, `auli-collections`). Cada afirmação
relevante cita o arquivo que a sustenta. Quando algo não pôde ser confirmado no código,
está marcado como **NÃO CONFIRMADO NO CÓDIGO**.

**Operação:** para **compilar, gerar os dados e subir o app** (com Cloudflare Tunnel) e saber **onde ficam
os logs**, ver o runbook [auli_operations.md](auli_operations.md).

> Convenção deste documento: distingo explicitamente o que está **implementado e ativo**
> do que está apenas **modelado/inativo** (rotas sem fluxo completo, dependências
> declaradas mas não usadas, tipos de conteúdo ingeridos mas não consultados, código
> duplicado/divergente entre repositórios).

---

## 1. O que é a Auli

Auli é um assistente virtual (RAG) que auxilia servidores tributários no atendimento ao
cidadão sobre impostos estaduais. É concebido como **multi-tenant** (um servidor/uma UI
para várias entidades = secretarias estaduais), open-source, com ênfase em privacidade
(embeddings locais). O piloto é a SEFAZ-RS.

Pipeline de resposta (caminho realmente executado):

```
pergunta  →  embedding local (fastembed/BGE-M3, in-process)  →  busca vetorial (vector store in-process)  →  LLM externo gera a resposta
```

O ecossistema tem três partes:

| Repositório | Papel | Linguagem/stack |
| --- | --- | --- |
| `auli-server` → `auli` | Backend REST + pipeline RAG | Rust (Axum, Tokio) |
| `auli-frontend` | Interface web (chat + navegação) | React 19 + TypeScript + Vite |
| `auli-collections` | Scrapers que produzem o conteúdo ingerido | Rust (síncrono, `ureq`) |

> **Atualização — backend refatorado para o workspace `auli-engine`.** O `auli-server` (monólito) foi
> reorganizado em um **workspace Cargo único** `auli-engine/` com crates em camadas
> (`vector-store` ← `auli-core` ← `auli-cli`) e **um binário** com dois subcomandos
> (`auli server` / `auli update`). A lógica do `auli-server` foi **preservada verbatim** nos
> novos crates (ver §9 para o detalhe e a prova de paridade). O diretório `auli-server/`
> permanece em disco como **baseline de referência** pré-refatoração. As seções §3 a §3.10
> descrevem esse baseline; **§9 descreve o estado atual (workspace `auli-engine`)**.
>
> **Atualização — `auli-contract` (2026-06-23).** O workspace ganhou o crate magro
> **`auli-contract`** (serde-only): a **forma do dado** (`Table<P>`, `Faq`, `Servico`, trait
> `Embeddable`) compartilhada entre o scraper e o engine. O **`auli-collections` foi movido para
> dentro do workspace** (`auli-engine/crates/auli-collections`, 5º membro). Fluxo novo: o scraper compila
> `Table<P>` preenchendo `text_to_embed` → `data/<id>/raw/<id>-<kind>.json`; o `auli update` lê o
> contrato, embeda `text_to_embed` e armazena `stored_repr` (sem mais parsing de `portal-*.txt`,
> que viraram só *print* de auditoria). `STRATEGY_VERSION` foi para **2**. Caminhos `auli-collections/…`
> nas seções abaixo agora vivem em **`auli-engine/crates/auli-collections/…`**.

Há ainda uma pasta `auli-docs/` no workspace (origem histórica dos scrapers), fora do
escopo dos três repositórios principais.

---

## 2. Arquitetura e fluxo de dados ponta a ponta

> **Atualização — integração unificada sob `data/`.**
> O fluxo de "cópia manual" descrito abaixo foi **substituído** por uma pasta única `data/` na raiz:
> `data/registry.toml` (entidades, fonte única), `data/prompts/`, e por estado
> `data/<id>/{raw (scraper), ref (autorado, versionado), packs (`auli update`)}`. O server lê os
> packs de `data/<id>/packs/` e as entidades do registry; o frontend tem `entities.ts` e
> `public/<id>/` **gerados** de `data/` por `scripts/`. Não há mais cópia à mão entre pastas. O
> diagrama abaixo descreve o estado **anterior** (baseline).

```
[auli-collections]                  [auli-server]                         [auli-frontend]
  scrape do portal                    POST /v1/question                     Chat (React)
        │                                   │                                    │
   portal-<kind>.txt  ──(cópia manual)──>  entities/<id>/portal-*.txt           │
   + <kind>.json                            │  ingestão (rotas /v1/{kind}/...)   │
        │                                    ▼                                   │
        └──(cópia para public/)──>   vector store in-process  <id>-<kind>.json   │
                                            ▲   embeddings in-process (fastembed) │
                                            │                                    │
                            pergunta ──> embedding ──> busca ──> LLM externo ──> resposta ──> UI
                                                                                  │
   <kind>.json / portal-*.txt ──(cópia para auli-frontend/public/<id>/)──> abas de referência
```

Observações importantes confirmadas no código:

- A integração entre os três repositórios é por **arquivos copiados manualmente**, não por
  chamadas diretas. O `auli-server` lê de `entities/<id>/` ([auli-server/src/domain/entities.rs](auli-server/src/domain/entities.rs));
  o `auli-frontend` lê de `public/<id>/` ([auli-frontend/src/shared/fetchers.ts](auli-frontend/src/shared/fetchers.ts));
  o `auli-collections` escreve em `data/<id>/` ([auli-collections/src/domain/entities.rs](auli-collections/src/domain/entities.rs)).
  Não há, no código, automação que sincronize essas três pastas.
- Os três repositórios mantêm **cópias quase idênticas** de um módulo `domain` (registro de
  entidades + registro de coleções), mas com divergências relevantes — ver §6.

---

## 3. auli-server (backend Rust) — baseline pré-refatoração

> Esta seção documenta o **monólito `auli-server`**, mantido em disco como baseline de
> referência. O backend **atual** é o workspace `auli-engine` (§9), que reaproveita esta lógica verbatim.
> Onde §3 diz "o servidor faz X", o workspace faz o mesmo X — reorganizado entre os três crates.

### 3.1 Manifesto e dependências

Fonte: [auli-server/Cargo.toml](auli-server/Cargo.toml). **Pacote v0.2.0**, edição 2021;
a lib é fixada como `auli_server` ([lib].name). Dependências declaradas e seu uso real
(verificado por `grep` em `src/`):

| Dependência | Uso confirmado |
| --- | --- |
| `axum` 0.8, `tokio`, `tower-http` | servidor web/async/CORS — ativo |
| `jsonwebtoken`, `rsa`, `argon2`, `bcrypt`, `rand` | JWT RS256 e hashing — ativo ([auth/jwt.rs](auli-server/src/auth/jwt.rs), [auth/handler.rs](auli-server/src/auth/handler.rs)) |
| `sqlx` (postgres, uuid, chrono), `uuid` | acesso ao Postgres — ativo; `uuid` ainda aparece sobretudo em código comentado |
| `reqwest` 0.13 | chamada ao LLM — ativo ([clients/llm.rs](auli-server/src/clients/llm.rs)) |
| `fastembed` 5 | **embeddings locais in-process** (BGE-M3 ONNX INT8) — ativo ([clients/embedder.rs](auli-server/src/clients/embedder.rs)) |
| `tracing`, `tracing-subscriber` | logging estruturado (`RUST_LOG`) — ativo |
| `serde`/`serde_json`, `chrono`, `dotenvy`, `anyhow`, `thiserror` | ativos |
| `derive_more` | usado só para `Display` em [api/dto.rs](auli-server/src/api/dto.rs) |
| `axum-client-ip` 1.3 | **declarada mas não usada** — nenhum `grep` encontra `axum_client_ip` em `src/`; a extração de IP é feita manualmente em [api/handlers/question.rs](auli-server/src/api/handlers/question.rs) |
| `futures` 0.3 | **declarada mas não usada** em `src/` |

**Mudança de arquitetura (v0.1 → v0.2.0):** foram **removidas** as dependências `ollama-rs`
(cliente Ollama HTTP) e `chromadb`. Embedding e busca vetorial passaram a ser **in-process**
(`fastembed` + um vector store próprio em Rust); não há mais serviços externos de Ollama nem
ChromaDB. Os módulos `clients/ollama.rs` e `clients/chroma.rs` deixaram de existir — ver §3.6.

**NÃO CONFIRMADO NO CÓDIGO / incorreto na documentação:** o README do servidor lista
"E-mail | Lettre (SMTP)" e o `CLAUDE.md` cita `lettre` como "imported but inactive". O
crate `lettre` **não consta no Cargo.toml** e só aparece em **código comentado**
([auth/handler.rs](auli-server/src/auth/handler.rs), [auth/types.rs](auli-server/src/auth/types.rs)).
Não há funcionalidade de e-mail/SMTP no código.

### 3.2 Estrutura do crate

`main.rs` é um entrypoint fino que chama `auli_server::run()` ([main.rs](auli-server/src/main.rs));
toda a aplicação vive na biblioteca ([lib.rs](auli-server/src/lib.rs)), o que permite aos
testes montarem o router sem abrir socket.

Camadas (todas confirmadas em `src/`):

- `config.rs` — `Config` centralizado atrás de `config()` (`LazyLock`), carregado uma vez do
  ambiente; obrigatórias (`req`) dão `panic`, opcionais (`opt`/`parse_opt`) têm default.
  `log_summary()` imprime um resumo não-secreto (com redação de `DATABASE_URL`)
  ([config.rs](auli-server/src/config.rs)).
- `state.rs` — `AppState`: pool Postgres + `Arc<VectorStore>` + `Arc<Embedder>` (ambos
  construídos uma vez no startup) + campos de JWT marcados `#[allow(dead_code)]`
  ([state.rs](auli-server/src/state.rs)).
- `errors.rs` — `Error`/`Result` unificados via `thiserror`: `Custom`, `Anyhow`, `Io`,
  `SerdeJson`, `Reqwest` (a variante `OllamaError` foi **removida**); `From<String>`/`From<&str>`
  ([errors.rs](auli-server/src/errors.rs)).
- `util.rs` — `run_blocking`: roda um closure bloqueante no pool do Tokio (`spawn_blocking`) e
  achata o erro de `JoinError` + erro do closure em um único `crate::Error`
  ([util.rs](auli-server/src/util.rs)).
- `api/` — camada HTTP (rotas, DTOs, handlers).
- `domain/` — registros de entidades e de coleções (sem I/O).
- `rag/` — orquestração do Q&A.
- `clients/` — adaptadores: `embedder` (fastembed), `vector_store` (in-process), `ingest`, `llm`.
- `auth/` — JWT e handlers de autenticação.

Observabilidade: `run()` inicializa `tracing_subscriber` com `EnvFilter` (default `info`;
`RUST_LOG=auli_server=debug` mostra os arrays de score e o prompt RAG completo) ([lib.rs](auli-server/src/lib.rs)).

### 3.3 Rotas (montadas em [lib.rs](auli-server/src/lib.rs) e [api/mod.rs](auli-server/src/api/mod.rs))

Públicas:

| Método | Caminho | Handler | Observação |
| --- | --- | --- | --- |
| GET | `/v1/health` | `health_handler` | ativo |
| POST | `/v1/question` | `question_handler` | caminho RAG ativo |
| GET | `/v1/gen_rsa_keypair` | `gen_rsa_keypair_handler` | gera par RSA 2048 ([auth/jwt.rs](auli-server/src/auth/jwt.rs)) |
| POST | `/v1/signin` | `sign_in_handler` | ativo |
| POST | `/register` | `user_register_handler` | cria usuário **não verificado**, sem retornar token |

Protegidas (middleware `auth_middleware`):

| Método | Caminho | Handler |
| --- | --- | --- |
| GET | `/v1/protected` | `user_get_handler` |
| POST | `/v1/protected_question` | `question_handler` (o **mesmo** handler de `/v1/question`) |

Gestão de dados (genéricas por `{kind}`, exigem JWT — `route_layer` com `auth_middleware`):

| Método | Caminho |
| --- | --- |
| GET | `/v1/{kind}/list` |
| GET | `/v1/{kind}/load_from_file` |
| POST | `/v1/{kind}/load_from_web` |

`{kind}` ∈ `services | faqs | pareceres | notas`, resolvido por
`collections::from_kind` ([domain/collections.rs](auli-server/src/domain/collections.rs)).
Aceitam entidade via `?entity=<id>` (GET) ou campo `entity` no corpo (POST); ausente →
`rs` ([api/handlers/collections.rs](auli-server/src/api/handlers/collections.rs)).

CORS: origens **hardcoded** (auli.com.br, www, e portas locais 3000/5173/8080), métodos
GET/POST/OPTIONS, `allow_credentials(true)` ([api/mod.rs](auli-server/src/api/mod.rs)).

### 3.4 Multi-tenancy (entidades)

[domain/entities.rs](auli-server/src/domain/entities.rs):

- Registro `static ENTITIES: LazyLock<HashMap<String, EntityConfig>>`, carregado uma vez na
  inicialização lendo `./entities/` (`ENTITIES_DIR`).
- Cada entidade precisa de `entity.json` (`{ "id", "name" }`); `prompt.txt` é opcional (há
  `DEFAULT_SYSTEM_PROMPT` de fallback).
- `EntityConfig::collection(kind)` → `"<id>-<kind>"`; `EntityConfig::data_file(name)` →
  `"<data_dir>/<name>"`.
- `get_entity(Option<&str>)`: `None`/vazio → `DEFAULT_ENTITY = "rs"`; id desconhecido →
  `Err(String)` amigável (em português).

**Estado real:** há **apenas a entidade `rs`** configurada no servidor
([auli-server/entities/rs/entity.json](auli-server/entities/rs/entity.json) → `{ "id":"rs", "name":"SEFAZ-RS" }`).
Não existe diretório `entities/sc/` no servidor. Ou seja, o servidor é multi-tenant por
projeto, mas single-tenant na prática hoje.

### 3.5 Caminho RAG ativo (`exec_all_question`)

Fonte: [rag/pipeline.rs](auli-server/src/rag/pipeline.rs), acionado por
[api/handlers/question.rs](auli-server/src/api/handlers/question.rs). Assinatura atual:
`exec_all_question(vector: Arc<VectorStore>, embedder: Arc<Embedder>, question, entity)`.

1. Resolve a entidade (`get_entity`); entidade desconhecida → a própria mensagem de erro é
   retornada como resposta, com HTTP 200 (sem panic).
2. Gera o embedding da pergunta **uma vez**, in-process (`embedder.embed_dense`, executado fora
   da thread async via `run_blocking`/`spawn_blocking`); a pergunta já é uma "chave" curta.
3. Consulta **duas** coleções de forma **concorrente** (`tokio::try_join!`): `<id>-services`
   (`SERVICES.n_results = 10`) e `<id>-faqs` (`FAQS.n_results = 20`). Cada `query_scored` roda em
   thread bloqueante e retorna `(texto, distância cosseno)` ordenado do mais próximo ao mais distante.
4. **Estreitamento por proximidade** (`select_by_proximity`): mantém sempre os `floor` melhores e,
   além disso, os documentos dentro de `band` (distância acima do melhor) — por kind
   (`SVC_FLOOR/SVC_BAND`, `FAQ_FLOOR/FAQ_BAND`). **Os defaults são `floor=0`, `band=∞`**, ou seja,
   hoje há *paridade* com o "take fixo" antigo (nenhum descarte); os bandos só passam a filtrar
   após calibração contra perguntas reais (os arrays de score são logados em `debug`).
5. Concatena os documentos como contexto RAG; o system prompt = `prompt.txt` da entidade +
   contexto + delimitador `'''`.
6. Chama o LLM ([clients/llm.rs](auli-server/src/clients/llm.rs)).
7. Retorna `{ question, answer }` e **anexa o diálogo a `./logs/<timestamp>.txt`**.

**Distinção crucial (ativo vs modelado):** apenas `services` e `faqs` alimentam as
respostas. `pareceres` e `notas` possuem rotas de ingestão e listagem, mas **não são
consultados** por `exec_all_question` — confirmado: [rag/pipeline.rs](auli-server/src/rag/pipeline.rs)
importa apenas `FAQS, SERVICES`.

### 3.6 Clientes e adaptadores (`clients/`)

Toda a parte de embeddings/busca vetorial passou a ser **in-process** (sem Ollama nem ChromaDB).

- **Embedder** ([clients/embedder.rs](auli-server/src/clients/embedder.rs)): `fastembed` com
  **BGE-M3 ONNX INT8** (`Bgem3Model::BGEM3Q`), dimensão **1024** (`EMBED_DIM`), apenas a saída
  *dense* (Fase 1). O modelo fica atrás de um `Mutex` porque `embed` é `&mut self`; `max_length`
  é **512** (dimensionado à "chave" curta, não ao documento inteiro). `embed_dense` é
  **bloqueante/CPU-bound** — os chamadores usam `run_blocking`. Construído uma vez no startup
  (lento; baixa o modelo do Hugging Face para `EMBED_CACHE_DIR` no 1º run).
- **VectorStore** ([clients/vector_store.rs](auli-server/src/clients/vector_store.rs)): índice
  plano **puro-Rust**, in-process. Cada coleção `<id>-<kind>` é uma lista de
  `(id, embedding, document)` mantida em memória e persistida em `<base_path>/<name>.json`
  (`VECTOR_DB_PATH`, default `./vectors`). Métodos: `get_or_open` (carrega do disco no 1º uso),
  `upsert` (ids `id-1..id-N`, substitui por id ou anexa), `reset` (reload limpo, evita órfãos),
  `query_scored` (varredura **brute-force** por **distância cosseno**, ordena melhor-primeiro,
  trunca em `max_results`) e `list`. `cosine_distance` ∈ `[0,2]`; larguras diferentes ou vetor
  zero ⇒ distância máxima `2.0` (o máximo real da métrica, `1 - cos` com `cos = -1`), para que
  afundem abaixo de documentos legitimamente anti-correlacionados.
- **ingest** ([clients/ingest.rs](auli-server/src/clients/ingest.rs)): `load_collection`
  (substitui `chroma::load_collection`) — `prepare_documents` → embeda as chaves (`run_blocking`)
  → `reset` + `upsert`; devolve o mesmo log legível que `build_response` espera.
- **LLM** ([clients/llm.rs](auli-server/src/clients/llm.rs)): chat completions compatível com
  Groq. `temperature 0.5`, `top_p 0.5`, `max_completion_tokens 1024`, `stream:false`. Até
  **3 tentativas** em erros de conexão/timeout (sleep 500ms). Erro de API vira mensagem
  legível (não `Err`).

**Implicação operacional:** trocar o modelo de embedding ⇒ **re-ingestão total** de todas as
coleções. Os vetores não carregam tag de dimensão e larguras incompatíveis pontuam como
distância máxima — uma coleção com vetores do modelo antigo retorna lixo silenciosamente.

### 3.7 Ingestão e formato dos dados

[domain/collections.rs](auli-server/src/domain/collections.rs) descreve cada `kind` como
dado (`Collection`: `kind`, `file`, `delimiter`, `embed`, `n_results`):

| const | kind | file | delimiter | embed | n_results |
| --- | --- | --- | --- | --- | --- |
| `SERVICES` | `services` | `portal-servicos.txt` | `//` | `Description` | 10 |
| `FAQS` | `faqs` | `portal-faqs.txt` | `## pergunta` | `QuestionKey` | 20 |
| `PARECERES` | `pareceres` | `portal-pareceres.txt` | `## pergunta` | `QuestionKey` | 3 |
| `NOTAS` | `notas` | `portal-notas.txt` | `## pergunta` | `FullText` | 1 |

A estratégia de embedding agora separa explicitamente a **chave embedada** do **payload
armazenado/servido** (`EmbedStrategy`: `FullText | Description | QuestionKey`):

- `QuestionKey` (**novo**, `faqs`/`pareceres`): armazena o bloco Q+A **completo**, mas embeda
  **apenas o texto do campo `## pergunta`** (`extract_question` — extração estruturada do campo,
  não corte por caractere; cai para o bloco inteiro se o marcador faltar). Chave curta e de alto
  sinal ⇒ vetor mais nítido. **Tradeoff:** conteúdo que só existe no corpo da resposta não
  aparece na busca densa — é o gatilho documentado da Fase 2 (sparse/hybrid).
- `FullText` (`notas`): armazena e embeda o mesmo bloco. `notas` é intencionalmente **um único
  bloco** (1 registro), e não é consultado no RAG ativo.
- `Description` (só `services`): armazena o serviço "limpo" inteiro (`clean_servico`), mas embeda
  só as 4 primeiras linhas não vazias (última truncada a 300 chars) — `extract_servico_description`.
- `parse_blocks` / `parse_blocks_from_text` dividem em blocos pelo delimitador; `prepare_documents`
  produz `(stored_documents, texts_to_embed)`. Em modo Q&A (delimitador ≠ `//`) ignora linhas
  `//` e só começa a coletar no primeiro delimitador.

**Detalhe verificado em dados reais:** o arquivo [auli-server/entities/rs/portal-servicos.txt](auli-server/entities/rs/portal-servicos.txt)
**não** usa blocos separados por `//` puro — usa o formato `## pergunta`/`## resposta` com
uma linha de comentário `// N.` no topo de cada bloco. Como `SERVICES.delimiter = "//"` e
cada bloco começa exatamente com `// N.`, o parser ainda separa um bloco por serviço; mas a
estratégia `Description` acaba embedando as linhas `## pergunta`/breadcrumb/título/`## resposta`.
Esse descompasso entre o modelo declarado (`//`/`Description`) e o formato real dos dados
(`## pergunta`/`## resposta`) é uma divergência em relação ao que o scraper produz hoje
(ver §6).

### 3.8 Autenticação e banco

[auth/handler.rs](auli-server/src/auth/handler.rs) e [auth/jwt.rs](auli-server/src/auth/jwt.rs):

- JWT **RS256** (chave RSA do ambiente). `sign_in_handler` busca o usuário no Postgres,
  exige `is_verified = TRUE`, verifica hash **Argon2** (com fallback **bcrypt** para hashes
  legados `$2a/$2b/$2y$`) e emite o token.
- `user_register_handler` faz hash Argon2, insere usuário **não verificado** e **não**
  retorna token.
- `auth_middleware` valida `Authorization: Bearer <token>`, revalida que o usuário existe e
  está verificado, e injeta `CurrentUser`.

Migrações ([auli-server/migrations/](auli-server/migrations/)): `users`, `refresh_tokens`,
`verification_tokens`, `password_reset_tokens` (com índices).

**Distinção crucial (modelado vs ativo):** as tabelas `refresh_tokens`,
`verification_tokens` e `password_reset_tokens` existem nas migrações, mas **não há handlers
ativos** para refresh, verificação de e-mail ou reset de senha — toda a lógica
correspondente está **comentada** em [auth/handler.rs](auli-server/src/auth/handler.rs) e
[auth/types.rs](auli-server/src/auth/types.rs) (este último é integralmente um bloco
comentado). Novos usuários ficam não verificados (precisam ser verificados fora de banda).
Não há autorização por papel/admin além de "usuário verificado".

### 3.9 Testes

[auli-server/tests/api.rs](auli-server/tests/api.rs): um único teste de integração que monta
`public_routes()` e confirma que `GET /v1/health` responde 200, sem socket nem banco.

### 3.10 Artefatos soltos (não compilados)

A limpeza da migração v0.2.0 **removeu** os módulos mortos antigos (`exec_*`, `embedding_api.rs`,
`auth_old.rs`, `errors_module/`, `clients/ollama.rs`) e os rascunhos `reranking.rs` e `x-notas/`
— confirmado por `find` em `src/` e na raiz. O que resta fora de `src/`: `scripts/raw/*.json`
(artefatos de scraping antigos) e os scripts de operação (`scripts/start_*.sh`,
`sync-to-wsl.ps1`, `auli-sync.sh`). Os loaders leem apenas de `entities/<id>/`.

---

## 4. auli-frontend (React + TypeScript)

### 4.1 Manifesto e stack

Fonte: [auli-frontend/package.json](auli-frontend/package.json) (nome do pacote `auli-ui`,
versão 0.1.43). Confirmado:

- **React 19** + **Vite 8** (com Rolldown — `rolldownOptions` em
  [vite.config.js](auli-frontend/vite.config.js)).
- **Chakra UI v3** (`@chakra-ui/react`, `@emotion/react`, `@emotion/styled`).
- **SWR** + **Axios** (busca de dados), **Framer Motion** (animações), **react-markdown**,
  **react-icons**, **next-themes** (modo claro/escuro).
- **TypeScript** em modo `strict` ([tsconfig.json](auli-frontend/tsconfig.json), `allowJs:true`).
- Testes com **Vitest** + Testing Library; ambiente Node por padrão, `jsdom` por arquivo.

Scripts: `dev`, `build` (`tsc --noEmit && vite build`), `lint`, `typecheck`, `test`,
`test:watch`, `doctor` ([package.json](auli-frontend/package.json)).

A versão exibida e um `__BUILD_ID__` para cache-busting são injetados em build via `define`
([vite.config.js](auli-frontend/vite.config.js)). Chunks manuais separam react/chakra/icons/utils.

### 4.2 Estrutura da aplicação

- **Single-page sem roteador.** [App.tsx](auli-frontend/src/App.tsx) monta um `EntityProvider`
  e mostra `StateSelection` até uma entidade ser escolhida; depois mostra `Home`. Não há
  React Router; a navegação interna é por **abas**.
- **Seleção de entidade.** [shared/EntityContext.tsx](auli-frontend/src/shared/EntityContext.tsx)
  guarda a entidade selecionada, persistida em `localStorage` (chave `auli.entity`).
- **Registro de entidades (frontend).** [shared/entities.ts](auli-frontend/src/shared/entities.ts)
  lista **duas** entidades hardcoded:
  - `rs` = SEFAZ-RS, coleções `["servicos","faqs","pareceres","notas","conteudos"]`.
  - `sc` = SEF-SC, coleções `["servicos"]` (somente serviços, por enquanto).
  `hasCollection(entity, collection)` dirige os estados "em breve".
- **Abas** ([pages/home/Home.tsx](auli-frontend/src/pages/home/Home.tsx)): Chat, Serviços,
  FAQs, Pareceres, Notas, Conteúdos, Sobre. Implementam o padrão WAI-ARIA de tabs (roving
  tabindex, navegação por setas). Cada aba é montada na primeira ativação e mantida montada
  (preserva rolagem/busca/histórico).

### 4.3 Chat (caminho ativo)

- [pages/chat/Chat.tsx](auli-frontend/src/pages/chat/Chat.tsx): a URL da API é
  `import.meta.env.VITE_API_URL ?? "https://api.auli.com.br/v1/question"`. Passa
  `entityId: entity.id` ao backend.
- [pages/chat/utils/callServerAPI.ts](auli-frontend/src/pages/chat/utils/callServerAPI.ts):
  `POST` via axios do corpo `{ question, entity? }`, com **timeout de 25s** via
  `AbortController`; mensagens de erro/timeout em pt-BR. Lê `res.data.answer`.
- [pages/chat/utils/prompt.ts](auli-frontend/src/pages/chat/utils/prompt.ts): validação do
  prompt (mínimo de 10 caracteres) — fonte única usada pelo guard de envio, botão e contador.
- [pages/chat/utils/useMessages.ts](auli-frontend/src/pages/chat/utils/useMessages.ts):
  estado das mensagens, iniciando com a saudação "Olá! Como posso ajudar?".

Há subpasta [pages/chat/ui/](auli-frontend/src/pages/chat/ui/) com 4 snippets utilitários do
Chakra em `.jsx` (color-mode, provider, toaster, tooltip) — confirmando a observação do
README de que apenas esses arquivos permanecem em JSX.

### 4.4 Páginas de referência (conteúdo estático por entidade)

Todas usam SWR + `entityPath(entityId, file)` →
`/<entityId>/<file>?v=<buildId>` ([shared/fetchers.ts](auli-frontend/src/shared/fetchers.ts)),
ou seja, leem arquivos servidos de `public/<id>/`:

| Aba | Arquivo lido | Fonte |
| --- | --- | --- |
| Serviços | `servicos-index.json` (manifesto de abas) + `<filename>.json` por público | [pages/servicoslist/ServicosList.tsx](auli-frontend/src/pages/servicoslist/ServicosList.tsx) |
| FAQs | `faqs.json` (árvore recursiva) | [pages/faqslist/FaqsList.tsx](auli-frontend/src/pages/faqslist/FaqsList.tsx), [parseFaqs.ts](auli-frontend/src/pages/faqslist/parseFaqs.ts) |
| Pareceres | `portal-pareceres.txt` (texto) | [pages/parecereslist/PareceresList.tsx](auli-frontend/src/pages/parecereslist/PareceresList.tsx) |
| Notas | `portal-notas.txt` (texto) | [pages/notaslist/NotasList.tsx](auli-frontend/src/pages/notaslist/NotasList.tsx) |
| Conteúdos | `conteudo_site_tree.json` | [pages/conteudoslist/ConteudosList.tsx](auli-frontend/src/pages/conteudoslist/ConteudosList.tsx) |

- A aba **Serviços** lê `servicos-index.json` para montar as abas de público; se ausente,
  usa o fallback hardcoded `getDefaultTipoServicos()` (5 tipos RS) em
  [pages/servicoslist/utils.ts](auli-frontend/src/pages/servicoslist/utils.ts). Agrupa por
  `classe` em acordeões e busca por `titulo`.
- Pareceres/Notas usam `editLinks` ([shared/linkify.tsx](auli-frontend/src/shared/linkify.tsx))
  para transformar links no texto.
- Quando a entidade não tem a coleção (`hasCollection` falso), renderiza
  [shared/CollectionEmpty.tsx](auli-frontend/src/shared/CollectionEmpty.tsx) ("em breve").

**Dados estáticos presentes** (confirmado na árvore de `public/`, gerada por
[scripts/build-frontend-public.sh](scripts/build-frontend-public.sh) a partir de `data/<id>/{raw,ref}/`):
[public/rs/](auli-frontend/public/rs/) tem `faqs.json`, `conteudo_site_tree.json`,
`servicos*.json`, `servicos-index.json`, `portal-notas.txt`, `portal-pareceres.txt`.
[public/sc/](auli-frontend/public/sc/) tem **apenas** arquivos de
serviços (`servicos-*.json`, `servicos-index.json`), coerente com
`sc.collections = ["servicos"]`. Os `portal-{faqs,servicos}.txt` (grandes, não usados pela UI —
só alimentam os packs) são excluídos do `public/` pelo gerador.

### 4.5 Seleção de estado e mapa

[pages/stateselection/StateSelection.tsx](auli-frontend/src/pages/stateselection/StateSelection.tsx)
apresenta um mapa interativo do Brasil ([BrazilMap.tsx](auli-frontend/src/pages/stateselection/BrazilMap.tsx),
[brazilPaths.ts](auli-frontend/src/pages/stateselection/brazilPaths.ts)) e cards das
entidades; ambos chamam `selectEntity`.

### 4.6 Tema, erros e testes

- Tokens semânticos de cor centralizados em [src/theme/system.js](auli-frontend/src/theme/system.js)
  (claro/escuro); modo de cor via `next-themes`.
- `ErrorBoundary` ([shared/ErrorBoundary.tsx](auli-frontend/src/shared/ErrorBoundary.tsx)).
- Testes presentes: `SearchInput`, `AsyncContent`, `linkify`, `Input`, `Messages`,
  `callServerAPI`, `prompt`, `parseFaqs`, `parseConteudos` (arquivos `*.test.ts(x)`).

### 4.7 Distinção ativo vs modelado (frontend)

- O frontend é **single-tenant em tempo de execução por deploy**: serve os arquivos de uma
  pasta `public/` específica; a seleção `rs`/`sc` muda apenas qual `public/<id>/` é
  consultado. Não há, no código, busca de uma lista de entidades vinda do backend — a lista
  é hardcoded em [shared/entities.ts](auli-frontend/src/shared/entities.ts).
- O frontend **não** consome as rotas autenticadas nem as de gestão de dados do servidor; o
  único endpoint de backend efetivamente chamado é `POST /v1/question` (via `VITE_API_URL`).
  Não há, no código do frontend, uso de login/JWT. **NÃO CONFIRMADO NO CÓDIGO:** qualquer
  fluxo de autenticação no cliente.

---

## 5. auli-collections (scrapers Rust)

Repositório foco da fase atual (consolida os antigos scrapers de FAQs e Serviços em um único
programa reutilizável). Fonte de orientação: [auli-collections/description.md](auli-collections/description.md)
(usei-o como guia, mas as afirmações abaixo foram verificadas no código).

### 5.1 Manifesto e dependências

Fonte: [auli-collections/Cargo.toml](auli-collections/Cargo.toml). **Edição 2024**.
Totalmente **síncrono** — confirmado: não há `tokio`/`async fn`/`.await` no código compilado
(`grep` não encontra nenhum fora de `legacy/`). Dependências: `ureq` (HTTP síncrono),
`scraper`, `regex`, `url`, `serde`/`serde_json`, `headless_chrome`, `thiserror`, `anyhow`.

### 5.2 CLI e dispatch

[auli-engine/crates/auli-collections/src/main.rs](auli-engine/crates/auli-collections/src/main.rs):
`cd auli-engine && cargo run -p auli-collections -- [--usecache] <entity> <collection>`.

- `<entity>` (omitido/vazio → `rs`) é resolvido e validado por
  `domain::entities::get_entity`.
- `<collection>` (omitido → `faqs`).
- `--usecache` ativa modo offline (só cache; cache miss vira erro).
- Dispatch: `faqs` → `run_faqs`; `servicos` → `servicos::run(&entity.id, &entity.data_dir, use_cache)`;
  qualquer outro → erro.

### 5.3 Saída: o contrato `auli-contract`

A cópia divergente `domain/collections.rs` (que duplicava o `Collection`/`EmbedStrategy` do engine)
foi **apagada**. O scraper agora compila o conteúdo no **contrato tipado** (`auli-contract`): para
cada kind monta uma `Table<P>` (`Table<Faq>` / `Table<Servico>`), preenchendo o `text_to_embed` de
cada registro, e grava em `data/<id>/raw/<id>-<kind>.json`. O engine (`auli update`) lê isso direto
— não há mais `EmbedStrategy`/parsing de `portal-*.txt` deste lado nem do outro.

- **`text_to_embed`** (D2): faqs → breadcrumb `origin` + a pergunta (reproduz a key do antigo
  `QuestionKey`); servicos → `tipo | classe` + título + início do corpo da descrição.
- **`stored_repr`** reproduz o bloco `## pergunta`/`## resposta` (mesma forma do `portal-*.txt`), então
  o contexto servido ao RAG continua coerente.
- Os antigos `faqs.json` (árvore) e `servicos.json` (flat agregado) foram **descartados**; o
  `portal-<kind>.txt` continua sendo escrito como *print* legível (auditoria), nunca lido de volta.
- `pareceres`/`notas` são autorados (sem scraper) e ainda não têm fonte struct — ficam ausentes nos
  packs até serem modelados como `Table<P>`.

### 5.4 Registro de entidades

[auli-collections/src/domain/entities.rs](auli-collections/src/domain/entities.rs): config e
dados **separados** — config em `./src/entities/<id>/` (`ENTITIES_DIR`), saídas em
`./data/<id>/` (`DATA_DIR`). Entidades presentes: `rs` (SEFAZ-RS) e `sc` (SEF-SC), cada uma
com `entity.json` + `prompt.txt` em [src/entities/](auli-collections/src/entities/).

### 5.5 Scraper de FAQs (ativo, `src/faqs/`)

[auli-collections/src/faqs/mod.rs](auli-collections/src/faqs/mod.rs) e submódulos
(`faq.rs`, `fetch.rs`, `html.rs`, `portal.rs`):

- Caminha o portal de FAQs da SEFAZ-RS a partir de uma raiz, classificando páginas por
  `data-matriz-source-uri` (`sanfona.comnavegacao` → `Faq`; `categoriafaq` → `Menu`; demais
  → `Geral`).
- Modelo de saída `FaqNode { title, url, page_type, origin, children, faq_items }`
  ([faqs/faq.rs](auli-collections/src/faqs/faq.rs)).
- `fetch.rs` usa agente `ureq` síncrono + cache em disco; o conteúdo vem de um endpoint AJAX
  (campo `body` com markup). Retentativas com backoff exponencial.
- `html.rs` é um conjunto de helpers HTML→texto feitos à mão (sem crate de DOM).
- `portal.rs` achata a árvore em `portal-faqs.txt`, um bloco `## pergunta`/`## resposta`
  numerado (`// N.`) por pergunta ([faqs/portal.rs](auli-collections/src/faqs/portal.rs)).
- Saídas: `<data_dir>/faqs.json` e `<data_dir>/portal-faqs.txt`
  (`output_path`/`portal_path` em [faqs/mod.rs](auli-collections/src/faqs/mod.rs)).
- Usa o `crate::errors` unificado.

[auli-collections/src/faqs/legacy/](auli-collections/src/faqs/legacy/) contém os arquivos
originais copiados — **não declarados como módulos, portanto não compilados** (confirmado:
`faqs/mod.rs` declara só `faq`, `fetch`, `html`, `portal`). É onde aparece a única referência
a `reqwest` no repositório (em código não compilado).

### 5.6 Scraper de Serviços (`src/servicos/`)

[auli-collections/src/servicos/mod.rs](auli-collections/src/servicos/mod.rs):
`run(entity_id, data_dir, use_cache)` despacha por entidade:

- **`rs`** ([servicos/extrair_descricoes.rs](auli-collections/src/servicos/extrair_descricoes.rs)):
  renderiza as 5 páginas de público com **headless Chrome** (`Browser`/`LaunchOptions`),
  esperando o elemento `.card`; depois busca cada página de detalhe via **`ureq`** e extrai a
  descrição. Falhas de detalhe são coletadas e reportadas no fim (não fatais).
- **`sc`** ([servicos/sc.rs](auli-collections/src/servicos/sc.rs)): consome a **API JSON do
  Next.js** da SEF-SC (sem browser, sem parse de HTML); normaliza links `[[url anchor]]` para
  o formato RS.
- `finish()` (compartilhado) gera `portal-servicos.txt`
  ([servicos/gerar_portal_servicos.rs](auli-collections/src/servicos/gerar_portal_servicos.rs)),
  agrega `servicos.json` e escreve `servicos-index.json` (manifesto de abas consumido pelo
  frontend).
- **Dedup por `link`** (não por `id`): um serviço pode aparecer em vários públicos; o
  `link` é a chave única.
- O formato de `portal-servicos.txt` gerado é `// N.` + `## pergunta` (breadcrumb
  `tipo | classe` + título) + `## resposta` (corpo + `Link:`), idêntico ao de faqs
  ([gerar_portal_servicos.rs](auli-collections/src/servicos/gerar_portal_servicos.rs)).

**Distinção ativo vs em refatoração:** o módulo `servicos` ainda usa
`Box<dyn std::error::Error>` (não o `crate::errors`) e tem URLs de tipo hardcoded em
[servicos/utils.rs](auli-collections/src/servicos/utils.rs); está "wired" e funcional, mas
não totalmente refatorado para o formato limpo do `faqs`.

**Correção a uma afirmação da própria base:** o comentário em
[servicos/mod.rs](auli-collections/src/servicos/mod.rs) diz que o `rs` usa "reqwest fetches
each service's detail page". Isso está **incorreto**: o fetch de detalhe usa **`ureq`**
([servicos/extrair_descricoes.rs](auli-collections/src/servicos/extrair_descricoes.rs), linha 7
`use ureq::Agent;`). `reqwest` só existe em `faqs/legacy/` (não compilado).

### 5.7 Cache e modo offline

Cache em disco para ambos os scrapers (`cache/faqs/` em [faqs/fetch.rs](auli-collections/src/faqs/fetch.rs);
`cache/servicos/` em [servicos/cache.rs](auli-collections/src/servicos/cache.rs)). O
`.gitignore` ignora `/data/*/cache`. `--usecache` propaga o flag até a camada de fetch e
torna um cache miss um erro.

### 5.8 Cobertura por kind (collections)

| kind | scraper existe? |
| --- | --- |
| `faqs` | sim (`rs`); `sc` **não** implementado — arm placeholder em `faq_source_for` |
| `servicos` | sim para `rs` (Chrome) e `sc` (JSON) |
| `pareceres` | **não** há scraper — só existe o `portal-pareceres.txt` como dado |
| `notas` | **não** há scraper — só existe o `portal-notas.txt` como dado |

Confirmado: `main.rs::faq_source_for` tem um arm `sc` com URLs corretas mas comentário
explícito de que o parser de HTML do RS **não** funcionará para SC sem reescrita.

---

## 6. Divergências e inconsistências entre repositórios (confirmadas no código)

> **RESOLVIDO — integração `data/` (Fases 1–4 da unificação sob `data/`).**
> As divergências 1–4 abaixo foram **eliminadas**: (1) a triplicação do `domain` deixou de existir —
> `data/registry.toml` é a fonte única de entidades, lida por `auli-cli` e `auli-collections`, e o
> frontend gera `entities.ts` dela; o kind vetorial canônico é `services` (o registry mapeia o rótulo
> de UI `servicos`). (2) Os dados de serviços passaram a vir da raspagem nova (decisão #1b), com packs
> e frontend **consistentes**. (3) `sc` virou entidade real do server (208 serviços). (4)
> `pareceres`/`notas`/`conteudos` (autorados, sem scraper) ficam versionados em `data/<id>/ref/`. O
> texto abaixo descreve o estado **anterior** (baseline `auli-server/`, mantido como referência).

1. **`domain` triplicado e divergente.** `auli-server` e `auli-collections` mantêm cópias do
   registro de coleções com diferenças reais:
   - Nome do kind de serviços: **`services`** (servidor, [auli-server/src/domain/collections.rs](auli-server/src/domain/collections.rs))
     vs **`servicos`** (collections, [auli-collections/src/domain/collections.rs](auli-collections/src/domain/collections.rs)).
     Como o nome do kind compõe o nome da coleção vetorial (`<id>-<kind>`, hoje o arquivo
     `<id>-<kind>.json` do vector store in-process), o servidor cria `rs-services`, enquanto o
     domínio (não usado) do scraper modelaria `rs-servicos`.
   - Estratégia/delimitador de serviços: servidor = `//` + `Description`; collections =
     `## pergunta` + `FullText`.
   - `errors.rs`: servidor tem variantes `Custom`/`Anyhow`/`Io`/`SerdeJson`/`Reqwest` (a antiga
     `OllamaError` foi removida na migração v0.2.0); collections tem `Http(ureq::Error)` e
     nenhuma variante de embeddings.
   - `entities.rs`: servidor lê config+dados de `./entities/<id>/`; collections separa config
     (`./src/entities/<id>/`) de dados (`./data/<id>/`).
   - O frontend mantém um terceiro "espelho" do registro de entidades em
     [auli-frontend/src/shared/entities.ts](auli-frontend/src/shared/entities.ts) (hardcoded,
     com `rs` e `sc`).

2. **Formato dos dados de serviços vs modelo do servidor.** O scraper hoje emite
   `portal-servicos.txt` em formato `## pergunta`/`## resposta`, e os dados reais no servidor
   já estão nesse formato; mas o servidor ainda declara `SERVICES.delimiter = "//"` e
   `EmbedStrategy::Description`. Funciona por coincidência (cada bloco começa com `// N.`),
   mas o texto efetivamente embedado para `services` são as primeiras linhas (incluindo os
   marcadores), não o conteúdo completo como em `faqs`.

3. **Entidade `sc` é parcial e desalinhada entre repositórios.** Existe em `auli-collections`
   (serviços ok; faqs não) e no `auli-frontend` (só `servicos`), mas **não existe** no
   `auli-server` (sem `entities/sc/`). Logo o chat/RAG não atende `sc` hoje.

4. **`pareceres` e `notas`**: têm dados (`portal-*.txt`) e são exibidos no frontend (RS) e
   ingeríveis/listáveis no servidor, mas **não são consultados** no RAG e **não têm scraper**
   em `auli-collections`.

---

## 7. Resumo: implementado e ativo vs modelado/inativo

**Ativo e funcionando (confirmado no código):**

- `auli-server` (v0.2.0): `GET /v1/health`, `POST /v1/question` (RAG completo **in-process**:
  fastembed/BGE-M3 → vector store próprio → LLM externo, com log em `./logs/`), ingestão/listagem
  genérica por `{kind}`, autenticação por JWT RS256 (signin/register/middleware), CORS,
  configuração centralizada por `.env` (`config()`), logging via `tracing`.
- RAG consulta efetivamente apenas `services` (10) + `faqs` (20); estreitamento por proximidade
  presente mas em modo paridade (`band=∞`) até calibração.
- `auli-frontend`: SPA com seleção de entidade (rs/sc), chat contra `POST /v1/question` com
  timeout de 25s, abas de referência lendo `public/<id>/`, tema claro/escuro, testes Vitest.
- `auli-collections`: scraper de `faqs` (rs) e de `servicos` (rs via Chrome, sc via JSON),
  cache em disco, modo `--usecache`, geração de `*.json` + `portal-*.txt` + `servicos-index.json`.

**Modelado/declarado mas inativo ou incompleto:**

- Servidor: refresh tokens, verificação de e-mail e reset de senha (tabelas existem; handlers
  comentados). `pareceres`/`notas` ingeríveis mas não consultados no RAG. A rota-placeholder
  `GET /login` foi removida na migração v0.2.0.
- Servidor: dependências `axum-client-ip` e `futures` declaradas mas não usadas.
- Collections: módulo `domain` `#![allow(dead_code)]` (não consumido por pipeline);
  `EmbedStrategy::Description` não usado; `faqs` para `sc` não implementado; sem scraper de
  `pareceres`/`notas`; `servicos` ainda em `Box<dyn Error>` (refatoração pendente).
- Frontend: sem fluxo de autenticação/JWT no cliente; multi-tenant apenas por troca de
  `public/<id>/` (lista de entidades hardcoded).

---

## 8. Itens marcados como NÃO CONFIRMADO NO CÓDIGO

- E-mail/SMTP via `lettre` no servidor: **não** existe (nem no Cargo.toml, só em comentários).
  A documentação (README/CLAUDE.md do servidor) afirma o contrário — incorreto.
- Qualquer sincronização automática entre `auli-collections/data/<id>/`,
  `auli-server/entities/<id>/` e `auli-frontend/public/<id>/`: não há código que faça isso —
  a cópia é manual.
- Fluxo de autenticação no frontend: inexistente no código.
- Entidade `sc` no backend RAG: não há configuração `entities/sc/` no servidor.
- Reranking: o antigo arquivo `auli-server/reranking.rs` foi **removido** na migração v0.2.0;
  não há reranking no caminho RAG ativo. O único estreitamento de resultados é o
  `select_by_proximity` (por distância cosseno), hoje em modo paridade (`band=∞`).

---

## 9. Backend refatorado — o workspace `auli-engine`

O `auli-server` (monólito descrito em §3) foi reorganizado em um **workspace Cargo único**
`auli-engine/`, com **três crates em camadas** e **um binário** que troca de modo por subcomando. A
lógica de negócio (parsing, `EmbedStrategy`, embedder, `cosine_distance`, `select_by_proximity`,
LLM, RAG) foi movida **verbatim**; o que mudou foi a *fronteira entre módulos* e o
*ciclo de vida* (ingestão separada do atendimento). O `auli-server/` segue em disco como baseline.

> **Atualização — auth e banco removidos do workspace.** A camada de autenticação (JWT RS256,
> signin/register, `auth_middleware`, rotas protegidas) e o **PostgreSQL** foram **removidos** do
> `auli` (eram usados *só* para auth). O `server` hoje não tem auth nem banco: expõe apenas as
> rotas **públicas** `GET /v1/health`, `POST /v1/question` e `GET /v1/{kind}/list`, e não carrega
> `JWT_*`/`DATABASE_URL`. As menções a auth/Postgres em §9.2–§9.4 abaixo refletem o estado anterior;
> o baseline `auli-server/` (§3.8) ainda descreve o auth original.

### 9.1 Estrutura (camadas estritas, acoplamento só para baixo)

```
auli-engine/                       # workspace único, Cargo.lock compartilhado
└── crates/
    ├── vector-store/   # BAIXO — store plano por cosseno, agnóstico (sabe só id+vetor+payload P)
    ├── auli-core/      # MEIO  — domínio auli: embed (BGE-M3), corpus, manifest
    └── auli-cli/       # TOPO  — o binário `auli`: server (RAG) + update (ingestão)
```

`vector-store` ← `auli-core` ← `auli-cli`. O `Cargo.lock` único garante que os modos `update`
(embeda documentos) e `server` (embeda a pergunta) usem o **mesmo** `fastembed`/modelo — o espaço
vetorial é compartilhado por construção, não por convenção.

| Crate | Conteúdo |
| --- | --- |
| `vector-store` | `Record<P>`/`CollectionData<P>` (payload genérico; chave JSON em disco continua `document`), `cosine_distance` (fallback `2.0`), IO de arquivo, e a **separação leitura/escrita por tipo**: `ReadStore` (`query_scored`/`list`, imutável) vs `Writer` (`reset`/`upsert`/persistência). Enforcement de dimensão no 1º insert (`Error::DimensionMismatch`). |
| `auli-core` | `embed` (`Embedder` BGE-M3, `EMBED_DIM=1024`), `corpus` (`EmbedStrategy`, tabela `Collection`, `parse_blocks*`, `prepare_documents`, `extract_question`, `clean_servico`, `extract_servico_description` — movidos de [domain/collections.rs](auli-server/src/domain/collections.rs)), `manifest` (identidade do embedding + schema/validação). |
| `auli-cli` | `server` (axum, RAG, config, packs) + `update` (vetorizador). Despacho por `clap`. |

### 9.2 Os dois modos (subcomandos)

```
auli update  --entity <id> --source <dir_com_portal_txt> --out <dir> [--version <v>]
auli server  --port <p> --packs-dir <dir>
```

- **`auli update`** é o **único escritor**: lê `portal-*.txt`, `parse_blocks` →
  `prepare_documents` → `embed_dense` (tudo via `auli-core`), grava `<id>-<kind>.json` +
  `<id>.manifest.json` em `--out`. Não usa o `Config` do server (não precisa de LLM/JWT/DB) —
  só lê `EMBED_CACHE_DIR`/`EMBED_THREADS` do ambiente.
- **`auli server`** é **estritamente leitor**: no boot carrega (eager) todas as coleções via
  `ReadStore`, **valida o manifest** contra a identidade local (modelo+dim+`strategy_version`) e
  **recusa subir** em divergência. Em consulta, embeda **só a pergunta**; nunca escreve e **não
  linka o `Writer`** — incapaz de gravar por construção. A ingestão deixou de ser rota HTTP
  (antes `load_from_file`/`load_from_web`); resta apenas `GET /v1/{kind}/list` (leitura).

### 9.3 Manifest e identidade do embedding

[auli-core::manifest] grava, por entidade, `{ entity, version, built_at, embed_model_id, embed_dim,
strategy_version, collections: [{ kind, count, dim, file, bytes, hash }] }`. `hash` é FNV-1a 64 do
arquivo da coleção (integridade — detecta pacote meio-copiado). `STRATEGY_VERSION` é bumpado sempre
que `prepare_documents`/`parse_*` mudarem (muda *o que* é embedado), transformando "esqueci de
re-gerar os pacotes" em **erro de boot**, não em retrieval ruim. O fallback `2.0` do
`cosine_distance` vira segunda linha de defesa.

### 9.4 Distribuição (decorrência do desenho)

Servir o `auli server` exige **apenas o binário + a pasta de pacotes** (`<id>-<kind>.json` +
`<id>.manifest.json`). Sem banco para subir, sem ChromaDB/Ollama, sem serviço de
embedding, sem rede para ingestão. (Com a remoção do auth, o `server` **não conecta mais a
nenhum banco** — é autossuficiente de ponta a ponta.) Read-only + eager-load + binário único ⇒ N cópias coexistem sem
coordenação.

### 9.5 Verificação (estado de fato, neste repo)

- **Build/test:** `cargo build --workspace` e `cargo test --workspace` passam **sem warnings**;
  **24 testes** (vector-store 10, auli-core 8, auli-cli 5 + 1 de integração) + 1 teste e2e gated.
- **Prova de paridade (refactor puro):** o `diff` das funções movidas (`parse_blocks*`,
  `prepare_documents`, `extract_question`, `clean_servico`, `extract_servico_description`) entre
  [domain/collections.rs](auli-server/src/domain/collections.rs) e `auli-core::corpus` mostra como
  **única** diferença a visibilidade de `parse_block_lines` (`fn` → `pub fn`); `cosine_distance`,
  `select_by_proximity` e o embedder também idênticos.
- **Pacotes reais gerados** via `auli update` para `rs`: services 627, faqs 1734, pareceres 331,
  notas 1. O manifest confere — `bytes` e `hash` FNV-1a batem com os arquivos (hash conferido
  também por uma implementação independente em Python); todos os vetores em dim 1024; chave
  `document` preservada (compatível com o formato anterior).
- **Caminho de atendimento e2e** (teste gated, modelo real): manifest validado → `ReadStore`
  carrega 1734 faqs → pergunta embedada → `query_scored` retorna 20 hits ordenados
  (melhor distância ≈ **0,28**, um match real), com relevância semântica correta.

### 9.6 Diferenças intencionais vs §3 (não são regressões)

- **Ingestão fora do server.** As rotas `load_from_file`/`load_from_web` saíram do server (o
  frontend nunca as usou — só `POST /v1/question`); a vetorização é o `auli update`.
- **Store em memória imutável.** O `get_or_open` *lazy* + `HashMap<.., RwLock<..>>` (§3.6) virou
  carga *eager* + `ReadStore` imutável (sem lock no caminho de consulta).
- **Divergência de `domain` resolvida no backend.** A triplicação descrita em §6 deixa de existir
  *dentro do backend*: `corpus`/`vector-store` são fonte única para `server` e `update`. O
  `auli-frontend` e o `auli-collections` seguem com seus próprios espelhos (fora deste workspace).
