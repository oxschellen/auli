//! `mddoc` — o contrato do arquivo `.md` que é a **fonte** de um parecer/consulta.
//!
//! Um arquivo por documento: frontmatter tipado + `## sinopse` (opcional) + `## corpo`.
//!
//! ```text
//! ---
//! numero: CONSULTA COPAT nº 0037/26
//! assunto: ICMS. REGIME DE SUBSTITUIÇÃO TRIBUTÁRIA...
//! link: https://legislacao.sef.sc.gov.br/...
//! sinopse_modelo: llama-3.3-70b-versatile
//! sinopse_prompt_versao: 1
//! sinopse_gerada_em: 2026-07-18T14:00:00Z
//! ---
//!
//! ## sinopse
//! ### Descrição Resumida do Assunto
//! ...
//!
//! ## corpo
//! <texto oficial integral — última seção, engole tudo até o fim>
//! ```
//!
//! **Parser canônico próprio, sem dependência nova.** O frontmatter é a *nossa* forma canônica
//! (`chave: valor`, uma por linha, valor = tudo após o primeiro `: `), não YAML arbitrário: somos o
//! único escritor e o único leitor. `serde_yaml` está arquivado/não-mantido (RUSTSEC, 2024) e
//! violaria o invariante deste crate ("leve: só serde/serde_json + anyhow/time"). O parser é
//! **estrito** — chave desconhecida, duplicada, linha fora da forma ou `sinopse_*` parcial ⇒ erro
//! alto: um arquivo "quase certo" falha ruidosamente em vez de parsear silenciosamente errado.
//!
//! Bônus preservado: a forma canônica plana **continua sendo YAML válido**, então Obsidian, pandoc e
//! geradores de site leem o frontmatter normalmente — nós é que não dependemos de um parser de YAML.

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use crate::SinopseInfo;

/// Delimitador do frontmatter e âncoras das seções (início de linha).
const CERCA: &str = "---";
const ANCORA_SINOPSE: &str = "## sinopse";
const ANCORA_CORPO: &str = "## corpo";

/// Cabeçalho tipado do documento — o que vive no frontmatter.
///
/// `sinopse_info` ausente = **sinopse pendente**. No arquivo ele é achatado em três chaves planas
/// (`sinopse_modelo`, `sinopse_prompt_versao`, `sinopse_gerada_em`) — sem aninhamento, que é a única
/// coisa que complicaria o parser. As três andam juntas: estado misto é erro.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocHeader {
    pub numero: String,
    pub assunto: String,
    pub link: String,
    pub sinopse_info: Option<SinopseInfo>,
}

/// Colapsa o valor em **linha única**: qualquer corrida de espaços/quebras vira um espaço só.
/// Os valores do frontmatter são escalares de uma linha; as ementas já vivem assim no `.txt`.
fn colapsa_linha(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Parseia um documento inteiro: `(cabeçalho, sinopse, corpo)`.
///
/// `sinopse` é `None` quando a seção `## sinopse` está ausente (pendente). O `corpo` é a última
/// seção e engole tudo até o fim do arquivo — por isso **nada dentro dele quebra o parse**, nem
/// linhas que pareçam âncoras ou cercas.
pub fn parse_doc(texto: &str) -> Result<(DocHeader, Option<String>, String)> {
    let (frontmatter, resto) = separa_frontmatter(texto)?;
    let header = parse_frontmatter(frontmatter)?;
    let (sinopse, corpo) = separa_secoes(resto)?;
    Ok((header, sinopse, corpo))
}

/// Fatia o bloco entre as duas cercas `---`. Devolve `(frontmatter, resto_do_arquivo)`.
fn separa_frontmatter(texto: &str) -> Result<(&str, &str)> {
    let t = texto.strip_prefix('\u{feff}').unwrap_or(texto); // tolera BOM
    let Some(apos_abertura) = t.strip_prefix(CERCA).and_then(|r| r.strip_prefix('\n')) else {
        bail!("documento não começa com a cerca de frontmatter `---` em linha própria");
    };
    // Fecha na primeira linha que seja exatamente `---`.
    let mut offset = 0usize;
    for linha in apos_abertura.split_inclusive('\n') {
        if linha.trim_end() == CERCA {
            let fm = &apos_abertura[..offset];
            let resto = &apos_abertura[offset + linha.len()..];
            return Ok((fm, resto));
        }
        offset += linha.len();
    }
    bail!("frontmatter não foi fechado por uma linha `---`");
}

/// Parser canônico do frontmatter: `chave: valor` por linha, estrito.
fn parse_frontmatter(fm: &str) -> Result<DocHeader> {
    let (mut numero, mut assunto, mut link) = (None, None, None);
    let (mut s_modelo, mut s_versao, mut s_gerada) = (None, None, None);

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
        // Fecha a porta para chave duplicada em cada braço (o `já definida` abaixo).
        let poe = |alvo: &mut Option<String>| -> Result<()> {
            if alvo.is_some() {
                bail!("frontmatter linha {n}: chave duplicada `{chave}`");
            }
            *alvo = Some(valor.to_string());
            Ok(())
        };
        match chave {
            "numero" => poe(&mut numero)?,
            "assunto" => poe(&mut assunto)?,
            "link" => poe(&mut link)?,
            "sinopse_modelo" => poe(&mut s_modelo)?,
            "sinopse_gerada_em" => poe(&mut s_gerada)?,
            "sinopse_prompt_versao" => poe(&mut s_versao)?,
            outra => bail!("frontmatter linha {n}: chave desconhecida `{outra}`"),
        }
    }

    let obrigatoria = |campo: Option<String>, nome: &str| -> Result<String> {
        campo.ok_or_else(|| anyhow::anyhow!("frontmatter: falta a chave obrigatória `{nome}`"))
    };

    // As três `sinopse_*` andam juntas: ou nenhuma (pendente) ou todas.
    let sinopse_info = match (s_modelo, s_versao, s_gerada) {
        (None, None, None) => None,
        (Some(modelo), Some(versao), Some(gerada_em)) => {
            let prompt_versao: u32 = versao.parse().map_err(|_| {
                anyhow::anyhow!("frontmatter: `sinopse_prompt_versao` não é inteiro — {versao:?}")
            })?;
            Some(SinopseInfo { modelo, prompt_versao, gerada_em })
        }
        _ => bail!(
            "frontmatter: `sinopse_*` parcial — as três chaves (sinopse_modelo, \
             sinopse_prompt_versao, sinopse_gerada_em) devem estar todas presentes ou todas ausentes"
        ),
    };

    Ok(DocHeader {
        numero: obrigatoria(numero, "numero")?,
        assunto: obrigatoria(assunto, "assunto")?,
        link: obrigatoria(link, "link")?,
        sinopse_info,
    })
}

/// Separa `## sinopse` (opcional) e `## corpo` (obrigatória, última e engole até o fim).
fn separa_secoes(resto: &str) -> Result<(Option<String>, String)> {
    let Some(ini_corpo) = posicao_ancora(resto, ANCORA_CORPO) else {
        bail!("seção obrigatória `{ANCORA_CORPO}` ausente");
    };
    // O corpo é tudo após a linha da âncora — inclusive linhas que pareçam âncoras ou cercas.
    let apos = &resto[ini_corpo..];
    let corpo = apos.split_once('\n').map(|(_, c)| c).unwrap_or("").trim().to_string();

    // `## sinopse`, se houver, precisa vir ANTES do corpo (só olhamos a região anterior).
    let antes = &resto[..ini_corpo];
    let sinopse = posicao_ancora(antes, ANCORA_SINOPSE).map(|ini| {
        let apos = &antes[ini..];
        apos.split_once('\n').map(|(_, s)| s).unwrap_or("").trim().to_string()
    });

    Ok((sinopse, corpo))
}

/// Offset da primeira linha que começa exatamente com `ancora` (em início de linha).
fn posicao_ancora(texto: &str, ancora: &str) -> Option<usize> {
    let mut offset = 0usize;
    for linha in texto.split_inclusive('\n') {
        if linha.trim_end() == ancora {
            return Some(offset);
        }
        offset += linha.len();
    }
    None
}

/// Renderiza o documento na forma canônica. Round-trip com `parse_doc` garantido por teste.
///
/// Os valores do frontmatter são colapsados em linha única (ver [`colapsa_linha`]); a sinopse e o
/// corpo são gravados aparados, com uma linha em branco antes de cada âncora.
pub fn render_doc(header: &DocHeader, sinopse: Option<&str>, corpo: &str) -> String {
    let mut out = String::with_capacity(corpo.len() + 512);
    out.push_str(CERCA);
    out.push('\n');
    out.push_str(&format!("numero: {}\n", colapsa_linha(&header.numero)));
    out.push_str(&format!("assunto: {}\n", colapsa_linha(&header.assunto)));
    out.push_str(&format!("link: {}\n", colapsa_linha(&header.link)));
    if let Some(si) = &header.sinopse_info {
        out.push_str(&format!("sinopse_modelo: {}\n", colapsa_linha(&si.modelo)));
        out.push_str(&format!("sinopse_prompt_versao: {}\n", si.prompt_versao));
        out.push_str(&format!("sinopse_gerada_em: {}\n", colapsa_linha(&si.gerada_em)));
    }
    out.push_str(CERCA);
    out.push('\n');
    if let Some(s) = sinopse {
        out.push_str(&format!("\n{ANCORA_SINOPSE}\n{}\n", s.trim()));
    }
    out.push_str(&format!("\n{ANCORA_CORPO}\n{}\n", corpo.trim()));
    out
}

/// Grava `<dir>/<slug(numero)>.md` **se ainda não existir**; devolve `true` se criou, `false` se já
/// estava lá. É o ponto único que os produtores (scrapers de pareceres e o derive) usam para emitir
/// documentos novos.
///
/// **"Existe ⇒ pula" é o incremental** (G5): uma re-coleta só acrescenta o que é inédito e nunca
/// toca o que já está na árvore — e o que já está pode carregar uma `## sinopse` que custou LLM.
/// Por isso o pulo é incondicional: nem sequer lê o arquivo existente.
///
/// Corolário: **correção de conteúdo não chega por aqui.** Se o portal corrigir o corpo de uma
/// consulta já coletada, o produtor não a atualiza — é preciso remover o `.md` (decisão humana,
/// porque isso descarta a sinopse) e recoletar.
pub fn escrever_se_ausente(dir: &std::path::Path, header: &DocHeader, corpo: &str) -> Result<bool> {
    let slug = slug(&header.numero);
    if slug.is_empty() {
        bail!("`numero` não gera slug: {:?}", header.numero);
    }
    let destino = dir.join(format!("{slug}.md"));
    if destino.exists() {
        return Ok(false);
    }
    std::fs::create_dir_all(dir)?;
    // Escrita atômica (`.tmp` + rename), como o resto do pipeline: uma queda no meio nunca deixa um
    // `.md` truncado — que o parser rejeitaria e travaria o passo seguinte.
    let tmp = destino.with_extension("md.tmp");
    std::fs::write(&tmp, render_doc(header, None, corpo))?;
    std::fs::rename(&tmp, &destino)?;
    Ok(true)
}

/// Emite um LOTE de documentos inéditos em `dir`; devolve `(criados, pulados)`.
///
/// É o ponto de entrada dos produtores (scrapers e derive) — e o único lugar onde a **colisão de
/// slug** é detectável. `escrever_se_ausente`, sozinha, não distingue "já coletei este documento"
/// de "outro `numero` gera o mesmo arquivo": nos dois casos o arquivo existe. Só quem vê o lote
/// inteiro percebe que dois números diferentes disputam o mesmo `.md` — e isso é violação de
/// identidade, não um pulo legítimo. Sem esta checagem o segundo documento sumiria em silêncio.
pub fn escrever_lote_se_ausente(
    dir: &std::path::Path,
    docs: &[(DocHeader, String)],
) -> Result<(usize, usize)> {
    let mut vistos: std::collections::HashMap<String, &str> = std::collections::HashMap::new();
    for (header, _) in docs {
        let s = slug(&header.numero);
        if let Some(anterior) = vistos.insert(s.clone(), &header.numero)
            && anterior != header.numero
        {
            bail!("colisão de slug: {anterior:?} e {:?} geram o mesmo arquivo `{s}.md`", header.numero);
        }
    }
    let (mut criados, mut pulados) = (0usize, 0usize);
    for (header, corpo) in docs {
        if escrever_se_ausente(dir, header, corpo)? {
            criados += 1;
        } else {
            pulados += 1;
        }
    }
    Ok((criados, pulados))
}

/// Slug do `numero` para nome de arquivo: minúsculas, sem acento, `[^a-z0-9]+` → `-`, aparado.
///
/// Ex.: `CONSULTA COPAT nº 0037/26` → `consulta-copat-no-0037-26`. Colisão de slug dentro da
/// entidade é violação de identidade — quem materializa a árvore é que detecta (mesma doutrina do
/// dedup por `numero`).
pub fn slug(numero: &str) -> String {
    let mut out = String::with_capacity(numero.len());
    let mut pendente_hifen = false;
    for c in numero.chars().flat_map(sem_acento) {
        if c.is_ascii_alphanumeric() {
            if pendente_hifen && !out.is_empty() {
                out.push('-');
            }
            pendente_hifen = false;
            out.push(c.to_ascii_lowercase());
        } else {
            pendente_hifen = true;
        }
    }
    out
}

/// Dobra um caractere acentuado para ASCII. Cobre o pt-BR e os ordinais (`º`/`ª`) que aparecem nos
/// números de parecer (`nº` → `no`). Devolve iterador porque `ß`/`æ` viram dois caracteres.
fn sem_acento(c: char) -> impl Iterator<Item = char> {
    let s: &str = match c {
        'á' | 'à' | 'â' | 'ã' | 'ä' | 'å' | 'Á' | 'À' | 'Â' | 'Ã' | 'Ä' | 'Å' | 'ª' => "a",
        'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => "e",
        'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' | 'Î' | 'Ï' => "i",
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' | 'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' | 'º' | '°' => "o",
        'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Û' | 'Ü' => "u",
        'ç' | 'Ç' => "c",
        'ñ' | 'Ñ' => "n",
        'ý' | 'ÿ' | 'Ý' => "y",
        'æ' | 'Æ' => "ae",
        'ß' => "ss",
        _ => return OneOrMany::Um(Some(c)),
    };
    OneOrMany::Muitos(s.chars())
}

/// Iterador de 1 char (passa direto) ou de vários (dobra que expande) — evita alocar por caractere.
enum OneOrMany {
    Um(Option<char>),
    Muitos(std::str::Chars<'static>),
}

impl Iterator for OneOrMany {
    type Item = char;
    fn next(&mut self) -> Option<char> {
        match self {
            OneOrMany::Um(c) => c.take(),
            OneOrMany::Muitos(it) => it.next(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header_completo() -> DocHeader {
        DocHeader {
            numero: "CONSULTA COPAT nº 0037/26".into(),
            assunto: "ICMS. REGIME DE SUBSTITUIÇÃO TRIBUTÁRIA: pneumáticos".into(),
            link: "https://legislacao.sef.sc.gov.br/doc?id=1".into(),
            sinopse_info: Some(SinopseInfo {
                modelo: "llama-3.3-70b-versatile".into(),
                prompt_versao: 1,
                gerada_em: "2026-07-18T14:00:00Z".into(),
            }),
        }
    }

    #[test]
    fn round_trip_com_sinopse() {
        let h = header_completo();
        let sin = "### Descrição Resumida do Assunto\nAnalisa a ST.\n\n### Palavras Chave do Tema\n- **ICMS**";
        let corpo = "CONSULTA Nº 0037/26\n\nÉ o parecer.";
        let texto = render_doc(&h, Some(sin), corpo);
        let (h2, sin2, corpo2) = parse_doc(&texto).unwrap();
        assert_eq!(h2, h);
        assert_eq!(sin2.as_deref(), Some(sin));
        assert_eq!(corpo2, corpo);
        // Idempotente: renderizar o que foi parseado dá o mesmo texto.
        assert_eq!(render_doc(&h2, sin2.as_deref(), &corpo2), texto);
    }

    #[test]
    fn round_trip_sem_sinopse_pendente() {
        let mut h = header_completo();
        h.sinopse_info = None;
        let texto = render_doc(&h, None, "Corpo cru.");
        assert!(!texto.contains("sinopse_modelo"), "pendente não emite chaves sinopse_*");
        assert!(!texto.contains(ANCORA_SINOPSE), "pendente não emite a seção");
        let (h2, sin2, corpo2) = parse_doc(&texto).unwrap();
        assert_eq!(h2, h);
        assert_eq!(sin2, None);
        assert_eq!(corpo2, "Corpo cru.");
    }

    #[test]
    fn corpo_com_linhas_que_parecem_marcadores_nao_quebra() {
        let h = header_completo();
        let corpo = "Texto oficial.\n---\n## corpo\n## sinopse\nnumero: falso\nFim.";
        let texto = render_doc(&h, Some("### Descrição Resumida do Assunto\nx"), corpo);
        let (h2, _sin, corpo2) = parse_doc(&texto).unwrap();
        assert_eq!(h2, h);
        // O corpo é a última seção e engole tudo — inclusive as pegadinhas acima.
        assert_eq!(corpo2, corpo);
    }

    #[test]
    fn valor_com_dois_pontos_interno_parseia_certo() {
        let texto = "---\nnumero: PARECER Nº 1\nassunto: ICMS: crédito na entrada: energia\nlink: http://x/y?a=1:2\n---\n\n## corpo\nc\n";
        let (h, _, _) = parse_doc(texto).unwrap();
        assert_eq!(h.assunto, "ICMS: crédito na entrada: energia");
        assert_eq!(h.link, "http://x/y?a=1:2");
    }

    #[test]
    fn chave_desconhecida_e_erro_alto() {
        let texto = "---\nnumero: N\nassunto: A\nlink: L\nresumo: nao vai aqui\n---\n\n## corpo\nc\n";
        let e = parse_doc(texto).unwrap_err().to_string();
        assert!(e.contains("desconhecida") && e.contains("resumo"), "erro: {e}");
    }

    #[test]
    fn chave_duplicada_e_erro_alto() {
        let texto = "---\nnumero: N\nnumero: N2\nassunto: A\nlink: L\n---\n\n## corpo\nc\n";
        let e = parse_doc(texto).unwrap_err().to_string();
        assert!(e.contains("duplicada"), "erro: {e}");
    }

    #[test]
    fn sinopse_parcial_e_erro_alto() {
        let texto = "---\nnumero: N\nassunto: A\nlink: L\nsinopse_modelo: m\n---\n\n## corpo\nc\n";
        let e = parse_doc(texto).unwrap_err().to_string();
        assert!(e.contains("parcial"), "erro: {e}");
    }

    #[test]
    fn prompt_versao_nao_inteiro_e_erro() {
        let texto = "---\nnumero: N\nassunto: A\nlink: L\nsinopse_modelo: m\nsinopse_prompt_versao: um\nsinopse_gerada_em: z\n---\n\n## corpo\nc\n";
        let e = parse_doc(texto).unwrap_err().to_string();
        assert!(e.contains("prompt_versao"), "erro: {e}");
    }

    #[test]
    fn frontmatter_ausente_ou_aberto_e_erro() {
        let e = parse_doc("## corpo\nc\n").unwrap_err().to_string();
        assert!(e.contains("cerca"), "erro: {e}");
        let e = parse_doc("---\nnumero: N\n\n## corpo\nc\n").unwrap_err().to_string();
        assert!(e.contains("não foi fechado"), "erro: {e}");
    }

    #[test]
    fn corpo_ausente_e_erro() {
        let texto = "---\nnumero: N\nassunto: A\nlink: L\n---\n\n## sinopse\nx\n";
        let e = parse_doc(texto).unwrap_err().to_string();
        assert!(e.contains("## corpo"), "erro: {e}");
    }

    #[test]
    fn falta_chave_obrigatoria_e_erro() {
        let texto = "---\nnumero: N\nlink: L\n---\n\n## corpo\nc\n";
        let e = parse_doc(texto).unwrap_err().to_string();
        assert!(e.contains("assunto"), "erro: {e}");
    }

    #[test]
    fn writer_colapsa_valor_multilinha() {
        let mut h = header_completo();
        h.assunto = "ICMS.\n  REGIME  DE\nST".into();
        let texto = render_doc(&h, None, "c");
        assert!(texto.contains("assunto: ICMS. REGIME DE ST\n"), "texto:\n{texto}");
        // E o que volta do parse é a forma colapsada (round-trip estável a partir daí).
        let (h2, _, _) = parse_doc(&texto).unwrap();
        assert_eq!(h2.assunto, "ICMS. REGIME DE ST");
    }

    #[test]
    fn escrever_se_ausente_cria_e_depois_pula() {
        let dir = std::env::temp_dir().join(format!("auli-mddoc-emit-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut h = header_completo();
        h.sinopse_info = None; // produtor emite pendente

        // 1ª vez: cria.
        assert!(escrever_se_ausente(&dir, &h, "Corpo coletado.").unwrap());
        let destino = dir.join("consulta-copat-no-0037-26.md");
        assert!(destino.exists());
        let (h2, sin, corpo) = parse_doc(&std::fs::read_to_string(&destino).unwrap()).unwrap();
        assert_eq!(h2.numero, h.numero);
        assert_eq!(sin, None, "produtor emite SEM sinopse (pendente)");
        assert_eq!(corpo, "Corpo coletado.");

        // 2ª vez: pula e NÃO sobrescreve — nem com corpo diferente.
        assert!(!escrever_se_ausente(&dir, &h, "CORPO NOVO QUE NAO DEVE ENTRAR").unwrap());
        let (_, _, corpo2) = parse_doc(&std::fs::read_to_string(&destino).unwrap()).unwrap();
        assert_eq!(corpo2, "Corpo coletado.", "existente é intocável");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn escrever_se_ausente_nao_apaga_sinopse_de_documento_ja_sinopsado() {
        // O cenário que a regra protege: re-coleta encontrando um doc que já custou LLM.
        let dir = std::env::temp_dir().join(format!("auli-mddoc-emit2-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let h = header_completo(); // com sinopse_info
        std::fs::create_dir_all(&dir).unwrap();
        let destino = dir.join("consulta-copat-no-0037-26.md");
        std::fs::write(&destino, render_doc(&h, Some("SINOPSE CARA"), "corpo")).unwrap();

        let mut h_novo = header_completo();
        h_novo.sinopse_info = None;
        assert!(!escrever_se_ausente(&dir, &h_novo, "corpo recoletado").unwrap());
        let (_, sin, _) = parse_doc(&std::fs::read_to_string(&destino).unwrap()).unwrap();
        assert_eq!(sin.as_deref(), Some("SINOPSE CARA"), "sinopse preservada");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn lote_recusa_colisao_de_slug_nomeando_os_dois() {
        // A regressão que isto guarda: sem a checagem, o segundo documento seria "pulado" como se
        // já existisse — e sumiria em silêncio.
        let dir = std::env::temp_dir().join(format!("auli-mddoc-colisao-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mk = |n: &str| mddoc_header(n);
        let docs = vec![(mk("CONSULTA 1/26"), "a".to_string()), (mk("CONSULTA 1-26"), "b".to_string())];
        let e = escrever_lote_se_ausente(&dir, &docs).unwrap_err().to_string();
        assert!(e.contains("colisão de slug"), "erro: {e}");
        assert!(e.contains("CONSULTA 1/26") && e.contains("CONSULTA 1-26"), "deve nomear os dois: {e}");
        // E não escreveu nada: a checagem roda ANTES de tocar o disco.
        assert!(!dir.exists() || std::fs::read_dir(&dir).unwrap().count() == 0);
    }

    #[test]
    fn lote_conta_criados_e_pulados_e_o_mesmo_numero_repetido_nao_e_colisao() {
        let dir = std::env::temp_dir().join(format!("auli-mddoc-lote-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        // O MESMO numero duas vezes no lote não é colisão de identidade — é duplicata; o 2º é pulado.
        let docs = vec![(mddoc_header("A 1"), "x".to_string()), (mddoc_header("A 1"), "x".to_string())];
        assert_eq!(escrever_lote_se_ausente(&dir, &docs).unwrap(), (1, 1));
        // Re-rodar o lote inteiro: tudo pulado (incremental).
        assert_eq!(escrever_lote_se_ausente(&dir, &docs).unwrap(), (0, 2));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Header mínimo para os testes de lote.
    fn mddoc_header(numero: &str) -> DocHeader {
        DocHeader {
            numero: numero.into(),
            assunto: format!("assunto de {numero}"),
            link: format!("http://x/{numero}"),
            sinopse_info: None,
        }
    }

    #[test]
    fn escrever_se_ausente_recusa_numero_sem_slug() {
        let dir = std::env::temp_dir().join(format!("auli-mddoc-emit3-{}", std::process::id()));
        let mut h = header_completo();
        h.numero = "///".into();
        assert!(escrever_se_ausente(&dir, &h, "c").is_err());
    }

    #[test]
    fn slug_acentos_ordinais_e_barra() {
        assert_eq!(slug("CONSULTA COPAT nº 0037/26"), "consulta-copat-no-0037-26");
        assert_eq!(slug("PARECER Nº 25148"), "parecer-no-25148");
        assert_eq!(slug("Consulta nº 12/2008 — ICMS/ST"), "consulta-no-12-2008-icms-st");
        assert_eq!(slug("ÁÉÍÓÚ ção"), "aeiou-cao");
        // Sem hífen sobrando nas pontas.
        assert_eq!(slug("///abc///"), "abc");
        assert_eq!(slug(""), "");
    }

    #[test]
    fn slug_colide_para_numeros_que_so_diferem_em_pontuacao() {
        // Documenta a doutrina: quem materializa a árvore detecta a colisão e erra.
        assert_eq!(slug("CONSULTA 1/26"), slug("CONSULTA 1-26"));
    }
}
