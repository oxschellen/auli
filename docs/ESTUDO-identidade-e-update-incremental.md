# ESTUDO — Identidade de documento e o `update` incremental

**Data:** 09/08/2026 · **Escopo:** só análise; nenhuma linha de código proposta para escrever agora.
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

Não é proposta de implementação — é o que as medições acima permitem afirmar.

1. **Id do pack = `doc_path`.** Já é único, já é estável, já está no pack. Não inventar chave nova.
2. **Guardar o hash do `text_to_embed`** por registro. Diferente ⇒ re-embeddar; igual ⇒ reaproveitar
   o vetor. É o único jeito de detectar a re-edição do §3 sem pagar o embedding.
3. **Remoção = diff de conjuntos de id.** Hoje o `reset` resolve por construção; no incremental vira
   explícito, e é o ponto onde um erro apaga documento bom.
4. **A troca de esquema de id muda o formato do pack** ⇒ um reembed geral de migração, uma vez.
   Ironia já registrada na §31: para nunca mais re-embeddar tudo, re-embedda-se tudo uma última vez.

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

## Apêndice — como cada número foi obtido

```bash
# links por coleção (§1)
grep -h "^link:" data/<ent>/docs/<col>/*.md | sed 's/^link: *//' | sort -u | wc -l

# revisões e demais anomalias (§3)
awk -F'\t' '{print $1}' data/rs/raw/tarf-anomalias.txt | sort | uniq -c

# publicação por ano (§5)
grep -h "^titulo:" data/rs/docs/tarf/*.md | grep -oE '[0-9]{2,3}/[0-9]{2}' | cut -d/ -f2 | sort | uniq -c
```

Nenhum número deste estudo é estimativa de terceiros: todos saem da árvore em disco em 09/08/2026,
com 48.408 documentos.
