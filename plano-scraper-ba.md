# Plano: auli-scraper-ba (SEFAZ-BA)

**Status**: discovery concluído com HTML real (listagem + ficha `senha`, capturas de 2026-07-04)
**Entidade**: `ba`
**Fonte**: Carta de Serviços ao Cidadão — `https://portal.sefaz.ba.gov.br/scripts/cartadeservicos/index.asp`

---

## 1. Achados do discovery

| Aspecto | Achado |
|---|---|
| Stack | ASP clássico + Bootstrap 3, **server-rendered** (zero JS de conteúdo) |
| Listagem | Página única, `ul#search_list`, **204 serviços ativos** (206 hrefs no fonte, 2 comentados/desativados), grupos A–V, sem paginação, zero duplicatas |
| Padrão de link | Único: `index.asp?id=<slug>` (nenhum href externo na listagem) |
| Ficha | `section#content`: `panel-title` (**público**: "Serviços aos Cidadãos"), `.title-page h3` (título + `<small>` = **classe**, ex.: "Requerimento"), parágrafo(s) de introdução, blocos `div.media-service` (`h4.media-heading` + `div.media-content`): Documentos Necessários, Como Fazer, Canal, Tempo Médio, Base Legal |
| Charset | `<meta charset="utf-8">`; guarda de runtime no fetch mesmo assim (ASP clássico já traiu muita gente) |
| robots.txt | Restritivo a crawlers genéricos (fetch automatizado do Claude recusado) — mesma etiqueta do PE |
| Fonte alternativa | `www.sefaz.ba.gov.br/carta-de-servicos/` (site novo) lista os mesmos serviços linkando para as fichas ASP — redundante, ignorar |

## 2. Arquitetura

Padrão **PR completo** (listagem + fichas de detalhe), ureq + `scraper`, sem headless.

- Binário `auli-scraper-ba` (kit + contract, schema v3)
- **Fase única**: 1 GET na listagem + 204 GETs de ficha (cortesia 500ms ≈ 2min; cache em disco)
- Identidade: `(link, titulo)` — titulo da **listagem** (canônico); ficha fornece público/classe/corpo
- Link canônico: `https://portal.sefaz.ba.gov.br/scripts/cartadeservicos/index.asp?id=<slug>`
- `Ocorrencia { publico, classe }`:
  - `publico`: do `panel-title` da ficha, mapeado ("Serviços aos Cidadãos" → Cidadãos, "Serviços às Empresas" → Empresas); rótulo desconhecido → slugificado com warning (D-BA1)
  - `classe`: do `<small>` do título da ficha (ex.: "Requerimento"); ausente → "Geral" (D-BA2)
- `descricao`: introdução + seções `Heading:\ncorpo` com links normalizados `texto "url"` (helpers do PR)
- Ficha que falhar no fetch: item mantido com público "Cidadãos" + classe "Geral" + corpo vazio, com warning (D-BA3)
- `publicos_ordem`: ordem de primeira aparição dos públicos na coleta

## 3. Decisões (D-BA)

- **D-BA1** público por ficha com mapa de rótulos conhecidos + fallback slugificado.
- **D-BA2** classe = subtítulo `<small>` da ficha (taxonomia do portal, não inventada).
- **D-BA3** falha de ficha degrada com warning, não derruba a coleta.
- **D-BA4** etiqueta: UA de navegador, 500ms de cortesia, cache — robots restritivo, coleta de
  baixíssima frequência (mesma justificativa do PE/PR).

## 4. Verificação

- Fixtures REAIS: listagem completa (206 itens) + ficha `senha`, capturadas via view-source.
- Testes offline: contagem 204, unicidade, canônico absoluto, público/classe/corpo da ficha.
- Real-scrape no desktop: snapshot com 204 itens; amostrar 3 fichas contra o site.
