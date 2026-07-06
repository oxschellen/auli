# Relatório de descoberta — Carta de Serviços SEFAZ-TO (servicos.to.gov.br)

**Data:** 2026-07-05 · **Escopo:** descoberta + base para a implementação (18ª entidade). · **Órgão:**
Secretaria da Fazenda do Tocantins (SEFAZ-TO).

> **TL;DR — portal ASP.NET WebForms legado (HTML server-rendered, IIS 10), molde HTML-scraping (como
> BA/RJ), NÃO um SPA/JSON-API.** SEFAZ = órgão **`cod_empresa=37`**. Listagem (1 GET):
> `listar_servico.aspx?cod_empresa=37` → **45 serviços** (`card-servico-nome` + link
> `servico_detalhado.aspx?cod_assunto_documento_tipo={id}`). Detalhe rico por serviço (spans ASP.NET
> `lbl*` com id estável): descrição, público (`lblTipoRelacionamento`), categoria (`lblTxtServicoGrupo`),
> requisitos, documentos, taxa, prazo, legislação. **Cenário B** (público estruturado). Escopo escolhido:
> **descrição rica (45 detalhes)**.

---

## Fase 0 — robots

`servicos.to.gov.br/robots.txt` → **404** (IIS 10, sem robots). D-PA-ROBOTS cobre o TO como caso
preventivo (UA institucional AuliBot, cortesia entre GETs, nunca autenticar).

## Fase 1 — plataforma e listagem

- **ASP.NET WebForms / IIS 10** (páginas `.aspx`, jQuery 3.3.1 + Bootstrap admin theme, cookie
  `SERVICOS_SERVER_ID`). HTML server-rendered — **não** há JSON API nem SPA. Molde HTML-scraping.
- **Filtro por órgão:** SEFAZ = **`cod_empresa=37`** (achado no card "SECRETARIA DA FAZENDA / SEFAZ" da
  home). Contador dinâmico do card: **"45 serviços"** (guard).
- **Listagem completa (1 GET):** `https://servicos.to.gov.br/listar_servico.aspx?cod_empresa=37` →
  **45 serviços**, cada um: `<span class="card-servico-nome">{nome}</span>` +
  `<a href="servico_detalhado.aspx?cod_assunto_documento_tipo={id}">`. Nomes×links casam 1:1 (45/45).
  `detalhar_orgao.aspx` só mostra 4 (é a "vitrine"); a lista real é `listar_servico.aspx`.

## Fase 2 — enumeração

45 serviços, identidade = **`cod_assunto_documento_tipo`** (inteiro). Sem paginação (a lista vem
inteira). Contagem dinâmica = nº de links distintos na listagem (== contador "45"). Nunca hardcode.

## Fase 3 — payload de detalhe (`servico_detalhado.aspx?cod={id}`)

HTML server-rendered com o padrão gov.br "Carta de Serviços" (accordions). O conteúdo está em spans
ASP.NET com **id estável** (`ctl00_ContentPlaceHolder1_lbl*`) — mais robusto que parsear os accordions
aninhados. Mapa campo → snapshot:

| span (`lbl*`) | conteúdo | → snapshot |
|---|---|---|
| `lblTxtServico` | nome do serviço | `titulo` (ou o da listagem) |
| `lblTxtConceituacao` | "O que é" | `descricao` (lead) |
| `lblTxtRequisitoAcesso` | "Como solicitar" | `descricao` |
| `lblTxtDocumentacao` | "Documentos necessários" | `descricao` |
| `lblTituloTaxa` | "Custos e despesas envolvidas …" | `descricao` |
| `lblTituloPrazoExecutar` | "Prazo para conclusão …" | `descricao` |
| `lblTituloLegislacao` | "Legislação …" | `descricao` (opcional) |
| **`lblTipoRelacionamento`** | **público** ("Cidadão Empresa", "Órgão Público"…) | `publico` |
| **`lblTxtServicoGrupo`** | **categoria** ("Finanças, Impostos e Gestão Pública") | `classe` |
| `lblOrgaoGestor`/`lblSetorRecebedor` | órgão/setor (canal) | (contato, opcional) |

O HTML dos spans usa **entidades** (`&ccedil;`, `&eacute;`…) → decodificar (html5ever/`scraper`, lição
GO/ES). `descricao` rica = Conceituação + Como solicitar + Documentos + Custos + Prazo (seções não-vazias).

## Fase 4 — público e categoria (Cenário B)

- **Público = `lblTipoRelacionamento`** — vocabulário FIXO concatenado por espaço: observados
  **"Cidadão", "Empresa", "Cidadão Empresa", "Órgão Público"**. Parsear por vocabulário (longest-first:
  `Órgão Público` antes de `Cidadão`/`Empresa`) para não quebrar o "Órgão Público" (um valor, dois
  tokens). `ocorrencias` = público × classe.
- **Classe = `lblTxtServicoGrupo`** — a categoria; varia (maioria "Finanças, Impostos e Gestão Pública";
  há serviços cross-listados, ex. "Saúde e Vigilância Sanitária…"). Usar o valor cru (fidelidade),
  fallback "Geral".

## Fase 5 — destinos link-only

Muitos serviços apontam para os sistemas transacionais da própria SEFAZ-TO (`sefaz.to.gov.br`: CND,
DARE, etc.) e para `www.to.gov.br/sefaz` — **destinos link-only** citados no conteúdo. O `link` canônico
do snapshot é a página `servico_detalhado.aspx?cod={id}`; nunca autenticar nos sistemas destino.

## Pontos de decisão (candidatos a D-TO*)

1. **Escopo** — só SEFAZ (`cod_empresa=37`). O portal é **multi-órgão** (dezenas de `cod_empresa`) →
   3ª ocorrência da oportunidade "scraper estadual genérico" (D-PA-ACERVO), mas em ASP.NET/HTML, não
   JSON — parametrização menos direta. Registrar; não fazer agora.
2. **Robots** — coberto por D-PA-ROBOTS (TO = 3º caso; 404 = sem robots, sem bloqueio).
3. **Descrição rica** — escolhido: buscar os 45 detalhes (1 listagem + 45 GETs), montando a `descricao`
   das seções da Carta.
4. **Guard de contagem** — nº de links distintos na `listar_servico.aspx` (== "45"), dinâmico.

## Evidência bruta

Em `scratchpad/to/`: `home.html`, `lista37.html` (listagem SEFAZ), `det8017.html` (detalhe-modelo),
`sefaz37.html`. Endpoints e o mapa de `lbl*` acima.
