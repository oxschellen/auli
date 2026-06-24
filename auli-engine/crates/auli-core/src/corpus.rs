// Per-kind retrieval knobs for the four content kinds (services, faqs, pareceres, notas).
//
// The *shape* of the data and how `text_to_embed`/`stored_repr` are derived now live in
// `auli-contract` (materialized by the scraper). This module only carries what the engine needs at
// query time: the kind name (vector-collection suffix / route param) and how many documents to
// retrieve in RAG. Nothing here affects what was embedded, so it is independent of
// `manifest::STRATEGY_VERSION`.

#[derive(Debug, Clone, Copy)]
pub struct Collection {
    pub kind: &'static str, // "services" -> collection suffix, route param, EntityConfig::collection(kind)
    pub n_results: usize,   // how many documents to retrieve for RAG
}

pub const SERVICES: Collection = Collection { kind: "services", n_results: 10 };
pub const FAQS: Collection = Collection { kind: "faqs", n_results: 20 };
pub const PARECERES: Collection = Collection { kind: "pareceres", n_results: 3 };
pub const NOTAS: Collection = Collection { kind: "notas", n_results: 1 };

// All four kinds, for callers that iterate (boot-time pack loading in `packs::load_all`).
pub const ALL: [&Collection; 4] = [&SERVICES, &FAQS, &PARECERES, &NOTAS];

// Resolve a kind name to its Collection. Unknown -> friendly Err.
pub fn from_kind(kind: &str) -> std::result::Result<&'static Collection, String> {
    match kind {
        "services" => Ok(&SERVICES),
        "faqs" => Ok(&FAQS),
        "pareceres" => Ok(&PARECERES),
        "notas" => Ok(&NOTAS),
        _ => Err(format!(
            "Tipo de coleção desconhecido: '{}'. Tipos válidos: services, faqs, pareceres, notas.",
            kind
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_kind_resolves_known_and_rejects_unknown() {
        assert_eq!(from_kind("services").unwrap().kind, "services");
        assert_eq!(from_kind("faqs").unwrap().n_results, 20);
        // The UI/scraper label `servicos` is NOT a vector kind — only `services` is.
        assert!(from_kind("servicos").is_err());
    }
}
