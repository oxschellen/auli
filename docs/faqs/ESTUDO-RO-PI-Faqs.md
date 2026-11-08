# ESTUDO-RO-PI-Faqs — FAQs da SEFIN-RO e da SEFAZ-PI (coleção `faqs` das entidades `ro` e `pi`)

**Data:** 2026-11-07 (sondagem direta, UA AuliBot, cortesia de 1 s, ~50 GETs). · **Estado:** decidido (07/11: as
recomendações da §5 — D-SYDFAQ-1 B, -2 A, -8 A, as demais como propostas) e implementado (§7). · **Por que juntos:** os dois portais são Sydle ONE da geração conecta-360 (D-XX-SYDLE-COMPARTILHADO):
token anônimo no HTML inicial, `GET …/_classId/5cd32901df14eb3d461160f0/_search?_body=<ES>`. **Hoje a `ro` e a `pi` têm
só `servicos`** (194 e 28 — [REGISTRO-scrapers.md#ro](../REGISTRO-scrapers.md#ro), [#pi](../REGISTRO-scrapers.md#pi)).

> **TL;DR — a FAQ dos dois está dentro de páginas que o acervo já conhece, cada uma num formato.**
>
> **RO:** não há página de FAQ. As perguntas são **seções das fichas de serviço** da Agência Virtual: depois das 6
> seções do modelo (O que é?, Legislação, Etapas e prazos, Onde e como solicitar, Documentação, Taxa), 57 fichas trazem
> seções cujo título é uma pergunta. São **212 pares, 193 sem as cópias** entre fichas irmãs, no mesmo molde das FAQs
> do CE (`section-wrapper`, `#section-<slug>`), e vêm inteiros no `_search` — **2 GETs**, sem `getItem` nem POST. Duas
> seções agrupam mais 20 perguntas, em formatos próprios. O serviço do RO usa só a `description` curta: nada disso está
> no acervo.
>
> **PI:** a árvore de temas "Informações sobre a legislação tributária | Perguntas Frequentes" existe, mas está **vazia**
> (8 pastas sem nenhum artigo, como o resto da taxonomia de temas criada em 03/2026). A FAQ real está nas **8 páginas de
> Documentos Eletrônicos** (NF-e, NFC-e, EFD, MDF-e, CT-e, DeSTDA, NFF, BP-e), cada uma com uma seção "Perguntas
> Frequentes": **99 perguntas**, a pergunta em negrito como na PB/PA — segmentação do `kit::linhas`, com dois reparos
> (links para o ambiente de homologação e o texto-modelo "Título da sanfona" do Elementor). **2 GETs.** E há um PDF, "Perguntas
> Frequentes EFD ICMS IPI" (nov/2021, **35 perguntas**), que o repositório de documentos do portal entrega a um GET anônimo
> (§3.1, D-SYDFAQ-8).
>
> Recomendação: RO = as 213 (193 + 20, D-SYDFAQ-1 B); PI = as 99 (D-SYDFAQ-2 A); o parser de seções do CE vai para o
> kit, com o RO como 2º consumidor (D-SYDFAQ-6).

---

## 1. O que foi avaliado (07/11/2026)

Todas as sondagens com `AuliBot/0.1 (+https://github.com/oxschellen/auli)`, pelo `curl`, ~1 s entre requisições. Os dois
hosts aceitam o AuliBot (o scraper de serviços do PI ainda usa UA de navegador por legado — pendência §11).

| Fonte / rota | Veredito | Evidência |
|---|---|---|
| **RO** `_search` da classe `5cd32901…` (`sydleone.sefin.ro.gov.br`, app `servicedesk-embedded`) | **a fonte** | 387 ativos: 224 artigos (`5d66ed07…`), 158 pastas (`5ca3bd5d…`), 5 catálogos. Os artigos vêm **com o `contentMarkdown` inteiro** no `_search` |
| RO catálogos | "Serviços" 193 artigos, "Temas" 41 pastas, "Conteúdos" 29, "Catálogo Modelo" 1, "Dê sua Sugestão" 1 | agregação por `parent._id` |
| RO `agenciavirtual…/perguntas-frequentes/` (o link "Perguntas Frequentes" do índice A-Z) | **casca da SPA**, igual a qualquer rota | 39.531 B, os mesmos `_classId` da home; as rotas numéricas antigas (`/18/informacoes-sobre-a-isencao…`) também |
| RO `faq.sefin.ro.gov.br/sefin_az.html` | **índice A-Z de 94 links** (serviços, legislação, sistemas), não FAQ | 20.544 B |
| RO `www.sefin.ro.gov.br` | sem FAQ | 231 KB, nenhum link com "pergunt", "faq" ou "dúvid" |
| RO pasta "Dúvidas sobre ICMS" (tema ICMS, 168 visitas) | **sem perguntas** | 1 artigo ligado ("ICMS"), sem seção-pergunta; a seção "Dúvidas" é o texto padrão de contato |
| RO artigo "Dúvidas sobre Indeferimento da Opção pelo Simples Nacional" | **passo a passo**, não FAQ | uma seção só ("O que é?"), com "1. Consulte o motivo do indeferimento…" |
| **PI** `_search` da classe `5cd32901…` (`portal.sefaz.pi.gov.br`, app `govPiPortalInstitucional`) | **a fonte** | 8.515 ativos: 8.295 artigos (6.645 legislações, 1.425 notícias…), 191 pastas, 29 catálogos |
| PI tema "Informações sobre a legislação tributária \| Perguntas Frequentes" (`69b9aa26…`) e suas 7 subpastas (ICMS, Agenda tributária, Regime Especial…) | **vazias** | nenhum filho (nem inativo) e nenhum artigo ligado (`linkedParents`, consulta `nested`) |
| PI pastas "Dúvidas gerais" (Nota Piauiense) e "Dúvidas gerais de IPVA" | **vazias** | idem |
| PI taxonomia "Temas e categorias - Portal Institucional" (`69653110…`) | 14 temas, 75 subtemas, **0 artigos** | só "API Legislação" e "LGPD → Atendimento ao Titular" recebem artigos ligados |
| PI notícia "Sefaz publica documento de perguntas frequentes sobre EFD ICMS IPI" | aponta para um **PDF** num endereço antigo (`documentoseletronicos/arquivos/…`), que hoje redireciona para a SPA; o arquivo mudou para o repositório de documentos (§3.1) | e para `documentoseletronicos/portal/efd/documentos.php`, que redireciona para a pasta "Documentos Eletrônicos" da SPA |
| **PI pasta "Conteúdos - Institucional → Documentos Eletrônicos"** (`69d6979d…`) | **8 páginas com FAQ** | §3 |
| PI "Carta de Serviços" | sem FAQ | as seções com "?" são do modelo da ficha (O que é?, Como solicitar?, Onde solicitar?, Quem pode solicitar? — 20 de cada) |
| Headless | não | — |

**`linkedParents` é `nested`.** As fichas se ligam às pastas de tema por `linkedParents[].parent._id`, que só responde à
consulta `{"nested": {"path": "linkedParents", …}}` (o `term` direto dá 0). O `parent` direto é o catálogo.

## 2. RO — as seções-pergunta das fichas

O molde é o das FAQs do CE: cada seção é `<div class="section-wrapper">` com `<div class="heading" id="section-<slug>">`, o
título em `<h2>` ou numa linha `## `, e a resposta no `<sy-collapse trigger-by="#section-<slug>">`. 219 dos 224 artigos
usam o molde (201 em MARKDOWN, 22 em HTML). As 6 seções da ficha: O que é? 176, Legislação 165, Etapas e prazos 164, Onde e
como solicitar 163, Documentação 156, Taxa 155.

| O quê | Medida |
|---|---|
| Seções-pergunta (título termina em "?", fora as 6 da ficha) | **212**, em **57 artigos** (55 em "Serviços", 2 em "Conteúdos"), todos MARKDOWN |
| Maiores | Protocolar defesa/recurso no TATE 21, Nota Fiscal Fácil 16, Petição ao núcleo de cadastro 15, Contribuinte Legal 15, Notificação ITCD 13, Cartão Cidade 11, Isenção do ICMS para PcD 11 |
| Cópias | 19 pares idênticos em fichas irmãs (ex.: "Preciso fazer uma defesa referente a um auto de infração, porém ele não está disponível…" em 7) → **193** |
| Mesma pergunta, outra resposta | 3 títulos: "Quando o benefício pode ser utilizado?" (2 anos no taxista, 4 no PcD), "O acesso ao E-PAT pode ser feito por procuração?", "Onde consulto a situação da Inscrição Estadual?" — ficam todos (a ficha é o contexto) |
| Resposta | 15 a 3.665 caracteres (mediana 351); a mais curta é "Download do PDF" (um link). 56 são a última seção do artigo e arrastam o `<script>` do fim, que o conversor tira |
| Perguntas dentro de uma seção | "DFe-Download – Perguntas e respostas" (Arquivos XML): 10, em `<h3>1. …?</h3>` — o `kit::linhas` cobre. "Perguntas e Respostas" (Substituição Tributária - Convênios): 10, em "**1. Pergunta:**" / a pergunta no parágrafo seguinte / "**Resposta:**" — regra própria |
| Fora | as seções "Dúvidas" (5) e as de tema do Nota Legal (7): texto de contato e orientação, sem pergunta |
| Datas | `_lastUpdateDate` de 54 dos 57 artigos em 11/2026 — parece regravação em lote, não serve de sinal de atualidade |

Coleta: o HTML inicial (token) + **um `_search`** dos artigos ativos (`_class._id = 5d66ed07…`, `size` 500; 20,9 MB em 3 s: 14,5
MB só os 193 de "Serviços"). Não precisa de `getItem` nem de POST.

## 3. PI — as 8 páginas de Documentos Eletrônicos

As páginas são HTML livre (`contentHtml`, sem `section-wrapper`), divididas em `<section class="secao"><h3>…</h3><div
class="conteudo-html">`. A última seção é "Perguntas Frequentes <documento>".

| Página | Perguntas | Formato da pergunta |
|---|---|---|
| NFC-e | 23 | `<p><strong>…?</strong></p>` |
| NF-e | 20 | idem |
| EFD | 11 | idem |
| MDF-e | 10 | idem |
| CT-e | 10 | `<p><a href="https://portal-h.sefaz.pi.gov.br/…?q=13282">…?</a></p>` — **link, sem negrito** |
| DeSTDA | 10 | `<h4 … role="tab"><a href="https://portal-h…?q=13306">…?</a></h4>` (copiado de um Elementor) |
| NFF | 8 | `<p><strong>` |
| BP-e | 7 | `<p><strong>` |
| **Total** | **99** | |

- **O `kit::linhas` cobre 6 páginas e meia**: destaque = `strong`/`b`/`h1`–`h6`, e o link herda o destaque de quem o
  contém. O DeSTDA passa (`<h4>`); o CT-e não (`<p><a>`, sem negrito nem número — daria 0).
- **`portal-h` é o ambiente de homologação**: os 20 links (10 no CT-e, 10 no DeSTDA) apontam para a mesma busca
  (`?q=13282`, `?q=13306`). Não devem ir para o texto.
- **Duas perguntas corrompidas pelo texto-modelo do Elementor**: "Título da sanfoO que é a inutilização de número do
  CT-e?na" (o editor colou a pergunta no meio de "Título da sanfona") e "A declaração pode ser retificada?Título da
  sanfona" (DeSTDA). Sem reparo, as duas não terminam em "?" e grudam na resposta anterior.
- Duas linhas em negrito sem "?" na NFC-e ("A NFC-e substitui:", "Observação: …") ficam na resposta, como devem.
- Publicadas em 08–09/04/2026, alteradas em 26/06/2026. Link = `https://portal.sefaz.pi.gov.br/<friendlyUrl>` (como os
  serviços); a seção de FAQ não tem âncora.
- A DeSTDA: a PB tirou a dela porque deixou de exigir a declaração. A página do PI a dá como obrigatória (Ajuste SINIEF
  12/2015), sem menção a dispensa.

Coleta: o HTML inicial (token) + **um `_search`** dos artigos ligados à pasta (`nested` em `linkedParents`, 2,6 MB).

### 3.1 O PDF "Perguntas Frequentes EFD ICMS IPI"

A página da EFD leva a "Legislação e Documentos EFD" (`/documentos?folderId=69d900af…`), uma pasta do repositório de
documentos do portal com 5 subpastas: 1 e 2. Regras de pós-validação (EFD e OIE), 3. Guia Prático da EFD (7 versões), **4.
Perguntas Frequentes** (1 arquivo) e 5. Esclarecimentos técnicos (2 arquivos: saldo credor, pendência de integridade). Só a
4 é FAQ; as outras são manuais e regras.

| O quê | Evidência |
|---|---|
| Listagem | o componente `portal-lib-folder-browser` chama `obterPastaComItens` da classe `68e40169df43e345a988b6dd`. **GET anônimo**: `…/govPiPortalInstitucional/_classId/68e40169df43e345a988b6dd/obterPastaComItens?_body={"folderIdentifier":"legislacao-e-documentos-efd-4-perguntas-frequentes"}` → `items[]` com `title`, `fileName`, `fileUrl`, `lastUpdateDate`. As classes de arquivo do Sydle (`000…0e`/`0f`) dão 403 ao anônimo; este método não |
| Arquivo | "Perguntas Frequentes EFD ICMS IPI – Versão 1.0.pdf", 425.638 B, `https://portal.sefaz.pi.gov.br/api/1/main/_ecm/_defaultFile/www/zu5a3chxym/` (GET anônimo, `application/pdf`). No portal, "Atualizado em 03/11/2026" é a data da carga; o PDF é de **22/11/2021** (Word, 21 páginas) |
| Texto | camada de texto completa; nenhuma figura além da capa e do logo de cada página. `pdftotext -layout` (poppler) separa bem: o parágrafo começa recuado e a continuação vem na margem; o marcador "▪" fica na linha do item; o rodapé ("Secretaria da Fazenda do Estado do Piauí" + nº da página, 20×) ocupa uma linha |
| Estrutura | sumário com 6 capítulos (Cadastro, Registros EFD, Multas, Regras de pós-validação, EPE e Portal EFD, Outros) e **35 perguntas** numeradas (1.1 … 6.2), com página; um subgrupo sem pergunta ("1.5 Certificado Digital"). No corpo, o mesmo número abre a pergunta — em 3 delas (1.5.2, 2.7, 4.9) separado por espaço não separável (U+00A0) |
| Atualidade | a 2.11 cita o "Decreto 13.500/2008, art. 565-A" e a 6.2 o "artigo 89 do RICMS" — o RICMS do PI hoje é o Decreto 21.866/2023. Dois links apontam para o portal antigo (`documentoseletronicos/…`, que redireciona) |

Seria o **primeiro PDF nas FAQs da frota** (o `faq.pdf` do SE, os PDFs de IPVA/ITCD do RN e o PDF federal da EFD no CE
ficaram fora) e a primeira dependência do poppler: o scraper chamaria o `pdftotext` como processo, como GO e DF chamam o
`curl`; os testes usam o texto extraído como fixture e não precisam dele.

## 4. Modelagem proposta

| `FaqRaw` | RO | PI |
|---|---|---|
| `pergunta` | o título da seção (`kit::html`) | a linha em destaque (`kit::linhas`) |
| `resposta` | o corpo da seção, pelo `kit::html` em modo markdown (como o CE) | as linhas até a próxima pergunta |
| `origin` (trilha) | o nome da ficha | o nome da página ("NF-e - Nota Fiscal Eletrônica") |
| `url` | a ficha + `#section-<slug>`: `https://agenciavirtual.sefin.ro.gov.br/catalogo-servicos+<identifier>+<_id>#section-…` (`catalogo-conteudos+…` nas 2 de "Conteúdos") | a página |

Árvore: raiz → um nó por ficha/página → as perguntas (molde CE/AM).

**Guardas** (erro antes de gravar): o `_search` cobre o total (`hits.total` = itens lidos, abaixo do `size`); âncoras =
blocos (a do CE); pergunta ou resposta vazia; par (url, pergunta) repetido; piso (RO 180, PI 90). No PI, também: cada
uma das 8 páginas com a seção de FAQ e ≥ 1 pergunta; "sanfo" ou "portal-h" no texto final. **Aviso** (não erro) no PI:
uma das pastas de FAQ vazias (§1) passou a ter artigo — é a hora de rever o escopo.

## 5. Decisões propostas (D-SYDFAQ-*)

- **D-SYDFAQ-1 — Escopo do RO.**
  - **A — as seções-pergunta das fichas: 193.**
  - **B — A + as 20 de dentro de 2 seções: 213.** O DFe-Download sai pelo `kit::linhas`; a Substituição Tributária
    precisa de uma regra para os rótulos "Pergunta:"/"Resposta:".
  - **C — não coletar.**

  Recomendo **B**: as 20 são FAQ de verdade (as perguntas básicas da ST estão lá), e o custo é uma regra de poucas
  linhas. **A** é a alternativa mais simples.
- **D-SYDFAQ-2 — Escopo do PI.** **A — as 8 páginas, 99 perguntas** (recomendada); **B — sem a DeSTDA, 89**; **C — não
  coletar.** A página da DeSTDA não fala em dispensa, ao contrário do caso da PB. O PDF da FAQ da EFD é a D-SYDFAQ-8.
- **D-SYDFAQ-3 — Cópias (RO).** O par idêntico (pergunta e resposta, sem contar espaços) fica uma vez, na ficha de menor
  `_id` (D-CEFAQ-5). A mesma pergunta com outra resposta fica em cada ficha.
- **D-SYDFAQ-4 — Link e trilha** como na §4: no RO, a ficha com a âncora da seção; no PI, a página.
- **D-SYDFAQ-5 — Reparos no PI**, antes da segmentação: os links para o `portal-h` saem e o texto deles vira destaque
  (cobre a pergunta do CT-e); o texto-modelo "Título da sanfona" sai das 2 perguntas. Sem mexer no `kit::linhas` (a PB e
  a PA ficam como estão).
- **D-SYDFAQ-6 — Parser de seções Sydle no kit.** O `secao()` das FAQs do CE (âncora, `<h2>`/`## `, `<sy-collapse>`) vai
  para o kit (`kit::sydle`), com o RO como 2º consumidor; o CE passa a chamá-lo e sai byte a byte igual. O token e o
  `_search` de cada crate ficam onde estão — as FAQs reusam as funções dos serviços do próprio crate (D-XX-SYDLE-
  COMPARTILHADO continua pendente).
- **D-SYDFAQ-7 — Transporte.** UA AuliBot (`crate::UA`) nos dois; 1 s entre os GETs; `ureq`/rustls (cadeias completas);
  sempre à rede, cache só com `--usecache`. 2 GETs por estado.
- **D-SYDFAQ-8 — O PDF da FAQ da EFD (PI).**
  - **A — incluir: +35, o PI vai a 134.** O arquivo é achado pela pasta (`obterPastaComItens`), não por endereço fixo:
    exatamente um PDF na pasta 4, ou erro (a SEFAZ costuma deixar as versões antigas ao lado, como no Guia Prático — uma
    versão nova pede revisão). Texto pelo `pdftotext -layout`; pergunta = o número do sumário até o "?"; o subgrupo sem
    "?" vai para a trilha; o rodapé sai; as linhas do parágrafo se juntam. Guarda: as perguntas do corpo são as 35 do
    sumário. Trilha "Perguntas Frequentes EFD ICMS IPI – Versão 1.0 (nov/2021) | Capítulo", com a data à vista, como o
    "(até 2022)" das FAQs do GO; link = o PDF com `#page=<página do sumário>`. 2 GETs a mais.
  - **B — deixar fora**, pela regra da frota para PDF.

  Recomendo **A**: é a FAQ que a SEFAZ publica hoje para a EFD, as 35 têm texto inteiro, e o arquivo é achado sem
  endereço fixo. O custo é a 1ª dependência de PDF na frota e duas respostas que citam o RICMS antigo — a data na trilha
  avisa o leitor.

## 6. Reprodução

```sh
UA='AuliBot/0.1 (+https://github.com/oxschellen/auli)'
q() { python3 -c 'import sys,urllib.parse;print(urllib.parse.quote(sys.argv[1],safe=""))' "$1"; }
# RO: token + os artigos
TOK=$(curl -sS -A "$UA" https://agenciavirtual.sefin.ro.gov.br/ | grep -o '"Authorization":"Bearer [^"]*' | cut -d'"' -f4)
curl -sS -A "$UA" -H "Authorization: $TOK" "https://sydleone.sefin.ro.gov.br/api/1/servicedesk-embedded/_classId/5cd32901df14eb3d461160f0/_search?_body=$(q '{"query":{"bool":{"must":[{"term":{"active":true}},{"term":{"_class._id":"5d66ed07e7c98a556a499a36"}}]}},"size":500}')" -o ro.json
# PI: token + as 8 páginas de Documentos Eletrônicos
TOK=$(curl -sS -A "$UA" https://portal.sefaz.pi.gov.br/ | grep -o '"Authorization":"Bearer [^"]*' | cut -d'"' -f4)
curl -sS -A "$UA" -H "Authorization: $TOK" "https://portal.sefaz.pi.gov.br/api/1/govPiPortalInstitucional/_classId/5cd32901df14eb3d461160f0/_search?_body=$(q '{"query":{"bool":{"must":[{"term":{"active":true}},{"nested":{"path":"linkedParents","query":{"term":{"linkedParents.parent._id":"69d6979dc41c7a09017fa749"}}}}]}},"size":20}')" -o pi.json
# PI: a pasta "4. Perguntas Frequentes" do repositório de documentos → o PDF
curl -sS -A "$UA" -H "Authorization: $TOK" "https://portal.sefaz.pi.gov.br/api/1/govPiPortalInstitucional/_classId/68e40169df43e345a988b6dd/obterPastaComItens?_body=$(q '{"folderIdentifier":"legislacao-e-documentos-efd-4-perguntas-frequentes"}')"
curl -sS -A "$UA" -o faq-efd.pdf https://portal.sefaz.pi.gov.br/api/1/main/_ecm/_defaultFile/www/zu5a3chxym/ && pdftotext -layout faq-efd.pdf -
```

## 7. Implementação (07/11/2026)

`auli-scraper-ro faqs` e `auli-scraper-pi faqs`; `scripts/tools/update-{ro,pi}-faqs.sh`. O que mudou em relação ao
estudo:

| | estudo | implementação |
|---|---|---|
| RO | 213 (193 + 20) | **211 de 46 fichas** (191 + 20): as cópias são comparadas sem contar espaços e quebras (D-SYDFAQ-3, como o CE), e "O acesso ao E-PAT pode ser feito por procuração?" tem 2 versões, não 4 — saem 21 cópias, não 19. As 11 fichas que só tinham cópias somem da árvore |
| RO, transporte | 1 `_search` de 20,9 MB | o `ureq` para em 10 MB por padrão: o `GetOpts` do kit ganhou o `limite_bytes` (64 MB nas FAQs do RO). O mesmo `_search` traz os 5 catálogos, de onde sai o prefixo do link (`catalogo-servicos`/`catalogo-conteudos`) |
| RO, artigos | MARKDOWN com seções | igual, e o filtro importa: a "Unidade Padrão Fiscal - UPF" é HTML com blocos sem título, que o parser estrito recusaria |
| PI, páginas | 99 | 99 |
| PI, PDF | 35 | 35. O sumário tem pontilhado de um ponto em três entradas (1.4, 1.5.4, 2.1) — a 1ª versão do parser leu 32 sem erro, e por isso entrou a guarda da numeração contínua do sumário. A guarda "perguntas do corpo = do sumário" saiu: por construção, as duas contas nunca diferem |
| PI, vigia | aviso nas pastas de FAQ vazias | um `_search` de contagem (size 0) nas 10 pastas (o tema, os 7 subtemas e as 2 "Dúvidas gerais"); 0 em 07/11 (a mesma consulta na pasta de Documentos Eletrônicos dá 8) |

**Kit.** `kit::sydle::secoes` é o `secao()` das FAQs do CE, que passa a chamá-lo: a árvore e o snapshot do CE saíram
idênticos (2.706 perguntas, `--usecache` antes e depois). O `Secao` traz também o corpo cru, para as perguntas de dentro
de uma seção (RO).

**Resultado da rodada de 07/11.** RO: 211 perguntas, 46 fichas (resposta de 30 a 3.490 caracteres, mediana 316), 210
links em `catalogo-servicos` e 1 em `catalogo-conteudos`. PI: 134 — NFC-e 23, NF-e 20, EFD 11, MDF-e 10, CT-e 10, DeSTDA 10,
NFF 8, BP-e 7 e o PDF 35 (6 capítulos). Nenhuma tag, entidade, rodapé, "sanfona" ou `portal-h` no texto. `--usecache`
reproduz a árvore byte a byte e o snapshot a menos do `coletado_em`, nos dois. Smoke (`/v1/retrieve`, faqs): 7 de 8
perguntas com o alvo em 1º; na 8ª ("download de XML bloqueado"), as duas primeiras são perguntas vizinhas da mesma seção.

**Testes.** `kit::sydle` 6; RO 14 (sobre 13 documentos reais do `_search`: 2 catálogos, 11 artigos); PI 19 (sobre 4 das 8
páginas, a listagem da pasta e o texto do PDF, mais um PDF sintético mínimo). Mutação: 16 no RO, 23 no PI (cada regra e
cada guarda), 6 no `kit::sydle` — todas pegas.

**Aparte.** O AuliBot passa no PI (as sondagens e as FAQs usam ele); os serviços do PI seguem com o UA de navegador até
uma recoleta verificada (pendência §11).

