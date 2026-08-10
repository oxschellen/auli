# Registro — âncoras de restart para o scraping incremental da jurisprudência

**Data:** 10/08/2026 · **Escopo:** medição das fontes de jurisprudência (TARF, pareceres RS/SC/SP/PR)
para decidir **se e como** ancorar um ponto de reinício, evitando relistar o acervo inteiro a cada
coleta. Nenhuma linha de scraper alterada — este documento é só o registro das medições e do que
elas significam.

**Motivação declarada:** "há produção de documentos; o scraping não pode recomeçar da primeira
página web todas as vezes". A medição do TARF **refutou parcialmente a premissa** (§1.6) e inverteu
a recomendação (§6).

**Convenção deste registro:** distingo explicitamente **medido** (saiu no retorno do `curl`),
**deduzido** (inferência com a evidência ao lado) e **não estabelecido** (fica como pendência).
Nenhuma inferência é apresentada como fato.

---

## 0. A estratégia de cada tabela/entidade, em linguagem simples

O princípio único, do qual todo o resto decorre: **listar é barato; buscar o corpo do documento e
gerar o embedding é caro.** Então listamos sempre tudo, e gastamos só no que é novo. Quem diz o que é
novo não é um arquivo de controle — é a **própria árvore de `.md` já coletada**. Isso se auto-repara:
o que escapou numa rodada entra na seguinte, sem ninguém ter previsto nada.

| entidade / tabela | como listamos | o que buscamos | âncora |
|---|---|---|---|
| **TARF** — acórdãos (API JSON) | tudo, 46 requisições ≈ 45 s | corpo só dos ausentes na árvore | a árvore |
| **RS** — pareceres (WebForms) | tudo, ~17 postbacks ≈ s | detalhe só dos ausentes | a árvore |
| **SC** — COPAT (WebForms) | tudo, **1 GET** (6,8 MB) | detalhe só dos ausentes | a árvore |
| **SP** — RCs (SharePoint) | tudo, 136 páginas ≈ 2,3 min | nada extra: o corpo vem na listagem | a árvore |
| **PR** — consultas (PDF anual) | 1 GET no índice | **o PDF do ano corrente** | o ano (calendário) |
| **RS** — serviços e FAQs | tudo, sempre | tudo, sempre | nenhuma (rebuild total) |

**TARF (acórdãos).** Pedimos a listagem inteira do grupo 14543 em páginas de 500. Para cada acórdão
listado, se já existe `.md` na árvore, pula; se não existe, busca o corpo e escreve o `.md`. Depois o
update re-embeda apenas os documentos cuja chave de embedding mudou. Nenhuma janela de data, nenhum
"último número visto".

**RS — pareceres.** Igual ao TARF, só muda o transporte: a listagem é paginada por postback do
ASP.NET (com cookie e `__VIEWSTATE`), via `curl` subprocess. ~331 chaves em ~17 páginas.

**SC — COPAT.** O caso mais simples da frota: uma única requisição traz o índice completo de todas as
consultas. Diff contra a árvore, busca o detalhe dos inéditos.

**SP — RCs.** A biblioteca do SharePoint é **mista** (RCs convivem com Comunicados DICAR e páginas de
sistema), então filtramos as RCs pelo padrão do nome do arquivo já no cliente. Vantagem: a própria
listagem devolve ementa e corpo, então não há segunda requisição por documento.

**PR — consultas.** Único caso diferente, porque não existe documento individual: cada ano é **um PDF
com todas as consultas daquele ano**, que precisa ser baixado e convertido em texto. Anos encerrados
nunca mudam ⇒ ficam congelados no cache. Só o PDF do **ano corrente** é rebaixado e reprocessado a
cada rodada. A "âncora" aqui é o calendário, não um arquivo de estado.

**RS — serviços e FAQs.** Regra à parte, e deliberadamente oposta: **rescrapeia, reconstrói a árvore e
re-embeda tudo, sempre**, gerando pack novo completo. São poucos registros, mudam muito no portal, e
descobrir com segurança o que foi retirado de lá é complexo. Reconstruir do zero é mais simples e mais
seguro que diferenciar — e é a mesma doutrina que a árvore dessas coleções já segue (delete + rebuild).

**Por que não guardamos um "ponto de restart".** A tentação natural era gravar a última data ou o
último número coletado. Duas medições mataram a ideia (§1.6, §2.4): os portais publicam **em lotes
esparsos** e carregam documentos **antigos com atraso** — um acórdão de 2024 apareceu em 2026, uma RC
de 2025 no meio do lote de 2026. Qualquer marca por número ou ano perderia esses casos em silêncio. E
como relistar custa segundos ou poucos minutos, o preço de não ter marca é irrelevante frente ao
benefício de continuar verificando, a cada rodada, que **coletamos tantos quanto o portal anuncia**.

## 1. TARF — `tarf.sefaz.rs.gov.br` (Angular SPA "Legislacao 3.0")

### 1.1 Tabela consultada

Endpoint de listagem `GET /api/PesquisaDocumentos`, grupo **14543 = "ACÓRDÃOS A PARTIR DE 2005"**
(22.590 itens em 08/08/2026, 22.590 confirmados nos testes de 10/08/2026).

### 1.2 Parâmetros — o que foi exercitado

| Parâmetro | Capitalização | Exercitado em 10/08 | Comportamento observado |
|---|---|---|---|
| `TamanhoPagina` | maiúsc. | sim (`10`, `500`) | obrigatório; `500` devolveu os 364 itens da janela de 2026 numa só página |
| `NumeroPagina` | maiúsc. | sim (`1`) | obrigatório, base 1 |
| `grupos` | **minúsc.** | sim (`14543`) | array repetível; id **numérico** (o item devolve `grupoId` GUID — não é o que a query aceita) |
| `DataInicio` | maiúsc. | **sim** (`2026-07-01T00:00:00.000Z`, `2026-01-01T00:00:00.000Z`) | **funciona**; formato ISO com `Z` aceito (o mesmo que a SPA envia via `.toISOString()`) |
| `DataFim` | maiúsc. | não | inclusividade do limite **não estabelecida** |
| `areas`, `assuntos` | minúsc. | não | arrays repetíveis (do discovery) |
| `NumeroDoc`, `Termos` | maiúsc. | não | opcionais (do discovery) |

### 1.3 Campos do envelope de resposta

| Campo | Significado |
|---|---|
| `totalItens` | total **da consulta**, já com filtro aplicado — é a base do invariante `únicos == total` |
| `numeroPagina`, `tamanhoPagina`, `totalPaginas` | eco da paginação |
| `temPaginaPosterior` | booleano de continuação |
| `resultado[]` | os itens |

### 1.4 Campos do item — semântica estabelecida nos testes

| Campo | Exemplo | Significado |
|---|---|---|
| `titulo` | `ACÓRDÃO 510/24` | rótulo humano; é a base do `doc_path`/slug na árvore |
| `numero` | `51024` | número achatado (510/24 → 51024) |
| `grupo` / `grupoId` | `ACÓRDÃOS A PARTIR DE 2005` / GUID | grupo do documento |
| `area` | `TARF` | área do backend multiárea (serve também `cage`, `receita`, `tesouro`) |
| `ementa` | `""` | **sempre vazia na listagem**; a ementa real está no corpo |
| `id` | GUID | chave para `/api/Documentos/{id}` e `/api/documentos/{id}/DocumentoHtml` |
| `dataDocumento` | `2026-05-23T11:49:44` | **carimbo de LOTE de publicação** — ver §1.5 |
| `dataCriacao` | `2026-05-22T15:31:43.34` | **inserção do registro**, granularidade real por documento |
| `dataAtualizacao` | `2026-05-22T18:06:37.37` | última alteração do registro |

**`dataDocumento` NÃO é a data de julgamento.** Medido: o `ACÓRDÃO 510/24` (numeração de 2024) traz
`dataDocumento` de 17/07/2026 — dois anos depois. É um carimbo do portal, não do tribunal. Corolário
já registrado: **a numeração do documento não serve de âncora de restart**, porque a ordem de
publicação não respeita a ordem numérica.

### 1.5 Teste 1 — o filtro funciona; qual campo ele usa

```bash
UA='AuliBot/0.1 (+https://github.com/oxschellen/auli; carlos.schellenberger@gmail.com)'
B='https://tarf.sefaz.rs.gov.br/api/PesquisaDocumentos'
curl -s "$B?TamanhoPagina=10&NumeroPagina=1&grupos=14543&DataInicio=2026-07-01T00:00:00.000Z" \
  | jq '{totalItens, itens: [.resultado[] | {titulo, dataDocumento, dataCriacao, dataAtualizacao}]}'
```

**Medido:** `totalItens: 3` (contra 22.590 sem filtro) ⇒ o filtro **funciona** e é o caminho barato.
Os três itens:

| ordem devolvida | `dataDocumento` | `dataCriacao` | `dataAtualizacao` |
|---|---|---|---|
| ACÓRDÃO 510/24 | 17/07 **14:18:52** | 17/07 09:59:36 | 17/07 18:06:51 |
| ACÓRDÃO 657/25 | 17/07 **13:10:54** | 17/07 11:01:29 | 17/07 18:08:00 |
| ACÓRDÃO 322/24 | 15/07 **14:18:52** | 15/07 11:22:58 | 15/07 18:07:52 |

**Deduzido — a ordenação é decrescente por `dataDocumento`.** As três hipóteses se separam na ordem
em que os itens saíram: por `dataDocumento` é estritamente decrescente (14:18:52 > 13:10:54 > dia
anterior); por `dataCriacao` seria **crescente** (09:59 → 11:01 → 11:22); por `dataAtualizacao` não é
monotônico (18:06 → 18:08 → 18:07). Sobra uma hipótese.

**Não estabelecido neste teste:** qual campo o **filtro** usa — os três caem no mesmo dia em todos os
itens devolvidos, então o teste não separa. Resolvido no teste 2.

### 1.6 Teste 2 — campo do filtro, cadência de publicação e o carimbo de lote

```bash
curl -s "$B?TamanhoPagina=500&NumeroPagina=1&grupos=14543&DataInicio=2026-01-01T00:00:00.000Z" \
| jq '{total: .totalItens,
       por_mes: (.resultado | group_by(.dataDocumento[0:7])
                 | map({mes: .[0].dataDocumento[0:7], n: length})),
       divergentes: [.resultado[]
         | {titulo, doc: .dataDocumento, cri: .dataCriacao, atu: .dataAtualizacao}
         | select(.doc[0:10] != .cri[0:10] or .doc[0:10] != .atu[0:10])][0:15]}'
```

**Medido — `total: 364`** e a distribuição por mês do `dataDocumento`:

| mês (2026) | documentos |
|---|---|
| 01 | 30 |
| 02 | **0** |
| 03 | 2 |
| 04 | 20 |
| 05 | **309** |
| 06 | **0** |
| 07 | 3 |
| 08 (até o dia 10) | **0** |

(Soma 364 = `totalItens` ⇒ `TamanhoPagina=500` cobriu a janela inteira em uma página.)

**Deduzido — o filtro é por `dataDocumento`.** Dois argumentos independentes: (a) todos os 364 itens
caem em meses de 2026 **pelo `dataDocumento`**; se o filtro fosse por `dataAtualizacao`, republicações
apareceriam na distribuição com `dataDocumento` antigo (2015, 2019…) — nenhuma apareceu; (b) o
carimbo de atualização em lote das 18:06–18:08 se repete em maio **e** em julho, indicando processo
recorrente que toca registros de várias épocas; se ele fosse o campo do filtro, o total seria muito
maior que 364.

**Medido — `dataDocumento` é carimbo de lote, `dataCriacao` é por registro.** Os 15 divergentes
devolvidos (corte do `jq`; podem existir mais entre os 309) — acórdãos 080/26 e 096/26 a 109/26 —
têm `dataDocumento` **idêntico ao segundo**, `2026-05-23T11:49:44`, enquanto o `dataCriacao` avança
de 1 a 2 minutos por documento ao longo da tarde de 22/05 (15:08 → 15:31). Leitura: inserção manual
um a um na véspera; publicação do lote inteiro por processo no dia seguinte. O `dataAtualizacao`
desses mesmos itens concentra-se em 18:06–18:07 de 22/05 — outro processo em lote.

**Consequência técnica — empates massivos na chave de ordenação.** Ao menos 15 (provavelmente os 309
de maio) compartilham o mesmo instante de `dataDocumento`, que é simultaneamente o campo de
ordenação e o de filtro. Paginar lista ordenada por chave não única é **instável**: o desempate fica
a critério do banco e pode variar entre requisições, fazendo item pular de página (perda silenciosa)
ou repetir. O censo completo atravessa 46 páginas confiando nessa ordem. Isto promove o invariante
`únicos coletados == totalItens` de boa prática a **defesa necessária** — ele já está implementado na
frota.

**A premissa da "produção diária" não se sustenta no TARF.** 364 documentos em 2026, dos quais 309
num único lote (23/05); fevereiro, junho e agosto com zero. O tribunal julga continuamente, mas o
**portal publica em rajadas esparsas**. Corolário para o desenho: uma janela de restart precisaria ser
generosa (90 dias) para não ficar defasada — e uma janela generosa em fonte que despeja 309 de uma
vez não economiza quase nada.

### 1.7 Pendências do TARF (não estabelecidas)

1. **Republicação altera `dataDocumento`?** Se sim, os 68 republicados entram na janela naturalmente.
   Se não, **só o censo completo os encontra** — e a janela nunca os veria. Teste: pegar um dos 68
   pelo `NumeroDoc` e comparar suas três datas com a data da republicação conhecida.
2. **`DataFim` é inclusivo?** Relevante só se adotarmos janelas fechadas.
3. **Limite do boundary de `DataInicio`** (`>=` ou `>`) — idem.
4. O `dataDocumento` aparece **entre** `dataCriacao` e `dataAtualizacao` nos três itens do teste 1, e
   dois deles compartilham o horário `14:18:52`: mais um indício de campo carimbado por processo, não
   por evento. Sem consequência prática identificada.

### 1.8 Republicação no TARF — a mecânica real e a lacuna do `existe⇒pula`

**Republicação NÃO é edição de registro: é um segundo registro.** O `dedup_por_slug` documenta que 60
números aparecem em **2+ GUIDs** (num caso, byte a byte idênticos). O portal não altera o documento A;
publica um documento B com o mesmo número. Corolário: nenhum campo de data do registro A muda, e
**nenhum campo novo no frontmatter detectaria a revisão** — a hipótese que motivava este teste cai.

**A lacuna, encadeando o código.** (1) `dedup_por_slug` elege o vencedor por `dataDocumento` mais
recente e registra o perdedor em `tarf-anomalias.txt`; (2) o filtro de inéditos é
`!dir.join("{slug}.md").exists()` — **olha só o slug, ignora de qual GUID o `.md` veio**; (3) logo,
quando B aparece numa rodada posterior, o dedup elege B corretamente e o `existe⇒pula` **pula**: o
corpo da revisão nunca entra na árvore e o `link` do frontmatter segue apontando para a versão
superada.

**Medição do defeito hoje (10/08/2026, local, sem rede):**

```bash
awk -F'\t' '$1=="SLUG-DUPLICADO"{print $3}' data/rs/raw/tarf-anomalias.txt | sort -u > /tmp/perdedores.txt
wc -l < /tmp/perdedores.txt
grep -rl -F -f /tmp/perdedores.txt data/rs/docs/tarf/ | wc -l
```

**Resultado: 68 perdedores registrados, 0 arquivos `.md` apontando para GUID superado.** A árvore
guarda o **vencedor** em todos os 68 casos. Explicação: a carga inicial foi uma passada única, em que
o dedup rodou **antes** de qualquer `.md` existir — o vencedor foi escrito de saída. Portanto o defeito
é **latente, não materializado**: ele só morde numa republicação que apareça *depois* de a árvore já
ter aquele slug.

Ressalva do que a medição cobre: ela compara a árvore com o conjunto de perdedores **do estado atual
do portal**. Uma versão que o portal tenha removido não aparece em lugar nenhum e não seria detectada.

**Correção proposta (sem campo novo, porque o GUID já está na árvore):** o frontmatter grava
`link = DEEP_LINK_BASE + id`, então a regra passa de `existe ⇒ pula` para
**`existe E mesmo GUID ⇒ pula`**; GUID diferente ⇒ rebusca corpo e reescreve o `.md`. O `key_hash` faz
o resto: corpo novo na árvore ⇒ chave muda ⇒ re-embed só daquele documento. Custo: ler a linha `link:`
dos `.md` existentes (~22,5 k leituras locais, ~1 s) em vez de um `exists()`.

**Risco novo, visível só depois de §1.6.** O desempate do `dedup_por_slug` é
`(dataDocumento, GUID)` — e §1.6 mostrou que `dataDocumento` é **carimbo de lote com empates
massivos** (309 itens no mesmo instante). Se um acórdão e sua revisão caírem **no mesmo lote**, os
`dataDocumento` empatam e o desempate cai no **GUID, que é arbitrário** — o scraper pode eleger a
versão antiga. Medição pendente: para os 68 pares, vencedor e perdedor têm `dataDocumento` distintos?

```bash
awk -F'\t' '$1=="SLUG-DUPLICADO"{print $2}' data/rs/raw/tarf-anomalias.txt | head -5
# para cada número (achatado: 510/24 -> 51024):
curl -sA "$UA" "$B?TamanhoPagina=10&NumeroPagina=1&grupos=14543&NumeroDoc=51024" \
  | jq '[.resultado[] | {titulo, id, dataDocumento, dataCriacao, dataAtualizacao}]'
```

**Executado em 10/08/2026 com `NumeroDoc=51024`:** devolveu **1 item** (o 510/24, GUID
`ee87f4bc-…`). Dois aprendizados: (a) **`NumeroDoc` funciona** — filtrou 1 de 22.590 pelo número
achatado, e fica registrado como parâmetro utilizável; (b) o teste **não respondeu à pergunta**,
porque 510/24 era o exemplo do discovery e **não está entre os duplicados** — daí o único resultado.

Método melhor, que dispensa achatar número (e o risco de zero à esquerda: `061/25` → `6125`?
`06125`?): pareia cada **perdedor** do arquivo de anomalias com o **vencedor** que já está na árvore
(§1.8 provou que a árvore guarda os vencedores) e pede os metadados dos dois GUIDs por
`/api/Documentos/{id}`:

```bash
awk -F'\t' '$1=="SLUG-DUPLICADO"{print $2"\t"$3}' data/rs/raw/tarf-anomalias.txt | head -5 |
while IFS=$'\t' read -r numero perdedor; do
  arq=$(grep -rl "^numero: $numero$" data/rs/docs/tarf/ | head -1)
  vencedor=$(grep -m1 '^link:' "$arq" | grep -oE '[0-9a-f-]{36}')
  for g in "$vencedor" "$perdedor"; do
    curl -sA "$UA" "https://tarf.sefaz.rs.gov.br/api/Documentos/$g" \
      | jq -r --arg n "$numero" --arg g "$g" \
          '[$n, $g, .dataDocumento, .dataCriacao, .dataAtualizacao] | @tsv'
  done
  sleep 1
done
```

**Executado nos 5 pares mais recentes (10/08/2026) — sem empate:**

| número | vencedor `dataDocumento` | perdedor `dataDocumento` |
|---|---|---|
| 418/25, 407/25, 399/25, 398/25, 397/25 | **2026-01-06T12:53:47** (todos) | **2025-12-18T13:17:37** (todos) |

**Medido:** os cinco vencedores compartilham um carimbo de lote, os cinco perdedores compartilham
outro, e os dois lotes distam ~3 semanas. Os `dataCriacao` avançam de 1 a 2 minutos dentro de cada
lote (vencedores 10:21→10:27 de 06/01; perdedores 15:01→15:09 de 18/12) — inserção manual documento a
documento, como em §1.6.

**Hipótese levantada aqui e REFUTADA em §1.10:** "a republicação acontece por lote inteiro, então a
revisão sempre vem num lote posterior e o `dataDocumento` nunca empata". O evento 18/12/2025 →
06/01/2026 é real, mas responde por apenas 11 dos 68 casos — a distribuição completa (§1.10) mostra que
a maioria é republicação **avulsa**, e que o empate ocorre.

**Ressalva de amostra — n = 1 evento, não 5 casos.** O `head -5` pega os duplicados mais recentes, e
os cinco pertencem ao **mesmo** par de lotes. Não é uma amostra de cinco eventos independentes.

**GUID sequencial confirmado (SQL Server).** No par 418/25 o **último** bloco cresce no vencedor
(`…08de4d245ef4` > `…08de3e5bfe62`) enquanto o **primeiro** diminui (`e3d…` > `d19…`, mas nos outros
pares a relação se inverte). Ou seja, a parte crescente do GUID está no fim, e a comparação de string
— que pesa o primeiro bloco antes — é **arbitrária em relação ao tempo**. O desempate por GUID
continua sendo um sorteio; ele só não é acionado porque `dataDocumento` difere.

### 1.9 Varredura dos 68 pares — **o empate EXISTE** (2 casos)

Varredura completa (vencedor da árvore × perdedor das anomalias, `dataDocumento` dos dois pelo
`/api/Documentos/{id}`), 10/08/2026:

| classe | casos | leitura |
|---|---|---|
| **OK** (vencedor em lote posterior) | **64** | hipótese "revisão vem em lote posterior" vale na maioria |
| **EMPATE** (mesmo `dataDocumento`) | **2** | o desempate caiu no **GUID**, que §1.8 mostrou ser sorteio |
| **SEM-ARQUIVO** (`titulo` sem `.md` na árvore) | **2** | investigar — hipótese: o vencedor foi recusado por outra anomalia (sem-ementa / falha-corpo), então nenhum `.md` existe |
| total | 68 | |

**Conclusão: o risco NÃO é teórico.** Em 2 dos 66 pares avaliáveis, acórdão e revisão foram publicados
**no mesmo lote**, os `dataDocumento` empataram e a escolha entre as duas versões foi decidida por
comparação de string de GUID — que, sendo GUID sequencial do SQL Server (parte crescente no fim), é
arbitrária em relação ao tempo. **Nesses 2 casos o scraper pode ter elegido a versão antiga.**

**Correção do critério.** O desempate passa a ser `(dataDocumento, dataCriacao, GUID)` — `dataCriacao`
é por registro, com granularidade de minutos (§1.6), então quebra o empate de lote. Mas a escolha
segue sendo **heurística**, não fato: "inserido depois no mesmo lote" é um proxy plausível para "é a
revisão", não uma garantia. Portanto, na doutrina já praticada pelo repo (a decisão sobre anomalia é
humana), o empate de `dataDocumento` deve **também** ser registrado em `tarf-anomalias.txt` como classe
própria, para revisão — em vez de resolvido em silêncio.

### 1.10 Mapa dos eventos de republicação — e a refutação do "lote inteiro"

Agrupando os 68 pares por (lote do vencedor, lote do perdedor): **51 pares de lotes distintos**.

| tamanho do evento | quantos eventos | leitura |
|---|---|---|
| 11 casos | 1 (06/01/2026 ← 18/12/2025) | o evento que a amostra de §1.8 pegou |
| 4 casos | 1 (29/06/2022) | mesmo dia, horas diferentes |
| 2 casos | 5 | |
| **1 caso** | **~43** | **a maioria: republicação avulsa** |

**Refutado:** republicação não é "lote inteiro refeito". O evento grande de dezembro→janeiro é a
exceção; o padrão dominante é **um documento por vez**. Isso explica por que o empate aparece: sem a
regularidade do lote, nada garante que a revisão caia num carimbo posterior.

**Os 2 empates são antigos:** `ACÓRDÃO 613/22` (ambos `2023-04-19T16:53:46`) e `ACÓRDÃO 697/20` (ambos
`2022-07-03T11:53:26`). Ou seja, **o defeito já se materializou no acervo** — nesses dois casos a
versão que está na árvore foi escolhida por sorteio de GUID, e pode ser a antiga. É a exceção ao "zero
`.md` com GUID superado" de §1.8, que mediu só se o `.md` aponta para um perdedor **registrado**: no
empate, quem virou "perdedor" pode ter sido a versão certa.

**Cadeias de republicação existem.** O mapa traz linhas em que um mesmo carimbo aparece como lote do
vencedor numa e do perdedor em outra (ex.: `2022-12-13T11:57:14`). Batendo com o comentário do código
(60 números em 2+ GUIDs) e os 68 perdedores: **~8 números têm 3+ versões**. O `dedup_por_slug` já
trata isso por reduzir ao melhor, mas a regra `existe E mesmo GUID ⇒ pula` também cobre a cadeia sem
mudança.

**Granularidade do `dataDocumento` varia com a época.** Registros antigos vêm com hora zerada
(`2018-08-08T00:00:00`, `2010-05-20T00:00:00`) e alguns com horas redondas
(`2024-01-31T03:00:00`, `06:00:00`, `09:00:00`) — cara de data-only e de deslocamento de fuso
(meia-noite local gravada em UTC). Não afeta o desempate, mas **aumenta a chance de empate** entre
documentos antigos, cujo carimbo tem resolução de dia.

### 1.11 Anatomia dos 2 empates — `dataCriacao` desempata, mas não diz quem está certo

| número | papel | GUID | `dataCriacao` | `dataAtualizacao` |
|---|---|---|---|---|
| 613/22 | **na árvore** | `b76aa1c4-…-1a2b-08db40f6c22f` | 19/04/2023 **14:42:25** | 27/04/2023 16:08:44 |
| 613/22 | outro | `88e73bd3-…-1a33-08db40f6c22f` | 19/04/2023 **14:56:23** | 27/04/2023 16:08:40 |
| 697/20 | **na árvore** | `6fe07b59-…-00d2-08da5db4de16` | 04/07/2022 **09:28:09** | 15/12/2022 12:07:08 |
| 697/20 | outro | `08c20572-…-00cc-08da5db4de16` | 04/07/2022 **09:01:02** | **09/06/2025** 12:59:10 |

**Medido — `dataCriacao` desempata os dois pares** (14:42 × 14:56; 09:28 × 09:01). O critério
`(dataDocumento, dataCriacao, GUID)` é mecanicamente viável.

**Medido — e os dois casos apontam para lados OPOSTOS.** Em 613/22 a árvore guarda o **criado antes**;
em 697/20, o **criado depois**. Ou seja, adotar `dataCriacao` **mudaria** a escolha de 613/22 e
manteria a de 697/20.

**Confirmação de que a comparação de GUID é sorteio.** A ordenação de string pesa o primeiro bloco:
`b76…` > `88e…` (venceu o criado antes) e `6fe…` > `08c…` (venceu o criado depois). Uma para cada lado
— exatamente o esperado de uma moeda. Note também que, dentro de cada par, o **último** bloco do GUID
é idêntico e o **quarto** difere, e é o quarto que acompanha o `dataCriacao` (`1a33` > `1a2b`,
`00d2` > `00cc`). A parte sequencial existe, mas não é a que a comparação textual olha primeiro.

**Correção importante sobre a natureza do defeito:** a comparação de GUID é **determinística** — a
mesma entrada dá sempre a mesma escolha. O problema não é instabilidade entre rodadas; é que a escolha
**não guarda relação com recência**. O defeito é erro silencioso, não intermitência.

**E nenhum campo de data resolve com segurança quem é a revisão.** Em 697/20, o registro que **não**
está na árvore foi atualizado em **09/06/2025** — quase três anos depois do outro (15/12/2022). Se
`dataAtualizacao` fosse o sinal, a escolha seria a inversa da que `dataCriacao` indica. Em 613/22 os
`dataAtualizacao` distam 4 segundos (carimbo de lote, sem significado). Sinais contraditórios.

**Conclusão operacional:** adotar `(dataDocumento, dataCriacao, GUID)` como critério — ele elimina a
arbitrariedade e é defensável — **e registrar o empate como anomalia de classe própria**, porque a
pergunta "qual das duas versões é a vigente" não é decidível pelos metadados. São 2 casos em 22.476
documentos; a decisão certa é humana, por comparação de conteúdo:

```bash
# corpo das duas versões de um par, para comparar
for g in b76aa1c4-9327-4e24-1a2b-08db40f6c22f 88e73bd3-7f6e-4e63-1a33-08db40f6c22f; do
  curl -sA "$UA" "https://tarf.sefaz.rs.gov.br/api/documentos/$g/DocumentoHtml?NumeroPagina=1&TamanhoPagina=4000" \
    | jq -r '.resultado[].texto' | sed -E 's/<[^>]+>//g' > "/tmp/$g.txt"; sleep 1
done
diff /tmp/b76aa1c4-*.txt /tmp/88e73bd3-*.txt | head -40
```

**Resultado do 613/22 (10/08/2026): as duas versões são IDÊNTICAS** — 5.039 palavras cada, `diff`
vazio. Confirma o que o comentário do `dedup_por_slug` já registrava ("num caso, byte a byte
idênticas") e classifica este empate como **duplicata pura** (inserção repetida pelo operador), não
revisão. Consequência: aqui a escolha entre os dois GUIDs **não tem efeito no conteúdo servido** — o
sorteio de GUID acertou por não haver nada para errar. Falta medir o par 697/20.

Nuance que isso introduz no desenho: nem todo slug duplicado é revisão. Há **duplicata pura**
(conteúdo igual ⇒ empate inócuo) e **revisão** (conteúdo diferente ⇒ a escolha importa). O registro em
anomalia deveria distinguir os dois, e a distinção é barata: comparar o conteúdo das duas versões, que
o scraper já busca de qualquer modo.

### 1.12 O 697/20 não é republicação: é **título errado no portal** — e um acórdão distinto foi descartado

Comparação de conteúdo das duas versões (10/08/2026): **7.685 palavras contra 3.151**, e o `diff`
mostra que são **documentos diferentes**:

| campo no corpo | versão na árvore | versão descartada |
|---|---|---|
| ACÓRDÃO Nº (no corpo) | **697/20** | **687/20** |
| RECURSO Nº | 1193/19 | 1437/19 |
| RECORRENTE | S J JOALHERIA E ÓTICA EIRELI | SANREMO S/A |
| PROCESSO Nº | 19/1404-0006523-0 | 17/1404-0025090-8 |

**O registro descartado é o ACÓRDÃO 687/20**, cujo campo `titulo` na listagem do portal foi digitado
como "ACÓRDÃO 697/20". Recorrentes, recursos e processos distintos: não há relação entre os dois
documentos.

**Terceira causa de slug duplicado, além das duas já conhecidas:**

1. **duplicata pura** — mesmo documento inserido duas vezes (613/22, conteúdo idêntico);
2. **revisão/republicação** — mesmo documento, corpo novo, lote posterior (o caso dominante);
3. **título errado no portal** — **documentos distintos** colidindo no slug por erro de digitação.

**Correção do que este registro afirmava antes.** Eu havia escrito que nos 2 empates "o defeito já se
materializou, a versão na árvore pode ser a antiga". Está errado nos dois: em 613/22 as versões são
idênticas (nada a errar) e em 697/20 a árvore guarda o documento **certo** — o que se perdeu foi **outro
acórdão**, o 687/20, descartado em silêncio como se fosse revisão.

**Por que o empate é justamente o sinal disto.** Revisão vem em lote **posterior** (§1.9: 64 de 66).
Empate de `dataDocumento` significa **mesmo lote** — e dois registros do mesmo lote são muito mais
provavelmente **irmãos** (acórdãos sequenciais: 687 e 697 do mesmo ano) ou duplicata operacional do que
duas versões do mesmo documento. Ou seja: **empate ⇒ suspeita de erro de metadado, não de revisão.**
Isso vale para os dois empates medidos.

**Consequência para o desenho — o desempate por `dataCriacao` é o remédio errado aqui.** Ele escolheria
"melhor" entre dois documentos que **não deveriam competir**. O que resolve é uma **verificação de
consistência**: o corpo do acórdão traz `ACÓRDÃO Nº:` no cabeçalho (o `extrair` já percorre essas
linhas rotuladas). Comparar esse número com o `titulo` da listagem detecta exatamente este caso, e
o documento vai para anomalia com a divergência explícita — em vez de ser descartado como perdedor.
Mais forte ainda: **o número do corpo é a identidade mais confiável que o título da listagem**.

**RESOLVIDO (10/08/2026): não houve perda.** O `acordao-687-20.md` **está na árvore**, e o portal tem
uma entrada correta `ACÓRDÃO 687/20` (GUID `28107344-…-00ef-08da5db4de16`,
`dataDocumento 2022-07-04T11:53:26`). Logo o portal tem **três** registros envolvidos:

| registro | GUID | título na listagem | número no corpo |
|---|---|---|---|
| 697/20 correto (na árvore) | `6fe07b59-…-00d2-…` | ACÓRDÃO 697/20 | 697/20 |
| duplicata com título errado (descartada) | `08c20572-…-00cc-…` | ACÓRDÃO 697/20 | **687/20** |
| 687/20 correto (na árvore) | `28107344-…-00ef-…` | ACÓRDÃO 687/20 | 687/20 |

Reclassificação: o registro descartado é **duplicata do 687/20 com título digitado errado**. Como o
687/20 correto já está na árvore, descartá-lo foi o **resultado certo** — obtido por sorte (a ordenação
de GUID), não por critério. **Nenhum documento se perdeu; o acervo está íntegro nos 68 casos.**

**Refinamento importante sobre a granularidade do `dataDocumento`.** Os três GUIDs compartilham o
último bloco (`08da5db4de16`) — mesmo lote de inserção — e os carimbos são
`2022-07-03T11:53:26` (os dois primeiros) e `2022-07-04T11:53:26` (o terceiro): **mesma hora, dia
seguinte**. O mesmo `11:53:26` reaparece em outras linhas do mapa de eventos. Ou seja, o
`dataDocumento` se comporta como **data + hora fixa do processo**: a resolução efetiva é o **dia**.
Isso torna o empate estruturalmente mais provável do que eu supunha e faz de "lote posterior" uma
comparação de granularidade diária.

**O que sobra para o desenho, depois de tudo isto:**

1. **A correção necessária continua sendo `existe E mesmo GUID ⇒ pula`** — ela atende os 64 casos de
   republicação genuína, que são o problema real daqui para frente.
2. **A verificação `número do corpo == título da listagem` vale como anomalia**, não como critério de
   desempate. Ela não conserta nada no acervo (que está íntegro), mas teria revelado a natureza do
   697/20 numa linha de log — em vez de uma hora de `curl` investigativo.
3. **O desempate por `dataCriacao` deixa de ser prioridade.** Nos dois únicos empates observados, o
   resultado atual está correto; e o empate se mostrou sinal de **duplicata/erro de metadado**, não de
   revisão. Trocar o critério resolveria um problema que não se manifestou, e mascararia o sinal.

**Consulta usada para fechar:**

```bash
ls data/rs/docs/tarf/ | grep -E '687-20'
grep -n '687/20' data/rs/raw/tarf-anomalias.txt
curl -sA "$UA" "https://tarf.sefaz.rs.gov.br/api/PesquisaDocumentos?TamanhoPagina=10&NumeroPagina=1&grupos=14543&NumeroDoc=68720" \
  | jq '[.resultado[]|{titulo,id,dataDocumento}]'
```

Se o portal não tiver nenhuma entrada intitulada "ACÓRDÃO 687/20", então **o acórdão 687/20 não existe
na árvore nem no acervo do Auli** — perda silenciosa de um documento inteiro, causada por um erro de
digitação no portal somado a um dedup que confia no título.

**Passo de medição anterior** — os 2 empates e os 2 sem-arquivo:

```bash
ls data/rs/docs/tarf/ | grep -E '162-21|003-21'          # os SEM-ARQUIVO existem com outro título?
grep -E '^EMPATE' /tmp/pares.tsv                          # os 2 pares a inspecionar
```

Nos SEM-ARQUIVO, atenção ao segundo: o título registrado é **`Acordão 003/21`** — grafia diferente
(sem acento no "o", "ã" no lugar de "a"). Ela colapsa no mesmo slug de `ACÓRDÃO 003/21`, o que
**explica a duplicidade** e também por que a busca por `^titulo: <perdedor>$` não achou arquivo: o
vencedor tem o título na outra grafia. Logo a hipótese "vencedor recusado por outra anomalia" é
provavelmente **falsa** — é variação de grafia no portal, não ausência.

---

## 2. SP — Respostas de Consulta (SharePoint, `legislacao.fazenda.sp.gov.br`)

### 2.1 Tabela consultada

Biblioteca **Páginas**, lista SharePoint `{F286D0D1-5624-47DA-A856-A8571296EB7F}` (~27 k itens, dos
quais ~15–20 k são RCs; o resto é página de sistema). Colunas usadas: `FileLeafRef`
(`RC{numero}_{ano}.aspx` — chave natural), `Ementa`, `PublishingPageContent` (corpo integral).

### 2.2 Paginação atual e a âncora candidata

O filtro por `FileLeafRef` estoura o *list view threshold* (campo não indexado, lista > 5000 itens);
por isso o scraper hoje pagina por `$orderby=Id` + `$skiptoken` (via `odata.nextLink`), 200 por
página, ≈ **136 páginas**, e filtra as RCs no cliente.

Âncora candidata: **`Id`** — auto-incremento e **campo indexado** da lista, o que em princípio
permite `$filter=Id gt {marca}` sem estourar o threshold (é justamente a exceção que o SharePoint
concede a campos indexados).

### 2.3 Teste executado (10/08/2026) — `$filter=Id gt N` funciona

```bash
L='https://legislacao.fazenda.sp.gov.br/_api/web/lists(guid%27F286D0D1-5624-47DA-A856-A8571296EB7F%27)/items'
curl -s -H 'Accept: application/json;odata=nometadata' \
  "$L?\$select=Id,FileLeafRef&\$filter=Id%20gt%2020000&\$orderby=Id&\$top=5" | jq .
```

**Medido — a API anônima aceita o filtro.** Nenhum erro de *list view threshold*, nenhuma restrição
de operador: veio `value[]` com 5 itens e o `odata.nextLink` **preservando `$filter`, `$orderby` e
`$select`**, com `$skiptoken=Paged=TRUE&p_ID=1357993`. Confirma que filtro + ordenação + `$skiptoken`
combinam — o `Id`, sendo campo indexado, escapa do threshold como esperado.

| `Id` | `FileLeafRef` |
|---|---|
| 1.245.568 | `Atos.aspx` |
| 1.245.633 | `AtosPopulares.aspx` |
| 1.245.670 | `Search.aspx` |
| 1.335.274 | `AtosTodos.aspx` |
| 1.357.993 | `AjudaSearch.aspx` |

**Medido — o espaço de `Id` é alto e esparso.** Numa lista de ~27 k itens, os `Id` estão na faixa de
**1,24–1,36 milhão**, com saltos de dezenas de milhares entre itens consecutivos (1.245.670 →
1.335.274 → 1.357.993).

**Deduzido — o piso da lista é ≈ 1.245.568.** Como a ordenação é `Id` ascendente e o filtro era
`Id gt 20000`, os cinco primeiros itens acima de 20.000 são estes: **não existe item algum com `Id`
entre 20.001 e 1.245.567**. Ou seja, o corte escolhido no teste ficou **abaixo do piso** e não
exercitou um ponto de corte real — o teste provou que o operador funciona, não que ele corta no meio
do acervo.

**Deduzido — `Id` não é contador denso local.** Faixa alta + lacunas grandes indicam sequência
compartilhada ou herdada de migração, não um `1..27k` por lista. Sem consequência prática se a marca
for sempre calibrada por medição (`$orderby=Id desc&$top=1`), nunca por constante no código.

**Deduzido — a marca por `Id` seria cega a EDIÇÕES.** O `Id` do SharePoint é atribuído na **criação**
e nunca muda; logo um high-water mark é seguro para **itens novos** (todo novo `Id` é maior que
qualquer existente), mas **não vê alteração de RC já publicada**. É exatamente a mesma lacuna da
republicação no TARF (§1.7.1) — e o mesmo remédio: censo.

**Aberto ao fim de §2.3, resolvido em §2.4:** (a) se a ordem de `Id` acompanha a cronologia de
publicação das RCs, ou se as RCs entraram num bloco único de migração; (b) o teto do `Id` e onde as
RCs se situam no espaço — os 5 itens devolvidos aqui são **todos páginas de sistema**
(`Atos.aspx`, `Search.aspx`…), nenhuma RC. Teste usado:

```bash
curl -s -H 'Accept: application/json;odata=nometadata' \
  "$L?\$select=Id,FileLeafRef&\$orderby=Id%20desc&\$top=20" | jq '.value'
```

### 2.4 Teste do topo do espaço de `Id` (10/08/2026) — o `Id` é orgânico

```bash
curl -s -H 'Accept: application/json;odata=nometadata' \
  "$L?\$select=Id,FileLeafRef&\$orderby=Id%20desc&\$top=20" | jq '.value'
```

**Medido — teto ≈ `1.529.586`**, e os 20 primeiros ocupam um bloco **estritamente contíguo**
(1.529.567 … 1.529.586, sem lacuna). Conteúdo do bloco: 14 RCs (`RC34027_2026.aspx`,
`RC33974_2026.aspx`, …, e uma `RC32896_2025.aspx`) e 6 `Comunicado-DICAR-NN-de-2026.aspx`.

**Deduzido — o `Id` é atribuído na criação e o topo é o material recente.** RCs de 2026 no topo, e as
páginas de sistema (`Atos.aspx`, `Search.aspx`) no piso do espaço (§2.3) — exatamente a ordem em que
um site é construído. Portanto **a ordem de `Id` acompanha a cronologia de publicação**, e a hipótese
de bloco único de migração cai.

**Medido — a numeração da RC NÃO acompanha o `Id`.** Dentro do bloco contíguo, os números saem
embaralhados: 34027, 33974, 34079, 33909, 33842, 33275, 33979, 33732, 33708, 33334. E há uma
**RC de 2025 publicada no meio do lote de 2026** (`RC32896_2025`, `Id` 1.529.570). É o mesmo fenômeno
do TARF (§1.4): documento antigo carregado atrasado ⇒ **nem o número nem o ano do arquivo servem de
âncora**; só o carimbo do portal serve.

**Deduzido — publicação em lote também no SP.** Um bloco de 20 `Id` consecutivos misturando RCs e
Comunicados indica inserção em rajada, como o lote de 309 do TARF (§1.6).

**Medido — a biblioteca é heterogênea:** os Comunicados DICAR convivem com as RCs na mesma lista, o
que confirma a necessidade do filtro por padrão do `FileLeafRef` no cliente.

**Densidade do espaço:** piso ≈ 1.245.568, teto ≈ 1.529.586 ⇒ vão de ~284 k `Id` para ~27 k itens
(~10 %). Esparso no todo, contíguo nos lotes recentes.

**Variante habilitada (não adotada):** como todo item novo recebe `Id` maior que qualquer existente,
os inéditos formam sempre um **sufixo no topo** da ordem — dá para caminhar `Id desc` e parar ao bater
em território conhecido da árvore, **sem arquivo de estado**, no mesmo padrão auto-reparável do
`existe⇒pula`. Duas ressalvas: parar no **primeiro** conhecido é frágil (uma coleta interrompida no
meio de uma página deixa buraco que o early stop nunca reencontra) ⇒ exigiria margem de K conhecidos
consecutivos; e o early stop **abandona a verificação `únicos == total` por rodada**. Continua valendo
o §6: censo completo, que aqui custa ~2,3 min.

---

## 3. SC — COPAT (`legislacao.sef.sc.gov.br`, WebForms)

Índice `Copat.aspx` devolve **TODAS** as consultas em **uma única resposta** (~6,8 MB, acordeão por
ano; linhas `<td><a href="DocumentoLegalViewer.ashx?id=GUID">NNNN/YY</a></td><td>EMENTA</td>`).

**Custo de relistagem completa: 1 GET.** Não há âncora a projetar nem estado a guardar — o diff
contra a árvore já resolve, e o corpo (`DocumentoLegalViewer.ashx?id=GUID`) é buscado só para os
inéditos. Escopo do scraper: consultas de 2011+.

---

## 4. RS pareceres — Portal de Legislação (`legislacao.sefaz.rs.gov.br`, WebForms/IIS, windows-1252)

Listagem `Search.aspx?CodArea=3&CodGroup=159`, paginada por **postback**
(`__EVENTTARGET=LinkToPage` + `__VIEWSTATE` da resposta anterior), ~20 por página,
**~17 páginas ⇒ ~331 chaves**. Transporte por
**curl subprocess** com cookie jar (o `ureq` sempre volta à página 1 — camada de cache/WAF).

**Custo de relistagem completa: ~17 requisições, segundos.** Não compensa âncora. Detalhe em
`DocumentView.aspx?inpKey=N`, buscado só para os inéditos.

---

## 5. PR — Consultas Tributárias do Setor Consultivo (`sefanet.pr.gov.br`)

Estrutura diferente das demais: a página `_l_DownloadLegislacao2.asp?eTpDoc=16` lista **um PDF por
ano** (2007→…), cada um uma compilação anual com todas as consultas do ano (ex.: 2007 = 613 páginas,
137 consultas). Pipeline: baixar PDF → `pdftotext -layout` → remover mobília de página → dividir no
marcador `CONSULTA Nº: NN, de <data>`.

**Aqui a listagem é barata mas o DOWNLOAD+PARSE é caro** — é a única fonte em que o restart paga.
**Âncora natural: o ano corrente.** Anos anteriores são compilações fechadas ⇒ congelam no cache;
só o PDF do ano em curso cresce e precisa ser rebaixado e reprocessado. Vantagem: o "estado" é
derivado do calendário, não guardado em arquivo — não há marca para dessincronizar.

---

## 6. Síntese — a inversão da conclusão

| fonte | relistagem completa | o que é caro | âncora paga? |
|---|---|---|---|
| SC COPAT | 1 GET | corpo dos inéditos | **não** |
| RS pareceres | ~17 postbacks (~s) | detalhe dos inéditos | **não** |
| TARF | 46 req ≈ **45 s** | 22.590 corpos + embeddings | **não** |
| SP RCs | 136 req ≈ **2,3 min** | corpo (já vem na listagem) | marginal (`$filter=Id gt N` funciona, §2.3) |
| PR | 1 GET no índice | **PDFs anuais + pdftotext** | **sim** (ano corrente) |

**O que a medição mostrou:** o custo alto nunca esteve em **relistar** — está em **buscar corpo** e em
**gerar embeddings**. E esses dois já estão resolvidos por outros mecanismos: o cache agressivo +
`existe⇒pula` cuidam do corpo; o `key_hash` do update incremental cuida do embedding.

**Recomendação decorrente:** **não criar marca de restart no TARF** (nem no SP, nem em SC/RS). Censo
completo a cada rodada; a âncora é o **próprio acervo** — `existe⇒pula` já funciona como high-water
mark e é **auto-reparável**: documento carregado atrasado entra na rodada seguinte sem que ninguém
precise ter previsto a janela. Some o arquivo de estado, some a classe de bug de janela defasada, e o
invariante `únicos == totalItens` — cuja necessidade §1.6 acabou de demonstrar — continua verificado
de ponta a ponta em toda coleta. **Marca só no PR, por ano corrente.**

A premissa inicial ("não posso recomeçar da primeira página, senão rescrapeio tudo") se dissolve:
**relistar é barato; rebuscar corpo é caro — e isso o cache já impede.**
