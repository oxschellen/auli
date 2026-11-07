# REGISTRO-scrapers — como cada portal foi investigado

> **Documento de MANUTENÇÃO, não de arquivo.** Um relatório por fonte: 14 de **serviços** por
> entidade + 1 de **pareceres** (RS), o **TARF** e 14 de **FAQs** (RS, SC, PR, SP, MG, MS, MT, GO, DF, AM, MA, PE, CE e RN). Cada seção registra como o portal foi investigado, o que ele
> tem de peculiar e por que o scraper faz o que faz — coisas que o CÓDIGO usa mas não explica, e que
> não estão documentadas em lugar nenhum do lado de lá.
>
> É o que se lê quando um scraper quebra: a cadeia TLS incompleta do AC, o WAF que só passa por
> `curl` no GO e no DF, o RSC flight do Next.js no AM, o catálogo hardcoded dentro do bundle JS no
> AP e no RR. Recuperar isto significaria refazer a investigação portal a portal, contra sites que
> já podem ter mudado.
>
> Referenciado por `SCRAPERS.md` (uma âncora por entidade), `docs/auli_pendencias.md` e comentários
> nos crates — âncoras `docs/REGISTRO-scrapers.md#<uf>` e `#rs-pareceres`. Renomeado de
> `descobertas.md` em 08/08/2026, para o nome dizer o que ele é.

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
- [TARF — Tribunal Administrativo de Recursos Fiscais do RS (tarf.sefaz.rs.gov.br)](#tarf)
- [RS FAQs — Perguntas Frequentes do Portal de Serviços da Receita (atendimento.receita.rs.gov.br)](#rs-faqs)
- [SC FAQs — Perguntas Frequentes da SEF-SC (www.sef.sc.gov.br/perguntas)](#sc-faqs)
- [PR FAQs — Perguntas Frequentes do SAC da SEFA-PR (atendimento.fazenda.pr.gov.br/sacsefa)](#pr-faqs)
- [SP FAQs — Perguntas Frequentes da SEFAZ-SP (listas SharePoint por subsite)](#sp-faqs)
- [MG FAQs — Perguntas Frequentes do Portal SPED MG e da Base de Conhecimento do CSM](#mg-faqs)
- [MS FAQs — Perguntas Frequentes da FAQ do Fale Conosco MS (www.faleconosco.ms.gov.br/faq)](#ms-faqs)
- [MT FAQs — Dúvidas do programa Nota MT (www.sefaz.mt.gov.br/notamt/duvida)](#mt-faqs)
- [GO FAQs — Perguntas Frequentes do site Orientação Tributária (orientacaotributaria.sefaz.go.gov.br)](#go-faqs)
- [DF FAQs — Perguntas e Respostas do Portal da Receita e Perguntas Frequentes do ISS/NFS-e](#df-faqs)
- [AM FAQs — Perguntas Frequentes das páginas do Portfólio de Serviços (www.sefaz.am.gov.br)](#am-faqs)
- [MA FAQs — Perguntas Frequentes do Portal SGC (portal-sgc.sefaz.ma.gov.br/portal/perguntas-frequentes)](#ma-faqs)
- [PE FAQs — Perguntas e Respostas do IPVA e do ITCMD (www.sefaz.pe.gov.br/Servicos/…/Perguntas-e-Respostas.aspx)](#pe-faqs)
- [CE FAQs — Perguntas e respostas do Portal de Serviços da SEFAZ-CE (portalservicos.sefaz.ce.gov.br)](#ce-faqs)
- [RN FAQs — FAQ do programa Nota Potiguar (np.sefaz.rn.gov.br/portal)](#rn-faqs)
- [ES FAQs — portal Receita Orienta (s1-internet.sefaz.es.gov.br/ReceitaOrienta)](#es-faqs)

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

```text
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
> flight RSC. Sem headless no scraper (headless foi usado só aqui, para *verificar* que não há XHR).

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

## Pontos de decisão a catalogar (candidatos a D-AM* em `docs/auli_pendencias.md` — NÃO decididos aqui)

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

```text
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

```text
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

```text
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
   paradigital/Prodepa — outra plataforma; ver `docs/REGISTRO-scrapers.md#pa`.)
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

---

<a id="tarf"></a>

# Descoberta — TARF / Tribunal Administrativo de Recursos Fiscais (tarf.sefaz.rs.gov.br)

**Discovery:** 08/08/2026. **Implantado:** mesma data (PRs #127 e #132) — o bloco final desta seção
registra o que a implementação confirmou, decidiu e descobriu depois.

## Fonte

- Ponto de partida: `https://tarf.sefaz.rs.gov.br/pesquisa?termo=&areas=&grupos=14543&assuntos=&dataInicio=&dataFim=`
- **Angular SPA** chamada internamente **"Legislacao 3.0"** (`<app-root>`, bundles webpack).
- Sistema **multiárea**: o mesmo backend serve `cage`, `receita`, `tarf` e `tesouro` (rotas
  `/areas/{area}` no bundle). O mesmo scraper cobriria as outras áreas da legislação da SEFAZ-RS com
  custo marginal baixo.
- `robots.txt` bloqueia acesso automatizado genérico; o discovery foi feito por `curl` local,
  garimpando o `main.bundle.js` — que está **pretty-printed**, o que facilitou ler os services
  Angular gerados por OpenAPI.

## Método

1. `curl` na URL de pesquisa → HTML de SPA vazio ⇒ os dados vêm por XHR.
2. Download do `main.bundle.js` + greps para mapear os services do `LegislacaoAPIClient` e as rotas
   `/api/...`.
3. Leitura do corpo dos services (assinaturas + montagem de `HttpParams`) para extrair os nomes
   EXATOS e a capitalização dos parâmetros — que é irregular (ver a tabela abaixo).
4. Testes de fumaça com `curl` direto.

## Contrato da API

### Listagem — `GET /api/PesquisaDocumentos`

| Parâmetro | Obs |
|---|---|
| `TamanhoPagina` | obrigatório |
| `NumeroPagina` | obrigatório, começa em 1 |
| `grupos` | **minúsculo**, repetível (`grupos=A&grupos=B`) — é array |
| `areas` | minúsculo, repetível — é array |
| `assuntos` | minúsculo, repetível — é array |
| `NumeroDoc`, `Termos` | opcionais |
| `DataInicio` / `DataFim` | opcionais; a SPA envia `.toISOString()` |

```text
GET /api/PesquisaDocumentos?TamanhoPagina=5&NumeroPagina=1&grupos=14543
```

```json
{
  "totalItens": 22590,
  "numeroPagina": 1, "tamanhoPagina": 5, "totalPaginas": 4518,
  "resultado": [
    { "titulo": "ACÓRDÃO 510/24", "grupo": "ACÓRDÃOS A PARTIR DE 2005",
      "grupoId": "b750f7f4-2628-4d5e-a669-1621a14e41d8", "area": "TARF",
      "numero": 51024, "dataDocumento": "2026-07-17T14:18:52", "ementa": "",
      "id": "ee87f4bc-12b9-4666-02cb-08dee4034157",
      "dataCriacao": "…", "dataAtualizacao": "…" }
  ],
  "temPaginaPosterior": true
}
```

- **Grupo 14543 = "ACÓRDÃOS A PARTIR DE 2005"**, 22.590 itens na data do discovery. O item traz o
  `grupoId` GUID; o filtro da query usa o id **numérico**.
- A `ementa` vem **vazia na listagem** — a real está no corpo.
- `numero` é o número do acórdão achatado (510/24 → `51024`).

### Metadados — `GET /api/Documentos/{id}`

Só metadados (campos da listagem + `assuntos: []`). **Não contém o corpo.**

### Corpo — `GET /api/documentos/{id}/DocumentoHtml`

- `NumeroPagina` (≥1) e `TamanhoPagina` (**entre 10 e 4000**, validado pelo servidor com mensagem
  explícita) são obrigatórios.
- Pagina os **dispositivos** do documento, no mesmo envelope da listagem.
- Cada item tem `id` (GUID do dispositivo) e `texto` (HTML).

O `texto` traz um cabeçalho em `<table>` com campos rotulados: ACÓRDÃO Nº, RECURSO Nº,
RECORRENTE(S) (com nomes de empresa e de pessoas físicas), RECORRIDA(S), PROCESSO Nº, AUTO DE
LANÇAMENTO Nº, DECISÃO DE 1ª INSTÂNCIA Nº e **EMENTA** — a ementa em CAIXA ALTA seguida de
parágrafos de fundamentação. Depois, o corpo do acórdão.

### Endpoints mapeados e não usados

`DocumentoPdf`, `informacao`, `IndiceDocumento`, `MenuDocumento`, `Assuntos`,
`Assuntos/GetByArea`, `Grupos` (útil se um dia quisermos enumerar grupos em vez de fixar 14543).
`/api/PesquisaConteudoDocumentos` é a busca DENTRO de um documento aberto (recebe `IdDocumento`),
não a listagem — descartado.

## Reproduzir o discovery

```bash
curl -s 'https://tarf.sefaz.rs.gov.br/main.bundle.js' -o /tmp/tarf-main.js
curl -s 'https://tarf.sefaz.rs.gov.br/api/PesquisaDocumentos?TamanhoPagina=5&NumeroPagina=1&grupos=14543'
curl -s 'https://tarf.sefaz.rs.gov.br/api/Documentos/ee87f4bc-12b9-4666-02cb-08dee4034157'
curl -s 'https://tarf.sefaz.rs.gov.br/api/documentos/ee87f4bc-12b9-4666-02cb-08dee4034157/DocumentoHtml?NumeroPagina=1&TamanhoPagina=4000'
```

## O que a implementação decidiu (08/08/2026)

O discovery fechou com seis pendências "para quando o assunto voltar". Ele voltou no mesmo dia; o
que ficou decidido:

- **Kind `tarf`** (o discovery cogitava `acordaos-tarf`), árvore `.md` no vocabulário unificado das
  demais coleções. Constantes em `auli-scraper-rs/src/tarf.rs`: grupo `14543`, página de listagem
  500, página de dispositivos 4000, lote de gravação 200.
- **Sem sinopse LLM.** O `## resumo` é a **fundamentação que o próprio acórdão traz no cabeçalho**,
  extraída na coleta; sem fundamentação, o fallback é a própria ementa. Nada é gerado por modelo —
  é uma diferença real frente aos pareceres, e é o que faz a coleção não depender de um passo LLM.
- **Regra do parser, unificada entre os dois layouts de tabela:** varrer TODAS as tabelas atrás da
  linha rotulada `EMENTA`; a ementa é o **primeiro parágrafo** da célula, e a fundamentação é o
  resto dela mais as linhas seguintes de rótulo vazio. Medido em amostra estratificada por ano
  (103 documentos) antes de fixar.
- **Árvore append-only** (`escrever_lote_se_ausente_com_resumo`): "existe ⇒ pula". Isso responde a
  pendência da mutabilidade — as atualizações (`dataAtualizacao`) são **ignoradas**. Decisão
  consciente, não omissão: revisitar se aparecer evidência de acórdão materialmente alterado.
- **Múltiplos dispositivos**: o `DocumentoHtml` é paginado de verdade, não se assume `totalItens=1`.

## O que só apareceu ao rodar

- **A contagem não fecha por identidade.** A varredura completa das 46 páginas achou 22.588 dos
  22.590 anunciados, e **68 slugs duplicados** — acórdãos distintos cujo número colide no nome de
  arquivo. A guarda de invariante teve de virar tolerante (três passadas acumulando antes de abortar),
  senão derrubaria uma coleta de 6h no fim.
- **Um terceiro layout**, ~1,5% do acervo (~340 acórdãos do Pleno): a ementa vem inline, num
  `<p><strong>EMENTA</strong>:`, sem âncora para o fim da fundamentação. **Ainda sem regra** — a
  decisão espera o censo que uma rodada completa produz.
- **`data/rs/raw/tarf-anomalias.txt`** é o registro dessas exceções, e é reescrito a cada rodada
  (logo, só vale como censo depois de uma rodada que vá até o fim). Na parcial de 3.800 documentos:
  68 `SLUG-DUPLICADO`, 18 `SEM-EMENTA`, 2 `FALHA-CORPO`.
- **Um acórdão com hífen no lugar da barra** (`Acórdão do pleno 100-25`) — não casa com a chave
  cronológica e vai para o fim da lista da aba, sem data inventada. Um em 3.800.
- **Dado pessoal no corpo**: nomes de recorrentes (empresas e pessoas físicas), números de processo
  e de auto de lançamento. São documentos públicos do portal, mas o fato está aqui de propósito,
  para quem for pensar em exposição ou em geração automática de texto sobre eles.

## Estado

**Acervo completo, atualizado em 18/10/2026: 22.514 acórdãos na árvore.** A pausa em 3.800 ficou para
trás; desde então a coleta é incremental — `auli-scraper-rs tarf` refaz a listagem inteira (46 GETs)
e só busca o corpo do que ainda não tem `.md`, e o `build-packs.sh rs` vetoriza só o novo. A rodada de
18/10 levou ~10 min e acrescentou 38 (22.476 → 22.514).

Como a conta fecha, na listagem dessa data:

| | |
|---|---|
| anunciados pelo portal (`totalItens`) | 22.627 |
| − `SLUG-DUPLICADO` (revisões do mesmo acórdão) | 68 |
| = únicos na listagem | 22.559 |
| − `SEM-EMENTA` | 29 |
| − `FALHA-CORPO` | 17 |
| = com `.md` na árvore | 22.513 |
| + acórdão que saiu da listagem e segue na árvore | 1 |
| **= árvore** | **22.514** |

As exceções são re-tentadas a cada rodada (do cache, quando o corpo já veio), então reaparecem no
`tarf-anomalias.txt` sem ser regressão. Esse arquivo é reescrito **durante** a rodada — para comparar
com a anterior, copie-o antes de rodar. Três `FALHA-CORPO` falham sempre, confirmadas por requisição
direta:

- **233/23** — o corpo tem 14 MB (o curl o traz em 76 s) e passa do limite de 10 MB da resposta no
  scraper. Subir esse limite o recupera.
- **202/23** e **457/23** — `504` do gateway do portal depois de ~88 s. É do lado do servidor; não há
  o que fazer daqui.

**ACÓRDÃO 066/26** (`aa0f538a-fb65-4d2b-ada4-08deb7f7f86a`, pedido de esclarecimento) saiu da listagem
do portal e não reaparece com outro id. A árvore é append-only, então ele continua no acervo e
citável pelo chat; tirá-lo é decisão humana (apagar o `.md` e rodar o `build-packs.sh rs`).

---

<a id="rs-faqs"></a>

# RS FAQs — Perguntas Frequentes do Portal de Serviços da Receita (atendimento.receita.rs.gov.br)

**Scraper:** coleção `faqs` do `auli-scraper-rs` (`auli-scraper-rs faqs`, ou `all` = FAQs + serviços), a
primeira coleção de FAQs do acervo — anterior ao REGISTRO, e por isso sem seção até a troca de raiz de
06/11/2026, registrada aqui. Módulo `src/faqs/` (`mod.rs` caminha, `fetch.rs` busca, `html.rs` extrai).

## Fonte

Portal "Matriz" da PROCERGS. Cada página declara em `data-matriz-source-uri` a chamada AJAX que traz o seu
miolo (`/_service/conteudo/pagedlistfilho?id=<N>&templatename=<template>`), e o **template** diz o que a
página é: `pagina.listapagina.categoriafaq[.comnavegacao]` = **menu** (filhos = categorias ou listas),
`pagina.listapagina.sanfona.comnavegacao` = **lista de FAQ** (sanfona de pergunta/resposta); qualquer outro
template é `Geral` (nó sem filhos e sem itens). O scraper lê a raiz, pede o AJAX com `currentPage=1&pageSize=100`,
fatia o `body` (JSON) por `class="panel panel-default"` e desce recursivamente pelos menus; a trilha (`origin`)
vem do breadcrumb da página-folha. Na coleta de 01/11/2026 eram 330 nós (60 menus, 251 listas, 19 `Geral`) e
644 arquivos no cache — ~650 requisições por rodada.

## As armadilhas

1. **O F5 na frente do `_service/*` (desde out/2026)** rejeita o ureq pela impressão do cliente e devolve o
   desafio JS do Bot Defense **com HTTP 200** (`window["bobcmn"]`/TSPD); o bloqueio do ASM é
   `<title>Request Rejected</title>`. Por isso o AJAX vai por `get_via_curl` com o `Accept` de um `$.getJSON`
   (`application/json, text/javascript, */*; q=0.01`), as páginas seguem no ureq, e nenhuma das duas páginas
   do F5 entra no cache (cache-first SEMPRE: gravada uma vez, envenenaria todas as rodadas seguintes) — na
   página quem a recusa é `f5::bloqueio_f5`; no AJAX, o desafio não é JSON e a leitura do `body` falha antes
   de gravar. As páginas de conteúdo ainda passam com UA de navegador (D-FROTA-UA: legado,
   nunca testado com `AuliBot` neste host).
2. **A raiz mudou de template em nov/2026.** `/perguntas-frequentes` virou um **índice novo**: sem
   `data-matriz-source-uri`, sem `panel panel-default`, ~5 MB com **1.854 perguntas de 233 páginas inline**
   (links `/<página>#panel-<N>`, 2.701 "Publicação:") agrupadas em oito acordeões (`#abx-icms`, `-ipva`,
   `-itcd`, `-taxas`, `-nfg`, `-ecac`, `-cert`, `-pref`). A caminhada o classifica como `Geral` e colhe **zero** — o que a
   guarda de `faqs::run` transforma em erro sem tocar snapshot nem árvore. O portal preservou a raiz antiga em
   **`/perguntas-frequentes-antigo`** (`id=108&templatename=pagina.listapagina.categoriafaq.comnavegacao`) — o
   **mesmo nó**: no cache de 01/11 a raiz `/perguntas-frequentes` declarava exatamente esse `id=108` —, e as
   páginas-filhas não mudaram de URL; a raiz do scraper passou a apontar para lá (06/11/2026). O 1º nível da
   árvore é o que o `body` do `id=108` lista (17 nós em 01/11), não a barra "Seções" das páginas de categoria.
   "-antigo" é nome de página em despedida: se sumir, a mesma guarda avisa. O índice novo foi estudado em
   [`ESTUDO-RS-Faqs-Indice.md`](ESTUDO-RS-Faqs-Indice.md) (06/11/2026): o `panel-N` dele é o mesmo do AJAX, todo
   painel dele está na coleta com o mesmo texto, a trilha sai igual em 94% das páginas, e a diferença 1.990 →
   1.854 são 107 cópias da caminhada (o mesmo painel em duas URLs) e as 29 do Piloto RTC – IBS, que ele não lista.
   Ele é gerado, não ao vivo; fica como guarda e plano B (D-RSFAQ-1..6, pendências §20).
3. **Queda parcial é silenciosa.** Erro ao caminhar uma página vira `eprintln!` e o laço segue; falha no AJAX
   devolve nó vazio — uma categoria que caia leva a subárvore inteira. Vale também para o F5: abaixo da raiz,
   o desafio/bloqueio é um erro como outro qualquer (`Error walking` / `Error fetching body`). Quem pega isso é a guarda de
   encolhimento de `scripts/tools/update-rs-faqs.sh` (compara com o `.anterior`).
4. **Botões de UI dentro do painel.** Depois da linha "Publicação:", cada `panel-body` tem `<button
   onclick="copiarLink(N)">Copiar link</button>` e `<button onclick="copiar(N)">Copiar resposta</button>` (2 por
   painel, nenhum outro `<button>` no acervo). Até 06/11/2026 o texto deles entrava em toda resposta — e nos
   vetores; o conversor agora descarta o `<button>` inteiro (D-RSFAQ-3).
5. **Páginas-alias.** O portal publica alguns painéis em duas URLs (`/integracao` e `/notaintegrada`,
   `/1-2-declaracao-de-conteudo-eletronica-dc-e` e `/faq-dc-e`…), e a caminhada visita as duas: em 06/11/2026, 107 dos
   1.990 itens eram cópias. O `panel-N` é global, então o snapshot fica com a primeira ocorrência de cada painel na
   ordem da caminhada (D-RSFAQ-4); a árvore da aba de FAQs segue com as duas páginas, como o portal.

## Guardas e transporte

- `Err` sem gravar nada: zero FAQs na caminhada; página da **raiz** que devolva o desafio/bloqueio do F5.
  Nas páginas-filhas, e no AJAX de qualquer nó, o mesmo erro só derruba a subárvore daquele nó (armadilha 3)
  — o AJAX da raiz leva a árvore toda, e aí quem para é a guarda de zero FAQs.
- Cache-first sempre (`data/rs/raw/cache/faqs/`, página e `_body` por URL); `--usecache` = miss vira erro.
  Trocar a raiz cria um arquivo de cache novo para ela; o das filhas continua válido.
- `retry` 3× com backoff; AJAX por `curl`, páginas por ureq; UA de navegador nos dois.
- Saída: `data/rs/raw/rs-faqs-tree.json` (a árvore que a aba de FAQs consome — único artefato sem outra
  fonte) e `data/rs/rs-faqs-snapshot.json`, nessa ordem e atômicos. Cada pergunta com o próprio link,
  `<página>#panel-N` — o hash que o portal usa para abrir o painel (`expandirAccordionPorHash`) e o que o botão
  "Copiar link" copia (D-RSFAQ-5); o snapshot sem as cópias de painel (armadilha 5).
- Atualização: `scripts/tools/update-rs-faqs.sh` (apaga o cache antes; guarda de encolhimento).

---

<a id="sc-faqs"></a>

# SC FAQs — Perguntas Frequentes da SEF-SC (www.sef.sc.gov.br/perguntas)

**Discovery e implantação:** 31/10 a 01/11/2026. Estudo completo, com as medições
e as decisões D-SCFAQ-1..8: [`docs/ESTUDO-SC-Faqs.md`](ESTUDO-SC-Faqs.md). Coleção `faqs` do
`auli-scraper-sc` (`auli-scraper-sc faqs`, ou `all` = serviços + FAQs). **1.306 perguntas** na rodada de
01/11.

## Fonte

O mesmo portal Next.js (Pages Router) dos serviços. A base antiga,
`caf2.sef.sc.gov.br/.../ConsultarBaseConhecimento.aspx`, hoje redireciona (307) para `/perguntas`. Os chunks
JS não chamam API externa: os dados saem do `getServerSideProps`, pelo `/_next/data/<buildId>/…`. O
`buildId` vem do HTML de `/perguntas` e muda a cada deploy, então o cache usa a URL lógica, sem ele.

- **Árvore:** `GET /_next/data/<buildId>/perguntas.json` → `respostaApi.assuntosBusca`, com 31 raízes. As
  raízes são as do JSON, não os 15 cards do HTML; as 16 extras dão 0 perguntas, exceto a `72` (DTEC), com 1.
  Não declarar o `IconeBase64`, que traz PNGs inteiros.
- **Perguntas:** `GET /_next/data/<buildId>/perguntas/buscar.json?assunto=<raiz>`, uma chamada por raiz,
  sem paginação. A lista já traz a resposta íntegra; não há detalhe por pergunta. Só `assunto`, `texto` e
  `numero` são reconhecidos. Qualquer outro nome volta `buscar:false` em silêncio, por isso `buscar:false`
  é erro, não "zero resultados".
- **Link oficial:** `/perguntas/buscar?numero=<BaseConhecimentoID>`. A `/perguntas/<AssuntoID>` mostra só
  parte do assunto; para o nó da aba, o link é `/perguntas/buscar?assunto=<AssuntoID>`.

## As três armadilhas

1. **O dobro.** `assunto=<nó intermediário>` devolve cada descendente duas vezes (`75` → 44 linhas, 22
   únicas; `2955` → 134/67). O dobro vem da fonte e é determinístico. Dedup por `BaseConhecimentoID`
   dentro de cada raiz, com `linhas → únicas` no log. A guarda de verdade é estrutural: uma pergunta
   tem um `Assunto`, então pertence a uma raiz só, e `Σ únicas por raiz == únicas no total`.
2. **A árvore tem só 2 níveis.** O 3º (ex.: Benefícios Fiscais › TTD/Fundos › Aspectos Gerais, `166`)
   só aparece no `Assunto{AssuntoID, Nome, SuperiorHierarquicoID}` de cada pergunta. Em Benefícios
   Fiscais, 49 de 67 perguntas estão num assunto ausente da árvore. O mapa de assuntos é a árvore
   **completada pelas perguntas**, e a trilha sobe por `SuperiorHierarquicoID`. Rodada de 01/11:
   4 trilhas de 1 nível, 513 de 2, 789 de 3.
3. **HTML às vezes.** `Questao`/`Resposta` vêm ora em texto puro com `\n`, ora em HTML (`p`, `br`, `a`,
   `font`, `span`, `strong` + entidades; 22 de 67 em Benefícios Fiscais). O `html_para_texto` (desde 07/11/2026, o do kit, com `quebras_do_fonte`) converte
   `<a href>` → `texto "url"`, bloco → quebra de linha, tira o resto e decodifica as entidades. O texto
   puro passa sem mudar, inclusive as quebras no meio da frase que vêm da fonte. Nenhum `[[url âncora]]`
   (o formato dos serviços) no acervo.

## Guardas e transporte

- `Err` antes de gravar qualquer arquivo: `buscar != true` ou `errorCode` preenchido em qualquer
  chamada; uma pergunta em duas raízes; cadeia de pais que não resolve; zero perguntas no total;
  pergunta sem texto de `Questao` (D-SCFAQ-3f; `Resposta` vazia passa).
  `Visibilidade ≠ 2` só avisa: em 01/11, todas eram 2.
- **Sempre à rede.** Diferente do resto do kit (cache-first), o cache em `data/sc/raw/cache/faqs/` só é
  lido com `--usecache`, que reproduz snapshot e árvore byte a byte. Ler do cache por padrão faria a
  atualização seguinte repetir a primeira coleta em silêncio, como aconteceu com os serviços do SC em
  31/10.
- 33 GETs com o UA `AuliBot` e 1 s de cortesia (as constantes do `pareceres.rs`); `ureq` passa.
- Saída: `data/sc/raw/sc-faqs-tree.json` (o `FaqNode` de `auli_scraper_kit::faqtree`, o mesmo do RS;
  um nó pode ser `Menu` e ter `faq_items`; nó sem pergunta na subárvore é omitido) e
  `data/sc/sc-faqs-snapshot.json`, nessa ordem e atômicos.
- Atualização: `scripts/tools/update-sc-faqs.sh` (guarda de encolhimento contra o `.anterior`).

## Limitação conhecida — 4º nível

A `perguntas.json` só tem 2 níveis. O 3º entra no mapa de assuntos pelo `Assunto` das perguntas
(`completar_mapa`), então um assunto só está no mapa se está na árvore **ou** tem ao menos uma
pergunta pendurada diretamente nele. Se a SEF-SC criar um 4º nível `A › B › C › D` em que só `D`
tem pergunta, a subida de `D` chega a `C`, que não está em lugar nenhum, e a coleta inteira para
**antes de gravar** (guarda D-SCFAQ-3c, teste `pai_fora_do_mapa_e_erro`):

```
assunto <C> fora do mapa de assuntos — a cadeia de pais de <D> não resolve
```

É a falha certa, alta e sem tocar no acervo, mas não se resolve sozinha. Em 01/11/2026 não havia
4º nível (trilhas 4/513/789 de 1/2/3 níveis); a limitação é latente.

**Remédio, se acontecer** (não implementado — é código para um caso que não existe). A página do
assunto, `GET /_next/data/<buildId>/perguntas/<id>.json?id=<id>`, traz em `respostaApi.assunto` o
`Nome` e os `Subassuntos` **reais** (a do `75` lista 166, 169, 293, 168 e 167, que a `perguntas.json`
mostra como `[]`). Mas ela traz `SuperiorHierarquicoID: null` em tudo, inclusive no próprio assunto
(conferido em 01/11/2026). Então buscar a página do elo ausente `C` não diz quem é o pai dele: o
remédio é **descer**. A partir dos assuntos já no mapa, busca-se a página de cada um e acrescentam-se
os `Subassuntos` com o pai que a própria descida conhece, até `C` aparecer, com um teto de
profundidade contra ciclo.

---

<a id="pr-faqs"></a>

# PR FAQs — Perguntas Frequentes do SAC da SEFA-PR (atendimento.fazenda.pr.gov.br/sacsefa)

**Discovery e implantação:** 01/11/2026. Estudo completo, com as medições e as decisões D-PRFAQ-1..9:
[`docs/ESTUDO-PR-Faqs.md`](ESTUDO-PR-Faqs.md). Coleção `faqs` do `auli-scraper-pr` (`auli-scraper-pr faqs`,
ou `all` = serviços + FAQs). **1.321 perguntas** em 48 referentes e 274 assuntos na rodada de 01/11.

## Fonte

As 6 fichas "perguntas frequentes" da Carta de Serviços (`www.fazenda.pr.gov.br`, o Drupal dos serviços)
levam todas ao **Portal de Atendimento do SAC**, uma aplicação PHP à parte, em outro host. HTML
server-side, sem API, sem sessão para as páginas, sem `robots.txt` (404).

- **Índice:** `GET /sacsefa/portal/index` → 48 links `assuntosReferente/<id>`, com o nome longo do
  referente no `h4` do mesmo cabeçalho ("IPVA - Imposto sobre a Propriedade de Veículos Automotores").
  Os ids têm lacunas, e toda lacuna dá 302 para o `index` (referente desativado — a ficha do ITCMD da
  Carta apontava para o `19` até a coleta de serviços de 01/11/2026; o ITCMD de hoje é o `60`). **O `index`
  é a fonte de verdade**; nunca
  varrer ids.
- **Referente:** `GET /sacsefa/portal/assuntosReferente/<id>` traz o referente inteiro, sem paginação:
  cards de assunto (`headPanel<idAssunto>`) → painéis de pergunta (`heading<N>` + `collapse<N>`), com
  `Pergunta Nº <N>` no cabeçalho e `Referente:`/`Assunto:` + a resposta em `<p>` no corpo. De
  52 KB a 594 KB (o ITCMD, 185 perguntas); 6,3 MB os 48.
- **Link:** não há rota por pergunta (`pergunta/<N>` e afins dão 302; a busca é `POST`). O link é a
  página do referente + `#heading<N>`. A âncora não abre o painel (ele fica num card recolhido, e a
  página não lê `location.hash`): o usuário cai no topo do referente certo.

## As quatro armadilhas

1. **TLS com cadeia incompleta.** O servidor manda só o folha `*.fazenda.pr.gov.br`, emitido pelo
   **Sectigo Public Server Authentication CA DV R36** — outro intermediário, não o OV R36 do AC. `curl`
   e `ureq` falham com o store do sistema. O PEM do DV R36 (AIA do folha, válido até 21/03/2036) vai
   embutido como trust anchor do rustls num agent só do módulo `faqs` (o molde do AC). **O folha vence
   em 21/11/2026**: se a renovação vier de outro emissor, a coleta falha alta no handshake e o PEM tem
   de ser trocado — o procedimento (conferir o emissor novo com `openssl s_client`) está na mensagem de
   falha do `update-pr-faqs.sh`. O `www.fazenda.pr.gov.br` manda a cadeia completa e não precisa disso.
2. **`<` como texto.** As respostas usam `<` como seta de menu ("Cadastro de Contribuintes < Alteração
   Cadastral < Dados Cadastrais", 12 ocorrências) e a 1775 cita `<Consulta Pendencias>`. O `<[^>]+>` do
   SC apagaria texto em 2 respostas (a 1775 e a 2274, onde a seta vem antes de um `<b>`). A conversão
   só tira as tags de uma **lista fechada** (`a b br center div em font hr i img li ol p span strong sub
   sup u ul`); qualquer outro `<…>` fica.
3. **"Assunto:" dentro da pergunta.** A 2247 (referente 28) tem `Assunto: "Outorga de..."` no texto
   da pergunta. Cada campo é lido só na sua metade do painel — número e pergunta no cabeçalho,
   `Assunto:` e resposta no corpo —, senão a guarda de assunto dispara (foi o que ela fez na 1ª rodada).
4. **Pergunta repetida e imagem.** Há 17 pares de pergunta com o mesmo texto no mesmo referente (em
   assuntos diferentes): sem a âncora `#heading<N>`, a url seria a mesma e os `.md` colidiriam. E 11
   respostas trazem tabela ou ícone em imagem (`/sacsefa/images/…`), 3 delas só a imagem: `<img src>`
   vira `"<url absoluta>"`, a convenção da âncora vazia.

## Guardas e transporte

- `Err` antes de gravar qualquer arquivo: `index` sem referente; referente sem nenhuma pergunta na
  página (o 302 de um referente desativado cai no `index`, que reusa `perguntaFrequentePanelHeader` e
  `heading<N>` para os cards de referente — nada disso vira pergunta); nº de
  `perguntaFrequentePanelHeader` no HTML ≠ painéis parseados; `heading<N>` ≠ `Pergunta Nº`; `Assunto:`
  do painel ≠ título do card que o contém; um número de pergunta em dois lugares; pergunta sem texto.
  Só avisam: resposta vazia depois da conversão e resposta com tag de estrutura fora da lista fechada
  (`table`/`tr`/`td`, `h1`–`h6`, `blockquote`, `pre`, `form`…), que sairia literal no `.md` (0 e 0 em 01/11).
- **Sempre à rede**, como no SC: o cache em `data/pr/raw/cache/faqs/` só é lido com `--usecache`, que
  reproduz snapshot e árvore byte a byte.
- 49 GETs com o UA `AuliBot` e 1 s de cortesia (as constantes do `pareceres.rs`), ~1 min.
- Saída: `data/pr/raw/pr-faqs-tree.json` (raiz → 48 `Menu` de referente → 274 `Faq` de assunto, cada
  pergunta com o próprio `url`) e `data/pr/pr-faqs-snapshot.json`, nessa ordem e atômicos. Trilha
  sempre de 2 níveis: `<nome longo do index> | <assunto>`.
- Resposta = todos os `<p>` do corpo, um por linha, com o corpo limitado ao `<script` da página (o último
  painel encosta no rodapé, que também tem `<p>`). Em 01/11/2026, um `<p>` por painel em 1.321/1.321.
- Atualização: `scripts/tools/update-pr-faqs.sh` (guarda de encolhimento contra o `.anterior`).

---

<a id="sp-faqs"></a>

# SP FAQs — Perguntas Frequentes da SEFAZ-SP (listas SharePoint por subsite)

**Discovery e implantação:** 01/11/2026. Estudo, medições e decisões D-SPFAQ-1..9:
[`docs/ESTUDO-SP-Faqs.md`](ESTUDO-SP-Faqs.md). Coleção `faqs` do `auli-scraper-sp` (`auli-scraper-sp faqs`, ou
`all` = serviços + FAQs). **2.665 perguntas** em 64 subsites com conteúdo na rodada de 01/11.

## Fonte

O mesmo portal SharePoint (`portal.fazenda.sp.gov.br`) e a mesma `_api` anônima dos serviços. A página
central `/Paginas/perguntas-frequentes.aspx` não traz pergunta no HTML: é um plugin JS
(`/BibliotecaDeScripts/pluginPerguntasFrequentes.js`) que faz duas chamadas, e o scraper repete as duas.

- **Subsites:** `GET /servicos/_api/web/lists/getByTitle('Homes 360')/items?$select=Id,Title,URLHome` →
  99 Homes. O `URLHome` vem de quatro jeitos: relativo com barra (94), relativo **sem** a barra
  (`/servicos/isencao-icms-veiculos` — o botão do portal monta um link quebrado com ele; o scraper põe a
  barra), ausente (3) e absoluto para outro domínio (o de Recursos Humanos, em `www.sfp.sp.gov.br` — o
  plugin não o lê, e o scraper o pula com aviso). `/_api/web/webs` não é público: a `Homes 360` é a fonte.
- **Perguntas:** `GET <URLHome>_api/web/lists/getByTitle('Perguntas Frequentes')/items?$select=Id,Title,
  Resposta,Assunto&$orderby=Title&$top=1000`, uma por subsite. 95 listas respondem 200 (31 vazias); nenhuma
  passa de 1.000, mas o `d.__next` é seguido. `Assunto` é MultiChoice (`results: [...]`): 388 perguntas têm
  mais de um. O `ItemCount` da lista conta rascunhos que o anônimo não vê — não serve de guarda.
- **Link:** o que o botão "copiar link" do portal copia (`TemplatePerguntasFrequentesGeral.html`):
  `<URLHome>Paginas/perguntas-frequentes.aspx#ID=<Id>`. O `DispForm.aspx` do item dá 401. O `#ID=` separa as
  perguntas de mesmo texto no mesmo subsite (32 em 01/11) — sem ele, os `.md` colidiriam.

## As armadilhas

1. **O `Id` é por lista.** A identidade é (subsite, `Id`); o `Id 68` existe em vários subsites.
2. **HTML do editor SharePoint.** A `Resposta` vem com entidades numéricas por toda parte (`&#58;` 12 mil
   vezes, `&#160;`, emojis `&#128073;`), o U+200B do editor, quebras de linha cruas, `<style>`, `<svg>`,
   tabelas e `href`/`src` relativos com entidades (`https&#58;//`, `/servicos/…`). Diferente do PR, o editor
   escapa o `<` do texto (`&lt;escrituras&gt;`): toda tag sai, e as entidades só são decodificadas depois.
   O espaço cru é colapsado antes de os blocos virarem linha (senão a frase se parte), `<style>`/`<svg>`
   saem com o conteúdo, célula de tabela vira espaço ("CST_ICMS 53") — não `|`, porque as respostas citam
   registros SPED (`|C197|SP90090278|…`) — e `href`/`src` relativos ganham o domínio. O decodificador é o
   `decode_html` do `pareceres.rs`, que já tratava as numéricas.
3. **O `Title` é texto puro.** A pergunta 530 do SAT tem "(`<IM>`)" de verdade: o título só passa pelo
   `clean`, nunca pela retirada de tags.

## Guardas e transporte

- `Err` antes de gravar qualquer arquivo: nenhum Home com `URLHome` do portal; `Id` repetido numa lista;
  pergunta sem texto; zero perguntas no total; qualquer chamada que falhe (inclusive uma página `__next`).
  Resposta vazia só avisa (0 em 01/11).
- **Sempre à rede**, como no SC e no PR: o cache em `data/sp/raw/cache/faqs/` só é lido com `--usecache`, que
  reproduz snapshot e árvore byte a byte.
- 97 GETs com o UA `AuliBot` e 1 s de cortesia (as constantes do `pareceres.rs`), ~2 min. A lista do Profisco
  estourou o timeout de 60 s uma vez na 1ª rodada e passou na 2ª tentativa.
- Saída: `data/sp/raw/sp-faqs-tree.json` (raiz → 64 `Menu` de subsite → 365 `Faq` de assunto; a pergunta com
  vários assuntos aparece em cada um, como no portal, num total de 3.068 itens) e
  `data/sp/sp-faqs-snapshot.json` (um `FaqRaw` por pergunta, trilha `<Home> | <assunto>, <assunto>`), nessa
  ordem e atômicos.
- Atualização: `scripts/tools/update-sp-faqs.sh` (guarda de encolhimento contra o `.anterior`).

## Limitação conhecida — dois subsites com o link quebrado no próprio portal

O link do botão "copiar link" leva a uma página inexistente em dois subsites: `cfaiim` (Conta Fiscal AIIM, 33
perguntas, página 404) e `evolucao-funcional` (1 pergunta, 401 — intranet). O scraper usa o mesmo formato do
portal; quem abre a pergunta desses dois cai no erro do SharePoint. O `sefazidentity` redireciona (301) para
`www.sfp.sp.gov.br` e funciona.

---

<a id="mg-faqs"></a>

# MG FAQs — Perguntas Frequentes do Portal SPED MG e da Base de Conhecimento do CSM

**Discovery e implantação:** SPED em 01/11/2026, CSM em 02/11/2026. Estudo, medições e decisões
D-MGFAQ-1..15: [`docs/ESTUDO-MG-Faqs.md`](ESTUDO-MG-Faqs.md) (§10 = o SPED, §11 = o CSM). Coleção `faqs` do
`auli-scraper-mg` (`auli-scraper-mg faqs`, ou `all` = serviços + FAQs). **310 perguntas** na rodada de 02/11:
SPED 149 (NF-e 87, NFC-e 62) e CSM 161 (rejeições de DF-e 61, Sintegra - PED 100).

## Fonte 1 — o Portal SPED MG (`faqs.rs`)

Outro host que o CSM ServiceNow dos serviços: OpenCms exportando HTML estático (sem API, sem AJAX, `robots.txt`
404).

- **Módulos:** a barra da home (`<a class="menu menu-N" href="/spedmg/<sigla>/" title="…">`) — 15. O nome vem da
  sigla e do `title`: "NF-e - Nota Fiscal Eletrônica".
- **Entrada de FAQ:** o menu lateral de cada módulo. Seis têm "Perguntas e Respostas", com o caminho que cada um
  quis (`nfe/Perguntas-Frequentes/`, `cte/faq/`, `cteos/Perguntas-e-Respostas/`, `efd/faq/`, `mdfe/perguntas/`…) —
  não dá para adivinhar a URL. **Só NF-e e NFC-e são FAQ em HTML no SPED**: a página do CT-e e a do CT-e OS
  apontam para o portal nacional do CT-e, a do MDF-e para o da SVRS, e a da EFD para dois PDFs e um `.docx`. O
  scraper avisa esses quatro e segue; os outros nove não têm a entrada.
- **Índice:** título de seção em `<p><strong>` (romano: "I. Conceito, uso e obrigatoriedade da NF-e") e, abaixo,
  uma tabela: linha com o número e o link é pergunta; linha sem link é **sub-tópico** (no NF-e, os 8 da seção III).
  O link vai para `respostas_<romano>/index.html#<âncora>`.
- **Respostas:** uma página por seção, exportada do Word, cortada entre o `<div id="conteudo">` e o menu lateral.

## As armadilhas do SPED

1. **A âncora é por página, não por módulo.** `#1` existe na I e na II do NF-e; a NFC-e usa `#a1`, `#b1`… (a letra
   da seção). A identidade é (página, âncora), e o link é o do índice, exato.
2. **Cada página de respostas é de um jeito.** Âncora `<a name="N">` (NF-e), `<a id="aN">` (NFC-e), nenhuma (NF-e
   VIII e IX, e a 1ª pergunta da I), pergunta numerada em texto ("8. É possível…") ou só pelo `<ol start="2">`
   (NFC-e I), pergunta e resposta no **mesmo `<p>`** separadas por `<br>` (NF-e VIII). A pergunta é localizada
   pela âncora que o índice anuncia, trocada por uma marca antes da conversão para texto; sem ela, pela primeira
   linha, depois da pergunta anterior, que começa com o `N.` do índice. As âncoras que o índice não anuncia
   (`emissao`, os `OLE_LINK` do Word) são ignoradas. Nas 7 sem âncora, o link abre o topo da página, como no portal.
3. **O índice e a página não escrevem a pergunta igual** (9 de 149: "ás"/"às", "IEmpresa"/"Empresa", "–
   NF-e"/"(NF-e)"). O texto vem da página, que é o que se lê no destino do link e tem as correções; o do índice
   serve de conferência.
4. **O sub-tópico é do índice.** A página o repete como "• Título" antes da pergunta que o abre (6 dos 8) ou o
   omite ("Pessoa Física", "Novas Informações com Versão 3.10"). A linha "• <sub-tópico>" no fim de uma resposta
   sai; no meio, é texto.
5. **HTML do editor:** só `&nbsp;` e `&gt;` como entidades e nenhum `<` cru nas 17 páginas → toda tag sai e as
   entidades são decodificadas depois (a regra do SP); célula vira espaço; 3 `href` relativos ganham o host.
   "novo conteúdo" (marcador editorial em vermelho, na IX.3) fica, como está.

## Fonte 2 — a base "SRE - Dúvidas Frequentes" do CSM (`csm.rs`)

O mesmo ServiceNow dos serviços (`atendimento2.fazenda.mg.gov.br/csm`). **`robots.txt` = `Disallow: /`** no host
inteiro: a coleta é decisão do mantenedor, sem autorização da SEF-MG ([auli_pendencias.md](auli_pendencias.md)
§42). 81 artigos em três categorias; a natureza vem da categoria, numa lista fechada no código:

| Categoria | Artigos | Vira |
|---|---|---|
| DF-e - Correção de Erros / Rejeições | 61 | 61 perguntas: o título ("Rejeição 215: Falha no schema XML.") e a seção "Resposta para o Erro" |
| Sintegra - PED | 1 | 100 perguntas: um par P./R. da tabela por pergunta, sem o `N)` |
| Orientações Tributárias DOLT/SUTRI | 19 | fora — documentos de 4 a 65 mil caracteres, não FAQ (não são baixados) |

As armadilhas do CSM, na mesma numeração:

6. **A lista só vem com sessão.** A página `kb_search` não traz os resultados: o widget "Knowledge Result" os pede
   num POST que exige o cookie e o token `g_ck` que a página HTML entrega a qualquer visitante (sem login). Sem
   eles, `invalid_token: true`. Um GET da página abre a sessão; o POST, com a janela `end: 100`, traz a base inteira
   e a contagem do portal (`meta.count`).
7. **Filtrar pela base.** Sem o filtro a busca devolve 235: a base "SRE - Serviços públicos" é a dos serviços do
   `mg.rs`, já no acervo.
8. **Robô não recebe o texto.** Com o AuliBot, o ServiceNow responde `isCrawlerBot: true` e omite o artigo: o
   módulo usa o UA do kit, como os serviços (D-MGFAQ-8).
9. **O que a página do artigo engana:** a "Descrição do Erro" vem cortada em ~40 caracteres (a pergunta é o
   título, que para em 100 em 6 deles — limite da base); o "Assunto" (só na 690) repete o título; o editor emite
   entidades numéricas (`&#34;`, `&#61;`, `&#43;`), decodificadas antes das do kit. A lista vem por visualizações:
   as rejeições saem pelo título, para o snapshot só mudar quando a base muda.

## Guardas e transporte

- `Err` antes de gravar qualquer arquivo: home sem módulo; página sem `#conteudo` ou sem menu lateral; link de
  pergunta do índice fora de linha de tabela (contagem crua ≠ parseados); (página, âncora) repetida; pergunta do
  índice não localizada na página; linha localizada que não é a pergunta do índice (menos da metade das palavras
  do índice — a menor medida é 0,91); página de "Perguntas…" que linka páginas do SPED mas nenhuma de respostas
  (índice com markup novo — sem isto o NF-e sumiria com um aviso); zero perguntas no total; qualquer chamada que
  falhe. Resposta vazia só avisa (0 em 01/11).
- `Err` antes de gravar, no CSM: sessão recusada; busca vazia ou com menos itens que o `meta.count`; artigo de
  outra base, sem exatamente uma categoria, de categoria fora da lista ou repetido; página servida a robô ou sem o
  número do artigo; rejeição sem "Resposta para o Erro"; tabela do PED sem pares P./R. alternados ou com buraco
  na numeração; pergunta ou resposta vazia.
- **Sempre à rede**, como no SC, no PR e no SP: o cache em `data/mg/raw/cache/faqs/` só é lido com `--usecache`
  (no CSM, a lista e os artigos, sem sessão), que reproduz a árvore byte a byte e o snapshot a menos do
  `coletado_em`. Os serviços do MG seguem cache-first.
- SPED: 37 GETs com o UA `AuliBot`. CSM: 64 requisições (a página, o POST, 62 artigos) com o UA do kit. 1 s de
  cortesia nos dois, ~2 min.
- Saída: `data/mg/raw/mg-faqs-tree.json` (raiz → `Menu` por módulo do SPED → seção: `Faq` sem sub-tópico, `Menu`
  com um `Faq` por sub-tópico quando tem; a III.1 do NF-e, que vem antes do primeiro sub-tópico, fica nos
  `faq_items` do `Menu` da seção, e a aba a mostra depois dos filhos; depois, `Menu` "SRE - Dúvidas Frequentes" → um
  `Faq` por categoria) e `data/mg/mg-faqs-snapshot.json` (um `FaqRaw` por pergunta, trilha `<módulo> | <seção>`,
  `<módulo> | <seção> | <sub-tópico>` ou `SRE - Dúvidas Frequentes | <categoria>`), nessa ordem e atômicos.
- Atualização: `scripts/tools/update-mg-faqs.sh` (guarda de encolhimento contra o `.anterior`).

---

<a id="ms-faqs"></a>

# MS FAQs — Perguntas Frequentes da FAQ do Fale Conosco MS (www.faleconosco.ms.gov.br/faq)

**Discovery e implantação:** 02/11/2026. Estudo, medições e decisões D-MSFAQ-1..12:
[`docs/ESTUDO-MS-Faqs.md`](ESTUDO-MS-Faqs.md) (§8 = a sondagem da API e a implementação). Coleção `faqs` do
`auli-scraper-ms` (`auli-scraper-ms faqs`, ou `all` = serviços + FAQs). **1.200 perguntas** em 71 assuntos e 384
subtemas na rodada de 02/11.

## Fonte

O site da SEFAZ-MS (WordPress) não tem FAQ própria — as páginas de tema linkam "Perguntas Frequentes – SEFAZ MS" para
a FAQ do Fale Conosco, uma aplicação AngularJS 1.2 sobre uma API REST pública: POST
`https://www.faleconosco.ms.gov.br/faq/recursos/faq/<ação>` com corpo `[{"key": …, "value": …}]`, sem sessão. Sem
`robots.txt` (404); o AuliBot passa.

- **Assuntos:** `listassunto` → 71, cada um com `qtdeFaq`, a contagem do portal (a guarda natural).
- **Subtemas:** `listdetalhamentoassunto` (`idAssunto`) → 384 ao todo (o NF-e tem 24).
- **Perguntas:** `listfaqpordetalhamentoassunto` (`idAssunto`, `idDetalhamentoAssunto`) → `id`, `pergunta` e uma
  prévia da resposta.
- **Resposta:** `getrespostafaq` (`idFaq`) → `{"data": ["<html>"]}`.

## As armadilhas

1. **A resposta da lista é uma prévia.** Cortada em ~100 caracteres, no meio da tag e do link; a inteira é uma
   chamada por pergunta — 1.200 das 1.656 requisições da rodada.
2. **A página de ouvidoria (`ouvidoria.jsf`) não é o catálogo.** Lista 70 assuntos; a API tem 71 — faltam BP-e (26
   perguntas) e NFGas (1).
3. **Sem rota por pergunta.** A pergunta abre num clique dentro da página do subtema. O link é o do subtema com
   `?idFaq=<id>`, que o AngularJS 1.2 ignora na rota (abre o subtema) e que separa no acervo as duas perguntas que o
   portal repete no mesmo subtema em versões diferentes.
4. **HTML do Word.** Comentários condicionais com `<xml>` e `<style>`, e entidades nomeadas que o kit não conhece
   (`&ndash;`, `&ordm;`, `&sect;`, `&ldquo;`…) → o texto sai pelo parser HTML (`scraper`), que decodifica tudo e
   descarta os comentários.
5. **A ordem dos assuntos é da API**, e não é alfabética nem por id: a coleta ordena assuntos e subtemas pelo nome e
   as perguntas pelo `id`.

## Guardas e transporte

- `Err` antes de gravar qualquer arquivo: lista de assuntos vazia; resposta fora do formato `{"data": [...]}`;
  perguntas únicas de um assunto ≠ `qtdeFaq`; acervo vazio; pergunta sem texto; qualquer chamada que falhe. Avisos:
  subtema sem perguntas, resposta vazia (0 e 0 em 02/11).
- **Sempre à rede**; o cache em `data/ms/raw/cache/faqs/` (chave = a url da ação com os parâmetros na query) só é
  lido com `--usecache`, que reproduz a árvore byte a byte e o snapshot a menos do `coletado_em`.
- 1.656 POSTs com o UA `AuliBot` e 1 s de cortesia, ~30 min.
- Saída: `data/ms/raw/ms-faqs-tree.json` (raiz → `Menu` por assunto → `Faq` por subtema) e
  `data/ms/ms-faqs-snapshot.json` (um `FaqRaw` por pergunta, trilha `<assunto> | <subtema>`), nessa ordem e atômicos.
- Atualização: `scripts/tools/update-ms-faqs.sh` (apaga o cache antes; guarda de encolhimento contra o `.anterior`).

---

<a id="mt-faqs"></a>

# MT FAQs — Dúvidas do programa Nota MT (www.sefaz.mt.gov.br/notamt/duvida)

**Discovery e implantação:** 02/11/2026. Estudo, medições e decisões D-MTFAQ-1..9:
[`docs/ESTUDO-MT-Faqs.md`](ESTUDO-MT-Faqs.md). Coleção `faqs` do `auli-scraper-mt` (`auli-scraper-mt faqs`, ou
`all` = serviços + FAQs). **135 perguntas** em 9 grupos na rodada de 02/11.

## Fonte

A FAQ do Nota MT, o programa de cidadania fiscal da SEFAZ-MT. A página `notamt/duvida` é um shell Angular; o módulo
da dúvida (chunk lazy) chama `GET https://www.sefaz.mt.gov.br/notamt/api/duvidas` — sem sessão, sem corpo — e
agrupa o resultado num acordeão. **Uma chamada** traz tudo: `id`, `descDuvida`, `descResposta` (a resposta
inteira), `descGrupoDuvida` (`"3 - PRÊMIOS E ENTIDADES"`), `statusDuvida` (`{ativo, valor, id}`) e
`numrOrdemDuvida`. O `robots.txt` do host é inalcançável (o https redireciona para o http e de volta, em laço);
o AuliBot passa.

## As armadilhas

1. **Não é o site institucional.** O `www5.sefaz.mt.gov.br` (Liferay) tem um hub "Perguntas frequentes" e PDFs
   "FAQ <tema>", mas declara `Disallow: /` para todo robô que não seja Google ou Bing — fase 2, pendente da decisão
   de robots (D-MTFAQ-3). O catálogo X-Via dos serviços não tem FAQ.
2. **Sem rota por pergunta.** O SPA só tem a rota `duvida`: todas as 135 perguntas têm o mesmo link, e o nome do
   `.md` (pergunta + hash de link e pergunta) depende só da pergunta — repetida, um arquivo sobrescreveria o outro.
   Guarda `Err` (0 repetidas em 02/11).
3. **A ordem da API é embaralhada.** A coleta ordena os grupos pelo número do prefixo e as perguntas pela
   `numrOrdemDuvida` (uma sem ordem, a 136, vai para o fim do grupo; o `id` desempata).
4. **HTML simples e às vezes malformado** (`</li>•item</li></br>`): `<br>`, listas, `<b>`, dois links. Conversão
   por regex (link → `texto "url"`, bloco → quebra de linha, resto sai); nenhuma entidade em 02/11.
5. **Grupo sozinho não diz de quê.** "PROGRAMA" e "CADASTRO" são do Nota MT: a trilha é `Nota MT | <GRUPO>` (entra
   no texto vetorizado) e a árvore tem o nível do programa (a aba de FAQs começa pelos filhos da raiz).

## Guardas e transporte

- `Err` antes de gravar qualquer arquivo: resposta fora do formato (array de dúvidas com todos os campos); `id`
  repetido; grupo fora de `N - NOME`; acervo vazio (nenhuma dúvida ativa); pergunta sem texto; pergunta repetida;
  a chamada que falhe. Avisos: dúvidas inativas (ficam fora), resposta vazia (0 e 0 em 02/11).
- **Sempre à rede**; o cache em `data/mt/raw/cache/faqs/` só é lido com `--usecache`, que reproduz a árvore byte a
  byte e o snapshot a menos do `coletado_em`.
- 1 GET com o UA `AuliBot`, segundos.
- Saída: `data/mt/raw/mt-faqs-tree.json` (raiz → `Menu` Nota MT → `Faq` por grupo, tudo com o link da página,
  itens sem `url`) e `data/mt/mt-faqs-snapshot.json` (um `FaqRaw` por pergunta), nessa ordem e atômicos.
- Atualização: `scripts/tools/update-mt-faqs.sh` (apaga o cache antes; guarda de encolhimento contra o `.anterior`).

---

<a id="go-faqs"></a>

# GO FAQs — Perguntas Frequentes do site Orientação Tributária (orientacaotributaria.sefaz.go.gov.br)

**Discovery e implantação:** 02/11/2026. Estudo, medições e decisões D-GOFAQ-1..10:
[`docs/ESTUDO-GO-Faqs.md`](ESTUDO-GO-Faqs.md). Coleção `faqs` do `auli-scraper-go` (`auli-scraper-go faqs`, ou
`all` = serviços + FAQs). **1.298 perguntas** em 115 termos de categoria (44 temas) na rodada de 02/11.

## Fonte

O site "Orientação Tributária" da Economia-GO, um SharePoint 16 instalado no órgão, linkado pela página de orientação
tributária do `goias.gov.br`. A REST `_api` é anônima: `GET /_api/web/lists/getbytitle('Perguntas Frequentes')/items`
com `$top=5000` traz a lista inteira (`Pergunta0` e `Resposta` em HTML, `CategoriaId` = o `TermGuid`, o campo oculto
`m611689e352b49adae244c5880ecdf12` = `Nome|TermGuid`, `Modified`). Sem `robots.txt` (404). A mesma `_api` lista a
biblioteca `Pareceres` (1.266 documentos) — fora desta coleção.

## As armadilhas

1. **O host "canônico" está quebrado.** A página declara `orientacaotributaria.economia.go.gov.br` como canônico, mas
   ele responde 404 a tudo; o site e os links são do `…sefaz.go.gov.br`.
2. **A REST não dá o caminho da categoria.** O `Label` do campo de taxonomia é o índice na lista oculta (`"3"`), o
   campo oculto dá só o nome do termo, e "Informações Gerais" nomeia ~20 termos de temas diferentes. A lista oculta
   da taxonomia exige login (401); `TaxCatchAll` e `FieldValuesAsText` não servem; o `owssvr.dll` é barrado pelo WAF
   (403). O caminho ("Decreto nº 4.852/97 - RCTE:Informações Gerais") vem do **formulário público do item**
   (`DispForm.aspx?ID=`), que embute o item em JSON com a categoria pelo caminho (`IsPathRendered`): um formulário
   por termo, o do menor `Id`.
3. **Termo renomeado.** O campo oculto guarda o nome de quando o item foi salvo: 3 itens de 2018 dizem
   "Informações Fiscais" num termo que hoje é "Informações Gerais". Vale o formulário; a conferência que pega
   formulário mal lido é por termo (o caminho termina no nome que algum item dá ao termo).
4. **A FAQ está parada.** O site avisa "atualizadas até 31/12/2019" (07/07/2021); há itens alterados até 26/08/2022.
   A trilha abre com `Orientação Tributária (até <ano da alteração mais recente>)` (decisão do mantenedor).
5. **Perguntas repetidas** (49 textos, 99 itens) → o link é o formulário de cada item.
6. **HTML do editor SharePoint** (o mesmo das FAQs do SP): `&#58;` no lugar de `:` até nos `href`, links relativos
   para PDFs de pareceres e manuais, tabelas, imagens, símbolos matemáticos (`&#119872;`). Respostas só com imagem
   (6) viram o link da imagem.
7. **Dados pessoais no formulário:** autor e editor (nome e e-mail de servidores). O scraper só lê o `"Categoria"`.

## Guardas e transporte

- `Err` antes de gravar qualquer arquivo: resposta da `_api` fora do formato; `__next` (a lista passou de 5.000);
  lista vazia; `Id` repetido; pergunta vazia; item sem categoria ou sem o nome do termo; formulário sem exatamente
  um `"Categoria"`; caminho com nível vazio ou que não termina no nome de nenhum item do termo; qualquer GET que
  falhe. Avisos: itens com nome antigo do termo (3), respostas vazias (0).
- **Sempre à rede**; o cache em `data/go/raw/cache/faqs/` só é lido com `--usecache`, que reproduz a árvore byte a
  byte e o snapshot a menos do `coletado_em`.
- 116 GETs (a lista + 115 formulários) pelo `ureq`, UA `AuliBot`, 1 s de cortesia, ~2 min. O WAF JA3 do GO é só do
  `api.go.gov.br` (serviços).
- Saída: `data/go/raw/go-faqs-tree.json` (raiz → `Menu` com a marca do site → os níveis do caminho; nós na home do
  site, que é pública — a página de FAQ exige login —, itens com o formulário) e `data/go/go-faqs-snapshot.json`
  (um `FaqRaw` por pergunta), nessa ordem e atômicos.
- Atualização: `scripts/tools/update-go-faqs.sh` (apaga o cache antes; guarda de encolhimento contra o `.anterior`).

---

<a id="df-faqs"></a>

# DF FAQs — Perguntas e Respostas do Portal da Receita e Perguntas Frequentes do ISS/NFS-e

**Discovery e implantação:** 02/11/2026. Estudo, medições e decisões D-DFFAQ-1..10:
[`docs/ESTUDO-DF-Faqs.md`](ESTUDO-DF-Faqs.md). Coleção `faqs` do `auli-scraper-df` (`auli-scraper-df faqs`, ou
`all` = serviços + FAQs). **993 perguntas** na rodada de 02/11: 900 da Receita (22 temas, 81 tipos de atendimento) e
93 do ISS/NFS-e (6 seções).

## Fonte

- **(A) Receita** — `www.receita.fazenda.df.gov.br/aplicacoes/perguntasfrequentes/` (ColdFusion, o host da Carta de
  Serviços): `perguntasfrequentes.cfm` é uma casca com um formulário por tema (`codAssunto` + o `select` dos tipos de
  atendimento); as perguntas chegam por `POST perguntasFreqBusca_div.cfm` (`selectTipoAtendimento`, `codAssunto`,
  `buscaPergunta`), com `tipo=0` = o tema inteiro. `robots.txt`: `Allow: /`.
- **(B) ISS/NFS-e** — `iss.fazenda.df.gov.br/online/Default/PerguntasFrequentes.aspx` (ASP.NET, plataforma terceirizada
  sob o domínio oficial): uma página, `<h5>` por seção e pares `div.accordionHeader`/`div.accordionContent`. Sem
  `robots.txt` (404).
- Fora: o site do Nota Legal (403 ao AuliBot; o tema está na Receita com 58 perguntas).

## As armadilhas

1. **O WAF JA3 da Carta vale para a FAQ** (mesmo host): o `ureq` leva reset; o curl passa — e passa com o AuliBot.
   O kit ganhou o POST de formulário pelo curl e o UA como parâmetro do caminho curl.
2. **Respostas truncadas no meio de um comentário do Word.** O banco corta o campo em ~64 KB; 3 respostas acabam
   num `<!-- [if gte mso 9]><xml>…` sem `-->`. Num parse único do fragmento, o comentário aberto engole as perguntas
   seguintes (no navegador, o tema ITCD mostra 13 das 28). O fragmento é cortado nos embrulhos de cada pergunta
   (`div#collapsePerguntasFrequentes`, id repetido) antes do parse.
3. **Lixo do Word nas respostas** — CSS `mso-` e XML em comentários condicionais (33 respostas): o conversor tira
   `<style>`, `<script>` e comentários inteiros. O `html_to_text` dos serviços não serve (deixa o CSS vazar e apaga
   os links).
4. **`href` que não é endereço:** caminho de arquivo do Outlook (`file:///C:/Users/…`), um parágrafo inteiro no lugar
   da URL, `javascript:` no ISS — só `http(s)` e `mailto` entram no texto.
5. **Prefixo "R."** em 723 respostas, com variantes (`R:`, `R .`, `R.:`, colado, dobrado, `R` sem pontuação) — sai só
   no começo.
6. **Perguntas de texto repetido:** 50 textos em mais de um id na Receita (30 com respostas diferentes), 18 entre
   seções do ISS. A app não tem endereço por pergunta → o link leva o id (`#collapse<id>`) na Receita e a seção
   (`#<slug>`) no ISS; senão um `.md` sobrescreveria outro.
7. **O ISS não tem identidade estável:** os GUIDs dos itens mudam a cada carga. Repetição dentro da mesma seção (2,
   uma só com outras quebras de linha) sai uma vez; resposta diferente de verdade é `Err`.
8. **Os tipos de atendimento não são as classes dos serviços** (0 de 81): são a taxonomia do Atendimento Virtual. O
   sufixo `" - serviço"` (49 rótulos) sai da trilha.

## Guardas e transporte

- `Err` antes de gravar qualquer arquivo: menos de 18 temas; tema repetido ou sem título na casca; `codAssunto` ou
  tipo não numérico; embrulho sem exatamente um `div.card`, card sem link ou sem resposta; tipo com pergunta fora do
  tema; id repetido entre temas; pergunta vazia; menos de 800 perguntas da Receita ou de 80 do ISS; repetição
  divergente na mesma seção do ISS; erro do ColdFusion; qualquer chamada que falhe. Avisos: pergunta em dois tipos
  (fica o primeiro; 0), tema sem perguntas (0), repetição na seção do ISS (2), resposta vazia (0).
- **Sempre à rede**; o cache em `data/df/raw/cache/faqs/` (chave = URL + campos do POST) só é lido com `--usecache`,
  que reproduz a árvore byte a byte e o snapshot a menos do `coletado_em`.
- Receita pelo curl (1 GET + 103 POSTs), ISS pelo `ureq` (1 GET); UA `AuliBot`, 1 s de cortesia, ~2 min.
- Saída: `data/df/raw/df-faqs-tree.json` (raiz → `Receita-DF` → temas → tipos, e `ISS/NFS-e` → seções, na ordem do
  portal; nós na página da fonte, itens com o link de cada pergunta) e `data/df/df-faqs-snapshot.json` (um `FaqRaw`
  por pergunta), nessa ordem e atômicos.
- Atualização: `scripts/tools/update-df-faqs.sh` (apaga o cache antes; guarda de encolhimento contra o `.anterior`).

---

<a id="am-faqs"></a>

# AM FAQs — Perguntas Frequentes das páginas do Portfólio de Serviços (www.sefaz.am.gov.br)

**Discovery e implantação:** 02/11/2026. Estudo, medições e decisões D-AMFAQ-1..10:
[`docs/ESTUDO-AM-Faqs.md`](ESTUDO-AM-Faqs.md). Coleção `faqs` do `auli-scraper-am` (`auli-scraper-am faqs`, ou
`all` = serviços + FAQs). **176 perguntas** na rodada de 02/11, de 47 das 306 páginas da árvore (285 lidas, 109 cópias
entre páginas irmãs).

## Fonte

- **A seção "Perguntas Frequentes" do detalhe** de cada página do Portfólio de Serviços —
  `GET /portfolio-servicos/detalhes/{id}` com `RSC: 1` (o flight RSC, o mesmo transporte dos serviços). No
  `serviceDetails`, `visibleSections` diz se a seção aparece e `perguntasRespostasHtml` traz o HTML — inline ou como
  referência `"$c"` a um bloco de texto `c:T<tamanho hex>,<html>` do mesmo flight.
- O índice é a árvore `items` de `/portfolio-servicos/todos` (a dos serviços): os 243 serviços com detalhe e as 63
  categorias com página — uma delas, a 302, tem perguntas. `robots.txt`: não existe (404).
- Fora: o Portal do Servidor Fazendário (64 páginas no mesmo flight, 5 com FAQ — RH dos servidores); as Perguntas e
  Respostas do GCC (casca Struts2-jQuery, conteúdo por JS — pendência); a FAQ do Portal NFC-e (PDF de 2017).

## As armadilhas

1. **Os blocos de texto do flight vêm colados** (`…</p>b:T…`, sem `\n`) e o tamanho é em **bytes**. Buscar `c:T`
   direto acharia o `1c:T` (os ids de linha são hex e passam de 30 num detalhe): o flight é lido de linha em linha
   desde o começo.
2. **Seção oculta com marcação no campo** (a 1143: 1.264 bytes de `<p>&nbsp;</p>`): o portão é `visibleSections`, não
   o campo.
3. **O flight da listagem tem mais do que a árvore:** 370 ids em URLs de detalhe, 64 deles no bloco
   `linksPortalServidorFazendario`. O índice é a árvore `items`, como nos serviços.
4. **HTML de editor livre.** Perguntas numeradas `1)`, `1.`, `1 -`, `1 –` em `<p>`, `<h3>`, `<div>` ou `<li>`; pergunta
   e resposta no mesmo `<p>`, separadas por `<br>` (às vezes `<br data-start=…>`); `<div>` aninhado; tabela e
   listas na resposta; comentários do Word (4 páginas); links (16 páginas). O conversor das FAQs do DF dá as linhas.
5. **Número que não é pergunta:** `2.1)` é subtítulo dentro da resposta do REDAR; o REDAR tem 2 tópicos numerados sem
   "?" em 5 (viram a pergunta inteira); um passo a passo numerado sem "?" não pode virar perguntas — daí a regra dos
   50 %.
6. **Seção sem nenhuma pergunta** (302, texto corrido sobre regime especial) → 1 FAQ, "Dúvidas frequentes — <nome>".
7. **Cópias entre páginas irmãs:** o Refis em 5 páginas, parcelamentos, REDAR, certidões, PCD, IE — ficam na de menor
   id.
8. **`?profile=…` e `?gerarTaxa=true&profile=…`** nas URLs da árvore: o detalhe é o mesmo (conferido no 882); o link
   sai sem query, como o dos serviços.

## Guardas e transporte

- `Err` antes de gravar qualquer arquivo: flight sem a âncora; `visibleSections` ou `perguntasRespostasHtml` fora de
  "uma vez só"; seção visível com o campo `null`, com referência que não leva a um bloco ou com HTML sem texto;
  referência RSC de outro tipo; pergunta sem resposta; a mesma pergunta duas vezes na página com respostas diferentes;
  menos de 280 páginas, de 40 com perguntas ou de 150 perguntas; qualquer GET que falhe. Avisos: numeração fora de
  ordem (0), linhas antes da primeira pergunta (0).
- **Sempre à rede**; o cache em `data/am/raw/cache/faqs/` (chave = URL) só é lido com `--usecache`, que reproduz a
  árvore byte a byte e o snapshot a menos do `coletado_em`.
- `ureq`, UA `AuliBot`, 1 s de cortesia; 307 GETs (~230 KB cada), ~13 min.
- Saída: `data/am/raw/am-faqs-tree.json` (raiz → categorias de topo → páginas, na ordem do portfólio; a categoria com
  perguntas próprias as tem no próprio nó; nós no detalhe, ou na listagem quando a categoria não tem perguntas) e
  `data/am/am-faqs-snapshot.json` (um `FaqRaw` por pergunta), nessa ordem e atômicos.
- Atualização: `scripts/tools/update-am-faqs.sh` (apaga o cache antes; guarda de encolhimento contra o `.anterior`).

---

<a id="ma-faqs"></a>

# MA FAQs — Perguntas Frequentes do Portal SGC (portal-sgc.sefaz.ma.gov.br/portal/perguntas-frequentes)

**Discovery e implantação:** 06/11/2026. Estudo, medições e decisões D-MAFAQ-1..4:
[`docs/ESTUDO-MA-Faqs.md`](ESTUDO-MA-Faqs.md). Coleção `faqs` do `auli-scraper-ma` (`auli-scraper-ma faqs`, ou
`all` = serviços + FAQs). **336 perguntas** em 20 raízes e 55 subcategorias na rodada de 06/11.

## Fonte

A página "Perguntas Frequentes" do Portal SGC — o mesmo SPA Angular sobre a API Spring dos serviços (`#ma`). O módulo
da página (chunk lazy `src_app_portal_faq_faq_module_ts`, achado pelo mapa de chunks do `runtime.js`) chama
`GET /sgc/api/faq/perguntas/faq?publicado=true` com o mesmo token anônimo (login com as credenciais públicas do
bundle, header `AuthorizationPortal`); sem token, 401. **Uma chamada** traz tudo: `id`, `pergunta` (texto puro),
`resposta` (HTML de editor rico), `ativo`, `publicado` e `categoria` (`{id, nome, ordem, ativo, publico,
categoriaPai}`). Nenhum host da SEFAZ-MA tem `robots.txt` (404).

## As armadilhas

1. **O filtro é obrigatório e é o único que vale.** Sem `publicado=true` vêm 353 itens: os 17 a mais são da categoria
   inativa "DFE.02 - CANAL VERDE", que a página não mostra — e que dizem `publicado: true` no próprio item. A coleta
   manda o filtro exatamente como a página e não filtra por flag nenhum do lado dela.
2. **A `categoriaPai` aninhada vem rasa.** Dentro de uma subcategoria, a mãe chega com `ordem: null` e `ativo: false`
   (valores de omissão); na pergunta pendurada direto na raiz, a mesma categoria vem completa (`ordem` 0–20). Nome e
   ordem da raiz saem só daí — como na página, que só desenha um grupo que tem pergunta própria. Por isso toda raiz
   com subcategorias tem 1 pergunta de apresentação ("Sobre o IPVA", "Sobre a EFD"…), e subcategoria cuja raiz não
   tem nenhuma é `Err` (a página não a mostraria).
3. **Sem rota por pergunta.** O template é um acordeão (`ngbCollapse` + `innerHTML`) sem `id`, âncora nem parâmetro:
   as 336 têm o link da página, e o nome do `.md` (pergunta + hash de link e pergunta) depende só da pergunta. Duas
   "Sobre a seção" (RTC e Ativação de Novas Empresas) colidiriam → a pergunta que aparece em mais de uma raiz ganha o
   nome da raiz entre parênteses (D-MAFAQ-2); repetida dentro da mesma raiz é `Err`.
4. **Espaços nos nomes** (`" REFIS - …"`, `"NOVO SISTEMA DE  DOWNLOAD …"`) e nas perguntas (`"IP.01.001 -  Qual…"`):
   normalizados. **`U+FEFF`** colado ao texto em 6 respostas: removido (o `clean` do kit não o tira).
5. **HTML de editor rico**: `<p>`, `<br>`, `<strong>`, listas, 370 links, muito `<p><br></p>`; sem imagem nem iframe.
   Conversão por regex (link → `texto "url"`, bloco → quebra de linha, entidades decodificadas, linhas vazias fora).
   Três `href` vêm malformados na fonte (um e-mail sem `mailto:`, um host sem esquema, um trecho de texto) e ficam
   como vieram.

## Guardas e transporte

- `Err` antes de gravar qualquer arquivo: resposta fora do formato; `id` repetido; categoria sem nome; categoria de
  três níveis; subcategoria sem raiz com pergunta própria; acervo vazio; pergunta sem texto; pergunta repetida
  depois da desambiguação; o login ou a chamada que falhe. Aviso: resposta vazia depois da conversão (0 em 06/11).
- **Sempre à rede**; o cache em `data/ma/raw/cache/faqs/` só é lido com `--usecache`, que reproduz a árvore byte a
  byte e o snapshot a menos do `coletado_em`.
- 1 login + 1 GET com o UA `AuliBot`, segundos. O agente é o dos serviços, com o intermediário GlobalSign embutido;
  desde 11/2026 o `portal-sgc` manda a cadeia inteira, e a âncora segue valendo (mesmo emissor).
- Ordem: raízes pela `ordem`, subcategorias pelo `id` (como a página), perguntas pelo texto (`id` desempata).
- Saída: `data/ma/raw/ma-faqs-tree.json` (raiz → `Faq` por raiz sem subcategoria, `Menu` com a pergunta de
  apresentação e um `Faq` por subcategoria nas outras; tudo com o link da página, itens sem `url`) e
  `data/ma/ma-faqs-snapshot.json` (um `FaqRaw` por pergunta, trilha `<raiz>` ou `<raiz> | <subcategoria>`), nessa
  ordem e atômicos.
- Atualização: `scripts/tools/update-ma-faqs.sh` (apaga o cache antes; guarda de encolhimento contra o `.anterior`).

---

<a id="pe-faqs"></a>

# PE FAQs — Perguntas e Respostas do IPVA e do ITCMD (www.sefaz.pe.gov.br/Servicos/…/Perguntas-e-Respostas.aspx)

**Discovery e implantação:** 06/11/2026. Estudo, medições e decisões D-PEFAQ-1..6:
[`docs/ESTUDO-PE-Faqs.md`](ESTUDO-PE-Faqs.md). Coleção `faqs` do `auli-scraper-pe` (`auli-scraper-pe faqs`, ou
`all` = serviços + FAQs). **59 perguntas** (IPVA 40, ITCMD 19) na rodada de 06/11.

## Fonte

Duas páginas de publicação do mesmo SharePoint 2013 dos serviços: `/Servicos/IPVA/Paginas/Perguntas-e-Respostas.aspx`
e `/Servicos/ICD/Paginas/Perguntas-e-Respostas.aspx`, ambas atualizadas para 2025/2026 (EC 137/2025, LC 563/2025). A
`_api` e o `listdata.svc` respondem 403 ("Acesso Negado") → HTML server-side, lista curada (molde PA/PR), 2 GETs de
~800 KB com 1 s entre eles. O corpo de cada página é o único `div.ms-rtestate-field[id$="_RichHtmlField"]`.
`robots.txt` gerido pela Cloudflare: `User-agent: *` → `Allow: /`, `Content-Signal: search=yes,ai-train=no,
use=reference`; ~35 robôs nomeados com `Disallow: /`, não o AuliBot.

## As armadilhas

1. **A pergunta é o número, não o negrito.** O negrito é irregular (`<b>` depois de `<a name>`, número e texto em
   `<strong>` separados, `<strong>` partido no meio da palavra). A coleta segmenta pela sequência: `N. ` com N = a
   anterior + 1 abre pergunta; `N.M - ` com N = a atual, M = o subitem seguinte e texto terminado em "?" também
   (IPVA 5.1; ITCMD 11.1, 11.2). Os 4.1/4.2 do ITCMD não terminam em "?" — são partes da resposta 4.
2. **As respostas têm listas numeradas** (IPVA: 14 linhas; ITCMD: 2) que recomeçam em 1 e nunca alcançam a pergunta
   seguinte. Número acima do esperado é `Err` (a numeração pulou); o número da pergunta atual repetido numa linha
   com "?" também — seria o sinal de uma lista que "roubou" a vez da pergunta.
3. **Espaço depois do ponto.** Na tabela de alíquotas do ITCMD, "19.600,00" e "34.600,00" casariam "19." e "34.".
4. **Tabelas são resposta** (calendário de vencimentos do IPVA, alíquotas do ITCMD): as células embrulham o texto em
   `<p>`, então linha com mais de uma célula vira UMA linha, células com ` | `. Linha de uma célula só (as caixas
   "IMPORTANTE") segue como bloco.
5. **O IPVA fecha com `<hr>` + rodapé** da Gerência de IPVA ("O órgão que normatiza… Estamos prontos a lhe prestar
   serviços"): o `<hr>` corta a resposta até a próxima pergunta. O que vem antes da 1ª pergunta também sai (ITCMD:
   a nota da LC 563/2025).
6. **Entidades numéricas** (`&#58;` ~2,6 mil vezes, `&#160;`) e `U+200B` em rajadas: DOM (html5ever) + `clean`, não
   regex + `decode_entities`.
7. **Âncoras parciais.** 16 das 39 perguntas de 1º nível do IPVA têm `<a name="N">`: essas levam `página#N`; as outras
   (e todo o ITCMD) levam a página.

## Guardas e transporte

- `Err` antes de gravar qualquer arquivo: página sem o campo de conteúdo ou com mais de um; página sem pergunta;
  numeração que pula; número repetido; pergunta sem resposta; par (link, pergunta) repetido; um GET que falhe.
- **Sempre à rede**; o cache em `data/pe/raw/cache/faqs/` só é lido com `--usecache`, que reproduz a árvore byte a
  byte e o snapshot a menos do `coletado_em`.
- `ureq` com o UA `AuliBot` (D-FROTA-UA, D-PEFAQ-5) — o mesmo `crate::UA` dos serviços desde 06/11/2026 (D-PE4).
- Saída: `data/pe/raw/pe-faqs-tree.json` (raiz → um `Faq` por tributo; item com `url` só quando tem âncora) e
  `data/pe/pe-faqs-snapshot.json` (um `FaqRaw` por pergunta, trilha = o tributo), nessa ordem e atômicos.
- Atualização: `scripts/tools/update-pe-faqs.sh` (apaga o cache antes; guarda de encolhimento contra o `.anterior`).

---

<a id="ce-faqs"></a>

# CE FAQs — Perguntas e respostas do Portal de Serviços da SEFAZ-CE (portalservicos.sefaz.ce.gov.br)

**Discovery e implantação:** 03/11/2026. Estudo, medições e decisões D-CEFAQ-1..10:
[`docs/ESTUDO-CE-Faqs.md`](ESTUDO-CE-Faqs.md). Coleção `faqs` do `auli-scraper-ce` (`auli-scraper-ce faqs`, ou `all` =
serviços + FAQs). **2.706 perguntas** na rodada de 03/11, de 93 dos 96 artigos de FAQ (3.179 lidas, 473 cópias).

## Fonte

- **Os artigos de FAQ do catálogo `conteudo-geral`** ("Conteúdo - Geral", `_id 648af6fc64778b7336bf45d3`), o irmão do
  `servico-geral` dos serviços, na mesma API do Sydle ONE (`…/api/1/servicedesk-embedded/_classId/5cd5e83d…/`), com o
  mesmo Bearer anônimo do shell:
  - `POST getChildren` (o corpo dos serviços com `id`/`rootId` = a raiz e `path` = `conteudo-geral`; `pageSize` 10,
    até a página vazia): 43 páginas com itens + a vazia, **428 conteúdos** (`hits` diz 438 — o mesmo índice inflado
    dos serviços);
  - seleção pelo nome, `(?i)d[úu]vidas|pergunta|faq` (D-CEFAQ-2): **96 artigos** (a lista está no fim desta seção);
  - `POST getItem` `{"application":"sefaz-ceara","id":"<_id>"}` por artigo: o artigo inteiro. É o único método de
    leitura liberado ao anônimo — `get`, `getContent`, `getPage`, `getArticle`, `getDocument`, `getById` e `getFaq` dão
    403 `ERROR_USER_UNAUTHORIZED_TO_EXECUTE_METHOD`; `getChildren` com o `_id` do artigo dá `hits: 0` (é folha).
- O campo é o **`contentMarkdown`**. O `content`, ao lado, é um derivado com perda: sem as perguntas e com o
  `handleState(r)"` do atributo no texto.
- Fora (D-CEFAQ-7): `www.sefaz.ce.gov.br/perguntas-frequentes/` (301 → `www.ce.gov.br/sefaz/perguntas-frequentes/`, que
  não responde); as FAQs do **DAE** (`servicos.sefaz.ce.gov.br/internet/dae/…`) e da **NFA**
  (`www2.sefaz.ce.gov.br/nfaweb/perguntasFrequentes.do`) — o handshake TLS 1.2/1.3 não fecha; os PDFs "Perguntas
  Frequentes 7.x" (a FAQ federal da EFD, espelhada); a wiki do Siafe. Os 332 conteúdos do `conteudo-geral` que não
  são FAQ (Pareceres CECON, Comunicados COTRI, Notas Explicativas…) são pendência própria (§18 das pendências).

## As armadilhas

1. **Dois templates.** O de 2026 (85 artigos): pergunta em `<h2>` no `div.heading`, resposta em HTML com entidades
   (48 diferentes, de `&atilde;` a `&clubs;` — decodificadas pelo html5ever). O de 2024 (8 artigos — CTe,
   Geração/Exclusão de Débitos, Localização e Liberação, Alteração EFD, Procedimentos de Escrituração, Restituição de
   Taxas, isenções, Representação fiscal): pergunta na linha `## ` e resposta em **markdown**, onde a quebra de linha é
   estrutura (listas `- C860 …`, itens `a)`, parágrafos — 116 respostas multi-linha em texto puro). No HTML a quebra
   do fonte é espaço; no markdown, quebra.
2. **`>` dentro do atributo.** A tag do template de 2024 é `<sy-collapse ref="(r) => handleState(r)" … trigger-by=…>`:
   a resposta começa depois da **linha** da tag, não no primeiro `>`. A `<sy-collapse>` nunca fecha — a resposta vai
   até o fim do bloco, e o último bloco traz o `<script>` do botão "Expandir todas" (sai inteiro).
3. **A resposta pode ter `<h2>` próprio** ("Quais os canais de atendimento sobre mercadorias em trânsito?", template de
   2024, com `<h2>Unidades de atendimento</h2>` na resposta): a pergunta é procurada só no cabeçalho, antes da linha
   da `<sy-collapse>`.
4. **A mesma pergunta duas vezes no artigo.** O portal sufixa a âncora (`…-2`; 187 âncoras com sufixo), e 59 dessas
   repetições têm **outra resposta**. Com o `url` do artigo, dariam o mesmo `.md` (nome = slug da pergunta + hash de
   url + pergunta) e o `process` recusaria: o `url` é o artigo **com a âncora**, `…/<identifier>+<_id>#section-<slug>`.
5. **Âncora repetida sem sufixo** (1, na Escrituração): `ICMS/IPI` e `ICMS|IPI` dão o mesmo slug, com respostas
   diferentes. As duas entram, com aviso — a pergunta difere, o `.md` também.
6. **Cópias** — entre artigos e dentro do mesmo artigo (o Diferimento repete 5 das suas 17): 473. Comparadas a menos
   de espaços (7 grupos só diferem por um `<br>` no meio da frase); ficam no artigo de menor `_id`. A "Restituição de
   Taxas" inteira está em outro artigo e some da árvore. Os dois pares de artigos de mesmo nome ("Dúvidas sobre a
   Legislação", "Dúvidas sobre ICMS relacionado aos combustíveis") **não** são duplicatas inteiras: cada um fica com
   perguntas próprias.
7. **Artigos de FAQ sem seções** (3, `contentFormat: HTML`): "Bancos Conveniados e PIX" (lista), "Desvinculação de
   débitos" (`-`) e "Reforma Tributária – Perguntas e Respostas pelo Ministério da Fazenda" (link para o gov.br).

## Guardas e transporte

- `Err` antes de gravar qualquer arquivo: listagem com `_id` repetido ou acima de 80 páginas; `getItem` com outro
  `_id` ou corpo que não é o envelope; artigo com nº de âncoras `id="section-` ≠ nº de blocos; bloco sem âncora, sem o
  seu `<sy-collapse>` (ou com o gatilho fora da tag), sem pergunta ou sem resposta; mais de 10 artigos sem seções;
  a mesma pergunta na mesma âncora com respostas diferentes; menos de 80 artigos com perguntas ou de 2.400 perguntas;
  qualquer chamada que falhe. Avisos: âncora repetida (1); artigo sem seções (3, com o nome).
- **Sempre à rede**; o cache em `data/ce/raw/cache/faqs/` (chave = URL do método + `#conteudo-geral&page=N` ou
  `#<_id>`) só é lido com `--usecache`, que reproduz a árvore byte a byte e o snapshot a menos do `coletado_em`.
- `ureq`, UA `AuliBot` (os serviços do CE seguem com o UA de navegador, decisão registrada), 400 ms de cortesia;
  1 GET (shell) + 44 POSTs `getChildren` + 96 POSTs `getItem`, ~1 min.
- Saída: `data/ce/raw/ce-faqs-tree.json` (raiz → artigos, na ordem da listagem → perguntas; o nó aponta para o artigo,
  a pergunta para a sua âncora) e `data/ce/ce-faqs-snapshot.json` (um `FaqRaw` por pergunta: `origin` = o nome do
  artigo, íntegro), nessa ordem e atômicos.
- Conferência: uma implementação independente em Python sobre o cache da rodada dá o mesmo snapshot, item a item.
- Atualização: `scripts/tools/update-ce-faqs.sh` (apaga o cache antes; guarda de encolhimento contra o `.anterior`).

## Os 96 artigos de FAQ (03/11/2026, na ordem da listagem)

Perguntas = as que ficam no acervo depois das cópias.

| # | Artigo | `_id` | Perguntas |
|---|---|---|---|
| 1 | Dúvidas Gerais sobre IPVA | `650360ffd048b208a5468b09` | 7 |
| 2 | Dúvidas acerca da Contribuição sobre Bens e Serviços (CBS) | `66f690ee6e78d03226d13ead` | 6 |
| 3 | Dúvidas acerca do Fundo Nacional de Desenvolvimento Nacional | `66f690f36e78d03226d1819b` | 7 |
| 4 | Dúvidas acerca do Imposto sobre Bens e Serviços (IBS) | `66f690fa6e78d03226d2c565` | 27 |
| 5 | Dúvidas de Acesso e Erros do Sistema | `6a198b2060562f7e9f0d7451` | 8 |
| 6 | Dúvidas em relação a serviços e mercadorias do exterior | `6504811762f08b64cdd5f1bc` | 19 |
| 7 | Dúvidas gerais outros temas decorrentes da Reforma Tributária | `66f691002712ed018a0d0f5c` | 5 |
| 8 | Dúvidas gerais sobre o IPVA 2026 | `658d6441555b84511e3d79d6` | 8 |
| 9 | Dúvidas gerais sobre o SIMBA | `66f1bb8015b7d474f44d7fb7` | 10 |
| 10 | Dúvidas sobre Devolução do ICMS | `670fc29ac24b754fd8f02bca` | 9 |
| 11 | Dúvidas sobre ICMS relacionado aos combustíveis | `6504811a62f08b64cdd63f4b` | 26 |
| 12 | Dúvidas sobre ICMS relacionado aos combustíveis | `67d42a3e03bb4b5b8b3af690` | 31 |
| 13 | Dúvidas sobre ICMS relacionado aos serviços de comunicação | `6504811b62f08b64cdd64cd8` | 29 |
| 14 | Dúvidas sobre ICMS relacionado à energia elétrica | `6504811a62f08b64cdd63edb` | 14 |
| 15 | Dúvidas sobre Liberação de Mercadorias | `6504811a62f08b64cdd63f1b` | 23 |
| 16 | Dúvidas sobre Procedimentos para Emissão e Escrituração de Documentos Fiscais de Contribuintes no Regime Normal de ICMS | `671f9464bee91b2025a12e41` | 21 |
| 17 | Dúvidas sobre Regras de Transição decorrentes da Reforma Tributária | `66f6910525a1cd0148581e23` | 21 |
| 18 | Dúvidas sobre Suspensão e Cassação | `6504811962f08b64cdd62ce8` | 15 |
| 19 | Dúvidas sobre a Legislação | `650360f5d048b208a541c4b4` | 57 |
| 20 | Dúvidas sobre a Legislação | `6504811862f08b64cdd5fbaf` | 12 |
| 21 | Dúvidas sobre a devedor contumaz e suas implicações | `66f1bb7015b7d474f44bf388` | 8 |
| 22 | Dúvidas sobre a não-cumulatividade do ICMS | `6504811762f08b64cdd56aa9` | 8 |
| 23 | Dúvidas sobre a responsabilidade solidária do ICMS | `6504811762f08b64cdd564fd` | 9 |
| 24 | Dúvidas sobre a transferência de crédito do ICMS | `6504811a62f08b64cdd62d2b` | 15 |
| 25 | Dúvidas sobre alterações na Constituição Federal decorrentes da Reforma Tributária | `66f691ac2712ed018a1cdb69` | 9 |
| 26 | Dúvidas sobre alterações no Ato das Disposições Constitucionais Transitórias (ADCT) decorrentes da Reforma Tributária | `66f690e825a1cd0148538852` | 7 |
| 27 | Dúvidas sobre cálculos e valores relacionados ao ICMS | `6504811862f08b64cdd61434` | 42 |
| 28 | Dúvidas sobre diferimento do ICMS | `6504811762f08b64cdd578ba` | 12 |
| 29 | Dúvidas sobre isenções | `6503592f49e79469ba11764a` | 8 |
| 30 | Dúvidas sobre isenções e descontos | `650360efd048b208a53df6f1` | 27 |
| 31 | Dúvidas sobre isenções, descontos e benefícios | `6504811862f08b64cdd5fd69` | 25 |
| 32 | Dúvidas sobre o local da operação e da prestação | `6504811762f08b64cdd5f209` | 19 |
| 33 | Dúvidas sobre os contribuintes do ICMS | `6504811762f08b64cdd5f20b` | 12 |
| 34 | Dúvidas sobre produtos específicos | `6504811862f08b64cdd5f75d` | 15 |
| 35 | Dúvidas sobre reduções da base de cálculo | `6504811762f08b64cdd57367` | 9 |
| 36 | Dúvidas sobre representação fiscal para fins penais em Ilícitos Penais Cometidos por Meio de Fraudes Fiscais e suas implicações para o contribuinte | `66f1bb55542b7404da08e65f` | 3 |
| 37 | Dúvidas sobre transporte de mercadorias e passageiros | `6504811862f08b64cdd5fd4c` | 25 |
| 38 | Perguntas e Respostas Frequentes | `6638d071ccb4702dbe021ca7` | 27 |
| 39 | Principais Dúvidas Sobre BPe - Bilhete de Passagem Eletronico | `64da50f7170f5a26dc5ea02a` | 20 |
| 40 | Principais Dúvidas Sobre Pagamento de IPVA | `64da50f6170f5a26dc5e9135` | 15 |
| 41 | Principais Dúvidas sobre Acompanhamento Processual | `64da5102170f5a26dc5efb8f` | 6 |
| 42 | Principais Dúvidas sobre Alteração e Atualização EFD | `64da510a170f5a26dc5f3761` | 75 |
| 43 | Principais Dúvidas sobre Alteração e atualização Cadastral | `64da50f7170f5a26dc5e9e84` | 15 |
| 44 | Principais Dúvidas sobre Aproveitamento e Compensação de Crédito/Débito | `64da5103170f5a26dc5f0776` | 28 |
| 45 | Principais Dúvidas sobre Autorregularização | `64de7ebdd29b0c14e1a7136e` | 14 |
| 46 | Principais Dúvidas sobre Bancos Conveniados e PIX | `6501e95d3c373727041947fe` | sem seções |
| 47 | Principais Dúvidas sobre CTe - Conhecimento de Transporte Eletrônico | `64da50f7170f5a26dc5e9ea5` | 46 |
| 48 | Principais Dúvidas sobre Cadastro de Empresa | `64da50f7170f5a26dc5e9e65` | 44 |
| 49 | Principais Dúvidas sobre Compensação, Restituição e Ressarcimento | `64da5104170f5a26dc5f152a` | 12 |
| 50 | Principais Dúvidas sobre Consulta | `64da50f8170f5a26dc5eb6f7` | 45 |
| 51 | Principais Dúvidas sobre Consultas Processuais | `64da5106170f5a26dc5f23a3` | 29 |
| 52 | Principais Dúvidas sobre Consultas de operações e prestações registradas no SITRAM | `64da5106170f5a26dc5f23c7` | 64 |
| 53 | Principais Dúvidas sobre Contribuinte de Outra UF | `64da50ea170f5a26dc5e655a` | 2 |
| 54 | Principais Dúvidas sobre Credenciamento de Empresa | `64da5103170f5a26dc5f091a` | 2 |
| 55 | Principais Dúvidas sobre Desvinculação de débitos | `64da5026170f5a26dc5c717e` | sem seções |
| 56 | Principais Dúvidas sobre Editais | `64da50ec170f5a26dc5e6e4c` | 4 |
| 57 | Principais Dúvidas sobre Envio, Incorporação e Retificação de EFD | `64da5109170f5a26dc5f3282` | 53 |
| 58 | Principais Dúvidas sobre Exclusão/Desenquadramento do SIMEI/Simples Nacional | `64de7861d29b0c14e16135d6` | 20 |
| 59 | Principais Dúvidas sobre Exoneração, Correção ou Impugnação de Débitos | `64da50fc170f5a26dc5ed956` | 1 |
| 60 | Principais Dúvidas sobre Geração/Exclusão de Débitos | `64da5107170f5a26dc5f289f` | 3 |
| 61 | Principais Dúvidas sobre Informação Geral e Legislação | `64da5100170f5a26dc5eefa5` | 91 |
| 62 | Principais Dúvidas sobre Isenção e Dispensa de ICMS | `64da5102170f5a26dc5efc82` | 22 |
| 63 | Principais Dúvidas sobre Isenção, Dispensa, Não-incidência e Anistia de IPVA | `64da50ec170f5a26dc5e6e13` | 9 |
| 64 | Principais Dúvidas sobre Livros e Documentos Fiscais | `64da50ff170f5a26dc5ee817` | 38 |
| 65 | Principais Dúvidas sobre Localização e Liberação de mercadoria | `64da5107170f5a26dc5f2e1c` | 21 |
| 66 | Principais Dúvidas sobre MDFe - Manifesto Eletrônico de Cargas | `64da50f6170f5a26dc5e914e` | 17 |
| 67 | Principais Dúvidas sobre MFe - Módulo Fiscal Eletrônico/CFe - Cupom Fiscal Eletrônico | `64da50f8170f5a26dc5eb79d` | 56 |
| 68 | Principais Dúvidas sobre NF3e - Nota Fiscal de Energia Elétrica Eletrônica | `65034342bbf1401cc0760cea` | 22 |
| 69 | Principais Dúvidas sobre NFAe - Nota Fiscal Avulsa Eletrônica | `64da50f6170f5a26dc5e913d` | 22 |
| 70 | Principais Dúvidas sobre NFCe - Nota Fiscal de Consumidor Eletrônica | `64da50f7170f5a26dc5e9b08` | 26 |
| 71 | Principais Dúvidas sobre NFe - Nota Fiscal Eletrônica | `64da5102170f5a26dc5efa67` | 138 |
| 72 | Principais Dúvidas sobre Obrigatoriedade/Dispensa de Registro | `64da5109170f5a26dc5f34f4` | 195 |
| 73 | Principais Dúvidas sobre Opção ao Regime | `64de7848d29b0c14e15dfb68` | 16 |
| 74 | Principais Dúvidas sobre Pagamento e Parcelamento | `64de7855d29b0c14e161008a` | 17 |
| 75 | Principais Dúvidas sobre Pagamento e Parcelamento de Taxas | `6501c806a8327f2a095fb583` | 24 |
| 76 | Principais Dúvidas sobre Pagamento, Parcelamento e Autorregularização do ICMS | `64da5103170f5a26dc5f08aa` | 58 |
| 77 | Principais Dúvidas sobre Procedimentos de Escrituração | `64da510a170f5a26dc5f377d` | 252 |
| 78 | Principais Dúvidas sobre Regimes Especiais de Tributação e Recolhimento | `64da5107170f5a26dc5f2908` | 20 |
| 79 | Principais Dúvidas sobre Restituição de Taxas | `6501c993a8327f2a09a51c14` | 0 (só cópias) |
| 80 | Principais Dúvidas sobre Restituição e Ressarcimento | `64da50ea170f5a26dc5e655c` | 2 |
| 81 | Principais Dúvidas sobre Substituição Tributária | `64da50ee170f5a26dc5e75b7` | 27 |
| 82 | Principais Dúvidas sobre Tributação e Alíquotas de Produtos | `64da5104170f5a26dc5f1727` | 302 |
| 83 | Principais Dúvidas sobre [FECOP] Fundo Estadual de Combate à Pobreza | `64da50fe170f5a26dc5edf02` | 4 |
| 84 | Principais Dúvidas sobre a Descontinuação do MFe - Módulo Fiscal Eletrônico | `680f5d580b24775f91469fbe` | 13 |
| 85 | Principais Dúvidas sobre a utilização de Sistemas de ICMS | `64da5104170f5a26dc5f1688` | 46 |
| 86 | Principais Dúvidas sobre o Centro de Memória | `64da50fe170f5a26dc5edfa4` | 7 |
| 87 | Principais Dúvidas sobre o Programa Empreendedor Legal ? Moda Ceará | `66b5148c46c6e1738b274a26` | 4 |
| 88 | Principais Dúvidas sobre o Simples Nacional | `64de7ea9d29b0c14e1a6c200` | 47 |
| 89 | Principais dúvidas do Menu: Manifestação do Destinatário | `6a2830ec26fd8047bca34a14` | 2 |
| 90 | Principais dúvidas sobre DIMP - Declaração de Informações de Meios de Pagamentos | `692079c24745cf1198617e25` | 14 |
| 91 | Principais dúvidas sobre SITRAM e Ambiente Seguro | `64da5105170f5a26dc5f20b6` | 32 |
| 92 | Principais dúvidas sobre a emissão de DAE de pagamento de ITCD01 | `68f255822bfc484943cac302` | 12 |
| 93 | Principais dúvidas sobre o Atendimento por agendamento | `680fda2fc5e9af5e784df4cd` | 15 |
| 94 | Principais dúvidas sobre o Menu: Downloads | `6a28349cc78eb856cb55c60e` | 1 |
| 95 | Reforma Tributária – Perguntas e Respostas pelo Ministério da Fazenda | `66faa50d1cbb061624cd362e` | sem seções |
| 96 | Principais Dúvidas sobre Ouvidoria e Corregedoria | `64da50ee170f5a26dc5e7586` | 1 |

---

<a id="rn-faqs"></a>

# RN FAQs — FAQ do programa Nota Potiguar (np.sefaz.rn.gov.br/portal)

**Discovery e implantação:** 06/11/2026. Estudo, medições e decisões D-RNFAQ-1..7:
[`docs/ESTUDO-RN-Faqs.md`](ESTUDO-RN-Faqs.md). Coleção `faqs` do `auli-scraper-rn` (`auli-scraper-rn faqs`, ou `all`
= serviços + FAQs). **23 perguntas** na rodada de 06/11.

## Fonte

O site da SEFAZ-RN (WordPress, `#rn`) não tem FAQ tributária: nenhum tipo de conteúdo de FAQ no `wp-json`, a Loteria
Estadual (29 perguntas) fora do escopo, e as "FAQs" de IPVA e ITCD são PDFs. A FAQ está no **Nota Potiguar**, o
programa de cidadania fiscal: `np.sefaz.rn.gov.br` → `/portal`, AngularJS sobre IIS/ASP.NET, sem `robots.txt` (404).
O HTML das telas vem escrito no próprio bundle — `js/templates.js` (~1 MB, nome fixo), um módulo por tela que chama
`$templateCache.put("<tela>", "…" + "…" + …)`. A da coleta é `components/oaplicativo/views/faq.html` (rota
`#/o-aplicativo/faq`): `<h3>` por pergunta e a resposta nos `<p>`/`<ul>` até o próximo `<h3>`. **1 GET**.

## As armadilhas

1. **Duas telas de FAQ.** "O Aplicativo" (23) e "Comunicação" (`components/comunicacao/views/faq.html`, rota
   `#/comunicacao/faq`, 22): as 22 também estão na primeira, 10 com a mesma resposta e 12 em redação mais curta. A
   coleta lê "O Aplicativo" (tem o desconto no IPVA e as respostas atualizadas) e usa a outra só como conferência:
   pergunta dela que falte na v1 é aviso no log.
2. **O HTML é JavaScript.** O argumento do `put` é uma concatenação de literais de string com escapes (`\"`, `\n`,
   `\uXXXX`); o scraper os lê um a um (literal, `+`, literal… até `)`), sem `eval` e sem regex sobre o JS inteiro.
   Literal sem fim ou um token que não seja `+`/`)` é `Err`.
3. **Sem âncora por pergunta.** O app usa `hashPrefix("")` e a rota é da tela: as 23 têm o mesmo link,
   `https://np.sefaz.rn.gov.br/portal/#/o-aplicativo/faq`, e a pergunta é a identidade do `.md` (repetida é `Err`).
4. **PDF ao lado.** A tela linka `downloads/manuais/faq-np.pdf` ("Perguntas e Respostas — NOVO!", 4,9 MB, de
   20/07/2023); PDF fica fora (D-RNFAQ-3).

## Guardas e transporte

- `Err` antes de gravar qualquer arquivo: o GET que falhe; o `templates.js` sem a tela; literal quebrado; `<h3>`
  vazio; tela sem pergunta; pergunta sem resposta; menos de 15 perguntas (piso); pergunta repetida. Aviso: pergunta só
  na tela de conferência, ou a de conferência ilegível.
- **Sempre à rede**; o cache em `data/rn/raw/cache/faqs/` só é lido com `--usecache`, que reproduz a árvore byte a
  byte e o snapshot a menos do `coletado_em`.
- `ureq` com o UA AuliBot do crate (`crate::UA`, o mesmo dos serviços), 1 GET, segundos.
- Saída: `data/rn/raw/rn-faqs-tree.json` (raiz `Menu` → um `Faq` "Nota Potiguar" com as 23, itens sem `url`) e
  `data/rn/rn-faqs-snapshot.json` (um `FaqRaw` por pergunta, trilha "Nota Potiguar"), nessa ordem e atômicos.
- Atualização: `scripts/tools/update-rn-faqs.sh` (apaga o cache antes; guarda de encolhimento contra o `.anterior`).


---

<a id="es-faqs"></a>

# ES FAQs — portal Receita Orienta (s1-internet.sefaz.es.gov.br/ReceitaOrienta)

**Discovery e implantação:** 07/11/2026. Estudo, medições e decisões D-ESFAQ-1..8:
[`docs/ESTUDO-ES-Faqs.md`](ESTUDO-ES-Faqs.md). Coleção `faqs` do `auli-scraper-es` (`auli-scraper-es faqs`, ou `all` =
serviços + FAQs). **1.003 perguntas** na rodada de 07/11 (ICMS 874, Nota Premiada Capixaba 42, IPVA 35, ITCMD 32,
Reforma Tributária 18, Agendar Atendimento 2), de 46 assuntos em 6 categorias.

## Fonte

O Receita Orienta é um portal à parte (ASP.NET/IIS), não o `portal.es.gov.br` dos serviços; o site institucional
(`sefaz.es.gov.br`) só aponta para ele ("Perguntas Frequentes" → `s1-internet…/faleconosco` → `/ReceitaOrienta`).
Sem `robots.txt` (404). HTML servido pronto, sem token nem JavaScript para o conteúdo. A navegação é a fonte
(D-ESFAQ-1): a barra de menu de qualquer página de categoria lista as 6 categorias; a semente é a categoria ICMS
(`/categoria/61`, D-ESFAQ-7), porque a home tem 17,8 MB de imagens em base64. Cada categoria lista os assuntos
(`href=Assunto/{id}/{Categoria}/{catId}`, relativo); cada assunto é uma página com o acordeão inteiro. 52 GETs
(6 categorias + 46 assuntos, ~3,7 MB) com 1 s entre eles.

## As armadilhas

1. **A busca não é fonte.** A `BuscaV2` devolve um JSON embutido (`var modelo = {…}`) com 1.176 perguntas — 173 a mais
   que a navegação: 155 de 12 assuntos despublicados (a página do assunto responde "Erro: Você não deveria estar
   aqui.") e 18 de 4 subassuntos que o portal não exibe; e traz o `nomeLogin` de quem cadastrou. Não é usada.
2. **Três tipos de cabeçalho no acordeão.** `heading{ID}_faq` = pergunta direto no assunto; `heading{SID}_sa` =
   subassunto; `heading{ID}_saf` = pergunta do subassunto, dentro do `collapse{SID}_sa`. O subassunto da pergunta sai
   do ancestral `collapse{SID}_sa`, não da ordem do documento.
3. **Títulos repetidos na mesma página**, em subassuntos diferentes (10 hoje: "a) Quem pode requerer a isenção?" em
   PcD e em taxistas). O link leva a âncora da pergunta (`#heading{ID}_…`), e o nome do `.md` sai de link + pergunta:
   não colidem.
4. **O anexo fica fora do corpo**, num `.card-footer` com `href='/ReceitaOrienta/Categoria/DownloadAnexo/{id}'`
   (relativo, aspas simples): vira a linha final `Anexo: <url absoluta>` da resposta. 31 anexos (`.doc`/PDF), não
   baixados. Uma resposta depende dele ("Em anexo está o laudo para preenchimento").
5. **O nome da categoria no menu** traz o separador `<p> |</p>` dentro do link: o nome é só o texto direto do `<a>`.
6. **A categoria colapsa** na trilha e na árvore quando tem um assunto só, de mesmo nome (IPVA, ITCMD, Nota Premiada
   Capixaba, Reforma Tributária, Agendar Atendimento — D-ESFAQ-3).

## Guardas e transporte

- `Err` antes de gravar qualquer arquivo: o GET que falhe; menu sem categoria; categoria sem assunto; página
  "Você não deveria estar aqui"; página sem `#accordion` ou sem pergunta; cabeçalho sem título; pergunta de
  subassunto fora de um `collapse…_sa` ou num subassunto sem cabeçalho; pergunta sem `collapse` ou sem
  `.user-content`; resposta vazia; subassunto sem pergunta; id de pergunta repetido; menos de 900 perguntas (piso).
- **Sempre à rede**; o cache em `data/es/raw/cache/faqs/` só é lido com `--usecache`, que reproduz a árvore byte a
  byte e o snapshot a menos do `coletado_em`.
- `ureq`/rustls (TLS 1.2, GCM, cadeia completa com o Sectigo OV R36) com o UA AuliBot (`crate::UA`, D-ESFAQ-8).
- Resposta pelo conversor do kit (`kit::html::html_para_texto`; link → `texto "url"`, bloco e `<hr>` → linha,
  entidades no fim); "Atualizado em" fica (645 respostas).
- Saída: `data/es/raw/es-faqs-tree.json` (raiz → categoria → assunto → subassunto; item com o link da pergunta) e
  `data/es/es-faqs-snapshot.json` (um `FaqRaw` por pergunta), nessa ordem e atômicos.
- Atualização: `scripts/tools/update-es-faqs.sh` (apaga o cache antes; guarda de encolhimento contra o `.anterior`).
