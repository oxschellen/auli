# auli

Single Cargo workspace for the Auli RAG assistant. One binary, two modes; three layered crates.

> **Operations runbook** (compile, build packs, start with ngrok, log locations):
> [`../auli_operations.md`](../auli_operations.md).

```
auli/
├── crates/
│   ├── vector-store/   # bottom — agnostic flat cosine store (Record<P>, ReadStore/Writer split)
│   ├── auli-core/      # middle — embed (BGE-M3), corpus (EmbedStrategy), manifest (identity)
│   └── auli-cli/       # top    — the `auli` binary: `server` (read-only RAG) + `update` (writer)
```

Layering is strict and downward-only: `vector-store` ← `auli-core` ← `auli-cli`. A shared
`Cargo.lock` guarantees the `update` and `server` modes embed with the **same** fastembed/model,
so the document vectors and the live query vector share one cosine space by construction.

## The two modes

```bash
# Build vectors ("packs") from an entity's portal-*.txt sources (the only writer):
auli update --entity rs --source ./sources/rs --out ./packs [--version 1]

# Serve the API read-only from pre-built packs (validates the manifest at boot):
auli server --port 3000 --packs-dir ./packs
```

`update` writes `<entity>-<kind>.json` + `<entity>.manifest.json`. `server` eager-loads those into
immutable `ReadStore`s, validates each manifest against its own embedding identity (model + dim +
strategy version), and **refuses to start on a mismatch**. At query time the server embeds only the
user's question — it never writes, and links no `Writer`.

## Distribution

A new machine needs only the binary + the packs folder. No DB to stand up for serving, no ChromaDB
or Ollama, no embedding service, no network for ingestion. Copy two artifacts and run.

## Build / test

```bash
cargo build --workspace
cargo test  --workspace

# end-to-end serving-path test against real generated packs (otherwise ignored):
# Run from auli-server/: the model cache lives at the repo root (../models).
AULI_PACKS_DIR=./packs EMBED_CACHE_DIR=../models \
  cargo test -p auli-cli --release --test packs_smoke -- --ignored --nocapture
```

> Build needs `cmake` + a C compiler (for `aws-lc-sys`) and network on first build (`ort` downloads
> the ONNX Runtime; BGE-M3 downloads from Hugging Face into `EMBED_CACHE_DIR` on first `update`/serve).
> The launchers point `EMBED_CACHE_DIR` at `<repo-root>/models` (absolute); the code default `./models`
> only applies to manual runs. Builds on Linux; see `auli-cli` for the env caveats.

## Environment (server mode)

`server` reads a `.env` (LLM + embedding settings only — no auth, no database) via `auli-cli`'s
`config`. `update` needs only `EMBED_CACHE_DIR` / `EMBED_THREADS`. See `crates/auli-cli/src/config.rs`.
