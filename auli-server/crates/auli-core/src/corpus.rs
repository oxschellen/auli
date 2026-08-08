// Per-kind retrieval knobs for the four content kinds (servicos, faqs, pareceres, notas).
//
// The *shape* of the data and how `text_to_embed`/`stored_repr` are derived now live in
// `auli-contract` (materialized by the scraper). This module only carries what the engine needs at
// query time: the kind name (vector-collection suffix / route param) and how many documents to
// retrieve in RAG. Nothing here affects what was embedded, so it is independent of
// `manifest::STRATEGY_VERSION`.

#[derive(Debug, Clone, Copy)]
pub struct Collection {
    // Collection kind — one vocabulary shared everywhere: the vector-collection suffix, the pack
    // file name (`<id>-servicos.json`), `EntityConfig::collection(kind)`, and the `/v1/{kind}/list`
    // route param (also the UI/registry/scraper label). See `from_kind`.
    pub kind: &'static str,
    pub n_results: usize, // how many documents to retrieve for RAG
}

pub const SERVICES: Collection = Collection {
    kind: "servicos",
    n_results: 10,
};
pub const FAQS: Collection = Collection {
    kind: "faqs",
    n_results: 20,
};
pub const PARECERES: Collection = Collection {
    kind: "pareceres",
    n_results: 10,
};
// Acórdãos do TARF (D-A3): MESMA política dos pareceres na v1 — mesma família de documento
// (jurisprudência), mesmo formato de payload no pack, mesmas bandas de score. Revisar só com
// evidência de uso real, não por simetria estética.
pub const TARF: Collection = Collection {
    kind: "tarf",
    n_results: 10,
};
pub const NOTAS: Collection = Collection {
    kind: "notas",
    n_results: 1,
};

// All five kinds, for callers that iterate (boot-time pack loading in `packs::load_all`).
pub const ALL: [&Collection; 5] = [&SERVICES, &FAQS, &PARECERES, &TARF, &NOTAS];

// Resolve a kind name to its Collection. `servicos` is the single vocabulary — the `/v1/{kind}/list`
// route param, the vector-collection suffix, and the UI/registry label all agree. Unknown -> Err.
pub fn from_kind(kind: &str) -> std::result::Result<&'static Collection, String> {
    match kind {
        "servicos" => Ok(&SERVICES),
        "faqs" => Ok(&FAQS),
        "pareceres" => Ok(&PARECERES),
        "tarf" => Ok(&TARF),
        "notas" => Ok(&NOTAS),
        _ => Err(format!(
            "Tipo de coleção desconhecido: '{}'. Tipos válidos: servicos, faqs, pareceres, tarf, notas.",
            kind
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_kind_resolves_known_and_rejects_unknown() {
        // `servicos` is the one vocabulary — route param, vector-collection suffix, and UI label.
        assert_eq!(from_kind("servicos").unwrap().kind, "servicos");
        assert_eq!(from_kind("faqs").unwrap().n_results, 20);
        // The old English spelling `services` is no longer a kind.
        assert!(from_kind("services").is_err());
    }
}
