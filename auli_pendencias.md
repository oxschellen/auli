# Pendências — auli-contract e integração de dados

Registro dos assuntos em aberto **após** o merge do `auli-contract` (PR #3) no `main`. O refactor
fez a **struct tipada** (`auli_contract::Table<P>`) virar a fonte única do dado: o scraper grava o
contrato em `data/<id>/raw/<id>-<kind>.json` e o `auli update` o consome. O que ficou para depois:

---

## 1. Verificação ponta a ponta (Fase 5) — **não executada**

A validação de equivalência nunca rodou (precisa de rede externa + chave de LLM; não há cache local
em `data/rs/cache/`). Antes de confiar nos packs novos:

1. Re-raspar `rs` ao vivo: `cd auli-engine && cargo run -p auli-collections -- rs faqs` e `... rs servicos`
   (headless Chrome na SEFAZ-RS — lento; gera `data/rs/raw/rs-faqs.json` e `rs-servicos.json`).
2. `scripts/build-packs.sh rs` → `data/rs/packs/` com `strategy_version: 2`.
3. `./start_server.sh --no-tunnel` → conferir boot: **services ≈ 627, faqs ≈ 1914** (pareceres/notas
   ausentes — esperado, ver item 4).
4. Rodar as **5 perguntas de referência** e comparar: cada resposta deve citar o **mesmo serviço
   principal e o mesmo `servico=NNNN`**. Equivalência, não bit-paridade (re-vetorização é esperada).

> Os binários release já estão compilados (workspace inteiro builda). Só falta a parte de rede.
>
> **Atalho offline já aplicado (2026-06-23).** Após o bump `STRATEGY_VERSION`→2, o server passou a
> recusar os packs antigos (strategy=1). Para desbloquear **sem re-raspar**, foi adicionado o modo
> `cargo run -p auli-collections -- <id> rebuild`, que reconstrói o contrato
> (`<id>-faqs.json`/`<id>-servicos.json`) a partir do que já está em `data/<id>/raw/` (árvore
> `faqs.json` + per-tipo de serviços), e em seguida `scripts/build-packs.sh <id>` regerou os packs
> `strategy=2` (modelo BGE-M3 em cache, offline). Boot OK. **Ressalvas que um scrape ao vivo
> reconcilia:** (a) `rs-services` deu **586** (não 627) — os per-tipo em `raw/` são de uma raspagem
> mais antiga que o `portal-servicos.txt`; (b) os packs **`rs-pareceres` (331) / `rs-notas` (1)
> antigos (strategy=1) continuam em disco** e o `load_all` os carrega (não são reconferidos por
> coleção) — resíduo inofensivo (não entram no RAG); para zerar, apagar
> `data/rs/packs/rs-{pareceres,notas}.json` ou regerar tudo do zero.

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
([servicos/mod.rs:133](auli-engine/crates/auli-collections/src/servicos/mod.rs#L133), `servico_text_to_embed`).
O plano deixou a fórmula exata como pendência. Validar/ajustar contra as 5 perguntas (item 1) e fixar.
Para FAQs a key é `origin + pergunta` (preserva o antigo `QuestionKey`) e está estável.

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
tradução vive só no [update.rs](auli-engine/crates/auli-cli/src/update.rs) (`servicos.json` → kind
`services`) e num comentário do `registry.toml`. Pendência do plano: derivar `registry.toml` e o
frontend de um `Kind` tipado único, eliminando a chance de divergência.

---

## Itens relacionados (da revisão de código anterior, ainda abertos)

- **Defaults stale do CLI:** `--packs-dir` default `./vectors` e `--source <dir_com_portal_txt>`
  na docstring ([main.rs:5,24](auli-engine/crates/auli-cli/src/main.rs#L24)) — só funciona porque
  `start_server.sh`/`build-packs.sh` passam os caminhos explícitos. Atualizar defaults/docstring.
- **`public/<id>/servicos.json` (~660KB)** copiado pelo gerador mas **sem consumidor** no frontend —
  peso morto; filtrar em [build-frontend-public.sh](scripts/build-frontend-public.sh).
- **Abas hardcoded no frontend:** [ServicosList.tsx](auli-frontend/src/pages/servicoslist/ServicosList.tsx)
  não usa `hasCollection`; [Home.tsx](auli-frontend/src/pages/home/Home.tsx) hardcoda as abas em vez
  de derivar de `collections` (SC mostra abas vazias).
- **`EMBED_CACHE_DIR` com duas fontes:** `.env` (`./models`) vs `build-packs.sh` (`$ROOT/auli-engine/models`).
- **Comentário histórico:** [faqs/mod.rs:99](auli-engine/crates/auli-collections/src/faqs/mod.rs#L99) cita
  `EmbedStrategy::QuestionKey` (tipo já removido do engine) — referência de lineage, cosmética.
