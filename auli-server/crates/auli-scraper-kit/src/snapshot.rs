//! I/O do *snapshot de coleta* — a fronteira **scraper → collections**.
//!
//! Cada scraper grava aqui a sua coleção (`faqs` ou `servicos`) em `../data/<id>/<id>-snapshot.json`.
//! A gravação é **merge, não overwrite**: raspar só `faqs` não pode apagar `colecoes.servicos` que já
//! esteja no arquivo (e vice-versa), por isso carregamos as coleções existentes antes de atualizar só
//! a raspada. Os tipos vivem no `auli-contract`; aqui só há I/O.

use std::path::{Path, PathBuf};

use anyhow::Result;
use auli_contract::{
    Colecoes, ColetaFaqs, ColetaServicos, FaqRaw, Publico, SNAPSHOT_SCHEMA_VERSION, ScraperInfo,
    ServicoRaw, Snapshot,
};

/// Caminho do snapshot: `../data/<id>/<id>-snapshot.json`, irmão de `raw/`. O `data_dir` aponta para
/// `.../<id>/raw`, então subimos um nível.
fn snapshot_path(id: &str, data_dir: &str) -> PathBuf {
    let base = Path::new(data_dir).parent().unwrap_or_else(|| Path::new(data_dir));
    base.join(format!("{}-snapshot.json", id))
}

/// Instante atual em RFC 3339 (UTC). Metadado de auditoria; não entra em nenhum artefato derivado.
fn now_rfc3339() -> String {
    use time::OffsetDateTime;
    use time::format_description::well_known::Rfc3339;
    OffsetDateTime::now_utc().format(&Rfc3339).unwrap_or_default()
}

/// Grava a coleta de faqs, preservando a coleção de serviços já presente. `scraper` identifica quem
/// gravou (cada scraper passa o seu nome/versão).
pub fn write_faqs(id: &str, data_dir: &str, scraper: &ScraperInfo, items: Vec<FaqRaw>) -> Result<()> {
    let coleta = ColetaFaqs { coletado_em: now_rfc3339(), items };
    merge_and_save(id, data_dir, scraper, |c| c.faqs = Some(coleta))
}

/// Grava a coleta de serviços, preservando a coleção de faqs já presente.
pub fn write_servicos(
    id: &str,
    data_dir: &str,
    scraper: &ScraperInfo,
    publicos_ordem: Vec<Publico>,
    items: Vec<ServicoRaw>,
) -> Result<()> {
    let coleta = ColetaServicos { coletado_em: now_rfc3339(), publicos_ordem, items };
    merge_and_save(id, data_dir, scraper, |c| c.servicos = Some(coleta))
}

/// Carrega as coleções já gravadas (preservando a não raspada), aplica a atualização e regrava o
/// snapshot inteiro com o header atual.
fn merge_and_save(
    id: &str,
    data_dir: &str,
    scraper: &ScraperInfo,
    update: impl FnOnce(&mut Colecoes),
) -> Result<()> {
    let path = snapshot_path(id, data_dir);
    let mut colecoes = load_colecoes(&path)?;
    update(&mut colecoes);

    let snapshot = Snapshot {
        schema_version: SNAPSHOT_SCHEMA_VERSION,
        entidade: id.to_string(),
        scraper: scraper.clone(),
        colecoes,
    };

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, serde_json::to_string_pretty(&snapshot)?)?;
    println!("Wrote {} (snapshot)", path.display());
    Ok(())
}

/// Prefixo mínimo do snapshot lido **antes** da desserialização tipada. Um snapshot de schema
/// incompatível (ex.: v1 com `classe`/`publicos` em vez de `ocorrencias`) falharia com um erro cru
/// do serde ao desserializar o `Snapshot` inteiro; lendo só o header primeiro conseguimos dar a
/// mensagem amigável de "re-raspe" tanto no merge dos scrapers quanto no `process`.
#[derive(serde::Deserialize)]
struct SnapshotHeader {
    schema_version: u32,
}

/// Lê o header e recusa, com mensagem amigável, um schema diferente do atual — antes da
/// desserialização tipada. `entidade` fica para o chamador (`process`) conferir.
fn check_schema_version(bytes: &[u8]) -> Result<()> {
    let header: SnapshotHeader = serde_json::from_slice(bytes)
        .map_err(|e| anyhow::anyhow!("snapshot ilegível (nem o header desserializa): {e}"))?;
    if header.schema_version != SNAPSHOT_SCHEMA_VERSION {
        anyhow::bail!(
            "snapshot na versão de schema v{} (esperado v{}). Re-raspe a entidade — o snapshot é \
             regenerável do cache, não há migração.",
            header.schema_version,
            SNAPSHOT_SCHEMA_VERSION
        );
    }
    Ok(())
}

/// Coleções já gravadas no snapshot, ou vazias se o arquivo ainda não existe.
fn load_colecoes(path: &Path) -> Result<Colecoes> {
    if !path.exists() {
        return Ok(Colecoes::default());
    }
    let bytes = std::fs::read(path)?;
    check_schema_version(&bytes)?;
    let snapshot: Snapshot = serde_json::from_slice(&bytes)?;
    Ok(snapshot.colecoes)
}

/// Carrega o snapshot inteiro de `../data/<id>/<id>-snapshot.json`, ou `None` se ainda não existe.
/// Usado pelo `process` para validar `schema_version`/`entidade` antes de derivar.
pub fn load(id: &str, data_dir: &str) -> Result<Option<Snapshot>> {
    let path = snapshot_path(id, data_dir);
    if !path.exists() {
        return Ok(None);
    }
    let bytes = std::fs::read(&path)?;
    check_schema_version(&bytes)?;
    Ok(Some(serde_json::from_slice(&bytes)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_data_dir(tag: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("auli_snap_test_{}_{}", std::process::id(), tag));
        p.push("raw");
        p
    }

    fn scraper() -> ScraperInfo {
        ScraperInfo { nome: "test".into(), versao: "0".into() }
    }

    fn faq(pergunta: &str) -> FaqRaw {
        FaqRaw { pergunta: pergunta.into(), resposta: "r".into(), origin: String::new(), url: "u".into() }
    }

    fn svc(link: &str) -> ServicoRaw {
        ServicoRaw {
            titulo: "t".into(),
            descricao: "d".into(),
            link: link.into(),
            orgao: "o".into(),
            ocorrencias: vec![auli_contract::Ocorrencia { publico: "Cidadãos".into(), classe: "c".into() }],
        }
    }

    #[test]
    fn merge_preserves_the_other_collection() {
        let data_dir = tmp_data_dir("merge");
        let dd = data_dir.to_str().unwrap();
        let path = snapshot_path("rs", dd);
        let sc = scraper();

        write_faqs("rs", dd, &sc, vec![faq("q1")]).unwrap();
        write_servicos(
            "rs",
            dd,
            &sc,
            vec![Publico { nome: "Cidadãos".into(), slug: "servicos-ao-cidadao".into() }],
            vec![svc("l1")],
        )
        .unwrap();

        let snap: Snapshot = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert!(snap.colecoes.faqs.is_some());
        assert!(snap.colecoes.servicos.is_some());

        // Re-raspar só faqs não pode apagar a coleção de serviços.
        write_faqs("rs", dd, &sc, vec![faq("q2")]).unwrap();
        let snap: Snapshot = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert!(snap.colecoes.servicos.is_some());
        assert_eq!(snap.colecoes.faqs.unwrap().items[0].pergunta, "q2");

        let _ = std::fs::remove_dir_all(data_dir.parent().unwrap());
    }

    #[test]
    fn load_rejects_old_schema_with_friendly_message() {
        let data_dir = tmp_data_dir("oldschema");
        let dd = data_dir.to_str().unwrap();
        let path = snapshot_path("rs", dd);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        // A v1-shaped snapshot: only the header matters — the typed body would fail serde, but the
        // header check must fire first with the "re-raspe" message.
        std::fs::write(&path, r#"{"schema_version":1,"entidade":"rs","classe":"x"}"#).unwrap();

        let err = load("rs", dd).unwrap_err().to_string();
        assert!(err.contains("Re-raspe"), "esperava mensagem amigável, veio: {err}");

        let _ = std::fs::remove_dir_all(data_dir.parent().unwrap());
    }
}
