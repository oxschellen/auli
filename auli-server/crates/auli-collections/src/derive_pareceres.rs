//! Ingestão dos pareceres a partir do texto autorado `<id>-portal-pareceres.txt` (em `data/<id>/ref`),
//! **offline**. Passo incremental "de trás pra frente": ainda não há scraper de pareceres, então
//! derivamos a `Table<Consulta>` (`data/<id>/raw/<id>-pareceres.json`, com `text_to_embed`) a partir
//! do arquivo de referência já existente. O `auli update` vetoriza esse contrato como qualquer outro.
//!
//! Formato do arquivo (um parecer por bloco, delimitado por `// N`):
//! ```text
//! // 1
//! ## pergunta:
//! descricao: PARECER Nº 25148
//! assunto  : ICMS – ...
//! resumo   : ### Descrição Resumida ... (multilinha, palavras-chave)
//! link: http://legislacao.sefaz.rs.gov.br/...
//! ## resposta:
//! PARECER Nº 25148 ... (corpo integral) ...
//! ```

use std::path::Path;

use auli_contract::{Consulta, Table};

use crate::domain::entities::EntityConfig;
use crate::errors::Result;

/// Lê o `.txt` de referência da entidade, parseia os pareceres e grava a `Table<Consulta>` no `raw/`.
pub fn run(entity: &EntityConfig) -> Result<()> {
    let id = &entity.id;
    let data_dir = &entity.data_dir; // .../data/<id>/raw

    // O `.txt` autorado é irmão do `raw/`, em `ref/`.
    let base = Path::new(data_dir)
        .parent()
        .ok_or_else(|| format!("data_dir sem pai: {data_dir}"))?;
    let ref_path = base.join("ref").join(format!("{id}-portal-pareceres.txt"));
    if !ref_path.exists() {
        return Err(format!(
            "arquivo de referência ausente: {} — nada a ingerir para pareceres.",
            ref_path.display()
        )
        .into());
    }

    let content = std::fs::read_to_string(&ref_path)?;
    let items = parse_pareceres(&content);
    if items.is_empty() {
        return Err(format!("nenhum parecer parseado de {}", ref_path.display()).into());
    }

    let table = Table::new(id.as_str(), "pareceres", items);
    let out_path = format!("{data_dir}/{id}-pareceres.json");
    if let Some(parent) = Path::new(&out_path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&out_path, serde_json::to_string_pretty(&table)?)?;
    println!("Wrote {} ({} pareceres)", out_path, table.len());
    Ok(())
}

/// Uma linha é um delimitador de registro se for `// <número>` (com espaços à vontade).
fn is_delimiter(line: &str) -> bool {
    line.trim()
        .strip_prefix("//")
        .map(|rest| rest.trim().parse::<u32>().is_ok())
        .unwrap_or(false)
}

/// `label` seguido (opcionalmente por espaços) de `:` — devolve o valor à direita do `:`, aparado.
fn strip_label<'a>(line: &'a str, label: &str) -> Option<&'a str> {
    let rest = line.strip_prefix(label)?.trim_start();
    Some(rest.strip_prefix(':')?.trim())
}

/// Quebra o texto em blocos por `// N` e converte cada um em `Consulta` (blocos vazios são descartados).
pub(crate) fn parse_pareceres(content: &str) -> Vec<Consulta> {
    let mut records: Vec<Vec<&str>> = Vec::new();
    let mut current: Option<Vec<&str>> = None;
    for line in content.lines() {
        if is_delimiter(line) {
            if let Some(rec) = current.take() {
                records.push(rec);
            }
            current = Some(Vec::new());
        } else if let Some(rec) = current.as_mut() {
            rec.push(line);
        }
        // Linhas antes do primeiro `// N` (não deveria haver) são ignoradas.
    }
    if let Some(rec) = current.take() {
        records.push(rec);
    }
    records.iter().filter_map(|lines| parecer_from_lines(lines)).collect()
}

/// Monta um `Consulta` das linhas de um bloco. `## pergunta:` guarda os rótulos + resumo; `## resposta:`
/// separa o corpo integral. Devolve `None` se o bloco não tem conteúdo aproveitável.
fn parecer_from_lines(lines: &[&str]) -> Option<Consulta> {
    let resp_idx = lines.iter().position(|l| l.trim_start().starts_with("## resposta"))?;

    let mut numero = String::new();
    let mut assunto = String::new();
    let mut link = String::new();
    let mut resumo_lines: Vec<String> = Vec::new();

    for l in &lines[..resp_idx] {
        let t = l.trim_start();
        if t.starts_with("## pergunta") {
            continue;
        }
        if let Some(v) = strip_label(t, "descricao") {
            numero = v.to_string();
        } else if let Some(v) = strip_label(t, "assunto") {
            assunto = v.to_string();
        } else if let Some(v) = strip_label(t, "link") {
            link = v.to_string();
        } else if let Some(v) = strip_label(t, "resumo") {
            resumo_lines.push(v.to_string());
        } else {
            resumo_lines.push(l.trim_end().to_string());
        }
    }

    let resumo = resumo_lines.join("\n").trim().to_string();
    let corpo = lines[resp_idx + 1..].join("\n").trim().to_string();

    if corpo.is_empty() && assunto.is_empty() && resumo.is_empty() {
        return None;
    }

    // Key de busca: assunto + resumo (o essencial semântico). Fallbacks para o que existir.
    let text_to_embed = match (assunto.is_empty(), resumo.is_empty()) {
        (false, false) => format!("{assunto}\n{resumo}"),
        (false, true) => assunto.clone(),
        (true, false) => resumo.clone(),
        (true, true) => numero.clone(),
    };

    Some(Consulta { numero, assunto, resumo, corpo, link, text_to_embed, sinopse_info: None })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
// 1
## pergunta:
descricao: PARECER Nº 25148
assunto  : ICMS – crédito fiscal na cesta básica
resumo   : ### Descrição Resumida
Analisa o crédito fiscal.
link: http://legislacao/25148
## resposta:
PARECER Nº 25148

É o parecer.

// 2
## pergunta:
descricao: PARECER Nº 25091
assunto  : Remessa entre estabelecimentos
link: http://legislacao/25091
## resposta:
Corpo do 25091.
";

    #[test]
    fn parses_all_records_with_fields() {
        let items = parse_pareceres(SAMPLE);
        assert_eq!(items.len(), 2);

        let p = &items[0];
        assert_eq!(p.numero, "PARECER Nº 25148");
        assert_eq!(p.assunto, "ICMS – crédito fiscal na cesta básica");
        assert_eq!(p.resumo, "### Descrição Resumida\nAnalisa o crédito fiscal.");
        assert_eq!(p.link, "http://legislacao/25148");
        assert_eq!(p.corpo, "PARECER Nº 25148\n\nÉ o parecer.");
        // text_to_embed = assunto + resumo (the searchable key).
        assert_eq!(p.text_to_embed, "ICMS – crédito fiscal na cesta básica\n### Descrição Resumida\nAnalisa o crédito fiscal.");
    }

    #[test]
    fn record_without_resumo_falls_back_to_assunto_for_the_key() {
        let items = parse_pareceres(SAMPLE);
        let p = &items[1];
        assert_eq!(p.numero, "PARECER Nº 25091");
        assert_eq!(p.resumo, "");
        assert_eq!(p.corpo, "Corpo do 25091.");
        assert_eq!(p.text_to_embed, "Remessa entre estabelecimentos");
    }

    #[test]
    fn ignores_blank_and_unterminated_input() {
        assert!(parse_pareceres("").is_empty());
        assert!(parse_pareceres("// 1\n## pergunta:\n## resposta:\n").is_empty());
    }
}
