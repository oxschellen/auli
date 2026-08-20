# TAREFA-LEGISLACAO-LACUNAS — fechar as lacunas do acervo de legislação

**Data:** 18/08/2026. **Origem:** auditoria do repo (main @ #151) contra o estado das entregas geradas no chat.
**Escopo:** as 3 leis processadas até agora — Lei 6.537/73, CF (parte tributária) e LC 123/2006.
**Regra geral:** cada fase termina em portão humano (Carlos aprova antes da seguinte).

## Estado atual (main, PR #151 de 17/08)

| Lei | Gerado | Integrado no acervo | Lacuna |
|---|---|---|---|
| Lei 6.537/73 | 134 pares | 134 | nenhuma |
| CF (STN) | 112 pares (6 entregas) | 92 (entregas 1–4) | entregas 5–6 (20 pares) + decisão de fronteiras |
| LC 123/2006 | 72 pares (4 entregas) | 62 (entregas 1–3) | entrega 4 (10 pares) + blocos não gerados |
| **Total** | **318** | **288** | |

Material de entrada: os ZIPs das entregas não integradas, fornecidos por Carlos
(CF: 5ª = art. 156, 8 pares 93–100; 6ª = arts. 157–162, 12 pares 101–112;
LC 123: 4ª = art. 18 §§ 6º–27, 10 pares 63–72). Cada zip contém pares + ref + REGISTRO.

## Fase A — Integrações pendentes (material pronto nos zips)

A1. **CF, entregas 5 e 6** (20 pares): pipeline idêntico ao das entregas 1–4 —
pares para `docs/legislacao/Constituição Federal — Sistema Tributário Nacional/`,
rename canônico via `nome_faq_de` (conferir idempotência), refs
(`cf-art-156-*` e `cf-arts-157-162-*`, família `texto-vigente`) para `data/rs/ref/`.

A2. **LC 123, entrega 4** (10 pares 63–72): pares para a pasta da lei, ref
`lc-123-2006-art-18-6-27-texto-vigente.md` para `data/rs/ref/`,
`REGISTRO-lc123-entrega4.md` para `docs/` (commitado, como os 3 anteriores).
Isto RESOLVE a ressalva do #151 ("§§ 6º a 27 sem base textual conferida"):
a base é a fonte Câmara/LEGIN, registrada no próprio REGISTRO da entrega.

A3. **Ref órfão da 2ª entrega da LC 123** (apontado pelo #151): extrair
`lc-123-2006-arts-1-3b-texto-vigente.md` do zip da entrega 2 para `data/rs/ref/`.
Os 17 pares já estão ingeridos; falta só o texto de referência.

A4. Rebuild e verificação: `build-packs.sh rs` (docs_hash divergente ⇒ o boot
RECUSA até rodar), boot ok, retrieve de amostra na legislacao
(sugestão: "sublimite ICMS Simples Nacional" deve trazer o 13-A e os §§ 17/17-A),
conferir no índice `rs-legislacao-index.json` a ordem do art. 18 completo
(§ 5º-B < § 5º-C < ... < § 15-A < § 16 < § 22-A — exercita a chave do #151).

**Portão A:** Carlos confere a aba Legislação (ordem e totais: 318 pares, 3 leis).

## Fase B — Correções documentais

B1. **Inventário do `REGISTRO-legislacao-perguntas.md` (§4)**: hoje declara 226
pares e não conhece a LC 123 — a cada entrega um dos dois lugares fica errado.
RECOMENDAÇÃO: o §4 deixa de declarar totais por lei e
passa a APONTAR para os REGISTROs por entrega (fonte única: um total geral,
atualizado junto com cada integração).

B2. Conferir se algum total "mantido à mão" no frontend/manuais menciona a
legislacao (padrão da aba "Conectar sua IA"); se sim, atualizar junto.

**Portão B:** diff dos documentos revisado por Carlos.

## Fase C — Lacunas de GERAÇÃO da LC 123 (conteúdo novo)

Os blocos abaixo do escopo aprovado ainda não têm pares. Gerar no MESMO método
das entregas 1–4 (ver §Convenções); uma entrega por bloco, com ref + REGISTRO:

C1. **Arts. 18-A a 18-F (MEI)** — 5ª entrega, a maior lacuna e a mais perguntada
(limite 81 mil, DAS-MEI valores fixos, desenquadramento, contratação de MEI,
seguro-defeso, MEI caminhoneiro da LC 188). Estimativa: 15–20 pares.
C2. **Arts. 19–20 (sublimites estaduais)** — casa com os pares 17/65 já ingeridos.
C3. **Arts. 21–27** — recolhimento, ISS retido (art. 21 § 4º), repasse,
créditos (art. 23: regra e exceção do § 1º), obrigações acessórias.
C4. **Arts. 28–32 (exclusão)** — ofício vs comunicação, efeitos, prazos.
C5. **Arts. 33–41** — fiscalização, omissão de receita, processo, MS coletivo.
C6. **Anexos I a V (tabelas)** — entrega própria, SÓ com fetch verificado
(decisão da 3ª entrega: nunca de memória). Fonte primária: o PDF Câmara/LEGIN;
alternativa: `Lcp155.htm` do Planalto (contém os 5 anexos integralmente e não trunca).

**Fonte padrão** (decisão de Carlos, 14/08): PDF consolidado Câmara/LEGIN
"normaatualizada" — o `lcp123.htm` do Planalto trunca no art. 18 § 15-A para
qualquer extrator (testado; não insistir). Links dos pares seguem nas âncoras
do Planalto (o truncamento é do fetch, não do navegador).
ATENÇÃO no corte: o PDF da Câmara consolida a LC 214/2025 — dispositivo com
"redação dada pela LC 214" mostra o texto PÓS-reforma; recuperar a redação
anterior na página da LC que a escreveu antes (Lcp147/155/188.htm, pequenas).

**Portão C:** um bloco por vez, aprovação de Carlos entre blocos (piloto: C1).

## Fase D — CF: fronteiras (ENCERRADA em 20/08/2026 — ficam FORA)

Carlos decidiu: o art. 195 (contribuições sociais) e o ADCT tributário **não
entram** no acervo. Nada a gerar. A CF fica fechada nos 112 pares das Seções I a
VI. Registrado no §2.2-g do `REGISTRO-legislacao-perguntas.md`.

## Convenções (resumo vinculante — detalhe nos REGISTROs por entrega)

- Par = .md no vocabulário unificado: `titulo` = pergunta; `trilha` hierárquica
  com 1º segmento = nome da pasta da lei, separador `|`; `ementa` = dispositivo
  ("Art. 18, § 5º-M"); `link` = âncora oficial; corpo = resposta.
- Nomes provisórios `NNN-slug.md` até o rename canônico via `nome_faq_de`.
- Numeração contínua POR LEI (LC 123 segue de 73; CF fechada em 112).
- Corte temporal: CF = pré-EC 132/2023; LC 123 = pré-LC 214/2025 (e pré-LC 227/2026).
  Sentinelas já existem (CF par 2 do 155; LC 123 par 2) — NÃO duplicar.
- Notas rotuladas permitidas: (infraconstitucional), (jurisprudência),
  (corte pré-reforma). Refs consolidados emenda a emenda na família
  `<id>-*-texto-vigente.md` em `data/rs/ref/` (o copy_ref já pula a família).

## Fase E — Lei 8.821/1989 (ITCD-RS) — 4ª lei do acervo [ADENDO 18/08]

APROVADA por Carlos em 18/08. Pasta: "Lei 8821-1989 — ITCD". Link dos pares:
SEFAZ inpKey=109695. Numeração própria da lei. Corte: texto vigente consolidado
(SEM corte de reforma — sentinela própria no par 2: EC 132 não internalizada).

E1 (arts. 1º–7º): **GERADA em 18/08** — 20 pares + ref + REGISTRO no zip
`lei8821-entrega1-arts1-7.zip` (Carlos fornece). Integrar no mesmo pipeline
da Fase A. Atenção: primeira lei do acervo cuja pasta começa com "Lei 8821" —
conferir a posição na ordenação alfabética do índice.

E2 e E3: GERADAS em 18/08 (lei COMPLETA, 40 pares; conferir pendência do art. 12, § 2º). ADENDO: RITCD (Decreto 33.156/89) também COMPLETO em 18/08 — 26 pares em 3 zips (ritcd-entrega1/2/3), pasta própria "Decreto 33156-1989 — Regulamento do ITCD", fonte = arquivo oficial SEFAZ enviado por Carlos, refs por extração direta. Integrar lei + decreto juntos; par ITCD total: 66 pares.
Base textual: LegisWeb id=153614 (LEGIS AL-RS e SEFAZ bloqueiam fetch).
Armadilha vinculante: valores em UPF-RS da data da avaliação — nunca converter
a reais; art. 7º, IX está REVOGADO (não recriar a isenção de 10.509 UPF).

**Portão E:** Carlos confere a aba com a 4ª lei e a coerência da sentinela.
