# RELATÓRIO — o teto do `text_to_embed`, da medição à verificação

**Para:** Claude Fable · **Origem:** `TAREFA-TETO-TEXT-TO-EMBED` (removida; executada)
**Estado:** fechado. `TETO_TEXT_TO_EMBED = 6.000`, `STRATEGY_VERSION = 3`, mesclado pelo PR #145,
27 entidades regeneradas em 12/08.

Este documento é a fusão dos dois relatórios da tarefa — o da **Fase 0**, que mediu antes de você
escolher o N, e o das **Fases 1 a 3**, que implementou e verificou. Ficaram juntos porque separados
obrigavam o leitor a reconciliar duas medições da mesma coisa: a Fase 3 corrige duas leituras da
Fase 0 (o que a coluna "termo quadrático previsto" prometia, e a razão chars/token), e essas
correções só fazem sentido ao lado do que corrigem.

**A ordem é cronológica de propósito**, porque a cronologia é o argumento: mediu-se, você escolheu,
implementou-se, verificou-se contra o que foi medido. As §§1 a 6 são de 11/08 (a §6, de 12/08); as
§§7 a 10, de 12/08.

**Os dois resultados, na primeira linha.** O corte é barato onde é raso e caro onde é fundo, e a
degradação é previsível pela razão *cortado/total* — não há regra binária (§1). E o pack novo difere
do antigo **exatamente** nos documentos cortados, nas quatro coleções, como igualdade de conjuntos e
não como contagem (§8).

---

# Fase 0 — a medição que precedeu a escolha do N

**Portão da entrega:** as duas medições feitas, **parado ali**. Nenhuma linha de código de
produção alterada, nem para medir — a 0.a rodou num binário descartável **fora do
repositório**, com dependência de caminho para o `auli-core` (296 vetores, ~90 s).

## 1 — (0.a) A cauda está sendo representada? Depende de quanta cauda

Cosseno entre o vetor do documento inteiro e o dos seus primeiros N caracteres. Amostra: **todos** os
14 documentos das FAQs acima de 6.000 chars e **60 dos 404** do TARF (sistemática, 1 em 6).

| faixa de comprimento | n | cos mediano em N=6.000 | mín |
|---|---:|---:|---:|
| 6.000 – 8.000 | 31 | **0,978** | 0,960 |
| 8.000 – 12.000 | 22 | **0,959** | 0,922 |
| 12.000 – 20.000 | 17 | **0,916** | 0,820 |
| acima de 20.000 | 4 | **0,662** | 0,636 |

**Nenhum dos dois ramos da sua regra se aplica ao conjunto**, e é por isso que a leitura tem de ser
por faixa: cortar 25% de um documento é praticamente grátis (0,978); cortar 80% destrói o vetor
(0,662). A degradação é monótona e previsível pela razão *cortado/total*, não pelo N em si.

Os três piores casos são os três maiores documentos do acervo:

```text
acordao-651-19.md      26.987 chars   cos 0,6357
acordao-no-1428-11.md  32.722 chars   cos 0,6514
acordao-483-20.md      28.780 chars   cos 0,6720
```

**E aqui está o nó da decisão.** Os documentos cujo vetor o corte destruiria são **exatamente** os
que fixam o pico de memória — porque o pico é quadrático no maior texto. Não é coincidência: é a
mesma cauda vista por duas métricas. Cortar em 6.000 troca **16,6 GiB de pico** pelos vetores de:

| | acima de 20.000 chars | 12.000–20.000 | 8.000–12.000 | 6.000–8.000 |
|---|---:|---:|---:|---:|
| TARF (22.476) | **22** (0,10%) | 89 (0,40%) | 131 (0,58%) | 162 (0,72%) |
| FAQs (1.947) | **1** (0,05%) | 1 (0,05%) | 6 (0,31%) | 6 (0,31%) |
| efeito no vetor | destruído (0,66) | sensível (0,92) | leve (0,96) | nulo (0,98) |

**23 documentos** concentram o dano. Eles são 0,09% do acervo — e são a razão inteira dos 16,6 GiB.

**Uma ressalva de leitura, para o número não ser lido como pior do que é:** o cosseno mede quanto o
vetor *se move*, não se o documento deixa de ser encontrável. A referência do `ESTUDO-busca-hibrida`
para "nada próximo" é 0,55–0,58; um documento a 0,66 do próprio vetor original ainda é mais próximo
dele do que de um documento aleatório. O que se perde é **precisão de posicionamento**, não a
existência do documento no índice.

## 2 — (0.b) O teste do conluio: o teto quase não custa aqui

| teto | dos 40 docs com o termo, quantos seriam cortados | quantos perderiam **todas** as ocorrências |
|---|---:|---:|
| 4.000 | 7 | **2** |
| 6.000 | 6 | **1** |
| 8.000 | 5 | **0** |

Posição da primeira ocorrência dentro do `text_to_embed`: **mínima 59, mediana 492, máxima 7.820**.
O termo vive no começo — no número, no assunto ou na abertura da fundamentação —, não na cauda.

**Isso responde à sua preocupação, e no sentido tranquilizador:** um teto de 6.000 custa **um**
documento entre 40 para esse termo. O caso que já era o pior da busca densa **não** é o que o teto
piora.

## 3 — O dial, com os números reais

| N | cortados no TARF | cortados nas FAQs | dos cortados, < 0,90 | termo quadrático previsto |
|---|---:|---:|---:|---:|
| 4.000 | 902 (4,0%) | 46 (2,4%) | 27% | ~240 MiB |
| 6.000 | 404 (1,8%) | 14 (0,7%) | 14% | ~540 MiB |
| 8.000 | 242 (1,1%) | 8 (0,4%) | 11% | ~960 MiB |
| *hoje* | *0* | *0* | *0* | *16.637 MiB (medido)* |

(o `< 0,90` é medido sobre a amostra de documentos acima de 6.000 chars; documentos entre N e 6.000
são cortes rasos e ficam acima de 0,96)

Vale a sua ressalva sobre o coeficiente não valer extrapolado para baixo: o `rs-pareceres`, com
máximo de 1.807 chars, gastou 229 MiB onde a lei previa 49. **Prometa entre 540 MiB e ~1 GiB**, como
você mesmo escreveu — a previsão da coluna é o termo quadrático, não o pico.

## 4 — (d) Suposições da TAREFA que não bateram

**Os 69 acórdãos com "conluio" não se reproduzem hoje — são 40.** Não é discrepância de método: o
`rs-tarf-index.json`, o próprio artefato que a aba do frontend consome, tem **40** documentos com o
termo em `[numero, assunto, resumo]` (7 no assunto, 36 no resumo, com sobreposição). A minha
recomposição do `text_to_embed` deu o mesmo 40, independentemente. O número que existe e é grande é
outro: **364 arquivos** contêm a palavra em algum lugar — todos eles na seção `## corpo`, que **não é
embedada nem indexada**. Não sei de onde veio o 69 do `ESTUDO-busca-hibrida` de 08/08 e não vou
escolher a explicação menos ruim; registro que hoje os números verificáveis são 40 e 364.

Isso não muda a conclusão da 0.b — muda o denominador, e o argumento fica **mais** forte: dos 40 que
a busca pode ver, o teto de 6.000 tira um.

**"~4% do TARF, ~2% das FAQs" é o perfil de um teto de 4.000, não de 6.000.** Em 6.000 são 1,8% e
0,7%. A estimativa estava deslocada em um degrau do dial.

**Confirmado:** 0% dos `rs-pareceres` seriam cortados por qualquer N ≥ 2.000 — o máximo deles é 1.807.

## 5 — O que eu não fiz, e o que falta

Não escolhi o N: é seu, e a escolha agora tem os dois eixos quantificados. Não implementei corte, não
toquei em `FATIA`, `batch_size` ou `EMBED_MAX_TOKENS`, e não desenhei chunking — embora valha
registrar que a Fase 0.a produziu, sim, resultados abaixo de 0,90 numa minoria concentrada, que é a
condição que a sua §"O que NÃO fazer" associa a "o remédio é dividir, não cortar". **Essa minoria são
23 documentos.** Se dividir 23 documentos for barato, talvez não seja preciso escolher entre as duas
coisas — mas isso é desenho, é seu, e é outra TAREFA.

O harness da 0.a está fora do repositório e é reaproveitável para qualquer N que você queira medir
antes de decidir.

---

## 6 — Os 23, enumerados por `doc_path` (acrescentado em 12/08)

**Dívida paga com atraso.** Você pediu a enumeração como conjunto nomeado e o relatório original
trouxe só a contagem e três exemplos. A lista existe para o A/B de retrieval: são os documentos cujo
vetor o teto de fato move (cosseno mediano 0,66), e a pergunta que o A/B responde é se eles pioraram.

Recomposição do `text_to_embed` **sem o teto**, pela fórmula do contrato — `compose_text_to_embed`
para a jurisprudência e `compose_faq_text_to_embed` para as FAQs. Validada contra os três valores já
publicados na §1 deste relatório (26.987, 32.722 e 28.780), que batem exatamente.

| # | chars | `doc_path` |
|---:|---:|---|
| 1 | 32.722 | `docs/tarf/acordao-no-1428-11.md` |
| 2 | 31.279 | `docs/tarf/acordao-no-286-14.md` |
| 3 | 29.526 | `docs/tarf/acordao-326-20.md` |
| 4 | 28.781 | `docs/tarf/acordao-481-20.md` |
| 5 | 28.780 | `docs/tarf/acordao-483-20.md` |
| 6 | 28.778 | `docs/tarf/acordao-482-20.md` |
| 7 | 28.417 | `docs/tarf/acordao-no-1194-10.md` |
| 8 | 28.037 | `docs/tarf/acordao-no-436-11.md` |
| 9 | 27.686 | `docs/tarf/acordao-no-932-10.md` |
| 10 | 27.534 | `docs/tarf/acordao-443-20.md` |
| 11 | 26.987 | `docs/tarf/acordao-654-19.md` |
| 12 | 26.987 | `docs/tarf/acordao-653-19.md` |
| 13 | 26.987 | `docs/tarf/acordao-652-19.md` |
| 14 | 26.987 | `docs/tarf/acordao-651-19.md` |
| 15 | 26.981 | `docs/tarf/acordao-737-19.md` |
| 16 | 25.120 | `docs/faqs/7-atualizacao-da-legislacao-do-pit-07abbe50.md` |
| 17 | 24.341 | `docs/tarf/acordao-738-20.md` |
| 18 | 23.984 | `docs/tarf/acordao-no-355-14.md` |
| 19 | 23.138 | `docs/tarf/acordao-do-pleno-no-057-09.md` |
| 20 | 21.707 | `docs/tarf/acordao-no-420-05.md` |
| 21 | 21.676 | `docs/tarf/acordao-no-421-05.md` |
| 22 | 21.519 | `docs/tarf/acordao-no-257-11.md` |
| 23 | 21.209 | `docs/tarf/acordao-457-21.md` |

**Vinte e dois dos 23 são acórdãos; uma única FAQ entra** — a nº 16, que é o documento removido no
teste de falseamento da TAREFA-MEMORIA e sozinho respondia por 35% do pico das FAQs.

**Quatro deles têm o comprimento idêntico** (11 a 14: `651` a `654-19`, 26.987 chars) e um quinto
quase (`737-19`, 26.981). São acórdãos irmãos com a mesma fundamentação — o que importa para o A/B:
eles não são cinco evidências independentes, são uma repetida cinco vezes, e uma amostra que os trate
como cinco superestima a confiança em qualquer conclusão sobre eles.

---

# Fases 1 a 3 — a implementação e a verificação

**Estado na entrega:** N = 6.000 implementado, `STRATEGY_VERSION` em 3, entidade de verificação
rodada de ponta a ponta — mesclado em `main` pelo PR #145. **As 27 entidades não foram rodadas** —
eram suas, e foram rodadas em 12/08 (§10).

**Revisado em 12/08, depois da sua leitura:** duas correções (§7 e §9), e a remoção do `conta_tokens`
com o número preservado onde a decisão mora.

## 7 — (c) A entidade de verificação

`rs`, num diretório separado (`/home/ubu/verif-teto`) com o `docs/` hardlinkado e o `packs/` vazio —
sem manifesto anterior, portanto **cache desligado e re-vetorização integral das 25.381 keys**. É a
guarda da Fase 5 da TAREFA-UPDATE-INCREMENTAL sendo exercida de verdade, como você previu: o
`update` reconheceu sozinho que a identidade mudou e não reaproveitou nada.

```text
🧊 Cache de vetores DESLIGADO — não há manifesto anterior, ou a identidade de embedding mudou.
🔢 servicos:   586 registros — 0 reaproveitados, 586 a vetorizar (reescrita total)
🔢 faqs:     1.947 registros — 0 reaproveitados, 1.947 a vetorizar
✂️  faqs: 14 de 1947 keys no teto de 6000 chars — a cauda não entra no vetor
🔢 pareceres:  372 registros — 0 reaproveitados, 372 a vetorizar
🔢 tarf:    22.476 registros — 0 reaproveitados, 22.476 a vetorizar
✂️  tarf: 404 de 22476 keys no teto de 6000 chars — a cauda não entra no vetor
```

Duração: 54 minutos na janela amostrada (o coletor de RSS entrou logo depois do `servicos`, que é a
coleção mais barata; as três seguintes estão inteiras).

### O pico medido contra o previsto

| | pico | base | Δ |
|---|---:|---:|---:|
| **medido, com teto** | **3.531 MiB** | 1.091 | 2.440 |
| previsto (base + termo quadrático em 6.000) | ~1.631 MiB | 1.091 | 540 |
| medido, sem teto, `rs` como roda em produção (TARF do cache) | 10.720 MiB | | |
| medido, sem teto, TARF vetorizando de fato | 17.724 MiB | | |

**O número honesto de comparação é 17.724 → 3.531, isto é 5,0×**, não os 3,0× que sairiam do 10.720.
O 10.720 foi medido com o TARF vindo do cache; um bump de `STRATEGY_VERSION` proíbe exatamente esse
atalho, então o pico que a Fase 3 evita é o de re-vetorização, e é esse que caiu cinco vezes.

**A previsão do dial ficou ~1.900 MiB curta, e o resíduo ainda não tem dono.** A coluna "termo
quadrático previsto" da §3 nunca foi previsão de pico — você mesmo escreveu isso ao
pedir que eu prometesse "entre 540 MiB e ~1 GiB" só para o termo.

> **Correção (12/08), depois da sua leitura.** A primeira versão deste parágrafo dizia que o resíduo
> era do `FATIA`, com 7% de erro. **Isso estava errado, e o erro é meu:** eu apliquei a fórmula do
> colbert só ao caso em que ela batia. Aplicada ao `sp-pareceres`, que rodou sem teto, ela prevê
> ~4,7 GiB contra um `Δ` total medido de 2.057 MiB — mais que o dobro do consumo inteiro da coleção.
> **Como enunciada, a fórmula está refutada**, e o segundo sinal que eu mesmo tinha registrado aponta
> na mesma direção. O candidato barato para o resíduo é o pipeline de pack, já medido em ~620 MiB
> para o TARF numa rodada em que ele nem vetorizou. A nota do `RELATORIO-MEMORIA-UPDATE.md` foi
> reescrita para registrar a tensão, não o reforço.
>
> O que a razão de 2,17 **confirma** é o outro termo: `16 cabeças × seq² × 4 bytes` dá
> 1,3×10⁻⁵ MiB/char² contra os 1,46–1,96×10⁻⁵ medidos. O quadrático deixa de ser lei empírica e
> passa a ter mecanismo.

Pico por coleção (num único processo, então é marca d'água acumulada — não são Δ isolados
comparáveis aos da TAREFA-MEMORIA): faqs 2.659, pareceres 2.668, TARF 3.531.

### O teste decisivo, pack a pack

Comparação id a id do `embedding` contra os packs de produção (strategy 2):

| pack | registros | ids iguais | vetores diferentes | docs acima do teto | são os mesmos? |
|---|---:|:---:|---:|---:|:---:|
| `rs-servicos` | 586 | ✓ | **0** (byte a byte idêntico) | 0 | — |
| `rs-pareceres` | 372 | ✓ | **0** (byte a byte idêntico) | 0 | — |
| `rs-faqs` | 1.947 | ✓ | **14** | 14 | **sim** |
| `rs-tarf` | 22.476 | ✓ | **404** | 404 | **sim** |

Nos dois casos com corte a igualdade é de conjuntos: zero vetores mudaram sem estar cortados, zero
cortados deixaram de mudar. E os dois packs sem nenhum documento longo saírem **byte a byte
idênticos** depois de uma re-vetorização integral prova duas coisas de uma vez — que o corte não
vazou para onde não devia, e que o embedder é determinístico no caminho que o `batch_size = 1`
garante.

---

## 8 — (b) O corte, os testes de fórmula congelada, o `relatar_teto`

**O corte** mora na `compose_unificado`, no `auli-contract` — um ponto só, o mesmo por onde as quatro
coleções passam. `TETO_TEXT_TO_EMBED = 6_000`, em **caracteres** (`chars`, não bytes: cortar
acentuação ao meio produziria UTF-8 inválido).

**Os testes de fórmula congelada não sumiram — passaram a afirmar equivalência até o teto**, como
você pediu. Eles agora roteiam pelo helper `ate_o_teto`, e ganharam casos longos: a fórmula continua
byte a byte a mesma para todo texto abaixo de 6.000, e o teto é afirmado à parte, por um teste novo
(`o_teto_corta_em_chars_pelo_fim_e_preserva_os_slots_densos`) que verifica que o corte vem **do fim**
e que os campos densos do começo — os que a busca mais usa — sobrevivem inteiros.

**O `avisar_truncados` virou `relatar_teto`**, e mudou de sentido em três pontos:

- **não chama mais o tokenizer.** O aviso antigo perguntava ao encoder quantos tokens cada item
  tinha; agora o critério é `chars() >= TETO`, que é a mesma condição que o corte usa. O aviso deixa
  de poder discordar do que aconteceu de fato;
- **conta todas as keys, não só as vetorizadas.** Antes, com o cache ligado, um item reaproveitado
  não era contado, e o número dependia do estado do cache em vez do acervo;
- **é estatística, não item a item.** Uma linha por coleção com "quantos de quantos".

Os números que ele imprimiu — 14 de 1.947 e 404 de 22.476 — batem com os da §3 na unidade.

**Suíte:** tudo verde no workspace, clippy limpo. Cada defeito consertado nesta linha tem um teste
que falha quando o defeito volta.

---

## 9 — O `conta_tokens`, gasto na verificação como você pediu

Contagem real de tokens sobre as 25.381 keys **já com o teto aplicado**, pelo tokenizer do próprio
BGE-M3:

| coleção | n | P50 | P99 | máx | teto do encoder | no teto |
|---|---:|---:|---:|---:|---:|---:|
| `rs-faqs` | 1.947 | 216 | 1.452 | 2.769 | 8.192 | **0** |
| `rs-pareceres` | 372 | 371 | 517 | 547 | 8.192 | **0** |
| `rs-servicos` | 586 | 93 | 132 | 143 | 8.192 | **0** |
| `rs-tarf` | 22.476 | 242 | 1.464 | 1.774 | 8.192 | **0** |

**"O truncamento do encoder virou inalcançável por construção" está medido, não argumentado:** o
texto mais longo que existe depois do corte usa **34% do teto do encoder**, e a margem é de 3×. Um
subproduto útil: a razão chars/token do pior caso é **2,17** (2.769 tokens em 6.000 chars, numa FAQ),
bem abaixo do ~3,9 que eu vinha supondo — quem for mexer no teto no futuro deve usar 2,2, não 3,9,
para saber onde os 8.192 tokens ficam. Pelos 2,17, o encoder só voltaria a truncar com um teto acima
de ~17.800 caracteres.

**Uma premissa sua caiu aqui, e vale isolada.** Quando o N foi escolhido, 6.000 caracteres foram
lidos como "~1.500 tokens". A key mais longa dá **2.769** — 84% acima. O N continua defensável, e por
razões medidas: o P99 é 1.452 nas FAQs e 1.464 no TARF, dentro da faixa boa, e são 14 documentos no
teto. Mas **o A/B de retrieval deve incluir um N mais baixo**: pelos 2,17, o teto que de fato entrega
as ~1.500 tokens pretendidas é de **~3.300 caracteres**.

**Decisão sua, executada: o `conta_tokens` foi removido, e o número ficou.** Os 2,17 chars/token e o
limiar de ~17.800 caracteres estão agora no doc comment do `TETO_TEXT_TO_EMBED`, que é onde a decisão
mora — junto com a instrução de dividir por 2,17 e não por 3,9 para mirar um comprimento em tokens.
Saíram o método e o teste `#[ignore]` que o exercia; ficou o resultado, no lugar onde alguém tropeça
nele antes de mexer no teto.

Ajustei também dois comentários do `embed.rs` que prometiam o aviso item a item que deixou de existir
— são meus, criados pela mudança.

---

## 10 — (d) O que não bateu, e uma coisa que você precisa saber antes de rodar as 27

**⚠️ ~~O servidor em execução está a um restart de recusar o boot.~~ Resolvido em 12/08.** O binário
em disco (`target/release/auli`) era strategy 3; os packs de `data/` eram strategy 2. O processo
469984 seguia no ar porque carregava a imagem antiga (`/proc/469984/exe` apontando para o arquivo
`(deleted)`), mas qualquer reinício bateria na guarda de identidade e o boot seria recusado —
**corretamente**, é a proteção fazendo o trabalho dela. A ordem segura era: rodar as 27, e só então
reiniciar.

> **Foi o que se fez.** As 27 entidades foram regeneradas e o servidor reiniciado às 09:58 de 12/08,
> com as 27 validações de manifesto passando no boot e a primeira consulta real trazendo o carimbo
> `EMBED · modelo: bge-m3-q-int8 · dim: 1024 · strategy: 3`. Números da operação: **419 vetores
> alterados em ~50 mil documentos, todos anunciados pela linha `✂️` antes da comparação**, e nenhum
> vetor mudou sem estar cortado — a igualdade de conjuntos da §8, que a entidade de verificação
> provou numa, valendo nas 27. Registro completo em `RECOMENDACOES-CONSOLIDADAS.md`.

**A previsão do dial não era previsão de pico**, e a §3 dizia isso, mas a tabela
convidava à leitura errada. Registrado aqui com os dois números lado a lado.

**A nota do FATIA no `RELATORIO-MEMORIA-UPDATE.md` foi reescrita, e mudou de sinal.** Ela tinha sido
escrita com o pico de 3,1 GiB da corrida *em andamento* e apostava que o `FATIA` viraria o maior
termo; com os tokens contados, a fórmula que sustentava a aposta está **refutada** — ver a correção
no §7. A nota agora registra a tensão, o mecanismo confirmado do termo quadrático, e o pipeline de
pack como candidato mais barato para o resíduo.

**Nada do "o que NÃO fazer" foi tocado:** `EMBED_MAX_TOKENS`, `batch_size`, `FATIA` e
`dedup_por_slug` estão como estavam, e não há chunking.
