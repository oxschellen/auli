# TAREFA — Integração completa da LC 123 no acervo (entregas 5 a 9)

Para o Claude Code, no working copy `~/Desktop/auli`. Data: 19/08/2026. Autor do insumo: chat (Fase C executada em 19/08).

## 1. Contexto

A LC 123/2006 está no acervo `rs` com **72 pares** (entregas 1–4, integradas até o PR #153; acervo total 384 pares / 5 normas). A Fase C gerou em 19/08 as **entregas 5–9**, que completam os pares da lei:

| entrega | zip | bloco | pares | ref |
|---|---|---|---|---|
| 5 | `lc123-entrega5-mei.zip` | arts. 18-A–18-F (MEI) | 73–90 (18) | `lc-123-2006-arts-18a-18f-texto-vigente.md` |
| 6 | `lc123-entrega6-sublimites.zip` | arts. 19–20 | 91–96 (6) | `lc-123-2006-arts-19-20-texto-vigente.md` |
| 7 | `lc123-entrega7-arts21-27.zip` | arts. 21–27 | 97–111 (15) | `lc-123-2006-arts-21-27-texto-vigente.md` |
| 8 | `lc123-entrega8-exclusao.zip` | arts. 28–32 | 112–118 (7) | `lc-123-2006-arts-28-32-texto-vigente.md` |
| 9 | `lc123-entrega9-arts33-41-FINAL.zip` | arts. 33–41 | 119–130 (12) | `lc-123-2006-arts-33-41-texto-vigente.md` |

**Total: 58 pares novos → a LC 123 fecha em 130; acervo esperado após integração: 442.** Da entrega 9, usar SOMENTE o zip **FINAL** (é a reconciliação única das duas gerações; qualquer outro zip da E9 é obsoleto). O escopo de pares da lei está COMPLETO — o C6 (Anexos I–V) está FORA desta tarefa (formato pendente de decisão do Carlos).

Fatos que mudam o procedimento em relação às integrações anteriores:

- **Formato canônico na origem**: as cinco entregas já saem no formato do `render_doc` (frontmatter plano sem aspas, 4 chaves `titulo/trilha/ementa/link`, `## corpo` única seção). O script de normalização usado no ITCD **não é necessário** — mas a validação com o parser real (mddoc) continua obrigatória.
- **Refs com corte aplicado**: os refs das entregas 7, 8 e 9 NÃO são espelho da fonte — carregam marcas editoriais `[REDAÇÃO PRÉ-CORTE ...]` e `[FORA DO CORTE ...]` (restaurações e exclusões do corte pré-LC 214, fontes anotadas em cada marca). Isso é deliberado; não "corrigir".
- **Verificação de capuz cumprida**: as redações restauradas dos capuz 22, 33 e 39 foram confirmadas por varredura integral das Lcp147 e Lcp155 (nenhuma os altera). Risco residual documentado no REGISTRO da E9.

## 2. Decisões

- **D-INT-1** — Destino dos pares: `data/rs/docs/legislacao/Lei Complementar 123-2006 — Simples Nacional/` (pasta EXISTENTE das entregas 1–4). Nomes de arquivo dos zips são PROVISÓRIOS (prefixo = nº do par); o rename canônico via `nome_faq_de` (mddoc real) acontece na ingestão, como sempre. O prefixo numérico é descartável — a identidade é hash(link+pergunta).
- **D-INT-2** — Destino dos refs: `data/rs/ref/` (família `<id>-*-texto-vigente.md`, pulada pelo `copy_ref`), ao lado dos refs das entregas 1–4.
- **D-INT-3** — REGISTROs (`REGISTRO-lc123-entrega5.md` … `entrega9.md`) vão para `docs/` e entram num **PR só de docs**, no padrão dos #152/#153. O arquivo `fontes/redacoes-pre-corte-c5.md` da E9 é material de auditoria das restaurações: versionar em `docs/` como `REGISTRO-lc123-entrega9-fontes.md` (2,8 KB; não é dado do acervo, é prova documental — não deixar em `data/`).
- **D-INT-4** — Conteúdo dos pares é INTOCÁVEL na integração. Qualquer erro de conteúdo detectado volta para o Carlos/chat; o Code não edita par.
- **D-INT-5** — Após a integração, atualizar as contagens da LC 123 no `docs/REGISTRO-legislacao-perguntas.md` (§2.4 e §4: 72 → 130), respeitando a regra da Fase B de contagem única.

## 3. Fases (com portões humanos)

**Fase I0 — staging.** Carlos fornece os 5 zips. Extrair; validar cada par com o parser mddoc real (deve passar sem normalização); montar a lista de renames provisório→canônico; verificar colisão de identidade contra os 72 existentes (esperado: zero). **PORTÃO G1**: apresentar ao Carlos o resumo (58 pares, renames, zero colisões, refs e REGISTROs mapeados) antes de copiar qualquer coisa para `data/`.

**Fase I1 — ingestão.** Copiar pares (com rename canônico) para a pasta da lei; copiar os 5 refs para `data/rs/ref/`; rebuild da coleção `legislacao` da entidade `rs` (delete+rebuild do kind, como nas integrações anteriores — `auli-collections` roda de dentro de `auli-server/`); regenerar `rs-legislacao-index.json`.

**Fase I2 — verificações.**

1. Contagens: LC 123 = 130; acervo rs = 442.
2. **Ordem no derivador/aba** via `chave_dispositivo`: 18 < 18-A < … < 18-F < 19; 21 < 21-A < 21-B < 22; 38 < 38-A < 38-B < 39 (sufixos de artigo já suportados desde a #151). A sentinela da lei (par 2, ementa fora da fórmula) continua indo para o FIM do grupo com aviso do derivador — comportamento esperado, não é bug.
3. Smoke tests de retrieve (chat tipo 4, coleção legislacao): "qual o limite de receita do MEI?" → par 73 no topo; "prazo mínimo de vencimento do ICMS-ST para o Simples" → par 103; "sublimite de 3,6 milhões" → par 92; "multa por atraso no PGDAS" → par 124; "quem julga auto de infração do Simples no RS" → par 126.
**PORTÃO G2**: resultados ao Carlos antes de subir.

**Fase I3 — publicação.** `build-packs.sh rs` (lembrete operacional: `docs_hash` divergente faz o servidor RECUSAR boot até o pack novo — sequência pack→restart); restart do serviço; conferência da aba Legislação na UI. PR único com os 5 REGISTROs + fontes-C5 + atualização do REGISTRO-legislacao-perguntas (D-INT-5). **PORTÃO G3**: PR revisado pelo Carlos.

## 4. Fora de escopo

C6 (Anexos I–V da LC 123 — tabelas, formato a decidir); qualquer geração de conteúdo novo; alterações de código (a coleção legislacao está implantada desde #149/#151 e nada aqui exige mudança de código — se exigir, PARAR e reportar).
