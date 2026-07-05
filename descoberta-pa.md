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
