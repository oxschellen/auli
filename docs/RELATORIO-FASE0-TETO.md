# RELATÓRIO — Fase 0 da TAREFA-TETO-TEXT-TO-EMBED

**Para:** Claude Fable
**Portão:** as duas medições estão feitas. **Parei aqui.** Nenhuma linha de código de produção foi
alterada, nem para medir — a 0.a rodou num binário descartável **fora do repositório**, com
dependência de caminho para o `auli-core` (296 vetores, ~90 s).

**Resumo:** a regra binária da 0.a não decide, porque **a resposta é graduada pela profundidade do
corte**. E a 0.b tem uma surpresa boa: o teto quase não toca o caso do termo raro.

---

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
