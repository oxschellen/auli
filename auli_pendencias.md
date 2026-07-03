# Pendências — auli-contract e integração de dados

Registro dos assuntos em aberto **após** o merge do `auli-contract` (PR #3) no `main`. O refactor
fez a **struct tipada** (`auli_contract::Table<P>`) virar a fonte única do dado: o scraper grava o
contrato em `data/<id>/raw/<id>-<kind>.json` e o `auli update` o consome.

> **Revisão 2026-07-02 (pós fases 1 e 2).** O modelo evoluiu: o **snapshot v2**
> (`data/<id>/<id>-snapshot.json`, tipos em `auli_contract::snapshot`) virou a fronteira
> scraper→collections; a coleta saiu para os binários **`auli-scraper-<e>`** e o
> **`auli-collections <e>`** só deriva os artefatos (contrato/prints/index/per-público).
>
> **Revisão 2026-07-03 (multi-estado + deploy).** Quase tudo abaixo foi **resolvido** nesta sessão.
> As **4 entidades estão no ar e raspadas ao vivo**: `rs` (serviços 586 + FAQs 1937), `sc` (208),
> `sp` (537), `pr` (141) — todas com packs `strategy_version: 2`, kind `servicos`. O server sobe
> carregando as 4, valida cada manifesto contra a identidade local, `/v1/health → OK`,
> `api.auli.com.br` roteia, e o RAG responde por estado citando os links certos (verificado ponta a
> ponta). Status item a item abaixo.

---

## 1. Verificação ponta a ponta — ✅ **feita (2026-07-03)**

O scrape ao vivo rodou para os 4 estados → snapshot v2 → `auli-collections <e>` → `build-packs.sh <e>`
→ boot. O server carregou `[pr, rs, sc, sp]`, cada manifesto validado, e o **smoke test de RAG**
respondeu por estado com o serviço/link corretos:

- **rs** — certidão negativa → Certidão de Situação Fiscal (`sefaz.rs.gov.br/sat/…`)
- **sc** — regularidade fiscal → Emitir CND (`sat.sef.sc.gov.br/…`)
- **sp** — cadastro ICMS → CADESP (`portal.fazenda.sp.gov.br/servicos/cadesp/…`)
- **pr** — serviços ao cidadão → FAQ/agendamento (`fazenda.pr.gov.br/…`)

> **Resíduo resolvido:** os packs antigos `rs-pareceres`/`rs-notas` (strategy=1) **foram removidos**
> na reconstrução — `data/rs/packs/` tem só `rs-faqs.json`, `rs-servicos.json` e o manifesto.

---

## 2. Frontend desacoplado do contrato (árvore de FAQ) — ✅ **resolvido (opção a)**

A aba de FAQs voltou a ter fonte: o scraper agora **serializa a árvore** (`faqs-tree.json`, com
`page_type`/`children`) ao lado do snapshot
([faqs/mod.rs](auli-server/crates/auli-scraper-rs/src/faqs/mod.rs)), e o frontend a busca
([FaqsList.tsx:17](auli-frontend/src/pages/faqslist/FaqsList.tsx#L17), `faqs-tree.json`). O
`build-frontend-public.sh` agora **pula os contratos do engine** (`<id>-faqs.json`/`<id>-servicos.json`,
que a UI não consome) — sem peso morto nem colisão de nome em `public/`.

---

## 3. Fórmula de `text_to_embed` de serviços — **provisória (validada ponta a ponta)**

Continua `tipo | classe` + título + 300 chars do corpo
([servicos/mod.rs](auli-server/crates/auli-collections/src/servicos/mod.rs), `servico_text_to_embed`).
Ainda **rotulada provisória no código**, mas o smoke test do item 1 mostrou recuperação boa nos 4
estados (respostas citam o serviço principal certo). Falta só a decisão de **fixar** a fórmula (ou
ajustá-la) e tirar o rótulo. FAQs seguem em `origin + pergunta` (`faq_from_raw`), estável.

---

## 4. `pareceres` / `notas` / `conteudos` — **adiados (sem fonte struct)**

Inalterado. São conteúdos **autorados** (em `data/<id>/ref/`), sem scraper — não há `Table<P>`. O
`auli update` os pula; o server sobe com a coleção vazia. Para reentrarem nos packs: modelar cada um
como struct no `auli-contract` (+ `text_to_embed`/`stored_repr`) e ter um produtor que preencha o
contrato.

---

## 5. Vocabulário de kinds (`servicos` ↔ `services`) — ✅ **resolvido (auditoria PR #4)**

A auditoria de consistência **unificou o kind vetorial `services` → `servicos` ponta a ponta**
(`corpus`/`update`/`rag`/`packs`/`manifest`). Não há mais tradução por convenção: o `registry.toml`,
o scraper, a UI e o engine falam um vocabulário só. Os packs saem `<id>-servicos.json`.

---

## Itens relacionados (revisões de código anteriores)

- **`public/<id>/servicos.json` (~660KB) e contratos do engine — ✅ resolvido:** o `build-frontend-public.sh`
  copia só os artefatos que a UI busca (per-público + index + `faqs-tree.json` + `ref/`), prefixando
  com `<id>-`; os contratos `<id>-{faqs,servicos}.json` **não** vão para `public/`.
- **Gate de coleção no frontend — ✅ resolvido (auditoria #4):** `ServicosList` gateia em
  `hasCollection` como as irmãs; coleção ausente rende `CollectionEmpty` (sem 404). **Menor/aberto:**
  [Home.tsx](auli-frontend/src/pages/home/Home.tsx) ainda lista as abas de forma estática (a barra
  mostra todas), mas cada lista se auto-gateia — logo estados sem uma coleção mostram
  `CollectionEmpty`, não erro.
- **Prompts `data/prompts/*.txt` — aberto (nota da auditoria #14):** os prompts que de fato rodam
  divergem no marcador de serviço (`rs.txt` tem `## servico`; `sc/pr/sp.txt` só `## pergunta`).
  Alinhar muda o comportamento do LLM ao vivo — decisão à parte.
- **Comentário histórico:** [derive_faqs.rs:29](auli-server/crates/auli-collections/src/derive_faqs.rs#L29)
  cita `EmbedStrategy::QuestionKey` (tipo já removido) — referência de lineage, cosmética.
