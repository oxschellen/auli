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
