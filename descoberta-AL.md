# descoberta-AL.md — SEFAZ-AL (Secretaria de Estado da Fazenda de Alagoas)

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

```
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
