# Codebase Inconsistency Audit — auli-main

**Date:** 2026-07-02
**Scope:** Full read of `auli-frontend` (React/TypeScript) and `auli-server` (Rust workspace, 10 crates), plus the static data contract under `public/` and the `/v1/question` chat API seam. Build artifacts (`auli-server/target/`) excluded.

**Method:** Three parallel deep-read passes — frontend, backend, and the cross-boundary contract — with overlapping findings cross-confirmed. Nothing in this document has been changed in the code; this is a report only.

---

## Summary

| # | Severity | Area | Inconsistency |
|---|----------|------|---------------|
| 1 | High | Frontend / Data | `pr` and `sp` entities declared but have no data folder — both states 404 |
| 2 | High | Pipeline ↔ Data | Scraper emits `<id>-`-prefixed slugs; deployed data + frontend expect unprefixed |
| 3 | High | Backend | One "Servico" concept modeled by three divergent structs |
| 4 | High | Frontend | `ServicosList` is the only section not gated on `hasCollection` |
| 5 | Medium | Contract | Dead response DTO `QuestionResponse` (never emitted) |
| 6 | Medium | Contract | 429 / all HTTP errors collapse into one generic "server unavailable" |
| 7 | Medium | Backend | `/v1/{kind}/list` route unused + `services` vs `servicos` vocabulary mismatch |
| 8 | Medium | Tests | `packs_smoke` test uses flat layout, not the server's nested layout |
| 9 | Medium | Backend | Manifest integrity hash computed but never verified at load |
| 10 | Medium | Backend | Divergent embed-key formulas (services vs faqs vs `stored_repr`) |
| 11 | Medium | Scraper | Two divergent page caches inside the RS scraper; duplicated `url_to_filename` |
| 12 | Medium | Docs / Types | Stale env vars, false doc comments, dropped JSON fields, hand-rolled scaffold |
| 13 | Low | Deps | `reqwest = "0.13.4"` — nonexistent version; other pins unusually high |
| 14 | Low | Backend | Two default system prompts disagree on the service marker |
| 15 | Low | Backend | Inconsistent error strategy (`thiserror` vs `Box<dyn Error>`/`anyhow`) |
| 16 | Low | Frontend | Mixed styling tokens, generic `DESIGN.md`, dead `servicos.json`, `page_type` casing |

---

## Resolution status (2026-07-02)

Items **1–4** have been fixed. `cargo build` + `cargo test` pass for the touched crates
(`auli-contract`, `auli-collections`, `auli-scraper-{rs,sc,sp,pr}`, `auli-scraper-kit`): 14 tests pass,
1 golden test ignored by design (`AULI_GOLDEN_DATA` gate). The frontend edits mirror the existing
`FaqsList` pattern; they were not test-run here because `node` is unavailable in this environment.

| # | Status | Resolution |
|---|--------|-----------|
| 1 | ✅ Fixed | PR/SP kept as **"coming soon"** (decision): `collections = []` in `data/registry.toml` and the generated `src/shared/entities.ts` (hand-synced — `node` unavailable to run the generator). They stay selectable on the map but every tab now renders `CollectionEmpty` instead of a 404. |
| 2 | ✅ Fixed | Dropped the `<id>-` slug prefix so the pipeline output matches the deployed/unprefixed contract: the four scraper `publicos()`/`get_tipo_servicos()` lists, the frontend fallback `getDefaultTipoServicos()`, plus the `snapshot.rs` field-doc example and `description.md` filename examples. The contract file `<id>-servicos.json` keeps its prefix (correct — it's the table file, not a per-público UI file). |
| 3 | ⏸️ Left as-is (decision) | Documented, deliberate architectural boundary (`auli-scraper-kit/src/servico.rs:1-5` "NOTA de acoplamento … Sem ação agora"); the three structs serve distinct roles (raw snapshot → contract → per-público output). Not a functional bug; consolidation would be a risky cross-crate refactor with no behavior payoff. |
| 4 | ✅ Fixed | `ServicosList` now gates on `hasCollection(entity, "servicos")` like every sibling list: skips both SWR fetches when absent and returns `<CollectionEmpty label="Serviços" />`, restoring the documented no-404 invariant. |
| 5 | ✅ Fixed | Deleted the dead `QuestionResponse` struct from `auli-cli/src/api/dto.rs` (zero references; handler returns `Answer`). The frontend's own local `QuestionResponse` interface is a different, used type — left alone. |
| 6 | ✅ Fixed | `callServerAPI` now distinguishes HTTP 429: reads the server's `{ error }` body (falls back to a local pt-BR `rateLimited` string) instead of showing the generic "server unavailable." Added a test for the 429 path (was untested). |
| 12 | ◐ Partially fixed | Env/doc drift resolved: removed unused `EMBED_API_MODEL` from `.env.example`; fixed the `EMBED_THREADS` example (was `24` vs documented default `16` — now a commented `16`); corrected the false `exec_all_question`/`answer` docstrings in `auli-collections/src/errors.rs` and `auli-scraper-rs/src/errors.rs` (that path exists only in `auli-cli`, whose comment is accurate). The `ConteudoItem` `extra_links` gap and the `About.tsx` hand-rolled scaffold (also listed under #12) are behavior/refactor changes and remain open. |
| 7 | ✅ Fixed (unified to `servicos` everywhere) | Reframed first: the `/v1/{kind}/list` route is **not** dead — it's a mounted, documented read-only admin/list endpoint (`vector-store::list` exists to serve it). The real issue was the two-namespace split: the vector kind was `services` while the whole UI/registry/scraper side says `servicos`. Per your call, **renamed the internal vector kind `services` → `servicos` end-to-end** — one vocabulary now. Functional changes: `corpus.rs` `SERVICES.kind = "servicos"` and `update.rs` ingests as `"servicos"` (so packs are written `<id>-servicos.json`). Everything else cascades from `Collection.kind`: pack filename, collection-map key, `EntityConfig::collection`, `rag.rs` lookup, `packs::load_all`, and the manifest `kind`/`file`. Route param is now `servicos` (via `from_kind`), matching the registry/scraper/UI. Comments + the `from_kind` test updated (`"services"` kept only as a regression guard that the old spelling errors); `auli_code.md` route table updated. Left incidental non-kind uses alone (the `.services` CSS selector and `let services: Vec<Servico>` locals in the scrapers). **No data migration in-repo** (packs are generated, none committed) — but any packs generated *before* this change need a one-time `auli update` re-run, since the server now loads `<id>-servicos.json`. |
| 8 | ✅ Fixed | `packs_smoke.rs` now resolves paths exactly like the server's `packs::load_all` — the nested `<AULI_PACKS_DIR>/<id>/packs/` layout (`<id>-<kind>.json` + `<id>.manifest.json`) — instead of the flat layout, so it truly exercises the server's read path as its docstring claims. |
| 9 | ✅ Fixed (as a warning) | `packs::load_all` now captures the validated manifest and re-hashes each collection file against the manifest's `hash` entry (previously computed by `auli update` but never checked). A mismatch logs a loud non-fatal warning — the identity triple stays the hard boot gate; this makes the integrity hash actually catch a corrupted/half-copied pack. Chose warn-not-fail to avoid bricking boot on benign manifest staleness; hard-fail is a one-line change if preferred. |
| 13 | ❌ Not a bug (false positive) | Verified against `Cargo.lock`: `reqwest 0.13.4` **does** resolve (a transitive `0.12.28` also present), and `axum 0.8.9` / `tokio 1.52.3` / `toml 1.1.2` all resolve — `cargo build` passes. The audit's "0.13.4 doesn't exist" came from stale model knowledge. No change needed. |
| 15 | ✅ Fixed (scoped) | `auli-collections::servicos::process` (and its `write_servicos_index` helper) now return the crate's `errors::Result` like their sibling `derive_faqs::process`, so the stringly-typed `.map_err(|e| e.to_string())` bridge in `process.rs` is gone — the two halves of the module now share one error type. Left the `sc`/`sp`/`pr` **binaries** on `anyhow` (idiomatic for bins; not an inconsistency worth churning). |
| 14 | ✅ Fixed (defaults) | The two `DEFAULT_SYSTEM_PROMPT` constants now carry identical, accurate marker lines: `## servico` for services (matches `rag.rs:122`) + a `## pergunta` line for the inner block. Fixed `auli-cli/src/entities.rs` (its 2nd line was a copy-paste typo repeating "Cada serviço … ## pergunta") and added the missing `## servico` line to `auli-collections/src/domain/entities.rs`. Zero runtime impact — the defaults only fire when an entity has no prompt file, and all four ship one. **Note (broader, left open):** the real `data/prompts/*.txt` files *also* disagree (`rs.txt` has the servico+typo pair, `sc.txt` only the `## pergunta` line, `pr.txt`/`sp.txt` say `## pergunta` with no `## servico`). Those are the prompts that actually run; fixing them changes live LLM behavior and is a separate decision. |
| 16 | ◐ Partially fixed | Safe tidy-ups done: redirected the two dangling `COLOR_MODE_PLAN.md` references (`theme/system.js`, `eslint.config.js`) to the real `THEME.md`; normalized the two Chakra `<Button color=…>` props (`UserMessage`, `SystemMessage`) from `var(--chakra-colors-fg-muted)` to the `fg.muted` token; fixed the stale `useIsMobileKeyboardVisible.jsx` header comment; aligned the `parseFaqs.test.ts` `page_type` fixture + assertion to production casing (`"FAQ"` → `"Faq"`). **Left intentionally:** the `<MdCopyAll>`/`<MdInbox>` icon `color` props stay as CSS vars (react-icons can't take Chakra tokens — those were correct); `DESIGN.md` (generic template — a doc rewrite, not a surgical fix); the dead `public/{rs,sc}/servicos.json` (deleting committed data that `build-frontend-public.sh` would regenerate — needs a build-script decision); and `UserMessage`'s unused `showButton` (threading an always-true no-op prop is churn). |

**Follow-up to regenerate data:** items 1–4 fix the *code*; the committed `public/rs` and `public/sc`
data already match the corrected (unprefixed) contract, so no data regeneration is required now. If the
scraper→collections pipeline is re-run, it will now emit unprefixed filenames that the frontend loads
correctly. When `node` is available, run `node scripts/gen-frontend-entities.mjs` to confirm the
hand-synced `entities.ts` matches the generator output byte-for-byte.

Fixed: 5, 6, 7, 8, 9, 14, 15. Partially fixed: 12, 16. Reclassified as non-bug: 13. Still open: 10, 11 and the remaining #12 sub-parts (`ConteudoItem.extra_links`, `About.tsx`), #16 sub-parts (`DESIGN.md`, dead `servicos.json`, `showButton`), and the broader #14 note (`data/prompts/*.txt` disagree).

---

## 🔴 High severity

### 1. `pr` and `sp` states are declared as entities but have no data — ✅ Fixed (kept as "coming soon")
*Confirmed independently by the frontend and contract-boundary passes.*

`src/shared/entities.ts:41-54` declares Paraná (`pr`, SEFA-PR) and São Paulo (`sp`, SEFAZ-SP), each with `collections: ["servicos"]`. The landing page renders a selectable card for every entity (`StateSelection.tsx:44-46`) and the map lights up every UF that has an entity (`BrazilMap.tsx:41`).

But only `public/rs/` and `public/sc/` exist on disk — there is no `public/pr/` or `public/sp/`. Data is resolved as `public/<id>/<file>` via `entityPath` (`src/shared/fetchers.ts:23-24`), so selecting PR or SP fetches `/pr/servicos-index.json` and `/pr/servicos-*.json`, all of which 404. Because `ServicosList` has no empty-state guard (see #4), the user gets an error banner rather than the friendly empty state.

Every doc contradicts the code: `README.md:7,11`, `DESCRIPTION.md:17,101-102,114`, and `BrazilMap.tsx:12` all say only RS and SC ship.

**Impact:** Two of four advertised states are non-functional.
**Files:** `src/shared/entities.ts:41-54`, `src/pages/stateselection/{StateSelection.tsx:44-46, BrazilMap.tsx:41}`, `src/shared/fetchers.ts:23-24`.

### 2. Slug-prefix mismatch between pipeline output and deployed data — ✅ Fixed
*Confirmed independently by all three passes — the single most-cited issue.*

The scrapers generate audience slugs **with** an `<id>-` prefix:
- `auli-server/crates/auli-scraper-rs/src/servicos/utils.rs:7-35` → `rs-servicos-ao-cidadao`, `rs-servicos-a-empresas`, …
- `auli-scraper-sc/src/sc.rs:47-55` → `sc-servicos-ao-cidadao`, …
- `auli-scraper-sp/src/scrape.rs:25-32` → `sp-servicos-ao-cidadao`, …
- `auli-scraper-pr/src/scrape.rs:26-36` → `pr-servicos-ao-cidadao`, …

These slugs flow into `Publico.slug` → `write_servicos_index` writes `filename: p.slug` and the per-audience output filename `{data_dir}/{slug}.json` (`auli-collections/src/servicos/mod.rs:92,121-143`).

But the checked-in golden data and the frontend expect **unprefixed** names:
- `public/rs/servicos-index.json` → `"filename": "servicos-ao-cidadao"` (no `rs-`).
- `public/sc/servicos-index.json` → `"filename": "servicos-ao-cidadao"` (no `sc-`).
- The frontend builds `entityPath(entity.id, `${activeTipo.filename}.json`)` = `public/rs/<filename>.json` (`ServicosList.tsx:39-43`) — the entity id is already in the path, so a prefixed `filename` resolves to `public/rs/rs-servicos-ao-cidadao.json`, which does not exist.

Additionally, the frontend's own fallback list `getDefaultTipoServicos()` (`src/pages/servicoslist/utils.ts:22-26`) uses the **prefixed** names — so even the two frontend sources (fallback vs live `servicos-index.json`) disagree with each other.

**Impact:** If `public/` is regenerated from the current pipeline, every audience tab's data file 404s. The fallback path is already dead-and-broken.
**Files:** the four scraper `utils.rs`/`sc.rs`/`scrape.rs` files above, `auli-collections/src/servicos/mod.rs:92,121-143`, `public/{rs,sc}/servicos-index.json`, `src/pages/servicoslist/{ServicosList.tsx:39-43, utils.ts:22-26}`.

### 3. One "Servico" concept, three divergent structs — ⏸️ Left as-is (deliberate boundary)
The same domain object exists in three incompatible shapes:

- `auli_contract::Servico` — `auli-contract/src/lib.rs:102-119`: `id, tipo, classe, orgao, link, titulo, descricao, text_to_embed`.
- `auli_scraper_kit::Servico` — `auli-scraper-kit/src/servico.rs:12-28`: same minus `text_to_embed`, and its `descricao` still carries the 3-line `tipo/classe/titulo` header.
- `auli_contract::ServicoRaw` — `auli-contract/src/snapshot.rs:113-126`: `titulo, descricao, link, orgao, ocorrencias: Vec<Ocorrencia>` — no `id/tipo/classe`; público×classe is a list.

The per-audience JSON output re-uses the kit `Servico` (`auli-collections/src/servicos/types.rs:3`, written at `servicos/mod.rs:81-93`), so on-disk files carry `descricao` + `tipo/classe` while the contract `Table<Servico>` has the derived `text_to_embed` the files lack. The `descricao` semantics differ (kit = "with header", contract = "body only") — a genuine trap. Meanwhile the frontend `Servico` interface (`src/pages/servicoslist/utils.ts:8-13`) only reads `id/classe/titulo/link` and silently drops `tipo/orgao/descricao`.

**Impact:** High drift surface; a silent contract mismatch already exists between what the backend writes and what the frontend reads.

### 4. `ServicosList` is the only section not gated on `hasCollection` — ✅ Fixed
Every other list gates on `hasCollection(entity, kind)`, passes `null` to `useSWR` to skip the fetch, and renders `<CollectionEmpty>` when the collection is absent:
- `FaqsList.tsx:15,35`, `ConteudosList.tsx:15,32`, `NotasList.tsx:12,19`, `PareceresList.tsx:12,19`.

`ServicosList.tsx` does none of this — no `hasCollection` import, no `CollectionEmpty`, always fetches. This directly contradicts the documented invariant in `DESCRIPTION.md:238` ("every section gates on `hasCollection(entity, kind)` … and never fires the fetch, so there is no 404"). It is also the mechanism that turns #1 into a loud error banner instead of a graceful empty state.

**Impact:** For shipping RS/SC it's harmless (both have `servicos`), but it breaks the documented no-404 invariant and makes broken states fail loudly.

---

## 🟡 Medium severity

### 5. Dead response DTO `QuestionResponse`
*Confirmed by all three passes.*

`auli-cli/src/api/dto.rs:23-29` defines `QuestionResponse { status, question, answer }`, but the handler actually returns `Answer { question, answer }` (`api/handlers/question.rs:38`; `Answer` at `dto.rs:32-37`). The frontend reads only `res.data?.answer` (`callServerAPI.ts:16-18,70`). `QuestionResponse` is never constructed anywhere; its `status` field implies an older wire shape that no longer exists.

### 6. 429 and all HTTP errors collapse into one generic message
The server returns `429 TOO_MANY_REQUESTS` with a friendly pt-BR body `{ "error": "Muitas requisições. Aguarde alguns instantes e tente novamente." }` (`auli-cli/src/api/ratelimit.rs:44-50`). The frontend catch block (`callServerAPI.ts:81-93`) only distinguishes `axios.isCancel(e)` (→ timeout copy) from everything else (→ `MESSAGES.unavailable` = "o Servidor Auli não está disponível."). It never reads `e.response.status` or `e.response.data.error`.

**Impact:** A rate-limited user is told the server is *down* rather than "you're sending too fast." The server's friendly text is discarded.

### 7. `/v1/{kind}/list` route unused + `services` vs `servicos` mismatch
`data_routes` mounts `GET /v1/{kind}/list` (`auli-cli/src/api/mod.rs:36-40`, handler `collections.rs:22-55`). `from_kind` accepts only `services|faqs|pareceres|notas` (`auli-core/src/corpus.rs:24-35`) — so `/v1/servicos/list` returns "unknown kind"; only `/v1/services/list` works. Every other layer (frontend tabs, scrapers, registry `collections=["servicos"]`) speaks `servicos`. The frontend never calls this route anyway (list data is served from static `public/` files), and its `EntityQuery` DTO (`dto.rs:16-20`) is orphaned with it.

### 8. `packs_smoke` test doesn't exercise the real pack layout
`auli-cli/tests/packs_smoke.rs:31,36` loads packs from a flat layout (`packs.join("rs-faqs.json")`), but the real server loader `packs::load_all` reads nested `<packs_root>/<id>/packs/<id>-<kind>.json` and `<id>/packs/<id>.manifest.json` (`auli-cli/src/packs.rs:35-53`). The test docstring claims it "exercises exactly the server's read path" — it does not, so a regression in nested-path resolution would pass the smoke test.

### 9. Manifest integrity hash computed but never verified
`CollectionEntry.hash` (FNV-1a of the pack file) is written by `update.rs:113` and documented as catching "a half-copied/corrupted pack" (`auli-core/src/manifest.rs:52-53`). But `validate_manifest` only compares the `EmbedIdentity` triple (`manifest.rs:112-123`) and `packs::load_all` (`packs.rs:37-54`) never re-hashes the files. The corruption detection the hash was added for does not actually run.

### 10. Divergent embed-key formulas
- Services: `"{tipo} | {classe}\n{titulo}\n{first 300 chars of body}"` (`auli-collections/src/servicos/mod.rs:113-119`).
- Services `stored_repr`: `"## pergunta\n{tipo} | {classe}\n{titulo}\n\n## resposta\n{descricao}\nLink: {link}"` (`auli-contract/src/lib.rs:126-133`).
- FAQs: `"{origin} {pergunta}"` — key only, no body (`derive_faqs.rs:30-43`).

The two content kinds embed inconsistently, and the services formula is self-labeled "Provisional" in the code (`servicos/mod.rs:114-115`). Any change here must bump `manifest::STRATEGY_VERSION` (`auli-core/src/manifest.rs:26`) — an easy-to-forget coupling.

### 11. Two divergent page caches inside the RS scraper
`url_to_filename` is defined identically in `auli-scraper-kit/src/cache.rs:50-57` and `auli-scraper-rs/src/faqs/mod.rs:251-258`. More importantly, the RS **faqs** fetcher implements its own disk cache (`auli-scraper-rs/src/faqs/fetch.rs:58-142`) instead of using `auli_scraper_kit::cache` (which the RS **servicos** path does use, `extrair_descricoes.rs:104,157,246`). The two caches have different semantics: the kit cache treats empty files as a miss (`cache.rs:22-25`); the faqs cache treats any existing file as a hit (`fetch.rs:64`).

### 12. Stale env vars, false doc comments, dropped fields, hand-rolled scaffold
- `EMBED_API_MODEL` is listed in `.env.example` but read by no crate (`config.rs` reads only `LLM_API_*`, `EMBED_CACHE_DIR`, `EMBED_THREADS`).
- `EMBED_THREADS` code default is `16` (`config.rs:39`, `update.rs:30`) but `.env.example` sets `24`.
- Identical stale docstrings referencing `exec_all_question` → `answer` appear in `auli-collections/src/errors.rs:7-8` and `auli-scraper-rs/src/errors.rs:7-8`; neither crate has that function or field (it lives only in `auli-cli`'s `rag.rs`).
- `ConteudoItem` (`src/pages/conteudoslist/parseConteudos.ts:2-6`) omits `extra_links` present in `public/rs/conteudo_site_tree.json:11-20`; the UI drops them. The top-level `source` field is also unmodeled.
- `About.tsx:14-30` hand-rolls the spinner+`Alert` scaffold that `AsyncContent` (`src/shared/AsyncContent.tsx:15-18`) exists to replace, unlike every other SWR page.

---

## 🟢 Low severity

### 13. Suspicious dependency versions
`auli-cli/Cargo.toml:36` pins `reqwest = "0.13.4"` — reqwest's published line is `0.12.x`; `0.13.4` does not exist on crates.io. Likely a typo. Other pins look unusually high (`axum = "0.8.9"`, `tokio = "1.52.3"`, `toml = "1.1.2"`) — worth verifying against `Cargo.lock`.

### 14. Two default system prompts disagree
The `auli-cli` fallback prompt (`config.rs:20-29`) mentions the marker `## servico`; the `auli-collections` fallback prompt (`domain/entities.rs:24-32`) says only `## pergunta`. The actual RAG context uses `## servico` for services (`rag.rs:122`), so the collections-side default mis-describes the format. Only relevant when an entity ships no prompt file.

### 15. Inconsistent error strategy across crates
Library/domain crates use `thiserror` enums (`auli-cli`, `auli-core`, `vector-store`, `auli-collections`, `auli-scraper-rs`), but the `sc`/`sp`/`pr` binaries and `auli-collections/src/servicos/mod.rs:13-17` use `Box<dyn std::error::Error>` / bare `anyhow`. The two service-derivation halves in the same module (`derive_faqs.rs` uses the crate `Result`, `servicos/mod.rs` uses `Box<dyn Error>`) force a stringly-typed bridge in `process.rs:43-44`.

### 16. Frontend styling / data odds and ends
- Mixed color tokens: `color="fg.muted"` (token) vs `color="var(--chakra-colors-fg-muted)"` (inline var) for the same role — e.g. `UserMessage.tsx:41,56`, `SystemMessage.tsx:48,56`, `CollectionEmpty.tsx:19`. `THEME.md:15-20` prescribes the token form.
- `DESIGN.md` is a generic Apple-marketing template ("museum gallery", "product tiles", "store utility links") unrelated to a tax-assistant chat app; only the accent color and SF Pro/17px body carried into `system.js`/`index.css`.
- `public/{rs,sc}/servicos.json` (aggregate, ~660 KB RS / ~284 KB SC) ships but is never fetched — only `servicos-index.json` and the per-audience files are.
- `parseFaqs.test.ts` fixtures use `page_type: "FAQ"` (all-caps) while real data uses `"Faq"`; the test is self-consistent but wouldn't catch a casing bug.
- Stale header comment `// useIsMobileKeyboardVisible.jsx` in `src/pages/chat/utils/useIsKeyboardVisible.ts:1`; dangling references to a non-existent `COLOR_MODE_PLAN.md` in `theme/system.js:6` and `eslint.config.js:45-46`.
- `UserMessage.tsx` ignores the `showButton` flag that the data model sets, so `showButton` is dead for user messages.

---

## Recommended priority

Two clusters dominate:

1. **Service-data pipeline vs deployed contract (#1, #2, #3, #4)** — the functional cluster most likely to break in production if `public/` is regenerated. Fix first.
2. **Dead / misleading contract surface (#5, #7, #8, #9)** — types, routes, tests, and hashes that claim guarantees they don't deliver. These give false confidence.

Low-risk quick wins for a first pass: #5 (delete dead DTO), #12 (docs/env), #13 (dep version sanity check).
