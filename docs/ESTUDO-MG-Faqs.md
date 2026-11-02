# ESTUDO-MG-Faqs — Perguntas Frequentes da SEF-MG (coleção `faqs` da entidade `mg`)

**Data:** 2026-11-01 · **Escopo:** descoberta concluída para a fonte B (Portal SPED) e dimensionada para a fonte A
(Base de Conhecimento do CSM), com a enumeração de A pendente (§6). **Implementado:** B em 01/11/2026 (§10), A em
02/11/2026 (§11). Especificação para a 2ª coleção da entidade `mg`
(hoje só `servicos`, 149 itens) — 5ª entidade com FAQs (RS 1.947 · SC 1.306 · PR 1.321 · SP 2.665). · **Órgão:**
Secretaria de Estado de Fazenda de Minas Gerais (SEF-MG).

> **TL;DR — MG não tem um sistema de FAQ; tem dois que valem e um que não.**
> **(B) Portal SPED MG** (`portalsped.fazenda.mg.gov.br/spedmg`, OpenCms, HTML estático): FAQs **numeradas, com
> âncora por pergunta**, em **2 módulos** — **NF-e 87** perguntas/9 seções e **NFC-e 62**/6 — **149 no total**; os
> outros 14 módulos não têm FAQ em HTML (EFD só em PDF). Anatomia simples, link exato de graça, ~17 GETs. **Pronta
> para implementar (v1).**
> **(A) Base de Conhecimento do CSM ServiceNow** (`atendimento2.fazenda.mg.gov.br/csm`): base **"SRE - Dúvidas
> Frequentes" com 81 artigos** em categorias como *DF-e - Correção de Erros / Rejeições*, *Orientações Tributárias
> DOLT/SUTRI*, *Sintegra - PED*. O artigo é público pela page API (`kb_article_view`) — **mas só com UA de navegador**:
> o ServiceNow marca `AuliBot` como `isCrawlerBot` e esconde o conteúdo. Categoria/base exigem login e a busca não
> devolve pela page API ⇒ **enumeração dos 81 ainda não resolvida** (3 rotas a testar, §6). E os artigos são de três
> naturezas (pergunta única / compilação P.R. / orientação longa) — modelagem própria. **Fase 2.**
> **(C) Site institucional** (`www.fazenda.mg.gov.br`, OpenCms): prosa dispersa (PED, Transação Tributária) — fora.
> **Aproveitar a rodada para a pendência D-NAMING** do MG (`SEF/MG` → `SEF-MG`), em commit de dados separado.

---

## 1. Hierarquia discovery-first

| Fonte / rota | Veredito | Evidência (01/11/2026) |
|---|---|---|
| **B** `spedmg/<módulo>/Perguntas-Frequentes/` | **✅ HTML estático server-side** (OpenCms exporta arquivos; `/spedmg/export/sites/...`). Sem API, sem AJAX, sem headless | `curl` dos índices e de `respostas_iii/index.html` (56 KB) |
| B — módulos com FAQ | só **`nfe`** e **`nfce`** (200); `bpe cte cteos dce efd gtve mdfe nf3e nfag nfcom nfeabi nff nfgas` → 404; `efd` tem só PDF (`export/sites/spedmg/efd/downloads/Perguntas-Frequentes-EFD-v.2024.01.pdf`) | laço pelos `href` da home |
| B — robots | `/spedmg/robots.txt` **404** (não existe) | `curl` |
| **A** `/api/now/sp/page?id=sef_mg_kb_home` | **✅ pública**: widget `edx_kb-bases-browse_custom` → `total_kb_count: 2`, `total_articles_count: 81`, bases "Comunicados do Portal do Contribuinte" (0) e **"SRE - Dúvidas Frequentes" (81, `f18a2fc11bbec61029f686afe54bcbed`)**; widgets anônimos com `kb[]` (destaques: `number`, `short_description`, `sys_id`, `kb_category`, `description`) | page API com `X-Portal` |
| A `id=kb_article_view&sysparm_article=KB…` | **✅ pública com UA de navegador** (`edx_article_header` + `edx_article_content` → `kbContentData.data` HTML, `category`, `kbName`, `kbSysId`, `number`, `shortDesc`, `permalink`, `viewCount`, `replacementAlert`, `isArticleExpired`); com `AuliBot` → `isCrawlerBot: true`, `knowledgeExists: false` (conteúdo omitido) | comparação dos dois UAs |
| A `id=kb_category…`, `kb_view2`, `kb_home`, `kb_article&sys_id=` | **login** (`eds_csm_login`) | — |
| A `id=kb_search&q=…` (page API) e `POST /api/now/sp/widget/kb-result` | **vazio** (`total: null`, `n: 0`) nos dois UAs — a busca roda no cliente com estado que a page API não reproduz | — |
| A `/api/now/table/kb_knowledge` | 401 | — |
| **C** `www.fazenda.mg.gov.br` | OpenCms; sem FAQ central; `empresas/ped/duvidas_frequentes/`, `empresas/transacao-tributaria/perguntas-e-respostas` em prosa | WebFetch |

## 2. Fonte B — Portal SPED MG (v1)

### 2.1 Enumeração

Home `https://portalsped.fazenda.mg.gov.br/spedmg/` → `href="/spedmg/<módulo>/"` (16 módulos + `noticias`). Para cada
módulo, `GET /spedmg/<módulo>/Perguntas-Frequentes/`: **200 ⇒ tem FAQ**, 404 ⇒ pular com aviso (D-MGFAQ-4). Nunca
hardcodar `nfe`/`nfce`: a lista vem da home a cada rodada (um módulo novo com FAQ entra sozinho).

### 2.2 Índice — `…/Perguntas-Frequentes/` (ou `index.html`)

Seções em algarismos romanos com título (NF-e: *I. Conceito, uso e obrigatoriedade da NF-e* … *IX. Outras
Informações:*) e, sob cada uma, as perguntas numeradas **com link para a âncora na página de respostas**:

```html
href="/spedmg/nfe/Perguntas-Frequentes/respostas_i/index.html#12"
```

O número da pergunta é **global no módulo** (NF-e 1–87; a seção III vai de 8 a 41). A seção `respostas_<romano>` é a
página; o `#N` é a pergunta.

### 2.3 Respostas — `…/Perguntas-Frequentes/respostas_<romano>/index.html`

```html
<p class="rvps2" style="text-align: justify;"><span class="rvts7"><a name="8"></a><strong>8.&nbsp;</strong></span><strong><span class="rvts7">É possível o envio por lote de NF-e ou a emissão deve ser feita nota a nota?</span></strong></p>
<p style="text-align: justify;"><span class="rvts6">A NF-e é um documento autônomo e a sua emissão deve ser feita nota a nota, …</span></p>
<p class="rvps2" style="text-align: justify;"><span class="rvts6">O processo de transmissão da NF-e, no entanto, deve ser realizado em lotes. …</span></p>
<p class="rvps2" style="text-align: justify;"><span class="rvts6">&nbsp;</span></p>
<p class="rvps2" style="text-align: justify;"><strong><span class="rvts7"><a name="9"></a>9.&nbsp;</span><span class="rvts7">Se alguma NF-e for rejeitada, todo o lote será rejeitado também?</span></strong></p>
…
<p class="rvps8" style="text-align: justify;"><span class="rvts6">• Correção, cancelamento e inutilização de NF-e</span></p>
```

- **Pergunta** = o `<p>` que contém `<a name="N">` (N numérico); texto = o `<p>` inteiro sem tags, sem o prefixo
  `N.`/`N)` e sem `&nbsp;`. A posição do `<strong>` varia (ora envolve o `<a>`, ora só o texto) — não depender dela.
- **Resposta** = os `<p>` seguintes até o próximo `<a name="<numérico>">` ou o fim do conteúdo, descartando `<p>`
  vazios (`&nbsp;`) e os **marcadores `• …`** (`class="rvps8"`), que são **sub-tópicos** dentro da seção (3º nível
  da trilha, D-MGFAQ-3). Links `<a href>` dentro da resposta → `texto "url"` (convenção da frota).
- Âncoras **não numéricas** (`<a name="emissao">`) são ignoradas.
- O HTML é exportado do Word (`rvps*`/`rvts*`): só `<p>`, `<span>`, `<strong>`, `<a>`, `<br>`, eventualmente
  `<table>` — a lista fechada do PR cobre; adicionar `table/tr/td/th` à lista (célula → espaço, como no SP).
- Há `<h2>`/`<h4>` de template da página (título, modal) **fora** do conteúdo — o parser começa na primeira âncora
  numérica e termina no fim do bloco de conteúdo (ou no rodapé; medir onde o conteúdo acaba na fixture).

### 2.4 Modelagem → `FaqRaw`

| `FaqRaw` | origem |
|---|---|
| `pergunta` | texto do `<p>` da âncora, sem `N.` |
| `resposta` | `html_para_texto` dos `<p>` seguintes |
| `origin` | `"<Módulo> \| <Seção> \| <Sub-tópico>"` — ex.: `NF-e - Nota Fiscal Eletrônica \| III. Modelo Operacional - O que muda com a NF-e \| Correção, cancelamento e inutilização de NF-e`; sem sub-tópico, 2 níveis. Nome do módulo = título da página do módulo/índice (`<title>`/`<h1>`), não a sigla da URL. Título da seção = o do índice, com o romano. |
| `url` | `https://portalsped.fazenda.mg.gov.br/spedmg/<módulo>/Perguntas-Frequentes/respostas_<romano>/index.html#<N>` — **exato, do próprio portal** |

Identidade = `(módulo, N)`. **Guardas** (D-MGFAQ-5): (a) por seção, `perguntas do índice == perguntas parseadas da
página de respostas` e **mesmos números** (o índice anuncia; a página entrega — "únicos == total anunciado"); (b)
texto da pergunta no índice ≈ texto na página (igualdade após `clean`; divergência → aviso com os dois, não erro — o
índice pode estar abreviado); (c) pergunta sem texto `Err`; (d) módulo com 200 e zero perguntas `Err`; (e) zero no
total `Err`. Resposta vazia e tag sobrando: aviso.

### 2.5 Árvore do frontend (`mg-faqs-tree.json`)

Raiz `Menu` ("Perguntas frequentes — Portal SPED MG", home do SPED) → `Menu` por módulo (NF-e, NFC-e) → `Menu` por
seção (`url` = página `respostas_<romano>`) → `Faq` por sub-tópico (ou direto `Faq` na seção quando não há
sub-tópico), `faq_items` com `url` próprio (`#N`). Três ou quatro níveis — o `faqtree` aceita.

## 3. Fonte A — Base de Conhecimento do CSM (fase 2)

### 3.1 O que está pronto

- Inventário: `sef_mg_kb_home` → 81 artigos na base `SRE - Dúvidas Frequentes`. Guarda natural:
  `coletados == total_articles_count` (dinâmica, do próprio portal).
- Conteúdo: `kb_article_view&sysparm_article=<number>` → `kbContentData.data` (HTML), `category`, `kbName`,
  `number`, `shortDesc`, `permalink`, `replacementAlert` ("foi desabilitado e substituído pelo artigo atual" — sinal de
  versão), `isArticleExpired`. **Mesmos widgets `edx_article_*` que o `mg.rs` já parseia** para serviços → reuso do
  conversor de artigo.
- Link oficial: `https://atendimento2.fazenda.mg.gov.br/csm?id=kb_article_view&sysparm_article=<number>`.

### 3.2 Os dois nós

**(i) Enumeração.** Categoria/base pedem login; busca não responde pela page API. Rotas a testar (§6):
`GET /api/now/sp/search?q=…&t=kb_knowledge` (API do typeahead); `GET /api/sn_km_api/knowledge/articles?query=…&kb=…`
(Knowledge API, às vezes anônima); **BFS pelo `kb_related_articles`** a partir dos destaques da home, com a guarda
`alcançados == 81` (funciona se o grafo for conexo — a rodada mostra). Último recurso, **a descartar**: varrer
`KB00xxxxx` por faixa (é "varrer ids", contra a doutrina da frota).

**(ii) Modelagem.** Três naturezas de artigo: **pergunta única** ("Rejeição 215: Falha no schema XML." → `FaqRaw`
direto: `shortDesc` = pergunta, corpo = resposta, `origin = SRE - Dúvidas Frequentes | <category>`); **compilação
P./R.** ("PED - Dúvidas Frequentes": tabela `<td>P.</td><td>1) pergunta</td>` / `<td>R.</td><td>resposta</td>` →
**explodir em N `FaqRaw`**, `url` = artigo + `#<n>`? não há âncora — limitação; identidade `(number, n)`);
**orientação longa** ("ICMS - Simples Nacional - 002/2015", DOLT/SUTRI → é orientação tributária, não FAQ; candidata
a **outra coleção** ou a ficar fora). Decisão por `kb_category` (lista fechada no código, com teste):
*DF-e - Correção de Erros / Rejeições* e *Sintegra - PED* entram; *Orientações Tributárias DOLT/SUTRI* fica fora
(ou vira pendência de coleção própria).

**(iii) Doutrina de UA (levar ao usuário).** O conteúdo público só vem com UA de navegador; `AuliBot` é tratado como
robô e recebe `knowledgeExists: false`. Precedente na frota: DF/GO (`get_via_curl` por WAF JA3). Proposta:
D-MGFAQ-8 — UA de navegador **só neste módulo e só para o host `atendimento2.`**, com a cortesia padrão (1 s) e o
motivo registrado no código e no REGISTRO; a identidade `AuliBot` continua em todo o resto (inclusive no SPED).

## 4. Fonte C — fora

Prosa sem identidade por pergunta. Registrar como pendência de **documentos** (não FAQ): `ped/duvidas_frequentes/` pode
até já estar coberto pelo KB0011140 do CSM.

## 5. Decisões (D-MGFAQ-*)

- **D-MGFAQ-1 — v1 = B (SPED, 149 perguntas); A = fase 2** depois de resolver a enumeração e ratificar a modelagem
  (§3.2) e a UA (§3.2 iii). C fora.
- **D-MGFAQ-2 — Link (B)** = página de respostas + `#N` (exato, do portal). (A) = `csm?id=kb_article_view&sysparm_article=<number>`.
- **D-MGFAQ-3 — Trilha (B)** = `Módulo | Seção | Sub-tópico` (3 níveis quando há `•`; 2 quando não).
- **D-MGFAQ-4 — Enumeração (B)** pela home do SPED; módulo sem `Perguntas-Frequentes/` → aviso e pula; FAQ só em PDF
  (EFD) → aviso "fora: PDF" e pendência.
- **D-MGFAQ-5 — Guardas (B)**: §2.4.
- **D-MGFAQ-6 — `html_para_texto`** do PR como molde (lista fechada + `table/tr/td/th`), local ao crate.
- **D-MGFAQ-7 — Subcomando `faqs` + `all`** no `auli-scraper-mg`; default segue `servicos`.
- **D-MGFAQ-8 — UA**: `AuliBot` + 1 s no SPED; para o CSM (fase 2), UA de navegador restrito ao host, se o usuário
  ratificar.
- **D-MGFAQ-9 — D-NAMING na mesma rodada**: `SEF/MG` → `SEF-MG` em `mg.rs:222`, recoleta de serviços verificada,
  **commit de dados separado** do das FAQs (a pendência pede "a próxima vez que o MG for tocado").
- **D-MGFAQ-10 — Rede sempre; cache só com `--usecache`** (padrão SC/PR/SP). Censo completo a cada rodada (~17 GETs).
- **D-MGFAQ-11 — Ingerir como está**: títulos de seção com `:` final, numeração `N.`/`N)` normalizada só para extrair
  o número (o texto da pergunta sai sem o prefixo).

## 6. Sondagem pendente (só para a fase 2 — A)

```bash
cd /tmp; B=https://atendimento2.fazenda.mg.gov.br; PID=89275a53cb13020000f8d856634c9c51; KB=f18a2fc11bbec61029f686afe54bcbed
UA="Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0 Safari/537.36"
H=(-H "Accept: application/json" -H "X-Portal: $PID" -H "X-Requested-With: XMLHttpRequest" -A "$UA")
# 1) Service Portal search API (a do typeahead)
curl -s "${H[@]}" "$B/api/now/sp/search?q=icms&t=kb_knowledge&portal_id=$PID&limit=100" | jq -c '{keys: keys, n: (.result.results|length?), total: .result.count}' | head -c 400; echo
# 2) Knowledge Management API
curl -s -o km.json -w 'km api: %{http_code}\n' "${H[@]}" "$B/api/sn_km_api/knowledge/articles?query=icms&limit=100&kb=$KB"; jq -c '{count: .result.meta.count, n: (.result.articles|length?)}' km.json 2>/dev/null; echo
# 3) grafo de relacionados (base do BFS com guarda = 81)
curl -s "${H[@]}" "$B/api/now/sp/page?id=kb_article_view&sysparm_article=KB0011056&portal_id=$PID" | jq -c '.result.containers | .. | objects | select(.widget.id=="kb_related_articles") | .widget.data | {keys: keys}' | head -c 400; echo
```

## 7. Plano de implementação (v1 = B)

**F1 — parser puro + fixtures** (`auli-scraper-mg/src/faqs.rs`): `modulos_da_home(html)`, `parse_indice(html) ->
Vec<Secao{romano, titulo, perguntas: Vec<(N, texto, pagina)>}>`, `parse_respostas(html) -> Vec<Pergunta{N, texto,
resposta_html, subtopico}>`, `casar_indice_respostas` (guarda a), `html_para_texto`, `montar_faq_raw`, `montar_arvore`.
Fixtures: índice NF-e recortado (2 seções) + `respostas_iii` recortada (3 perguntas + 1 marcador `•` + 1 âncora não
numérica `emissao`). Casos `Err`: índice ≠ respostas (número faltando), pergunta vazia, zero total. Mutação em cada guarda.
**F2 — transporte**: `build_agent(AuliBot)`, cortesia 1 s, home → módulos → índice → páginas de respostas (distintas
por módulo: `sort -u` dos `respostas_<romano>` do índice); `write_faqs("mg", …)` + `mg-faqs-tree.json` atômico;
subcomando. → rodada real: NF-e 87, NFC-e 62, total 149, 14 módulos "sem FAQ", EFD "só PDF". **Portão humano:** abrir
3 `.md` (uma com sub-tópico, uma com link na resposta) e comparar com o portal.
**F3 — pipeline**: `registry.toml` (`collections` da `mg` + `faqs`) → `gen-frontend-entities` → `auli-collections mg`
→ `build-packs.sh mg faqs` → smoke `/v1/retrieve kind=faqs entity=mg`: "posso enviar NF-e em lote?" → NF-e #8.
Conferir `config/prompts/mg.txt` (fala em FAQs?).
**F4 — registro**: `REGISTRO-scrapers.md#mg-faqs` (+ a seção A como "dimensionada, enumeração pendente"),
`update-mg-faqs.sh`, README/SCRAPERS.md ("FAQs: RS, SC, PR, SP, MG"). **Commit à parte:** D-NAMING `SEF/MG` →
`SEF-MG` + recoleta de serviços.

## 8. Critérios de aceitação (v1)

- [x] Testes F1 verdes, mutações verificadas (19 testes; 13 mutações, cada uma derruba o teste da sua guarda).
- [x] Rodada real: 2 módulos com FAQ, 149 perguntas, `índice == respostas` em todas as seções; nenhum `Err`.
- [x] `--usecache` reproduz a árvore byte a byte e o snapshot a menos do `coletado_em`.
- [x] `.md` com `trilha` de 2–3 níveis e `link` com a âncora do índice (§10.4).
- [ ] Aba FAQs do MG renderiza módulo → seção → (sub-tópico) → perguntas; chat tipo 1 cita `FAQ n` — depende do
  `./start_server.sh` e do `scripts/deploy-frontend.sh`, que são do operador.
- [x] `check-scraper-boundary.sh`, `check-sem-navi.sh`, `check-registry-sync.sh` verdes.
- [ ] Commit D-NAMING separado — à parte deste PR (§10.5).

## 9. Evidência bruta (01/11/2026, Desktop Server)

```bash
S=https://portalsped.fazenda.mg.gov.br/spedmg
curl -s -A AuliBot/0.1 $S/ | grep -oE 'href="/spedmg/[a-z0-9-]+/"' | sort -u | wc -l          # 16 módulos (+ noticias)
for m in nfe nfce cte efd; do curl -s -o /dev/null -w "$m %{http_code}\n" -A AuliBot/0.1 $S/$m/Perguntas-Frequentes/; done  # 200 200 404 404
curl -s -A AuliBot/0.1 $S/nfe/Perguntas-Frequentes/ | grep -oE 'respostas_[ivx]+/index.html#[0-9]+' | sort -u | wc -l     # 87
curl -s -A AuliBot/0.1 $S/nfce/Perguntas-Frequentes/ | grep -oE 'respostas_[ivx]+/index.html#[0-9]+' | sort -u | wc -l    # 62
curl -s -A AuliBot/0.1 $S/robots.txt -o /dev/null -w '%{http_code}\n'                                                       # 404
# CSM: total_articles_count 81 (sef_mg_kb_home); kb_article_view público só com UA de navegador (isCrawlerBot)
```

## 10. Revisão e implementação (01/11/2026)

O censo foi refeito pela rede antes do código (home, os 15 módulos, as 6 páginas de "Perguntas…" e as 15 páginas
de respostas). O total do estudo se confirmou (NF-e 87 + NFC-e 62 = 149), mas a anatomia não: cinco pontos do
§2 estavam errados e mudaram o desenho do parser.

### 10.1 Correções do censo

1. **Enumeração (§2.1, D-MGFAQ-4).** Sondar `<módulo>/Perguntas-Frequentes/` acha só NF-e e NFC-e, mas por acaso:
   seis módulos têm "Perguntas e Respostas" no menu lateral, cada um num caminho (`cte/faq/`,
   `cteos/Perguntas-e-Respostas/`, `efd/faq/`, `mdfe/perguntas/`). O 404 do estudo dizia "sem FAQ" para quatro
   que têm a entrada — e que apontam para fora (CT-e e CT-e OS → portal nacional do CT-e; MDF-e → SVRS; EFD → dois
   PDFs e um `.docx` novo, `Perguntas-frequentes-Desobrigar-DAPI-2026-09-24.docx`). O scraper segue o menu.
2. **A âncora não é numérica nem global (§2.2).** O número **recomeça por página** (`respostas_ii#1` existe) e a
   NFC-e usa `#a1`, `#b1`… (letra da seção). A identidade é (página, âncora), não (módulo, N).
3. **A página de respostas não é uniforme (§2.3).** `<a name>` no NF-e, `<a id>` na NFC-e; NF-e VIII e IX e a 1ª
   pergunta da I **sem âncora** (7 perguntas); na NFC-e I o número é o `<ol start>` e não está no texto; na NF-e
   VIII pergunta e resposta estão no **mesmo `<p>`**, separadas por `<br>`. "Começar na primeira âncora numérica
   e dividir pelo `<a name>`" perderia 7 perguntas e todas as 62 da NFC-e.
4. **O sub-tópico é do índice, não do `•` (§2.3, D-MGFAQ-3).** O índice tem as linhas sem número (8 na III do
   NF-e); a página repete 6 delas como "• Título" antes da pergunta que abre o sub-tópico e omite 2 ("Pessoa
   Física", "Novas Informações com Versão 3.10"). E `•` também é marcador de lista dentro de respostas (NF-e I.2).
5. **Índice e página escrevem a pergunta diferente** em 9 de 149, e a página é a versão revisada ("às", "Empresa"
   onde o índice tem "ás", "IEmpresa"). Igualdade após `clean` (guarda (b) do §2.4) avisaria 9 vezes a cada rodada.

Menores: os módulos são 15 (o "16" do §9 contava `noticias`); o RS tem 1.990 FAQs, não 1.947 (cabeçalho).

### 10.2 Decisões revistas

- **D-MGFAQ-4 (revista):** módulos pela barra da home; a entrada de FAQ pelo menu lateral do módulo. A página é
  índice (tem links `respostas_…`), aponta para fora (só links de outros hosts ou para arquivos → aviso e pula)
  ou é `Err` (linka páginas do SPED mas nenhuma de respostas: índice com markup novo — pular perderia o NF-e).
- **D-MGFAQ-5 (revista):** guardas do índice — contagem crua de links de pergunta == parseados, (página, âncora)
  única; da página — toda pergunta do índice localizada, na ordem; a linha localizada tem de ter ≥ metade das
  palavras do texto do índice (a menor medida é 0,91; uma linha errada, como um item numerado de resposta, fica
  perto de zero) — isto substitui a igualdade da (b) e cobre a (c) "pergunta sem texto"; do acervo — zero `Err`,
  resposta vazia avisa. A (d) "módulo com 200 e zero perguntas" virou a classificação da D-MGFAQ-4.
- **D-MGFAQ-6 (revista):** **toda tag sai**, como no SP, e as entidades são decodificadas depois. A lista fechada
  do PR existe porque o texto do SAC tem `<` cru; nas 17 páginas do SPED não há nenhum (só `&nbsp;` e `&gt;`).
  Célula → espaço; `href` relativo (3) ganha o host.
- **D-MGFAQ-11 (mantida):** "novo conteúdo" (marcador editorial na IX.3) fica como está.
- **D-MGFAQ-12 (nova) — texto da pergunta = o da página**, sem o `N.` (é o que se lê no destino do link e tem as
  correções); o do índice serve só de conferência (D-MGFAQ-5). A pergunta é localizada pela âncora anunciada no
  índice (trocada por uma marca de uso privado antes da conversão para texto, que tira as tags) e, na página sem
  ela, pela primeira linha depois da anterior que começa com o `N.` do índice.

### 10.3 Árvore

Como no §2.5, com um detalhe: na III do NF-e a pergunta 1 vem antes do primeiro sub-tópico. A seção vira `Menu`
com um `Faq` por sub-tópico e a III.1 nos `faq_items` do próprio `Menu` (o precedente do SC); a aba lista os
itens de um `Menu` depois dos filhos, então a III.1 aparece no fim da III. Seção sem sub-tópico é `Faq` direto.

### 10.4 Rodada de 01/11/2026

- 37 GETs (`AuliBot`, 1 s), ~45 s. 15 módulos: NF-e 87 em 9 seções, NFC-e 62 em 6; CT-e, CT-e OS, MDF-e e EFD
  avisados como "apontam para fora"; 9 sem a entrada.
- 149 urls e 149 perguntas únicas, 23 trilhas, 0 resposta vazia, nenhuma tag nem marca sobrando; respostas de 91 a
  10.620 caracteres (a I.6 do NF-e, a lista de setores obrigados por protocolo — 1 key no teto de 6.000 do vetor).
- `--usecache`: árvore idêntica byte a byte; snapshot idêntico a menos do `coletado_em`. A 2ª rodada pela rede
  (`update-mg-faqs.sh --no-serve`) saiu idêntica à 1ª: 149 → 149, 149 vetores reaproveitados.
- 149 `.md`; pack `mg-faqs` (serviços preservados no manifesto); smoke `/v1/retrieve kind=faqs entity=mg`: "posso
  enviar NF-e em lote?" → III.8; "o que é o CSC da NFC-e?" → NFC-e I `#a6`; "como o destinatário faz o download
  do XML da NF-e?" → VIII.1 (sem âncora); "empresa do simples nacional que emite NFC-e precisa entregar SPED
  fiscal?" → NFC-e IV `#d1` — todas em 1º.
- `config/prompts/mg.txt` já fala em FAQs (§7 F3): sem mudança.

### 10.5 Fora deste PR

- **D-NAMING (D-MGFAQ-9):** `SEF/MG` → `SEF-MG` no `orgao` dos serviços muda os 153 `.md` de serviços — vai num
  commit próprio, fora do squash deste PR. O `config/prompts/mg.txt` também escreve "SEF/MG" (2 vezes), na prosa.
- **Fonte A (CSM, fase 2):** a sondagem do §6 usa UA de navegador, que é a D-MGFAQ-8 ainda não ratificada — não
  foi rodada.

## 11. Fase 2 — fonte A, a base do CSM (02/11/2026)

O mantenedor ratificou a D-MGFAQ-8 e decidiu coletar apesar do robots
([auli_pendencias.md](auli_pendencias.md) §42). Código em
[`csm.rs`](../auli-server/crates/scrapers/auli-scraper-mg/src/csm.rs); o `faqs.rs` só junta a base ao acervo e à
árvore, depois dos módulos do SPED.

### 11.1 Correções do §3 e do §6

1. **Robots.** O estudo não olhou o `robots.txt` do `atendimento2`: é `User-agent: *` / `Disallow: /`, o host
   inteiro — inclusive os serviços do `mg.rs`.
2. **Enumeração (§3.2 i).** Nenhuma das três rotas: o card da base na home do CSM leva à página `kb_search`
   ("Pesquisa de Base de Conhecimento"), que lista os 81 artigos para qualquer visitante. A página não traz os
   resultados — o widget "Knowledge Result" os pede num POST (`/api/now/sp/widget/ff26f2b1…`), com o payload que o
   `Knowledge Portal Service.jsdbx` do portal monta. O POST exige o cookie e o token `g_ck` que a página HTML
   entrega a qualquer visitante, sem login: sem eles a resposta é `invalid_token: true` — o `total: null` da
   tabela do §1. A `sn_km_api` responde 401. Com a janela `end: 100`, um POST traz a base inteira, com a
   contagem do próprio portal (`meta.count` = 81).
3. **A base de serviços.** Sem o filtro, a busca devolve 235: a base "SRE - Serviços públicos"
   (`bccaaf011bbec61029f686afe54bcbfc`), que a home não mostra, é a dos serviços — os 153 do cache do `mg.rs` vêm
   todos dela. O filtro pela base `f18a2fc…` é obrigatório.
4. **Identidade.** A lista identifica o artigo pelo `sys_kb_id`, não pelo número KB; a página do artigo é pedida
   por ele, e o número (`KB00…`) vem da página. O link segue a D-MGFAQ-2 (`sysparm_article=<número>`), que
   sobrevive às versões.
5. **As três naturezas, uma por categoria:** *DF-e - Correção de Erros / Rejeições* 61 (pergunta única, artigo de
   modelo com seções), *Sintegra - PED* 1 (a compilação: tabela de **100 pares P./R.**, numerados de 1 a 100 sem
   buraco) e *Orientações Tributárias DOLT/SUTRI* 19 (texto livre). Na rejeição, a seção "Descrição do Erro" vem
   **cortada em ~40 caracteres** e o "Assunto" (só na 690) repete o título; o título (`shortDesc`) é a pergunta e
   coincide com o da lista nos 81 — 6 títulos param em 100 caracteres, o limite do campo na própria base.
6. **Entidades.** O editor do CSM emite entidades numéricas (`&#34;` 178, `&#61;` 123, `&#43;` 6 nos 62 artigos
   coletados), que o `decode_entities` do kit não trata. O SPED não tem nenhuma.

### 11.2 Decisões

- **D-MGFAQ-8 (ratificada, 02/11/2026):** no `atendimento2`, o UA do kit, como os serviços — o AuliBot recebe a
  página sem o texto (`isCrawlerBot`). O SPED segue com o AuliBot.
- **D-MGFAQ-13 — Enumeração da fonte A:** GET da página `kb_search` (cookie + `g_ck`) → POST ao "Knowledge
  Result" filtrado pela base → cada artigo pela page API (`kb_article_view&sys_kb_id=…`, sem sessão, como no
  `mg.rs`), widget `edx_article_header`. 1 s entre requisições: 64 por rodada (1 + 1 + 62 — as orientações não são
  baixadas). Guardas: sessão recusada, base vazia, menos itens que o `meta.count`, artigo de outra base, artigo sem
  exatamente uma categoria, artigo repetido, página servida a robô, artigo sem número — todos `Err` antes de
  gravar. Rede sempre; `--usecache` lê a lista e os artigos do cache, sem sessão (D-MGFAQ-10).
- **D-MGFAQ-14 — Natureza pela categoria, lista fechada no código:** categoria nova é `Err` (a natureza dela é
  decisão, não palpite). As **orientações da DOLT/SUTRI ficam fora**: são documentos de 4 a 65 mil caracteres, de
  2004 a 2016, orientação tributária e não FAQ (§3.2 ii).
- **D-MGFAQ-15 — Modelo:** rejeição → pergunta = título, resposta = "Resposta para o Erro" (sem ela, `Err`);
  compilação → um `FaqRaw` por par, pergunta sem o `N)` (D-MGFAQ-11), numeração conferida de 1 a N, todas com o
  link do artigo (não há âncora; o nome do `.md` é hash de link + pergunta, então não colide); trilha `SRE - Dúvidas
  Frequentes | <categoria>`. As rejeições saem **pelo título**: a lista vem por visualizações, que mudam a cada
  rodada, e mudariam o snapshot sem a base mudar.

### 11.3 Árvore

A raiz ganha um 3º filho, ao lado do NF-e e da NFC-e: `Menu` "SRE - Dúvidas Frequentes" (url = a busca filtrada
pela base) → um `Faq` por categoria, DF-e (61) e Sintegra - PED (100).

### 11.4 Rodada de 02/11/2026

- SPED 149 (idêntico: os 149 vetores foram reaproveitados) + CSM 161 = **310**. 81 artigos na lista; 62 baixados.
- As 161: pares (link, pergunta) únicos; 62 links (61 rejeições + o PED); 0 pergunta ou resposta vazia; nenhuma tag
  nem entidade sobrando; respostas de 4 ("Sim.", no PED) a 2.965 caracteres.
- `--usecache`: árvore idêntica byte a byte; snapshot idêntico a menos do `coletado_em`.
- 310 `.md`; pack `mg-faqs` com 149 vetores reaproveitados e 161 novos. Smoke `/v1/retrieve kind=faqs entity=mg`:
  "erro 215 falha no schema" → Rejeição 215; "rejeição 539 duplicidade" → 539; "registro 54 na aquisição de
  material de uso e consumo" → o par do PED; "posso cancelar uma NF-e vinculada a um MDF-e?" → Rejeição 690, com
  as do SPED sobre cancelamento logo depois; "NFC-e em contingência" → a do SPED — todas em 1º. "Quem é obrigado
  a entregar o Sintegra" traz 5 pares do PED, com "Dispensa… aos obrigados à EFD" em 2º.
- 43 testes no crate (20 do CSM); cada guarda do `csm.rs` foi desligada e o teste dela falhou.

### 11.5 Fora

- **As 19 orientações da DOLT/SUTRI** — candidatas a uma coleção própria, como os pareceres; algumas têm blocos
  de perguntas e respostas dentro (a ITCD 002/2006, a mais vista, se chama "Dúvidas Frequentes").
- **"Comunicados do Portal do Contribuinte"**, a outra base da home, tem 0 artigos.
