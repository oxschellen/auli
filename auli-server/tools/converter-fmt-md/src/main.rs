//! `converter-fmt-md` — conversor **one-shot** das árvores append-only para o vocabulário
//! unificado (D-FMT-8).
//!
//! Varre `data/*/docs/{pareceres,tarf}/*.md` no formato velho e regrava no novo:
//!
//! ```text
//! numero:                →  titulo:
//! (nada)                 →  trilha:            (vazia — jurisprudência não tem classificação)
//! assunto:               →  ementa:
//! link:                  →  link:              (igual)
//! sinopse_modelo:        →  resumo_modelo:
//! sinopse_prompt_versao: →  resumo_prompt_versao:
//! sinopse_gerada_em:     →  resumo_gerada_em:
//! ## sinopse             →  ## resumo
//! ## corpo               →  ## corpo           (igual)
//! ```
//!
//! Os **valores** são preservados byte a byte: só as chaves e a âncora da seção mudam. Serviços e
//! faqs NÃO passam por aqui — suas árvores são regeneradas do zero pelo próximo `process`, já no
//! formato novo (delete + rebuild é a doutrina delas).
//!
//! **Idempotente**: um arquivo já convertido é reconhecido (parseia pelo formato novo) e pulado, e
//! rodar duas vezes é o mesmo que rodar uma. **Escrita atômica** (`.tmp` + rename), como todo o
//! resto do pipeline. Nada é apagado: em caso de dúvida, o remédio é o backup exigido pelo roteiro
//! de migração (D-FMT-9), feito ANTES desta rodada.
//!
//! Uso: `converter-fmt-md <raiz-do-data>` (ex.: `converter-fmt-md ../data`). Sem argumento, usa
//! `../data`, que é o caminho relativo de quem roda do diretório do workspace.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use auli_contract::mddoc;

/// As duas coleções de jurisprudência — as únicas append-only, e portanto as únicas que precisam de
/// conversão em vez de regeneração.
const COLECOES: &[&str] = &["pareceres", "tarf"];

fn main() -> Result<()> {
    let raiz = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "../data".to_string());
    let raiz = PathBuf::from(raiz);
    if !raiz.is_dir() {
        bail!("raiz de dados não encontrada: {}", raiz.display());
    }

    let (mut convertidos, mut ja_no_formato, mut arvores) = (0usize, 0usize, 0usize);
    let mut entidades: Vec<String> = std::fs::read_dir(&raiz)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    entidades.sort();

    for entidade in &entidades {
        for colecao in COLECOES {
            let dir = raiz.join(entidade).join("docs").join(colecao);
            if !dir.is_dir() {
                continue;
            }
            arvores += 1;
            let (c, j) = converter_arvore(&dir)?;
            convertidos += c;
            ja_no_formato += j;
            println!(
                "📄 {}: {c} convertido(s), {j} já no formato novo.",
                dir.display()
            );
        }
    }

    println!(
        "✅ {arvores} árvore(s) varrida(s): {convertidos} documento(s) convertido(s), \
         {ja_no_formato} já estavam no formato novo."
    );
    Ok(())
}

/// Converte todos os `.md` de uma árvore. Devolve `(convertidos, já_no_formato_novo)`.
fn converter_arvore(dir: &Path) -> Result<(usize, usize)> {
    let mut caminhos: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "md"))
        .collect();
    caminhos.sort();

    let (mut convertidos, mut ja) = (0usize, 0usize);
    for caminho in &caminhos {
        let texto = std::fs::read_to_string(caminho)?;
        // Idempotência: se já parseia no formato novo, não há o que fazer.
        if mddoc::parse_doc(&texto).is_ok() {
            ja += 1;
            continue;
        }
        let (header, resumo, corpo) = parse_formato_velho(&texto).with_context(|| {
            format!(
                "`{}` não parseia em nenhum dos dois formatos",
                caminho.display()
            )
        })?;
        escrever_atomico(
            caminho,
            &mddoc::render_doc(&header, resumo.as_deref(), &corpo),
        )?;
        convertidos += 1;
    }
    Ok((convertidos, ja))
}

fn escrever_atomico(destino: &Path, conteudo: &str) -> Result<()> {
    let tmp = destino.with_extension("md.tmp");
    std::fs::write(&tmp, conteudo)?;
    std::fs::rename(&tmp, destino)?;
    Ok(())
}

/// Parser do formato VELHO de jurisprudência, reimplementado aqui de propósito.
///
/// O contrato não o carrega mais (os parsers irmãos foram removidos, não depreciados — D-FMT-5), e
/// não faz sentido ressuscitá-lo lá para servir a uma ferramenta de janela única. São cinco chaves
/// e duas âncoras; quando esta ferramenta sair de cena, o formato velho existe só no histórico.
fn parse_formato_velho(texto: &str) -> Result<(mddoc::DocHeader, Option<String>, String)> {
    let t = texto.strip_prefix('\u{feff}').unwrap_or(texto);
    let apos = t
        .strip_prefix("---\n")
        .context("documento não começa com a cerca `---`")?;
    let fim = apos
        .split_inclusive('\n')
        .scan(0usize, |off, l| {
            let aqui = *off;
            *off += l.len();
            Some((aqui, l))
        })
        .find(|(_, l)| l.trim_end() == "---")
        .context("frontmatter não foi fechado por uma linha `---`")?;
    let frontmatter = &apos[..fim.0];
    let resto = &apos[fim.0 + fim.1.len()..];

    let (mut numero, mut assunto, mut link) = (None, None, None);
    let (mut modelo, mut versao, mut gerada) = (None, None, None);
    for linha in frontmatter.lines() {
        if linha.trim().is_empty() {
            continue;
        }
        let (chave, valor) = linha
            .split_once(':')
            .with_context(|| format!("linha fora da forma `chave: valor`: {linha:?}"))?;
        let valor = valor.trim().to_string();
        match chave.trim() {
            "numero" => numero = Some(valor),
            "assunto" => assunto = Some(valor),
            "link" => link = Some(valor),
            "sinopse_modelo" => modelo = Some(valor),
            "sinopse_prompt_versao" => versao = Some(valor),
            "sinopse_gerada_em" => gerada = Some(valor),
            outra => bail!("chave desconhecida no formato velho: `{outra}`"),
        }
    }

    let resumo_info = match (modelo, versao, gerada) {
        (None, None, None) => None,
        (Some(modelo), Some(versao), Some(gerada_em)) => Some(auli_contract::ResumoInfo {
            modelo,
            prompt_versao: versao
                .parse()
                .with_context(|| format!("`sinopse_prompt_versao` não é inteiro: {versao:?}"))?,
            gerada_em,
        }),
        _ => bail!("`sinopse_*` parcial — as três chaves andam juntas"),
    };

    let header = mddoc::DocHeader {
        titulo: numero.context("falta a chave `numero`")?,
        // Jurisprudência não tem classificação: a trilha nasce vazia (D-FMT-2).
        trilha: String::new(),
        ementa: assunto.context("falta a chave `assunto`")?,
        link: link.context("falta a chave `link`")?,
        // `orgao` é exclusivo de serviços, que não passam por este conversor.
        orgao: None,
        resumo_info,
    };

    let (sinopse, corpo) = separa_secoes_velhas(resto)?;
    Ok((header, sinopse, corpo))
}

/// Separa `## sinopse` (opcional) e `## corpo` (obrigatória, engole até o fim) do formato velho.
fn separa_secoes_velhas(resto: &str) -> Result<(Option<String>, String)> {
    let pos = |ancora: &str, texto: &str| -> Option<usize> {
        let mut off = 0usize;
        for l in texto.split_inclusive('\n') {
            if l.trim_end() == ancora {
                return Some(off);
            }
            off += l.len();
        }
        None
    };
    let ini_corpo = pos("## corpo", resto).context("seção `## corpo` ausente")?;
    let corpo = resto[ini_corpo..]
        .split_once('\n')
        .map(|(_, c)| c)
        .unwrap_or("")
        .trim()
        .to_string();
    let antes = &resto[..ini_corpo];
    let sinopse = pos("## sinopse", antes).map(|i| {
        antes[i..]
            .split_once('\n')
            .map(|(_, s)| s)
            .unwrap_or("")
            .trim()
            .to_string()
    });
    Ok((sinopse, corpo))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Um documento no formato velho, com sinopse e proveniência — o caso completo.
    const VELHO_COM_SINOPSE: &str = "\
---
numero: PARECER Nº 25148
assunto: ICMS – crédito fiscal na cesta básica
link: http://legislacao/25148
sinopse_modelo: llama-3.3-70b-versatile
sinopse_prompt_versao: 1
sinopse_gerada_em: 2026-07-18T14:00:00Z
---

## sinopse
### Descrição Resumida do Assunto
Analisa o crédito fiscal.

## corpo
PARECER Nº 25148

É o parecer.
";

    /// Um documento pendente: sem as chaves `sinopse_*` e sem a seção.
    const VELHO_PENDENTE: &str = "\
---
numero: ACÓRDÃO 141/26
assunto: ITCD. DOAÇÃO DISSIMULADA.
link: https://tarf.sefaz.rs.gov.br/documento/abc
---

## corpo
Corpo do acórdão.
";

    #[test]
    fn converte_preservando_os_valores_e_a_proveniencia() {
        let (h, resumo, corpo) = parse_formato_velho(VELHO_COM_SINOPSE).unwrap();
        assert_eq!(h.titulo, "PARECER Nº 25148");
        assert_eq!(h.ementa, "ICMS – crédito fiscal na cesta básica");
        assert_eq!(h.trilha, "", "jurisprudência não tem trilha");
        assert_eq!(h.link, "http://legislacao/25148");
        assert_eq!(h.orgao, None);
        let info = h.resumo_info.as_ref().unwrap();
        assert_eq!(info.modelo, "llama-3.3-70b-versatile");
        assert_eq!(info.prompt_versao, 1);
        assert_eq!(info.gerada_em, "2026-07-18T14:00:00Z");
        assert_eq!(
            resumo.as_deref(),
            Some("### Descrição Resumida do Assunto\nAnalisa o crédito fiscal.")
        );
        assert_eq!(corpo, "PARECER Nº 25148\n\nÉ o parecer.");
    }

    #[test]
    fn equivalencia_de_campos_entre_os_dois_formatos() {
        // O teste que D-FMT-8 exige: o que sai do parser velho e o que volta do parser novo, depois
        // de renderizado, são os MESMOS campos. Se um dia divergirem, a conversão está perdendo
        // dado — e é aqui que isso tem que doer.
        for velho in [VELHO_COM_SINOPSE, VELHO_PENDENTE] {
            let (h, resumo, corpo) = parse_formato_velho(velho).unwrap();
            let novo = mddoc::render_doc(&h, resumo.as_deref(), &corpo);
            let (h2, resumo2, corpo2) = mddoc::parse_doc(&novo).unwrap();
            assert_eq!(h2, h);
            assert_eq!(resumo2, resumo);
            assert_eq!(corpo2, corpo);
        }
    }

    #[test]
    fn pendente_continua_pendente_depois_de_convertido() {
        // A pendência é estado legal e informa a guarda do update: converter não pode inventar
        // resumo nem apagar o que existe.
        let (h, resumo, _) = parse_formato_velho(VELHO_PENDENTE).unwrap();
        assert_eq!(resumo, None);
        assert_eq!(h.resumo_info, None);
        let novo = mddoc::render_doc(&h, None, "Corpo do acórdão.");
        assert!(!novo.contains("## resumo"));
        assert!(!novo.contains("resumo_modelo"));
    }

    #[test]
    fn documento_no_formato_novo_nao_parseia_como_velho() {
        // É o que sustenta a idempotência: o conversor tenta o novo primeiro e só cai no velho se
        // aquele falhar. As chaves novas são desconhecidas para o parser velho.
        let (h, resumo, corpo) = parse_formato_velho(VELHO_COM_SINOPSE).unwrap();
        let novo = mddoc::render_doc(&h, resumo.as_deref(), &corpo);
        assert!(parse_formato_velho(&novo).is_err());
        assert!(mddoc::parse_doc(&novo).is_ok());
    }

    #[test]
    fn arvore_convertida_e_idempotente() {
        let dir = std::env::temp_dir().join(format!("auli-conv-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.md"), VELHO_COM_SINOPSE).unwrap();
        std::fs::write(dir.join("b.md"), VELHO_PENDENTE).unwrap();

        assert_eq!(converter_arvore(&dir).unwrap(), (2, 0));
        let depois = std::fs::read_to_string(dir.join("a.md")).unwrap();
        assert!(depois.contains("titulo: PARECER Nº 25148"));
        assert!(depois.contains("trilha: \n"));
        assert!(depois.contains("resumo_modelo: llama-3.3-70b-versatile"));
        assert!(depois.contains("## resumo\n"));
        assert!(!depois.contains("numero:") && !depois.contains("## sinopse"));

        // 2ª passada: nada a fazer, e o conteúdo não muda.
        assert_eq!(converter_arvore(&dir).unwrap(), (0, 2));
        assert_eq!(std::fs::read_to_string(dir.join("a.md")).unwrap(), depois);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
