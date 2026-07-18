//! Subcomando `sinopse` — esqueleto offline (F3). Toda a mecânica do passo `auli-sinopse`
//! (§6 do plano) **sem nenhuma chamada de LLM**: carga do `.raw.json`, mescla por `numero`
//! (retomável/incremental), contagem, `--dry-run`, geração `--fake` (dev-only) e gravação atômica.
//! A geração real (prompt, validação, env `SINOPSE_*`) chega na F4.
//!
//! Fronteira = snapshot tipado `Table<Consulta>` (o `.txt` é só print legível — F5). O `resumo`
//! vazio marca "pendente"; preenchido marca "reaproveitado" (sinopse já gerada ou sumário autorado
//! legado). Invariante do passo: `reaproveitados + gerados + falhas + pendentes_restantes == total`.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use auli_contract::{Consulta, Table};

use crate::domain::entities::EntityConfig;
use crate::errors::Result;

/// Opções do subcomando (parseadas no `main`).
pub struct SinopseOpts {
    pub dry_run: bool,
    pub limit: Option<usize>,
    /// `numero` a regenerar mesmo com `resumo` preenchido.
    pub force: Option<String>,
    /// Dev-only: gera placeholder determinístico em vez de LLM.
    pub fake: bool,
}

/// Único ponto que compõe o `text_to_embed` de consultas (decisão §2.5 do plano). Reproduz
/// **exatamente** o formato hoje gerado pelo `derive_pareceres` (`assunto` + `resumo` como key de
/// busca, com fallbacks): ambos → `"{assunto}\n{resumo}"`; só um → esse; nenhum → vazio (o
/// fallback para `numero` do derive não é alcançável aqui, pois a sinopse sempre preenche `resumo`).
pub fn compose_text_to_embed(assunto: &str, resumo: &str) -> String {
    match (assunto.is_empty(), resumo.is_empty()) {
        (false, false) => format!("{assunto}\n{resumo}"),
        (false, true) => assunto.to_string(),
        (true, false) => resumo.to_string(),
        (true, true) => String::new(),
    }
}

/// Timestamp atual em ISO-8601 UTC (`YYYY-MM-DDTHH:MM:SSZ`), só com `std` (sem chrono — F3 não
/// acrescenta dependências). Algoritmo civil-from-days de Howard Hinnant.
fn now_iso8601() -> String {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64;
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let (h, mi, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    // civil_from_days: dias desde 1970-01-01 -> (ano, mês, dia).
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

/// Caminho do snapshot bruto (entrada) e da saída (`.raw.json` → `.json`).
fn raw_path(entity: &EntityConfig) -> PathBuf {
    PathBuf::from(&entity.data_dir).join(format!("{}-pareceres.raw.json", entity.id))
}
fn out_path(entity: &EntityConfig) -> PathBuf {
    PathBuf::from(&entity.data_dir).join(format!("{}-pareceres.json", entity.id))
}

/// `true` se o registro precisa de sinopse (resumo vazio, ignorando espaços).
fn pendente(c: &Consulta) -> bool {
    c.resumo.trim().is_empty()
}

/// Serializa a `Table` inteira para `<out>.tmp` e faz `rename` atômico sobre `<out>` (mesmo dir).
/// Queda em qualquer ponto perde no máximo o documento em voo.
fn write_atomic(out: &Path, id: &str, items: &[Consulta]) -> Result<()> {
    let table = Table::new(id, "pareceres", items.to_vec());
    let tmp = PathBuf::from(format!("{}.tmp", out.display()));
    std::fs::write(&tmp, serde_json::to_string_pretty(&table)?)?;
    std::fs::rename(&tmp, out)?;
    Ok(())
}

/// Mescla o `.raw.json` (ordem = fonte da verdade) com a saída anterior, por `numero`. Registros
/// pendentes cujo anterior já tem `resumo` são reaproveitados inteiros; `--force` re-pendura o alvo.
fn merge(raw: Vec<Consulta>, prev: &HashMap<String, Consulta>, force: Option<&str>) -> Result<Vec<Consulta>> {
    let mut seen = HashSet::new();
    let mut merged = Vec::with_capacity(raw.len());
    let mut force_hit = false;
    for rc in raw {
        if !seen.insert(rc.numero.clone()) {
            return Err(format!("numero duplicado no raw: {:?} — viola a identidade da listagem.", rc.numero).into());
        }
        if force == Some(rc.numero.as_str()) {
            // Re-pendura: zera resumo/sinopse/key para forçar regeneração.
            force_hit = true;
            merged.push(Consulta { resumo: String::new(), text_to_embed: String::new(), sinopse_info: None, ..rc });
            continue;
        }
        // Reaproveita a saída anterior só quando o bruto está pendente e o anterior já tem resumo.
        if pendente(&rc)
            && let Some(prev_c) = prev.get(&rc.numero)
            && !pendente(prev_c)
        {
            merged.push(prev_c.clone());
            continue;
        }
        merged.push(rc);
    }
    if let Some(n) = force
        && !force_hit
    {
        return Err(format!("--force: numero inexistente no raw: {n:?}").into());
    }
    Ok(merged)
}

/// Placeholder determinístico (dev-only) — a F4 troca por LLM real. `prompt_versao: 0` marca fake.
fn fake_resumo(assunto: &str) -> String {
    format!("### Descrição Resumida do Assunto\n[FAKE] {assunto}\n\n### Palavras Chave do Tema\n- **fake**")
}

pub fn run(entity: &EntityConfig, opts: SinopseOpts) -> Result<()> {
    // 1. Carga do snapshot bruto.
    let raw_file = raw_path(entity);
    if !raw_file.exists() {
        return Err(format!(
            "snapshot bruto ausente: {} — rode o scraper (ou derive) antes.",
            raw_file.display()
        )
        .into());
    }
    let raw: Table<Consulta> = serde_json::from_str(&std::fs::read_to_string(&raw_file)?)?;

    // 2. Mescla por numero com a saída anterior (se houver).
    let out_file = out_path(entity);
    let prev: HashMap<String, Consulta> = if out_file.exists() {
        let t: Table<Consulta> = serde_json::from_str(&std::fs::read_to_string(&out_file)?)?;
        t.items.into_iter().map(|c| (c.numero.clone(), c)).collect()
    } else {
        HashMap::new()
    };
    let mut merged = merge(raw.items, &prev, opts.force.as_deref())?;

    // 3. Contagem (invariante dinâmico: reaproveitados + pendentes == total).
    let total = merged.len();
    let pendentes = merged.iter().filter(|c| pendente(c)).count();
    let reaproveitados = total - pendentes;
    debug_assert_eq!(reaproveitados + pendentes, total);
    println!("📊 {}: total {total} | reaproveitados {reaproveitados} | pendentes {pendentes}", entity.id);

    // 4. Dry-run: estimativa de tokens dos pendentes e retorna SEM escrever.
    if opts.dry_run {
        let chars: usize = merged
            .iter()
            .filter(|c| pendente(c))
            .map(|c| c.assunto.chars().count() + c.corpo.chars().count())
            .sum();
        let tokens_est = chars / 4;
        println!("🔎 dry-run: ~{} tokens de entrada nos {pendentes} pendentes (nada foi escrito).", milhar(tokens_est));
        return Ok(());
    }

    // 5. Geração. Nesta fase só o modo --fake gera; sem ele, pendentes são um erro claro.
    if pendentes > 0 && !opts.fake {
        return Err("geração real de sinopse chega na F4 — use --dry-run ou --fake.".into());
    }

    // 6. Geração --fake com gravação atômica após cada documento.
    let mut gerados = 0usize;
    let falhas = 0usize; // sem LLM ainda — o campo já nasce para a F4 preencher.
    if opts.fake {
        let limit = opts.limit.unwrap_or(usize::MAX);
        for i in 0..merged.len() {
            if gerados >= limit {
                break;
            }
            if !pendente(&merged[i]) {
                continue;
            }
            let assunto = merged[i].assunto.clone();
            let resumo = fake_resumo(&assunto);
            merged[i].text_to_embed = compose_text_to_embed(&assunto, &resumo);
            merged[i].resumo = resumo;
            merged[i].sinopse_info = Some(auli_contract::SinopseInfo {
                modelo: "fake".into(),
                prompt_versao: 0,
                gerada_em: now_iso8601(),
            });
            write_atomic(&out_file, &entity.id, &merged)?;
            gerados += 1;
        }
    }

    // 7. Relatório final + invariante de guarda do passo.
    let pendentes_restantes = merged.iter().filter(|c| pendente(c)).count();
    println!(
        "✅ {}: total {total} | reaproveitados {reaproveitados} | gerados {gerados} | falhas {falhas} | pendentes-restantes {pendentes_restantes}",
        entity.id
    );
    assert_eq!(
        reaproveitados + gerados + falhas + pendentes_restantes,
        total,
        "invariante de guarda violado"
    );
    Ok(())
}

/// Formata um inteiro com separador de milhar (`1234567` → `1.234.567`).
fn milhar(n: usize) -> String {
    let s = n.to_string();
    let bytes = s.as_bytes();
    let mut out = String::new();
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(*b as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn consulta(numero: &str, resumo: &str) -> Consulta {
        Consulta {
            numero: numero.into(),
            assunto: format!("assunto de {numero}"),
            resumo: resumo.into(),
            corpo: format!("corpo integral de {numero}"),
            link: format!("https://exemplo/{numero}"),
            text_to_embed: String::new(),
            sinopse_info: None,
        }
    }

    /// EntityConfig apontando para um dir temporário exclusivo do teste (limpo no início).
    fn temp_entity(tag: &str) -> EntityConfig {
        let dir = std::env::temp_dir().join(format!("auli_sinopse_test_{tag}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        EntityConfig {
            id: "xx".into(),
            name: "Teste".into(),
            system_prompt: String::new(),
            data_dir: dir.to_string_lossy().into_owned(),
        }
    }

    fn write_table(entity: &EntityConfig, suffix: &str, items: Vec<Consulta>) {
        let path = PathBuf::from(&entity.data_dir).join(format!("{}-pareceres{suffix}", entity.id));
        let table = Table::new(&entity.id, "pareceres", items);
        std::fs::write(&path, serde_json::to_string_pretty(&table).unwrap()).unwrap();
    }

    fn read_out(entity: &EntityConfig) -> Vec<Consulta> {
        let t: Table<Consulta> =
            serde_json::from_str(&std::fs::read_to_string(out_path(entity)).unwrap()).unwrap();
        t.items
    }

    #[test]
    fn compose_reproduz_formato_do_derive() {
        assert_eq!(compose_text_to_embed("A", "R"), "A\nR");
        assert_eq!(compose_text_to_embed("A", ""), "A");
        assert_eq!(compose_text_to_embed("", "R"), "R");
        assert_eq!(compose_text_to_embed("", ""), "");
    }

    #[test]
    fn mescla_reaproveita_registro_com_resumo() {
        let e = temp_entity("mescla");
        write_table(&e, ".raw.json", vec![consulta("A", ""), consulta("B", ""), consulta("C", "")]);
        // Saída anterior: B já tem resumo (e sinopse_info) — deve ser reaproveitado.
        let mut b = consulta("B", "### Descrição\nresumo de B\n\n### Palavras Chave\n- **x**");
        b.sinopse_info = Some(auli_contract::SinopseInfo { modelo: "m".into(), prompt_versao: 1, gerada_em: "2026-01-01T00:00:00Z".into() });
        b.text_to_embed = "assunto de B\n...".into();
        write_table(&e, ".json", vec![b.clone()]);

        run(&e, SinopseOpts { dry_run: false, limit: None, force: None, fake: true }).unwrap();
        let out = read_out(&e);
        assert_eq!(out.len(), 3);
        // B preservado byte a byte na mescla (resumo/sinopse_info/text_to_embed do anterior).
        let out_b = out.iter().find(|c| c.numero == "B").unwrap();
        assert_eq!(out_b.resumo, b.resumo);
        assert_eq!(out_b.sinopse_info, b.sinopse_info);
        // A e C eram os 2 pendentes → gerados como fake.
        for n in ["A", "C"] {
            let c = out.iter().find(|c| c.numero == n).unwrap();
            assert!(c.resumo.contains("[FAKE]"));
            assert_eq!(c.sinopse_info.as_ref().unwrap().prompt_versao, 0);
        }
    }

    #[test]
    fn force_rependuza_registro_reaproveitavel() {
        let e = temp_entity("force");
        write_table(&e, ".raw.json", vec![consulta("A", ""), consulta("B", ""), consulta("C", "")]);
        let b = consulta("B", "resumo pronto de B");
        write_table(&e, ".json", vec![b]);
        // --force B: mesmo com resumo no anterior, B volta a pendente → 3 pendentes.
        run(&e, SinopseOpts { dry_run: false, limit: None, force: Some("B".into()), fake: true }).unwrap();
        let out = read_out(&e);
        // Todos foram (re)gerados como fake.
        assert!(out.iter().all(|c| c.resumo.contains("[FAKE]")));
    }

    #[test]
    fn duplicata_de_numero_no_raw_e_erro() {
        let e = temp_entity("dup");
        write_table(&e, ".raw.json", vec![consulta("A", ""), consulta("A", "")]);
        let err = run(&e, SinopseOpts { dry_run: false, limit: None, force: None, fake: true }).unwrap_err();
        assert!(format!("{err}").contains("duplicado"));
    }

    #[test]
    fn fake_com_limit_e_retomada() {
        let e = temp_entity("retomada");
        write_table(&e, ".raw.json", vec![consulta("A", ""), consulta("B", ""), consulta("C", "")]);
        // 1ª rodada: fake + limit 1 → 1 gerado, 2 pendentes restantes.
        run(&e, SinopseOpts { dry_run: false, limit: Some(1), force: None, fake: true }).unwrap();
        let out1 = read_out(&e);
        assert_eq!(out1.iter().filter(|c| c.resumo.contains("[FAKE]")).count(), 1);
        assert_eq!(out1.iter().filter(|c| pendente(c)).count(), 2);
        // 2ª rodada: fake sem limit → o gerado é reaproveitado; os 2 restantes geram.
        run(&e, SinopseOpts { dry_run: false, limit: None, force: None, fake: true }).unwrap();
        let out2 = read_out(&e);
        assert_eq!(out2.len(), 3);
        assert!(out2.iter().all(|c| c.resumo.contains("[FAKE]")));
    }

    #[test]
    fn dry_run_nao_escreve() {
        let e = temp_entity("dryrun");
        write_table(&e, ".raw.json", vec![consulta("A", ""), consulta("B", "")]);
        run(&e, SinopseOpts { dry_run: true, limit: None, force: None, fake: false }).unwrap();
        assert!(!out_path(&e).exists(), "dry-run não deve criar o .json");
    }
}
