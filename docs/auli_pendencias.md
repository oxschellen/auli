# Pendências — auli-contract e integração de dados

Registro dos assuntos em aberto **após** o merge do `auli-contract` (PR #3) no `main`. O refactor
fez a **struct tipada** (`auli_contract::Table<P>`) virar a fonte única do dado: o scraper grava o
contrato em `data/<id>/raw/<id>-<kind>.json` e o `auli update` o consome.

> **Revisão 2026-07-02 (pós fases 1 e 2).** O modelo evoluiu: o **snapshot v2**
> (um por coleção: `data/<id>/<id>-<kind>-snapshot.json`, tipos em `auli_contract::snapshot`) virou a fronteira
> scraper→collections; a coleta saiu para os binários **`auli-scraper-<e>`** e o
> **`auli-collections <e>`** só deriva os artefatos (contrato/prints/index/per-público).
>
> **Revisão 2026-07-03 (multi-estado + deploy).** Quase tudo abaixo foi **resolvido** nesta sessão.
> As **4 entidades estão no ar e raspadas ao vivo**: `rs` (serviços 586 + FAQs 1937), `sc` (208),
> `sp` (537), `pr` (141) — todas com packs `strategy_version: 2`, kind `servicos`. O server sobe
> carregando as 4, valida cada manifesto contra a identidade local, `/v1/health → OK`,
> `api.auli.com.br` roteia, e o RAG responde por estado citando os links certos (verificado ponta a
> ponta). Status item a item abaixo.
>
> **Revisão 2026-07-04 (scraper RS sem navegador + mg).** Duas correções ao quadro acima:
>
> 1. **Serviços do RS agora sem headless Chrome** (PR #10). Os cards das 5 listagens eram tidos como
>    "JS-rendered" e vinham do Chrome; na verdade são montados no cliente por `capaservicos.js` a
>    partir de um endpoint JSON interno do CMS (`/_service/tudofacil/capaservicos?parent=<ids>&page=<n>`,
>    com `parent` no atributo `data-servico-parent` do shell server-rendered). Não há barreira de JS
>    de fato, então o `ureq` basta: `extrair_servicos_da_api` reproduz **byte a byte** o snapshot e os
>    artefatos `raw/` do scrape com Chrome. `headless_chrome` foi removido (−18 crates) e **nenhum**
>    estado usa mais navegador.
>    `D-RS-OBSCURA: gate de equivalência (Obscura) aprovado em 2026-07-04, mas a adoção foi o caminho`
>    `ureq/API JSON — mais leve e robusto; headless_chrome removido.`
> 2. **`mg` entrou**: são **5** entidades no ar (rs, sc, sp, pr, mg), não 4. E os snapshots são **v3**
>    por coleção (`<id>-<kind>-snapshot.json`, full-overwrite, sem merge), não v2.

---

## 1. Verificação ponta a ponta — ✅ **feita (2026-07-03)**

O scrape ao vivo rodou para os 4 estados → snapshot v2 → `auli-collections <e>` → `build-packs.sh <e>`
→ boot. O server carregou `[pr, rs, sc, sp]`, cada manifesto validado, e o **smoke test de RAG**
respondeu por estado com o serviço/link corretos:

- **rs** — certidão negativa → Certidão de Situação Fiscal (`sefaz.rs.gov.br/sat/…`)
- **sc** — regularidade fiscal → Emitir CND (`sat.sef.sc.gov.br/…`)
- **sp** — cadastro ICMS → CADESP (`portal.fazenda.sp.gov.br/servicos/cadesp/…`)
- **pr** — serviços ao cidadão → FAQ/agendamento (`fazenda.pr.gov.br/…`)

> **Resíduo resolvido:** os packs antigos `rs-pareceres`/`rs-notas` (strategy=1) **foram removidos**
> na reconstrução — `data/rs/packs/` tem só `rs-faqs.json`, `rs-servicos.json` e o manifesto.

---

## 2. Frontend desacoplado do contrato (árvore de FAQ) — ✅ **resolvido (opção a)**

A aba de FAQs voltou a ter fonte: o scraper agora **serializa a árvore** (`faqs-tree.json`, com
`page_type`/`children`) ao lado do snapshot
([faqs/mod.rs](auli-server/crates/scrapers/auli-scraper-rs/src/faqs/mod.rs)), e o frontend a busca
([FaqsList.tsx:17](auli-frontend/src/pages/faqslist/FaqsList.tsx#L17), `faqs-tree.json`). O
`build-frontend-public.sh` agora **pula os contratos do engine** (`<id>-faqs.json`/`<id>-servicos.json`,
que a UI não consome) — sem peso morto nem colisão de nome em `public/`.

---

## 3. Fórmula de `text_to_embed` de serviços — **provisória (validada ponta a ponta)**

Continua `tipo | classe` + título + 300 chars do corpo
([servicos/mod.rs](auli-server/crates/auli-collections/src/servicos/mod.rs), `servico_text_to_embed`).
Ainda **rotulada provisória no código**, mas o smoke test do item 1 mostrou recuperação boa nos 4
estados (respostas citam o serviço principal certo). Falta só a decisão de **fixar** a fórmula (ou
ajustá-la) e tirar o rótulo. FAQs seguem em `origin + pergunta` (`faq_from_raw`), estável.

---

## 4. `pareceres` / `notas` / `conteudos` — **adiados (sem fonte struct)**

Inalterado. São conteúdos **autorados** (em `data/<id>/ref/`), sem scraper — não há `Table<P>`. O
`auli update` os pula; o server sobe com a coleção vazia. Para reentrarem nos packs: modelar cada um
como struct no `auli-contract` (+ `text_to_embed`/`stored_repr`) e ter um produtor que preencha o
contrato.

---

## 5. Vocabulário de kinds (`servicos` ↔ `services`) — ✅ **resolvido (auditoria PR #4)**

A auditoria de consistência **unificou o kind vetorial `services` → `servicos` ponta a ponta**
(`corpus`/`update`/`rag`/`packs`/`manifest`). Não há mais tradução por convenção: o `registry.toml`,
o scraper, a UI e o engine falam um vocabulário só. Os packs saem `<id>-servicos.json`.

---

## 6. Conteúdo dos docs de scrapers desatualizado — ✅ **resolvido**

`docs/auli_code.md` (§3.1 layout, §5 scrapers/cobertura, §7 resumo) e o runbook `docs/auli_operations.md`
foram atualizados para as **9 entidades**: RS descrito como **API JSON `tudofacil`** (não mais
headless), tabela/cobertura completas (pe/ba/rj/ce), contagens atuais, o rótulo `snapshot v2 → v3`,
e a nota do `auli-docs/` obsoleto removida. Referência viva:
[`crates/scrapers/SCRAPERS.md`](auli-server/crates/scrapers/SCRAPERS.md).

---

## 7. Fronteira do snapshot absorvida pelo contrato (D-C1/2/3) — ✅ **resolvido**

O **I/O do snapshot** (`load`/`write_faqs`/`write_servicos`/`snapshot_path`) e o shape per-público
`ServicoPerPublico` saíram do `auli-scraper-kit` para o `auli-contract`:

- **D-C1:** a fronteira (tipos + versão + caminho + leitura/escrita) mora só no contrato; o kit
  fica com o "como raspar" (cache, agente `ureq`, agregação). Recíproca: **nada fora de
  `crates/scrapers/` depende do kit**.
- **D-C2:** `ServicoPerPublico` (o JSON per-público que o frontend consome) é contrato, não infra de
  scraper — os scrapers e o `process` usam o alias `auli_contract::ServicoPerPublico as Servico`.
- **D-C3:** o `auli-collections` **larga a dep do kit** e usa `auli_contract::snapshot::load` — seu
  grafo não puxa mais `ureq` (o alvo pesado; `time` fica, via contrato, mas é leve e só do write).

Verificado: build/test/clippy verdes; invariante geográfico vazio; golden do `auli-collections`
sem diff.

---

## 8. Entidade `ms` (SEFAZ-MS) integrada — ✅ **resolvida (10ª entidade)**

Scraper `auli-scraper-ms` no molde HTML (RJ/PR): catálogo próprio `sefaz.ms.gov.br/servicos/`
(WordPress server-rendered), grade **5 perfis × 19 categorias** descoberta da própria página,
`ServicoRaw` direto com `ocorrencias` = P(s)×C(s) (D-MS3). **276 serviços, 601 ocorrências**, 0
órfãos; snapshot v3 → `auli-collections ms` → packs BGE-M3 (276) → boot com manifesto validado →
RAG responde citando o link canônico `ms.gov.br`.

- **D-MS5 — invariante sem contador:** o "Mostrando X de N" do portal é **JS-only** (não existe no
  HTML server-rendered; o "277" citado na descoberta era ruído de coordenada SVG). O guard foi
  re-ancorado em sinais que existem e se cruzam: `união(filtros) ⊆ Todos` (link de filtro fora do
  "Todos" ⇒ listagem capada) + cap-detect por `pp` + piso estático 240. `N` = âncoras distintas do
  "Todos" (dinâmico, 276 hoje).
- Demais decisões: D-MS1 (fonte = catálogo próprio; Portal Único = link canônico + fonte da Fase 2),
  D-MS2 (identidade = link), D-MS4 (v1 sem descrição), D-MS6 (rótulos fiéis, inclui typo
  `comunicacao-e-transparencia`). Detalhe em [`crates/scrapers/SCRAPERS.md`](auli-server/crates/scrapers/SCRAPERS.md).
- **Fase 2 (futura):** descrições dos 276 via API de detalhe do Portal Único (SPA) — os ids
  numéricos já vêm nos links.

---

## 9. Entidade `mt` (SEFAZ-MT) integrada — ✅ **resolvida (11ª entidade)**

Scraper `auli-scraper-mt` no molde CE (JSON direto): o "Catálogo de Serviços do X-Via Portal" (SPA
React de MT) expõe a listagem por órgão na **API pública `POST /v1/search/department`** — **anônima,
sem token, sem Keycloak** (o `#error=login_required` do shell é ruído do silent-SSO, não do
catálogo). **27 serviços, 42 ocorrências**, 0 órfãos; descrição rica inline (sem chamada de detalhe).
Pipeline: snapshot v3 → `auli-collections mt` (2 tabs) → packs BGE-M3 (27) → boot com manifesto
validado → RAG responde citando o link canônico `portal.mt.gov.br/app/catalog/…`.

- **D-MT1** fonte = catálogo X-Via filtrado pelo órgão SEFAZ (escopo só SEFAZ); a "Carta de Serviços"
  (PDF Liferay ~85, base GPAS) é fonte divergente → só cross-check.
- **D-MT2** identidade = `slug`; link canônico `…/app/catalog/<categorySlug>/<slug>`.
- **D-MT3** anonimato estrito satisfeito (API pública, sem login).
- **D-MT4 (Cenário B)** públicos = `targets` (Cidadão, Empresa); `classe` = `category`; `ocorrencias`
  = targets × category.
- **D-MT5** invariante dinâmico `únicos == resultTotal` (a API dá o próprio total) + piso 15; sem
  paginação (1 POST traz o catálogo do órgão). Detalhe em
  [`crates/scrapers/SCRAPERS.md`](auli-server/crates/scrapers/SCRAPERS.md).

---

## 10. Extração das funções comuns da frota para o kit — ✅ **resolvida (Camada 1)**

As cópias por-entidade de `fetch`/retry, `USER_AGENT`, `clean` e `scraper_info` (~500 linhas
duplicadas nas 11 entidades) foram extraídas para o `auli-scraper-kit`: `http::{get_string,
post_json}` (retry + backoff, `GetOpts` com `headers`), `USER_AGENT`, `clean`/`clean_decoded`/
`decode_entities`, `cache::read_or_bail`; e `ScraperInfo::new` no contrato. **Equivalência provada por
entidade** (recompute `--usecache` ≡ snapshot commitado, byte a byte) e os 3 UAs Chrome (mg/sc/rs)
migrados para Firefox com **recoleta ao vivo verificada** (eram cópia acidental, nenhum portal exige
Chrome). Cardápio + regra do UA em [`SCRAPERS.md`](auli-server/crates/scrapers/SCRAPERS.md).

Exceções que ficam locais (com comentário): `fetch` do ba (charset latin1 + native-tls), do mg (page
API ServiceNow: headers + `Value` + parse-antes-de-cachear) e do rs-faqs (cache path-based próprio +
retry genérico); o `clean_text` line-based (ba/mg/pr/rs); e o cache-terminador de paginação do ce.

**Pendências abertas (fora desta TAREFA):**

- **Camada 2 (2º-consumidor já satisfeito, extrair em TAREFA curta):** `kit::validar_contagem(unicos,
  total, min, dica)` — o invariante dinâmico de ce (`hits`) e mt (`resultTotal`); `kit::absolutize(base,
  href)` — o miolo comum de `canonical` de rj/sp/pe; `kit::cli::parse_args()` — o parse de
  `[--usecache] <cmd>` repetido nos `main.rs`. (Nota: `ms` foi re-ancorado em `união⊆Todos`, então o
  `validar_contagem` tem 2 consumidores reais — ce, mt —, não 3.)
- **`AuliBot/x.y (+url)` institucional:** avaliar um User-Agent identificável para a frota toda (hoje
  todos usam o Firefox/124 de navegador). Decisão de projeto — muda a identidade de rede em todos os
  portais, exige recoleta verificada.

---

## 11. Entidade `go` (SEFAZ-GO / Economia) integrada — ✅ **resolvida (12ª entidade)**

Scraper `auli-scraper-go` (molde CE/MT): API do **Portal Expresso** (WSO2), `servicosOrgaos/20` do
órgão Economia. **94 serviços, 120 ocorrências**, 3 sem categoria (Geral); descrição HTML rica
(html5ever). Pipeline: snapshot v3 → `auli-collections go` → packs BGE-M3 (94) → boot com manifesto
validado → RAG responde citando `go.gov.br/servicos/servico/…` e usando "Secretaria de Estado da
Economia" (a ponte do prompt).

- **D-GO1** fonte = Expresso `servicosOrgaos/20`; CKAN descartado (não tem o catálogo).
- **D-GO2** id=`go`, name/orgao=`SEFAZ-GO` (a SEFAZ virou Secretaria da Economia; o `go.txt` faz a
  ponte institucional). **D-GO3** client_credentials anônimo (credenciais públicas do bundle).
  **D-GO4** Cenário A (público único; classe=categoria via `/categorias`). **D-GO5** slug cru (braille).

**D-GO-WAF (o achado do spike JA3):** `api.go.gov.br` faz allowlist por fingerprint TLS; o `ureq`
(rustls E native-tls) leva "Acesso Negado", curl passa — com token/headers/HTTP idênticos. Medido:

| cliente | JA3 hash | passa? |
|---|---|---|
| curl (baseline) | `32e4b881…a769` | ✅ |
| ureq rustls | `7822ed71…a179` | ❌ |
| ureq native-tls | `284deaea…5422` | ❌ |

Campo divergente = **extensões** (curl tem ALPN 16 + ext 49; ureq tem session_ticket 35; ciphers do
native-tls idênticos ao curl). O `TlsConfig` do ureq 3.3.0 não expõe ALPN/cipher-list/connector →
§4 do spike morta. Decisão: **`kit::http::get_via_curl`** (subprocess curl contido no kit; só os GETs
de catálogo — o token sai por `ureq`, o SSO não tem WAF). Diagnóstico completo em `go_waf.md`.
**Dependência de runtime: `curl` no PATH** (registrado no runbook — desktop E túnel).

## 12. Entidade `pi` (SEFAZ-PI) integrada — ✅ **resolvida (13ª entidade)**

Portal `portal.sefaz.pi.gov.br` = **SPA Sydle ONE (molde CE)**, edge Azion.

- **D-PI1 — fonte:** a classe de conteúdo `5cd32901…` guarda o CMS inteiro (~8421 docs: notícias,
  legislação, páginas institucionais). O catálogo **cidadão** é **"Carta de Serviços"**
  (`parent._id = 69381ceceecdd6684a84c49c`) → **29 serviços ativos**. Listagem via **`GET _search`**
  (ElasticSearch; corpo ES url-encoded em `?_body=`). Escopo confirmado com o usuário: **só a Carta de
  Serviços** (os catálogos "Serviços de Pessoal", "Tesouro/servidor", etc. NÃO entram — não são a
  carta-cidadã).
- **D-PI2 — auth:** Bearer **anônimo** embutido no shell (`useCookieAuthentication:false`), efêmero →
  re-extraído do shell a cada rodada (idêntico ao CE). Sem token, `_search` = 403.
- **⚠️ D-PI-POST — o edge Azion reseta TODO POST** do nosso cliente (curl/ureq/Chromium: h2
  `PROTOCOL_ERROR`, h1.1 `eof`); **GET passa**. Como `_search` é GET, o scraper **só usa GET** (ureq
  h1.1, sem browser-headers) — não precisou do `get_via_curl` (diferente do GO: aqui o GET do ureq não
  é bloqueado por JA3). O ground-truth do XHR foi capturado com Chrome real + `--disable-http2`
  (Playwright), pois o Chromium bundled também falha no h2 desse edge.
- **D-PI3 — Cenário A:** os serviços têm `tags`/`classification`, mas essas classes **não autorizam
  `_search` anônimo (403)** e o `getTags` é POST (bloqueado) — facetas irresolúveis sem login. Público
  único "Serviços", classe "Geral". Identidade = `_id`; link = `…/<friendlyUrl>` (rota `/:pathWithId`;
  sem `friendlyUrl` → `…/<_id>`). Órgão "SEFAZ-PI".
- Cache com chave lógica CURTA (`SEARCH_URL#<catálogo>`): a URL real carrega o `_body` gigante
  url-encoded, longo demais para virar nome de arquivo. 10 testes. `ServicoRaw` direto.

## 13. Entidade `am` (SEFAZ-AM) integrada — ✅ **resolvida (14ª entidade)**

Portal `www.sefaz.am.gov.br/portfolio-servicos` = **Next.js App Router (RSC)**. Descoberta completa em
`docs/REGISTRO-scrapers.md#am`.

- **D-AM1 — transporte (App Router, não Pages Router):** sem `__NEXT_DATA__` nem `/_next/data/{buildId}`
  (buildId irrelevante). A listagem inteira vem no **flight RSC** via header **`RSC: 1`** na URL
  (`text/x-component`). Componente `$L8` → `{"items":[…]}` = árvore JSON categoria→(subcat)→serviço,
  extraída pela âncora única + balanceamento. `ureq` GET basta (Apache/HTTP1.1, sem WAF).
- **D-AM2 — zero XHR (ponto crítico da descoberta):** o conteúdo do detalhe (accordions: passo a passo,
  documentação, legislação, FAQ, contato) é **todo server-rendered** no RSC do detalhe (chunks
  `$a/$b/$c` no mesmo payload); expandir os accordions no browser NÃO dispara rede. → o scraper nunca
  precisa de navegador (headless foi usado só para _provar_ isso na descoberta).
- **D-AM3 — escopo = só a listagem:** por decisão, coletamos só o `resumo` curto da listagem (não os
  278 detalhes). O conteúdo rico do detalhe fica para uma eventual **v2** — ver **D-AM-V2** abaixo.
- **D-AM4 — público:** 3 rotas de perfil (`pessoa-fisica/juridica/orgaos-publicos`) com **sobreposição**
  (pf∩pj=98); `ocorrencias` = {público × classe} por pertencimento. **classe** = categoria de topo (19).
  `agendaveis` NÃO é público (a rota devolve os 278) — atributo, ignorado como faceta. **Duplicatas**
  publicadas (nome igual, `id` distinto: 5 pares) **mantidas** (fidelidade). Identidade = `id`.
- **D-AM5 — anomalia do portal:** o id **1436** aparece em uma rota de perfil mas não em `/todos`;
  a fonte de verdade é `/todos` (278) — o 1436 fica fora. Registrado.
- 278 serviços, 423 ocorrências, 3 públicos (PF 147 / PJ 210 / Órgãos 66). 9 testes. `ServicoRaw` direto.

### D-AM-V2 — descrição rica do detalhe (pendência aberta, NÃO feita)

A v1 usa só o `description` curto da listagem (≈1 frase). O AM publica, por serviço, uma **descrição
rica** (o diferencial do portal) que hoje NÃO entra no snapshot. Recuperá-la é a **v2 do `am`** — sem
navegador (a descoberta provou zero-XHR: todo o conteúdo é server-rendered no RSC do detalhe).

**Fonte:** `GET https://www.sefaz.am.gov.br/portfolio-servicos/detalhes/{id}?profile=todos` com header
`RSC: 1` (mesmo transporte da v1). No flight, o objeto **`serviceDetails`** (extrair por âncora
`"serviceDetails":{` + balanceamento) traz o mapa de seções:

| campo em `serviceDetails` | seção | forma |
|---|---|---|
| `resumo` | O que é | texto plano (inline) |
| `perfis[]` | A quem se destina | array de strings (= o público, já temos via rotas) |
| `comoProcederHtml` | Passo a passo / Como proceder | **ref de chunk `$a`** |
| `documentacaoHtml` | Documentação necessária | **ref de chunk `$b`** |
| `legislacaoHtml` | Legislação Aplicada | HTML inline |
| `perguntasRespostasHtml` | Perguntas Frequentes | **ref de chunk `$c`** |
| `setorResponsavel{nome,sigla}` + `email` + `phone` + `tempoMedioEmDias` | Contato | campos escalares |
| `visibleSections[]` | quais seções existem | enum (ex.: id 63 não tem `PERGUNTAS_FREQUENTES`) |

**Trabalho técnico da v2 (o que a v1 evitou):**

1. **Resolver refs de chunk do flight:** `"$a"/"$b"/"$c"` apontam para chunks-texto no MESMO payload,
   no formato `a:T<hexlen>,<html>` — **alguns colados ao chunk anterior sem `\n`** (o parser tem de
   varrer por `<ref>:T<hexlen>,` e ler `hexlen` bytes, não confiar em quebra de linha).
2. **Decodificar entidades HTML** (`&ccedil;`, `&atilde;`, `&ordm;`…) e limpar as tags — a tabela fixa
   do `kit::decode_entities` NÃO cobre tudo; usar **html5ever (crate `scraper`)**, mesma lição do GO.
3. Montar a `descricao` concatenando as seções presentes (`visibleSections`) num texto uniforme.

**Custo/risco:** **~278 GETs** (um por serviço, cacheáveis; cortesia entre eles) — mais pesado que
qualquer scraper atual. Os **39 serviços link-only** (externo/submenu) NÃO têm página de detalhe → sem
`serviceDetails`; a v2 mantém para eles só o `resumo` da listagem. Guard sugerido: um piso de seções
não-vazias por serviço para pegar regressão de parser. Evidência e amostras em `docs/REGISTRO-scrapers.md#am` (Fase 3).

## 14. Entidade `pa` (SEFA-PA) integrada — ✅ **resolvida (15ª entidade)**

Fonte = catálogo estadual **paradigital** (API Prodepa/Spring), escolhida na descoberta (`docs/REGISTRO-scrapers.md#pa`);
o candidato `portal-digital` estava fora do ar (522) e o Joomla foi extinto.

- **D-PA-FONTE — API anônima:** `para-digital.sistemas.pa.gov.br/para-digital-service/portal`, tudo GET
  sem login. Multi-tenant por órgão: SEFA = **órgão 48**. `GET /orgao/48` → `[{id, nome}]` (listagem
  magra) → obriga o detalhe `GET /servico/{id}` (rico: finalidade, `etapaServicos`, `requisitoServicos`,
  contatos, tema, flags de público, `linkAcesso`). 34 serviços.
- **D-PA-ROBOTS — robots desconsiderado (decisão do mantenedor)** por ser conteúdo público (LAI), baixo
  volume. **Mitigações aplicadas no scraper:** UA institucional **`AuliBot/0.1 (+repo; email)`** (1ª
  entidade da frota a usá-lo, não o UA Firefox do kit), **cortesia ≥1s** entre GETs (sem paralelismo),
  cache agressivo (cada URL 1×; `--usecache` miss=erro), **nunca autenticar**. Na prática o paradigital
  libera por robots — a mitigação é preventiva.
- **D-PA-MODELO:** `ServicoRaw` direto; identidade = `id`; `descricao` = finalidade + "Como proceder"
  (etapas) + "Requisitos" + "Acesso" (linkAcesso); `classe` = `tema.descricao` (SEFA: tema único
  "Tributos e empresas"); `publico` via flags `cidadao/empresa/estado` (sobrepostos → 3 públicos,
  `ocorrencias` = público × classe); `link` = `paradigital.pa.gov.br/servico/{id}`; órgão "SEFA-PA".
  54 ocorrências (Cidadão 21 / Empresa 30 / Estado 3). 8 testes.
- **⏳ D-PA-ACERVO (aberta, NÃO feita):** o paradigital é um catálogo estadual **multi-tenant com 63
  órgãos** sob o MESMO contrato (`/orgao/{idOrgao}` + `/servico/{id}`). Um **scraper estadual genérico**
  parametrizado por `idOrgao` cobriria os 63 órgãos sem novo código — oportunidade forte para o Acervo.
  Registrado; não implementado (foge do modelo "1 entidade = 1 órgão" atual).
- **⏳ D-PA-PORTALDIGITAL (aberta):** `portal-digital.sefa.pa.gov.br` (SPA SEFA-específica, ids Mongo
  ObjectId) estava em 522 na coleta. Se voltar, reavaliar (pode ter conteúdo próprio).

## 15. Entidade `es` (SEFAZ-ES) integrada — ✅ **resolvida (16ª entidade)**

Fonte = `portal.es.gov.br` (SPA React sobre **X-Via**, MESMO stack do MT). O `conectacidadao`/
`guiadeservicos` do enunciado migraram/morreram (307 → portal.es.gov.br). Descoberta em `docs/REGISTRO-scrapers.md#es`.

- **D-ES-FONTE / D-ES-MOLDE-MT:** listagem por órgão = **`POST /v1/search`**
  `{query:"", groups:["CATALOG"], departmentSlug, from, size}`, **anônima**. SEFAZ =
  `departmentSlug "secretaria-de-estado-da-fazenda"` (via `GET /v1/department`). Molde MT: array JSON,
  conteúdo rico inline, invariante `únicos == resultTotal`, sem paginação (um `size` alto basta).
- **D-ES-MODELO:** identidade = `slug`; `descricao` = `description` (resumo) + `serviceLetterContent`
  (a carta, **HTML** → `html_to_text` com html5ever); `classe` = `category` (5); `link` =
  `portal.es.gov.br/servico/{slug}`; órgão "SEFAZ-ES".
- **D-ES-PUBLICO:** público = `targets` **normalizados** — o dado publicado traz `cidadao` **E**
  `Cidadão` (mesma pessoa, grafias diferentes) → colapsam num só "Cidadão" (senão duplicaria a
  ocorrência). Resultado: Cidadão 43 / Empresa 17 (sobrepostos). `agendável` = atributo, não público
  (alinhado ao AM).
- **D-ES-ROBOTS:** coberto por **D-PA-ROBOTS** (ES = 2º caso) — UA institucional AuliBot + ≥1s + cache;
  a API não bloqueia. Nenhuma autenticação (Acesso Cidadão intocado).
- 45 serviços, 60 ocorrências, 2 públicos. 8 testes. `ServicoRaw` direto. O X-Via do ES tem **48
  órgãos** sob a mesma API → **D-PA-ACERVO** ganha 2º caso (scraper estadual genérico serviria PA/Prodepa
  **e** ES/X-Via).

## 16. Entidade `ro` (SEFIN-RO) integrada — ✅ **resolvida (17ª entidade)**

Agência Virtual = SPA **Sydle ONE geração "conecta-360" (= molde PI, NÃO o CE)**. Descoberta em
`docs/REGISTRO-scrapers.md#ro`.

- **D-RO-FONTE:** shell em `agenciavirtual.sefin.ro.gov.br` (Bearer anônimo efêmero → re-extrair a cada
  rodada), API em `sydleone.sefin.ro.gov.br` (**tenant por host**, sem header de conta como o CE). App
  `servicedesk-embedded`. Listagem = **`GET _search`** (ES, `?_body=`) na classe de conteúdo
  `5cd32901…` (a MESMA do PI), filtrando o catálogo **"Serviços"** (`parent._id 662c1875…`). Escopo = só
  "Serviços" (194); "Temas" (42)/"Conteúdos" (28) informativos, fora (consistente com o CE).
- **D-RO-MODELO — Cenário A:** `tags` null e `classification` **403 anon** → público único "Serviços",
  classe "Geral". Identidade = `_id`; `link` = `agenciavirtual…/catalogo-servicos+{identifier}+{_id}`.
  Invariante `únicos == total ES`. UA institucional AuliBot (D-PA-ROBOTS preventivo).
- 194 serviços, 1 público. 8 testes.

### D-XX-SYDLE-COMPARTILHADO (aberta — a decisão de arquitetura mais importante)

**PI e RO são a MESMA geração Sydle ONE (conecta-360):** mesmo contrato `_search`, mesmas classes de
plataforma (conteúdo `5cd32901…`, catálogo `5ca3bca7…`, classification `5d66ec59…`). Diferem só em
`{BASE_API_URL host, app, catálogo _id, prefixo-de-link}`. → oportunidade de **um scraper parametrizável
cobrindo PI + RO** (e futuros conecta-360). **O CE NÃO entra** — é a geração antiga (`getChildren`,
classes diferentes, tenant por header `X-Explorer-Account-Token`). **Não decidir/refatorar sem alinhar:**
o trade-off é DRY (1 crate, N estados) × acoplar 2 estados a um contrato de terceiros que evolui. O
"3º tenant PA/portal-digital" ficou **inconclusivo** (estava em 522 na descoberta do PA; usamos o
paradigital/Prodepa — outra plataforma). Correlato ao D-AM-V2: RO/PI têm `contentHtml` inline para uma
eventual descrição rica v2.

## 17. Entidade `to` (SEFAZ-TO) integrada — ✅ **resolvida (18ª entidade)**

Carta de Serviços em `servicos.to.gov.br` — **ASP.NET WebForms / IIS (HTML server-rendered)**, molde
HTML-scraping (como BA/RJ), NÃO SPA/JSON. Descoberta em `docs/REGISTRO-scrapers.md#to`.

- **D-TO-FONTE:** SEFAZ = órgão **`cod_empresa=37`**. Listagem (1 GET) `listar_servico.aspx?cod_empresa=37`
  → 45 serviços; identidade = `cod_assunto_documento_tipo`. Detalhe (1 GET/serviço)
  `servico_detalhado.aspx?cod=…` — conteúdo rico (padrão gov.br "Carta de Serviços") em spans com id
  ASP.NET estável (`ctl00_…_lbl*`), parseados por id via `scraper` (html5ever decodifica entidades) —
  mais robusto que os accordions aninhados.
- **D-TO-MODELO — Cenário B:** `descricao` = Conceituação + Como solicitar + Documentos + Custos + Prazo
  (seções não-vazias, ~1,1 KB mediana). **público** = `lblTipoRelacionamento` (vocabulário fixo
  concatenado — Cidadão/Empresa/Órgão Público/Servidor; parse longest-first p/ "Órgão Público" não virar
  dois). **classe** = `lblTxtServicoGrupo`. `ocorrencias` = público × classe. `link` = a página de
  detalhe. UA institucional AuliBot + cortesia 500ms (D-PA-ROBOTS, 3º caso; robots.txt = 404).
- 45 serviços, 79 ocorrências, 4 públicos (Cidadão 35 / Empresa 38 / Órgão Público 5 / Servidor 1),
  2 classes. 8 testes. Portal multi-órgão → **3ª ocorrência de D-PA-ACERVO** (mas em ASP.NET/HTML, não
  JSON — parametrização menos direta que PA/ES).

## 18. Entidade `ma` (SEFAZ-MA) integrada — ✅ **resolvida (19ª entidade)**

Portal SGC (`portal-sgc.sefaz.ma.gov.br`) = **SPA Angular + API REST Spring Boot** (`/sgc/api`).
Descoberta em `docs/REGISTRO-scrapers.md#ma`.

- **D-MA-AUTH — anônima (molde GO):** o front loga com **credenciais PÚBLICAS baked no bundle**
  (`{id_cliente:"41", senha:"<bcrypt>", portal:true}` → `POST /sgc/api/login` → `{authtoken}`); token no
  header **`AuthorizationPortal`** (não `Authorization`). **Não são segredo** (servidas a todo visitante)
  — comentário no código p/ scanners (lição GO). Re-login a cada rodada (JWT efêmero).
- **D-MA-CATALOGO:** `GET /portal/servicos` com filtros obrigatórios (`flgPublicado=true&flgLocal=PORTAL&notOutros=false&page&pageSize`)
  → `{items, total}` (38). Descrição rica = `GET /portal/conteudos/{idConteudo}` (HTML→texto; 27 têm, 11
  link-only). JSON é UTF-8. Guard = `total` (invariante `únicos == total`).
- **⚠️ D-MA-TLS (gotcha novo):** o servidor manda **cadeia incompleta** (só a folha; falta o
  intermediário GlobalSign GCC R3). curl/ureq/rustls rejeitam; browser passa via AIA. Fix: **embutir o
  intermediário como trust anchor** (`RootCerts::new_with_certs`) — rustls padrão, SEM native-tls (cipher
  moderno). Difere do BA (native-tls por CBC). Se o cert for reemitido por outro intermediário, o
  handshake falha e avisa.
- **D-MA-MODELO:** `público` = `flgTipoServico` (COMPANY/CITIZEN/PUBLIC_AGENCY/CERTIFICATE →
  Empresa/Cidadão/Órgão Público/Certidões); `classe` = "Geral" (portal não usa categoria). `link` =
  `linkExterno` ou página de conteúdo. Identidade = `id`.
- 38 serviços, 38 ocorrências, 4 públicos. 6 testes.

## 19. Entidade `ap` (SEFAZ-AP) integrada — ✅ **resolvida (20ª entidade)**

Portal `www.sefaz.ap.gov.br` = **SPA Angular (FUSE)**. Descoberta em `docs/REGISTRO-scrapers.md#ap`.

- **D-AP-FONTE — catálogo hardcoded no bundle JS:** a página `#/categorias/{cat}/{servico}` mostra
  descrição rica, mas **nenhuma API dispara** — os dados são arrays `mock*` embutidos no chunk lazy
  `categorias_routes`, renderizados em runtime. O HTML servido é o shell vazio (pegar do DOM exigiria
  headless por página; ~50 renders). Pegamos do JS: **headless-free**. As chaves `route`/`titulo`/
  `descricao` **não são minificadas** → parse estável; só o **hash do chunk muda por deploy**.
- **D-AP-CHUNK — descoberta do hash:** shell → `runtime.<hash>.js` → mapa `"<CHUNK_NAME>":"<hash>"` →
  `<CHUNK_NAME>.<hash>.js`. Parse (regex) por categoria (`const mock<X> =`) casando
  `route → introducao.titulo → introducao.descricao`; `descricao` é template literal HTML → `html_to_text`.
- **D-AP-MODELO:** 5 categorias = **classe** (Cadastro/ICMS/ITCMD/Regime Especial/Veículos); público
  único "Serviços"; `link` = `…/#/categorias/{slug}/{route}`; identidade = link. `introducao.descricao`
  já traz "o que é" + Quem Pode Utilizar + Setor + Tipo. v2 possível: `documentos`/`requisitos`/`legislacao`.
- **Fragilidade:** é a fonte mais frágil da frota (parse de JS webpack). Se o `mock*` mudar de forma
  (chaves minificadas, virar API), o parse quebra; o guard de contagem (piso 44) avisa.
- 49 serviços, 1 público. 4 testes. `ServicoRaw` direto.

## 20. Entidade `ac` (SEFAZ-AC) integrada — ✅ **resolvida (21ª entidade)**

Portal `sefaz.ac.gov.br` = **WordPress + Elementor** (HTML server-rendered). Descoberta em `docs/REGISTRO-scrapers.md#ac`.

- **D-AC-FONTE:** `wp-json` = 404 (sem REST). A **Carta de Serviços** (`?page_id=6732`) lista **17
  serviços** em cards por categoria (Geral / Notas Fiscais / Cadastros / IPVA); cada card → post
  (`?p=NNNNN`) com descrição rica. Parse (regex) da Carta + `scraper` no detalhe: o corpo está em
  `.elementor-widget-theme-post-content` (1× por post) — **removendo `<style>`/`<script>`** (o Elementor
  injeta CSS inline no container; sem remover, a descrição vem poluída de CSS).
- **⚠️ D-AC-TLS:** o servidor manda o intermediário ERRADO (Sectigo RSA OV antigo) faltando o **R36**
  (emissor real do leaf) → nem sistema nem Mozilla/rustls fecham. Fix: embutir o R36 como trust anchor
  no rustls (`RootCerts::new_with_certs`), como o MA. (Diferente do MA, onde faltava o intermediário;
  aqui o servidor manda o intermediário ERRADO.)
- **D-AC-MODELO:** classe = categoria (Geral 6 / Notas Fiscais e Documentos Eletrônicos 3 / Cadastros 4 /
  IPVA 4); público único "Serviços"; `link` = `…/?p={post}`; identidade = o post. Robots desconsiderado
  (decisão do usuário; UA AuliBot).
- 17 serviços, 4 classes. 4 testes.

## 21. Entidade `df` (SEFAZ-DF) integrada — ✅ **resolvida (22ª entidade)**

Portal da Receita/SEEC-DF: **Carta de Serviços em ColdFusion** (`receita.fazenda.df.gov.br/aplicacoes/CartaServicos/`).
Descoberta em `docs/REGISTRO-scrapers.md#df`.

- **D-DF-FONTE:** **qualquer** `listaSubCategorias.cfm?...` (independente dos params) embute a **árvore
  inteira** do catálogo como objeto JS — subcategorias → `{'item':[{'url':'…servico.cfm?…','desc':'Título'}]}`.
  Logo **1 fetch** enumera os **472** serviços; cada `servico.cfm` traz descrição rica num **accordion**
  (`div.panel-body`). Parse (regex) dos tuplos `url`/`desc`; classe = chave-pai imediata (subcategoria,
  142 distintas). `index.cfm`/`/` = 404/erro CF (não existem). Sem headless.
- **⚠️ D-DF-WAF (JA3):** o host **reseta a conexão do `ureq`** (rustls/native-tls: `Connection reset by
  peer`) mas responde **200 ao `curl`** com o mesmo UA/URL → allowlist por fingerprint TLS, **igual ao
  GO** (§11). Toda a coleta via `kit::http::get_via_curl` (subprocess curl; requer `curl` no PATH). A
  cadeia de certificados fecha (curl `ssl_verify_result=0`); o bloqueio é do ClientHello do ureq.
- **D-DF-MODELO:** público = `codTipoPessoa` (Cidadão 6/22 = 168; Empresa 7/8 = 304); classe =
  subcategoria; `link` = URL absoluta do `servico.cfm` (única por serviço); identidade = `codServico`
  (0 serviços multi-subcategoria → 1:1). `ServicoRaw` direto.
- 472 serviços, 142 classes, descrição rica (~893). 4 testes.

## 22. Entidade `rn` (SEFAZ-RN) integrada — ✅ **resolvida (23ª entidade)**

Portal `www.sefaz.rn.gov.br` = **WordPress + SPA React**; a UVT (`uvt.sefaz.rn.gov.br`) é app
AngularJS/IIS transacional. Descoberta em `docs/REGISTRO-scrapers.md#rn`.

- **D-RN-FONTE:** o RN **não tem uma Carta de Serviços descritiva**. O único catálogo estruturado é o
  CPT **`servicos`** da WP REST (`/wp-json/wp/v2/servicos`, **15 cards**): `title` + `acf.categories`
  (classe) + `acf.link` (destino), **sem corpo próprio**. A UVT é transacional (login/emitir), sem
  catálogo público (`usuarios-api` sem swagger, `/api/servicos`=404) → fora de escopo.
- **D-RN-MODELO (decisão B do usuário):** montar os 15 cards e **enriquecer** os que apontam para um
  post (`/postagem/<slug>/`) buscando o corpo no ACF **`Matéria`** (o `content.rendered` desse tema é
  `null`); os demais (UVT/SEI externos, ou `acf.link=false`) ficam com descrição vazia. **5/15 ricos.**
  `titulo`/`Matéria`/categoria vêm com entidades HTML → `html_to_text`. `link` = `acf.link`
  (absolutizado se relativo; permalink quando `false`); identidade = link; público único; classe =
  categoria WP. `ServicoRaw` direto, UA AuliBot.
- **D-RN-ACERVO (aberta):** RN é intrinsecamente menu-only (molde RJ/PE, mas via API JSON limpa).
  Reavaliar se/quando o RN publicar uma Carta descritiva ou a UVT expor um catálogo público.
- 15 serviços (5 ricos), 4 classes. 4 testes.

## 23. Entidade `pb` (SEFAZ-PB) integrada — ✅ **resolvida (24ª entidade)**

Carta de Serviços em **PHP** (`cartaservico.sefaz.pb.gov.br`; o portal institucional
`www.sefaz.pb.gov.br` é Joomla). Descoberta em `docs/REGISTRO-scrapers.md#pb`.

- **D-PB-FONTE:** `servicos.php` = accordion aninhado (categoria → público → subcategoria → serviço) com
  links `saibamais.php?id=N` (**101 serviços**; cada id aparece **2×** — árvores por público → dedup por
  id). Cada `saibamais.php?id=N` = ficha rica com pares `<h3>Rótulo:</h3><h6>Valor</h6>`. Sem headless.
- **D-PB-MODELO (molde TO/DF):** `titulo` = `title=` do `inputbutton01`; `descricao` = os pares (menos o
  Público-alvo) + "Acessar o serviço: {URL}" (URL do `redireciona('id','URL')`, decodificada — o onclick
  às vezes vem com `&amp;amp;` duplo-encodado → html_to_text + colapsar `&amp;`); campos "-" descartados.
  **público** = campo "Público-alvo" da ficha (Cidadão/Empresa, per-serviço, pode ser ambos); `classe` =
  subcategoria imediata da listagem (botão de accordion mais próximo ≠ rótulo de público); `link` =
  `saibamais.php?id=N` (identidade). `ocorrencias` = público × classe. ureq OK (sem gotcha JA3).
- 101 serviços, 164 ocorrências, 51 classes, descrição rica (~1584). 4 testes.

## 24. Entidade `al` (SEFAZ-AL) integrada — ✅ **resolvida (25ª entidade)**

A SEFAZ-AL não tem portal próprio: serviços no **Portal Alagoas Digital** (API REST pública "Dados
Abertos", sem auth). Descoberta/validação em `docs/REGISTRO-scrapers.md#al`.

- **D-AL-FONTE:** `organs.json` (deriva o UUID da SEFAZ: `acronym=SEFAZ` + `nature=Estadual`, 1 match) →
  `services.json?organ_id={UUID}` (stubs) → `services/{id}.json` (detalhe rico). `robots.txt` libera
  tudo (`Disallow:` vazio) — D-AL-2 trivial, sem disregard. Sem headless.
- **D-AL-GUARDA (lição CE):** não hardcodar 60 nem o UUID; ler o tamanho da lista em runtime; bail se
  vazia; **guarda de coerência** `organ==UUID` em todo stub (senão o filtro falhou); cache só após guardas.
- **D-AL-MODELO:** `titulo`=`name`; `descricao`=`description`+prazo+etapas(canais)+requisitos+outras
  (HTML→texto); **público** = `audiences[]` (vocab controlado — **corrigido na validação:** NÃO
  `applicants[].type`, que é texto livre com 35+ valores); `classe` = `categories[]` (grosso: ~tudo
  "Economia e Finanças"); `link`=`url`; `ocorrencias`=público×classe.
- **Gotchas confirmados na validação:** `active` string↔bool (organs vs detalhe) → `serde_json::Value`;
  `requirements`/`estimated_time.min|max` **nuláveis**; enum `providing_channels[].type` =
  {WEB,TELEFONE,APLICATIVO-MOVEL,E-MAIL} (não o chutado PRESENCIAL/EMAIL/APP) + fallback; textos com tags
  **entity-encodadas** (`&lt;b&gt;`) → strip antes E depois do decode; público fallback = **"Contribuinte"**
  (não "Serviços": slug `servicos` colidiria com o arquivo agregado `al-servicos.json`).
- **D-AL-1 (escopo SEFAZ-only, aberta):** o portal é multi-órgão (1664 serviços/71 órgãos) → **D-PA-ACERVO
  puro**; basta relaxar o filtro `organ_id` para virar scraper estadual genérico. Não implementado.
- 60 serviços, 166 ocorrências, 7 públicos, 3 classes, descrição rica (~1030). 6 testes.

## 25. Entidade `se` (SEFAZ-SE) integrada — ✅ **resolvida (26ª entidade)**

Portal SharePoint 2013 (molde do PE), mas a Carta é uma **única página HTML**. Descoberta em
`docs/REGISTRO-scrapers.md#se`.

- **D-SE-FONTE:** a Carta de Serviços é `SitePages/servicos_cidadao.aspx` (~890 KB, Bootstrap accordion,
  **91 painéis**), chegada pelo menu SERVIÇOS → CARTAS DE SERVIÇOS. **1 GET**, sem detalhe por serviço.
  Becos descartados: "Servicos Importantes" (lista SP de 12 atalhos), "Biblioteca de Servicos" (library
  de PDFs/ZIPs por tema), `servicos_empresa.aspx` (404).
- **D-SE-MODELO:** `titulo` = heading (`<a href="#{id}">▾ Título</a>`, `id` = identidade); `descricao` =
  `panel-body` (campos `<strong>Rótulo:</strong> valor`), cortado no próximo `panel-heading` (evita vazar
  com `<div>` aninhado) + cap 2500 (1 painel do ITCMD tem ~22 KB); **público único "Serviços"**; `classe`
  = tema do `accordion_<tema>` que contém o painel (DFe/ICMS/ITCMD/IPVA/Simples/Contencioso/Cadastro;
  standalone antes do 1º tema → "Serviços Gerais"); `link` = `…#{id}`.
- **⚠️ D-SE-REDE:** o `ureq` falha com `unexpected end of file` na página de 890 KB (o SharePoint fecha a
  conexão de um jeito que o rustls rejeita, o curl tolera) → coleta via `kit::http::get_via_curl` (curl
  no PATH), como GO/DF — mas por **EOF**, não por JA3.
- **Raw string Rust:** o `href="#id"` contém `"#`, que fecha `r#"…"#` cedo → usar `r##"…"##` nas regex/
  fixtures que contêm âncoras.
- 91 serviços, 8 classes, descrição rica (~900). 4 testes.

## 26. Entidade `rr` (SEFAZ-RR) integrada — ✅ **resolvida (27ª entidade)**

Portal `www.sefaz.rr.gov.br` = site custom sem catálogo server-rendered. Descoberta em `docs/REGISTRO-scrapers.md#rr`.

- **D-RR-FONTE:** o nav só tem Ouvidoria/Transparência/Downloads; os serviços são apps GeneXus/SIATE em
  `portalweb.sefaz.rr.gov.br` (sem landing "Central de Serviços", tudo 404). MAS o `script.js` da home
  embute `const apps = [{category, title, description, href}, …]` (molde AP) — catálogo estruturado com
  descrição curta. **20 entradas → 16 hrefs distintos** (4 em `cidadao` E `empresa`).
- **D-RR-MODELO:** parse (regex) do array; `titulo`=`title`; `descricao`=`description` (curta ~97, teto da
  fonte); **público**=`category` (Cidadão/Empresa); `classe`="Serviços"; `link`=`href` (identidade, dedup
  acumulando o outro público). O FAQ (`faq-chat.php`) é matcher, não catálogo → fora de escopo.
- 16 serviços, 20 ocorrências (Empresa 14 / Cidadão 6), 2 públicos. 3 testes.

## 27. Resíduos das TAREFAs concluídas — os `TAREFA-*.md` foram apagados (2026-07-25)

As cinco especificações abaixo foram entregues e mergeadas, e os arquivos saíram da raiz do repo.
Esta seção guarda **só o que não está no código**: o que ficou em aberto, e os pontos em que a spec
divergia da implementação final. Onde houver divergência, **o código é a verdade** — a spec ficou
congelada no plano.

> **Se você chegou aqui vindo de um doc-comment:** o código cita as TAREFAs pelo nome
> (`TAREFA-SERVICOS-MD`, `TAREFA-FAQS-MD`, `TAREFA-FAQ-PR`, `TAREFA-EXTRACAO`, `TAREFA-CANONIZADOR`)
> em ~20 lugares. Os arquivos não existem mais — **é esta seção que os substitui**. As citações são
> linhagem deliberada, não links quebrados.
>
> **Segunda leva, apagada em 08/08/2026.** Mesmo tratamento, mas com um ponteiro melhor: cada uma
> tem o número do PR, e a mensagem de merge dele carrega o raciocínio. `TAREFA-TARF-SCRAPER`
> (#127), `TAREFA-FORMATO-MD` (#128), `TAREFA-MARCADORES` (#129), `TAREFA-DOCUMENTO` (#132/#133/#134)
> e `TAREFA-CONFIG` (#135). O `PEDIDO-verificacoes-TARF` saiu por outro motivo: as três perguntas
> dele foram respondidas pela implementação. A arquitetura que a `TAREFA-DOCUMENTO` entregou está
> descrita em [docs/auli_code.md](docs/auli_code.md) §3.13 — não aqui.

### TAREFA-SERVICOS-MD (PR #107) e TAREFA-FAQS-MD (PR #108) — ✅ entregues

Serviços e FAQs viraram árvores `.md` (`data/<id>/docs/{servicos,faqs}/*.md`), a FONTE do
`auli update`, irmãs da de pareceres. As 27 entidades migraram na campanha de jul/2026 — cada
migração exigiu **re-raspar**, porque a árvore deriva do snapshot e as entidades materializadas antes
dessa fronteira não tinham um (daí o `migrar-arvore-servicos.sh`, hoje [scripts/atualizar-servicos.sh](scripts/atualizar-servicos.sh)).
A campanha rendeu um efeito colateral: 13 das 25 entidades voltaram com o catálogo corrigido, porque
os portais tinham mudado desde a coleta anterior.

**✅ Fechado em 02/08/2026 — os dois fallbacks de transição (D9) saíram do
[update.rs](auli-server/crates/auli-cli/src/update.rs).**

1. **`servicos`** — o portal do Amapá voltou (200) e o `scripts/atualizar-servicos.sh ap` (então
   `migrar-arvore-servicos.sh`) migrou a última entidade: 49 serviços em `data/ap/docs/servicos/`.
   O gate do diff passou limpo — todos os artefatos de `raw/` idênticos ao backup, ou seja, o portal
   não mudou desde a coleta de 06/07 e a migração foi só troca de fonte.
2. **`faqs`** — o ramo já era inalcançável e o aviso, enganoso: imprimia `⚠️ <id>: sem árvore
   docs/faqs` em toda rodada, recomendando um `process` que não tinha o que produzir (só o RS tem
   FAQs, e o RS já tinha a árvore).

Com os dois fora, `auli update` lê **só** as árvores `docs/<kind>/*.md`; coleção sem árvore é
pulada em silêncio. A flag `--source`, que existia para apontar o JSON legado em `raw/`, saiu junto
— ficou sem leitor. Os hashes do pack do `ap` antes e depois da remoção são idênticos
(`3b956d3ec3238ac7` / `docs_hash 22ffb1e65d91211e`): nada do que é produzido mudou.

### TAREFA-FAQ-PR (PR #109) — ✅ entregue

A key de embedding das FAQs passou de `breadcrumb + pergunta` para `assunto + P: pergunta + R: resposta`
(`compose_faq_text_to_embed`). `STRATEGY_VERSION` **resetado 4 → 1** (D-FAQPR-7), todas as entidades
re-vetorizadas, smoke aprovado em 2026-07-25: o top-1 de _"quero emitir certidão de regularidade
fiscal"_ é uma FAQ cujo breadcrumb diz `2. IPVA ou Veículos` — pela fórmula antiga o contexto
indexado apontava para o lado errado. É o caso exato que a TAREFA existia para resolver.

**Duas decisões da spec foram superadas na execução:**

- **D-FAQPR-5 revogada.** A spec proibia truncamento silencioso e mandava erro alto agregado. Decisão
  do Carlos: **truncar em 8192 tokens + avisar**, nomeando os `.md` afetados (`avisar_faqs_truncadas`).
  Hoje nenhuma FAQ chega perto do teto.
- **D-FAQPR-4 superada.** O estimador `chars/4` deu lugar ao **tokenizer real** do fastembed
  (`Embedder::conta_tokens`), e `EMBED_MAX_TOKENS` subiu para **8192** (o `max_length` era 512).

**Aberto:** `data/eval/faq_pr_consultas.txt` — 32 consultas reais colhidas dos `logs/`, 22 delas
rotuladas com fragmentos aceitáveis. É o insumo dos harnesses
`crates/auli-cli/tests/{ab_faq_pr,ab_pool}.rs`, e mora **só em disco**: `data/*` é gitignored. O
conjunto rotulado custou trabalho manual (pooling estilo TREC) e não sobrevive à perda da máquina.
Já **encerrado:** não existe mais nenhum manifesto `strategy_version: 4` — a janela de rollback do
P6.3 fechou.

### TAREFA-EXTRACAO + TAREFA-CANONIZADOR (PRs #104–#106) — ✅ entregues, **só no RS**

Pipeline do knowledge graph em três subcomandos do `auli-collections`: **`extrair`** (LLM, one-shot)
→ **`canonizar`** (determinístico) → **`grafo`** (determinístico). Números reais do RS:
372/372 pareceres extraídos com **zero falhas** (`erros.jsonl` nem chegou a existir); **1.894
ocorrências** de dispositivo colapsadas em **719 chaves canônicas**; `grafo.json` com 113 nós
(97 dispositivos + 16 temas).

**Spec desatualizada:** o D7 da EXTRACAO fixava `max_completion_tokens = 2048`. O valor real é
**16384**, com resgate `reasoning_effort: low` no retry — com 2048 o gpt-oss entrava em runaway de
reasoning e estourava o teto sem emitir o JSON.

**Aberto:**

- **Cauda dura: 431 das 1.894 ocorrências (22,7%)** ficaram `canonizavel: false`, dentro da previsão
  de ~25% da spec. O grosso é **anáfora** (`§ 4º do referido artigo 31`), que por construção não se
  resolve sem o corpo do parecer — a v2 das regras precisaria de contexto, não de mais regex.
- **Vocabulário controlado de temas** — declarado fora de escopo nas duas TAREFAs e nunca feito. É o
  que falta para os temas virarem faceta confiável no grafo, em vez de texto livre do LLM.
- **Outros estados** — a tabela de aliases de norma (K5) é hardcoded em `ricms-rs`. Rodar SC/PR/SP
  exige a tabela do regulamento de cada um; o resto do pipeline generaliza.

---

## 28. Dívida de formatação zerada + guarda no CI — ✅ resolvida (2026-07-25)

**O sintoma:** rodar `cargo fmt` num crate qualquer reformatava código alheio e enchia o diff de
ruído. **A causa não era config:** os 36 crates são edition 2024 na mesma toolchain, e o único
`rustfmt.toml` do repo (em `auli-collections`) continha apenas os defaults escritos por extenso — mas,
por ser a única config, fazia aquele crate divergir dos outros 35. O código simplesmente **nunca
tinha sido formatado**: `cargo fmt --all -- --check` acusava **723 divergências / ~10 mil linhas**.

Foi testado se dava para evitar a reformatação alinhando a config ao estilo já escrito:
`use_small_heuristics = "Max"` derrubava para 438 hunks, mas as divergências restantes **invertiam de
direção** (o rustfmt passava a querer colapsar cadeias que o código quebrou). O código era
inconsistente consigo mesmo ⇒ não havia config que zerasse; qualquer saída passava por uma
reformatação em massa.

**Decisão: defaults do rustfmt, sem `rustfmt.toml` nenhum** — o padrão do ecossistema, que qualquer
editor com format-on-save reproduz sem configurar nada. Commit `95938f5` (114 arquivos,
`+4088/-1136`, só formatação; 430 testes verdes antes e depois, e os 6 warnings do clippy conferidos
como preexistentes contra o HEAD anterior). O `.git-blame-ignore-revs` (`79048f9`) faz o `git blame`
atravessá-lo — o GitHub honra sozinho; localmente, uma vez por clone:
`git config blame.ignoreRevsFile .git-blame-ignore-revs`.

**Guarda:** [.github/workflows/fmt.yml](.github/workflows/fmt.yml) roda `cargo fmt --check` em PR e
push. Barato pelo mesmo critério do `scraper-boundary`: o rustfmt parseia, não compila (nada de
fastembed/ort).

**⏳ Risco em aberto — deriva de versão do rustfmt.** Não existe `rust-toolchain.toml`: a máquina do
Carlos usa 1.96.0 stable e o runner do GitHub usa a stable que estiver instalada nele. O rustfmt é
estável entre versões, mas se algum dia o CI acusar divergência que não reproduz local, a causa é
essa — e o remédio é pinar a toolchain (o que também tornaria o build reprodutível, decisão maior que
afeta todo mundo, por isso não foi feita agora).

---

## 29. Página **Sobre**: duas afirmações falsas — ✅ **resolvidas (2026-08-02)**

O commit `509a021` atualizou [about.md](auli-frontend/public/about.md) e removeu duas alegações
mortas da lista de tecnologias — **ChromaDB** e **Ollama**, aposentados quando a busca passou a ser o
crate `vector-store` embutido e os embeddings o BGE-M3 via fastembed, ambos in-process. Sobraram
outras duas, da mesma família, registradas abaixo.

> **Resolvidas pelo mantenedor em 2026-08-02.** A linha do JWT foi retirada. A seção Privacidade foi
> reescrita em torno do que é verdadeiro e verificável: a Auli **não identifica quem pergunta** (sem
> cadastro, cookie de rastreamento ou IP em disco), **registra a pergunta como escrita** — com o
> aviso explícito de que dado pessoal digitado pelo usuário fica gravado — e **mascara CPF/CNPJ/
> telefone/e-mail antes de qualquer chamada ao provedor do modelo**. A promessa genérica de "sem
> coleta de dados pessoais" saiu: ela induzia exatamente o comportamento contra o qual parecia
> proteger. O gatilho da revisão foi a inclusão do MCP no log de auditoria, que tornou o texto
> antigo ainda mais distante do comportamento real.

**1. "Autenticação JWT (RS256)" — não existe.** `grep -rl "jsonwebtoken\|RS256"` no workspace
inteiro (`.rs`, `.toml`, `.ts`, `.tsx`) devolve **zero**. O que protege o servidor hoje é rate
limiting — o próprio (`api/ratelimit.rs`) mais o do Cloudflare —, e o item **D-MCP-9** desta mesma
página já registra "hoje sem autenticação: é conteúdo público e somente-leitura". Herança de uma
arquitetura anterior, igual ao ChromaDB.

**2. A seção Privacidade promete mais do que o sistema entrega.** Ela afirma que o sistema registra
"apenas a pergunta e a resposta gerada — sem ... coleta de dados pessoais". Isso conflita com a
doutrina do log fechada em 2026-07-25 (§7.0 do `docs/auli_operations.md`): o log de auditoria é
**deliberadamente íntegro** e guarda a pergunta crua e a resposta restaurada. Na auditoria dos 118
arquivos, **3 tinham PII do usuário** na pergunta e 1 na resposta. Se a pessoa digita o CNPJ, ele vai
para o disco.

Esta segunda **já estava mapeada** em [docs/auli-anon_pendencias.md](docs/auli-anon_pendencias.md) ("Aba
'Sobre' (frontend). Não prometer 'dados 100% anônimos'"), com a redação sugerida — que é verdadeira
e continua favorável, porque descreve a fronteira que o `auli-anon` de fato protege:

> identificadores estruturados (CPF, CNPJ, telefone, e-mail, …) são mascarados automaticamente antes
> de sair do processo

O disclaimer de IA que o mesmo commit acrescentou (rodapé do chat + seção "Aviso importante") cobre
**variação e imprecisão da resposta**, que é outro eixo — não substitui nenhuma das duas correções
acima.

---

## 31. `update` incremental — a maior alavanca de performance do sistema (aberta — 2026-08-02)

O custo **não está em embeddar devagar, está em re-embeddar o que não mudou**.

O throughput do embedder satura em **~6 texts/s** e é insensível a hardware: medido de 1 a 32
threads, de 8 para 24 (o triplo dos núcleos) compram-se 17%, e com 32 piora — existe um componente
serial/limitado por memória que núcleo nenhum atravessa. Paralelismo entre PROCESSOS também não
escapa: 4×6 threads deram 4,6 texts/s agregados, pior que 5,3 de um processo com 24. Consequência
grátis: `EMBED_THREADS=8` entrega ~85% do throughput com um terço dos núcleos.

**O problema.** O `update` é `Writer::reset` + `upsert` por coleção (`update.rs`, ~linha 369). Toda
execução re-vetoriza a entidade inteira do zero. Chegaram 40 pareceres novos em SP? Os 15.605 são
re-embeddados. A ~6 texts/s medidos, isso são **~46 min** para produzir ~40 vetores inéditos.

Um update incremental faria o caso típico cair para **segundos**. É ganho de duas ordens de
grandeza — e, pela saturação acima, nenhum hardware chega perto disso.

**Por que o `reset` existe** — e é aqui que mora o trabalho de verdade. Não é preguiça: os ids são
**posicionais**, `id-1..id-N` atribuídos pela ordem alfabética da árvore. Inserir um documento no
meio desloca o id de todos os seguintes, e sem o `reset` sobrariam órfãos `id-(N+1)..`. O `upsert`
já substitui por id (`vector-store/src/write.rs:45`); o que falta é o id ser **estável**.

**Esboço** (não é decisão fechada — a pendência é justamente decidir):

1. **Id estável e derivado do conteúdo.** O candidato natural já existe: o `mddoc::slug` usado na
   materialização e no `doc_path` dos pareceres. Trocar `id-N` pelo slug torna o id independente de
   posição, e o `reset` deixa de ser necessário.
2. **Saber o que mudou sem embeddar.** Guardar por registro o hash do `text_to_embed` (a key, não o
   documento — é a key que determina o vetor). Diferença de hash ⇒ re-embeddar; igual ⇒ reaproveitar
   o vetor que já está no pack.
3. **Remoções.** Documento que sumiu da árvore = id presente no pack e ausente no disco; hoje o
   `reset` resolve por construção, no incremental vira um diff explícito de conjuntos de id.

**Riscos e pontos a resolver:**

- Trocar o esquema de id **muda o formato do pack** ⇒ um reembed geral de migração (uma vez só) ou
  um caminho de compatibilidade. Ironia registrada: para nunca mais re-embeddar tudo, re-embedda-se
  tudo uma última vez.
- O `docs_hash` do manifesto (`manifest::validate_docs_hash`) cobre a árvore inteira e protege o boot
  contra pack/árvore divergentes. Ele **continua valendo como está** — é ortogonal ao incremental —,
  mas convém reconferir a interação antes de mexer.
- A guarda `recusar_pareceres_sem_sinopse` roda sobre a árvore toda e deve continuar rodando sobre a
  árvore toda, não só sobre o delta: uma sinopse apagada em documento antigo precisa seguir sendo
  recusada.
- Determinismo: o pack resultante de um incremental tem que ser **idêntico** ao de um update do zero.
  Isso pede um teste — vetorizar do zero vs. incremental e comparar byte a byte — que é a única
  garantia real de que o atalho não introduziu deriva.

### 31.1 A outra metade da dívida: a RAM — **remedido em 2026-08-08**

O tempo é só metade do problema. O `update` carrega a coleção INTEIRA na memória antes de embeddar
(`preparar` monta um `Vec<Documento>` com o corpo de cada documento; `ingest_items` deriva dele mais
dois vetores de String). Medido com `/usr/bin/time -v` na migração de pack v2, que re-vetorizou as
27 entidades:

| entidade | documentos | tempo | RSS máximo |
|---|---|---|---|
| 22 menores | 15–474 | 2–20 s | ~1,13 GB (é o modelo, não o dado) |
| sc / pr | ~1.950 / ~2.200 | ~4min30s | ~4,5 GB |
| rs | 6.705 | 16min21s | 16,4 GB |
| **sp** | **16.123** | **49min21s** | **35,4 GB** |

**O número do SP mudou**: o registro anterior (HANDOFF, 2026-08-08 de manhã) dizia **24,7 GB**. Não
sei afirmar a causa da diferença — pode ser instrumentação distinta ou crescimento da árvore entre
as duas medições. O que vale é o dado novo, medido com `time -v` e reproduzível.

**Por que isso é a mesma pendência.** O remédio do tempo (processar por documento, re-embeddar só o
que mudou) é também o remédio da RAM: a unidade de trabalho uniforme que a `Documento` trouxe é o
que torna o streaming um caminho só, em vez de quatro. Quem atacar a §31 paga as duas de uma vez.

**Consequência prática hoje.** Nesta máquina (60 GB) o SP passa. Num servidor de 16 GB, não — e é
esse o teto que a dívida impõe a onde o Auli pode rodar.

---

## 32. Redesenho do frontend, entrega 3: botão "Entrar" no cabeçalho (aberta — 2026-08-02)

Última das três partes de um redesenho cujas duas primeiras já estão em produção na `v0.1.60`. As
outras duas **evitaram deliberadamente** o `AppHeader.tsx` para que ele fosse mexido de uma vez só,
aqui.

| entrega | o que era | estado |
| --- | --- | --- |
| 1 | navegação em sidebar agrupada, no lugar das 9 abas planas | ✅ PR #118 |
| 2 | seletor de tipo de consulta dentro da caixa de mensagem | ✅ PR #119 |
| 3 | **botão "Entrar" no cabeçalho — casca, sem backend** | aberta |

**O escopo declarado é só a casca.** Nada de autenticação de verdade: sem provedor, sem sessão, sem
token, sem rota protegida. A motivação registrada na entrega 1 era outra — a barra de abas "não tem
para onde crescer quando o login chegar" —, ou seja, o botão existe para reservar o lugar e provar
que o cabeçalho comporta o crescimento.

**Onde mexe.** Só em [`AppHeader.tsx`](../auli-frontend/src/shared/AppHeader.tsx). O cabeçalho hoje é
um `Flex` de três colunas, com `minW="96px"` nas laterais para manter o "Auli" centralizado:

- **esquerda** — a pílula `UF + trocar` (só aparece com entidade escolhida);
- **centro** — "Auli" e o subtítulo (`v.0.1.60 · SEFAZ-RS`);
- **direita** — o `ColorModeButton`, sozinho.

O "Entrar" naturalmente cai à direita, ao lado do botão de tema. O `minW="96px"` das laterais
provavelmente precisa subir para os dois lados continuarem simétricos e o título não sair do centro.

**O que a tarefa NÃO decidiu, e é o que a pendência guarda:**

- **O que o botão faz sem backend.** Abre um diálogo com "em breve"? Não faz nada e fica
  `aria-disabled`? Aponta para um formulário externo? Casca que não comunica o que é vira ruído.
- **Se há estado a fingir.** Um "Entrar" que nunca muda para "Sair"/avatar é meia casca; um que muda
  exige decidir onde mora esse estado — e hoje o único estado global do app é a entidade
  (`EntityContext`, persistida em `localStorage`).
- **Para que serviria a conta**, quando existir. Isso não é detalhe de UI: hoje **nada** no sistema é
  por usuário — o log de auditoria é deliberadamente anônimo (§7.0 do `auli_operations.md`: "não
  registramos nomes ou IPs"), e a `about.md` afirma ao usuário que "a Auli não sabe quem você é". Um
  login real colidiria com essa promessa e exigiria revisá-la **antes**, não depois. A casca não
  colide; o que ela promete, sim.

**Restrições herdadas das entregas 1 e 2** (não reabrir sem motivo):

- O cabeçalho é `position: sticky` com `zIndex={100}`, acima do `zIndex={20}` da barra de composição
  do chat — que é `fixed` e recua a largura da sidebar (ver `shared/layout.ts` e a PR #121).
- A superfície é `bg.inverted` (preta nos dois modos), então o botão precisa das folhas `fg.inverted`
  / `border.inverted`, como a pílula de troca de estado já faz. Ver a tabela de tokens no
  `auli-frontend/THEME.md`.
- O guarda-corpo de cor (`no-restricted-syntax`) proíbe literal de cor em componente; qualquer tom
  novo entra como token semântico no `system.js` **e** na tabela do `THEME.md`.

---

## 33. Clippy: dívida zerada, mas **sem guarda no CI** (parcial — 2026-08-04)

Irmã da §28, com um desfecho diferente: a dívida foi zerada, o gate **não** foi criado.

**O sintoma.** `cargo clippy --workspace --all-targets -- -D warnings` reprovava desde o bump para o
Rust 1.96 — 5 erros de lints que não existiam quando o código foi escrito (`collapsible_if` em
`auli-scraper-pe`, `unnecessary_get_then_check` e três `unnecessary_literal_unwrap` em `mcp.rs`).
Apareceu ao verificar a Fase 4 do `auli-anon`; zerado no PR #123, sem mudança de comportamento.

**Uma sugestão do clippy foi recusada, de propósito.** Em `top_k_e_limitado_como_no_retrieve`
([mcp.rs](auli-server/crates/auli-cli/src/mcp.rs)), a correção que o lint sugere deixaria
`assert_eq!(DEFAULT_TOP_K.clamp(1, MAX_TOP_K), 5)` — que testa o `clamp` da std e apaga o que o teste
existe para provar: que a `Option` ausente cai no padrão. A expressão passou a vir por closure, com o
porquê em comentário no código. **Lição:** lint que incide sobre teste às vezes pede para apagar a
asserção; ler antes de aplicar.

**Por que não há workflow de clippy.** O argumento que torna o `fmt.yml` barato **não vale aqui**: o
rustfmt parseia, o clippy **compila**. No workspace inteiro isso puxa `fastembed`/`ort` — exatamente
o custo que o `scraper-boundary` existe para evitar. Um gate honesto precisa de decisão própria:
escopar aos crates leves (`scrapers/*` + `auli-contract`), ou aceitar o workspace inteiro com cache de
`~/.cargo` e `target/`.

**⏳ Consequência assumida:** enquanto não houver gate, esta dívida só aparece para quem roda
localmente e **volta a crescer no próximo bump de toolchain** — foi assim que os 5 chegaram. O mesmo
risco de deriva de versão descrito no fim da §28 vale aqui, e com mais força: o clippy muda de lints
entre versões bem mais do que o rustfmt muda de estilo.

---

## 34. Dependências Rust: lockfile atualizado, **`fastembed`/`ort` segurados** (parcial — 2026-08-04)

Análise minuciosa de tudo que dava para subir no workspace. O lockfile andou (PR #124, um arquivo,
nenhuma linha de código); o embedder **não**, de propósito — e o motivo é o miolo desta seção.

### 34.1 O que entrou — ✅ resolvido

`cargo update` moveu **109 pacotes** dentro do semver: tokio 1.52.3 → 1.53.1, regex 1.12.4 → 1.13.1,
rustls 0.23.40 → 0.23.43, hyper 1.10.1 → 1.11.0, http 1.4.2 → 1.5.0, serde 1.0.229, clap 4.6.5,
time 0.3.55, anyhow 1.0.104.

Rodei os 452 pacotes do lock contra a **OSV** (não há `cargo-audit` instalado). Eram 4 alertas; 3
saíram:

| Pacote | Aviso | Nota |
|---|---|---|
| anyhow 1.0.102 → 1.0.104 | RUSTSEC-2026-0190 — unsoundness em `Error::downcast_mut` | não chamamos `downcast_mut` em lugar nenhum |
| crossbeam-epoch 0.9.18 → 0.9.20 | RUSTSEC-2026-0204 — deref inválido no `fmt::Display` de `Atomic`/`Shared` | transitivo via rayon ← tokenizers ← fastembed |
| quinn-proto 0.11.14 → 0.11.16 | RUSTSEC-2026-0185 / GHSA-4w2j-m93h-cj5j — **HIGH**, exaustão de memória remota | **não compilava aqui** (`http3` opcional do reqwest; `cargo tree --invert --target all` não o acha). Só existia no lock — mas some dele e deixa de ser falso positivo de auditoria |

Sobra `paste 1.0.15` (não mantido, informativo, **sem correção upstream**), transitivo via
tokenizers ← fastembed. Nada a fazer deste lado.

### 34.2 A armadilha: subir `fastembed` muda os vetores — **medido**

**O `fastembed` pina o `ort` por igualdade exata** — `5.17.2 → ort =2.0.0-rc.12`,
`5.17.4 → ort =2.0.0-rc.13`. Não existe pegar as correções de um e segurar o outro: **o par anda
junto ou não anda**.

Sonda reproduzindo a construção do `Embedder` ([embed.rs](auli-server/crates/auli-core/src/embed.rs))
— BGEM3Q, `max_length` 8192, `batch_size` 1, o modelo local de `models/` —, 6 frases, sob os dois
pares:

| Texto | cosseno rc.12 × rc.13 |
|---|---|
| PARECER — ICMS, crédito de energia elétrica | **0,983785** |
| PARECER — ICMS, ST de medicamentos | **0,987717** |
| IPVA — isenção para PCD | **0,984923** |
| Como emitir a guia do ITCD? | 1,000000 |
| Alíquota do ICMS interestadual, bens importados | **0,986802** |
| Regime especial para contribuinte substituto | 1,000000 |

- **Controle:** duas execuções do _mesmo_ build dão arquivos byte-idênticos → a diferença é da
  versão, não do processo.
- **Escala:** textos **distintos** ficam entre 0,31 e 0,63 de cosseno. A faixa angular útil vai de
  ~0,63 a 1,0.

Quatro das seis frases mudam de vetor, na **mesma ordem de grandeza do vazamento de padding que
forçou a `STRATEGY_VERSION` v4** (cosseno 0,978 — e `embed::testes_ordem` registra que aquilo
"trocava documentos na fronteira do top-k").

**E a guarda de boot não pega.** `EmbedIdentity` é `(embed_model_id, embed_dim, strategy_version)`
([manifest.rs](auli-server/crates/auli-core/src/manifest.rs)); **nenhum dos três muda com o `ort`**.
Packs construídos sob rc.12 seriam servidos por um encoder de query rc.13 sem uma linha de
reclamação no boot. Os testes existentes também não pegam: `embedding_e_deterministico_entre_chamadas`
e `embedding_independe_da_ordem_e_da_composicao_do_lote` comparam **dentro de um build**.

**Regra que fica:** a identidade do espaço de embeddings não está só no modelo — está também no
runtime que executa o modelo. Subir `fastembed` é tarefa própria: **bump de `STRATEGY_VERSION` +
re-ingestão completa das entidades**, decidido de propósito e não de carona num refresh de lockfile.

**A receita para repetir a atualização sem arrastar o embedder** (`cargo update --exclude` **não
existe** no cargo 1.96 — a tentativa óbvia falha):

```bash
cargo update
cargo update -p fastembed --precise 5.17.2   # o pin exato traz o ort de volta sozinho
```

### 34.3 Um item que merecia checagem e passou

**zlib-rs 0.6.6 → 0.6.7** é o backend deflate do `zip`, e o
[bundle.rs](auli-server/crates/auli-cli/src/bundle.rs) exige zip **byte-idêntico** com nível fixo 6 —
mudar a compressão mudaria todos os `sha256` do manifesto de downloads de uma vez. Comprimi o mesmo
corpus de markdown tributário nas duas versões: `sha256` **igual** nos níveis 1, 6 e 9. As mudanças
da 0.6.7 são LoongArch/LSX, não tocam x86-64.

### 34.4 Aberto — o que a análise levantou e o PR não fez

**⏳ Teste de vetor-ouro no `embed.rs`.** É o que fecharia o buraco da §34.2: algumas frases com
vetores conhecidos gravados, `#[ignore]` como os outros, falhando alto no próximo bump de `ort`.
Ficou de fora por ser código novo e merecer decisão própria. **Enquanto não existir, o único
guarda-costas é esta seção.**

**⏳ `tower-http` pede `features = ["full"]` para usar só o `CorsLayer`**
([Cargo.toml](auli-server/crates/auli-cli/Cargo.toml), [api/mod.rs](auli-server/crates/auli-cli/src/api/mod.rs)),
arrastando async-compression/flate2/zstd/brotli/fs para o build. Estreitar para `["cors"]` rende mais
que qualquer bump de versão e independe dele. A 0.7.0 em si é de baixo risco — os quebras são em
compression, follow-redirect, services/fs e trace/classify; em CORS a única mudança é relaxar o
`Vary`.

**⏳ `rmcp 2.2 → 3.x` é migração de protocolo, não bump.** A 3.0 implementa o MCP 2026-07-28, que
**remove as sessões do protocolo** (sem `Mcp-Session-Id`, sem GET standalone, sem término por DELETE,
sem retomada por `Last-Event-ID`) — e é exatamente o caminho que o `LocalSessionManager` monta em
[api/mod.rs](auli-server/crates/auli-cli/src/api/mod.rs). `stateful_mode` vira `legacy_session_mode`;
`call_tool` passa a devolver enums cientes de MRTR. Há guia oficial de migração. Ver também a seção
**MCP v2**.

**❌ `sha2 0.10.9 → 0.11.0` — descartado.** Verificado compilando: a 0.11 troca `GenericArray` por
`hybrid_array::Array`, que **não implementa `LowerHex`**, e o `format!("{:x}", hasher.finalize())` do
`bundle.rs` para de compilar. Em troca de nada — o digest SHA-256 é o mesmo e não há aviso de
segurança.

**Higiene do lock, para quem for auditar:** há **dois majors de reqwest** (0.12.28 via hf-hub,
0.13.4 via auli-llm) e o `quinn` inteiro, e nenhum aparece na árvore compilada — são entradas de
dependência **opcional** que o lock guarda mas o build nunca toca.

### 34.5 O elo com a §33

**Nenhum workflow da CI compila Rust** — `fmt`, `frontend`, `registry-sync` e `scraper-boundary`
parseiam ou rodam script, e a decisão continua certa pelo mesmo motivo da §33 (clippy puxaria
`fastembed`/`ort`). A consequência concreta, agora com um caso nomeado: **um `cargo update` que
mudasse os embeddings passaria por toda a CI sem um vermelho.** Quem sustentou o PR #124 foi a suíte
local — 483 testes, os três `--ignored` do embedder contra o modelo real, e `cargo fmt --check`.

---

## MCP v2 (aberta — o que a v1 deliberadamente deixou de fora)

A v1 (`auli-retrieval` + `/v1/retrieve` + `/mcp`, gates G1..G5) subiu com três ferramentas e rate
limit; a quarta (`consultar_servicos_faqs`) entrou depois — ver o item riscado abaixo. Em 08/08/2026
as duas de jurisprudência ganharam o parâmetro opcional `colecao` (`pareceres`, o padrão, ou `tarf`),
com compatibilidade byte a byte para quem não o manda (#132). O que ficou registrado como próximo
passo:

- **Auth opcional no `/mcp` (D-MCP-9).** Hoje sem autenticação: é conteúdo público e somente-leitura,
  atrás do tunnel e com rate limit próprio (10 req/s, burst 30). Uma API key por header
  `Authorization` é o passo natural se o acervo deixar de ser inteiramente público — ou se o custo
  de CPU do embedder passar a incomodar.
- **Rate limit do `/mcp` em JSON-RPC.** O 429 sai como JSON simples, não como erro JSON-RPC.
  Correto no nível HTTP e os clientes tratam, mas um middleware próprio para `/mcp` daria uma
  mensagem melhor ao assistente.
- ~~**Ferramenta `buscar_servicos_faqs` (D-MCP-7).**~~ ✅ **resolvida (2026-08-02)** — entrou como
  **`consultar_servicos_faqs`** (o nome mudou porque as tools `buscar_*` devolvem JSON de hits e
  esta devolve o bloco de contexto do chat, não uma lista). Reusa `rag::retrieve` e
  `rag::montar_rag_servicos_faqs`, com as MESMAS constantes do chat — a saída é byte a byte a do
  portal, e calibrar as bandas (item abaixo) move as duas faces junto. `listar_entidades` passou a
  reportar os três kinds.
- ~~**`buscar_pareceres` multi-UF numa chamada.**~~ ❌ **decidido: NÃO fazer (2026-08-02).** Fica uma
  UF por chamada, com o assistente iterando. O custo real é um round-trip por estado numa pergunta
  comparativa — barato perto do que a alternativa traria: juntar UFs numa resposta só convida a
  ordenar os resultados entre elas, e **score de cosseno só é comparável dentro da mesma identidade
  de embedding**. Hoje todos os packs compartilham a identidade, então funcionaria; no dia em que
  uma entidade for reembedada com outro modelo, viraria erro silencioso de ranqueamento. Iterar
  mantém cada resposta dentro de um pack só. (O assistente ainda pode comparar entre chamadas —
  isso o servidor não controla —, mas ele não é induzido a isso pela forma da resposta.)
- **Calibração das bandas.** `SVC_BAND`/`FAQ_BAND`/`PAR_BAND` seguem em `f32::INFINITY` (mantendo
  a paridade com o comportamento antigo de "pega tudo até o teto"). Agora que o `/v1/retrieve`
  **expõe os scores**, dá para calibrar com dados reais: rodar os ~10 testes do SC, ler os arrays
  de distância e baixar cada banda para logo acima de onde os matches genuínos se separam do
  enchimento. É a pendência com maior efeito sobre a qualidade da resposta, e ganhou um argumento
  novo em 08/08/2026: com as bandas em infinito, uma busca que não acha NADA devolve `n_results`
  distratores ao LLM em vez de contexto vazio — medido em
  [docs/ESTUDO-busca-hibrida.md](docs/ESTUDO-busca-hibrida.md) §3. (Ao mexer nelas, rodar
  `scripts/parity-replay.py` — ver `docs/auli_operations.md` §6.1 — para ver EXATAMENTE quais documentos
  mudam de contexto.)

### Latência percebida no chat via MCP (medido 2026-07-21)

Primeiro uso real (degrau 2): o assistente encadeou `listar_entidades` → `buscar_pareceres` →
`obter_parecer` corretamente, mas a resposta veio **lenta**. Diagnóstico medido — **o servidor NÃO
é o gargalo**:

| Etapa | Custo | Nosso? |
| ----- | ----- | ------ |
| 3 tool calls sequenciais = ~4 turnos de LLM | dominante | não — é como tool-chaining funciona |
| Modelo ingerir o texto devolvido (10–24 KB numa pergunta com `obter_parecer`) | relevante | em parte (tamanho do payload) |
| Embed + varredura vetorial (até SP, 15.605 docs) | 17–49 ms | já rápido |
| Round-trip pelo tunnel Cloudflare | ~140 ms | não |

Tamanhos medidos: `buscar_pareceres` top_k=5 ≈ **10 KB** (5 hits, cada um com a sinopse inteira);
`obter_parecer` = **13,6 KB** (SC) e até **54 KB** no pior caso do SP. O modelo lê tudo isso antes
de responder — é daí que vem a lentidão, não do nosso compute (~50 ms).

Alavancas reais, se a lentidão incomodar no uso contínuo (nenhuma urgente):

- **Sem código, do lado do usuário:** pedir menos quando precisa de menos — "só títulos e links,
  não o texto integral" pula o `obter_parecer` (o payload de 14–54 KB) e o modelo responde direto
  dos hits da busca.
- **Enxugar a sinopse no `buscar_pareceres`.** O grosso dos 10 KB é o campo `resumo` inteiro dos 5
  hits, com blocos de palavras-chave. Um preview mais curto por hit corta o payload do turno do
  meio — MAS a sinopse é justamente o que deixa o modelo julgar relevância; é troca real, não ganho
  de graça. Medir antes/depois com o tamanho do `buscar_pareceres`, não com o `parity-replay.py`
  (que é ordem de recuperação, não payload).
- **Handshake só na 1ª chamada.** `initialize`/`tools/list` rodam uma vez por sessão; as chamadas
  seguintes reaproveitam. Não é recorrente.

## 35. Página **Sobre** e manuais MCP não mencionam o TARF (aberta — 2026-08-09)

Achado ao revisar os quatro documentos vivos. Não é afirmação falsa como as da §29 — é **omissão**,
e ela aparece justamente no texto voltado ao público.

O [about.md](auli-frontend/public/about.md) descreve o acervo em três lugares como "serviços,
perguntas frequentes e pareceres" (linhas 13, 27 e 39). Os acórdãos do TARF estão no chat como fonte
própria, na aba, no MCP e nos zips de download desde 08/08 — quem lê a página conclui que não
existem. A linha 39, que fala do conector MCP, cita "o acervo de pareceres e respostas a consultas"
sem o parâmetro `colecao`.

**Por que ainda não corrigi:** o total do TARF muda a cada rodada de coleta (3.800 de ~22.522), e o
texto do Sobre é do tipo que se escreve uma vez. Corrigir agora significa ou omitir o número — o que
é aceitável, o Sobre não dá números hoje — ou voltar nele quando a coleta fechar. **Decisão a
tomar:** escrever sem número agora, ou esperar o fecho da coleta.

Os manuais em `manuais/` estão na mesma situação por decisão explícita e registrada: eles ganharam a
descrição do parâmetro `colecao`, mas **não** um total do TARF, para não publicar um número que muda
toda semana (#132).

---

## D-NAMING (pendência separada — MG, NÃO é do GO)

Política da frota: separador sigla–UF sempre `-`. Normalizar o `orgao` do **MG** `"SEF/MG"` →
`"SEF-MG"` em [`auli-scraper-mg/src/mg.rs:222`](auli-server/crates/scrapers/auli-scraper-mg/src/mg.rs#L222)
(o `registry.toml` já usa `SEF-MG`; confirmado em todos os itens do snapshot — 149 na coleta atual). Muda bytes do snapshot MG →
**recoleta verificada + commit de dados próprio**, na próxima vez que o MG for tocado.

---

## Itens relacionados (revisões de código anteriores)

- **`public/<id>/servicos.json` (~660KB) e contratos do engine — ✅ resolvido:** o `build-frontend-public.sh`
  copia só os artefatos que a UI busca (per-público + index + `faqs-tree.json` + `ref/`), prefixando
  com `<id>-`; os contratos `<id>-{faqs,servicos}.json` **não** vão para `public/`.
- **Gate de coleção no frontend — ✅ resolvido (auditoria #4):** `ServicosList` gateia em
  `hasCollection` como as irmãs; coleção ausente rende `CollectionEmpty` (sem 404). **Menor/aberto:**
  [Home.tsx](auli-frontend/src/pages/home/Home.tsx) ainda lista as abas de forma estática (a barra
  mostra todas), mas cada lista se auto-gateia — logo estados sem uma coleção mostram
  `CollectionEmpty`, não erro.
- **Prompts `config/prompts/*.txt` — ✅ resolvido (TAREFA-MARCADORES, #129):** a nota da auditoria #14
  registrava que os prompts divergiam no marcador de serviço (`rs.txt` tinha `## servico`;
  `sc/pr/sp.txt` só `## pergunta`). Os quatro invólucros do contexto RAG viraram um só
  (`## documento {i}: {rótulo}`) e os 31 prompts foram alinhados no mesmo commit do código.
  Conferido em 08/08/2026: **32 de 32** prompts de RAG citam `## documento`, **nenhum** cita
  `## servico` (os três de fora — `extracao*.txt`, `sinopse.txt` — não consomem contexto RAG).
- **Comentário histórico:** [derive_faqs.rs:29](auli-server/crates/auli-collections/src/derive_faqs.rs#L29)
  cita `EmbedStrategy::QuestionKey` (tipo já removido) — referência de lineage, cosmética.
- **Formato de links/slugs não uniforme — aceito (não corrigir por ora):** FAQs do RS emitem
  `[texto](url)`; os serviços (RS/SC/PR/SP/MG) emitem `texto "url"`. O `linkify.tsx` do frontend
  linkifica URLs cruas nos dois casos, então funciona, mas o texto enviado ao LLM não é homogêneo.
  Idem os slugs de servidores: `servicos-a-servidores-publicos` (RS/SC) vs `servicos-a-servidores`
  (SP). Uniformizar re-gera todos os packs (muda o texto embarcado / o espaço vetorial) sem ganho
  funcional — **documentado como dívida aceita**.
