//! Derivação dos artefatos de faqs a partir da coleta do snapshot (offline). O scraper produz a
//! `ColetaFaqs` (`Vec<FaqRaw>`); aqui materializamos a `Table<Faq>` (`<id>-faqs.json`, com
//! `text_to_embed`) e o print `portal-faqs.txt`. Não toca rede nem a árvore do scraper — só o snapshot.

use std::path::Path;

use crate::errors::Result;

/// Deriva `<id>-faqs.json` (contrato) + `portal-faqs.txt` da coleta de faqs.
pub fn process(id: &str, data_dir: &str, coleta: &auli_contract::ColetaFaqs) -> Result<()> {
    let items: Vec<auli_contract::Faq> = coleta.items.iter().map(faq_from_raw).collect();
    let table = auli_contract::Table::new(id, "faqs", items);
    let contract_path = format!("{}/{}-faqs.json", data_dir, id);
    if let Some(parent) = Path::new(&contract_path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&contract_path, serde_json::to_string_pretty(&table)?)?;
    println!("Wrote {} ({} faqs)", contract_path, table.len());

    let portal_path = format!("{}/portal-faqs.txt", data_dir);
    let portal = render_portal_faqs(&coleta.items);
    std::fs::write(&portal_path, &portal)?;
    println!("Wrote {} ({} bytes)", portal_path, portal.len());
    Ok(())
}

/// Um `Faq` (contrato) a partir de um `FaqRaw`: materializa `text_to_embed` = breadcrumb `origin` +
/// a pergunta (só a pergunta quando não há breadcrumb) — a mesma key do antigo
/// `EmbedStrategy::QuestionKey`. Demais campos copiados 1:1.
fn faq_from_raw(raw: &auli_contract::FaqRaw) -> auli_contract::Faq {
    let text_to_embed = if raw.origin.is_empty() {
        raw.pergunta.clone()
    } else {
        format!("{} {}", raw.origin, raw.pergunta)
    };
    auli_contract::Faq {
        pergunta: raw.pergunta.clone(),
        resposta: raw.resposta.clone(),
        origin: raw.origin.clone(),
        url: raw.url.clone(),
        text_to_embed,
    }
}

/// Renderiza `portal-faqs.txt` a partir do flat do snapshot — bloco `// N.` / `## pergunta`
/// breadcrumb+pergunta / `## resposta` resposta + `Link:`, na ordem do snapshot.
fn render_portal_faqs(items: &[auli_contract::FaqRaw]) -> String {
    let mut out = String::new();
    for (i, item) in items.iter().enumerate() {
        out.push_str(&format!("// {}.\n", i + 1));
        out.push_str("## pergunta\n");
        if !item.origin.is_empty() {
            out.push_str(&item.origin);
            out.push('\n');
        }
        out.push_str(&item.pergunta);
        out.push('\n');
        out.push('\n');
        out.push_str("## resposta\n");
        out.push_str(&item.resposta);
        out.push('\n');
        out.push_str(&format!("Link: {}\n", item.url));
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faq_from_raw_derives_embed_key() {
        let raw = |origin: &str| auli_contract::FaqRaw {
            pergunta: "q1".into(),
            resposta: "r1".into(),
            origin: origin.into(),
            url: "u".into(),
        };
        assert_eq!(faq_from_raw(&raw("Inicial | A")).text_to_embed, "Inicial | A q1");
        assert_eq!(faq_from_raw(&raw("")).text_to_embed, "q1");
    }

    #[test]
    fn render_portal_from_flat_matches_block_shape() {
        let items = vec![
            auli_contract::FaqRaw {
                pergunta: "q1".into(),
                resposta: "r1".into(),
                origin: "Inicial | A".into(),
                url: "ua".into(),
            },
            auli_contract::FaqRaw {
                pergunta: "q2".into(),
                resposta: "r2".into(),
                origin: String::new(),
                url: "ub".into(),
            },
        ];
        let out = render_portal_faqs(&items);
        assert_eq!(
            out,
            "// 1.\n## pergunta\nInicial | A\nq1\n\n## resposta\nr1\nLink: ua\n\n\
             // 2.\n## pergunta\nq2\n\n## resposta\nr2\nLink: ub\n\n"
        );
    }
}
