# Auli — Descrição Técnica do Código

Documento técnico auditável do Projeto Auli, baseado na leitura direta do código-fonte
dos três componentes (`auli-server`, `auli-frontend`, `auli-collections`). Cada afirmação
relevante cita o arquivo que a sustenta. Quando algo não pôde ser confirmado no código,
está marcado como **NÃO CONFIRMADO NO CÓDIGO**.

**Operação:** para **compilar, gerar os dados e subir o app** (com Cloudflare Tunnel) e saber **onde ficam
os logs**, ver o runbook [auli_operations.md](auli_operations.md).

> Convenção deste documento: distingo explicitamente o que está **implementado e ativo**
> do que está apenas **modelado/inativo** (rotas sem fluxo completo, dependências
> declaradas mas não usadas, tipos de conteúdo ingeridos mas não consultados, código
> duplicado/divergente entre repositórios).

---

## 1. O que é a Auli

Auli é um assistente virtual (RAG) que auxilia servidores tributários no atendimento ao
cidadão sobre impostos estaduais. É concebido como **multi-tenant** (um servidor/uma UI
para várias entidades = secretarias estaduais), open-source, com ênfase em privacidade
(embeddings locais). O piloto é a SEFAZ-RS.

Pipeline de resposta (caminho realmente executado):

```
pergunta  →  embedding local (fastembed/BGE-M3, in-process)  →  busca vetorial (vector store in-process)  →  LLM externo gera a resposta
```

O ecossistema tem três partes:

| Componente                | Papel                                                             | Linguagem/stack              |
| ------------------------- | ----------------------------------------------------------------- | ---------------------------- |
| `auli-server` (workspace) | Backend REST + pipeline RAG (binário `auli`: `server` + `update`) | Rust (Axum, Tokio)           |
| `auli-frontend`           | Interface web (chat + navegação)                                  | React 19 + TypeScript + Vite |
| scrapers + `auli-collections` | Coleta (um `auli-scraper-<e>` por entidade) + derivação do contrato ingerido | Rust (síncrono, `ureq`)      |

> **Atualização — `auli-contract` (2026-06-23).** O workspace ganhou o crate magro
> **`auli-contract`** (serde-only): a **forma do dado** (`Table<P>`, `Faq`, `Servico`, trait
> `Embeddable`) compartilhada entre o scraper e o engine. O **`auli-collections` foi movido para
> dentro do workspace** (`auli-server/crates/auli-collections`, 5º membro). Fluxo: o scraper compila
> `Table<P>` preenchendo `text_to_embed` → `data/<id>/raw/<id>-<kind>.json`; o `auli update` lê o
> contrato, embeda `text_to_embed` e armazena `stored_repr` (sem mais parsing de `portal-*.txt`,
> que viraram só _print_ de auditoria). `STRATEGY_VERSION` foi para **2**. Caminhos `auli-collections/…`
> nas seções abaixo vivem hoje em **`auli-server/crates/auli-collections/…`**.

Há ainda uma pasta `auli-docs/` no workspace (origem histórica dos scrapers), fora do
escopo dos três componentes principais.

---

## 2. Arquitetura e fluxo de dados ponta a ponta

A integração entre os componentes é por uma pasta única **`data/`** na raiz (fonte única),
não por chamadas diretas nem cópia manual. `data/` tem `registry.toml` (entidades, fonte
única), `prompts/`, e por estado `data/<id>/{raw, ref, packs}`:

- `raw/` — saída do scraper (`auli-collections`): o contrato `<id>-<kind>.json`.
- `ref/` — conteúdo autorado e versionado (pareceres/notas/conteúdos, sem scraper).
- `packs/` — pacotes vetoriais gerados pelo `auli update`.

O **server** lê os packs de `data/<id>/packs/` e as entidades do registry; o **frontend** tem
`entities.ts` e `public/<id>/` **gerados** de `data/` por `scripts/`.

```text
[auli-scraper-<e>]  →  data/<id>/<id>-snapshot.json  →  [auli-collections <e>]  →  data/<id>/raw/<id>-<kind>.json
  scrape do portal            (snapshot v2)                deriva (offline)                    │
                                                                                              │
[auli-server] auli update  → data/<id>/packs/ (vetoriza o contrato)  ←─────────────────────────┘
[auli-server] auli server (somente leitura)                          [auli-frontend] entities.ts + public/<id>/
        │                                                            (gerados de data/ por scripts/)
        ▼
pergunta ──> embedding in-process ──> busca vetorial ──> LLM externo ──> resposta ──> UI
```

Observações confirmadas no código:

- Os **scrapers** (`auli-scraper-<e>`) escrevem o **snapshot v2**; o **`auli-collections <e>`** deriva
  o **contrato tipado** (`auli_contract::Table<P>`) em `data/<id>/raw/`, que o `auli update` lê direto
  (sem parsing de `portal-*.txt`, que viraram só _print_ — ver §5).
- O `auli-frontend` lê de `public/<id>/`, **gerado** de `data/` por
  [scripts/build-frontend-public.sh](scripts/build-frontend-public.sh) — não há mais cópia à
  mão entre pastas.
- O registro de entidades é único (`data/registry.toml`); o frontend mantém um espelho
  **gerado** (`entities.ts`), não mais divergente (ver §6).

---

## 3. Backend — o workspace `auli-server`

O backend é um **workspace Cargo único** (`auli-server/`), com **três crates em camadas** e
**um binário** `auli` que troca de modo por subcomando (`auli server` / `auli update`). A camada
de autenticação (JWT) e o **PostgreSQL** foram **removidos** (eram usados só para auth): o server
hoje é **público, sem banco**, e expõe apenas rotas de leitura.

### 3.1 Estrutura (camadas estritas, acoplamento só para baixo)

```
auli-server/                       # workspace único, Cargo.lock compartilhado
└── crates/
    ├── vector-store/      # BAIXO — store plano por cosseno, agnóstico (sabe só id+vetor+payload P)
    ├── auli-core/         # MEIO  — domínio auli: embed (BGE-M3), corpus, manifest
    ├── auli-cli/          # TOPO  — o binário `auli`: server (RAG) + update (ingestão)
    ├── auli-contract/     # forma do dado (serde-only) compartilhada scraper↔engine
    ├── auli-collections/  # DERIVA os artefatos do snapshot (offline) — ver §5
    ├── auli-scraper-kit/  # kit compartilhado dos scrapers (cache, agente HTTP, snapshot, aggregate)
    ├── auli-scraper-rs/   # scraper SEFAZ-RS (FAQs + serviços; headless Chrome)
    ├── auli-scraper-sc/   # scraper SEF-SC (serviços; API JSON Next.js)
    ├── auli-scraper-sp/   # scraper SEFAZ-SP (serviços; REST SharePoint)
    └── auli-scraper-pr/   # scraper SEFA-PR (serviços; HTML Drupal server-side)
```

> **Fase 2 (scrapers em crates próprios).** A coleta saiu do `auli-collections` para **um binário por
> entidade** (`auli-scraper-<e>`) sobre o **`auli-scraper-kit`**; todos gravam a mesma fronteira, o
> **snapshot v2** (`data/<id>/<id>-snapshot.json`, tipos em `auli_contract::snapshot`). O
> `auli-collections <e>` virou **só derivação** (offline): lê o snapshot e produz o contrato +
> prints + index + per-público. As citações a `auli-collections/src/{faqs,servicos}/…` na §5 vivem
> hoje em `auli-scraper-rs/src/…` (ver §5 reescrita).

`vector-store` ← `auli-core` ← `auli-cli`. O `Cargo.lock` único garante que os modos `update`
(embeda documentos) e `server` (embeda a pergunta) usem o **mesmo** `fastembed`/modelo — o espaço
vetorial é compartilhado por construção, não por convenção.

| Crate          | Conteúdo                                                                                                                                                                                                                                                                                                                                                  |
| -------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `vector-store` | `Record<P>`/`CollectionData<P>` (payload genérico; chave JSON em disco continua `document`), `cosine_distance` (fallback `2.0`), IO de arquivo, e a **separação leitura/escrita por tipo**: `ReadStore` (`query_scored`/`list`, imutável) vs `Writer` (`reset`/`upsert`/persistência). Enforcement de dimensão no 1º insert (`Error::DimensionMismatch`). |
| `auli-core`    | `embed` (`Embedder` BGE-M3, `EMBED_DIM=1024`), `corpus` (`EmbedStrategy`, tabela `Collection`, `parse_blocks*`, `prepare_documents`, `extract_question`, `clean_servico`, `extract_servico_description`), `manifest` (identidade do embedding + schema/validação).                                                                                        |
| `auli-cli`     | `server` (axum, RAG, config, packs) + `update` (vetorizador). Despacho por `clap`.                                                                                                                                                                                                                                                                        |

### 3.2 Os dois modos (subcomandos)

```
auli update  --entity <id> --source <dir_com_contrato_json> --out <dir> [--version <v>]
auli server  [--port <p>] [--bind <addr>] [--packs-dir <dir>]   # --bind default 0.0.0.0; --packs-dir default = $AULI_DATA_DIR (./data)
```

- **`auli update`** é o **único escritor**: lê o contrato tipado (`auli_contract::Table<P>` em
  `data/<id>/raw/<id>-<kind>.json`), embeda o `text_to_embed` de cada registro (via `auli-core`) e
  grava `<id>-<kind>.json` + `<id>.manifest.json` em `--out`. Não usa o `Config` do server (não
  precisa de LLM) — só lê `EMBED_CACHE_DIR`/`EMBED_THREADS` do ambiente.
- **`auli server`** é **estritamente leitor**: no boot carrega (eager) todas as coleções via
  `ReadStore`, **valida o manifest** contra a identidade local (modelo+dim+`strategy_version`) e
  **recusa subir** em divergência. Em consulta, embeda **só a pergunta**; nunca escreve e **não
  linka o `Writer`** — incapaz de gravar por construção. Sem `--packs-dir`, herda `AULI_DATA_DIR`.

### 3.3 Rotas e CORS

O server expõe apenas rotas **públicas** (não há mais auth/JWT, rotas protegidas, nem ingestão por HTTP):

| Método | Caminho           | Observação                                                                             |
| ------ | ----------------- | -------------------------------------------------------------------------------------- |
| GET    | `/v1/health`      | health check                                                                           |
| POST   | `/v1/question`    | caminho RAG ativo                                                                      |
| GET    | `/v1/{kind}/list` | listagem de uma coleção (leitura); `{kind}` ∈ `servicos \| faqs \| pareceres \| notas` |

A ingestão deixou de ser rota HTTP (antes `load_from_file`/`load_from_web`) — virou o `auli update`.
CORS: origens **hardcoded** (auli.com.br, www, e portas locais 3000/5173/8080), métodos
GET/POST/OPTIONS.

### 3.4 Caminho RAG ativo (`exec_all_question`)

Acionado por `POST /v1/question`. Assinatura: `exec_all_question(collections, embedder, question, entity)`:

1. Resolve a entidade (registry); entidade desconhecida → a própria mensagem de erro é retornada
   como resposta, com HTTP 200 (sem panic).
2. Gera o embedding da pergunta **uma vez**, in-process (`embed_dense`, executado fora da thread
   async via `run_blocking`/`spawn_blocking`); a pergunta já é uma "chave" curta.
3. Consulta **duas** coleções de forma **concorrente** (`tokio::try_join!`): `<id>-servicos`
   (`n_results = 10`) e `<id>-faqs` (`n_results = 20`). Cada `query_scored` roda em thread bloqueante
   e retorna `(texto, distância cosseno)` ordenado do mais próximo ao mais distante.
4. **Estreitamento por proximidade** (`select_by_proximity`): mantém os `floor` melhores e, além
   disso, os documentos dentro de `band` (distância acima do melhor) — por kind. **Os defaults são
   `floor=0`, `band=∞`**, ou seja, hoje há _paridade_ com o "take fixo" antigo (nenhum descarte); os
   bandos só passam a filtrar após calibração contra perguntas reais (scores logados em `debug`).
5. Concatena os documentos como contexto RAG; o system prompt = prompt da entidade (registry) +
   contexto + delimitador `'''`.
6. Chama o LLM externo.
7. Retorna `{ question, answer }` e **anexa o diálogo a `$AULI_LOG_DIR/<timestamp>.txt`** (default
   `./logs` do CWD; o `start_server.sh` aponta para `<raiz>/logs`).

**Distinção crucial (ativo vs modelado):** apenas `servicos` e `faqs` alimentam as respostas.
`pareceres` e `notas` possuem listagem, mas **não são consultados** por `exec_all_question` (as
`Collection`s consultadas são só `SERVICES` — kind `servicos` — e `FAQS`).

### 3.5 Clientes e adaptadores (embeddings/busca/LLM in-process)

Toda a parte de embeddings/busca vetorial é **in-process** (sem Ollama nem ChromaDB):

- **Embedder** (`auli-core::embed`): `fastembed` com **BGE-M3 ONNX INT8** (`Bgem3Model::BGEM3Q`),
  dimensão **1024** (`EMBED_DIM`), apenas a saída _dense_. O modelo fica atrás de um `Mutex` porque
  `embed` é `&mut self`; `max_length` é **512** (dimensionado à "chave" curta). `embed_dense` é
  **bloqueante/CPU-bound** — os chamadores usam `run_blocking`. Construído uma vez no startup (lento;
  baixa o modelo do Hugging Face para `EMBED_CACHE_DIR` no 1º run).
- **Vector store** (`vector-store`): índice plano **puro-Rust**, in-process. Cada coleção
  `<id>-<kind>` é uma lista de `(id, embedding, document)` persistida em `<name>.json`. `query_scored`
  faz varredura **brute-force** por **distância cosseno**, ordena melhor-primeiro e trunca em
  `max_results`. `cosine_distance` ∈ `[0,2]`; larguras diferentes ou vetor zero ⇒ distância máxima
  `2.0` (`1 - cos` com `cos = -1`), para afundarem abaixo de documentos legitimamente
  anti-correlacionados. No server o store é só-leitura (`ReadStore`, carga eager, sem lock no caminho
  de consulta).
- **LLM** (`auli-cli`): chat completions compatível com Groq. `temperature 0.5`, `top_p 0.5`,
  `max_completion_tokens 1024`, `stream:false`. Até **3 tentativas** em erros de conexão/timeout
  (sleep 500ms). Erro de API vira mensagem legível (não `Err`).

**Implicação operacional:** trocar o modelo de embedding ⇒ **re-ingestão total** de todas as
coleções. Os vetores não carregam tag de dimensão e larguras incompatíveis pontuam como distância
máxima; o `strategy_version` do manifest transforma "esqueci de re-gerar" em erro de boot (§3.6).

### 3.6 Manifest e identidade do embedding

`auli-core::manifest` grava, por entidade, `{ entity, version, built_at, embed_model_id, embed_dim,
strategy_version, collections: [{ kind, count, dim, file, bytes, hash }] }`. `hash` é FNV-1a 64 do
arquivo da coleção (integridade — detecta pacote meio-copiado). `STRATEGY_VERSION` é bumpado sempre
que `prepare_documents`/`parse_*` mudarem (muda _o que_ é embedado), transformando "esqueci de
re-gerar os pacotes" em **erro de boot**, não em retrieval ruim. O fallback `2.0` do `cosine_distance`
vira segunda linha de defesa.

### 3.7 Multi-tenancy (entidades)

As entidades vêm de `data/registry.toml` (fonte única), lido por `auli-cli` e `auli-collections`; o
frontend gera seu `entities.ts` a partir dele. Cada entidade tem `id`, `name`, prompt e as coleções
disponíveis. `AULI_DATA_DIR` (default `./data`) é a raiz de `registry.toml`, `prompts/` e
`<id>/packs/` — e também o default de `--packs-dir`, então registry e packs compartilham uma raiz por
construção. Entidades hoje (todas ativas no server): `rs` (SEFAZ-RS), `sc` (SEF-SC), `sp` (SEFAZ-SP)
e `pr` (SEFA-PR).

### 3.8 Distribuição (decorrência do desenho)

Servir o `auli server` exige **apenas o binário + a pasta de pacotes** (`<id>-<kind>.json` +
`<id>.manifest.json`). Sem banco para subir, sem ChromaDB/Ollama, sem serviço de embedding, sem rede
para ingestão — o server é autossuficiente de ponta a ponta. Read-only + eager-load + binário único ⇒
N cópias coexistem sem coordenação.

### 3.9 Verificação (estado de fato, neste repo)

- **Build/test:** `cargo build --workspace` e `cargo test --workspace` passam **sem warnings**
  (inclui `clippy -D warnings`); 1 teste e2e fica _gated_ (`packs_smoke`, exige packs + modelo).
- **Pacotes reais gerados** via `auli update` (kind `servicos`, `strategy_version: 2`): **rs** serviços
  586 + FAQs 1937, **sc** 208, **sp** 537, **pr** 141, **mg** 148. O manifest confere — `bytes` e `hash` FNV-1a
  batem com os arquivos (o `packs::load_all` re-hasheia e alerta em divergência); todos os vetores em
  dim 1024; chave `document` preservada.
- **Boot e2e (modelo real):** o server carrega as 5 entidades, valida cada manifest contra a
  identidade local e responde `POST /v1/question` por estado citando os links do próprio catálogo
  (verificado ao vivo: rs/sc/sp/pr/mg).

### 3.10 Decisões de desenho

- **Ingestão fora do server.** As rotas `load_from_file`/`load_from_web` saíram do server (o frontend
  nunca as usou — só `POST /v1/question`); a vetorização é o `auli update`.
- **Store em memória imutável.** Carga _eager_ + `ReadStore` imutável (sem lock no caminho de consulta).
- **Domínio como fonte única no backend.** `corpus`/`vector-store` são fonte única para `server` e
  `update`. O `auli-frontend` segue com seu próprio espelho **gerado** do registro (ver §6).
- **Auth e banco removidos.** O server não tem mais auth/JWT nem PostgreSQL — é público.

---

## 4. auli-frontend (React + TypeScript)

### 4.1 Manifesto e stack

Fonte: [auli-frontend/package.json](auli-frontend/package.json) (nome do pacote `auli-ui`,
versão 0.1.43). Confirmado:

- **React 19** + **Vite 8** (com Rolldown — `rolldownOptions` em
  [vite.config.js](auli-frontend/vite.config.js)).
- **Chakra UI v3** (`@chakra-ui/react`, `@emotion/react`, `@emotion/styled`).
- **SWR** + **Axios** (busca de dados), **Framer Motion** (animações), **react-markdown**,
  **react-icons**, **next-themes** (modo claro/escuro).
- **TypeScript** em modo `strict` ([tsconfig.json](auli-frontend/tsconfig.json), `allowJs:true`).
- Testes com **Vitest** + Testing Library; ambiente Node por padrão, `jsdom` por arquivo.

Scripts: `dev`, `build` (`tsc --noEmit && vite build`), `lint`, `typecheck`, `test`,
`test:watch`, `doctor` ([package.json](auli-frontend/package.json)).

A versão exibida e um `__BUILD_ID__` para cache-busting são injetados em build via `define`
([vite.config.js](auli-frontend/vite.config.js)). Chunks manuais separam react/chakra/icons/utils.

### 4.2 Estrutura da aplicação

- **Single-page sem roteador.** [App.tsx](auli-frontend/src/App.tsx) monta um `EntityProvider`
  e mostra `StateSelection` até uma entidade ser escolhida; depois mostra `Home`. Não há
  React Router; a navegação interna é por **abas**.
- **Seleção de entidade.** [shared/EntityContext.tsx](auli-frontend/src/shared/EntityContext.tsx)
  guarda a entidade selecionada, persistida em `localStorage` (chave `auli.entity`).
- **Registro de entidades (frontend).** [shared/entities.ts](auli-frontend/src/shared/entities.ts) é
  **gerado** de `data/registry.toml` por [scripts/gen-frontend-entities.mjs](scripts/gen-frontend-entities.mjs)
  (guardado por `scripts/check-registry-sync.sh`), com **quatro** entidades:
  - `rs` = SEFAZ-RS, coleções `["servicos","faqs","pareceres","notas","conteudos"]`.
  - `sc` = SEF-SC, `sp` = SEFAZ-SP, `pr` = SEFA-PR — coleções `["servicos"]`.
    `hasCollection(entity, collection)` dirige os estados "em breve".
- **Abas** ([pages/home/Home.tsx](auli-frontend/src/pages/home/Home.tsx)): Chat, Serviços,
  FAQs, Pareceres, Notas, Conteúdos, Sobre. Implementam o padrão WAI-ARIA de tabs (roving
  tabindex, navegação por setas). Cada aba é montada na primeira ativação e mantida montada
  (preserva rolagem/busca/histórico).

### 4.3 Chat (caminho ativo)

- [pages/chat/Chat.tsx](auli-frontend/src/pages/chat/Chat.tsx): a URL da API é
  `import.meta.env.VITE_API_URL ?? "https://api.auli.com.br/v1/question"`. Passa
  `entityId: entity.id` ao backend.
- [pages/chat/utils/callServerAPI.ts](auli-frontend/src/pages/chat/utils/callServerAPI.ts):
  `POST` via axios do corpo `{ question, entity? }`, com **timeout de 25s** via
  `AbortController`; mensagens de erro/timeout em pt-BR. Lê `res.data.answer`.
- [pages/chat/utils/prompt.ts](auli-frontend/src/pages/chat/utils/prompt.ts): validação do
  prompt (mínimo de 10 caracteres) — fonte única usada pelo guard de envio, botão e contador.
- [pages/chat/utils/useMessages.ts](auli-frontend/src/pages/chat/utils/useMessages.ts):
  estado das mensagens, iniciando com a saudação "Olá! Como posso ajudar?".

Há subpasta [pages/chat/ui/](auli-frontend/src/pages/chat/ui/) com 4 snippets utilitários do
Chakra em `.jsx` (color-mode, provider, toaster, tooltip) — confirmando a observação do
README de que apenas esses arquivos permanecem em JSX.

### 4.4 Páginas de referência (conteúdo estático por entidade)

Todas usam SWR + `entityPath(entityId, file)` →
`/<entityId>/<entityId>-<file>?v=<buildId>` ([shared/fetchers.ts](auli-frontend/src/shared/fetchers.ts)):
os arquivos em `public/<id>/` são **prefixados com `<id>-`** (globalmente únicos), e o `entityPath`
adiciona esse prefixo ao buscar (o consumidor passa o nome "cru"). Nomes crus por aba:

| Aba       | Arquivo lido (cru; servido como `<id>-…`)                                 | Fonte                                                                                                                                       |
| --------- | ------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| Serviços  | `servicos-index.json` (manifesto de abas) + `<filename>.json` por público | [pages/servicoslist/ServicosList.tsx](auli-frontend/src/pages/servicoslist/ServicosList.tsx)                                                |
| FAQs      | `faqs-tree.json` (árvore recursiva)                                       | [pages/faqslist/FaqsList.tsx](auli-frontend/src/pages/faqslist/FaqsList.tsx), [parseFaqs.ts](auli-frontend/src/pages/faqslist/parseFaqs.ts) |
| Pareceres | `portal-pareceres.txt` (texto)                                            | [pages/parecereslist/PareceresList.tsx](auli-frontend/src/pages/parecereslist/PareceresList.tsx)                                            |
| Notas     | `portal-notas.txt` (texto)                                                | [pages/notaslist/NotasList.tsx](auli-frontend/src/pages/notaslist/NotasList.tsx)                                                            |
| Conteúdos | `conteudo_site_tree.json`                                                 | [pages/conteudoslist/ConteudosList.tsx](auli-frontend/src/pages/conteudoslist/ConteudosList.tsx)                                            |

- A aba **Serviços** lê `servicos-index.json` para montar as abas de público; se ausente,
  usa o fallback hardcoded `getDefaultTipoServicos()` (5 tipos RS) em
  [pages/servicoslist/utils.ts](auli-frontend/src/pages/servicoslist/utils.ts). Agrupa por
  `classe` em acordeões e busca por `titulo`.
- Pareceres/Notas usam `editLinks` ([shared/linkify.tsx](auli-frontend/src/shared/linkify.tsx))
  para transformar links no texto.
- Quando a entidade não tem a coleção (`hasCollection` falso), renderiza
  [shared/CollectionEmpty.tsx](auli-frontend/src/shared/CollectionEmpty.tsx) ("em breve").

**Dados estáticos presentes** (confirmado na árvore de `public/`, gerada por
[scripts/build-frontend-public.sh](scripts/build-frontend-public.sh) — que aceita um `<id>` opcional
para regenerar **só uma** entidade — a partir de `data/<id>/{raw,ref}/`, prefixando cada arquivo com
`<id>-`): [public/rs/](auli-frontend/public/rs/) tem `rs-faqs-tree.json`, `rs-conteudo_site_tree.json`,
`rs-servicos-*.json`, `rs-servicos-index.json`, `rs-portal-notas.txt`, `rs-portal-pareceres.txt`.
`public/{sc,sp,pr}/` têm **apenas** arquivos de serviços (`<id>-servicos-*.json`, `<id>-servicos-index.json`),
coerente com `collections = ["servicos"]`. O gerador **exclui** do `public/`: os `portal-{faqs,servicos}.txt`
(grandes) e os **contratos do engine** `<id>-{faqs,servicos}.json` — nenhum é buscado pela UI (só
alimentam os packs, lidos de `data/<id>/raw/`).

### 4.5 Seleção de estado e mapa

[pages/stateselection/StateSelection.tsx](auli-frontend/src/pages/stateselection/StateSelection.tsx)
apresenta um mapa interativo do Brasil ([BrazilMap.tsx](auli-frontend/src/pages/stateselection/BrazilMap.tsx),
[brazilPaths.ts](auli-frontend/src/pages/stateselection/brazilPaths.ts)) e cards das
entidades; ambos chamam `selectEntity`.

### 4.6 Tema, erros e testes

- Tokens semânticos de cor centralizados em [src/theme/system.js](auli-frontend/src/theme/system.js)
  (claro/escuro); modo de cor via `next-themes`.
- `ErrorBoundary` ([shared/ErrorBoundary.tsx](auli-frontend/src/shared/ErrorBoundary.tsx)).
- Testes presentes: `SearchInput`, `AsyncContent`, `linkify`, `Input`, `Messages`,
  `callServerAPI`, `prompt`, `parseFaqs`, `parseConteudos` (arquivos `*.test.ts(x)`).

### 4.7 Distinção ativo vs modelado (frontend)

- O frontend é **single-tenant em tempo de execução por deploy**: serve os arquivos de uma
  pasta `public/` específica; a seleção `rs`/`sc`/`sp`/`pr`/`mg` muda apenas qual `public/<id>/` é
  consultado. Não há, no código, busca de uma lista de entidades vinda do backend — a lista vem de
  [shared/entities.ts](auli-frontend/src/shared/entities.ts), **gerado** de `data/registry.toml`
  (não editado à mão).
- O frontend **não** consome rotas de gestão de dados do servidor; o único endpoint de backend
  efetivamente chamado é `POST /v1/question` (via `VITE_API_URL`). Não há, no código do
  frontend, uso de login/JWT. **NÃO CONFIRMADO NO CÓDIGO:** qualquer fluxo de autenticação no cliente.

---

## 5. Coleta: scrapers por entidade + `auli-collections` (derivação)

A coleta é **um binário por entidade** (`auli-scraper-<e>`) sobre o **`auli-scraper-kit`**; o
`auli-collections` virou **só derivação** (offline). A fronteira entre os dois é o **snapshot v2**.

```text
auli-scraper-<e> (rede)  →  data/<id>/<id>-snapshot.json (v2)  →  auli-collections <e> (offline)  →  data/<id>/raw/
```

### 5.1 O kit compartilhado (`auli-scraper-kit`)

[crates/auli-scraper-kit](auli-server/crates/auli-scraper-kit): peças comuns aos scrapers —
`snapshot::{load, write_faqs, write_servicos}` (grava/lê o snapshot v2), `cache::{read, write}`
(cache de página por URL; arquivo vazio conta como _miss_), `build_agent(user_agent, timeout)`
(agente `ureq`), e `aggregate_servicos` + `descricao_body` (dobra registros per-público em `ServicoRaw`
**deduplicando por `link`**). Sem `fastembed`/`ort` — os scrapers compilam leves.

### 5.2 O snapshot v2 (`auli_contract::snapshot`)

Cada scraper grava `data/<id>/<id>-snapshot.json` (`SNAPSHOT_SCHEMA_VERSION = 2`): metadados do
scraper + as coleções. Serviços são `ServicoRaw { titulo, descricao, link, orgao, ocorrencias:
Vec<Ocorrencia> }` com `Ocorrencia { publico, classe }` — um serviço em N públicos/classes é **N
ocorrências nativas** (resolveu o limite multi-classe do modelo antigo). FAQs são `Vec<FaqRaw>`
(pergunta/resposta/origin/url), lista **achatada**.

### 5.3 Os scrapers (um por entidade)

| Crate | Entidade | Plataforma / técnica | CLI |
| ----- | -------- | -------------------- | --- |
| [auli-scraper-rs](auli-server/crates/auli-scraper-rs) | SEFAZ-RS | FAQs (portal CMS via **headless Chrome** + `ureq`) + serviços (5 públicos via Chrome, detalhe via `ureq`) | `[--usecache] faqs\|servicos\|all` |
| [auli-scraper-sc](auli-server/crates/auli-scraper-sc) | SEF-SC | serviços via **API JSON Next.js** (sem browser) | `[--usecache] servicos` |
| [auli-scraper-sp](auli-server/crates/auli-scraper-sp) | SEFAZ-SP | serviços via **REST SharePoint** (JSON anônimo: listas `Serviços` + `Homes 360`) | `[--usecache] servicos` |
| [auli-scraper-pr](auli-server/crates/auli-scraper-pr) | SEFA-PR | serviços via **HTML Drupal** server-side (mega-menu "Serviços para você!") | `[--usecache] servicos` |

- **RS** é o único com FAQs e o único que puxa **headless Chrome**. A **árvore** de FAQ
  (`FaqNode { title, url, page_type, origin, children, faq_items }`) é serializada em
  `faqs-tree.json` para a aba de FAQs do frontend ([faqs/mod.rs](auli-server/crates/auli-scraper-rs/src/faqs/mod.rs));
  o snapshot leva a lista achatada.
- **SC/RS/PR** dobram os registros per-público via `aggregate_servicos` (**dedup por `link`**). **SP
  é a exceção**: no portal paulista a URL **não** é única (vários serviços distintos compartilham um
  login), então o scraper monta os `ServicoRaw` **direto** (um por linha do catálogo), sem o
  aggregate, para não colapsar serviços distintos ([sp/scrape.rs](auli-server/crates/auli-scraper-sp/src/scrape.rs)).
- **Cache + `--usecache`:** cada scraper cacheia páginas por URL (`data/<id>/raw/cache/…`, ignorado
  pelo git); `--usecache` reprocessa offline e torna um _miss_ um erro.

### 5.4 A derivação (`auli-collections <e>`)

[crates/auli-collections](auli-server/crates/auli-collections) (`main`, `process`, `derive_faqs`,
`servicos/{mod,types}`, `domain/entities`): **não raspa** — lê o snapshot e produz, offline, em
`data/<id>/raw/`:

- o **contrato** `<id>-faqs.json` / `<id>-servicos.json` (`auli_contract::Table<P>`, com
  `text_to_embed` materializado) — o que o `auli update` embeda;
- os **prints** `portal-<kind>.txt` (auditoria, nunca relidos);
- o **`servicos-index.json`** (manifesto de abas) + os JSONs **per-público** (`<slug>.json`).

`text_to_embed`: faqs → breadcrumb `origin` + pergunta (a mesma key do antigo `QuestionKey`);
serviços → `tipo | classe` + título + início do corpo (fórmula ainda **provisória**, ver
[auli_pendencias.md](auli_pendencias.md)). A tentativa de raspar por aqui é rejeitada com erro
explícito ("a coleta agora é feita pelos binários `auli-scraper-*`").

`pareceres`/`notas` seguem **autorados** em `data/<id>/ref/` (sem scraper, sem `Table<P>`) — ausentes
nos packs até serem modelados.

### 5.5 Cobertura por entidade

| entidade | serviços | FAQs |
| -------- | -------- | ---- |
| `rs` | ✅ (Chrome) | ✅ (Chrome) |
| `sc` | ✅ (JSON Next.js) | — |
| `sp` | ✅ (REST SharePoint) | — |
| `pr` | ✅ (HTML Drupal) | — |

`pareceres`/`notas`/`conteudos` não têm scraper (autorados). FAQs hoje só no RS.

---

## 6. Divergências entre componentes — resolvidas pela unificação sob `data/`

> **RESOLVIDO — integração `data/` (Fases 1–4 da unificação).** As divergências históricas entre
> os domínios duplicados foram **eliminadas** pela fonte única `data/`.

1. **Triplicação do `domain` resolvida.** `data/registry.toml` é a fonte única de entidades, lida
   por `auli-cli` e `auli-collections`; o frontend gera `entities.ts` dela. O kind vetorial foi
   **unificado para `servicos` ponta a ponta** pela auditoria (PR #4) — não há mais o descasamento
   `services`↔`servicos` (o antigo `services` sobrou só como guarda de regressão em `from_kind`) nem
   cópias divergentes de `domain`/`errors`/`entities`.
2. **Dados de serviços consistentes.** Packs e frontend vêm da **mesma** raspagem (contrato
   `auli-contract`); o engine não declara mais `delimiter`/`EmbedStrategy` próprios para serviços.
3. **`rs`, `sc`, `sp`, `pr` e `mg` são entidades reais** do server (serviços 586/208/537/141/148;
   FAQs 1937 no RS), não só do frontend.
4. **`pareceres`/`notas`/`conteudos`** (autorados, sem scraper) ficam versionados em `data/<id>/ref/`,
   exibidos no frontend e (pareceres/notas) ingeríveis nos packs quando modelados como `Table<P>`;
   ainda **não** são consultados no RAG ativo.

Resíduo: dentro do backend o domínio é fonte única (`corpus`/`vector-store`); o `auli-frontend`
mantém um espelho **gerado** (não mais divergente) do registro. Pendências em
[auli_pendencias.md](auli_pendencias.md).

---

## 7. Resumo: implementado e ativo vs modelado/inativo

**Ativo e funcionando (confirmado no código):**

- `auli server` (workspace `auli-server`): `GET /v1/health`, `POST /v1/question` (RAG completo
  **in-process**: fastembed/BGE-M3 → vector store próprio → LLM externo, com log em `$AULI_LOG_DIR`,
  na raiz do repo) e `GET /v1/{kind}/list` (leitura). Escuta configurável (`--bind`, default `0.0.0.0`).
  Público, **sem auth nem banco**; CORS; configuração por `.env` (`config()`); logging via `tracing`.
  Vetorização separada pelo `auli update`.
- **Cinco estados ativos** (rs, sc, sp, pr, mg). RAG consulta efetivamente apenas `servicos` (10) +
  `faqs` (20); estreitamento por proximidade presente mas em modo paridade (`band=∞`) até calibração.
- `auli-frontend`: SPA com seleção de entidade (rs/sc/sp/pr/mg), chat contra `POST /v1/question` com
  timeout de 25s, abas de referência lendo `public/<id>/` (arquivos prefixados `<id>-`), tema
  claro/escuro, testes Vitest.
- **Scrapers por entidade** (`auli-scraper-{rs,sc,sp,pr,mg}` sobre `auli-scraper-kit`): FAQs (rs) e
  serviços (rs Chrome, sc JSON, sp SharePoint, pr Drupal, mg ServiceNow JSON), cache + `--usecache`,
  gravando o snapshot v2; `auli-collections <e>` deriva o contrato + `portal-*.txt` +
  `servicos-index.json` + per-público.

**Modelado/declarado mas inativo ou incompleto:**

- Server: `pareceres`/`notas` listáveis mas **não consultados** no RAG ativo.
- Coleta: sem scraper de `pareceres`/`notas` (autorados); FAQs só no RS. Os binários de scraper
  `sc`/`sp`/`pr` usam `anyhow` (idiomático p/ bin); a derivação em `auli-collections` já usa o
  `crate::errors` tipado.
- Frontend: sem fluxo de autenticação/JWT no cliente; multi-tenant apenas por troca de
  `public/<id>/` (lista de entidades **gerada** do registry).

---

## 8. Itens marcados como NÃO CONFIRMADO NO CÓDIGO

- Fluxo de autenticação no frontend: inexistente no código (o backend atual também não tem auth).
- Reranking: não há reranking no caminho RAG ativo; o único estreitamento de resultados é o
  `select_by_proximity` (por distância cosseno), hoje em modo paridade (`band=∞`).
