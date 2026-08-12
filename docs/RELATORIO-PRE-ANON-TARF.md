# RELATÓRIO — apuração prévia da TAREFA-ANON-TARF

**Para:** Claude Fable
**Natureza:** medição, antes de qualquer código. Nada foi implementado; `tools/anon-tarf` não existe.
**Motivo de reportar antes:** três suposições da TAREFA foram verificadas, **uma não bate**, e ela
muda o valor esperado do programa. Melhor decidir agora que depois de 300 linhas.

---

## 1 — (f) A divergência de documentação: quem erra é o `ESTUDO-anon-ner-v2`

**O código concorda com o `auli-anon_pendencias`.** Em
[`reconhecedores/`](../auli-server/crates/auli-anon/src/reconhecedores/) não existe
`nome_dicionario.rs` nem `razao_segmento.rs`, e o registro do
[`lib.rs:64`](../auli-server/crates/auli-anon/src/lib.rs#L64) monta **doze** reconhecedores —
`CnpjAlfanumerico`, `TelefoneBr`, `InscricaoEstadual`, `Cep`, `Protocolo`, `Ga`, `Renavam`, `Placa`,
`DataNascimento`, `RazaoSocial`, `Endereco`, `NomePessoa`. Nenhum é das Fases 5 ou 6.

A tabela do §1 do `ESTUDO-anon-ner-v2` lista as duas como cobertura vigente, com as confianças 0,6 e
0,65 e os números dos dicionários. **É a descrição dos patches, não do que roda.** É a linha mais
cara dos três documentos: um estudo de NER que parta dali mede o ganho contra uma baseline que não
existe, e superestima o determinístico exatamente nas duas lacunas que o NER deveria atacar.

**Baseline real: Fases 1–4.** Não corrigi o estudo — a TAREFA manda reportar, não corrigir.

## 2 — A fronteira de reuso aguenta, e isso responde ao §8 antes de começar

Eu esperava ter de tocar no `auli-anon` para o relatório por reconhecedor. **Não é preciso.** O
`PromptMappingEntry` do `cloakrs-core` já expõe, por detecção: `placeholder`, `entity_type`,
`original`, `span_start`, `span_end`, `confidence` e `recognizer_id`. Todo o §3.2 sai da API pública
(`anonimizar` → `Anonimizado { texto, mapping }`), sem contorno e sem mudança na biblioteca.

O teste da fronteira, portanto, passa no papel. Só a implementação confirma.

## 3 — (g) As linhas rotuladas: medidas nos 22.476, não inferidas de dois

### 3.1 As duas eras existem, e a virada é limpa

| período | valor na mesma linha | valor após quebra |
|---|---:|---:|
| 2005–2015 | 100% | 0 |
| 2016–2019 | transição | transição |
| 2020–2026 | 0 | 100% |
| **acervo** | **16.708 (74,3%)** | **5.766 (25,7%)** |

O layout antigo põe valor e rótulo na mesma linha; o moderno separa por quebra. A transição começa
em 2016 e se completa em 2020.

### 3.2 A taxa de falha de extração é ~0%, mas quase virou 2,1% por armadilha minha

A primeira medição deu **463 documentos sem rótulo (2,1%)** — eu já ia reportar isso como viés
temporal, porque os 463 eram todos da era antiga. **Estava errado: os 463 têm rótulo, no plural sem
parênteses** (`RECORRENTES:` / `RECORRIDAS:`), forma que a minha regex não cobria. Com ela incluída,
sobram **2** documentos em 22.476 — e os dois têm o rótulo partido por extração de PDF
(`RECORRENTE S :`), recuperável com `\s*` dentro da palavra.

**Cobertura efetiva do gold set: 22.454 de 22.476 (99,9%).** Registro a armadilha porque ela é do
tipo que produz "gold set silenciosamente enviesado" — o cenário que o §7 nomeia como inaceitável —
e porque a falha aparecia concentrada numa era, o que faria a explicação errada parecer boa.

### 3.3 O valor não é uma entidade só

Em **11,2%** dos casos o campo traz duas partes unidas por ` E `, e em **14,6%** uma delas é
`FAZENDA ESTADUAL` — que não é PII e não deve entrar no gold set. Há ainda `AS MESMAS` como valor de
`RECORRIDO`. O extrator precisa quebrar por ` E `, descartar a Fazenda e os anafóricos, e só então
formar o alvo.

---

## 4 — A suposição que não bateu: **o TARF é sanitizado na fonte em 56% dos casos**

Esta é a parte que muda o plano. A TAREFA parte de que "o TARF **nomeia as partes**", em contraste
com os pareceres da §6.1. **Nomeia em parte.** Distribuição dos 22.454 valores de `RECORRENTE`:

| | n | % |
|---|---:|---:|
| **redigido na própria fonte** (`(...)`) | 12.568 | **56,0%** |
| **nome real presente** | 7.212 | **32,1%** |
| — destes, pessoa jurídica (sufixo ou termo de segmento) | 5.545 | 24,7% |
| — destes, sem sufixo (candidato a pessoa física) | 1.667 | 7,4% |
| só `FAZENDA ESTADUAL` / `AS MESMAS` | 2.674 | 11,9% |

O `(...)` literal é o mesmo mecanismo que a §6.1 já observou nos pareceres do RS. **O TARF não é a
exceção limpa que a TAREFA supôs — é o mesmo saneamento de origem, aplicado a pouco mais da metade
do acervo.**

### O que isso faz com cada um dos cinco motivos da §1

- **(1) corpus auto-rotulado — permanece, com o denominador certo.** 7.212 valores reais, 6.283
  distintos no total. Não são "milhares" no sentido de 22 mil, mas 7 mil é gold set de sobra, e é
  material real anotado sem uma linha de anotação manual.
- **(2) medir a lacuna 5 — permanece, e é o número mais valioso daqui.** Dos nomes reais, **1.667
  (23%) não têm sufixo societário nem termo de segmento óbvio** — é a primeira medição direta do
  tamanho da lacuna 5, que hoje é suposição.
- **(3) testar a §6.1 — a resposta já está acima, e é "parcialmente".** A §6.1 precisa ser reescrita
  para dizer que o saneamento de origem cobre 56% do TARF, não que "não há o que anonimizar". Os 44%
  restantes são exposição real que ninguém tinha contado.
- **(4) fixtures commitáveis — permanece integralmente.**
- **(5) dicionário minerado — permanece, sobre os 5.545 nomes de pessoa jurídica.**

**Nenhum dos cinco morre.** Dois encolhem de escala e um (o 3) inverte de sinal: deixa de ser
"confirmar que o TARF é exceção" e passa a ser "medir quanto do TARF a fonte já protege".

---

## 5 — Uma ressalva metodológica, antes de medir

O valor do `RECORRENTE` aparece **na própria linha rotulada**, dentro do `## corpo`. Se o recall
contar essa ocorrência junto com as do texto livre, o número sobe por um contexto regular demais — e
cria a tentação de "consertar" o recall acrescentando `RECORRENTE` como gatilho do
`NomePessoaRecognizer`, o que seria ganhar o teste sem ganhar nada em produção.

**Proposta: duas taxas separadas** — mascarado na linha rotulada, e mascarado nas ocorrências fora
dela. A segunda é a que mede capacidade; a primeira é quase um controle de sanidade.

Isto é irmão da cerca do §5 da TAREFA (não minerar e medir no mesmo texto), aplicado dentro do mesmo
documento em vez de entre metades do corpus.

---

## 6 — O que peço antes de implementar

Nada bloqueia — os cinco motivos sobrevivem e o programa continua valendo. Mas duas coisas mudam de
peso e são sua decisão:

1. **O gold set é de 7.212, não de 22.476.** Se isso muda o desenho da amostra da §3.4 ou o critério
   do gate, é melhor saber agora.
2. **A §6.1 precisará ser reescrita de qualquer jeito** — o número já existe, independente do resto
   do programa rodar. Posso separar essa correção num commit próprio.

E confirmo o que já está decidido e não vou tocar: patches das Fases 5 e 6 não aplicados, NER não
rodado, `auli-anon` não alterado, nada escrito em `data/rs/docs/tarf/` nem em pack.
