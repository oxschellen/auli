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

`auli_code.md` (§3.1 layout, §5 scrapers/cobertura, §7 resumo) e o runbook `auli_operations.md`
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
`descoberta-am.md`.

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
não-vazias por serviço para pegar regressão de parser. Evidência e amostras em `descoberta-am.md` (Fase 3).

## 14. Entidade `pa` (SEFA-PA) integrada — ✅ **resolvida (15ª entidade)**

Fonte = catálogo estadual **paradigital** (API Prodepa/Spring), escolhida na descoberta (`descoberta-pa.md`);
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
`guiadeservicos` do enunciado migraram/morreram (307 → portal.es.gov.br). Descoberta em `descoberta-es.md`.

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
`descoberta-ro.md`.

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
HTML-scraping (como BA/RJ), NÃO SPA/JSON. Descoberta em `descoberta-to.md`.

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
Descoberta em `descoberta-ma.md`.

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

Portal `www.sefaz.ap.gov.br` = **SPA Angular (FUSE)**. Descoberta em `descoberta-ap.md`.

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

Portal `sefaz.ac.gov.br` = **WordPress + Elementor** (HTML server-rendered). Descoberta em `descoberta-ac.md`.

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
Descoberta em `descoberta-df.md`.

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
AngularJS/IIS transacional. Descoberta em `descoberta-rn.md`.

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
`www.sefaz.pb.gov.br` é Joomla). Descoberta em `descoberta-pb.md`.

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
Abertos", sem auth). Descoberta/validação em `descoberta-AL.md`.

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

## D-NAMING (pendência separada — MG, NÃO é do GO)

Política da frota: separador sigla–UF sempre `-`. Normalizar o `orgao` do **MG** `"SEF/MG"` →
`"SEF-MG"` em [`auli-scraper-mg/src/mg.rs:222`](auli-server/crates/scrapers/auli-scraper-mg/src/mg.rs#L222)
(o `registry.toml` já usa `SEF-MG`; confirmado 148/148 no snapshot). Muda bytes do snapshot MG →
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
- **Prompts `data/prompts/*.txt` — aberto (nota da auditoria #14):** os prompts que de fato rodam
  divergem no marcador de serviço (`rs.txt` tem `## servico`; `sc/pr/sp.txt` só `## pergunta`).
  Alinhar muda o comportamento do LLM ao vivo — decisão à parte.
- **Comentário histórico:** [derive_faqs.rs:29](auli-server/crates/auli-collections/src/derive_faqs.rs#L29)
  cita `EmbedStrategy::QuestionKey` (tipo já removido) — referência de lineage, cosmética.
- **Formato de links/slugs não uniforme — aceito (não corrigir por ora):** FAQs do RS emitem
  `[texto](url)`; os serviços (RS/SC/PR/SP/MG) emitem `texto "url"`. O `linkify.tsx` do frontend
  linkifica URLs cruas nos dois casos, então funciona, mas o texto enviado ao LLM não é homogêneo.
  Idem os slugs de servidores: `servicos-a-servidores-publicos` (RS/SC) vs `servicos-a-servidores`
  (SP). Uniformizar re-gera todos os packs (muda o texto embarcado / o espaço vetorial) sem ganho
  funcional — **documentado como dívida aceita**.
