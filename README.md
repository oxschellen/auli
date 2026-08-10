# Auli

**Auli** is an open-source, privacy-first **RAG assistant for Brazilian state taxes**. It helps
tax-office staff answer citizens by turning a natural-language question into a grounded answer
built from the _official content_ of a given state's revenue secretariat (Secretaria da Fazenda) —
services, FAQs, tax-court rulings (_acórdãos_ of the TARF), legal opinions (_pareceres_) and
administrative notes (_notas_) — with links back to the source.

The pilot tenant is **SEFAZ-RS** (Rio Grande do Sul); the system is **multi-tenant by state**, so
one codebase serves many secretariats from isolated data. **All 27 registered states** have their
service catalog collected. As of 2026-08-09 the corpus is **48.408 documents**: 22.476 acórdãos of
the TARF, 19.780 pareceres, 4.205 serviços and 1.947 FAQs.

- 🌐 Production: [auli.com.br](https://auli.com.br) · API: `https://api.auli.com.br/v1` ·
  MCP: `https://api.auli.com.br/mcp`
- 🔒 **Privacy by design** — embeddings run **locally, in-process** (fastembed / BGE-M3 ONNX). No
  external embedding service; question/document text never leaves the process. Only the final
  answer drafting calls an external LLM.
- 🤖 **Usable from your own AI** — the MCP server exposes the case-law corpora (pareceres and TARF
  acórdãos, picked with a `colecao` parameter) to ChatGPT, Claude and Copilot; step-by-step guides
  live in [`manuais/`](manuais/). Prefer your own pipeline? Each
  state's corpus is downloadable as a `.zip` of one `.md` per document.
- 📄 License: **MIT**

> 📚 In-depth docs (Portuguese): **[docs/auli_features.md](docs/auli_features.md)** (product),
> **[docs/auli_code.md](docs/auli_code.md)** (code-audited technical reference), and
> **[docs/auli_operations.md](docs/auli_operations.md)** (build/run/deploy runbook).

---

## How it works

```text
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

Official content is **scraped** from each secretariat's portal, **derived into a tree of one
`.md` per document** (`data/<id>/docs/<kind>/`, frontmatter + body), and **vectorized into
per-state packs** (`auli update`) that the server loads read-only. That `.md` tree — not the JSON
contract — is the vectorization source for every collection; it is also what the
download `.zip` ships, so what an external AI reads is byte-for-byte what the engine indexed.

### Three faces, one engine

The retrieval engine (`auli-retrieval`) is shared by three interfaces in the **same process**, so
the heavy BGE-M3 model is loaded once:

```text
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
| [`auli-server/`](auli-server/)                                                 | **auli workspace**   | The current backend: the `auli` binary in three modes — `auli server` (read-only RAG), `auli update` (vectorizer) and `auli bundle` (download zips). Plus the shared `auli-contract` crate and the scrapers, all in one workspace. | Rust (Axum, Tokio)           |
| [`auli-frontend/`](auli-frontend/)                                             | **auli-frontend**    | Web UI: state selection (interactive Brazil map), chat, and reference tabs.                                                                                                                      | React 19 + TypeScript + Vite |
| [`auli-server/crates/auli-collections/`](auli-server/crates/auli-collections/) | **auli-collections** | Offline derivation step: turns a scraper snapshot into the typed `auli-contract` (`Table<P>`) + artifacts. The scraping itself is the per-entity `auli-scraper-<id>` crates (sharing `auli-scraper-kit`). | Rust (synchronous)           |
| [`config/`](config/)                                                           | **catalog**          | **Start here to adapt Auli.** `registry.toml` (entities/collections) and `prompts/` — configuration, versioned, no recompile needed.                                                              | TOML + txt                   |
| [`data/`](data/)                                                               | **collected data**   | Per-state `data/<id>/{raw,ref,docs,packs}/`. Gitignored: rebuilt by the pipeline.                                                                                                                 | JSON/txt + Markdown          |
| [`scripts/`](scripts/)                                                         | **operation**        | Everything needed to publish and serve: `publicar.sh` (one command for the lot), `deploy-frontend.sh` (staged, atomic publish), `build-frontend-public.sh`, `build-packs.sh` (vectorize). Call order in [`scripts/README.md`](scripts/README.md). | Bash + Python                |
| [`scripts/tools/`](scripts/tools/)                                             | **occasional tools** | What is _not_ in the operation path: re-scraping (`atualizar-servicos.sh`, `update-rs-faqs.sh`), codegen (`gen-frontend-entities.mjs`), verification (`mcp-smoke.sh`, `parity-replay.py`) and the CI guards.                          | Bash + Node + Python         |
| [`docs/`](docs/)                                                               | **docs**             | Product, technical and operations references (Portuguese).                                                                                                                                       | —                            |
| [`manuais/`](manuais/)                                                         | **end-user guides**  | How to reach the MCP server from ChatGPT, Claude, GitHub Copilot and Microsoft 365 Copilot. Served in-app by the **MCP** tab.                                                                    | —                            |
| [`start_server.sh`](start_server.sh)                                           | **runbook script**   | Build (incremental) + run the server + Cloudflare tunnel.                                                                                                                                        | Bash                         |

> **Catalog and data are separate, on purpose.** Entities/collections live once in
> [`config/registry.toml`](config/registry.toml); the scraper writes `data/<id>/raw/`, reference
> content lands in `data/<id>/ref/`, and `auli update` builds `data/<id>/packs/`. The frontend's
> `entities.ts` and `public/<id>/` are **generated** by `scripts/` (the prior hand-copying is gone).
> See [docs/auli_code.md](docs/auli_code.md) §2.
>
> **The repo holds code + config, not collected data.** Only `config/` is versioned.
> Everything under `data/<id>/**` (ref, raw, packs, scraper cache)
> is **gitignored**: it lives on the collection machine and is rebuilt by the pipeline
> (scraper → `auli-collections` → `auli update`). A fresh clone therefore has **no** state data —
> run the pipeline to populate it. Steps per content type, including the pareceres/consultas flow
> (scrape → sinopse → vectorize), are in [docs/auli_operations.md](docs/auli_operations.md) §4.

---

## Adapting Auli to your organization

**Start at [`config/registry.toml`](config/registry.toml).** That file is the catalog — the point
where the code's vocabulary meets the inventory in `data/`. It lives in `config/`, not `data/`,
precisely so you can find it.

**Adding an entity** (another revenue office, another agency) is three steps and **no code**:

1. one `[[entities]]` block in [`config/registry.toml`](config/registry.toml);
2. `config/prompts/<id>.txt` — the LLM instructions for that entity;
3. `data/<id>/` with the content. The scrapers live in
   [`auli-server/crates/scrapers/`](auli-server/crates/scrapers/); the guide for writing a new one
   is [SCRAPERS.md](auli-server/crates/scrapers/SCRAPERS.md).

Then run `node scripts/tools/gen-frontend-entities.mjs` — the frontend reads a generated mirror of the
registry, so this is what makes the new entity appear in the UI. No recompilation.

**Adding a collection** (another _kind_ of document, beyond services / FAQs / rulings / tribunal
decisions) **still requires code**: a variant in the `Kind` enum of `auli-contract` and a
projection into `Documento`. It is small — the TARF collection went in exactly that way — but it is
not configuration, and pretending otherwise would waste your time. The design — one struct, one
enum — is in [docs/auli_code.md](docs/auli_code.md) §3.13.

**What you will need besides the repo:** an LLM endpoint (any OpenAI-compatible one — see
[`.env.example`](.env.example)) and ~2 GB of disk for the embedding model, which is downloaded once
and runs locally. Questions are embedded in-process and never leave the machine; only the assembled
prompt reaches the LLM.

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
| [`crates/auli-cli`](auli-server/crates/auli-cli/)                 | The `auli` binary — `server` (Axum, RAG, `/v1/retrieve`, MCP via `rmcp`, config), `update` (vectorizer) and `bundle` (deterministic download `.zip` per state). Dispatch via `clap`.                                                                                                                   |
| [`crates/auli-collections`](auli-server/crates/auli-collections/) | Offline **derivation** (`<id> process`): snapshot → `auli-contract` tables (`<id>-<kind>.json`) + artifacts.                                                                                                                            |
| [`crates/scrapers/auli-scraper-<id>`](auli-server/crates/scrapers/) + [`auli-scraper-kit`](auli-server/crates/scrapers/auli-scraper-kit/) | The **scrapers** — **27 crates, one binary per state**, each writing a snapshot; `auli-scraper-kit` is their shared HTTP/cache / aggregation / snapshot I/O. Per-portal notes in [`SCRAPERS.md`](auli-server/crates/scrapers/SCRAPERS.md).                                                                                                                            |

Three subcommands:

```bash
auli update --entity <id> --out <packs-dir> [--version <v>]   # only writer; source = ../docs/
auli server [--packs-dir <dir>] [--port 3000]   # read-only; --packs-dir defaults to $AULI_DATA_DIR
auli bundle [--data-root data] [--out auli-frontend/public/downloads]   # download .zip per state
```

`auli update` reads the `.md` tree in `data/<id>/docs/<kind>/`, embeds each record's
`text_to_embed` and stores its `stored_repr`.

`auli bundle` zips each state's `docs/` tree **deterministically** (fixed entry order, fixed
permissions, one content-derived timestamp), so an unchanged corpus produces a byte-identical
file. It writes a `downloads.json` manifest with a size, a `sha256`, per-kind document counts and
an `atualizado_em` that only moves when the content does.

`auli server` is read-only by construction: it eager-loads collections via `ReadStore`, **validates
the pack manifest** against the local embedding identity at boot (and refuses to start on mismatch),
and only ever embeds the incoming question.

### `auli-frontend/` — web UI

A single-page app (no router; **tab navigation**) built with React 19, Vite, and Chakra UI v3.

- **State selection** with an interactive map of Brazil; choice persisted in `localStorage`.
- **Chat** against `POST /v1/question` (25 s timeout, friendly errors, copy button, markdown), with
  a **query-type selector**: serviços+FAQs, pareceres or TARF acórdãos.
- **Reference tabs** — Serviços, FAQs, Pareceres, Notas, Conteúdos — each reading static files from
  `public/<id>/`; "coming soon" placeholders for collections a state doesn't have yet.
- **Downloads** — every state's corpus as a `.zip`, with document counts, size and last-changed
  date from the `bundle` manifest.
- **MCP** — the connector guides from [`manuais/`](manuais/), rendered inline.
- Light/dark mode, mobile-first, virtual-keyboard aware. Tested with Vitest.

The last three tabs are **global** (not scoped to the selected state), so they stay available even
for a state whose data isn't collected yet.

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

**27 scrapers are active**, one crate per state, and no two portals are alike: Next.js and Drupal,
SharePoint and ColdFusion, WordPress and Angular, public REST APIs and hand-rolled HTML. Some need
a JA3-evading `curl` subprocess, one embeds its whole catalog in a lazy-loaded JS chunk, another
serves an incomplete TLS chain that has to be pinned. Each crate documents its portal's shape in
[`SCRAPERS.md`](auli-server/crates/scrapers/SCRAPERS.md).

- **RS** (`auli-scraper-rs`) — FAQs (portal CMS via AJAX/`ureq`) + serviços (**headless Chrome** for
  the listing, `ureq` for details) + **TARF**: 22.476 acórdãos from the state tax court, the single
  largest collection in the project.
- **SP / PR / SC** — plus **pareceres**: 15.605, 2.060 and 1.743 opinions respectively.
- On-disk **cache** with an offline `--usecache` mode; **dedup** of services shared across audiences.

The cache is **cache-first always** — `--usecache` only changes what a miss does (bail vs. network).
A genuine re-scrape therefore means deleting that collection's cache first, which is what
`scripts/tools/update-rs-faqs.sh` does before running the six-step scrape → derive → vectorize chain.

```bash
cd auli-server
cargo run -p auli-scraper-rs -- [--usecache] servicos   # scrape RS -> snapshot (faqs|servicos|all)
cargo run -p auli-collections -- rs process             # derive artifacts from the snapshot (offline)
```

---

## Quick start (backend, the live path)

Full runbook (cmake notes, Cloudflare Tunnel, logs, troubleshooting): **[docs/auli_operations.md](docs/auli_operations.md)**.

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
changes) — `build-packs.sh` runs `auli update` over the `.md` trees in `data/<id>/docs/` into
`data/<id>/packs/`, and derives the case-law indexes the UI reads — pareceres and TARF (`notas` has
no struct source yet and is skipped):

```bash
scripts/build-packs.sh rs          # per entity, any of the 27 ids in config/registry.toml
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

| Type          | What it is                                     | Where it appears today                          | Volume |
| ------------- | ---------------------------------------------- | ----------------------------------------------- | ------ |
| **TARF**      | Rulings of the state tax court (2nd instance)  | Chat (dedicated query type) + tab + **MCP**     | 22.476 (RS) |
| **Pareceres** | Legal/technical opinions                       | Chat (dedicated query type) + tab + **MCP**     | 19.780 (SP, PR, SC, RS) |
| **Serviços**  | The secretariat's service catalog, by audience | Chat (RAG) + Serviços tab                       | 4.205 across 27 states |
| **FAQs**      | Official frequently-asked questions            | Chat (RAG) + FAQs tab                           | 1.947 (RS) |
| **Notas**     | Administrative/tax notes                       | Notas tab (reference)                           | RS |
| **Conteúdos** | Misc reference materials                       | Conteúdos tab (reference)                       | RS |

Rows are ordered by volume, and the order is the point: the TARF alone outweighs every parecer
combined.

**Serviços and FAQs** feed the default chat mode. **Pareceres and TARF acórdãos** each have their
own query type and are the corpora exposed over MCP; pareceres additionally get **graph expansion
by co-citation** — opinions citing the same legal provisions as the retrieved ones are surfaced as
related. **Notas and Conteúdos** remain reference-only navigation.

---

## Status

- **Working today:** RAG chat for the configured state, the full UI (chat, reference tabs,
  downloads, connector guides, state selection with map), local embeddings, an MCP server for
  external assistants. **All 27 registered states have serviços collected** (4.205), pareceres for
  four (19.780), and — for RS — FAQs (1.947) and the TARF acórdãos (22.476, collection closed
  2026-08-09). The backend is open (no auth) and database-free — it serves from packs alone.
- **In progress:** FAQs beyond RS, automated scraping of notas, and a controlled vocabulary for the
  legal-provision graph.

For the precise active-vs-modeled breakdown (routes, auth flows, cross-repo divergences), see
**[docs/auli_code.md](docs/auli_code.md)** §7.

---

## License

[MIT](LICENSE) — © 2026 Carlos Henrique Schellenberger and contributors.
