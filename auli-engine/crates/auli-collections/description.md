# auli-collections — project description

Orientation doc for future coding sessions. Working directory for all Auli "collections" work.

## Goal

Build **one reusable scraping program** that can harvest several *collection kinds* (servicos, faqs,
pareceres, notas) from a tax-authority portal. Each collection scraper emits a **standard structured
JSON** (e.g. `faqs.json`); the flat `portal-<kind>.txt` files that the downstream RAG system ingests are
derived from that. The original logic was copied in from `../auli-docs/auli-docs-faqs` and
`../auli-docs/auli-docs-servicos`; the job is to refactor it into clean per-collection modules sharing
the `domain::collections::Collection` model.

> Status: **both scrapers run and the crate builds clean** — fully **synchronous** (no `tokio`/`async`;
> `ureq` HTTP client). `faqs` (`src/faqs/`) is refactored into the clean per-collection shape and emits
> `faqs.json` + `portal-faqs.txt`. `servicos` (`src/servicos/`) is wired and partially modernized
> (entity-aware paths, on-disk caching, faqs-shaped output) but not yet fully refactored; it emits
> `servicos.json` + `portal-servicos.txt`. `domain`, `errors`, `faqs`, `servicos` are all wired into the
> crate (`domain` not yet consumed by a pipeline). See "Current state".
>
> CLI: `cargo run [--usecache] <entity> <collection>` — e.g. `cargo run rs faqs` / `cargo run rs servicos`.
> `<entity>` (empty/omitted → default entity `rs`) is resolved + validated against the entity
> registry; `<collection>` (omitted → `faqs`) selects the scraper. `--usecache` runs offline: pages
> are read only from the on-disk cache and a cache miss is an error (no network / no headless Chrome) —
> handy for rebuilding the JSON/portal outputs from a previous scrape.

## The bigger system (context)

This is one piece of "Auli", a multi-tenant RAG assistant for Brazilian state tax authorities (sibling
folder `../../auli-frontend`). Flow:

```text
scrape portal  →  portal-<kind>.txt  →  entities/<id>/  →  parse into blocks  →  embed (Ollama)
                  (this project's job)                      └──── domain layer ────┘     ↓
                                                                              upsert into ChromaDB
                                                                              collection "<id>-<kind>"
```

The **scraper produces the left side; the `domain/` model defines the contract** for what those files
must look like. Keep the scraper output matching the `Collection` registry exactly.

## The collection model (`domain/collections.rs`)

A `Collection` describes one content "kind" as data, so all kinds share a single processing path.

| kind        | file                   | delimiter     | embed      | n_results | scraper exists?                              |
|-------------|------------------------|---------------|------------|-----------|----------------------------------------------|
| `servicos`  | `portal-servicos.txt`  | `## pergunta` | `FullText` | 10        | wired; partially modernized (`src/servicos`) |
| `faqs`      | `portal-faqs.txt`      | `## pergunta` | `FullText` | 20        | yes (`src/faqs`)                             |
| `pareceres` | `portal-pareceres.txt` | `## pergunta` | `FullText` | 3         | **no** (data only)                           |
| `notas`     | `portal-notas.txt`     | `## pergunta` | `FullText` | 1         | **no** (data only)                           |

- `EmbedStrategy::FullText` — store and embed the same block. Used by all current kinds.
- `EmbedStrategy::Description` — store the full cleaned servico, but embed only a derived description
  (first 4 non-empty lines, last line truncated to 300 chars). **Currently unused** — servicos moved to
  `FullText` when its output switched to the `## pergunta` shape; kept for future kinds.
- `from_kind(&str)` resolves a kind name (Portuguese error on unknown).
- `parse_blocks` / `parse_blocks_from_text` split a file/string into blocks: all current kinds use
  delimiter `## pergunta`, so they skip `//` comment lines and start collecting at the first delimiter.
  (A `//` delimiter, where the `//` lines are content collected from the top, is still supported but
  unused.)
- `prepare_documents` returns `(stored_documents, texts_to_embed)`.

## Output file formats (what the scraper must produce)

Each lives in `data/<id>/`. Blocks are separated by the delimiter above.

**servicos** (`portal-servicos.txt`) — now uses the same `## pergunta` / `## resposta` block shape as
faqs (one numbered `// N.` block per service): a `tipo | classe` breadcrumb + the service title in the
`## pergunta` block, the description body + link in `## resposta`:

```text
// 1.
## pergunta
Cidadãos | Atendimento - Site, e-CAC, PPF e DT-e
Agendamento para Atendimento Presencial

## resposta
<description body …>
Link: https://www.fazenda.rs.gov.br/...
```

**faqs / pareceres / notas** — `## pergunta` / `## resposta` blocks (a `// N` comment line precedes each):

```text
// 1.
## pergunta
<breadcrumb / title / question>

## resposta
<answer text, links inline>
Link: https://...
```

`pareceres` packs structured metadata into the `## pergunta` block (`descricao:`, `assunto:`, `resumo:`
with keywords, `link:`); `notas` answers are long LLM instruction templates ("Texto de Referência" blocks).

## The faqs scraper (`src/faqs/`)

The first collection ported to the new program. Walks the SEFAZ-RS FAQ portal and writes **two** files:
the structured **`<collection>.json`** (currently `faqs.json` — same shape as the legacy
`faq_site_tree.json`, verified byte-for-byte round-trip) and the flattened **`portal-<collection>.txt`**
(`portal-faqs.txt`) RAG knowledge file. Filenames derive from the collection name (matching `domain::collections`).

- **Output model** (`faq.rs`): a recursive `FaqNode { title, url, page_type, origin, children, faq_items }`
  where `page_type ∈ {Menu, Faq, Geral}` and `FaqItem { pergunta, resposta }`. `origin`/`children`/`faq_items`
  are `skip_serializing_if` empty. The root node is what's serialized to `<collection>.json`.
- **`mod.rs`**: `FaqSource` config (`base_url`, `root_url`, `root_title`, `collection`, `data_dir`,
  `cache_dir`); output paths via `FaqSource::output_path()` (`<data_dir>/<collection>.json`) and
  `portal_path()` (`<data_dir>/portal-<collection>.txt`). `scrape() -> FaqNode` and `run()` (scrape +
  write both files). Page classification reads
  `data-matriz-source-uri`: `…sanfona.comnavegacao` → `Faq`, `…categoriafaq` → `Menu`, else `Geral`.
  Menus are walked recursively; FAQ pages collect Q/A panels. Uses the crate's unified
  `crate::errors::{Result, Error}` (string errors become `Error::Custom` via `From<String>`).
- **`fetch.rs`**: synchronous `ureq` agent + on-disk cache under `cache_dir`. Pages are fetched once;
  the real content comes from the portal's AJAX list endpoint (`<base><source_uri>&…&pageSize=100`,
  JSON with markup in a `body` field). Network ops retry up to 3× with exponential backoff (800ms base)
  to ride out transient failures (e.g. connection resets).
- **`html.rs`**: hand-rolled HTML→text helpers (no DOM crate) — `format_html`, `remove_html_tags`,
  entity decode, breadcrumb/og:title extraction, panel title/body, `[text](url)` link conversion.
- **`portal.rs`**: flattens the `FaqNode` tree into `portal-faqs.txt` (one numbered `## pergunta` /
  `## resposta` block per question). Ported from the legacy `gerar_arquivo_portal_faqs_txt`; verified
  format-identical to the existing file (content differs only by newer scraped questions).
- **`legacy/`**: the original copied faqs files (`main.rs`, `extract_page_urls.rs`, `html_utils.rs`,
  `types.rs`, `utils.rs`, `diversos/`), kept for reference. Not declared as modules → not compiled.
  Supersede/remove once `servicos` is ported the same way. (Originals also live in `../auli-docs`.)

Run it with `cargo run` — **note: ~313 live requests to the SEFAZ-RS portal** (each fetch retries 3×
with backoff), caching pages under `data/rs/cache/faqs/` and writing `data/rs/faqs.json` +
`data/rs/portal-faqs.txt`.

## The servicos scraper (`src/servicos/`)

The raw copied servicos scraper. Partially modernized (entity-aware paths, caching, faqs-shaped output,
error handling, **now fully synchronous**) but **not yet refactored** into the clean per-collection
shape — it still uses `Box<dyn std::error::Error>` (not `crate::errors`), headless Chrome, and hardcoded
tipo URLs. Run with `cargo run rs servicos`. The whole crate is sync (the `ureq` HTTP client +
`std::fs`/`std::thread::sleep`, synchronous headless Chrome); there is no `tokio`/`async` anywhere —
swapping `reqwest`→`ureq` removed tokio (and the hyper/tower/h2 stack) from the dependency tree entirely.

- **`mod.rs`**: `run(data_dir, use_cache)` runs three stages — `extrair_descricoes_json` (scrape), then
  `gerar_portal_services_txt` (build the portal txt), then `write_servicos_json` (aggregate the per-tipo
  files into `servicos.json`). Finishes by calling `report_failed_detail_urls`, which prints any
  detail-page URLs that failed to load (or a success line).
- **`extrair_descricoes.rs`**: for each of the 5 audience pages (tipos), renders the JS listing via
  headless Chrome, **waiting for the `.card` element** — if cards never appear within 15s the page did
  not load and the whole program **aborts with an error** (rather than caching/parsing an empty page).
  It then fetches each service's detail page via `ureq` and extracts the description. Detail-page
  load failures are **collected and reported at the end** (not fatal — one bad link doesn't abort the
  run). Per-tipo `servicos-<tipo>.json` is written incrementally.
- **`cache.rs`**: on-disk page cache under `<data_dir>/cache/servicos/`, keyed by sanitized URL —
  mirrors the faqs cache. Covers both the headless-Chrome listing renders and the ureq detail pages;
  only successful fetches are cached.
- **`gerar_portal_servicos.rs`**: reads the per-tipo JSON and writes `portal-servicos.txt` in the
  `## pergunta` / `## resposta` block shape (one `// N.` block per service, matching faqs). `descricao_body`
  strips the `tipo / classe / titulo` header that `build_descricao` prepends, so it isn't duplicated.
- **`types.rs`** / **`utils.rs`**: `Servico` / `TipoServicos` models; `get_tipo_servicos()` lists the 5
  SEFAZ-RS audience URLs (the per-entity part still hardcoded — the main thing to move into config when
  porting).

## The `sc` entity (SEF-SC) — servicos DONE, faqs not yet built

A second entity, `sc` = **SEF-SC**. Config lives in `src/entities/sc/` (`entity.json`
`{ "id": "sc", "name": "SEF-SC" }` + `prompt.txt`, copied from `rs`); outputs go to `data/sc/`. The
registry auto-discovers it, so `get_entity("sc")` resolves and `cargo run sc faqs|servicos` is
recognized. **`cargo run sc servicos` is implemented and working** (see below); `sc faqs` is still a
placeholder arm in `main.rs::faq_source_for` (real URLs, but the faqs walk/parse for SC is not written
— the existing `src/faqs` is RS-specific).

**SC servicos — implemented (`src/servicos/sc.rs`, 2026-06-02).** `cargo run sc servicos` scrapes SC's
Next.js JSON API and writes, under `data/sc/`: 5 per-público files (`sc-servicos-ao-cidadao`,
`sc-servicos-a-empresas`, `sc-servicos-a-servidores-publicos`, `sc-servicos-a-estudantes`,
`sc-servicos-a-prefeituras`), the contract `sc-servicos.json`, the `servicos-index.json` tab manifest, and
`portal-servicos.txt`. Verified run: 213 listing services → 208 unique in the txt/json (5 have no
público so land in no audience file). Same `Servico` shape + `## pergunta`/`## resposta` block format
as RS, so the frontend UI and the RAG ingest are drop-in. Notes:

- `src/servicos/mod.rs::run(entity_id, data_dir, use_cache)` dispatches `rs` (headless Chrome) vs
  `sc` (JSON). Both backends return a `Vec<TipoServicos>`; the shared `finish()` then writes the
  portal txt + `servicos.json` + `servicos-index.json` from that list.
- **Dedup is by `link`, not `id`** (RS ids restart at 1 per file → not unique; links are unique in
  both). `gerar_portal_servicos` and `write_servicos_json` both dedup by link, so SC's multi-público
  services appear once in the txt/json while still showing under each audience tab.
- SC text fields use wiki-style `[[https://url anchor]]` links; `sc.rs::normalize_links` rewrites
  them to RS's `anchor "url"` form so the RAG text is consistent.
- Cache (`super::cache`) is keyed by a **logical URL without the buildId**, so a SC deploy that
  changes the buildId doesn't bust the cache. `--usecache` runs SC fully offline.

**Key fact: SC is a DIFFERENT platform from RS — and much easier.** Where RS is a CMS scraped via
`data-matriz-source-uri` markers + headless Chrome + HTML→text, **SC (`www.sef.sc.gov.br`) is a Next.js
app backed by a clean JSON API.** No browser, no HTML parsing needed. Plan (investigated 2026-06-02):

- **buildId**: every page embeds `<script id="__NEXT_DATA__">…"buildId":"<id>"…`. Data endpoints are
  `/_next/data/<buildId>/<path>.json`. The buildId changes on each site deploy, so the scraper must
  read it once from any page's `__NEXT_DATA__` at startup, then build the JSON URLs.
- **servicos** (`/servicos`, `/servicos/buscar`):
  - Listing: `/_next/data/<buildId>/servicos/buscar.json` → `pageProps.respostaApi.responseServicos`
    = `{ itens[10], itensTotais:"213", paginaAtual, paginasTotais:"22" }`. Paginated (HTML accepts
    `?page=N`; confirm the JSON param). Each item: `id, nome, finalidade, slug, grupoServico, publicos`.
  - Also returns `publicosDosServicos` (5 audiences: Cidadão/Empresa/Servidor Público/Estudante/
    Prefeitura) and `gruposDosServicos` (24 groups). Unlike RS, audience AND topic are structured fields.
  - Detail: `/_next/data/<buildId>/servicos/<slug>.json?slug=<slug>` →
    `respostaApi.servico.dadosJson`: `finalidade`, `etapasProcesso[]` (steps), `requisitosExigidosus[]`,
    `legislacaoAplicavel[]`, `termosRelacionados[]`, `urlSite`, `publicos[]`, `grupoServico`, `tema`,
    phone/email. Plain text — no HTML to strip.
- **faqs/perguntas** (`/perguntas`):
  - Root: `/_next/data/<buildId>/perguntas.json` → `respostaApi.assuntos[15]` top subjects, each
    `{ AssuntoID, Nome, Subassuntos[] }`.
  - Subject: `/_next/data/<buildId>/perguntas/<id>.json?id=<id>` → `respostaApi.perguntas[]`
    `{ BaseConhecimentoID, Questao, Resposta, Assunto{...} }` + nested `subassuntos[]` to recurse.
    (`/perguntas/5` CADASTRO alone = 38 questions.) `Resposta` mostly plain text (handle light HTML).
- **Reuse**: the SC build reuses the existing output models + writers — `FaqNode`/`FaqItem` →
  `faqs.json`, `portal.rs` → `portal-faqs.txt`, the `Servico` model + `gerar_portal_servicos` →
  `portal-servicos.txt`, the on-disk URL cache, and the entity registry. Only a new JSON fetch+map
  layer is needed (less work than RS — no HTML/Chrome). Proposed modules: `src/faqs/sc.rs` (assuntos
  tree → models) and `src/servicos/sc.rs` (paged list + detail → models), sharing one buildId+cache
  fetch helper; then replace the placeholder arms in `main.rs`.

### SC servicos: per-público JSON compatibility (verified 2026-06-02)

The goal is for SC to emit the **same per-público `Servico` JSON structure as RS** (one file per
audience, e.g. `sc-servicos-a-empresas.json`) so the **same `gerar_portal_servicos` txt routine** and the
**same frontend UI** (`auli-frontend/src/pages/servicoslist/`) work unchanged. Verified the two
consumers' field requirements:

- **portal-txt** (`gerar_portal_servicos.rs`) reads `tipo` + `classe` → `## pergunta` breadcrumb,
  `titulo` → title line, `descricao` → `## resposta` body (it `skip(3)`s the first 3 lines, expecting
  `descricao` to begin with a `tipo / classe / titulo` header — see `build_descricao`), `link` → `Link:`.
- **frontend** (`servicoslist/utils.ts` + `ServicosList.tsx`) needs only `id`, `classe` (groups
  services into accordions), `titulo` (link text), `link`. Loads one file per tab via
  `/<filename>.json`; the 5 tabs come from a hardcoded `getTipoServicos()`.

So the union of required fields = exactly the RS `Servico` struct (`id, tipo, classe, orgao, link,
titulo, descricao`). **SC maps cleanly** from the JSON API: `id`←`servico.id`; `tipo`←the público being
scraped; `classe`←`dadosJson.grupoServico.nome` (what the UI groups by); `orgao`←`dadosJson.orgao.nome`
(≈ constant "Secretaria de Estado da Fazenda"); `link`←`https://www.sef.sc.gov.br/servicos/<slug>` (or
`dadosJson.urlSite`); `titulo`←`nome`; `descricao`← **must be built** as
`"{tipo}\n{classe}\n{titulo}\n{finalidade + etapasProcesso…}"` to satisfy the `skip(3)` header convention.

**Two deliberate changes were required — both DONE (2026-06-02):**

1. **Multi-público dedup for the txt.** RS: each service has exactly ONE tipo → one file. SC:
   `dadosJson.publicos` is a LIST (e.g. "Acompanhar processo" is both Cidadão + Empresa), so a service
   lands in multiple per-público files — fine for the frontend (browse by audience), but the txt/json
   aggregation would otherwise emit duplicate blocks → duplicate vectors. **Done:** `gerar_portal_servicos`
   and `write_servicos_json` dedup by **`link`** (NOT `id` as originally guessed — RS ids restart at 1
   per file, so they're not unique; links are unique in both entities).
2. **Frontend tabs are hardcoded to RS.** Was: `getTipoServicos()` hardcoded RS's 5 tipos/filenames.
   **Done:** the scraper now emits `data/<id>/servicos-index.json` (`{ tipo, filename }[]`), and
   `ServicosList.tsx` loads `/servicos-index.json` via SWR to drive the tabs, falling back to
   `getDefaultTipoServicos()` (the old RS list) when the manifest is absent. So RS keeps working
   untouched and SC shows its own audience tabs. (The frontend is still single-tenant — it serves one
   entity's JSON from `public/`; switching which entity it shows is a deploy/asset concern, not yet a
   runtime selector.)

## Multi-tenant entities (`domain/entities.rs`)

- **Config vs data are split.** The registry scans `./src/entities/*/` (`ENTITIES_DIR`) at startup;
  each entity dir holds only `entity.json` (`{ "id", "name" }`) + `prompt.txt`. The collection output
  files live separately under `./data/<id>/` (`DATA_DIR = ./data`).
- `EntityConfig`: `id`, `name`, `system_prompt` (from `prompt.txt`, else `DEFAULT_SYSTEM_PROMPT`),
  `data_dir` (= `./data/<id>`). Helpers: `.collection(kind)` → `"rs-faqs"`;
  `.data_file(base)` → `"./data/rs/portal-faqs.txt"`.
- `DEFAULT_ENTITY = "rs"`; `get_entity(Option<&str>)` (empty → default, unknown → Portuguese error).
- Only entity present: **`rs` = SEFAZ-RS** (config in `src/entities/rs/`, data in `data/rs/`).

## Errors (`errors.rs`)

Wired into the crate (`mod errors;`) and used by `main` and the faqs scraper. `thiserror` enum `Error`
with `Result<T>` alias; `#[from]` wraps `anyhow`, `std::io`, `serde_json`, `ureq`; plus `Custom(String)`
with `From<String>`/`From<&str>` (so `format!(...)?` works). `Display` is user-facing (Portuguese),
`Debug` for logs. Needs `thiserror` + `anyhow` deps (both present). No Ollama variant in this crate.

## Directory layout

```text
auli-collections/
├── Cargo.toml              # ureq (sync HTTP), scraper, headless_chrome, serde + thiserror/anyhow
├── description.md          # this file
├── src/
│   ├── main.rs             # CLI dispatch: `cargo run [--usecache] <entity> <collection>`
│   ├── errors.rs           # unified Error/Result (wired; used by faqs)
│   ├── domain/             # core types & registries
│   │   ├── mod.rs
│   │   ├── collections.rs  # the collection-kind registry (the "types")
│   │   └── entities.rs     # entity registry (ENTITIES_DIR=./src/entities, DATA_DIR=./data)
│   ├── entities/rs/        # entity CONFIG only: entity.json + prompt.txt
│   ├── faqs/               # ACTIVE faqs scraper
│   │   ├── mod.rs          # FaqSource, walk, scrape(), run()
│   │   ├── faq.rs          # FaqNode/FaqItem/PageType output model → faqs.json
│   │   ├── fetch.rs        # ureq agent + on-disk cache + AJAX body fetch (3× retry)
│   │   ├── html.rs         # hand-rolled HTML→text helpers
│   │   ├── portal.rs       # FaqNode tree → portal-faqs.txt RAG text
│   │   └── legacy/         # original copied faqs files (not compiled; reference only)
│   └── servicos/           # servicos scraper (wired, sync; partially modernized, not fully refactored)
│       ├── mod.rs          # run(data_dir, use_cache): scrape + aggregate + report failed detail URLs
│       ├── cache.rs        # on-disk page cache (listing + detail pages), mirrors faqs cache
│       ├── extrair_descricoes.rs  # headless-Chrome listing render + ureq detail fetch
│       ├── gerar_portal_servicos.rs
│       ├── types.rs
│       └── utils.rs
└── data/
    └── rs/                 # entity OUTPUTS: faqs.json, servicos.json, portal-{servicos,faqs,pareceres,notas}.txt
                            #   per-tipo servicos intermediates: servicos-*.json (5 audience files)
                            #   cache/faqs/  + cache/servicos/  (fetched pages, created on run)
```

## Current state / things to know before coding

- **Builds & runs**: `cargo build` is clean. `src/main.rs` takes `cargo run <entity> <collection>`: it
  resolves+validates `<entity>` via `domain::entities::get_entity` (empty/omitted → default `rs`), then
  dispatches `<collection>` — `faqs` (default) runs the faqs scrape, `servicos` runs the servicos
  scraper. The whole crate is **synchronous** (no `tokio`/`async`; both scrapers use the sync `ureq`
  HTTP client, servicos drives synchronous headless Chrome). Both scrapers' output/cache paths derive from the entity's
  `data_dir` (`servicos::run(data_dir, use_cache)` / `faqs::FaqSource`); the portal/tipo URLs are still
  per-entity constants guarded to `rs` in `main.rs`. All four modules (`domain`, `errors`, `faqs`,
  `servicos`) are wired in; faqs uses `crate::errors::{Result, Error}`, servicos still uses `Box<dyn Error>`.
- **Page caching**: both scrapers cache fetched pages on disk so re-runs don't re-hit the portal — faqs
  under `cache/faqs/` (`src/faqs/fetch.rs`), servicos under `cache/servicos/` (`src/servicos/cache.rs`,
  covering both the headless-Chrome listing renders and the ureq detail pages). Only successful
  fetches are cached; delete the cache dir to force a fresh scrape.
- **`--usecache` (offline mode)**: threads a `use_cache` flag from `main.rs` down through the fetch
  layer (`FaqSource.use_cache` → `faqs::fetch`; `servicos::run(data_dir, use_cache)` → `extrair_descricoes`).
  When set, a cache miss returns an error instead of fetching, so nothing touches the network and
  servicos never launches headless Chrome. Verified: `cargo run rs faqs --usecache` and
  `cargo run rs servicos --usecache` both rebuild their outputs entirely from cache.
- **All modules wired**: `domain`, `errors`, `faqs`, and `servicos` are declared from `main.rs` and
  compile clean. `domain` isn't consumed by the pipeline yet, so `src/domain/mod.rs` carries
  `#![allow(dead_code)]` to silence not-yet-used warnings; drop it once the pipeline calls into `domain`.
- **Data layout** (consolidated; old `data/faqs`, `data/servicos`, and root-level JSON duplicates
  removed): per-entity outputs live in `data/<id>/`. For `rs`: the four `portal-*.txt` (read by `domain`
  via `data_file`), the structured `faqs.json` / `servicos.json`, and per-tipo `servicos-*.json`.
- **No scraper yet** for `pareceres` or `notas` — only their `portal-*.txt` data exists. The unified
  scraper will need to learn these sources.
- **Naming collisions** are why the scrapers are namespaced under `src/faqs/` and `src/servicos/`
  (both originally defined `main.rs`, `types.rs`, `utils.rs`). `faqs` is now refactored; `servicos` still
  raw.
- **Prompt markers**: `prompt.txt` / `DEFAULT_SYSTEM_PROMPT` describe the block markers. All kinds
  (servicos included) use `## pergunta` — the `// N.` line is just a numbered comment — so both now
  carry a single line: "Cada serviço e cada pergunta do texto inicia com o marcador: ## pergunta".
