//! `mddoc_servico` — o contrato do arquivo `.md` que é a **fonte** de um serviço.
//!
//! Irmão do [`crate::mddoc`] (pareceres), com as diferenças do domínio:
//!
//! - **Sem `## sinopse`**: documento = frontmatter + `## corpo` (a descrição).
//! - **Árvore regenerada a cada coleta** (delete + rebuild): serviços mudam no
//!   portal, então não há incremental — ver [`materializar_arvore`].
//! - Frontmatter **achatado**: só a ocorrência primária `(tipo, classe)`, como o
//!   `Table<Servico>` já materializa.
//!
//! ```text
//! ---
//! titulo: Emissão de Guia de IPVA
//! tipo: Cidadãos
//! classe: IPVA
//! orgao: Receita Estadual
//! link: https://atendimento.receita.rs.gov.br/emissao-guia-ipva
//! ---
//!
//! ## corpo
//! <descrição integral — última seção, engole tudo até o fim>
//! ```
//!
//! Mesma forma canônica estrita do `mddoc`: somos o único escritor e o único
//! leitor; um arquivo "quase certo" falha ruidosamente.

use std::path::Path;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use crate::mddoc::{ANCORA_CORPO, CERCA, colapsa_linha, separa_frontmatter, slug};

/// Cabeçalho tipado do documento de serviço — a ocorrência primária achatada.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServicoHeader {
    pub titulo: String,
    pub tipo: String,
    pub classe: String,
    pub orgao: String,
    pub link: String,
}

/// Parseia um documento inteiro: `(cabeçalho, corpo)`.
///
/// O `corpo` é a última (e única) seção e engole tudo até o fim do arquivo — nada
/// dentro dele quebra o parse. Corpo vazio é válido (serviço sem descrição).
pub fn parse_doc_servico(texto: &str) -> Result<(ServicoHeader, String)> {
    let (frontmatter, resto) = separa_frontmatter(texto)?;
    let header = parse_frontmatter_servico(frontmatter)?;
    let corpo = separa_corpo(resto)?;
    Ok((header, corpo))
}

/// Parser canônico do frontmatter de serviço: `chave: valor` por linha, estrito.
fn parse_frontmatter_servico(fm: &str) -> Result<ServicoHeader> {
    let (mut titulo, mut tipo, mut classe, mut orgao, mut link) = (None, None, None, None, None);

    for (i, linha) in fm.lines().enumerate() {
        let n = i + 1;
        if linha.trim().is_empty() {
            continue;
        }
        let Some((chave, valor)) = linha.split_once(':') else {
            bail!("frontmatter linha {n}: fora da forma canônica `chave: valor` — {linha:?}");
        };
        let chave = chave.trim();
        let valor = valor.trim();
        let poe = |alvo: &mut Option<String>| -> Result<()> {
            if alvo.is_some() {
                bail!("frontmatter linha {n}: chave duplicada `{chave}`");
            }
            *alvo = Some(valor.to_string());
            Ok(())
        };
        match chave {
            "titulo" => poe(&mut titulo)?,
            "tipo" => poe(&mut tipo)?,
            "classe" => poe(&mut classe)?,
            "orgao" => poe(&mut orgao)?,
            "link" => poe(&mut link)?,
            outra => bail!("frontmatter linha {n}: chave desconhecida `{outra}`"),
        }
    }

    let exige = |o: Option<String>, nome: &str| -> Result<String> {
        o.ok_or_else(|| anyhow::anyhow!("frontmatter sem a chave obrigatória `{nome}`"))
    };
    Ok(ServicoHeader {
        titulo: exige(titulo, "titulo")?,
        tipo: exige(tipo, "tipo")?,
        classe: exige(classe, "classe")?,
        orgao: exige(orgao, "orgao")?,
        link: exige(link, "link")?,
    })
}

/// Acha a âncora `## corpo` e devolve tudo depois dela (aparado). Antes da âncora
/// só linhas em branco são toleradas — conteúdo perdido ali seria engolido em
/// silêncio, então é erro alto.
fn separa_corpo(resto: &str) -> Result<String> {
    let mut offset = 0usize;
    for linha in resto.split_inclusive('\n') {
        if linha.trim_end() == ANCORA_CORPO {
            return Ok(resto[offset + linha.len()..].trim().to_string());
        }
        if !linha.trim().is_empty() {
            bail!("conteúdo inesperado antes de `{ANCORA_CORPO}`: {:?}", linha.trim_end());
        }
        offset += linha.len();
    }
    bail!("documento sem a seção `{ANCORA_CORPO}`");
}

/// Renderiza o documento inteiro a partir das partes. Inverso exato de
/// [`parse_doc_servico`] (round-trip nos testes).
pub fn render_doc_servico(header: &ServicoHeader, corpo: &str) -> String {
    let mut out = String::with_capacity(corpo.len() + 256);
    out.push_str(CERCA);
    out.push('\n');
    out.push_str(&format!("titulo: {}\n", colapsa_linha(&header.titulo)));
    out.push_str(&format!("tipo: {}\n", colapsa_linha(&header.tipo)));
    out.push_str(&format!("classe: {}\n", colapsa_linha(&header.classe)));
    out.push_str(&format!("orgao: {}\n", colapsa_linha(&header.orgao)));
    out.push_str(&format!("link: {}\n", colapsa_linha(&header.link)));
    out.push_str(CERCA);
    out.push('\n');
    out.push_str(&format!("\n{ANCORA_CORPO}\n{}\n", corpo.trim()));
    out
}

/// FNV-1a 64 bits. Gêmeo do `auli_core::manifest::fnv1a64` — duplicado de
/// propósito: este crate é "leve" (só serde/serde_json + anyhow/time) e não pode
/// depender do `auli-core`. Se as constantes mudarem lá, NÃO precisam mudar aqui:
/// este hash só nomeia arquivos desta árvore.
fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Nome de arquivo do serviço: `slug(titulo)-<hash8(link)>`. O hash curto (8 hex
/// do FNV-1a 64 truncado) desambigua títulos repetidos e sobrevive a renomeações
/// de título só até o próximo rebuild — a identidade real é o `link`. Título sem
/// slug (só símbolos) fica só com o hash.
pub fn slug_servico(titulo: &str, link: &str) -> String {
    let s = slug(titulo);
    let h = format!("{:08x}", fnv1a64(link.as_bytes()) as u32);
    if s.is_empty() { h } else { format!("{s}-{h}") }
}

/// **Regenera a árvore inteira**: apaga `dir` (se existir), recria e grava um
/// `<slug_servico>.md` por item. Colisão de nome de arquivo é violação de
/// identidade — detectada ANTES de escrever qualquer arquivo, nomeando os dois
/// títulos em conflito. Devolve o total gravado.
///
/// Delete + rebuild é a doutrina dos serviços (mutáveis no portal): o estado da
/// árvore após esta função é função SÓ da coleta, nunca do que havia antes.
pub fn materializar_arvore(dir: &Path, docs: &[(ServicoHeader, String)]) -> Result<usize> {
    // 1. Pré-cheque de colisão, com o par ofensor no erro.
    let mut vistos: std::collections::HashMap<String, &str> =
        std::collections::HashMap::with_capacity(docs.len());
    for (header, _) in docs {
        let nome = slug_servico(&header.titulo, &header.link);
        if let Some(anterior) = vistos.insert(nome.clone(), &header.titulo) {
            bail!(
                "colisão de nome de arquivo: {anterior:?} e {:?} geram o mesmo `{nome}.md`",
                header.titulo
            );
        }
    }
    // 2. Delete + rebuild.
    if dir.exists() {
        std::fs::remove_dir_all(dir)?;
    }
    std::fs::create_dir_all(dir)?;
    for (header, corpo) in docs {
        let nome = slug_servico(&header.titulo, &header.link);
        std::fs::write(dir.join(format!("{nome}.md")), render_doc_servico(header, corpo))?;
    }
    Ok(docs.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(titulo: &str, link: &str) -> ServicoHeader {
        ServicoHeader {
            titulo: titulo.into(),
            tipo: "Cidadãos".into(),
            classe: "IPVA".into(),
            orgao: "Receita Estadual".into(),
            link: link.into(),
        }
    }

    fn tempdir(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir()
            .join(format!("auli-mddoc-servico-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn round_trip() {
        let h = header("Emissão de Guia de IPVA", "https://x/svc/1");
        // Pegadinhas DENTRO do corpo: cerca e âncora não podem quebrar o parse.
        let corpo = "Passo 1.\n---\n## corpo\ntitulo: falso\nFim.";
        let texto = render_doc_servico(&h, corpo);
        let (h2, corpo2) = parse_doc_servico(&texto).unwrap();
        assert_eq!(h2, h);
        assert_eq!(corpo2, corpo);
        // Idempotente: renderizar o que foi parseado dá o mesmo texto.
        assert_eq!(render_doc_servico(&h2, &corpo2), texto);
    }

    #[test]
    fn corpo_vazio_e_valido() {
        // Há serviços sem descrição no portal — a seção existe, vazia.
        let h = header("Serviço sem descrição", "https://x/svc/2");
        let (h2, corpo) = parse_doc_servico(&render_doc_servico(&h, "")).unwrap();
        assert_eq!(h2, h);
        assert_eq!(corpo, "");
    }

    #[test]
    fn recusa_chave_desconhecida() {
        let texto = "---\ntitulo: T\ntipo: C\nclasse: K\norgao: O\nlink: L\nresumo: nao vai aqui\n---\n\n## corpo\nc\n";
        let e = parse_doc_servico(texto).unwrap_err().to_string();
        assert!(e.contains("desconhecida") && e.contains("resumo"), "erro: {e}");
    }

    #[test]
    fn recusa_chave_duplicada() {
        let texto = "---\ntitulo: T\ntitulo: T2\ntipo: C\nclasse: K\norgao: O\nlink: L\n---\n\n## corpo\nc\n";
        let e = parse_doc_servico(texto).unwrap_err().to_string();
        assert!(e.contains("duplicada"), "erro: {e}");
    }

    #[test]
    fn recusa_chave_faltante() {
        let texto = "---\ntipo: C\nclasse: K\norgao: O\nlink: L\n---\n\n## corpo\nc\n";
        let e = parse_doc_servico(texto).unwrap_err().to_string();
        assert!(e.contains("titulo"), "erro: {e}");
    }

    #[test]
    fn recusa_sem_corpo() {
        let texto = "---\ntitulo: T\ntipo: C\nclasse: K\norgao: O\nlink: L\n---\n";
        let e = parse_doc_servico(texto).unwrap_err().to_string();
        assert!(e.contains("## corpo"), "erro: {e}");
    }

    #[test]
    fn recusa_conteudo_antes_do_corpo() {
        // `## sinopse` é justamente o que serviço NÃO tem: se aparecer, é erro alto,
        // não conteúdo engolido em silêncio.
        let texto = "---\ntitulo: T\ntipo: C\nclasse: K\norgao: O\nlink: L\n---\n\n## sinopse\nx\n\n## corpo\nc\n";
        let e = parse_doc_servico(texto).unwrap_err().to_string();
        assert!(e.contains("inesperado") && e.contains("sinopse"), "erro: {e}");
    }

    #[test]
    fn slug_servico_determinismo_e_distincao() {
        // Determinístico: mesmo (titulo, link) ⇒ mesmo nome.
        assert_eq!(
            slug_servico("Emissão de Guia", "https://x/1"),
            slug_servico("Emissão de Guia", "https://x/1")
        );
        // O hash do link desambigua títulos repetidos — o caso que a árvore precisa suportar.
        assert_ne!(
            slug_servico("Emissão de Guia", "https://x/1"),
            slug_servico("Emissão de Guia", "https://x/2")
        );
        assert!(slug_servico("Emissão de Guia", "https://x/1").starts_with("emissao-de-guia-"));
        // Título só de símbolos: a identidade vem do link, o nome é só o hash.
        let so_hash = slug_servico("§§§", "https://x/3");
        assert_eq!(so_hash.len(), 8);
        assert!(so_hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn materializar_apaga_orfaos() {
        // A doutrina do delete+rebuild: serviço removido do portal some da árvore.
        let dir = tempdir("orfaos");
        let a = (header("Serviço A", "https://x/a"), "corpo a".to_string());
        let b = (header("Serviço B", "https://x/b"), "corpo b".to_string());
        assert_eq!(materializar_arvore(&dir, &[a.clone(), b.clone()]).unwrap(), 2);
        let arquivo_b = dir.join(format!("{}.md", slug_servico(&b.0.titulo, &b.0.link)));
        assert!(arquivo_b.exists());

        assert_eq!(materializar_arvore(&dir, std::slice::from_ref(&a)).unwrap(), 1);
        assert!(!arquivo_b.exists(), "órfão deve sumir no rebuild");
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn materializar_recusa_colisao() {
        let dir = tempdir("colisao");
        let docs = vec![
            (header("Mesmo", "https://x/igual"), "a".to_string()),
            (header("Mesmo", "https://x/igual"), "b".to_string()),
        ];
        let e = materializar_arvore(&dir, &docs).unwrap_err().to_string();
        assert!(e.contains("colisão de nome de arquivo"), "erro: {e}");
        assert!(e.matches("Mesmo").count() >= 2, "deve nomear os dois: {e}");
        // Pré-cheque roda ANTES do delete: o diretório não foi tocado.
        assert!(!dir.exists());
    }
}
