# TAREFA — Integração da Lei 8.115/85 (IPVA) na coleção legislacao

Data: 19/08/2026. Executor: Claude Code, no repo `~/Desktop/auli`.
Padrão de referência: TAREFA-LC123-INTEGRACAO.md e as integrações #149/#151/#152/#153.

## 0. Contexto

Entrega única gerada no chat em 19/08/2026: a Lei 8.115/85 (IPVA-RS) completa em
40 pares P/R, já no formato canônico do render_doc, + ref texto-vigente + REGISTRO.
Fonte textual: PDF compilado oficial da AL-RS (repLegisComp), atualizado até a
Lei 16.307/2025, com discriminação do vigente feita pelas redações riscadas.
É a 1ª lei nova depois do ITCD; não há corte de reforma — o par 1 é sentinela
avisando que a EC 132/2023 não está internalizada na lei gaúcha.

## 1. Insumo

Zip `ipva-lei-8115-entrega-unica.zip` (42,7 KB), fornecido por Carlos
(confirmar o caminho com ele; sugestão: `~/Downloads/`). Conteúdo:

- `ipva/pares/` — 40 arquivos `NN-slug.md` (prefixo numérico PROVISÓRIO, §3.6 do
  REGISTRO-legislacao-perguntas)
- `ipva/ref/lei-8115-1985-texto-vigente.md` — texto vigente consolidado (22,7 KB)
- `ipva/REGISTRO-ipva-entrega-unica.md` — fontes, convenções, mapa dos pares,
  armadilhas de vigência e checks executados

Não usar nenhuma outra versão: esta entrega é única (não houve E1/E2/E3).

## 2. Decisões vinculantes

- **D-INT-IPVA-1 (pasta):** criar `data/rs/legislacao/Lei 8115-1985 — Imposto sobre a Propriedade de Veículos Automotores (IPVA)/`
  (D-LEG-4; nome idêntico ao 1º segmento da trilha dos 40 pares — conferir byte a
  byte antes de criar, o doc_path deriva daí).
- **D-INT-IPVA-2 (link — RESOLVER ANTES DO RENAME):** os pares saíram com
  `link: https://www.legislacao.sefaz.rs.gov.br/Site/Document.aspx?inpKey=109693`.
  Conferir o host/formato REAL contra os pares já integrados do ITCD
  (inpKey=109695) e do RITCD (inpKey=109696) em `data/rs/legislacao/`. Se
  divergirem, ajustar o frontmatter `link` dos 40 arquivos para o formato do
  acervo **antes** de qualquer rename: `nome_faq_de` é FNV-1a64 de
  `link+"\n"+pergunta` — trocar o link depois do rename órfã os slugs.
- **D-INT-IPVA-3 (rename canônico):** aplicar `nome_faq_de` real (mddoc.rs) aos
  40, descartando o prefixo numérico, como na integração da CF (41/41
  idempotente). Zero colisões esperadas; qualquer colisão é STOP para humano.
- **D-INT-IPVA-4 (ref):** mover o ref para `data/rs/ref/` mantendo o nome
  `lei-8115-1985-texto-vigente.md` (já segue a família `*texto-vigente.md` que o
  copy_ref pula — validar que o padrão de exclusão pega este nome).
- **D-INT-IPVA-5 (documentação versionada):** `REGISTRO-ipva-entrega-unica.md`
  vai para `docs/` no padrão #152/#153. `data/**` segue fora do git.
- **D-INT-IPVA-6 (conteúdo intocável):** nenhum ajuste de texto nos corpos dos
  pares nem no ref além do `link` da D-INT-IPVA-2. Erro factual encontrado =
  reportar, não corrigir.
- **D-INT-IPVA-7 (contagens):** atualizar o REGISTRO-legislacao-perguntas §4:
  somar +40 pares e +1 norma à contagem VIGENTE no arquivo (ler, não assumir —
  o valor depende de a integração da LC 123 E5–E9 já ter rodado: 384/5 normas
  antes dela, 442/5 depois; com o IPVA fica 424/6 ou 482/6). Registrar a Lei
  8.115 na tabela de normas com fonte (PDF AL-RS até 16.307/25) e link.

## 3. Fases e portões

### I0 — Preparação (leitura, sem escrita)

1. Ler `docs/REGISTRO-legislacao-perguntas.md` (§3 convenções, §4 contagens),
   `auli-contract/src/mddoc.rs` (render_doc, slug, nome_faq_de) e o REGISTRO da
   entrega dentro do zip.
2. Extrair o zip em staging fora de `data/` (ex.: `/tmp/ipva-staging/`).
3. Conferir: 40 pares, ref, REGISTRO presentes; parse dos 40 pelo parser mddoc
   real (não reimplementar); 1º segmento da trilha idêntico nos 40.

### G1 — Portão de staging (humano)

Apresentar a Carlos: resultado do parse 40/40, decisão tomada na D-INT-IPVA-2
(link mantido ou ajustado, com o formato encontrado no ITCD/RITCD) e a lista de
renames previstos (provisório → canônico). Só avançar com OK dele.

### I1 — Ingestão

1. Aplicar D-INT-IPVA-2 (se necessário) e D-INT-IPVA-3 no staging.
2. Criar a pasta da lei (D-INT-IPVA-1) e mover os 40 pares.
3. Mover o ref (D-INT-IPVA-4) e o REGISTRO (D-INT-IPVA-5).
4. Atualizar contagens (D-INT-IPVA-7).

### I2 — Verificações (G2, automatizadas; falha = STOP)

1. **Contagem:** 40 arquivos na pasta nova; total da coleção legislacao = valor
   anterior + 40.
2. **Ordem na aba:** a pasta "Lei 8115-1985 …" deve ordenar entre
   "Lei 6537-1973 …" e "Lei 8821-1989 …" (ordem alfabética de string do
   caminho relativo — comparar no índice gerado).
3. **Ordem interna (chave_dispositivo):** regenerar
   `rs-legislacao-index.json` e conferir a sequência do art. 4.º, que concentra
   12 pares: `Art. 4.º` < `Art. 4.º, II` < … < `Art. 4.º, VII…` <
   `Art. 4.º, VIII e X` < `Art. 4.º, IX e § 7.º` < `Art. 4.º, §§ 1.º e 2.º` <
   `Art. 4.º, §§ 8.º a 10` < `Art. 4.º, § 11` — atenção ao § 11 vs § 2
   (desempate NUMÉRICO, não alfabético). Conferir também `Arts. 14 e 15`,
   `Arts. 18 a 20` (chave lê o primeiro artigo).
4. **Sentinela:** o par de ementa `Art. 1.º (escopo do acervo)` segue a fórmula
   padrão (§3.5) — deve ordenar junto do art. 1.º, sem warning do derivador
   (diferente do caso divergente documentado da LC 123).
5. **Smoke tests de retrieve** (top-3 deve conter o par alvo):
   - "qual a alíquota de IPVA para locadora de veículos" → par do art. 9.º, V
   - "carro com mais de 20 anos paga IPVA" → par do art. 4.º, IV
   - "vendi o carro e não transferiram, IPVA é meu?" → par do art. 6.º, II
   - "até quando posso pagar o IPVA" → par do art. 11, § 3.º
   - "perdi o carro na enchente, preciso pagar IPVA?" → par do art. 4.º, § 11
6. **copy_ref:** rodar e confirmar que `lei-8115-1985-texto-vigente.md` NÃO é
   copiado para a árvore de pares.

### I3 — Publicação (G3)

1. `build-packs.sh rs` (docs_hash divergente ⇒ o servidor RECUSA boot —
   comportamento esperado até o restart; manifest.rs:188).
2. Restart do auli-server; smoke manual: aba Legislação exibe a lei nova na
   posição esperada, chat tipo "4" responde uma das perguntas do item 5.
3. PR único com: docs (REGISTRO + atualização de contagens) e qualquer script
   de verificação novo. Mensagem citando esta TAREFA. `data/**` fora, como
   sempre.

## 4. Fora de escopo

- C6 da LC 123 (Anexos I–V) e qualquer pendência da TAREFA-LC123-INTEGRACAO —
  se ela ainda não rodou, executá-la é tarefa separada (apenas coordenar a
  D-INT-IPVA-7 com o estado real das contagens).
- Mudanças de código (enum, derivador, chave_dispositivo, UI).
- RIPVA (Decreto do IPVA) — não decidido; não criar placeholder.
- Qualquer edição de conteúdo dos pares (D-INT-IPVA-6).

## 5. Armadilhas conhecidas (do REGISTRO da entrega — ler antes de mexer)

- Art. 4.º, III (GAP) e § 11 (desastre natural) são alterações de **2025**
  (Leis 16.307/25 e 16.288/25) — se algum reviewer estranhar, é isso mesmo.
- Art. 12: caput vetado pelo Governador e MANTIDO pela Assembleia (DOE 71/86) —
  vigente.
- Par 17 (Copa 2014) é registro histórico de dispositivo exaurido — deliberado.
- A numeração 1–40 dos arquivos provisórios NÃO segue a ordem dos artigos
  (o par 40 é o art. 7.º, acrescentado após a checagem de cobertura) — a ordem
  final vem da chave_dispositivo, não do prefixo.
