# ESTUDO-DF-Faqs — Perguntas Frequentes da Receita-DF (coleção `faqs` da entidade `df`)

**Data:** 2026-11-02 · **Estado:** **implementado** (§8) — descoberta em 3 rodadas de sondagem na máquina do usuário,
revisada e completada pela F0 (§7). · **Órgão:** Subsecretaria da Receita / Secretaria de Economia do DF (o Auli
registra `SEFAZ-DF`). · A `df` tinha só `servicos` (475 serviços, 139 classes em 02/11/2026, D-DF-WAF).

> **TL;DR — duas fontes oficiais, HTML server-side, sem headless; 993 FAQs.**
> **(A) Receita — `aplicacoes/perguntasfrequentes/`**: a página `perguntasfrequentes.cfm` (617 KB) é a **casca**
> com **22 temas** (cards), cada um com `codAssunto` e um `select` de **81 tipos de atendimento**. As perguntas chegam
> por **`POST perguntasFreqBusca_div.cfm`** (`selectTipoAtendimento`, `codAssunto`, `buscaPergunta`) — fragmento HTML
> com 1 card por pergunta: `<a data-target="#collapse<ID>">pergunta</a>` + `<div id="collapse<ID>">R. resposta</div>`.
> **`tipo=0` devolve o tema inteiro**: 22 POSTs → **900 perguntas** (02/11/2026), id estável = `<ID>`. Mesmo host da
> Carta ⇒ **mesmo WAF JA3 ⇒ curl** (o `ureq` leva reset — medido na F0); o host aceita o **AuliBot** pelo curl.
> `robots.txt`: `Allow: /`.
> **(B) ISS/NFS-e — `iss.fazenda.df.gov.br/online/Default/PerguntasFrequentes.aspx`**: página única (121 KB),
> `#DynamicAccordions` com **95 pares `.accordionHeader`/`.accordionContent`** sob 6 `<h5>` (seções) → **93** depois
> de tirar 2 repetições dentro da mesma seção. Sem `robots.txt` (404); o `ureq` passa. O DF é estado **e** município:
> ISS é tributo da própria SEFAZ-DF.
> **Fora:** Nota Legal (`notalegal.df.gov.br` → **403** ao AuliBot; e o tema "Programa Nota Legal" já está na (A) com
> 58 perguntas), `df.gov.br` (WordPress institucional), `economia.df.gov.br` (Liferay, notícias).

---

## 1. Hierarquia discovery-first — o que foi avaliado

| Rota | Veredito | Evidência |
|---|---|---|
| `receita.fazenda.df.gov.br/robots.txt` | `User-agent: *` / `Allow: /` + sitemap — sem restrição | sondagem r2 |
| **`…/perguntasfrequentes/perguntasfrequentes.cfm`** | casca: 22 `form#formBusca<A>` + `select#selectTipoAtendimento<A>` + `input#codAssunto=<A>` + `input#pgDivListarPerguntas<A>=perguntasFreqBusca_div.cfm`; títulos dos temas em `<h5>` dentro de `button[data-target="#collapseOne<A>"]`; 0 árvore JS | r2, F0 |
| **`POST …/perguntasFreqBusca_div.cfm`** | **fonte real**; `tipo=0` → tema inteiro (ex. `codAssunto=24` → 26.725 B, 18 perguntas); `tipo=80` → 10.481 B (subconjunto) | r3, F0 |
| `POST …/perguntasFreqBuscaGeral_div.cfm` (`buscaGeralPergunta`) | busca livre; vazia → 4.744 B (nada útil); **não usar** | r3 |
| `area.cfm?id_area=1568` | home do Portal da Receita; só linka a FAQ | r2 |
| `mobile.receita.fazenda.df.gov.br` | espelho da mesma app | busca |
| **`iss.fazenda.df.gov.br/online/Default/PerguntasFrequentes.aspx`** | 200, 121.448 B; `h5` seções; 95 × `accordionHeader`/`accordionContent`; plataforma NotaControl/ISSNet sob domínio oficial | r2 + WebFetch, F0 |
| `iss.fazenda.df.gov.br/robots.txt` | **404** — sem restrição | F0 |
| `notalegal.df.gov.br` | **403** ao AuliBot (5.467 B) — fora; tema coberto pela (A) | r2 |
| Headless | não | — |

## 2. Receita de coleta (A)

`B = https://www.receita.fazenda.df.gov.br/aplicacoes/perguntasfrequentes/`. Tudo por `curl` (D-DF-WAF), com o AuliBot.

1. **Casca:** `GET B/perguntasfrequentes.cfm` → para cada `form[id^=formBusca]` com `input[name=codAssunto]` (o da busca
   geral não tem): `A` = o `value`; **tema** = texto do `<h5>` dentro de `button[data-target="#collapseOne<A>"]`;
   **tipos** = `select[name=selectTipoAtendimento] option[value!="0"]` → `(cod_tipo, rótulo)`, sem o sufixo
   `" - serviço"` (49 dos 81 rótulos). Hoje: 22 temas, 81 tipos; piso 18 (`Err`) e `A` único.
2. **Tema inteiro:** `POST B/perguntasFreqBusca_div.cfm` com os campos `selectTipoAtendimento=0`, `codAssunto=<A>`,
   `buscaPergunta=` → fragmento. **Cada pergunta vem embrulhada num `div#collapsePerguntasFrequentes`** (o id se
   repete — não serve de chave): o fragmento é **cortado nesses embrulhos antes do parse** (§2.1). Em cada pedaço,
   exatamente um `div.card`: `id` = sufixo numérico do `data-target` do `a[data-toggle=collapse]`, `pergunta` = o
   texto do `<a>`, `resposta_html` = o `div#collapse<id>`.
3. **Trilha de 3 níveis (D-DFFAQ-3):** `POST` por `(A, cod_tipo)` → os ids do tipo; trilha =
   `Receita-DF | <Tema> | <Tipo>`; pergunta que não cai em nenhum tipo → `Receita-DF | <Tema>`. Guarda de partição:
   id de tipo fora do tema = `Err`; id em 2 tipos → o primeiro + aviso. **Medido na F0:** 0 ids fora do tema, 0 em
   dois tipos, 1 sem tipo (Programa Nota Legal), nenhum tipo vazio — a partição é exata.
4. Cortesia **1 s** entre chamadas (AuliBot); 1 + 22 + 81 = 104 chamadas ≈ 2 min.

### 2.1 Respostas truncadas no meio de um comentário do Word

Três respostas (Cadastro Fiscal 1687, ITCD 363 e 31800) têm exatamente **64.171 bytes**: o banco do portal corta o
campo em ~64 KB, e o corte cai no meio de um comentário condicional do Word (`<!-- [if gte mso 9]><xml>…` sem
`-->`). Num parse do fragmento inteiro, o comentário aberto **engole as perguntas seguintes** — no navegador, o tema
ITCD inteiro mostra 13 das 28 (o parser do HTML5 faz o mesmo). Cortado por pergunta, o comentário fica na resposta
dele; o texto útil vem antes do corte (o que se perde é XML do Word).

**Censo de 02/11/2026 (`tipo=0`):**

| `codAssunto` | Tema (`<h5>`) | perguntas | tipos | bytes |
|---|---|---|---|---|
| 24 | Agênci@Net | 18 | 2 | 26.725 |
| 17 | Cadastro Fiscal do DF | 40 | 11 | 522.412 |
| 8 | Certidão | 19 | 3 | 46.713 |
| 19 | Documentos Fiscais do ICMS | 70 | 6 | 131.450 |
| 8256 | Documentos Fiscais do ISS | 13 | 1 | 46.805 |
| 8250 | Escrituração Fiscal Digital - EFD ICMS IPI (SPED) | 12 | 1 | 31.178 |
| 4 | ICMS - Pessoa Física | 2 | 1 | 14.251 |
| 15 | ICMS - Pessoa Jurídica | 54 | 4 | 177.879 |
| 2 | IPTU / TLP | 111 | 14 | 504.991 |
| 3 | IPVA | 56 | 8 | 253.662 |
| 16 | ISS - Pessoa Jurídica | 80 | 3 | 321.866 |
| 6 | ITBI | 78 | 5 | 291.361 |
| 7 | ITCD | 28 | 3 | 247.855 |
| 181 | Livro de Registro de Utilização de Doc. Fiscais e Termo de Ocorrências | 1 | 1 | 1.624 |
| 18 | Malha Fiscal do DF | 27 | 1 | 85.407 |
| 20 | Nota Fiscal Avulsa | 11 | 2 | 99.053 |
| 8252 | Orgaos do GDF | 3 | 1 | 39.460 |
| 10 | Pagamento / Parcelamento / Emissão de Guias | 47 | 3 | 144.693 |
| 61 | Precatório | 15 | 1 | 39.565 |
| 1 | Programa Nota Legal | 58 | 3 | 844.001 |
| 4250 | Restituição | 1 | 1 | 2.918 |
| 81 | Simples Nacional | 156 | 6 | 532.595 |
| | **Total** | **900** | **81** | ~4,4 MB |

(O scraper lê temas e tipos do DOM, não desta tabela.) Bytes altos = respostas coladas do Word (`MsoNormal`, CSS
`mso-`, comentários condicionais).

**Medido na F0 sobre as 900 respostas:**

- **Perguntas repetidas:** 50 textos aparecem em mais de um id (55 ocorrências a mais) — 20 com a mesma resposta, 30
  com respostas diferentes. Os ids são distintos e todos entram; o link leva o id (D-DFFAQ-4).
- **Lixo do Word:** 33 respostas com `<style>`, 72 com comentários, 291 com CSS `mso-`. O `html_to_text` dos serviços
  (só tira as tags) deixaria vazar `Normal 0 21 false false false PT-BR X-NONE … /* Style Definitions */
  table.MsoNormalTable {mso-style-name:…}` em 33 delas.
- **Links:** 622 respostas têm `<a href>` — 1.392 absolutos, 1 `mailto`, nenhum relativo à raiz, e 7 lixos: caminho de
  arquivo do Outlook (`file:///C:/Users/…/INetCache/Content.Outlook/…`) e um `href` com um parágrafo inteiro no lugar
  da URL. Nenhuma imagem.
- **Prefixo "R.":** 723 começam com `R.`; há `R:`, `R .`, `R.:`, `R.` colado (`R.Basta`), `R.` duas vezes e um `R`
  sem pontuação.
- **Tipos ≠ classes dos serviços:** nenhum dos 81 rótulos é uma das 139 classes do `servicos` do DF (e só 9 são
  título de serviço). São a taxonomia própria do Atendimento Virtual.

## 3. Receita de coleta (B)

`GET https://iss.fazenda.df.gov.br/online/Default/PerguntasFrequentes.aspx` pelo `ureq` (F0: 200; o host não tem o WAF
da Receita), AuliBot. Parser: os elementos `h5`, `div.accordionHeader` e `div.accordionContent` em ordem de documento;
cada `<h5>` (fora de uma resposta) abre uma **seção**, e os pares seguintes pertencem a ela.

Seções em 02/11/2026 (pares): REFORMA TRIBUTÁRIA – INFORMAÇÕES GERAIS SOBRE NFS-e (7 — comunicados e rótulos
informativos, todos com conteúdo, entram como FAQ), Novo sistema de gestão de ISS do Distrito Federal - NFS-e (39),
Prestador de Serviços (25), Tomador de Serviços (10), Módulos Especiais (7), Declaração Eletrônica de Serviços de
Instituições Financeiras (DES-IF) (7); "Não encontrou o que procurava?" é rodapé (contato), sem pares. Nenhuma
resposta vazia.

- **Sem identidade estável:** os GUIDs dos itens (`ncToggle('<guid>', …)`) **mudam a cada carga** da página. A
  identidade é (seção, pergunta).
- **Repetições:** 20 textos de pergunta repetidos — 18 entre seções (15 Novo sistema ↔ Prestador, 2 Novo sistema ↔
  Tomador, 1 Prestador ↔ Tomador; 15 com a mesma resposta, 3 com respostas diferentes) e **2 dentro da mesma seção** (Novo sistema; mesma resposta, uma só com outras
  quebras de linha). Entre seções, todas entram (o link leva a seção); dentro da mesma seção, sai uma vez, e resposta
  diferente de verdade (comparada sem o espaço) é `Err`.

## 4. Modelagem → `FaqRaw` e árvore

| `FaqRaw` | (A) Receita | (B) ISS |
|---|---|---|
| `pergunta` | texto do `<a>`, numa linha | `.accordionHeader` numa linha (o ícone `nc-plus` é um `<i>` vazio) |
| `resposta` | HTML → texto (D-DFFAQ-7) **+ sem o prefixo `R.`** e variantes | HTML → texto (D-DFFAQ-7) |
| `origin` | `"Receita-DF \| <Tema> \| <Tipo>"` ou `"Receita-DF \| <Tema>"` | `"ISS/NFS-e \| <Seção>"` |
| `url` | `B/perguntasfrequentes.cfm#collapse<id>` (D-DFFAQ-4) | `…/PerguntasFrequentes.aspx#<slug da seção>` |

Identidade: (A) `id` numérico (`collapse209` → 209); (B) (seção, pergunta). Guardas (A): ≥ 18 temas, `id` único no
total (`Err`), card sem exatamente um `div.card` ou sem a resposta (`Err`), pergunta vazia (`Err`), partição dos tipos
(§2.3), **piso 800** (`Err`); (B): pergunta vazia (`Err`), resposta vazia (aviso + descarte), repetição divergente na
seção (`Err`), **piso 80**. Fail-closed antes de gravar.

`df-faqs-tree.json`: uma raiz "Perguntas frequentes" (a aba começa pelos filhos dela) → `Receita-DF` → 22 temas →
tipos (folhas; a pergunta sem tipo fica nos itens do tema) e `ISS/NFS-e` → 6 seções; os filhos na ordem do portal.

## 5. Decisões (D-DFFAQ-*)

- **D-DFFAQ-1** — Duas fontes num só subcomando `faqs`, snapshot único `df-faqs-snapshot.json`; árvore com uma raiz e
  as duas fontes como filhos.
- **D-DFFAQ-2** — Transporte (A) via `curl` (D-DF-WAF, host idêntico). O kit ganha **`post_form_via_curl(url, ua, form,
  opts)`** (`--data-urlencode nome=valor` por campo), e o **UA passa a ser parâmetro** dos dois caminhos curl — o
  `get_via_curl` tinha o UA de navegador fixo; os 5 call sites antigos passam o `kit::USER_AGENT` explícito, com o
  motivo (D-FROTA-UA). Não há timeout no caminho curl (nem antes). (B) pelo `ureq` (F0).
- **D-DFFAQ-3** — Trilha de 3 níveis via POST por tipo (81 POSTs a mais, ~1,5 min). **Revista na F0:** a justificativa
  original ("o rótulo do tipo é a subcategoria da Carta, a mesma `classe` do `servicos`") não se confirmou — 0 de 81.
  O nível fica pela navegação: IPTU tem 111 perguntas em 14 tipos, Simples 156 em 6. A partição é exata (§2.3).
- **D-DFFAQ-4** — `url` da (A) = a casca **com o id no fragmento** (`#collapse<id>`): a app não tem endereço por
  pergunta, e com o link único as 55 perguntas de texto repetido sobrescreveriam outras em silêncio (o nome do `.md`
  sai de link + pergunta). O fragmento não rola a página (as perguntas chegam por POST), mas não atrapalha. (B):
  `#<slug da seção>`, pelo mesmo motivo entre seções.
- **D-DFFAQ-5** — ISS entra como FAQ da SEFAZ-DF; `origin` prefixado `ISS/NFS-e`; plataforma terceirizada não aparece
  no `.md`, só o link oficial.
- **D-DFFAQ-6** — Nota Legal (site) fora: 403 ao AuliBot e tema já coberto (58 perguntas na (A)). Sem pendência.
- **D-DFFAQ-7** — **Revista na F0:** o `html_to_text` dos serviços não serve (vaza o CSS e o XML do Word em 33
  respostas e apaga os links de 622). As FAQs usam o conversor das FAQs do SP e do GO (`<style>`, `<script>` e
  comentários saem inteiros; link → `texto "url"`, ou só `"url"` quando o texto é a própria URL; bloco → linha;
  entidades no fim), com um filtro a mais: só `http(s)` e `mailto` entram no texto. O strip do `R.` é função própria
  (todas as variantes do §2, só no começo).
- **D-DFFAQ-8** — `AuliBot` (no `curl -A` também — agora é parâmetro), cortesia **1 s** (era 300 ms no plano, como
  os serviços; 1 s é a prática do AuliBot na frota), rede sempre, cache só com `--usecache` (chave = URL + os campos
  do POST).
- **D-DFFAQ-9** — `faqs` + `all` no `auli-scraper-df`; default segue `servicos`.
- **D-DFFAQ-10** — Ingerir como está: as perguntas de texto repetido entram todas (ids distintos na (A), seções
  distintas na (B)); só a repetição dentro da mesma seção do ISS sai uma vez.

## 6. Plano — executado (§8)

- **F0** — feita (§7): robots do ISS, `ureq` × curl nos dois hosts, censo dos tipos e partição completa, repetições,
  Word, links, prefixos, tipos × classes.
- **F1 parsers puros + fixtures reais**, **F2 transporte**, **F3 pipeline**, **F4 registro** — §8.

## 7. Evidência

Sondagem r1–r3 (máquina do usuário, `curl -A AuliBot`):

```text
GET robots.txt                         → User-agent: * / Allow: / / Sitemap
GET perguntasfrequentes.cfm            → 200, 617.377 B; 22 formBusca<A>; 0 árvore JS; 115 "collapse"
POST perguntasFreqBusca_div.cfm tipo=0 codAssunto=24 → 200, 26.725 B, 18 cards
   <a href="#" data-toggle="collapse" data-target="#collapse209" …>O que é Agênci@net?</a>
   <div class="card-body collapse border" data-parent="#child1" id="collapse209"><div …><b>R.</b> É uma agência…</div>
POST … tipo=80 codAssunto=24           → 200, 10.481 B (subconjunto)
22 POSTs tipo=0                        → 900 perguntas (tabela §2)
POST perguntasFreqBuscaGeral_div.cfm vazio → 4.744 B (inútil)
GET iss…/PerguntasFrequentes.aspx      → 200, 121.448 B; 6 <h5>; 95 accordionHeader/accordionContent
GET notalegal.df.gov.br                → 403, 5.467 B
```

F0 (02/11/2026, AuliBot, 1 s entre chamadas):

```text
GET iss…/robots.txt                    → 404 (página de erro do IIS)
GET perguntasfrequentes.cfm (curl)     → 200, 617.381 B; 22 temas, 81 tipos
22 POSTs tipo=0 + 81 POSTs por tipo    → todos 200; 900 ids únicos; partição: 0 fora, 0 em 2 tipos, 1 sem tipo
ureq → iss…/PerguntasFrequentes.aspx   → 200, 121.448 B
ureq → receita…/perguntasfrequentes.cfm → Connection reset by peer (WAF JA3, como os serviços)
GET iss… duas vezes                    → os GUIDs dos 95 itens mudam entre as cargas
```

Fontes: `docs/REGISTRO-scrapers.md#df` e `#df-faqs`, `auli-scraper-df/src/df.rs` (D-DF-WAF) e `faqs.rs`.

## 8. Implementação (02/11/2026)

- **`auli-scraper-df/src/faqs.rs`** (`auli-scraper-df faqs`, ou `all`): casca → 22 POSTs `tipo=0` + 81 por tipo, pelo
  curl; página do ISS pelo `ureq`. Corte do fragmento por pergunta (§2.1), partição dos tipos, prefixo `R.`, conversor
  do §5, árvore na ordem do portal, guardas do §4. Pisos fora dos parsers (no `run`), para os testes rodarem sobre
  recortes.
- **Kit:** `kit::http::post_form_via_curl` e o UA como parâmetro de `get_via_curl` (D-DFFAQ-2); a montagem dos
  argumentos do curl virou função pura, com testes dos argumentos exatos.
- **Primeira rodada:** a guarda de partição barrou o ITCD — era o comentário truncado do §2.1 engolindo 15 perguntas
  num parse único. Segunda: a guarda do ISS barrou uma repetição dentro da seção que difere só nas quebras de linha
  → a comparação passou a ignorar o espaço.
- **Rodada de 02/11/2026:** **993 FAQs** (900 da Receita em 22 temas + 93 do ISS em 6 seções); 0 respostas vazias,
  0 com lixo do Word, 0 com `file:///`, 0 começando com "R"; 626 com links; nenhum par (link, pergunta) repetido →
  993 `.md`. `--usecache` reproduz a árvore byte a byte e o snapshot a menos do `coletado_em`.
- **Testes:** 15 em `faqs.rs` (fixtures reais: casca com 2 temas, tema 24 inteiro e os seus 2 tipos, a pergunta 408
  colada do Word, 2 seções do ISS) + 2 no kit. Mutações pegas: parse único sem o corte, sem o strip do `R.`,
  comentário mantido, qualquer `href` no texto, repetição comparada sem normalizar, sem a guarda de partição, tipo
  repetido ficando com o último (o teste achou esse bug na primeira versão), link sem o id, URL duplicada quando o
  texto é a URL.
- **Smoke** (`/v1/retrieve`, instância temporária): certidão por terceiros, NFS-e no padrão nacional, inscrição
  temporária de construtora de outra UF e área construída do IPTU → a FAQ certa em 1º; isenção de IPVA para pessoa
  com deficiência → em 1º a do ICMS de veículo para PcD, com a de concessão da isenção do IPVA em 3º (a FAQ geral
  "Quem possui direito à isenção do IPVA" ficou fora do top 3) — 4 de 5.
- **Atualização:** `scripts/tools/update-df-faqs.sh` (apaga o cache, guarda de encolhimento contra o `.anterior`),
  testado de ponta a ponta (993 = 993; 962 vetores reaproveitados).
