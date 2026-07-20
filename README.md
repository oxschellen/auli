# Auli

**Auli** is an open-source, privacy-first **RAG assistant for Brazilian state taxes**. It helps
tax-office staff answer citizens by turning a natural-language question into a grounded answer
built from the _official content_ of a given state's revenue secretariat (Secretaria da Fazenda) —
services, FAQs, legal opinions (_pareceres_) and administrative notes (_notas_) — with links back
to the source.

The pilot tenant is **SEFAZ-RS** (Rio Grande do Sul); the system is **multi-tenant by state**, so
one codebase serves many secretariats from isolated data.

- 🌐 Production: [auli.com.br](https://auli.com.br) · API: `https://api.auli.com.br/v1`
- 🔒 **Privacy by design** — embeddings run **locally, in-process** (fastembed / BGE-M3 ONNX). No
  external embedding service; question/document text never leaves the process. Only the final
  answer drafting calls an external LLM.
- 📄 License: **MIT**

> 📚 In-depth docs (Portuguese): **[auli_features.md](auli_features.md)** (product),
> **[auli_code.md](auli_code.md)** (code-audited technical reference), and
> **[auli_operations.md](auli_operations.md)** (build/run/deploy runbook).

---

## How it works

```
question
   │
   ▼
local embedding  (fastembed / BGE-M3, in-process, no network)
   │
   ▼
vector search    (in-process flat cosine store, per-state collections)
   │
   ▼
external LLM      (Groq-compatible) drafts the answer from the retrieved context
   │
   ▼
answer + official links
```

Official content is **scraped** from each secretariat's portal, **transformed into structured
text**, and **vectorized into per-state packs** (`auli update`) that the server loads read-only.

### Three faces, one engine

The retrieval engine (`auli-retrieval`) is shared by three interfaces in the **same process**, so
the heavy BGE-M3 model is loaded once:

```
                        ┌──────────────────────┐
  frontend (browser) ──▶│ HTTP  /v1/question   │──▶ chat: prompt + external LLM
                        │ HTTP  /v1/retrieve   │──▶ pure retrieval, no LLM
  an auditor's AI ─────▶│ MCP   /mcp (rmcp)    │──▶ pure retrieval, no LLM
                        └──────────┬───────────┘
                                   │  same process, same Arc<Engine>
                        ┌──────────▼───────────┐
                        │  auli-retrieval      │  BGE-M3 embedder + ReadStores + docs/
                        └──────────────────────┘
```

Only the chat path talks to an external LLM. On `/v1/retrieve` and `/mcp` the question is embedded
locally and **never leaves the process** — so those paths skip the anonymizer and log metadata
only (entity, kind, top_k, hit count, latency), never the question text.

---

## Repository layout

This is a **monorepo** of four cooperating components plus shared docs.

| Path                                                                           | Component            | Role                                                                                                                                                                                             | Stack                        |
| ------------------------------------------------------------------------------ | -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------- |
| [`auli-server/`](auli-server/)                                                 | **auli workspace**   | The current backend: the `auli` binary in two modes — `auli server` (read-only RAG) and `auli update` (vectorizer). Plus the shared `auli-contract` crate and the scraper, all in one workspace. | Rust (Axum, Tokio)           |
| [`auli-frontend/`](auli-frontend/)                                             | **auli-frontend**    | Web UI: state selection (interactive Brazil map), chat, and reference tabs.                                                                                                                      | React 19 + TypeScript + Vite |
| [`auli-server/crates/auli-collections/`](auli-server/crates/auli-collections/) | **auli-collections** | Offline derivation step: turns a scraper snapshot into the typed `auli-contract` (`Table<P>`) + artifacts. The scraping itself is the per-entity `auli-scraper-<id>` crates (sharing `auli-scraper-kit`). | Rust (synchronous)           |
| [`data/`](data/)                                                               | **shared data**      | Single source of truth: `registry.toml` (entities/collections), `prompts/`, and per-state `data/<id>/{raw,ref,packs}/`.                                                                          | TOML + JSON/txt              |
| [`scripts/`](scripts/)                                                         | **tooling**          | `build-packs.sh` (vectorize), `gen-frontend-entities.mjs` + `build-frontend-public.sh` (regen frontend from `data/`).                                                                            | Bash + Node                  |
| `auli_*.md`                                                                    | **docs**             | Product, technical and operations references (Portuguese).                                                                                                                                       | —                            |
| [`start_server.sh`](start_server.sh)                                           | **runbook script**   | Build (incremental) + run the server + Cloudflare tunnel.                                                                                                                                        | Bash                         |

> **One shared `data/` tree, no manual copies.** Entities/collections live once in
> [`data/registry.toml`](data/registry.toml); the scraper writes `data/<id>/raw/`, reference content
> lands in `data/<id>/ref/`, and `auli update` builds `data/<id>/packs/`. The frontend's
> `entities.ts` and `public/<id>/` are **generated** from `data/` by `scripts/` (the prior
> hand-copying is gone). See [auli_code.md](auli_code.md) §2.
>
> **The repo holds code + config, not collected data.** Only `data/registry.toml` and
> `data/prompts/` are versioned. Everything under `data/<id>/**` (ref, raw, packs, scraper cache)
> is **gitignored**: it lives on the collection machine and is rebuilt by the pipeline
> (scraper → `auli-collections` → `auli update`). A fresh clone therefore has **no** state data —
> run the pipeline to populate it. Steps per content type, including the pareceres/consultas flow
> (scrape → sinopse → vectorize), are in [auli_operations.md](auli_operations.md) §4.

---

## Components

### `auli-server/` — backend workspace (current)

A single Cargo workspace with **strict layering** (`auli-contract` is the shared data shape;
`vector-store` ← `auli-core` ← `auli-retrieval` ← `auli-cli`) and **one binary** with two
subcommands. A shared
`Cargo.lock` guarantees the `update` and `server` modes use the _same_ embedding model — the vector
space is shared by construction.

| Crate                                                             | Responsibility                                                                                                                                                                                                     |
| ----------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| [`crates/auli-contract`](auli-server/crates/auli-contract/)       | The **shared data shape** (serde-only): `Table<P>`, `Faq`, `Servico`, and the `Embeddable` trait (`text_to_embed` / `stored_repr`). The single point where the scraper (producer) and the engine (consumer) agree. |
| [`crates/vector-store`](auli-server/crates/vector-store/)         | Generic flat cosine store. Read/write split: `ReadStore` (query, immutable) vs `Writer` (ingest). Dimension enforced on first insert.                                                                              |
| [`crates/auli-core`](auli-server/crates/auli-core/)               | Auli domain: BGE-M3 embedder (dim 1024), the per-kind retrieval knobs (`corpus`), and the pack **manifest** (embedding identity + integrity hash).                                                                 |
| [`crates/auli-retrieval`](auli-server/crates/auli-retrieval/)     | The **retrieval engine**: embedder + `ReadStore`s + proximity narrowing. Read-only by construction (it only ever sees `ReadStore`, never `Writer`) and free of HTTP/LLM/anonymizer — the piece shared by all three faces (`/v1/question`, `/v1/retrieve`, `/mcp`).                    |
| [`crates/auli-cli`](auli-server/crates/auli-cli/)                 | The `auli` binary — `server` (Axum, RAG, `/v1/retrieve`, MCP via `rmcp`, config) and `update` (vectorizer). Dispatch via `clap`.                                                                                                                   |
| [`crates/auli-collections`](auli-server/crates/auli-collections/) | Offline **derivation** (`<id> process`): snapshot → `auli-contract` tables (`<id>-<kind>.json`) + artifacts.                                                                                                                            |
| [`crates/scrapers/auli-scraper-<id>`](auli-server/crates/) + [`auli-scraper-kit`](auli-server/crates/scrapers/auli-scraper-kit/) | The **scrapers** — one binary per state (`rs`/`sc`/`sp`/`pr`/`mg`) writing a snapshot; `auli-scraper-kit` is their shared cache / aggregation / snapshot I/O.                                                                                                                            |

Two modes:

```bash
auli update --entity <id> --source <data/<id>/raw> --out <packs-dir> [--version <v>]   # only writer
auli server [--packs-dir <dir>] [--port 3000]   # read-only; --packs-dir defaults to $AULI_DATA_DIR
```

`auli update` reads the scraper's typed contract (`<source>/<id>-faqs.json`, `<id>-servicos.json` =
`auli_contract::Table<P>`), embeds each record's `text_to_embed` and stores its `stored_repr`.

`auli server` is read-only by construction: it eager-loads collections via `ReadStore`, **validates
the pack manifest** against the local embedding identity at boot (and refuses to start on mismatch),
and only ever embeds the incoming question.

### `auli-frontend/` — web UI

A single-page app (no router; **tab navigation**) built with React 19, Vite, and Chakra UI v3.

- **State selection** with an interactive map of Brazil; choice persisted in `localStorage`.
- **Chat** against `POST /v1/question` (25 s timeout, friendly errors, copy button, markdown).
- **Reference tabs** — Serviços, FAQs, Pareceres, Notas, Conteúdos — each reading static files from
  `public/<id>/`; "coming soon" placeholders for collections a state doesn't have yet.
- Light/dark mode, mobile-first, virtual-keyboard aware. Tested with Vitest.

```bash
cd auli-frontend
npm install
npm run dev        # Vite dev server
npm run build      # tsc --noEmit && vite build
npm test           # Vitest
```

The only backend endpoint the frontend calls is `POST /v1/question` (via `VITE_API_URL`).

### Scraping pipeline — per-entity scrapers + `auli-collections`

Collection is a **two-step, synchronous** pipeline. First a **per-entity scraper binary**
(`auli-scraper-<id>`) fetches the portal and writes a versioned **snapshot**
(one per collection: `data/<id>/<id>-servicos-snapshot.json`, plus `<id>-faqs-snapshot.json` for RS); the scrapers share `auli-scraper-kit` (HTTP cache, service
aggregation, snapshot I/O). Then **`auli-collections <id> process`** derives, offline, the typed
`auli-contract` artifacts (`Table<Faq>` / `Table<Servico>` → `data/<id>/raw/<id>-<kind>.json`,
materializing each record's `text_to_embed`) plus the human-readable `portal-<kind>.txt` audit
_print_ and the per-público fan-out files.

Active scrapers (one crate per state):

- **RS** (`auli-scraper-rs`) — FAQs (portal CMS via AJAX/`ureq`) + serviços (**headless Chrome** for
  the listing, `ureq` for details). The only crate that uses Chrome.
- **SC** (`auli-scraper-sc`) — serviços via SEF-SC Next.js JSON API.
- **SP** (`auli-scraper-sp`) — serviços via SharePoint REST (anonymous JSON).
- **PR** (`auli-scraper-pr`) — serviços via server-side Drupal HTML.
- **MG** (`auli-scraper-mg`) — serviços via ServiceNow CSM page API (JSON).
- On-disk **cache** with an offline `--usecache` mode; **dedup** of services shared across audiences.

```bash
cd auli-server
cargo run -p auli-scraper-rs -- [--usecache] servicos   # scrape RS -> snapshot (faqs|servicos|all)
cargo run -p auli-collections -- rs process             # derive artifacts from the snapshot (offline)
```

---

## Quick start (backend, the live path)

Full runbook (cmake notes, Cloudflare Tunnel, logs, troubleshooting): **[auli_operations.md](auli_operations.md)**.

**Prerequisites:** Rust (stable) · `cmake` + a C compiler (for `aws-lc-sys`) · a `.env` in the repo
root (see below). No database is required. First build/run downloads the ONNX Runtime and the
BGE-M3 model from Hugging Face.

```bash
# 1. Configure
cp .env.example .env        # then fill in LLM_API_* (LLM endpoint)

# 2. Build + run server + Cloudflare Tunnel (from repo root)
./start_server.sh                  # build (incremental) + server + cloudflared tunnel
./start_server.sh --no-build       # fast restart, no recompile
./start_server.sh --no-tunnel      # local server only
```

Generate the vector packs the server serves (only needed when content or the embedding strategy
changes) — `build-packs.sh` runs `auli update` over the scraper's contract in `data/<id>/raw/` into
`data/<id>/packs/` (`pareceres`/`notas` have no struct source yet and are skipped):

```bash
scripts/build-packs.sh rs          # per entity: rs | sc | sp | pr | mg
```

A healthy boot logs the loaded entities, a validated manifest, per-collection record counts, the
embedder, and `✅ Server started successfully at 0.0.0.0:3000`.

**Smoke test:**

```bash
curl -s localhost:3000/v1/health
curl -s -X POST localhost:3000/v1/question -H 'Content-Type: application/json' \
  -d '{"entity":"rs","question":"Como obtenho certidão negativa de débitos?"}'
```

---

## Environment

The server loads its config from a `.env` in the repo root (see **[.env.example](.env.example)**).
Required variables panic at startup if missing.

| Variable                                        | Required | Purpose                                                                                                 |
| ----------------------------------------------- | -------- | ------------------------------------------------------------------------------------------------------- |
| `LLM_API_URL` / `LLM_API_KEY` / `LLM_API_MODEL` | ✅       | External LLM (Groq-compatible) that drafts the answer                                                   |
| `EMBED_CACHE_DIR`                               | —        | BGE-M3 model cache dir. Launchers set it to `<repo-root>/models` (absolute); code default is `./models` |
| `EMBED_THREADS`                                 | —        | ONNX Runtime intra-op threads (default 16)                                                              |

> Secrets (`.env`, `*.pem`) and build artifacts (`target/`, `node_modules/`, `models/`, `packs/`,
> `vectors/`, `logs/`) are **gitignored** and never committed.

---

## Content types

| Type          | What it is                                     | Where it appears today    |
| ------------- | ---------------------------------------------- | ------------------------- |
| **Serviços**  | The secretariat's service catalog, by audience | Chat (RAG) + Serviços tab |
| **FAQs**      | Official frequently-asked questions            | Chat (RAG) + FAQs tab     |
| **Pareceres** | Legal/technical opinions                       | Pareceres tab (reference) |
| **Notas**     | Administrative/tax notes                       | Notas tab (reference)     |
| **Conteúdos** | Misc reference materials                       | Conteúdos tab (reference) |

Today **Serviços and FAQs** feed the assistant's answers; **Pareceres, Notas and Conteúdos** are
available as reference navigation in the UI.

---

## Status

- **Working today:** RAG chat for the configured state, full UI (chat + reference tabs + state
  selection with map), and local embeddings. **Five states active** — Serviços for RS, SC, SP, PR
  and MG, plus FAQs for RS. The backend is open (no auth) and database-free — it serves from packs
  alone.
- **In progress:** Serviços for more states, FAQs beyond RS, automated scraping of Pareceres/Notas,
  and using those reference types in the assistant's answers.

For the precise active-vs-modeled breakdown (routes, auth flows, cross-repo divergences), see
**[auli_code.md](auli_code.md)** §7.

---

## License

[MIT](LICENSE) — © 2026 Carlos Henrique Schellenberger and contributors.
