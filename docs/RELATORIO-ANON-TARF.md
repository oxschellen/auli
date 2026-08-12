# RELATÓRIO — `tools/anon-tarf`, execução sobre os 22.476 acórdãos

**Para:** Claude Fable
**Programa:** `auli-server/tools/anon-tarf`, 400 linhas, one-shot. Rodou em **1m28s**, um documento por
vez. Saída em `data/anon-tarf/` (gitignored): árvore anonimizada + quatro relatórios.
**Cercas respeitadas:** nada escrito em `data/rs/docs/tarf/` nem em pack; `auli-anon` intocado;
patches das Fases 5 e 6 não aplicados; NER não rodado.

**Os dois achados que mudam decisão vêm primeiro.**

---

## 1 — (a) Round-trip: **18 falhas em 22.476**, e o defeito é do `cloakrs`

O crate promete `anonimizar → restaurar == original`. Sobre dado real, quebra em 18 documentos —
e as 18 têm a **mesma causa**, no reconhecedor nativo `credit_card_luhn_v1`
(`cloakrs-patterns 0.3.0`, `src/credit_card.rs:41-45`):

```rust
span: Span::new(matched.start(), matched.end()),  // inclui o separador final
text: matched.as_str().trim().to_string(),        // não inclui
```

O regex `\b(?:\d[ -.]?){13,19}\b` permite que o casamento termine em espaço, ponto ou hífen. O
**span** cobre esse separador; o **texto guardado** é o `trim()` dele. Na restauração, o valor sem o
separador volta para um vão que o tinha — e some um caractere:

```text
orig : Produto: ROST OFF WURTH 300 ML 27101932 6910 12 UN
volta: Produto: ROST OFF WURTH 300 ML 27101932 6910 12UN
```

**Nossos reconhecedores não têm esse defeito** — nenhum deles faz `trim()` no texto guardado.

**Gravidade real, sem alarme:** o caminho de restauração serve a resposta do LLM ao usuário
(`Anonimizador::restaurar`), então o efeito visível é uma palavra colada na anterior, em texto que
contém um número de 13–19 dígitos. Não corrompe pack nem log. **Mas a garantia enunciada é falsa**, e
o que ela realmente expõe é que `span` e `original` podem discordar — o que em outro reconhecedor
poderia doer mais.

Todas as 18 detecções são, além disso, **falsos positivos**: são quantidades, protocolos e números de
processo que passam no Luhn. `CREDIT_CARD` deu 78 detecções em 51 documentos e não há cartão de
crédito em acórdão de ICMS.

---

## 2 — (d) O saneamento na fonte foi **abandonado em 2016**

A apuração prévia disse "56% do TARF vem saneado". Está certo, e é **a leitura errada do número**. A
tabulação por ano — que você pediu no refinamento 6, e que eu esperava que apenas enfraquecesse um
receio — mostra outra coisa:

| ano | saneados | ano | saneados |
|---|---:|---|---:|
| 2005 | 84,6% | 2013 | 80,1% |
| 2006 | 88,5% | 2014 | 87,2% |
| 2007 | 90,6% | 2015 | **90,9%** |
| 2008 | 89,8% | **2016** | **3,3%** |
| 2009 | 92,2% | 2017 | **0,0%** |
| 2010 | 80,3% | 2018–2026 | **0,0%** |
| 2011 | 86,9% | | |
| 2012 | 84,2% | | |

**Não é uma taxa: é uma prática que acabou.** Até 2015 o TARF publicava com o nome substituído por
`(...)` em ~87% dos casos; a partir de 2017, **nenhum acórdão é saneado**. Os 56% são o peso da era
antiga no acervo, não uma propriedade do que a fonte publica hoje.

**Consequência que reordena a §6 do `auli-anon_pendencias`:** a exposição prospectiva é **100%**, não
44%. Cada acórdão novo nomeia as partes, e o acervo cresce só nesse regime. O trabalho de
anonimização estrutural deixa de ser sobre um passivo de 44% e passa a ser sobre todo o fluxo futuro.

---

## 3 — (b) Recall por coorte: **o reconhecedor não vê a forma em que o TARF escreve**

Medido sobre 22.454 valores de `RECORRENTE`, com as duas taxas separadas:

| coorte | posição | recall |
|---|---|---:|
| pessoa jurídica | linha rotulada | 336/7.505 = **4,5%** |
| pessoa jurídica | texto livre | 189/1.817 = **10,4%** |
| sem sufixo | linha rotulada | 27/1.800 = **1,5%** |
| sem sufixo | texto livre | 124/1.336 = **9,3%** |

**A causa não é calibração — é incompatibilidade de forma, e ela é medível.** O
`auli_razao_social_v1` exige **≥2 tokens `Titlecase`**, e `comum::titlecase` define Titlecase como
"primeira maiúscula **e** alguma minúscula" (é o que separa `Andrade` de `ICMS`). Cruzando a forma do
nome rotulado com o mascaramento:

| forma do nome no rótulo | n | mascarados |
|---|---:|---:|
| **≥2 tokens Titlecase** (a forma que o reconhecedor procura) | **32** | 18,8% |
| 1 token Titlecase | 103 | 1,9% |
| **0 tokens Titlecase** (caixa alta) | **7.022** | 4,1% |

**Trinta e dois.** De ~7.157 nomes reais no bloco de identificação, **0,4%** estão na forma que a
Fase 4 foi construída para reconhecer. O bloco do acórdão é escrito em **caixa alta**, e caixa alta
produz zero tokens Titlecase — o porteiro fecha antes de qualquer heurística rodar.

**Isto é invisível nas fixtures**, e é o ponto: fixture de razão social é escrita em prosa
(`A empresa Andrade Comércio Ltda. recolheu…`), que é exatamente a forma que o corpus real não usa no
lugar onde a identificação vive. **Uma corrida de 400 documentos reais mostrou o que 8 fixtures
verdes não mostravam.**

**E inverte a leitura do refinamento 2.** Você previu "taxa alta na linha rotulada e baixa fora dela"
como o caso quantitativo de estrutura sobre léxico. O medido é **baixo nos dois, e mais baixo na
linha** — porque o bloco é caixa alta e o corpo em prosa não é. O argumento pela anonimização
estrutural sai **mais forte**, não mais fraco: o léxico é mais cego justamente onde a identificação é
certa.

**Detecções fundidas** (uma cobrindo duas entidades do mesmo rótulo): **13**. A regra aplicada, que a
TAREFA pedia explícita: cada entidade esperada conta como coberta se **alguma** detecção intersecta
sua ocorrência; uma detecção que cobre duas conta para as duas **e** é somada à parte. Com 13 casos
em 9.296 entidades, a escolha da regra não move nenhum número.

---

## 4 — (c) O tamanho da lacuna 5

Dos 9.296 nomes reais extraídos das linhas rotuladas:

| | n | % |
|---|---:|---:|
| com sufixo societário ou termo de segmento | 7.505 | 80,7% |
| **sem nenhum dos dois** — a lacuna 5 | **1.800** | **19,3%** |

É a primeira medição direta de uma lacuna que era "nomeada e assumida". E ela é **maior** do que a
tabela sugere isoladamente, porque a §3 mostra que a Fase 4 não pega nem os 80,7% que deveria pegar,
por caixa alta.

---

## 5 — (e) Os dois dicionários

**Termos de segmento — 247 termos** (contra ~120 curados à mão na Fase 6), minerados só na metade 0
do corpus. O critério é bom e vale registrar: **token que aparece em muitos nomes distintos** é termo
de ramo; nome próprio aparece em poucos. Topo: `ltda.` (1.008 nomes distintos), `comércio` (374),
`indústria` (290), `s.a.` (136), `alimentos` (127), `distribuidora` (94), `transportes` (66),
`cooperativa` (59).

**Precisa de uma passada antes de virar dicionário:** `brasil` (115) e `maria` (36) entraram, e não
são termos de segmento. O critério de frequência não distingue "palavra comum em razão social" de
"termo que indica ramo" — a curadoria continua necessária, só que sobre 247 candidatos ordenados em
vez de sobre o vazio.

**Stop-list do domínio — 32 entradas**, semeada com `FAZENDA ESTADUAL` e `AS MESMAS` (as duas
armadilhas mais frequentes: multi-token, capitalizadas, em posição rotulada, milhares de ocorrências,
jamais mascaráveis) e completada pelo que o anonimizador mascarou nos blocos provadamente sem nome.

**Sobre a cerca do §5 (não minerar e medir no mesmo texto):** ela está satisfeita, mas por um motivo
mais fraco do que parece — **nada minerado alimentou nenhuma medição desta rodada**. O recall usa um
classificador de coorte escrito à mão, não o dicionário. A divisão por hash do `doc_path` está
implementada e a mineração respeita a metade 0; a cerca passa a morder de verdade quando alguém usar
o dicionário para medir, e aí a metade 1 é que vale.

---

## 6 — (refinamento 3) O controle negativo funcionou, e o volume é humano

**12.622 documentos** têm `RECORRENTE = (...)`: o bloco está provadamente sem nome de parte, então
detecção de NOME/RAZAO_SOCIAL ali é falso positivo **com rótulo**. Total: **64 detecções**.

Sessenta e quatro é revisável por uma pessoa numa sentada — que era exatamente o objetivo de trocar a
amostra aleatória estratificada pelas discordâncias. As mais frequentes:
`Com. de Cereais União Ltda.` (15×), `Ministro Relator` (6×), `Ordem da Operação` (4×),
`Geraldo Schilling` (3×).

**Uma ressalva que o número exige:** o bloco de identificação de um acórdão saneado ainda pode nomear
**terceiros** — advogado, relator, perito. `Ministro Relator` e `Geraldo Schilling` são disso. Então
64 é **teto** de falso positivo, não contagem final; parte deles é detecção correta de gente que não
é parte. A revisão humana decide, e por isso o arquivo `discordancias.tsv` existe.

O outro lado das discordâncias — **2.840 misses** no gold set — é o que a §3 já explica.

---

## 7 — Falsos positivos nativos, separados desde o relatório

Como você previu, o corpo do acórdão dispara os reconhecedores nativos em volume:

| classe | detecções | documentos |
|---|---:|---:|
| `URL` | 1.022 | 665 |
| `IP_ADDRESS` | 1.077 | 399 |
| `MAC_ADDR` | 274 | 92 |
| `CREDIT_CARD` | 78 | 51 |
| `HOSTNAME` | 28 | 10 |
| `USER_PATH` | 18 | 6 |
| `CRYPTO_ADDR` | 4 | 2 |

Numeração legal e endereço de portal, como a §5.6 do pendencias já suspeitava. **Nenhum deles é PII
em acórdão**, e é justamente o `CREDIT_CARD` — o menos plausível de todos — que carrega o defeito de
round-trip da §1.

---

## 8 — A fronteira de reuso: passa, com um atrito de uma linha

O programa **não depende de `cloakrs-core`**, de propósito: tudo o que lê das detecções sai de campos
`String`/`f64` do `mapping`. O único atrito: `auli-anon` expõe `Anonimizado.mapping` como
`cloakrs_core::PromptMapping` mas **não reexporta o tipo** — então ler o `entity_type` (um enum de lá)
exigiria depender da biblioteca de baixo direto. Contornei lendo a classe do **prefixo do
placeholder** (`[NOME_3]` → `NOME`), que é uma linha.

Reporto em vez de contornar em silêncio, como a cerca pede, mas **não recomendo mudar o `auli-anon`
por isto**: um `pub use cloakrs_core::{PromptMapping, EntityType}` resolveria e é tentador, só que
transforma um detalhe de implementação em contrato público. O contorno de uma linha é o preço certo.

---

## 9 — (g) Suposições que não bateram

**"Falso positivo não é automático, por falta de rótulo negativo"** (TAREFA §4) — você mesmo já tinha
corrigido no refinamento 3, e a execução confirma: os 12.622 saneados dão o rótulo negativo, e o
resultado (64) é pequeno o bastante para revisão humana integral.

**"56% saneados"** era leitura estática de uma prática descontinuada. Ver §2.

**A minha hipótese de que caixa alta explicaria tudo** — testei e ela não sobrevive na forma simples:
cruzando "tem alguma minúscula" com mascaramento, os de caixa alta são mascarados **mais** (3,9%)
que os mistos (0,8%). A explicação correta é mais específica e está na §3: o critério não é
maiúscula/minúscula, é **`Titlecase`** — e nomes em caixa alta *e* em caixa baixa dão zero tokens
Titlecase. Registro o erro intermediário porque ele passaria por bom se eu tivesse parado na primeira
tabela.

**Sobre o formato do gold set:** as formas plurais sem parênteses (`RECORRENTES:`) valem 463
documentos e a extração as cobre; sobram 2 casos com o rótulo partido por OCR (`RECORRENTE S :`),
também cobertos. Cobertura efetiva **22.454 de 22.476**.

---

## 10 — O que isto sugere como próximo passo

Não implementei nada além do programa, e estas são recomendações, não decisões:

1. **Reportar o defeito do `credit_card_luhn_v1` ao `cloakrs`** — é upstream, tem duas linhas de
   causa e reprodução em dado público. Enquanto isso, a garantia de round-trip do `auli-anon` merece
   uma ressalva escrita no doc comment: ela vale para os nossos reconhecedores, não para todos os
   nativos.
2. **A cegueira a caixa alta é a decisão de fundo.** Ela não se conserta baixando o porteiro
   (`titlecase → comeca_maiuscula` inundaria de FP: em caixa alta, *toda* palavra passa). Ou o
   reconhecedor ganha um modo específico para blocos em caixa alta, ou a resposta é estrutural — o
   que a §2 deste relatório torna mais urgente, porque o fluxo novo é 100% não saneado.
3. **A lacuna 5 tem número** (1.800 de 9.296) e a fonte de sobrenomes que a Fase 5 precisava está
   nos 1.800 — mas nada disso adianta enquanto o porteiro de forma barrar o caminho antes.

---

## 11 — Duas medições posteriores (12/08), a pedido do Fable

### 11.1 O defeito do `credit_card_luhn_v1` aparece **duas vezes**, em corpora independentes

O mesmo programa rodou sobre os **19.780 pareceres** das quatro entidades (`rs`, `sc`, `pr`, `sp`),
montados numa árvore separada: **16 falhas de round-trip, todas `CREDIT_CARD`**, e com assinatura
diferente da do TARF — tabelas CEST/NCM, do tipo `20.054.00 8214.10.00 Espátulas`. **Códigos fiscais
pontuados passam no Luhn.** Duas famílias de documento, dois padrões de texto, o mesmo defeito.

**Qual é exatamente o defeito, já que a pergunta se repete:** não é falso positivo, nem falso
negativo, nem pânico. É `span` e `text` discordarem, e o efeito é **perda silenciosa de um
caractere** na restauração — o valor volta menor que o vão que ocupava. Não estoura, não avisa, e o
texto restaurado passa por íntegro.

A limitação de projeto é outra coisa, e convive: o Luhn valida **qualquer** corrida de 13–19 dígitos.
Daí as 78 detecções de cartão de crédito num acervo de ICMS. **Esta segunda parte nos afeta em
produção** — uma pergunta com número de processo pode virar `[CREDIT_CARD_1]` e o número não chega ao
LLM. É o que justifica um contorno local, declarado como temporário e amarrado ao issue upstream.

**Reconciliação com a §5.6 do `auli-anon_pendencias` — a hipótese da key se confirma no ponto que
importa, e só nele.** Ela traz `CREDIT_CARD 1×`, `URL 3447×`, `IP_ADDRESS 58×`; o scan do `## corpo`
dá 43, 2.953 e 212. Rodei então o mesmo anonimizador sobre a **key** dos 19.780 pareceres
(`numero + assunto + resumo`, que **não inclui o corpo**):

| | §5.6 | key | `## corpo` |
|---|---:|---:|---:|
| `CREDIT_CARD` | **1** | **1** | 43 |
| `IP_ADDRESS` | 58 | 88 | 212 |
| `URL` | 3.447 | 22 | 2.953 |

**O `CREDIT_CARD` bate exatamente**, e a detecção é identificável: um número de **TTD** (`TTD nº …`)
num `## resumo` de consulta da COPAT-SC, mascarado como cartão de crédito. Mesmo reconhecedor, mesma
classe de falso positivo.

**Mas não é o mesmo defeito, e a diferença é boa notícia:** o scan da key deu **zero falhas de
round-trip**. O defeito exige que o casamento termine em separador, o que acontece nas tabelas
CEST/NCM e nas listas de quantidade — todas no `## corpo`. E **o corpo não é embedado, nem
anonimizado em produção**: o único caminho de produção do anonimizador é a **pergunta**. Isso limita
o defeito a perguntas que contenham corrida de 13 a 19 dígitos terminada em espaço, ponto ou hífen.

`URL` e `IP_ADDRESS` continuam sem reconciliar — 3.447 não sai nem da key nem do corpo, então o
escopo daquela varredura era outro (provavelmente incluía serviços e FAQs, cheios de link). Fica
registrado como aberto, sem escolher a explicação menos ruim.

### 11.2 A exposição cresce em volume, não só em proporção

A §2 mostra que o saneamento acabou em 2016. Em números absolutos:

| era | proporção exposta | expostos/ano | total no período |
|---|---:|---:|---:|
| 2005–2015 (com saneamento) | 9 a 20% | ~180 | **2.004** |
| 2016–2026 (sem saneamento) | 97 a 100% | ~780 | **7.825** |

**3,9× mais documentos expostos na era nova** — e o detalhe que fecha o argumento: **o acervo
encolheu por ano** no mesmo período (1.916 acórdãos em 2013, 539 em 2024). A exposição quadruplicou
enquanto a produção caía pela metade. Não é passivo histórico crescendo por acumulação; é **operação
corrente inteiramente descoberta**.

### 11.3 O escopo da correção: a unidade é o valor rotulado, não o bloco

Registrado porque é onde a discussão parou. Tornar os reconhecedores insensíveis a caixa
**globalmente** é onde os falsos positivos moram — texto institucional é feito de caixa alta
(títulos, rótulos, siglas). O escopo natural é o bloco de identificação, onde a posição já diz que
ali há uma parte.

**Mas o bloco sozinho não basta:** a `FAZENDA ESTADUAL` está dentro dele, em caixa alta, em **14,6%**
dos casos, e é a coisa que jamais pode ser mascarada. A posição diz que ali *há* uma parte; não diz
*quais tokens* são a parte. A unidade correta é **o valor da linha rotulada, menos a Fazenda e os
anafóricos** — que é exatamente o que o extrator deste programa faz, e a razão de o gold set ter
9.296 entidades e não 22.454. **A stop-list continua necessária mesmo dentro do bloco.**
