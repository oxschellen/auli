# TAREFA-FAQS-MD — FAQs gravadas em árvore `.md` (fonte para o `update`)

## Contexto

Espelho direto da TAREFA-SERVICOS-MD (mergeada no PR #107), agora para as FAQs.
A doutrina é a mesma dos serviços — leia aquela tarefa e o `mddoc_servico.rs`
antes de começar; esta é mais curta porque só registra as DIFERENÇAS.

- **FAQs mudam no portal** ⇒ árvore `data/<id>/docs/faqs/*.md` **regenerada do
  zero** a cada `derive_faqs::process` (delete + rebuild).
- **Sem sinopse.** Documento = frontmatter + `## corpo` (a resposta).
- **Recorte:** a árvore é fonte **só para o `update`**. Os artefatos de `raw/`
  (`<id>-faqs.json`, `<id>-portal-faqs.txt`) continuam idênticos.
- **A fórmula do `text_to_embed` NÃO muda nesta tarefa.** Existe decisão tomada de
  embedar pergunta+resposta juntas, mas ela é tarefa FUTURA separada: aqui a
  fórmula atual (`origin` + pergunta) é preservada byte a byte, para a regressão
  por multiset ser limpa. Não antecipar.

## Decisões fechadas (não rediscutir)

| # | Decisão |
|---|---------|
| D1 | Árvore `data/<id>/docs/faqs/*.md` regenerada do zero a cada `derive_faqs::process`. Sem incremental. |
| D2 | **A `url` NÃO é única entre FAQs** (várias perguntas na mesma página) — a identidade é `(url, pergunta)`. Nome do arquivo = `slug(pergunta)` **truncado a ~60 chars em fronteira de hífen** + `-` + hash8 do FNV-1a 64 de `url + "\n" + pergunta`. Slug vazio ⇒ só o hash. Colisão de nome = erro alto ANTES de escrever qualquer arquivo. |
| D3 | Frontmatter achatado e estrito: `pergunta`, `origin`, `url` — as três chaves obrigatórias; `origin` pode ter **valor vazio** (chave presente, valor vazio é legal — espelha o `#[serde(default)]` do contrato). |
| D4 | Sem `## sinopse`. `## corpo` = resposta; corpo vazio é válido. |
| D5 | Parser = **terceiro irmão** `mddoc_faq.rs` no `auli-contract`. `mddoc.rs` e `mddoc_servico.rs` ficam intocados em comportamento — só o `fnv1a64` do `mddoc_servico.rs` vira `pub(crate)` para reuso. SEM generalização de frontmatter: com três casos na mão, a duplicação real é só o loop de ~30 linhas; generaliza-se quando doer. |
| D6 | A fórmula do `text_to_embed` sobe para o contrato como `compose_faq_text_to_embed(origin, pergunta)`, **byte a byte igual** à `faq_from_raw` atual: `origin` vazio ⇒ só a pergunta; senão `"{origin} {pergunta}"`. Produtor e `update` passam pelo ponto único. |
| D7 | `stored_repr` do `Faq` NÃO muda. |
| D8 | No `update`, a árvore substitui o `<id>-faqs.json` com **fallback de transição** ao JSON + aviso `⚠️`, idêntico ao dos serviços. Ordem de leitura = nome de arquivo (pack reproduzível). `Faq` não tem campo `id` — rematerialização é só a struct + `text_to_embed`. |
| D9 | `docs_hash`: NADA a fazer — desde a TAREFA-SERVICOS-MD ele cobre `docs/` inteiro, e `docs/faqs/` entra de graça. |

## Fases

- **FQ1** — `auli-contract`: `pub(crate)` no `fnv1a64` + módulo novo `mddoc_faq.rs` + testes.
- **FQ2** — `auli-contract` + `auli-collections`: `compose_faq_text_to_embed` no contrato; `derive_faqs::faq_from_raw` delega.
- **FQ3** — `auli-collections`: `derive_faqs::process` materializa a árvore.
- **FQ4** — `auli-cli`: `preparar_faqs` no `update` + fiação com fallback.
- **FQ5** — regressão RS (roteiro manual no fim; executado pelo Carlos).

Padrão atômico da casa para edições: localizar a string antiga (1 ocorrência),
todas as substituições em memória, gravar uma vez.

---

## FQ1 — `auli-contract`: `mddoc_faq.rs`

### FQ1a — edição em `crates/auli-contract/src/mddoc_servico.rs`

| antiga (ocorre 1×) | nova |
|---|---|
| `fn fnv1a64(bytes: &[u8]) -> u64 {` | `pub(crate) fn fnv1a64(bytes: &[u8]) -> u64 {` |

(Ajustar o doc-comment do `fnv1a64` mencionando que o `mddoc_faq` também o usa.)

### FQ1b — arquivo NOVO: `crates/auli-contract/src/mddoc_faq.rs`

Espelho do `mddoc_servico.rs`; abaixo só o que DIFERE — o esqueleto (imports,
forma do parser estrito, `separa_corpo`, doc-comments no mesmo tom) copia o irmão.

```rust
//! `mddoc_faq` — o contrato do arquivo `.md` que é a **fonte** de uma FAQ.
//! Terceiro irmão de [`crate::mddoc`] (pareceres) e [`crate::mddoc_servico`].
//!
//! - **Sem `## sinopse`**: documento = frontmatter + `## corpo` (a resposta).
//! - **Árvore regenerada a cada coleta** (delete + rebuild), como os serviços.
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

use crate::mddoc::{ANCORA_CORPO, CERCA, colapsa_linha, separa_frontmatter, slug};
use crate::mddoc_servico::fnv1a64;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FaqHeader {
    pub pergunta: String,
    /// Breadcrumb da página. Pode ser vazio — a chave é obrigatória no arquivo,
    /// o VALOR pode ser vazio (espelha o `#[serde(default)]` do contrato).
    pub origin: String,
    pub url: String,
}

pub fn parse_doc_faq(texto: &str) -> Result<(FaqHeader, String)> { /* = irmão */ }
pub fn render_doc_faq(header: &FaqHeader, corpo: &str) -> String { /* = irmão; a
    linha `origin` é SEMPRE escrita, mesmo com valor vazio (D3) */ }

/// Nome de arquivo da FAQ: `slug(pergunta)` truncado + `-` + hash8 de
/// `url + "\n" + pergunta`. O truncamento (60 chars, cortado na última fronteira
/// de hífen dentro do limite) mantém o nome legível sem estourar o filesystem —
/// a identidade real está no hash, que cobre o par `(url, pergunta)`.
pub fn slug_faq(pergunta: &str, url: &str) -> String {
    let s = slug_truncado(&slug(pergunta), 60);
    let h = format!("{:08x}", fnv1a64(format!("{url}\n{pergunta}").as_bytes()) as u32);
    if s.is_empty() { h } else { format!("{s}-{h}") }
}

/// Trunca um slug (ASCII puro: `[a-z0-9-]`, então fatiar por byte é seguro) na
/// última fronteira de hífen dentro de `max`; sem hífen no trecho, corte seco.
fn slug_truncado(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    match s[..max].rfind('-') {
        Some(i) => &s[..i],
        None => &s[..max],
    }
}

/// Delete + rebuild, pré-cheque de colisão nomeando as duas PERGUNTAS em
/// conflito — mesma mecânica do irmão dos serviços.
pub fn materializar_arvore(dir: &Path, docs: &[(FaqHeader, String)]) -> Result<usize> { /* = irmão,
    trocando `slug_servico(titulo, link)` por `slug_faq(pergunta, url)` */ }
```

Detalhes obrigatórios do espelhamento:
- `parse_frontmatter_faq`: chaves `pergunta`/`origin`/`url`, todas obrigatórias
  (`exige`), duplicada/desconhecida/fora-da-forma ⇒ erro alto. **Valor vazio é
  válido para as três no parser** (a semântica de obrigatoriedade é da CHAVE);
  quem nunca produz `pergunta`/`url` vazias é o scraper.
- `separa_corpo`: idêntico ao irmão (conteúdo antes da âncora ⇒ erro; corpo
  engole até o fim; corpo vazio válido).

### FQ1c — fiação em `crates/auli-contract/src/lib.rs`

Ao lado dos `pub mod` existentes: `pub mod mddoc_faq;`

### FQ1d — testes (embutidos, mesmos nomes do irmão adaptados)

1. `round_trip` (corpo multilinha com `---` e `## corpo` dentro).
2. `round_trip_com_origin_vazio` — **novo, específico das FAQs**: header com
   `origin: ""` renderiza, parseia e devolve `""`.
3. `corpo_vazio_e_valido`.
4. `recusa_chave_desconhecida` / `_duplicada` / `_faltante` (para `origin`
   AUSENTE também — chave faltante ≠ valor vazio) / `recusa_sem_corpo` /
   `recusa_conteudo_antes_do_corpo`.
5. `slug_faq_identidade_e_truncamento`: (a) mesma pergunta em urls diferentes ⇒
   nomes diferentes; (b) perguntas diferentes na MESMA url ⇒ nomes diferentes;
   (c) pergunta longa (>60 chars de slug) ⇒ nome ≤ 60+1+8 e sem hífen pendurado
   no fim do trecho truncado; (d) pergunta só de símbolos ⇒ só o hash.
6. `materializar_apaga_orfaos` e `materializar_recusa_colisao` (duplicar o
   MESMO par `(url, pergunta)` ⇒ erro nomeando as duas perguntas; diretório
   intocado).

---

## FQ2 — `compose_faq_text_to_embed` no contrato

### FQ2a — `crates/auli-contract/src/lib.rs`

Ao lado de `compose_servico_text_to_embed`, **byte a byte igual** à lógica do
`faq_from_raw` atual:

```rust
/// Key de embedding de uma FAQ: breadcrumb `origin` + a pergunta (só a pergunta
/// quando não há breadcrumb) — a mesma key do antigo `EmbedStrategy::QuestionKey`.
/// Vive no contrato porque os DOIS lados precisam dela: o produtor
/// (`derive_faqs`) e o `auli update` ao rematerializar da árvore `docs/faqs/*.md`.
///
/// NOTA: existe decisão de evoluir para pergunta+resposta — é tarefa FUTURA
/// separada (mudança de vetor isolada, atribuível só a ela). Não antecipar aqui.
pub fn compose_faq_text_to_embed(origin: &str, pergunta: &str) -> String {
    if origin.is_empty() {
        pergunta.to_string()
    } else {
        format!("{} {}", origin, pergunta)
    }
}
```

Teste no contrato congelando os dois ramos (com e sem `origin`).

### FQ2b — `crates/auli-collections/src/derive_faqs.rs`

No `faq_from_raw`, substituir o `if raw.origin.is_empty() { ... } else { ... }`
local por `auli_contract::compose_faq_text_to_embed(&raw.origin, &raw.pergunta)`.

---

## FQ3 — materialização no `derive_faqs::process`

No fim do `process(id, data_dir, coleta)`, depois dos dois artefatos de `raw/`
(nenhum muda), o mesmo bloco do irmão dos serviços trocando os nomes:

```rust
// Árvore `.md` — a FONTE do `auli update` para faqs (TAREFA-FAQS-MD). Regenerada
// do zero a cada process (D1). Os artefatos de `raw/` acima seguem intactos.
let base = std::path::Path::new(data_dir)
    .parent()
    .ok_or_else(|| format!("data_dir sem pai: {data_dir}"))?;
let docs_dir = base.join("docs").join("faqs");
let docs: Vec<(auli_contract::mddoc_faq::FaqHeader, String)> = table
    .items
    .iter()
    .map(|f| {
        (
            auli_contract::mddoc_faq::FaqHeader {
                pergunta: f.pergunta.clone(),
                origin: f.origin.clone(),
                url: f.url.clone(),
            },
            f.resposta.clone(),
        )
    })
    .collect();
let gravados = auli_contract::mddoc_faq::materializar_arvore(&docs_dir, &docs)
    .map_err(|e| format!("materialização da árvore de faqs: {e}"))?;
println!("📄 docs: {gravados} faqs materializadas em {}", docs_dir.display());
```

Se o teste golden do RS existir para faqs, ajustar como no PR #107 (tempdir
imitando `<base>/raw` para a árvore cair dentro do temporário).

---

## FQ4 — `update` lê a árvore

Em `crates/auli-cli/src/update.rs` (o `docs_dir` já é montado antes dos blocos
desde o PR #107):

1. Função nova `preparar_faqs(docs_dir) -> Result<Option<Vec<auli_contract::Faq>>>`,
   espelho da `preparar_servicos`: lê `docs_dir.join("faqs")`, ordena por nome,
   parseia via `parse_doc_faq`, monta:

```rust
auli_contract::Faq {
    text_to_embed: auli_contract::compose_faq_text_to_embed(&header.origin, &header.pergunta),
    pergunta: header.pergunta,
    resposta: corpo,
    origin: header.origin,
    url: header.url,
}
```

2. Substituir o bloco atual de faqs (o `ingest::<auli_contract::Faq>` no topo do
   `run_update`) pelo `match` com fallback de transição, idêntico ao dos serviços
   (aviso `⚠️ ... rode auli-collections {entity} process para migrar`). Atenção:
   o bloco de faqs vem ANTES da declaração do `docs_dir` hoje — mover o bloco de
   faqs para DEPOIS dela (pura reordenação; a ordem faqs/serviços entre si é
   indiferente, os packs são independentes).

3. Testes: `preparar_faqs_le_arvore_em_ordem` (dois `.md`, ordem por nome,
   `text_to_embed` pelo ponto único, `resposta == corpo`, `origin` vazio
   preservado) e `preparar_faqs_none_sem_arvore`.

---

## Aceite

1. `cargo test` verde no workspace; `cargo clippy` sem warnings novos.
2. Artefatos de `raw/` do `derive_faqs::process` byte a byte idênticos aos de
   antes (a materialização só ACRESCENTA a árvore).
3. `process` duas vezes seguidas: segunda regenera sem erro e sem órfãos.
4. `docs_hash`: nenhum código novo — verificar por teste existente que a árvore
   `docs/faqs/` altera o hash (o `hash_docs_tree` é recursivo sobre `docs/`).

## FQ5 — regressão RS (roteiro manual, executado pelo Carlos)

1. Contagem e amostra da coleção `rs-faqs` atual (ou o próprio `rs-faqs.json`).
2. `auli-collections rs process` → `data/rs/docs/faqs/` criada; contagem de `.md`
   == itens do `rs-faqs.json`.
3. `auli update` no RS → `📄 docs: N faqs lidas`, N igual à contagem do JSON.
4. Multiset de `text_to_embed` novo vs. `rs-faqs.json`: conjuntos idênticos (a
   ordem muda — fonte agora ordena por nome de arquivo).
5. Observação esperada: `stored_repr` pode divergir só por espaços nas BORDAS da
   resposta (o render apara o corpo) — diferença de trim é aceitável; qualquer
   outra divergência é bug.
6. Boot sem recusa de `docs_hash`; tab de FAQs do frontend inalterada; 2–3
   perguntas de FAQ conhecidas no Chat devolvem os mesmos resultados.
