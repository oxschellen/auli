# Auli

**Auli** is an open-source, privacy-first **RAG assistant for Brazilian state taxes**. It helps
tax-office staff answer citizens by turning a natural-language question into a grounded answer
built from the *official content* of a given state's revenue secretariat (Secretaria da Fazenda) —
services, FAQs, legal opinions (*pareceres*) and administrative notes (*notas*) — with links back
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

---

## Repository layout

This is a **monorepo** of four cooperating components plus shared docs.

| Path | Component | Role | Stack |
| --- | --- | --- | --- |
| [`auli-engine/`](auli-engine/) | **auli workspace** | The current backend: the `auli` binary in two modes — `auli server` (read-only RAG) and `auli update` (vectorizer). Plus the shared `auli-contract` crate and the scraper, all in one workspace. | Rust (Axum, Tokio) |
| [`auli-frontend/`](auli-frontend/) | **auli-frontend** | Web UI: state selection (interactive Brazil map), chat, and reference tabs. | React 19 + TypeScript + Vite |
| [`auli-engine/crates/auli-collections/`](auli-engine/crates/auli-collections/) | **auli-collections** | Scrapers that collect official content and compile it into the typed `auli-contract` (`Table<P>`). Now a workspace crate. | Rust (synchronous) |
| [`data/`](data/) | **shared data** | Single source of truth: `registry.toml` (entities/collections), `prompts/`, and per-state `data/<id>/{raw,ref,packs}/`. | TOML + JSON/txt |
| [`scripts/`](scripts/) | **tooling** | `build-packs.sh` (vectorize), `gen-frontend-entities.mjs` + `build-frontend-public.sh` (regen frontend from `data/`). | Bash + Node |
| `auli_*.md` | **docs** | Product, technical and operations references (Portuguese). | — |
| [`start_server.sh`](start_server.sh) | **runbook script** | Build (incremental) + run the server + Cloudflare tunnel. | Bash |

> **One shared `data/` tree, no manual copies.** Entities/collections live once in
> [`data/registry.toml`](data/registry.toml); the scraper writes `data/<id>/raw/`, authored
> reference content is versioned in `data/<id>/ref/`, and `auli update` builds `data/<id>/packs/`.
> The frontend's `entities.ts` and `public/<id>/` are **generated** from `data/` by `scripts/`
> (the prior hand-copying is gone). See [auli_code.md](auli_code.md) §2.

---

## Components

### `auli-engine/` — backend workspace (current)

A single Cargo workspace with **strict layering** (`auli-contract` is the shared data shape;
`vector-store` ← `auli-core` ← `auli-cli`) and **one binary** with two subcommands. A shared
`Cargo.lock` guarantees the `update` and `server` modes use the *same* embedding model — the vector
space is shared by construction.

| Crate | Responsibility |
| --- | --- |
| [`crates/auli-contract`](auli-engine/crates/auli-contract/) | The **shared data shape** (serde-only): `Table<P>`, `Faq`, `Servico`, and the `Embeddable` trait (`text_to_embed` / `stored_repr`). The single point where the scraper (producer) and the engine (consumer) agree. |
| [`crates/vector-store`](auli-engine/crates/vector-store/) | Generic flat cosine store. Read/write split: `ReadStore` (query, immutable) vs `Writer` (ingest). Dimension enforced on first insert. |
| [`crates/auli-core`](auli-engine/crates/auli-core/) | Auli domain: BGE-M3 embedder (dim 1024), the per-kind retrieval knobs (`corpus`), and the pack **manifest** (embedding identity + integrity hash). |
| [`crates/auli-cli`](auli-engine/crates/auli-cli/) | The `auli` binary — `server` (Axum, RAG, config) and `update` (vectorizer). Dispatch via `clap`. |
| [`crates/auli-collections`](auli-engine/crates/auli-collections/) | The scrapers — compile portal content into `auli-contract` tables (`<id>-<kind>.json`). |

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

### `auli-engine/crates/auli-collections/` — scrapers

A synchronous Rust program that collects content from a secretariat's portal and compiles it into
the typed `auli-contract` (`Table<Faq>` / `Table<Servico>` → `data/<id>/raw/<id>-<kind>.json`),
materializing each record's `text_to_embed`. It also writes the human-readable `portal-<kind>.txt`
(an audit *print* of the struct, never read back).

- **FAQs** — SEFAZ-RS portal (headless Chrome + `ureq`).
- **Services** — RS (headless Chrome) and SC (SEF-SC Next.js JSON API).
- On-disk **cache** with an offline `--usecache` mode; **dedup** of services shared across audiences.

```bash
cd auli-engine
cargo run -p auli-collections -- [--usecache] <entity> <collection>   # e.g. ... -- rs servicos
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
scripts/build-packs.sh rs          # and: scripts/build-packs.sh sc
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

| Variable | Required | Purpose |
| --- | --- | --- |
| `LLM_API_URL` / `LLM_API_KEY` / `LLM_API_MODEL` | ✅ | External LLM (Groq-compatible) that drafts the answer |
| `EMBED_CACHE_DIR` | — | BGE-M3 model cache dir. Launchers set it to `<repo-root>/models` (absolute); code default is `./models` |
| `EMBED_THREADS` | — | ONNX Runtime intra-op threads (default 16) |

> Secrets (`.env`, `*.pem`) and build artifacts (`target/`, `node_modules/`, `models/`, `packs/`,
> `vectors/`, `logs/`) are **gitignored** and never committed.

---

## Content types

| Type | What it is | Where it appears today |
| --- | --- | --- |
| **Serviços** | The secretariat's service catalog, by audience | Chat (RAG) + Serviços tab |
| **FAQs** | Official frequently-asked questions | Chat (RAG) + FAQs tab |
| **Pareceres** | Legal/technical opinions | Pareceres tab (reference) |
| **Notas** | Administrative/tax notes | Notas tab (reference) |
| **Conteúdos** | Misc reference materials | Conteúdos tab (reference) |

Today **Serviços and FAQs** feed the assistant's answers; **Pareceres, Notas and Conteúdos** are
available as reference navigation in the UI.

---

## Status

- **Working today:** RAG chat for the configured state, full UI (chat + reference tabs + state
  selection with map), scraping of Serviços/FAQs (RS) and Serviços (SC), and local embeddings. The
  backend is open (no auth) and database-free — it serves from packs alone.
- **In progress:** expanding state **SC** (FAQs and other content) on the backend, automated
  scraping of Pareceres/Notas, and using those types in the assistant's answers.

For the precise active-vs-modeled breakdown (routes, auth flows, cross-repo divergences), see
**[auli_code.md](auli_code.md)** §7.

---

## License

[MIT](LICENSE) — © 2026 Carlos Henrique Schellenberger and contributors.
