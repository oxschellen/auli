# ESTUDO-MS-Faqs — Perguntas Frequentes da SEFAZ-MS (coleção `faqs` da entidade `ms`)

**Data:** 2026-11-02 · **Estado:** **implementado** (§8) — a sondagem da §5 rodou do ambiente de análise, que o
portal não bloqueia. · **Órgão:** Secretaria de Estado de Fazenda de Mato Grosso do Sul (SEFAZ-MS). · **6ª entidade
com FAQs** (RS 1.990 · SC 1.306 · PR 1.321 · SP 2.665 · MG 310 · **MS 1.200**). · A `ms` tinha só `servicos`
(WordPress, 282 links, sem descrição — D-MS4).

> **TL;DR — a FAQ da SEFAZ-MS mora no Fale Conosco, não no site da SEFAZ.** O site institucional
> (`www.sefaz.ms.gov.br`, WordPress, tema `new-ms`) não tem FAQ própria — as páginas de DF-e (NF-e, NFC-e, CT-e,
> NFCom, DC-e…) linkam "Perguntas Frequentes – SEFAZ MS" para **`https://www.faleconosco.ms.gov.br/faq/#/assunto/<N>`**.
> O Fale Conosco é uma aplicação **Java/JSF** (`/faleconosco/ouvidoria/ouvidoria.jsf`, `;jsessionid=`) cuja página de
> ouvidoria lista **70 assuntos** (ids 1–85 com buracos: NF-e, CT-e, MDF-e, EFD, IPVA, ITCD, Simples, ~20 de
> Substituição Tributária por segmento, Cadastro, Certidões, Restituição, e-Fazenda, Regularize Já…). A FAQ em si é
> um **SPA com hash routing** (`/faq/#/assunto/N`): o HTML é casca e as perguntas vêm de uma **API JSON** do mesmo
> app, ainda não identificada. **Três comandos na máquina do usuário fecham** (§5). Hipótese de custo: 1 GET por
> assunto (≈ 70) sem JS, se a API for aberta como o resto do Fale Conosco.

---

## 1. Hierarquia discovery-first — o que foi avaliado

| Rota | Veredito | Evidência (02/11/2026) |
|---|---|---|
| `www.sefaz.ms.gov.br` (WordPress) — páginas de DF-e, "Fale Conosco", "Serviços em destaque" | **sem FAQ inline**; só o link externo "Perguntas Frequentes – SEFAZ MS" → `faleconosco.ms.gov.br/faq/#/assunto/7` (NFC-e) | WebFetch |
| `www.sefaz.ms.gov.br/wp-content/uploads/…/Pergunas-e-Respostas-DeSTDA.pdf` | FAQ **em PDF** (DeSTDA/SEDIF-SN, 2016) — fora do v1, como o EFD do MG | WebSearch |
| `www.faleconosco.ms.gov.br/faleconosco/ouvidoria/ouvidoria.jsf` | **JSF server-side, público**, lista os **70 assuntos** com links `faq/#/assunto/N` — é o catálogo (§2) | WebFetch |
| `www.faleconosco.ms.gov.br/faq/` | **SPA** (hash routing) — não alcançável do ambiente de análise; o endpoint de dados é o que falta | — |
| Headless | não esperado: o SPA chama uma API; o resto do app é Java sem WAF visível | — |

## 2. Catálogo de assuntos (70, do `ouvidoria.jsf`, 02/11/2026)

```
1 NF Eletrônica · 2 CTe · 3 MDFe · 4 EFD · 6 Ouvidoria · 7 NFCe · 8 CFeECF · 9 SIMPLESNAC · 10 Outros · 11 ITCD ·
12 REGIMES · 13 IPVA · 14 GARANTIDO · 15 ST MEDICAMENTO/COSME · 16 PRODRURAL · 17 CARNES · 18 CEREAIS · 19 LATICINIOS ·
20 ENERGIA ELÉTRICA · 21 STCOMBUSTIV · 22 DIFCON · 23 ST ALIMENTOS · 24 ST BEBIDAS · 25 ST CIGARROS · 26 ST CONSTRUÇÃO ·
27 ST VEÍCULOS · 28 ST AUTOPEÇAS · 29 AGENFA FAZENDÁRIA · 30 NFPe · 31 NFAe · 33 COMÉRCIO EXTERIOR · 35 ST DISTRIBUIDORA ·
36 ST - Combustível - B · 37 ST–Posto Combustível · 38 ST-USINAS · 39 ST LUBRIFICANTES · 40 ST-Combustível - GLP ·
41 ST-Combustível - TRR · 42 ST-Combustível-SCANC · 43 ST-Combus-Refinarias · 44 ST-Comb-Gás Natural · 45 CT-e OS ·
46 CADASTRO FISCAL · 53 TRANSPORTE · 54 TELECOM · 55 PAUTA FISCAL · 56 MERC. EM TRÂNSITO · 57 ST MATERIAIS LIMPEZA ·
58 ST IMPORT COMBUSTÍVE · 60 NFP-SE · 61 ST INFORMÁTICA · 62 ICMS Equalização SN · 63 CREFIR · 64 FISCALIZAÇÃO varejo ·
65 DAEMS/GNRE · 66 TVF/TA · 67 Nota MS Premiada · 68 PROACAP · 69 Revisão Cobrança ICMS Equalização e Sub Tributária ·
71 CORONAVÍRUS · 72 Nota Fiscal de Energia Elétrica Eletrônica · 73 CERTIDÕES E PARCELAMENTO · 74 RESTITUIÇÃO ·
75 BENEFÍCIO FISCAL · 76 CADASTRO - OUTRAS UF's · 78 INCENTIVOS FISCAIS - CIDEC · 79 Plataforma e-Fazenda ·
80 NFCom · 81 REGULARIZE JÁ · 85 DC-e
```

Observações: nomes **truncados a ~20 chars** pela fonte (`ST MEDICAMENTO/COSME`, `ST IMPORT COMBUSTÍVE`) — é o rótulo
do sistema de atendimento, ingerir como está (D-MSFAQ-9); a API da FAQ pode trazer o nome completo. Os ids com buraco
(5, 32, 34, 47–52, 59, 70, 77, 82–84) são assuntos desativados. **A enumeração deve vir da página JSF (ou da API, se
ela listar assuntos), nunca de lista no código.** Alguns assuntos são de atendimento interno/ouvidoria (6 Ouvidoria,
71 Coronavírus) — podem vir vazios; nada a filtrar a priori.

## 3. Modelagem prevista → `FaqRaw` (a confirmar na §5)

| `FaqRaw` | origem provável |
|---|---|
| `pergunta` | campo da pergunta do item da API |
| `resposta` | campo da resposta (HTML ou texto) → `html_para_texto` (molde do PR: lista fechada + `table/tr/td/th`) |
| `origin` | `"<Assunto>"` (um nível; se a API tiver sub-assunto, `"Assunto \| Sub"`) |
| `url` | `https://www.faleconosco.ms.gov.br/faq/#/assunto/<N>` + âncora/`id` da pergunta se o SPA tiver rota por pergunta (`#/assunto/N/pergunta/M`?) — ver §5 |

Identidade = id da pergunta na API (ou `(assunto, posição)` se não houver). Guarda natural: se a API devolver
`total`/contagem por assunto, `únicos == total`; se não, `Σ por assunto == únicos` + `assuntos da página JSF ==
assuntos consultados` (nenhum link do catálogo sem resposta da API).

## 4. Decisões propostas (D-MSFAQ-*) — a ratificar após a §5

- **D-MSFAQ-1** — Fonte = API JSON do SPA `faq`, uma chamada por assunto (ou uma só, se listar tudo); enumeração dos
  assuntos pela `ouvidoria.jsf` (HTML, público, sem sessão) ou pelo endpoint de assuntos da API, se existir.
- **D-MSFAQ-2** — Link oficial = `faq/#/assunto/N` (+ rota por pergunta se o SPA tiver; senão limitação registrada).
- **D-MSFAQ-3** — Guardas (`Err` antes de gravar): catálogo de assuntos vazio; assunto do catálogo que a API não
  conhece (404/erro); pergunta sem texto; zero no total; `total` anunciado ≠ únicos (se houver). Avisos: assunto com
  0 perguntas (esperado em alguns), resposta vazia, tag fora da lista fechada.
- **D-MSFAQ-4** — `UA = AuliBot`, cortesia 1 s, rede sempre, cache só com `--usecache` (padrão das FAQs).
- **D-MSFAQ-5** — Subcomando `faqs` + `all` no `auli-scraper-ms`; default segue `servicos`.
- **D-MSFAQ-6** — `jsessionid`: o JSF emite cookie de sessão; a API da FAQ pode exigir o cookie (um GET no `/faq/`
  antes). Cookie jar por rodada, como no SP-KB do MG.
- **D-MSFAQ-7** — O PDF DeSTDA fica fora (pendência, como o EFD-PDF do MG).
- **D-MSFAQ-8** — Árvore do frontend: raiz → `Faq` por assunto (um nível), ou dois se houver sub-assunto.
- **D-MSFAQ-9** — Ingerir como está (rótulos truncados, caixa alta, typos como `LATICINIOS`).

## 5. Sondagem a rodar na máquina do usuário (fecha a descoberta)

```bash
cd /tmp; F=https://www.faleconosco.ms.gov.br; UA="AuliBot/0.1 (+https://github.com/oxschellen/auli)"
# 1) a casca do SPA: scripts e qualquer URL de API embutida
curl -s -c cj.txt -A "$UA" "$F/faq/" -o faq.html -w 'faq/: %{http_code} %{size_download}B\n'
grep -oE 'src="[^"]+\.js[^"]*"' faq.html | sort -u
grep -oiE '(https?:)?/[A-Za-z0-9_./-]*(api|rest|service|faq|assunto|pergunta)[A-Za-z0-9_./-]*' faq.html | sort -u | head -20
# 2) os bundles JS: onde está a chamada (base da API + rotas)
for s in $(grep -oE 'src="[^"]+\.js[^"]*"' faq.html | sed -E 's/src="([^"]+)"/\1/'); do u="$s"; case "$u" in http*) ;; /*) u="$F$u";; *) u="$F/faq/$u";; esac; echo "== $u"; curl -s -A "$UA" "$u" | grep -oiE '"(https?://[^"]+|/[A-Za-z0-9_./-]*(api|rest|faq|assunto|pergunta)[^"]*)"|(get|post|\$http\.(get|post)|fetch|axios\.(get|post))\([^)]{3,100}' | sort | uniq -c | sort -rn | head -20; done
# 3) teste direto das hipóteses mais prováveis (ajustar com o que o passo 2 mostrar)
for u in "$F/faq/api/assunto/7" "$F/faq/api/assuntos" "$F/faq/rest/assunto/7/perguntas" "$F/faleconosco/rest/faq/assunto/7" "$F/faleconosco/api/faq/assunto/7"; do printf "%-60s " "$u"; curl -s -b cj.txt -A "$UA" -H "Accept: application/json" "$u" -o r.json -w '%{http_code} %{size_download}B '; jq -c 'if type=="array" then "array("+(length|tostring)+")" elif type=="object" then (keys|tostring) else "?" end' r.json 2>/dev/null || echo "não-JSON"; done
```

Com a URL certa: um item de exemplo (`jq '.[0]'`), a contagem por assunto para os 70, e se há `total`/sub-assunto/
id de pergunta. Aí o estudo fecha com shape, totais e as D-MSFAQ ratificadas.

## 6. Plano (após a §5)

F1 parser puro + fixtures (catálogo JSF recortado + JSON real de 2 assuntos; casos `Err`: assunto sem resposta,
pergunta vazia, zero total) → F2 transporte (JSF → ids; 1 GET por assunto; cookie jar; `write_faqs("ms", …)` +
`ms-faqs-tree.json`) → F3 pipeline (`registry.toml` `collections` da `ms` + `faqs`; `gen-frontend-entities`;
`auli-collections ms`; `build-packs.sh ms faqs`; smoke "como emitir guia de IPVA?") → F4 registro
(`REGISTRO-scrapers.md#ms-faqs`, `update-ms-faqs.sh`, README/SCRAPERS.md "FAQs: RS, SC, PR, SP, MG, MS").

## 7. Evidência (02/11/2026)

- `https://www.sefaz.ms.gov.br/documentos-fiscais-eletronicos/nfc-e/` — "Canais de atendimento" → `https://www.faleconosco.ms.gov.br/faq/#/assunto/7`.
- `https://www.faleconosco.ms.gov.br/faleconosco/ouvidoria/ouvidoria.jsf` — JSF, público; 70 links `faq/#/assunto/N` (lista na §2).
- `https://www.sefaz.ms.gov.br/wp-content/uploads/2016/04/Pergunas-e-Respostas-DeSTDA.pdf` — FAQ em PDF (fora).
- `auli-scraper-ms/src/ms.rs` — catálogo WordPress de serviços (D-MS1..6); sem FAQ.

## 8. Sondagem, decisões e implementação (02/11/2026)

A §5 rodou daqui: o Fale Conosco não barra o AuliBot e não tem `robots.txt` (404); o `robots.txt` da SEFAZ-MS
só fecha o `/wp-admin/`. Código em [`faqs.rs`](../../auli-server/crates/scrapers/auli-scraper-ms/src/faqs.rs).

### 8.1 O que a API mostrou

| Hipótese (§1–§6) | Medido |
|---|---|
| 70 assuntos, pela `ouvidoria.jsf` | **71**, pela API. A página JSF omite **BP-e (26)** e **NFGas (1)** e lista um PROACAP que a API não tem: a enumeração vem da API |
| API desconhecida; 1 GET por assunto | AngularJS 1.2.28 sobre **POST `faq/recursos/faq/<ação>`**, corpo `[{key, value}]`, sem sessão nem cookie (a D-MSFAQ-6 cai) |
| Um nível, talvez sub-assunto | **Três níveis**: `listassunto` (com `qtdeFaq`, a contagem do portal) → `listdetalhamentoassunto` (384 subtemas) → `listfaqpordetalhamentoassunto` |
| A resposta vem na lista | A lista traz só uma **prévia** (≤ ~100 caracteres, cortada no meio da tag e do link). A resposta inteira é `getrespostafaq(idFaq)`, uma chamada por pergunta — o que a aplicação faz no clique |
| Rota por pergunta, se houver | Não há: as rotas são `#/assunto/N`, `#/detalhamentoAssunto/N/M` e `#/busca/…` |
| "Site sem FAQ inline" | A página `/ipva/` tem um acordeão "Perguntas Frequentes" com 22 itens (IPVA 2026), **nenhum** igual aos 20 do assunto IPVA do Fale Conosco |

Censo: **1.200 perguntas**, todas com `id` único; nenhuma em dois subtemas; em todos os 71 assuntos as únicas
batem com o `qtdeFaq`; nenhuma vencida (`dataFimVigencia` nula em todas) nem começando no futuro; nenhuma pergunta
ou resposta vazia. As respostas são HTML colado do Word: 43 comentários condicionais (`<!--[if gte mso 9]>`) com
`<xml>` e `<style>` dentro, 23 entidades nomeadas (`&ndash;` 1.577, `&ordm;` 1.231, `&sect;` 347…), 1.575 links,
todos absolutos, nenhuma imagem nem tabela. Duas perguntas aparecem **duas vezes no mesmo subtema** com `id`s e
respostas diferentes (CT-e "Como faço para me credenciar…", EFD "Como efetuar o lançamento do diferencial de
alíquota?") — versões que o portal mostra lado a lado.

### 8.2 Decisões

- **D-MSFAQ-1 (revista):** assuntos pela API (`listassunto`), subtemas e perguntas pelos dois níveis seguintes,
  a resposta por `getrespostafaq`. **1.656 requisições** por rodada (1 + 71 + 384 + 1.200) a 1 s: ~30 min.
- **D-MSFAQ-2 (revista):** link = a página do subtema **com `?idFaq=<id>`**. O AngularJS 1.2 casa a rota só pelo
  caminho e ignora o parâmetro — o link abre o subtema —, e cada pergunta ganha link próprio. É o que separa as
  duas versões repetidas no acervo (o nome do `.md` sai de link + pergunta).
- **D-MSFAQ-3 (revista):** `Err` antes de gravar — lista de assuntos vazia; resposta da API fora do formato
  `{"data": [...]}`; perguntas únicas de um assunto ≠ `qtdeFaq`; acervo vazio; pergunta sem texto. Avisos:
  subtema sem perguntas, resposta vazia.
- **D-MSFAQ-4 (mantida):** AuliBot, 1 s, rede sempre, cache só com `--usecache`.
- **D-MSFAQ-6 (descartada):** a API não pede sessão.
- **D-MSFAQ-8 (revista):** árvore raiz → `Menu` por assunto → `Faq` por subtema; trilha `Assunto | Subtema`.
- **D-MSFAQ-10 (nova) — ordem estável:** a API devolve os assuntos numa ordem própria; a coleta ordena assuntos e
  subtemas pelo nome e as perguntas pelo `id`, para o snapshot só mudar quando a FAQ muda.
- **D-MSFAQ-11 (nova) — HTML → texto pelo parser** (`scraper`/html5ever): decodifica todas as entidades nomeadas,
  que o `decode_entities` do kit não conhece, e descarta os comentários do Word com o `<xml>` e o `<style>` dentro.
  Links como `texto "url"`, bloco → quebra de linha, a convenção da frota.
- **D-MSFAQ-12 (nova) — o acordeão do IPVA no site fica fora desta versão:** 22 itens, conteúdo distinto do Fale
  Conosco; candidato a uma 2ª fonte (uma página WordPress, pela API REST).

### 8.3 Rodada de 02/11/2026

- 1.656 requisições, todas 200; 71 assuntos, 1.200 perguntas, nenhum aviso. `--usecache` reproduz a coleta.
- 1.200 pares (link, pergunta) únicos; nenhuma tag, entidade ou sobra do Word no texto; respostas de 45 a 20.697
  caracteres (mediana 800; 21 passam do teto de 6.000 do vetor).
- 1.200 `.md`; pack `ms-faqs` com 1.200 vetores. Smoke `/v1/retrieve kind=faqs entity=ms`: credenciamento de CT-e,
  assinatura do DTE, rejeição 404 (NF-e e NFC-e), isenção de IPVA e tempo de autorização da NF-e — o alvo em 1º.
- 16 testes no crate (9 das FAQs, com recortes reais); cada guarda e a conversão pelo parser foram desligadas e o
  teste delas falhou.

