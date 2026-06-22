// Centralized configuration, loaded once from the environment (.env) at first access.
//
// Replaces the scattered `static_envs` LazyLock statics and the ad-hoc `env::var(...)` reads
// that used to live in main.rs and the embedding client. Required variables panic at load
// with a clear message; optional ones fall back to documented defaults.

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

    // Auth / JWT
    pub jwt_rsa_private_key: String,
    pub jwt_rsa_public_key: String,
    pub jwt_secret: String,
    pub jwt_access_expiration_minutes: i64,
    pub jwt_refresh_expiration_days: i64,
    pub verification_token_expiry_hours: i64,
    pub password_reset_token_expiry_hours: i64,

    // Database
    pub database_url: String,
    pub postgres_user: String,

    // Vector store (in-process; directory holding the per-collection JSON files)
    pub vector_db_path: String,
}

static CONFIG: LazyLock<Config> = LazyLock::new(Config::from_env);

/// Access the process-wide configuration (loaded on first call).
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

            jwt_rsa_private_key: req("JWT_RSA_PRIVATE_KEY"),
            jwt_rsa_public_key: req("JWT_RSA_PUBLIC_KEY"),
            jwt_secret: req("JWT_SECRET"),
            jwt_access_expiration_minutes: parse_opt("JWT_ACCESS_EXPIRATION_MINUTES", 15),
            jwt_refresh_expiration_days: parse_opt("JWT_REFRESH_EXPIRATION_DAYS", 7),
            verification_token_expiry_hours: parse_opt("VERIFICATION_TOKEN_EXPIRY_HOURS", 24),
            password_reset_token_expiry_hours: parse_opt("PASSWORD_RESET_TOKEN_EXPIRY_HOURS", 1),

            database_url: req("DATABASE_URL"),
            postgres_user: opt("POSTGRES_USER", ""),

            vector_db_path: opt("VECTOR_DB_PATH", "./vectors"),
        }
    }

    /// Print a non-secret summary at startup (mirrors the old static_env_variables_init).
    pub fn log_summary(&self) {
        println!("🔧 Configuração:");
        println!("LLM_API_URL: {}", self.llm_api_url);
        println!("LLM_API_MODEL: {}", self.llm_api_model);
        println!("EMBED_CACHE_DIR: {}", self.embed_cache_dir);
        println!("EMBED_THREADS: {}", self.embed_threads);
        println!("DATABASE_URL: {}", redact_database_url(&self.database_url));
        println!("POSTGRES_USER: {}", self.postgres_user);
        println!("VECTOR_DB_PATH: {}", self.vector_db_path);
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

fn redact_database_url(url: &str) -> String {
    let Some(scheme_end) = url.find("://") else {
        return url.to_string();
    };
    let auth_start = scheme_end + 3;
    let Some(at_offset) = url[auth_start..].find('@') else {
        return url.to_string();
    };

    let auth_end = auth_start + at_offset;
    let auth = &url[auth_start..auth_end];
    let redacted_auth = auth
        .split_once(':')
        .map(|(user, _)| format!("{user}:***"))
        .unwrap_or_else(|| "***".to_string());

    format!("{}{}{}", &url[..auth_start], redacted_auth, &url[auth_end..])
}
