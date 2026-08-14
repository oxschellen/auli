// Multi-tenant entity registry (server side) — lê o registro ÚNICO `config/registry.toml`.
//
// O catálogo vive em `config/`, irmã da raiz de dados; quem resolve o caminho é o
// `auli_contract::config`, e ninguém mais (ver o doc daquele módulo). O `registry.toml` lista as
// entidades; o system prompt de cada uma é lido de `<config>/<prompt>`. Os campos de UI do registry
// (uf, state, collections) são ignorados aqui (serde descarta chaves desconhecidas) — só interessam
// ao frontend.
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
    read_registry()
        .entities
        .into_iter()
        .next()
        .map(|e| e.id)
        .unwrap_or_else(|| "rs".to_string())
});

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

// Fallback prompt for pareceres queries (type=2) when an entity has no `prompt_pareceres` file.
const DEFAULT_PARECERES_PROMPT: &str = r#"
'''
### Instructions
### Responda sempre no idioma português do brasil, usando exclusivamente os pareceres apresentados abaixo.
### Cite o número do parecer (ex.: PARECER Nº 25148) e apresente o link https correspondente.
### Cada parecer inicia com o marcador: ## documento (seguido do número e do tipo — Parecer ou Parecer relacionado); o assunto vem após ## pergunta e o conteúdo após ## resposta.
### Não responda perguntas fora do assunto relativo a tributos.
### Se a pergunta não puder ser respondida com os pareceres disponíveis, responda que não é possível responder.
"#;

// Fallback prompt for legislação queries (type=4) when an entity has no `prompt_legislacao` file.
//
// O fallback é GLOBAL, e não o `prompt_pareceres` da entidade (como faz o TARF): acórdão sem prompt
// próprio ainda é jurisprudência e as instruções de citar número e link continuam certas, mas
// legislação não é jurisprudência nenhuma — cair no prompt de pareceres mandaria o modelo citar
// "o número do parecer" sobre um texto de lei.
const DEFAULT_LEGISLACAO_PROMPT: &str = r#"
'''
### Instructions
### Responda sempre no idioma português do brasil, usando exclusivamente os documentos de legislação apresentados abaixo.
### Cite sempre o dispositivo (ex.: art. 28; art. 11, § 3º) e a lei a que ele pertence, e apresente o link https correspondente.
### Cada documento inicia com o marcador: ## documento (seguido do número e do tipo — Legislação); a hierarquia da lei, a pergunta e o dispositivo vêm após ## pergunta, e a resposta após ## resposta.
### NÃO extrapole o texto legal: responda o que o dispositivo diz, sem completar com prática administrativa, jurisprudência ou analogia.
### Reproduza prazos, percentuais e valores exatamente como estão no documento.
### Não responda perguntas fora do assunto relativo a tributos.
### Se a pergunta não puder ser respondida com os documentos disponíveis, responda que não é possível responder.
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
    // Optional per-entity prompt for TARF queries (type=3); empty -> DEFAULT_PARECERES_PROMPT. O
    // fallback é o de pareceres, e não um terceiro texto: sem prompt próprio, um acórdão ainda é
    // jurisprudência, e as instruções de citar o número e o link valem igual.
    #[serde(default)]
    prompt_tarf: String,
    // Optional per-entity prompt for legislação queries (type=4); empty -> DEFAULT_LEGISLACAO_PROMPT
    // (global, não o de pareceres da entidade — ver o doc da const).
    #[serde(default)]
    prompt_legislacao: String,
}

#[derive(Debug, Clone)]
pub struct EntityConfig {
    pub id: String,
    pub name: String,
    pub system_prompt: String,
    // Prompt used for pareceres queries (type=2); the entity's `prompt_pareceres` or the default.
    pub pareceres_prompt: String,
    // Prompt used for TARF queries (type=3); the entity's `prompt_tarf` or the pareceres default.
    pub tarf_prompt: String,
    // Prompt used for legislação queries (type=4); the entity's `prompt_legislacao` or the global
    // legislação default.
    pub legislacao_prompt: String,
}

impl EntityConfig {
    // kind ∈ {"servicos", "faqs", "pareceres", "tarf", "notas"} -> "rs-faqs"
    pub fn collection(&self, kind: &str) -> String {
        format!("{}-{}", self.id, kind)
    }

    /// O prompt de sistema de UMA coleção consultada sozinha — jurisprudência ou legislação.
    ///
    /// Serviços e FAQs não passam por aqui: eles são consultados JUNTOS e compartilham o
    /// `system_prompt`, que é o prompt do chat de atendimento. Se um dia passarem, cairiam no
    /// prompt de pareceres, que é errado; daí o `debug_assert`, que faz o engano aparecer em teste
    /// em vez de na resposta. (Antes o assert era `exige_resumo()`, que era o mesmo teste enquanto
    /// jurisprudência era a única coleção solitária — a legislação separou as duas ideias.)
    pub fn prompt_de(&self, kind: auli_contract::Kind) -> &str {
        use auli_contract::Kind;
        debug_assert!(
            !matches!(kind, Kind::Servicos | Kind::Faqs),
            "prompt_de é só para coleção consultada sozinha; servicos/faqs usam o system_prompt"
        );
        match kind {
            Kind::Tarf => &self.tarf_prompt,
            Kind::Legislacao => &self.legislacao_prompt,
            _ => &self.pareceres_prompt,
        }
    }
}

/// Root of the shared `data/` dir, from `AULI_DATA_DIR` (default `./data`). Holds the per-entity
/// `<id>/{docs,packs}/`. O CATÁLOGO (`registry.toml`, `prompts/`) NÃO mora aqui: vive na `config/`
/// irmã, resolvida por `auli_contract::config` — daí esta função continuar sendo só a raiz de dado.
/// `auli server --packs-dir` overrides the packs root; when it's omitted the server falls back to
/// this same dir (see `run_server`), so the two never silently look in different places.
pub fn data_dir() -> PathBuf {
    std::env::var("AULI_DATA_DIR")
        .unwrap_or_else(|_| "./data".to_string())
        .into()
}

pub static ENTITIES: LazyLock<HashMap<String, EntityConfig>> = LazyLock::new(load_entities);

/// Parse `registry.toml` (entities in file order). Empty on read/parse error (already logged), so
/// callers degrade gracefully. Read at startup by both `ENTITIES` and `DEFAULT_ENTITY`.
fn read_registry() -> Registry {
    // Guarda de migração: resíduo do layout antigo é erro, nunca fallback silencioso (D-CFG-6).
    if let Err(e) = auli_contract::config::recusar_layout_antigo(&data_dir()) {
        eprintln!("❌ {e}");
        return Registry {
            entities: Vec::new(),
        };
    }
    let registry_path = auli_contract::config::registry_path(&data_dir());
    let text = match fs::read_to_string(&registry_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!(
                "⚠️  Não foi possível ler o registro de entidades {:?}: {}",
                registry_path, e
            );
            return Registry {
                entities: Vec::new(),
            };
        }
    };
    toml::from_str(&text).unwrap_or_else(|e| {
        eprintln!("⚠️  registry.toml inválido ({:?}): {}", registry_path, e);
        Registry {
            entities: Vec::new(),
        }
    })
}

/// Read a prompt file relative to the CONFIG dir (the `base` given), falling back to `default` when
/// the path is empty (entity didn't configure it) or the file can't be read (missing/unreadable —
/// logged).
fn load_prompt(
    base: &std::path::Path,
    id: &str,
    rel_path: &str,
    kind: &str,
    default: &str,
) -> String {
    if rel_path.is_empty() {
        return default.to_string();
    }
    fs::read_to_string(base.join(rel_path)).unwrap_or_else(|_| {
        eprintln!(
            "⚠️  prompt de {} ausente para a entidade '{}' ({}), usando o padrão.",
            kind, id, rel_path
        );
        default.to_string()
    })
}

fn load_entities() -> HashMap<String, EntityConfig> {
    let mut map = HashMap::new();
    // Os caminhos de prompt do registry (`prompts/rs.txt`) são relativos à pasta DO REGISTRY, e
    // sempre foram — por isso a migração não tocou no conteúdo dele.
    let base = auli_contract::config::config_dir(&data_dir());

    for ent in read_registry().entities {
        let system_prompt = load_prompt(
            &base,
            &ent.id,
            &ent.prompt,
            "sistema",
            DEFAULT_SYSTEM_PROMPT,
        );
        let pareceres_prompt = load_prompt(
            &base,
            &ent.id,
            &ent.prompt_pareceres,
            "pareceres",
            DEFAULT_PARECERES_PROMPT,
        );
        // Sem `prompt_tarf`, cai no prompt de PARECERES da própria entidade (não no default global):
        // ele já fala a língua do acervo daquela UF, e é a aproximação mais útil disponível.
        let tarf_prompt = load_prompt(&base, &ent.id, &ent.prompt_tarf, "tarf", &pareceres_prompt);
        // Legislação cai no default GLOBAL, não no prompt de pareceres da entidade — ver o doc de
        // `DEFAULT_LEGISLACAO_PROMPT`.
        let legislacao_prompt = load_prompt(
            &base,
            &ent.id,
            &ent.prompt_legislacao,
            "legislacao",
            DEFAULT_LEGISLACAO_PROMPT,
        );
        map.insert(
            ent.id.clone(),
            EntityConfig {
                id: ent.id,
                name: ent.name,
                system_prompt,
                pareceres_prompt,
                tarf_prompt,
                legislacao_prompt,
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
    use super::{DEFAULT_LEGISLACAO_PROMPT, DEFAULT_PARECERES_PROMPT, EntityConfig, load_prompt};
    use auli_contract::Kind;

    fn cfg_de(legislacao_prompt: &str) -> EntityConfig {
        EntityConfig {
            id: "xx".into(),
            name: "XX".into(),
            system_prompt: "SISTEMA".into(),
            pareceres_prompt: "PARECERES".into(),
            tarf_prompt: "TARF".into(),
            legislacao_prompt: legislacao_prompt.into(),
        }
    }

    #[test]
    fn prompt_de_serve_um_texto_por_colecao_solitaria() {
        let cfg = cfg_de("LEGISLACAO");
        assert_eq!(cfg.prompt_de(Kind::Pareceres), "PARECERES");
        assert_eq!(cfg.prompt_de(Kind::Tarf), "TARF");
        // O que este teste impede: legislação caindo no prompt de pareceres, que mandaria o modelo
        // citar "o número do parecer" sobre um texto de lei — resposta plausível e errada.
        assert_eq!(cfg.prompt_de(Kind::Legislacao), "LEGISLACAO");
    }

    /// O fallback da legislação é o default GLOBAL, e não o `prompt_pareceres` da entidade (que é o
    /// do TARF). Entidade sem prompt próprio é o caso das outras 26 no dia em que ganharem a
    /// coleção, então o default é o que vai ao LLM na estreia — e ele precisa falar de dispositivo.
    #[test]
    fn sem_prompt_proprio_a_legislacao_cai_no_default_global() {
        let base = std::env::temp_dir();
        let escolhido = load_prompt(&base, "xx", "", "legislacao", DEFAULT_LEGISLACAO_PROMPT);
        assert_eq!(escolhido, DEFAULT_LEGISLACAO_PROMPT);
        assert_ne!(escolhido, DEFAULT_PARECERES_PROMPT);
        // E o default diz o que a coleção exige: dispositivo e link.
        assert!(escolhido.contains("dispositivo"), "{escolhido}");
        assert!(escolhido.contains("link"), "{escolhido}");
        assert!(!escolhido.contains("parecer"), "{escolhido}");
    }

    #[test]
    fn load_prompt_falls_back_when_empty_or_missing() {
        let base = std::env::temp_dir();
        // Unconfigured (empty path) -> default.
        assert_eq!(load_prompt(&base, "rs", "", "pareceres", "DEF"), "DEF");
        // Configured but the file is missing -> default (logged).
        assert_eq!(
            load_prompt(
                &base,
                "rs",
                "prompts/does-not-exist.txt",
                "pareceres",
                "DEF"
            ),
            "DEF"
        );
    }

    #[test]
    fn load_prompt_reads_the_file_when_present() {
        let mut dir = std::env::temp_dir();
        dir.push(format!("auli_prompt_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("p.txt"), "conteúdo do prompt").unwrap();
        assert_eq!(
            load_prompt(&dir, "rs", "p.txt", "pareceres", "DEF"),
            "conteúdo do prompt"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
