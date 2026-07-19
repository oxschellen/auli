//! Materialização da árvore `docs/pareceres/*.md` — um arquivo por consulta.
//!
//! **Fase G2 (A-via-B): a árvore é DERIVADA do JSON**, que segue sendo a fonte. Por isso ela é um
//! *espelho fiel*: consultas removidas do JSON têm o `.md` correspondente apagado, para o
//! `docs_hash` refletir exatamente o conteúdo indexado. Quando a árvore virar a fonte (G5), essa
//! poda sai — apagar passaria a ser destrutivo.
//!
//! O contrato do arquivo (frontmatter + `## sinopse` + `## corpo`) vive em `auli_contract::mddoc`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use auli_contract::{Consulta, Table, mddoc};

use crate::error::Result;

/// Subdiretório da árvore dentro de `docs/` (um por kind; hoje só pareceres).
const KIND_DIR: &str = "pareceres";

/// Materializa `<docs_dir>/pareceres/<slug>.md` a partir do contrato de pareceres.
///
/// Devolve `None` se a entidade não tem pareceres (arquivo ausente) — nada a materializar, e o
/// manifesto fica sem `docs_hash`. Colisão de slug dentro da entidade é **erro** (violação de
/// identidade, mesma doutrina do dedup por `numero`).
pub fn materializar_pareceres(entity: &str, source: &Path, docs_dir: &Path) -> Result<Option<usize>> {
    let src = source.join(format!("{entity}-pareceres.json"));
    let bytes = match std::fs::read(&src) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let table: Table<Consulta> = serde_json::from_slice(&bytes)?;

    let dir = docs_dir.join(KIND_DIR);
    std::fs::create_dir_all(&dir)?;

    // Slug -> numero, para detectar colisão com mensagem útil (quais dois números colidiram).
    let mut vistos: HashMap<String, String> = HashMap::with_capacity(table.items.len());
    let mut esperados: Vec<PathBuf> = Vec::with_capacity(table.items.len());

    for c in &table.items {
        let slug = mddoc::slug(&c.numero);
        if slug.is_empty() {
            return Err(format!(
                "consulta com `numero` que não gera slug: {:?} (entidade {entity})",
                c.numero
            )
            .into());
        }
        if let Some(anterior) = vistos.insert(slug.clone(), c.numero.clone()) {
            return Err(format!(
                "colisão de slug em '{entity}': {:?} e {:?} geram o mesmo arquivo `{slug}.md`",
                anterior, c.numero
            )
            .into());
        }

        let header = mddoc::DocHeader {
            numero: c.numero.clone(),
            assunto: c.assunto.clone(),
            link: c.link.clone(),
            sinopse_info: c.sinopse_info.clone(),
        };
        let resumo = c.resumo.trim();
        let texto = mddoc::render_doc(&header, (!resumo.is_empty()).then_some(resumo), &c.corpo);

        let destino = dir.join(format!("{slug}.md"));
        escrever_atomico(&destino, texto.as_bytes())?;
        esperados.push(destino);
    }

    let podados = podar_orfaos(&dir, &esperados)?;
    if podados > 0 {
        println!("🧹 docs: {podados} arquivo(s) órfão(s) removido(s) (não estão mais no contrato)");
    }
    Ok(Some(table.items.len()))
}

/// Remove `.md` que não estão no conjunto esperado — mantém a árvore um espelho fiel do contrato
/// (ver a nota de fase no topo do módulo).
fn podar_orfaos(dir: &Path, esperados: &[PathBuf]) -> Result<usize> {
    let manter: std::collections::HashSet<&Path> = esperados.iter().map(|p| p.as_path()).collect();
    let mut n = 0;
    for entry in std::fs::read_dir(dir)? {
        let p = entry?.path();
        if p.is_file() && p.extension().is_some_and(|e| e == "md") && !manter.contains(p.as_path()) {
            std::fs::remove_file(&p)?;
            n += 1;
        }
    }
    Ok(n)
}

/// Escrita atômica (`.tmp` + rename), como todo o resto do pipeline: uma queda no meio nunca deixa
/// um `.md` truncado no lugar do bom.
fn escrever_atomico(destino: &Path, bytes: &[u8]) -> Result<()> {
    let tmp = destino.with_extension("md.tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, destino)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use auli_contract::SinopseInfo;

    fn consulta(numero: &str, resumo: &str) -> Consulta {
        Consulta {
            numero: numero.into(),
            assunto: "ICMS. ASSUNTO".into(),
            resumo: resumo.into(),
            corpo: "Corpo integral.".into(),
            link: "http://x/1".into(),
            text_to_embed: "irrelevante aqui".into(),
            sinopse_info: (!resumo.is_empty()).then(|| SinopseInfo {
                modelo: "m".into(),
                prompt_versao: 1,
                gerada_em: "2026-07-19T00:00:00Z".into(),
            }),
        }
    }

    fn cenario(tag: &str, items: Vec<Consulta>) -> (PathBuf, PathBuf) {
        let base = std::env::temp_dir().join(format!("auli-docs-mat-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let source = base.join("raw");
        let docs = base.join("docs");
        std::fs::create_dir_all(&source).unwrap();
        let t = Table::new("xx", "pareceres", items);
        std::fs::write(source.join("xx-pareceres.json"), serde_json::to_vec(&t).unwrap()).unwrap();
        (source, docs)
    }

    #[test]
    fn entidade_sem_pareceres_nao_materializa() {
        let base = std::env::temp_dir().join(format!("auli-docs-vazio-{}", std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        assert_eq!(materializar_pareceres("xx", &base, &base.join("docs")).unwrap(), None);
    }

    #[test]
    fn materializa_e_faz_round_trip_pelo_contrato() {
        let (source, docs) = cenario("rt", vec![consulta("CONSULTA Nº 1/26", "### Descrição Resumida do Assunto\nx")]);
        assert_eq!(materializar_pareceres("xx", &source, &docs).unwrap(), Some(1));

        let p = docs.join("pareceres/consulta-no-1-26.md");
        assert!(p.exists(), "arquivo com slug esperado não foi criado");
        let texto = std::fs::read_to_string(&p).unwrap();
        let (h, sin, corpo) = mddoc::parse_doc(&texto).unwrap();
        assert_eq!(h.numero, "CONSULTA Nº 1/26");
        assert_eq!(h.sinopse_info.unwrap().prompt_versao, 1);
        assert!(sin.unwrap().contains("Descrição Resumida"));
        assert_eq!(corpo, "Corpo integral.");
    }

    #[test]
    fn consulta_sem_sinopse_vira_md_pendente() {
        let (source, docs) = cenario("pend", vec![consulta("PARECER Nº 9", "")]);
        materializar_pareceres("xx", &source, &docs).unwrap();
        let texto = std::fs::read_to_string(docs.join("pareceres/parecer-no-9.md")).unwrap();
        let (h, sin, _) = mddoc::parse_doc(&texto).unwrap();
        assert_eq!(h.sinopse_info, None, "pendente não carrega proveniência");
        assert_eq!(sin, None, "pendente não tem seção de sinopse");
    }

    #[test]
    fn colisao_de_slug_e_erro_com_os_dois_numeros() {
        let (source, docs) = cenario("colisao", vec![consulta("CONSULTA 1/26", "r"), consulta("CONSULTA 1-26", "r")]);
        let e = materializar_pareceres("xx", &source, &docs).unwrap_err().to_string();
        assert!(e.contains("colisão de slug"), "erro: {e}");
        assert!(e.contains("CONSULTA 1/26") && e.contains("CONSULTA 1-26"), "erro: {e}");
    }

    #[test]
    fn re_materializar_poda_orfaos_e_e_idempotente() {
        let (source, docs) = cenario("poda", vec![consulta("A 1", "r"), consulta("B 2", "r")]);
        materializar_pareceres("xx", &source, &docs).unwrap();
        assert!(docs.join("pareceres/a-1.md").exists() && docs.join("pareceres/b-2.md").exists());

        // Rodar de novo com o mesmo contrato não muda nada (idempotente).
        let antes = std::fs::read_to_string(docs.join("pareceres/a-1.md")).unwrap();
        materializar_pareceres("xx", &source, &docs).unwrap();
        assert_eq!(std::fs::read_to_string(docs.join("pareceres/a-1.md")).unwrap(), antes);

        // Contrato encolhe ⇒ o órfão some (a árvore espelha o contrato).
        let t = Table::new("xx", "pareceres", vec![consulta("A 1", "r")]);
        std::fs::write(source.join("xx-pareceres.json"), serde_json::to_vec(&t).unwrap()).unwrap();
        materializar_pareceres("xx", &source, &docs).unwrap();
        assert!(docs.join("pareceres/a-1.md").exists());
        assert!(!docs.join("pareceres/b-2.md").exists(), "órfão devia ter sido podado");
    }

    #[test]
    fn nao_deixa_tmp_para_tras() {
        let (source, docs) = cenario("tmp", vec![consulta("A 1", "r")]);
        materializar_pareceres("xx", &source, &docs).unwrap();
        let sobras: Vec<_> = std::fs::read_dir(docs.join("pareceres"))
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(sobras.is_empty(), "escrita atômica não pode deixar .tmp");
    }
}
