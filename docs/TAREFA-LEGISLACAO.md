# TAREFA-LEGISLACAO — a 5ª coleção: legislação em pares P/R

> Para o Claude Code. Trabalhar **uma fase por vez**, com portão humano entre elas: ao fim de cada
> fase, parar, relatar e aguardar aprovação. `cargo build/test --workspace` sem warnings ao fim de
> cada fase (inclui `clippy -D warnings`).

## 1. Contexto e o que já existe

- O acervo ganha uma coleção nova, `legislacao`: **pares pergunta/resposta sobre textos legais**,
  gerados por Título fora do app e materializados como `.md` no vocabulário unificado. Ela
  funciona **como as FAQs** (sem `## resumo`, rebuild total), com uma diferença estrutural: a
  árvore tem **uma subpasta por lei**, e o nome da pasta é o nome da lei.
- Já existe o primeiro dataset: **134 pares da Lei 6.537/1973** (Processo Tributário
  Administrativo do RS), em `data/rs/docs/legislacao/Lei 6.537-1973 — Processo Tributário
  Administrativo/` (**Fase 0 concluída** — os três invariantes fecham em 134/134). Frontmatter:
  `titulo` = pergunta, `trilha` = hierarquia da lei, `ementa` = dispositivo (ex.: "Art. 21, I"),
  `link` = página da lei no site da SEFAZ (inpKey=109403). Nome de arquivo no padrão FAQ
  (slug da pergunta ~60 + hash8).
- Virão outras leis, na ordem: parte tributária da **CF** (piloto: art. 155, redação anterior à
  EC 132/2023 — o acervo é deliberadamente pré-reforma), **CTN**, **Lei Kandir**, leis estaduais.
  O desenho deve assumir N leis por entidade desde já.
- O playbook do próprio repo (`docs/auli_code.md` §3.13): coleção nova = uma variante no `Kind` +
  uma projeção/fórmula; os `match` exaustivos apontam cada ponto a preencher. O TARF entrou assim
  (PRs #132/#133) — usar aqueles PRs como referência de forma.

## 2. Decisões (D-LEG-*)

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
| **D-LEG-14** | MCP **fora da v1** | `buscar_pareceres`/`obter_parecer` são semanticamente de jurisprudência; expor legislação por ali confundiria o contrato. Registrar como pendência (ferramenta própria ou parâmetro novo, a decidir depois). Os 4 manuais MCP e os totais mantidos à mão **não mudam** nesta tarefa. |
| **D-LEG-15** | Sem bump de `STRATEGY_VERSION` nem de `PACK_FORMAT` | Nenhuma fórmula existente muda; a coleção é nova (pack novo). O `docs_hash` do rs muda porque a árvore cresce — o remédio é o de sempre: `build-packs.sh rs` recarimba. **Já divergiu na Fase 0**: manifesto `54891358d9fe1b3f`, disco `325e104d359f044f` (antes de mover o texto vigente para `ref/`). Até o `build-packs.sh rs` da Fase 2, **o boot do rs RECUSA** — é a guarda funcionando, não regressão. |
| **D-LEG-16** | O texto integral da lei fica em `data/<id>/ref/<id>-{nome}-texto-vigente.md`, **fora** de `docs/` | Achado da Fase 0. A árvore `docs/` é doutrina "um .md = uma unidade vetorizável": o `arquivos_md` filtra só por extensão, então o texto compilado dentro da subpasta quebraria o `parse_doc` assim que a leitura virasse recursiva. Mantê-lo como documento seria pior que quebrar — sem pergunta, a key fica degenerada, o teto de 6.000 corta o resto, e um hit despejaria 141 KB no prompt pela leitura tardia do `bloco()`. É insumo de origem, papel do `ref/`, onde já vivem os `portal-*.txt`. O `copy_ref` do `build-frontend-public.sh` pula o sufixo `-texto-vigente.md` pelo mesmo motivo do `portal-pareceres.txt`: KB mortos em cada deploy. **Dívida cosmética registrada na 2ª entrega**: o sufixo marca a família por NOME, e o texto da CF é deliberadamente congelado antes da EC 132 — chamá-lo de `rs-cf-art-155-texto-vigente.md` afirma vigência sobre um texto revogado. A leitura defensável é "vigente no corte do acervo", que é o que o par sentinela explica ao usuário. Se um dia incomodar, a saída limpa é o `copy_ref` pular por DIRETÓRIO (`ref/fontes/`, que o `find -maxdepth 1 -type f` já não visita) em vez de por família de nome — aí o arquivo pode se chamar `rs-cf-art-155-redacao-pre-ec132.md`, que é o que ele é. Não é urgência. |

## 3. Fases

### Fase 0 — preparo da árvore (sem código de produção)

1. Renomear `data/rs/docs/legislação/` → `data/rs/docs/legislacao/` (D-LEG-1).
2. Conferir que os 134 `.md` estão dentro da subpasta da lei e que **todos parseiam** no
   `mddoc::parse_doc` (parser estrito: chave desconhecida/duplicada ⇒ erro). Fazer a conferência
   com um utilitário descartável ou teste `#[ignore]` — não inventar ferramenta permanente.
3. Verificar o invariante do D-LEG-4 nos 134: 1º segmento da `trilha` == nome da pasta, e nome do
   arquivo == `nome_faq_de(titulo, link)`. **Onde divergir, corrigir o DADO** (renomear
   arquivo/ajustar trilha), nunca afrouxar a regra. Relatar quantos precisaram de correção.
4. Portão humano: relatório da conferência antes de qualquer mudança de código.

### Fase 1 — contrato (`auli-contract`)

1. `Kind::Legislacao`: variante nova + `TODOS: [Kind; 5]` (ordem: Servicos, Faqs, **Legislacao**,
   Pareceres, Tarf) + os quatro textos (D-LEG-6) + `exige_resumo=false`, `pack_incremental=false` +
   método novo `arvore_com_subpastas` (D-LEG-3). Deixar os `match` exaustivos apontarem o resto.
2. `compose_legislacao_text_to_embed` (D-LEG-7) no `lib.rs`, com teste de **fórmula congelada**
   próprio (reimplementação literal no módulo de testes, como `formulas_antigas`).
3. `Documento::doc_path()`: braço `Kind::Legislacao` (D-LEG-4). A pasta sai do 1º segmento da
   `trilha` (separador ` | `, o mesmo das trilhas existentes); o nome do arquivo é
   `mddoc::nome_faq_de`.
4. Testes de `kind.rs` atualizados (o `TODOS.len()`, distinção de nomes, rotulo≠as_str etc. —
   os existentes quebram e guiam).
5. Portão humano.

### Fase 2 — engine e update (`auli-core`, `auli-cli`)

1. `corpus.rs`: const `LEGISLACAO` + `ALL` 5→6 + `from_kind`.
2. `update.rs`: `arquivos_md` ganha o modo recursivo (um nível), acionado por
   `kind.arvore_com_subpastas()`; ordenação continua determinística (ordenar o caminho relativo
   completo — a ordem canônica de escrita do pack depende disso, D-INC-4). `documento_de` ganha o
   braço da key (D-LEG-7). Guarda de invariante do `doc_path` (D-LEG-4) no `preparar`.
3. Serving: nada estrutural — o pack v2 + `bloco()` únicos já servem qualquer `Kind`; conferir a
   degradação (`conteudo_indisponivel` cai no texto genérico, correto: `exige_resumo=false`).
4. `bundle.rs` (aba Downloads): incluir a coleção no zip do estado — o `match` exaustivo aponta.
5. Rodar `build-packs.sh rs` localmente; conferir boot com o pack novo (`rs-legislacao`, 134
   documentos no manifesto) e `docs_hash` recarimbado.
6. Portão humano: mostrar o relatório do update (contagens, teto) e uma query manual de retrieve.

### Fase 3 — chat (tipo 4)

1. `rag.rs`: `QueryType::from_code(Some(4))` → variante nova (sugestão: `QueryType::Legislacao`,
   ou generalizar `Jurisprudencia(Kind)` — escolher o que deixar o `match` mais honesto; a
   expansão por grafo continua exclusiva de pareceres). `LEG_FLOOR`/`LEG_BAND`. Label do log:
   `"legislacao"`.
2. `entities.rs`: `prompt_legislacao` opcional + default global (D-LEG-9); generalizar o
   `debug_assert` de `prompt_de`.
3. `config/prompts/rs-legislacao.txt` + default: instruir a responder com base no dispositivo,
   **citar artigo e link**, não extrapolar o texto legal, e observar que o acervo reflete a
   redação anterior à EC 132/2023.
4. `registry.toml`: `prompt_legislacao` no rs + `"legislacao"` em `collections` do rs (comentário
   explicando, no estilo do bloco do tarf).
5. Testes: `from_code(4)`, labels, prompt fallback.
6. Portão humano: uma pergunta real ("qual o prazo para impugnação de auto de lançamento?") com
   o contexto RAG e a resposta anexados.

### Fase 4 — índice e frontend

1. `auli-collections`: `indice` aceita `legislacao` (a guarda do `main.rs` hoje exige
   `exige_resumo` — passar a aceitar jurisprudência OU legislação). Derivação própria (módulo novo
   ou braço claro): leitura recursiva, shape do D-LEG-11, ordenação do D-LEG-12 com testes dos
   formatos reais ("Art. 1º", "Art. 21, I", "Art. 121, § 2º" — cobrir ordinal º e parágrafos).
2. `build-packs.sh`: bloco final deriva também o índice de legislação quando a árvore existe.
3. Frontend: tipo `Collection` += `"legislacao"` (via `gen-frontend-entities.mjs` após o registry);
   `COLECAO_DO_TIPO["4"] = "legislacao"`, `isQuestionType`, `ROTULOS["4"] = "Legislação"` (contrato
   com `QueryType::from_code` — documentar como nos demais); aba na sidebar (grupo Acervo, ícone
   `MdOutlineMenuBook` ou similar), `LegislacaoList` (D-LEG-13) com estados de carga/vazio via
   `AsyncContent`/`CollectionEmpty` como as irmãs.
4. Testes frontend: `tiposDisponiveis` com "4", parse/agrupamento/ordenação da lista, busca.
5. Portão humano: screenshot/descrição da aba e do seletor.

### Fase 5 — publicação

1. `build-frontend-public.sh rs` (o índice novo já entra por `raw/*.json`), `deploy-frontend.sh`
   (staging + troca atômica + smoke/rollback, como sempre).
2. Reiniciar o servidor com os packs novos (lembrar: `docs_hash` divergente ⇒ o boot RECUSA —
   `manifest.rs`; o `build-packs.sh rs` da Fase 2 já recarimbou).
3. Atualizar `docs/auli_code.md` §3.13 (quatro coleções → cinco; nota sobre a primeira árvore com
   subpastas) e o comentário do `registry.toml` que descreve as coleções.

## 4. O que NÃO fazer nesta tarefa

- **Não** mexer no MCP (D-LEG-14), nos manuais da aba "Conectar sua IA" nem nos totais mantidos à
  mão.
- **Não** tocar nas fórmulas/keys das coleções existentes, nem em `STRATEGY_VERSION`/`PACK_FORMAT`.
- **Não** criar scraper — a coleção é autorada; a geração de novos datasets (CF etc.) é processo à
  parte, fora do app.
- **Não** implementar grafo de conhecimento sobre legislação — está no radar do plano geral, mas é
  tarefa própria, depois que a coleção estiver servida.
- **Não** antecipar a decisão do gate P5 (prefixos P:/R:) — a coleção nasce com os prefixos, como
  as FAQs de hoje.

## 5. Riscos e pontos de atenção

- **Invariante pasta↔trilha** (D-LEG-4) é a peça nova mais delicada: a guarda no `preparar` é o que
  a torna segura. Escrever o teste dela ANTES do braço do `doc_path`.
- **Nomes de pasta com espaços/acentos** são legítimos (nome da lei) e aparecem dentro do
  `doc_path` gravado no pack — conferir que a leitura tardia do corpo (join com `docs_root`) e o
  `hash_docs_tree` (já recursivo, caminho relativo entra no hash) os tratam bem; há teste para o
  caminho com espaço.
- **Ordinal e incisos na ementa**: o parse do dispositivo (D-LEG-12) deve ser conservador —
  qualquer coisa fora de "Art. {número}" no começo ⇒ sem chave, fim do grupo. Nunca inventar ordem.
- **`localStorage auli.questionType`** já lida com tipo indisponível (deriva o efetivo por render)
  — conferir que "4" persiste e degrada certo ao trocar de entidade.
- A coleção é pequena (134 docs) — nenhum risco de memória; o teto de 6.000 chars praticamente não
  corta (respostas são curtas), mas o relatório do teto no update deve confirmar.
