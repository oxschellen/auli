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

pub const DEFAULT_ENTITY: &str = "rs";

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
}

#[derive(Debug, Clone)]
pub struct EntityConfig {
    pub id: String,
    pub name: String,
    pub system_prompt: String,
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

fn load_entities() -> HashMap<String, EntityConfig> {
    let mut map = HashMap::new();
    let base = data_dir();
    let registry_path = base.join("registry.toml");

    let text = match fs::read_to_string(&registry_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("⚠️  Não foi possível ler o registro de entidades {:?}: {}", registry_path, e);
            return map;
        }
    };
    let registry: Registry = match toml::from_str(&text) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("⚠️  registry.toml inválido ({:?}): {}", registry_path, e);
            return map;
        }
    };

    for ent in registry.entities {
        let system_prompt = if ent.prompt.is_empty() {
            DEFAULT_SYSTEM_PROMPT.to_string()
        } else {
            fs::read_to_string(base.join(&ent.prompt)).unwrap_or_else(|_| {
                eprintln!("⚠️  prompt ausente para a entidade '{}' ({}), usando o padrão.", ent.id, ent.prompt);
                DEFAULT_SYSTEM_PROMPT.to_string()
            })
        };
        map.insert(
            ent.id.clone(),
            EntityConfig { id: ent.id, name: ent.name, system_prompt },
        );
    }

    map
}

// Resolve an entity id. None / empty -> DEFAULT_ENTITY. Unknown id -> Err with a message.
pub fn get_entity(id: Option<&str>) -> Result<&'static EntityConfig, String> {
    let id = id.map(str::trim).filter(|s| !s.is_empty()).unwrap_or(DEFAULT_ENTITY);

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
