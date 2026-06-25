# Auli — Descrição Técnica do Código

Documento técnico auditável do Projeto Auli, baseado na leitura direta do código-fonte
dos três componentes (`auli-engine`, `auli-frontend`, `auli-collections`). Cada afirmação
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

| Componente | Papel | Linguagem/stack |
| --- | --- | --- |
| `auli-engine` (workspace) | Backend REST + pipeline RAG (binário `auli`: `server` + `update`) | Rust (Axum, Tokio) |
| `auli-frontend` | Interface web (chat + navegação) | React 19 + TypeScript + Vite |
| `auli-collections` | Scrapers que produzem o conteúdo ingerido | Rust (síncrono, `ureq`) |

> **Atualização — `auli-contract` (2026-06-23).** O workspace ganhou o crate magro
> **`auli-contract`** (serde-only): a **forma do dado** (`Table<P>`, `Faq`, `Servico`, trait
> `Embeddable`) compartilhada entre o scraper e o engine. O **`auli-collections` foi movido para
> dentro do workspace** (`auli-engine/crates/auli-collections`, 5º membro). Fluxo: o scraper compila
> `Table<P>` preenchendo `text_to_embed` → `data/<id>/raw/<id>-<kind>.json`; o `auli update` lê o
> contrato, embeda `text_to_embed` e armazena `stored_repr` (sem mais parsing de `portal-*.txt`,
> que viraram só *print* de auditoria). `STRATEGY_VERSION` foi para **2**. Caminhos `auli-collections/…`
> nas seções abaixo vivem hoje em **`auli-engine/crates/auli-collections/…`**.

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

```
[auli-collections]            [auli-engine]                       [auli-frontend]
  scrape do portal              auli update → data/<id>/packs/       entities.ts + public/<id>/
        │                          (vetoriza o contrato)            (gerados de data/ por scripts/)
        ▼                              │                                    │
  data/<id>/raw/<id>-<kind>.json ──────┘                                    │
                                   auli server (somente leitura)            │
                                       ▲                                    ▼
                  pergunta ──> embedding in-process ──> busca vetorial ──> LLM externo ──> resposta ──> UI
```

Observações confirmadas no código:

- Os scrapers escrevem o **contrato tipado** (`auli_contract::Table<P>`) em `data/<id>/raw/`;
  o `auli update` lê isso direto, sem parsing de `portal-*.txt` (ver §5).
- O `auli-frontend` lê de `public/<id>/`, **gerado** de `data/` por
  [scripts/build-frontend-public.sh](scripts/build-frontend-public.sh) — não há mais cópia à
  mão entre pastas.
- O registro de entidades é único (`data/registry.toml`); o frontend mantém um espelho
  **gerado** (`entities.ts`), não mais divergente (ver §6).

---

## 3. Backend — o workspace `auli-engine`

O backend é um **workspace Cargo único** (`auli-engine/`), com **três crates em camadas** e
**um binário** `auli` que troca de modo por subcomando (`auli server` / `auli update`). A camada
de autenticação (JWT) e o **PostgreSQL** foram **removidos** (eram usados só para auth): o server
hoje é **público, sem banco**, e expõe apenas rotas de leitura.

### 3.1 Estrutura (camadas estritas, acoplamento só para baixo)

```
auli-engine/                       # workspace único, Cargo.lock compartilhado
└── crates/
    ├── vector-store/      # BAIXO — store plano por cosseno, agnóstico (sabe só id+vetor+payload P)
    ├── auli-core/         # MEIO  — domínio auli: embed (BGE-M3), corpus, manifest
    ├── auli-cli/          # TOPO  — o binário `auli`: server (RAG) + update (ingestão)
    ├── auli-contract/     # forma do dado (serde-only) compartilhada scraper↔engine
    └── auli-collections/  # scrapers (ver §5)
```

`vector-store` ← `auli-core` ← `auli-cli`. O `Cargo.lock` único garante que os modos `update`
(embeda documentos) e `server` (embeda a pergunta) usem o **mesmo** `fastembed`/modelo — o espaço
vetorial é compartilhado por construção, não por convenção.

| Crate | Conteúdo |
| --- | --- |
| `vector-store` | `Record<P>`/`CollectionData<P>` (payload genérico; chave JSON em disco continua `document`), `cosine_distance` (fallback `2.0`), IO de arquivo, e a **separação leitura/escrita por tipo**: `ReadStore` (`query_scored`/`list`, imutável) vs `Writer` (`reset`/`upsert`/persistência). Enforcement de dimensão no 1º insert (`Error::DimensionMismatch`). |
| `auli-core` | `embed` (`Embedder` BGE-M3, `EMBED_DIM=1024`), `corpus` (`EmbedStrategy`, tabela `Collection`, `parse_blocks*`, `prepare_documents`, `extract_question`, `clean_servico`, `extract_servico_description`), `manifest` (identidade do embedding + schema/validação). |
| `auli-cli` | `server` (axum, RAG, config, packs) + `update` (vetorizador). Despacho por `clap`. |

### 3.2 Os dois modos (subcomandos)

```
auli update  --entity <id> --source <dir_com_contrato_json> --out <dir> [--version <v>]
auli server  [--packs-dir <dir>] [--port <p>]   # --packs-dir default = $AULI_DATA_DIR (./data)
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

| Método | Caminho | Observação |
| --- | --- | --- |
| GET | `/v1/health` | health check |
| POST | `/v1/question` | caminho RAG ativo |
| GET | `/v1/{kind}/list` | listagem de uma coleção (leitura); `{kind}` ∈ `services \| faqs \| pareceres \| notas` |

A ingestão deixou de ser rota HTTP (antes `load_from_file`/`load_from_web`) — virou o `auli update`.
CORS: origens **hardcoded** (auli.com.br, www, e portas locais 3000/5173/8080), métodos
GET/POST/OPTIONS.

### 3.4 Caminho RAG ativo (`exec_all_question`)

Acionado por `POST /v1/question`. Assinatura: `exec_all_question(collections, embedder, question, entity)`:

1. Resolve a entidade (registry); entidade desconhecida → a própria mensagem de erro é retornada
   como resposta, com HTTP 200 (sem panic).
2. Gera o embedding da pergunta **uma vez**, in-process (`embed_dense`, executado fora da thread
   async via `run_blocking`/`spawn_blocking`); a pergunta já é uma "chave" curta.
3. Consulta **duas** coleções de forma **concorrente** (`tokio::try_join!`): `<id>-services`
   (`n_results = 10`) e `<id>-faqs` (`n_results = 20`). Cada `query_scored` roda em thread bloqueante
   e retorna `(texto, distância cosseno)` ordenado do mais próximo ao mais distante.
4. **Estreitamento por proximidade** (`select_by_proximity`): mantém os `floor` melhores e, além
   disso, os documentos dentro de `band` (distância acima do melhor) — por kind. **Os defaults são
   `floor=0`, `band=∞`**, ou seja, hoje há *paridade* com o "take fixo" antigo (nenhum descarte); os
   bandos só passam a filtrar após calibração contra perguntas reais (scores logados em `debug`).
5. Concatena os documentos como contexto RAG; o system prompt = prompt da entidade (registry) +
   contexto + delimitador `'''`.
6. Chama o LLM externo.
7. Retorna `{ question, answer }` e **anexa o diálogo a `./logs/<timestamp>.txt`**.

**Distinção crucial (ativo vs modelado):** apenas `services` e `faqs` alimentam as respostas.
`pareceres` e `notas` possuem listagem, mas **não são consultados** por `exec_all_question` (importa
apenas `FAQS, SERVICES`).

### 3.5 Clientes e adaptadores (embeddings/busca/LLM in-process)

Toda a parte de embeddings/busca vetorial é **in-process** (sem Ollama nem ChromaDB):

- **Embedder** (`auli-core::embed`): `fastembed` com **BGE-M3 ONNX INT8** (`Bgem3Model::BGEM3Q`),
  dimensão **1024** (`EMBED_DIM`), apenas a saída *dense*. O modelo fica atrás de um `Mutex` porque
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
que `prepare_documents`/`parse_*` mudarem (muda *o que* é embedado), transformando "esqueci de
re-gerar os pacotes" em **erro de boot**, não em retrieval ruim. O fallback `2.0` do `cosine_distance`
vira segunda linha de defesa.

### 3.7 Multi-tenancy (entidades)

As entidades vêm de `data/registry.toml` (fonte única), lido por `auli-cli` e `auli-collections`; o
frontend gera seu `entities.ts` a partir dele. Cada entidade tem `id`, `name`, prompt e as coleções
disponíveis. `AULI_DATA_DIR` (default `./data`) é a raiz de `registry.toml`, `prompts/` e
`<id>/packs/` — e também o default de `--packs-dir`, então registry e packs compartilham uma raiz por
construção. Entidades hoje: `rs` (SEFAZ-RS) e `sc` (SEF-SC).

### 3.8 Distribuição (decorrência do desenho)

Servir o `auli server` exige **apenas o binário + a pasta de pacotes** (`<id>-<kind>.json` +
`<id>.manifest.json`). Sem banco para subir, sem ChromaDB/Ollama, sem serviço de embedding, sem rede
para ingestão — o server é autossuficiente de ponta a ponta. Read-only + eager-load + binário único ⇒
N cópias coexistem sem coordenação.

### 3.9 Verificação (estado de fato, neste repo)

- **Build/test:** `cargo build --workspace` e `cargo test --workspace` passam **sem warnings**;
  **24 testes** (vector-store 10, auli-core 8, auli-cli 5 + 1 de integração) + 1 teste e2e gated.
- **Pacotes reais gerados** via `auli update` para `rs`: services 627, faqs 1734, pareceres 331,
  notas 1. O manifest confere — `bytes` e `hash` FNV-1a batem com os arquivos (hash conferido também
  por implementação independente em Python); todos os vetores em dim 1024; chave `document` preservada.
- **Caminho de atendimento e2e** (teste gated, modelo real): manifest validado → `ReadStore` carrega
  1734 faqs → pergunta embedada → `query_scored` retorna 20 hits ordenados (melhor distância ≈ **0,28**,
  um match real), com relevância semântica correta.

### 3.10 Decisões de desenho

- **Ingestão fora do server.** As rotas `load_from_file`/`load_from_web` saíram do server (o frontend
  nunca as usou — só `POST /v1/question`); a vetorização é o `auli update`.
- **Store em memória imutável.** Carga *eager* + `ReadStore` imutável (sem lock no caminho de consulta).
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
- **Registro de entidades (frontend).** [shared/entities.ts](auli-frontend/src/shared/entities.ts)
  lista **duas** entidades hardcoded:
  - `rs` = SEFAZ-RS, coleções `["servicos","faqs","pareceres","notas","conteudos"]`.
  - `sc` = SEF-SC, coleções `["servicos"]` (somente serviços, por enquanto).
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
`/<entityId>/<file>?v=<buildId>` ([shared/fetchers.ts](auli-frontend/src/shared/fetchers.ts)),
ou seja, leem arquivos servidos de `public/<id>/`:

| Aba | Arquivo lido | Fonte |
| --- | --- | --- |
| Serviços | `servicos-index.json` (manifesto de abas) + `<filename>.json` por público | [pages/servicoslist/ServicosList.tsx](auli-frontend/src/pages/servicoslist/ServicosList.tsx) |
| FAQs | `faqs.json` (árvore recursiva) | [pages/faqslist/FaqsList.tsx](auli-frontend/src/pages/faqslist/FaqsList.tsx), [parseFaqs.ts](auli-frontend/src/pages/faqslist/parseFaqs.ts) |
| Pareceres | `portal-pareceres.txt` (texto) | [pages/parecereslist/PareceresList.tsx](auli-frontend/src/pages/parecereslist/PareceresList.tsx) |
| Notas | `portal-notas.txt` (texto) | [pages/notaslist/NotasList.tsx](auli-frontend/src/pages/notaslist/NotasList.tsx) |
| Conteúdos | `conteudo_site_tree.json` | [pages/conteudoslist/ConteudosList.tsx](auli-frontend/src/pages/conteudoslist/ConteudosList.tsx) |

- A aba **Serviços** lê `servicos-index.json` para montar as abas de público; se ausente,
  usa o fallback hardcoded `getDefaultTipoServicos()` (5 tipos RS) em
  [pages/servicoslist/utils.ts](auli-frontend/src/pages/servicoslist/utils.ts). Agrupa por
  `classe` em acordeões e busca por `titulo`.
- Pareceres/Notas usam `editLinks` ([shared/linkify.tsx](auli-frontend/src/shared/linkify.tsx))
  para transformar links no texto.
- Quando a entidade não tem a coleção (`hasCollection` falso), renderiza
  [shared/CollectionEmpty.tsx](auli-frontend/src/shared/CollectionEmpty.tsx) ("em breve").

**Dados estáticos presentes** (confirmado na árvore de `public/`, gerada por
[scripts/build-frontend-public.sh](scripts/build-frontend-public.sh) a partir de `data/<id>/{raw,ref}/`):
[public/rs/](auli-frontend/public/rs/) tem `faqs.json`, `conteudo_site_tree.json`,
`servicos*.json`, `servicos-index.json`, `portal-notas.txt`, `portal-pareceres.txt`.
[public/sc/](auli-frontend/public/sc/) tem **apenas** arquivos de
serviços (`servicos-*.json`, `servicos-index.json`), coerente com
`sc.collections = ["servicos"]`. Os `portal-{faqs,servicos}.txt` (grandes, não usados pela UI —
só alimentam os packs) são excluídos do `public/` pelo gerador.

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
  pasta `public/` específica; a seleção `rs`/`sc` muda apenas qual `public/<id>/` é
  consultado. Não há, no código, busca de uma lista de entidades vinda do backend — a lista
  é hardcoded em [shared/entities.ts](auli-frontend/src/shared/entities.ts).
- O frontend **não** consome rotas de gestão de dados do servidor; o único endpoint de backend
  efetivamente chamado é `POST /v1/question` (via `VITE_API_URL`). Não há, no código do
  frontend, uso de login/JWT. **NÃO CONFIRMADO NO CÓDIGO:** qualquer fluxo de autenticação no cliente.

---

## 5. auli-collections (scrapers Rust)

Repositório foco da fase atual (consolida os antigos scrapers de FAQs e Serviços em um único
programa reutilizável). Fonte de orientação: [auli-collections/description.md](auli-collections/description.md)
(usei-o como guia, mas as afirmações abaixo foram verificadas no código).

### 5.1 Manifesto e dependências

Fonte: [auli-collections/Cargo.toml](auli-collections/Cargo.toml). **Edição 2024**.
Totalmente **síncrono** — confirmado: não há `tokio`/`async fn`/`.await` no código compilado
(`grep` não encontra nenhum fora de `legacy/`). Dependências: `ureq` (HTTP síncrono),
`scraper`, `regex`, `url`, `serde`/`serde_json`, `headless_chrome`, `thiserror`, `anyhow`.

### 5.2 CLI e dispatch

[auli-engine/crates/auli-collections/src/main.rs](auli-engine/crates/auli-collections/src/main.rs):
`cd auli-engine && cargo run -p auli-collections -- [--usecache] <entity> <collection>`.

- `<entity>` (omitido/vazio → `rs`) é resolvido e validado por
  `domain::entities::get_entity`.
- `<collection>` (omitido → `faqs`).
- `--usecache` ativa modo offline (só cache; cache miss vira erro).
- Dispatch: `faqs` → `run_faqs`; `servicos` → `servicos::run(&entity.id, &entity.data_dir, use_cache)`;
  qualquer outro → erro.

### 5.3 Saída: o contrato `auli-contract`

A cópia divergente `domain/collections.rs` (que duplicava o `Collection`/`EmbedStrategy` do engine)
foi **apagada**. O scraper agora compila o conteúdo no **contrato tipado** (`auli-contract`): para
cada kind monta uma `Table<P>` (`Table<Faq>` / `Table<Servico>`), preenchendo o `text_to_embed` de
cada registro, e grava em `data/<id>/raw/<id>-<kind>.json`. O engine (`auli update`) lê isso direto
— não há mais `EmbedStrategy`/parsing de `portal-*.txt` deste lado nem do outro.

- **`text_to_embed`** (D2): faqs → breadcrumb `origin` + a pergunta (reproduz a key do antigo
  `QuestionKey`); servicos → `tipo | classe` + título + início do corpo da descrição.
- **`stored_repr`** reproduz o bloco `## pergunta`/`## resposta` (mesma forma do `portal-*.txt`), então
  o contexto servido ao RAG continua coerente.
- Os antigos `faqs.json` (árvore) e `servicos.json` (flat agregado) foram **descartados**; o
  `portal-<kind>.txt` continua sendo escrito como *print* legível (auditoria), nunca lido de volta.
- `pareceres`/`notas` são autorados (sem scraper) e ainda não têm fonte struct — ficam ausentes nos
  packs até serem modelados como `Table<P>`.

### 5.4 Registro de entidades

[auli-collections/src/domain/entities.rs](auli-collections/src/domain/entities.rs): config e
dados **separados** — config em `./src/entities/<id>/` (`ENTITIES_DIR`), saídas em
`./data/<id>/` (`DATA_DIR`). Entidades presentes: `rs` (SEFAZ-RS) e `sc` (SEF-SC), cada uma
com `entity.json` + `prompt.txt` em [src/entities/](auli-collections/src/entities/).

### 5.5 Scraper de FAQs (ativo, `src/faqs/`)

[auli-collections/src/faqs/mod.rs](auli-collections/src/faqs/mod.rs) e submódulos
(`faq.rs`, `fetch.rs`, `html.rs`, `portal.rs`):

- Caminha o portal de FAQs da SEFAZ-RS a partir de uma raiz, classificando páginas por
  `data-matriz-source-uri` (`sanfona.comnavegacao` → `Faq`; `categoriafaq` → `Menu`; demais
  → `Geral`).
- Modelo de saída `FaqNode { title, url, page_type, origin, children, faq_items }`
  ([faqs/faq.rs](auli-collections/src/faqs/faq.rs)).
- `fetch.rs` usa agente `ureq` síncrono + cache em disco; o conteúdo vem de um endpoint AJAX
  (campo `body` com markup). Retentativas com backoff exponencial.
- `html.rs` é um conjunto de helpers HTML→texto feitos à mão (sem crate de DOM).
- `portal.rs` achata a árvore em `portal-faqs.txt`, um bloco `## pergunta`/`## resposta`
  numerado (`// N.`) por pergunta ([faqs/portal.rs](auli-collections/src/faqs/portal.rs)).
- Saídas: `<data_dir>/faqs.json` e `<data_dir>/portal-faqs.txt`
  (`output_path`/`portal_path` em [faqs/mod.rs](auli-collections/src/faqs/mod.rs)).
- Usa o `crate::errors` unificado.

[auli-collections/src/faqs/legacy/](auli-collections/src/faqs/legacy/) contém os arquivos
originais copiados — **não declarados como módulos, portanto não compilados** (confirmado:
`faqs/mod.rs` declara só `faq`, `fetch`, `html`, `portal`). É onde aparece a única referência
a `reqwest` no repositório (em código não compilado).

### 5.6 Scraper de Serviços (`src/servicos/`)

[auli-collections/src/servicos/mod.rs](auli-collections/src/servicos/mod.rs):
`run(entity_id, data_dir, use_cache)` despacha por entidade:

- **`rs`** ([servicos/extrair_descricoes.rs](auli-collections/src/servicos/extrair_descricoes.rs)):
  renderiza as 5 páginas de público com **headless Chrome** (`Browser`/`LaunchOptions`),
  esperando o elemento `.card`; depois busca cada página de detalhe via **`ureq`** e extrai a
  descrição. Falhas de detalhe são coletadas e reportadas no fim (não fatais).
- **`sc`** ([servicos/sc.rs](auli-collections/src/servicos/sc.rs)): consome a **API JSON do
  Next.js** da SEF-SC (sem browser, sem parse de HTML); normaliza links `[[url anchor]]` para
  o formato RS.
- `finish()` (compartilhado) gera `portal-servicos.txt`
  ([servicos/gerar_portal_servicos.rs](auli-collections/src/servicos/gerar_portal_servicos.rs)),
  agrega `servicos.json` e escreve `servicos-index.json` (manifesto de abas consumido pelo
  frontend).
- **Dedup por `link`** (não por `id`): um serviço pode aparecer em vários públicos; o
  `link` é a chave única.
- O formato de `portal-servicos.txt` gerado é `// N.` + `## pergunta` (breadcrumb
  `tipo | classe` + título) + `## resposta` (corpo + `Link:`), idêntico ao de faqs
  ([gerar_portal_servicos.rs](auli-collections/src/servicos/gerar_portal_servicos.rs)).

**Distinção ativo vs em refatoração:** o módulo `servicos` ainda usa
`Box<dyn std::error::Error>` (não o `crate::errors`) e tem URLs de tipo hardcoded em
[servicos/utils.rs](auli-collections/src/servicos/utils.rs); está "wired" e funcional, mas
não totalmente refatorado para o formato limpo do `faqs`.

**Correção a uma afirmação da própria base:** o comentário em
[servicos/mod.rs](auli-collections/src/servicos/mod.rs) diz que o `rs` usa "reqwest fetches
each service's detail page". Isso está **incorreto**: o fetch de detalhe usa **`ureq`**
([servicos/extrair_descricoes.rs](auli-collections/src/servicos/extrair_descricoes.rs), linha 7
`use ureq::Agent;`). `reqwest` só existe em `faqs/legacy/` (não compilado).

### 5.7 Cache e modo offline

Cache em disco para ambos os scrapers (`cache/faqs/` em [faqs/fetch.rs](auli-collections/src/faqs/fetch.rs);
`cache/servicos/` em [servicos/cache.rs](auli-collections/src/servicos/cache.rs)). O
`.gitignore` ignora `/data/*/cache`. `--usecache` propaga o flag até a camada de fetch e
torna um cache miss um erro.

### 5.8 Cobertura por kind (collections)

| kind | scraper existe? |
| --- | --- |
| `faqs` | sim (`rs`); `sc` **não** implementado — arm placeholder em `faq_source_for` |
| `servicos` | sim para `rs` (Chrome) e `sc` (JSON) |
| `pareceres` | **não** há scraper — só existe o `portal-pareceres.txt` como dado |
| `notas` | **não** há scraper — só existe o `portal-notas.txt` como dado |

Confirmado: `main.rs::faq_source_for` tem um arm `sc` com URLs corretas mas comentário
explícito de que o parser de HTML do RS **não** funcionará para SC sem reescrita.

---

## 6. Divergências entre componentes — resolvidas pela unificação sob `data/`

> **RESOLVIDO — integração `data/` (Fases 1–4 da unificação).** As divergências históricas entre
> os domínios duplicados foram **eliminadas** pela fonte única `data/`.

1. **Triplicação do `domain` resolvida.** `data/registry.toml` é a fonte única de entidades, lida
   por `auli-cli` e `auli-collections`; o frontend gera `entities.ts` dela. O kind vetorial canônico
   é `services` (o registry mapeia o rótulo de UI `servicos`), então não há mais o descasamento
   `services`↔`servicos` nem cópias divergentes de `domain`/`errors`/`entities`.
2. **Dados de serviços consistentes.** Packs e frontend vêm da **mesma** raspagem (contrato
   `auli-contract`); o engine não declara mais `delimiter`/`EmbedStrategy` próprios para serviços.
3. **`sc` é entidade real** do server (208 serviços), não só do frontend.
4. **`pareceres`/`notas`/`conteudos`** (autorados, sem scraper) ficam versionados em `data/<id>/ref/`,
   exibidos no frontend e (pareceres/notas) ingeríveis nos packs quando modelados como `Table<P>`;
   ainda **não** são consultados no RAG ativo.

Resíduo: dentro do backend o domínio é fonte única (`corpus`/`vector-store`); o `auli-frontend`
mantém um espelho **gerado** (não mais divergente) do registro. Pendências em
[auli_pendencias.md](auli_pendencias.md).

---

## 7. Resumo: implementado e ativo vs modelado/inativo

**Ativo e funcionando (confirmado no código):**

- `auli server` (workspace `auli-engine`): `GET /v1/health`, `POST /v1/question` (RAG completo
  **in-process**: fastembed/BGE-M3 → vector store próprio → LLM externo, com log em `./logs/`) e
  `GET /v1/{kind}/list` (leitura). Público, **sem auth nem banco**; CORS; configuração por `.env`
  (`config()`); logging via `tracing`. Vetorização separada pelo `auli update`.
- RAG consulta efetivamente apenas `services` (10) + `faqs` (20); estreitamento por proximidade
  presente mas em modo paridade (`band=∞`) até calibração.
- `auli-frontend`: SPA com seleção de entidade (rs/sc), chat contra `POST /v1/question` com
  timeout de 25s, abas de referência lendo `public/<id>/`, tema claro/escuro, testes Vitest.
- `auli-collections`: scraper de `faqs` (rs) e de `servicos` (rs via Chrome, sc via JSON),
  cache em disco, modo `--usecache`, geração do contrato + `portal-*.txt` + `servicos-index.json`.

**Modelado/declarado mas inativo ou incompleto:**

- Server: `pareceres`/`notas` listáveis mas **não consultados** no RAG ativo.
- Collections: módulo `domain` `#![allow(dead_code)]` (não consumido por pipeline);
  `faqs` para `sc` não implementado; sem scraper de `pareceres`/`notas`; `servicos` ainda em
  `Box<dyn Error>` (refatoração pendente).
- Frontend: sem fluxo de autenticação/JWT no cliente; multi-tenant apenas por troca de
  `public/<id>/` (lista de entidades hardcoded).

---

## 8. Itens marcados como NÃO CONFIRMADO NO CÓDIGO

- Fluxo de autenticação no frontend: inexistente no código (o backend atual também não tem auth).
- Reranking: não há reranking no caminho RAG ativo; o único estreitamento de resultados é o
  `select_by_proximity` (por distância cosseno), hoje em modo paridade (`band=∞`).
