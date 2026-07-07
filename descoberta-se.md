# Descoberta — SEFAZ-SE (Sergipe), 26ª entidade

## Fonte
- Portal: `https://www.sefaz.se.gov.br/` = **SharePoint 2013** on-prem (MicrosoftSharePointTeamServices
  15.0; molde do PE). O REST `_api` é anônimo (parcial), mas o catálogo real é uma **página HTML**.
- Menu **SERVIÇOS → CARTAS DE SERVIÇOS** (`manuais_servicos.aspx`) aponta para o catálogo do cidadão:
  **`https://www.sefaz.se.gov.br/SitePages/servicos_cidadao.aspx`** (única página, ~890 KB).
- (Becos sem saída: "Servicos Importantes" = lista SP de só 12 atalhos; "Biblioteca de Servicos" = library
  de PDFs/ZIPs por tema — arquivos, não texto; `servicos_empresa.aspx` = 404. Tudo isso NÃO é a Carta.)

## `servicos_cidadao.aspx` — a Carta rica
Página única, **Bootstrap accordion**. **91 serviços**, cada um um painel:
- **Título** no heading: `<a href="#{id}">▾ Título</a>` (o `id` é a âncora/identidade do serviço).
- **Corpo** em `<div class="panel-collapse collapse" id="{id}"><div class="panel-body">` com campos
  `<p><strong>Rótulo:</strong> valor</p>`: Descrição do serviço, Legislação vinculada, Área responsável,
  Subsecretaria responsável, Requisitos exigidos, Onde solicitar, Forma de prestação, Canais de
  relacionamento, etc. Corpo típico ~900 chars (mediana), rico.

## Classe (tema)
Os serviços estão agrupados em 7 accordions-tema (`id="accordion_<tema>"`): DFe, ICMS, ITCMD, IPVA,
Simples Nacional, Contencioso, Cadastro de Contribuinte. Os 9 serviços "standalone" (Plantão Fiscal,
Consultas Tributárias, …) vêm ANTES do 1º tema → `classe = tema imediatamente anterior; senão "Serviços
Gerais"`. Distribuição: Cadastro 33, ICMS 17, IPVA 10, Serviços Gerais 9, Contencioso 8, Simples 6,
DFe 4, ITCMD 4 (= 91).

## Modelagem (molde PB/AC, rico, 1 GET)
- `titulo` = texto do heading (sem o "▾"); `descricao` = texto do `panel-body` (corte no próximo
  `panel-heading` p/ evitar vazamento; cap ~2500 chars — 1 painel do ITCMD tem ~22 KB de formulário).
- **público único "Serviços"** (a Carta é geral, cobre PF e PJ dentro dos requisitos — não há eixo de
  audiência por serviço); `classe` = tema; `link` = `servicos_cidadao.aspx#{id}` (identidade, único).

## Rede / TLS
- Apache/SharePoint, HTTPS padrão, UA AuliBot aceito. Sem gotcha de fingerprint (ureq funciona).

## Decisão
Catálogo genuinamente rico (91 fichas, 1 página) → parse HTML direto, sem headless, sem detalhe por
serviço. Público único; classe por tema.
