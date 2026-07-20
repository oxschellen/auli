mod derive_faqs;
mod derive_pareceres;
mod domain;
mod errors;
mod process;
mod servicos;
mod sinopse;

use domain::entities::get_entity;
use sinopse::SinopseOpts;

fn main() -> errors::Result<()> {
    // CLI: `auli-collections <entity> [<subcomando>] [flags]`
    //   <entity>   entity id (ex.: `rs`); vazio/omitido -> entidade padrão.
    //   process    (padrão) deriva os artefatos do snapshot, offline.
    //   pareceres  bootstrap: ingere o `.txt` legado -> árvore `docs/pareceres/*.md`.
    //   sinopse    gera/mescla sinopses (aceita flags próprias; ver `sinopse::run`).
    // Só o `sinopse` aceita flags; os demais subcomandos continuam rejeitando (comportamento atual).
    let args: Vec<String> = std::env::args().skip(1).collect();

    // Posicionais vêm antes de qualquer flag (`<entity> <subcomando> [--flags]`).
    let mut positional = Vec::new();
    let mut rest = args.iter();
    for a in rest.by_ref() {
        if a.starts_with("--") {
            let mut flags = vec![a.clone()];
            flags.extend(rest.cloned());
            return dispatch(positional, flags);
        }
        positional.push(a.clone());
    }
    dispatch(positional, Vec::new())
}

fn dispatch(positional: Vec<String>, flags: Vec<String>) -> errors::Result<()> {
    let entity_arg = positional.first().cloned();
    let collection = positional.get(1).cloned().unwrap_or_else(|| "process".to_string());

    let entity = get_entity(entity_arg.as_deref())?;
    println!("🏛️  Entidade: {} ({})", entity.id, entity.name);

    // Flags só são aceitas pelo `sinopse`; para os demais, o erro atual permanece.
    if collection != "sinopse"
        && let Some(flag) = flags.first()
    {
        return Err(format!("flag desconhecida: '{}'. O auli-collections não aceita flags.", flag).into());
    }

    match collection.as_str() {
        // OFFLINE: deriva contrato, prints, index e per-público do snapshot já gravado.
        "process" => process::run(entity)?,
        // OFFLINE (bootstrap): ingere o `.txt` legado de `ref/` -> árvore `docs/pareceres/*.md`;
        // rode `sinopse` em seguida. Os scrapers já emitem `.md` direto (G5).
        "pareceres" => derive_pareceres::run(entity)?,
        // OFFLINE: preenche as sinopses pendentes na árvore `docs/pareceres/*.md` (G4).
        "sinopse" => sinopse::run(entity, parse_sinopse_flags(&flags)?)?,
        "faqs" | "servicos" => {
            return Err(
                "a coleta agora é feita pelos binários `auli-scraper-rs` / `auli-scraper-sc`; \
                 o auli-collections só deriva (rode `auli-collections <entity>`)"
                    .into(),
            );
        }
        other => {
            return Err(format!(
                "subcomando desconhecido: '{}'. Use: process (padrão) | pareceres | sinopse",
                other
            )
            .into());
        }
    }

    Ok(())
}

/// Parsing manual das flags do `sinopse` (estilo da casa — a collections não usa clap).
/// `--fake` é dev-only: reconhecida, mas não listada na mensagem de uso.
fn parse_sinopse_flags(flags: &[String]) -> errors::Result<SinopseOpts> {
    let mut opts = SinopseOpts { dry_run: false, limit: None, force: None, fake: false };
    let mut it = flags.iter();
    while let Some(f) = it.next() {
        match f.as_str() {
            "--dry-run" => opts.dry_run = true,
            "--fake" => opts.fake = true,
            "--limit" => {
                let v = it.next().ok_or("--limit exige um valor inteiro > 0")?;
                let n: usize = v.parse().map_err(|_| format!("--limit inválido: {v:?} (inteiro > 0)"))?;
                if n == 0 {
                    return Err("--limit deve ser > 0".into());
                }
                opts.limit = Some(n);
            }
            "--force" => {
                let v = it
                    .next()
                    .ok_or("--force exige o <numero> exato (ex.: \"PARECER Nº 25148\")")?;
                opts.force = Some(v.clone());
            }
            other => {
                return Err(format!(
                    "flag desconhecida: {other:?}. Válidas: --dry-run, --limit N, --force <numero>"
                )
                .into());
            }
        }
    }
    Ok(opts)
}
