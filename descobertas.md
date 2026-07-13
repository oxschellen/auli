# Relatórios de Descoberta — auli

> Consolidação dos relatórios de descoberta: 14 de **serviços** por entidade (antes arquivos
> `descoberta-*.md` separados) + 1 de **pareceres** (RS). Cada seção documenta como o portal foi
> investigado. Referenciado por `SCRAPERS.md`, `auli_pendencias.md` e comentários nos crates
> (âncoras `descobertas.md#<uf>` e `#rs-pareceres`).

## Índice
- [AC — Carta de Serviços SEFAZ-AC (sefaz.ac.gov.br)](#ac)
- [AL — SEFAZ-AL / Portal Alagoas Digital](#al)
- [AM — Portfólio de Serviços da SEFAZ-AM](#am)
- [AP — Portal SEFAZ-AP (www.sefaz.ap.gov.br)](#ap)
- [DF — SEFAZ-DF (Distrito Federal)](#df)
- [ES — Conecta Cidadão / portal.es.gov.br (SEFAZ-ES)](#es)
- [MA — Portal de Serviços SEFAZ-MA (portal-sgc.sefaz.ma.gov.br)](#ma)
- [PA — Ecossistema de serviços da SEFA-PA](#pa)
- [PB — SEFAZ-PB (Paraíba)](#pb)
- [RN — SEFAZ-RN (Rio Grande do Norte)](#rn)
- [RO — Agência Virtual SEFIN-RO](#ro)
- [RR — SEFAZ-RR (Roraima)](#rr)
- [SE — SEFAZ-SE (Sergipe)](#se)
- [TO — Carta de Serviços SEFAZ-TO (servicos.to.gov.br)](#to)
- [RS Pareceres — Portal de Legislação / Consultas Formais Respondidas](#rs-pareceres)


---

<a id="ac"></a>

# Relatório de descoberta — Carta de Serviços SEFAZ-AC (sefaz.ac.gov.br)

**Data:** 2026-07-06 · **Escopo:** descoberta + base para a 21ª entidade. · **Órgão:** Secretaria de
Estado da Fazenda do Acre (SEFAZ-AC). · **Robots:** desconsiderado (decisão do usuário).

> **TL;DR — WordPress + Elementor, HTML server-rendered (sem wp-json).** A "Carta de Serviços"
> (`?page_id=6732`) lista **17 serviços** agrupados em categorias (Notas Fiscais / Cadastros / IPVA +
> Geral); cada serviço é um **post** (`?p=NNNNN`) com descrição rica. O scraper: parseia a Carta →
> serviços (título, categoria, post) → busca cada post → extrai o corpo (`.elementor-widget-theme-post-content`).
> **⚠️ Gotcha TLS:** o servidor manda o **intermediário ERRADO** (Sectigo RSA OV antigo) faltando o
> **R36** (emissor real do leaf) → curl/rustls/certifi rejeitam. Fix: embutir o R36 como trust anchor
> (igual MA; provado).

---

## Plataforma

- **WordPress + Elementor** (título "Carta de Serviços"; `wp-content`, classes `elementor-*`). HTML
  server-rendered. **`wp-json` = 404** (REST API desativada) → não há JSON; parseamos o HTML.
- URL da Carta: `https://sefaz.ac.gov.br/2021/?page_id=6732`.

## ⚠️ TLS — cadeia quebrada (intermediário errado)

O servidor envia 2 certs: o **leaf** (`*.sefaz.ac.gov.br`, emitido por *Sectigo … CA OV **R36***) e um
intermediário **ERRADO** (*Sectigo RSA Organization Validation Secure Server CA*, um CA antigo que NÃO
emitiu o leaf). Falta o **R36** → nem o store do sistema nem o bundle Mozilla (certifi/rustls) fecham a
cadeia (`CERTIFICATE_VERIFY_FAILED`). Fix (provado): baixar o R36 do AIA do leaf
(`http://crt.sectigo.com/SectigoPublicServerAuthenticationCAOVR36.crt`) e **embuti-lo como trust
anchor** no rustls (`RootCerts::new_with_certs(&[R36])`) — o leaf encadeia direto nele. Mesmo padrão do
MA (PEM embutido no crate). O R36 é emitido pela *Sectigo Public Server Authentication Root R46*.

## Catálogo — a Carta (`?page_id=6732`)

Seção "Lista de Serviços": cards agrupados por **categoria** (heading `Serviços …`), cada card com
título + link `?p=NNNNN` (o post do serviço) + "Acesse a descrição completa do serviço »". **17 serviços:**

| categoria | nº | exemplos |
|---|---|---|
| (Geral) | 6 | Isenção de ICMS para PCD, Consulta Tributária, Certidão Negativa de Débitos, Notas Fiscais, Gráficas, Diversos |
| Notas Fiscais e Documentos Eletrônicos | 3 | Emitir Nota Fiscal Avulsa (Floresta / Mercadorias / Conserto) |
| Cadastros | 4 | Domicílio Eletrônico, Sefaz Online, Cadastro de Contribuintes, Cadastro de Credores |
| IPVA | 4 | IPVA Isenção (Táxi/Mototáxi, PCD, PJ), IPVA Baixa de débito |

Parse (regex): na seção "Lista de Serviços", casar `href="…?p=(\d+)">TÍTULO` (ignorar "Acesse…"),
atribuir a categoria pelo heading `Serviços …` anterior; dedup por post id.

## Detalhe — o post (`?p=NNNNN`)

O corpo do serviço (descrição rica) está no container **`.elementor-widget-theme-post-content`**
(aparece **1× por post** — isola do header/footer/sidebar). Extrair com o crate `scraper` (DOM):
`select(".elementor-widget-theme-post-content").text()` → `clean`. Ex. (IPVA Isenção Táxi): *"São
isentos de IPVA, além dos veículos reconhecidos pela Lei Complementar nº 114/2002, os veículos
destinados à condução de passageiros…"*.

## Modelagem (v1)

- **titulo** = título do card na Carta; **descricao** = corpo do post (`.elementor-widget-theme-post-content`).
- **classe** = a categoria (Geral / Notas Fiscais e Documentos Eletrônicos / Cadastros / IPVA).
- **público** = único "Serviços" (sem eixo de audiência). `ocorrencias` = {Serviços × categoria}.
- **link** = `https://sefaz.ac.gov.br/2021/?p={post}`; identidade = o post. **órgão** = "SEFAZ-AC".
- Guard = piso ~15 (os 17 da Carta).

## Pontos de decisão (D-AC*)

1. **D-AC-TLS** — embutir o intermediário R36 (Sectigo) como trust anchor (rustls), pois o servidor
   manda a cadeia errada. Documentar como exceção; se o cert for reemitido, o handshake avisa.
2. **D-AC-FONTE** — HTML (WordPress/Elementor), sem wp-json. Carta (`page_id=6732`) → 17 posts de serviço.
3. **D-AC-ROBOTS** — desconsiderado (decisão do usuário); coberto pela política D-PA-ROBOTS (UA AuliBot).
4. **Fragilidade** — parse de HTML Elementor (classes estáveis: `elementor-widget-theme-post-content`,
   headings `Serviços …`, links `?p=`); guard de contagem avisa se a Carta mudar.

## Evidência

Em `scratchpad/ac/`: `page.html` (a Carta), `post.html` (um serviço), `r36.pem` (intermediário Sectigo),
`chain.txt` (a cadeia quebrada). wp-json = 404.


---

<a id="al"></a>

# Descoberta — SEFAZ-AL (Secretaria de Estado da Fazenda de Alagoas)

- **UF:** AL
- **Órgão:** SEFAZ-AL
- **Fonte:** Portal Alagoas Digital (catálogo whole-of-government do estado)
- **Data da descoberta:** 07/07/2026
- **Status:** ✅ Fonte tier-1 confirmada. Pronto para TAREFA de implementação.
- **Escopo decidido:** apenas SEFAZ-AL (60 serviços). Multi-órgão fica de fora por ora (ver §9).

---

## 1. TL;DR

A SEFAZ-AL não mantém portal de serviços próprio: seus serviços vivem no **Portal Alagoas Digital**, um catálogo estadual único que serve os 71 órgãos do estado através de uma **API REST pública, documentada e sem autenticação**, publicada pelo próprio governo como "Dados Abertos".

Coleta: uma chamada filtrada por `organ_id` devolve os 60 serviços da SEFAZ; cada serviço é enriquecido por uma chamada de detalhe estruturada que mapeia quase 1:1 para o `ServicoRaw`. Sem HTML server-rendered, sem headless, sem auth. É a fonte mais limpa desde o RS.

---

## 2. Hierarquia discovery-first (o que foi avaliado)

| Camada | O que é | Veredito |
|---|---|---|
| **API REST `/api/v1`** | JSON público documentado, sem auth | ✅ **ESCOLHIDA** — tier-1 |
| HTML server-rendered `/orgaos`, `/orgao/{id}` | Portal paginado (`?page=N`) | ❌ Desnecessário — a API cobre tudo |
| Guia de Serviços `/guia-de-servicos*` | Páginas de serviço renderizadas | ❌ Mesmos dados, versão HTML |
| Headless | — | ❌ Nunca (doutrina) |

A própria página `/api/` documenta o contrato com a linha textual **"Autenticação: Não necessária"**. Não é inferência — é declaração da fonte.

---

## 3. Endpoint e parâmetros

**Base:** `https://alagoasdigital.al.gov.br/api/v1`

| Recurso | Uso no scraper |
|---|---|
| `GET /services.json?organ_id={UUID}` | Lista os 60 stubs da SEFAZ. **Filtro funciona** (confirmado: 60 filtrado == 60 client-side). |
| `GET /services/{id}.json` | Detalhe estruturado completo de cada serviço. |
| `GET /organs.json` | Resolve o `organ_id` da SEFAZ e valida que segue ativo. |

**`organ_id` da SEFAZ-AL:** `e1799779-d21d-411e-8387-03cbc106c6c1`

> ⚠️ **Nota sobre o filtro `organ_id`:** ele funciona com o UUID. A doc diz "passar no tipo inteiro" — está **errada** (os IDs não são inteiros). Ignore essa linha da doc. Alternativa robusta: puxar `services.json` inteiro (1664 itens, array único, sem paginação) e filtrar client-side por `organ == UUID`. Ambos os caminhos dão os mesmos 60.

---

## 4. Guardas dinâmicas (lição CE)

**Nunca hardcodar 60.** Os 60 são o valor observado hoje, não um contrato.

Sequência de guarda:
1. Resolver `organ_id` da SEFAZ via `organs.json` (procurar `acronym == "SEFAZ"` e `nature == "Estadual"`) — não hardcodar o UUID tampouco; derivar.
2. Buscar a lista filtrada; **ler o comprimento do array em runtime**.
3. Bail se o array vier vazio (miss). Array vazio = fonte quebrada ou UUID mudou, não "SEFAZ sem serviços".
4. Guard de coerência: todo item da lista deve ter `organ == UUID_SEFAZ`. Item fora do órgão = filtro falhou silenciosamente → bail.
5. **Escrever cache só depois de todas as guardas passarem.**

---

## 5. Schema do serviço → `ServicoRaw`

Detalhe confirmado com serviço real ("Regularização Espontânea Simples Nacional - SEFAZ", `588fba6f8c36c7239917ff55`). Campos:

| Campo API | Tipo | Destino / tratamento |
|---|---|---|
| `id` | string (ObjectId) | id do serviço |
| `name` | string | título |
| `active` | **bool** no detalhe | flag |
| `agendavel` | bool | metadado |
| `description` | **HTML + entidades** | `clean()`: strip tags + decode entidades |
| `free` / `maturity_level` | bool / string ("Semi-digital") | metadados |
| `other_informations` | **HTML + entidades** | canais/contato; `clean()` |
| `steps[]` | array | `title`, `description` (HTML), `cost{coin}`, `providing_channels[]{description,type}` |
| `estimated_time` | obj | `min`/`max` (podem ser null), `description` (HTML), `unit`, `type` |
| `applicants[]` | array | `type` (Cidadão/Empresa), `requirements` (string, pode vir "") |
| `audiences[]` / `categories[]` | array de taxonomia | `{id, name}` — normalizável se for facetar |
| `date` / `date_modified` | string `DD/MM/YYYY HH:MM` | parse de data |
| `organ` | string (UUID) | chave de faceta / guard de coerência |
| `url` / `url_relativa` | string | link canônico |

Cobertura: onde a SEFAZ preenche, preenche bem (steps com canais tipados, estimated_time, applicants). Alguns campos vêm esparsos (`requirements: ""`, `min/max: null`) — isso é característica da fonte, ingerir como está.

---

## 6. Pegadinhas de serde (para o TAREFA)

1. **`active` muda de tipo entre endpoints:** vem **string** `"true"` no `organs.json` e **bool** `true` no detalhe do serviço. `OrganRaw` e `ServicoRaw` precisam de deserializers diferentes, ou um custom que aceite ambos.
2. **Todo texto é HTML + entidades HTML** (`&ccedil;`, `&atilde;`, `<p>…</p>`). O `clean()` precisa strippar tags **e** decodificar entidades. Não basta remover `<>`.
3. **Datas em `DD/MM/YYYY HH:MM`** — não ISO.
4. **Dois formatos de ID de órgão convivem:** ObjectId Mongo legado (`596e172e…`) nos órgãos antigos, UUID (`e1799779…`) nos novos. SEFAZ é UUID. `organ_id` sempre string.
5. **`providing_channels[].type` é enum** (visto: `WEB`, `TELEFONE`). Não hardcodar o conjunto — o enum completo (provável: PRESENCIAL, EMAIL, APP…) só se revela varrendo os 60. Mini-lição-CE: descobrir dinâmico, tratar desconhecido como fallback, não como panic.

---

## 7. Quirks de dado da fonte (ingerir como está, não "consertar")

- **"Isenção de IPVA"** está tagueada sob o órgão **ARSAL** (`596e172e…`), não SEFAZ, apesar do texto dizer "perante a SEFAZ". Inconsistência de cadastro na origem. Com escopo SEFAZ-only, esse serviço **não entra** — decisão consciente, documentada aqui para não parecer omissão.

---

## 8. Estratégia de coleta

```
1. GET organs.json → derivar UUID da SEFAZ (acronym=SEFAZ, nature=Estadual)
2. GET services.json?organ_id={UUID} → N stubs (N=60 hoje)
3. guardas §4 (bail se vazio ou incoerente)
4. para cada id: GET services/{id}.json   [≥1s entre chamadas; honrar --usecache; miss=bail]
5. clean() em cada campo HTML (strip tags + decode entidades)
6. escrever cache só após guardas
```

- **User-Agent:** `AuliBot/0.1 (github.com/oxschellen/auli; <contato>)`
- **Rate limit:** ≥1s. 60 detalhes ≈ 60s. Aceitável.
- **Cache:** `--usecache`, miss = bail.
- **Fetch:** array único (sem paginação) simplifica; uma chamada de lista + 60 de detalhe.

---

## 9. Decisões

- **D-AL-1 — Escopo SEFAZ-only.** Coletar apenas os 60 serviços do órgão SEFAZ, via filtro `organ_id`. O Portal Alagoas Digital é multi-órgão (1664 serviços, 71 órgãos) e seria a realização pura do D-PA-ACERVO — mas isso puxa saúde/trânsito/crédito para o acervo e sai do escopo "acervo das SEFAZes". O scraper nasce pronto para virar multi-órgão depois: basta relaxar o filtro. Oportunidade multi-órgão registrada, não implementada.
- **D-AL-2 — robots.txt / civic-purpose (PENDENTE de confirmação).** A API é publicada pelo próprio governo como "Dados Abertos" (LAI), sinal civic-purpose forte que praticamente esvazia a questão robots. Confirmar o conteúdo de `robots.txt` e formalizar como decisão D-XX (disregard justificado por LAI, se aplicável), no padrão das demais UFs.

---

## 10. Pendências antes do TAREFA

- [ ] `curl -sA "AuliBot/0.1" https://alagoasdigital.al.gov.br/robots.txt` — fechar D-AL-2.
- [ ] Varrer os 60 detalhes uma vez para enumerar o enum completo de `providing_channels[].type` (§6.5).
- [ ] Confirmar comportamento de `applicants[].requirements` quando preenchido (só vimos string vazia) — pode ser string ou array em outros serviços.


---

<a id="am"></a>

# Relatório de descoberta — Portfólio de Serviços da SEFAZ-AM

**Data da coleta:** 2026-07-05 · **Escopo:** só descoberta (nenhuma linha de scraper). · **Alvo:**
`https://www.sefaz.am.gov.br/portfolio-servicos/todos`

> **TL;DR.** O portal é **Next.js App Router** (não Pages Router). A hipótese da tarefa
> (`__NEXT_DATA__` / `/_next/data/{buildId}/*.json`) está **refutada**: não há `__NEXT_DATA__` nem
> rota `/_next/data`. Os dados vêm como **RSC (React Server Components) flight**, obtido com o header
> **`RSC: 1`** na própria URL da página (`text/x-component`) — ou inline no HTML via `self.__next_f.push`.
> **A listagem inteira e o conteúdo COMPLETO de cada detalhe (todos os accordions) vêm server-rendered
> numa única resposta cada — ZERO XHR lazy.** Rota de menor atrito confirmada: `ureq` GET + parse do
> flight RSC. Sem headless no scraper (headless foi usado só aqui, para _verificar_ que não há XHR).

---

## Fase 1 — Framework e transporte

- `GET /portfolio-servicos/todos` → **HTTP/1.1 200**, `Server: Apache`, `Content-Type: text/html`.
  Header revelador: **`Vary: RSC,Next-Router-State-Tree,Next-Router-Prefetch`** → **Next.js App Router**.
- `grep -c __NEXT_DATA__` = **0**. Não há `/_next/data/{buildId}/…`. **buildId é irrelevante** para a
  coleta: o App Router serve o flight RSC pela própria URL + header `RSC: 1` (o `buildId` aparece no
  flight mas não é parâmetro de nenhuma rota de dados). → A restrição "extrair buildId por deploy" da
  tarefa **não se aplica** a este portal.
- Cookie de sessão sticky (`OLSESSIONID=sticky.sefazN`) — irrelevante para GETs anônimos.

**Como obter o JSON (as duas vias, equivalentes):**
```bash
# via A (recomendada): flight RSC puro
curl -s -H 'RSC: 1' 'https://www.sefaz.am.gov.br/portfolio-servicos/todos' -o todos.rsc      # text/x-component
# via B: inline no HTML
curl -s 'https://www.sefaz.am.gov.br/portfolio-servicos/todos' | grep -o 'self.__next_f.push(\[1,"'  # presente
```

## Fase 2 — Listagem (árvore de serviços)

No flight, o componente de listagem é `["$","$L8",null,{"items":[…]}]`. `items` é a **árvore pura em
JSON**: categoria → (subcategoria opcional) → serviço. Extração = achar `{"items":[` e ler o array por
balanceamento de colchetes.

- **19 categorias** de topo, **61 nós** de categoria/subcategoria, **278 serviços-folha**.
- Folhas em profundidade 2 (76) ou 3 (202) — nem toda categoria tem subcategoria.
- **Contagem bate com o contador dinâmico da página: 278 = "278 serviços encontrados"** ✅ (nunca
  hardcodar; ler sempre da árvore/contador — lição CE respeitada).

**Nó-folha (serviço) — mapa de campos → snapshot:**
```json
{ "id": 882,
  "name": "Pedir Inscrição Estadual: Comércio, Indústria Não incentivada e Transportes",
  "description": "Permite a obtenção de inscrição estadual para pessoas jurídicas …",
  "url": "/portfolio-servicos/detalhes/882?profile=todos",
  "actions": [ {"label":"Iniciar","url":"https://sistemas.sefaz.am.gov.br/…"},
               {"label":"Chat","url":"https://tawk.to/chat/…"},
               {"label":"Agendar Serviço","url":"https://online.sefaz.am.gov.br/agendamento/novo/882…"} ],
  "fontAwesomeIconClassName": null, "children": [] }
```
| campo do nó | → snapshot | observação |
|-------------|-----------|-----------|
| `id` | identidade | inteiro; = id da página de detalhe |
| `name` | `titulo` | — |
| `description` | `descricao` (curta) | = `resumo` do detalhe |
| `url` | `link` | interno `/portfolio-servicos/detalhes/{id}` **ou** externo/submenu (Fase 5) |
| `actions[]` | ações/atributos | botões (Iniciar/Chat/Agendar); "Agendar" ⇒ agendável (Fase 4) |
| `path` (breadcrumb, no detalhe) | categoria/classe | árvore de categoria também dá o caminho |

## Fase 3 — Detalhe (ponto crítico) — **RESOLVIDO**

Página `/portfolio-servicos/detalhes/{id}?profile=todos`, também RSC. O objeto `serviceDetails` vem
embutido no flight:
```json
{ "id":882, "name":"…", "url":"https://sistemas.sefaz.am.gov.br/…",  // destino "Iniciar"
  "resumo":"…",                         // = "O que é" / RESUMO (texto plano)
  "perfis":["Pessoa Jurídica"],         // público(s) — ver Fase 4
  "visibleSections":["RESUMO","PUBLICO_ALVO","COMO_PROCEDER","DOCUMENTACAO","LEGISLACAO","PERGUNTAS_FREQUENTES","CONTATO"],
  "comoProcederHtml":"$a", "documentacaoHtml":"$b", "perguntasRespostasHtml":"$c",  // refs de chunk
  "descricaoHtml":"<p>…</p>", "legislacaoHtml":"<p>…</p>",   // HTML inline
  "setorResponsavel":{"nome":"GERENCIA DE CADASTRO","sigla":"GCAD"},
  "email":"gcad@sefaz.am.gov.br", "phone":"(92) 3026-4641 / 4944", "tempoMedioEmDias":20,
  "taxa":{"descricao":null,"valor":null}, "chatUrl":"…", "podeSerAvaliado":true,
  "actions":[…], "canaisAtendimento":[…] }
```

**De onde vem o conteúdo dos accordions (a pergunta crítica):**
- **Tudo na resposta única do detalhe.** As seções longas usam **referências de chunk do flight**
  (`"$a"/"$b"/"$c"`), que resolvem para chunks-texto **no mesmo payload** no formato `a:T<hexlen>,<html>`
  (ex.: `comoProcederHtml → $a → a:Td68,<p>As solicita&ccedil;&otilde;es …`). `documentacaoHtml → $b`,
  `perguntasRespostasHtml → $c` idem (os chunks vêm colados ao anterior, sem `\n` — cuidado no parser).
  `resumo`, `descricaoHtml`, `legislacaoHtml` vêm **inline**.
- **Verificação empírica (headless, só para provar):** renderizei 882, 502 e 63 no Chrome e **expandi
  todos os accordions** (Documentação, Perguntas Frequentes, etc.). **Nenhuma requisição de rede** foi
  disparada na expansão (único POST na página = `sistemas.sefaz.am.gov.br/portal/api/logs`, analytics).
  Todos renderizaram o conteúdo completo. ⇒ **Não há XHR lazy; o scraper NÃO precisa de navegador.**
- `visibleSections` controla **quais** seções existem por serviço (ex.: id 63 não tem
  `PERGUNTAS_FREQUENTES`). Seções ausentes = não renderizadas (não vazias).
- **Encoding:** o HTML das seções usa **entidades** (`&ccedil;`, `&atilde;`, `&eacute;`, `&ordm;`…) →
  o scraper terá de **decodificar entidades** (mesma lição do GO: `kit::decode_entities` é tabela fixa
  e não cobre tudo → usar html5ever/`scraper`).

## Fase 4 — Semântica dos perfis

Contagens por rota (árvore RSC, 2026-07-05):

| rota | serviços-folha |
|------|----------------|
| `/todos` | **278** |
| `/pessoa-fisica` | 147 |
| `/pessoa-juridica` | 210 |
| `/orgaos-publicos` | 67 |
| `/agendaveis` | **278** (= todos) |

- **Público = filtro por rota (Hipótese A), COM sobreposição.** `pf ∩ pj = 98` (um serviço pode servir
  a vários públicos). `pf ∪ pj ∪ op = 279`. O detalhe também expõe `perfis:[…]` por serviço
  (Hipótese B disponível). ⇒ modelagem natural = **`ServicoPerPublico`** com públicos
  *Pessoa Física / Pessoa Jurídica / Órgãos Públicos* (multi-valor por serviço).
  - **Rota mais barata:** 3 fetches (`pessoa-fisica`, `pessoa-juridica`, `orgaos-publicos`) + união,
    deduzindo o público por pertencimento; OU 1 fetch `todos` + `perfis` de cada detalhe (278 detalhes).
- **`agendaveis` NÃO é público:** a rota `/agendaveis` devolve os **278** (não filtra no servidor).
  O atributo "agendável" real é **por-serviço, detectável pela ação "Agendar Serviço"** → **113 serviços**
  têm essa ação. ⇒ tratar agendável como **flag/atributo**, não como público (confirma a recomendação
  preliminar da tarefa).
- **⚠️ Inconsistência do portal:** o id **1436** aparece em um perfil (`pf/pj/op`) mas **não** na árvore
  `/todos` (por isso união=279 > 278). Registrar; decidir na implementação se entra pela união de perfis.

## Fase 5 — Inventário de tipos de link (dos 278 `url` da árvore `todos`)

| tipo | qtde | exemplo |
|------|------|---------|
| **interno com detalhe** `/portfolio-servicos/detalhes/{id}` | **239** | `/portfolio-servicos/detalhes/882` |
| **externo direto** (sem página de detalhe) | **35** | `sistemas.sefaz.am.gov.br`, `online.sefaz.am.gov.br`, `buscapreco…`, `portalnfce…`, `dfe-portal.svrs.rs.gov.br`, `www.transparenciafiscal.am.gov.br`, `www.gnre.pe.gov.br:444` |
| **`/submenu/{id}`** (institucional) | **4** | `/submenu/{id}` |

- Soma = **239 + 35 + 4 = 278** ✅ (= contador dinâmico).
- **39 serviços são "link-only"** (35 externos + 4 submenu): não têm `serviceDetails` próprio — só
  `name` + `description` + `url`. Decidir representação no snapshot (ver pontos de decisão).
- Domínios externos distintos: `sistemas.sefaz.am.gov.br` (13), `online.sefaz.am.gov.br` (10),
  `www.transparenciafiscal.am.gov.br` (6), `www.sefaz.am.gov.br` (5), + `buscapreco`, `portalnfce`,
  `dfe-portal.svrs.rs.gov.br`, `www.gnre.pe.gov.br:444` (1 cada). 5 serviços têm `actions:[]` (sem botões).

## Fase 6 — Duplicatas (nome idêntico, ids distintos)

**5 pares** publicados pelo portal:

| ids | nome |
|-----|------|
| 502 · 3285 | Parcelar Débitos: IPVA (Eletrônico) |
| 3382 · 3402 | Parcelar Débitos: IPVA (Representante Legal) |
| 3502 · 3503 | Parcelar Débitos: ITCMD (Eletrônico) |
| 501 · 3262 | Parcelar Débitos: ITCMD - Representante Legal (Protocolo Virtual) |
| 1203 · 3282 | Pedir Retificação de DAR (REDAR - ITCMD) |

Cada par tem ids distintos (categorias/fluxos diferentes) — reais, publicados. (O caso 3285 vs 502 da
tarefa confirmado.)

---

## Pontos de decisão a catalogar (candidatos a D-AM* em `auli_pendencias.md` — NÃO decididos aqui)

1. **Agendáveis** → **flag/atributo** (ação "Agendar Serviço"), não público. Evidência: rota
   `/agendaveis` == `todos` (278); 113 serviços têm a ação. *Recomendação: flag.*
2. **Serviços link-only (39)** → sem `serviceDetails`. Representar com seções ausentes (não vazias);
   `link` = a URL externa/submenu. Distinção semântica importa no `auli-collections` (descrição só do
   campo `description` curto). *Recomendação: manter como serviço, seções ausentes.*
3. **Duplicatas publicadas (5 pares)** → **manter ambas** (fidelidade ao portal), identidade = `id`.
   *Recomendação: manter, registrar ids.*
4. **Público** → `ServicoPerPublico` multi-valor (PF/PJ/Órgãos Públicos), via união das 3 rotas de
   perfil OU via `perfis` do detalhe. Sobreposição real (pf∩pj=98). *Decidir a fonte na implementação.*
5. **`/submenu/{id}` (4)** → incluir como serviço ou excluir como página institucional. *Sem recomendação
   forte; inspecionar os 4 no momento da implementação.*
6. **id órfão 1436** (em perfil, não em `todos`) → decidir se a fonte de verdade é `todos` (278) ou a
   união dos perfis (279).
7. **Transporte** → `ureq` GET com header `RSC: 1` (ou parse do `__next_f` no HTML). Confirmar que o
   `ureq` recebe o flight (Apache/HTTP1.1, sem WAF observado). `--usecache`: miss = erro (nunca fallback).

## Restrições / observações

- **App Router / RSC:** sem `buildId` a extrair; a coleta usa a URL + `RSC: 1`. Formato flight = chunks
  `<ref>:<payload>`; refs `$a/$b/$c` resolvem chunks-texto `a:T<hexlen>,<html>` no mesmo payload (alguns
  colados sem `\n` — parser deve tolerar).
- **Encoding:** entidades HTML nas seções → decodificar (html5ever, não a tabela fixa do kit).
- **Sem headless no scraper:** provado que todo conteúdo é server-rendered (0 XHR na expansão dos
  accordions). Chrome foi usado só para esta verificação.
- **Rate limiting:** nenhum observado (algumas dezenas de GETs sem bloqueio).

## Evidência bruta (reprodutível)

Salva no diretório de trabalho da sessão (`scratchpad/am/`): `am-todos.html`, `am-todos.rsc`,
`am-{pessoa-fisica,pessoa-juridica,orgaos-publicos,agendaveis}.rsc`, `am-det-882.{html,rsc}`,
`sd-882.json` (objeto `serviceDetails` extraído), `items.json` (árvore da listagem). Comandos-chave
embutidos acima (Fases 1–2). Nenhuma evidência depende de estado autenticado.

## Critérios de aceitação

- [x] 6 fases executadas com evidência bruta salva e referenciada.
- [x] Ponto crítico (Fase 3) resolvido: conteúdo dos accordions é 100% server-rendered no RSC do
  detalhe (chunks `$a/$b/$c`), **zero XHR**.
- [x] Contagens batem: árvore RSC = 278 = contador dinâmico; soma dos tipos de link = 278.
- [x] Nenhuma linha de scraper implementada.


---

<a id="ap"></a>

# Relatório de descoberta — Portal SEFAZ-AP (www.sefaz.ap.gov.br)

**Data:** 2026-07-06 · **Escopo:** descoberta + base para a 20ª entidade (descrição rica). · **Órgão:**
Secretaria da Fazenda do Amapá (SEFAZ-AP).

> **TL;DR — SPA Angular (FUSE) cujos serviços com descrição rica estão HARDCODED no bundle JS** (não em
> API). A página `#/categorias/{cat}/{servico}` renderiza, em runtime, a partir de arrays `mock*`
> embutidos no chunk lazy `categorias_routes`. **49 serviços** em 5 categorias, cada um com `titulo` +
> `descricao` (blob HTML autocontido: "o que é" + Quem Pode Utilizar + Setor + Tipo). O scraper baixa o
> chunk (headless-free) e parseia os `mock*`. Fonte mais frágil da frota (JS webpack), mas as **chaves
> NÃO estão minificadas** (`route`/`titulo`/`descricao`), então o parse é estável; só o **hash do chunk
> muda por build** → descoberto dinamicamente via `runtime.js`.

---

## Plataforma

- **Angular 17 + template FUSE 19**, nginx. Home = shell SPA (2,3 KB) — o conteúdo é client-rendered.
- Há um **gateway anônimo** (`api-gateway.sefaz.ap.gov.br`, microserviços `links-api`/`noticias-api`/…)
  MAS os "serviços" da home (`links-api/links/acesso-rapido`) são **só links sem descrição** (≈22).
  O catálogo RICO é outra coisa: a página **`#/categorias`**.
- **`#/categorias/{cat}/{servico}`** (ex.: `/categorias/cadastro/mei`) mostra descrição, "Quem Pode
  Utilizar", "Setor Responsável", "Tipo de Atendimento". **Nenhuma API dispara** ao abrir — os dados são
  **hardcoded no chunk Angular** (arrays `mock*`), renderizados por `<app-page-servicos [dados]>`.
- **Por que não pegar do HTML renderizado:** o HTML servido é o shell vazio; o `<p>` com o conteúdo só
  existe DEPOIS do Angular rodar. Pegá-lo exigiria headless por página (~50 renders). O MESMO HTML já
  está no campo `descricao` do mock → parseamos o chunk (headless-free).

## Descoberta do chunk (hash dinâmico)

1. `GET https://www.sefaz.ap.gov.br/` → o shell referencia `runtime.<hash>.js` (num `<script src>`).
2. `GET /runtime.<hash>.js` → contém o mapa nome→hash:
   `"src_app_modules_landing_page_categorias_categorias_routes_ts":"<hash>"`.
3. `GET /src_app_modules_landing_page_categorias_categorias_routes_ts.<hash>.js` → o chunk com os `mock*`.

O NOME do chunk é estável; só o hash muda por deploy → sempre re-descobrir via runtime.

## Estrutura dos dados (`mock*`)

5 arrays, um por categoria (nome do array → slug de rota → nome exibido):

| array | slug (URL) | categoria | nº serviços |
|---|---|---|---|
| `mockCadastro` | `cadastro` | Cadastro | 10 |
| `mockIcms` | `icms` | ICMS | 15 |
| `mockItcd` | `itcmd` | ITCMD | 2 |
| `mockRegimeEspecial` | `regime-especial` | Regime Especial | 5 |
| `mockVeiculo` | `veiculos` | Veículos | 17 |

**Total: 49 serviços.** Cada item:
```js
{ route: 'mei',                          // (mockCadastro usa chaves JS; os outros usam "route" JSON)
  introducao: [{
    titulo: 'Pedido de Inscrição Estadual … MEI',
    descricao: `Este serviço permite … (NUIEF).<br><br><b>Quem Pode Utilizar:</b> …<br><br><b>Setor
                Responsável:</b> …`     // template literal (backtick), HTML autocontido
  }],
  documentos: [...], requisitos: [...], legislacao: [...], canais: [...]   // ignorados na v1
}
```

**Parser:** por categoria (fatia entre `const mock<X> =` e o próximo), casar por serviço
`route → introducao.titulo → introducao.descricao`:
- chaves com/sem aspas: `["']?route["']?\s*:` etc.;
- `descricao` é template literal — capturar entre backticks (0 backticks escapados no dado; alguns
  `${}` aparecem literais, cosmético). O HTML da `descricao` → `html_to_text` (html5ever, lição GO/TO).

## Modelagem (v1)

- **titulo** = `introducao.titulo`; **descricao** = `html_to_text(introducao.descricao)` (rica,
  mediana ~260 chars, com Quem Pode/Setor/Tipo embutidos).
- **classe** = a categoria (Cadastro/ICMS/ITCMD/Regime Especial/Veículos) — enriquece o `text_to_embed`.
- **público** = único "Serviços" (não há eixo de audiência). `ocorrencias` = {Serviços × categoria}.
- **link** = `https://www.sefaz.ap.gov.br/#/categorias/{slug}/{route}`; identidade = o link.
- **órgão** = "SEFAZ-AP". Guard = ≥ ~45 (piso; os `mock*` são estáticos).

## Pontos de decisão (D-AP*)

1. **D-AP-FONTE** — o catálogo rico é **hardcoded no chunk JS** (`mock*`), não em API. Parse do chunk
   (headless-free). Chaves estáveis; hash do chunk descoberto via `runtime.js` a cada rodada.
2. **D-AP-ESCOPO** — v1 = as 5 categorias de `#/categorias` (49 serviços, com descrição). Os links da
   `acesso-rapido` (≈22, sem descrição) ficam de fora (redundantes / pobres).
3. **D-AP-CAMPOS** — v1 usa só `introducao` (titulo+descricao, que já traz Quem Pode/Setor/Tipo). Os
   arrays `documentos`/`requisitos`/`legislacao` ficam para uma eventual v2.
4. **Fragilidade** — se a estrutura do `mock*` mudar (webpack renomear chaves, virar API), o parse
   quebra; o guard de contagem avisa.

## Evidência

Em `scratchpad/ap/`: `home.html`, `main.*.js`, `runtime.js` (mapa de chunks), `cat.js` (o chunk
`categorias_routes` com os `mock*`), `cap*.mjs` (capturas headless que provaram o zero-XHR do detalhe).


---

<a id="df"></a>

# Descoberta — SEFAZ-DF (Distrito Federal), 22ª entidade

## Fonte
- Portal institucional: `https://www.economia.df.gov.br` (Secretaria de Economia / Subsecretaria da Receita — a Fazenda do DF fica sob a SEEC).
- **Carta de Serviços** (fonte real): `https://www.receita.fazenda.df.gov.br/aplicacoes/CartaServicos/`
  - App **ColdFusion** (`.cfm`), HTML server-rendered.
  - `/` e `/index.cfm` → 403 / erro CF (não existem). Entrada útil:
    - `listaSubCategorias.cfm?codCategoriaServico=X&codTipoPessoa=Y` → **página de listagem**
    - `servico.cfm?codServico=Z&codTipoPessoa=Y&codSubCategoria=W` → **detalhe do serviço**

## Achado central — a listagem já é o catálogo inteiro
Qualquer `listaSubCategorias.cfm?...` (independente de `X`/`Y`) devolve **a mesma árvore completa**
de serviços embutida como **objeto JS** na página (~228 KB). Estrutura:

```
'TEMA DE TOPO': { 'item': [
    'Subcategoria - Nome': { 'item': [
        {'url':'/aplicacoes/CartaServicos/servico.cfm?codTipoPessoa=6&codServico=298&codSubCategoria=272',
         'desc':'Solicitar Inclusão de Imóveis'},
        ...
    ]},
    ...
]}
```

- **472 serviços distintos** (`codServico`), com **título** no `'desc'`.
- **163 subcategorias** (chaves `'...': {'item':[...]}`) → candidata natural a **classe**.
- ~25 **temas de topo** (IPTU/TLP, IPVA, ITBI, ITCD, ISENÇÃO ICMS VEÍCULO, PROGRAMA NOTA LEGAL,
  CONTRIBUINTES DE ICMS/ISS, NOTA FISCAL AVULSA, CERTIDÃO CIDADÃO/EMPRESA, ...).
- ⇒ **1 único fetch** enumera todo o catálogo. Sem paginação, sem headless.

## Detalhe do serviço (`servico.cfm`) — descrição rica
Cada detalhe (~70 KB) tem um **accordion** (`div.panel-group#accordion` → `div.panel-body`) com ~5 painéis:
**Descrição**, prazo, requisitos/documentação, canais/como acessar, legislação, arquivos p/ download.
Concatenar os `panel-body` (strip de tags/entidades via html5ever) dá descrição limpa de ~400–1.300 chars.
(Há também um `var states=[...]` de autocomplete com os 472 títulos — ignorar; é ruído.)

## Público (`codTipoPessoa`)
Frequência nos 472: **7**=287, **6**=145, **22**=26, **8**=21. Semântica observada:
- `6` = Pessoa Física / Cidadão (ex. tema "IPTU/TLP")
- `7` = Pessoa Jurídica (títulos com sufixo "- PJ"; tema "IPTU/TLP - PJ")
- `8` = pessoa jurídica/negócios (REDESIM, Junta Comercial, SEBRAE, TERRACAP, TARF)
- `22` = nichos (NOTA FISCAL AVULSA, PRODUTOR RURAL, FEIRANTE AMBULANTE, REFORMA TRIBUTÁRIA)

## TLS / rede — ⚠️ WAF por fingerprint (JA3)
- O host **reseta a conexão do `ureq`** (rustls e native-tls: `Connection reset by peer`), mas responde
  **200 ao `curl`** (OpenSSL) com o mesmo UA/URL → allowlist por **fingerprint TLS (JA3)**, exatamente
  como o GO. Solução: toda a coleta via `kit::http::get_via_curl` (subprocess curl; requer curl no PATH).
- A cadeia de certificados em si fecha (curl `ssl_verify_result=0`); o bloqueio é do ClientHello do ureq.

## Identidade / chaves
- Identidade estável do serviço = `codServico`.
- Link = `https://www.receita.fazenda.df.gov.br/aplicacoes/CartaServicos/servico.cfm?codServico={cs}&codTipoPessoa={tp}&codSubCategoria={sc}`.
- Órgão = "SEFAZ-DF" (Subsecretaria da Receita / SEEC-DF).

## Molde de referência
Novo molde "ColdFusion CartaServicos": listagem = árvore JS única (parse por regex dos tuplos
`{'url':...,'desc':...}` + chave-pai = classe), detalhe = accordion `panel-body`. Extração de texto
= mesmo `html_to_text` (strip + html5ever) de GO/ES/TO/MA/AP/AC.

## Decisões pendentes (levar ao usuário)
1. **Riqueza**: rico (472 fetches de detalhe, ~5 min, cacheado) vs listagem-só (472 títulos + classe, instantâneo).
2. **Público**: split Cidadão/Empresa (via `codTipoPessoa`) vs público único "Serviços".
3. **classe**: subcategoria imediata (parse confiável, 163) — default proposto.


---

<a id="es"></a>

# Relatório de descoberta — Conecta Cidadão / portal.es.gov.br (SEFAZ-ES)

**Data:** 2026-07-05 · **Escopo:** só descoberta (nenhum scraper, nenhuma autenticação). · **Órgão:**
Secretaria de Estado da Fazenda do ES (SEFAZ-ES).

> **TL;DR — o ecossistema migrou para `portal.es.gov.br` (X-Via), e é o MESMO stack do MT.** O
> `conectacidadao.es.gov.br` e o `guiadeservicos.es.gov.br` do enunciado **não existem mais** como app
> próprio: tudo **307-redireciona para `portal.es.gov.br`** (SPA React "ES.GOV" sobre a plataforma
> **X-Via**, igual a `portal.mt.gov.br`). A fonte canônica é a API X-Via, **anônima**: SEFAZ =
> `departmentSlug: "secretaria-de-estado-da-fazenda"`; **`POST /v1/search`** `{groups:["CATALOG"],
> departmentSlug, from, size}` devolve os **45 serviços** já ricos (inclui `serviceLetterContent` = a
> carta completa, inline). **`resultTotal` = 45** é o contador dinâmico (invariante estilo MT).
> Implementação futura = **molde MT** (trocar host + departmentSlug). Contagem nunca hardcode.

---

## Fase 0 — robots.txt / redirecionamentos

| domínio | resultado | leitura |
|---|---|---|
| `conectacidadao.es.gov.br` | `/robots.txt` **307 → portal.es.gov.br**; home e `/Servicos?...` também **307 → portal.es.gov.br** | app antigo **migrado** |
| `guiadeservicos.es.gov.br` | **000** (não resolve / fora do ar) | **morto** |
| `sefaz.es.gov.br` | `/robots.txt` **404** | institucional (sem robots) |

O `portal.es.gov.br` (destino) responde à API anonimamente. Robots do PA (**D-PA-ROBOTS**) cobre o ES
como 2º caso — mitigações idênticas (UA AuliBot, ≥1s, cache, nunca autenticar); na prática a API não
bloqueia.

## Fase 1 — listagem SEFAZ e filtragem (ponto crítico) — **X-Via, molde MT**

`portal.es.gov.br` é uma **SPA React** (CRA: `main.*.chunk.js`) sobre a **X-Via Suite**
(`REACT_APP_PAGE_DESCRIPTION: "Portal web part of the X-Via Suite"`; hosts `*.api.prod.xvia.es.gov.br`).
O bundle expõe a API (base = `https://portal.es.gov.br`, sem prefixo de gateway para o catálogo):

| chamada | uso |
|---|---|
| `GET /v1/department` | 48 órgãos `{id (GUID), slug, name, shortName}` — **SEFAZ**: `slug="secretaria-de-estado-da-fazenda"`, `shortName="SEFAZ"`, `id=a09cdf7d-f164-4a58-8aab-d9cd9fbc3811` |
| **`POST /v1/search`** `{query:"", groups:["CATALOG"], departmentSlug, from, size}` | **listagem do órgão** → array de serviços ricos |
| `GET /v1/category` | categorias/temas |

**Sem paginação necessária:** `size=500` devolve os **45** de uma vez; `from`/`size` existem para
paginar se preciso. **Guard dinâmico:** cada item traz **`resultTotal: 45`** (o total do próprio índice
— idêntico ao invariante do MT: `únicos == resultTotal`). **Anônimo** (sem token/cookie). O `oi`
GUID/`od=SEFAZ` do enunciado era do app antigo — o novo filtro é `departmentSlug`.

## Fase 2 — formatos de URL

- **Legado morto:** `conectacidadao.es.gov.br/Servicos/Detalhes/{id}` → **307 → portal.es.gov.br** (o id
  numérico **não** é preservado). `guiadeservicos.es.gov.br` **não resolve** (000). → nenhum id legado
  reutilizável.
- **Canônico novo:** `https://portal.es.gov.br/servico/{slug}` (rende 200). Identidade do serviço =
  **`slug`** (estável, único; 0 duplicatas nos 45). `link` do snapshot = `…/servico/{slug}`.

## Fase 3 — payload de detalhe — **inline (sem fetch de detalhe)**

O `POST /v1/search` já traz o conteúdo COMPLETO por serviço (como o MT). Mapa campo → snapshot:

| campo | → snapshot | observação |
|---|---|---|
| `title` | `titulo` | — |
| `slug` | identidade + `link` (`/servico/{slug}`) | único |
| `description` | `descricao` (resumo) | texto curto |
| `serviceLetterContent` | `descricao` (rico) | **carta completa**, ~5 KB, **HTML** → decodificar/limpar (html5ever, lição GO/AM) |
| `category` / `categorySlug` | `classe` | 5 categorias (ver Fase 4) |
| `targets[]` | `publico` | normalizar (ver Fase 4) |
| `link` | canal/ação | 36/45 têm; ex. `acessocidadao.es.gov.br/Perfil/Servicos` (transacional) |
| `isOnline` / `isDigital` / `isFree` | atributos | flags |
| `active` | filtro | todos ativos |
| `resultTotal` | guard de contagem | 45 |

Todos os 45 têm `description` e `serviceLetterContent`. Detalhe via `/v1/catalog/{slug}` → 404 (não é
necessário; o conteúdo é inline).

## Fase 4 — público, categorias, agendável

- **Público = `targets[]`** — **precisa de normalização**: os dados trazem `cidadao` **e** `Cidadão`
  (mesmo público, grafias/caixa diferentes) além de `Empresa`. Normalizados por serviço:
  **Cidadão 43 · Empresa 17** (sobrepostos — 15 servem aos dois → `ServicoPerPublico`). Nenhum serviço
  sem target. (Nota de qualidade do portal: a duplicata `cidadao`/`Cidadão` é do dado publicado.)
- **Classe = `category`** — 5: **IMPOSTOS E MULTAS 23 · EMPRESAS 15 · AGRICULTURA E VIDA RURAL 3 ·
  DIREITOS E CIDADANIA 2 · DOCUMENTOS E CERTIDÕES 2**. `ocorrencias` = público × classe.
- **Agendável:** não há flag de público "agendável"; `isOnline`/`isDigital` são atributos (não público)
  — alinhado à decisão do AM (agendável = atributo, não faceta de público). A integração Agenda ES não
  aparece como target no catálogo SEFAZ.

## Fase 5 — cobertura (vs. serviços conhecidos da SEFAZ-ES)

**Cobertura excelente** — os 45 do catálogo incluem os serviços centrais: **CND** ("Certidão Negativa de
Débito ou Positiva com efeito de Negativa"), **DUA** ("Emissão de DUA", "Retificação de DUA"), **IPVA**
(parcelamento, isenção TÁXI/PcD/ônibus, restituição, boleto, não incidência), **ITCMD** (parcelamento,
restituição, apuração da base), **NFAe** ("Emissão de Nota Fiscal Avulsa" / "Avulsa Eletrônica -
Produtor Rural"), **Inscrição/Alteração/Baixa/Reativação Estadual - ICMS**, **Credenciamento ST**,
**AGV - Agência Virtual**, **e-DOCS - Envio de documentos à SEFAZ**, **Parcelamento ICMS**, isenções de
ICMS. → o Conecta/portal.es.gov.br é fonte suficiente; **não precisa** de fonte complementar do site
institucional.

---

## Pontos de decisão a catalogar (candidatos a D-ES*, NÃO decididos aqui)

1. **D-ES-ROBOTS** — coberto por **D-PA-ROBOTS** (ES = 2º caso). Mesmas mitigações; a API não bloqueia.
2. **D-ES-ID/URL** — identidade = **`slug`** (ids legados morreram no redirect); `link` =
   `portal.es.gov.br/servico/{slug}`.
3. **D-ES-ESCOPO** — nasce filtrado por SEFAZ (`departmentSlug`). **Multi-tenant estadual:** 48 órgãos
   sob a MESMA API X-Via (`/v1/department` + `POST /v1/search`) → mesma oportunidade de scraper genérico
   do PA (**D-PA-ACERVO**); registrar o ES como 2º caso.
4. **D-ES-PUBLICO** — normalizar `targets` (`cidadao`/`Cidadão` → Cidadão) antes de mapear para público;
   agendável tratado como atributo (não público), alinhado ao AM.
5. **D-ES-CONTAGEM** — guard = `resultTotal` (45) da resposta; nunca hardcode.
6. **D-ES-MOLDE-MT** — o scraper é **molde MT** (X-Via): `POST /v1/search` com `departmentSlug`, campos
   `title/slug/description/serviceLetterContent/category/targets`, invariante `resultTotal`. Reaproveitar
   a estrutura do `auli-scraper-mt` (trocar host `portal.es.gov.br` + slug SEFAZ + decode do HTML rico).

## Restrições / observações

- **X-Via / React SPA:** API anônima em `portal.es.gov.br/v1/*`; `POST /v1/search` (JSON), `GET
  /v1/department`. Sem `__NEXT_DATA__`, sem sessão.
- **`serviceLetterContent` é HTML** → decodificar entidades + limpar tags (html5ever).
- **Migração recente:** conectacidadao/guiadeservicos redirecionam/morreram; se algum voltar, é só
  espelho do portal.es.gov.br.
- **Sem rate limiting observado**; ainda assim aplicar as mitigações D-PA-ROBOTS.

## Evidência bruta (reprodutível)

Em `scratchpad/es/`: `robots-*.txt`, `portal-home.html`, `main.*.chunk.js` (bundles X-Via),
`dep.json` (48 órgãos), `sefaz-all.json` (45 serviços SEFAZ, `size=500`), `search.json`. Comandos-chave
embutidos acima.

## Critérios de aceitação

- [x] Fases 0–5 executadas com evidência bruta salva e referenciada.
- [x] Ponto crítico (Fase 1): a listagem/detalhe vem por **JSON X-Via** (`POST /v1/search`); filtro SEFAZ
  = `departmentSlug`; conteúdo (inclusive a carta) é **inline**.
- [x] Equivalência guiadeservicos↔conectacidadao: **ambos extintos** — tudo migrou para
  `portal.es.gov.br` (307).
- [x] Contagem dinâmica documentada: **`resultTotal` = 45** (invariante).
- [x] Nenhuma linha de scraper; nenhuma autenticação (Acesso Cidadão intocado).


---

<a id="ma"></a>

# Relatório de descoberta — Portal de Serviços SEFAZ-MA (portal-sgc.sefaz.ma.gov.br)

**Data:** 2026-07-06 · **Escopo:** descoberta + base para a 19ª entidade (descrição rica). · **Órgão:**
Secretaria de Estado da Fazenda do Maranhão (SEFAZ-MA).

> **TL;DR — Angular SPA + API REST Spring Boot (`/sgc/api`), catálogo JSON acessível com token
> ANÔNIMO.** O front loga com **credenciais públicas baked no bundle** (`{id_cliente:"41",
> senha:"<bcrypt>", portal:true}` → `POST /sgc/api/login`) e chama o catálogo com o token no header
> **`AuthorizationPortal`** (não o `Authorization` padrão). Catálogo = **`GET /sgc/api/portal/servicos`**
> (com filtros obrigatórios) → **38 serviços**; a descrição rica vem de **`GET /sgc/api/portal/conteudos/{idConteudo}`**
> (27 têm; 11 são link-only). **JSON é UTF-8** (só corpos de erro são latin1). **⚠️ Gotcha TLS:** o servidor
> manda cadeia incompleta (falta o intermediário GlobalSign) → curl/ureq rejeitam; solução = empacotar
> o intermediário como trust anchor (provado).

---

## Plataforma e auth

- **Angular SPA** (`runtime/polyfills/vendor/main.*.js`, `<base href="/">`) + **API Spring Boot** em
  `/sgc/api` (`apiUrl:"/sgc/api"` no bundle). Erros no formato Spring (`{timestamp,status,error,path}`).
- **Auth = client_credentials ANÔNIMO** (molde GO): o `environment` do bundle traz
  `clientId:"41"`, `secret:"$2a$12$…"` (bcrypt, **público** — servido a todo visitante). O front faz
  `POST /sgc/api/login` body `{id_cliente:"41", senha:"<secret>", portal:true}` → `{authtoken:"Bearer …",
  refreshtoken, …}`. **Sem login de usuário.** O token (JWT efêmero) é re-obtido a cada carga.
- **Header do token:** o interceptor Angular envia `AuthorizationPortal: Bearer <jwt>` (NÃO
  `Authorization` — este último dá 401). Descoberto no `intercept()`: `setHeaders:{AuthorizationPortal:e}`.

## ⚠️ TLS — cadeia de certificado incompleta (o gotcha)

O servidor (`*.sefaz.ma.gov.br`, GlobalSign, TLS 1.3 AES-128-GCM) manda **só a folha** — falta o
intermediário **`GlobalSign GCC R3 DV TLS CA 2020`** (a raiz GlobalSign R3 está no store do sistema).
curl/ureq/rustls rejeitam ("unable to verify the first certificate"); o browser passa via AIA-fetch.
**Solução (provada):** baixar o intermediário do AIA
(`http://secure.globalsign.com/cacert/gsgccr3dvtlsca2020.crt`) e **adicioná-lo como trust anchor**. No
Rust: `ureq::tls::RootCerts::new_with_certs(&[intermediário])` (rustls trata como anchor; a folha
encadeia direto nele). **Não** precisa de native-tls (o cipher é moderno). O PEM fica embutido no crate.

## Catálogo — `GET /sgc/api/portal/servicos`

Params OBRIGATÓRIOS (sem eles → 500): `flgPublicado=true&flgLocal=PORTAL&notOutros=false&page=0&pageSize=N`
(+ `nomeServico=&flgTipoServico=&flgDestaqueNovo=&flgPaginaPrincipal=&sortOrder=&sortField=`). Resposta
`{items:[…], total:38}`. **38 serviços.** `pageSize=1000` traz todos numa GET; **guard dinâmico = `total`**.

Campos do item (listagem magra):

| campo | → snapshot | observação |
|---|---|---|
| `id` | identidade | inteiro |
| `nomeServico` | `titulo` | — |
| `flgTipoServico` | **público** | `COMPANY`/`CITIZEN`/`PUBLIC_AGENCY`/`CERTIFICATE` |
| `idConteudo` | busca da descrição rica | 27/38 têm; 11 são link-only |
| `linkExterno` | `link` (quando houver) | destino externo (11) |
| `idServicoCategoria` | — | **0 em todos** → sem categoria (classe única "Geral") |

## Descrição rica — `GET /sgc/api/portal/conteudos/{idConteudo}`

Para os 27 com `idConteudo`: → `{titulo, descricao (HTML), introducao, …}`. A `descricao` é **HTML** →
`html_to_text` (html5ever, lição GO/ES/TO). Os 11 sem `idConteudo` (link-only) ficam só com título +
`linkExterno`.

## Modelagem (Cenário B por público, classe única)

- **titulo** = `nomeServico`; **descricao** = conteúdo (`conteudos/{idConteudo}.descricao`, HTML→texto);
  vazia p/ link-only.
- **público** = `flgTipoServico` mapeado: `COMPANY`→Empresa, `CITIZEN`→Cidadão, `PUBLIC_AGENCY`→Órgão
  Público, `CERTIFICATE`→Certidões. **1 público por serviço**. `classe` = "Geral" (não há categoria).
- **link** = `linkExterno` quando houver; senão `…/portal/conteudo/{idConteudo}`; senão `…/portal/servicos`.
- **identidade** = `id`; **órgão** = "SEFAZ-MA". `total` do catálogo = guard.

## Pontos de decisão (D-MA*)

1. **D-MA-TLS** — empacotar o intermediário GlobalSign como trust anchor (rustls). Documentar como
   exceção (o servidor deveria mandar a cadeia completa). Se o cert for reemitido por outro
   intermediário, o guard/erro de handshake avisa.
2. **D-MA-AUTH** — creds públicas baked (id_cliente=41 + secret bcrypt) → token anônimo; header
   `AuthorizationPortal`. **Não são segredo** (bundle público) — comentar no código p/ scanners (lição GO).
3. **D-MA-RICO** — buscar os 27 conteúdos (descrição rica) — decisão do usuário.
4. **D-MA-CONTAGEM** — guard = `total` da resposta (38 hoje), dinâmico.
5. **Escopo** — só SEFAZ (portal é da própria SEFAZ; `flgLocal=PORTAL`). Não multi-órgão.

## Evidência

Em `scratchpad/ma/`: `main.*.js` (bundle Angular, creds + apiUrl + interceptor), `all.json` (38
serviços), conteudo 3171, `inter.pem` (intermediário GlobalSign), `cap.mjs` (captura do XHR real).


---

<a id="pa"></a>

# Relatório de descoberta — ecossistema de serviços da SEFA-PA

**Data:** 2026-07-05 · **Escopo:** só descoberta (nenhum scraper, nenhuma autenticação). · **Órgão:**
Secretaria de Estado da Fazenda do Pará (SEFA-PA).

> **TL;DR — fonte canônica recomendada: `paradigital` (API Prodepa).** Das 4 fontes mapeadas, a única
> **anônima, estruturada e rica** hoje é o catálogo estadual **paradigital**, servido pela API Spring
> da Prodepa em `para-digital.sistemas.pa.gov.br`. A SEFA é o **órgão 48**: `GET /orgao/48` lista os
> **34 serviços**; `GET /servico/{id}` traz o detalhe completo (finalidade, etapas, requisitos,
> contatos, público, taxa, link). O candidato primário **portal-digital está FORA DO AR** (Cloudflare
> 522) e o **Joomla foi extinto** (o site SEFA virou SPA Angular + WordPress fechado). Nenhum conteúdo
> de carta de serviços exige login — só a *execução* (link para o `pservicos`).

---

## Fase 0 — robots.txt (documental, NÃO usado como gate)

| domínio | robots | resumo |
|---|---|---|
| `portal-digital.sefa.pa.gov.br` | **522** (Cloudflare, origin down) | inacessível — ver Fase 1 |
| `www.paradigital.pa.gov.br` | **200** | `User-agent: *` **sem `Disallow`** (libera tudo) + `Sitemap`. Sem bloqueio dirigido a bots/IA. |
| `www.sefa.pa.gov.br` | **410 Gone** | não publica robots (site novo, SPA Angular) |
| `app.sefa.pa.gov.br` | falha TLS | portal transacional; fora de escopo |

A **API canônica** (`para-digital.sistemas.pa.gov.br`) responde a tudo anonimamente; o front
(`paradigital`) libera por robots. Não há conflito com a decisão do mantenedor de desconsiderar robots
(**D-PA-ROBOTS**, abaixo) — na prática nada relevante é proibido.

## Fase 1 — portal-digital (candidato primário) — **FORA DO AR**

`portal-digital.sefa.pa.gov.br` retorna **HTTP 522** (Cloudflare "connection timed out" origin→CDN) em
**todas** as rotas, inclusive `/favicon.ico`, com UA de browser — ou seja **origin caído**, não
bot-block. DNS resolve para Cloudflare. Era uma SPA SEFA-específica com ids **MongoDB ObjectId** (24
hex, ex. `623d1fe9b820e763c7802275`). **Não é a mesma plataforma do paradigital** (que usa ids inteiros
/ backend Spring-Prodepa) → a hipótese multi-tenant #1↔#2 fica **refutada** pela evidência de id/stack.
Recomendação: **monitorar**; se voltar, revisitar (pode ter conteúdo SEFA-específico), mas hoje é
inutilizável.

## Fase 2 — paradigital (API Prodepa) — **confirmada, anônima**

`www.paradigital.pa.gov.br` é uma **SPA Quasar/Vue** (`#q-app`, `app.270248a1.js`). O bundle revela o
interceptor axios que monta toda URL como:

```
BASE = https://para-digital.sistemas.pa.gov.br/para-digital-service/portal
```

Endpoints (todos **GET, anônimos**, `Content-Type: application/json`, sem sessão/cookie):

| endpoint | resposta | uso |
|---|---|---|
| `GET /orgao/` | `[{idOrgao, nome, …}]` — **63 órgãos** | descobrir o id da SEFA (= **48**) |
| `GET /orgao/{idOrgao}` | `[{id, nome}]` — **serviços do órgão** | **listagem SEFA: 34 serviços** |
| `GET /servico/{id}` | objeto rico (ver Fase 3) | detalhe |
| `GET /categoria/` | temas/categorias | classe |
| `GET /buscageral?pageSize&pageNumber&nome=X` | busca paginada | busca (exige `nome`) |
| `GET /top10` | destaques | — |

`POST /servico/list/` responde **500** (método não suportado) — a listagem real é por órgão. O tenant
não é header/param: cada catálogo é um **órgão** (`/orgao/{id}`). **Multi-tenant estadual verdadeiro:**
os 63 órgãos usam o MESMO contrato → ver **D-PA-ACERVO**.

## Fase 3 — payload de detalhe (`GET /servico/{id}`) — mapa de campos

Verificado nos 34 serviços SEFA (100% preenchidos). Mapa API → snapshot:

| campo API | → snapshot | observação |
|---|---|---|
| `nome` | `titulo` | — |
| `finalidade` | `descricao` (base) | "o que é", texto plano |
| `etapaServicos[].descricao` | passo a passo | lista ordenada (rico) |
| `requisitoServicos[].descricao` | documentos/requisitos | lista |
| `contatos[].contato` + `tipoContato.descricao` | contato | telefone/email/etc. |
| `tema.descricao` | `classe` | SEFA: **todos "Tributos e empresas"** (tema único) |
| `cidadao` / `empresa` / `estado` (bool) | `publico` | **sobrepostos** — ver distribuição |
| `linkAcesso` | `link` (ação) | 18 → `app.sefa` (transacional/login), 6 externos, 10 vazios |
| `taxa` / `valor` | taxa | — |
| `digital` / `presencial` / `site` | canal | — |
| `ativo` | filtro | 34/34 ativos |
| `orgaoCadastrado.idOrgao` (48) / `.site` | órgão | "SEFA-PA" |

**Distribuições (34 serviços SEFA, 2026-07-05):** público **Empresa 30 · Cidadão 21 · Estado/Órgão 3**
(um serviço pode ter mais de um → `ServicoPerPublico`). Tema: 1 (“Tributos e empresas”) — para a SEFA
a `classe` do paradigital não discrimina (todos iguais). Profundidade: **100%** têm finalidade +
etapas + requisitos + contatos.

## Fase 4 — Manual de Atendimento (Joomla) — **EXTINTO**

`www.sefa.pa.gov.br` **não é mais Joomla**: foi reconstruído como **SPA Angular** (`main.*.js`,
`data-critters-container`, Cloudflare Turnstile). As URLs antigas do Manual
(`/index.php/orientacoes/…`, `/27-orientacoes/manual-de-atendimento/941-…`) retornam **o mesmo shell
Angular (31 649 b)** — o conteúdo com campos padronizados ("Quem pode solicitar", "Documentos
necessários", "Taxa"…) **não existe mais publicamente**. O novo backend é **WordPress**
(`site-sefa-wordpress.sefa.pa.gov.br/wp-json/wp/v2`) mas está **fechado (HTTP 401)**; a app aponta
"serviços" para o `pservicos` (transacional/login). → a fonte #3 do enunciado **não é mais aproveitável**.

## Fase 5 — comparação e escolha da fonte canônica

| critério | portal-digital (#1) | **paradigital (#2)** | Joomla/SEFA-site (#3) | pservicos (#4) |
|---|---|---|---|---|
| disponível anônimo | ❌ 522 (down) | ✅ **sim** | ❌ extinto/401 | ❌ login |
| estruturado (JSON) | ? (down) | ✅ **rico** | — | — |
| cobertura SEFA | ? | **34 serviços** | — | ~70–85 (transacional) |
| profundidade | ? | **finalidade+etapas+requisitos+contatos (100%)** | — | — |
| frescor | ? | serviços ativos, dados atuais | — | — |

**Recomendação: `paradigital` como fonte canônica** — `GET /orgao/48` (índice) + `GET /servico/{id}`
(conteúdo). É a única fonte pública, estruturada e rica hoje. O `pservicos` entra só como **destino de
link** (`linkAcesso`), nunca autenticado.

---

## Pontos de decisão a catalogar (candidatos a D-PA*, NÃO decididos aqui)

1. **D-PA-ROBOTS** — desconsiderar robots já decidido pelo mantenedor (conteúdo público, LAI, baixo
   volume). Formalizar com as mitigações obrigatórias no futuro scraper: **UA identificado do projeto**
   (`AuliBot/…+repo+email`, nunca UA de browser falso), **rate-limit ≥1s sem paralelismo por host**,
   **cache agressivo** (cada URL 1×/execução; `--usecache` miss=erro), **nunca autenticar**. Observação:
   o paradigital libera por robots, então a decisão é preventiva, não necessária aqui.
2. **D-PA-FONTE** — canônica = **paradigital** (`/orgao/48` + `/servico/{id}`). portal-digital (down) e
   Joomla (extinto) descartados; reavaliar se o portal-digital voltar.
3. **D-PA-RESTRITO** — nenhum *conteúdo* exige login (tudo público na API); só a *execução* aponta para
   o `pservicos` via `linkAcesso`. Representar `linkAcesso` vazio (10/34) / transacional (18/34) como
   metadado; serviço nunca é "link-only" de conteúdo. `link` do snapshot = `linkAcesso` quando houver.
4. **D-PA-ACERVO** — o paradigital é um **catálogo estadual multi-tenant** (63 órgãos, MESMO contrato
   `/orgao/{id}` + `/servico/{id}`). Oportunidade forte para um **scraper estadual genérico** (Acervo):
   um crate parametrizado por `idOrgao` cobriria os 63 órgãos sem novo código. Registrar; não implementar.
5. **D-PA-CONTAGEM** — guard dinâmico = `len(GET /orgao/48)` (34 hoje); **nunca** os números da imprensa
   (70/72/74/85, que são do `pservicos` transacional). Classe da SEFA é tema único → não usar como guard.
6. **D-PA-NAMING** — órgão = **"SEFA-PA"** (Secretaria da Fazenda; separador sigla–UF com `-`, política
   da frota). Público via flags → **Cidadão / Empresa / Estado** (`ServicoPerPublico`, sobrepostos).

## Restrições / observações

- **API Spring-Prodepa** (`para-digital.sistemas.pa.gov.br/para-digital-service/portal`): GET anônimo,
  respostas JSON; `booleans vêm como strings` `"True"/"False"`; sem paginação em `/orgao/{id}`.
- **Sem rota `/servico/:id` no front** paradigital (detalhe é modal em `/servicoOrgao`) → link canônico
  cidadão = `linkAcesso` (quando houver) ou a página do órgão.
- **portal-digital instável** (522 no dia da coleta) — se for reincorporado, medir de novo.
- **`app.sefa.pa.gov.br`** teve falha de TLS na Fase 0; é transacional (login), fora de escopo.

## Evidência bruta (reprodutível)

Em `scratchpad/pa/`: `robots-*.txt`, `pdg-home.html` + `pdg-app.js` (bundle Quasar), `orgaos.json`
(63 órgãos), `orgao48.json` (34 serviços SEFA), `det/*.json` (34 detalhes), `sefa-main.js` (bundle
Angular do site novo), `wptypes-*.json` (WP 401). Endpoints e exemplos embutidos acima.

## Critérios de aceitação

- [x] Fases 0–5 executadas com evidência bruta salva e referenciada.
- [x] Ponto crítico (Fase 1) resolvido: portal-digital está fora do ar; **existe** API JSON utilizável
  sem sessão — a do **paradigital** (Fase 2).
- [x] Hipótese multi-tenant #1↔#2 **refutada** (plataformas/ids distintos); porém o paradigital É
  multi-tenant estadual (63 órgãos, mesmo contrato).
- [x] Recomendação de fonte canônica fundamentada (cobertura + profundidade + frescor + disponibilidade).
- [x] Nenhuma linha de scraper; nenhuma tentativa de autenticação.


---

<a id="pb"></a>

# Descoberta — SEFAZ-PB (Paraíba), 24ª entidade

## Fonte
- Portal institucional: `https://www.sefaz.pb.gov.br/` = **Joomla** (generator "Envolute", `com_content`).
- **Carta de Serviços** (fonte real): `https://cartaservico.sefaz.pb.gov.br/` — app **PHP** server-rendered
  (Apache, PHP/7.4). `servicos.php` = listagem; `saibamais.php?id=N` = ficha do serviço.

## Listagem (`servicos.php`)
- Accordion aninhado: **categoria de topo → público ("Para Empresa"/"Para o Cidadão") → subcategoria →
  serviço** (`<li><a href="saibamais.php?id=N">Título</a></li>`).
- **101 serviços** (id 1..101). Cada id aparece **2×** na listagem (uma árvore por público) → dedup por id.
- `classe` = a subcategoria imediata = o botão de accordion mais próximo ANTES do link que **não** é
  rótulo de público. ~51 classes distintas (fino mas fiel, como DF/142).

## Ficha (`saibamais.php?id=N`) — rica
Ficha estruturada com pares `<h3>Rótulo:</h3><h6 class="h6">Valor</h6>`:
**O que é o serviço**, **Público-alvo**, Forma de prestação, Taxa, Agendamento, Exigências, Quanto
tempo leva, Etapas do serviço, Documentação necessária, Unidades Físicas, Horário de Atendimento,
Contato, Informações adicionais. Além disso:
- **título** em `<div class="inputbutton01" title="…">` (nome completo do serviço);
- **link real** do serviço em `onclick="redireciona('id','URL')"` (o botão ACESSAR SERVIÇO).

## Modelagem (molde TO/DF, rico)
- `titulo` e `descricao` vêm do detalhe; descrição = os pares (menos o Público-alvo) + "Acessar o
  serviço: {URL}"; campos vazios ("-") descartados.
- **público** = campo "Público-alvo" da ficha (lista por vírgula → Cidadão/Empresa, per-serviço).
- `classe` = subcategoria da listagem; `link` = `saibamais.php?id=N` (único, identidade); órgão SEFAZ-PB.
- `ocorrencias` = público-alvo × classe (1–2 por serviço).

## Rede / TLS
- `cartaservico.sefaz.pb.gov.br` responde 200 em HTTPS padrão (Apache); UA AuliBot aceito. Sem gotcha
  de fingerprint (ureq funciona, diferente de GO/DF).

## Decisão
Sem fork relevante: catálogo genuinamente rico (101 fichas estruturadas) → **buscar os 101 detalhes**
(molde TO/DF), público vindo do próprio dado. Sem headless.


---

<a id="rn"></a>

# Descoberta — SEFAZ-RN (Rio Grande do Norte)

## Fonte
- Portal: `https://www.sefaz.rn.gov.br/` = **WordPress** (tema `govrn_adi`) hospedando um **SPA React**
  (create-react-app: `static/js/main.chunk.js`). Conteúdo renderizado client-side; o mesmo shell de
  8,6 KB é servido para qualquer rota (`/servicos/...` etc.).
- **WP REST API pública** (`/wp-json/`, 200). O React consome custom post types via `wp/v2`.

## O que existe de "serviço"
- CPT **`servicos`** (`/wp-json/wp/v2/servicos`, **X-WP-Total = 15**): são **cards de atalho** da home,
  não uma Carta. Cada item = `title` + `acf.categories` (taxonomia WP) + `acf.link` (destino) +
  `acf.local_exibicao` (DESTAQUE / MAIS ACESSADOS). **Sem `content`/`excerpt`** (ambos vazios).
  - Destinos: **5** apontam para `/postagem/...` (posts WP com corpo rico em `acf['Matéria']`);
    **9** para apps externos (`uvt.sefaz.rn.gov.br/#/services/...`, `sei.rn.gov.br`, `np.sefaz...`);
    **1** ("Carta de Serviços") tem `acf.link = false` (sem destino).
- CPT **`central-servico`** = 1 item ("Central de serviços SEFAZ"), `acf = []` (vazio).
- CPT **`postagem`** = posts informativos (notícias/calendários/legislação); corpo em **`acf['Matéria']`**
  (o `content.rendered` é `null`). Categoria "Finanças e Impostos" tem 143 posts. É conteúdo de
  notícia/informação, **não** um catálogo de serviços estruturado.

## UVT (o "catálogo transacional")
- `uvt.sefaz.rn.gov.br` = app **AngularJS / IIS** (Microsoft-IIS/10.0; `js/core.js`+`components.js`
  hand-built, rotas `#/services/...` com `templateUrl` hardcoded) sobre `usuarios-api.sefaz.rn.gov.br`.
- É **transacional** (login, emitir certidão, consultar contribuinte, agendamento) — **não** expõe uma
  lista/catálogo público de serviços com descrições. `usuarios-api` sem swagger; `/api/servicos` = 404.

## Conclusão
RN **não tem uma Carta de Serviços descritiva** (como DF/CE/BA/TO). O único catálogo estruturado é o
CPT `servicos` = **15 cards menu-only** (título + categoria + link, sem descrição própria). Isso é o
molde **"vazia (menu-only)"** da frota (RJ/PE) — só que via API JSON limpa (WP REST), não HTML.

## Decisões (levar ao usuário)
1. **A — Menu-only (15 cards, descrição vazia):** molde RJ/PE. Título + categoria + link via WP REST.
   Rápido, consistente, mas 15 itens e a maioria são só links externos.
2. **B — Menu-only + enriquecer os 5 que apontam para `/postagem/`** (buscar `acf['Matéria']`): 5 ricos,
   10 só link. Aproveita o conteúdo disponível, mas fica inconsistente (descrição só em 1/3).
3. **C — Não integrar RN agora:** não há Carta descritiva; o catálogo real (UVT) é transacional e fora
   do escopo. Reavaliar se/quando o RN publicar uma Carta.


---

<a id="ro"></a>

# Relatório de descoberta — Agência Virtual SEFIN-RO

**Data:** 2026-07-05 · **Escopo:** só descoberta (nenhum scraper, nenhum refactor do CE, nenhuma
autenticação). · **Órgão:** Secretaria de Estado de Finanças de Rondônia (SEFIN-RO).

> **TL;DR — VEREDITO: RO é Sydle ONE, MESMA GERAÇÃO do PI (produto "conecta-360"), NÃO a do CE.** A
> hipótese "mesma plataforma do CE" está **parcialmente certa** (é Sydle ONE) mas **imprecisa na
> mecânica**: o CE usa a geração antiga (`getChildren` POST); **RO e PI usam a geração nova
> (`_search` GET)** e compartilham os MESMOS ObjectIds de classe de plataforma (conteúdo
> `5cd32901…`, catálogo `5ca3bca7…`, classification `5d66ec59…`). Fonte canônica = `GET _search`
> anônimo em `sydleone.sefin.ro.gov.br`, catálogo **"Serviços" = 194 serviços**. A implementação é
> **parametrização do `auli-scraper-pi`** (não do CE). Referências de confronto no repo:
> `auli-scraper-ce/src/ce.rs` e `auli-scraper-pi/src/pi.rs`.

---

## Fase 0 — robots

| domínio | robots | resumo |
|---|---|---|
| `agenciavirtual.sefin.ro.gov.br` | **200** | `User-agent: * / Allow: *`; `Disallow: /my-panel/`, `/t/` (libera o catálogo); Sitemap `sitemap_conecta-360.xml` (o produto Sydle "conecta-360") |
| `www.sefin.ro.gov.br` | **404** | JSP legado, sem robots |

Sem bloqueio ao catálogo. D-PA-ROBOTS cobre o RO como 3º caso preventivo (mitigações), embora aqui não
seja necessário.

## Fase 1 — confronto com o CE (ponto crítico) — **RO ≈ PI, ≠ CE**

O shell de `agenciavirtual.sefin.ro.gov.br/` traz `window.SYDLE.config` (Sydle ONE, como CE/PI):

```
BASE_API_URL          = https://sydleone.sefin.ro.gov.br/api/1/
APPLICATION_IDENTIFIER = servicedesk-embedded   (+ servicedesk-embedded-guest p/ /one-form-guest)
activityIdentifier    = conecta-360
useCookieAuthentication = false  + Bearer anônimo embutido (efêmero → re-extrair do shell)
catalogs[] (classe 5ca3bca7d18bdb280c9c6c2c): 4 catálogos
```

**Teste dos dois contratos contra o RO (mesmo Bearer):**

| contrato | chamada | resultado |
|---|---|---|
| **PI (novo)** | `GET …/servicedesk-embedded/_classId/5cd32901…/_search?_body={bool:{must:[active,parent._id]}}` | **200** `{hits:{total,…}}` ✅ |
| CE (antigo) | `…/getChildren` (classe `5cd5e83d…`) | **400** `applicationIdentifier inválido` ❌ |

**Classes de plataforma idênticas RO ↔ PI** (prova de mesma geração): conteúdo
`5cd32901df14eb3d461160f0`, catálogo `5ca3bca7d18bdb280c9c6c2c`, classification
`5d66ec59e7c98a556a48d945`, getPageContent `645be3fb…`, sd-components `647a2d0f…`. O CE tem outros ids
(catálogo `5cd5e83d…`, sem classification) e outra mecânica. **Tenant:** o RO não usa header de conta
(o CE usa `X-Explorer-Account-Token: sefazce` + app `sefaz-ceara`); no RO o isolamento é por **host**
(`sydleone.sefin.ro.gov.br`) + app `servicedesk-embedded`. ObjectIds do RO ~abr/2024 (`662c18…`) vs
CE ~2023 — implantações sucessivas da mesma stack.

## Fase 2 — os 4 catálogos (classe `5ca3bca7…`, contagem dinâmica via `_search size:0`)

| `_id` catálogo | nome | serviços ativos | escopo |
|---|---|---|---|
| `662c1875ee982159b7b199c9` | **Serviços** | **194** | ✅ o deliverable |
| `662c1859ee982159b79a6479` | Temas | 42 | informativo — excluir |
| `662c1891ee982159b7b943d5` | Conteúdos | 28 | informativo — excluir |
| `6638f6984ad5362fc7eefe4e` | Dê sua Sugestão | 1 | formulário — excluir |

**Contagem = `hits.total` do `_search` filtrado por `parent._id` do catálogo** (dinâmico, nunca
hardcode). Sem paginação necessária (`size:500` traz os 194; terminar na página vazia se algum dia
paginar). **Escopo recomendado: só "Serviços"** — igual à decisão do CE (só `servico-geral`); Temas e
Conteúdos são informativos.

## Fase 3 — payload de detalhe + diff RO × CE

`_search` no catálogo Serviços já traz o item completo (sem chamada de detalhe). Campos usados:

| campo (RO, `_source`) | → snapshot | observação |
|---|---|---|
| `_id` (24-hex) | identidade | estável |
| `identifier` (slug) | link | ex.: `requisitar-csc` |
| `name` | `titulo` | — |
| `description` | `descricao` | **curta** (mediana 62, máx 290 chars) — todas não-vazias |
| `classification` (ref) | `classe` | classe `5d66ec59…` **NÃO resolvível anon (403)** → cai em "Geral" |
| `tags` | público | **null** em todos → sem eixo de público (Cenário A) |
| `contentHtml`/`contentMarkdown`/`content` | (rico, opcional) | conteúdo completo inline — v2 possível (como AM), fora do escopo base |

**Link canônico** = `https://agenciavirtual.sefin.ro.gov.br/catalogo-servicos+{identifier}+{_id}`
(rende 200 — formato do enunciado confirmado).

**Diff RO × CE:**

| | CE (`ce.rs`) | **RO** |
|---|---|---|
| mecânica | `getChildren` (POST) | **`_search` (GET, `?_body=` ES)** |
| app (URL) | `servicedesk-embedded` (+ body `sefaz-ceara`) | `servicedesk-embedded` |
| tenant | header `X-Explorer-Account-Token: sefazce` | **host** (`sydleone.sefin.ro.gov.br`), sem header |
| classe conteúdo | (getChildren no catálogo `648af76…`) | conteúdo `5cd32901…` / catálogo `5ca3bca7…` |
| classification | ausente | presente (mas 403 anon) |
| link | `servico-geral+{id}+{_id}` | `catalogo-servicos+{identifier}+{_id}` |
| público | Cenário A | Cenário A |

→ **RO = molde PI** (mesma geração), com host/catálogo/prefixo-de-link próprios.

## Fase 4 — público e escopo

- **Público:** `tags` = null em todos os 194 → **Cenário A** (público único "Serviços", como CE/PI).
  Não há filtro de público na URL nem campo estruturado. `agendável` não aparece como faceta.
- **Escopo dos tipos:** só **Serviços** (194). Temas (42) e Conteúdos (28) são informativos (repasses
  ICMS/IPVA, bases de cálculo, divulgações) — **excluir**, consistente com o CE.

## Fase 5 — destinos link-only

As descrições/conteúdos apontam para os sistemas transacionais (todos `*.sefin.ro.gov.br`):
`dare` (DARE), `ipva`, `epat` (processo eletrônico), `det` (domicílio eletrônico), `portalcontribuinte`,
`ssocontribuinte`/`sitafe-sso` (SSO — **login, nunca autenticar**, atenção à IN nº 29/2026),
`legislacao`. Esses são **destinos link-only**; o scraper registra a URL, jamais autentica. O `link`
canônico do snapshot é a página da Agência Virtual (`catalogo-servicos+…`), não o sistema destino.

---

## Pontos de decisão a catalogar (candidatos a D-RO*/D-XX)

1. **D-XX-SYDLE-COMPARTILHADO (a decisão mais importante):** **PI e RO são a MESMA geração Sydle ONE
   (conecta-360)** — mesmo contrato `_search`, mesmas classes de plataforma. Um scraper parametrizável
   por `{BASE_API_URL, app, catálogo_id, prefixo-de-link}` cobriria PI + RO (e futuros conecta-360). **O
   CE NÃO entra nessa parametrização** — é a geração antiga (`getChildren`, classes diferentes, header de
   conta). Prós de parametrizar PI/RO: um crate, N estados; contras: acoplar dois estados a um contrato
   de terceiros que evolui. **NÃO decidir/refatorar aqui.** (O "3º tenant PA/portal-digital" do enunciado
   ficou **inconclusivo**: na descoberta do PA aquele host estava fora do ar (522) e usamos o
   paradigital/Prodepa — outra plataforma; ver `descobertas.md#pa`.)
2. **Escopo dos tipos** — só `catalogo-servicos` (194), como o CE. Temas/Conteúdos fora.
3. **Id canônico** — `_id` (24-hex) estável; `identifier` (slug) estável. Link usa ambos
   (`{identifier}+{_id}`). Igual CE/PI.
4. **Guard de contagem** — `hits.total` do `_search` no catálogo Serviços (194 hoje), dinâmico.
5. **Descrição curta vs rica** — a base usa `description` (curta, como CE/PI); há `contentHtml` inline
   para uma eventual v2 rica (mesma oportunidade do AM/D-AM-V2). Registrar, não implementar.

## Restrições / observações

- **Sydle ONE conecta-360:** Bearer anônimo efêmero (re-extrair do shell a cada rodada, como CE/PI);
  `classification` 403 anon (classe = "Geral"); `_search` é **GET** (corpo ES url-encoded em `?_body=`).
- **Sem WAF/rate-limit observado** (host próprio `sydleone.sefin.ro.gov.br`, resposta rápida). Aplicar
  as mitigações D-PA-ROBOTS mesmo assim.
- **Nunca autenticar** — reforçado pela IN nº 29/2026 (novo modelo de acesso logado desde 15/06/2026).

## Evidência bruta (reprodutível)

Em `scratchpad/ro/`: `robots-*.txt`, `av-home.html` (shell Sydle), `token.txt`, `svcs.json` (194
serviços do catálogo Serviços), `s3.json`/`full.json` (amostras de detalhe), `a.json` (prova do
`_search`). Referências de confronto: `auli-server/crates/scrapers/auli-scraper-{ce,pi}/src/{ce,pi}.rs`.

## Critérios de aceitação

- [x] Relatório/contrato do CE localizado e usado como confronto (`auli-scraper-ce/src/ce.rs`; + PI).
- [x] Veredito explícito: **RO = Sydle ONE geração conecta-360 = molde PI (≠ CE)**, com payloads salvos.
- [x] Fases 0–5 executadas com evidência bruta salva.
- [x] Contagens dinâmicas por catálogo documentadas (via `hits.total` do `_search`).
- [x] Nenhuma linha de scraper; nenhum refactor do CE; nenhuma autenticação.


---

<a id="rr"></a>

# Descoberta — SEFAZ-RR (Roraima), 27ª entidade

## Fonte
- Portal: `https://www.sefaz.rr.gov.br/` = site **custom/estático** ("Portal de Aplicações"; JS puro
  `/script.js`, sem CMS/framework). O nav institucional só tem Ouvidoria / Transparência / Downloads —
  **nenhuma seção "Serviços" server-rendered**, nenhuma rota `/servicos` (404).
- Os serviços são apps transacionais **GeneXus/SIATE** em `portalweb.sefaz.rr.gov.br` (CND, DARE,
  Sintegra, CIPE, GIM…) — sem página "Central de Serviços" (tudo 404) e sem catálogo descritivo próprio.

## Achado central — catálogo embutido no `script.js`
O `script.js` da home traz um **array `const apps = [{category, title, description, href}, …]`** — um
catálogo **estruturado com descrição** (molde AP, mas array limpo, chaves não-minificadas):
```js
const apps = [
  { category: "cidadao", title: "Certidão Negativa",
    description: "Emite a certidão negativa de débitos estaduais para consulta e comprovação…",
    href: "https://portalweb.sefaz.rr.gov.br/cnd/servlet/wp_siate_emitircndcentralservicopublica" },
  …
];
```
- **20 entradas** → **16 hrefs distintos** (4 serviços aparecem em `cidadao` E `empresa`: Certidão
  Negativa, Consultar Pagamento DARE, Emissão CIPE, Validação CIPE → 2 ocorrências cada).
- `category`: empresa (14) / cidadao (6). `description`: 1 linha real (~71–108 chars, "curta").
  `href`: o app real (portalweb GeneXus, ou externo: Detran-RR, IBGE CNAE).

## Modelagem (molde AP/RN, parse do array JS)
- `titulo` = `title`; `descricao` = `description` (curta); **público** = `category` (Cidadão/Empresa);
  `classe` = "Serviços" (não há eixo de tema); `link` = `href` (identidade, dedup); `ocorrencias` =
  público × classe (o serviço em 2 categorias vira 2 ocorrências).
- 16 serviços, 20 ocorrências, 2 públicos.

## FAQ (não usado)
Há um chatbot `faq-chat.php?q=` com Q&A, mas é um **matcher** (retorna a melhor resposta), não um
catálogo listável — fora de escopo por ora.

## Rede
- Apache/HTTP2, UA AuliBot aceito. `script.js` é pequeno; sem gotcha aparente.

## Decisão
Fonte única e limpa (o `apps[]`) → parse direto, sem headless. Descrições curtas (o teto da fonte; os
apps GeneXus são transacionais, sem detalhe rico). Público = categoria.


---

<a id="se"></a>

# Descoberta — SEFAZ-SE (Sergipe), 26ª entidade

## Fonte
- Portal: `https://www.sefaz.se.gov.br/` = **SharePoint 2013** on-prem (MicrosoftSharePointTeamServices
  15.0; molde do PE). O REST `_api` é anônimo (parcial), mas o catálogo real é uma **página HTML**.
- Menu **SERVIÇOS → CARTAS DE SERVIÇOS** (`manuais_servicos.aspx`) aponta para o catálogo do cidadão:
  **`https://www.sefaz.se.gov.br/SitePages/servicos_cidadao.aspx`** (única página, ~890 KB).
- (Becos sem saída: "Servicos Importantes" = lista SP de só 12 atalhos; "Biblioteca de Servicos" = library
  de PDFs/ZIPs por tema — arquivos, não texto; `servicos_empresa.aspx` = 404. Tudo isso NÃO é a Carta.)

## `servicos_cidadao.aspx` — a Carta rica
Página única, **Bootstrap accordion**. **91 serviços**, cada um um painel:
- **Título** no heading: `<a href="#{id}">▾ Título</a>` (o `id` é a âncora/identidade do serviço).
- **Corpo** em `<div class="panel-collapse collapse" id="{id}"><div class="panel-body">` com campos
  `<p><strong>Rótulo:</strong> valor</p>`: Descrição do serviço, Legislação vinculada, Área responsável,
  Subsecretaria responsável, Requisitos exigidos, Onde solicitar, Forma de prestação, Canais de
  relacionamento, etc. Corpo típico ~900 chars (mediana), rico.

## Classe (tema)
Os serviços estão agrupados em 7 accordions-tema (`id="accordion_<tema>"`): DFe, ICMS, ITCMD, IPVA,
Simples Nacional, Contencioso, Cadastro de Contribuinte. Os 9 serviços "standalone" (Plantão Fiscal,
Consultas Tributárias, …) vêm ANTES do 1º tema → `classe = tema imediatamente anterior; senão "Serviços
Gerais"`. Distribuição: Cadastro 33, ICMS 17, IPVA 10, Serviços Gerais 9, Contencioso 8, Simples 6,
DFe 4, ITCMD 4 (= 91).

## Modelagem (molde PB/AC, rico, 1 GET)
- `titulo` = texto do heading (sem o "▾"); `descricao` = texto do `panel-body` (corte no próximo
  `panel-heading` p/ evitar vazamento; cap ~2500 chars — 1 painel do ITCMD tem ~22 KB de formulário).
- **público único "Serviços"** (a Carta é geral, cobre PF e PJ dentro dos requisitos — não há eixo de
  audiência por serviço); `classe` = tema; `link` = `servicos_cidadao.aspx#{id}` (identidade, único).

## Rede / TLS
- Apache/SharePoint, HTTPS padrão, UA AuliBot aceito. Sem gotcha de fingerprint (ureq funciona).

## Decisão
Catálogo genuinamente rico (91 fichas, 1 página) → parse HTML direto, sem headless, sem detalhe por
serviço. Público único; classe por tema.


---

<a id="to"></a>

# Relatório de descoberta — Carta de Serviços SEFAZ-TO (servicos.to.gov.br)

**Data:** 2026-07-05 · **Escopo:** descoberta + base para a implementação (18ª entidade). · **Órgão:**
Secretaria da Fazenda do Tocantins (SEFAZ-TO).

> **TL;DR — portal ASP.NET WebForms legado (HTML server-rendered, IIS 10), molde HTML-scraping (como
> BA/RJ), NÃO um SPA/JSON-API.** SEFAZ = órgão **`cod_empresa=37`**. Listagem (1 GET):
> `listar_servico.aspx?cod_empresa=37` → **45 serviços** (`card-servico-nome` + link
> `servico_detalhado.aspx?cod_assunto_documento_tipo={id}`). Detalhe rico por serviço (spans ASP.NET
> `lbl*` com id estável): descrição, público (`lblTipoRelacionamento`), categoria (`lblTxtServicoGrupo`),
> requisitos, documentos, taxa, prazo, legislação. **Cenário B** (público estruturado). Escopo escolhido:
> **descrição rica (45 detalhes)**.

---

## Fase 0 — robots

`servicos.to.gov.br/robots.txt` → **404** (IIS 10, sem robots). D-PA-ROBOTS cobre o TO como caso
preventivo (UA institucional AuliBot, cortesia entre GETs, nunca autenticar).

## Fase 1 — plataforma e listagem

- **ASP.NET WebForms / IIS 10** (páginas `.aspx`, jQuery 3.3.1 + Bootstrap admin theme, cookie
  `SERVICOS_SERVER_ID`). HTML server-rendered — **não** há JSON API nem SPA. Molde HTML-scraping.
- **Filtro por órgão:** SEFAZ = **`cod_empresa=37`** (achado no card "SECRETARIA DA FAZENDA / SEFAZ" da
  home). Contador dinâmico do card: **"45 serviços"** (guard).
- **Listagem completa (1 GET):** `https://servicos.to.gov.br/listar_servico.aspx?cod_empresa=37` →
  **45 serviços**, cada um: `<span class="card-servico-nome">{nome}</span>` +
  `<a href="servico_detalhado.aspx?cod_assunto_documento_tipo={id}">`. Nomes×links casam 1:1 (45/45).
  `detalhar_orgao.aspx` só mostra 4 (é a "vitrine"); a lista real é `listar_servico.aspx`.

## Fase 2 — enumeração

45 serviços, identidade = **`cod_assunto_documento_tipo`** (inteiro). Sem paginação (a lista vem
inteira). Contagem dinâmica = nº de links distintos na listagem (== contador "45"). Nunca hardcode.

## Fase 3 — payload de detalhe (`servico_detalhado.aspx?cod={id}`)

HTML server-rendered com o padrão gov.br "Carta de Serviços" (accordions). O conteúdo está em spans
ASP.NET com **id estável** (`ctl00_ContentPlaceHolder1_lbl*`) — mais robusto que parsear os accordions
aninhados. Mapa campo → snapshot:

| span (`lbl*`) | conteúdo | → snapshot |
|---|---|---|
| `lblTxtServico` | nome do serviço | `titulo` (ou o da listagem) |
| `lblTxtConceituacao` | "O que é" | `descricao` (lead) |
| `lblTxtRequisitoAcesso` | "Como solicitar" | `descricao` |
| `lblTxtDocumentacao` | "Documentos necessários" | `descricao` |
| `lblTituloTaxa` | "Custos e despesas envolvidas …" | `descricao` |
| `lblTituloPrazoExecutar` | "Prazo para conclusão …" | `descricao` |
| `lblTituloLegislacao` | "Legislação …" | `descricao` (opcional) |
| **`lblTipoRelacionamento`** | **público** ("Cidadão Empresa", "Órgão Público"…) | `publico` |
| **`lblTxtServicoGrupo`** | **categoria** ("Finanças, Impostos e Gestão Pública") | `classe` |
| `lblOrgaoGestor`/`lblSetorRecebedor` | órgão/setor (canal) | (contato, opcional) |

O HTML dos spans usa **entidades** (`&ccedil;`, `&eacute;`…) → decodificar (html5ever/`scraper`, lição
GO/ES). `descricao` rica = Conceituação + Como solicitar + Documentos + Custos + Prazo (seções não-vazias).

## Fase 4 — público e categoria (Cenário B)

- **Público = `lblTipoRelacionamento`** — vocabulário FIXO concatenado por espaço: observados
  **"Cidadão", "Empresa", "Cidadão Empresa", "Órgão Público"**. Parsear por vocabulário (longest-first:
  `Órgão Público` antes de `Cidadão`/`Empresa`) para não quebrar o "Órgão Público" (um valor, dois
  tokens). `ocorrencias` = público × classe.
- **Classe = `lblTxtServicoGrupo`** — a categoria; varia (maioria "Finanças, Impostos e Gestão Pública";
  há serviços cross-listados, ex. "Saúde e Vigilância Sanitária…"). Usar o valor cru (fidelidade),
  fallback "Geral".

## Fase 5 — destinos link-only

Muitos serviços apontam para os sistemas transacionais da própria SEFAZ-TO (`sefaz.to.gov.br`: CND,
DARE, etc.) e para `www.to.gov.br/sefaz` — **destinos link-only** citados no conteúdo. O `link` canônico
do snapshot é a página `servico_detalhado.aspx?cod={id}`; nunca autenticar nos sistemas destino.

## Pontos de decisão (candidatos a D-TO*)

1. **Escopo** — só SEFAZ (`cod_empresa=37`). O portal é **multi-órgão** (dezenas de `cod_empresa`) →
   3ª ocorrência da oportunidade "scraper estadual genérico" (D-PA-ACERVO), mas em ASP.NET/HTML, não
   JSON — parametrização menos direta. Registrar; não fazer agora.
2. **Robots** — coberto por D-PA-ROBOTS (TO = 3º caso; 404 = sem robots, sem bloqueio).
3. **Descrição rica** — escolhido: buscar os 45 detalhes (1 listagem + 45 GETs), montando a `descricao`
   das seções da Carta.
4. **Guard de contagem** — nº de links distintos na `listar_servico.aspx` (== "45"), dinâmico.

## Evidência bruta

Em `scratchpad/to/`: `home.html`, `lista37.html` (listagem SEFAZ), `det8017.html` (detalhe-modelo),
`sefaz37.html`. Endpoints e o mapa de `lbl*` acima.


---

<a id="rs-pareceres"></a>

# RS Pareceres — Portal de Legislação (Consultas Formais Respondidas)

**Data:** 2026-07-13 · **Escopo:** scraper da coleção **pareceres** do RS (não é entidade nova; o RS já
existe — coleta `pareceres` do `auli-scraper-rs`). · **Fonte:** `legislacao.sefaz.rs.gov.br/Site`
(Portal de Legislação, ASP.NET WebForms / IIS 10, **windows-1252**).

## Autorização / robots

`robots.txt` = `Disallow: /`; a busca declara-se **"Acesso Restrito"** (título + meta
`NOINDEX,NOFOLLOW`). A coleta destes pareceres **públicos** foi **autorizada pelo mantenedor** (a própria
SEFAZ-RS). UA institucional `AuliBot`, cortesia 500 ms, retry (o servidor pendura de vez em quando).

## Transporte — curl subprocess (como GO/DF/SE)

Com `ureq` a paginação sempre volta à página 1 / entrega resposta degradada — a camada na frente do IIS
serve conteúdo diferente ao ClientHello/headers do `ureq`. O **curl** — com os MESMOS cookie+viewstate —
devolve a página certa. Então listagem e detalhe vão por **curl subprocess**, com um **cookie jar** que
mantém a sessão `ASP.NET_SessionId`: **o postback pendura a conexão sem o cookie**, então o GET da página
1 é sempre fresco para semeá-lo.

## Listagem — o ponto crítico (estado de sessão + full-form)

`Search.aspx?CodArea=3&CodGroup=159` **NÃO é um grupo de pareceres**: cru (busca padrão), tem **367 docs
mistos** — 177 instruções normativas, 175 decretos, 26 pareceres, 7 leis. Os **372 pareceres** que o
navegador mostra vêm de um **filtro guardado na sessão**: o checkbox **"Consultas Formais Respondidas"**
(`RepeaterAreasGrupos$ctl01$cblAreasGrupos$4`) do `FormBuscaAvancada`, submetido por `BtnBuscar_Click`.

A paginação é postback WebForms (`__EVENTTARGET=LinkToPage`, `__EVENTARGUMENT=<pág>`). **Só o `__VIEWSTATE`
NÃO basta:** sem os campos do form o servidor perde o filtro e reverte à busca padrão (mistos). É preciso
**reenviar o formulário INTEIRO** a cada página — todos os inputs/hidden, o checkbox marcado, os selects —
como o browser faz (`collect_form_fields`). Com full-form, todas as ~19 páginas ficam em pareceres →
**372** (367 "PARECER" + 5 "INFORMAÇÃO"; a coleção inclui os dois tipos). Diagnóstico decisivo: `curl`
replicando o cookie+viewstate exatos do scraper devolvia a página 2 certa; a diferença estava no
formulário não reenviado, não no transporte.

Cada resultado: `<h5><a href="javascript:goDocument(<inpKey>,'')">TÍTULO</a></h5><p>ASSUNTO</p>`.

## Detalhe

`DocumentView.aspx?inpKey=N` — **público/anônimo** (não precisa da sessão), corpo em `#DOCContent .content`
(número + assunto + texto integral). Entidades HTML jurídicas/latinas decodificadas localmente
(`&ndash;`, `&ccedil;`, `&ordm;`, `&otilde;`, … — o `kit::decode_entities` só cobre um punhado).

## Saída / validação

Grava **só o intermediário** `data/rs/ref/rs-pareceres-temp.txt` (numero/assunto/corpo/link, **sem
`resumo`** — o estágio de resumo autorado é posterior e preserva o resumo já existente), e para. Validação:
**330 das 332 chaves do `.txt` autorado reproduzidas**, + ~43 pareceres novos; 1 corpo vazio (doc sem
`#DOCContent`). Intermediário é gitignored (`data/*/ref/*-temp.txt`).

## Checklist

- [x] Transporte curl + cookie jar (sessão) resolvidos.
- [x] Ponto crítico (full-form pagination preserva o filtro "Consultas Formais Respondidas"): 372.
- [x] Detalhe público confirmado (`#DOCContent`).
- [ ] Estágio de resumo autorado (`-temp.txt` → `rs-portal-pareceres.txt`) — próximo incremento.

