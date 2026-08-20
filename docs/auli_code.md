# Auli — Descrição Técnica do Código

Documento técnico auditável do Projeto Auli, baseado na leitura direta do código-fonte
dos três componentes (`auli-server`, `auli-frontend`, `auli-collections`). Cada afirmação
relevante cita o arquivo que a sustenta. Quando algo não pôde ser confirmado no código,
está marcado como **NÃO CONFIRMADO NO CÓDIGO**.

**Operação:** para **compilar, gerar os dados e subir o app** (com Cloudflare Tunnel) e saber **onde ficam
os logs**, ver o runbook [docs/auli_operations.md](docs/auli_operations.md).

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

```text
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
> `Table<P>` preenchendo `text_to_embed`; desde a G5b o `auli update` lê as **árvores `.md`**, não o
> contrato JSON, recompõe o `text_to_embed` pelo ponto único e armazena o payload leve
> (`DocumentoPack`). `STRATEGY_VERSION` está em **2** (pack v2, ago/2026). Caminhos `auli-collections/…`
> nas seções abaixo vivem hoje em **`auli-server/crates/auli-collections/…`**.

---

## 2. Arquitetura e fluxo de dados ponta a ponta

A integração entre os componentes é por **duas pastas na raiz**, não por chamadas diretas nem cópia
manual: **`config/`** (catálogo versionado: `registry.toml` + `prompts/`) e **`data/`** (dado
coletado, gitignored), com `data/<id>/{raw, ref, docs, packs}` por estado:

- `raw/` — saída da derivação: contratos, índices e o cache do scraper.
- `ref/` — conteúdo autorado (notas, conteúdos).
- `docs/` — **as árvores `.md`, uma por coleção** (`servicos`, `faqs`, `pareceres`, `tarf`). É a
  FONTE do `auli update` e é lida na query; o manifesto carimba um `docs_hash` sobre ela e o boot
  recusa se divergir.
- `packs/` — pacotes vetoriais gerados pelo `auli update`.

O **server** lê os packs de `data/<id>/packs/` e as entidades do registry; o **frontend** tem
`entities.ts` e `public/<id>/` **gerados** de `data/` por `scripts/`.

```text
[auli-scraper-<e>]  →  data/<id>/<id>-<kind>-snapshot.json  →  [auli-collections <e>]  →  data/<id>/raw/<id>-<kind>.json
  scrape do portal            (snapshot v3)                deriva (offline)                    │
                                                                                              │
[auli-server] auli update  → data/<id>/packs/ (vetoriza o contrato)  ←─────────────────────────┘
[auli-server] auli server (somente leitura)                          [auli-frontend] entities.ts + public/<id>/
        │                                                            (gerados de data/ por scripts/)
        ▼
pergunta ──> embedding in-process ──> busca vetorial ──> LLM externo ──> resposta ──> UI
```

Observações confirmadas no código:

- Os **scrapers** (`auli-scraper-<e>`) escrevem o **snapshot v3**; o **`auli-collections <e>`** deriva
  o **contrato tipado** (`auli_contract::Table<P>`) em `data/<id>/raw/`, que o `auli update` lê direto
  (sem parsing de `portal-*.txt`, que viraram só _print_ — ver §5).
- O `auli-frontend` lê de `public/<id>/`, **gerado** de `data/` por
  [scripts/build-frontend-public.sh](scripts/build-frontend-public.sh) — não há mais cópia à
  mão entre pastas.
- O registro de entidades é único (`config/registry.toml`); o frontend mantém um espelho
  **gerado** (`entities.ts`), não mais divergente (ver §6).

---

## 3. Backend — o workspace `auli-server`

O backend é um **workspace Cargo único** (`auli-server/`), com **três crates em camadas** e
**um binário** `auli` que troca de modo por subcomando (`auli server` / `auli update`). A camada
de autenticação (JWT) e o **PostgreSQL** foram **removidos** (eram usados só para auth): o server
hoje é **público, sem banco**, e expõe apenas rotas de leitura.

### 3.1 Estrutura (camadas estritas, acoplamento só para baixo)

```text
auli-server/                       # workspace único, Cargo.lock compartilhado
└── crates/
    ├── vector-store/      # BAIXO — store plano por cosseno, agnóstico (sabe só id+vetor+payload P)
    ├── auli-core/         # MEIO  — domínio auli: embed (BGE-M3), corpus, manifest
    ├── auli-retrieval/    # MEIO  — o MOTOR: embedder + ReadStores + estreitamento (§3.11)
    ├── auli-cli/          # TOPO  — o binário `auli`: server (RAG + retrieve + MCP) + update
    ├── auli-contract/     # forma + I/O do snapshot (D-C1) — a fronteira, compartilhada scraper↔engine
    ├── auli-collections/  # DERIVA os artefatos do snapshot (offline) — ver §5
    └── scrapers/          # a frota (compila leve; nunca importa o engine) — ver §5 / SCRAPERS.md
        ├── auli-scraper-kit/    # kit compartilhado: o "como raspar" (cache, agente HTTP, aggregate)
        └── auli-scraper-<id>/   # um binário por entidade (27) — ver §5.3
```

> **Fase 2 (scrapers em crates próprios).** A coleta saiu do `auli-collections` para **um binário por
> entidade** (`auli-scraper-<e>`) sobre o **`auli-scraper-kit`**; todos gravam a mesma fronteira, o
> **snapshot v3** (um por coleção: `data/<id>/<id>-servicos-snapshot.json` + `<id>-faqs-snapshot.json` no RS; tipos em `auli_contract::snapshot`). O
> `auli-collections <e>` virou **só derivação** (offline): lê o snapshot e produz o contrato +
> prints + index + per-público. As citações a `auli-collections/src/{faqs,servicos}/…` na §5 vivem
> hoje em `auli-scraper-rs/src/…` (ver §5 reescrita).

`vector-store` ← `auli-core` ← `auli-retrieval` ← `auli-cli`. O `Cargo.lock` único garante que os modos `update`
(embeda documentos) e `server` (embeda a pergunta) usem o **mesmo** `fastembed`/modelo — o espaço
vetorial é compartilhado por construção, não por convenção.

| Crate            | Conteúdo                                                                                                                                                                                                                                                                                                                                                  |
| ---------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `vector-store`   | `Record<P>`/`CollectionData<P>` (payload genérico; chave JSON em disco continua `document`), `cosine_distance` (fallback `2.0`), IO de arquivo, e a **separação leitura/escrita por tipo**: `ReadStore` (`query_scored`/`list`, imutável) vs `Writer` (`reset`/`upsert`/persistência). Enforcement de dimensão no 1º insert (`Error::DimensionMismatch`). |
| `auli-core`      | `embed` (`Embedder` BGE-M3, `EMBED_DIM=1024`), `corpus` (`EmbedStrategy`, tabela `Collection`, `parse_blocks*`, `prepare_documents`, `extract_question`, `clean_servico`, `extract_servico_description`), `manifest` (identidade do embedding + schema/validação).                                                                                        |
| `auli-retrieval` | O **motor de recuperação** (§3.11): `Engine` (embedder + `Collections` + raiz `docs/`) e o núcleo em funções livres — `search_embedded`, `entidades_com`, `documento_por_numero`, `ler_corpo`, `decode_parecer`, `select_by_proximity`. Depende só de `auli-core`/`vector-store`/`auli-contract`: **sem HTTP, sem LLM, sem anonimizador**, e só enxerga `ReadStore`. |
| `auli-cli`       | `server` (axum, RAG, `/v1/retrieve`, MCP, config, packs) + `update` (vetorizador). Despacho por `clap`.                                                                                                                                                                                                                                                                        |

### 3.2 Os dois modos (subcomandos)

```text
auli update  --entity <id> --out <dir> [--version <v>]   # fonte: as árvores `<dir>/../docs/`
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
| POST   | `/v1/question`    | caminho RAG ativo (o único que chama LLM externo)                                      |
| POST   | `/v1/retrieve`    | recuperação **pura** (§3.11): embeda local, devolve documentos + score, sem LLM        |
| GET    | `/v1/{kind}/list` | listagem de uma coleção (leitura); `{kind}` ∈ `servicos \| faqs \| pareceres \| notas` |
| —      | `/mcp`            | servidor **MCP** (rmcp, streamable HTTP) aninhado por `nest_service` (§3.12)           |

A ingestão deixou de ser rota HTTP (antes `load_from_file`/`load_from_web`) — virou o `auli update`.
CORS: origens **hardcoded** (auli.com.br, www, e portas locais 3000/5173/8080), métodos
GET/POST/OPTIONS. O layer de CORS envolve `/mcp` também, e é inócuo ali: clientes MCP não são
browsers (o Claude conecta a partir da nuvem da Anthropic, sem header `Origin`).

**Rate limit por IP** (`api/ratelimit.rs`, GCRA via `governor`, chaveado pelo IP real do
`CF-Connecting-IP`): `/v1/question` e `/v1/retrieve` **compartilham um único limiter** (1 req/s,
burst 2) construído uma vez no `app()` — o recurso disputado é o embedder, e como
`question_rate_limiter()` cria um limiter novo a cada chamada, instanciar um por rota dobraria a
cota efetiva. O `/mcp` tem limiter **próprio e mais folgado** (10 req/s, burst 30): o handshake MCP
faz várias requisições em sequência e quebraria sob 1 req/s — mas quota diferente não é o mesmo
que ausência de quota.

### 3.4 Caminho RAG ativo (`exec_all_question`)

Acionado por `POST /v1/question`. Assinatura:
`exec_all_question(engine, anonimizador, question, entity, query_type)`:

1. **Anonimiza a pergunta**, _fail-closed_: em erro usa um placeholder fixo, nunca o texto cru. O
   `mapping` fica em memória, no escopo da requisição, para restaurar a resposta do LLM — nunca é
   persistido.
2. Resolve a entidade (registry); entidade desconhecida → a própria mensagem de erro é retornada
   como resposta, com HTTP 200 (sem panic).
3. Gera o embedding da pergunta **uma vez**, in-process (`embed_dense`, executado fora da thread
   async via `run_blocking`/`spawn_blocking`); a pergunta já é uma "chave" curta.
4. **O `query_type` decide o caminho** — é o seletor de fonte da caixa de mensagem, que chega como
   inteiro no campo `type`:
   - **`ServicosFaqs`** (código 1, o default): consulta **duas** coleções de forma **concorrente**
     (`tokio::try_join!`), `<id>-servicos` (`n_results = 10`) e `<id>-faqs` (`n_results = 20`).
   - **`Jurisprudencia(Kind)`** (código 2 = `pareceres`, 3 = `tarf`): uma coleção só,
     `n_results = 10`. Nos **pareceres** há ainda a expansão por grafo — pareceres que citam os
     mesmos dispositivos —, exclusiva deles (o `dispositivos-index.json` é derivado da árvore de
     pareceres).

   Cada `query_scored` roda em thread bloqueante e retorna `(payload, distância cosseno)` ordenado
   do mais próximo ao mais distante.
5. **Estreitamento por proximidade** (`select_by_proximity`): mantém os `floor` melhores e, além
   disso, os documentos dentro de `band` (distância acima do melhor) — por kind. **Os defaults são
   `floor=0`, `band=∞`**, ou seja, hoje há _paridade_ com o "take fixo" antigo (nenhum descarte); os
   bandos só passam a filtrar após calibração contra perguntas reais (scores logados em `debug`).
6. **Monta o bloco de cada documento** (`rag::blocos`): o payload do pack é leve, então o conteúdo é
   lido da árvore `docs/` na hora, só para os k escolhidos. Conteúdo ilegível degrada (o `resumo`, na
   jurisprudência; um aviso, em serviços e FAQs) e **nunca** derruba a query.
7. Envolve cada bloco em `## documento {i}: {rótulo}` e concatena; o system prompt = prompt da
   entidade **para aquele tipo** (`system_prompt`, `pareceres_prompt` ou `tarf_prompt`) + contexto +
   delimitador `'''`.
8. Chama o LLM externo — com a pergunta **anonimizada** se o flag estiver ligado (default), e
   restaura os placeholders na resposta antes de devolvê-la.
9. Retorna `{ question, answer }` e **anexa o diálogo a `$AULI_LOG_DIR/<timestamp>.txt`** (default
   `./logs` do CWD; o `start_server.sh` aponta para `<raiz>/logs`).

**As CINCO coleções alimentam respostas.** `servicos` e `faqs` juntas (o chat de atendimento),
`pareceres`, `tarf` e `legislacao` separadamente — uma coleção por consulta, cada uma com o próprio
prompt de sistema. Só `notas` fica de fora: tem rota de listagem, mas não tem fonte struct nem
pack — ver §3.13.

### 3.5 Clientes e adaptadores (embeddings/busca/LLM in-process)

Toda a parte de embeddings/busca vetorial é **in-process** (sem Ollama nem ChromaDB):

- **Embedder** (`auli-core::embed`): `fastembed` com **BGE-M3 ONNX INT8** (`Bgem3Model::BGEM3Q`),
  dimensão **1024** (`EMBED_DIM`), apenas a saída _dense_. O modelo fica atrás de um `Mutex` porque
  `embed` é `&mut self`; `max_length` é **8192**, o teto do próprio BGE-M3 (`EMBED_MAX_TOKENS`) — era
  512 até a TAREFA-FAQ-PR, e com 512 a resposta da FAQ era cortada em silêncio. `embed_dense` é
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
  `max_completion_tokens 4096`, `stream:false`, timeout **30s**. Até **3 tentativas** em erros de
  conexão/timeout (sleep 500ms). Erro de API vira mensagem legível (não `Err`).
  - **Modelo de referência** (via `LLM_API_MODEL`): **openai/gpt-oss-120b** (Groq). Janela de contexto
    **131.072** tokens; saída máx. **65.536**; ~**500 tok/s** (≈8s para os 4.096 tokens do teto).
    Preço/1M tokens: input **$0,15** (cache **$0,075**), output **$0,60**. Suporta tool use, browser
    search, code execution, JSON object/schema mode e reasoning. MoE 120B total / 5,1B ativos.
    → O `max_completion_tokens` (4.096) e o contexto do RAG ficam bem abaixo dos limites do modelo.

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

### 3.6.1 Formato 2 do pack e update incremental — as catorze decisões (D-INC-*)

O manifesto carrega **dois** números de versão, e a fronteira entre eles é o que separa migração de
re-embed: `strategy_version` = "o que entra no vetor mudou" ⇒ **reembed obrigatório**; `pack_format` =
"como o arquivo é escrito mudou" ⇒ **migração, nunca reembed**.

As catorze decisões vinham da `TAREFA-UPDATE-INCREMENTAL.md`, removida em 12/08/2026 depois de
executada (PRs #139–#143). Resumidas aqui porque são o registro de _por que_ o pack tem a forma que
tem — sem elas, "por que `id = doc_path`?" e "por que o `upsert` recebe `Vec<Record>`?" viram
arqueologia. Texto integral no git.

| | decisão | por quê, em uma linha |
|---|---|---|
| **D-INC-1** | identidade do registro é o `doc_path` | com `id-1..id-N` posicional, inserir um documento no meio da ordem alfabética desloca a identidade de todos os seguintes, e upsert incremental fica impossível |
| **D-INC-2** | `key_hash` ao lado do vetor | hash do `text_to_embed`; igual ⇒ reaproveita, diferente ou ausente ⇒ re-embeda. Opcional no serde, então pack antigo degrada para "tudo alterado" em vez de quebrar |
| **D-INC-3** | só o **vetor** é cache; o payload é sempre reconstruído da árvore | há campo no payload fora da key (o `link` da jurisprudência): reaproveitá-lo seria uma classe inteira de staleness silenciosa |
| **D-INC-4** | ordenação canônica por `id` na escrita | é o que permite exigir igualdade **byte a byte** entre rebuild total e incremental; como `id` é o nome do arquivo, preserva a ordem de antes |
| **D-INC-5** | `apply(upserts, removes)` numa operação, com `tmp` + `rename` | eram duas escritas do arquivo inteiro; com escrita frequente, crash no meio deixa de ser hipótese |
| **D-INC-6** | índice `HashMap<id, usize>` no lugar de `iter_mut().find` | o find era O(n) por registro — ~250 M comparações de string no rebuild do TARF |
| **D-INC-7** | `pack_format: u32` separado do `strategy_version` | é a fronteira do parágrafo acima; `serde(default = 1)` faz manifesto sem o campo **ser** o formato 1 |
| **D-INC-8** | política por **família** de coleção, não por flag | jurisprudência (append-only) é incremental; serviços/faqs (delete + rebuild) são reset total sempre. Formato único, duas políticas, e o lugar da política é um método em `Kind` — entidade nova herda a certa |
| **D-INC-9** | remoções **nunca** automáticas na jurisprudência | órfão vira linha num log regenerado a cada rodada, não exclusão; dissolve a discussão de teto percentual, porque nada sai sem aprovação |
| **D-INC-10** | aprovação por **subcomando**, não por flag | uma flag acabaria dentro do `update-*.sh` "para não rodar duas vezes", e o portão humano sumiria sem ninguém decidir removê-lo. Aplica `aprovados ∩ órfãos_atuais`, é idempotente e permite aprovação seletiva por linha |
| **D-INC-11** | o subcomando recarimba o manifesto | ele escreve o pack, logo atualiza `CollectionEntry` e `docs_hash` pela **mesma** função do `update`; sem isso o boot recusa corretamente e ninguém descobre por quê |
| **D-INC-12** | Fase 0 do estudo cancelada como portão | as decisões posteriores esvaziaram a pergunta que ela responderia; a medição ficou como subproduto — o update imprime quantos vetores reaproveitou |
| **D-INC-13** | migração sem reembed, com guarda de `docs_hash` | o `doc_path` já está no payload, então migrar é reescrever o `id`; mas carimbar `key_hash` com a árvore fora de sincronia faz o cache **nascer mentindo** |
| **D-INC-14** | `upsert` recebe `Vec<Record<P>>`, não um quarto slice paralelo | com slices paralelos o descasamento é detectável em runtime (`ArityMismatch`); com `Vec<Record>` ele é **inexpressável pelo tipo** |

**Uma precisão do D-INC-10 que a implementação exigiu (PR #141):** "os não aprovados permanecem no
log" é o **efeito**, não a mecânica. O subcomando **consome** o log; quem o regenera é o `auli update`
da rodada seguinte. Se o subcomando o reescrevesse com os órfãos não aprovados, devolveria ao arquivo
exatamente as linhas que o humano apagou para **não** aprovar — e a invocação seguinte as leria como
aprovação. O "não" viraria "sim" sozinho.

**Três fatos de escrita que a execução deixou** (relatórios de execução removidos em 12/08/2026):

- **`write_manifest` é atômico** (`.tmp` + `rename`), como todo o resto do pipeline. O que o motivou:
  escrever por cima de um arquivo **hardlinkado** altera o original — foi assim que uma árvore de
  teste modificou o manifesto de produção;
- **pack e manifesto não são atômicos ENTRE SI.** Cada um se escreve inteiro ou não se escreve; uma
  queda entre os dois deixa estado misto, que o `docs_hash` no boot detecta e recusa;
- **a guarda de `docs_hash` no `remover` e a não-recarimbagem defendem cenários disjuntos** — a guarda
  cobre árvore à frente do pack; não recalcular o hash cobre o pack ficar à frente da árvore.

### 3.7 Multi-tenancy (entidades)

As entidades vêm de `config/registry.toml` (fonte única), lido por `auli-cli` e `auli-collections`; o
frontend gera seu `entities.ts` a partir dele. Cada entidade tem `id`, `name`, prompt e as coleções
disponíveis. `AULI_DATA_DIR` (default `./data`) é a raiz dos dados; o catálogo mora na `config/`
irmã (`AULI_CONFIG_DIR` sobrepõe). A raiz de dados cobre
`<id>/packs/` — e também o default de `--packs-dir`, então registry e packs compartilham uma raiz por
construção. Entidades hoje (9, todas ativas no server): `rs` (SEFAZ-RS), `sc` (SEF-SC), `sp`
(SEFAZ-SP), `pr` (SEFA-PR), `mg` (SEF-MG), `pe` (SEFAZ-PE), `ba` (SEFAZ-BA), `rj` (SEFAZ-RJ) e
`ce` (SEFAZ-CE).

### 3.8 Distribuição (decorrência do desenho)

Servir o `auli server` exige **apenas o binário + a pasta de pacotes** (`<id>-<kind>.json` +
`<id>.manifest.json`). Sem banco para subir, sem ChromaDB/Ollama, sem serviço de embedding, sem rede
para ingestão — o server é autossuficiente de ponta a ponta. Read-only + eager-load + binário único ⇒
N cópias coexistem sem coordenação.

### 3.9 Verificação (estado de fato, neste repo)

- **Build/test:** `cargo build --workspace` e `cargo test --workspace` passam **sem warnings**
  (inclui `clippy -D warnings`); **9 testes ficam _gated_** por `#[ignore]` — os que exigem packs +
  modelo real (`manifest_validates_and_query_retrieves`, os dois de `/v1/retrieve`, os três de
  `embed::testes_ordem`), o golden do RS (`golden_rs_equivalence`, precisa de `AULI_GOLDEN_DATA`) e
  as duas ferramentas de A/B (`ab_faq_pr`, `dump_pool`).
- **Pacotes reais gerados** via `auli update` (`strategy_version: 2`): **29.732 documentos** nas 27
  entidades — 4.205 serviços, 19.780 pareceres, 1.947 FAQs e 3.800 acórdãos do TARF. O manifest
  confere — `bytes` e `hash` FNV-1a batem com os arquivos (o `packs::load_all` re-hasheia e alerta em
  divergência); todos os vetores em dim 1024; chave `document` preservada. Distribuição do momento:
  ver `docs/auli_operations.md` §4.1, que traz o comando que a imprime dos manifestos.
- **Boot e2e (modelo real):** o server carrega as 27 entidades, valida cada manifest contra a
  identidade local e responde `POST /v1/question` por estado citando os links do próprio catálogo
  (verificado ao vivo em várias, incl. rs/ba/rj/ce).

### 3.10 Decisões de desenho

- **Ingestão fora do server.** As rotas `load_from_file`/`load_from_web` saíram do server (o frontend
  nunca as usou — só `POST /v1/question`); a vetorização é o `auli update`.
- **Store em memória imutável.** Carga _eager_ + `ReadStore` imutável (sem lock no caminho de consulta).
- **Domínio como fonte única no backend.** `corpus`/`vector-store` são fonte única para `server` e
  `update`. O `auli-frontend` segue com seu próprio espelho **gerado** do registro (ver §6).
- **Auth e banco removidos.** O server não tem mais auth/JWT nem PostgreSQL — é público.

### 3.11 O motor: crate `auli-retrieval`

O que uma consulta semântica precisa — embedder, `ReadStore`s por `<entidade>-<kind>` e
estreitamento por proximidade — vive num crate próprio, extraído do `auli-cli/src/rag.rs`. É a peça
compartilhada pelas **três faces** do servidor: chat (`/v1/question`), retrieval HTTP
(`/v1/retrieve`) e MCP (`/mcp`).

**Fronteira (D-MCP-2).** Depende só de `auli-core` (Embedder), `vector-store` (ReadStore) e
`auli-contract` (payload/mddoc) — **nunca** de axum, rmcp, `auli-llm` ou `auli-anon`. Enxerga só
`ReadStore`, jamais `Writer`: é somente-leitura por construção, como o servidor. A garantia
verificável é `cargo tree -p auli-retrieval` sem `axum`/`rmcp`/`reqwest`/`auli-llm`/`auli-anon` —
`fastembed`/`ort` **estão** lá, via `auli-core`, e devem estar; a fronteira é sobre HTTP/LLM/PII,
não sobre leveza de build.

**Forma: funções livres + `Engine` delegando (D-MCP-4).** O núcleo são funções sobre
`&Collections`/`&Path` — `search_embedded`, `entidades_com`, `documento_por_numero`, `ler_corpo`,
`decode_parecer`, `select_by_proximity` — e os métodos do `Engine` são delegações de uma linha. O
motivo é testabilidade: `Engine` guarda `Arc<Embedder>`, e construir um carregaria o BGE-M3; com as
funções livres os testes cobrem o motor inteiro sem tocar o modelo.

**Sincronia (D-MCP-3).** Tudo blocking (o embed é CPU-bound); quem é async envelopa em
`spawn_blocking`, como o `rag.rs` já fazia.

**Semântica de erro — importa mais do que parece.** Com o `packs::load_all` atual, **toda entidade
registrada tem TODOS os kinds no mapa**: arquivo ausente vira store **vazio**, não ausência. (É o
que faz uma coleção nova entrar sem tocar nas outras 26 entidades: elas sobem servindo zero
registros dela, em vez de recusar o boot.) Logo
`Error::ColecaoAusente` só dispara para coleção realmente fora do mapa (entidade não registrada), e
store vazio é **sucesso com zero hits**. Quem precisa distinguir "tem acervo de verdade" usa
`entidades_com`, que exige store não-vazio — é a guarda correta das ferramentas MCP (§3.12), onde
um teste por `store().is_some()` aceitaria uma UF registrada e vazia.

### 3.12 A face MCP (`auli-cli/src/mcp.rs`)

Servidor **Model Context Protocol** sobre o mesmo motor, via **`rmcp` 2.2** (SDK oficial). O
`StreamableHttpService` é um serviço tower aninhado em `/mcp` por `nest_service` no mesmo `Router`
axum — mesmo processo, mesma porta, mesmo `Arc<Engine>`. É a razão de não ser um binário separado:
o BGE-M3 carrega **uma vez**.

Quatro ferramentas em pt-BR (D-MCP-7, ampliada) — o consumidor é a IA de um auditor brasileiro:

| Ferramenta                | Argumentos                | Devolve                                                        |
| ------------------------- | ------------------------- | -------------------------------------------------------------- |
| `listar_entidades`        | —                         | UFs com acervo **não-vazio** e, por UF, os kinds e seus totais |
| `buscar_pareceres`        | `uf`, `pergunta`, `top_k` | metadados + sinopse + link + score; **sem** o corpo            |
| `obter_parecer`           | `uf`, `numero`            | o parecer com o **corpo integral**, lido da árvore `docs/`     |
| `consultar_servicos_faqs` | `uf`, `pergunta`          | o bloco de contexto RAG serviços+FAQs, byte a byte o do chat   |

A quarta é a face MCP do tipo `ServicosFaqs` do chat: reusa `rag::retrieve` e
`rag::montar_rag_servicos_faqs` (ambos `pub(crate)` por isso) com as MESMAS constantes
(`SERVICES.n_results`/`FAQS.n_results`, `SVC_*`/`FAQ_*`). Não é reimplementação — é o mesmo código,
e é o que torna a paridade automática em vez de combinada.

**Decisões fechadas do `consultar_servicos_faqs`** (PR #115) — registradas porque cada uma é uma
alternativa razoável que foi recusada por um motivo, e sem o motivo alguém a "conserta" depois:

- **Uma ferramenta só**, não `buscar_servicos` + `buscar_faqs`. O chat também trata a dupla como um
  tipo único (`QueryType::ServicosFaqs`); separar criaria no MCP uma granularidade que o motor não
  tem.
- **A saída é o bloco de texto do chat, não JSON.** As outras ferramentas devolvem JSON porque são
  listas de hits com metadados; esta devolve o **contexto já montado**, e o formato é o contrato —
  travado por `montar_rag_servicos_faqs_pina_o_formato` em `rag.rs`. Converter para JSON exigiria
  desmontar o que o `montar_rag_*` monta, e a paridade morreria na primeira divergência.
- **Sem `top_k`.** É o que garante que as constantes sejam as mesmas nas duas faces. Um knob no MCP
  faria o assistente pedir recortes que o chat nunca produz, e a calibração das bandas deixaria de
  valer para os dois de graça.
- **Coleção ausente não é erro.** A guarda é `serviços OU FAQs`; UF com só um dos dois responde com
  a parte que tem, como o chat. Só a dupla ausência vira `invalid_params`.
- **Nenhum LLM neste caminho** (D-MCP-5). Quem raciocina sobre o bloco é a IA do usuário — é o papel
  que o LLM do Auli cumpre no chat.

Detalhes que o código explicita:

- **A guarda da UF roda antes do embed** e usa `entidades_com` (§3.11), não `store().is_some()`.
- **`#[tool_handler(router = self.tool_router)]`** — o default do macro é `Self::tool_router()`,
  que **reconstrói** o roteador a cada `list_tools`/`call_tool`; apontar para o campo montado no
  `new()` monta uma vez só.
- **`Implementation::new("auli", …)`**, não `from_build_env()`: o `env!` daquele helper expande no
  build do **RMCP**, e o servidor se anunciaria como `"rmcp" 2.2.0` ao assistente.
- **`allowed_hosts` precisa do hostname público.** O `StreamableHttpServerConfig::default()` só
  aceita **loopback** (guarda de DNS rebinding, pensada para MCP rodando na máquina do usuário);
  atrás do tunnel o `Host` chega como `api.auli.com.br` e é recusado com _"rejected request with
  disallowed Host header"_. Daí a constante `MCP_ALLOWED_HOSTS` em `api/mod.rs`, que **amplia** a
  lista sem desligar a guarda (host desconhecido segue tomando 403).
- **Privacidade (D-MCP-5):** a pergunta é embedada localmente e nunca sai do processo; o tracing
  registra `uf`/`top_k`/`hits`, nunca o texto.

> ⚠️ O smoke de protocolo roda em **localhost**, que o default do rmcp já permite — então ele
> **não pega** uma `allowed_hosts` mal configurada. Foi assim que o problema chegou à
> produção. O que cobre isso é o teste `mcp_allowed_hosts_inclui_o_hostname_publico` (`api/mod.rs`)
> e, no runbook, o smoke apontado para a **URL pública** ([docs/auli_operations.md](docs/auli_operations.md) §12.1).

Smoke de protocolo: [`scripts/tools/mcp-smoke.sh`](scripts/tools/mcp-smoke.sh) (initialize → initialized →
tools/list → tools/call). Conexão de clientes: [docs/auli_operations.md](docs/auli_operations.md) §12.

### 3.13 As cinco coleções: UMA struct, UM enum (ago/2026)

Serviços, FAQs, legislação, pareceres e acórdãos do TARF passam por **um caminho de código só**.
Duas peças, as duas em `auli-contract`:

- **`Kind { Servicos, Faqs, Legislacao, Pareceres, Tarf }`** ([kind.rs](auli-server/crates/auli-contract/src/kind.rs))
  — o nome da coleção. É o MESMO texto em todos os papéis, de propósito: subdiretório da árvore
  (`docs/<kind>/`), sufixo da coleção vetorial (`<id>-<kind>`), nome do pack, parâmetro da rota
  `/v1/{kind}/list`, rótulo do `registry.toml` e argumento da CLI. Antes disso o nome era um literal
  repetido em oito lugares, e errar um deles compila, roda e só aparece na tela do usuário.
- **`Documento`** ([lib.rs](auli-server/crates/auli-contract/src/lib.rs)) — `{ kind, trilha, titulo,
  ementa, resumo, corpo, link, text_to_embed, resumo_info }`, no vocabulário unificado das árvores.
  `Servico` e `Faq` recuaram para as BORDAS (o `process` e os JSONs do frontend) e se projetam nela
  por `para_documento()`; quem é vetorizado é a projeção.

O que isso desfez, e é a medida do ganho: **quatro renderizações do bloco RAG viraram uma**
(`bloco()`), **três funções de leitura de árvore viraram uma** (`preparar`), e o pack passou a
guardar o mesmo payload leve (`DocumentoPack`) para todas — antes, serviços e FAQs carregavam o
texto integral duplicado, uma cópia na árvore e outra ao lado do vetor. A prova de que o caminho
único pegou: a quinta coleção entrou sem nenhuma renderização, leitura ou payload novos.

A tabela de PAPÉIS (qual campo do `.md` carrega o quê em cada coleção) vive no doc de módulo do
[`mddoc.rs`](auli-server/crates/auli-contract/src/mddoc.rs) — ao lado do parser que a implementa, que
é onde ela não envelhece.

**Coleção nova** = uma variante no `Kind` + uma projeção para `Documento` (+ struct de borda, se a
fonte pedir). O TARF entrou exatamente assim (PRs #132/#133), e a **legislação** depois dele
(§3.13.1) — esta trazendo duas primeiras vezes que valem nota:

- **A primeira árvore com subpastas.** `docs/legislacao/<lei>/*.md`, uma pasta por lei, porque uma
  entidade tem N leis. A recursão é **política do `Kind`** (`arvore_com_subpastas()`), não flag de
  operação: quem lê a árvore pergunta ao enum e acerta sem parâmetro para errar, e as outras quatro
  seguem planas. O nome da pasta é o **1º segmento da `trilha`**, e o `preparar` recusa o `.md` cujo
  `doc_path` derivado não seja o caminho de onde ele foi lido — sem essa guarda, trilha errada
  produz um caminho que não existe e o corpo some na query, sem erro em lugar nenhum.
- **A primeira coleção solitária que não é jurisprudência.** Por isso o `QueryType::Jurisprudencia`
  virou `QueryType::ColecaoUnica(Kind)`: as duas ideias — "consultada sozinha" e "tem `## resumo`" —
  coincidiam até aqui. A legislação é a primeira sem resumo e sem expansão por grafo, e o chat a
  atende no **tipo 4**, com prompt próprio (citar dispositivo e link, não extrapolar o texto legal).

---

### 3.13.1 A coleção de legislação — as vinte e duas decisões (D-LEG-*)

Vieram da `TAREFA-LEGISLACAO.md`, removida em 20/08/2026 depois de executada (dos
PRs #149 a #156). Estão aqui, e não no git, pelo mesmo motivo dos `D-INC-*` da §3.6.1:
**doze arquivos de código de produção citam esses identificadores em comentário** —
`kind.rs`, `update.rs`, `indice_legislacao.rs`, `LegislacaoList.tsx`, `textSearch.ts`
e outros. Sem a tabela, cada `(D-LEG-4)` num comentário vira arqueologia.

A tabela é o registro de **por que a quinta coleção tem a forma que tem**: por que a
árvore dela é a única com subpastas, por que a chave de ordenação desce até o item da
alínea, por que o corpo entra no índice quando nas outras coleções não entra. As
emendas (`4a`, `4b`, `11a`, `12a`, `17a`) ficam onde nasceram, ao lado da decisão que
corrigem — é a sequência que conta a história.

| | decisão | por quê |
|---|---|---|
| **D-LEG-1** | O diretório é `docs/legislacao/` — **sem acento** | Doutrina nome=diretório=sufixo: o mesmo string é subdiretório, sufixo de pack (`rs-legislacao`), rota `/v1/legislacao/list`, rótulo do registry e argumento de CLI. Acento em sufixo de pack/rota/CLI é atrito permanente. A Fase 0 renomeia a pasta existente. |
| **D-LEG-2** | Árvore com **um nível de subpasta por lei**; nome da pasta = nome legível da lei | Decisão de produto. Primeira coleção não-plana do projeto. |
| **D-LEG-3** | A leitura recursiva é **política do `Kind`** (`fn arvore_com_subpastas(self) -> bool`, `true` só para `Legislacao`) | Mesmo raciocínio do D-INC-8: a política acompanha a coleção, não uma flag de operação. As demais árvores continuam planas e a leitura delas não muda em nada. |
| **D-LEG-4** | `doc_path` continua **derivado**: `docs/legislacao/{primeiro segmento da trilha}/{nome_faq_de(titulo, link)}.md` — e o `preparar` ganha uma **guarda de invariante**: se o caminho derivado ≠ caminho onde o arquivo foi de fato lido, **erro alto** com os dois caminhos na mensagem | O `doc_path` gravado no pack é como o corpo é lido tarde na query; recomputá-lo com outra regra produz corpo indisponível silencioso. A convenção "1º segmento da trilha = nome da pasta" é autoral (quem gera o dataset escreve os dois), e a guarda transforma o descumprimento em erro de build, não em defeito de query. |
| **D-LEG-4a** | O 1º segmento da trilha é `Lei {número com a barra trocada por hífen} — {nome}`. Ex.: `Lei 6.537-1973 — Processo Tributário Administrativo` | Achado da Fase 0: o 1º segmento **é** nome de diretório, e todo número de lei brasileira tem barra (`5.172/1966`, `87/1996`) — `Lei 6.537/1973` abriria um diretório fantasma dentro do `doc_path`. A troca por hífen é mecânica, e o nome depois do travessão é o que o acordeão do frontend rotula (D-LEG-13). Vale para o CTN e a Kandir, que são leis numeradas. |
| **D-LEG-4b** | Norma **sem número de lei** usa nome DESCRITIVO no 1º segmento, recortado pela parte que entra no acervo: `Constituição Federal — Sistema Tributário Nacional` | Emenda da 2ª entrega. O D-LEG-4a pressupõe `Lei {número}`, e a CF não tem número — aplicá-lo produziria um nome torto. O recorte no nome não é cosmético: se um dia entrar outra parte da CF (art. 195, ADCT), ela vira **outra pasta**, e o acordeão passa a agrupar por recorte temático em vez de despejar a Constituição inteira num grupo só. O invariante do `doc_path` não muda — o 1º segmento continua sendo, literalmente, o nome do diretório. |
| **D-LEG-5** | Propriedades do `Kind::Legislacao`: `as_str = "legislacao"`, `exige_resumo = false`, `pack_incremental = false` (delete+rebuild, família das mutáveis/curadas), posição em `TODOS` logo após `Faqs` | "Funciona como as FAQs" — coleção pequena, autorada, reconstruída por inteiro. |
| **D-LEG-6** | Textos do `Kind`: `rotulo = "Legislação"`, `plural = "perguntas de legislação"`, `titulo = "Legislação"` | O rótulo rotula UM documento no bloco RAG (`## documento {i}: Legislação`). |
| **D-LEG-7** | Key própria: `compose_legislacao_text_to_embed(trilha, pergunta, dispositivo, resposta)` = `compose_unificado(trilha, "P: {pergunta}", dispositivo, "R: {resposta}")` | Igual à das FAQs **mais a ementa no slot dela**: o dispositivo ("Art. 21") é token de altíssimo valor — analista cita artigo. A fórmula é própria (adaptador novo, teste de fórmula congelada próprio) para que o gate P5 das FAQs e esta coleção possam divergir sem re-vetorizar uma à outra. Os prefixos `P:`/`R:` entram pelo VALOR dos slots, como nas FAQs — se o gate P5 os remover lá, decidir aqui na mesma ocasião. |
| **D-LEG-8** | Chat: **novo tipo de consulta `"4"` = Legislação** (não fundir no tipo 1) | O tipo 1 tem prompt de atendimento/serviços; pergunta de lei pede prompt que instrui a citar o dispositivo e o link e a não extrapolar o texto. Seletor gateado pelo registry como o TARF: entidade sem a coleção não vê a opção. |
| **D-LEG-9** | Prompt: campo opcional `prompt_legislacao` no registry + default global embutido (mesmo mecanismo do `prompt_tarf`); criar `config/prompts/rs-legislacao.txt` | Segue o padrão existente em `entities.rs`. Atenção: `prompt_de` tem `debug_assert!(kind.exige_resumo())` — generalizar o assert (o método passa a servir jurisprudência E legislação). |
| **D-LEG-10** | Retrieval: `corpus::LEGISLACAO { kind: "legislacao", n_results: 10 }`; bandas ∞/piso 0 como todas (`LEG_FLOOR`/`LEG_BAND` no `rag.rs`) | Coerente com a doutrina vigente ("bandas TODAS ∞"; a calibragem está adiada e acumula base no log). |
| **D-LEG-11** | Índice do frontend: `auli-collections <id> indice legislacao` → `data/<id>/raw/<id>-legislacao-index.json`, com **shape novo e nomes limpos** — `[{titulo, trilha, ementa, link}]` | Diferente do TARF, não há contrato legado a preservar. O `build-frontend-public.sh` já copia `raw/*.json`; nada a mudar lá. |
| **D-LEG-11a** | O `corpo` **entra** no índice: `[{titulo, trilha, ementa, corpo, link}]`, e a linha da aba abre para lê-lo (markdown, com realce da busca). O `corpo` também entra no haystack da busca | Emenda posterior ao D-LEG-11, que o deixava de fora "porque a aba é índice, não leitor". A premissa era herdada das coleções de jurisprudência, e lá ela é certa: o corpo é a íntegra de um acórdão — foram 176 MB servidos em `public/` uma vez. Aqui o corpo é a RESPOSTA de um par P/R. **Medido**: 187 corpos somam 73 KB, média de 398 caracteres, maior 1.036; o índice vai de 75 KB para 154 KB. Ao custo de 79 KB, a aba deixa de mandar o usuário ao portal para ler duas linhas que já estavam prontas — e sem requisição por linha aberta, porque não há árvore publicada em `public/`. O `corpo` na busca segue o `buildPareceresIndex`, que já indexa a sinopse exibida: texto visível que o filtro não acha é armadilha — o termo aparece marcado dentro de uma linha que a busca não devolveu. |
| **D-LEG-12** | Ordenação do índice: **por lei** (1º segmento da trilha, ordem alfabética) e, dentro da lei, **por dispositivo** — chave numérica extraída da `ementa` ("Art. {N}" → N; sem chave → fim do grupo, sem número inventado) | A ordem natural de navegação de uma lei é a do texto. Ordenar pela trilha quebraria em algarismos romanos ("Título IX" > "Título X" lexicográfico); o número do artigo é monotônico no texto legal. Mesmo espírito da `chave_cronologica` do TARF: parse conservador, `None` vai para o fim. |
| **D-LEG-12a** | A chave desce até a alínea: `(artigo, sufixo, parágrafo, inciso, alínea)`. **Ausência de subdispositivo, ou a palavra `caput`, vale o menor valor.** Corte por vírgula/ponto-e-vírgula, nunca por espaço; o primeiro nível escrito manda e o resto do segmento é ignorado | Emenda da 2ª entrega (CF art. 155). Com chave só de artigo, os 41 pares da CF — todos do mesmo artigo — empatavam em bloco e caíam na ordem de caminho, que depois do nome canônico é alfabética pela pergunta: o grupo abria em `§ 2º, IX, a` e o `caput` vinha em 15º. Mesma filosofia do `Arts.` plural: ler o que a ementa já diz, chavear pelo primeiro subdispositivo escrito, não inventar. O corte por pontuação não é estilo — o acervo tem `Art. 138, II, e` (o `e` é ALÍNEA) e `Art. 155, III e § 6º` (o `e` é CONJUNÇÃO), e só a vírgula os distingue. **Empate residual é aceito**: as entradas de nível de caput empatam entre si, inclusive a sentinela de escopo, e desempatá-las exigiria ordenar pelo parêntese de texto livre — frágil. Ganho colateral: fecha a pendência dos parágrafos dentro do artigo na 6.537 (`Art. 17 → § 1º → §§ 3º e 4º → § 5º → 17-A`), que a #149 deixou anotada. |
| **D-LEG-13** | Aba nova "Legislação" no grupo Acervo, **componente próprio** (`LegislacaoList`), com agrupamento em acordeão por lei e busca client-side compartilhada (acentos + multi-termo E) sobre `titulo+trilha+ementa` | Não reusar `JurisprudenciaList`: o shape e o agrupamento diferem (lá é lista cronológica plana com `resumo`; aqui é hierarquia por lei sem resumo). |
| **D-LEG-14** | MCP fora da v1 — **revogada em 20/08/2026 pelo D-LEG-14a** | `buscar_pareceres`/`obter_parecer` são semanticamente de jurisprudência; expor legislação por ali confundiria o contrato. A decisão de adiar valia enquanto a coleção era nova e pequena; com 509 pares e sete normas, o custo virou o inverso — um assistente externo enxergava a interpretação da norma e o atendimento, mas não a norma. |
| **D-LEG-14a** | A legislação ganha ferramenta MCP **própria** (`buscar_legislacao`), não um valor novo de `colecao` | A alternativa era estender o `colecao` das duas ferramentas de jurisprudência, e a estrutura recusa: elas devolvem `ParecerHit` = `{numero, assunto, resumo, link}`, e um par de legislação **não tem número** (a identidade dele é a pergunta) nem **resumo** (D-LEG-11 — a coleção não promete `## resumo`). Três dos quatro campos sairiam vazios, e o `obter_parecer`, que busca pelo número exato, ficaria sem chave. O `search_jurisprudencia` inclusive carrega `debug_assert!(kind.exige_resumo())`. A legislação segue o caminho dos serviços/FAQs, que já existe e serve: payload `DocumentoPack` → `rag::blocos` → bloco de contexto, com as MESMAS constantes do tipo 4 do chat (`LEG_FLOOR`/`LEG_BAND`/`n_results`) — calibrar a banda move as duas faces junto. O `KINDS_LISTADOS` do `listar_entidades` foi de 4 para 5 kinds, na ordem que conta a história ao assistente: quem interpreta a norma, a norma, o atendimento. Kind zerado já era omitido da linha, então as 26 UFs sem legislação não mudam. |
| **D-LEG-15** | Sem bump de `STRATEGY_VERSION` nem de `PACK_FORMAT` | Nenhuma fórmula existente muda; a coleção é nova (pack novo). O `docs_hash` do rs muda porque a árvore cresce — o remédio é o de sempre: `build-packs.sh rs` recarimba. **Já divergiu na Fase 0**: manifesto `54891358d9fe1b3f`, disco `325e104d359f044f` (antes de mover o texto vigente para `ref/`). Até o `build-packs.sh rs` da Fase 2, **o boot do rs RECUSA** — é a guarda funcionando, não regressão. |
| **D-LEG-16** | O texto integral da lei fica em `data/<id>/ref/<id>-{nome}-texto-vigente.md`, **fora** de `docs/` | Achado da Fase 0. A árvore `docs/` é doutrina "um .md = uma unidade vetorizável": o `arquivos_md` filtra só por extensão, então o texto compilado dentro da subpasta quebraria o `parse_doc` assim que a leitura virasse recursiva. Mantê-lo como documento seria pior que quebrar — sem pergunta, a key fica degenerada, o teto de 6.000 corta o resto, e um hit despejaria 141 KB no prompt pela leitura tardia do `bloco()`. É insumo de origem, papel do `ref/`, onde já vivem os `portal-*.txt`. O `copy_ref` do `build-frontend-public.sh` pula o sufixo `-texto-vigente.md` pelo mesmo motivo do `portal-pareceres.txt`: KB mortos em cada deploy. **Dívida cosmética registrada na 2ª entrega**: o sufixo marca a família por NOME, e o texto da CF é deliberadamente congelado antes da EC 132 — chamá-lo de `rs-cf-art-155-texto-vigente.md` afirma vigência sobre um texto revogado. A leitura defensável é "vigente no corte do acervo", que é o que o par sentinela explica ao usuário. Se um dia incomodar, a saída limpa é o `copy_ref` pular por DIRETÓRIO (`ref/fontes/`, que o `find -maxdepth 1 -type f` já não visita) em vez de por família de nome — aí o arquivo pode se chamar `rs-cf-art-155-redacao-pre-ec132.md`, que é o que ele é. Não é urgência. |
| **D-LEG-17** | Para **lei estadual do RS**, o ref sai da página da SEFAZ (`legislacao.sefaz.rs.gov.br/Site/DocumentView.aspx?inpKey=N`), não de compilação de terceiro. Onde as duas divergirem, **prevalece a SEFAZ**, inclusive contra o texto que originou os pares | Decisão de 20/08/2026, tomada com os dois textos do IPVA na mesa. Não é preferência por autoridade: é que essa página **elimina a etapa de leitura**. A compilação da AL-RS marca a redação superada com tachado, que a camada de texto do PDF não preserva — separar vigente de revogado exigiu olhar as IMAGENS das páginas. A da SEFAZ serve só o vigente, num `<div class="struct <tipo>_container">` por dispositivo, com o tipo na classe, o rótulo no `identificador` e a atribuição de alteração no `reference`: extração determinística, sem heurística e sem olho humano. **Medido na troca**: dos 112 dispositivos da Lei 8.115, 110 conferiam palavra a palavra; os dois que não eram erros do lado do PDF — o art. 1º na redação de 1985 (sem a sigla, com a Lei 8.494/87 rebaixada a "Vide") e o art. 4º, § 7º parafraseado. E a SEFAZ trouxe o que o PDF não tinha: a inconstitucionalidade da parte final do art. 12 (Representação 1.342-1/RS, STF, 17/12/1986). Vale também que é a MESMA página que os `link` dos pares apontam — o usuário que clica lê o texto de onde o ref saiu. **Ressalva de robots**: o host declara `Disallow: /`, e o acesso se apoia na autorização da própria SEFAZ-RS que o `auli-scraper-rs` documenta, com UA institucional `AuliBot`. Sem isso, a decisão não vale. |
| **D-LEG-17a** | Divergência entre o ref e um par **não** se resolve reescrevendo o par junto com o ref: o ref é trocado, a divergência fica anotada, e o par volta para revisão de conteúdo | Emenda imediata do D-LEG-17, do mesmo dia. Trocar a fonte é operação determinística e verificável; reescrever um par é autoria. Misturar as duas num movimento só faria a correção de conteúdo entrar sem revisão, escondida atrás de um commit de "atualização de fonte". Caso corrente: o par da repartição da receita do IPVA descreve como vigente a parte do art. 12 que o STF derrubou. |

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
  **gerado** de `config/registry.toml` por [scripts/tools/gen-frontend-entities.mjs](scripts/tools/gen-frontend-entities.mjs)
  (guardado por `scripts/tools/check-registry-sync.sh`), com **nove** entidades:
  - `rs` = SEFAZ-RS, coleções `["servicos","faqs","pareceres","notas","conteudos"]`.
  - `sc`/`sp`/`pr`/`mg`/`pe`/`ba`/`rj`/`ce` — coleções `["servicos"]`.
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
  pasta `public/` específica; a seleção de entidade (as 12) muda apenas qual `public/<id>/` é
  consultado. Não há, no código, busca de uma lista de entidades vinda do backend — a lista vem de
  [shared/entities.ts](auli-frontend/src/shared/entities.ts), **gerado** de `config/registry.toml`
  (não editado à mão).
- O frontend **não** consome rotas de gestão de dados do servidor; o único endpoint de backend
  efetivamente chamado é `POST /v1/question` (via `VITE_API_URL`). Não há, no código do
  frontend, uso de login/JWT. **NÃO CONFIRMADO NO CÓDIGO:** qualquer fluxo de autenticação no cliente.

---

## 5. Coleta: scrapers por entidade + `auli-collections` (derivação)

A coleta é **um binário por entidade** (`auli-scraper-<e>`) sobre o **`auli-scraper-kit`**; o
`auli-collections` virou **só derivação** (offline). A fronteira entre os dois é o **snapshot v3**.

```text
auli-scraper-<e> (rede)  →  data/<id>/<id>-<kind>-snapshot.json (v3)  →  auli-collections <e> (offline)  →  data/<id>/raw/
```

### 5.1 O kit compartilhado (`auli-scraper-kit`)

[crates/scrapers/auli-scraper-kit](auli-server/crates/scrapers/auli-scraper-kit): o **como raspar** —
`cache::{read, write, read_or_bail}` (cache de página por URL; arquivo vazio conta como _miss_;
`read_or_bail` encapsula o miss-vira-erro do `--usecache`), `http::{get_string, post_json}` (GET/POST
com retry 3× + backoff ×2, via `GetOpts { log_prefix, accept, headers, … }`), `USER_AGENT` (identidade
de rede padrão da frota), `clean`/`clean_decoded`/`decode_entities` (normalização de texto),
`build_agent(user_agent, timeout)` (agente `ureq`), e `aggregate_servicos` + `descricao_body` (dobra
registros per-público em `ServicoRaw` **deduplicando por `link`**). Essas funções comuns foram
**extraídas das cópias por-entidade** (fetch/UA/clean/scraper_info, ~500 linhas duplicadas); as
variantes genuinamente próprias ficam locais com comentário (charset do ba, page-API `Value` do mg,
`clean_text` line-based, cache do rs-faqs). O `ScraperInfo::new(nome, versao)` (no contrato) substitui
o `fn scraper_info()` boilerplate. O **I/O do snapshot** (`load`/`write_faqs`/`write_servicos`) e o
shape per-público `ServicoPerPublico` **saíram do kit para o `auli-contract`** (D-C1) — a fronteira
mora no contrato. Sem `fastembed`/`ort` — os scrapers compilam leves; nada fora de `scrapers/`
depende do kit.

### 5.2 O snapshot v3 (`auli_contract::snapshot`)

Cada scraper grava um snapshot por coleção `data/<id>/<id>-<kind>-snapshot.json` (`SNAPSHOT_SCHEMA_VERSION = 3`): metadados do
scraper + a coleta daquela coleção (sem merge entre coleções). Serviços são `ServicoRaw { titulo, descricao, link, orgao, ocorrencias:
Vec<Ocorrencia> }` com `Ocorrencia { publico, classe }` — um serviço em N públicos/classes é **N
ocorrências nativas** (resolveu o limite multi-classe do modelo antigo). FAQs são `Vec<FaqRaw>`
(pergunta/resposta/origin/url), lista **achatada**.

### 5.3 Os scrapers (um por entidade)

| Crate | Entidade | Plataforma / técnica | CLI |
| ----- | -------- | -------------------- | --- |
| [auli-scraper-rs](auli-server/crates/scrapers/auli-scraper-rs) | SEFAZ-RS | FAQs (portal CMS via **AJAX/`ureq`**) + serviços (5 públicos via **API JSON `tudofacil`**); sem browser | `[--usecache] faqs\|servicos\|all` |
| [auli-scraper-sc](auli-server/crates/scrapers/auli-scraper-sc) | SEF-SC | serviços via **API JSON Next.js** (sem browser) | `[--usecache] servicos` |
| [auli-scraper-sp](auli-server/crates/scrapers/auli-scraper-sp) | SEFAZ-SP | serviços via **REST SharePoint** (JSON anônimo: listas `Serviços` + `Homes 360`) | `[--usecache] servicos` |
| [auli-scraper-pr](auli-server/crates/scrapers/auli-scraper-pr) | SEFA-PR | serviços via **HTML Drupal** server-side (mega-menu "Serviços para você!") | `[--usecache] servicos` |
| [auli-scraper-mg](auli-server/crates/scrapers/auli-scraper-mg) | SEF-MG | serviços via **API JSON ServiceNow CSM** (page API, sem browser) | `[--usecache] servicos` |
| [auli-scraper-pe](auli-server/crates/scrapers/auli-scraper-pe) | SEFAZ-PE | serviços via **HTML SharePoint 2013** (menu `#menu_servicos`, 1 GET) | `[--usecache] servicos` |
| [auli-scraper-ba](auli-server/crates/scrapers/auli-scraper-ba) | SEFAZ-BA | serviços via **HTML ASP clássico** (listagem + fichas; **native-tls**/OpenSSL) | `[--usecache] servicos` |
| [auli-scraper-rj](auli-server/crates/scrapers/auli-scraper-rj) | SEFAZ-RJ | serviços via **HTML WordPress** (1 página, 1 GET) | `[--usecache] servicos` |
| [auli-scraper-ce](auli-server/crates/scrapers/auli-scraper-ce) | SEFAZ-CE | serviços via **API JSON Sydle ONE** (`getChildren` POST, token anônimo) | `[--usecache] servicos` |

- **RS** é o único com FAQs. **Nenhum scraper usa headless Chrome** — todos são ureq + HTML/JSON (o
  RS migrou para a API JSON `tudofacil`; o **BA** usa `native-tls`/OpenSSL pelo TLS 1.2-CBC do
  portal). A **árvore** de FAQ
  (`FaqNode { title, url, page_type, origin, children, faq_items }`) é serializada em
  `faqs-tree.json` para a aba de FAQs do frontend ([faqs/mod.rs](auli-server/crates/scrapers/auli-scraper-rs/src/faqs/mod.rs));
  o snapshot leva a lista achatada.
- **SC/PR/MG/PE/BA** dobram os registros per-público via `aggregate_servicos` (**dedup por `link`**).
  **RS/SP/RJ/CE montam os `ServicoRaw` direto** (sem aggregate) porque o `link` não é a identidade:
  no SP a URL de login é compartilhada por vários serviços; no RJ a identidade é `(link, título)`;
  no CE é o `_id` do documento (o slug repete) ([sp/scrape.rs](auli-server/crates/scrapers/auli-scraper-sp/src/scrape.rs)).
- **Cache + `--usecache`:** cada scraper cacheia páginas por URL (`data/<id>/raw/cache/…`, ignorado
  pelo git); `--usecache` reprocessa offline e torna um _miss_ um erro.

### 5.4 A derivação (`auli-collections <e>`)

[crates/auli-collections](auli-server/crates/auli-collections) (`main`, `process`, `derive_faqs`,
`servicos/{mod,types}`, `domain/entities`): **não raspa** — lê o snapshot e produz, offline, em
`data/<id>/raw/`:

- as **árvores `.md`** `data/<id>/docs/{servicos,faqs}/*.md` — **a fonte que o `auli update` lê**
  (ele recompõe o `text_to_embed` na leitura, pelo ponto único do contrato);
- o **contrato** `<id>-faqs.json` / `<id>-servicos.json` (`auli_contract::Table<P>`), hoje artefato
  de auditoria e de borda — o update NÃO o lê mais desde a G5b;
- os **prints** `portal-<kind>.txt` (auditoria, nunca relidos);
- o **`servicos-index.json`** (manifesto de abas) + os JSONs **per-público** (`<slug>.json`).

`text_to_embed` — as três fórmulas vivem no contrato (`compose_*`), e todas são adaptadores da MESMA
geometria (`trilha → titulo → ementa → resumo`, pulando os vazios): faqs → breadcrumb + `P:` pergunta +
`R:` resposta (desde a TAREFA-FAQ-PR; antes era cega para a resposta); serviços → `tipo | classe` +
título + 300 chars do corpo; jurisprudência → número + ementa + resumo. Mudar qualquer uma
re-vetoriza os packs daquela coleção. A tentativa de raspar por aqui é rejeitada com erro explícito
("a coleta agora é feita pelos binários `auli-scraper-*`").

`pareceres` **tem scraper dedicado** em rs/sc/sp/pr, e `tarf` no rs — os dois emitem `.md` direto na
árvore, sem passar pelo `auli-collections`. Só `notas` segue autorada e ausente dos packs, por não ter
fonte struct.

### 5.5 Cobertura por entidade

**27 entidades com serviços**, cada uma com plataforma e armadilhas próprias (API JSON, Next.js,
SharePoint, WordPress, ServiceNow, ColdFusion, catálogo hardcoded em bundle JS…). A tabela por
entidade **não é duplicada aqui**: ela vive em
[SCRAPERS.md](auli-server/crates/scrapers/SCRAPERS.md), ao lado dos crates, e cada linha aponta a
âncora do relatório de descoberta em [`docs/REGISTRO-scrapers.md`](docs/REGISTRO-scrapers.md).

Além de serviços: **FAQs** só no `rs`; **pareceres** em `rs`/`sc`/`sp`/`pr`; **tarf** só no `rs`.
`notas` e `conteudos` seguem autorados, sem scraper.

---

## 6. Divergências entre componentes — resolvidas pela unificação sob `data/`

> **RESOLVIDO — integração `data/` (Fases 1–4 da unificação).** As divergências históricas entre
> os domínios duplicados foram **eliminadas** pela fonte única `data/`.

1. **Triplicação do `domain` resolvida.** `config/registry.toml` é a fonte única de entidades, lida
   por `auli-cli` e `auli-collections`; o frontend gera `entities.ts` dela. O kind vetorial foi
   **unificado para `servicos` ponta a ponta** pela auditoria (PR #4) — não há mais o descasamento
   `services`↔`servicos` (o antigo `services` sobrou só como guarda de regressão — o teste que exige `from_kind("services").is_err()`) nem
   cópias divergentes de `domain`/`errors`/`entities`.
2. **Dados de serviços consistentes.** Packs e frontend vêm da **mesma** raspagem (contrato
   `auli-contract`); o engine não declara mais `delimiter`/`EmbedStrategy` próprios para serviços.
3. **As 27 entidades são reais** do server (serviços rs 586, sc 208, sp 537, pr 141, mg 148, pe 38,
   ba 204, rj 91, ce 382, ms 276, mt 27, go 94, pi 29, am 278, pa 34, es 45, ro 194, to 45, ma 38, ap 49, ac 17, df 472, rn 15, pb 101, al 60, se 91, rr 16; FAQs 1937 no RS), não só do frontend.
4. **`pareceres`/`notas`/`conteudos`** (autorados, sem scraper) ficam versionados em `data/<id>/ref/`,
   exibidos no frontend e (pareceres/notas) ingeríveis nos packs quando modelados como `Table<P>`;
   ainda **não** são consultados no RAG ativo.

Resíduo: dentro do backend o domínio é fonte única (`corpus`/`vector-store`); o `auli-frontend`
mantém um espelho **gerado** (não mais divergente) do registro. Pendências em
[docs/auli_pendencias.md](docs/auli_pendencias.md).

---

## 7. Resumo: implementado e ativo vs modelado/inativo

**Ativo e funcionando (confirmado no código):**

- `auli server` (workspace `auli-server`): `GET /v1/health`, `POST /v1/question` (RAG completo
  **in-process**: fastembed/BGE-M3 → vector store próprio → LLM externo, com log em `$AULI_LOG_DIR`,
  na raiz do repo) e `GET /v1/{kind}/list` (leitura). Escuta configurável (`--bind`, default `0.0.0.0`).
  Público, **sem auth nem banco**; CORS; configuração por `.env` (`config()`); logging via `tracing`.
  Vetorização separada pelo `auli update`.
- **Doze estados ativos** (rs/sc/sp/pr/mg/pe/ba/rj/ce/ms/mt/go). RAG consulta efetivamente apenas `servicos` (10) +
  `faqs` (20); estreitamento por proximidade presente mas em modo paridade (`band=∞`) até calibração.
- `auli-frontend`: SPA com seleção de entidade (as 12), chat contra `POST /v1/question` com
  timeout de 25s, abas de referência lendo `public/<id>/` (arquivos prefixados `<id>-`), tema
  claro/escuro, testes Vitest.
- **Scrapers por entidade** (`auli-scraper-{rs,sc,sp,pr,mg,pe,ba,rj,ce,ms,mt,go}` sobre `auli-scraper-kit`): FAQs (rs) e
  serviços (rs API JSON, sc JSON, sp SharePoint, pr Drupal, mg ServiceNow, pe/ba/rj/ms HTML, ce/mt/go JSON — go via curl por WAF JA3), cache + `--usecache`,
  gravando o snapshot v3; `auli-collections <e>` deriva o contrato + `portal-*.txt` +
  `servicos-index.json` + per-público.

**Modelado/declarado mas inativo ou incompleto:**

- Server: `pareceres`/`notas` listáveis mas **não consultados** no RAG ativo.
- Coleta: sem scraper de `pareceres`/`notas` (autorados); FAQs só no RS. Os binários de scraper
  usam `anyhow` (idiomático p/ bin); a derivação em `auli-collections` já usa o
  `crate::errors` tipado.
- Frontend: sem fluxo de autenticação/JWT no cliente; multi-tenant apenas por troca de
  `public/<id>/` (lista de entidades **gerada** do registry).

---

## 8. Itens marcados como NÃO CONFIRMADO NO CÓDIGO

- Fluxo de autenticação no frontend: inexistente no código (o backend atual também não tem auth).
- Reranking: não há reranking no caminho RAG ativo; o único estreitamento de resultados é o
  `select_by_proximity` (por distância cosseno), hoje em modo paridade (`band=∞`).
