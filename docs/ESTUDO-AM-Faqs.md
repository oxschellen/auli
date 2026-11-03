# ESTUDO-AM-Faqs — Perguntas Frequentes da SEFAZ-AM (coleção `faqs` da entidade `am`)

**Data:** 2026-11-02 · **Estado:** **implementado** (F0 feita, §8). · **Órgão:** SEFAZ-AM. · Antes deste estudo a
`am` tinha só `servicos` (a árvore do Portfólio de Serviços, Next.js App Router, via flight RSC).

> **TL;DR — as FAQs do AM moram dentro das páginas do portfólio, e só lá.** Cada detalhe
> `https://www.sefaz.am.gov.br/portfolio-servicos/detalhes/{id}` pedido com **`RSC: 1`** traz
> `serviceDetails.visibleSections` e o campo **`perguntasRespostasHtml`** — ou como referência a um bloco de texto do
> mesmo flight (`"$c"` → `c:T<tamanho hex>,<html>`) ou como **HTML inline**. Zero XHR, zero headless. Coleta = a
> listagem (o mesmo flight dos serviços) + **306 detalhes** — os 243 serviços e as 63 categorias que têm página — a
> 1 s. **47 páginas** mostram a seção. O HTML é de **editor livre** (Word/CKEditor): perguntas numeradas `1)` /
> `1.` / `1 -` em `<p>`, `<h3>`, `<div>` ou `<li>`, às vezes com a resposta na mesma linha depois do `<br>`, prefixo
> `Resposta:`/`R.`, listas e tabelas dentro das respostas. O parser (§3) é por **linhas do texto + número da
> pergunta**, validado pelos "?"; sem número, a linha com "?" abre a pergunta; sem nenhuma, a seção vira **1 FAQ**
> (D-AMFAQ-4). **Cópias exatas** entre páginas irmãs (Refis ×5, parcelamentos, REDAR, certidões) ficam na de menor
> id. `origin` = `"<Categoria> | <Página>"`; `url` = o detalhe, sem query, igual ao `link` dos serviços.
> **Fora:** o **Portal do Servidor Fazendário** (o RH dos servidores, que o mesmo flight lista e o `servicos` já
> deixa de fora — D-AMFAQ-10); as Perguntas e Respostas do GCC (casca Struts2-jQuery, conteúdo por JS → pendência);
> a FAQ do Portal NFC-e (PDF v4.0 de jan/2017).

---

## 1. Hierarquia discovery-first — o que foi avaliado

| Rota | Veredito | Evidência |
|---|---|---|
| **`GET /portfolio-servicos/detalhes/{id}` + `RSC: 1`** | **fonte única**; `text/x-component`, 200; `perguntasRespostasHtml` por referência ou inline; seção visível ⇔ texto não vazio (0 divergências) | sondagem 02/11 + F0 |
| Listagem `/portfolio-servicos/todos` + `RSC: 1` | índice: a árvore `items` (a mesma dos serviços) — 283 folhas (243 com detalhe) e 64 categorias (63 com página) | F0 |
| `www.sefaz.am.gov.br/robots.txt` | **não existe** (404, a página "não encontrada" do Next.js) | F0 |
| `sistemas.sefaz.am.gov.br/gcc/menucontribuinte/publico/perguntasRespostas.action` | 200, 12.830 B, só a casca Struts2-jQuery, texto vazio → conteúdo por JS; **fora**, pendência | sondagem |
| `portalnfce.sefaz.am.gov.br/empresario/perguntas-e-respostas/` | 200, WordPress; a FAQ é **PDF** de 2017; `/consumidor/…` → redireciona; `wp-json` → 404; **fora** | sondagem |
| FAQ geral em `www.sefaz.am.gov.br` | não existe (o institucional é o próprio portfólio) | busca |
| Headless | não (verificado em jul/2026: expandir accordions não dispara rede) | REGISTRO#am |

**O que a listagem traz além da árvore (F0).** O flight de `/todos` tem 370 ids distintos em URLs de detalhe — o
número da sondagem —, mas a árvore `items` só tem 306: os outros 64 estão todos no bloco
`linksPortalServidorFazendario`, depois da árvore (os blocos `servicosMaisAcessados`/`servicosDestaques` repetem ids
da árvore). Os serviços do AM não cresceram 55%: a árvore tem 283 folhas (eram 278 em 05/07/2026), 243 com detalhe
(eram 239).

## 2. Receita de coleta

1. **Índice:** o flight de `/todos` (âncora `"items":[`) → `parse_items` do `am.rs` → cada nó da árvore cuja `url`
   começa por `/portfolio-servicos/detalhes/` (serviço ou categoria), na ordem do portfólio, sem a query (há
   `?profile=todos` e `?gerarTaxa=true&profile=todos`; o detalhe é o mesmo — o campo sai igual com e sem `profile`).
   Externos e submenus não têm detalhe. Guard: ≥ 280 páginas (hoje 306).
2. **Detalhe:** o flight de cada página (âncora `"visibleSections":[`). Cortesia 1 s, AuliBot.
3. **Seção:** `visibleSections` sem `PERGUNTAS_FREQUENTES` → sem FAQ (seções ocultas podem trazer marcação vazia no
   campo: a 1143 tem 1.264 bytes de `<p>&nbsp;</p>`). Senão, `perguntasRespostasHtml` (string JSON): `"$<hex>"` → o
   bloco `<hex>:T<tamanho>,` do flight, lido em **bytes**, com o flight percorrido **de linha em linha desde o começo**
   (os blocos vêm colados, sem `\n`: `…</p>b:T…`; e buscar `c:T` direto acharia `1c:T`); qualquer outro `$…` → `Err`;
   senão o valor **é** o HTML. As duas chaves aparecem uma vez em cada detalhe; repetida → `Err`. Seção visível com
   campo `null` ou HTML sem texto → `Err` (hoje 0).
4. **Parse** (§3) → pares → cópias fora → `write_faqs("am", …)` + `am-faqs-tree.json` (categoria → página →
   perguntas).

## 3. Parser (fechado nos formatos medidos)

**Linhas.** HTML da seção → texto pelo conversor das FAQs do SP/GO/DF: `<style>`, `<script>` e comentários fora (4
páginas têm comentários do Word); `<a href>` → `texto "url"` (16 páginas têm links, todos absolutos); `p`, `div`,
`li`, `tr`, `h1–h6`, **`<br>`** etc. → quebra de linha; célula → espaço; entidades no fim. As linhas não vazias são
os blocos. (O 502 e o Refis põem pergunta e resposta no mesmo `<p>`, separadas por `<br>` — no Refis,
`<br data-start=…>`; o 3162 usa `<div>` aninhado; o 1082 e o 962 têm tabela e listas na resposta.)

**Número de pergunta** = a linha começa por `(\d+)\s*[).\-–]` e o caractere logo depois da pontuação não é dígito
(`1)`, `1.`, `1 -`, `1 –`; **não** `2.1)`, subtítulo dentro da resposta do REDAR, nem `1.000`).

**Quem abre pergunta:** se há linhas numeradas **e** ≥ 50 % delas têm "?", a linha numerada (o REDAR tem 2 tópicos sem
"?" em 5 — "2) Situações que podem ser objeto de REDAR" —, que viram a pergunta inteira); senão, a linha que tem
"?" (446, 982, 1825: uma pergunta sem número no topo). A regra dos 50 % impede que um passo a passo numerado sem "?"
vire perguntas. A pergunta vai até o **primeiro "?"**, inclusive; o que sobra na linha abre a resposta (nenhuma linha
numerada tem dois "?"). Resposta = isso + as linhas seguintes até a próxima pergunta, com `^(Resposta\s*:|R\s*[.:])\s*`
removido do começo (16 "Resposta:", 1 "R."). Linhas antes da 1ª pergunta → aviso (hoje 0). Pergunta sem resposta →
**`Err`** (hoje 0). Numeração fora de ordem → aviso (hoje 0: todas as páginas numeradas vão de 1 em 1).

**Sem nenhuma pergunta** (302, "Isenções de ICMS e Regimes Especiais", a página de uma categoria): **1 FAQ** —
pergunta `"Dúvidas frequentes — <nome da página>"`, resposta = o texto inteiro.

**Cópias:** par `(pergunta, resposta)` idêntico em páginas distintas → fica o de **menor `id`**, na ordem do portfólio
(Refis `4302/4324/4325/4326/4327`; parcelamentos `501/3262/3502/3503` e `502/3285/3382/3402`; REDAR `1202/1203/3282`;
certidões `541/1042/1123/1124`; PCD `4484/4502`; táxi/furto `63/83`; IE `882/962`).

## 4. Modelagem → `FaqRaw`

| `FaqRaw` | origem |
|---|---|
| `pergunta` | §3 (sem o número) |
| `resposta` | §3 (texto; links como `texto "url"`) |
| `origin` | `"<Categoria de topo> \| <Nome da página>"`; a página de uma categoria de topo, só o nome |
| `url` | `https://www.sefaz.am.gov.br/portfolio-servicos/detalhes/{id}` (o `link` dos serviços) |

Identidade = `(link, pergunta)`: o nome do `.md` sai dos dois, então a mesma pergunta duas vezes na página com
respostas diferentes → `Err` (hoje 0). Guardas: ≥ 280 páginas, ≥ 40 com perguntas (hoje 47), ≥ 200 perguntas depois das
cópias (hoje 176); fail-closed antes de gravar.

**Censo (02/11/2026):** 47 páginas com a seção (45 por referência, 2 inline — 2922 e 2923): 43
numeradas, 3 com uma pergunta sem número (446, 982, 1825), 1 sem pergunta (302) → **285 pares → 109 cópias → 176
perguntas** de 37 páginas. Maiores: 1082 Isenção de IPVA PCD-responsável 17, 1222 NF-e Avulsa 15, 4502 Isenção de IPVA
PCD-condutor 10, Refis 10 (×5), parcelamentos 8–9. Por categoria: IPVA e Veículos 62; Refis, Pagamentos,
Parcelamentos e REDAR 52; Restituição e Ressarcimento ST 22; Documentos Fiscais Eletrônicos 15; Certidão 8;
Cadastro 6; ITCMD 5; Isenções de ICMS e Regimes Especiais 4; Fiscalização 1; Fornecedores 1.

## 5. Decisões (D-AMFAQ-*)

- **D-AMFAQ-1** — Fonte única = `perguntasRespostasHtml` dos detalhes RSC (306 GETs/rodada). O `servicos` **não muda**
  (segue na listagem).
- **D-AMFAQ-2** — Reuso do `am.rs`: `fetch_flight` (a âncora virou parâmetro), `parse_items` e `Node`, como
  `pub(crate)`. Novas funções puras no `faqs.rs`: `paginas`, `secao` (portão `visibleSections` + campo), `valor_unico`,
  `bloco_de_texto`, `numero`, `parear`, `sem_copias`, `sem_colisao`. O conversor HTML → texto é o das FAQs do DF, sem
  mudança (4ª cópia — pendência §10 do kit).
- **D-AMFAQ-3** — `origin` em 2 níveis categoria | página (a categoria é a `classe` do `servicos`).
- **D-AMFAQ-4** — Seção sem nenhuma pergunta → 1 FAQ por página, com a pergunta sintética
  `"Dúvidas frequentes — <nome>"` (o único texto que não é da fonte; hoje 1 página, a 302).
- **D-AMFAQ-5** — Cópia exata entre páginas fica na de menor `id`. Alternativa (manter as cópias) descartada: 5 cópias do
  Refis no vetor degradam o retrieve sem ganho.
- **D-AMFAQ-6** — GCC e NFC-e fora: o GCC vira **pendência** ([auli_scraper_pendências.md](auli_scraper_pendências.md)
  §16: sondar o XHR do Struts2); a NFC-e não (PDF de 2017, técnico).
- **D-AMFAQ-7** — `AuliBot` (o portal aceita), 1 s de cortesia (o padrão das FAQs da frota), rede sempre, cache lido
  só com `--usecache`.
- **D-AMFAQ-8** — `faqs` + `all` no `auli-scraper-am`; o default segue `servicos`.
- **D-AMFAQ-9** — Ingerir como está (depois do §3).
- **D-AMFAQ-10** — Escopo = a árvore `items` do portfólio, a mesma do `servicos`. O bloco
  `linksPortalServidorFazendario` do mesmo flight (64 páginas do RH dos servidores — atestado médico, dados bancários
  da folha, férias, posse, consignados; 5 delas com FAQ) fica fora: não é atendimento ao contribuinte. Consequência:
  o único caso de pares sem número numa lista (`<li>` "pergunta? resposta", o 3742) era do servidor.

## 6. Plano (feito)

- **F0 rede** — robots.txt, AuliBot na listagem e no detalhe, com e sem `profile`, os 7 inline, a origem dos 370.
- **F1 parsers puros + fixtures reais** — flights recortados (882 por referência, 1143 oculta com marcação, 2923
  inline) e o HTML da seção de 502, 4302, 1202, 3162, 1082, 446 e 302; contagem exata por fixture conferida por uma
  implementação independente em Python; mutações (§8).
- **F2 transporte** — 306 GETs com cache; `write_faqs("am", …)` + árvore.
- **F3 pipeline** — `registry.toml`, `gen-frontend-entities`, `auli-collections am`, `build-packs.sh am faqs`, smoke
  `/v1/retrieve kind=faqs entity=am`.
- **F4 registro** — `REGISTRO-scrapers.md#am-faqs`, `update-am-faqs.sh` (guarda de encolhimento), README/SCRAPERS.md,
  pendências.

## 7. Evidência (02/11/2026)

```
robots.txt → 404 (página "não encontrada" do Next.js)
GET /todos + RSC:1 (AuliBot) → 200, 389.543 B, text/x-component
  árvore items: 283 folhas (243 com detalhe, 40 externas/submenu) + 64 categorias (63 com página) = 306 páginas
  370 ids distintos em "url":"/portfolio-servicos/detalhes/N" no flight inteiro: 306 da árvore + 64 do bloco
  linksPortalServidorFazendario (todos depois da árvore)
GET /detalhes/882 + RSC:1 → 200, 237.883 B; sem ?profile → 237.854 B, o mesmo perguntasRespostasHtml
  linhas do flight: … a:T (3.439) b:T (3.668) c:T (1.704) 2:["$",…"serviceDetails"…]  — blocos colados ('>' antes de b e c)
  "visibleSections":[…,"PERGUNTAS_FREQUENTES",…]   "perguntasRespostasHtml":"$c"  →  c:T6a1,<p>1) O que &eacute; REDESIM?</p>…
FAQ no censo (52 páginas): 47 na árvore, 5 no Portal do Servidor (3742, 3843, 3847, 3872, 4022)
seções ocultas com $ref no campo: 10+ (1143: 1.264 B de marcação vazia) — o portão é visibleSections
linhas numeradas sem "?": só no REDAR (2 tópicos × 3 cópias); com dois "?": 0; numeração fora de ordem: 0
prefixos de resposta: "Resposta:" 16, "R." 1; links: 16 páginas, 0 relativos, 0 lixo; tabela: 1; comentário: 4
GCC perguntasRespostas.action → 200, 12.830 B, casca Struts2-jQuery sem conteúdo
```

## 8. Implementação (02/11/2026)

- `auli-scraper-am/src/faqs.rs` (novo), `faqs` e `all` no `main.rs`; no `am.rs`, `fetch_flight` recebe a âncora e
  `parse_items`, `Node`, `BASE`, `LISTA_URL` e `ANCORA_LISTA` viram `pub(crate)` (os serviços não mudam). Dependências
  novas: `scraper` (entidades) e `regex`.
- **Rodada real** (rede, AuliBot, 1 s): 307 GETs em ~13 min (~2,5 s por página: a cortesia mais ~230 KB de flight).
  176 perguntas (§4). Avisos 0. Lixo (`<`, `mso-`, entidade) 0, prefixo restante 0, colisão 0; sem "?" no fim só os 2
  tópicos do REDAR e a sintética da 302.
- **A primeira rodada parou na guarda**: o piso de perguntas estava em 200, posto sem medir (este estudo dizia ~175
  únicas); ela barrou antes de gravar, como deve. Piso corrigido para 150.
- **Conferência cruzada:** uma implementação independente das regras do §3 em Python, sobre os mesmos flights, dá os
  mesmos 285 → 176, os mesmos pares na mesma ordem; as 13 respostas que diferem são só os links (o protótipo não os
  converte).
- **`--usecache`** reproduz a árvore byte a byte e o snapshot a menos do `coletado_em`.
- **Testes:** 20 novos, com fixtures reais (3 flights recortados + o HTML de 7 seções; as contagens conferidas pelo
  Python). **Mutações** (9, todas pegas): `<br>` fora das quebras; tamanho do bloco em caracteres; sem a regra do
  `2.1)`; sem a regra dos 50 %; bloco buscado direto, sem ler o flight de linha em linha; cópia na primeira página e
  não na de menor id; sem o portão `visibleSections`; texto depois do "?" descartado; prefixo sem exigir pontuação.
  As duas primeiras escaparam no começo — o `<br>` e o texto depois do "?" se cobrem no Refis e no 502 — e ganharam um
  caso cada.
- **`scripts/tools/update-am-faqs.sh --no-serve`** de ponta a ponta: apaga o cache, coleta (um 502 retentado),
  176 = 176 na guarda de encolhimento, `auli-collections am`, 176 vetores.
- **Smoke** `/v1/retrieve kind=faqs entity=am` numa instância temporária: 5/5 em 1º (renovação da isenção de IPVA PCD,
  parcelamento de IPVA, REDAR, débitos do Refis 2025, NF-e Avulsa).
- **Conferência humana** (portão da F2): 502 "Quando posso parcelar meu IPVA?" (pergunta e resposta separadas por
  `<br>`), 4302 "Quais débitos podem ser incluídos no Refis 2025?" e 446 (uma pergunta sem número) — a conferir no
  site pelo mantenedor.

Fontes: `docs/REGISTRO-scrapers.md#am` e `#am-faqs`, `auli-scraper-am/src/{am,faqs}.rs`.
