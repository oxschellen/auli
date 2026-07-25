//! `mddoc_faq` — o contrato do arquivo `.md` que é a **fonte** de uma FAQ.
//! Terceiro irmão de [`crate::mddoc`] (pareceres) e [`crate::mddoc_servico`].
//!
//! - **Sem `## sinopse`**: documento = frontmatter + `## corpo` (a resposta).
//! - **Árvore regenerada a cada coleta** (delete + rebuild), como os serviços —
//!   ver [`materializar_arvore`].
//! - **Identidade = `(url, pergunta)`**: a `url` é da página de origem e NÃO é
//!   única — várias perguntas moram na mesma página. Por isso o hash do nome de
//!   arquivo cobre `url + pergunta`, não só a url.
//!
//! ```text
//! ---
//! pergunta: Como faço para emitir a guia de IPVA?
//! origin: Inicial | Perguntas Frequentes | IPVA
//! url: https://atendimento.receita.rs.gov.br/faq-ipva
//! ---
//!
//! ## corpo
//! <resposta integral — última seção, engole tudo até o fim>
//! ```
//!
//! Mesma forma canônica estrita dos irmãos: somos o único escritor e o único
//! leitor; um arquivo "quase certo" falha ruidosamente. Os valores do
//! frontmatter passam por [`crate::mddoc::colapsa_linha`] — um valor de
//! frontmatter tem que caber em UMA linha —, então o round-trip pela árvore é
//! lossy para espaço redundante dentro da pergunta.

use std::path::Path;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use crate::mddoc::{ANCORA_CORPO, CERCA, colapsa_linha, separa_frontmatter, slug, slug_truncado};
use crate::mddoc_servico::fnv1a64;

/// Cabeçalho tipado do documento de FAQ.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FaqHeader {
    pub pergunta: String,
    /// Breadcrumb da página. Pode ser vazio — a chave é obrigatória no arquivo,
    /// o VALOR pode ser vazio (espelha o `#[serde(default)]` do contrato).
    pub origin: String,
    pub url: String,
}

/// Parseia um documento inteiro: `(cabeçalho, corpo)`.
///
/// O `corpo` é a resposta: última (e única) seção, engole tudo até o fim do
/// arquivo — nada dentro dele quebra o parse. Corpo vazio é válido.
pub fn parse_doc_faq(texto: &str) -> Result<(FaqHeader, String)> {
    let (frontmatter, resto) = separa_frontmatter(texto)?;
    let header = parse_frontmatter_faq(frontmatter)?;
    let corpo = separa_corpo(resto)?;
    Ok((header, corpo))
}

/// Parser canônico do frontmatter de FAQ: `chave: valor` por linha, estrito.
///
/// As três chaves são obrigatórias, mas a obrigatoriedade é da CHAVE: valor
/// vazio é legal (o caso real é `origin`; `pergunta`/`url` vazias o scraper
/// nunca produz).
fn parse_frontmatter_faq(fm: &str) -> Result<FaqHeader> {
    let (mut pergunta, mut origin, mut url) = (None, None, None);

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
            "pergunta" => poe(&mut pergunta)?,
            "origin" => poe(&mut origin)?,
            "url" => poe(&mut url)?,
            outra => bail!("frontmatter linha {n}: chave desconhecida `{outra}`"),
        }
    }

    let exige = |o: Option<String>, nome: &str| -> Result<String> {
        o.ok_or_else(|| anyhow::anyhow!("frontmatter sem a chave obrigatória `{nome}`"))
    };
    Ok(FaqHeader {
        pergunta: exige(pergunta, "pergunta")?,
        origin: exige(origin, "origin")?,
        url: exige(url, "url")?,
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
/// [`parse_doc_faq`] (round-trip nos testes).
///
/// A linha `origin` é SEMPRE escrita, mesmo com valor vazio: a chave é parte da
/// forma canônica, e chave ausente é erro no parser.
pub fn render_doc_faq(header: &FaqHeader, corpo: &str) -> String {
    let mut out = String::with_capacity(corpo.len() + 256);
    out.push_str(CERCA);
    out.push('\n');
    out.push_str(&format!("pergunta: {}\n", colapsa_linha(&header.pergunta)));
    out.push_str(&format!("origin: {}\n", colapsa_linha(&header.origin)));
    out.push_str(&format!("url: {}\n", colapsa_linha(&header.url)));
    out.push_str(CERCA);
    out.push('\n');
    out.push_str(&format!("\n{ANCORA_CORPO}\n{}\n", corpo.trim()));
    out
}

/// Nome de arquivo da FAQ: `slug(pergunta)` truncado + `-` + hash8 de
/// `url + "\n" + pergunta`. O truncamento (60 chars, cortado na última fronteira
/// de hífen dentro do limite) mantém o nome legível sem estourar o filesystem —
/// a identidade real está no hash, que cobre o par `(url, pergunta)`, porque a
/// url sozinha não distingue perguntas da mesma página.
pub fn slug_faq(pergunta: &str, url: &str) -> String {
    let s = slug(pergunta);
    let s = slug_truncado(&s, 60);
    let h = format!("{:08x}", fnv1a64(format!("{url}\n{pergunta}").as_bytes()) as u32);
    if s.is_empty() { h } else { format!("{s}-{h}") }
}

/// **Regenera a árvore inteira**: apaga `dir` (se existir), recria e grava um
/// `<slug_faq>.md` por item. Colisão de nome de arquivo é violação de identidade
/// — detectada ANTES de escrever qualquer arquivo, nomeando as duas perguntas em
/// conflito. Devolve o total gravado.
///
/// Delete + rebuild é a doutrina das faqs (mutáveis no portal, como os serviços):
/// o estado da árvore após esta função é função SÓ da coleta, nunca do que havia
/// antes.
pub fn materializar_arvore(dir: &Path, docs: &[(FaqHeader, String)]) -> Result<usize> {
    // 1. Pré-cheque de colisão, com o par ofensor no erro.
    let mut vistos: std::collections::HashMap<String, &str> =
        std::collections::HashMap::with_capacity(docs.len());
    for (header, _) in docs {
        let nome = slug_faq(&header.pergunta, &header.url);
        if let Some(anterior) = vistos.insert(nome.clone(), &header.pergunta) {
            bail!(
                "colisão de nome de arquivo: {anterior:?} e {:?} geram o mesmo `{nome}.md`",
                header.pergunta
            );
        }
    }
    // 2. Delete + rebuild.
    if dir.exists() {
        std::fs::remove_dir_all(dir)?;
    }
    std::fs::create_dir_all(dir)?;
    for (header, corpo) in docs {
        let nome = slug_faq(&header.pergunta, &header.url);
        std::fs::write(dir.join(format!("{nome}.md")), render_doc_faq(header, corpo))?;
    }
    Ok(docs.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(pergunta: &str, url: &str) -> FaqHeader {
        FaqHeader {
            pergunta: pergunta.into(),
            origin: "Inicial | Perguntas Frequentes | IPVA".into(),
            url: url.into(),
        }
    }

    fn tempdir(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("auli-mddoc-faq-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn round_trip() {
        let h = header("Como faço para emitir a guia de IPVA?", "https://x/faq-ipva");
        // Pegadinhas DENTRO do corpo: cerca e âncora não podem quebrar o parse.
        let corpo = "Acesse o portal.\n---\n## corpo\npergunta: falsa\nFim.";
        let texto = render_doc_faq(&h, corpo);
        let (h2, corpo2) = parse_doc_faq(&texto).unwrap();
        assert_eq!(h2, h);
        assert_eq!(corpo2, corpo);
        // Idempotente: renderizar o que foi parseado dá o mesmo texto.
        assert_eq!(render_doc_faq(&h2, &corpo2), texto);
    }

    #[test]
    fn round_trip_com_origin_vazio() {
        // O caso próprio das faqs: breadcrumb ausente na coleta. A CHAVE continua
        // no arquivo (D3) e o valor vazio sobrevive ao round-trip.
        let mut h = header("Onde consulto meu débito?", "https://x/faq-1");
        h.origin = String::new();
        let texto = render_doc_faq(&h, "No portal.");
        assert!(texto.contains("\norigin:"), "a chave `origin` é sempre escrita:\n{texto}");
        let (h2, corpo) = parse_doc_faq(&texto).unwrap();
        assert_eq!(h2.origin, "");
        assert_eq!(h2, h);
        assert_eq!(corpo, "No portal.");
    }

    #[test]
    fn corpo_vazio_e_valido() {
        let h = header("Pergunta sem resposta publicada", "https://x/faq-2");
        let (h2, corpo) = parse_doc_faq(&render_doc_faq(&h, "")).unwrap();
        assert_eq!(h2, h);
        assert_eq!(corpo, "");
    }

    #[test]
    fn recusa_chave_desconhecida() {
        let texto = "---\npergunta: P\norigin: O\nurl: U\nresposta: nao vai aqui\n---\n\n## corpo\nc\n";
        let e = parse_doc_faq(texto).unwrap_err().to_string();
        assert!(e.contains("desconhecida") && e.contains("resposta"), "erro: {e}");
    }

    #[test]
    fn recusa_chave_duplicada() {
        let texto = "---\npergunta: P\npergunta: P2\norigin: O\nurl: U\n---\n\n## corpo\nc\n";
        let e = parse_doc_faq(texto).unwrap_err().to_string();
        assert!(e.contains("duplicada"), "erro: {e}");
    }

    #[test]
    fn recusa_chave_faltante() {
        let texto = "---\norigin: O\nurl: U\n---\n\n## corpo\nc\n";
        let e = parse_doc_faq(texto).unwrap_err().to_string();
        assert!(e.contains("pergunta"), "erro: {e}");
        // Chave AUSENTE ≠ valor vazio: `origin: ` passa, `origin` ausente não.
        let texto = "---\npergunta: P\nurl: U\n---\n\n## corpo\nc\n";
        let e = parse_doc_faq(texto).unwrap_err().to_string();
        assert!(e.contains("origin"), "erro: {e}");
        let texto = "---\npergunta: P\norigin: \nurl: U\n---\n\n## corpo\nc\n";
        assert_eq!(parse_doc_faq(texto).unwrap().0.origin, "");
    }

    #[test]
    fn recusa_sem_corpo() {
        let texto = "---\npergunta: P\norigin: O\nurl: U\n---\n";
        let e = parse_doc_faq(texto).unwrap_err().to_string();
        assert!(e.contains("## corpo"), "erro: {e}");
    }

    #[test]
    fn recusa_conteudo_antes_do_corpo() {
        // `## sinopse` é justamente o que a FAQ NÃO tem: se aparecer, é erro alto,
        // não conteúdo engolido em silêncio.
        let texto = "---\npergunta: P\norigin: O\nurl: U\n---\n\n## sinopse\nx\n\n## corpo\nc\n";
        let e = parse_doc_faq(texto).unwrap_err().to_string();
        assert!(e.contains("inesperado") && e.contains("sinopse"), "erro: {e}");
    }

    #[test]
    fn slug_faq_identidade_e_truncamento() {
        // Determinístico.
        assert_eq!(slug_faq("Como emitir?", "https://x/1"), slug_faq("Como emitir?", "https://x/1"));
        // (a) mesma pergunta em urls diferentes ⇒ nomes diferentes.
        assert_ne!(slug_faq("Como emitir?", "https://x/1"), slug_faq("Como emitir?", "https://x/2"));
        // (b) perguntas diferentes na MESMA url ⇒ nomes diferentes. É o caso que a
        // identidade `(url, pergunta)` existe para cobrir.
        assert_ne!(slug_faq("Como emitir?", "https://x/1"), slug_faq("Onde pagar?", "https://x/1"));
        assert!(slug_faq("Como emitir?", "https://x/1").starts_with("como-emitir-"));

        // (c) pergunta longa: nome ≤ 60 + 1 + 8, cortado em fronteira de hífen.
        let longa = "Como faço para emitir a guia de recolhimento do IPVA atrasado de anos anteriores?";
        let nome = slug_faq(longa, "https://x/3");
        assert!(nome.len() <= 69, "nome longo demais ({}): {nome}", nome.len());
        let trecho = &nome[..nome.len() - 9]; // tira o `-hash8`
        assert!(!trecho.ends_with('-'), "hífen pendurado no truncamento: {nome}");
        assert!(slug(longa).starts_with(trecho), "truncamento deve ser prefixo do slug: {nome}");
        // Truncou mesmo (o slug inteiro passa de 60).
        assert!(slug(longa).len() > 60);

        // (d) pergunta só de símbolos: a identidade vem do hash, o nome é só ele.
        let so_hash = slug_faq("§§§", "https://x/4");
        assert_eq!(so_hash.len(), 8);
        assert!(so_hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn materializar_apaga_orfaos() {
        // A doutrina do delete+rebuild: pergunta removida do portal some da árvore.
        let dir = tempdir("orfaos");
        let a = (header("Pergunta A", "https://x/a"), "resposta a".to_string());
        let b = (header("Pergunta B", "https://x/a"), "resposta b".to_string());
        assert_eq!(materializar_arvore(&dir, &[a.clone(), b.clone()]).unwrap(), 2);
        let arquivo_b = dir.join(format!("{}.md", slug_faq(&b.0.pergunta, &b.0.url)));
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
            (header("Mesma pergunta", "https://x/igual"), "a".to_string()),
            (header("Mesma pergunta", "https://x/igual"), "b".to_string()),
        ];
        let e = materializar_arvore(&dir, &docs).unwrap_err().to_string();
        assert!(e.contains("colisão de nome de arquivo"), "erro: {e}");
        assert!(e.matches("Mesma pergunta").count() >= 2, "deve nomear as duas: {e}");
        // Pré-cheque roda ANTES do delete: o diretório não foi tocado.
        assert!(!dir.exists());
    }
}
