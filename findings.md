# findings.md — Revisão de consistência do monorepo `oxschellen/auli`

- **Commit revisado:** `efdb97a` (branch `main`, inclui `--bind`, fix de clippy e a reescrita do `auli_code.md`)
- **Método:** leitura estática integral do Rust (61 arquivos, ~6.2k linhas), scripts, `registry.toml`, docs (`README`, `auli_code/features/operations/pendencias.md`) e dos pontos de contato do frontend (fetchers, entities, listas, chat). Sem compilação/execução (ver §6).
- **Escopo:** inconsistências código↔código, código↔documentação, doc↔doc, e hazards de comportamento decorrentes delas.

---

## 1. Achados de maior impacto (comportamento real ou risco concreto)

### 1.1 Colisão de caminho: arquivos per-tipo do scraper RS × per-público do `process`
O scraper RS grava seus arquivos de **recuperação incremental** em `data/rs/raw/<filename>.json` (`extrair_descricoes.rs`, via `save_servicos_to_json` a cada serviço), com `descricao` **contendo o header de 3 linhas** e uma entrada por link listado. O `auli-collections process` grava os JSONs **per-público derivados** no **mesmo caminho** (`servicos/mod.rs`, passo 4: `{data_dir}/{slug}.json`, e os slugs do RS são idênticos aos filenames), porém com semântica diferente: uma entrada por `(link, classe)`, ids renumerados e `descricao` = **corpo limpo, sem header**. Consequências: (a) o `process` sobrescreve os arquivos de recuperação do scraper; (b) a ordem dos passos vira contrato implícito — se `scripts/build-frontend-public.sh` rodar entre o scrape e o process, o `public/` publica descrições com header e **sem** o fan-out multi-classe; (c) o mesmo nome de arquivo carrega dois formatos ao longo do pipeline. O doc do kit agrava: `auli-scraper-kit/src/servico.rs:10-12` afirma que os per-público que o `process` grava "carregam o header" — o `process` grava exatamente o contrário. Sugestão: mover a recuperação do scraper para um caminho próprio (ex.: `raw/scrape/<slug>.json`) ou sufixá-la, e corrigir o doc do `servico.rs`.

### 1.2 A checagem amigável de `schema_version` é inalcançável para mudanças estruturais
`auli-scraper-kit/src/snapshot.rs` (`load`, l.88, e `load_colecoes` no merge) desserializa o `Snapshot` **inteiro com os tipos v2** antes de qualquer verificação. Um snapshot v1 em disco (com `classe`/`publicos` em vez de `ocorrencias`) falha com **erro cru do serde** tanto no merge dos scrapers quanto no `process` — a mensagem amigável de `process.rs:22` ("Re-raspe a entidade — não há migração") nunca executa, pois só roda **depois** de a desserialização ter sucesso. Ou seja, a checagem só cobre bumps de versão estruturalmente compatíveis, que é o caso onde ela menos importa. Sugestão: ler primeiro um header mínimo (`struct { schema_version, entidade }` ou `serde_json::Value`) e só então desserializar tipado.

### 1.3 `Writer::upsert` trunca silenciosamente com aridades divergentes
`vector-store/src/write.rs:62` faz `ids.iter().zip(embeddings).zip(payloads.iter())`: se os três vetores tiverem comprimentos diferentes, grava só o mínimo e retorna `Ok` — perda silenciosa num crate que se apresenta como store genérico com "loud write-time errors" (a checagem de dimensão existe; a de aridade não). Hoje o único chamador (`auli-cli/src/update.rs::ingest`) deriva os três do mesmo `table.items`, então não dispara — mas é contrato implícito não verificado. Sugestão: erro explícito quando `ids.len() != embeddings.len() || ids.len() != payloads.len()`.

### 1.4 Rate limiter (e log de IP) confiam em headers spoofáveis
`api/ratelimit.rs::client_ip` aceita, em ordem, `CF-Connecting-IP`, …, `X-Forwarded-For` (l.61, o próprio comentário diz "can be spoofed") de **qualquer** requisição. Fora do túnel Cloudflare — ou em acesso direto à porta, que o novo `--bind 0.0.0.0` mantém como default — o cliente controla a chave do limiter: bypass trivial do 1 req/s **e** crescimento sem GC do mapa keyed (o comentário do módulo dimensiona memória para "hundreds of organization IPs", mas as chaves são controláveis pelo atacante). O mesmo `client_ip` alimenta o log do `question_handler`. Sugestão: atrás do túnel, confiar apenas em `CF-Connecting-IP`; sem ele, cair direto no peer do socket.

---

## 2. Documentação × código

### 2.1 `README.md` (não atualizado nesta rodada) contradiz o pipeline atual
(a) Descreve `auli-collections` como "Scrapers" e dá o exemplo `cargo run -p auli-collections -- [--usecache] <entity> <collection>` — hoje isso **erra em runtime**: o `main.rs` do collections rejeita qualquer flag `--` e rejeita `faqs|servicos` com "a coleta agora é feita pelos binários `auli-scraper-*`". (b) "FAQs — SEFAZ-RS portal (**headless Chrome** + ureq)" — o scraper de FAQs é ureq/AJAX puro; Chrome só existe em `servicos/extrair_descricoes.rs`. (c) "Services — RS … and SC" omite **SP e PR**, que existem como crates, estão no registry e têm dados em `public/`. (d) A tabela de layout não menciona `auli-scraper-kit` nem os binários por entidade. Como o README é a porta de entrada e aponta os `auli_*.md` como detalhe, ele é hoje o doc mais desalinhado do repositório.

### 2.2 `auli_code.md`: FAQs do RS marcados como Chrome
A tabela §5.3 diz "FAQs (portal CMS via **headless Chrome** + `ureq`)" e a §5.5 (l.444) marca a célula de FAQs do RS como `✅ (Chrome)`. No código, `auli-scraper-rs/src/faqs/` não importa `headless_chrome` em nenhum módulo. O `auli_operations.md` (l.58-60/79) acerta no nível de crate ("só o `-rs` puxa headless Chrome"), mas a atribuição por coleção no code.md está errada. (De resto, a reescrita do `auli_code.md` neste commit eliminou quase todo o drift anterior — `services`→`servicos`, 4 entidades, `faqs-tree.json`, prefixo `<id>-` no `entityPath`, `$AULI_LOG_DIR` — bom trabalho.)

### 2.3 `auli_code.md` §6.1 (l.461): precisão sobre a "guarda de regressão"
"o antigo `services` sobrou só como guarda de regressão **em `from_kind`**" — em `from_kind` (corpus.rs) não há braço `services`; a guarda é o `assert!(from_kind("services").is_err())` no **teste** do módulo. Nit, mas o doc se propõe "code-audited".

### 2.4 `data/registry.toml` (l.15): comentário fóssil na fonte-da-verdade
"`servicos` mapeia ao kind vetorial `services`" — contradiz `corpus.rs` (kind único `servicos`; `services` é rejeitado) e o próprio `auli_pendencias.md` §5, que declara a unificação resolvida. Por estar no arquivo que todo mundo edita, é o fóssil com maior chance de reintroduzir confusão.

### 2.5 Comentários fósseis intra-código (cada um contradiz o próprio arquivo ou o vizinho)
- `auli-scraper-sc/src/sc.rs:9` — passo 5 do cabeçalho: "write one per-público file … the caller aggregates" — contradito pelo doc de `scrape` no mesmo arquivo (l.146-148: "SC no longer writes per-público files — the fan-out is now `process`'s job").
- `auli-scraper-sc/src/sc.rs:232` — `build_descricao` referencia `gerar_portal_servicos::descricao_body`; esse módulo não existe mais (hoje é `auli_scraper_kit::descricao_body`).
- `auli-scraper-rs/src/faqs/faq.rs:4` — "the tree itself is **not persisted anymore**" e, no doc do `FaqNode`, "the root node is what gets written to `<collection>.json`" — `faqs::run` (mod.rs) grava `faqs-tree.json` explicitamente, e o doc do próprio mod.rs diz isso.
- `vector-store/src/lib.rs` (doc de `scan`) — "Shared by ReadStore and the **Phase-1 server registry**" — não existe mais tal registry; o único consumidor é o `ReadStore`.
- `auli-scraper-kit/src/cache.rs:4-5` — "expensive **headless-Chrome** renders" num kit compartilhado por SC/SP/PR, que não usam Chrome; comentário RS-específico em código genérico.
- `auli-cli/src/config.rs:5` e `update.rs:14` — "LLM/**JWT/DB** vars" — JWT/DB não existem em nenhum `Config` atual; o resquício histórico sugere dependências que nunca serão "forçadas".

### 2.6 Amostras de teste contradizem o doc do contrato (slug com prefixo)
`auli-contract/src/snapshot.rs:90` documenta `Publico::slug` **sem** prefixo de entidade (ex.: `"servicos-ao-cidadao"`), e todos os quatro scrapers reais gravam sem prefixo. Mas os samples de teste do próprio contrato (`snapshot.rs:161`) e do kit (`kit/snapshot.rs:138`) usam `"rs-servicos-ao-cidadao"`. Nada quebra (o campo é opaco), porém os testes ensinam exatamente a convenção do `public/` (onde o prefixo é adicionado pelo `build-frontend-public.sh`), que é o lugar errado para ela existir.

### 2.7 Doc do contrato × exceção do SP na unicidade do `link`
`auli-contract/src/snapshot.rs:120` afirma incondicionalmente: "URL do serviço — **a chave natural única** do snapshot". O SP viola isso por decisão documentada (vários serviços compartilham a URL de login; o scraper monta `ServicoRaw` direto, sem o `aggregate_servicos` que deduplica por link — `sp/main.rs` e `auli_code.md` §5.3 explicam corretamente). O contrato, que é "o único ponto onde produtor e consumidor concordam", é hoje o único lugar que ainda promete a unicidade.

---

## 3. Robustez e consistência de comportamento (menores)

### 3.1 Cache-first mesmo sem `--usecache` → "re-raspar" não atualiza nada
Todos os fetchers (kit `cache::read` incondicional; `faqs/fetch.rs` idem via `cache_path.exists()`) leem o cache **antes** de decidir ir à rede; `--usecache` só transforma o miss em erro. Logo, rodar o scraper de novo nunca refaz páginas já cacheadas sem apagar `data/<id>/raw/cache/` à mão — no SC isso inclui a **listagem paginada e o buildId** (URL lógica), então serviços novos do portal não aparecem. Não há flag `--refresh` nem instrução de limpeza no runbook (`auli_operations.md`), e o print "usando apenas páginas em cache" só no modo offline sugere, por omissão, que o modo normal busca a rede.

### 3.2 `.gitignore`: padrão de cache não casa com o caminho real
`**/data/*/cache/` e `data/*/cache/` (l.44/58) miram `data/<id>/cache/`, mas o cache real vive em `data/<id>/raw/cache/…`. Hoje nada vaza apenas porque `data/*/raw/` inteiro é ignorado; se `raw/` um dia for parcialmente versionado, o cache entra junto. O comentário "Scraper cache" ficou órfão do caminho que descreve.

### 3.3 `process` com snapshot ausente sugere um comando que não existe
`auli-collections/src/process.rs:17` interpola `entity.id` como se fosse o binário: a mensagem sai "rode `rs faqs` e/ou `rs servicos`". Deveria apontar `auli-scraper-rs faqs` etc. (ou o padrão `auli-scraper-<id>`).

### 3.4 `llm.rs`: três arestas
(a) `println!("LLM_API_MODEL: …")` a cada request (l.16) duplica o `log_summary` do boot e fura o `tracing`. (b) A retentativa cobre só `is_connect()/is_timeout()` do `send`; um erro na leitura do corpo (`resp.text()`) não retenta, e um 4xx/5xx com corpo não-JSON vira `SerdeJson` cru exposto como `answer`. (c) `Client::new()` sem timeout: um LLM pendurado segura o handler indefinidamente, enquanto o frontend desiste em 25 s (`callServerAPI.ts`) — o usuário vê timeout e o servidor continua gastando a chamada.

### 3.5 `SystemTime::elapsed().unwrap()` em caminho de request
`api/handlers/question.rs:36` e `collections.rs:67` — pânico possível em ajuste de relógio (NTP para trás). `Instant` elimina o caso.

### 3.6 Manifesto carimba `dim: EMBED_DIM` sem conferir os vetores
`update.rs` grava `dim = 1024` pela constante, sem checar `embeddings[0].len()`. O `DimensionMismatch` do store só garante consistência **interna** da coleção, e o boot valida a identidade do manifesto, não a largura real do arquivo — se o modelo mudar de largura sem bump do `EMBED_MODEL_ID`, o manifesto mente e nada grita.

### 3.7 "Entidade padrão" com duas fontes divergentes
Server e collections: `DEFAULT_ENTITY = "rs"` hardcoded. Frontend: `DEFAULT_ENTITY_ID = entities[0].id`, derivado da **ordem** do `registry.toml` (gen-frontend-entities.mjs). Reordenar o registry muda o default do frontend e não o do backend — exatamente o tipo de duplicação que o registry-único quis eliminar.

### 3.8 Fallback de abas do frontend ≠ ordem real do RS
`getDefaultTipoServicos()` (servicoslist/utils.ts) começa por "Empresas"; o scraper RS e o `servicos-index.json` começam por "Cidadãos". Só afeta deploys sem index, mas é uma segunda cópia da lista de públicos, com ordem divergente.

### 3.9 SP: `link` vazio possível, sem aviso
`sp/scrape.rs:148` — `URL` ausente vira `""` via `unwrap_or_default` + `canonical("")`; entra no contrato e no `stored_repr` como "Link: " vazio. Um `⚠️` no padrão do contador `sem_publico` fecharia o ciclo de qualidade.

### 3.10 PR: ids de painel aparentam typos do portal, sem guarda por aba
`pr/scrape.rs::publicos()` usa `servicos-tema-cidado`, `…-municpio`, `…-legislao`. Provavelmente são os ids reais (typados) do DOM do portal — mas um id errado renderia a aba com **0 ocorrências silenciosamente** (o `bail!` cobre só o container do mega-menu). Vale um aviso "aba vazia" análogo ao `orphan_check`.

---

## 4. Cosméticos / higiene

- **Banner de versão duplicado:** `lib.rs` imprime "Auli Server v0.3.0" hardcoded — segunda cópia da versão do `Cargo.toml` (0.3.0 hoje; vai divergir no próximo bump). `env!("CARGO_PKG_VERSION")` resolve.
- **`url_to_filename` duplicado** em `auli-scraper-kit/cache.rs` e `auli-scraper-rs/faqs/mod.rs` (mesma implementação; o cache de FAQs não usa o kit).
- **`DEFAULT_SYSTEM_PROMPT` duplicado** em `auli-cli/entities.rs` e `auli-collections/domain/entities.rs` (texto idêntico; no collections nem é usado pelo pipeline — o módulo já é `#![allow(dead_code)]`).
- **`auli-cli/Cargo.toml`:** `toml = "1.1.2"` agrupado sob o comentário "Rate limiting".
- **Formato de links não uniforme por kind:** FAQs RS emitem `[texto](url)`; serviços RS/SC/PR emitem `texto "url"`. O `linkify.tsx` do frontend só linkifica URLs cruas, então ambos funcionam, mas o texto exibido/enviado ao LLM não é homogêneo. Idem slugs: `servicos-a-servidores-publicos` (RS/SC) vs `servicos-a-servidores` (SP).
- **`check-registry-sync.sh`** regenera `entities.ts` antes do diff — em caso de dessincronia, deixa a árvore suja como efeito colateral do check (comportamento razoável, mas vale saber).

---

## 5. Verificado e consistente (amostra do que **não** é problema)

- A cadeia contrato→update→packs→server está coerente: `text_to_embed`/`stored_repr` materializados na origem; `strategy_version 2` + `EMBED_MODEL_ID` validados no boot; `ReadStore`/`Writer` separados de fato (o server não linka o Writer); hash FNV re-conferido no `load_all`.
- `snapshot` v2: merge preserva a coleção não raspada (testado); `aggregate_servicos` preserva multi-classe na ordem de descoberta (testado); `primary_ocorrencia` segue `publicos_ordem` (testado); exceção do SP ao aggregate é deliberada e agora documentada no code.md.
- Frontend: `entityPath` (`/<id>/<id>-<file>`) casa com `build-frontend-public.sh` (prefixo `<id>-`, exclusão dos contratos e dos `portal-*.txt`); `parseFaqs` tolera os campos omitidos pelo `skip_serializing_if` do `FaqNode`; abas gateadas por `hasCollection` não disparam fetch; 429 do limiter é lido pelo `callServerAPI` (`data.error`). `entities.ts` commitado bate com o `registry.toml` atual (4 entidades).
- Mudanças deste push (`--bind` no clap/lib/start_server, if-let colapsado no `packs.rs`) estão internamente consistentes; `vite ^8` justifica o `rolldownOptions` no config.

## 6. Limitações desta revisão

Sem toolchain Rust no ambiente (rustup bloqueado pela rede do container), **não compilei nem rodei `cargo test`** — a análise é estática. Os números de packs citados nos docs (rs 586/1937, sc 208, sp 537, pr 141) e o e2e "verificado ao vivo" do §3.9 do code.md não são verificáveis pelo repositório (packs são gerados/ignorados). Nenhum problema sintático saltou aos olhos além do listado; o código usa edition 2024 + let-chains de forma consistente.
