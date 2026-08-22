# RECOMENDAÇÕES CONSOLIDADAS — estado, fila e cercas

**Para:** Claude Code · **Data:** 12/08/2026 · **De:** Claude Fable

Documento único, substituindo a leitura de seis TAREFAs e cinco relatórios. Ele diz **o que está
fechado** (para ninguém refazer), **o que está na fila e por quê**, e **o que não se toca**.

Cada item da fila traz a mesma estrutura: o que fazer, o que decide se está certo, e a cerca. Onde a
recomendação nasceu de uma medição, o número está ao lado — nesta trilha várias hipóteses plausíveis
caíram quando medidas, e a lição virou doutrina (`CLAUDE.md` §4: *verify by mutation*).

---

## 1 — O que está FECHADO (não reabrir)

**Trilha do pack** — PRs #139 a #143: formato 2 (`id = doc_path`, `key_hash`), caminho incremental na
jurisprudência, remoções com portão humano (`auli remover`), `write_manifest` atômico, `snapshot.json`
e `faqs-tree.json` atômicos, consolidação do `escrever_atomico` em `auli-contract`, e a linha de
doutrina *verify by mutation* no `CLAUDE.md`.

**Trilha do teto** — PRs #144 a #146: carimbo do espaço vetorial no log, `TETO_TEXT_TO_EMBED = 6_000`
com corte em `chars` pelo fim, `relatar_teto` como estatística, `STRATEGY_VERSION` 3, remoção do
`Embedder::conta_tokens` com a razão **2,17 chars/token** preservada no doc comment. As 27 entidades
foram regeneradas.

**Investigação de memória** — causa identificada: o pico é **quadrático no texto mais longo da
coleção** (coeficiente 1,46–1,96 ×10⁻⁵ MiB/char²), não no `FATIA` nem no tamanho do corpus. Provado
por falseamento: remover **um** documento das FAQs derrubou o pico de 10.333 para 6.758 MiB. Depois do
teto: 17,7 → 3,5 GiB.

**Duas conclusões que foram corrigidas e não devem voltar na forma antiga:** o `FATIA` **era dominado,
não irrelevante**; e os 13,1 GB históricos ficam como **número sem procedência conhecida**, sem
explicação escolhida.

> **Correção deste documento (12/08, mesmo dia):** a redação original desta linha e do §4 dizia que o
> `FATIA` "passou a ser provavelmente o maior termo — hipótese registrada, não medida por decisão".
> **Essa é a posição anterior à refutação**, e reintroduzi-la num documento que substitui a leitura
> dos outros onze é o pior lugar possível para ela.
>
> A fórmula `256 × seq_max × 4 KiB` foi **refutada** assim que a razão real de 2,17 chars/token
> permitiu aplicá-la fora do caso em que ela batia: no `sp-pareceres`, que rodou **sem** teto, ela
> prevê **~4,7 GiB** contra um `Δ` **total** medido de **2.057 MiB** — mais que o dobro do consumo
> inteiro da coleção. Segundo sinal na mesma direção: o `rs-tarf` **com** teto custa mais que o
> `sp-pareceres` **sem** teto (2.440 contra 2.057 MiB de `Δ`), embora tenha o texto mais longo bem
> menor.
>
> O que a mesma razão **confirmou** é o outro termo: `16 cabeças × seq² × 4 bytes` dá 1,3×10⁻⁵
> MiB/char² contra os 1,46–1,96×10⁻⁵ medidos — o quadrático deixou de ser lei empírica e passou a
> ter mecanismo. E o resíduo tem candidato mais barato que o `FATIA`: o pipeline de pack, já medido
> em ~620 MiB, que cobre cerca de um terço dos ~1.900 sem dono e portanto **não fecha a conta**.
>
> A cerca do §5 (**não mudar e não medir o `FATIA`**) segue valendo, e agora com um motivo a mais: o
> teste que a decidiria perdeu o alvo, porque a hipótese que ele confirmaria já está contrariada por
> um dado que temos. Fonte: nota de 12/08 no `RELATORIO-MEMORIA-UPDATE.md`.

**Pendência operacional — resolvida em 12/08.** O servidor foi reiniciado às 09:58, com as 27
validações de manifesto passando no boot, e a primeira consulta real trouxe o carimbo
`EMBED · modelo: bge-m3-q-int8 · dim: 1024 · strategy: 3`. A cadeia dos PRs #144 a #146 fechou ponta a
ponta: identidade carimbada, vetores novos, e as distâncias do log atadas ao espaço que as produziu.
Números da operação: 419 vetores alterados em ~50 mil documentos, **todos anunciados pela linha `✂️`
antes da comparação**, e nenhum vetor mudou sem estar cortado.

---

## 2 — FILA, na ordem recomendada

### 2.1 — `tools/anon-tarf` (o programa one-shot sobre o TARF)

**✅ EXECUTADO (PR #147), e o resultado está em `RELATORIO-ANON-TARF.md`** — que é o documento a ler.
A `TAREFA-ANON-TARF.md` e o `RELATORIO-PRE-ANON-TARF.md`, que eram a base, foram removidos em
12/08/2026: a TAREFA por executada, a apuração prévia por ter sido **corrigida** pela execução. Texto
integral no git. Os sete refinamentos abaixo ficam como o registro do desenho — eles governaram o
programa e continuam explicando por que ele mede assim.

> **Os números deste parágrafo estão superados; ficam pelo raciocínio, não pelas cifras.** A leitura
> de "56% saneados" tratava saneamento como propriedade estática do acervo, e a execução mostrou que
> é uma **prática que acabou em 2016**: 20 acórdãos com nome de parte em onze anos até 2015, contra
> 7.099 nos onze seguintes. E "exposto" não é o complemento de "redigido" — somava os 2.710 casos de
> `FAZENDA ESTADUAL`, que não têm PII. Exposição real: **7.122**, não 9.832. Ver `RELATORIO-ANON-TARF`
> §2 e a §6.1 do `auli-anon_pendencias`.

**A premissa que caiu:** eu escrevi que "o TARF nomeia as partes", a partir de **dois** acórdãos. A
apuração prévia media **56% saneados na fonte** (`(...)`), 32,1% com nome real e 11,9% só
`FAZENDA ESTADUAL`/`AS MESMAS`. Nenhum dos cinco motivos morre; o terceiro inverte de sinal — deixa
de ser "confirmar que o TARF é exceção à §6.1" e passa a ser "medir quanto do TARF a fonte já
protege".

**Refinamentos de desenho, todos obrigatórios:**

1. **Recall por COORTE, nunca agregado.** Os 5.545 com sufixo (a Fase 4 deve pegar) e os 1.667 sem
   sufixo (`main` não tem nada) são populações de natureza oposta. A média dos dois é a aritmética de
   "a Fase 4 funciona" com "a lacuna 5 existe", e não diz nada sobre nenhum dos dois.
2. **Duas taxas, dentro do mesmo documento** (proposta do relatório, aceita): mascarado **na linha
   rotulada** × mascarado **fora dela**. A segunda mede capacidade. E a primeira **não é só controle de
   sanidade** — é o **teto do que a anonimização estrutural alcançaria de graça**: taxa alta na linha e
   baixa fora dela é o caso quantitativo de estrutura sobre léxico, que hoje a §6 do pendencias sustenta
   só por argumento.
3. **Os 12.568 saneados são CONTROLE NEGATIVO, não resto.** Um `RECORRENTE` igual a `(...)` tem o bloco
   de identificação provadamente livre de nome de parte: detecção ali é FP **com rótulo**. Isso corrige
   o que eu afirmei na TAREFA — que o lado do FP não podia ser automático por falta de rótulo negativo.
   Pode, no escopo do bloco (o resto do corpo ainda nomeia advogado, relator, doutrinador). Consequência
   prática: a amostra humana deixa de ser aleatória estratificada e passa a revisar as
   **discordâncias** — detecção onde o rótulo diz que não há nome, e miss no gold set.
4. **Regra de acerto parcial para os 11,2% com ` E `.** Se o extrator quebra em duas entidades e o
   reconhecedor emite **uma** detecção fundida cobrindo as duas, isso é acerto, erro ou parcial? Sem
   regra explícita, 11% do gold set vira ruído. E note o bônus: esses casos são conjunto de teste
   **real e rotulado** para a falha nº 2 da Fase 5 (o conector `e` encadeando itens).
5. **`FAZENDA ESTADUAL` (14,6%) e `AS MESMAS` semeiam a stop-list automaticamente.** É a armadilha de FP
   mais frequente do acervo: multi-token, capitalizada, em posição rotulada, milhares de ocorrências,
   jamais mascarável.
6. **Tabulação cruzada saneado × ano.** Meu receio de viés de época fica enfraquecido pela própria
   aritmética (56% saneados contra 25,7% de layout moderno ⇒ há saneamento em volume na era antiga),
   mas a tabulação fecha com uma agregação sobre dados já extraídos.
7. **Mantém-se:** dividir o corpus por **hash do `doc_path`** (não por ordem alfabética, que correlaciona
   com ano e layout) para não minerar e medir no mesmo texto; round-trip `anonimizar → restaurar ==
   original` em **todos** os 22.476, que é o teste mais severo que essa propriedade já recebeu; um
   documento por vez; só o `## corpo`; saída em diretório novo.

**Cercas:** não alterar o `auli-anon` (a fronteira de reuso já foi verificada — o `PromptMappingEntry`
expõe `recognizer_id`, `confidence` e spans; se algo não couber, **isso é o achado**); não aplicar os
patches das Fases 5 e 6; não rodar NER; nada em `data/rs/docs/tarf/` nem em pack.

### 2.2 — Correção da §6.1 do `auli-anon_pendencias`, em commit próprio

O número já existe e não depende do programa rodar. A §6.1 afirma "para o acervo de hoje, não há o que
anonimizar" — verdadeiro para os pareceres, **falso para o TARF**: 56% cobertos pela fonte, **44% de
exposição que ninguém tinha contado**.

Commit próprio porque outras decisões se apoiaram nessa conclusão, e num commit de programa ela ficaria
soterrada. **Preserve a formulação original ao lado** — ela não errou sobre os pareceres; errou em
generalizar deles para o acervo. E registre que **o marcador difere por coleção** (`XYZ` nos pareceres
do RS, `(...)` no TARF): qualquer verificação futura de "isto já vem saneado?" precisa de um inventário
de marcadores, não de um literal.

### 2.3 — Corrigir a tabela do §1 do `ESTUDO-anon-ner-v2`

Ela lista as Fases 5 e 6 como cobertura vigente. **É a descrição dos patches, não do que roda** — o
código tem doze reconhecedores e nenhum é delas. Baseline real: **Fases 1–4**.

**Registre a direção do erro, porque ela é contraintuitiva:** baseline superestimado faz o ganho
marginal do NER parecer **menor**, e a lista de lacunas do §2 (que descreve o que sobra *depois* das
Fases 5–6) **subestima** o que realmente sobra. Os dois efeitos apontam para o mesmo lado: **o estudo
subestima o que o NER acrescentaria.** O gate 1 (FP zero em `C-dec`) é absoluto e não é afetado.

### 2.4 — TAREFA do scraper: `existe E mesmo GUID ⇒ pula`

O defeito é **latente, não materializado** — medido: 68 perdedores em `tarf-anomalias.txt` e **zero**
`.md` apontando para GUID superado, porque a carga inicial foi passada única e o dedup rodou antes de
existir árvore. A correção é profilaxia: quando uma republicação aparecer **depois** de a árvore já ter
aquele slug, o `existe⇒pula` atual a engole em silêncio.

**Sem campo novo:** o frontmatter já grava `link = DEEP_LINK_BASE + GUID`, então a regra é comparação de
string. GUID diferente ⇒ rebusca corpo e reescreve o `.md`; o `key_hash` re-embeda só aquele documento.
Isso acorda o caminho de re-edição do TARF, hoje dormente.

**Junto, e não como critério de desempate:** a verificação **número do corpo × título da listagem**,
registrada como **anomalia de classe própria**. Ela não conserta nada (o acervo está íntegro nos 68
casos) mas teria revelado o caso do `697/20` numa linha de log em vez de uma hora de `curl` — era
**título errado no portal**, dois acórdãos distintos colidindo no slug.

**Cerca explícita: NÃO mexer no desempate do `dedup_por_slug`.** A investigação concluiu que empate de
`dataDocumento` é sinal de **duplicata ou erro de metadado**, não de revisão; trocar o critério
resolveria um problema que não se manifestou **e mascararia o sinal**.

~~**Duas migalhas na mesma leva**~~ — **as duas já estão feitas** (verificadas em 12/08). O comentário
do `anexar_anomalias` está em `tarf.rs:512`, com as três razões que só valem juntas e o fecho *"mudar
isso é decidir sobre o PAR, não sobre uma linha"*. E o comentário da **K8 foi para a chamada**, em
`canonizar.rs:387`, com a separação explícita — *"a atomicidade é do helper"*. O helper ficou sem
menção à K8, corretamente: ele não sabe nada sobre idempotência de derivação.

### 2.5 — §10 do `ESTUDO-busca-hibrida`: decidir o que fazer com os 324

O achado mais consequente da semana e o único que pode mudar a arquitetura de retrieval: **364 arquivos
contêm o termo raro, e 324 deles só na seção `## corpo`, que não é embedada nem indexada.** Os "69
acórdãos com conluio" são **40** na verdade (confirmado por dois métodos), e a procedência do 69 é
desconhecida — ficou registrada como tal.

Consequência que reordena aquele estudo: **indexar o corpo é a única opção que ataca os 324**; calibrar
bandas e busca híbrida apenas disputam a ordenação dos 40. Antes eram alternativas comparáveis; agora
estão em categorias diferentes. Isto é conversa de desenho antes de virar TAREFA — quanto mais cedo se
decide, menos se constrói sobre a suposição antiga.

### 2.6 — A/B de retrieval (agora possível)

O log foi arquivado em `data/eval/log-strategy-2/` e o novo carimba o espaço vetorial, então comparar
eras deixou de ser expurgo e passou a ser filtro.

Duas perguntas para ele: **os 23 documentos cortados pioraram?** e **N = 4.000 seria melhor que
6.000?** — a segunda foi decidida
por conservadorismo, não por dado, e a razão real de **2,17 chars/token** mostrou que 6.000 caracteres
dão até **2.769 tokens**, não os ~1.500 que eu supunha ao escolher o número. Para chegar aos 1.500
pretendidos, o teto seria ~3.300.

> **Correção (12/08):** este item dava a enumeração dos 23 como existente — *"estão enumerados por
> `doc_path` como conjunto nomeado"*. **Não estavam**; o relatório da Fase 0 trazia a contagem e três
> exemplos. A dívida era minha e foi paga: a lista está agora na **§6 do `RELATORIO-TETO.md`**.
>
> Ela traz um detalhe que muda o desenho do A/B: **quatro dos 23 têm comprimento idêntico** (`651` a
> `654-19`, 26.987 chars) e um quinto quase — são acórdãos irmãos com a mesma fundamentação. Tratá-los
> como cinco evidências independentes superestima a confiança de qualquer conclusão sobre eles. E dos
> 23, **22 são acórdãos**: só uma FAQ entra, justamente a do teste de falseamento da memória.

### 2.7 — Sinopses do TARF: **não rodar agora**

Base: `ESTUDO-sinopses-tarf.md`. A justificativa mudou de natureza: o teto já resolveu a memória, então
a sinopse virou questão de **qualidade**, e como qualidade é necessária em **404 documentos** (os que
batem no teto e têm a fundamentação cortada pelo meio), não em 22.476 — nos outros a fundamentação tem
mediana de ~900 chars e já é boa key, escrita pelo relator.

Se avançar: apurar o **RPD real** (a restrição é cota, não tempo), verificar se o **dispositivo** é
extraível nos dois layouts (o corpo truncado em 24.000 chars arrisca entregar o relatório sem a
decisão — pior modo de falha possível num acervo jurídico), sinopsar uma amostra dos 404 com `--limit`,
e A/B de recall. Bloqueios pequenos e conhecidos: `dir_pareceres()` é fixo; o predicado deve virar
**`resumo_info == None`** (proveniência, não presença, senão o lote conta 22.476 reaproveitados e gera
zero); `SINOPSE_PROMPT_VERSION` é const **global** e precisa virar por coleção; prompt próprio proibindo
**nomear as partes** e generalizar caso concreto.

**Interação a não esquecer:** a TAREFA 2.4 passará a **reescrever** o `.md` de acórdão republicado, o
que apagaria uma sinopse. Ao reescrever por republicação, o certo é **re-pendurar** (limpar o
`resumo_info`), não perder nem manter a sinopse do texto antigo.

### 2.8 — NER: rodar o ESTUDO, não implantar a feature

Os gates ficam de pé: **Passo 0 de prevalência** (menos de 1% das perguntas com vazamento residual ⇒ o
estudo **encerra** e as lacunas ficam como limitação documentada) e o go/no-go (FP zero em `C-dec`,
ganho material de recall por lacuna, RSS ≤ ~250 MB, p95 ≤ ~50 ms).

**Refinamento da minha posição, com a lista de FPs na mão:** dos 397 falsos positivos que reprovaram a
Fase 5, só **`Aliomar Baleeiro` (44×)** é nome de pessoa de verdade — `Franca de Manaus`, `Coleta de
Carga`, `Marinha Mercante`, `VGBL`, `Laudo Médico`, `Não Incidência` não são nomes. Um NER
provavelmente elimina ~89% deles. **Mas piora o que resta**, porque detecta o doutrinador citado com
*mais* confiança — e o gate é FP **zero**. É exatamente aí que o diagnóstico **forma × posição** morde:
o que separa doutrinador de parte não é a forma, é onde está no documento. Isso reforça a anonimização
**estrutural**, que resolve por posição o que nenhum reconhecedor lexical nem modelo resolve.

Se o NER passar, o desenho é **sidecar em socket local** chamado só para a cascata residual, com queda
para o resultado determinístico — nunca o pipeline inteiro atrás da rede. O `auli-anon` fica
**biblioteca**.

---

## 3 — Guardado para quando a Fase 5 for retomada

Do patch da Fase 5, **removido da árvore em 22/08/2026** e recuperável em
`git show e3fedc1:docs/0001-feat-anon-Fase-5-nome-de-pessoa-por-dicion-rio-de-pr.patch`
(a Fase 6 é o `0001-feat-anon-Fase-6-raz-o-social-sem-sufixo-via-dicion-.patch` do mesmo commit).

> **Atualização (12/08): o `.patch` é a única cópia, e isso é deliberado.** A branch
> `feat/anon-fase5` foi apagada depois de conferido que o patch em `docs/` é byte a byte o mesmo
> conteúdo — o commit dela (`569e758`) sobrevive como objeto solto, **alcançável por nenhuma ref**,
> até o próximo `gc`. Verificado: `git log --all` sobre `nome_dicionario`, `razao_segmento` e
> `prenomes.txt` volta vazio; nada das Fases 5 e 6 jamais entrou em `main`.
>
> **Atualização (22/08): os dois `.patch` saíram da árvore.** O motivo não tem a ver com o valor
> deles — é que o da Fase 6 nomeava a unidade de atendimento, e um patch não se edita sem deixar
> de ser o que é. O arquivamento não mudou de natureza, mudou de endereço: era "o arquivo em
> `docs/`", passa a ser "o blob em `e3fedc1`". O raciocínio de 12/08 continua valendo — branch
> guarda o código e esquece o motivo, e o motivo segue aqui, ao lado da §5 do `pendencias`. O que
> se perde é a leitura sem `git`; o que não se perde é o conteúdo.

- **A guarda de topônimo por contenção sobre a sequência inteira**, não igualdade sobre o candidato
  recortado. O defeito está visível no código: `valida_candidato` chama `e_toponimo(candidato)` — no
  candidato já cortado no primeiro prenome — e `e_toponimo` faz `binary_search`, isto é igualdade
  exata. Por isso `São Paulo e Paraná` vira `Paulo e Paraná` e `SAO PAULO`, que **está** no arquivo,
  nunca é consultado. Barato: naquele ponto do `scan` o `trecho` (`m.as_str()`) ainda está em mão —
  aplicar a guarda antes do corte é trocar o argumento de uma chamada.
- **Tirar o `e` dos conectores** deste reconhecedor (ele encadeia itens de lista sem gatilho que segure
  o contexto).
- **Exigir sobrenome de dicionário no 2º token** — a mudança que decide a viabilidade, e que exige uma
  fonte de sobrenomes que o patch não tem. **O programa 2.1 pode produzi-la:** as linhas `RECORRENTE`
  dos 1.667 candidatos a pessoa física dão nomes completos reais, logo sobrenomes brasileiros reais.
- **Gate obrigatório:** repetir a medição de FP sobre o acervo antes de aprovar qualquer versão. O corpo
  do TARF é essa bancada, em escala 7× maior que a original (22.476 documentos contra 3.000 / 23 MB).
- Achados menores: o `debug_assert!(v.is_sorted())` deixa a busca binária sem verificação em release
  (funciona hoje porque o arquivo é ASCII maiúsculo e a ordem do Python coincide com a de bytes do
  Rust — coincidência não testada); `normaliza()` é a segunda implementação de um contrato que já existe
  em Python, sem teste de concordância (um teste barato: `prenomes.txt` deve ser ponto fixo de
  `normaliza`); e `scan()` aceita o prenome em qualquer posição enquanto `validate()` exige o primeiro
  token — os dois contratos discordam sobre `Doutor João Silva`.

---

## 4 — Correções de documentação, batcháveis

- ~~**`auli_operations.md` fala de `## sinopse` onde o código usa `## resumo`**~~ — **feito** em
  `887e123`. Eram **seis** ocorrências, não uma: além da seção, o documento dizia `sinopse_*` para o
  frontmatter (é `resumo_*`) e `SinopseInfo` para o tipo (é `ResumoInfo`), heranças da mesma
  renomeação. E ele se contradizia sozinho — o diagrama do §4.5 já usava `## resumo`. Continua
  `sinopse` o que deve: o nome do **passo**, o prompt `config/prompts/sinopse.txt`, as variáveis
  `SINOPSE_API_*` e a const `SINOPSE_PROMPT_VERSION`.
- ~~A nota do `FATIA` deve ficar como **hipótese não verificada**~~ — **superado**. Ela está como
  **refutação**, que é mais forte, e corretamente: ver a correção no §1. Confirmar "hipótese não
  verificada" teria revertido a nota.

---

## 5 — Cercas globais (valem em todas as tarefas acima)

- **`batch_size = 1`**: não tocar. Está registrado que, com lote automático, o mesmo texto saía com
  cosseno 0,978 entre lotes diferentes. Qualquer mudança invalida o acervo.
- **`EMBED_MAX_TOKENS = 8192`**: não tocar. É capacidade do modelo, não política nossa, e baixá-lo
  sozinho não economiza nada — a alocação segue o comprimento **real** do texto (`rs-servicos`, com
  ~130 tokens, custou 79 MiB acima da base, não 8 GiB).
- **`FATIA`**: não mudar **e não medir**. A nota da §1 é documento, não experimento.
- **`dedup_por_slug`**: desempate intocado, por decisão.
- **Invariante de escopo, verificável:** depois de qualquer tarefa acima, `auli update` de uma entidade
  deve produzir pack **byte a byte idêntico** ao de produção (o manifesto pode diferir no `built_at`).
  Se não produzir, algo saiu do escopo — **pare e diga**.
- **Suposição que não bate com o código: pare e reporte, não contorne.** Nesta trilha isso aconteceu
  seis vezes e em todas o relatório valeu mais que o contorno teria valido — inclusive quando a
  suposição errada era minha, que foi a maioria das vezes.
