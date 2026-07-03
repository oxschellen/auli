//! The `auli` binary — a thin clap dispatcher over two modes, `server` and `update`.
//!
//! Subcommands (not flags) so each mode has its own exclusive options:
//!   auli server  --port <p> [--bind <addr>] [--packs-dir <dir>]   (--packs-dir defaults to $AULI_DATA_DIR or ./data)
//!   auli update  --entity <id> --source <dir_com_contrato_json> --out <dir> [--version <v>]

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "auli", version, about = "Auli — assistente RAG tributário (server + update)")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Sobe a API (axum) servindo os pacotes de vetores (somente leitura).
    Server {
        #[arg(long, default_value_t = 3000)]
        port: u16,
        /// Endereço de escuta. Default `0.0.0.0` (todas as interfaces — instância única).
        /// Em multi-instância atrás de reverse proxy local, use `127.0.0.1`.
        #[arg(long, default_value = "0.0.0.0")]
        bind: String,
        /// Raiz dos pacotes (layout `<dir>/<id>/packs/`). Omitido ⇒ usa `AULI_DATA_DIR` (default `./data`).
        #[arg(long)]
        packs_dir: Option<String>,
    },
    /// Vetoriza o contrato tipado (`auli_contract::Table<P>`) de uma entidade em pacotes `<id>-<kind>.json` + manifesto.
    Update {
        #[arg(long)]
        entity: String,
        #[arg(long)]
        source: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        version: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Server { port, bind, packs_dir } => {
            auli_cli::run_server(packs_dir, port, bind).await;
        }
        Command::Update { entity, source, out, version } => {
            // Synchronous, CPU-bound work — runs to completion on the async entrypoint thread.
            if let Err(e) = auli_cli::run_update(entity, source, out, version) {
                eprintln!("Erro no update: {e}");
                std::process::exit(1);
            }
        }
    }
}
