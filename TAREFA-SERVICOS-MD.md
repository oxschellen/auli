# TAREFA-SERVICOS-MD — serviços gravados em árvore `.md` (fonte para o `update`)

## Contexto

Os pareceres já vivem numa árvore `data/<id>/docs/pareceres/*.md` que é a FONTE do
`auli update` (G5b). Esta tarefa dá aos **serviços** o mesmo tratamento, com as
diferenças que o domínio impõe:

- **Serviços mudam no portal** (descrições editadas, serviços removidos). A árvore
  `data/<id>/docs/servicos/*.md` é **regenerada do zero a cada `process`**
  (delete + rebuild) — não existe incremental nem "existe ⇒ pula".
- **Sem sinopse.** O documento é só frontmatter + `## corpo` (a descrição). Não há
  seção `## sinopse` nem etapa `sinopse` para serviços.
- **Recorte:** a árvore vira fonte **só para o `update`** (coleção vetorial). Os
  artefatos JSON atuais (`<id>-servicos.json`, per-públicos, `<id>-servicos-index.json`,
  print `.txt`) continuam sendo gerados exatamente como hoje — o frontend segue
  consumindo os per-públicos nas tabs. Nada muda no frontend nesta tarefa.

A árvore fica em `data/<id>/docs/` — já fora do git pela política de repo
(código+config no GitHub, dados fora).

## Decisões fechadas (não rediscutir)

| # | Decisão |
|---|---------|
| D1 | Árvore `data/<id>/docs/servicos/*.md` **regenerada do zero** a cada `servicos::process`: apaga o diretório inteiro e reescreve. Sem semântica incremental. |
| D2 | Nome do arquivo = `slug(titulo)` + `-` + **hash curto do link** (8 hex do FNV-1a 64 truncado). Se `slug(titulo)` for vazio (título só de símbolos), usa só o hash — a identidade vem do link. Colisão de nome de arquivo dentro da entidade = erro alto ANTES de escrever qualquer arquivo. |
| D3 | Frontmatter **achatado**: `titulo`, `tipo`, `classe`, `orgao`, `link` — só a ocorrência primária, exatamente como o `Table<Servico>` já materializa. Parser estrito na mesma forma canônica do `mddoc` (chave desconhecida, duplicada, faltante ou linha fora de `chave: valor` ⇒ erro alto). |
| D4 | Sem `## sinopse` e sem etapa de sinopse. Documento = frontmatter + `## corpo`. Corpo pode ser vazio (seção presente, conteúdo vazio) — há serviços sem descrição. |
| D5 | Parser = **módulo irmão** `mddoc_servico.rs` no `auli-contract`. Reaproveita as funções mecânicas do `mddoc.rs` via `pub(crate)` (`CERCA`, `ANCORA_CORPO`, `separa_frontmatter`, `colapsa_linha`). O `mddoc.rs` dos pareceres fica **intocado em comportamento** — só visibilidade. |
| D6 | `fnv1a64` é reimplementado **privado** no `mddoc_servico.rs` (10 linhas, mesmas constantes do gêmeo em `auli-core/src/manifest.rs`). O contrato não pode ganhar dependência de `auli-core` — invariante "leve" do crate. Comentário aponta o gêmeo. |
| D7 | A fórmula do `text_to_embed` de serviços sobe para o contrato: `pub fn compose_servico_text_to_embed(tipo, classe, titulo, descricao)`, **byte a byte igual** à `servico_text_to_embed` atual do `auli-collections` (breadcrumb + título + snippet de 300 chars). Produtor (`process`) e consumidor (`update`) passam pelo ponto único — mesma doutrina do `compose_text_to_embed` dos pareceres. |
| D8 | `stored_repr` do `Servico` NÃO muda: o bloco `## pergunta`/`## resposta` servido ao RAG é idêntico. Esta tarefa muda a FONTE, não o conteúdo. |
| D9 | No `update`, a árvore substitui o `<id>-servicos.json` como fonte da coleção vetorial. **Transição:** árvore ausente ⇒ fallback ao JSON com aviso `⚠️` (entidades ainda não re-processadas continuam funcionando). O fallback é temporário e documentado para remoção futura. |
| D10 | `id` sequencial e `text_to_embed` são **rematerializados na leitura** da árvore, em ordem de nome de arquivo (ordem estável ⇒ pack reproduzível), como nos pareceres. |
| D11 | `docs_hash` do manifesto passa a ser computado sobre `docs/` **sempre que o diretório existir**, independentemente de haver pareceres (hoje o cálculo mora dentro de `preparar_pareceres` e entidade sem pareceres fica sem hash). `hash_docs_tree` já é recursivo — cobre `pareceres/` e `servicos/` juntos sem mudança. |

## Fases

- **S1** — `auli-contract`: visibilidade `pub(crate)` no `mddoc.rs` + módulo novo `mddoc_servico.rs` (header, parser, render, slug, materialização) + testes.
- **S2** — `auli-contract` + `auli-collections`: `compose_servico_text_to_embed` no contrato; `servicos/mod.rs` delega para ela (edição cirúrgica).
- **S3** — `auli-collections`: `servicos::process` materializa a árvore após derivar a `Table<Servico>`.
- **S4** — `auli-cli`: `update` lê a árvore (`preparar_servicos`), com fallback D9.
- **S5** — `auli-cli`: `docs_hash` movido para fora de `preparar_pareceres` (D11).
- **S6** — regressão RS (roteiro manual no fim; executado pelo Carlos, não por esta tarefa).

Padrão atômico da casa para edições: localizar a string antiga (deve ocorrer
exatamente 1 vez), preparar todas as substituições em memória, gravar uma vez.

---

## S1 — `auli-contract`: visibilidade + módulo novo

### S1a — edições em `crates/auli-contract/src/mddoc.rs` (só visibilidade)

| antiga (ocorre 1×) | nova |
|---|---|
| `const CERCA: &str = "---";` | `pub(crate) const CERCA: &str = "---";` |
| `const ANCORA_CORPO: &str = "## corpo";` | `pub(crate) const ANCORA_CORPO: &str = "## corpo";` |
| `fn colapsa_linha(s: &str) -> String {` | `pub(crate) fn colapsa_linha(s: &str) -> String {` |
| `fn separa_frontmatter(texto: &str) -> Result<(&str, &str)> {` | `pub(crate) fn separa_frontmatter(texto: &str) -> Result<(&str, &str)> {` |

Nada mais muda no `mddoc.rs`. (`slug` já é `pub`.)

### S1b — arquivo NOVO: `crates/auli-contract/src/mddoc_servico.rs`

```rust
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
    let (mut titulo, mut tipo, mut classe, mut orgao, mut link) =
        (None, None, None, None, None);

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
```

### S1c — fiação em `crates/auli-contract/src/lib.rs`

Ao lado do `pub mod mddoc;` existente, acrescentar:

```rust
pub mod mddoc_servico;
```

### S1d — testes (embutidos no `mddoc_servico.rs`, `#[cfg(test)] mod tests`)

1. `round_trip`: `render_doc_servico` → `parse_doc_servico` devolve header e corpo
   idênticos (corpo multilinha com linha `---` e linha `## corpo` DENTRO dele —
   nada quebra o parse, como nos pareceres).
2. `corpo_vazio_e_valido`: render com `corpo=""` → parse devolve `""`.
3. `recusa_chave_desconhecida`, `recusa_chave_duplicada`, `recusa_chave_faltante`
   (uma por chave obrigatória basta para `titulo`), `recusa_sem_corpo`,
   `recusa_conteudo_antes_do_corpo` (ex.: linha `## sinopse` — serviço não tem).
4. `slug_servico_determinismo_e_distincao`: mesmo `(titulo, link)` ⇒ mesmo nome;
   mesmo título com links diferentes ⇒ nomes diferentes; título só de símbolos
   (`"§§§"`) ⇒ só o hash.
5. `materializar_apaga_orfaos`: gravar árvore com A e B; regravar só com A; o
   arquivo de B não existe mais.
6. `materializar_recusa_colisao`: dois itens com mesmo título e mesmo link ⇒ erro
   nomeando os dois; diretório NÃO foi tocado (pré-cheque vem antes do delete).

---

## S2 — `compose_servico_text_to_embed` no contrato

### S2a — `crates/auli-contract/src/lib.rs`

Logo após `compose_text_to_embed` (pareceres), acrescentar — **byte a byte igual**
à `servico_text_to_embed` atual do `auli-collections`:

```rust
/// Key de embedding de um serviço: breadcrumb `tipo | classe` + título + snippet
/// da descrição (300 chars). Vive no contrato porque os DOIS lados precisam dela:
/// o produtor (`servicos::process`) ao materializar, e o `auli update` ao
/// rematerializar na leitura da árvore `docs/servicos/*.md`.
pub fn compose_servico_text_to_embed(tipo: &str, classe: &str, titulo: &str, descricao: &str) -> String {
    let snippet: String = descricao.chars().take(300).collect();
    format!("{} | {}\n{}\n{}", tipo, classe, titulo, snippet.trim())
        .trim()
        .to_string()
}
```

Teste no contrato: fixture com descrição > 300 chars; comparar com o valor
esperado literal (congela a fórmula).

### S2b — `crates/auli-collections/src/servicos/mod.rs`

A função local vira delegação (assinatura preservada — call sites não mudam):

```rust
fn servico_text_to_embed(tipo: &str, classe: &str, titulo: &str, body: &str) -> String {
    auli_contract::compose_servico_text_to_embed(tipo, classe, titulo, body)
}
```

---

## S3 — materialização no `servicos::process`

Em `crates/auli-collections/src/servicos/mod.rs`, no fim do `process(id, data_dir, coleta)`,
depois de todos os artefatos de `raw/` (nenhum deles muda):

```rust
// Árvore `.md` — a FONTE do `auli update` para serviços (TAREFA-SERVICOS-MD).
// Regenerada do zero a cada process: serviços mudam no portal (D1). Os artefatos
// JSON acima continuam: o frontend consome os per-públicos, o update lê a árvore.
let base = std::path::Path::new(data_dir)
    .parent()
    .ok_or_else(|| format!("data_dir sem pai: {data_dir}"))?;
let docs_dir = base.join("docs").join("servicos");
let docs: Vec<(auli_contract::mddoc_servico::ServicoHeader, String)> = items
    .iter()
    .map(|s| {
        (
            auli_contract::mddoc_servico::ServicoHeader {
                titulo: s.titulo.clone(),
                tipo: s.tipo.clone(),
                classe: s.classe.clone(),
                orgao: s.orgao.clone(),
                link: s.link.clone(),
            },
            s.descricao.clone(),
        )
    })
    .collect();
let gravados = auli_contract::mddoc_servico::materializar_arvore(&docs_dir, &docs)
    .map_err(|e| format!("materialização da árvore de serviços: {e}"))?;
println!("📄 docs: {gravados} serviços materializados em {}", docs_dir.display());
```

Atenção: `items` precisa continuar vivo neste ponto (hoje ele é movido para
`Table::new` — clonar os campos ANTES, ou materializar antes de construir a
`Table`, o que for a edição menor; preservar o conteúdo dos artefatos existentes
byte a byte).

---

## S4 — `update` lê a árvore

Em `crates/auli-cli/src/update.rs`:

### S4a — função nova `preparar_servicos`

Espelho da `preparar_pareceres`, sem sinopse e sem hash (o hash sai na S5):

```rust
/// Lê a árvore `docs/servicos/*.md` — a FONTE (TAREFA-SERVICOS-MD) — e devolve os
/// serviços prontos para vetorizar, ou `None` se a entidade não tem árvore
/// (fallback de transição: o chamador cai no JSON com aviso).
///
/// `id` sequencial e `text_to_embed` são rematerializados aqui, em ordem de nome
/// de arquivo — pack reproduzível, mesma doutrina dos pareceres.
fn preparar_servicos(docs_dir: &Path) -> Result<Option<Vec<auli_contract::Servico>>> {
    let dir = docs_dir.join("servicos");
    if !dir.exists() {
        return Ok(None);
    }
    let mut caminhos: Vec<std::path::PathBuf> = std::fs::read_dir(&dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "md"))
        .collect();
    caminhos.sort();
    if caminhos.is_empty() {
        return Ok(None);
    }

    let mut servicos = Vec::with_capacity(caminhos.len());
    for (i, caminho) in caminhos.iter().enumerate() {
        let texto = std::fs::read_to_string(caminho)?;
        let (header, corpo) = auli_contract::mddoc_servico::parse_doc_servico(&texto)
            .map_err(|e| format!("`{}` não parseia ({e})", caminho.display()))?;
        servicos.push(auli_contract::Servico {
            id: i + 1,
            text_to_embed: auli_contract::compose_servico_text_to_embed(
                &header.tipo, &header.classe, &header.titulo, &corpo,
            ),
            tipo: header.tipo,
            classe: header.classe,
            orgao: header.orgao,
            link: header.link,
            titulo: header.titulo,
            descricao: corpo,
        });
    }
    println!("📄 docs: {} serviços lidos de {}", servicos.len(), dir.display());
    Ok(Some(servicos))
}
```

### S4b — fiação no `run_update`

Substituir o bloco atual de serviços (o `ingest::<auli_contract::Servico>` com
comentário `// servicos: <entity>-servicos.json (contract) -> pack <entity>-servicos.`) por:

```rust
// servicos: a FONTE é a árvore `docs/servicos/*.md` (TAREFA-SERVICOS-MD).
// Fallback de TRANSIÇÃO: entidade sem árvore ainda lê o `<entity>-servicos.json`
// com aviso — remover quando todas as entidades tiverem sido re-processadas.
match preparar_servicos(&docs_dir)? {
    Some(servicos) => {
        println!("🔢 servicos: {} registros → vetorizando...", servicos.len());
        entries.push(ingest_items(&embedder, &writer, &entity, "servicos", &servicos, &out)?);
    }
    None => {
        println!("⚠️  {}: sem árvore docs/servicos — lendo o JSON legado (rode `auli-collections {} process` para migrar).", entity, entity);
        if let Some(entry) = ingest::<auli_contract::Servico>(
            &embedder, &writer, &entity, "servicos", &format!("{}-servicos.json", entity), &source, &out,
        )? {
            entries.push(entry);
        }
    }
}
```

Isso exige mover a construção do `docs_dir` (hoje declarada logo antes do bloco de
pareceres) para ANTES do bloco de serviços — pura reordenação, sem mudança de valor.

---

## S5 — `docs_hash` fora do `preparar_pareceres` (D11)

1. `preparar_pareceres` muda de assinatura:
   `Result<Option<(Option<String>, Vec<Consulta>)>>` → `Result<Option<Vec<Consulta>>>`.
   Internamente: remover a linha `let hash = manifest::hash_docs_tree(docs_dir)?;`
   e devolver `Ok(Some(consultas))`. O parâmetro `docs_dir` passa a receber
   diretamente o diretório `docs/` (como hoje) só para montar `docs_dir.join("pareceres")` —
   inalterado.
2. No `run_update`, o hash é computado uma única vez, depois dos dois blocos
   (serviços e pareceres):

```rust
// Hash da árvore `docs/` inteira (pareceres E serviços): fonte de conteúdo
// vetorizado/servido, então divergir dela é tão grave quanto pack corrompido.
// `hash_docs_tree` devolve `None` se `docs/` não existe — entidade sem árvore.
let docs_hash = manifest::hash_docs_tree(&docs_dir)?;
```

(Apagar o `let mut docs_hash = None;` e o `docs_hash = hash;` do fluxo antigo.)

Efeito colateral desejado: entidade que só tem árvore de serviços também ganha
`docs_hash` — e o `validate_docs_hash` do boot passa a proteger a árvore de
serviços de graça.

### Teste (S4/S5, em `update.rs` ou módulo de teste existente)

`preparar_servicos_le_arvore_em_ordem`: gravar em tempdir dois `.md` via
`render_doc_servico` (nomes `b-....md` e `a-....md`), chamar `preparar_servicos`,
verificar: ordem por nome de arquivo, `id` 1 e 2, `text_to_embed` igual a
`compose_servico_text_to_embed` dos campos, `descricao == corpo`. E
`preparar_servicos_none_sem_arvore`: tempdir vazio ⇒ `Ok(None)`.

---

## Aceite

1. `cargo test` verde no workspace inteiro.
2. `cargo clippy` sem warnings novos.
3. Os artefatos de `raw/` gerados pelo `process` são **byte a byte idênticos** aos
   de antes da tarefa (a materialização só ACRESCENTA a árvore).
4. Rodar `process` duas vezes seguidas: a segunda regenera a árvore sem erro e sem
   órfãos (contagem de arquivos == contagem de serviços da coleta).

## S6 — regressão RS (roteiro manual, executado pelo Carlos)

1. Backup do estado atual: contagem da coleção `rs-servicos` e amostra de
   `text_to_embed` (ou o próprio `rs-servicos.json`).
2. `auli-collections rs process` (snapshot existente) → confere
   `data/rs/docs/servicos/` criada, contagem de `.md` == itens do
   `rs-servicos.json`.
3. `auli update` no RS → log mostra `📄 docs: N serviços lidos`, N igual à
   contagem do JSON.
4. Comparar o multiset de `text_to_embed` da coleção nova com o do
   `rs-servicos.json` (a ORDEM pode mudar — a fonte agora ordena por nome de
   arquivo; o CONJUNTO não pode). Sugestão: extrair os dois conjuntos, ordenar,
   `diff`.
5. Reiniciar o servidor: boot valida `docs_hash` sem recusa; tab de Serviços do
   frontend inalterada (JSONs intactos).
6. Sanidade de retrieval: 2–3 perguntas de serviço conhecidas no Chat devolvem os
   mesmos serviços de antes.
