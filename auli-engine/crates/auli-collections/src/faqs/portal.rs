// Flattens the FAQ tree into the `portal-faqs.txt` knowledge file consumed by the RAG pipeline.
//
// Each leaf `Faq` node contributes one block per question, numbered sequentially:
//
//   // 1.
//   ## pergunta
//   <origin breadcrumb, if any>
//   <pergunta>
//
//   ## resposta
//   <resposta>
//   Link: <node url>
//
// `Menu`/`Geral` nodes contribute nothing themselves; the walk recurses through their children.
// Ported from the legacy `gerar_arquivo_portal_faqs_txt` / `write_portal_faq_items`.

use super::{FaqNode, PageType};

/// Renders the whole tree to the `portal-faqs.txt` text format.
pub fn render_portal_faqs(root: &FaqNode) -> String {
    let mut out = String::new();
    let mut counter = 1usize;
    // The root node is a menu container; start from its children (matches legacy behavior).
    for child in &root.children {
        write_items(child, &mut counter, &mut out);
    }
    out
}

fn write_items(node: &FaqNode, counter: &mut usize, out: &mut String) {
    if node.page_type == PageType::Faq {
        for item in &node.faq_items {
            out.push_str(&format!("// {}.\n", counter));
            out.push_str("## pergunta\n");
            if !node.origin.is_empty() {
                out.push_str(&node.origin);
                out.push('\n');
            }
            out.push_str(&item.pergunta);
            out.push('\n');
            out.push('\n');
            out.push_str("## resposta\n");
            out.push_str(&item.resposta);
            out.push('\n');
            out.push_str(&format!("Link: {}\n", node.url));
            out.push('\n');
            *counter += 1;
        }
    }

    for child in &node.children {
        write_items(child, counter, out);
    }
}
