# ESTUDO — Identidade de documento e o `update` incremental

**Data:** 09/08/2026 · **Escopo:** análise (§1–8) e proposta de implementação (§9). Nenhum código escrito.
**Pergunta de partida (Carlos):** *"Os pareceres e o TARF são imutáveis (mas podem ser reeditados) —
conseguimos um identificador único para cada URL?"*

**Resposta curta:** a URL **não** serve como identidade universal — quebra em duas das sete coleções
medidas, e por um motivo estrutural, não por defeito de coleta. Mas a identidade existe, já está
implementada e é **estável**: é o `doc_path`. E os documentos **não são imutáveis** — 68 dos 22.590
acórdãos do TARF são republicações do mesmo número, o que o pipeline já detecta e resolve.

---

## 1. A URL como identidade: medido, não suposto

Contagem de `link:` no frontmatter de toda a árvore, hoje:

| coleção | documentos | links distintos | serve como id? |
| --- | ---: | ---: | --- |
| `rs/tarf` | 22.476 | 22.476 | ✅ |
| `rs/pareceres` | 372 | 372 | ✅ |
| `sp/pareceres` | 15.605 | 15.605 | ✅ |
| `sc/pareceres` | 1.743 | 1.743 | ✅ |
| `rs/servicos` | 586 | 586 | ✅ |
| **`pr/pareceres`** | **2.060** | **20** | ❌ |
| **`rs/faqs`** | **1.947** | **252** | ❌ |

As duas falhas têm a mesma causa: **a URL aponta para o contêiner, não para o documento.**

- **PR** — o link é um PDF que agrupa um ano inteiro de consultas.
  `…/SEFADOCUMENTOS/116201602016.pdf` cobre **180** pareceres de 2016. Não há URL por consulta:
  o portal não publica uma.
- **RS faqs** — o link é a página temática, e cada página traz várias perguntas.
  `…/modelo-operacional` cobre **30** FAQs. A página é a unidade de publicação; a pergunta não é.

Isso não é conserto de scraper. É propriedade da fonte: nesses dois portais **o documento não tem
endereço próprio**. Qualquer desenho que assuma "uma URL = um documento" quebra em 4.007 documentos
(8,3% do acervo) no dia em que for ligado.

## 2. A identidade que já existe — e é melhor que a URL

O `Documento::doc_path` ([auli-contract/src/lib.rs:247](../auli-server/crates/auli-contract/src/lib.rs))
já resolve isso, com uma regra por coleção:

| coleção | chave | por quê |
| --- | --- | --- |
| pareceres, tarf | `slug(titulo)` | o título **é** o número do documento |
| servicos | `slug(titulo)` + hash do `link` | o link é a identidade real |
| faqs | `slug(titulo)` + hash de `(url, pergunta)` | a url sozinha não distingue perguntas da mesma página |

**Verificado: 100% único nas sete coleções**, inclusive nas duas em que a URL colide. Os nomes de
arquivo na árvore são a prova viva disso — são exatamente esses slugs, e não há colisão.

Repare que o `doc_path` é **mais forte** que a URL: onde a URL basta, ele a usa (serviços); onde não
basta, ele acrescenta o que falta (faqs); e onde ela sequer existe por documento, ele não depende
dela (PR).

**Conclusão da pergunta original:** não precisamos de um identificador novo. Precisamos **promover o
`doc_path` de nome de arquivo a chave do pack** — que é uma decisão diferente, e o resto deste
estudo é sobre ela.

## 3. "Imutáveis, mas podem ser reeditados" — medido

A premissa está meio certa, e a metade errada é a que importa.

O scraper do TARF já registra o problema em código:

> *"O título do acórdão não é identidade: 60 números aparecem em 2+ GUIDs, e conferindo o conteúdo
> são revisões/republicações do mesmo acórdão (num caso, byte a byte idênticas)."*

O `dedup_por_slug` colapsa as revisões mantendo a de `dataDocumento` mais recente, com desempate
pelo GUID para não depender da ordem do portal. Os perdedores vão para
`data/rs/raw/tarf-anomalias.txt`, porque a decisão sobre eles é humana.

**Números de hoje** (`tarf-anomalias.txt`, coleta completa de 22.590):

| ocorrência | quantidade | % do acervo |
| --- | ---: | ---: |
| `SLUG-DUPLICADO` (revisões) | 68 | 0,30% |
| `SEM-EMENTA` | 29 | 0,13% |
| `FALHA-CORPO` | 17 | 0,08% |

**Duas identidades concorrentes, e a escolha importa:**

- **GUID da URL** = identidade da **revisão**. Uma re-edição é um documento NOVO; a anterior vira
  órfã e precisa ser removida.
- **`slug(numero)`** = identidade do **acórdão**. Uma re-edição é o MESMO documento com corpo novo;
  o vetor precisa ser recalculado, e nada vira órfão.

A árvore usa a segunda, e é a certa para o incremental: substituição no lugar, sem crescimento
espúrio do acervo. Mas ela transforma "imutável" em "**estável na chave, mutável no conteúdo**" — e
é isso que o desenho tem de suportar.

## 4. O que o pack guarda hoje, e o que falta

`DocumentoPack` ([lib.rs:290](../auli-server/crates/auli-contract/src/lib.rs)) tem: `kind`, `trilha`,
`titulo`, `ementa`, `resumo`, `link`, `doc_path`. **O `doc_path` já está lá.**

O que **não** está:

1. **O id é posicional.** `ingest_items` faz `(1..=n).map(|i| format!("id-{i}"))` sobre a árvore em
   ordem alfabética. Inserir um documento no meio desloca o id de todos os seguintes — é por isso
   que existe o `Writer::reset`, e é a razão real de o update ser total.
2. **Não há hash do `text_to_embed`.** O `pack()` descarta a key de propósito ("já foi consumida — o
   vetor É ela"). Sem ela, não há como saber se um documento mudou sem re-embeddá-lo. **É a peça que
   falta**, e é pequena: 8 ou 16 bytes por registro.

## 5. Quanto se ganharia — o número que justifica o trabalho

Taxa de publicação, medida pelo ano no número do documento:

| coleção | por ano | por mês | acervo | % re-embeddado à toa |
| --- | ---: | ---: | ---: | ---: |
| `rs/tarf` | 680–1.020 | ~70 | 22.476 | **99,7%** |
| `sp/pareceres` | ~1.000 | ~85 | 15.605 | **99,5%** |

Somando com a taxa de re-edição (0,30%), uma re-coleta mensal do TARF toca **~0,6% do acervo** e
re-vetoriza 100% dele. A ~6 textos/s medidos, são **~72 minutos para produzir ~140 vetores novos**.

O caso que dói mais hoje é o `update-rs-faqs.sh`: atualizar 1.947 FAQs custa 72 min porque o
`auli update` re-vetoriza a entidade inteira — e 22.476 acórdãos intactos vão junto. Antes de o TARF
fechar eram 16 min. **A §31 deixou de ser otimização e virou bloqueio operacional.**

## 6. O desenho que os dados sustentam

As afirmações que as medições acima sustentam. A proposta de implementação em si é o §9.

1. **Id do pack = `doc_path`.** Já é único, já é estável, já está no pack. Não inventar chave nova.
2. **Guardar o hash do `text_to_embed`** por registro. Diferente ⇒ re-embeddar; igual ⇒ reaproveitar
   o vetor. É o único jeito de detectar a re-edição do §3 sem pagar o embedding.
3. **Remoção = diff de conjuntos de id.** Hoje o `reset` resolve por construção; no incremental vira
   explícito, e é o ponto onde um erro apaga documento bom.
4. **A troca de esquema de id muda o formato do pack** ⇒ uma migração, uma vez — mas **sem
   re-embeddar**. A §31 registra a ironia de "para nunca mais re-embeddar tudo, re-embedda-se tudo
   uma última vez"; ela não se aplica aqui, e o §9.0 mostra por quê.

**O que NÃO muda:** o `docs_hash` do manifesto continua cobrindo a árvore inteira e recusando o boot
na divergência — é ortogonal. E a guarda `recusar_pareceres_sem_sinopse` tem de continuar rodando
sobre a árvore toda, não sobre o delta: uma sinopse apagada em documento antigo precisa seguir sendo
recusada.

## 7. Os testes — o que precisa existir antes de confiar

Em ordem de importância, não de esforço.

### 7.1 Equivalência total vs. incremental (o teste que decide tudo)

Vetorizar do zero e vetorizar incrementalmente têm de produzir packs **byte a byte idênticos**.
É a única garantia real de que o atalho não introduziu deriva, e o precedente existe: foi assim que
validamos o fatiamento do `embed_dense` hoje (`rr` e `sc`, packs idênticos).

Cenários que o teste tem de cobrir, cada um com o pack final comparado ao do zero:

| cenário | o que exercita |
| --- | --- |
| nada mudou | o caminho de 100% cache — o mais comum |
| documento acrescentado no FIM | crescimento simples |
| documento acrescentado no MEIO | é aqui que o id posicional quebra hoje |
| documento REMOVIDO | o diff de conjuntos; o modo de falha é órfão no pack |
| documento com corpo alterado, mesmo id | a re-edição do §3 |
| documento renomeado (id muda, corpo igual) | remoção + inserção, não atualização |
| dois documentos trocando de posição | prova que a ordem não vaza para o vetor |

### 7.2 Estabilidade do `doc_path` sob mudança de título

O `doc_path` de jurisprudência é `slug(titulo)`. Se o portal corrigir um espaço ou um acento no
número, **o id muda** — e o incremental verá remoção + inserção, não atualização. Consequência: o
vetor é recalculado (correto) mas o histórico se perde (aceitável).

O que o teste precisa fixar é o contrário disso: **variações que NÃO devem mudar o id**. O
`mddoc::slug` já normaliza acento e espaço; há um teste do `numero_colapsa_espaco_do_titulo` no
scraper. Falta a garantia no nível do `doc_path`, com casos reais das sete coleções.

### 7.3 Unicidade do `doc_path` como invariante, não como sorte

Hoje a unicidade é fato observado (medido no §2), não invariante testado. Um teste sobre a árvore
real — "nenhuma coleção tem dois documentos com o mesmo `doc_path`" — é barato e pega o dia em que
uma entidade nova violar a regra. Sem ele, o modo de falha do incremental é **um documento
sobrescrever outro em silêncio**.

### 7.4 O hash do `text_to_embed` detecta o que deve, e só isso

- Corpo alterado ⇒ hash diferente.
- Reprocessamento sem alteração ⇒ hash idêntico (senão o cache nunca acerta e o ganho evapora).
- Mudança em campo que **não** entra na key (por exemplo `resumo_gerada_em`) ⇒ hash idêntico.

Este último importa: a key é composta por `compose_*_text_to_embed`, e nem todo campo do frontmatter
entra nela. Um hash sobre o arquivo inteiro, em vez de sobre a key, invalidaria o cache a cada
regeneração de sinopse.

### 7.5 Guardas que não podem afrouxar

- `recusar_pareceres_sem_sinopse` sobre a árvore TODA, com um teste que apague a sinopse de um
  documento **fora do delta** e exija a recusa;
- `docs_hash` continua batendo depois de um incremental;
- a identidade do embedder (`STRATEGY_VERSION`) invalida o cache inteiro — mudar a fórmula da key
  tem de forçar reembed geral, não reaproveitar vetores de estratégia antiga. **É o modo de falha
  mais perigoso do desenho**, porque passa em todos os outros testes.

## 8. O que ainda não sabemos

Honestamente, três coisas — e a primeira é a que eu mediria antes de escrever qualquer código.

1. **A taxa de mudança real entre duas coletas.** O §5 estima por data de publicação, não por
   observação: nunca comparamos duas árvores do mesmo acervo em momentos diferentes. Um `docs_hash`
   por documento, gravado numa coleta e conferido na seguinte, dá o número verdadeiro em uma rodada.
   Se ele vier muito acima de 1%, o desenho muda.
2. **Se os outros portais republicam como o TARF.** As 68 revisões são um dado do RS. SP, PR e SC
   podem ter comportamento diferente — inclusive pior, se lá o número for reaproveitado para um
   documento *distinto*, o que quebraria a chave.
3. **O custo do reembed de migração.** Trocar o esquema de id re-vetoriza tudo uma vez: hoje isso é
   ~72 min só para o RS, e o SP some. Com o fatiamento de hoje a memória deixou de ser o limite, mas
   o tempo não.

---

## 9. A proposta de implementação

### 9.0 A notícia boa: a migração NÃO precisa re-embeddar

A §31 registra uma ironia — *"para nunca mais re-embeddar tudo, re-embedda-se tudo uma última vez"*.
**Ela está errada, e dá para provar pelo formato do pack.**

Cada registro é `{id, embedding, document}`, e o `document` é o `DocumentoPack` serializado — que
**já contém o `doc_path`**:

```json
{"id":"id-1","embedding":[…],"document":"{…,\"doc_path\":\"docs/servicos/alteracao-de-senha-802e672c.md\"}"}
```

Verificado em duas coleções (`rr-servicos`, `sc-pareceres`): `doc_path` presente em 100% dos
registros, único, e na mesma ordem alfabética da árvore. Então trocar `id-N` por `doc_path` é
**reescrever o campo `id` a partir do payload que já está no arquivo** — sem tocar num vetor.

O `key_hash` precisa do `text_to_embed`, que exige ler a árvore `.md` e recompor a key pelo ponto
único — mas isso é I/O e string, não inferência. Custa segundos, não horas.

**Consequência:** a migração é uma reescrita local de ~50 MB de JSON por entidade. Não há reembed de
migração, e portanto não há a barreira de "some o SP por uma hora" que travava a decisão.

### 9.1 O que muda, em quatro peças

| # | onde | mudança |
| --- | --- | --- |
| 1 | `vector-store` | `upsert` passa a casar por id de verdade (hoje já substitui por id — só falta o id ser estável) e ganha `remove(&[id])` |
| 2 | `auli-contract` | `DocumentoPack` ganha `key_hash: String` — o hash do `text_to_embed`, não do arquivo |
| 3 | `auli-cli/update.rs` | `ingest_items` deixa de fazer `reset` + `1..=n`: lê o pack atual, compara `key_hash` por `doc_path`, embedda só a diferença, e remove o que sumiu |
| 4 | manifesto | um campo de **versão de formato do pack**, distinto do `strategy_version` |

Sobre a peça 4, que é a menos óbvia: `strategy_version` significa *"o que entra no vetor mudou"* e
obriga reembed. Aqui o vetor **não** muda — muda o esquema do arquivo. Misturar os dois forçaria um
reembed desnecessário e apagaria a distinção que a §34.2 depende. São conceitos diferentes e
precisam de campos diferentes.

**O `id` não é lido por nada em produção** — verificado: `id-N` só aparece no ponto que o gera
(`update.rs:308`) e em testes de `vector-store`. Trocar o esquema não quebra o servidor, o MCP nem o
retrieve.

### 9.2 As fases, na ordem em que reduzem risco

**Fase 0 — medir a taxa de mudança real.** Antes de qualquer código de produção: gravar o
`key_hash` de cada documento numa coleta, conferir na seguinte, e publicar o número. É o §8.1 do
estudo, e é barato. Se a taxa vier acima de ~5%, o ganho encolhe e o desenho merece revisão. Sem
este número, tudo abaixo é aposta.

**Fase 1 — formato do pack: `doc_path` como id + `key_hash`.** Ainda com `reset` + reescrita total,
ou seja **sem mudança de comportamento**. Entrega: packs no formato novo, migração sem reembed, e a
prova de que `auli server`, MCP e retrieve não notaram diferença.
Verificação: os vetores saem byte a byte idênticos aos de hoje (mesmo teste que validou o
fatiamento), e a suíte inteira passa.

**Fase 2 — o caminho incremental.** `ingest_items` passa a ler o pack anterior, casar por
`doc_path`, e embeddar só quem tem `key_hash` diferente ou ausente. Sem remoções ainda: documento
que sumiu da árvore continua no pack (regressão temporária e conhecida, coberta por um teste que a
declara).
Verificação: os sete cenários do §7.1, com o pack final comparado byte a byte ao de um update do
zero.

**Fase 3 — remoções.** O diff de conjuntos, que é onde um erro apaga documento bom. Fecha a
regressão da fase 2.
Verificação: o cenário "documento removido" do §7.1 passa a exigir igualdade, e um teste de
propriedade — aplicar N mutações aleatórias à árvore e exigir que incremental == do zero.

**Fase 4 — desligar o atalho quando a identidade muda.** `STRATEGY_VERSION` ou `EMBED_MODEL_ID`
diferentes do manifesto ⇒ **ignorar o cache inteiro** e re-embeddar tudo. É a guarda mais importante
das quatro fases e a mais fácil de esquecer, porque **passa em todos os outros testes**: um pack com
vetores de estratégia antiga e `key_hash` correto parece perfeitamente saudável.

### 9.3 O que pode dar errado, em ordem de gravidade

1. **Reaproveitar vetor de estratégia antiga.** Silencioso, e degrada a recuperação sem erro em
   lugar nenhum. Mitigação: a fase 4, mais um teste que muda o `STRATEGY_VERSION` e exige reembed
   total.
2. **Apagar documento bom no diff de remoção.** Uma falha do scraper que devolva a árvore pela
   metade viraria "remover 11 mil acórdãos". Mitigação: um teto de segurança — recusar um delta que
   remova mais que X% do acervo sem confirmação explícita. O `docs_hash` **não** protege disso: ele
   valida o pack contra a árvore, e a árvore já estaria errada.
3. **`doc_path` colidir numa entidade nova.** Hoje é fato observado, não invariante. Mitigação: o
   teste do §7.3, que é barato e deveria entrar já na fase 1.
4. **Hash sobre o arquivo em vez de sobre a key.** Invalidaria o cache a cada regeneração de sinopse
   (o `resumo_gerada_em` muda). Mitigação: o teste do §7.4.

### 9.4 Custo e retorno

O trabalho é **das fases 1 a 3**, e a peça difícil é a 3 — as outras são mecânicas. Não arrisco
estimativa de horas; o que dá para afirmar é o retorno, com os números do §5:

| | hoje | depois |
| --- | ---: | ---: |
| re-coleta mensal do TARF | ~72 min | segundos |
| `update-rs-faqs.sh` | ~72 min | ~1 min |
| coleta de entidade nova | inalterada | inalterada |

E a §31 pode ser fechada — ela é a última pendência de performance com ordem de grandeza em jogo.

---

## Apêndice — como cada número foi obtido

```bash
# doc_path presente, único e alinhado com a árvore (§9.0)
python3 -c "import json;print([json.loads(r['document'])['doc_path'] for r in json.load(open('data/rr/packs/rr-servicos.json'))['records']][:2])"

# links por coleção (§1)
grep -h "^link:" data/<ent>/docs/<col>/*.md | sed 's/^link: *//' | sort -u | wc -l

# revisões e demais anomalias (§3)
awk -F'\t' '{print $1}' data/rs/raw/tarf-anomalias.txt | sort | uniq -c

# publicação por ano (§5)
grep -h "^titulo:" data/rs/docs/tarf/*.md | grep -oE '[0-9]{2,3}/[0-9]{2}' | cut -d/ -f2 | sort | uniq -c
```

Nenhum número deste estudo é estimativa de terceiros: todos saem da árvore em disco em 09/08/2026,
com 48.408 documentos.
