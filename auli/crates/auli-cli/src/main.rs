//! The `auli` binary — a thin clap dispatcher over two modes, `server` and `update`.
//!
//! Subcommands (not flags) so each mode has its own exclusive options:
//!   auli server  --port <p> --packs-dir <dir>
//!   auli update  --entity <id> --source <dir_com_portal_txt> --out <dir> [--version <v>]

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
        #[arg(long, default_value = "./vectors")]
        packs_dir: String,
    },
    /// Vetoriza os `portal-*.txt` de uma entidade em pacotes `<id>-<kind>.json` + manifesto.
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
        Command::Server { port, packs_dir } => {
            auli_cli::run_server(packs_dir, port).await;
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
