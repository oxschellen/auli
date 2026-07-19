//! Ingestão dos pareceres a partir do texto autorado `<id>-portal-pareceres.txt` (em `data/<id>/ref`),
//! **offline**. Passo incremental "de trás pra frente": ainda não há scraper de pareceres, então
//! derivamos a `Table<Consulta>` a partir do arquivo de referência já existente.
//!
//! Produz o **snapshot bruto** `data/<id>/raw/<id>-pareceres.raw.json` — a entrada do passo
//! `sinopse`, que gera as sinopses e promove a saída final `<id>-pareceres.json` (que o `auli update`
//! vetoriza). Pipeline: `derive → .raw.json → sinopse → .json → update`.
//!
//! O derive **continua materializando** `text_to_embed` (com os fallbacks atuais): é o caminho legado
//! para registros que já chegam com `resumo` autorado — o `sinopse` reaproveita esses e só recompõe a
//! key dos que ele mesmo gera. O ponto único para sinopses novas é `compose_text_to_embed` no
//! `sinopse.rs`; esta materialização legada aposenta junto com este derive quando houver scraper.
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

/// Lê o `.txt` de referência da entidade, parseia os pareceres e grava o snapshot bruto
/// `<id>-pareceres.raw.json` no `raw/` (entrada do passo `sinopse`).
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
    let out_path = format!("{data_dir}/{id}-pareceres.raw.json");
    if let Some(parent) = Path::new(&out_path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&out_path, serde_json::to_string_pretty(&table)?)?;
    println!("Wrote {} ({} pareceres). Rode `sinopse` em seguida.", out_path, table.len());
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

    // Key de busca: ponto único `compose_text_to_embed` (numero + assunto + resumo). O `sinopse`
    // recompõe com a mesma fórmula na promoção — aqui é a materialização legada (registros já com
    // resumo autorado que dispensam sinopse).
    let text_to_embed = crate::sinopse::compose_text_to_embed(&numero, &assunto, &resumo);

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
        // text_to_embed = numero + assunto + resumo (a key indexa também o título).
        assert_eq!(p.text_to_embed, "PARECER Nº 25148\nICMS – crédito fiscal na cesta básica\n### Descrição Resumida\nAnalisa o crédito fiscal.");
    }

    #[test]
    fn record_without_resumo_usa_numero_e_assunto_na_key() {
        let items = parse_pareceres(SAMPLE);
        let p = &items[1];
        assert_eq!(p.numero, "PARECER Nº 25091");
        assert_eq!(p.resumo, "");
        assert_eq!(p.corpo, "Corpo do 25091.");
        // Sem resumo, a key ainda indexa numero + assunto.
        assert_eq!(p.text_to_embed, "PARECER Nº 25091\nRemessa entre estabelecimentos");
    }

    #[test]
    fn ignores_blank_and_unterminated_input() {
        assert!(parse_pareceres("").is_empty());
        assert!(parse_pareceres("// 1\n## pergunta:\n## resposta:\n").is_empty());
    }
}
