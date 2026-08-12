# RELATÓRIO — Fases 1 a 3 da TAREFA-TETO-TEXT-TO-EMBED

**Para:** Claude Fable
**Estado:** N = 6.000 implementado, `STRATEGY_VERSION` em 3, entidade de verificação rodada de ponta
a ponta — mesclado em `main` pelo PR #145. **As 27 entidades não foram rodadas** — são suas.

**Revisado em 12/08, depois da sua leitura:** duas correções (§1 e §3), e a remoção do `conta_tokens`
com o número preservado onde a decisão mora.

**O resultado que a Fase 3 pedia, na primeira linha:** o pack novo difere do antigo **exatamente**
nos documentos cortados, nas quatro coleções, como igualdade de conjuntos e não como contagem.

---

## 1 — (c) A entidade de verificação

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
quadrático previsto" do relatório da Fase 0 nunca foi previsão de pico — você mesmo escreveu isso ao
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

## 2 — (b) O corte, os testes de fórmula congelada, o `relatar_teto`

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

Os números que ele imprimiu — 14 de 1.947 e 404 de 22.476 — batem com os da Fase 0 na unidade.

**Suíte:** tudo verde no workspace, clippy limpo. Cada defeito consertado nesta linha tem um teste
que falha quando o defeito volta.

---

## 3 — O `conta_tokens`, gasto na verificação como você pediu

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

## 4 — (d) O que não bateu, e uma coisa que você precisa saber antes de rodar as 27

**⚠️ O servidor em execução está a um restart de recusar o boot.** O binário em disco
(`target/release/auli`) agora é strategy 3; os packs de `data/` são strategy 2. O processo 469984
segue no ar porque carrega a imagem antiga (`/proc/469984/exe` aponta para o arquivo `(deleted)`),
mas qualquer reinício bate na guarda de identidade e o boot é recusado — **corretamente**, é a
proteção fazendo o trabalho dela. A ordem segura é: rodar as 27, e só então reiniciar.

**A previsão do dial não era previsão de pico**, e o relatório da Fase 0 dizia isso, mas a tabela
convidava à leitura errada. Registrado aqui com os dois números lado a lado.

**A nota do FATIA no `RELATORIO-MEMORIA-UPDATE.md` foi reescrita, e mudou de sinal.** Ela tinha sido
escrita com o pico de 3,1 GiB da corrida *em andamento* e apostava que o `FATIA` viraria o maior
termo; com os tokens contados, a fórmula que sustentava a aposta está **refutada** — ver a correção
no §1. A nota agora registra a tensão, o mecanismo confirmado do termo quadrático, e o pipeline de
pack como candidato mais barato para o resíduo.

**Nada do "o que NÃO fazer" foi tocado:** `EMBED_MAX_TOKENS`, `batch_size`, `FATIA` e
`dedup_por_slug` estão como estavam, e não há chunking.
