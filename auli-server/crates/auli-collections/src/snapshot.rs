//! Escrita do *snapshot de coleta* — a fronteira **scraper → collections**.
//!
//! Cada fluxo de scrape grava aqui a sua coleção (`faqs` ou `servicos`) em
//! `../data/<id>/<id>-snapshot.json`. A gravação é **merge, não overwrite**: rodar `rs faqs` não
//! pode apagar `colecoes.servicos` que já esteja no arquivo (e vice-versa), por isso carregamos as
//! coleções existentes antes de atualizar só a raspada.
//!
//! Os tipos vivem no `auli-contract` (módulo `snapshot`); aqui só há I/O. Fase 1 é aditiva: os
//! artefatos atuais seguem sendo gravados — o `process` (etapa C) passará a derivá-los deste arquivo.

use std::path::{Path, PathBuf};

use auli_contract::{
    Colecoes, ColetaFaqs, ColetaServicos, FaqRaw, Publico, SNAPSHOT_SCHEMA_VERSION, ScraperInfo,
    ServicoRaw, Snapshot,
};

use crate::errors::Result;

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

/// Grava a coleta de faqs, preservando a coleção de serviços já presente.
pub fn write_faqs(id: &str, data_dir: &str, items: Vec<FaqRaw>) -> Result<()> {
    let coleta = ColetaFaqs { coletado_em: now_rfc3339(), items };
    merge_and_save(id, data_dir, |c| c.faqs = Some(coleta))
}

/// Grava a coleta de serviços, preservando a coleção de faqs já presente.
pub fn write_servicos(
    id: &str,
    data_dir: &str,
    publicos_ordem: Vec<Publico>,
    items: Vec<ServicoRaw>,
) -> Result<()> {
    let coleta = ColetaServicos { coletado_em: now_rfc3339(), publicos_ordem, items };
    merge_and_save(id, data_dir, |c| c.servicos = Some(coleta))
}

/// Carrega as coleções já gravadas (preservando a não raspada), aplica a atualização e regrava o
/// snapshot inteiro com o header atual.
fn merge_and_save(id: &str, data_dir: &str, update: impl FnOnce(&mut Colecoes)) -> Result<()> {
    let path = snapshot_path(id, data_dir);
    let mut colecoes = load_colecoes(&path)?;
    update(&mut colecoes);

    let snapshot = Snapshot {
        schema_version: SNAPSHOT_SCHEMA_VERSION,
        entidade: id.to_string(),
        scraper: ScraperInfo {
            nome: env!("CARGO_PKG_NAME").to_string(),
            versao: env!("CARGO_PKG_VERSION").to_string(),
        },
        colecoes,
    };

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, serde_json::to_string_pretty(&snapshot)?)?;
    println!("Wrote {} (snapshot)", path.display());
    Ok(())
}

/// Coleções já gravadas no snapshot, ou vazias se o arquivo ainda não existe.
fn load_colecoes(path: &Path) -> Result<Colecoes> {
    if !path.exists() {
        return Ok(Colecoes::default());
    }
    let bytes = std::fs::read(path)?;
    let snapshot: Snapshot = serde_json::from_slice(&bytes)?;
    Ok(snapshot.colecoes)
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

    fn faq(pergunta: &str) -> FaqRaw {
        FaqRaw { pergunta: pergunta.into(), resposta: "r".into(), origin: String::new(), url: "u".into() }
    }

    fn svc(link: &str) -> ServicoRaw {
        ServicoRaw {
            titulo: "t".into(),
            descricao: "d".into(),
            link: link.into(),
            orgao: "o".into(),
            classe: "c".into(),
            publicos: vec!["Cidadãos".into()],
        }
    }

    #[test]
    fn merge_preserves_the_other_collection() {
        let data_dir = tmp_data_dir("merge");
        let dd = data_dir.to_str().unwrap();
        let path = snapshot_path("rs", dd);

        write_faqs("rs", dd, vec![faq("q1")]).unwrap();
        write_servicos(
            "rs",
            dd,
            vec![Publico { nome: "Cidadãos".into(), slug: "rs-servicos-ao-cidadao".into() }],
            vec![svc("l1")],
        )
        .unwrap();

        let snap: Snapshot = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert!(snap.colecoes.faqs.is_some());
        assert!(snap.colecoes.servicos.is_some());

        // Re-raspar só faqs não pode apagar a coleção de serviços.
        write_faqs("rs", dd, vec![faq("q2")]).unwrap();
        let snap: Snapshot = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert!(snap.colecoes.servicos.is_some());
        assert_eq!(snap.colecoes.faqs.unwrap().items[0].pergunta, "q2");

        let _ = std::fs::remove_dir_all(data_dir.parent().unwrap());
    }
}
