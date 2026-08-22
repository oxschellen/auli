# ESTUDO-pos-documento — O que a struct genérica destrava (roadmap da plataforma)

**Data:** 08/08/2026 · **Status:** estudo/roadmap, sem decisão de implementação.
**Pré-requisito de tudo aqui: ✅ cumprido** — a TAREFA-DOCUMENTO fechou em 08/08/2026, Etapas A, B e
C implantadas (PRs #132/#133/#134). A fundação está de pé; os sete itens abaixo estão liberados para
serem escolhidos.
**Leitura prévia:** `docs/auli_code.md` §3.13 — a arquitetura das quatro coleções (`Documento` +
`Kind`). O detalhe do que divergiu do plano está nas mensagens de merge dos PRs #132, #133 e #134.

A `Documento` não é um fim: é uma plataforma. Este estudo registra as sete melhorias que ela
torna baratas, com valor, custo e dependências — para escolher com calma quando a fundação
estiver de pé.

---

## 1. Busca federada — "pergunte ao acervo", não "escolha a gaveta"

**O quê:** uma consulta corre TODOS os packs da entidade e mescla os hits num contexto só;
o seletor de fonte vira filtro opcional, não obrigação. O invólucro
`## documento {i}: {rotulo}` já apresenta fontes mistas sem mudança.
**Por quê:** o analista não sabe de antemão se a resposta mora num parecer, num acórdão ou
numa FAQ — hoje ele precisa adivinhar a gaveta antes de perguntar.
**O desafio honesto:** scores de coleções diferentes NÃO são comparáveis (distribuições
distintas — a ementa telegráfica do TARF pontua diferente da pergunta de FAQ). Fusão por
score é armadilha; a técnica padrão é **fusão por ranking (RRF — reciprocal rank fusion)**,
que só usa a posição de cada hit no ranking da sua coleção.
**Depende de:** Etapa B (pack uniforme). Combina com o item 3 (a fusão híbrida usa o mesmo
RRF). **Valor de usuário: o maior da lista.**

## 2. `data` no núcleo do `Documento` (o campo que falta)

**O quê:** `data: Option<...>` no núcleo + chave `data:` opcional no frontmatter.
Jurisprudência preenche (o portal fornece `dataDocumento`); serviços/FAQs podem omitir.
**Destrava:** filtros temporais no retrieve ("acórdãos desde 2020", "pareceres após a Lei
X"), recency como desempate, ordenação uniforme em qualquer superfície (a
`chave_cronologica` de hoje vive presa ao índice do frontend e é derivada do numero — com o
campo, vira dado de primeira classe).
**Custo:** mínimo — o menor da lista. Scraper do TARF já tem o dado; pareceres exigem
conferir se a data está no snapshot/portal. Frontmatter novo ⇒ passa pelo mesmo regime do
`orgao` (chave opcional).
**É o aquecimento natural pós-Documento.**

## 3. Retrieval híbrido (léxico + vetorial)

**O quê:** um componente lexical fundido com o vetorial por RRF. **Duas vias, e a segunda apareceu
depois deste estudo:** (a) índice próprio sobre a mesma árvore, com Tantivy — o candidato original;
(b) o componente **esparso do próprio BGE-M3**, que o `fastembed` já travado no `Cargo.lock` expõe
via `SparseTextEmbedding` — mesmo modelo, sem índice novo a manter. A via (b) foi medida em
[docs/ESTUDO-busca-hibrida.md](docs/ESTUDO-busca-hibrida.md), que também traz o obstáculo dela: o
`SparseModel::BGEM3` aponta para o fp32 completo, e o nosso é o int8 de 560 MB — o teste que decide o
custo é se o `UserDefinedSparseModel` aceita o arquivo que já temos.
**Por quê:** embeddings erram exatamente onde quem atende é forte — identificadores exatos:
"acórdão 510/24", "art. 5º da Lei nº 8.820/89", CFOP, números de protocolo. O léxico acerta
esses de graça.
**O ganho da unificação:** com `Documento`, é UMA implementação para as quatro coleções;
antes seriam quatro variantes.
**Depende de:** Etapa B. Par natural do item 1 (mesma fusão).

## 4. Política de retrieval como dado, não código

**O quê:** score bands, k, expansão por relacionados viram tabela de config por `Kind`
(no contrato ou registry), em vez de constantes espalhadas.
**Por quê:** os "botões" do protocolo A5 (ex.: apertar o piso da band do tarf) viram giro de
configuração auditável em vez de PR. É a versão retrieval do que o `registry.toml` fez com
entidades.
**Custo:** pequeno; vem de carona em qualquer mexida nos itens 1/3.

## 5. O servidor de RAGs separado (a visão antiga, agora madura)

**O quê:** extrair o `auli-retrieval` para um servidor autônomo cujo artefato de deploy =
packs v2 + árvore (a decisão já registrada na D-B3). Produto: N coleções de `Documento`,
mesma API HTTP, mesmo MCP. O Auli vira o primeiro cliente entre vários.
**Por quê agora ficou maduro:** a API só podia ser estável quando o shape servido fosse um —
a `Documento` é esse shape. RFB (pedido do auditor), súmulas, leis: entram como coleções
DESSE servidor, não do Auli.
**Custo:** o maior da lista (operação de mais um serviço, autenticação, versionamento de
API). É maturidade, não urgência.

## 6. Grafo de citações cross-coleção

**O quê:** generalizar a expansão por relacionados (hoje doutrina exclusiva dos pareceres)
para arestas `(chave_origem, kind_origem) → (chave_destino, kind_destino)` — acórdão cita
parecer, parecer cita lei.
**A fábrica das arestas já está planejada:** a TAREFA-EXTRACAO (triplas na fase de
sinopse/extração) produz exatamente isso; falta a canonização das referências para `chave`.
**Valor:** "este acórdão aplica o entendimento do Parecer X" dentro do contexto do chat é
ouro para o analista.
**Depende de:** Etapa B + extração v1 rodada e analisada.

## 7. Update incremental por documento (paga a dívida dos 35,4 GB)

**O quê:** o update deixa de carregar a coleção inteira na RAM (`docs/auli_pendencias.md` §31.1:
SP sozinho = **35,4 GB**, remedido em 08/08 — o número antigo, 24,7 GB, ficou para trás): hash por
documento, re-embed só do que mudou, escrita do pack em streaming.
**Por quê a Documento ajuda:** a unidade de trabalho uniforme (um `Documento` por vez) é o
que torna o streaming um caminho só, em vez de quatro.
**Bônus:** update do acervo completo em segundos quando nada mudou; viabiliza servidores
menores — hoje um de 16 GB não roda o SP (§31.1).

---

## Ordem recomendada (pós A+B)

1. **Item 2** (`data`) — aquecimento barato, valor imediato para jurisprudência.
2. **Itens 1+3 juntos** (federada híbrida via RRF) — o salto de valor de usuário; o item 4
   vem de carona.
3. **Itens 5, 6, 7** — a maturidade, cada um gatilhado pelo seu evento: 5 quando houver um
   segundo cliente real do acervo (o auditor via MCP já é meio caminho); 6 quando a
   extração v1 tiver sido rodada e analisada; 7 quando a RAM apertar de verdade ou o acervo
   dobrar.

As fundações (A e B) fecharam em 08/08/2026 — a lista está liberada. A régua de sempre vale para
cada item: TAREFA própria, decisões numeradas, portão humano, medição antes de promover.
