# Scrapers por entidade (`auli-scraper-<id>`)

Referência das 9 implementações de scraper da frota Auli. Cada crate é um **binário por
entidade** que raspa o catálogo de serviços (e, no RS, também as FAQs) de uma SEFAZ estadual e
grava um **snapshot v3** que o `auli-collections` deriva em artefatos e o `auli update` vetoriza.

Fonte da verdade das entidades: [`data/registry.toml`](../../../data/registry.toml). Este doc
descreve o *como* de cada scraper; a lista de entidades vive lá.

> Última atualização: 2026-07-05 (frota com 18 entidades; TO = a mais recente).

---

## 1. Arquitetura comum

Todos os scrapers seguem o mesmo esqueleto:

- **Um crate binário por entidade** (D-F2.1). Cada um conhece só o seu `ENTITY` e **não lê o
  registry** — a entidade é hard-coded no crate.
- **Sem navegador headless.** Nenhum crate depende de `headless_chrome` (o RS já usou, hoje não).
  Doutrina vigente: *discovery-first; API JSON > HTML server-side; navegador nunca*.
- Dependem de **`auli-contract`** (tipos + I/O do snapshot, shape per-público) +
  **`auli-scraper-kit`** (o cardápio comum abaixo) — **nunca** de `fastembed`/`ort`/vector-store.
  Recíproca (D-C1): nada fora de `scrapers/` depende do kit.
- **CLI uniforme:** `auli-scraper-<id> [--usecache] servicos` (RS também aceita `faqs`).
- **Saída:** `data/<id>/<id>-servicos-snapshot.json` (schema v3), gravado por
  `kit::snapshot::write_servicos`. Cache de páginas/respostas em `data/<id>/raw/cache/` (gitignored).
- **Pipeline downstream:** `auli-collections <id>` (deriva `raw/*.json` + `.txt`) →
  `scripts/build-packs.sh <id>` (vetoriza BGE-M3) → `scripts/build-frontend-public.sh <id>`.

### Cardápio do kit (o "como raspar" compartilhado)

Extraído das cópias por-entidade (as ~500 linhas duplicadas de fetch/UA/clean/scraper_info):

| Item | O que faz | Quem NÃO usa (exceção documentada) |
|---|---|---|
| `kit::http::get_string(agent, url, &GetOpts)` | GET com retry 3× + backoff ×2; `GetOpts { log_prefix, accept, headers, attempts, base_delay }` | ba (charset latin1), mg (headers+`Value`), rs-faqs (infra própria) |
| `kit::http::post_json(agent, url, headers, body, &GetOpts)` | POST JSON com retry (ce/mt) | — |
| `kit::clean(s)` | zero-width + nbsp + squeeze | — |
| `kit::clean_decoded(s)` / `kit::decode_entities(s)` | decode de entidades HTML + squeeze (pr/ba) — **sem** strip de zero-width | — |
| `kit::cache::read_or_bail(dir, url, use_cache)` | cache-read + miss-vira-erro do `--usecache` | ce (terminador de paginação), rs-faqs (mensagem própria) |
| `kit::USER_AGENT` | identidade de rede padrão (Firefox/124 Linux) | — (os 3 antigos Chrome eram cópia acidental; recoleta ao vivo confirmou ≡) |
| `ScraperInfo::new(nome, versao)` (contrato) | substitui o `fn scraper_info()` boilerplate | — |

**Regra do UA:** todo scraper usa `kit::USER_AGENT`. Uma divergência local (portal que exija outro
UA) deve vir **com comentário do motivo** — senão é drift. As variantes *line-based* de limpeza
(`clean_text`, que preserva quebras) têm semântica própria por formato e **ficam locais** (ba, mg,
pr, rs).

### Modelo de dados (contrato v3)

Um serviço = um `ServicoRaw { titulo, descricao, link, orgao, ocorrencias[] }`. Cada
`Ocorrencia { publico, classe }` registra onde o serviço aparece no portal. Um serviço listado
sob vários públicos/classes tem **uma ocorrência por listagem** (o schema v2 nativo — não se perde
o caso multi-classe).

### Duas estratégias de identidade

| Estratégia | Como | Quem usa |
|---|---|---|
| **`aggregate_servicos` (kit)** | dedup **por `link`**; monta `Servico` per-público e agrega | sc, pr, mg, pe, ba |
| **`ServicoRaw` direto** | o crate monta os registros; o link **não** é a chave única | rs, sp, rj, ce, ms, mt, go |

O `ServicoRaw` direto existe porque em alguns portais o link não identifica: SP (vários serviços
compartilham a URL de login), RJ (identidade `(link, titulo)`), CE (identidade `_id`; slug não é
único). MS usa o direto por outro motivo: o link **é** único (id numérico embutido), mas as
`ocorrencias` são o produto P(s)×C(s) de taxonomias independentes — o crate monta o fold, não o
`aggregate_servicos`.

---

## 2. Tabela-resumo

| id | Órgão / Estado | Fonte & técnica | Fetch | Públicos | Serviços | Descrição | Identidade | Testes | TLS |
|----|----------------|-----------------|-------|----------|----------|-----------|-----------|--------|-----|
| **rs** | SEFAZ-RS / Rio Grande do Sul | HTTP `ureq`; FAQs (AJAX) + serviços (API JSON `tudofacil`) | JSON+HTML | 5 | 586 | rica (~444) | direto | 5 | rustls |
| **sc** | SEF-SC / Santa Catarina | API JSON Next.js | JSON | 5 | 208 | rica (~736) | agregada | 6 | rustls |
| **sp** | SEFAZ-SP / São Paulo | SharePoint REST `_api` anônimo (2 listas) | JSON | 4 | 537 | curta (~80) | direto | 8 | rustls |
| **pr** | SEFA-PR / Paraná | Drupal server-side; mega-menu "Serviços para você!" | HTML | 7 | 141 | rica (~1185) | agregada | 7 | rustls |
| **mg** | SEF-MG / Minas Gerais | ServiceNow CSM (API JSON) | JSON | 3 | 148 | rica (~3625) | agregada | 3 | rustls |
| **pe** | SEFAZ-PE / Pernambuco | SharePoint 2013 on-prem; menu `#menu_servicos`, 1 GET | HTML | 3 | 38 | **vazia** (menu-only) | agregada | 6 | rustls |
| **ba** | SEFAZ-BA / Bahia | ASP clássico; listagem + fichas de detalhe | HTML | 1 | 204 | rica (~1649) | agregada | 5 | **native-tls** |
| **rj** | SEFAZ-RJ / Rio de Janeiro | WordPress server-rendered; 1 página, 1 GET | HTML | 1 | 91 | **vazia** (v1) | direto | 8 | rustls |
| **ce** | SEFAZ-CE / Ceará | SPA Sydle ONE; API JSON `getChildren` (POST) | JSON | 1 | 382 | curta (~79) | direto | 6 | rustls |
| **ms** | SEFAZ-MS / Mato Grosso do Sul | WordPress server-rendered; listagem própria, filtros `?usuario=`/`?categoria=`, `pp` alto | HTML | 5 | 276 | **vazia** (v1) | direto | 7 | rustls |
| **mt** | SEFAZ-MT / Mato Grosso | X-Via Portal (SPA React); API pública `POST /v1/search/department`, sem token | JSON | 2 | 27 | rica (~168) | direto | 8 | rustls |
| **go** | SEFAZ-GO / Goiás (Secr. Economia) | Portal Expresso (SPA); API WSO2 `servicosOrgaos/20`, token client_credentials anônimo | JSON | 1 | 94 | rica (inline) | direto | 8 | **curl (WAF JA3)** |
| **pi** | SEFAZ-PI / Piauí | SPA Sydle ONE; API JSON `_search` (**GET**), catálogo "Carta de Serviços", Bearer anônimo do shell | JSON | 1 | 29 | curta | direto | 10 | rustls |
| **am** | SEFAZ-AM / Amazonas | Next.js **App Router**; flight **RSC** (header `RSC: 1`), árvore `items`; público via 3 rotas de perfil | JSON (RSC) | 3 | 278 | curta (resumo) | direto | 9 | rustls |
| **pa** | SEFA-PA / Pará | Catálogo estadual "paradigital" (API Prodepa/Spring); `GET /orgao/48` + `GET /servico/{id}`, anônimo | JSON | 3 | 34 | **rica** (etapas+requisitos) | direto | 8 | rustls |
| **es** | SEFAZ-ES / Espírito Santo | portal.es.gov.br (X-Via, molde MT); `POST /v1/search` por `departmentSlug`, anônimo | JSON | 2 | 45 | **rica** (`serviceLetterContent` HTML) | direto | 8 | rustls |
| **ro** | SEFIN-RO / Rondônia | Agência Virtual (Sydle ONE conecta-360, molde PI); `GET _search`, catálogo "Serviços", Bearer anônimo | JSON | 1 | 194 | curta | direto | 8 | rustls |
| **to** | SEFAZ-TO / Tocantins | Carta de Serviços (ASP.NET/IIS, HTML); `listar_servico.aspx?cod_empresa=37` + detalhe por span `lbl*` | HTML | 4 | 45 | **rica** (Carta) | direto | 8 | rustls |

Contagens de serviços = snapshot atual em `main`. Total de testes da frota: **128** (todos os crates cobertos).

---

## 3. Padrões transversais (gotchas)

### TLS — cipher-check antes de assumir
`ureq` usa **rustls** por padrão, que **só suporta ciphers AEAD (GCM/ChaCha20)**. Servidores gov
antigos podem oferecer só **TLS 1.2 CBC** — aí o handshake do rustls é resetado (`Connection
reset by peer`) enquanto curl/OpenSSL conectam. **Só o BA** caiu nisso (IIS antigo, CBC-only) e usa
**native-tls (OpenSSL)**. Diagnóstico de um novo portal:
```
curl -sS URL              # se 200 → não é rede/robots
openssl s_client -connect host:443 -cipher ECDHE-RSA-AES128-GCM-SHA256   # NONE → CBC-only → precisa native-tls
```

### Guards que falham alto (princípio D-RJ5)
Scrapers mais novos (rj, ce) validam contagens mínimas e **falham alto** se a página vier capada,
em vez de gravar um snapshot degradado. O **cache só grava depois dos guards** — uma resposta
capada nunca envenena o cache.

### Cache
`kit::cache` grava 1 arquivo por URL lógica em `data/<id>/raw/cache/`. `--usecache` lê só do cache
(sem rede). Para APIs paginadas (CE), a chave inclui o `pageSize` (respostas de pageSize diferente
não são reaproveitadas).

### Robots / etiqueta
Portais com robots restritivo (PE, BA, CE) usam **User-Agent de navegador** e coleta de baixíssima
frequência (cortesia entre fetches). São catálogos públicos, coleta rara.

---

## 4. Detalhe por entidade

### rs — SEFAZ-RS (Rio Grande do Sul)
- **Único com FAQs** além de serviços (`auli-scraper-rs [--usecache] faqs|servicos`).
- **Serviços:** API JSON do Tudo Fácil (`fazenda.rs.gov.br/_service/tudofacil/capaservicos`) — não
  precisa mais de headless Chrome (era o único que usava).
- **FAQs:** `atendimento.receita.rs.gov.br/perguntas-frequentes`, via AJAX.
- 586 serviços, 5 públicos (Cidadãos/Empresas/Fornecedores/Agentes/Servidores). `ServicoRaw`
  direto. 5 testes.

### sc — SEF-SC (Santa Catarina)
- **API JSON Next.js** (`www.sef.sc.gov.br`) — o portal expõe os dados de build/página em JSON.
- 208 serviços, 5 públicos (Cidadão/Empresa/Servidor Público/Estudante/Prefeitura). Agregada.
- 6 testes (normalize_links, parse_build_id, StringOrNum, parse listagem/detalhe, build_descricao).

### sp — SEFAZ-SP (São Paulo)
- **SharePoint REST `_api` anônimo** (`portal.fazenda.sp.gov.br/servicos/_api/web/lists`) — duas
  listas ('Serviços' e 'Homes 360') em JSON. Sem HTML parse.
- Um serviço pertence a várias **facetas** (Cidadão/Empresa/Servidor/Tributo) → múltiplas
  ocorrências. `ServicoRaw` **direto**: vários serviços compartilham a URL de login, então o link
  não é único.
- 537 serviços, 4 públicos. Descrições **curtas** (blurb do card, ~80 chars). 8 testes
  (clean, canonical, parse verbose/facet, build_corpo, build_servico — ocorrência por faceta).

### pr — SEFA-PR (Paraná)
- **Drupal server-side** (`fazenda.pr.gov.br/Pagina/Carta-de-servicos`), HTML pronto (padrão de
  referência dos scrapers HTML mais novos).
- Mega-menu **"Serviços para você!"** em 7 abas (público) × grupos (classe); um mesmo link aparece
  sob várias abas → agregação por link.
- 141 serviços, 7 públicos (Cidadão/Empresa/Município/Produtor rural/Receita-PR/Programas/Legislação).
  Descrições ricas (~1185). 7 testes (parse_panel do mega-menu, canonical/canonical_any,
  normalize_body_links, html_block_to_text, decode_entities).

### mg — SEF-MG (Minas Gerais)
- **ServiceNow CSM** (`atendimento2.fazenda.mg.gov.br`), API JSON da Service Portal page.
- 148 serviços, 3 públicos (Cidadão/Empresas/Produtor Rural). Agregada. **Descrições mais ricas da
  frota** (~3625 chars). 3 testes.

### pe — SEFAZ-PE (Pernambuco)
- **SharePoint 2013 on-prem**, server-side. Fase 1 raspa **só o menu global `#menu_servicos`**
  (1 GET) — **D-PE1**.
- **Descrições vazias** (menu-only); fase 2 (corpo das páginas `/Servicos/...`) ficou para depois.
- 38 serviços, 3 públicos (Cidadãos/Empresas/Municípios). Agregada (e-Fisco aparece nos 3 públicos
  → 3 ocorrências). Links externos (efisco, gnre, arevirtualws) preservados. 6 testes.
- **D-PE4:** UA de navegador, robots restritivo, 1 GET + cache.

### ba — SEFAZ-BA (Bahia)
- **ASP clássico server-rendered** (`portal.sefaz.ba.gov.br/scripts/cartadeservicos/`). Padrão PR
  completo: **listagem única + fichas de detalhe** (204 serviços; 206 hrefs, 2 comentados).
- **native-tls (OpenSSL)** — o servidor só oferece ciphers TLS 1.2 CBC, incompatíveis com rustls.
- `canonical()` encoda **espaço literal** (`%20`) em slugs `id=` — sem isso, 2 fichas falhavam.
- **D-BA1..4:** público do `panel-title` da ficha (fallback slugificado); classe do `<small>`;
  ficha que falha degrada (Cidadãos/Geral/vazio) sem derrubar a coleta; guarda de charset (UTF-8 →
  latin-1). 1 público (Cidadãos — o portal não tem split), descrições ricas (~1649). 5 testes.

### rj — SEFAZ-RJ (Rio de Janeiro)
- **WordPress server-rendered** (`portal2.fazenda.rj.gov.br/nossos-servicos/`), **UMA página, 1 GET**.
- **Parser agnóstico de CSS do tema:** menu = maior grupo de âncoras internas sob o mesmo contêiner;
  seção = alvo da âncora (3 formatos cobertos por teste).
- **D-RJ2 — identidade `(link, titulo)`:** o link não é único (CISC 2×, DARJ/ITD compartilham URL);
  mesmo par em várias categorias → um serviço com N ocorrências. `ServicoRaw` direto.
- **D-RJ3:** sem descrições (página não tem corpo). **D-RJ4:** público único "Serviços", classe =
  categoria. **D-RJ5:** guards (mín. 12 categorias / 60 ocorrências), cache pós-guards.
- 91 serviços (14 categorias). **8 testes** (o maior da frota).

### ce — SEFAZ-CE (Ceará)
- **SPA pura (Sydle ONE)** — sem HTML server-rendered. A listagem vem da **API JSON `getChildren`
  (POST)** no catálogo `servico-geral` (`portalservicos.sefaz.ce.gov.br/api/1/...`).
- **Auth:** Bearer token **anônimo e público** embutido no shell HTML (`useCookieAuthentication:
  false`); efêmero → o crate o **extrai fresh do shell a cada rodada**.
- **⚠️ Gotcha do `pageSize`:** o servidor entrega MENOS resultados quanto MAIOR o `pageSize`
  (10→382, 100→292, 500→0). Usa **`pageSize=10`** (o do front); paginação termina na **página
  vazia** (não em página curta), e **`hits` não é confiável** (dizia 392 com 382 reais).
- **D-CE2 — identidade `_id`:** o `identifier` (slug) não é único; link canônico
  `…/servico-geral+<identifier>+<_id>`. Descrição **inline** na listagem → sem chamada de detalhe.
- 382 serviços, 1 público ("Serviços", classe "Geral"). Guard mín. 350. 6 testes. `ServicoRaw` direto.
- POC de discovery em `~/Desktop/poc-ce/` (fora do repo).

### ms — SEFAZ-MS (Mato Grosso do Sul)

- **WordPress server-rendered** (catálogo próprio `sefaz.ms.gov.br/servicos/`, tema `new-ms`) — o
  Portal Único `ms.gov.br` (SPA) é só o destino dos links canônicos (id numérico embutido no slug)
  e a fonte futura de descrições (Fase 2). Sem headless, sem API.
- **Grade descoberta DA PÁGINA** (nada hardcoded): filtros `?usuario=<perfil>` (5 públicos) e
  `?categoria=<slug>` (19 classes), coletados por âncora na listagem "Todos". `pp` alto (`load
  more` cumulativo) traz o catálogo inteiro em 1 GET por filtro (~26 GETs no total).
- **D-MS3 — ocorrências = P(s) × C(s):** perfis e categorias são taxonomias **independentes**; um
  serviço tem um conjunto de perfis e um de categorias, e as `ocorrencias` são o produto. Fallback
  "Geral" para órfãos (0 hoje).
- **D-MS5 — invariante sem contador:** o "Mostrando X de N" do portal é **JS-only** (não existe no
  HTML). O guard é cruzado — `união(filtros) ⊆ Todos` (link de filtro fora do "Todos" = capado) +
  cap-detect por `pp` + piso 240. `N` = âncoras distintas do "Todos" (dinâmico).
- **D-MS2 — identidade `link`** (único por construção). **D-MS4 — v1 sem descrição** (a listagem é
  título+link). **D-MS6 — rótulos fiéis** (inclui o slug-typo `comunicacao-e-transparencia`).
- 276 serviços (601 ocorrências), 5 públicos. 7 testes. `ServicoRaw` direto.

### mt — SEFAZ-MT (Mato Grosso)

- **SPA React (X-Via Portal**, o front do X-Road de MT) — sem HTML server-rendered. A listagem por
  órgão vem da API pública **`POST /v1/search/department`** com corpo `{groups:["CATALOG"],
  departmentSlug:"secretaria-de-estado-de-fazenda"}` → **array JSON** de serviços.
- **D-MT3 — anônimo:** sem token, sem Keycloak. O `#error=login_required` no fragment da URL é
  ruído do silent-SSO (`prompt=none`) do shell; o catálogo em si é público (curl reproduz).
- **Listagem rica, sem detalhe:** cada item traz `title`, `description` (inline, ~168 chars médios),
  `category`+`categorySlug` e `targets` — nenhuma chamada de detalhe.
- **D-MT4 — Cenário B:** públicos = `targets` (Cidadão, Empresa); `classe` = `category` (uma por
  serviço); `ocorrencias` = targets × category. Fallback "Geral" para órfãos (0 hoje).
- **D-MT2 — identidade `slug`**; link canônico `…/app/catalog/<categorySlug>/<slug>`.
- **D-MT5 — invariante:** a API dá o próprio total em `resultTotal` → guard duro `únicos ==
  resultTotal` + piso 15. Sem paginação (1 POST traz o catálogo do órgão inteiro).
- 27 serviços (42 ocorrências), 2 públicos. 8 testes. `ServicoRaw` direto. Escopo = só o órgão
  SEFAZ (a Carta PDF ~85 é fonte GPAS divergente — só cross-check, D-MT1).

### go — SEFAZ-GO (Goiás / Secretaria de Estado da Economia)

- **SPA Angular (Portal Expresso)** — sem HTML server-rendered. A listagem por órgão vem da API
  WSO2 **`GET /expresso/2.0.0/servicosOrgaos/20`** (órgão Economia = id 20); `/orgaos`
  (`qtdeServicosPublicados` = invariante) e `/categorias` (id→nome da classe) completam. Descrição
  (`infoServico`) é HTML inline — limpa via html5ever (as entidades `&ccedil;`/`&atilde;`/… ficam
  fora da tabela do `kit::decode_entities`).
- **D-GO3 — auth client_credentials ANÔNIMO:** `POST sso.go.gov.br/oauth2/token` (Basic com as
  credenciais **públicas** do bundle Angular — não são segredo) → Bearer efêmero. Sem login.
- **⚠️ D-GO-WAF — WAF por fingerprint TLS (JA3):** `api.go.gov.br` só aceita o ClientHello do
  curl/browser; o `ureq` (rustls **e** native-tls) recebe "Acesso Negado" (medido no spike: diferem
  nas extensões — falta ALPN, sobra session_ticket; o `TlsConfig` do ureq 3 não expõe ALPN/cipher).
  Por isso os GETs de catálogo usam **`kit::http::get_via_curl`** (subprocess curl — dependência de
  runtime). O **token** sai pelo `ureq` normal (o host de SSO não tem o WAF).
- **D-GO2** id=`go`, name/orgao=`SEFAZ-GO` (a SEFAZ virou Secretaria da Economia; o `go.txt` carrega
  a ponte). **D-GO4** Cenário A (público único "Serviços"); `classe` = categoria. **D-GO5** slug cru
  (braille `⠳` incluído). Identidade = `idServico`.
- 94 serviços (120 ocorrências), 1 público. 8 testes. `ServicoRaw` direto.

### pi — SEFAZ-PI (Piauí)

- **SPA Sydle ONE (molde CE)** — sem HTML server-rendered. A classe de conteúdo `5cd32901…` guarda o
  CMS inteiro (~8421 docs: notícias, legislação, páginas); os serviços do cidadão são o catálogo
  **"Carta de Serviços"** (`parent._id = 69381cec…`). Listagem = **`GET _search`** (ElasticSearch, o
  corpo ES vai url-encoded em `?_body=`) → `{hits:{total,hits[]}}`. Cada item traz `name`,
  `description` (texto plano) e `friendlyUrl`; sem chamada de detalhe.
- **Auth:** Bearer **anônimo** embutido no shell (`useCookieAuthentication:false`), efêmero →
  re-extraído do shell a cada rodada (idêntico ao CE). Sem token, `_search` = 403.
- **⚠️ Gotcha de transporte — o edge Azion reseta TODO POST** do nosso cliente (curl/ureq/Chromium:
  h2 `PROTOCOL_ERROR`, h1.1 `eof`). Mas `_search` é **GET** e GET passa — então o scraper só usa GET
  (ureq h1.1, sem browser-headers). Não precisou do `get_via_curl` (o GET do ureq não é bloqueado).
- **Cenário A** (como CE/RJ): os serviços têm `tags`/`classification`, mas essas classes **não
  autorizam `_search` anônimo (403)** e o `getTags` é POST (bloqueado) — facetas irresolúveis sem
  login. Público único "Serviços", classe "Geral". Identidade = `_id`; link = `…/<friendlyUrl>`
  (rota SPA `/:pathWithId`; sem `friendlyUrl` → `…/<_id>`). Órgão "SEFAZ-PI".
- 29 serviços (Carta de Serviços). 10 testes. `ServicoRaw` direto.

### am — SEFAZ-AM (Amazonas)

- **Next.js App Router (RSC), NÃO Pages Router** — sem `__NEXT_DATA__` nem `/_next/data/{buildId}`
  (buildId irrelevante). A listagem inteira vem **server-rendered no flight RSC**, obtido com o header
  **`RSC: 1`** na própria URL (`text/x-component`). No flight, o componente `$L8` traz `{"items":[…]}`
  = a **árvore pura em JSON** categoria → (subcategoria) → serviço; extraída pela âncora única
  `{"items":[` + balanceamento de colchetes. **Zero XHR:** o conteúdo do detalhe (accordions) é todo
  server-rendered — verificado expandindo no Chrome (não dispara rede). Coleta = `ureq` GET, sem navegador.
- **Público via 3 rotas de perfil** (`/portfolio-servicos/{pessoa-fisica,pessoa-juridica,orgaos-publicos}`):
  `ocorrencias` = {público × classe} por pertencimento; públicos **se sobrepõem** (um serviço pode ser
  PF+PJ). **classe** = categoria de topo da árvore (19). `agendaveis` NÃO é público (a rota devolve tudo)
  — atributo, ignorado como faceta. Identidade = `id`; `link` absolutiza relativos e tira `?profile=`.
  **Escopo: só a listagem** (resumo curto); o conteúdo rico do detalhe ficou de fora por decisão.
- 278 serviços, 423 ocorrências, 3 públicos (PF 147 / PJ 210 / Órgãos 66). 9 testes. `ServicoRaw` direto.
  Links: 239 detalhe / 34 externo / 4 submenu / 1 interno. Detalhes de descoberta em `descoberta-am.md`.

### pa — SEFA-PA (Pará)

- **Catálogo estadual "paradigital"** (SPA Quasar/Vue), API **Prodepa/Spring** em
  `para-digital.sistemas.pa.gov.br/para-digital-service/portal` — tudo **GET anônimo, sem login**.
  Multi-tenant **por órgão**: a SEFA é o **órgão 48**. `GET /orgao/48` → `[{id, nome}]` (listagem magra,
  sem descrição) → obriga o detalhe. `GET /servico/{id}` → payload rico: `finalidade`, `etapaServicos[]`
  (passo a passo), `requisitoServicos[]`, `contatos[]`, `tema` (classe), flags `cidadao/empresa/estado`
  (público), `linkAcesso`. Descrição do snapshot = finalidade + "Como proceder" + "Requisitos" + "Acesso".
- **Público via flags** (sobrepostos → 3 públicos Cidadão/Empresa/Estado; `ocorrencias` = público × classe).
  `classe` = `tema.descricao` (SEFA: tema único "Tributos e empresas"). Identidade = `id`; `link` = a
  página do serviço no paradigital (`…/servico/{id}`). Órgão "SEFA-PA".
- **Primeira entidade com UA institucional `AuliBot`** (não o UA Firefox do kit) + **rate-limit ≥1s**
  entre GETs — mitigações da decisão de desconsiderar robots (D-PA-ROBOTS). O portal candidato
  `portal-digital.sefa.pa.gov.br` estava fora do ar (522) e o Joomla foi extinto — ver `descoberta-pa.md`.
- 34 serviços, 54 ocorrências, 3 públicos (Cidadão 21 / Empresa 30 / Estado 3). 8 testes. `ServicoRaw` direto.
  O paradigital cobre **63 órgãos** com o mesmo contrato → oportunidade de scraper genérico (D-PA-ACERVO).

### es — SEFAZ-ES (Espírito Santo)

- **portal.es.gov.br = SPA React sobre X-Via (MESMO stack do MT).** O `conectacidadao`/`guiadeservicos`
  do enunciado migraram/morreram (307 → portal.es.gov.br). Listagem por órgão = **`POST /v1/search`**
  `{query:"", groups:["CATALOG"], departmentSlug, from, size}` → **array JSON anônimo**. SEFAZ =
  `departmentSlug "secretaria-de-estado-da-fazenda"` (achado via `GET /v1/department`).
- Cada item traz o conteúdo COMPLETO inline (sem chamada de detalhe): `title`, `description` (resumo),
  **`serviceLetterContent`** (a carta, **HTML** → `html_to_text` com html5ever), `category` (classe),
  `targets` (público). `descricao` = resumo + carta. **público via `targets` NORMALIZADOS** (o dado
  publicado traz `cidadao` E `Cidadão` — colapsam num só; Cidadão/Empresa, sobrepostos). `classe` =
  `category` (5). Identidade = `slug`; `link` = `…/servico/{slug}`.
- **Invariante `únicos == resultTotal`** (a API dá o próprio total, lição MT). UA institucional
  **AuliBot** + ≥1s (D-PA-ROBOTS, ES = 2º caso). O X-Via tem 48 órgãos sob a mesma API → D-PA-ACERVO
  ganha 2º caso.
- 45 serviços, 60 ocorrências, 2 públicos (Cidadão 43 / Empresa 17). 8 testes. `ServicoRaw` direto.
  Detalhes de descoberta em `descoberta-es.md`.

### ro — SEFIN-RO (Rondônia)

- **Agência Virtual = SPA Sydle ONE, geração "conecta-360" (MESMO contrato do PI, NÃO do CE).** Shell em
  `agenciavirtual.sefin.ro.gov.br` (Bearer anônimo embutido → re-extrair a cada rodada), API em
  `sydleone.sefin.ro.gov.br` (tenant por **host**, sem header de conta como o CE). Listagem = **`GET
  _search`** (ES, `?_body=` url-encoded) na classe de conteúdo `5cd32901…` (compartilhada com o PI),
  filtrando o catálogo **"Serviços"** (`parent._id 662c1875…`). O CE (geração antiga) usa `getChildren`
  → dá 400 no RO; a prova está em `descoberta-ro.md`.
- **Cenário A** (como CE/PI): `tags` null e `classification` 403 anon → público único "Serviços", classe
  "Geral". Identidade = `_id`; `link` = `…/catalogo-servicos+{identifier}+{_id}`. **Escopo = só "Serviços"**
  (194); "Temas" (42) e "Conteúdos" (28) são informativos, fora. Invariante `únicos == total ES`.
- UA institucional **AuliBot** (D-PA-ROBOTS preventivo). Há `contentHtml` inline p/ uma v2 rica (como o AM).
- 194 serviços, 1 público. 8 testes. `ServicoRaw` direto. **RO + PI = mesma geração Sydle → oportunidade
  de scraper parametrizável** (não o CE); ver D-XX em `auli_pendencias.md` §16.

### to — SEFAZ-TO (Tocantins)

- **Carta de Serviços em `servicos.to.gov.br` — ASP.NET WebForms / IIS (HTML server-rendered)**, molde
  HTML-scraping (como BA/RJ), NÃO SPA/JSON. SEFAZ = órgão **`cod_empresa=37`**. **Listagem (1 GET):**
  `listar_servico.aspx?cod_empresa=37` → 45 serviços (identidade = `cod_assunto_documento_tipo`).
  **Detalhe (1 GET/serviço):** `servico_detalhado.aspx?cod=…` — conteúdo rico (padrão gov.br Carta) em
  spans com id ASP.NET estável (`ctl00_…_lbl*`), parseados por id via `scraper` (html5ever decodifica
  as entidades). Robusto contra os accordions aninhados.
- **Cenário B:** `descricao` = Conceituação + Como solicitar + Documentos + Custos + Prazo (seções
  não-vazias, ~1,1 KB mediana). **público** = `lblTipoRelacionamento` (vocabulário fixo concatenado —
  Cidadão/Empresa/Órgão Público/Servidor; parse longest-first p/ não quebrar "Órgão Público"). **classe**
  = `lblTxtServicoGrupo`. `link` = a própria página de detalhe. UA institucional AuliBot + cortesia 500ms
  (D-PA-ROBOTS, 3º caso).
- 45 serviços, 79 ocorrências, 4 públicos (Cidadão 35 / Empresa 38 / Órgão Público 5 / Servidor 1),
  2 classes. 8 testes. `ServicoRaw` direto. Descoberta em `descoberta-to.md`. Portal multi-órgão →
  3ª ocorrência de D-PA-ACERVO (mas em ASP.NET/HTML).

---

## 5. Checklist de integração de uma nova entidade

1. Crate em `auli-server/crates/scrapers/auli-scraper-<id>/`. O `members` do
   [`auli-server/Cargo.toml`](../../Cargo.toml) usa o glob `crates/scrapers/*` — **não precisa
   editá-lo** para uma entidade nova.
2. `cargo test -p auli-scraper-<id>` — o gate verde de verdade.
3. **Registrar em [`data/registry.toml`](../../../data/registry.toml)** (bloco `[[entities]]`) +
   criar `data/prompts/<id>.txt`. ⚠️ Passo fácil de esquecer — sem ele, `auli-collections <id>`
   falha com "Entidade desconhecida".
4. `node scripts/gen-frontend-entities.mjs` → regenera `auli-frontend/src/shared/entities.ts`
   (validar com `scripts/check-registry-sync.sh`).
5. `cargo run -p auli-scraper-<id> -- servicos` → grava o snapshot.
6. `cargo run -p auli-collections -- <id>` → deriva `data/<id>/raw/*`.
7. `cargo build --release` → `scripts/build-packs.sh <id>` (BGE-M3) → `scripts/build-frontend-public.sh <id>`.
8. Smoke-test: subir `auli server` e bater em `POST /v1/question` com `{"entity":"<id>"}`.

**Gitignored** (derivados, não commitar): `data/<id>/raw/`, `data/<id>/packs/`. **Versionado:**
o snapshot + `auli-frontend/public/<id>/`.

Antes de confiar num scraper de API JSON, **confira a contagem raspada contra uma contagem manual
no navegador** (a lição do CE: um `pageSize` errado escondeu 24% do catálogo).

---

## 6. Dívidas conhecidas

- **Cobertura de testes desigual:** todos os crates têm testes (3–8) desde a rodada sc/sp/pr, mas
  o **mg tem só 3** e nenhum exercita a paginação/loop de coleta ponta a ponta (só funções puras).
- **pe — descrições vazias** (menu-only, D-PE1). Fase 2 (corpo das páginas) melhoraria o RAG.
- **rj — descrições vazias** (v1, D-RJ3). Página não tem corpo; exigiria outra fonte.
