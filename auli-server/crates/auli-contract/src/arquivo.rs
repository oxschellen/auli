//! Escrita de arquivo derivado, com a disciplina que o resto do pipeline já pratica.

use std::path::{Path, PathBuf};

/// Escreve `conteudo` em `path` **atomicamente**: `.tmp` irmão + `rename` (D-INC-5).
///
/// O temporário é irmão de propósito — `rename` só é atômico dentro do mesmo sistema de arquivos.
/// O sufixo é ACRESCENTADO ao nome inteiro (`x.json` → `x.json.tmp`), e não substitui a extensão:
/// assim o temporário nunca colide com um arquivo real do pipeline.
///
/// **Por que isto é um ponto único.** A assimetria entre um escritor atômico e um `fs::write` direto
/// não é teórica: o `write_manifest` era o segundo caso, e ele se manifestou numa rodada de medição
/// isolada por *hardlinks* — os packs sobreviveram porque o `rename` troca o link em vez de escrever
/// no inode, e o manifesto de produção foi alcançado pelo `fs::write`. Quatro cópias idênticas desta
/// função viviam no `auli-collections`; a quinta seria o `snapshot.json`.
///
/// **Quando usar:** todo artefato de que outro passo depende para estar íntegro, e todo artefato cuja
/// reconstrução tenha **custo externo** — se refazê-lo exige bater no servidor de outra pessoa, ele é
/// atômico. Cache descartável e saída de diagnóstico não precisam.
pub fn escrever_atomico(path: &Path, conteudo: &str) -> std::io::Result<()> {
    let tmp = PathBuf::from(format!("{}.tmp", path.display()));
    std::fs::write(&tmp, conteudo)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escreve_por_rename_e_nao_deixa_tmp_para_tras() {
        let dir = std::env::temp_dir().join(format!("auli-atomico-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let alvo = dir.join("saida.json");

        escrever_atomico(&alvo, "{}\n").unwrap();

        assert_eq!(std::fs::read_to_string(&alvo).unwrap(), "{}\n");
        assert!(
            !dir.join("saida.json.tmp").exists(),
            "o temporário tem de sumir no rename"
        );

        // Reescrever por cima substitui o conteúdo, sem sobra do anterior.
        escrever_atomico(&alvo, "novo").unwrap();
        assert_eq!(std::fs::read_to_string(&alvo).unwrap(), "novo");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
