// Generic description of a content "kind" (services, faqs, pareceres, notas).
//
// Each kind shares one processing path: parse a source file into blocks, turn each block
// into a stored document + an embedding text, vectorize, and upsert into the entity's
// vector collection `<id>-<kind>`. The differences between kinds (block delimiter, whether
// we embed the full text or a derived description, and how many results to retrieve in RAG)
// are captured here as data instead of duplicated code.

use std::fs::File;
use std::io::{BufRead, BufReader};

use crate::errors::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbedStrategy {
    // Embed and store the same block (notas).
    FullText,
    // Embed a derived description but store the full cleaned text (servicos).
    Description,
    // Embed only the parsed `## pergunta` question key, but store the full block (faqs,
    // pareceres). A short, high-signal key gives a sharper vector; the full Q+A is still served.
    QuestionKey,
}

#[derive(Debug, Clone, Copy)]
pub struct Collection {
    pub kind: &'static str,      // "services" -> collection suffix, route param, EntityConfig::collection(kind)
    pub file: &'static str,      // source file base name -> EntityConfig::data_file(file)
    pub delimiter: &'static str, // block separator: "//" (servicos) or "## pergunta" (the rest)
    pub embed: EmbedStrategy,
    pub n_results: usize, // how many documents to retrieve for RAG
}

pub const SERVICES: Collection = Collection {
    kind: "services",
    file: "portal-servicos.txt",
    delimiter: "//",
    embed: EmbedStrategy::Description,
    n_results: 10,
};

pub const FAQS: Collection = Collection {
    kind: "faqs",
    file: "portal-faqs.txt",
    delimiter: "## pergunta",
    embed: EmbedStrategy::QuestionKey,
    n_results: 20,
};

pub const PARECERES: Collection = Collection {
    kind: "pareceres",
    file: "portal-pareceres.txt",
    delimiter: "## pergunta",
    embed: EmbedStrategy::QuestionKey,
    n_results: 3,
};

pub const NOTAS: Collection = Collection {
    kind: "notas",
    file: "portal-notas.txt",
    delimiter: "## pergunta",
    embed: EmbedStrategy::FullText,
    n_results: 1,
};

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

// Split a file into blocks separated by `delimiter`.
//
// A new block begins on any line whose trimmed start begins with `delimiter`. For the
// question/answer kinds (delimiter "## pergunta") we skip `// ` comment lines and only start
// collecting at the first delimiter. For servicos (delimiter "//") the delimiter lines are
// themselves part of the content, so we never skip comments and collect from the start. This
// preserves the behavior of the previous per-kind parsers.
pub fn parse_blocks(path: &str, delimiter: &str) -> Result<Vec<String>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let lines = reader.lines().collect::<std::io::Result<Vec<String>>>()?;
    Ok(parse_block_lines(lines, delimiter))
}

// Same block splitting as `parse_blocks`, but over text submitted via the web endpoint.
pub fn parse_blocks_from_text(text: &str, delimiter: &str) -> Vec<String> {
    let lines = text.lines().map(str::to_string).collect();
    parse_block_lines(lines, delimiter)
}

fn parse_block_lines(lines: Vec<String>, delimiter: &str) -> Vec<String> {
    let qa_mode = delimiter != "//"; // skip `// ` comments and gate until the first delimiter

    let mut blocks: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_entry = !qa_mode; // servicos collects immediately; q/a waits for first delimiter

    for line in lines {
        let trimmed = line.trim_start();

        if qa_mode && trimmed.starts_with("// ") {
            continue; // comment line
        }

        if trimmed.starts_with(delimiter) {
            if !current.trim().is_empty() {
                blocks.push(current.trim().to_string());
            }
            current.clear();
            in_entry = true;
        }

        if in_entry {
            current.push_str(&line);
            current.push('\n');
        }
    }

    if !current.trim().is_empty() {
        blocks.push(current.trim().to_string());
    }

    blocks
}

// Build (stored_documents, texts_to_embed) for a set of parsed blocks.
// For FullText the two are identical; for Description we store the full cleaned servico but
// embed only its short description.
pub fn prepare_documents(blocks: &[String], collection: &Collection) -> (Vec<String>, Vec<String>) {
    match collection.embed {
        EmbedStrategy::FullText => {
            let stored: Vec<String> = blocks.iter().map(|b| b.trim().to_string()).collect();
            let to_embed = stored.clone();
            (stored, to_embed)
        }
        EmbedStrategy::QuestionKey => {
            let mut stored = Vec::with_capacity(blocks.len());
            let mut to_embed = Vec::with_capacity(blocks.len());
            for block in blocks {
                stored.push(block.trim().to_string()); // payload: full Q+A block
                to_embed.push(extract_question(block)); // key: the `## pergunta` text only
            }
            (stored, to_embed)
        }
        EmbedStrategy::Description => {
            let mut stored = Vec::with_capacity(blocks.len());
            let mut to_embed = Vec::with_capacity(blocks.len());
            for block in blocks {
                let cleaned = clean_servico(block);
                let description = extract_servico_description(&cleaned);
                stored.push(cleaned);
                to_embed.push(description);
            }
            (stored, to_embed)
        }
    }
}

// Embed key for question/answer blocks: the text of the `## pergunta` field only — its lines
// up to the next `##` marker, joined to one line. A structured-field extraction (not a character
// cut), so keys stay clean and uniform. Falls back to the whole block if the marker is absent or
// the field is empty.
fn extract_question(block: &str) -> String {
    let question: String = block
        .lines()
        .skip_while(|l| !l.trim_start().starts_with("## pergunta"))
        .skip(1) // drop the "## pergunta" marker line itself
        .take_while(|l| !l.trim_start().starts_with("##"))
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ");

    if question.is_empty() {
        block.trim().to_string()
    } else {
        question
    }
}

// Drop blank lines, `//` separators and "Acessar o serviço" lines, then re-join.
fn clean_servico(servico: &str) -> String {
    let vec_lines: Vec<String> = servico
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty() && !line.starts_with("//") && !line.starts_with("Acessar o serviço"))
        .collect();
    vec_lines.join("\n").trim().to_string()
}

// First 4 lines of a cleaned servico, last line truncated to 300 chars — the text we embed.
fn extract_servico_description(cleaned: &str) -> String {
    let mut vec_lines: Vec<String> = cleaned
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty() && !line.starts_with("//"))
        .collect();

    vec_lines.truncate(4);
    if let Some(last) = vec_lines.last_mut() {
        *last = last.chars().take(300).collect();
    }
    vec_lines.join("\n")
}
