// Multi-tenant entity registry.
//
// Each entity (e.g. a state tax authority) lives in its own directory under
// `./entities/<id>/` containing:
//   - entity.json   { "id": "rs", "name": "SEFAZ-RS" }
//   - prompt.txt     the LLM system instructions for that entity
//   - portal-servicos.txt / portal-faqs.txt / portal-pareceres.txt / portal-notas.txt
//
// The registry is scanned once at startup. Vector collection names are derived
// per entity as `<id>-<kind>` (e.g. "rs-faqs"), keeping each entity's vectors isolated.

use std::collections::HashMap;
use std::fs;
use std::sync::LazyLock;

use serde::Deserialize;

pub const DEFAULT_ENTITY: &str = "rs";

const ENTITIES_DIR: &str = "./entities";

// Fallback system prompt used when an entity directory has no prompt.txt.
const DEFAULT_SYSTEM_PROMPT: &str = r#"
'''
### Instructions
### Responda sempre no idioma português do brasil.
### Para responder use as informações apresentadas na lista de serviços e nas perguntas frequentes (Faq) apresentados abaixo.
### Cada serviço do texto inicia com o marcador: ## servico
### Cada serviço do texto inicia com o marcador: ## pergunta
### Sempre apresente os links de chamadas https
### Se a pergunta não puder ser respondida com as informações disponíveis, responda que não é possível responder
"#;

#[derive(Debug, Deserialize)]
struct EntityManifest {
    id: String,
    name: String,
}

#[derive(Debug, Clone)]
pub struct EntityConfig {
    pub id: String,
    pub name: String,
    pub system_prompt: String,
    pub data_dir: String,
}

impl EntityConfig {
    // kind ∈ {"services", "faqs", "pareceres", "notas"} -> "rs-faqs"
    pub fn collection(&self, kind: &str) -> String {
        format!("{}-{}", self.id, kind)
    }

    // Source file for a given kind, e.g. "./entities/rs/portal-faqs.txt"
    pub fn data_file(&self, base_name: &str) -> String {
        format!("{}/{}", self.data_dir, base_name)
    }
}

pub static ENTITIES: LazyLock<HashMap<String, EntityConfig>> = LazyLock::new(load_entities);

fn load_entities() -> HashMap<String, EntityConfig> {
    let mut map = HashMap::new();

    let entries = match fs::read_dir(ENTITIES_DIR) {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!("⚠️  Não foi possível ler o diretório de entidades '{}': {}", ENTITIES_DIR, e);
            return map;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let manifest_path = path.join("entity.json");
        let manifest_str = match fs::read_to_string(&manifest_path) {
            Ok(s) => s,
            Err(_) => continue, // not an entity directory
        };

        let manifest: EntityManifest = match serde_json::from_str(&manifest_str) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("⚠️  entity.json inválido em {:?}: {}", manifest_path, e);
                continue;
            }
        };

        let data_dir = path.to_string_lossy().to_string();

        let system_prompt = fs::read_to_string(path.join("prompt.txt")).unwrap_or_else(|_| {
            eprintln!("⚠️  prompt.txt ausente para a entidade '{}', usando o prompt padrão.", manifest.id);
            DEFAULT_SYSTEM_PROMPT.to_string()
        });

        let cfg = EntityConfig {
            id: manifest.id.clone(),
            name: manifest.name,
            system_prompt,
            data_dir,
        };

        map.insert(manifest.id, cfg);
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

fn available_ids() -> String {
    let mut ids: Vec<&str> = ENTITIES.keys().map(String::as_str).collect();
    ids.sort_unstable();
    ids.join(", ")
}

// Force registry initialization and log the loaded entities at startup.
pub fn init() {
    println!("🏛️  Entidades carregadas: [{}]", available_ids());
}
