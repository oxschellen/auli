//! Normalização de texto compartilhada — a limpeza que os scrapers aplicam ao texto extraído.
//!
//! Duas variantes, ambas idempotentes e de uma linha lógica:
//! - [`clean`]: remove zero-width (`U+200B`) e no-break space (`U+00A0`) e comprime espaços. É a
//!   forma usada pela maioria (ce, mt, sp, rj, ms).
//! - [`clean_decoded`]: decodifica um punhado de entidades HTML e comprime espaços — **sem** o
//!   strip de zero-width/nbsp do `clean` (para bater byte a byte com o `clean_inline` de ba/pr,
//!   cujo texto pode conter zero-width preservado no snapshot).
//!
//! A variante *line-based* (mg/rs `clean_text`, que preserva quebras de linha) tem semântica própria
//! por formato e **fica local** em cada crate.

/// Comprime espaços e remove zero-width (`U+200B`) e no-break space (`U+00A0`).
pub fn clean(s: &str) -> String {
    s.replace('\u{200b}', "").replace('\u{00a0}', " ").split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Decodifica entidades HTML ([`decode_entities`]) e comprime espaços. **Não** remove zero-width/nbsp
/// (ver o header do módulo): preserva byte a byte o que o `clean_inline` de ba/pr produzia.
pub fn clean_decoded(s: &str) -> String {
    decode_entities(s).split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Decodifica o conjunto de entidades HTML que os portais da frota emitem. Superset do que ba usava
/// (pr acrescenta `&aacute;`); as entidades ausentes num dado texto são no-op, então é seguro como
/// padrão pt-BR.
pub fn decode_entities(s: &str) -> String {
    s.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&aacute;", "á")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_strippa_zero_width_e_comprime() {
        assert_eq!(clean("  a\u{200b}b   c \u{00a0}d "), "ab c d");
    }

    #[test]
    fn clean_decoded_decodifica_mas_preserva_zero_width() {
        // Decodifica entidades e comprime espaços...
        assert_eq!(clean_decoded("&aacute;gua &amp; sal  x"), "água & sal x");
        // ...mas NÃO remove o zero-width (diferença deliberada vs clean — ver D4/pr).
        assert_eq!(clean_decoded("a\u{200b}b"), "a\u{200b}b");
    }

    #[test]
    fn decode_entities_cobre_o_conjunto_da_frota() {
        assert_eq!(decode_entities("&aacute;gua &amp; sal &#39;x&#39; &lt;b&gt; &quot;y&quot; a&nbsp;b"),
                   "água & sal 'x' <b> \"y\" a b");
    }
}
