# ESTUDO — busca híbrida (denso + esparso) e calibragem das bandas

**Data:** 08/08/2026
**Origem:** o mantenedor pesquisou "conluio" no chat do TARF e não veio nada, embora **69 acórdãos** do
acervo contenham a palavra.
**Status:** medido, não decidido. Nenhuma linha de código foi mudada por causa disto.

---

## 1. O que foi verificado primeiro (e descartado)

Antes de concluir qualquer coisa sobre o mecanismo de busca, três hipóteses mais baratas foram
testadas e eliminadas:

| Hipótese | Verificação | Resultado |
|---|---|---|
| O documento não foi coletado | `ACÓRDÃO 109/26` está em `data/rs/docs/tarf/` | está lá |
| O documento não entrou no pack | busca por `numero` em `rs-tarf.json` | está lá, `doc_path` correto |
| O `resumo` não é indexado | `compose_text_to_embed(titulo, ementa, resumo)` | **é indexado** — o resumo é o 3º slot |
| O texto foi truncado no encoder | resumo de 3.169 chars contra teto de 8.192 **tokens** | passa inteiro, com folga |

A prova de que o resumo é indexado veio de outro lado, e é independente: numa busca por
*"decadência do direito de constituir o crédito tributário"*, o primeiro resultado foi o
`ACÓRDÃO Nº 1328/11`, cuja **ementa não menciona decadência** (`ICMS. CRÉDITO FISCAL. PRESUMIDO.
LIMITE DE CREDITAMENTO…`). O que casou foi o resumo, que abre com *"Inicialmente afasto a tese de
decadência arguida pela parte…"*. Sem o resumo no índice, aquele acórdão não teria sido achado.

## 2. A causa real: a busca é densa, e termo raro solto é o pior caso dela

O `auli update` chama `embedder.embed_dense(...)` — **só o componente denso** do BGE-M3. Busca densa
compara *sentido*, não palavra. O vetor de uma palavra solta e rara não se alinha com um documento
longo cujo assunto geral é outro; "conluio" aparece uma vez, no meio da fundamentação de um acórdão
que trata de desistência de recurso em omissão de saídas de arroz.

Medido contra o acervo real (3.800 acórdãos, `top_k` 5–10, `PAR_BAND = ∞`):

| Consulta | Melhor score (distância) | `ACÓRDÃO 109/26` aparece? |
|---|---|---|
| `conluio` | 0,5537 | não — nenhum dos 69 entrou no top-10 |
| `conluio entre empresas para omitir saídas e sonegar ICMS` | 0,4431 | não |
| `responsabilidade solidária por conluio na omissão de saídas de arroz` | 0,3634 | **sim, em 5º** (0,4008) |
| *(referência de casamento bom)* `decadência do direito de constituir o crédito tributário` | 0,2833 | — |

Duas leituras:

1. **A faixa 0,55–0,58 é o sistema dizendo "não achei nada próximo".** Os dez resultados da palavra
   solta ficaram todos espremidos ali, sem separação entre eles — a assinatura de "nenhum destes é
   melhor que os outros", que é diferente de um acervo que responde (0,28 no topo, com degrau).
2. **A busca funciona quando a pergunta tem contexto.** Com a frase inteira, o documento certo
   entrou no top-5. Não é o índice que está cego; é a consulta de uma palavra que não dá ao modelo
   o que ele usa.

## 3. Consequência prática hoje

`PAR_FLOOR`/`SVC_FLOOR`/`FAQ_FLOOR` são `0` e as três bandas são `f32::INFINITY`
(`auli-cli/src/rag.rs`). Isso significa que **o chat recebe `n_results` documentos sempre**, por pior
que seja o casamento. Na busca por "conluio" o LLM recebeu dez acórdãos a 0,55 de distância — nenhum
sobre o tema — e concluiu, corretamente, que não dava para responder.

O comportamento observado ("não encontrei") está certo. O que está errado é o **caminho**: o modelo
teve de descartar dez documentos irrelevantes em vez de receber contexto vazio, e o log de aderência
registra dez entradas como se fossem candidatos legítimos.

## 4. Onde a busca por palavra JÁ funciona

A aba **Acórdãos TARF** do frontend faz busca **literal** (`buildPareceresIndex`/`searchPareceres`),
insensível a acento, multi-termo com E, sobre `[numero, assunto, resumo]` — exatamente os campos onde
"conluio" está. Digitar `conluio` lá devolve os 69.

Ou seja: hoje o produto **tem** as duas buscas, mas separadas por interface — a aba faz léxica, o
chat faz semântica. Quem não sabe disso encontra o vazio do chat e conclui que o acervo não tem o
documento. Isso é, no mínimo, um problema de descoberta, e pode ser resolvido antes e mais barato do
que a busca híbrida.

## 5. Viabilidade da busca híbrida

**O componente esparso está disponível sem trocar de dependência.** O `fastembed 5.17.2`, a versão
que já está travada no `Cargo.lock`, expõe `SparseTextEmbedding` com `SparseModel::BGEM3` — o mesmo
modelo, produzindo pesos lexicais em vez de um vetor denso.

O obstáculo não é a API, é o **arquivo do modelo**:

- O denso de hoje é `gpahal/bge-m3-onnx-int8` (`EMBED_MODEL_ID = "bge-m3-q-int8"`), **560 MB**
  quantizado, e é ele que o manifesto carimba.
- O `SparseModel::BGEM3` do fastembed aponta para `BAAI/bge-m3` com `onnx/model.onnx` +
  `model.onnx_data` — o modelo **fp32 completo**, alguns GB, e um segundo download.
- Há uma saída: `UserDefinedSparseModel::new(onnx_file, tokenizer_files)` aceita um ONNX arbitrário.
  Se o nosso int8 expuser o tensor de hidden states que o `post_process_bgem3` consome, dá para
  reusar o arquivo que já temos. **Isto precisa ser verificado — é a primeira pergunta a responder,
  e ela decide o custo da fase inteira.**

### O que a híbrida custa, além do modelo

- **Formato do pack.** Cada registro passa a carregar dois vetores (denso + esparso). Mexe no
  `vector-store` e no tamanho dos packs.
- **`STRATEGY_VERSION` sobe** (hoje `1`), e com ela **todos os 28 packs são re-vetorizados**. O
  update do rs sozinho já leva 16 min e pico de 16,4 GB de RSS (medido na Etapa A); o do SP é maior.
- **Fusão dos rankings.** Denso e esparso produzem escalas diferentes; precisa de uma regra de
  combinação (RRF é o padrão simples) e de recalibrar as bandas em cima dela.
- **A aderência do log muda de unidade**, e as travas que pinam o formato vão junto.

## 6. Opções, do mais barato ao mais caro

**(a) Descoberta — apontar a aba quando o chat não acha.** Quando o contexto sai vazio (ou todo
acima de um limiar), a resposta do chat sugere a busca literal da aba correspondente. Custo baixo,
resolve o caso que originou este estudo do ponto de vista do usuário, e não toca em vetor nenhum.

**(b) Calibrar as bandas.** Rodar perguntas reais, ler os arrays de score que o
`search_embedded` já registra, e baixar cada banda para logo acima de onde os casamentos genuínos se
separam do enchimento. Não acha o documento, mas faz a falha ser honesta: contexto vazio em vez de
dez distratores, e o log passa a mostrar o corte. **Pré-requisito de (c)** — sem banda calibrada não
há como medir se a híbrida melhorou.

**(c) Busca híbrida.** A correção de verdade para termo raro. Só depois de (b), e só depois de
responder a pergunta do modelo int8 na §5.

## 7. O que este estudo NÃO afirma

- Não afirma que a busca densa está mal configurada. Ela responde bem a perguntas com contexto — é
  para isso que serve.
- Não afirma que os 69 acórdãos com "conluio" seriam todos relevantes para quem digitou a palavra.
  A busca literal devolve ocorrências, não pertinência; parte delas está em negativas
  ("nega-se qualquer conluio").
- Não mediu **pareceres**, **serviços** nem **faqs** — todas as medições são do `rs-tarf`. Os
  resumos das outras coleções têm perfil diferente (mediana de 1.172 chars nos pareceres, 745 no
  TARF, mas com máximo de 27.077 no TARF contra 1.689 nos pareceres), e a conclusão pode não
  transferir.

## 8. Próximo passo, se for adiante

1. Verificar se o `UserDefinedSparseModel` aceita o `model_quantized.onnx` que já está em `models/`.
   Um teste de meia hora que decide se a fase custa "somar um vetor" ou "baixar e servir um segundo
   modelo de vários GB".
2. Rodar (b) — calibragem — como fase independente, que tem valor sozinha.

## 9. Decisão: adiada, com a base sendo acumulada (09/08/2026)

**A calibragem das bandas fica para depois.** O critério do mantenedor: observar as respostas por mais
tempo, até as mudanças de código estabilizarem e haver base empírica de verdade. Escolher o corte
por intuição é o mesmo erro de estimar sem medir.

A boa notícia é que **a base se acumula sozinha** — não vai ser preciso montar uma campanha de
perguntas quando a hora chegar. Cada conversa grava um `.txt` em `logs/` com as quatro seções que a
calibragem precisa:

```text
----- PERGUNTA (ORIGINAL) / (ANONIMIZADA)
----- RESPOSTA
----- ADERÊNCIA (proximidade da pergunta)     ← acórdão tarf 1 · aderência 0.508 · distância 0.492
----- CONTEXTO RAG (documentos recuperados)
```

A `ADERÊNCIA` existe desde **02/08/2026**; em 09/08 são **107 logs** com ela, de 307 no total. Como
guarda o par (pergunta, distância de cada documento) **e** a resposta, dá para separar depois o
casamento genuíno do enchimento sem ter de re-rodar nada. É exatamente o procedimento que o
comentário do `rag.rs:29` já prescrevia.

> **Ressalva metodológica — os logs de hoje não servem para calibrar o `tarf`.** A distância
> pergunta↔documento não depende do tamanho do acervo, mas a distância do **melhor** resultado
> depende: com 22.590 acórdãos em vez de 3.800, o topo do ranking melhora, e o degrau que separa
> casamento de enchimento se desloca. A janela de observação que vale para o `tarf` começa quando a
> coleta fechar. Para `pareceres`, `servicos` e `faqs` — acervos estáveis — os logs já contam.

---

## 10. O problema é ~9× maior do que este estudo supôs — e é estrutural (11/08/2026)

Levantado de raspão pela Fase 0 da `TAREFA-TETO-TEXT-TO-EMBED`, ao medir onde o termo cai dentro do
`text_to_embed`. Dois números, os dois verificáveis hoje:

| onde | acórdãos com "conluio" |
|---|---:|
| no `text_to_embed` (= `[numero, assunto, resumo]`, o que é embedado E indexado) | **40** |
| no arquivo `.md`, em qualquer lugar | **364** |

Os 324 restantes têm a palavra **exclusivamente na seção `## corpo`** — o texto integral do acórdão,
que **não é embedado nem entra no índice da aba**. Para esses documentos, "conluio" nunca esteve
buscável, nem pela busca densa do chat nem pela busca literal do frontend.

**Isso reposiciona o diagnóstico deste estudo.** A §2 atribui o "não veio nada" ao ponto fraco da
busca densa com termo raro solto, e isso continua verdadeiro para os 40. Mas a maior parte do acervo
que contém a palavra está fora do alcance das **duas** buscas por uma razão anterior: o que se indexa
é a síntese, não o documento. Nenhuma calibragem de banda e nenhuma busca híbrida alcança conteúdo
que não está no índice.

**O que NÃO muda:** o teto de 6.000 caracteres da TAREFA-TETO não causa nem agrava isto. Dos 40
documentos que a busca pode ver, o teto corta 6 e apenas **1** perde todas as ocorrências do termo —
a primeira ocorrência tem mediana no caractere 492, porque o termo vive no começo.

**Consequência para a §6 (opções):** indexar o corpo — por chunking ou por um índice léxico sobre o
`## corpo` — passa a ser uma opção de natureza diferente das outras, porque é a única que ataca os
324. As demais disputam a ordenação dos 40.

> **Nota de procedência.** A abertura deste estudo diz "69 acórdãos". Esse número **não se reproduz**
> em 11/08/2026: o `rs-tarf-index.json` que a aba consome tem 40, e a recomposição independente do
> `text_to_embed` a partir da árvore deu 40 também. Dois métodos independentes concordam; o 69 fica
> registrado como número sem procedência conhecida, e não foi substituído por uma explicação
> inventada.
