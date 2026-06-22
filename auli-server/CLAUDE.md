# CLAUDE.md — auli-server

> **Superseded / baseline.** This monolith has been refactored into the single Cargo workspace
> `../auli/` — three layered crates (`vector-store` ← `auli-core` ← `auli-cli`) and one binary with
> two subcommands (`auli server` / `auli update`). The business logic here was carried over
> **verbatim**; `auli-server/` is kept as the pre-refactor reference baseline. For the current
> backend see `../auli/README.md` and `../auli_code.md` §9. Edit the workspace, not this tree,
> unless you specifically need the baseline.

Working context for this repository. Read this first; it reflects the current state of the
code, not just its history.

## What this is

`auli-server` is a Rust (edition 2021) async REST API that powers the virtual assistant at
[auli.com.br](https://auli.com.br). It started as a pilot for the Rio Grande do Sul state tax
authority (SEFAZ-RS) and is now **multi-tenant**: one server instance serves multiple
**entities** (states), each with its own data and LLM system prompt.

It answers natural-language questions via a **RAG pipeline**: embed the question locally
(fastembed/BGE-M3, in-process) → retrieve similar documents from an in-process vector store →
pass them as context to an external LLM (Groq-compatible) → return the answer. Domain content covers public services, FAQs,
legal opinions (*pareceres*), and administrative notes (*notas*). Logs and most code comments
are in Brazilian Portuguese.

## External services (all expected at these ports)

| Service | Port | Role |
| --- | --- | --- |
| This server (Axum) | 3000 | HTTP API, routes under `/v1/` |
| Vector store | — | **In-process** (no service). Pure-Rust flat cosine index, persisted as `<id>-<kind>.json` under `VECTOR_DB_PATH` (default `./vectors`). See `clients/vector_store.rs`. |
| Embedder | — | **In-process** embeddings (`fastembed` / BGE-M3 ONNX INT8, dim 1024). No external service; model cached under `EMBED_CACHE_DIR`. See `clients/embedder.rs`. |
| PostgreSQL | 5432 | User auth (largely inactive — see Caveats) |
| External LLM | — | Groq-compatible chat completions via `LLM_API_*` |

`./scripts/start_all.sh` opens two gnome-terminals: the Rust server (vector store + embedder run
in-process inside it) and an ngrok tunnel (`api.auli.com.br`). For a headless/WSL run without
ngrok, `./scripts/start_local.sh` kills any old instance (it matches the binary by *path*, so
stale-named binaries are caught too), recompiles `--release`, and serves on :3000 in the current
terminal.

## Multi-tenancy (the central design point)

Each entity lives in `./entities/<id>/`:

```
entities/rs/
├── entity.json          { "id": "rs", "name": "SEFAZ-RS" }
├── prompt.txt           LLM system instructions for this entity
├── portal-servicos.txt  source data (services)
├── portal-faqs.txt      source data (faqs)
├── portal-pareceres.txt source data (pareceres)
└── portal-notas.txt     source data (notas)
```

- The registry [src/domain/entities.rs](src/domain/entities.rs) scans `./entities/` once at startup into
  `static ENTITIES: LazyLock<HashMap<String, EntityConfig>>`. `get_entity(Option<&str>)`
  resolves an id (None/empty → `DEFAULT_ENTITY = "rs"`) and returns a friendly `Err(String)`
  for unknown ids.
- `EntityConfig::collection(kind)` → `"<id>-<kind>"`; `EntityConfig::data_file(name)` →
  `"<data_dir>/<name>"`. Kinds: `services`, `faqs`, `pareceres`, `notas`.
- The four kinds are described once in [src/domain/collections.rs](src/domain/collections.rs) as
  `static` `Collection` values (`SERVICES`, `FAQS`, `PARECERES`, `NOTAS`), each carrying
  `kind`, source `file`, block `delimiter`, `embed` strategy, and RAG `n_results`.
  `collections::from_kind(&str)` resolves the route's `{kind}` to one. All ingest/list/RAG
  code is generic over `&Collection` — adding a content kind = one new `Collection` entry.
- **Vector collections are namespaced per entity**: `rs-services`, `rs-faqs`,
  `rs-pareceres`, `rs-notas` (each a `<name>.json` file under `./vectors`). Each entity's
  vectors are isolated.
- **Entity selection per request**: optional `entity` field in the POST body
  (`Question`, `InputServices`), or `?entity=<id>` query param on GET data routes
  (`EntityQuery`). Missing → defaults to `rs`, so legacy clients keep working.
- **Adding a state**: drop a new `entities/<id>/` dir, restart (registry loads at startup),
  then ingest with `?entity=<id>`. CORS and LLM/embedding config remain global.

## Module map (`src/`)

Layered by concern. `main.rs` is a thin entrypoint; everything lives in the `auli_server`
library (`lib.rs`) so `tests/` can build the router.

```
main.rs    #[tokio::main] -> auli_server::run()
lib.rs     module decls; app(state) -> Router (router assembly); run() (pool + serve :3000)
config.rs  Config struct behind config() (loads/validates env once: LLM, embedder, JWT, DB)
errors.rs  Unified Error (thiserror) / Result alias
state.rs   AppState (PG pool + Arc<VectorStore> + Arc<Embedder> + JWT config); built in run()

api/                HTTP layer
  mod.rs            public/auth/protected/data route groups + CORS (hardcoded origins)
  dto.rs            Question, Answer, QuestionResponse, InputServices, EntityQuery
  handlers/
    health.rs       GET /v1/health
    question.rs     POST /v1/question — reads entity, calls rag::exec_all_question
    collections.rs  generic list/load_from_file/load_from_web, keyed by the {kind} path param

domain/             core types & registries (no I/O)
  entities.rs       Multi-tenant registry: EntityConfig, ENTITIES, get_entity, init
  collections.rs    Generic content-kind registry: Collection, EmbedStrategy, SERVICES/FAQS/
                    PARECERES/NOTAS consts, from_kind, parse_blocks(_from_text), prepare_documents

rag/
  pipeline.rs       ACTIVE Q&A path exec_all_question(vector, embedder, question, entity): embeds
                    the question once (via spawn_blocking), query_scored on <id>-services + <id>-faqs (ceilings from
                    collections::SERVICES/FAQS.n_results) concurrently via spawn_blocking,
                    narrows by proximity (select_by_proximity), calls the LLM, logs the exchange

clients/            adapters for external services + the in-process vector store / embedder
  embedder.rs       Embedder: Arc<Mutex<Bgem3Embedding>> (fastembed/BGE-M3); embed_dense (sync,
                    blocking; dense Vec<Vec<f32>>). Built once at startup; embed is &mut self.
  vector_store.rs   VectorStore: get_or_open registry, upsert, reset, query_scored, list
                    (pure-Rust flat cosine index over per-collection JSON; sync/blocking)
  ingest.rs         load_collection(vector, embedder, entity, collection, blocks): prepare ->
                    embed keys (fastembed, spawn_blocking) -> reset + upsert
  llm.rs            chat(system_prompt, user_message) -> Groq-compatible completion

auth/
  jwt.rs            RS256 JWT encode/decode, RSA keypair generation
  handler.rs        sign_in, register, auth_middleware, user_get
  types.rs          legacy/commented auth-flow sketches
```

The four content kinds share one generic code path (`domain/collections.rs` +
`api/handlers/collections.rs` + `clients::ingest::load_collection` + `clients::vector_store`).

## Request / RAG flow (the live path)

1. `POST /v1/question` with `{ "question": "...", "entity": "rs" }` (`entity` optional).
2. [api/handlers/question.rs](src/api/handlers/question.rs) (extracts `State<AppState>`,
   `entity` + `question`) calls
   `rag::exec_all_question(state.vector.clone(), state.embedder.clone(), question, entity)`.
3. [rag/pipeline.rs](src/rag/pipeline.rs): `get_entity` → resolve config;
   embed question (fastembed, via `spawn_blocking`); `query_scored` on `<id>-services` (ceiling 10) and `<id>-faqs`
   (ceiling 20), then narrow by proximity (`select_by_proximity`; bands default to ∞ =
   parity with the old fixed take — calibrate via the printed score arrays); concatenate as
   RAG context; build system prompt = entity `prompt.txt` + RAG; POST to `LLM_API_URL`
   (temp 0.5, max 1024 tokens, 3 connect retries).
4. Returns `{ "question", "answer" }`. Unknown entity → the answer is the friendly error
   string (HTTP still 200). Each call appends to `./logs/<timestamp>.txt`.

Only `services` + `faqs` feed live answers. `pareceres`/`notas` have ingest + list endpoints
but are not queried by `exec_all_question`.

## Data ingestion & collection naming

Content is loaded from each entity's `.txt` files into per-entity vector collections via
generic `{kind}` routes; pass `?entity=<id>` (default `rs`). `kind ∈ services | faqs |
pareceres | notas`. Each `load_*` does a clean full reload: `VectorStore::reset` then upsert
with ids `id-1..id-N` (so re-ingesting fewer blocks leaves no orphans). Each route resolves
`collections::from_kind(kind)` for the source file, delimiter, and embed strategy, then runs
the single `ingest::load_collection` path.
These routes require `Authorization: Bearer <token>` for a verified PostgreSQL user.

| Endpoint | Collection | Source |
| --- | --- | --- |
| `GET /v1/{kind}/load_from_file` | `<id>-<kind>` | `entities/<id>/portal-<...>.txt` |
| `POST /v1/{kind}/load_from_web` | `<id>-<kind>` | request body (`InputServices`, `entity` field) |
| `GET /v1/{kind}/list` | `<id>-<kind>` | — |

(`portal-<...>.txt` = `portal-servicos.txt` for `services`, otherwise `portal-<kind>.txt`.)

File formats & embed strategy (the **key/payload split** — embed a short high-signal key, store
& serve the full block; see `EmbedStrategy` in [domain/collections.rs](src/domain/collections.rs)):

- **FAQs / Pareceres** (`EmbedStrategy::QuestionKey`): blocks delimited by `## pergunta` (with
  following `## resposta` lines) — one block = one document. The **embedded key** is the parsed
  `## pergunta` field only (`extract_question`); the **stored/served payload** is the full Q+A
  block. (Tradeoff: answer-body-only content won't surface via dense retrieval — that's the
  Phase-2 sparse/hybrid trigger.)
- **Notas** (`EmbedStrategy::FullText`): `## pergunta`-delimited but intentionally a single
  block (one large prompt template, so `rs-notas` legitimately has 1 record); not queried live.
- **Services** (`EmbedStrategy::Description`): blocks delimited by `//` lines; first ~4 lines
  (≤300 chars of the description) are the embedded key, full cleaned text stored as the document.

## Routes summary

- Public: `GET /v1/health`, `POST /v1/question`, `POST /v1/signin`,
  `GET /v1/gen_rsa_keypair`, `POST /register` (`/register` creates an unverified user and
  does not return a token).
- Protected (JWT): `GET /v1/protected`, `POST /v1/protected_question`.
- Data mgmt: generic `GET /v1/{kind}/list`, `GET /v1/{kind}/load_from_file`,
  `POST /v1/{kind}/load_from_web` (kind ∈ services|faqs|pareceres|notas); JWT required.
- CORS allowed origins are hardcoded in [api/mod.rs](src/api/mod.rs) (auli.com.br + localhost
  dev ports). Methods GET/POST/OPTIONS, credentials enabled.

## Build / run / verify

```bash
cargo build                      # debug
cargo test                       # tests/api.rs builds the router and hits /v1/health (no DB)
./target/debug/auli-server       # needs .env + Postgres up (vector store + embedder are in-process; model downloads on first run)
# or: cargo build --release && ./target/release/auli-server
```

> Build note: this crate targets **Linux** (the deploy box) and builds cleanly there with a
> normal toolchain plus `cmake` + a C compiler (needed by `aws-lc-sys`, rustls's default crypto).
> TLS is **rustls** throughout (`reqwest` default-tls, `sqlx` `runtime-tokio-rustls`). `fastembed`
> pulls `ort`, which **downloads a prebuilt ONNX Runtime at build time** (network needed), and the
> BGE-M3 model downloads from Hugging Face on first run into `EMBED_CACHE_DIR` — pre-stage both for
> air-gapped deploys (`ort-load-dynamic` + `Bgem3Embedding::try_new_from_path`).
>
> It does **not** build on the local `x86_64-pc-windows-gnu` box: that toolchain ships **no
> assembler** (`as.exe`) and its bundled `dlltool.exe` needs one, so low-level deps that emit
> `raw-dylib` system imports — `windows-sys 0.61` (ntdll, via tokio/mio) and `parking_lot_core`
> (kernel32) — fail to link, and `windows-sys 0.61` can't be pinned down because tokio 1.52
> requires it. (`fastembed`/`ort` add an MSVC-vs-gnu prebuilt + firewalled download on top.)
> Editing/inspection work here; compile/verify on Linux. `cargo check` is possible locally only
> *without* fastembed.
>
> **Local dev loop (this machine): edit on Windows, build/run on WSL.** Two copies coexist — the
> Windows edit copy (this repo) and a WSL/Ubuntu build copy that compiles and runs clean.
> `scripts/sync-to-wsl.ps1` pushes Windows → WSL (one-way `rsync --delete`, but preserves the WSL
> copy's own `.env`, `*.pem`, `vectors/`, `models/`, `logs/`, `target/`); `-DryRun` previews.
> The rsync logic lives in `scripts/auli-sync.sh`, where the accented OneDrive path is a UTF-8
> literal *inside* the file — never passed as a `wsl.exe` argument (that breaks). After syncing,
> run `scripts/start_local.sh` inside WSL.

Startup logs print a non-secret env summary, `🏛️  Entidades carregadas: [rs]`, PG connect, and
`✅ Server started ... 0.0.0.0:3000`.

Smoke tests:

```bash
curl -s localhost:3000/v1/health
curl -s -X POST localhost:3000/v1/question -H 'Content-Type: application/json' \
  -d '{"entity":"rs","question":"Como obtenho certidão negativa?"}'
# unknown entity -> friendly error, no panic:
curl -s -X POST localhost:3000/v1/question -H 'Content-Type: application/json' \
  -d '{"entity":"zz","question":"x"}'
```

Inspect a vector collection (it's just a JSON file):

```
ls ./vectors                                   # rs-services.json, rs-faqs.json, …
curl -s 'localhost:3000/v1/faqs/list?entity=rs' -H 'Authorization: Bearer <token>'
```

## Environment variables (`.env` in repo root)

| Variable | Purpose |
| --- | --- |
| `DATABASE_URL` | PostgreSQL connection string |
| `LLM_API_URL` / `LLM_API_KEY` / `LLM_API_MODEL` | Groq-compatible chat completions |
| `EMBED_CACHE_DIR` | Dir for the in-process embedder's ONNX model cache (default `./models`) |
| `EMBED_THREADS` | ONNX Runtime intra-op CPU threads (default 16) |
| `JWT_RSA_PRIVATE_KEY` / `JWT_RSA_PUBLIC_KEY` | PEM RSA keypair for RS256 JWT (active) |
| `JWT_SECRET` | HMAC secret (legacy; RS256 is what the code uses) |
| `JWT_ACCESS_EXPIRATION_MINUTES` / `JWT_REFRESH_EXPIRATION_DAYS` | Token TTLs (default 15 / 7) |
| `POSTGRES_USER` / `POSTGRES_PASSWORD` | DB credentials |
| `VERIFICATION_TOKEN_EXPIRY_HOURS` / `PASSWORD_RESET_TOKEN_EXPIRY_HOURS` | default 24 / 1 |
| `VECTOR_DB_PATH` | Directory for the in-process vector store (default `./vectors`) |

Env access is centralized in [config.rs](src/config.rs) (a `Config` struct behind `config()`,
loaded once; required vars panic at load, optionals have defaults). See `.env.example`.

## Authentication & database

- JWTs are **RS256** (2048-bit RSA keypair from env). `POST /v1/signin` looks up the user in
  PostgreSQL, requires `is_verified = TRUE`, verifies Argon2 hashes (with bcrypt fallback for
  legacy hashes), and returns a token using `JWT_ACCESS_EXPIRATION_MINUTES`. `POST /register`
  hashes with **Argon2**, inserts an unverified user, and does not return a token.
  `auth_middleware` validates `Authorization: Bearer <token>`, re-checks that the user exists
  and is verified, and injects `CurrentUser`.
- Migrations in `migrations/` (applied via `sqlx migrate run`):

  | Migration | Table | Purpose |
  | --- | --- | --- |
  | `20240101` | `users` | accounts (email, password_hash, is_verified) |
  | `20240102` | `refresh_tokens` | refresh token storage |
  | `20240103` | `verification_tokens` | email verification flow |
  | `20240104` | `password_reset_tokens` | password reset flow |

  These tables are modeled but the flows are largely inactive (see Caveats).

## Key dependencies

`axum 0.8` + `tokio` (web/async) · `tower-http` (CORS) · in-process vector store
(pure-Rust, `serde_json`-persisted — no external vector DB) · `fastembed 5` (BGE-M3 ONNX,
in-process embeddings) · `reqwest` (LLM HTTP) · `sqlx`/postgres (DB) ·
`jsonwebtoken` + `rsa` + `argon2`/`bcrypt` (auth) · `serde`/`serde_json` ·
`anyhow`/`thiserror` (errors) · `tracing`/`tracing-subscriber` (logging) ·
`derive_more` (Display on types) · `dotenvy` · `chrono`.

## Caveats / known state (don't be surprised)

- The non-fastembed code is **warning-free** (last verified via `cargo check` on the
  collections/store/pipeline before the fastembed swap); the full build, including `fastembed`/`ort`,
  is only checkable on Linux (see Build note). Old dead modules (`exec_*`, `embedding_api.rs`,
  `auth_old.rs`, `errors_module/`) are gone, and Ollama (`ollama.rs` + `ollama-rs`) was removed in
  the fastembed migration. `exec_all_question` is the only Q&A executor.
- **Auth still has missing product flows**: refresh tokens, email verification delivery, and
  password reset are modeled in migrations but have no active handlers. New users remain
  unverified until updated out of band. Data-management routes require a verified user, but
  there is not yet role/admin authorization beyond that.
- **Secrets**: `.env`, `jwt_private_key.pem`, `jwt_public_key.pem` exist on disk but are
  **gitignored / untracked** (see `.gitignore`); `.env.example` is the committed template.
- **Scraping artifacts** (`servicos-a-*.json`, `faq_site_tree.json`) live under `scripts/raw/`;
  the loaders read only from `entities/<id>/`.
- **Vector store is brute-force**: every query is an exact cosine scan over the whole
  collection, loaded fully into memory on first access and written back to its JSON file on
  every ingest. Fine for the hundreds-of-docs corpora here; revisit (ANN index / on-disk
  format) if a collection grows to many thousands. `query_scored` returns a cosine *distance*
  (lower = closer); the pipeline's `*_BAND` defaults are ∞ (no narrowing) until calibrated.
- **Embedder is BGE-M3 INT8, CPU-only** (`BGEM3Q` — no CUDA EP; tune `EMBED_THREADS`). Loaded
  once at startup via `Embedder::new` (slow; downloads from Hugging Face into `EMBED_CACHE_DIR`
  on first run, so first boot needs network). `embed` is `&mut self`, hence the `Mutex` (calls
  serialize — fine for NAVI volume; pool it if throughput ever matters).
- **Changing the embedding model ⇒ full re-ingest of every collection.** Vectors carry no
  dimension tag and `cosine_distance` scores mismatched widths as max-distance (`2.0`, the
  metric's true maximum), so a collection still holding old-model vectors silently returns
  garbage. Reload each kind (`load_from_file`) against the new model; never mix.
- **Embed key vs. payload**: FAQs/pareceres embed only the parsed `## pergunta` *key* but store
  the full Q+A *payload* (`EmbedStrategy::QuestionKey`). Content that exists only in an answer
  body won't surface via dense retrieval — the documented Phase-2 sparse/hybrid trigger.

## Conventions

- Match the surrounding Portuguese naming and log style when editing handlers.
- New per-entity behavior goes through `EntityConfig` (`collection(kind)` / `data_file`),
  never hardcode `portal-*` collection names or root file paths again.
- Public handler fn names are the API contract used by `api/mod.rs`; keep them stable when
  moving code between files.
- External-service calls belong in `clients/`; keep `rag/pipeline.rs` orchestration-only.
