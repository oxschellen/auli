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
   paradigital/Prodepa — outra plataforma; ver `descoberta-pa.md`.)
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
