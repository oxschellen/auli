# DISCOVERY-TARF — Registro das descobertas

**Data:** 08/08/2026
**Status:** Discovery concluído. Nenhuma implementação iniciada — documento apenas para registro.
**Objetivo futuro:** nova tabela TARF no Auli (entidade no padrão `Table<P>`), com acórdãos do Tribunal Administrativo de Recursos Fiscais do RS.

---

## 1. O portal

- URL de pesquisa usada como ponto de partida:
  `https://tarf.sefaz.rs.gov.br/pesquisa?termo=&areas=&grupos=14543&assuntos=&dataInicio=&dataFim=`
- O site é uma **Angular SPA** chamada internamente **"Legislacao 3.0"** (`<app-root>`, bundles webpack, título da página "Legislacao 3.0").
- É um sistema multiárea: o mesmo backend serve as áreas **cage**, **receita**, **tarf** e **tesouro** (rotas `/areas/{area}` no bundle). Isso sugere que o mesmo scraper pode, no futuro, cobrir outras áreas da legislação da SEFAZ-RS com custo marginal baixo.
- `robots.txt` do portal bloqueia acesso automatizado genérico (o fetch por ferramenta externa foi recusado); todo o discovery foi feito via `curl` na máquina local, garimpando o `main.bundle.js` (o bundle está pretty-printed, o que facilitou muito a leitura dos services Angular gerados por OpenAPI).

## 2. Método do discovery

1. `curl` na URL de pesquisa → HTML de SPA vazio → confirmou que os dados vêm por XHR.
2. Download de `main.bundle.js` e greps sucessivos para mapear os services do `LegislacaoAPIClient` e suas rotas `/api/...`.
3. Leitura do corpo dos services (assinaturas + montagem de `HttpParams`) para extrair nomes exatos e capitalização dos parâmetros.
4. Testes de fumaça com `curl` direto nos endpoints.

## 3. Contrato da API (confirmado por teste)

### 3.1 Listagem — `GET /api/PesquisaDocumentos`

Parâmetros (query string):

| Parâmetro | Obs |
|---|---|
| `TamanhoPagina` | obrigatório |
| `NumeroPagina` | obrigatório, começa em 1 |
| `grupos` | **minúsculo**, repetível (`grupos=A&grupos=B`) — é array |
| `areas` | minúsculo, repetível — é array |
| `assuntos` | minúsculo, repetível — é array |
| `NumeroDoc` | opcional |
| `Termos` | opcional |
| `DataInicio` / `DataFim` | opcionais, a SPA envia `.toISOString()` |

Teste de fumaça:

```
GET /api/PesquisaDocumentos?TamanhoPagina=5&NumeroPagina=1&grupos=14543
```

Shape da resposta:

```json
{
  "totalItens": 22590,
  "numeroPagina": 1,
  "tamanhoPagina": 5,
  "resultado": [
    {
      "titulo": "ACÓRDÃO 510/24",
      "grupo": "ACÓRDÃOS A PARTIR DE 2005",
      "grupoId": "b750f7f4-2628-4d5e-a669-1621a14e41d8",
      "area": "TARF",
      "numero": 51024,
      "dataDocumento": "2026-07-17T14:18:52",
      "ementa": "",
      "id": "ee87f4bc-12b9-4666-02cb-08dee4034157",
      "dataCriacao": "2026-07-17T09:59:36.5331339",
      "dataAtualizacao": "2026-07-17T18:06:51.2050432"
    }
  ],
  "totalPaginas": 4518,
  "temPaginaPosterior": true,
  "numeroPaginaPosterior": 2,
  "numeroPaginaAnterior": 1
}
```

Fatos relevantes:

- **Grupo 14543 = "ACÓRDÃOS A PARTIR DE 2005"**, com **22.590 itens** na data do discovery (grupoId GUID `b750f7f4-2628-4d5e-a669-1621a14e41d8` aparece no item; o filtro da query usa o id numérico 14543).
- A `ementa` vem **vazia na listagem** — a ementa real está no corpo do documento (ver 3.3).
- `numero` é o número do acórdão achatado (510/24 → `51024`).
- A listagem parece vir ordenada por recência de criação/atualização (itens de datas recentes primeiro), mas isso não foi verificado formalmente.

### 3.2 Metadados de um documento — `GET /api/Documentos/{id}`

Devolve só os metadados (mesmos campos da listagem + `assuntos: []`). **Não contém o corpo.**

### 3.3 Corpo do documento — `GET /api/documentos/{id}/DocumentoHtml`

- Parâmetros obrigatórios: `NumeroPagina` (≥1) e `TamanhoPagina` (**entre 10 e 4000** — validado pelo servidor, mensagem de erro explícita).
- O endpoint pagina os **dispositivos** do documento; resposta no mesmo envelope da listagem (`totalItens`, `resultado[]`).
- Cada item de `resultado[]` tem `id` (GUID do dispositivo) e `texto` (HTML).
- No acórdão testado (510/24), `totalItens=1`: o acórdão inteiro veio num único dispositivo.

Conteúdo do `texto` (HTML): cabeçalho em `<table>` com campos rotulados —

- ACÓRDÃO Nº (510/24)
- RECURSO Nº (125/24)
- RECORRENTE(S) — inclui nomes de empresa e de pessoas físicas (sócios)
- RECORRIDA(s) (FAZENDA ESTADUAL)
- PROCESSO Nº (23/1404-0025519-8)
- AUTO DE LANÇAMENTO Nº
- DECISÃO DE 1ª INSTÂNCIA Nº
- EMENTA — a ementa completa em CAIXA ALTA + parágrafos de fundamentação resumida na sequência

Seguido do corpo/fundamentação do acórdão.

### 3.4 Alternativas mapeadas (não testadas)

- `GET /api/documentos/{id}/DocumentoPdf`
- `GET /api/documentos/{id}/informacao`
- `GET /api/IndiceDocumento/{...}` e `GET /api/MenuDocumento/{...}`
- `GET /api/Assuntos`, `/api/Assuntos/GetByArea/{...}`, `/api/Grupos` — úteis se quisermos enumerar grupos/assuntos dinamicamente em vez de fixar 14543.
- `/api/PesquisaConteudoDocumentos` (+ `/GetTotalSearchRecords`): é a **busca de termo dentro de um documento aberto** (recebe `IdDocumento`), não a listagem — descartado para o scraper.

## 4. Implicações para o futuro scraper (anotações, sem decisão)

- **Melhor cenário da doutrina**: JSON API limpa de ponta a ponta, sem HTML server-rendered para parsear na listagem e **zero headless**.
- **Identidade de dedup natural**: `id` (GUID) do documento.
- **Invariante de guarda pronto**: `totalItens` da listagem (`uniques coletados == totalItens`), sempre dinâmico.
- **Atenção — mutabilidade**: a existência de `dataAtualizacao` (e ela ser posterior à `dataCriacao` nos exemplos) sugere que acórdãos **podem ser atualizados** após publicação — diferente da doutrina append-only dos pareceres. A estratégia incremental precisará decidir: (a) só diff de conjuntos por `id` (ignora atualizações), ou (b) diff + re-fetch de quem tiver `dataAtualizacao` mais nova que a do snapshot.
- **Atenção — dispositivos múltiplos**: não assumir `totalItens=1` no `DocumentoHtml`; se um documento tiver mais de um dispositivo/página, o scraper deve detectar (assert/paginação), nunca truncar silenciosamente.
- **Conteúdo com dados pessoais**: o corpo traz nomes de recorrentes (empresas e pessoas físicas), números de processo e de auto de lançamento — são documentos públicos do portal, mas é bom ter isso em mente ao pensar em sinopse LLM e exposição no frontend/chat.
- **Rate limiting e cache**: valem as regras do kit (`AuliBot/0.1`, ≥1s entre requisições, cache agressivo). Com 22.590 documentos, a carga inicial completa a 1 req/s leva ~6h15 só para os corpos — planejar como job resumível com cache.

## 5. Comandos de referência (reproduzir o discovery)

```bash
# baixar o bundle
curl -s 'https://tarf.sefaz.rs.gov.br/main.bundle.js' -o /tmp/tarf-main.js

# listagem (teste de fumaça)
curl -s 'https://tarf.sefaz.rs.gov.br/api/PesquisaDocumentos?TamanhoPagina=5&NumeroPagina=1&grupos=14543'

# metadados de um documento
curl -s 'https://tarf.sefaz.rs.gov.br/api/Documentos/ee87f4bc-12b9-4666-02cb-08dee4034157'

# corpo (HTML por dispositivos)
curl -s 'https://tarf.sefaz.rs.gov.br/api/documentos/ee87f4bc-12b9-4666-02cb-08dee4034157/DocumentoHtml?NumeroPagina=1&TamanhoPagina=4000'
```

## 6. Pendências em aberto (para quando o assunto voltar)

1. Fechar o recorte da entidade: kind (`acordaos-tarf`?), campos do frontmatter, árvore .md, sinopse LLM sim/não.
2. Decidir a estratégia incremental frente à mutabilidade (`dataAtualizacao`).
3. Verificar se há acórdãos com múltiplos dispositivos no `DocumentoHtml`.
4. Avaliar se outros grupos/áreas do "Legislacao 3.0" interessam (súmulas? outros grupos do TARF? enumerar via `/api/Grupos`).
