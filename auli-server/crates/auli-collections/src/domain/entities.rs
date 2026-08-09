// Multi-tenant entity registry (scraper side) — lê o registro ÚNICO `../config/registry.toml`
// (a mesma fonte que o `auli-cli` usa). Antes havia uma cópia em `./src/entities/<id>/`; agora a
// lista de entidades vem só do registry.
//
// Layout:
//   - registry:  ../config/registry.toml  (id, name, prompt; campos de UI ignorados aqui)
//   - prompt:    ../config/<prompt>        (ex.: ../config/prompts/rs.txt)
//   - saída:     ../data/<id>/raw/        (metade GERADA pelo scraper; autorado fica em ref/)
//
// Nomes de coleção seguem `<id>-<kind>` (ex.: "rs-faqs"), isolando cada entidade.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use serde::Deserialize;

/// Default entity id — the **first** entity in the registry, the same rule the frontend and
/// `auli-cli` use, so reordering `registry.toml` moves every default together. Falls back to `"rs"`
/// if the registry is unreadable/empty.
pub static DEFAULT_ENTITY: LazyLock<String> = LazyLock::new(|| {
    read_registry()
        .entities
        .into_iter()
        .next()
        .map(|e| e.id)
        .unwrap_or_else(|| "rs".to_string())
});

// Repo-root data/ (relativo ao CWD do collections).
const DATA_DIR: &str = "../data";

// Fallback system prompt used when an entity has no prompt file.
const DEFAULT_SYSTEM_PROMPT: &str = r#"
'''
### Instructions
### Responda sempre no idioma português do brasil.
### Para responder use as informações apresentadas na lista de serviços e nas perguntas frequentes (Faq) apresentados abaixo.
### Cada documento do contexto inicia com o marcador: ## documento (seguido do número e do tipo — Serviço ou FAQ).
### Dentro de cada documento, o título vem após o marcador ## pergunta e o conteúdo após o marcador ## resposta.
### Sempre apresente os links de chamadas https
### Se a pergunta não puder ser respondida com as informações disponíveis, responda que não é possível responder
"#;

#[derive(Debug, Deserialize)]
struct Registry {
    #[serde(default)]
    entities: Vec<RegistryEntity>,
}

// Só os campos que o scraper precisa; serde ignora as chaves de UI (uf/state/collections).
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
    // Where this entity's generated outputs live, e.g. "../data/rs/raw".
    pub data_dir: String,
}

impl EntityConfig {
    // kind ∈ {"servicos", "faqs", "pareceres", "notas"} -> "rs-faqs"
    pub fn collection(&self, kind: &str) -> String {
        format!("{}-{}", self.id, kind)
    }

    /// Caminho de um prompt de PASSO (sinopse, extração) — os que não pertencem a uma entidade e
    /// sim ao pipeline.
    ///
    /// O `data_dir` da entidade é `<raiz>/<id>/raw`, então a raiz de dados está dois níveis acima;
    /// de lá, o `auli-contract` resolve a `config/` irmã. Existe aqui, e num lugar só, porque o
    /// `sinopse` e o `extracao` faziam esta subida duplicada, com um comentário em um deles dizendo
    /// "mesmo cálculo do outro" — que é como dois caminhos divergem.
    pub fn prompt_de_passo(&self, arquivo: &str) -> Result<PathBuf, String> {
        let raiz = Path::new(&self.data_dir)
            .parent()
            .and_then(Path::parent)
            .ok_or_else(|| format!("data_dir inesperado: {}", self.data_dir))?;
        Ok(auli_contract::config::prompt_path(
            raiz,
            &format!("prompts/{arquivo}"),
        ))
    }

    // Source file for a given kind, e.g. "../data/rs/raw/portal-faqs.txt"
    pub fn data_file(&self, base_name: &str) -> String {
        format!("{}/{}", self.data_dir, base_name)
    }
}

pub static ENTITIES: LazyLock<HashMap<String, EntityConfig>> = LazyLock::new(load_entities);

/// Parse `../config/registry.toml` (entities in file order). Empty on read/parse error (already
/// logged). Read at startup by both `ENTITIES` and `DEFAULT_ENTITY`.
fn read_registry() -> Registry {
    // Guarda de migração: resíduo do layout antigo é erro, nunca fallback silencioso (D-CFG-6).
    if let Err(e) = auli_contract::config::recusar_layout_antigo(Path::new(DATA_DIR)) {
        eprintln!("❌ {e}");
        return Registry {
            entities: Vec::new(),
        };
    }
    let registry_path = auli_contract::config::registry_path(Path::new(DATA_DIR))
        .display()
        .to_string();
    let text = match fs::read_to_string(&registry_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!(
                "⚠️  Não foi possível ler o registro de entidades '{}': {}",
                registry_path, e
            );
            return Registry {
                entities: Vec::new(),
            };
        }
    };
    toml::from_str(&text).unwrap_or_else(|e| {
        eprintln!("⚠️  registry.toml inválido ('{}'): {}", registry_path, e);
        Registry {
            entities: Vec::new(),
        }
    })
}

fn load_entities() -> HashMap<String, EntityConfig> {
    let mut map = HashMap::new();

    for ent in read_registry().entities {
        let system_prompt = if ent.prompt.is_empty() {
            DEFAULT_SYSTEM_PROMPT.to_string()
        } else {
            fs::read_to_string(auli_contract::config::prompt_path(
                Path::new(DATA_DIR),
                &ent.prompt,
            ))
            .unwrap_or_else(|_| {
                eprintln!(
                    "⚠️  prompt ausente para '{}' ({}), usando o padrão.",
                    ent.id, ent.prompt
                );
                DEFAULT_SYSTEM_PROMPT.to_string()
            })
        };
        let data_dir = format!("{}/{}/raw", DATA_DIR, ent.id);
        map.insert(
            ent.id.clone(),
            EntityConfig {
                id: ent.id,
                name: ent.name,
                system_prompt,
                data_dir,
            },
        );
    }

    map
}

// Resolve an entity id. None / empty -> DEFAULT_ENTITY. Unknown id -> Err with a message.
pub fn get_entity(id: Option<&str>) -> Result<&'static EntityConfig, String> {
    let id = id
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_ENTITY.as_str());

    ENTITIES.get(id).ok_or_else(|| {
        format!(
            "Entidade desconhecida: '{}'. Entidades disponíveis: [{}]",
            id,
            available_ids()
        )
    })
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
