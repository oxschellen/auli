# Implementation plan — resolving `findings.md`

**Base commit of the review:** `efdb97a`. **Main has since advanced to `cda2ee2`** (+6 commits), which adds a 5th state **MG (SEF-MG)** — a new `auli-scraper-mg` crate — plus a Windows launcher (`start_server.ps1`). The MG code postdates `findings.md` and was **not** covered by it; its impact on each finding is folded in below (see "MG scope" call-outs). The plan's file:line anchors were written against `efdb97a` but the referenced code is unchanged by `cda2ee2` except where noted.
**Author of plan:** verified each cited finding against the actual source before writing (see "Verification status" per item). The review's own caveat §6 was "no Rust toolchain" — **this machine has `cargo 1.96.0`**, so every Rust change below is compilable/testable locally. Verification commands are given per wave.

## MG (SEF-MG) — how the new scraper maps to existing findings

Read of `auli-server/crates/auli-scraper-mg/src/mg.rs` at `cda2ee2`:
- **1.1 (path collision): does NOT affect MG.** MG writes only the snapshot via `aggregate_servicos`; it never writes per-slug recovery files, so there is no scraper×process path collision. Clean.
- **3.1 (cache-first even without `--usecache`): DOES affect MG.** `fetch_page` calls `cache::read` before deciding to hit the network; `--usecache` only turns a miss into an error. Same pattern as RS/SC — MG is added to 3.1's scope in Wave 5.
- **3.9 (empty `link`): does NOT affect MG.** `push_servico` always builds `link` from `sys_id`, never empty.
- **3.10 (silent empty tab/category): largely handled in MG already** — categories with no readable items `eprintln!` a warning, and items whose tags map to no público fall back to all públicos with a warning. No new work needed for MG here.
- **4 (link-format non-uniformity): MG reproduces it** (`texto "url"`), consistent with the other serviços scrapers — remains "document, don't fix."

## How this is organized

Findings are regrouped from "by severity" into **execution waves** that make clean, reviewable commits. Order is chosen so low-risk correctness fixes land first and the one structural change (1.1) is isolated. Each item lists: **what**, **where** (file:line), **the fix**, **risk**, **verify**.

Global verification after every wave:
```bash
cd auli-server
cargo build --workspace
cargo test  --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Suggested commit granularity: **one commit per wave** (or per item for Wave 2). Conventional-commit prefixes noted.

---

## Wave 0 — Correctness & panic-safety (low risk, high value)

Small, self-contained, independently testable. Land first.

### 0.1 — `Writer::upsert` truncates silently on arity mismatch (finding 1.3)
- **Where:** [write.rs:62](auli-server/crates/vector-store/src/write.rs#L62)
- **Status:** ✅ confirmed. The `ids.iter().zip(embeddings).zip(payloads.iter())` writes `min(len)` and returns `Ok`. Dimension is checked; arity is not.
- **Fix:** before the dimension check, guard aridade:
  ```rust
  if ids.len() != embeddings.len() || ids.len() != payloads.len() {
      return Err(Error::ArityMismatch {
          ids: ids.len(), embeddings: embeddings.len(), payloads: payloads.len(),
      });
  }
  ```
  Add `ArityMismatch { ids, embeddings, payloads }` to `vector_store::Error` ([error.rs](auli-server/crates/vector-store/src/error.rs)). Keep the message in the crate's "loud write-time errors" style.
- **Risk:** none for current caller (`update.rs::ingest` derives all three from `table.items`). Pure defensive gate.
- **Verify:** add a unit test in write.rs `mod tests` mirroring `upsert_rejects_mismatched_dimension` — pass 2 ids / 1 payload, assert `ArityMismatch` and that nothing was written.
- **Commit:** `fix(vector-store): reject arity mismatch in upsert instead of truncating`

### 0.2 — Schema-version check is unreachable for structural bumps (finding 1.2)
- **Where:** [snapshot.rs:82](auli-server/crates/auli-scraper-kit/src/snapshot.rs#L82) (`load_colecoes`) and [:94](auli-server/crates/auli-scraper-kit/src/snapshot.rs#L94) (`load`); friendly message at [process.rs:22](auli-server/crates/auli-collections/src/process.rs#L22).
- **Status:** ✅ confirmed. Both `load` and `load_colecoes` do `serde_json::from_slice::<Snapshot>` (full typed) before any version check, so a v1 file dies with a raw serde error and the friendly text at process.rs:22 never runs.
- **Fix:** read a minimal header first, version-check, then deserialize typed. Add near the top of snapshot.rs:
  ```rust
  #[derive(serde::Deserialize)]
  struct SnapshotHeader { schema_version: u32, entidade: String }

  fn read_header(bytes: &[u8]) -> Result<SnapshotHeader> {
      serde_json::from_slice(bytes)
          .map_err(|e| anyhow::anyhow!("snapshot ilegível (nem o header desserializa): {e}"))
  }
  ```
  In both `load` and `load_colecoes`, after `std::fs::read`, call `read_header`; if `header.schema_version != SNAPSHOT_SCHEMA_VERSION`, return the **friendly** error here (move/duplicate the message from process.rs so it fires in the merge path too — a v1 file also breaks scraper re-merge, not just `process`). Only then `from_slice::<Snapshot>`.
- **Design note (assumption, stated):** the friendly check should live in `snapshot::load*` since that's the single choke point both `process` and the scrapers' merge go through. process.rs:22 can keep its check (defense in depth) or delegate — I recommend keeping process.rs's check as-is and adding one in snapshot.rs so the **merge** path is also covered. `entidade` mismatch stays where it is (process-specific).
- **Risk:** low. Header struct is a strict prefix of `Snapshot`; a valid v2 file deserializes both ways.
- **Verify:** unit test — write a JSON blob with `schema_version: 1` and v1-shaped `colecoes`, assert `load` returns the friendly message (string contains "Re-raspe"), not a serde error.
- **Commit:** `fix(scraper-kit): version-check snapshot header before typed deserialize`

### 0.3 — `SystemTime::elapsed().unwrap()` panics on clock step-back (finding 3.5)
- **Where:** [question.rs:36](auli-server/crates/auli-cli/src/api/handlers/question.rs#L36), [collections.rs:67](auli-server/crates/auli-cli/src/api/handlers/collections.rs#L67)
- **Status:** ✅ plausible per code (SystemTime is wall-clock, non-monotonic). `llm.rs` already uses `Instant` correctly — good precedent to copy.
- **Fix:** replace the `SystemTime::now()` + `.elapsed().unwrap()` timing pairs with `std::time::Instant::now()` + `.elapsed()` (infallible, monotonic). Mechanical.
- **Risk:** none — `Instant` is strictly better for elapsed-duration measurement.
- **Verify:** `cargo build`; grep to confirm no remaining `SystemTime` in request handlers.
- **Commit:** `fix(server): measure request latency with monotonic Instant`

### 0.4 — Manifest stamps `dim` from constant, not real vector width (finding 3.6)
- **Where:** [update.rs](auli-server/crates/auli-cli/src/update.rs) (manifest build, `dim = EMBED_DIM`)
- **Status:** ✅ plausible. Store guarantees intra-collection consistency; boot validates identity triple, not real width. If the model silently changes width without an `EMBED_MODEL_ID` bump, the manifest lies.
- **Fix:** after embedding a collection, assert `embeddings[0].len() == EMBED_DIM` (or stamp the manifest with the observed width and let boot compare). Minimal version:
  ```rust
  if let Some(first) = embeddings.first() {
      anyhow::ensure!(first.len() == EMBED_DIM,
          "embedder produziu dim {} ≠ EMBED_DIM {} — bump EMBED_MODEL_ID e re-gere", first.len(), EMBED_DIM);
  }
  ```
- **Risk:** none in steady state (widths already match). Turns a silent lie into a loud failure.
- **Verify:** covered by existing e2e `auli update` run; add no test (needs the real model).
- **Commit:** `fix(update): assert embedder width matches EMBED_DIM before stamping manifest`

### 0.5 — Version banner hardcoded (finding 4, banner)
- **Where:** [lib.rs](auli-server/crates/auli-cli/src/lib.rs) — `"Auli Server v0.3.0"`
- **Fix:** `concat!("Auli Server v", env!("CARGO_PKG_VERSION"))`. One line.
- **Risk:** none.
- **Commit:** folded into Wave 0 or Wave 6.

**Wave 0 verify:** full workspace build + test + clippy (commands above). All four new behaviors have unit tests except 0.4/0.5.

---

## Wave 1 — Security: rate-limiter / IP trust (finding 1.4)

- **Where:** [ratelimit.rs:56-71](auli-server/crates/auli-cli/src/api/ratelimit.rs#L56-L71) (`client_ip`), consumed by `rate_limit` middleware (:42) and `question_handler`'s IP log.
- **Status:** ✅ confirmed. `client_ip` trusts 6 headers from *any* caller; the code's own comment flags `X-Forwarded-For` as spoofable. With the new `--bind 0.0.0.0` default, direct-to-port access lets a client forge the limiter key → trivial 1 req/s bypass **and** unbounded key-map growth (keys are attacker-controlled, no GC).
- **Fix (recommended, matches the actual Cloudflare-Tunnel deployment):** trust **only** Cloudflare's headers, else fall to the socket peer. Reduce the list:
  ```rust
  const IP_HEADERS: &[&str] = &[
      "CF-Connecting-IP",   // Cloudflare Tunnel (our deployment)
      "True-Client-IP",     // Cloudflare Enterprise
  ];
  ```
  Drop `X-Real-IP`, `X-Forwarded-For`, `X-Cluster-Client-IP`, `X-Original-Forwarded-For`. `rate_limit` already falls back to `addr.ip()` when `client_ip` returns `None`, so direct-port callers are keyed by real socket peer — no code change to the fallback.
- **Why not a full "trusted proxy" config:** overkill for a single-tunnel deployment and adds config surface. If you later front the server with a non-CF proxy, reintroduce that proxy's header behind an explicit `--trust-proxy-headers` flag. Noted, not built now (Simplicity First).
- **Risk:** if anything *other* than Cloudflare currently sets the client IP, its rate-limit keying changes to socket peer. Given the documented deployment (CF Tunnel), that's the intended behavior. Update the module doc comment (:1-6) to say "trust CF headers only; else socket peer."
- **Verify:** unit test `client_ip`: `X-Forwarded-For: 1.2.3.4` alone → `None` (falls to peer); `CF-Connecting-IP: 1.2.3.4` → `Some(1.2.3.4)`.
- **Commit:** `fix(server): trust only Cloudflare IP headers for rate-limit keying`

---

## Wave 2 — Pipeline path collision (finding 1.1) — **isolate, review carefully**

This is the one structural change. Do it alone, commit alone.

- **Where:**
  - RS scraper incremental-recovery write: [extrair_descricoes.rs:40](auli-server/crates/auli-scraper-rs/src/servicos/extrair_descricoes.rs#L40) `format!("{}/{}.json", data_dir, file_s)` → `save_servicos_to_json` at :92.
  - collections per-público derived write: [servicos/mod.rs:94](auli-server/crates/auli-collections/src/servicos/mod.rs#L94) `format!("{}/{}.json", data_dir, pubx.slug)`.
- **Status:** ✅ confirmed collision. Both resolve to `data/<id>/raw/<slug>.json` (same `data_dir = .../<id>/raw`, and RS `file_s` == the slug). Two different schemas + semantics (scraper: header-in-`descricao`, one entry per link; process: clean body, one entry per `(link,classe)`, renumbered ids) share the path. Whichever ran last wins; if `build-frontend-public.sh` runs between scrape and process, `public/` ships the wrong format.
- **Fix (recommended):** give the scraper's recovery file a **distinct, namespaced path** so it can never be confused with a derived artifact:
  - In extrair_descricoes.rs, change the target dir to `{data_dir}/scrape/` (i.e. `data/<id>/raw/scrape/<slug>.json`), creating it with `create_dir_all` before the first write.
  - **Before implementing, grep for readers of the old path** to be sure nothing consumes the scraper's recovery file expecting the old location/format:
    ```bash
    rg -n 'raw/[^/]+\.json|save_servicos_to_json|file_s' auli-server/crates
    ```
    Expectation (from static read): the recovery file is write-only progress state; nothing reads it back except the scraper's own resume logic (which must be updated to the new dir too). Confirm before editing.
- **Assumption stated (no question asked):** I assume the per-slug recovery file is scraper-internal resume state, not a consumed artifact. If the grep shows an external reader, fall back to the lighter fix: **suffix** the recovery file (`<slug>.scrape.json`) instead of moving it. Both remove the collision; the subdir is cleaner.
- **Also fix the doc that states the inverse (finding 2, servico.rs):** [servico.rs:10-11](auli-server/crates/auli-scraper-kit/src/servico.rs#L10-L11) says the per-público JSONs "`descricao` carrega o header" — `process` writes `descricao = s.descricao.clone()` (clean body, servicos/mod.rs:90, comment :78). Correct the doc: the per-público files written by `process` carry the **clean body**; the header lives only in the scraper's recovery file and is stripped by `descricao_body` when materializing the snapshot.
- **Risk:** medium — touches scrape resume behavior. Mitigate by (a) the grep above, (b) a manual scrape→process→build-public dry run on RS if a snapshot/cache is present locally.
- **Verify:** `cargo test -p auli-scraper-rs -p auli-collections`; then, if data present, run the RS scrape (cache mode) → `process rs` → confirm `data/rs/raw/servicos-ao-cidadao.json` is the clean-body per-público format and the recovery file sits under `raw/scrape/`.
- **Commit:** `fix(pipeline): move RS scraper recovery files out of the derived-artifact path`

---

## Wave 3 — LLM client robustness (finding 3.4)

Three independent arestas in [llm.rs](auli-server/crates/auli-cli/src/llm.rs); do as one commit.

- **3.4(a) — per-request `println!` of the model (:16):** remove it (duplicates `Config::log_summary` at boot and bypasses `tracing`). If a per-request signal is wanted, downgrade to `tracing::debug!`. Also reconsider the two `println!`s at :52/:74 for consistency (tracing), but that's optional.
- **3.4(b) — retry/error gaps:**
  - `resp.text().await` (:48) is outside the retry — a body-read failure doesn't retry. Move it inside the retry arm so a transient read error re-loops.
  - A non-2xx with a non-JSON body currently hits `serde_json::from_str` (:65) and surfaces raw `SerdeJson` as the answer. Add an explicit `resp.status()` check: on non-success, read the body and return a friendly `format!("Erro na API do modelo (HTTP {status}): …")` instead of parsing.
- **3.4(c) — no client timeout (:21):** `Client::new()` never times out; the frontend gives up at 25 s ([callServerAPI.ts](auli-frontend/src/pages/chat/utils/callServerAPI.ts)) while the server keeps the handler + LLM call alive. Fix:
  ```rust
  let client = Client::builder()
      .timeout(std::time::Duration::from_secs(20))   // < frontend's 25s
      .build()
      .map_err(Error::from)?;
  ```
  Build it once (it's currently per-call anyway; leave that unless you want a shared client). A timeout now surfaces as `is_timeout()` → retried by the existing arm.
- **Risk:** low; improves failure behavior. Confirm the 20 s < 25 s ordering against the current frontend constant before committing.
- **Verify:** `cargo build`; a manual request against an unreachable/garbage LLM endpoint should now return a friendly message within ~20 s, not hang.
- **Commit:** `fix(server): LLM client timeout, retry body reads, friendly non-JSON error`

---

## Wave 4 — Documentation sync (findings 2.1–2.7, 2.2, 2.3, 2.4, 2.5, plus 3.1/3.2/3.3 doc parts)

No code behavior; high value because docs are the entry point. Group into one `docs:` commit (or split README vs inline-comments).

### 4.1 — `README.md` (finding 2.1) — most-drifted doc
- (a) `cargo run -p auli-collections -- [--usecache] <entity> <collection>` **errors at runtime** now — collections rejects `--` flags and rejects `faqs|servicos`. Replace with the real entry points: `auli-scraper-<id>` binaries for collection, `auli-collections <entity> process` for derivation.
- (b) "FAQs … headless Chrome + ureq" → FAQs are **pure ureq/AJAX**; Chrome is only in `servicos/extrair_descricoes.rs`.
- (c) "Services — RS … and SC" → add **SP and PR** (both are crates, in the registry, with `public/` data).
- (d) Layout table omits `auli-scraper-kit` and the per-entity binaries → add them.

### 4.2 — `auli_code.md` (findings 2.2, 2.3)
- §5.3 / §5.5 (l.444): FAQs of RS marked `✅ (Chrome)` → FAQs use no `headless_chrome` (grep confirms none imported in `faqs/`). Change to ureq/AJAX. (`auli_operations.md` is already correct at crate level.)
- §6.1 (l.461): "guarda de regressão em `from_kind`" → the guard is the `assert!(from_kind("services").is_err())` **test**, not a `from_kind` arm. Reword.

### 4.3 — `data/registry.toml:15` (finding 2.4) — fossil in the source of truth
- Comment "`servicos` mapeia ao kind vetorial `services`" contradicts `corpus.rs` (single kind `servicos`; `services` rejected) and `auli_pendencias.md §5`. Fix the comment to say `servicos` maps to vector kind `servicos`. Highest-leverage one-liner (everyone edits this file).

### 4.4 — Intra-code fossil comments (finding 2.5)
- [sc.rs:9](auli-server/crates/auli-scraper-sc/src/sc.rs#L9): header step 5 "write one per-público file … caller aggregates" contradicts the same file's :146-148 ("SC no longer writes per-público files — fan-out is `process`'s job"). Delete/fix the header line.
- [sc.rs:232](auli-server/crates/auli-scraper-sc/src/sc.rs#L232): `build_descricao` references `gerar_portal_servicos::descricao_body` — module gone; it's `auli_scraper_kit::descricao_body`. Fix reference.
- [faqs/faq.rs:4](auli-server/crates/auli-scraper-rs/src/faqs/faq.rs#L4): "the tree itself is not persisted anymore" / "root node is what gets written to `<collection>.json`" — `faqs::run` **does** write `faqs-tree.json` (mod.rs). Fix to match mod.rs's own doc.
- [vector-store/src/lib.rs](auli-server/crates/vector-store/src/lib.rs) (`scan` doc): "Shared by ReadStore and the Phase-1 server registry" — no such registry; only `ReadStore` consumes it. Drop the clause.
- [scraper-kit/cache.rs:4-5](auli-server/crates/auli-scraper-kit/src/cache.rs#L4-L5): "expensive headless-Chrome renders" in a kit shared by SC/SP/PR (no Chrome). Generalize the comment (it's a generic HTML cache).
- [config.rs:5](auli-server/crates/auli-cli/src/config.rs#L5) and [update.rs:14](auli-server/crates/auli-cli/src/update.rs#L14): "LLM/JWT/DB vars" — no JWT/DB in any current `Config`. Drop JWT/DB.

### 4.5 — Contract docs (findings 2.6, 2.7)
- [contract snapshot.rs:90](auli-server/crates/auli-contract/src/snapshot.rs#L90): `Publico::slug` documented **without** entity prefix (correct — all four scrapers write unprefixed; prefix is added by `build-frontend-public.sh`). But the sample at [:161](auli-server/crates/auli-contract/src/snapshot.rs#L161) and [kit/snapshot.rs:138](auli-server/crates/auli-scraper-kit/src/snapshot.rs#L138) use `"rs-servicos-ao-cidadao"` (prefixed), teaching the wrong convention. Change the test samples to unprefixed slugs (`"servicos-ao-cidadao"`). Nothing breaks (field is opaque); it stops miseducating.
- [contract snapshot.rs:120](auli-server/crates/auli-contract/src/snapshot.rs#L120): "URL … a chave natural única do snapshot" is unconditional, but SP deliberately shares login URLs across services (documented in `auli_code.md §5.3`; SP builds `ServicoRaw` directly, skipping `aggregate_servicos`). Soften the doc: link is the natural key **except SP**, where multiple services share a URL by design.

### 4.6 — smaller doc fixes rolled in here
- **3.3:** [process.rs:17](auli-server/crates/auli-collections/src/process.rs#L17) error interpolates `entity.id` as if it were the binary → "rode `rs faqs`". Change to the real pattern `auli-scraper-{id} faqs` / `auli-scraper-{id} servicos`.
- **3.2:** [.gitignore:44/58](.gitignore#L44) cache patterns `**/data/*/cache/` / `data/*/cache/` don't match the real `data/<id>/raw/cache/`. Add `data/*/raw/cache/` (harmless today because `data/*/raw/` is fully ignored; correct it so a future partially-versioned `raw/` won't leak cache). Fix the orphaned "Scraper cache" comment path.

- **Risk:** doc-only except 3.3 (string) and 4.4 sc.rs (comments) — all no-behavior. 2.6 changes test literals (must still compile/pass).
- **Verify:** `cargo test -p auli-contract -p auli-scraper-kit` after 2.6; `rg -n 'headless.?Chrome|JWT|DB|Phase-1|services'` to confirm no stale hits remain in the touched files.
- **Commits:** `docs(readme): realign scraper commands, FAQ engine, SP/PR, layout` · `docs(code): fix RS-FAQ Chrome mislabel and from_kind regression-guard note` · `chore(comments): remove fossil references (JWT/DB, Chrome, Phase-1 registry, gerar_portal)` · `docs(registry): correct servicos→services vector-kind fossil`

---

## Wave 5 — Behavior robustness minors (findings 3.1, 3.7, 3.8, 3.9, 3.10)

Optional-but-recommended; each independent. Batch or cherry-pick.

- **3.1 — cache-first even without `--usecache`:** re-scraping never refreshes cached pages; you must `rm -rf data/<id>/raw/cache/`. For SC this hides new portal services (paginated list + buildId are cached); **MG has the identical pattern** ([mg.rs](auli-server/crates/auli-scraper-mg/src/mg.rs) `fetch_page`: `cache::read` before the network, `--usecache` only converts a miss into an error), so a re-run never picks up new SEF-MG catalog items without clearing the cache. **Two-part fix:** (i) add a `--refresh` flag that bypasses `cache::read` and re-fetches (kit `cache::read` + `faqs/fetch.rs` + MG `fetch_page`); (ii) document the `rm` in `auli_operations.md` runbook. If a flag is too much, at minimum do (ii) + soften the "usando apenas páginas em cache" print so normal mode's cache-first behavior isn't misleading. **Recommended:** ship (ii) now (doc), defer (i) unless refresh is a real operational need.
- **3.7 — divergent DEFAULT_ENTITY sources:** backend hardcodes `DEFAULT_ENTITY = "rs"` (server + collections); frontend derives `entities[0].id` from registry order. Reordering the registry silently desyncs them. **Fix:** make backend derive the default from the registry too (first entity, or an explicit `default = true` field in `registry.toml`). **Recommended:** add `default` boolean to the registry entity schema and read it in both backend and `gen-frontend-entities.mjs` — single source, explicit. If that's too invasive now, at minimum add a comment in `registry.toml` that RS is the assumed default and reordering breaks it.
- **3.8 — frontend tab fallback ≠ RS order:** `getDefaultTipoServicos()` ([servicoslist/utils.ts](auli-frontend/src/pages/servicoslist/utils.ts)) starts "Empresas"; RS index starts "Cidadãos". Only affects deploys missing the index. **Fix:** reorder the fallback to match `servicos-index.json` (Cidadãos first), or better, drop the hardcoded fallback and treat a missing index as empty (the index is always shipped). **Recommended:** align the order (one-line) — removing the fallback is a bigger call.
- **3.9 — SP empty `link`:** [sp/scrape.rs:148](auli-server/crates/auli-scraper-sp/src/scrape.rs#L148) `unwrap_or_default` → `canonical("")` yields `"Link: "`. **Fix:** count + `⚠️` in the existing `sem_publico` counter style when URL is absent; keep ingesting (don't bail). Quality signal only.
- **3.10 — PR panel ids look like portal typos:** [pr/scrape.rs](auli-server/crates/auli-scraper-pr/src/scrape.rs) `publicos()` uses `servicos-tema-cidado`, `…-municpio`, `…-legislao`. Likely the real (mistyped) DOM ids. **Fix:** add an "aba vazia" warning analogous to `orphan_check` — if a tab yields 0 ocorrências, warn (a wrong id currently renders 0 silently; `bail!` only covers the mega-menu container).
- **Risk:** low each. 3.7's registry-schema change touches `gen-frontend-entities.mjs` + both backend readers — test the regen (`check-registry-sync.sh`).
- **Verify:** `cargo test` for Rust items; `npm test` / `npm run build` in auli-frontend for 3.8; run `scripts/check-registry-sync.sh` for 3.7.
- **Commits:** one per item, `feat(...)`/`fix(...)` as fits.

---

## Wave 6 — Cosmetics & hygiene (finding 4)

Lowest priority. Batch into one `chore:` commit or drop entirely.

- **Version banner** → `env!("CARGO_PKG_VERSION")` (already listed as 0.5; do here if not done).
- **`url_to_filename` duplicated** in `scraper-kit/cache.rs` and `scraper-rs/faqs/mod.rs` → extract to the kit and have FAQs use it (FAQs cache doesn't use the kit today; small refactor). *Only if touching those files anyway — Surgical Changes.*
- **`DEFAULT_SYSTEM_PROMPT` duplicated** in `auli-cli/entities.rs` and `auli-collections/domain/entities.rs` (collections' copy is in a `#![allow(dead_code)]` module, unused) → delete the collections copy, or leave (it's dead). **Recommended:** delete the unused copy.
- **`auli-cli/Cargo.toml`:** `toml = "1.1.2"` mis-grouped under "Rate limiting" comment → move under a correct heading.
- **Link format non-uniform** (`[t](url)` for FAQs vs `t "url"` for serviços) and slug divergence (`servicos-a-servidores-publicos` vs `servicos-a-servidores`) → **document, don't fix**. Changing emitted formats re-generates all packs and shifts embeddings; not worth it absent a concrete need. Note it in `auli_pendencias.md` as known/accepted.
- **`check-registry-sync.sh`** dirties the tree on desync (regenerates before diff) → acceptable; add a one-line comment noting the side effect.

- **Verify:** build + clippy.
- **Commit:** `chore: dedupe prompt/version constants, fix Cargo.toml grouping`

---

## Explicitly NOT doing (with reason)

- **Rewriting emitted link/slug formats** (4, last bullet) — regenerates every pack, shifts the vector space, no functional gain. Document as accepted.
- **Full trusted-proxy config** (1.4) — over-engineered for a single CF-Tunnel deployment. CF-only + socket-peer fallback is sufficient; revisit behind a flag if a second proxy appears.
- **A `--refresh` flag** (3.1) unless refresh is a stated operational need — ship the runbook doc first, add the flag only if manual cache-clearing proves painful.

## Suggested execution order & effort

| Wave | Theme | Risk | Rough effort |
|------|-------|------|--------------|
| 0 | correctness/panic-safety | low | 1–2 h (with tests) |
| 1 | rate-limiter IP trust | low | 30 min |
| 2 | path collision | **medium** | 1 h + a scrape dry-run |
| 3 | LLM robustness | low | 45 min |
| 4 | docs sync | none | 1–1.5 h |
| 5 | robustness minors | low | 2–3 h (pick subset) |
| 6 | cosmetics | none | 30 min |

Waves 0, 1, 3, 4 are safe to land in any order. **Wave 2 is the only one that needs a manual pipeline dry-run** before you trust it. Everything is `cargo test`/`clippy`-gated on this machine.
