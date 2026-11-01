# ESTUDO-SP-Faqs — Perguntas Frequentes da SEFAZ-SP (coleção `faqs` da entidade `sp`)

**Data:** 2026-11-01 · **Escopo:** descoberta concluída + especificação para a 2ª coleção de atendimento da
entidade `sp` (hoje `servicos` + `pareceres`) — 4ª entidade com FAQs (RS 1.947 · SC 1.306 · PR 1.321). ·
**Órgão:** Secretaria da Fazenda e Planejamento do Estado de São Paulo (SEFAZ-SP). · **Fonte:** listas SharePoint
`Perguntas Frequentes`, **uma por subsite de serviço**, via REST `_api` anônima (JSON verbose). · **Robots/TLS/WAF:**
nenhum obstáculo (mesmo portal e mesma `_api` do scraper de serviços).

> **TL;DR — o maior acervo de FAQs da frota, pela via mais barata.** A página central
> `/Paginas/perguntas-frequentes.aspx` é um plugin JS (`/BibliotecaDeScripts/pluginPerguntasFrequentes.js`) que:
> (1) lê a lista **`Homes 360`** em `/servicos/_api` (99 "Referentes", a mesma lista que o `auli-scraper-sp servicos`
> já usa para o `Assunto`) e (2) para o Referente escolhido chama
> **`<URLHome>/_api/web/lists/getByTitle('Perguntas Frequentes')/items?$select=Id,Title,Resposta,Assunto&$orderby=Title&$top=1000`**.
> Censo de 01/11/2026: **95 subsites têm a lista, 68 com conteúdo, ≈ 2.950 perguntas visíveis** (soma de `ItemCount`
> = 2.977, que inclui itens não publicados invisíveis ao anônimo). Shape: `Id`, `Title` (pergunta), `Resposta`
> (HTML do editor SharePoint), **`Assunto` = MultiChoice** (array; 6/154 na NFe têm >1). Custo: **1 + 95 GETs**
> (sem paginação: nenhuma lista passa de 1.000). O transporte (`_api` verbose, cache, cortesia) **já existe** em
> `scrape.rs`. Decisões a bater: escopo dos Referentes internos (RH, TI…) e o link oficial por pergunta (§5.2).

---

## 1. Hierarquia discovery-first

| Rota | Veredito | Evidência |
|---|---|---|
| HTML de `/Paginas/perguntas-frequentes.aspx` | só filtros "Referente a:"/"Assunto:" + 6 cards institucionais; **zero pergunta** | WebFetch + `curl` (102 KB, `pluginPerguntasFrequentes.js`) |
| `_api/web/lists` da raiz e de `/servicos` | **não há** lista de FAQ ali (raiz: Imagens 1413, Glossario 516…; `/servicos`: Serviços 544, Homes 360 102) | inventário `$select=Title,ItemCount` |
| **`<subsite>/_api/web/lists/getByTitle('Perguntas Frequentes')/items`** | **✅ a fonte** — uma lista por subsite; a URL é literalmente a do plugin | `plugin.js` (`ObterLinksPesquisa`) + NFe 162 / IPVA 138 / ITCMD 103 / Simples 82 / ICMS 67 |
| `/_api/web/webs` (enumerar subsites) | **não público** (`null`) — enumerar pelo `Homes 360`, como o portal | — |
| FAQs estáticas em subsites (`servicos/saep/Paginas/perguntas-frequentes.aspx`, Profisco, `FAQ-RPA.aspx`, `nfp/Downloads/faqtecnico.pdf`) | prosa heterogênea, sem identidade por pergunta — **fora do v1** (D-SPFAQ-7) | WebFetch |
| Headless | não | — |

## 2. Endpoints (todos GET, `Accept: application/json;odata=verbose`, anônimo)

`P = https://portal.fazenda.sp.gov.br`

### 2.1 Referentes — `P/servicos/_api/web/lists/getByTitle('Homes%20360')/items?$select=Id,Title,Assunto,URLHome&$orderby=Title%20asc&$top=200`

É **exatamente** a chamada do plugin (`ObterLinksPesquisaServico`). 99 itens; **3 sem `URLHome`** (ignorar). `URLHome`
é caminho relativo com barra final (`/servicos/nfe/`) — tirar a barra antes de concatenar `/_api`. `Title` = nome
longo (`NF-e - Nota Fiscal Eletrônica`, às vezes com espaço sobrando: `trim`). `Assunto` aqui é a **categoria do
Home** (`DOCUMENTOS FISCAIS`, `RECURSOS HUMANOS`, `TRABALHO A DISTÂNCIA`…) — não confundir com o `Assunto` da FAQ.

### 2.2 Lista do subsite — `P<URLHome>/_api/web/lists/getByTitle('Perguntas%20Frequentes')?$select=ItemCount`

Teto informativo. **Não é guarda**: `ItemCount` conta rascunhos/pendentes que o anônimo não vê (NFe: 162 anunciados,
154 visíveis, todos `_ModerationStatus = 0`). Subsite **sem a lista** → erro 404 da `_api` → pular com aviso (4 dos 96).

### 2.3 Perguntas — `P<URLHome>/_api/web/lists/getByTitle('Perguntas%20Frequentes')/items?$select=Id,Title,Resposta,Assunto,Modified&$orderby=Title&$top=1000`

```json
{ "d": { "results": [ {
  "__metadata": { "uri": "https://portal.fazenda.sp.gov.br/servicos/nfe/_api/Web/Lists(guid'860c5896-…')/Items(68)", "etag": "\"7\"" },
  "Id": 68,
  "Title": "A consulta a uma NF-e é um serviço pago ou gratuito?",
  "Resposta": "<div class=\"ExternalClass31964D98C84147B0A5F56EDB5E0A6B27\"><p style=\"text-align&#58;justify;\">​A consulta de uma NF-e … do DANFE que acompanhou a mercadoria.</p></div>",
  "Assunto": { "__metadata": { "type": "Collection(Edm.String)" }, "results": [ "Consulta NF-e" ] },
  "Modified": "2020-05-18T12:46:18Z"
} ] } }
```

- **Sem paginação na prática**: `$top=1000` e a maior lista tem 276 (SAT). Tratar `d.__next` mesmo assim (o
  `DBlock<T>` do `scrape.rs` já tem o campo) — custa nada e protege o dia em que uma lista passar de 1.000.
- `Resposta`: HTML do editor SharePoint — `<div class="ExternalClass…">`, entidades numéricas **`&#58;`** (`:`),
  **zero-width `​`** no início do texto, `<p style>`, `<span>`, `<br>`, `<a href>`, tabelas e `<img src="/PublishingImages/…">`
  (relativas → absolutizar com `P`). `Resposta` pode ser `null`.
- `Assunto`: **MultiChoice** — `results: []` possível (0 na NFe), `>1` em 6/154. Os valores possíveis vêm de
  `…/fields?$filter=EntityPropertyName eq 'Assunto'` (`Choices`; NFe tem 29) — o plugin usa só para popular o
  combo; o scraper **não precisa** (lê o que vier no item).
- `Modified` existe (não entra no `.md`; útil para o log).

## 3. Medições (01/11/2026)

| | |
|---|---|
| Homes 360 | 99 (3 sem `URLHome`) |
| Subsites com a lista | **95** (4 sem) |
| Subsites com conteúdo | **68** (27 listas vazias) |
| Σ `ItemCount` | **2.977** (teto; visíveis ≈ 2.950) |
| Top 8 | SAT 276 · SPED 260 · Parcelamentos Especiais 169 · NFe 162 · NFP 149 · IPVA 138 · CADESP 135 · ITCMD 103 |
| Multi-assunto (NFe) | 6 de 154; zero assuntos: 0 |
| Publicação (NFe) | 154 visíveis, 154 com `_ModerationStatus 0`; 8 invisíveis = `ItemCount − visíveis` |

## 4. Modelagem → `FaqRaw` (snapshot `sp-faqs-snapshot.json`)

| `FaqRaw` | origem |
|---|---|
| `pergunta` | `Title` (`clean`; pode trazer `​` e `&#58;` — decodificar) |
| `resposta` | `html_para_texto(Resposta)` — versão do PR (lista fechada) **estendida** para o vocabulário do editor SharePoint: `table/thead/tbody/tr/td/th`, `h1–h6`, `blockquote`; `<img src>` relativo → `P + src` (regra do PR); `&#58;` e `​` saem na decodificação/`clean` |
| `origin` | `"<Title do Home> \| <Assunto[0]>"`; com vários assuntos, `"<Home> \| A, B"` (ordem da fonte); sem assunto, só `"<Home>"` — **D-SPFAQ-3** |
| `url` | **D-SPFAQ-2** (§5.2) |

Identidade = **`(URLHome, Id)`** — o `Id` é por lista, repete entre subsites (NFe tem `Id 68`, IPVA também pode ter).
Dedup por esse par; guarda: dentro de uma lista `Id` único (sempre verdade no SharePoint — se falhar, a paginação
duplicou). `text_to_embed`, `docs/faqs/*.md` e `sp-faqs.json` saem do `auli-collections sp` (genérico, provado em SC e PR).
**Sem mudança em `auli-contract`/`auli-collections`/`auli-cli`.**

### 4.1 Árvore do frontend (`sp-faqs-tree.json`, `auli_scraper_kit::faqtree`)

Raiz `Menu` ("Perguntas frequentes", `P/Paginas/perguntas-frequentes.aspx`) → um `Menu` por Home com conteúdo
(`title` = Home, `url` = `P<URLHome>`) → um `Faq` por Assunto (`title` = assunto, `origin` = trilha, `faq_items` com
`url` próprio). Pergunta multi-assunto aparece **em cada** assunto na árvore (é como o portal mostra), mas é **um** `FaqRaw`
só no snapshot (D-SPFAQ-3). Pergunta sem assunto vai num `Faq` "Geral" do Home. Home sem pergunta fica de fora.

## 5. Decisões (D-SPFAQ-*)

- **D-SPFAQ-1 — Fonte = `_api` por subsite**, enumerada pelo `Homes 360` (a fonte de verdade do portal; nunca
  hardcodar subsites). 1 GET (Homes) + 1 GET por `URLHome` (≈ 96). Reusar `Resp<T>/DBlock<T>`, o `fetch_api` e a
  cortesia (300 ms) do `scrape.rs` — tornar `pub(crate)` o necessário, como SC fez com `NextData`.
- **D-SPFAQ-2 — Link oficial.** Não há GET por pergunta na página central (o plugin não lê `location`; o botão
  "copiar link" existe mas o template não veio com UA de bot — §5.2 fecha). Ordem de preferência, decidida pelo
  teste: **(a)** o que o botão copia; **(b)** `P<URLHome>Lists/Perguntas%20Frequentes/DispForm.aspx?ID=<Id>` se
  responder 200 anônimo (é o link canônico do SharePoint para o item); **(c)** `P/Paginas/perguntas-frequentes.aspx`
  (página central, sem âncora possível — o plugin monta o DOM depois). Registrar a limitação se cair em (c).
- **D-SPFAQ-3 — Multi-assunto**: um `FaqRaw` por pergunta, `origin = Home | A, B` (todos os assuntos, na ordem);
  na árvore do frontend a pergunta repete em cada assunto. Sem assunto: `origin = Home`.
- **D-SPFAQ-4 — Guardas (`Err` antes de gravar):** (a) `Homes 360` vazio ou sem nenhum `URLHome`; (b) **zero perguntas
  no total**; (c) `Id` repetido dentro de uma lista; (d) `Title` vazio após `clean`; (e) `__next` presente e a página
  seguinte falhar. **Avisos:** subsite sem a lista (404 — hoje 4), lista vazia (hoje 27), `Resposta` nula/vazia,
  tag fora da lista fechada (guarda (h) do PR), e `Σ ItemCount − visíveis` ("n não publicados") no log.
  **Não** usar `ItemCount` como guarda.
- **D-SPFAQ-5 — `html_para_texto` vai para o kit** (`auli_scraper_kit::text::html_para_texto`), partindo da versão do
  PR (lista fechada) com a lista estendida acima; SC e PR passam a importar (SC hoje usa `<[^>]+>` aberto — a troca
  é melhoria, mas **muda bytes** do SC se alguma resposta tiver `<` literal; medir no `--usecache` do SC antes de
  trocar lá; se mudar, deixar o SC como está nesta TAREFA e abrir pendência).
- **D-SPFAQ-6 — Subcomando `faqs` + `all`** no `auli-scraper-sp`; default segue `servicos`.
- **D-SPFAQ-7 — Escopo dos Referentes (levar ao usuário).** Dos 68 Homes com FAQ, uma parte é de **público interno**
  (Acesso Remoto, Área Financeira, APS/política salarial, Profisco…). Duas opções: **(i) ingerir todos** —
  fidelidade ao portal, que os mostra no mesmo combo; precedente: o catálogo de serviços já inclui a aba
  "Servidor Público"; **(ii) filtrar pela categoria do Home** (`Homes 360.Assunto`: deixar fora `RECURSOS HUMANOS`,
  `TRABALHO A DISTÂNCIA`, `TECNOLOGIA DA INFORMAÇÃO`… — lista fechada no código, com teste). Recomendação: **(i) no v1**
  e medir no log de auditoria se FAQs internas entram no top-10 de perguntas de contribuinte; o ruído é o argumento
  para (ii), e ele precisa de evidência. As FAQs **estáticas** em prosa (SAEP, Profisco, RPA, PDF NFP) ficam fora
  em qualquer opção.
- **D-SPFAQ-8 — Rede sempre; cache só com `--usecache`** (`data/sp/raw/cache/faqs/`, chave = URL). Censo completo a
  cada rodada (≈ 97 GETs × 300 ms ≈ 1 min). `Modified` não vira marca de restart (doutrina do TARF/SP-pareceres).
- **D-SPFAQ-9 — Ingerir como está**: nomes dos Homes com a grafia do portal (`trim` só), `Choices` como vêm
  (`Errros` na NFe é da fonte), respostas sem "conserto" além da conversão HTML→texto.

### 5.1 Pegadinhas de serde / fonte

- `Assunto` é objeto `{ "__metadata": …, "results": [..] }` — `#[serde(default)]` + struct com `results: Vec<String>`
  (o `Facet` do `scrape.rs` já é exatamente isso: reusar).
- `Resposta: Option<String>`; `Title` pode ter `​` (U+200B — o `clean` do kit já remove) e `&#58;`.
- `URLHome` com barra final; `Title` do Home com espaços nas pontas e, num caso, espaço **inicial**
  (`" AIDF - …"`).
- `Id` **não é global** — chave `(URLHome, Id)`.
- A `_api` devolve 404 (não vazio) para subsite sem a lista — distinguir de lista vazia (200 com `results: []`).
- O `$orderby=Title` é do plugin (ordem alfabética na tela); para o scraper tanto faz, mas manter igual deixa o cache
  comparável com o que o portal serve.

### 5.2 Sondagem pendente (2 comandos — fecha a D-SPFAQ-2)

```bash
cd /tmp; P=https://portal.fazenda.sp.gov.br
# (a) o que o botão "copiar link" monta — o template, pedido como o browser pede
curl -s -A "Mozilla/5.0" "$P/BibliotecaDeScripts/TemplateHTML/TemplatePerguntasFrequentesGeral.html" | grep -oiE ".{250}copiarLink.{250}" | head -2
# (b) o DispForm do item é público?
curl -s -o /dev/null -w '%{http_code}\n' -A "Mozilla/5.0" "$P/servicos/nfe/Lists/Perguntas%20Frequentes/DispForm.aspx?ID=68"
```

## 6. Plano de implementação (fases com portão humano)

**F1 — parser puro + fixtures** (`auli-scraper-sp/src/faqs.rs`): structs serde (`Home{Id,Title,URLHome}`, `Item{Id,Title,
Resposta,Assunto{results},Modified}`), `homes_com_url`, `montar_faq_raw`, `montar_arvore`, `html_para_texto` do kit
(D-SPFAQ-5). Fixtures: JSON real do `Homes 360` (3 itens, um sem `URLHome`) e de uma lista (3 itens: um multi-assunto, um
com `Resposta` em tabela + `&#58;` + `​`, um com `<img>` relativo). Casos `Err`: `Id` repetido na lista, `Title`
vazio, zero total. Mutação em cada guarda. → `cargo test -p auli-scraper-sp`.
**F2 — transporte**: reuso do `fetch_api`; laço pelos Homes com `URLHome`; 404 → aviso e segue; `__next` → segue;
`write_faqs("sp", …)` + `sp-faqs-tree.json` atômico; subcomando. → rodada real: log por Home
(`<Home>: <visíveis> perguntas (ItemCount <n>)`), total ≈ 2.950, 27 vazias, 4 sem lista. **Portão humano:** abrir 3
`.md` (uma multi-assunto, uma com tabela) e comparar com a página central filtrando o Referente.
**F3 — pipeline**: `registry.toml` (`collections` da `sp` + `faqs`) → `gen-frontend-entities` → `auli-collections sp`
→ `build-packs.sh sp faqs` (≈ 2.950 vetores — o maior lote de FAQs; RAM OK, bem abaixo do SP-pareceres) → smoke
`/v1/retrieve kind=faqs entity=sp`: "consulta de NF-e é paga?" → Id 68 da NFe no topo. Conferir `config/prompts/sp.txt`.
**F4 — registro**: `REGISTRO-scrapers.md#sp-faqs`, `update-sp-faqs.sh`, README/SCRAPERS.md ("FAQs: RS, SC, PR, SP").

## 7. Critérios de aceitação

- [ ] Testes F1 verdes, mutações verificadas.
- [ ] Rodada real: ≈ 96 GETs, 68 Homes com conteúdo, total visível entre 2.800 e 3.100; nenhum `Err`.
- [ ] `--usecache` reproduz snapshot e árvore byte a byte.
- [ ] `.md` com `trilha` `Home | Assunto` (multi-assunto com vírgula), `link` conforme D-SPFAQ-2 decidida.
- [ ] Aba FAQs do SP renderiza Home → Assunto → perguntas; chat tipo 1 cita `FAQ n` com o link certo.
- [ ] `check-scraper-boundary.sh`, `check-sem-navi.sh`, `check-registry-sync.sh` verdes.

## 8. Evidência bruta (reprodutível no Desktop Server, 01/11/2026)

```bash
P=https://portal.fazenda.sp.gov.br; A='Accept: application/json;odata=verbose'
curl -s $P/BibliotecaDeScripts/pluginPerguntasFrequentes.js | grep -oE "getByTitle\('[^']*'\)/[a-z]+\?[^\"]*" | sort -u
#  getByTitle('Homes%20360')/items?$select=Id,Title,Descricao,Assunto,URLHome,TermoDeBusca&$orderby= Title asc&$top=200
#  getByTitle('Perguntas%20Frequentes')/items?$select=Id,Title,Resposta,Assunto&$orderby=Title&$top=1000
#  getByTitle('Perguntas%20Frequentes')/fields?$filter=EntityPropertyName eq 'Assunto'
curl -s -H "$A" "$P/servicos/_api/web/lists/getByTitle('Homes%20360')/items?\$select=Id,Title,URLHome&\$top=200" | jq '.d.results|length'   # 99
curl -s -H "$A" "$P/servicos/nfe/_api/web/lists/getByTitle('Perguntas%20Frequentes')?\$select=ItemCount" | jq .d.ItemCount          # 162
curl -s -H "$A" "$P/servicos/nfe/_api/web/lists/getByTitle('Perguntas%20Frequentes')/items?\$select=Id&\$top=1000" | jq '.d.results|length'  # 154
# censo: 95 com lista, 27 vazias, Σ ItemCount 2.977 (sp_faq_totais.txt)
```

## 9. Implementação e correções (01/11/2026)

**Código:** `auli-server/crates/scrapers/auli-scraper-sp/src/faqs.rs` (subcomandos `faqs`/`all` no `main.rs`),
fixtures `tests/fixtures/sp-faqs-homes.json` (os quatro jeitos de `URLHome`) e `sp-faqs-lista-gia.json` (cinco
perguntas reais da GIA: dois assuntos, imagem, lista com registro SPED, `href` relativo, tabela). Registro de
manutenção: `docs/REGISTRO-scrapers.md#sp-faqs`. Atualização: `scripts/tools/update-sp-faqs.sh`.

**O censo de verdade** (as 96 listas lidas, não o `ItemCount`):

| | Estudo (§3) | Medido |
|---|---|---|
| Subsites com conteúdo | 68 | **64** |
| Listas vazias | 27 | **31** |
| Subsites sem a lista (404) | 4 | **0** — 95 respondem 200; o 96º é o Home de RH, com `URLHome` absoluto para `www.sfp.sp.gov.br`, que nem o plugin lê |
| Perguntas visíveis | ≈ 2.950 (estimado pelo `ItemCount`) | **2.665** |
| Com mais de um `Assunto` | 6/154 (NFe) | **388/2.665** (14,5%; nos Parcelamentos Especiais, 169/169) |

**Decisões fechadas:**
- **D-SPFAQ-2 → (a).** O template `TemplatePerguntasFrequentesGeral.html` mostra o que o botão copia:
  `webAbsoluteUrl + URLHome + 'Paginas/perguntas-frequentes.aspx#ID=' + Id`. O `DispForm.aspx` dá **401**. O link
  é único por (subsite, `Id`) — 32 perguntas repetem o texto no mesmo subsite, e sem o `#ID=` os `.md`
  colidiriam. Dois subsites têm a página quebrada no próprio portal (`cfaiim` 404, 33 perguntas;
  `evolucao-funcional` 401, 1) — limitação registrada no REGISTRO; o `isencao-icms-veiculos` vem sem a barra
  final (o botão do portal monta um link quebrado) e o scraper a põe.
- **D-SPFAQ-5 → não feito; conversor no próprio crate.** Medido: nenhum `<` como texto nas 2.665 respostas — o
  editor do SharePoint escapa (`&lt;escrituras&gt;`). A lista fechada do PR não é necessária aqui: toda tag sai, e
  as entidades são decodificadas depois. As regras próprias da fonte (entidades numéricas, espaço cru, `<style>`/
  `<svg>`, tabela) não servem ao SC nem ao PR, e mover para o kit obrigaria a reverificar os dois byte a byte. O
  decodificador é o `decode_html` do `pareceres.rs`, que já tratava as numéricas (o `decode_entities` do kit não).
- **D-SPFAQ-7 → (i), todos os subsites**, inclusive os de público interno, como o portal os mostra no mesmo
  combo. Medir no log de auditoria se FAQs internas entram no top-10 das perguntas de contribuinte.

**Desvios do desenho, com o porquê:**
- **404 de lista ausente não vira aviso.** Não houve nenhum; qualquer falha de chamada é `Err` e para a coleta,
  como nas outras FAQs. Se um subsite novo vier sem a lista, a coleta para alta com o 404 na mensagem.
- **O `ItemCount` não é lido**: era só informativo e dobraria as chamadas.
- **Cortesia de 1 s** (as constantes do `pareceres.rs`, como SC e PR), não os 300 ms dos serviços: ~2 min.
- **O `Title` é texto puro.** A pergunta 530 do SAT tem "(`<IM>`)" de verdade; nenhuma tem entidade. O título só
  passa pelo `clean` — tirar tag apagaria o `<IM>`.
- **Conversão** (`html_para_texto`): `<style>`, `<script>`, `<svg>` e comentários saem com o conteúdo; o espaço cru
  colapsa antes de os blocos virarem linha (o HTML traz `\n   ` no meio da frase); célula de tabela vira espaço e
  linha de tabela vira linha ("CST_ICMS 53") — não `|`, porque as respostas citam registros SPED
  (`|C197|SP90090278|…`); `href`/`src` são decodificados (`https&#58;//`) e os relativos ganham o domínio.
- O total do RS no cabeçalho é 1.990 (rodada de 01/11), não 1.947.

**Rodada real:** 97 GETs (a lista do Profisco estourou o timeout de 60 s uma vez e passou na 2ª tentativa),
**2.665 perguntas** em 95 subsites (64 com conteúdo), 2.665 urls únicas. No texto não sobra tag (os `<…>`
restantes são texto escapado na fonte: `<usuário>`, `<finNFe>`, envelopes SOAP) nem zero-width; 5 entidades
ficam como texto porque a fonte as codificou em dobro (`&amp;#10;`). Nenhuma resposta vazia; 3 passam de 6.000
caracteres (a cauda fica fora do vetor). `--usecache` reproduz snapshot e árvore byte a byte. Árvore: 64 `Menu`
de subsite, 365 `Faq` de assunto, 3.068 itens (a pergunta multi-assunto em cada assunto). `auli-collections sp`
→ 2.665 `.md`, sem colisão. `build-packs.sh sp faqs` → 2.665 vetores, com serviços e pareceres preservados no
manifesto. Smoke `POST /v1/retrieve` (instância temporária, `127.0.0.1:3999`, `kind=faqs`): "a consulta de NF-e é
paga?" → NFe `#ID=68` em 1º (o smoke da F3); "como informar ICMS substituído na GIA?" → GIA `#ID=71`; "erro ao
migrar o banco no programa GIA" → GIA `#ID=82`.

**Testes:** 15 das FAQs (27 no crate); 17 mutações — cada guarda, cada passo da conversão, a barra do
`URLHome`, o `#ID=`, a trilha com todos os assuntos e a árvore — derrubam o teste correspondente.
