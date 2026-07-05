// In-memory model for the `faqs` scrape.
//
// The scraper walks the FAQ portal and produces this tree in memory; `faqs::run` flattens it into the
// snapshot's `Vec<FaqRaw>` and also serializes the tree itself to `faqs-tree.json` (consumed by the
// frontend's FAQ tab).
//
// Tree shape:
//   - `Menu`  nodes group other nodes via `children` (no FAQ content of their own).
//   - `Faq`   nodes carry the actual question/answer pairs in `faq_items` (+ a breadcrumb `origin`).
//   - `Geral` nodes are plain pages with neither children nor FAQ items.
//
// `origin`, `children` and `faq_items` are omitted from the JSON when empty, so a leaf `Faq`
// node serializes without a `children` key and a `Menu` node without `faq_items`/`origin`.

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
