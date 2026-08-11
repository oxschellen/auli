//! `mddoc` — o contrato do arquivo `.md` que é a **fonte** de um documento de QUALQUER coleção.
//!
//! Um arquivo por documento: frontmatter tipado + `## resumo` (opcional) + `## corpo`.
//!
//! ```text
//! ---
//! titulo: CONSULTA COPAT nº 0037/26
//! trilha:
//! ementa: ICMS. REGIME DE SUBSTITUIÇÃO TRIBUTÁRIA...
//! link: https://legislacao.sef.sc.gov.br/...
//! resumo_modelo: llama-3.3-70b-versatile
//! resumo_prompt_versao: 1
//! resumo_gerada_em: 2026-07-18T14:00:00Z
//! ---
//!
//! ## resumo
//! ### Descrição Resumida do Assunto
//! ...
//!
//! ## corpo
//! <texto oficial integral — última seção, engole tudo até o fim>
//! ```
//!
//! **Vocabulário unificado (D-FMT-1).** Antes havia três parsers irmãos, um por coleção, com
//! chaves próprias (`numero`/`assunto` em jurisprudência, `titulo`/`tipo`/`classe` em serviços,
//! `pergunta`/`origin`/`url` em faqs). Os nomes divergiam mas os **papéis** eram os mesmos, e é o
//! papel que os dois funis do chat consomem — daí um vocabulário só:
//!
//! | papel | jurisprudência | serviço | faq |
//! |---|---|---|---|
//! | `titulo` — identificação humana | numero | titulo | pergunta |
//! | `trilha` — classificação | *(vazio)* | `tipo \| classe` | origin |
//! | `ementa` — síntese junto ao título | assunto | *(vazio)* | *(vazio)* |
//! | `## resumo` — síntese embedável | sinopse | *(ausente)* | *(ausente)* |
//! | `## corpo` — conteúdo integral | corpo | descrição | resposta |
//!
//! As quatro chaves do frontmatter são **sempre escritas**, com valor vazio quando o papel não se
//! aplica: o parser não distingue coleção por ausência de chave. `orgao` é a única exceção
//! (D-FMT-3) — chave opcional de passthrough dos serviços, que não entra em funil nenhum e existe
//! para quem baixa os zips.
//!
//! **Parser canônico próprio, sem dependência nova.** O frontmatter é a *nossa* forma canônica
//! (`chave: valor`, uma por linha, valor = tudo após o primeiro `: `), não YAML arbitrário: somos o
//! único escritor e o único leitor. `serde_yaml` está arquivado/não-mantido (RUSTSEC, 2024) e
//! violaria o invariante deste crate ("leve: só serde/serde_json + anyhow/time"). O parser é
//! **estrito** — chave desconhecida, duplicada, linha fora da forma ou `resumo_*` parcial ⇒ erro
//! alto: um arquivo "quase certo" falha ruidosamente em vez de parsear silenciosamente errado.
//!
//! Bônus preservado: a forma canônica plana **continua sendo YAML válido**, então Obsidian, pandoc e
//! geradores de site leem o frontmatter normalmente — nós é que não dependemos de um parser de YAML.

use std::path::Path;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use crate::ResumoInfo;

/// Delimitador do frontmatter e âncoras das seções (início de linha).
const CERCA: &str = "---";
const ANCORA_RESUMO: &str = "## resumo";
const ANCORA_CORPO: &str = "## corpo";

/// Cabeçalho tipado do documento — o que vive no frontmatter.
///
/// `resumo_info` ausente = **resumo pendente** (só jurisprudência tem essa noção). No arquivo ele é
/// achatado em três chaves planas (`resumo_modelo`, `resumo_prompt_versao`, `resumo_gerada_em`) —
/// sem aninhamento, que é a única coisa que complicaria o parser. As três andam juntas: estado
/// misto é erro.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocHeader {
    /// Identificação humana: número do parecer/acórdão, título do serviço, pergunta da FAQ.
    pub titulo: String,
    /// Classificação: `tipo | classe` nos serviços, breadcrumb (`origin`) nas faqs, vazio na
    /// jurisprudência.
    pub trilha: String,
    /// Síntese que acompanha o título: o assunto/ementa da jurisprudência; vazio nas demais.
    pub ementa: String,
    /// URL oficial do documento.
    pub link: String,
    /// **A única concessão** (D-FMT-3): metadado de coleção dos serviços, passado adiante sem ser
    /// consumido por funil nenhum — nem embed, nem bloco RAG. Existe pelo valor que tem para quem
    /// baixa os zips. Nenhuma outra chave específica de coleção é admitida.
    pub orgao: Option<String>,
    /// Proveniência do `## resumo` gerado por LLM. `None` nas coleções que não geram resumo e na
    /// jurisprudência ainda pendente.
    pub resumo_info: Option<ResumoInfo>,
}

/// Cabeçalho de jurisprudência (pareceres e acórdãos) no vocabulário unificado (D-FMT-2): o número
/// da consulta é o `titulo`, o assunto é a `ementa`, e a `trilha` fica vazia — jurisprudência não
/// tem classificação. Nasce SEM `resumo_info`: o resumo vem depois, do passo LLM ou da própria
/// coleta.
///
/// Ponto único porque os dois produtores precisam dele e não se enxergam: os scrapers (via o kit,
/// que é exclusivo de `crates/scrapers/`) e o `derive_pareceres` do `auli-collections`.
pub fn cabecalho_jurisprudencia(numero: &str, assunto: &str, link: &str) -> DocHeader {
    DocHeader {
        titulo: numero.to_string(),
        trilha: String::new(),
        ementa: assunto.to_string(),
        link: link.to_string(),
        orgao: None,
        resumo_info: None,
    }
}

/// Colapsa o valor em **linha única**: qualquer corrida de espaços/quebras vira um espaço só.
/// Os valores do frontmatter são escalares de uma linha; as ementas já vivem assim no `.txt`.
pub(crate) fn colapsa_linha(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Parseia um documento inteiro: `(cabeçalho, resumo, corpo)`.
///
/// `resumo` é `None` quando a seção `## resumo` está ausente — o estado normal de serviços e faqs,
/// e o estado "pendente" da jurisprudência. O `corpo` é a última seção e engole tudo até o fim do
/// arquivo — por isso **nada dentro dele quebra o parse**, nem linhas que pareçam âncoras ou cercas.
pub fn parse_doc(texto: &str) -> Result<(DocHeader, Option<String>, String)> {
    let (frontmatter, resto) = separa_frontmatter(texto)?;
    let header = parse_frontmatter(frontmatter)?;
    let (resumo, corpo) = separa_secoes(resto)?;
    Ok((header, resumo, corpo))
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
    let (mut titulo, mut trilha, mut ementa, mut link, mut orgao) = (None, None, None, None, None);
    let (mut r_modelo, mut r_versao, mut r_gerada) = (None, None, None);

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
            "titulo" => poe(&mut titulo)?,
            "trilha" => poe(&mut trilha)?,
            "ementa" => poe(&mut ementa)?,
            "link" => poe(&mut link)?,
            "orgao" => poe(&mut orgao)?,
            "resumo_modelo" => poe(&mut r_modelo)?,
            "resumo_gerada_em" => poe(&mut r_gerada)?,
            "resumo_prompt_versao" => poe(&mut r_versao)?,
            outra => bail!("frontmatter linha {n}: chave desconhecida `{outra}`"),
        }
    }

    // Obrigatoriedade é da CHAVE, não do valor: `trilha:` e `ementa:` vazias são o estado normal
    // de metade das coleções.
    let obrigatoria = |campo: Option<String>, nome: &str| -> Result<String> {
        campo.ok_or_else(|| anyhow::anyhow!("frontmatter: falta a chave obrigatória `{nome}`"))
    };

    // As três `resumo_*` andam juntas: ou nenhuma (pendente) ou todas.
    let resumo_info = match (r_modelo, r_versao, r_gerada) {
        (None, None, None) => None,
        (Some(modelo), Some(versao), Some(gerada_em)) => {
            let prompt_versao: u32 = versao.parse().map_err(|_| {
                anyhow::anyhow!("frontmatter: `resumo_prompt_versao` não é inteiro — {versao:?}")
            })?;
            Some(ResumoInfo {
                modelo,
                prompt_versao,
                gerada_em,
            })
        }
        _ => bail!(
            "frontmatter: `resumo_*` parcial — as três chaves (resumo_modelo, \
             resumo_prompt_versao, resumo_gerada_em) devem estar todas presentes ou todas ausentes"
        ),
    };

    Ok(DocHeader {
        titulo: obrigatoria(titulo, "titulo")?,
        trilha: obrigatoria(trilha, "trilha")?,
        ementa: obrigatoria(ementa, "ementa")?,
        link: obrigatoria(link, "link")?,
        orgao,
        resumo_info,
    })
}

/// Separa `## resumo` (opcional) e `## corpo` (obrigatória, última e engole até o fim).
///
/// Antes do `## corpo` só se admite a seção `## resumo` e linhas em branco: conteúdo solto ali
/// seria engolido em silêncio, então é erro alto (guarda herdada dos parsers de serviço e faq).
fn separa_secoes(resto: &str) -> Result<(Option<String>, String)> {
    let Some(ini_corpo) = posicao_ancora(resto, ANCORA_CORPO) else {
        bail!("seção obrigatória `{ANCORA_CORPO}` ausente");
    };
    // O corpo é tudo após a linha da âncora — inclusive linhas que pareçam âncoras ou cercas.
    let apos = &resto[ini_corpo..];
    let corpo = apos
        .split_once('\n')
        .map(|(_, c)| c)
        .unwrap_or("")
        .trim()
        .to_string();

    // `## resumo`, se houver, precisa vir ANTES do corpo (só olhamos a região anterior).
    let antes = &resto[..ini_corpo];
    let resumo = match posicao_ancora(antes, ANCORA_RESUMO) {
        Some(ini) => {
            recusar_conteudo_solto(&antes[..ini])?;
            let apos = &antes[ini..];
            Some(
                apos.split_once('\n')
                    .map(|(_, s)| s)
                    .unwrap_or("")
                    .trim()
                    .to_string(),
            )
        }
        None => {
            recusar_conteudo_solto(antes)?;
            None
        }
    };

    Ok((resumo, corpo))
}

/// Erra se houver qualquer coisa além de linhas em branco no trecho.
fn recusar_conteudo_solto(trecho: &str) -> Result<()> {
    for linha in trecho.lines() {
        if !linha.trim().is_empty() {
            bail!(
                "conteúdo inesperado fora de `{ANCORA_RESUMO}`/`{ANCORA_CORPO}`: {:?}",
                linha.trim_end()
            );
        }
    }
    Ok(())
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

/// Renderiza o documento na forma canônica. Round-trip com [`parse_doc`] garantido por teste.
///
/// Os valores do frontmatter são colapsados em linha única (ver [`colapsa_linha`]); o resumo e o
/// corpo são gravados aparados, com uma linha em branco antes de cada âncora.
pub fn render_doc(header: &DocHeader, resumo: Option<&str>, corpo: &str) -> String {
    let mut out = String::with_capacity(corpo.len() + 512);
    out.push_str(CERCA);
    out.push('\n');
    out.push_str(&format!("titulo: {}\n", colapsa_linha(&header.titulo)));
    out.push_str(&format!("trilha: {}\n", colapsa_linha(&header.trilha)));
    out.push_str(&format!("ementa: {}\n", colapsa_linha(&header.ementa)));
    out.push_str(&format!("link: {}\n", colapsa_linha(&header.link)));
    if let Some(o) = &header.orgao {
        out.push_str(&format!("orgao: {}\n", colapsa_linha(o)));
    }
    if let Some(ri) = &header.resumo_info {
        out.push_str(&format!("resumo_modelo: {}\n", colapsa_linha(&ri.modelo)));
        out.push_str(&format!("resumo_prompt_versao: {}\n", ri.prompt_versao));
        out.push_str(&format!(
            "resumo_gerada_em: {}\n",
            colapsa_linha(&ri.gerada_em)
        ));
    }
    out.push_str(CERCA);
    out.push('\n');
    if let Some(r) = resumo {
        out.push_str(&format!("\n{ANCORA_RESUMO}\n{}\n", r.trim()));
    }
    out.push_str(&format!("\n{ANCORA_CORPO}\n{}\n", corpo.trim()));
    out
}

// ---------------------------------------------------------------------------
// Escrita — duas doutrinas de árvore
// ---------------------------------------------------------------------------

/// Grava `<dir>/<slug(titulo)>.md` **se ainda não existir**; devolve `true` se criou, `false` se já
/// estava lá. É o ponto único que os produtores de jurisprudência (scrapers e derive) usam para
/// emitir documentos novos.
///
/// **"Existe ⇒ pula" é o incremental** (G5): uma re-coleta só acrescenta o que é inédito e nunca
/// toca o que já está na árvore — e o que já está pode carregar um `## resumo` que custou LLM.
/// Por isso o pulo é incondicional: nem sequer lê o arquivo existente.
///
/// Corolário: **correção de conteúdo não chega por aqui.** Se o portal corrigir o corpo de uma
/// consulta já coletada, o produtor não a atualiza — é preciso remover o `.md` (decisão humana,
/// porque isso descarta o resumo) e recoletar.
pub fn escrever_se_ausente(dir: &Path, header: &DocHeader, corpo: &str) -> Result<bool> {
    escrever_se_ausente_interno(dir, header, None, corpo)
}

/// Miolo de [`escrever_se_ausente`], com o resumo como parâmetro.
///
/// Existe porque há duas origens legítimas para o `## resumo`: o passo LLM (que reescreve o `.md`
/// depois) e a **própria coleta**, quando o produtor já recebe do portal um resumo autoral (o TARF
/// traz a fundamentação da ementa no cabeçalho do acórdão). Nos dois casos a regra de ouro é a
/// mesma — existe ⇒ pula, sem ler o arquivo — então só o argumento muda.
fn escrever_se_ausente_interno(
    dir: &Path,
    header: &DocHeader,
    resumo: Option<&str>,
    corpo: &str,
) -> Result<bool> {
    let slug = slug(&header.titulo);
    if slug.is_empty() {
        bail!("`titulo` não gera slug: {:?}", header.titulo);
    }
    let destino = dir.join(format!("{slug}.md"));
    if destino.exists() {
        return Ok(false);
    }
    std::fs::create_dir_all(dir)?;
    // Escrita atômica, como o resto do pipeline: uma queda no meio nunca deixa um `.md` truncado —
    // que o parser rejeitaria e travaria o passo seguinte.
    crate::arquivo::escrever_atomico(&destino, &render_doc(header, resumo, corpo))?;
    Ok(true)
}

/// Emite um LOTE de documentos de jurisprudência inéditos em `dir`; devolve `(criados, pulados)`.
///
/// É o ponto de entrada dos produtores (scrapers e derive) — e o único lugar onde a **colisão de
/// slug** é detectável. [`escrever_se_ausente`], sozinha, não distingue "já coletei este documento"
/// de "outro `titulo` gera o mesmo arquivo": nos dois casos o arquivo existe. Só quem vê o lote
/// inteiro percebe que dois títulos diferentes disputam o mesmo `.md` — e isso é violação de
/// identidade, não um pulo legítimo. Sem esta checagem o segundo documento sumiria em silêncio.
pub fn escrever_lote_se_ausente(
    dir: &Path,
    docs: &[(DocHeader, String)],
) -> Result<(usize, usize)> {
    recusar_colisao_de_slug(docs.iter().map(|(h, _)| h))?;
    let (mut criados, mut pulados) = (0usize, 0usize);
    for (header, corpo) in docs {
        if escrever_se_ausente_interno(dir, header, None, corpo)? {
            criados += 1;
        } else {
            pulados += 1;
        }
    }
    Ok((criados, pulados))
}

/// Variante do lote que aceita **resumo por documento**; devolve `(criados, pulados)`.
///
/// Produtores cujo resumo é autorado na coleta (o TARF o extrai da fundamentação que o próprio
/// acórdão traz no cabeçalho) emitem por aqui; os de pareceres continuam emitindo pendente por
/// [`escrever_lote_se_ausente`]. Fora o resumo, a semântica é idêntica — mesma checagem de colisão
/// de slug, mesmo "existe ⇒ pula", mesma escrita atômica.
pub fn escrever_lote_se_ausente_com_resumo(
    dir: &Path,
    docs: &[(DocHeader, Option<String>, String)],
) -> Result<(usize, usize)> {
    recusar_colisao_de_slug(docs.iter().map(|(h, _, _)| h))?;
    let (mut criados, mut pulados) = (0usize, 0usize);
    for (header, resumo, corpo) in docs {
        if escrever_se_ausente_interno(dir, header, resumo.as_deref(), corpo)? {
            criados += 1;
        } else {
            pulados += 1;
        }
    }
    Ok((criados, pulados))
}

/// Erra se dois `titulo` **distintos** do lote geram o mesmo arquivo. O mesmo `titulo` repetido não
/// é colisão — é duplicata, e o segundo é legitimamente pulado. Roda ANTES de tocar o disco: um lote
/// com colisão não escreve nada.
fn recusar_colisao_de_slug<'a>(headers: impl Iterator<Item = &'a DocHeader>) -> Result<()> {
    let mut vistos: std::collections::HashMap<String, &str> = std::collections::HashMap::new();
    for header in headers {
        let s = slug(&header.titulo);
        if let Some(anterior) = vistos.insert(s.clone(), &header.titulo)
            && anterior != header.titulo
        {
            bail!(
                "colisão de slug: {anterior:?} e {:?} geram o mesmo arquivo `{s}.md`",
                header.titulo
            );
        }
    }
    Ok(())
}

/// **Regenera a árvore inteira**: apaga `dir` (se existir), recria e grava um `.md` por item,
/// nomeado por `nomear`. Colisão de nome é violação de identidade — detectada ANTES de escrever
/// qualquer arquivo, nomeando os dois títulos em conflito. Devolve o total gravado.
///
/// Delete + rebuild é a doutrina de serviços e faqs (mutáveis no portal): o estado da árvore após
/// esta função é função SÓ da coleta, nunca do que havia antes. É o oposto do "existe ⇒ pula" da
/// jurisprudência, que é append-only porque seus documentos não mudam e carregam resumo caro.
pub fn materializar_arvore(
    dir: &Path,
    docs: &[(DocHeader, String)],
    nomear: fn(&DocHeader) -> String,
) -> Result<usize> {
    // 1. Pré-cheque de colisão, com o par ofensor no erro.
    let mut vistos: std::collections::HashMap<String, &str> =
        std::collections::HashMap::with_capacity(docs.len());
    for (header, _) in docs {
        let nome = nomear(header);
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
        std::fs::write(
            dir.join(format!("{}.md", nomear(header))),
            render_doc(header, None, corpo),
        )?;
    }
    Ok(docs.len())
}

// ---------------------------------------------------------------------------
// Nomeação — uma regra por doutrina de árvore
// ---------------------------------------------------------------------------

/// Slug do `titulo` para nome de arquivo: minúsculas, sem acento, `[^a-z0-9]+` → `-`, aparado.
///
/// Ex.: `CONSULTA COPAT nº 0037/26` → `consulta-copat-no-0037-26`. Colisão de slug dentro da
/// entidade é violação de identidade — quem materializa a árvore é que detecta (mesma doutrina do
/// dedup por `titulo`).
pub fn slug(titulo: &str) -> String {
    let mut out = String::with_capacity(titulo.len());
    let mut pendente_hifen = false;
    for c in titulo.chars().flat_map(sem_acento) {
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

/// Teto do slug no nome de arquivo do serviço. Existe por causa do **filesystem**, não de
/// legibilidade: 200 + `-hash8` + `.md` = 212 bytes, dentro do limite de 255 de ext4/APFS com folga.
/// (A FAQ corta em 60 por outro motivo — a pergunta é uma frase inteira.)
///
/// O número foi escolhido para **não renomear nada do que já está materializado**: o maior slug de
/// serviço nas entidades vivas é 144 (SP). Quem estourava era a BA, com dois títulos de 285 — daí a
/// truncagem ter chegado só quando ela foi migrar.
const MAX_SLUG_SERVICO: usize = 200;
/// Teto do slug da FAQ: a pergunta é uma frase inteira, então o corte é por legibilidade.
const MAX_SLUG_FAQ: usize = 60;

/// Nome de arquivo do serviço: `slug(titulo)` (truncado em [`MAX_SLUG_SERVICO`], na última fronteira
/// de hífen) + `-` + `hash8(link)`. O hash curto (8 hex do FNV-1a 64 truncado) desambigua títulos
/// repetidos e sobrevive a renomeações de título só até o próximo rebuild — a identidade real é o
/// `link`. Título sem slug (só símbolos) fica só com o hash.
pub fn nome_servico(h: &DocHeader) -> String {
    nome_servico_de(&h.titulo, &h.link)
}

/// [`nome_servico`] a partir dos campos crus, para quem precisa do nome de arquivo antes de ter um
/// cabeçalho montado (a fusão de duplicatas usa o nome como chave de identidade).
pub fn nome_servico_de(titulo: &str, link: &str) -> String {
    nome_com_hash(titulo, MAX_SLUG_SERVICO, link.as_bytes())
}

/// Nome de arquivo da FAQ: `slug(titulo)` truncado em [`MAX_SLUG_FAQ`] + `-` + hash8 de
/// `link + "\n" + titulo`. A identidade real está no hash, que cobre o par `(url, pergunta)`,
/// porque a url sozinha não distingue perguntas da mesma página.
pub fn nome_faq(h: &DocHeader) -> String {
    nome_faq_de(&h.titulo, &h.link)
}

/// [`nome_faq`] a partir dos campos crus, para quem precisa do nome de arquivo sem ter um cabeçalho
/// montado — o mesmo motivo do [`nome_servico_de`]. Sem isto, chamadores montavam um `DocHeader`
/// descartável só para pedir o nome.
pub fn nome_faq_de(titulo: &str, link: &str) -> String {
    nome_com_hash(titulo, MAX_SLUG_FAQ, format!("{link}\n{titulo}").as_bytes())
}

/// Miolo das duas nomeações: slug truncado + hash8 do que identifica o documento.
fn nome_com_hash(titulo: &str, max: usize, identidade: &[u8]) -> String {
    let s = slug(titulo);
    let s = slug_truncado(&s, max);
    let h = format!("{:08x}", fnv1a64(identidade) as u32);
    if s.is_empty() { h } else { format!("{s}-{h}") }
}

/// Trunca um slug (ASCII puro: `[a-z0-9-]`, então fatiar por byte é seguro) na última fronteira de
/// hífen dentro de `max`; sem hífen no trecho, corte seco. Nunca devolve hífen pendurado no fim.
fn slug_truncado(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    match s[..max].rfind('-') {
        Some(i) => &s[..i],
        None => &s[..max],
    }
}

/// FNV-1a 64 bits. Gêmeo do `auli_core::manifest::fnv1a64` — duplicado de propósito: este crate é
/// "leve" (só serde/serde_json + anyhow/time) e não pode depender do `auli-core`. Se as constantes
/// mudarem lá, NÃO precisam mudar aqui: este hash só nomeia arquivos das árvores deste crate.
fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Dobra um caractere acentuado para ASCII. Cobre o pt-BR e os ordinais (`º`/`ª`) que aparecem nos
/// números de parecer (`nº` → `no`). Devolve iterador porque `ß`/`æ` viram dois caracteres.
fn sem_acento(c: char) -> impl Iterator<Item = char> {
    let s: &str = match c {
        'á' | 'à' | 'â' | 'ã' | 'ä' | 'å' | 'Á' | 'À' | 'Â' | 'Ã' | 'Ä' | 'Å' | 'ª' => {
            "a"
        }
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

    fn header_jurisprudencia() -> DocHeader {
        DocHeader {
            titulo: "CONSULTA COPAT nº 0037/26".into(),
            trilha: String::new(),
            ementa: "ICMS. REGIME DE SUBSTITUIÇÃO TRIBUTÁRIA: pneumáticos".into(),
            link: "https://legislacao.sef.sc.gov.br/doc?id=1".into(),
            orgao: None,
            resumo_info: Some(ResumoInfo {
                modelo: "llama-3.3-70b-versatile".into(),
                prompt_versao: 1,
                gerada_em: "2026-07-18T14:00:00Z".into(),
            }),
        }
    }

    fn header_servico() -> DocHeader {
        DocHeader {
            titulo: "Emissão de Guia de IPVA".into(),
            trilha: "Cidadãos | IPVA".into(),
            ementa: String::new(),
            link: "https://atendimento.receita.rs.gov.br/emissao-guia-ipva".into(),
            orgao: Some("Receita Estadual".into()),
            resumo_info: None,
        }
    }

    fn header_faq() -> DocHeader {
        DocHeader {
            titulo: "Como faço para emitir a guia de IPVA?".into(),
            trilha: "Inicial | Perguntas Frequentes | IPVA".into(),
            ementa: String::new(),
            link: "https://atendimento.receita.rs.gov.br/faq-ipva".into(),
            orgao: None,
            resumo_info: None,
        }
    }

    #[test]
    fn round_trip_jurisprudencia_com_resumo() {
        let h = header_jurisprudencia();
        let res = "### Descrição Resumida do Assunto\nAnalisa a ST.\n\n### Palavras Chave do Tema\n- **ICMS**";
        let corpo = "CONSULTA Nº 0037/26\n\nÉ o parecer.";
        let texto = render_doc(&h, Some(res), corpo);
        let (h2, res2, corpo2) = parse_doc(&texto).unwrap();
        assert_eq!(h2, h);
        assert_eq!(res2.as_deref(), Some(res));
        assert_eq!(corpo2, corpo);
        // Idempotente: renderizar o que foi parseado dá o mesmo texto.
        assert_eq!(render_doc(&h2, res2.as_deref(), &corpo2), texto);
    }

    #[test]
    fn round_trip_servico_com_orgao_e_sem_resumo() {
        let h = header_servico();
        let texto = render_doc(&h, None, "Passos para emitir a guia.");
        assert!(texto.contains("trilha: Cidadãos | IPVA\n"));
        assert!(texto.contains("ementa: \n"), "chave presente, valor vazio");
        assert!(texto.contains("orgao: Receita Estadual\n"));
        assert!(!texto.contains(ANCORA_RESUMO));
        let (h2, res, corpo) = parse_doc(&texto).unwrap();
        assert_eq!(h2, h);
        assert_eq!(res, None);
        assert_eq!(corpo, "Passos para emitir a guia.");
    }

    #[test]
    fn round_trip_faq_sem_orgao() {
        let h = header_faq();
        let texto = render_doc(&h, None, "Acesse o portal.");
        assert!(!texto.contains("orgao:"), "orgao ausente não vira chave");
        let (h2, res, corpo) = parse_doc(&texto).unwrap();
        assert_eq!(h2, h);
        assert_eq!(res, None);
        assert_eq!(corpo, "Acesse o portal.");
    }

    #[test]
    fn corpo_vazio_e_valido() {
        // Serviço sem descrição / FAQ sem resposta são estados legais da árvore.
        let texto = render_doc(&header_servico(), None, "");
        let (_, _, corpo) = parse_doc(&texto).unwrap();
        assert_eq!(corpo, "");
    }

    #[test]
    fn corpo_com_linhas_que_parecem_marcadores_nao_quebra() {
        let h = header_jurisprudencia();
        let corpo = "Texto oficial.\n---\n## corpo\n## resumo\ntitulo: falso\nFim.";
        let texto = render_doc(&h, Some("### Descrição Resumida do Assunto\nx"), corpo);
        let (h2, _res, corpo2) = parse_doc(&texto).unwrap();
        assert_eq!(h2, h);
        // O corpo é a última seção e engole tudo — inclusive as pegadinhas acima.
        assert_eq!(corpo2, corpo);
    }

    #[test]
    fn valor_com_dois_pontos_interno_parseia_certo() {
        let texto = "---\ntitulo: PARECER Nº 1\ntrilha: \nementa: ICMS: crédito na entrada: energia\nlink: http://x/y?a=1:2\n---\n\n## corpo\nc\n";
        let (h, _, _) = parse_doc(texto).unwrap();
        assert_eq!(h.ementa, "ICMS: crédito na entrada: energia");
        assert_eq!(h.link, "http://x/y?a=1:2");
    }

    #[test]
    fn chave_desconhecida_e_erro_alto() {
        // As chaves do vocabulário ANTIGO são desconhecidas agora — um `.md` não convertido falha
        // ruidosamente em vez de parsear pela metade.
        for antiga in [
            "numero: N",
            "assunto: A",
            "pergunta: P",
            "origin: O",
            "url: U",
        ] {
            let texto = format!(
                "---\ntitulo: T\ntrilha: \nementa: \nlink: L\n{antiga}\n---\n\n## corpo\nc\n"
            );
            let e = parse_doc(&texto).unwrap_err().to_string();
            assert!(e.contains("desconhecida"), "{antiga} deveria falhar: {e}");
        }
    }

    #[test]
    fn chave_duplicada_e_erro_alto() {
        let texto = "---\ntitulo: N\ntitulo: N2\ntrilha: \nementa: \nlink: L\n---\n\n## corpo\nc\n";
        let e = parse_doc(texto).unwrap_err().to_string();
        assert!(e.contains("duplicada"), "erro: {e}");
    }

    #[test]
    fn resumo_parcial_e_erro_alto() {
        let texto =
            "---\ntitulo: N\ntrilha: \nementa: \nlink: L\nresumo_modelo: m\n---\n\n## corpo\nc\n";
        let e = parse_doc(texto).unwrap_err().to_string();
        assert!(e.contains("parcial"), "erro: {e}");
    }

    #[test]
    fn prompt_versao_nao_inteiro_e_erro() {
        let texto = "---\ntitulo: N\ntrilha: \nementa: \nlink: L\nresumo_modelo: m\nresumo_prompt_versao: um\nresumo_gerada_em: z\n---\n\n## corpo\nc\n";
        let e = parse_doc(texto).unwrap_err().to_string();
        assert!(e.contains("prompt_versao"), "erro: {e}");
    }

    #[test]
    fn frontmatter_ausente_ou_aberto_e_erro() {
        let e = parse_doc("## corpo\nc\n").unwrap_err().to_string();
        assert!(e.contains("cerca"), "erro: {e}");
        let e = parse_doc("---\ntitulo: N\n\n## corpo\nc\n")
            .unwrap_err()
            .to_string();
        assert!(e.contains("não foi fechado"), "erro: {e}");
    }

    #[test]
    fn corpo_ausente_e_erro() {
        let texto = "---\ntitulo: N\ntrilha: \nementa: \nlink: L\n---\n\n## resumo\nx\n";
        let e = parse_doc(texto).unwrap_err().to_string();
        assert!(e.contains("## corpo"), "erro: {e}");
    }

    #[test]
    fn falta_chave_obrigatoria_e_erro() {
        for faltante in ["titulo", "trilha", "ementa", "link"] {
            let texto: String = ["titulo", "trilha", "ementa", "link"]
                .iter()
                .filter(|c| **c != faltante)
                .map(|c| format!("{c}: v\n"))
                .collect();
            let texto = format!("---\n{texto}---\n\n## corpo\nc\n");
            let e = parse_doc(&texto).unwrap_err().to_string();
            assert!(e.contains(faltante), "faltando {faltante}: {e}");
        }
    }

    #[test]
    fn conteudo_solto_antes_do_corpo_e_erro() {
        let texto =
            "---\ntitulo: N\ntrilha: \nementa: \nlink: L\n---\n\nperdido aqui\n\n## corpo\nc\n";
        let e = parse_doc(texto).unwrap_err().to_string();
        assert!(e.contains("inesperado"), "erro: {e}");
    }

    #[test]
    fn writer_colapsa_valor_multilinha() {
        let mut h = header_jurisprudencia();
        h.ementa = "ICMS.\n  REGIME  DE\nST".into();
        let texto = render_doc(&h, None, "c");
        assert!(
            texto.contains("ementa: ICMS. REGIME DE ST\n"),
            "texto:\n{texto}"
        );
        let (h2, _, _) = parse_doc(&texto).unwrap();
        assert_eq!(h2.ementa, "ICMS. REGIME DE ST");
    }

    #[test]
    fn escrever_se_ausente_cria_e_depois_pula() {
        let dir = std::env::temp_dir().join(format!("auli-mddoc-emit-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut h = header_jurisprudencia();
        h.resumo_info = None; // produtor emite pendente

        assert!(escrever_se_ausente(&dir, &h, "Corpo coletado.").unwrap());
        let destino = dir.join("consulta-copat-no-0037-26.md");
        assert!(destino.exists());
        let (h2, res, corpo) = parse_doc(&std::fs::read_to_string(&destino).unwrap()).unwrap();
        assert_eq!(h2.titulo, h.titulo);
        assert_eq!(res, None, "produtor emite SEM resumo (pendente)");
        assert_eq!(corpo, "Corpo coletado.");

        // 2ª vez: pula e NÃO sobrescreve — nem com corpo diferente.
        assert!(!escrever_se_ausente(&dir, &h, "CORPO NOVO QUE NAO DEVE ENTRAR").unwrap());
        let (_, _, corpo2) = parse_doc(&std::fs::read_to_string(&destino).unwrap()).unwrap();
        assert_eq!(corpo2, "Corpo coletado.", "existente é intocável");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn escrever_se_ausente_nao_apaga_resumo_de_documento_ja_resumido() {
        // O cenário que a regra protege: re-coleta encontrando um doc que já custou LLM.
        let dir = std::env::temp_dir().join(format!("auli-mddoc-emit2-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let h = header_jurisprudencia(); // com resumo_info
        std::fs::create_dir_all(&dir).unwrap();
        let destino = dir.join("consulta-copat-no-0037-26.md");
        std::fs::write(&destino, render_doc(&h, Some("RESUMO CARO"), "corpo")).unwrap();

        let mut h_novo = header_jurisprudencia();
        h_novo.resumo_info = None;
        assert!(!escrever_se_ausente(&dir, &h_novo, "corpo recoletado").unwrap());
        let (_, res, _) = parse_doc(&std::fs::read_to_string(&destino).unwrap()).unwrap();
        assert_eq!(res.as_deref(), Some("RESUMO CARO"), "resumo preservado");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn lote_recusa_colisao_de_slug_nomeando_os_dois() {
        let dir = std::env::temp_dir().join(format!("auli-mddoc-colisao-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let docs = vec![
            (jurisprudencia("CONSULTA 1/26"), "a".to_string()),
            (jurisprudencia("CONSULTA 1-26"), "b".to_string()),
        ];
        let e = escrever_lote_se_ausente(&dir, &docs)
            .unwrap_err()
            .to_string();
        assert!(e.contains("colisão de slug"), "erro: {e}");
        assert!(
            e.contains("CONSULTA 1/26") && e.contains("CONSULTA 1-26"),
            "deve nomear os dois: {e}"
        );
        // E não escreveu nada: a checagem roda ANTES de tocar o disco.
        assert!(!dir.exists() || std::fs::read_dir(&dir).unwrap().count() == 0);
    }

    #[test]
    fn lote_conta_criados_e_pulados_e_o_mesmo_titulo_repetido_nao_e_colisao() {
        let dir = std::env::temp_dir().join(format!("auli-mddoc-lote-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let docs = vec![
            (jurisprudencia("A 1"), "x".to_string()),
            (jurisprudencia("A 1"), "x".to_string()),
        ];
        assert_eq!(escrever_lote_se_ausente(&dir, &docs).unwrap(), (1, 1));
        assert_eq!(escrever_lote_se_ausente(&dir, &docs).unwrap(), (0, 2));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn lote_com_resumo_emite_pendentes_e_resumidos_no_mesmo_lote() {
        let dir = std::env::temp_dir().join(format!("auli-mddoc-res-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let docs = vec![
            (
                jurisprudencia("ACÓRDÃO 510/24"),
                Some("Fundamentação do cabeçalho.".to_string()),
                "corpo A".to_string(),
            ),
            (
                jurisprudencia("ACÓRDÃO 141/26"),
                None,
                "corpo B".to_string(),
            ),
        ];
        assert_eq!(
            escrever_lote_se_ausente_com_resumo(&dir, &docs).unwrap(),
            (2, 0)
        );
        let ler = |slug: &str| {
            parse_doc(&std::fs::read_to_string(dir.join(format!("{slug}.md"))).unwrap()).unwrap()
        };
        let (_, res_a, corpo_a) = ler("acordao-510-24");
        assert_eq!(res_a.as_deref(), Some("Fundamentação do cabeçalho."));
        assert_eq!(corpo_a, "corpo A");
        let (_, res_b, _) = ler("acordao-141-26");
        assert_eq!(res_b, None, "sem resumo continua nascendo pendente");
        assert_eq!(
            escrever_lote_se_ausente_com_resumo(&dir, &docs).unwrap(),
            (0, 2)
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn escrever_se_ausente_recusa_titulo_sem_slug() {
        let dir = std::env::temp_dir().join(format!("auli-mddoc-emit3-{}", std::process::id()));
        let mut h = header_jurisprudencia();
        h.titulo = "///".into();
        assert!(escrever_se_ausente(&dir, &h, "c").is_err());
    }

    #[test]
    fn materializar_apaga_orfaos_e_regenera() {
        let dir = std::env::temp_dir().join(format!("auli-mddoc-mat-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("orfao.md"), "lixo de coleta anterior").unwrap();

        let docs = vec![(header_servico(), "Passos.".to_string())];
        assert_eq!(materializar_arvore(&dir, &docs, nome_servico).unwrap(), 1);
        assert!(!dir.join("orfao.md").exists(), "delete + rebuild");
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn materializar_recusa_colisao() {
        let dir = std::env::temp_dir().join(format!("auli-mddoc-mat2-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let h = header_servico();
        let docs = vec![(h.clone(), "a".to_string()), (h, "b".to_string())];
        let e = materializar_arvore(&dir, &docs, nome_servico)
            .unwrap_err()
            .to_string();
        assert!(e.contains("colisão de nome"), "erro: {e}");
    }

    #[test]
    fn nomes_de_arquivo_congelados() {
        // Trava de regressão: estes nomes são os que já estão materializados no disco. Mudá-los
        // renomeia árvores inteiras sem mudança de conteúdo.
        assert_eq!(
            slug("CONSULTA COPAT nº 0037/26"),
            "consulta-copat-no-0037-26"
        );
        assert_eq!(slug("PARECER Nº 25148"), "parecer-no-25148");
        assert_eq!(slug("ÁÉÍÓÚ ção"), "aeiou-cao");
        assert_eq!(slug("///abc///"), "abc");
        assert_eq!(slug(""), "");

        // Serviço: hash8 do LINK.
        let s = header_servico();
        assert_eq!(
            nome_servico(&s),
            format!(
                "emissao-de-guia-de-ipva-{:08x}",
                fnv1a64(s.link.as_bytes()) as u32
            )
        );
        // FAQ: hash8 de `link\ntitulo`, slug truncado em 60.
        let f = header_faq();
        assert_eq!(
            nome_faq(&f),
            format!(
                "como-faco-para-emitir-a-guia-de-ipva-{:08x}",
                fnv1a64(format!("{}\n{}", f.link, f.titulo).as_bytes()) as u32
            )
        );
    }

    #[test]
    fn slug_colide_para_titulos_que_so_diferem_em_pontuacao() {
        // Documenta a doutrina: quem materializa a árvore detecta a colisão e erra.
        assert_eq!(slug("CONSULTA 1/26"), slug("CONSULTA 1-26"));
    }

    /// Header mínimo de jurisprudência para os testes de lote.
    fn jurisprudencia(titulo: &str) -> DocHeader {
        DocHeader {
            titulo: titulo.into(),
            trilha: String::new(),
            ementa: format!("ementa de {titulo}"),
            link: format!("http://x/{titulo}"),
            orgao: None,
            resumo_info: None,
        }
    }
}
