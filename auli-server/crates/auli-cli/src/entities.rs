// Multi-tenant entity registry (server side) — lê o registro ÚNICO `data/registry.toml`.
//
// A pasta `data/` vem de `AULI_DATA_DIR` (default `./data`); o server roda em `auli/`, então o
// `start_server.sh` exporta `AULI_DATA_DIR=../data`. O `registry.toml` lista as entidades; o system
// prompt de cada uma é lido de `<data>/<prompt>`. Os campos de UI do registry (uf, state,
// collections) são ignorados aqui (serde descarta chaves desconhecidas) — só interessam ao frontend.
//
// Nomes de coleção vetorial seguem `<id>-<kind>` (ex.: "rs-faqs"), isolando os vetores por entidade.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::LazyLock;

use serde::Deserialize;

/// Default entity id — the **first** entity in the registry, the same rule the frontend uses
/// (`entities[0].id` in `gen-frontend-entities.mjs`), so reordering `registry.toml` moves BOTH
/// defaults together instead of silently diverging. Falls back to `"rs"` if the registry is
/// unreadable/empty.
pub static DEFAULT_ENTITY: LazyLock<String> = LazyLock::new(|| {
    read_registry().entities.into_iter().next().map(|e| e.id).unwrap_or_else(|| "rs".to_string())
});

// Fallback system prompt used when an entity has no prompt file.
const DEFAULT_SYSTEM_PROMPT: &str = r#"
'''
### Instructions
### Responda sempre no idioma português do brasil.
### Para responder use as informações apresentadas na lista de serviços e nas perguntas frequentes (Faq) apresentados abaixo.
### Cada serviço do texto inicia com o marcador: ## servico
### Cada serviço e cada pergunta do texto inicia com o marcador: ## pergunta
### Sempre apresente os links de chamadas https
### Se a pergunta não puder ser respondida com as informações disponíveis, responda que não é possível responder
"#;

// Fallback prompt for pareceres queries (type=2) when an entity has no `prompt_pareceres` file.
const DEFAULT_PARECERES_PROMPT: &str = r#"
'''
### Instructions
### Responda sempre no idioma português do brasil, usando exclusivamente os pareceres apresentados abaixo.
### Cite o número do parecer (ex.: PARECER Nº 25148) e apresente o link https correspondente.
### Cada parecer inicia com o marcador: ## parecer; o assunto vem após ## pergunta e o conteúdo após ## resposta.
### Não responda perguntas fora do assunto relativo a tributos.
### Se a pergunta não puder ser respondida com os pareceres disponíveis, responda que não é possível responder.
"#;

#[derive(Debug, Deserialize)]
struct Registry {
    #[serde(default)]
    entities: Vec<RegistryEntity>,
}

// Only the fields the server needs; serde ignores the UI-only keys (uf/state/collections).
#[derive(Debug, Deserialize)]
struct RegistryEntity {
    id: String,
    name: String,
    #[serde(default)]
    prompt: String,
    // Optional per-entity prompt for pareceres queries (type=2); empty -> DEFAULT_PARECERES_PROMPT.
    #[serde(default)]
    prompt_pareceres: String,
}

#[derive(Debug, Clone)]
pub struct EntityConfig {
    pub id: String,
    pub name: String,
    pub system_prompt: String,
    // Prompt used for pareceres queries (type=2); the entity's `prompt_pareceres` or the default.
    pub pareceres_prompt: String,
}

impl EntityConfig {
    // kind ∈ {"servicos", "faqs", "pareceres", "notas"} -> "rs-faqs"
    pub fn collection(&self, kind: &str) -> String {
        format!("{}-{}", self.id, kind)
    }
}

/// Root of the shared `data/` dir, from `AULI_DATA_DIR` (default `./data`). Holds
/// `registry.toml`, `prompts/`, and the per-entity `<id>/packs/` — so the registry and the
/// vector packs share one root by default. `auli server --packs-dir` overrides the packs root;
/// when it's omitted the server falls back to this same dir (see `run_server`), so the two never
/// silently look in different places.
pub fn data_dir() -> PathBuf {
    std::env::var("AULI_DATA_DIR").unwrap_or_else(|_| "./data".to_string()).into()
}

pub static ENTITIES: LazyLock<HashMap<String, EntityConfig>> = LazyLock::new(load_entities);

/// Parse `registry.toml` (entities in file order). Empty on read/parse error (already logged), so
/// callers degrade gracefully. Read at startup by both `ENTITIES` and `DEFAULT_ENTITY`.
fn read_registry() -> Registry {
    let registry_path = data_dir().join("registry.toml");
    let text = match fs::read_to_string(&registry_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("⚠️  Não foi possível ler o registro de entidades {:?}: {}", registry_path, e);
            return Registry { entities: Vec::new() };
        }
    };
    toml::from_str(&text).unwrap_or_else(|e| {
        eprintln!("⚠️  registry.toml inválido ({:?}): {}", registry_path, e);
        Registry { entities: Vec::new() }
    })
}

/// Read a prompt file relative to the data dir, falling back to `default` when the path is empty
/// (entity didn't configure it) or the file can't be read (missing/unreadable — logged).
fn load_prompt(base: &std::path::Path, id: &str, rel_path: &str, kind: &str, default: &str) -> String {
    if rel_path.is_empty() {
        return default.to_string();
    }
    fs::read_to_string(base.join(rel_path)).unwrap_or_else(|_| {
        eprintln!("⚠️  prompt de {} ausente para a entidade '{}' ({}), usando o padrão.", kind, id, rel_path);
        default.to_string()
    })
}

fn load_entities() -> HashMap<String, EntityConfig> {
    let mut map = HashMap::new();
    let base = data_dir();

    for ent in read_registry().entities {
        let system_prompt = load_prompt(&base, &ent.id, &ent.prompt, "sistema", DEFAULT_SYSTEM_PROMPT);
        let pareceres_prompt =
            load_prompt(&base, &ent.id, &ent.prompt_pareceres, "pareceres", DEFAULT_PARECERES_PROMPT);
        map.insert(
            ent.id.clone(),
            EntityConfig { id: ent.id, name: ent.name, system_prompt, pareceres_prompt },
        );
    }

    map
}

// Resolve an entity id. None / empty -> DEFAULT_ENTITY. Unknown id -> Err with a message.
pub fn get_entity(id: Option<&str>) -> Result<&'static EntityConfig, String> {
    let id = id.map(str::trim).filter(|s| !s.is_empty()).unwrap_or_else(|| DEFAULT_ENTITY.as_str());

    ENTITIES
        .get(id)
        .ok_or_else(|| format!("Entidade desconhecida: '{}'. Entidades disponíveis: [{}]", id, available_ids()))
}

pub fn available_ids() -> String {
    let mut ids: Vec<&str> = ENTITIES.keys().map(String::as_str).collect();
    ids.sort_unstable();
    ids.join(", ")
}

// Force registry initialization and log the loaded entities at startup.
pub fn init() {
    println!("🏛️  Entidades carregadas: [{}]", available_ids());
}

#[cfg(test)]
mod tests {
    use super::load_prompt;

    #[test]
    fn load_prompt_falls_back_when_empty_or_missing() {
        let base = std::env::temp_dir();
        // Unconfigured (empty path) -> default.
        assert_eq!(load_prompt(&base, "rs", "", "pareceres", "DEF"), "DEF");
        // Configured but the file is missing -> default (logged).
        assert_eq!(load_prompt(&base, "rs", "prompts/does-not-exist.txt", "pareceres", "DEF"), "DEF");
    }

    #[test]
    fn load_prompt_reads_the_file_when_present() {
        let mut dir = std::env::temp_dir();
        dir.push(format!("auli_prompt_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("p.txt"), "conteúdo do prompt").unwrap();
        assert_eq!(load_prompt(&dir, "rs", "p.txt", "pareceres", "DEF"), "conteúdo do prompt");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
