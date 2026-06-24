# Plano de implementação — `auli-contract` (struct-fonte tipada)

> **Arquivo único de trabalho.** Coloque-o na raiz do repo (`auli_new/`). O Claude Code deve ler
> este arquivo por inteiro antes de começar. Contém: instruções de trabalho, o roteiro completo por
> fases, e o **código pronto** do crate `auli-contract` (para criar verbatim na Fase 1).

---

## 0. Instruções para o Claude Code

**Objetivo da mudança:** a struct passa a ser a **única fonte da verdade** do dado. O scraper
(`auli-collections`) compila o dado bruto numa `Table<P>`, preenchendo o campo `text_to_embed` de
cada registro. O engine (`auli-cli`/`auli-core`) apenas consome a struct e embedda esse campo —
sem parsing. O `portal-*.txt` deixa de ser contrato e vira um *print* legível.

**Regras (importantes):**
- **NÃO edite `auli-server/`.** É o monólito pré-refactor, mantido no disco apenas como **baseline
  de referência auditável**. O programa atual é o binário `auli` (em `auli/crates/auli-cli`).
- O **programa principal** é o binário `auli`, compilado a partir do workspace em `auli/`
  (`cd auli && cargo build --release --bin auli`). O `CARGO_TARGET_DIR` aponta para
  `auli-server/target` (reuso de artefatos) — **não** mexa nisso.
- **Modo de trabalho incremental:** execute **uma fase por vez**. Ao fim de cada fase: compile
  (`cargo build`) e rode os testes (`cargo test`) do que mexeu, e **peça confirmação** antes de
  seguir. No máximo **uma pergunta por resposta**.
- **Comunique-se em português do Brasil.**
- O **código do `auli-contract` já está pronto** na seção 6 deste arquivo — crie os arquivos
  exatamente como estão (não reinvente), depois compile/teste.
- **Não tente consertar o frontend** neste trabalho (escopo separado). Se remover um arquivo
  quebrar o frontend, **sinalize** em vez de corrigir.

---

## 1. Contexto — estado real do código (já revisado)

1. **O scraper já serializa JSON estruturado.** `faqs::run` grava `faqs.json` (árvore `FaqNode`) +
   `portal-faqs.txt`; `servicos::run` grava `servicos.json` (flat), `servicos-<tipo>.json`,
   `servicos-index.json` + `portal-servicos.txt`, todos em `../data/<id>/raw/`.
   **Decisão do dono:** `faqs.json` e `servicos.json` são **descartados** — a `Table<P>` do
   contrato passa a ser a única saída estruturada.
2. **Existe uma cópia morta do `corpus` no scraper:** `auli-collections/src/domain/collections.rs`
   duplica `auli-core::corpus` e **já divergiu** (lá `SERVICOS` é `## pergunta`+FullText; no engine
   é `//`+Description). Não é usada no fluxo — **apague**.
3. **O server embedda só a pergunta** (`rag.rs`: `embed_dense([question])`) e só recupera
   `services`+`faqs` no RAG. `pareceres`/`notas` são vetorizados mas **nunca consultados**.
4. **`parse_blocks`/`prepare_documents` só têm um caller de produção** (`update.rs`); o resto é
   teste. `parse_blocks_from_text` é citado como "web endpoint" mas **nenhum handler o usa**
   (`list_handler` usa `corpus::from_kind` + `store.list()`). Remover o lado de documentos **não**
   quebra a query.
5. **`Writer::upsert<P>(name, ids, embeddings, payloads: &[P])`** — hoje `P = String`, com a chave
   on-disk `document`. Mantendo `P = String`, o `rag.rs` (que injeta o payload cru no prompt) **não
   muda**.
6. **`manifest::CollectionEntry.file`** é o nome do **pack** (`<id>-<kind>.json`), não da fonte —
   mudar o formato de origem **não** afeta o manifesto.

### Layout de dados por entidade (alvo)

```
data/<id>/raw/<id>-<kind>.json     # Table<P> (contrato) — entrada do `auli update`
data/<id>/raw/portal-<kind>.txt    # print legível (auditoria) — saída da struct, nunca lido
data/<id>/packs/<id>-<kind>.json   # vetores — saída do `auli update`
data/<id>/packs/<id>.manifest.json # manifesto (identidade do embedding)
```

---

## 2. Relação com a Fase 0 já assinada (`data_integration_phase0.md`)

- **Mantém:** `registry.toml` como fonte única de entidades; `raw/` como saída do scraper;
  mapeamento de UI `servicos` → kind vetorial `services`; meta de **equivalência** nas 5 perguntas
  de referência (Apêndice A daquele doc).
- **Supersede a decisão #3** (binário alheio ao split, agregação de texto no script): o binário
  passa a **ler JSON estruturado** do `raw/`; o `portal-*.txt` deixa de ser fonte de packs.
- **Adia** `pareceres`/`notas`/`conteudos` (todos baseados em `ref/`, fora por ora). O **alvo de
  regressão** passa a ser **services ≈ 627 e faqs ≈ 1734**; pareceres(331)/notas(1) ficam ausentes
  até serem modelados como struct.
- **Re-vetorização é esperada** (a decisão #1 já aceitou que as respostas podem mudar) — então a
  meta é **equivalência**, não bit-paridade dos vetores.

---

## 3. Decisões resolvidas (recomendadas — confirme com o dono se quiser mudar)

- **D1 (topologia):** trazer `auli-collections` para dentro do workspace e usar `auli-contract`
  como crate-membro. *Fallback:* `auli-contract` standalone, referenciado por `path` pelos dois
  lados.
- **D2 (`text_to_embed`):** derivar dos **campos estruturados** no scraper, não do bloco de texto.
  - `faqs`: `text_to_embed = pergunta` (opcional: `origin + " " + pergunta`).
  - `servicos`: composição curta (`tipo | classe`, `titulo`, início da `descricao`).
  Aceita re-vetorização; valida por equivalência.
- **D3 (payload armazenado):** manter textual — `P = String`, preenchido por `stored_repr()` do
  payload. `rag.rs` e `list_handler` **não** mudam.

---

## 4. Roteiro por fases

### Fase 1 — Workspace + crate `auli-contract`

1. **Topologia (D1):** mover `auli-collections/` para `auli/crates/auli-collections` e adicioná-lo
   a `members` em `auli/Cargo.toml`. Conferir que os paths relativos do scraper (`../data`)
   continuam corretos a partir do novo CWD (rodando de `auli/`, `../data` resolve para a raiz).
   Build de sanidade antes de prosseguir.
2. **Criar `auli/crates/auli-contract`** com o código da **seção 6** (verbatim). Adicionar a
   `members`.
3. `cargo build -p auli-contract && cargo test -p auli-contract` → confirmar.

### Fase 2 — `auli-collections` emite o contrato e preenche `text_to_embed`

1. Adicionar `auli-contract` às deps de `auli-collections/Cargo.toml`.
2. **Apagar** `auli-collections/src/domain/collections.rs` (cópia morta/divergente) e seus `use`.
3. **FAQs** (`faqs/`): após `scrape` montar a árvore `FaqNode`, **achatar** para `Vec<Faq>` com a
   mesma travessia de `portal::render_portal_faqs` (um `Faq` por `FaqItem` de nó folha; carregar
   `origin`/`url` do nó). Preencher `text_to_embed` (D2). Montar `Table::new(id, "faqs", items)` e
   gravar `../data/<id>/raw/<id>-faqs.json` (serde).
4. **Serviços** (`servicos/`): após o `finish`, converter o flat dedup-por-`link` para
   `Vec<Servico>` do contrato, preencher `text_to_embed` (D2), montar `Table::new(id, "servicos",
   items)` e gravar `../data/<id>/raw/<id>-servicos.json` (serde).
5. **Descartar** os outputs `faqs.json` e `servicos.json` (decisão do dono). Manter `portal-*.txt`
   (print) e, por ora, `servicos-index.json` (não mexer — é manifesto de abas; fora de escopo).
6. **Print:** manter `render_portal_faqs` e `gerar_portal_services_txt` gerando o `portal-*.txt`
   sem mudar o formato. (Opcional: reapontá-los para renderizar a partir de `Table<P>`.)
7. `main.rs`: por entidade/kind: scrape → `Table<P>` → grava `<id>-<kind>.json` (serde) +
   `portal-<kind>.txt` (print).
8. `cargo build && cargo test` → confirmar. Rodar o scraper de `rs` (use `--usecache` se possível)
   e inspecionar `raw/rs-faqs.json` / `raw/rs-servicos.json`.

### Fase 3 — Engine consome o contrato (remove o parsing)

1. **`auli-cli/src/update.rs` (`run_update`):** em vez de `parse_blocks`+`prepare_documents`, por
   kind (isolado): desserializar `Table<P>` de `<source>/<id>-<kind>.json`;
   `to_embed = items.map(|it| it.text_to_embed().to_string())`;
   `stored = items.map(|it| it.stored_repr())`; resto igual (`embed_dense` → ids `id-1..N` →
   `Writer::reset::<String>` + `upsert` → `CollectionEntry`). Despacho por kind:
   `match nome { "faqs" => ingest::<Faq>(...), "servicos" => ingest::<Servico>(...) }` com helper
   genérico `ingest<P: Embeddable + DeserializeOwned + Clone>(...)`.
2. **`auli-core/src/corpus.rs`:** remover (vira morto): `EmbedStrategy`, `prepare_documents`,
   `extract_question`, `clean_servico`, `extract_servico_description`, `parse_blocks`,
   `parse_blocks_from_text`, `parse_block_lines`, e os campos `file`/`delimiter`/`embed` de
   `Collection`. **Manter:** `kind`, `n_results`, `from_kind`, e as consts `SERVICES`/`FAQS`
   (usadas por `rag.rs`) + `PARECERES`/`NOTAS` (alcançadas por `from_kind` no `list_handler`).
3. **`auli-core/src/lib.rs` e docstrings:** atualizar menções a `prepare_documents`/`parse_*`.
4. **`rag.rs` e `api/handlers/collections.rs`:** **nenhuma mudança**.
5. **`auli-core/src/manifest.rs`:** incrementar `STRATEGY_VERSION` (1 → 2).
6. **Remover** os testes obsoletos de `corpus.rs`.
7. `cargo build && cargo test` → confirmar.

### Fase 4 — `build-packs` e fonte do `auli update`

1. **`scripts/build-packs.sh`:** parar de agregar `portal-*.txt`. Apontar `--source` para
   `data/<id>/raw` (ou ajustar o contrato de `--source`/`run_update` para a pasta dos JSON do
   contrato), que o `auli update` passa a ler.
2. **`ref/` fora por ora:** remover `ref/` da entrada. `pareceres`/`notas`/`conteudos` ficam sem
   fonte até reentrarem como struct.

### Fase 5 — Build e verificação ponta a ponta

1. `cd auli && cargo build --release && cd ..` (workspace inteiro, mesmo lock).
2. Re-raspar `rs` (ou `--usecache`); conferir `raw/rs-faqs.json` (`Table<Faq>`),
   `raw/rs-servicos.json` (`Table<Servico>`), e os `portal-*.txt` (formato inalterado).
3. `scripts/build-packs.sh rs` → conferir `data/rs/packs/rs-faqs.json` e `rs-services.json` +
   manifesto com `strategy_version: 2`.
4. `./start_server.sh --no-tunnel` → conferir o boot: **services ≈ 627, faqs ≈ 1734**
   (pareceres/notas ausentes — esperado); manifesto novo válido.
5. **Regressão de equivalência:** rodar as 5 perguntas de referência (Apêndice A da Fase 0) e
   comparar (mesmos serviços/links citados). Equivalência, não identidade.
6. `grep -rn 'parse_blocks\|prepare_documents\|extract_question\|EmbedStrategy' auli/crates` deve
   voltar vazio fora de testes remanescentes.

### Fase 6 — Limpeza e documentação

1. Remover código morto confirmado.
2. Atualizar `auli_code.md` (módulos + `auli-contract`), `README.md` (diagrama scraper → contrato →
   engine → frontend), `auli_operations.md` (`build-packs`/`--source` no `raw/`), e anotar em
   `data_integration_phase0.md`/`roteiro_integracao_data.md` que a decisão #3 foi superada e
   pareceres/notas adiados.

---

## 5. Ordem incremental e pendências

**Ordem (um kind por vez):** `faqs` primeiro (maior volume, key mais sensível), depois `servicos`,
depois (futuro) `notas`/`pareceres`/`conteudos`. O engine tolera kinds ausentes (store vazio),
então a migração parcial é segura — mas migrar `faqs` e `servicos` juntos evita um galho no
`build-packs`/`update` para origens mistas.

**Pendências (não bloqueiam):** `ref/` como struct; unificar o JSON do frontend com o contrato;
fixar a fórmula exata de `text_to_embed` de `servicos`; grau de unificação do vocabulário de kinds
(`registry.toml`/frontend gerados de um `Kind` tipado).

---

## 6. Código pronto — crate `auli-contract`

Crie estes dois arquivos **verbatim**.

### `auli/crates/auli-contract/Cargo.toml`

```toml
[package]
name = "auli-contract"
version = "0.1.0"
edition = "2021"

# Crate "magro" de contrato: APENAS serde. Nenhuma dependência pesada (sem fastembed/ort/aws-lc),
# para que tanto o scraper (auli-collections) quanto o engine possam depender dele sem arrastar a
# árvore do ONNX Runtime. É o lugar único onde produtor e consumidor concordam sobre a forma do dado.
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

### `auli/crates/auli-contract/src/lib.rs`

```rust
//! `auli-contract` — a forma do dado, compartilhada entre o scraper (`auli-collections`) e o
//! engine (`auli-core`/`auli-cli`).
//!
//! A **struct é a única fonte da verdade**. O scraper compila o dado bruto em uma [`Table<P>`],
//! preenchendo o campo `text_to_embed` de cada registro (a "key" a ser vetorizada). O engine
//! apenas consome: lê a [`Table<P>`], embedda `text_to_embed` e armazena [`Embeddable::stored_repr`].
//! O arquivo `portal-*.txt` deixa de ser contrato e passa a ser um *print* legível da struct
//! (unidirecional: só escrito, nunca lido de volta).
//!
//! Este crate é deliberadamente magro (só `serde`): nada de embedder, HTTP ou domínio de
//! tributação. É o único ponto onde produtor e consumidor concordam.

use serde::{Deserialize, Serialize};

/// Envelope genérico de uma tabela. Cada *tipo de tabela* é uma instanciação:
/// `Table<Faq>`, `Table<Servico>`, etc. As tabelas são sempre processadas isoladamente, então o
/// envelope nunca precisa segurar tipos diferentes juntos — daí o genérico (em vez de um enum).
///
/// Persistido como JSON em `data/<id>/raw/<id>-<nome>.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table<P> {
    /// Id da entidade (ex.: `"rs"`).
    pub id: String,
    /// Nome da tabela (ex.: `"faqs"`, `"servicos"`).
    pub nome: String,
    /// Os registros desta tabela.
    pub items: Vec<P>,
}

impl<P> Table<P> {
    /// Cria uma tabela a partir da entidade, do nome e dos registros.
    pub fn new(id: impl Into<String>, nome: impl Into<String>, items: Vec<P>) -> Self {
        Self { id: id.into(), nome: nome.into(), items }
    }

    /// Quantos registros a tabela contém.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Verdadeiro se a tabela não tem registros.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// O que o engine precisa de cada registro `P`, sem conhecer seus campos: a key a embeddar e a
/// representação textual a armazenar (servida ao LLM no contexto do RAG).
///
/// `text_to_embed` é um campo **materializado pelo scraper** — aqui só o expomos; o engine não o
/// recalcula. `stored_repr` é derivado dos campos do registro.
pub trait Embeddable {
    /// A key vetorizada (preenchida na origem pelo scraper).
    fn text_to_embed(&self) -> &str;
    /// O payload textual armazenado junto do vetor (entra no prompt do RAG).
    fn stored_repr(&self) -> String;
}

/// Um registro da tabela `faqs`: um par pergunta/resposta achatado (uma entrada por pergunta),
/// com a trilha de navegação (`origin`) e a URL da página de origem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Faq {
    /// Texto da pergunta.
    pub pergunta: String,
    /// Texto da resposta.
    pub resposta: String,
    /// Breadcrumb da página (ex.: `"Inicial | Perguntas Frequentes | ..."`). Pode ser vazio.
    #[serde(default)]
    pub origin: String,
    /// URL canônica da página de origem.
    pub url: String,
    /// Key a embeddar — preenchida pelo scraper (ver `EmbedStrategy` portada para o scraper).
    pub text_to_embed: String,
}

impl Embeddable for Faq {
    fn text_to_embed(&self) -> &str {
        &self.text_to_embed
    }

    /// Reproduz o bloco `## pergunta` / `## resposta` (mesma forma do antigo `portal-faqs.txt`),
    /// para o contexto do RAG continuar coerente.
    fn stored_repr(&self) -> String {
        let mut s = String::from("## pergunta\n");
        if !self.origin.is_empty() {
            s.push_str(&self.origin);
            s.push('\n');
        }
        s.push_str(&self.pergunta);
        s.push_str("\n\n## resposta\n");
        s.push_str(&self.resposta);
        s.push_str(&format!("\nLink: {}", self.url));
        s
    }
}

/// Um registro da tabela `servicos`. Campos do serviço raspado, mais a key materializada.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Servico {
    /// Id sequencial por arquivo (começa em 1). Não é globalmente único — use `link` para isso.
    pub id: usize,
    /// Público/categoria (ex.: `"Cidadãos"`, `"Empresas"`).
    pub tipo: String,
    /// Classe/grupo do serviço (do título do card).
    pub classe: String,
    /// Órgão de origem.
    pub orgao: String,
    /// URL do serviço.
    pub link: String,
    /// Título legível.
    pub titulo: String,
    /// Descrição do serviço (corpo da página de detalhe).
    pub descricao: String,
    /// Key a embeddar — preenchida pelo scraper.
    pub text_to_embed: String,
}

impl Embeddable for Servico {
    fn text_to_embed(&self) -> &str {
        &self.text_to_embed
    }

    /// Reproduz o bloco `## pergunta` / `## resposta` no mesmo formato dos demais kinds: breadcrumb
    /// `tipo | classe` + título no `## pergunta`, descrição + link no `## resposta`.
    fn stored_repr(&self) -> String {
        format!(
            "## pergunta\n{} | {}\n{}\n\n## resposta\n{}\nLink: {}",
            self.tipo, self.classe, self.titulo, self.descricao, self.link
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_faq() -> Faq {
        Faq {
            pergunta: "Como emitir nota?".into(),
            resposta: "Acesse o portal.".into(),
            origin: "Inicial | FAQ".into(),
            url: "https://exemplo/faq/1".into(),
            text_to_embed: "Como emitir nota?".into(),
        }
    }

    #[test]
    fn faq_table_roundtrips_through_json() {
        let table = Table::new("rs", "faqs", vec![sample_faq()]);
        let json = serde_json::to_string(&table).unwrap();
        let back: Table<Faq> = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "rs");
        assert_eq!(back.nome, "faqs");
        assert_eq!(back.len(), 1);
        assert_eq!(back.items[0].pergunta, "Como emitir nota?");
    }

    #[test]
    fn embeddable_exposes_key_and_renders_block() {
        let faq = sample_faq();
        assert_eq!(faq.text_to_embed(), "Como emitir nota?");
        let block = faq.stored_repr();
        assert!(block.starts_with("## pergunta\nInicial | FAQ\nComo emitir nota?"));
        assert!(block.contains("## resposta\nAcesse o portal."));
        assert!(block.contains("Link: https://exemplo/faq/1"));
    }

    #[test]
    fn servico_block_has_breadcrumb_and_link() {
        let s = Servico {
            id: 1,
            tipo: "Empresas".into(),
            classe: "ICMS".into(),
            orgao: "SEFAZ".into(),
            link: "https://exemplo/svc/1".into(),
            titulo: "Emitir guia".into(),
            descricao: "Passos para emitir a guia.".into(),
            text_to_embed: "Empresas | ICMS Emitir guia".into(),
        };
        let block = s.stored_repr();
        assert!(block.starts_with("## pergunta\nEmpresas | ICMS\nEmitir guia"));
        assert!(block.contains("Link: https://exemplo/svc/1"));
    }
}
```

### Alteração no `auli/Cargo.toml` (workspace)

Adicionar `crates/auli-contract` ao `members`:

```toml
[workspace]
resolver = "2"
members = ["crates/auli-contract", "crates/vector-store", "crates/auli-core", "crates/auli-cli"]
```

---

## 7. Checklist

- [ ] D1/D2/D3 confirmadas (ou ajustadas com o dono).
- [ ] `auli-collections` movido para o workspace; `auli-contract` criado (seção 6) e compilando.
- [ ] Cópia morta `domain/collections.rs` apagada.
- [ ] Scraper grava `<id>-<kind>.json` (contrato) + `portal-*.txt` (print); `faqs.json`/`servicos.json` descartados; `text_to_embed` preenchido.
- [ ] `update.rs` consome o contrato; `corpus.rs` enxuto (kind + n_results + from_kind); `rag.rs`/`list_handler` intactos.
- [ ] `STRATEGY_VERSION` 1 → 2.
- [ ] `build-packs.sh`/`--source` no `raw/`; `ref/` fora.
- [ ] Build do workspace OK; scrape + packs + boot validados (services ≈ 627, faqs ≈ 1734).
- [ ] 5 perguntas de referência equivalentes.
- [ ] Código morto removido; docs atualizadas (incl. nota de supersessão da decisão #3).
