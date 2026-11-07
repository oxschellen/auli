// auli-scraper-rs — scraper da SEFAZ-RS (faqs + serviços).
//
// Conhece UMA entidade ("rs"); não lê o registry.toml (isso é assunto do collections/engine).
// Grava os snapshots por coleção (v3: faqs e serviços em arquivos separados); a derivação dos
// artefatos é o `auli-collections rs`.

mod errors;
mod f5;
mod faqs;
mod pareceres;
mod servicos;
mod tarf;

/// A entidade que este scraper conhece (D-F2.1 — um crate binário por entidade).
pub const ENTITY: &str = "rs";
/// Onde as saídas geradas vivem (o snapshot é gravado em `../data/rs/`, irmão de `raw/`).
const DATA_DIR: &str = "../data/rs/raw";

/// Identidade deste scraper, gravada como metadado no snapshot. Compartilhada pelas coleções
/// `servicos` e `faqs` (o `env!` avalia neste crate).
pub(crate) fn scraper_info() -> auli_contract::ScraperInfo {
    auli_contract::ScraperInfo::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // CLI: auli-scraper-rs [--usecache] faqs|servicos|all|pareceres|tarf   (omitido -> all)
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let use_cache = raw.iter().any(|a| a == "--usecache");
    let cmd = raw
        .iter()
        .find(|a| !a.starts_with("--"))
        .map(String::as_str)
        .unwrap_or("all");

    println!("🏛️  Scraper RS (SEFAZ-RS) — coleção: {}", cmd);
    if use_cache {
        println!("📦 --usecache: usando apenas páginas em cache (sem rede).");
    }

    match cmd {
        "faqs" => run_faqs(use_cache)?,
        "servicos" => servicos::run(DATA_DIR, use_cache)?,
        "all" => {
            run_faqs(use_cache)?;
            servicos::run(DATA_DIR, use_cache)?;
        }
        // Coleta os pareceres para o intermediário `ref/rs-pareceres-temp.txt` (sem snapshot ainda —
        // o estágio de resumo autorado é posterior). Retorna sem a mensagem de snapshot abaixo.
        "pareceres" => {
            pareceres::run(use_cache)?;
            return Ok(());
        }
        // Materializa a árvore `../data/rs/docs/tarf/` (acórdãos do TARF). Como `pareceres`, é
        // opt-in explícito — não entra no `all` — e não gera snapshot: retorna antes da mensagem.
        "tarf" => {
            tarf::run(use_cache)?;
            return Ok(());
        }
        other => {
            return Err(format!(
                "coleção desconhecida: '{}'. Use: faqs | servicos | all | pareceres | tarf",
                other
            )
            .into());
        }
    }

    println!(
        "✅ Snapshot atualizado. Rode `auli-collections {}` para derivar os artefatos.",
        ENTITY
    );
    Ok(())
}

fn run_faqs(use_cache: bool) -> errors::Result<()> {
    faqs::run(&faqs::FaqSource {
        id: ENTITY.to_string(),
        base_url: "https://atendimento.receita.rs.gov.br".to_string(),
        // Desde nov/2026 `/perguntas-frequentes` é um índice novo (sem `data-matriz-source-uri`, sem
        // painéis: ~1.850 perguntas inline em ~230 páginas) — a caminhada o classifica como `Geral` e
        // colhe zero. O portal preservou a raiz `categoriafaq` em `-antigo`; as páginas-filhas não
        // mudaram de URL. Se o `-antigo` sumir, a guarda de zero FAQs em `faqs::run` é quem avisa.
        root_url: "https://atendimento.receita.rs.gov.br/perguntas-frequentes-antigo".to_string(),
        root_title: "Perguntas Frequentes".to_string(),
        data_dir: DATA_DIR.to_string(),
        cache_dir: format!("{}/cache/faqs", DATA_DIR),
        use_cache,
        // D-RSFAQ-7: os itens de menu que redirecionam para um PDF de perguntas. Ficam fora o "Manual para Declaração
        // de ITCD" (manual, decisão do mantenedor), o `/nfcom-faq` (o mesmo PDF do `/faq-nfcom`, no menu GES
        // Comunicações) e as orientações de transporte internacional (não são perguntas).
        pdfs: [
            "faq-nfcom",                       // NFCom (GES Comunicações, 11/2025)
            "faq-ges-bebidas",                 // GES Bebidas (07/2026)
            "duvidas-gerais-ges-com",          // GES Comunicações (07/2022)
            "convenio-icms-201-17",            // Convênio ICMS 201/17 (2023)
            "programas-de-autorregularizacao", // Autorregularização (09/2022)
        ]
        .iter()
        .map(|p| format!("https://atendimento.receita.rs.gov.br/{p}"))
        .collect(),
    })
}
