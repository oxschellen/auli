//! I/O do *snapshot de coleta* — a fronteira **scraper → collections**.
//!
//! Cada scraper grava a sua coleção no **seu próprio arquivo**: `../data/<id>/<id>-<kind>-snapshot.json`
//! (kind ∈ {`servicos`, `faqs`}). Sem merge nem read-modify-write: raspar `faqs` grava só o arquivo de
//! faqs e não toca no de serviços (e vice-versa). Os tipos vivem no `auli-contract`; aqui só há I/O.

use std::path::{Path, PathBuf};

use anyhow::Result;
use auli_contract::{
    CollectionSnapshot, ColetaFaqs, ColetaServicos, FaqRaw, Publico, SNAPSHOT_SCHEMA_VERSION,
    ScraperInfo, ServicoRaw,
};
use serde::Serialize;
use serde::de::DeserializeOwned;

/// Caminho do snapshot de uma coleção: `../data/<id>/<id>-<kind>-snapshot.json`, irmão de `raw/`. O
/// `data_dir` aponta para `.../<id>/raw`, então subimos um nível.
fn snapshot_path(id: &str, data_dir: &str, kind: &str) -> PathBuf {
    let base = Path::new(data_dir).parent().unwrap_or_else(|| Path::new(data_dir));
    base.join(format!("{}-{}-snapshot.json", id, kind))
}

/// Instante atual em RFC 3339 (UTC). Metadado de auditoria; não entra em nenhum artefato derivado.
fn now_rfc3339() -> String {
    use time::OffsetDateTime;
    use time::format_description::well_known::Rfc3339;
    OffsetDateTime::now_utc().format(&Rfc3339).unwrap_or_default()
}

/// Grava a coleta de faqs no arquivo `<id>-faqs-snapshot.json`. `scraper` identifica quem gravou.
pub fn write_faqs(id: &str, data_dir: &str, scraper: &ScraperInfo, items: Vec<FaqRaw>) -> Result<()> {
    let coleta = ColetaFaqs { coletado_em: now_rfc3339(), items };
    save(id, data_dir, "faqs", scraper, coleta)
}

/// Grava a coleta de serviços no arquivo `<id>-servicos-snapshot.json`.
pub fn write_servicos(
    id: &str,
    data_dir: &str,
    scraper: &ScraperInfo,
    publicos_ordem: Vec<Publico>,
    items: Vec<ServicoRaw>,
) -> Result<()> {
    let coleta = ColetaServicos { coletado_em: now_rfc3339(), publicos_ordem, items };
    save(id, data_dir, "servicos", scraper, coleta)
}

/// Serializa o snapshot de uma coleção no seu arquivo próprio — **sem merge**, sobrescrevendo apenas
/// o arquivo daquela coleção.
fn save<C: Serialize>(
    id: &str,
    data_dir: &str,
    kind: &str,
    scraper: &ScraperInfo,
    coleta: C,
) -> Result<()> {
    let snapshot = CollectionSnapshot {
        schema_version: SNAPSHOT_SCHEMA_VERSION,
        entidade: id.to_string(),
        scraper: scraper.clone(),
        coleta,
    };
    let path = snapshot_path(id, data_dir, kind);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, serde_json::to_string_pretty(&snapshot)?)?;
    println!("Wrote {} (snapshot)", path.display());
    Ok(())
}

/// Prefixo mínimo lido **antes** da desserialização tipada. Um snapshot de schema incompatível (ex.:
/// o v2 mesclado com `colecoes`) falharia com um erro cru do serde ao desserializar o
/// `CollectionSnapshot` inteiro; lendo só o header primeiro conseguimos dar a mensagem amigável.
#[derive(serde::Deserialize)]
struct SnapshotHeader {
    schema_version: u32,
    entidade: String,
}

/// Valida `schema_version` (contra [`SNAPSHOT_SCHEMA_VERSION`]) e `entidade` a partir do header,
/// antes da desserialização tipada.
fn check_header(bytes: &[u8], id: &str) -> Result<()> {
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
    if header.entidade != id {
        anyhow::bail!("entidade do snapshot ('{}') não bate com a pedida ('{}').", header.entidade, id);
    }
    Ok(())
}

/// Carrega o snapshot de uma coleção (`<id>-<kind>-snapshot.json`), ou `None` se o arquivo ainda não
/// existe. `C` é a coleta concreta esperada para o `kind` (o chamador escolhe: `ColetaServicos` para
/// `"servicos"`, `ColetaFaqs` para `"faqs"`). Valida `schema_version`/`entidade` antes de desserializar.
pub fn load<C: DeserializeOwned>(
    id: &str,
    data_dir: &str,
    kind: &str,
) -> Result<Option<CollectionSnapshot<C>>> {
    let path = snapshot_path(id, data_dir, kind);
    if !path.exists() {
        return Ok(None);
    }
    let bytes = std::fs::read(&path)?;
    check_header(&bytes, id)?;
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
    fn each_collection_writes_its_own_file_and_leaves_the_other_untouched() {
        let data_dir = tmp_data_dir("split");
        let dd = data_dir.to_str().unwrap();
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

        let faqs_path = snapshot_path("rs", dd, "faqs");
        let servicos_path = snapshot_path("rs", dd, "servicos");
        assert!(faqs_path.exists(), "faqs snapshot file must exist");
        assert!(servicos_path.exists(), "servicos snapshot file must exist");

        // Re-writing servicos must NOT touch the faqs file (the invariant the old merge protected).
        let faqs_before = std::fs::read(&faqs_path).unwrap();
        write_servicos(
            "rs",
            dd,
            &sc,
            vec![Publico { nome: "Cidadãos".into(), slug: "servicos-ao-cidadao".into() }],
            vec![svc("l2")],
        )
        .unwrap();
        assert_eq!(
            std::fs::read(&faqs_path).unwrap(),
            faqs_before,
            "a servicos write must not touch the faqs snapshot file"
        );

        // Each loads back as its own typed coleta.
        let faqs: CollectionSnapshot<ColetaFaqs> = load("rs", dd, "faqs").unwrap().unwrap();
        assert_eq!(faqs.coleta.items[0].pergunta, "q1");
        let servicos: CollectionSnapshot<ColetaServicos> = load("rs", dd, "servicos").unwrap().unwrap();
        assert_eq!(servicos.coleta.items[0].link, "l2");

        let _ = std::fs::remove_dir_all(data_dir.parent().unwrap());
    }

    #[test]
    fn load_missing_file_is_none() {
        let data_dir = tmp_data_dir("missing");
        let dd = data_dir.to_str().unwrap();
        let got: Option<CollectionSnapshot<ColetaServicos>> = load("rs", dd, "servicos").unwrap();
        assert!(got.is_none());
    }

    #[test]
    fn load_rejects_old_schema_with_friendly_message() {
        let data_dir = tmp_data_dir("oldschema");
        let dd = data_dir.to_str().unwrap();
        let path = snapshot_path("rs", dd, "servicos");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        // An old v2 (merged `colecoes`) file at the new path: the header check must fire on the
        // version before the typed body — with the "re-raspe" message, not a raw serde error.
        std::fs::write(&path, r#"{"schema_version":2,"entidade":"rs","colecoes":{}}"#).unwrap();

        let err = load::<ColetaServicos>("rs", dd, "servicos").unwrap_err().to_string();
        assert!(err.contains("Re-raspe"), "esperava mensagem amigável, veio: {err}");

        let _ = std::fs::remove_dir_all(data_dir.parent().unwrap());
    }
}
