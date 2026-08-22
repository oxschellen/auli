# Rascunho de issue para `kadir/cloakrs` — não publicado

**Estado:** redigido, **não enviado**. Publicar num repositório de terceiros é ação para fora e é
decisão do mantenedor. Texto em inglês abaixo, pronto para colar.

**Onde:** <https://github.com/kadir/cloakrs> · crate `cloakrs-patterns` 0.3.0 (MIT).

---

## `CreditCardRecognizer` reports a `span` wider than the `text` it stores, breaking sanitize→restore round-trip

**Version:** `cloakrs-patterns 0.3.0` (also affects anything using `PromptSanitizer` with the default
registry).

### Summary

`CreditCardRecognizer::scan` builds the `PiiEntity` with a **span over the raw regex match** but a
**trimmed `text`**:

```rust
// src/credit_card.rs
span: Span::new(matched.start(), matched.end()),
text: matched.as_str().trim().to_string(),
```

The regex is `\b(?:\d[ -.]?){13,19}\b`, and the optional `[ -.]` in the last repetition lets a match
**end on a separator**. When it does, the span covers a character the stored text does not, so
`PromptMapping::restore` writes back a shorter string than the region it replaced — and the sanitized
document silently loses that character.

### Reproduction

```rust
let scanner = default_registry().into_scanner_builder().build().unwrap();
let sanitizer = PromptSanitizer::new(scanner);

let original = "Produto: ROST OFF WURTH 300 ML 27101932 6910 12 UN";
let (masked, mapping) = sanitizer.sanitize(original).unwrap();
assert_eq!(mapping.restore(&masked), original); // fails
```

```text
masked:   "Produto: ROST OFF WURTH 300 ML [CREDIT_CARD_1]UN"
restored: "Produto: ROST OFF WURTH 300 ML 27101932 6910 12UN"
original: "Produto: ROST OFF WURTH 300 ML 27101932 6910 12 UN"
                                                        ^ space lost
```

Note the placeholder in `masked` is directly followed by `UN` — the span had swallowed the space.

### Impact

The loss is silent: no error, no warning, and the restored text still *contains* the original value,
so an assertion written as `restored.contains(value)` passes. Only equality catches it. That is how it
survived our own test suite until we ran the library over a large real corpus.

### How it was found

Running the library over two independent corpora of Brazilian tax documents:

| corpus | documents | round-trip failures |
|---|---:|---:|
| administrative tax court rulings | 22.476 | **18** |
| tax rulings (consultas) | 19.780 | **16** |

All 34 failures are this defect. The matches are never actual card numbers — they are quantities,
case numbers and fiscal code tables (`20.054.00 8214.10.00 …`) that happen to be 13–19 digits and
pass Luhn. That is expected for a Luhn-only heuristic and is **not** what this issue is about; the
issue is the span/text disagreement, which would corrupt the round-trip even for a true positive.

### Suggested fix

Make the span match the stored text. Either trim the span to the last digit:

```rust
let trimmed = matched.as_str().trim_end_matches([' ', '-', '.']);
let end = matched.start() + trimmed.len();
PiiEntity {
    span: Span::new(matched.start(), end),
    text: trimmed.to_string(),
    ...
}
```

or tighten the regex so a match cannot end on a separator (`\b(?:\d[ -.]?){12,18}\d\b`).

The first is the smaller change and also fixes leading whitespace if any pattern ever produces it.
A general guard — asserting `span.len() == text.len()` when constructing a `PiiEntity`, at least in
debug — would catch this class in any recognizer rather than only in this one.

### Environment

Rust 1.96, `cloakrs-core`/`cloakrs-patterns`/`cloakrs-locales` all `=0.3.0`.
