# Relatório — Fase 0 do Estudo-rag-dense: a busca híbrida medida no acervo do RS

Relatório da Fase 0 do [Estudo-rag-dense.md](Estudo-rag-dense.md), de 14/11/2026. Medido na entidade `rs`
(as cinco coleções), numa cópia de laboratório de `data/rs/docs` e `data/rs/packs` tirada em 14/11/2026
(packs de 13/11/2026, `bge-m3-fp32`, `STRATEGY_VERSION` 3, `PACK_FORMAT` 2). Nenhum arquivo em `data/` foi
alterado, nada foi revetorizado, e nenhuma linha do `update`, do motor ou do servidor mudou.

**A fase termina aqui.** As fases 1–4 só começam com decisão do mantenedor.

## A resposta curta

**Para documento procurado por número, a híbrida se paga. Para o termo raro, ela não resolve.**

- **Número de parecer e de acórdão:** o denso de hoje quase não acha documento pelo título. Pelo título
  exato, acerta o próprio documento em 1º em **17,8%** dos pareceres e **4,8%** dos acórdãos. O esparso do
  próprio BGE-M3 acerta **92,1%** e **57,0%**. Fundidos por RRF com profundidade 10, o documento certo fica
  entre os 10 primeiros em **100%** dos pareceres e **81,8%** dos acórdãos (o denso: 50,9% e 18,4%).
  Digitando como quem atende (minúsculas, sem acento), o ganho se mantém: entre os 10 primeiros, **91,6%**
  dos pareceres e **91,1%** dos acórdãos com a fusão, contra 60,4% e 22,3% do denso.
- **FAQs:** a híbrida não piorou nada. Acerto em 1º de 92,8% → 94,8%; 10 FAQs entraram no top 10 e
  nenhuma saiu.
- **Termo raro ("conluio"):** dos 234 acórdãos com a palavra, **217 a têm só no corpo**, fora de qualquer
  índice sobre a key. E o esparso distingue caixa: "conluio" e "CONLUIO" não têm um token em comum. A
  busca literal da aba continua melhor para o termo solto.
- **Custo:** +1,4 GB de memória no servidor, +41 ms por pergunta, +14% de disco, e uma passada a mais do
  modelo no update (da ordem das 2 h 35 min da migração de novembro, uma vez).
- **A fusão precisa ser rasa:** com as 100 primeiras posições de cada lista (a proposta do estudo), a
  fusão joga fora a maior parte do ganho do esparso; com as 10 primeiras, guarda. Nessa profundidade o
  `k` não muda nada.

## Como foi medido

**O encoder esparso** é o tipo `EmbedderEsparso`, em
[`auli-core/src/sparse.rs`](../auli-server/crates/auli-core/src/sparse.rs), separado do `Embedder`: uma
segunda sessão do `fastembed` (`SparseTextEmbedding` com `SparseModel::BGEM3`) sobre os mesmos arquivos do
denso, com as mesmas opções (`max_length` 8.192, `PreferCPU`, 16 threads, `batch_size` 1, `FATIA` de 256).
A única mudança fora do arquivo novo é o `FATIA` do `embed.rs`, que passou a `pub(crate)` para ser
reaproveitado. Nenhum caminho de produção chama o tipo.

**A ferramenta** é o módulo [`auli-cli/src/fase0_hibrida.rs`](../auli-server/crates/auli-cli/src/fase0_hibrida.rs),
compilado só em `cfg(test)`, com três testes `#[ignore]`:

| teste | o que faz |
|---|---|
| `fase0_calcular` | lê a árvore pelo `update::preparar`, confere a key contra o pack, calcula o esparso das keys e grava no laboratório |
| `fase0_medir` | M1–M6 e a impressão lado a lado, lendo os packs e os esparsos do passo anterior |
| `fase0_linha_de_base` | só o M1 do denso, sobre qualquer pack de FAQs (usado no pack de 07/11) |

- **Por que um módulo de teste dentro do crate**, e não um teste em `tests/` como o `ab_faq_pr.rs`: a key só
  nasce no `update::preparar`, que é `pub(crate)`. Abrir a visibilidade faria do `preparar` API pública do
  crate por causa de uma ferramenta de medição; de dentro do crate ele é alcançável sem mudar uma linha do
  `update`, e o `cfg(test)` não chega ao binário.
- **A key é a da produção.** Para cada um dos 8.258 documentos, o `hash_hex(key)` do `preparar` foi
  comparado com o `key_hash` do registro no pack, e os dois conjuntos de ids comparados entre si: tudo
  bateu, nas cinco coleções.
- **Os conjuntos de consulta são automáticos.** O mantenedor não tinha lista de consultas de termo solto
  (14/11/2026); o M3/M4 e a impressão lado a lado usam o "conluio" do estudo de agosto e, para o acento,
  "crédito presumido", o exemplo de ementa do próprio estudo.
- **A ordenação** é a do motor: distância cosseno crescente no denso, pontuação decrescente no esparso
  (só quem tem pontuação acima de zero entra na lista), empate sempre pelo id. A fusão é RRF,
  `Σ 1/(k + posição)`.

## M6 — determinismo (passou)

- Os três testes no molde dos `testes_ordem` do denso (`esparso_independe_da_ordem_e_da_composicao_do_lote`,
  `fatiar_a_entrada_nao_muda_um_unico_esparso`, `esparso_e_deterministico_entre_chamadas`) passam com
  igualdade **bit a bit** de índices e valores.
- Sobre o acervo: 178 documentos (um a cada 47, nas cinco coleções) recalculados em ordem inversa e noutra
  composição de chamada deram o mesmo esparso, bit a bit.

## A linha de base não reproduziu o piloto — e não é o acervo

O estudo manda reproduzir primeiro o 91,2% / 0,9434 do piloto de 07/11 na coluna "só denso". **Não
reproduziu:**

| pack | FAQs | só denso: acerto em 1º | MRR@10 |
|---|---|---|---|
| piloto de 07/11 (`Estudo-fastembed-update-v2.md`) | 2.056 | 91,2% | 0,9434 |
| esta ferramenta, pack de 07/11 (`auli (Copy)`, o da migração) | 2.056 | 92,8% | 0,9547 |
| esta ferramenta, pack de 13/11 (o de hoje) | 2.063 | 92,8% | 0,9546 |

O pack da migração de 07/11, com as mesmas 2.056 FAQs do piloto, dá o mesmo número que o de hoje. A
diferença, portanto, **não vem do acervo**: vem do método da ferramenta do piloto, que nunca foi commitada
(nem no repositório, nem no histórico do git, nem na descrição do PR #219), e cujo critério exato de
consulta e de acerto não está registrado em lugar nenhum. A ferramenta de agora fica no repositório
justamente para que isso não se repita.

A comparação entre as três colunas não depende disso: as três usam as mesmas perguntas, os mesmos packs e o
mesmo critério. E a verificação por mutação (abaixo) mostra que a ferramenta mede o que diz medir.

## M1 — FAQs: a pergunta procura a própria FAQ

2.063 consultas; a consulta é a pergunta (`titulo`) e o acerto é a própria FAQ. 75 perguntas têm texto
idêntico ao de outra FAQ, o que limita o acerto em 1º.

| coluna | acerto em 1º | entre os 10 | MRR@10 |
|---|---|---|---|
| só denso | 92,8% | 99,1% | 0,9546 |
| só esparso | 96,3% | 99,9% | 0,9786 |
| híbrido (RRF k=60, prof. 100) | 94,8% | 99,6% | 0,9686 |
| híbrido (RRF k=60, prof. 10) | 94,8% | 99,9% | 0,9698 |

No top 10, o híbrido trouxe 10 FAQs que o denso não trazia e não perdeu nenhuma.

**Este conjunto favorece o esparso:** a pergunta usada como consulta está escrita literalmente dentro da
key (`P: <pergunta>`). Ele mostra que a híbrida **não atrapalha** as FAQs — a condição do estudo —, e não
que o esparso seja melhor que o denso para a pergunta real, que é uma paráfrase.

## M2 — pareceres e acórdãos pelo título

A consulta é o título (o número) e o acerto é o próprio documento, na própria coleção.

**Pelo título exato** ("PARECER Nº 25148", "ACÓRDÃO 510/24"):

| coleção | coluna | acerto em 1º | entre os 10 | MRR@10 |
|---|---|---|---|---|
| pareceres (381) | só denso | 17,8% | 50,9% | 0,2733 |
| | só esparso | 92,1% | 100,0% | 0,9582 |
| | híbrido (k=60, prof. 100) | 45,7% | 82,7% | 0,5648 |
| | híbrido (k=60, prof. 10) | 47,5% | 100,0% | 0,6645 |
| TARF (4.712) | só denso | 4,8% | 18,4% | 0,0875 |
| | só esparso | 57,0% | 88,0% | 0,6749 |
| | híbrido (k=60, prof. 100) | 17,4% | 46,0% | 0,2567 |
| | híbrido (k=60, prof. 10) | 28,5% | 81,8% | 0,4683 |

No top 10 (prof. 100), o híbrido trouxe 121 pareceres e 1.303 acórdãos que o denso não trazia, e não perdeu
nenhum.

**Pelo título como quem atende digita** — em minúsculas e sem acento ("parecer nº 25148", "acordao
510/24"). É o conjunto que diz se o ganho vale na prática:

| coleção | coluna | acerto em 1º | entre os 10 | MRR@10 |
|---|---|---|---|---|
| pareceres (381) | só denso | 28,6% | 60,4% | 0,3803 |
| | só esparso | 70,3% | 89,2% | 0,7658 |
| | híbrido (k=60, prof. 100) | 48,3% | 83,7% | 0,5998 |
| | híbrido (k=60, prof. 10) | 52,2% | 91,6% | 0,6696 |
| TARF (4.712) | só denso | 8,3% | 22,3% | 0,1236 |
| | só esparso | 60,0% | 95,4% | 0,7324 |
| | híbrido (k=60, prof. 100) | 24,7% | 57,2% | 0,3309 |
| | híbrido (k=60, prof. 10) | 33,8% | 91,1% | 0,5325 |

**O ganho sobrevive à grafia de quem digita.** O número não tem caixa, e é ele que discrimina. Nos
pareceres o esparso cai (92,1% → 70,3% em 1º), provavelmente porque `parecer` em minúsculas casa com a
palavra no texto de todos os pareceres, e `PARECER` só casava com o título; no TARF ele sobe (57,0% →
60,0%), porque `acordao` sem acento não casa com nada e sobra só o número. É a única família em que o
híbrido **perdeu** alvos do top 10 em relação ao denso (6 pareceres, com prof. 100; ganhou 95).

**Por que o híbrido fica abaixo do esparso sozinho.** No RRF, um documento que aparece nas duas listas em
posições medianas (30ª e 30ª: 2/90 = 0,022) passa à frente do 1º do esparso que o denso não trouxe
(1/61 = 0,016). Na consulta por número, a lista do denso é quase ruído, e com profundidade 100 esse ruído
compete. Com profundidade 10 ele quase some.

## A fusão: a profundidade decide; o `k` só importa quando ela é grande

Acerto entre os 10, por profundidade de cada lista antes da fusão, com k = 60:

| profundidade | FAQs | pareceres | TARF |
|---|---|---|---|
| 10 | 99,9% | 100,0% | 81,8% |
| 20 | 99,8% | 100,0% | 80,4% |
| 50 | 99,7% | 77,2% | 66,0% |
| 100 | 99,6% | 82,7% | 46,0% |

**A profundidade 10 é a melhor nas três famílias**, e nas FAQs ela não custa nada. Com profundidade 10 ou
20, o acerto entre os 10 é o mesmo com k = 20, 60 ou 100, e o acerto em 1º varia no máximo 1,4 ponto. Com 50 ou 100, um k menor
atenua a perda (TARF, prof. 100: 72,4% com k = 20, 46,0% com k = 60, 43,0% com k = 100), mas não chega
perto da profundidade 10. Para a D-HIB-3: RRF com as ~10 primeiras posições de cada lista, e não 100.

## M3 — o termo na key × só no corpo

Comparação literal, sem caixa e sem acento:

| termo | coleção | na key | só no corpo |
|---|---|---|---|
| conluio | TARF | 17 | **217** |
| conluio | legislação | 2 | 0 |
| crédito presumido | TARF | 104 | 172 |
| crédito presumido | pareceres | 18 | 6 |
| crédito presumido | FAQs | 87 | 0 |
| crédito presumido | serviços | 10 | 1 |
| crédito presumido | legislação | 1 | 0 |

O caso que originou tudo continua de pé: **93% dos acórdãos com "conluio" (217 de 234) têm a palavra só no
corpo**, fora do alcance do denso, do esparso e da busca literal da aba. (Em agosto eram 40 na key e 324 só
no corpo, sobre o TARF de então; o de hoje é só de 2020 em diante, desde 07/11.) Com termo de vocabulário corrente
como "crédito presumido" a proporção é outra: nas FAQs e nos serviços, quase tudo está na key.

## M4 — caixa e acento separam os tokens

Como o tokenizer do modelo (XLM-RoBERTa) parte cada grafia:

| grafia | tokens (peça=id) |
|---|---|
| `conluio` | ▁con=158 lui=10683 o=31 |
| `CONLUIO` | ▁CON=14452 LU=22335 IO=17780 |
| `crédito presumido` | ▁crédito=52348 ▁presu=44136 mi=266 do=246 |
| `CRÉDITO PRESUMIDO` | ▁CR=25109 É=6642 D=397 ITO=42716 ▁PRE=23096 SUM=127944 IDO=119958 |
| `credito presumido` | ▁credito=96556 ▁presu=44136 mi=266 do=246 |

- **Caixa:** minúsculas e maiúsculas não têm **nenhum** token em comum. A pontuação esparsa entre
  `conluio` e `CONLUIO` é **0**; entre `crédito presumido` e `CRÉDITO PRESUMIDO`, também **0**.
- **Acento:** separa a palavra acentuada (▁crédito ≠ ▁credito), mas o resto casa: 0,0667 contra 0,1188 da
  grafia igual.
- **No acervo**, com a pergunta `crédito presumido`: dos documentos que têm o termo na key **só com outra
  grafia** (ementa em maiúsculas, ou sem acento), **nenhum** entra no top 10 de coluna nenhuma (15 FAQs, 25
  acórdãos). O top 10 fica com os da grafia igual, que são muitos — então isso mostra o efeito, sem medir o
  tamanho dele. Os 17 acórdãos com "conluio" na key têm todos a grafia em minúsculas; mesmo assim, 2 deles
  ficam com pontuação esparsa **zero** para `conluio`: o peso de cada token é contextual (`max(0, h·w+b)`),
  e nesses o modelo zerou os tokens da palavra.

**Consequência:** o esparso cru não substitui a busca literal da aba para o termo solto. A saída seria
normalizar o texto (minúsculas, sem acento) antes do esparso — o que só é possível com uma segunda passada
do modelo sobre outro texto, ou seja, com a D-HIB-1(a); na (b), o esparso sai da mesma passada do denso e
herda a grafia original. Se essa normalização ajuda ou atrapalha um modelo treinado com caixa, é medição a
fazer, não suposição.

## M5 — custo

**Tamanho** (não nulos por documento e o esparso serializado):

| coleção | docs | mín | mediana | P99 | máx | esparso JSON | binário (8 B/entrada) | pack hoje | JSON/pack |
|---|---|---|---|---|---|---|---|---|---|
| serviços | 593 | 16 | 51 | 74 | 77 | 0,5 MB | 0,2 MB | 7,9 MB | 7% |
| FAQs | 2.063 | 39 | 79 | 403 | 620 | 3,6 MB | 1,6 MB | 27,6 MB | 13% |
| legislação | 509 | 36 | 74 | 130 | 199 | 0,7 MB | 0,3 MB | 6,9 MB | 11% |
| pareceres | 381 | 52 | 101 | 185 | 200 | 0,7 MB | 0,3 MB | 5,5 MB | 13% |
| TARF | 4.712 | 16 | 104 | 527 | 712 | 10,9 MB | 5,0 MB | 68,9 MB | 16% |
| **total RS** | 8.258 | | | | | **16,5 MB** | 7,5 MB | 116,8 MB | **14%** |

Nas 27 entidades (532,3 MB de packs hoje), pela proporção do RS: **~75 MB** em JSON, ~34 MB em binário. O
estudo estimava "dezenas de por cento"; é 14%.

**Tempo e memória de calcular o esparso** (`/usr/bin/time -v`):

| recorte | keys | denso (piloto de 07/11) | esparso |
|---|---|---|---|
| as quatro coleções do RS sem o TARF | 3.538 → 3.546 | 12 min 49 s, pico 3,8 GB | **13 min 24 s, pico 3,75 GB** |
| TARF do RS | 4.712 | — | **25 min 14 s, pico 2,87 GB** |

Por key, o esparso custa o mesmo que o denso: o custo é a passada do modelo. A migração das 27 entidades
seria da ordem das 2 h 35 min da revetorização de novembro.

**Memória do servidor** (RSS do processo depois de carregar e usar cada encoder):

| | RSS |
|---|---|
| só o denso | 1,65 GB |
| denso + esparso (segunda sessão, D-HIB-1(a)) | 3,04 GB (**+1,39 GB**) |

**Por pergunta** (mediana, ms):

| família | vetorizar denso | vetorizar esparso | varrer denso | varrer esparso | coleção |
|---|---|---|---|---|---|
| FAQs | 41,8 | 41,1 | 1,36 | 0,55 | 2.063 docs |
| pareceres (título exato) | 34,3 | 33,8 | 0,25 | 0,10 | 381 docs |
| TARF (título exato) | 35,2 | 34,8 | 2,86 | 1,25 | 4.712 docs |
| pareceres (minúsculas) | 34,4 | 33,9 | 0,25 | 0,09 | 381 docs |
| TARF (minúsculas) | 34,3 | 33,9 | 2,85 | 1,09 | 4.712 docs |

Na (a), vetorizar a pergunta passa a custar o dobro (~+40 ms). Varrer o esparso por força bruta custa menos
que varrer o denso — não precisa de índice invertido nesta escala.

## Verificação por mutação

Com `AULI_F0_MUTACAO=1`, cada documento recebe os vetores (denso e esparso) do vizinho. Se a ferramenta
medisse algo além da associação documento ↔ vetor, o número não cairia:

| família | coluna | normal | com a mutação |
|---|---|---|---|
| FAQs | só denso | 92,8% | 0,8% |
| | só esparso | 96,3% | 0,8% |
| | híbrido | 94,8% | 0,6% |
| pareceres | só esparso | 92,1% | 0,0% |
| | híbrido | 45,7% | 3,9% |
| TARF | só esparso | 57,0% | 1,3% |
| | híbrido | 17,4% | 1,2% |

(Acerto em 1º; híbrido com k=60, prof. 100.) Todas as colunas vão ao chão.

A mutação rodou antes de as duas famílias em minúsculas existirem; elas passam pela mesma função de
medição (`medir_familia`), só com outra consulta. O passo 2 sem mutação rodou duas vezes, antes e depois de
elas entrarem, e as famílias comuns deram os mesmos números, célula por célula.

## Lado a lado — `conluio`, top 10 no TARF

✓ = o termo está na key. O esparso pontua 1.071 acórdãos acima de zero para `conluio`: o token `▁con` é o
começo de muitas palavras.

| # | denso | híbrido (k=60, prof. 100) |
|---|---|---|
| 1 | ACÓRDÃO 582/23 — multa por coautoria | ✓ ACÓRDÃO 510/24 — estabelecimento simulado |
| 2 | ACÓRDÃO 581/23 — multa por coautoria | ACORDAO 445/20 — ICMS próprio e de ST não registrado |
| 3 | ACÓRDÃO 275/22 — intempestividade | PLENO 048/20 — pressupostos de admissibilidade |
| 4 | ✓ ACÓRDÃO 510/24 — estabelecimento simulado | ACÓRDÃO 582/23 — multa por coautoria |
| 5 | PLENO 046/20 — intempestividade | ✓ ACÓRDÃO 194/23 — erro de descrição de produto |
| 6 | ACÓRDÃO 615/23 — multa por coautoria | ACÓRDÃO 581/23 — multa por coautoria |
| 7 | ACÓRDÃO PLENO 146/23 — nulidade | ACÓRDÃO 200/25 — pedido de esclarecimento |
| 8 | ACÓRDÃO 027/21 — repetição de indébito | ✓ ACÓRDÃO 097/24 — ITCD, multa |
| 9 | ACÓRDÃO 616/23 — multa por coautoria | ACÓRDÃO 275/22 — intempestividade |
| 10 | ACÓRDÃO 605/20 — recurso intempestivo | ✓ ACÓRDÃO 341/23 — grupo econômico |

Um ✓ no denso, quatro no híbrido, de 17 possíveis. Melhora, mas não é a busca literal. Nos pareceres
nenhum documento tem "conluio" na key, e as duas listas são ruído.

## Como ler o resultado — a tabela do estudo aplicada

| condição do estudo | resultado | leitura |
|---|---|---|
| M1 cai e M2 não sobe | não: M1 subiu, M2 subiu muito | — |
| **M2 sobe e M1 se mantém** | **sim**, também com o título digitado em minúsculas | **há caso para as fases 1–4; M5 decide D-HIB-1 e D-HIB-2** |
| M2 sobe só na jurisprudência e M1 cai nas FAQs | M1 não caiu | não é preciso ligar por `Kind` (D-HIB-5) |
| **M3 mostra que quase tudo está só no corpo** | **sim, para o "conluio"** (93%) | o ganho percebido no termo raro será pequeno; o estudo do índice do corpo continua necessário |
| **M4 mostra que caixa e acento separam os ids** | **sim** | o esparso cru não substitui a busca literal; ou se normaliza o texto antes do esparso, ou a opção (a) de agosto (o chat apontar a aba) segue a correção mais barata para o termo solto |

As três leituras valem ao mesmo tempo, porque respondem a perguntas diferentes: a híbrida é a correção para
**identificador na key**, e não é a correção para **termo raro** nem para **grafia diferente**.

## O que os números dizem sobre as decisões em aberto

- **D-HIB-1 (de onde sai o esparso).** A (a) custa +1,39 GB no servidor, +~40 ms por pergunta e uma passada a
  mais no update. A (b) evitaria os três, mas exige vendorizar os pesos e provar o denso bit a bit — mais
  barata do que o estudo diz, porque o denso de hoje é a saída `sentence_embedding` do próprio ONNX, e não
  um pooling feito pelo `fastembed` (ver abaixo). **O M4 pesa a favor da (a):** se o esparso precisar de
  texto normalizado, ele não pode sair da mesma passada do denso.
- **D-HIB-2 (onde o esparso mora).** 14% do pack em JSON. Nenhuma das duas opções é pressionada pelo
  tamanho; o arquivo irmão mantém o store agnóstico e o rollback trivial.
- **D-HIB-3 (fusão).** RRF, com **profundidade ~10** em vez de 100; nessa profundidade o `k` é indiferente
  (60 serve).
- **D-HIB-4 (o que o `score` significa).** Não medido aqui; continua a decidir antes da Fase 3.
- **D-HIB-5 (por coleção).** Não é necessário: a híbrida não piorou as FAQs.

## Achados que o estudo não registrava

1. **O ONNX FP32 tem duas saídas, nesta ordem: `token_embeddings` e `sentence_embedding`** (lido do
   protobuf do `model.onnx`). O esparso do `fastembed` pega a primeira saída (por token), então funciona —
   mas por causa da ordem; outro export do modelo pode trocá-la em silêncio. O denso de produção não é um
   pooling CLS calculado pelo `fastembed`: ele procura a saída chamada `sentence_embedding`, e o
   `Pooling::Cls` só repassa o tensor 2D que ela já traz.
2. **O modelo FP32 mora em `models/` na raiz do repositório**, não em `auli-server/models/`, que só guarda
   a pasta antiga `models--gpahal--bge-m3-onnx-int8` (sobra da migração; não foi mexida).
3. **A ferramenta do piloto de 07/11 se perdeu** (ver a linha de base). Os números do piloto não são mais
   reproduzíveis por ninguém.
4. **O peso esparso é contextual:** 2 dos 17 acórdãos que têm "conluio" em minúsculas na key ficam com
   pontuação zero para `conluio`. Ter a palavra na key não garante o casamento.

## Reproduzir

Da raiz de `auli-server/`, com uma cópia de laboratório `<lab>/rs/{docs,packs}`:

```bash
export AULI_F0_ENTIDADE=<lab>/rs AULI_F0_SAIDA=<lab>/saida EMBED_CACHE_DIR=../models
cargo test -p auli-core --release sparse -- --include-ignored --test-threads=1           # M6
cargo test -p auli-cli --release --lib --no-run                                          # imprime o binário
AULI_F0_KINDS=servicos,faqs,pareceres,legislacao /usr/bin/time -v <binário> fase0_hibrida::fase0_calcular --exact --ignored --nocapture
AULI_F0_KINDS=tarf /usr/bin/time -v <binário> fase0_hibrida::fase0_calcular --exact --ignored --nocapture
<binário> fase0_hibrida::fase0_medir --exact --ignored --nocapture                     # ~17 min
AULI_F0_MUTACAO=1 <binário> fase0_hibrida::fase0_medir --exact --ignored --nocapture   # a mutação
```

O passo 1 leva ~39 min no RS; os esparsos ficam em `<lab>/saida` (16,5 MB) e nunca em `data/`.
