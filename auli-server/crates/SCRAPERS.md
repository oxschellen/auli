# Scrapers por entidade (`auli-scraper-<id>`)

Referência das 9 implementações de scraper da frota Auli. Cada crate é um **binário por
entidade** que raspa o catálogo de serviços (e, no RS, também as FAQs) de uma SEFAZ estadual e
grava um **snapshot v3** que o `auli-collections` deriva em artefatos e o `auli update` vetoriza.

Fonte da verdade das entidades: [`data/registry.toml`](../../data/registry.toml). Este doc
descreve o *como* de cada scraper; a lista de entidades vive lá.

> Última atualização: 2026-07-04 (frota com 9 entidades; CE = a mais recente, PR #16).

---

## 1. Arquitetura comum

Todos os scrapers seguem o mesmo esqueleto:

- **Um crate binário por entidade** (D-F2.1). Cada um conhece só o seu `ENTITY` e **não lê o
  registry** — a entidade é hard-coded no crate.
- **Sem navegador headless.** Nenhum crate depende de `headless_chrome` (o RS já usou, hoje não).
  Doutrina vigente: *discovery-first; API JSON > HTML server-side; navegador nunca*.
- Dependem de **`auli-contract`** (tipos do snapshot) + **`auli-scraper-kit`** (agent `ureq`,
  cache em disco, agregação, I/O do snapshot) — **nunca** de `fastembed`/`ort`/vector-store.
- **CLI uniforme:** `auli-scraper-<id> [--usecache] servicos` (RS também aceita `faqs`).
- **Saída:** `data/<id>/<id>-servicos-snapshot.json` (schema v3), gravado por
  `kit::snapshot::write_servicos`. Cache de páginas/respostas em `data/<id>/raw/cache/` (gitignored).
- **Pipeline downstream:** `auli-collections <id>` (deriva `raw/*.json` + `.txt`) →
  `scripts/build-packs.sh <id>` (vetoriza BGE-M3) → `scripts/build-frontend-public.sh <id>`.

### Modelo de dados (contrato v3)

Um serviço = um `ServicoRaw { titulo, descricao, link, orgao, ocorrencias[] }`. Cada
`Ocorrencia { publico, classe }` registra onde o serviço aparece no portal. Um serviço listado
sob vários públicos/classes tem **uma ocorrência por listagem** (o schema v2 nativo — não se perde
o caso multi-classe).

### Duas estratégias de identidade

| Estratégia | Como | Quem usa |
|---|---|---|
| **`aggregate_servicos` (kit)** | dedup **por `link`**; monta `Servico` per-público e agrega | sc, pr, mg, pe, ba |
| **`ServicoRaw` direto** | o crate monta os registros; o link **não** é a chave única | rs, sp, rj, ce |

O `ServicoRaw` direto existe porque em alguns portais o link não identifica: SP (vários serviços
compartilham a URL de login), RJ (identidade `(link, titulo)`), CE (identidade `_id`; slug não é
único).

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

Contagens de serviços = snapshot atual em `main`. Total de testes da frota: **54** (todos os crates cobertos).

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

---

## 5. Checklist de integração de uma nova entidade

1. Crate em `auli-server/crates/auli-scraper-<id>/` + adicionar aos `members` do
   [`auli-server/Cargo.toml`](Cargo.toml).
2. `cargo test -p auli-scraper-<id>` — o gate verde de verdade.
3. **Registrar em [`data/registry.toml`](../../data/registry.toml)** (bloco `[[entities]]`) +
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
