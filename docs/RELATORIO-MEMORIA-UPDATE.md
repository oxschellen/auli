# RELATÓRIO — a memória do `auli update`: o que a medição mostrou

**Para:** Claude Fable
**Responde a:** `TAREFA-MEMORIA-UPDATE.md`
**Natureza:** medição. **Nenhuma linha de código foi alterada** — nem para instrumentar.

**Resultado:** há causa identificada, ela explica os três fatos juntos, e **não é a hipótese que a
TAREFA levantou**. O `FATIA = 256` não é o driver. O pico é fixado pelo **texto mais longo da
coleção, quadraticamente** — e um único documento entre 1.947 responde por 35% do pico das FAQs.

---

## 1 — Como foi medido, e por que não precisou de código novo

Cada coleção rodou **sozinha e em primeiro lugar**, como a correção do relatório anterior exigia. Não
foi preciso tocar no `Kind::TODOS`: o `preparar` pula coleção sem árvore, então um diretório de dados
contendo **só** `docs/<kind>/` produz exatamente esse isolamento. As árvores entraram por *hardlink*
(seguro desde os itens 6 e 7) e cada rodada escreveu num diretório de packs novo — produção intocada.

Isso importa metodologicamente: a instrumentação não pôde alterar o que ela mede, porque não houve
instrumentação — só `VmRSS` amostrado de fora, a cada 200 ms, do `/proc/<pid>/status`.

Cada rodada re-vetoriza tudo, que é o regime da **regeneração** (sem manifesto anterior, o cache
nasce desligado). São seis rodadas mais uma sétima, de falseamento (§5).

## 2 — (b) A linha de base é estável

RSS depois da carga do modelo e antes do primeiro texto, nas seis rodadas: **1.001, 1.070, 1.081,
1.087, 1.088, 1.089 MiB**. Variação de 9%, sem correlação com a coleção. **~1,05 GiB é a linha de
base de tudo o que segue**, e os `Δ` abaixo já a descontam.

## 3 — (a) As seis curvas

| coleção | n | base | **pico** | Δ | t(90% do pico) | duração |
|---|---:|---:|---:|---:|---:|---:|
| `rs-servicos` | 586 | 1.081 | **1.090** | 9 | 2 s | 20 s |
| `sp-servicos` | 518 | 1.001 | **1.090** | 89 | 2 s | 14 s |
| `rs-pareceres` | 372 | 1.070 | **1.299** | 229 | 20 s | 41 s |
| `sp-pareceres` | 15.605 | 1.088 | **3.145** | 2.057 | 868 s | 2.804 s |
| `rs-faqs` | 1.947 | 1.089 | **10.333** | 9.244 | 213 s | 274 s |
| `rs-tarf` | 22.476 | 1.087 | **17.724** | 16.637 | 491 s | 3.959 s |

(MiB)

**Três leituras imediatas.** As duas coleções de serviços dão **o mesmo pico em entidades
diferentes** — 1.090 MiB —, porque a key delas é limitada a ~300 chars pelo `snippet`; são o
controle natural. O `rs-faqs` **sozinho reproduz o platô** da rodada completa: 10.333 contra 10.358
MiB, 25 MiB de diferença. E o pico **não acompanha o tamanho do corpus**: `rs-pareceres`, com 372
documentos, custa 229 MiB; `sp-pareceres`, com 15.605, custa 2.057 — mas `rs-faqs`, com 1.947, custa
9.244.

## 4 — (c) A distribuição, e o teto que nunca é atingido

Comprimento do `text_to_embed` em caracteres, recomposto pela fórmula do contrato:

| coleção | P50 | P90 | P99 | **máx** |
|---|---:|---:|---:|---:|
| `rs-servicos` | 374 | 425 | 454 | **471** |
| `sp-servicos` | 154 | 240 | 379 | **420** |
| `rs-pareceres` | 1.306 | 1.529 | 1.772 | **1.807** |
| `sp-pareceres` | 1.847 | 2.523 | 3.414 | **10.240** |
| `rs-faqs` | 759 | 2.146 | 5.360 | **25.120** |
| `rs-tarf` | 914 | 2.597 | 8.417 | **32.722** |

**Nenhum texto, em nenhuma das seis coleções, atinge o teto de 8.192 tokens** — o `avisar_truncados`
não emitiu um único aviso. O truncamento não está protegendo nada hoje, e o maior acórdão (32.722
chars) passa **logo abaixo** do teto.

A cauda das FAQs é o que destoa: 25.120 caracteres é **33× a mediana delas**. É consequência direta
da TAREFA-FAQ-PR, que pôs a resposta inteira na key — e nada a limita, ao contrário do `snippet(300)`
dos serviços.

## 5 — (e) A causa, e o teste que a falseia

**O pico é fixado pelo TEXTO MAIS LONGO da coleção, e cresce com o QUADRADO dele.**

| coleção | Δ (MiB) | máx (chars) | Δ / máx² (×10⁻⁵) |
|---|---:|---:|---:|
| `rs-faqs` | 9.244 | 25.120 | **1,46** |
| `rs-tarf` | 16.637 | 32.722 | **1,55** |
| `sp-pareceres` | 2.057 | 10.240 | **1,96** |
| `rs-faqs` sem o maior | 5.669 | 17.233 | **1,91** |

O coeficiente é constante dentro de ±15% do centro da faixa, atravessando **duas entidades, três
coleções e uma variação de 8× no comprimento**. As três coleções pequenas (`servicos`, `rs-pareceres`)
ficam fora da tabela porque o `Δ` delas — 9 a 229 MiB — está na faixa em que custos fixos dominam.

**O teste que falseia.** Removi **um** documento da árvore de FAQs — o maior, de 25.120 caracteres,
0,05% do corpus — e rodei de novo:

```text
rs-faqs completo (1.947 docs)  → 10.333 MiB
rs-faqs sem 1 doc  (1.946)     →  6.758 MiB     (−3.575 MiB, −35%)
```

**Se o custo fosse da fatia de 256, remover um documento não mudaria nada** — a fatia continuaria com
255 outros e o mesmo tamanho. Um documento entre 1.947 respondendo por 35% do pico só é compatível
com "o maior texto individual manda".

**Segunda confirmação, independente:** a posição do maior documento na ordem de leitura prevê **quando**
o platô se forma. No `rs-faqs` o maior está a 86% da lista e o platô chega a 78% do tempo; no
`sp-pareceres`, a 30% e 31%. O RSS sobe quando aquele documento é processado, e **nunca cede** — o
arena do ONNX não devolve o bloco.

**Por que quadrático:** atenção é O(seq²). O coeficiente medido, ~1,5×10⁻⁵ MiB/char², dá ~290 bytes
por token² (a ~4,3 chars/token) — da ordem de alguns buffers de `[1, heads, seq, seq]` em `f32`
mantidos simultaneamente. É consistente com atenção; não afirmo mais que isso sem instrumentar o
runtime, o que esta TAREFA não pede.

## 6 — (d) Qual das três leituras, e a correção de um "fato conhecido"

**A primeira**, com uma precisão: o `rs-faqs` sozinho atinge o platô, então a causa está nos **dados**
das FAQs — mas não em nada peculiar ao *caminho* delas. O mesmo mecanismo governa todas as coleções;
as FAQs só têm o texto mais longo entre as que rodam antes do TARF.

**E um fato que a TAREFA listava como medido precisa ser corrigido.** "O TARF acrescenta apenas ~620
MiB" foi medido numa rodada em que **o TARF não vetorizou nada**: o cache estava ligado, 22.476
reaproveitados, zero a embedar. Aqueles 620 MiB são o pipeline de pack — ler o arquivo de 311 MB,
`apply`, serializar —, não o embedder. **Quando o TARF de fato vetoriza, ele pica em 17,7 GiB**, e
até esta medição ninguém tinha rodado isso.

Isso muda o número que decide a pergunta da §Por-que-existe da TAREFA: o custo de rodar o Auli numa
máquina modesta não é 10,7 GiB, é o **pico de regeneração**, que hoje é **17,7 GiB** para o `rs`.

## 7 — As três hipóteses da §3 da TAREFA, resolvidas

**O `FATIA = 256` não é o driver.** O pior caso documentado — `256 × 8.192 × 1.024 × 4 = 8,6 GB` —
supõe que o colbert da fatia inteira é alocado com todos os textos preenchidos ao teto. Os dados não
sustentam esse modelo: o pico não escala com o número de fatias (o `rs-tarf`, com 88 fatias, atinge
90% do pico a 12% da rodada), não escala com a soma da fatia (o `sp-pareceres` tem soma-de-fatia
**maior** que o `rs-faqs` e pico 3× menor), e cai 35% quando um único documento sai. O número 8,6 GB
está na ordem de grandeza certa por coincidência aritmética, não pelo mecanismo.

**A afirmação "que o corpus real não tem" está certa** — nenhum texto chega ao teto. Mas ela protege
menos do que parece: o maior acórdão passa **logo abaixo** de 8.192 tokens, e é justamente ele que
custa os 17,7 GiB.

> ### Nota posterior (12/08) — o mecanismo do quadrático se confirma; o do `FATIA` se refuta
>
> **Leia a afirmação acima com a data dela.** Ela é verdadeira no regime medido, com a cauda intacta.
> Depois do teto de 6.000 caracteres da TAREFA-TETO o quadro muda, mas **não** no sentido em que uma
> versão anterior desta nota apostava: eu tinha escrito que o `FATIA` viraria o maior termo. Com a
> razão chars/token medida, a fórmula que sustentava essa aposta **está refutada como enunciada**.
>
> **A medição que mudou tudo.** A contagem de tokens sobre as 25.381 keys do `rs` já cortadas dá
> **2,17 chars/token no pior caso** — não os ~3,9 que eu vinha usando. Todo cálculo desta nota que
> converte caracteres em tokens foi refeito com 2,17.
>
> **O termo quadrático deixa de ser hipótese e passa a ter mecanismo.** Se a atenção aloca
> `16 cabeças × seq² × 4 bytes`, então em caracteres isso é `16 × 4 / 2,17²` bytes por char², ou
> **1,3×10⁻⁵ MiB/char²** — contra o coeficiente medido de 1,46 a 1,96×10⁻⁵. A lei empírica deste
> relatório e a aritmética do transformer se encontram, e a razão de conversão medida é o que fecha
> a ponte entre as duas.
>
> **O termo do `FATIA`, ao contrário, não sobrevive ao próprio teste.** A fórmula candidata era:
> `256 × 1.024 × 4 bytes` é exatamente 1 MiB, logo um colbert alocado por fatia custaria, em MiB, o
> comprimento em tokens do maior texto da fatia. Aplicada ao `rs-tarf` com teto (1.774 tokens) ela
> prevê 1.774 MiB e o resíduo sem dono é ~1.900 — parece boa. Aplicada ao `sp-pareceres`, que rodou
> **sem** teto, ela prevê `10.240 / 2,17 ≈ 4.719` tokens ⇒ **~4,7 GiB só de colbert**, contra um `Δ`
> **total** medido de **2.057 MiB**. A fórmula prevê mais que o dobro do consumo inteiro da coleção.
> Isso não é imprecisão: é refutação.
>
> Um segundo sinal aponta na mesma direção: o `rs-tarf` **com** teto (máximo 1.774 tokens) custa
> **mais** que o `sp-pareceres` **sem** teto (~4.719 tokens) — `Δ` de 2.440 contra 2.057 MiB. Se o
> maior texto da fatia mandasse sozinho, a ordem seria a inversa.
>
> **O resíduo tem candidato mais barato que o `FATIA`: o pipeline de pack.** Este relatório já o
> mediu em ~620 MiB para o TARF — e naquela rodada o TARF *não vetorizou nada*. Numa re-vetorização
> integral ele é maior: os 22.476 registros ficam em memória com payload e depois viram 311 MB de
> JSON serializado. Não fecha a conta sozinho (620 é cerca de um terço de 1.900), mas é um termo
> **medido** que já existe, e explicar o resíduo com ele exige menos invenção do que com um
> mecanismo que a própria evidência contraria.
>
> **Consequência prática:** o teste do `FATIA = 128`, que eu tinha chamado de decisivo e barato,
> perdeu o alvo — a hipótese que ele confirmaria já está contrariada por um dado que temos. A
> decisão de não medir segue valendo, agora com um motivo a mais.

**Um limite útil que sai daí.** Com o teto de 8.192 tokens em vigor, o pico máximo alcançável é
~1,5×10⁻⁵ × (8.192 × 4,3)² ≈ **18,6 GiB acima da base**. O `rs-tarf`, em 16,6 GiB, já está a 89%
desse teto: **não pode piorar muito sem mudar o teto — e também não vai melhorar sozinho.**

## 8 — O que NÃO foi feito, e o que a decisão exige

Nada foi corrigido, conforme a TAREFA. Registro só o que a medição permite afirmar sobre as opções,
porque o desenho depende disso:

- **Mexer no `FATIA` não resolveria.** Ele não é o driver; reduzi-lo trocaria chamadas por memória
  sem tocar no termo quadrático.
- **O lever que a medição aponta é o comprimento da key.** Os serviços já fazem isso com o
  `snippet(300)` e custam 9 MiB. Limitar a key das FAQs e da jurisprudência limitaria o pico de forma
  **determinística e calculável** — mas é mudança de `text_to_embed`, ou seja, re-vetorização de tudo
  e bump de `STRATEGY_VERSION`, além de perda de recall no texto cortado. É decisão sua, e é grande.
- **Se for medir de novo**, o harness está pronto e é reaproveitável: isolamento por diretório de
  dados com uma árvore só, hardlinks, e amostragem de `VmRSS` por fora.

## 9 — Uma coisa que não expliquei

Os **13,1 GB** históricos da regeneração não batem com nada que medi: o `rs-tarf` regenerando dá
17,7 GiB, e nenhuma outra coleção chega perto de 13. Pode ser corpus menor à época, outra forma de
medir, ou outra coleção. **Não sei, e não vou escolher a explicação menos ruim** — o número novo é
17,7 GiB, medido, com curva e falseamento; o antigo fica como registro sem procedência conhecida.
