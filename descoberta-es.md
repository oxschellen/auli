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
