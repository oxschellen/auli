//! Árvore de FAQs que a aba de FAQs do frontend consome (`<id>-faqs-tree.json`, lida pelo
//! `parseFaqs.ts`). É contrato de fio com o frontend, por isso mora aqui e não em cada scraper
//! (D-SCFAQ-5): o RS a produz caminhando o portal, o SC a monta da árvore de assuntos.
//!
//! Tree shape:
//!   - `Menu`  nodes group other nodes via `children`. RS menus have no FAQ content of their own; a
//!     SC menu may also carry `faq_items` (a question attached to an intermediate subject).
//!   - `Faq`   nodes carry the actual question/answer pairs in `faq_items` (+ a breadcrumb `origin`).
//!     An item may carry its own `url` when the source links each question (SC); otherwise the UI
//!     links it to its node's page (RS).
//!   - `Geral` nodes are plain pages with neither children nor FAQ items.
//!
//! `origin`, `children` and `faq_items` are omitted from the JSON when empty, so a leaf `Faq`
//! node serializes without a `children` key and a `Menu` node without `faq_items`/`origin`.

use serde::{Deserialize, Serialize};

/// Kind of page a node represents. Serializes as `"Menu"` / `"Faq"` / `"Geral"`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PageType {
    Menu,
    Faq,
    Geral,
}

/// A single question/answer pair within a `Faq` node.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FaqItem {
    pub pergunta: String,
    pub resposta: String,
    /// The question's own link, when the source has one (SC: `…/perguntas/buscar?numero=<id>`).
    /// Absent in RS, where the question lives on its node's page — the UI then uses the node `url`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// One node of the FAQ tree. The root node is what gets written to `faqs-tree.json`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FaqNode {
    /// Human-readable page title (for `Faq` nodes this is the cleaned page title).
    pub title: String,
    /// Canonical page URL.
    pub url: String,
    /// Whether this node is a menu, a FAQ page, or a plain page.
    pub page_type: PageType,
    /// Breadcrumb trail (e.g. `"Inicial | Perguntas Frequentes | ... "`). Only on `Faq` nodes.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub origin: String,
    /// Child nodes, for `Menu` nodes that group other pages.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<FaqNode>,
    /// Question/answer pairs, for `Faq` nodes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub faq_items: Vec<FaqItem>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_sem_url_nao_escreve_a_chave() {
        // É o que deixa o `rs-faqs-tree.json` igual byte a byte: o RS não tem link por pergunta.
        let item = FaqItem {
            pergunta: "P?".to_string(),
            resposta: "R.".to_string(),
            url: None,
        };
        let json = r#"{"pergunta":"P?","resposta":"R."}"#;
        assert_eq!(serde_json::to_string(&item).unwrap(), json);
        assert_eq!(serde_json::from_str::<FaqItem>(json).unwrap(), item);
    }

    #[test]
    fn item_com_url_escreve_a_chave() {
        let item = FaqItem {
            pergunta: "P?".to_string(),
            resposta: "R.".to_string(),
            url: Some("https://x/p?numero=1".to_string()),
        };
        let json = serde_json::to_string(&item).unwrap();
        assert_eq!(
            json,
            r#"{"pergunta":"P?","resposta":"R.","url":"https://x/p?numero=1"}"#
        );
        assert_eq!(serde_json::from_str::<FaqItem>(&json).unwrap(), item);
    }
}
