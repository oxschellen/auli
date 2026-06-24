# vector-store

A tiny, agnostic, **pure-Rust** flat cosine vector store — the deliberate "anti-zvec" for small
corpora. No external service, no C++ toolchain, no ANN index, no fixed embedding dimension. It
knows only `(id, vector, payload)` and persists each collection as one JSON file.

## Why this exists

For corpora of a few thousand vectors per collection, an exact brute-force cosine scan is *faster
to reason about and fast enough to run* — proportional to the problem. Heavyweight vector DBs
(C++ build deps, ~500 MB downloads, ANN tuning, durability machinery for billions of vectors)
solve problems you don't have at this scale. This crate is the opposite: a few hundred lines, three
runtime dependencies (`serde`, `serde_json`, `thiserror`), and a file you can open in a text editor.

## Design: read and write are separate types

The load-bearing property is that **reading and writing are different types**:

- [`ReadStore`] — opens a collection read-only and exposes `query_scored` / `list`. Immutable once
  loaded, so queries need no lock and an `Arc<ReadStore>` shares freely across threads.
- [`Writer`] — `reset` / `upsert`, persists to disk. The single writer.

A consumer that only needs to read links `ReadStore` and **cannot construct a `Writer`** — it is
incapable of writing by construction, not by convention.

## Generic payload

`Record<P>` / `CollectionData<P>` are generic over the payload `P: Serialize + DeserializeOwned`.
The store never inspects it. The on-disk JSON key for the payload is `document` (via
`#[serde(rename)]`) for compatibility with existing collection files.

## Distance semantics

`cosine_distance` returns a distance in `[0, 2]` (`1 - cos`). Lower is closer. A zero vector or a
width mismatch returns `2.0` — the metric's true maximum — so a degenerate vector sinks *below*
even a genuinely anti-correlated document instead of ranking at the orthogonal midpoint (`1.0`).

`Writer::upsert` **fixes a collection's dimension on first insert** and rejects any later vector of
a different width with `Error::DimensionMismatch`. That makes a model/dimension change a loud
write-time error rather than a silent retrieval degrade, and leaves the `2.0` fallback to cover
only legitimate anti-correlation at query time.

## Honest tradeoff

The persistence format is the simplest thing that works: **the whole collection file is rewritten
on every `upsert`** — O(n) per write. That is ideal for the build-once/read-many pattern of small
packs and *wrong* for a write-hot collection of hundreds of thousands of records. If you outgrow
it, you've outgrown this crate; reach for a real index.

## Example

```rust
use vector_store::{Writer, ReadStore};

// write (ingestion side)
let w = Writer::new("./packs");
w.reset::<String>("rs-faqs")?;
w.upsert("rs-faqs", &["id-1".into()], vec![vec![0.1, 0.2, 0.3]], &["full Q+A block".to_string()])?;

// read (serving side) — no Writer in sight
let store = ReadStore::<String>::load("./packs/rs-faqs.json")?;
let hits = store.query_scored(&[0.1, 0.2, 0.3], 10); // Vec<(payload, distance)>, best-first
# Ok::<(), vector_store::Error>(())
```
