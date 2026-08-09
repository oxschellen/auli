# TAREFA-DOCUMENTO (v2) — Todos os passos até implantar a struct genérica `Documento`

> **CONCLUÍDA em 08/08/2026** — Etapa A no PR #132, Etapa B no #133, Etapa C neste.
>
> **Onde a implementação divergiu do plano, e por quê:**
>
> - **Sem `dir()` no `Kind`** (D-A1). Devolveria o mesmo texto do `as_str()`, e seria o primeiro
>   lugar onde os dois poderiam divergir. A unificação nome=diretório=sufixo=rota já era doutrina.
> - **`corpus.rs` ficou fora do `Kind`** (D-A1). É a tabela de parâmetros de *recuperação*, não de
>   coleções: carrega `notas`, um kind de rota sem árvore nem struct.
> - **Sem o campo `chave`** (D-B1). Nenhum consumidor precisou dele — a identidade é o `titulo` na
>   jurisprudência e o `link` nas bordas, e o `doc_path` já carrega o slug.
> - **`trilha` e `doc_path` entraram nas fases que os USAM** (B2 e B4), não na B1. Na B1 seriam
>   peso morto por duas fases.
> - **`ConsultaPackPayload` e `render_consulta_block` saíram na B4**, não na B5: com o pack v2 não
>   existe mais quem os alimente, e manter uma struct que se diz "o payload do pack" por um commit
>   é pior que removê-la junto.
> - **Uma divergência de saída, conhecida e medida**: o bloco da jurisprudência com `ementa` vazia
>   perde a linha em branco que o formato antigo deixava. Alcançável em **1 de 19.780 pareceres**
>   (`data/pr/docs/pareceres/consulta-no-32-2007.md`) e em 0 de 3.800 acórdãos.
> - **O ganho de tamanho do pack v2 foi marginal** (−1,7% no rs), contra a expectativa registrada
>   na D-B4. Os vetores são 90–98% do pack. O ganho real foi o caminho único.
> - **Os totais do TARF nos manuais MCP ficaram de fora** (C2): a coleta está em 3.800 de ~22.522,
>   e um número que muda a cada rodada é pior que mandar consultar `listar_entidades`.

**Revisão de 08/08/2026 (noite)** — substitui a v1, refletindo o que os PRs #130/#131 e o
HANDOFF.md já entregaram. **Leitura prévia:** `docs/ESTUDO-colecoes-trait.md` (Adendos 1–3).
(O `HANDOFF.md` citado abaixo foi apagado depois de implantada esta TAREFA — envelheceu; os números
de RSS que ele registrava vivem agora em `docs/auli_pendencias.md` §31.1, remedidos.)

**O que esta TAREFA fecha:** ao final, o contrato tem UMA struct `Documento`
(`chave, kind, link, doc_path, titulo, trilha, ementa, resumo`), UM compose (já existe:
`compose_unificado`, lib.rs:249), UM template de bloco RAG, `doc_path` lazy para TODAS as
coleções, e o TARF respondendo no chat/MCP como quarta coleção vetorizada.

**Já entregue — NÃO refazer:** vocabulário unificado das árvores + parser único (#128);
invólucro `## documento {i}: {rotulo}` + 31 prompts (#129); aba "Acórdãos TARF" com índice
leve derivado (`rs-tarf-index.json`, #130); downloads com rótulo e gate de coleção
(7ddf324/#131). O contrato do índice com o frontend (campos `numero`/`assunto`/`resumo`)
é DELIBERADO e não muda nesta TAREFA.

---

## Portão de entrada (pré-requisitos, fora desta TAREFA)

- [ ] **Coleta TARF completa**: censo == totalItens (~22.522), pelos 4 passos do §3 do
      HANDOFF (indice tarf → `build-packs.sh rs` → remover `nota_tipo("tarf")` e a nota da
      TarfList → deploy-frontend). Lembrete do HANDOFF: `docs_hash` divergente faz o
      servidor RECUSAR o boot do rs (manifest.rs:188) — a árvore precisa estar PARADA.
- [ ] **Regra do layout "Pleno, sem tabela" decidida com o censo** (~340 docs, ementa em
      `<p><strong>EMENTA</strong>:` sem âncora para o fim da fundamentação) e re-run
      emitindo os ex-anômalos do cache. Candidata registrada (decidir com o censo, não
      antes): ementa = o parágrafo do `<strong>EMENTA</strong>`; `## resumo` = a própria
      ementa (o fallback já decidido para acórdãos sem fundamentação) — nenhuma fronteira
      inventada. `tarf-anomalias.txt` final vazio ou com resíduo justificado.
- [ ] Fora do escopo permanente: prefixos `P:`/`R:` do embed FAQ (gate P5 — hoje vivem no
      VALOR dos slots, não na fórmula; a remoção é só mapeamento); qualquer mudança de
      fórmula de embed (`STRATEGY_VERSION` intocadas do início ao fim).

---

# ETAPA A — O TARF entra no serving (chat, retrieve e MCP)

### Decisões

- **D-A1 (o `Kind` nasce ABSORVENDO o `Colecao`):** enum `Kind { Pareceres, Tarf, Servicos,
  Faqs }` no `auli-contract`, com `as_str()`, `dir()`, `rotulo()` e `exige_resumo()` (só
  jurisprudência). O enum `Colecao` do `indice.rs` (#130) é REMOVIDO e o `indice` importa o
  `Kind` — um enum só no projeto, nunca dois paralelos. As strings hardcoded migram para
  ele: `update.rs` (58/66/82 + `docs_dir.join` em 159/203/262), `mcp.rs` (consts 35–42) e o
  `nota_tipo`/`titulo_tipo` do `bundle.rs`. O rótulo `rotulo::ACORDAO_TARF` do rag.rs
  (declarado adormecido no #129) liga aqui e perde o `#[allow(dead_code)]`.
- **D-A2 (pack próprio):** `docs/tarf/` vetorizado num pack de kind `"tarf"` — NUNCA
  misturado ao de pareceres. O caminho de jurisprudência do update é parametrizado pelo
  `Kind` (mesmo parser, mesma guarda de resumo, mesmo compose — que já são únicos desde
  #128). Atenção operacional (HANDOFF §5): o update do rs carrega a coleção na RAM; com
  ~22,5k acórdãos além dos pareceres, rodar e MEDIR o RSS — se apertar, a dívida "update em
  lotes" sobe de prioridade, mas não entra nesta TAREFA.
- **D-A3 (retrieve):** `"tarf"` aceito no parâmetro `kind` do `/v1/retrieve` (api/dto.rs:40)
  com a política de retrieval dos pareceres (mesma família; score bands idênticas na v1 —
  revisão só com evidência de uso).
- **D-A4 (sem expansão por relacionados na v1):** o grafo de citações segue exclusivo dos
  pareceres. Extensão futura registrada, fora desta TAREFA.
- **D-A5 (chat):** o seletor de fonte da caixa de mensagem ganha "Acórdãos TARF" (visível só
  onde a entidade tem a coleção — dirigido pelo registry, não hardcoded). Contexto usa
  `envolver(rotulo::ACORDAO_TARF, ...)`. Prompt novo `data/prompts/rs-tarf.txt` derivado do
  `rs-pareceres.txt` com a terminologia ajustada (acórdão/recurso/TARF; citação pelo deep
  link `tarf.sefaz.rs.gov.br/documento/{guid}`); registrado no `registry.toml` num campo
  `prompt_tarf` espelhando o `prompt_pareceres` existente.
- **D-A6 (MCP):** `buscar_pareceres`/`obter_parecer` ganham parâmetro opcional `colecao`
  (default `"pareceres"` — compatibilidade total; `"tarf"` aceito onde a entidade tem a
  coleção). `listar_entidades` reporta a coleção nova; `KINDS_LISTADOS` (mcp.rs:42) passa a
  vir do `Kind`; os manuais em `manuais/` atualizam os totais com o censo final.
- **D-A7 (frontend/downloads): JÁ ENTREGUE** (#130/#131). Nesta etapa só o que o fecho da
  coleta não cobriu: nada, se o portão de entrada foi cumprido. Conferir e seguir.

### Fases

**A1** — `Kind` no contrato + absorção do `Colecao` + varredura das strings (D-A1); testes:
`Kind::todos()` cobre 4; `indice` continua com default `pareceres` (compatibilidade CLI).
**A2** — update vetorizando `docs/tarf/` (D-A2) com fixture; contagens do rs no log listam a
coleção; medir RSS do update do rs e registrar no PR.
**A3** — retrieve + chat + prompt (D-A3/A5); teste do handler com kind "tarf".
**A4** — MCP (D-A6); teste de compatibilidade: chamada SEM `colecao` devolve byte a byte o
que devolvia antes.
**A5 — portão humano (Carlos):** rodada de perguntas reais sobre acórdãos no chat rs
(qualidade, deep links corretos, ADERÊNCIA índice a índice) + uma chamada MCP com e sem
`colecao`. Aprovado ⇒ **deploy da Etapa A** (build-packs rs + deploy-frontend + restart da
API) — a etapa entrega valor sozinha.

---

# ETAPA B — A struct genérica `Documento` no código

### Decisões

- **D-B1 (a struct):** `Consulta` (lib.rs:171) → `Documento`, campos no vocabulário
  unificado: `{ chave, kind: Kind, link, doc_path, titulo, trilha, ementa, resumo }` (o
  `text_to_embed` materializado permanece SE a B3 mostrar que o pack ainda o quer — é cache
  do compose, documentado como tal). O nome da struct não vaza em serialização (verificado:
  serde grava só campos); os NOMES DOS CAMPOS mudam no pack — absorvido pela migração de
  pack da B3, sem `serde(rename)` de compatibilidade (pack é regenerável, não é dado
  precioso). O contrato do ÍNDICE com o frontend NÃO muda (decisão do #130: aquele JSON fala
  `numero`/`assunto`/`resumo` de propósito).
- **D-B2 (bordas viram projeção):** `Servico` e `Faq` recuam para as bordas (process/derive
  e JSONs do frontend). Na entrada da vetorização, `fn para_documento(&self) -> Documento`
  preenche os papéis (tabela do Adendo 3 §3.1; os prefixos `P:`/`R:` da FAQ continuam
  entrando pelo VALOR dos slots, como hoje). O update processa `Vec<Documento>` nas quatro
  coleções — UM caminho de código.
- **D-B3 (bloco único + `doc_path` lazy para todos):** `fn bloco(d: &Documento, conteudo:
  &str) -> String` no contrato (template do Adendo 3 §3.2), com testes byte a byte contra os
  três `stored_repr` (lib.rs:94/136) e o `render_consulta_block` — inclusive omissão de
  trilha/ementa vazias. Pack v2: payload
  `{ chave, kind, titulo, trilha, ementa, link, doc_path, resumo }`, SEM bloco
  pré-renderizado; o serving monta o bloco na hora lendo o conteúdo da árvore via `doc_path`
  para os k selecionados. Degradação graciosa: jurisprudência → `resumo`; serviços/FAQs →
  `"[conteúdo indisponível — ver o link oficial]"`.
- **D-B4 (pack v2):** bump da versão de pack; `auli update` geral (28 packs). Vetores
  idênticos por construção — verificado por amostra: uma consulta fixa por coleção,
  distâncias antes/depois (o critério da migração de agosto). Rodar o update geral EM LOTES
  por entidade como na migração (HANDOFF §5: SP sozinho = 24,7 GB RSS).
- **D-B5 (o que morre):** os três `impl stored_repr`, `render_consulta_block` e
  `ConsultaPackPayload` (lib.rs:221) REMOVIDOS. Os adaptadores de compose FICAM (documentação
  viva de qual papel cada coleção preenche).
- **D-B6 (travas):** testes que pinam bytes de bloco/pack migram para o formato novo no
  MESMO commit, com o comentário-padrão de mudança deliberada. Paridade chat×MCP por
  construção.

### Fases

**B1** — Rename `Consulta`→`Documento` + campos, num commit MECÂNICO próprio (zero mudança
de comportamento; workspace inteiro verde antes de seguir).
**B2** — `bloco()` + testes byte a byte (a alma da etapa; sem eles, nada de merge).
**B3** — Projeções `para_documento` + update num caminho só (D-B2).
**B4** — Pack v2 + serving lazy + degradação (D-B3).
**B5** — Remoções e travas (D-B5/B6).
**B6** — Migração local: update geral em lotes; contagens vs censo pós-TARF; amostra de
distâncias por coleção.
**B7 — portão humano (Carlos):** perguntas reais nas QUATRO fontes do chat rs + uma entidade
só-serviços (ex.: rr) + MCP manual. Critérios: qualidade sem degradação, links corretos,
blocos do log idênticos aos de antes (agora montados lazy), ADERÊNCIA intacta.

---

# ETAPA C — Implantação e fecho

**C1** — Deploy (packs v2 + binário da API + frontend se tocado); smoke em produção: uma
consulta por coleção via interface pública + uma chamada MCP.
**C2** — Docs: manuais MCP com totais finais; `ESTUDO-colecoes-trait.md` ganha nota
"implantado em <data>"; esta TAREFA entra em `docs/` junto das irmãs; de carona, corrigir a
nota stale de `docs/auli_pendencias.md` sobre marcadores de prompt (pré-#129 — os 31 já
falam `## documento`).
**C3** — Sinal verde final do Carlos. A partir daqui, coleção nova = variante no `Kind` +
projeção (+ struct de borda se precisar) — o custo marginal que o estudo prometia.

## Critérios de aceite globais

- `cargo test` e `clippy -D warnings` verdes em todas as fases; nenhuma trava afrouxada.
- Zero mudança de vetor em qualquer pack (amostras de distância nas quatro coleções).
- Cliente MCP antigo (sem `colecao`), frontend em produção e o índice do #130 não quebram em
  nenhum estado intermediário.
- `grep` final: nenhuma string de kind fora do `Kind`; um único enum de coleção no projeto;
  `stored_repr`, `render_consulta_block` e `ConsultaPackPayload` ausentes do código vivo.
- Os dois portões humanos (A5, B7) registrados com data e resultado nos PRs.
