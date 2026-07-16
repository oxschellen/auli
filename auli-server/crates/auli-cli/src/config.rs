// Centralized configuration for the SERVER mode, loaded once from the environment (.env) at first
// access. Required variables panic at load with a clear message; optional ones fall back to
// documented defaults.
//
// `auli update` does NOT use this (it would force the LLM vars that ingestion doesn't need);
// it reads only the embedder settings directly from the environment. See `update.rs`.

use std::sync::LazyLock;

use dotenvy::dotenv;

pub struct Config {
    // External LLM (Groq-compatible chat completions)
    pub llm_api_url: String,
    pub llm_api_key: String,
    pub llm_api_model: String,

    // Local embeddings (fastembed / BGE-M3, in-process)
    pub embed_cache_dir: String,
    pub embed_threads: usize,

    // Anonimizar a pergunta ANTES de enviá-la ao LLM externo (e restaurar a resposta).
    // Default `true`: só desliga com valor explicitamente negativo. Ver `rag::exec_all_question`.
    pub anonimizar_llm: bool,
}

static CONFIG: LazyLock<Config> = LazyLock::new(Config::from_env);

/// Access the process-wide SERVER configuration (loaded on first call).
pub fn config() -> &'static Config {
    &CONFIG
}

impl Config {
    fn from_env() -> Self {
        dotenv().ok();
        Config {
            llm_api_url: req("LLM_API_URL"),
            llm_api_key: req("LLM_API_KEY"),
            llm_api_model: req("LLM_API_MODEL"),

            embed_cache_dir: opt("EMBED_CACHE_DIR", "./models"),
            embed_threads: parse_opt("EMBED_THREADS", 16),

            anonimizar_llm: opt_bool("AULI_ANONIMIZAR_LLM", true),
        }
    }

    /// Print a non-secret summary at startup.
    pub fn log_summary(&self) {
        println!("🔧 Configuração:");
        println!("LLM_API_URL: {}", self.llm_api_url);
        println!("LLM_API_MODEL: {}", self.llm_api_model);
        println!("EMBED_CACHE_DIR: {}", self.embed_cache_dir);
        println!("EMBED_THREADS: {}", self.embed_threads);
        println!("AULI_ANONIMIZAR_LLM: {}", self.anonimizar_llm);
    }
}

fn req(key: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| panic!("Variável de ambiente obrigatória ausente: {key}"))
}

fn opt(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

// Optional env var parsed to any `FromStr` type; falls back to `default` if unset or unparsable.
fn parse_opt<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

// Optional boolean env var. Recognized truthy/falsy values win; anything else (unset or
// unrecognized) falls back to `default` — so a typo can't silently flip a privacy-default-on flag.
fn opt_bool(key: &str, default: bool) -> bool {
    match std::env::var(key).ok().map(|v| v.trim().to_ascii_lowercase()) {
        Some(v) if matches!(v.as_str(), "true" | "1" | "yes" | "on") => true,
        Some(v) if matches!(v.as_str(), "false" | "0" | "no" | "off") => false,
        _ => default,
    }
}
