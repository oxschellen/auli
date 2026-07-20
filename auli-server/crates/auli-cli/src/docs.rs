//! Materialização da árvore `docs/pareceres/*.md` — um arquivo por consulta.
//!
//! **Fase G4 (A-via-B): propriedade dividida.** A ÁRVORE é dona da **sinopse** (o passo
//! `auli-collections <id> sinopse` escreve nos `.md`, não mais no JSON); o JSON segue dono do resto
//! (`numero`/`assunto`/`link`/`corpo`) e do **rol** de quais consultas existem. Materializar é, por
//! isso, um *merge*: regrava os campos do JSON e **preserva** a `## sinopse` do arquivo.
//!
//! **A poda saiu na G5a.** Enquanto o rol era só do JSON, apagar `.md` fora dele mantinha a árvore
//! um espelho. Agora os **produtores escrevem direto na árvore** (scrapers de pareceres emitem um
//! `.md` por consulta inédita), então documento presente na árvore e ausente do JSON é estado
//! **legal** — é uma consulta nova esperando o derive. Podar apagaria justamente o que acabou de ser
//! coletado, junto com a sinopse que ele já pudesse ter.
//!
//! O contrato do arquivo (frontmatter + `## sinopse` + `## corpo`) vive em `auli_contract::mddoc`.

use std::collections::HashMap;
use std::path::Path;

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

        let destino = dir.join(format!("{slug}.md"));
        // G4 — regra de propriedade: a ÁRVORE é dona da sinopse; o JSON é dono do resto
        // (numero/assunto/link/corpo). Num `.md` que já existe, a seção `## sinopse` e as chaves
        // `sinopse_*` vêm do arquivo (o passo `sinopse` escreve lá, não no JSON); os demais campos
        // são regravados do JSON, para correções de re-scrape propagarem. Arquivo novo nasce com o
        // que o JSON tiver (ponte: os dados de hoje ainda carregam `resumo` no JSON).
        let (sinopse, sinopse_info) = match ler_sinopse_existente(&destino)? {
            Some(par) => par,
            None => {
                let r = c.resumo.trim();
                ((!r.is_empty()).then(|| r.to_string()), c.sinopse_info.clone())
            }
        };
        let header = mddoc::DocHeader {
            numero: c.numero.clone(),
            assunto: c.assunto.clone(),
            link: c.link.clone(),
            sinopse_info,
        };
        let texto = mddoc::render_doc(&header, sinopse.as_deref(), &c.corpo);

        escrever_atomico(&destino, texto.as_bytes())?;
    }

    Ok(Some(table.items.len()))
}

/// Lê a sinopse de um `.md` já existente: `(seção ## sinopse, sinopse_info)`. `None` se o arquivo
/// não existe. Arquivo ilegível é **erro** — sobrescrever cegamente um `.md` corrompido apagaria
/// uma sinopse que custou LLM; melhor falhar alto e deixar o operador decidir.
fn ler_sinopse_existente(
    destino: &Path,
) -> Result<Option<(Option<String>, Option<auli_contract::SinopseInfo>)>> {
    let texto = match std::fs::read_to_string(destino) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let (header, sinopse, _corpo) = mddoc::parse_doc(&texto).map_err(|e| {
        format!("`{}` existe mas não parseia ({e}) — corrija ou remova antes de re-materializar", destino.display())
    })?;
    Ok(Some((sinopse, header.sinopse_info)))
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
    use std::path::PathBuf;

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
    fn re_materializar_e_idempotente_e_NAO_poda_o_que_esta_fora_do_json() {
        let (source, docs) = cenario("poda", vec![consulta("A 1", "r"), consulta("B 2", "r")]);
        materializar_pareceres("xx", &source, &docs).unwrap();
        assert!(docs.join("pareceres/a-1.md").exists() && docs.join("pareceres/b-2.md").exists());

        // Rodar de novo com o mesmo contrato não muda nada (idempotente).
        let antes = std::fs::read_to_string(docs.join("pareceres/a-1.md")).unwrap();
        materializar_pareceres("xx", &source, &docs).unwrap();
        assert_eq!(std::fs::read_to_string(docs.join("pareceres/a-1.md")).unwrap(), antes);

        // G5a: contrato encolhe, mas o `.md` SOBREVIVE. Documento na árvore e fora do JSON é estado
        // legal — é o que o produtor acabou de emitir (consulta nova, ainda sem passar pelo derive).
        // Podar aqui apagaria a coleta recém-feita, junto com a sinopse que ela já pudesse ter.
        let t = Table::new("xx", "pareceres", vec![consulta("A 1", "r")]);
        std::fs::write(source.join("xx-pareceres.json"), serde_json::to_vec(&t).unwrap()).unwrap();
        materializar_pareceres("xx", &source, &docs).unwrap();
        assert!(docs.join("pareceres/a-1.md").exists());
        assert!(
            docs.join("pareceres/b-2.md").exists(),
            "documento fora do JSON NÃO pode ser podado (G5a) — seria apagar coleta nova"
        );
    }

    #[test]
    fn re_materializar_preserva_a_sinopse_da_arvore_e_atualiza_o_resto() {
        // G4: o passo `sinopse` escreve na árvore; o JSON NÃO tem o resumo. Re-materializar não pode
        // apagar a sinopse — e deve trazer as correções de assunto/corpo vindas do JSON.
        let (source, docs) = cenario("g4merge", vec![consulta("A 1", "")]);
        materializar_pareceres("xx", &source, &docs).unwrap();
        let p = docs.join("pareceres/a-1.md");

        // Simula o passo sinopse editando o .md (árvore dona da sinopse).
        let (h, sin, corpo) = mddoc::parse_doc(&std::fs::read_to_string(&p).unwrap()).unwrap();
        assert_eq!(sin, None, "nasce pendente (JSON sem resumo)");
        let mut h2 = h.clone();
        h2.sinopse_info = Some(SinopseInfo {
            modelo: "m-da-arvore".into(),
            prompt_versao: 1,
            gerada_em: "2026-07-20T00:00:00Z".into(),
        });
        std::fs::write(&p, mddoc::render_doc(&h2, Some("SINOPSE DA ARVORE"), &corpo)).unwrap();

        // JSON muda assunto e corpo (re-scrape corrigiu), e segue SEM resumo.
        let mut c = consulta("A 1", "");
        c.assunto = "ASSUNTO CORRIGIDO".into();
        c.corpo = "Corpo corrigido.".into();
        let t = Table::new("xx", "pareceres", vec![c]);
        std::fs::write(source.join("xx-pareceres.json"), serde_json::to_vec(&t).unwrap()).unwrap();
        materializar_pareceres("xx", &source, &docs).unwrap();

        let (h3, sin3, corpo3) = mddoc::parse_doc(&std::fs::read_to_string(&p).unwrap()).unwrap();
        assert_eq!(sin3.as_deref(), Some("SINOPSE DA ARVORE"), "sinopse da árvore PRESERVADA");
        assert_eq!(h3.sinopse_info.unwrap().modelo, "m-da-arvore", "proveniência preservada");
        assert_eq!(h3.assunto, "ASSUNTO CORRIGIDO", "campo do JSON atualizado");
        assert_eq!(corpo3, "Corpo corrigido.", "corpo do JSON atualizado");
    }

    #[test]
    fn md_existente_ilegivel_e_erro_em_vez_de_sobrescrever() {
        let (source, docs) = cenario("g4ruim", vec![consulta("A 1", "r")]);
        materializar_pareceres("xx", &source, &docs).unwrap();
        std::fs::write(docs.join("pareceres/a-1.md"), "lixo sem frontmatter").unwrap();
        let e = materializar_pareceres("xx", &source, &docs).unwrap_err().to_string();
        assert!(e.contains("não parseia"), "erro: {e}");
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
