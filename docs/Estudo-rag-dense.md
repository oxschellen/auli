# Estudo: RAG denso + esparso — a busca híbrida com o esparso do BGE-M3, reaberta depois do FP32

Documento para o Claude Code. Versão 1, de 14/11/2026, escrita contra o `HEAD` `0bbe44f` (#241, 13/11/2026) e
contra o fonte do `fastembed 7.6.1` (o crate publicado, lido arquivo a arquivo).

**Natureza:** estudo, pergunta em aberto. A única parte executável agora é a **Fase 0**, que é medição e não toca em
nenhum caminho de produção. As fases 1–4 são esboço condicionado ao resultado dela e a um portão humano.

**Relação com o que já existe:** continua o [ESTUDO-busca-hibrida.md](ESTUDO-busca-hibrida.md) (08–11/08/2026) e o
item 3 do [ESTUDO-pos-documento.md](ESTUDO-pos-documento.md). **Supera a §5 e o passo 1 da §8 daquele estudo**; as
§§2, 3, 4, 9 e 10 continuam valendo. Onde este documento e o repositório divergirem, **o repositório vence** — e a
divergência é achado a registrar, não a contornar.

## A pergunta

Somar à busca densa de hoje o componente **esparso (lexical) do próprio BGE-M3**, calculado sobre a mesma key e
fundido por ranking, melhora o que quem atende encontra — o bastante para pagar um vetor a mais por registro, uma
segunda forma de pontuar e a mudança no que o log chama de aderência?

A resposta é um número medido no acervo real, não um raciocínio. Este estudo existe para dizer **o que medir, como,
e o que cada resultado decide**.

## O que mudou desde agosto (verificado no fonte em 14/11/2026)

**1. O obstáculo que travava a via do esparso deixou de existir.** A §5 do estudo de agosto dizia: o denso é o INT8
(`gpahal/bge-m3-onnx-int8`, 560 MB), o `SparseModel::BGEM3` aponta para o FP32 completo, e o "teste de meia hora"
era descobrir se o `UserDefinedSparseModel` aceitava o INT8. Desde 07/11/2026 o encoder **é** o FP32
(`embed.rs`, `EMBED_MODEL_ID = "bge-m3-fp32"`), e no `fastembed 7.6.1` os dois modelos apontam para os **mesmos
arquivos**:

| | `model_code` | `model_file` | `additional_files` |
|---|---|---|---|
| `EmbeddingModel::BGEM3` (`src/models/text_embedding.rs`) | `BAAI/bge-m3` | `onnx/model.onnx` | `onnx/model.onnx_data`, `onnx/Constant_7_attr__value` |
| `SparseModel::BGEM3` (`src/models/sparse.rs`) | `BAAI/bge-m3` | `onnx/model.onnx` | `onnx/model.onnx_data`, `onnx/Constant_7_attr__value` |

Não há segundo download nem segundo modelo: o esparso sai do arquivo que já está em `models/`. O teste de meia hora
não precisa mais ser feito — a pergunta que ele respondia sumiu.

**2. O `Bgem3Embedding` (denso + esparso + ColBERT numa passada) não serve.** A única variante é
`Bgem3Model::BGEM3Q`, o INT8 que foi abandonado justamente por mudar de vetor com a versão do ONNX Runtime
(`Estudo-fastembed-update-v2.md` §Resultado). Usá-lo seria voltar ao defeito.

**3. O resto do terreno também andou:** `STRATEGY_VERSION` é 3 (o estudo de agosto dizia 1), `PACK_FORMAT` é 2, o
`key_hash` existe e é o cache de vetores das cinco coleções (D-REU-1), a key tem teto de 6.000 caracteres, e o
acervo são 27 entidades e 38.000 registros (07–08/11/2026).

## O que NÃO muda — e limita o que este estudo pode entregar

**A §10 do estudo de agosto continua de pé e não é tocada aqui.** O esparso é calculado sobre o `text_to_embed`,
a mesma key do denso. O que está só na seção `## corpo` do `.md` não entra em nenhum dos dois. No caso que originou
tudo, isso significa: dos 364 acórdãos com "conluio", o esparso disputa a ordenação dos **40** que têm o termo na
key; os **324** que o têm só no corpo seguem invisíveis.

Este estudo, portanto, **não resolve o termo raro que mora no corpo**. O que ele pode entregar é outra coisa:
identificadores e termos exatos **que já estão na key** e que o denso ordena mal — número de parecer e de acórdão,
artigo e número de lei, códigos, palavra rara na ementa ou na fundamentação. A Fase 0 mede as duas coisas
separadamente (item M3), para que a decisão não seja tomada achando que se comprou o que não se comprou.

## Como o esparso do BGE-M3 funciona no `fastembed 7.6.1`

Lido em `src/sparse_text_embedding/impl.rs` e `bgem3_weights.rs`:

- `SparseTextEmbedding::embed(texts, batch_size)` roda a sessão e, para `SparseModel::BGEM3`, pega a **primeira
  saída** como os hidden states por token.
- `post_process_bgem3`: para cada posição não mascarada e que não seja token especial (ids 0, 1, 2, 3), calcula
  `peso = max(0, hidden · w + b)`; por id de token fica o **maior** peso; devolve
  `SparseEmbedding { indices: Vec<usize>, values: Vec<f32> }` com os índices em ordem crescente.
- `w` e `b` são a camada `sparse_linear` do BGE-M3 (1024 floats + 1), **embutida no crate**
  (`weights/sparse_linear.safetensors`, 4 KB). O módulo que a carrega é **privado** (`mod bgem3_weights;`).
- `SparseInitOptions` é `InitOptionsWithLength<SparseModel>`: tem `with_max_length`, `with_intra_threads`,
  `with_cache_dir` e `with_auto_device`, como o `TextInitOptions`. **O `max_length` padrão é 512** — precisa do
  mesmo `EMBED_MAX_TOKENS` do denso. O lote padrão é 256 — precisa do mesmo `Some(1)`.
- A pontuação entre pergunta e documento é a do próprio modelo: a soma, sobre os ids de token em comum, de
  `peso_pergunta × peso_documento`. Maior é melhor (o contrário da distância cosseno).

**Uma consequência que tem de ser medida, não suposta:** o vocabulário é o do tokenizer do modelo
(XLM-RoBERTa), que distingue maiúscula de minúscula e letra acentuada de não acentuada. A ementa do TARF é
telegráfica **em maiúsculas** ("ICMS. CRÉDITO FISCAL. PRESUMIDO."), e quem pergunta digita em minúsculas. Se
"conluio" e "CONLUIO" forem ids diferentes, o esparso não casa os dois — e a busca literal da aba, que ignora caixa
e acento, continua sendo melhor que ele nesse caso. É o item M4 da Fase 0.

## Contexto atual do código (verificado em 14/11/2026)

- **Encoder único:** `crates/auli-core/src/embed.rs` — `Embedder` sobre `TextEmbedding` com
  `EmbeddingModel::BGEM3`, `EMBED_MAX_TOKENS = 8192`, `PreferCPU`, `embed_dense` com `Some(1)` e `FATIA = 256`.
  Só o `auli-core` depende do `fastembed`. Três testes ignorados em `testes_ordem` fixam determinismo bit a bit.
- **Store:** `crates/vector-store` — `Record { id, embedding, payload, key_hash }`, varredura exata por cosseno
  (`scan`), `ReadStore::query_scored` devolve `(payload, distância)` em ordem crescente. O store é agnóstico por
  doutrina: conhece só `(id, vetor, payload)`.
- **Motor:** `crates/auli-retrieval/src/lib.rs` — `Engine::embed`, `search_embedded`, `select_by_proximity`
  (que tem um `debug_assert!` exigindo entrada em ordem crescente de score). `Hit { payload, score }`, com
  `score` = distância cosseno.
- **Quem consulta:** o chat (`auli-cli/src/rag.rs`, embeda a pergunta uma vez e reusa em duas coleções), o
  `/v1/retrieve` (`api/handlers/retrieve.rs`) e o MCP (`mcp.rs`). As bandas são todas `∞` com piso 0.
- **O `score` tem três consumidores:** as bandas, a seção `ADERÊNCIA` do log (`aderência = 1 − distância`,
  `rag.rs`) e o campo `score` dos DTOs do retrieve e do MCP.
- **Update:** `auli-cli/src/update.rs` — `preparar` lê a árvore e monta os `Documento` (é o único caminho que
  produz a key); `vetores_reaproveitaveis` casa por `(id, key_hash)` com gate `cache_valido`; `montar_records`;
  o trait `Vetorizador` tem só `embed_dense`.
- **Identidade:** `auli-core/src/manifest.rs` — `EMBED_MODEL_ID`, `EMBED_DIM`, `STRATEGY_VERSION = 3`,
  `PACK_FORMAT = 2`. O boot recusa divergência; o log carimba a identidade do espaço vetorial.
- **Ferramentas de medição que já existem:** `crates/auli-cli/tests/ab_faq_pr.rs` e `ab_pool.rs` (leem packs
  direto pelo `vector-store`, sem a validação do boot) e `examples/retrieval_test.rs`.

## Decisões em aberto

Nenhuma destas está tomada. Cada uma traz as opções e o que a Fase 0 mede para decidi-la.

**D-HIB-1 — De onde sai o esparso.**
- **(a) Segunda sessão**, `SparseTextEmbedding` ao lado do `TextEmbedding`. Só API pública. Custa uma segunda
  passada do modelo por texto e o modelo carregado duas vezes — no update **e no servidor**, que precisa do
  esparso da pergunta.
- **(b) Uma sessão só**, pelo `TextEmbedding::transform`, que é público e devolve a saída bruta
  (`EmbeddingOutput::into_raw` → `SingleBatchOutput { outputs, attention_mask_array }`); o esparso é calculado no
  `auli-core` a partir dos mesmos hidden states. Uma passada, um modelo em memória. Custa: os pesos do
  `sparse_linear` são privados no crate, então teriam de ser **vendorizados** (4 KB) junto com ~40 linhas de
  pós-processamento; o `SingleBatchOutput` não traz os `input_ids`, então é preciso re-tokenizar com o
  `tokenizer` (que é campo público); e o denso passa a ser extraído por nós, o que exige prova de igualdade **bit
  a bit** com o `embed` de hoje.
- **Mede-se na Fase 0:** tempo e RSS de (a). Se o custo de (a) for aceitável, ela vence por simplicidade; (b) só
  se justifica por número. A Fase 0 usa (a) — é a que não exige escrever nada além da ferramenta.

**D-HIB-2 — Onde o esparso mora.**
- **(a) No `Record`**, como campo opcional (o mesmo desenho do `key_hash`: `serde(default)` +
  `skip_serializing_if`). É mudança de **formato**, não de estratégia: os vetores densos não mudam, então
  `PACK_FORMAT` 2 → 3 e `STRATEGY_VERSION` fica. O store deixa de conhecer só `(id, vetor, payload)`.
- **(b) Arquivo irmão** por coleção (`<id>-<kind>.sparse.json`, `id → índices/valores`). Os packs não mudam um
  byte, o store continua agnóstico e o rollback é apagar o arquivo. Custa um invariante novo: os dois arquivos
  têm de ter exatamente os mesmos ids, e o manifesto e o boot têm de cobrir o segundo.
- **Mede-se na Fase 0:** o tamanho do esparso por registro. A ordem de grandeza esperada é de dezenas de por
  cento sobre o tamanho dos packs (508 MB nas 27 entidades antes da troca para o FP32; medir o de hoje) —
  estimativa de cabeça, a substituir pelo número medido.

**D-HIB-3 — Como fundir.** A proposta é **RRF** (fusão por posição: `Σ 1 / (k + posição)`, `k = 60`), a mesma que
o `ESTUDO-pos-documento.md` §1 já elegeu para a busca federada — denso e esparso têm escalas incomparáveis, e
fundir por score é a armadilha que aquele estudo descreve. Dois parâmetros a medir: `k` e a profundidade de cada
lista antes da fusão. Empate se resolve pelo `id`, para o resultado ser determinístico.

**D-HIB-4 — O que o `score` passa a significar.** Com fusão, um documento pode entrar pelo esparso com distância
cosseno ruim, e a lista deixa de estar em ordem crescente de distância. Isso quebra o `debug_assert!` do
`select_by_proximity`, muda o sentido da `ADERÊNCIA` do log e do `score` dos DTOs. A proposta a discutir:
`Hit.score` continua sendo a distância cosseno (a unidade não muda, o log e a base de calibragem da §9 seguem
comparáveis), e o `Hit` ganha a origem do documento (posição no denso, posição no esparso). As bandas, hoje `∞`,
passam a ser aplicadas antes da fusão ou deixam de se aplicar ao que veio do esparso — decidir com o número na mão.

**D-HIB-5 — Uma lista para todas as coleções, ou por coleção.** FAQs e serviços são perguntas em linguagem
natural, onde o denso já acerta 91,2% em 1º; jurisprudência e legislação são onde moram os identificadores. A
híbrida pode ajudar uma família e atrapalhar a outra. Se a Fase 0 mostrar isso, ligar por `Kind` é o mesmo
desenho do `pack_incremental`.

## Fase 0 — medir, sem tocar em produção

**Regra da fase:** nenhuma linha de `embed_dense`, do `update`, do motor ou do servidor muda. Nada é revetorizado.
Os packs de produção são só lidos. O que a fase acrescenta é uma ferramenta de medição e, no `auli-core`, o mínimo
para ela obter um vetor esparso — código que nenhum caminho de produção chama.

**Onde rodar:** num diretório de laboratório que reproduz a produção, como no piloto do
`Estudo-fastembed-update-v2.md`. Entidade: `rs` (tem as cinco coleções).

### O que construir

1. **No `auli-core`:** um tipo à parte do `Embedder` (não um método novo nele), embrulhando
   `SparseTextEmbedding` com `SparseModel::BGEM3` e **as mesmas opções** do denso: `EMBED_MAX_TOKENS`,
   `PreferCPU`, threads, `cache_dir`, `Some(1)` e fatiamento. Devolve índices e valores.
2. **A ferramenta**, no molde do `ab_faq_pr.rs` (teste `#[ignore]`, lê packs pelo `vector-store`). Ela precisa
   da **key** de cada documento, e a key só nasce em `update::preparar` (hoje `pub(crate)`). Não recompor a key
   por outro caminho: usar o `preparar` e **conferir** que `hash_hex(key)` é igual ao `key_hash` do registro
   correspondente no pack. Qualquer diferença é erro alto — significa que a medição não está sobre o que a
   produção indexou. Como dar à ferramenta acesso ao `preparar` (módulo de teste dentro do crate, ou abrir a
   visibilidade) é escolha de quem implementa; dizer qual foi e por quê.
3. Os vetores esparsos calculados ficam em memória ou num arquivo de laboratório. **Nada vai para `data/`.**

### O que medir

**M1 — Não piorou o que já funciona.** Repetir a medição do `Estudo-fastembed-update-v2.md` §Resultado: as
2.056 perguntas das FAQs do RS como consulta, acerto = a própria FAQ em 1º, e MRR@10. Linha de base: **91,2% /
0,9434** — reproduzi-la primeiro na coluna "só denso"; se não reproduzir, parar e entender antes de comparar.
Três colunas: só denso, só esparso, híbrido (RRF).

**M2 — Melhorou onde o denso é fraco.** Conjunto rotulado sozinho, sem julgamento humano: para cada parecer e cada
acórdão do `rs`, a consulta é o **título** do documento (que é só o número), e o acerto é o próprio documento. Na
migração de 07–08/11 esse foi exatamente o caso em que o denso falhou (as 4 faltas em 54 eram pareceres e TARF
perguntados pelo título). Medir acerto em 1º e entre os 10 primeiros, nas três colunas.

**M3 — O teto lexical, e quanto dele está na key.** Para uma lista de termos e identificadores (o "conluio" de
agosto e os que o mantenedor fornecer — ver o portão abaixo), contar por coleção: documentos com o termo na
**key**, e documentos com o termo **só no `## corpo`**. Comparação literal, sem caixa e sem acento, como a aba
faz. É a §10 de agosto generalizada: diz quanto do problema está ao alcance de **qualquer** índice sobre a key.

**M4 — Caixa e acento.** Para os mesmos termos: os ids de token do termo em minúsculas, em maiúsculas e sem
acento. E a pontuação esparsa de uma consulta em minúsculas contra um documento que tem o termo em maiúsculas.

**M5 — Custo.**
- número de entradas não nulas do esparso por documento (mínimo, mediana, P99, máximo), por coleção;
- tamanho estimado do esparso serializado, por coleção e total, contra o pack de hoje;
- tempo de calcular o esparso e pico de RSS (`/usr/bin/time -v`) **sobre o mesmo recorte do piloto de
  novembro** (as quatro coleções do RS sem o TARF, 3.538 documentos), para comparar com os 12 min 49 s e 3,8 GB
  do denso; e, à parte, o TARF do `rs`;
- tempo de vetorizar uma pergunta no esparso (mediana), ao lado dos 39 ms do denso;
- tempo de pontuar uma pergunta contra a maior coleção por varredura completa.

**M6 — Determinismo.** O equivalente dos três `testes_ordem` para o esparso: o mesmo texto dá os mesmos índices e
valores **bit a bit** entre chamadas, em qualquer ordem e composição do lote, e fatiado ou não. Se isso falhar, a
fase para aqui: sem determinismo não há pack reprodutível.

**Verificação por mutação** (CLAUDE.md §4): antes de confiar na tabela, quebrar a ferramenta de propósito e ver
o número cair — embaralhar a associação entre documento e vetor esparso deve levar o M2 ao chão. Uma ferramenta
que dá o mesmo resultado com o defeito dentro não está medindo nada.

### Portão humano da Fase 0

**Antes de rodar:** o mantenedor fornece (ou dispensa) uma lista de 10 a 20 consultas do tipo que quem atende
faz e o chat erra — termo solto, número, código. Elas entram no M3/M4 e numa impressão lado a lado (denso ×
híbrido, top 10) para julgamento a olho. Sem a lista, a fase roda só com o "conluio" e os conjuntos automáticos.

**Depois de rodar:** um `RELATORIO-rag-dense-fase0.md` com as tabelas M1–M6 e a impressão lado a lado. **A fase
termina aí.** Nenhuma das fases seguintes começa sem decisão explícita do mantenedor.

### Como ler o resultado

- **M1 cai e M2 não sobe de forma clara** → a híbrida sobre a key não se paga. Registrar e encerrar; a discussão
  volta para a §10 (indexar o corpo).
- **M2 sobe e M1 se mantém** → há caso para as fases 1–4; M5 decide D-HIB-1 e D-HIB-2.
- **M2 sobe só na jurisprudência e M1 cai nas FAQs** → D-HIB-5: ligar por `Kind`.
- **M3 mostra que quase tudo está só no corpo** → mesmo com M2 bom, o ganho percebido por quem atende será
  pequeno, e o próximo estudo é o índice do corpo, não este.
- **M4 mostra que caixa e acento separam os ids** → o esparso cru não substitui a busca literal; ou se normaliza
  o texto antes do esparso (mudança de fórmula, com tudo o que isso custa), ou a opção (a) do estudo de agosto
  (o chat apontar a aba) continua sendo a correção mais barata para o termo solto.

## Fases seguintes (esboço — não executar)

Só existem se a Fase 0 e o mantenedor disserem que sim. Estão aqui para que o custo total seja visível antes da
decisão, não como plano fechado.

**Fase 1 — Embedder.** Fechar D-HIB-1. O contrato do `embed_dense` não muda; o esparso entra por um caminho que
devolve os dois juntos. Em (b), o teste que decide é o denso bit a bit igual ao de hoje.

**Fase 2 — Pack, manifesto e update.** Fechar D-HIB-2. Dois pontos que nascem silenciosos e precisam de guarda:
- **A armadilha do cache.** `vetores_reaproveitaveis` casa por `(id, key_hash)`. Um registro antigo, sem
  esparso, casaria para sempre e nunca ganharia o esparso — o update diria "reaproveitado" e o acervo ficaria
  metade híbrido. O critério de reaproveitamento tem de incluir "tem esparso". É a mesma classe da armadilha
  descrita no `Estudo-fastembed-update-v2.md`.
- **A identidade.** O esparso depende do mesmo modelo e da mesma key; o manifesto tem de dizer se um pack tem ou
  não esparso, e o boot tem de recusar a mistura dentro de uma entidade, como já recusa as outras.

**Fase 3 — Motor, fusão e log.** Fechar D-HIB-3 e D-HIB-4. Uma implementação para as cinco coleções (é o ganho
da `Documento`). As três faces — chat, retrieve, MCP — passam pelo mesmo caminho; o log passa a registrar por
que cada documento entrou. Os DTOs públicos (`score`) só mudam com decisão explícita.

**Fase 4 — Migração.** As 27 entidades precisam do esparso. Em D-HIB-1(a) é uma passada a mais do modelo sobre
os 38.000 registros — da ordem das 2 h 35 min da revetorização de novembro, a confirmar pelo M5. O boot volta a
ser tudo-ou-nada durante a janela, como em novembro.

## Riscos

| Risco | O que o cobre |
|---|---|
| Comprar a híbrida achando que ela resolve o "conluio" | A seção "O que NÃO muda" e o M3 |
| A híbrida piorar FAQs e serviços, onde o denso já acerta | M1 como condição, e D-HIB-5 |
| Caixa e acento tornarem o esparso pior que a busca literal | M4 |
| Esparso não determinístico | M6, antes de qualquer outra coisa |
| Medir sobre uma key que não é a da produção | A conferência do `key_hash` na ferramenta |
| O `score` mudar de sentido em silêncio no log e nos DTOs | D-HIB-4 decidida antes da Fase 3 |
| Registros antigos reaproveitados sem esparso | A guarda do cache, na Fase 2 |
| Segunda sessão dobrar a memória do servidor | M5, e D-HIB-1(b) como saída |

## Fora de escopo

- **Indexar o `## corpo`** (por chunking ou por índice léxico). É a única opção que alcança os 324 e é assunto
  de outro estudo; o M3 daqui é o insumo dele.
- Calibrar as bandas (§9 de agosto: adiada, a base se acumula no log).
- Busca federada entre coleções (item 1 do `ESTUDO-pos-documento.md`), embora use a mesma fusão.
- Trocar de modelo, usar o ColBERT, GPU, ou mexer em `TETO_TEXT_TO_EMBED`, `FATIA`, `batch_size` ou
  `max_length`.
- Tantivy ou qualquer índice léxico próprio (a via (a) do item 3 do `ESTUDO-pos-documento.md`) — só volta à mesa
  se o M4 reprovar o esparso do modelo.
- Qualquer mudança no frontend.

## O que fazer com o estudo de agosto

Não reescrever. Acrescentar ao fim do `ESTUDO-busca-hibrida.md` uma **§11** curta, datada, dizendo que a §5 e o
passo 1 da §8 foram superados pela troca para o FP32 e apontando para este documento. As medições de agosto
(feitas com 3.800 acórdãos e o INT8) ficam como registro da época.

## Critério de pronto da Fase 0

- [ ] Tipo do esparso no `auli-core`, com as mesmas opções do denso, sem tocar no `Embedder`
- [ ] Ferramenta de medição que obtém a key pelo `preparar` e confere o `key_hash` contra o pack
- [ ] M6 (determinismo) passa — ou a fase parou e o porquê está registrado
- [ ] Tabelas M1–M5 preenchidas para o `rs`, com as três colunas (denso, esparso, híbrido)
- [ ] Verificação por mutação feita e registrada
- [ ] Impressão lado a lado das consultas do mantenedor (se fornecidas)
- [ ] `RELATORIO-rag-dense-fase0.md` em `docs/`
- [ ] §11 acrescentada ao `ESTUDO-busca-hibrida.md`
- [ ] `cargo fmt`, `clippy -D warnings` e `cargo test --workspace` limpos; nenhum arquivo em `data/` alterado
- [ ] Nenhuma fase seguinte iniciada
