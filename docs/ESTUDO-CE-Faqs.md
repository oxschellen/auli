# ESTUDO-CE-Faqs — Perguntas Frequentes da SEFAZ-CE (coleção `faqs` da entidade `ce`)

**Data:** 2026-11-03 · **Estado:** **implementado** (§8; o que mudou em relação a este estudo está lá) — API, raiz,
método, shape e censo confirmados sobre os 96 artigos reais (coletados na sua máquina, analisados aqui). · **Órgão:** SEFAZ-CE. · **Hoje a `ce` tem só `servicos`**
(382 serviços via Sydle ONE `getChildren`, D-CE1..3).

> **TL;DR — a maior FAQ da frota, servida pela mesma API JSON que o `servicos` já usa.** O Portal de Serviços
> (Sydle ONE) tem, ao lado do catálogo `servico-geral`, o catálogo **`conteudo-geral`** ("Conteúdo - Geral",
> `_id 648af6fc64778b7336bf45d3`) com **428 conteúdos** (`hits` diz 438 — inflado, como no `servicos`). Entre eles,
> **96 artigos de FAQ** ("Principais Dúvidas sobre …", "Dúvidas sobre …", "Perguntas e Respostas Frequentes") com
> **3.177 pares P/R → 2.702 únicos** (475 cópias exatas entre artigos irmãos). Receita: **token anônimo do shell**
> (já extraído pelo `ce.rs`) → `POST …/getChildren` na raiz `conteudo-geral` (44 páginas de 10) → filtrar os artigos
> de FAQ → **`POST …/getItem` `{application, id}`** por artigo (método **autorizado ao anônimo**; `get`, `getContent`,
> `getArticle`… dão 403) → campo **`contentMarkdown`**: blocos `<div class="section-wrapper">` com a pergunta num
> `<h2>` (ou `## ` markdown, nos 7 artigos de 2024) e a resposta dentro de `<sy-collapse … trigger-by="#section-<slug>">`.
> Parser validado nos 93 artigos MARKDOWN: **3.177 seções = 3.177 pares, 0 pergunta vazia, 0 resposta vazia**. Os 3
> artigos em `contentFormat: HTML` não têm seções (lista de bancos, link para o gov.br, "-") → pular com log.
> `url` = `https://portalservicos.sefaz.ce.gov.br/<identifier>+<_id>` (deep-link por artigo; o `#section-<slug>` da
> pergunta existe no DOM). **Fora:** `www.sefaz.ce.gov.br/perguntas-frequentes/` (migrou para `www.ce.gov.br/sefaz/…`,
> que não responde ao `curl`), FAQ do **DAE** e da **NFA** (servidores legados: handshake TLS não fecha nem no seu
> Ubuntu), PDFs "Perguntas Frequentes 7.x" (FAQ federal da EFD espelhado), wiki do Siafe.

---

## 1. Hierarquia discovery-first — o que foi avaliado

| Rota | Veredito | Evidência (03/11/2026, `curl -A AuliBot`) |
|---|---|---|
| **`POST /api/1/servicedesk-embedded/_classId/5cd5e83d59f8170cd4dc2c43/getChildren`** raiz `conteudo-geral` | **listagem**: 44 páginas (`pageSize` 10; pág. 44 vazia), 428 itens distintos, classe `5d66ed07e7c98a556a499a36` ("service" de conteúdo); campos `_id, identifier, name, description, tags[], lastUpdateDate, path, parent` | sondagem r2/r3 |
| **`POST …/getItem`** `{"application":"sefaz-ceara","id":"<_id>"}` | **200, artigo inteiro** (`contentMarkdown`, `contentFormat`, `name`, `identifier`, `description`, `tags`, `lastUpdateDate`, `publishedDate`, `viewCount`) | sondagem r1 (CTe: 115.962 B) |
| `get`, `getContent`, `getPage`, `getArticle`, `getDocument`, `getById`, `getFaq` | **403** `ERROR_USER_UNAUTHORIZED_TO_EXECUTE_METHOD` | r1 |
| `getChildren` com `id`/`rootId` = `_id` do artigo | `{"hits":0,"return":[]}` (artigo é folha) | r1 |
| Casca `…/principais-duvidas-sobre-…+<_id>` | 200, 41.843 B, só meta + `_classId`s — SPA | r1 |
| `www.sefaz.ce.gov.br/perguntas-frequentes/` | 301 → `www.ce.gov.br/sefaz/perguntas-frequentes/` → `000` (sem resposta) — **fora** | r1/r3 |
| `servicos.sefaz.ce.gov.br/internet/dae/…perguntas_mais_frequentes.asp`, `www2.sefaz.ce.gov.br/nfaweb/perguntasFrequentes.do` | `000`: handshake TLS 1.2/1.3 não completa (certificado/cipher legado) — **fora**, registrar | r1/r3 |
| PDFs `wp-content/uploads/sites/61/…/Perguntas-Frequentes-7.x.pdf` | FAQ **federal** da EFD — fora | busca |
| Headless | não | — |

## 2. Receita de coleta

`API = https://portalservicos.sefaz.ce.gov.br/api/1/servicedesk-embedded/_classId/5cd5e83d59f8170cd4dc2c43`;
headers `Authorization: <Bearer do shell>`, `X-Explorer-Account-Token: sefazce`, `Content-Type/Accept: application/json`.

1. **Token:** `fetch_token()`/`parse_token()` existentes (shell → `"Authorization":"Bearer …"`).
2. **Listagem:** `POST API/getChildren` body `{application:"sefaz-ceara", id: R, rootId: R, path:"conteudo-geral",
   view:"list", params:{filters:{}, pageIndex:n, pageSize:10, sorters:{sort:{"name._current.keyword":"asc"}}}}` com
   `R = 648af6fc64778b7336bf45d3`, até a página vazia (mesma mecânica do `servicos`: `pageSize` 10 obrigatório, `hits`
   inflado é só aviso, dedup por `_id`). Hoje: 44 páginas, 428 itens.
3. **Seleção dos artigos de FAQ (D-CEFAQ-2):** `name` casa `(?i)d[úu]vidas|pergunta|faq` → 96. O resto do catálogo
   são "COMUNICADO COTRI nº …" (orientações interpretativas — candidatos a outra coleção, pendência) e conteúdos
   institucionais.
4. **Artigo:** `POST API/getItem` `{application, id}` por artigo (96 POSTs, 400 ms → ~1 min). Cache por `id`.
5. **Parse** (§3) de `return.contentMarkdown`; `return.contentFormat != "MARKDOWN"` ou 0 seções → log "artigo sem
   seções" e pular (hoje 3). `write_faqs("ce", …)` + `ce-faqs-tree.json` (raiz → artigo → perguntas).

## 3. Parser (fechado no shape medido — dois templates)

- Dividir `contentMarkdown` em blocos por `<div class="section-wrapper">` (descartar o que vem antes do 1º: botão
  "Expandir todas"; e o `<script>`/`<style>` finais).
- Em cada bloco: `slug` = `id="section-<slug>"` do `div.heading`; **pergunta** = conteúdo de `<h2>…</h2>` (template
  2026, 86 artigos) **ou**, na falta, a linha `## …` (template 2024, 7 artigos: CTe, Geração/Exclusão de Débitos,
  Localização e Liberação, Alteração EFD, Procedimentos de Escrituração, Restituição de Taxas, isenções, Representação
  fiscal); **resposta** = tudo após a tag de abertura `<sy-collapse … trigger-by="#section-<slug>" …>` até o fim do
  bloco. **Atenção:** a tag `<sy-collapse>` do template 2024 tem `ref="(r) => handleState(r)"` — um `>` dentro do
  atributo; o fim da tag é o **fim da linha**, não o primeiro `>`.
- Texto: remover tags (html5ever ou strip) + decodificar entidades (`&atilde;`… — o template 2026 vem com entidades;
  o 2024 com UTF-8 literal), colapsar espaços; `</div>` sobrando é removido pelo strip. Links: manter o `href`
  absoluto como texto quando o anchor for diferente da URL (padrão da frota). Listas markdown (`1.`, `-`) ficam.
- Guardas por artigo: nº de `id="section-"` == nº de pares (`Err`), pergunta vazia (`Err`), resposta vazia (`Err`);
  por rodada: artigos com seções ≥ **80** (hoje 93), pares únicos ≥ **2.400** (hoje 2.702), `_id` repetido (`Err`),
  `slug` repetido dentro do artigo (`Err`).
- **Dedup:** par `(pergunta, resposta)` idêntico entre artigos → manter o de menor `_id`, log dos demais (hoje 475;
  há artigos inteiros duplicados: "Dúvidas sobre a Legislação" ×2, "ICMS … combustíveis" ×2).

## 4. Modelagem → `FaqRaw`

| `FaqRaw` | origem |
|---|---|
| `pergunta` | `<h2>`/`## ` (`clean`) |
| `resposta` | texto da `<sy-collapse>` (mediana 391 chars; 49 respostas > 3.000, máx. 19 KB — tabelas de alíquota; chunking é do pipeline) |
| `origin` | `name` do artigo, íntegro (ex.: `Principais Dúvidas sobre CTe - Conhecimento de Transporte Eletrônico`) |
| `url` | `https://portalservicos.sefaz.ce.gov.br/<identifier>+<_id>` |

Identidade = `(_id, slug)`. Público (`tags`: `cidadao`/`empresa`/`transportador`) não entra no `FaqRaw` (sem campo);
fica no `tree.json` como atributo do artigo, se útil ao frontend; senão só no log.

**Censo (03/11/2026):** 96 artigos (93 com seções); 3.177 pares; 2.702 únicos; maiores: Tributação e Alíquotas de
Produtos **304**, Procedimentos de Escrituração 254, Obrigatoriedade/Dispensa de Registro 200, NFe 138, Cálculos e
valores de ICMS 109, Informação Geral e Legislação 96, Legislação 91, Alteração EFD 77; `lastUpdateDate` majoritário
2026-07-29 (re-publicação em lote), alguns 2026-10 — acervo **vivo**.

## 5. Decisões (D-CEFAQ-*)

- **D-CEFAQ-1** — Fonte única = artigos de FAQ do catálogo `conteudo-geral` via `getChildren` + `getItem`.
- **D-CEFAQ-2** — Seleção por nome (`d[úu]vidas|pergunta|faq`) — critério editorial da própria SEFAZ-CE para
  esses artigos; registrar os 96 nomes no REGISTRO e **avisar** (não `Err`) se um item com `name` fora do padrão
  tiver ≥ 3 `section-wrapper` (sinal de FAQ nova fora da convenção) — isso exige `getItem` de todos os 428 (3 min);
  alternativa barata: só os 96. **Recomendação: só os 96**, com a lista no REGISTRO e o aviso como pendência.
- **D-CEFAQ-3** — Reuso: `fetch_token`/`parse_token`/`post_json`; `build_body(page)` vira `build_body(root, path,
  page)` (o `servicos` passa `CATALOGO_ID, "servico-geral"`); `fetch_page` idem. Sem outra mudança no `servicos`.
- **D-CEFAQ-4** — `origin` = nome do artigo íntegro (fidelidade; o prefixo "Principais Dúvidas sobre" varia de grafia).
- **D-CEFAQ-5** — Dedup exato entre artigos (menor `_id` fica). Alternativa (manter) descartada: 475 cópias, com
  artigos inteiros duplicados.
- **D-CEFAQ-6** — Artigos `HTML`/sem seções → log e pular (3 hoje); se o número crescer acima de 10, `Err` (template
  mudou).
- **D-CEFAQ-7** — DAE, NFA e `ce.gov.br` fora, com o diagnóstico no REGISTRO; **pendência**: "COMUNICADO COTRI" (≈300
  conteúdos interpretativos no mesmo catálogo) como candidata a coleção própria.
- **D-CEFAQ-8** — `AuliBot`, 400 ms, rede sempre, cache (`id` do artigo / página da listagem) só com `--usecache`.
- **D-CEFAQ-9** — `faqs` + `all` no `auli-scraper-ce`; default segue `servicos`.
- **D-CEFAQ-10** — Ingerir como está (após §3).

## 6. Plano

- **F1 parsers puros + fixtures reais** — `item-64da50f7170f5a26dc5e9ea5.json` (CTe, template 2024, 46), um artigo
  do template 2026 com entidades (ex. `6504811762f08b64cdd578ba`, Diferimento, 17), um `HTML` sem seções
  (`6501e95d3c373727041947fe`), 2 páginas de `getChildren` (com e sem FAQ). Testes: contagem exata; pergunta/resposta
  de 1 par de cada template; seleção por nome; dedup; `Err` de pergunta vazia / seções ≠ pares.
  **Mutação:** (a) fechar a tag `<sy-collapse` no primeiro `>` → no CTe a resposta vira `handleState(r)" class=…` →
  teste de texto cai; (b) remover o fallback `## ` → CTe dá 46 perguntas vazias → `Err` esperado pelo teste; (c) tirar
  o `html::unescape` → fixture Diferimento com `N&atilde;o` cai; (d) dedup desligado → fixture com 2 artigos
  partilhando um par muda a contagem → cai.
- **F2 transporte** — token + 44 + 96 POSTs; `write_faqs("ce", …)` + árvore. Rodada real: 93 artigos, ~2.700 únicos;
  log do dedup e dos pulados. Portão humano: 3 FAQs conferidas no portal (uma de cada template + uma longa).
- **F3 pipeline** — `registry.toml`, `gen-frontend-entities`, `auli-collections ce`, `build-packs.sh ce faqs`, smoke
  `/v1/retrieve kind=faqs entity=ce` (NFC-e, IPVA isenção, Simples).
- **F4 registro** — `REGISTRO-scrapers.md#ce-faqs` (raiz, `getItem`, 403 dos outros métodos, dois templates, censo,
  DAE/NFA/ce.gov.br), `update-ce-faqs.sh` (guarda de encolhimento), README/SCRAPERS.md, pendências (COTRI; aviso
  D-CEFAQ-2).

## 7. Evidência (03/11/2026)

```
GET  /  → 200, 39.464 B, "Authorization":"Bearer eyJ…"            (token anônimo)
POST getItem {application:"sefaz-ceara", id:"64da50f7170f5a26dc5e9ea5"} → 200, 115.962 B
     return.parent = {identifier:"conteudo-geral", name:"Conteúdo - Geral", _id:"648af6fc64778b7336bf45d3"}
     return.contentFormat="MARKDOWN"; contentMarkdown 60.445 B; 46 × <div class="section-wrapper">
POST get|getContent|getPage|getArticle|getDocument|getById|getFaq → 403 ERROR_USER_UNAUTHORIZED_TO_EXECUTE_METHOD
POST getChildren raiz conteudo-geral, pageSize 10 → págs 1–43 com 10/10/…/8, pág 44 vazia; hits=438; 428 _id distintos
     96 com name ~ dúvidas|pergunta|faq; 96 × getItem → 3.177 seções; parser: 3.177 pares, 0 vazios; 2.702 únicos
     3 artigos contentFormat=HTML sem seções: Desvinculação de débitos ("-"), Bancos Conveniados e PIX (lista),
     Reforma Tributária – P&R pelo Ministério da Fazenda (link gov.br)
www.sefaz.ce.gov.br/perguntas-frequentes/ → 301 www.ce.gov.br/sefaz/… → 000
servicos.sefaz.ce.gov.br (DAE) e www2.sefaz.ce.gov.br (NFA) → 000 (handshake TLS não completa)
```

Fontes: `auli-scraper-ce/src/ce.rs`; pacotes `ce-faqs.tgz` (shell, getItem CTe, 40 págs) e `ce-faqs2.tgz` (44 págs,
96 `item-*.json`, `censo-ce.txt`).

## 8. Implementação (03/11/2026)

- `auli-scraper-ce/src/faqs.rs` (novo), `faqs` e `all` no `main.rs`; no `ce.rs`, `build_body`/`fetch_page` recebem
  raiz e `path` (D-CEFAQ-3) e `fetch_token`, `parse_page`, `Page`/`Item` e as constantes da API viram `pub(crate)` (os
  serviços não mudam). Dependências novas: `scraper` (entidades) e `regex`. `update-ce-faqs.sh`.
- **O que mudou em relação ao estudo**, medido sobre os 96 artigos:
  1. **`url` com a âncora** — `…/<identifier>+<_id>#section-<slug>`, não o artigo puro (§4). **59** perguntas se
     repetem no mesmo artigo com outra resposta (o portal sufixa a âncora, `…-2`); com o `url` do artigo elas teriam o
     mesmo nome de `.md` (slug da pergunta + hash de url + pergunta), e o `materializar_arvore` recusa colisão. Com a
     âncora: 0 colisões. O nó do artigo na árvore continua apontando para o artigo.
  2. **Âncora repetida no artigo é aviso, não `Err`** (§3). Há 1 (Escrituração): `ICMS/IPI` e `ICMS|IPI` dão o mesmo
     slug, com respostas diferentes — a pergunta difere, então o `.md` também. O `Err` ficou na colisão de verdade: a
     mesma pergunta na mesma âncora com outra resposta.
  3. **O markdown guarda as quebras de linha.** No template de 2024 a resposta é markdown, e a quebra é estrutura (116
     respostas multi-linha em texto puro — listas `- C860 …`, itens `a)`); o conversor da frota, que trata a quebra do
     fonte como espaço (certo no HTML do template de 2026), as colaria numa linha só.
  4. **A pergunta é procurada só no cabeçalho** do bloco (antes da linha da `<sy-collapse>`): uma resposta do template
     de 2024 tem `<h2>Unidades de atendimento</h2>` próprio, e buscar `<h2>` no bloco inteiro a tomaria pela pergunta.
  5. **Cópias comparadas a menos de espaços** (§3): 7 grupos só diferem por um `<br>` no meio da frase. 473 cópias
     (o estudo contou 475 → 2.702; aqui 2.704 sobre os mesmos pacotes).
  6. **Templates: 85 + 8**, não 86 + 7 (a lista de 8 nomes do §3 estava certa; a contagem não). Os dois pares de
     artigos de mesmo nome não são duplicatas inteiras — cada um fica com perguntas próprias; a "Restituição de Taxas"
     inteira é que está em outro artigo e some da árvore (92 nós).
  7. **O resto do catálogo não é "≈300 COMUNICADO COTRI"** (D-CEFAQ-7): dos 332, 78 são Pareceres CECON, 71 Comunicados
     COTRI, 38 Notas Explicativas, 28 Comunicados NUPAF, 12 Normas de Execução e ~104 avulsos — pendência §18, com os
     Pareceres CECON como candidato a `pareceres`. O aviso do D-CEFAQ-2 ficou como pendência §19, como recomendado.
- **Rodada real** (rede, AuliBot, 400 ms): 1 GET + 44 + 96 POSTs, ~1 min (um reset do edge no shell, retentado).
  428 conteúdos, 96 artigos de FAQ, 93 com seções, **3.179 pares → 2.706 perguntas** (2 pares a mais que os pacotes
  da manhã: o acervo é vivo). Avisos: a âncora repetida (1) e os 3 artigos sem seções.
- **Conferência cruzada:** uma implementação independente em Python (regex + `html.unescape`), sobre o cache da
  rodada, dá o mesmo snapshot item a item, na mesma ordem. A primeira versão dela divergia num item — ela buscava o
  `<h2>` no bloco inteiro (o caso 4 acima); corrigida, 0 diferenças.
- **`--usecache`** reproduz a árvore byte a byte e o snapshot a menos do `coletado_em`.
- **Testes:** 14 novos (22 no crate), com fixtures reais recortadas aos campos lidos (CTe, Diferimento, Bancos, um
  recorte da Escrituração, 3 páginas da listagem; os valores esperados tirados do Python). **Mutações** (10, todas
  pegas, cada uma por um teste nomeado): (a) a tag fechada no primeiro `>`; (b) sem o fallback `## `; (c) sem
  decodificar as entidades; (d) dedup desligado; (e) quebra do markdown como espaço; (f) `url` sem a âncora; (g) sem a
  guarda de pergunta vazia; (h) sem a guarda âncoras = blocos; (i) sem a guarda do gatilho dentro da `<sy-collapse>`;
  (j) sem a guarda de colisão.
- **`scripts/tools/update-ce-faqs.sh --no-serve`** de ponta a ponta: apaga o cache, coleta, 2.706 = 2.706 na guarda de
  encolhimento, `auli-collections ce`, 2.706 vetores (7 chaves no teto de 6.000 caracteres — a cauda não entra no
  vetor).
- **Smoke** `/v1/retrieve kind=faqs entity=ce` numa instância temporária (:3099): 3/5 em 1º com paráfrases (diferimento
  × ST, repartição do IPVA, EFD "contribuinte não localizado"); nos outros dois o 1º é vizinho legítimo — "Como faço
  para emitir DAE de ITCD01?" (a mesma pergunta de outro artigo; a esperada veio em 2º) e "O CT-e pode ser emitido
  antes do carregamento da mercadoria? E o DACTE?" (a do DACTE veio em 5º).
- **Conferência humana** (portão da F2): pendente — uma FAQ de cada template e uma longa, no portal.
