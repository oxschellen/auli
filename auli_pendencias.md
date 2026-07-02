# Pendências — auli-contract e integração de dados

Registro dos assuntos em aberto **após** o merge do `auli-contract` (PR #3) no `main`. O refactor
fez a **struct tipada** (`auli_contract::Table<P>`) virar a fonte única do dado: o scraper grava o
contrato em `data/<id>/raw/<id>-<kind>.json` e o `auli update` o consome. O que ficou para depois:

> **Revisão 2026-07-02 (pós fases 1 e 2).** O modelo evoluiu: o **snapshot v2**
> (`data/<id>/<id>-snapshot.json`, tipos em `auli_contract::snapshot`) virou a fronteira
> scraper→collections; a coleta saiu para os binários **`auli-scraper-rs`** / **`auli-scraper-sc`** e
> o **`auli-collections <e>`** só deriva os artefatos (contrato/prints/index/per-público). O
> subcomando `rebuild` foi **removido** (a regeração é offline pelo `process` a partir do snapshot).
> Status atualizado item a item abaixo.

---

## 1. Verificação ponta a ponta — **scrape + packs feitos; faltam as 5 perguntas**

Nesta sessão (2026-07-02) o scrape ao vivo **rodou** e reconstruiu o cache em `data/rs/cache/`:

1. `cd auli-server && ./target/debug/auli-scraper-rs all` (faqs + serviços; headless Chrome na
   SEFAZ-RS) → grava o `rs-snapshot.json` (v2).
2. `./target/debug/auli-collections rs` (process) → deriva `rs-faqs.json`/`rs-servicos.json` +
   prints + index + per-público.
3. `scripts/build-packs.sh rs` → `data/rs/packs/` com `strategy_version: 2` (**faqs 1937,
   services 586**). Boot não conferido aqui (sem `--no-tunnel` nesta sessão).

**Equivalência confirmada (código):** todos os agregados RAG/engine saíram **byte a byte idênticos**
comparando o código novo × antigo sobre o **mesmo cache** (isolando drift de conteúdo). Falta só o
passo de RAG:

**Ainda pendente** — rodar as **5 perguntas de referência** contra o server (precisa de chave de LLM
+ boot) e conferir que cada resposta cita o **mesmo serviço principal e o mesmo `servico=NNNN`** —
equivalência, não bit-paridade.

> **Resíduo (inalterado):** os packs **`rs-pareceres` / `rs-notas` antigos (strategy=1)** continuam
> em `data/rs/packs/` e o `load_all` os carrega (não entram no RAG — inofensivo). Para zerar, apagar
> `data/rs/packs/rs-{pareceres,notas}.json`. (Contagem `services=586` já é a atual — o "627" antigo
> era de uma raspagem mais velha.)

---

## 2. Frontend desacoplado do contrato — **quebra conhecida**

A aba **FAQs lê `public/<id>/faqs.json`** (a árvore) em
[FaqsList.tsx:17](auli-frontend/src/pages/faqslist/FaqsList.tsx#L17), mas o scraper **deixou de
gravar `faqs.json`** (Fase 2 — agora grava `<id>-faqs.json`, o contrato). Quando o `public/` for
regerado por [build-frontend-public.sh](scripts/build-frontend-public.sh) (que copia `raw/*.json`),
o `faqs.json` não estará lá e **a aba FAQs quebra**.

Opções (escolher uma — é trabalho de frontend, fora do escopo do contrato):

- **(a)** o scraper/gerador também produzir o `faqs.json` (árvore) a partir do contrato;
- **(b)** a aba FAQs passar a ler o contrato `<id>-faqs.json` (`Table<Faq>`, lista achatada);
- **(c)** um endpoint de leitura no backend servindo de `data/<id>/`.

Notas:

- `servicos.json` (agregado) também foi descartado, mas o frontend **não** o usa (usa
  `servicos-index.json` + per-tipo) — sem impacto.
- O gerador agora copiaria `<id>-faqs.json`/`<id>-servicos.json` (contrato) para `public/` como
  arquivos novos sem consumidor — considerar filtrá-los.

---

## 3. Fórmula de `text_to_embed` de serviços — **provisória**

Hoje é `tipo | classe` + título + os primeiros 300 chars do corpo da descrição
([servicos/mod.rs:116](auli-server/crates/auli-collections/src/servicos/mod.rs#L116), `servico_text_to_embed`,
agora no lado da derivação/`process`). O plano deixou a fórmula exata como pendência. Validar/ajustar
contra as 5 perguntas (item 1) e fixar. Para FAQs a key é `origin + pergunta` (preserva o antigo
`QuestionKey`) e está estável — inalterada pelas fases 1/2 (a materialização virou `faq_from_raw` em
[derive_faqs.rs](auli-server/crates/auli-collections/src/derive_faqs.rs)).

---

## 4. `pareceres` / `notas` / `conteudos` — **adiados (sem fonte struct)**

São conteúdos **autorados** (em `data/<id>/ref/`), sem scraper — não há `Table<P>` para eles. O
`auli update` os encontra ausentes e os pula; o server tolera packs ausentes (sobe com a coleção
vazia). Para reentrarem nos packs:

- modelar cada um como struct no `auli-contract` (campos + `text_to_embed`/`stored_repr`);
- ter um produtor (scraper ou conversor do `portal-*.txt` autorado) que preencha o contrato.

---

## 5. Vocabulário de kinds (`servicos` ↔ `services`) — **não enforçado**

O label de UI/scraper `servicos` mapeia para o kind vetorial `services` apenas por convenção: a
tradução vive só no [update.rs](auli-server/crates/auli-cli/src/update.rs) (`servicos.json` → kind
`services`) e num comentário do `registry.toml`. Pendência do plano: derivar `registry.toml` e o
frontend de um `Kind` tipado único, eliminando a chance de divergência.

---

## Itens relacionados (da revisão de código anterior, ainda abertos)

- **`public/<id>/servicos.json` (~660KB) — resolvido na origem:** o agregado `servicos.json` **não é
  mais gerado** em `raw/` (o `process` grava só `<id>-servicos.json` + per-público + index), então o
  gerador não tem o que copiar. Resta só apagar cópias antigas eventualmente presentes em `public/`.
- **Abas hardcoded no frontend:** [ServicosList.tsx](auli-frontend/src/pages/servicoslist/ServicosList.tsx)
  não usa `hasCollection`; [Home.tsx](auli-frontend/src/pages/home/Home.tsx) hardcoda as abas em vez
  de derivar de `collections` (SC mostra abas vazias).
- **Comentário histórico:** [derive_faqs.rs:29](auli-server/crates/auli-collections/src/derive_faqs.rs#L29)
  cita `EmbedStrategy::QuestionKey` (tipo já removido do engine) — referência de lineage, cosmética.
