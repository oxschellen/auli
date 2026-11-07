# ESTUDO-BA-Faqs — FAQs da SEFAZ-BA (coleção `faqs` da entidade `ba`)

**Data:** 2026-11-07 (sondagem direta, UA AuliBot, cortesia de ~1 s, ~120 GETs). · **Estado:** decidido (07/11:
D-BAFAQ-1 A; 2–5 como propostas) e implementado (§7). · **Órgão:** Secretaria da Fazenda do Estado da Bahia (SEFAZ-BA). · **Hoje a `ba` tem só `servicos`**
(Carta de Serviços do `portal.sefaz.ba.gov.br` — [REGISTRO-scrapers.md#ba](REGISTRO-scrapers.md#ba)). Destravada pela
D-FROTA-ROBOTS: o `portal` tem `robots.txt` com `Disallow: /`.

> **TL;DR — a BA tem a maior FAQ que falta, mas quase toda antiga e em três formatos.**
>
> As páginas de ICMS, IPVA, ITD e Taxas do site novo (`www.sefaz.ba.gov.br`, WordPress) linkam o **"Perguntas e
> Respostas" do Plantão Fiscal**, que continua no portal antigo (`portal.sefaz.ba.gov.br`, HTML estático em latin1,
> servidor só com cifras TLS 1.2 CBC). É um frameset com 4 menus: **Inspetoria** (15 páginas HTML com perguntas e 5
> PDFs), **Legislação** (13 PDFs), **Contencioso** (1 página) e **Finanças Públicas** (24 páginas, para órgãos e
> fornecedores do Estado). No site novo há mais três: as "Dúvidas Frequentes" do Balcão Virtual (27, de 2026), o
> Cadastro Sincronizado (18) e um PDF da taxa de incêndio (12).
>
> São **~830 perguntas** sem as Finanças Públicas (~1.035 com elas). O conteúdo é na maior parte de 2000–2020: o PDF de
> alíquotas (2020) diz que a modal é 18% — é 20,5% desde 07/02/2024 (Lei 14.629/2023). O servidor dá a todas as páginas
> antigas a mesma data (27/08/2023, uma migração); a data real só existe nos PDFs.
>
> Recomendação: **o Plantão Fiscal para contribuintes e as três do site novo, com a data na trilha quando há** (opção A,
> ~830); Finanças Públicas fora; da NFC-e, só a versão 1.6.

---

## 1. O que foi avaliado (07/11/2026)

Todas as sondagens com `AuliBot/0.1 (+https://github.com/oxschellen/auli)`, pelo `curl`, ~1 s entre requisições. O
AuliBot passa nos dois hosts (os serviços da BA ainda usam o UA de navegador, por legado).

| Fonte / rota | Veredito | Evidência |
|---|---|---|
| `www.sefaz.ba.gov.br/default/perguntas_respostas/perguntas_respostas.htm` (o endereço da triagem) | **404** | o `www` virou WordPress (`/wp-json` aberto); TLS 1.3, leaf Sectigo R36 sem o intermediário (o mesmo caso do AC) |
| `www.sefaz.ba.gov.br/robots.txt` | só `/wp-admin/` fechado | — |
| Páginas ICMS, IPVA, ITD e Taxas do site novo | **linkam o Plantão Fiscal no `portal`** | "Perguntas e Respostas" → `portal.sefaz.ba.gov.br/default/perguntas_respostas/perguntas_respostas.htm` |
| **`portal.sefaz.ba.gov.br/default/perguntas_respostas/`** | **a FAQ** | frameset; o `menu.htm` tem 4 `<select>` de navegação (as opções são as URLs das páginas) |
| `portal.sefaz.ba.gov.br/robots.txt` | `Disallow: /` | D-FROTA-ROBOTS: não barra |
| Busca da API do WordPress ("perguntas", "dúvidas", "faq") | notícias e páginas; 3 com perguntas próprias | Balcão Virtual, Cadastro Sincronizado, o PDF da taxa de incêndio |
| "Perguntas Frequentes EFD" | FAQ federal (`sped.rfb.gov.br`) | fora |

## 2. O Plantão Fiscal (`portal.sefaz.ba.gov.br`)

| Menu | Itens | Perguntas (aprox.) | Formato | Idade |
|---|---|---|---|---|
| **Inspetoria** — HTML | 15 páginas com perguntas (Baixa de Inscrição, Certidões, Compra Confirmada, DAE, DIC-e, DPD, GNRE, IEF, IPVA, ITD, NF-e, SEPD, SENF, SINTEGRA, TRGM); a TEF diz "Conteúdo em Atualização", e o SIMPLES NACIONAL dá 404 | **~378** (NF-e 136, SINTEGRA 69, DPD 31, IPVA 25, Compra Confirmada 24…) | 3 marcações: a pergunta numa célula de tabela em negrito; em parágrafos em negrito; e a NF-e em seções expansíveis ("I. Conceito, uso e obrigatoriedade…") | referências de 2000–2011; ITD com 2024–2025 (alterada em 21/05/2025), IPVA com 2022 |
| **Inspetoria** — PDF | AIDF, DSTDA, DTE, NFC-e (2 versões) | AIDF 19, DSTDA ~26, DTE 18, NFC-e v1.6 ~45 (v1.1 ~61) | "1. Pergunta?" ou "1- Pergunta… Resposta:" | 2017; NFC-e 1.1 de 07/2015 e **1.6 de 05/2019** |
| **Legislação** — PDF | 13: Alíquota, Antecipação Parcial, Base de Cálculo, Cadastro, Débito e Crédito, Diferença de Alíquota, Documentos Fiscais/Escrituração, Isenção/Benefícios, Não Incidência/Suspensão, Resolução 13/12 do Senado, Substituição Tributária, Taxas, Outros Assuntos | **~273** | "1. Pergunta?" e a resposta recuada | criados em 04/2020; o de alíquota cita a modal de 18% (de 2016) |
| **Contencioso** — HTML | "dúvidas CONSEF" | 10 | célula em negrito | — |
| **Finanças Públicas** — HTML | 24 páginas (Empenho, SIGAP, FUNDEB, Convênios, Fluxo de Caixa, Retenções…) | ~207 | célula ou parágrafo em negrito | para órgãos do Estado, municípios e fornecedores; 2000–2010 |

- **As páginas são HTML estático em latin1** (`iso-8859-1`), diagramadas com tabelas. O `kit::linhas` trata uma tabela
  como uma linha só (para manter tabelas inteiras nas respostas); aqui a página inteira é tabela. Saída: desmontar as
  tabelas de layout antes de segmentar, e a NF-e por seção.
- **Perguntas em negrito sem "?"** na DPD (2) e no SINTEGRA (4), numeradas: a regra do `kit::linhas` (a linha em negrito
  que continua a numeração abre pergunta) cobre.
- **Os PDFs** têm camada de texto (`pdftotext -layout`, como o PI). A pergunta é a linha na margem que começa pelo número
  seguinte; há recuo variável ("  9."), saltos (a NFC-e 1.6 pula a 6) e listas numeradas dentro das respostas (DSTDA):
  a segmentação precisa tolerar recuo pequeno e salto, com aviso.
- **Data:** o servidor dá `Last-Modified` 27/08/2023 a quase tudo (migração), menos o ITD (21/05/2025) e o Portal das
  Prefeituras (2024). Os PDFs têm a data de criação (e a NFC-e, a da capa).
- **TLS:** o `portal` só negocia TLS 1.2 com cifras CBC — o scraper de serviços da BA já usa o `native-tls` por isso.

## 3. O site novo (`www.sefaz.ba.gov.br`, WordPress)

| Página | Perguntas | Formato | Idade |
|---|---|---|---|
| **Balcão Virtual** → "Dúvidas Frequentes (FAQS)" (`#duvidasfrequentes`) | **27**, em 3 grupos (atendimento por videoconferência, Balcão de ITD, Plantão Fiscal) — várias perguntas se repetem entre os grupos com a resposta do canal | acordeão do Elementor | página alterada em 07/2026 |
| Cadastro Sincronizado | 18 | negrito | conteúdo da implantação (2005), página alterada em 01/2026 |
| PDF "Perguntas e respostas — nova taxa de incêndio" | 12 | "1. PERGUNTA?" / "RESP." em caixa alta | 03/2014 |

## 4. Modelagem proposta

| `FaqRaw` | Plantão Fiscal | Site novo |
|---|---|---|
| `pergunta` | a linha em negrito (HTML) ou numerada (PDF) | o título do acordeão / o negrito / a linha numerada |
| `resposta` | até a próxima pergunta (`kit::html`) | idem |
| `origin` (trilha) | "Plantão Fiscal \| <menu> \| <página>" (+ a seção na NF-e), com a data quando há ("(abr/2020)", "(Versão 1.6, 23/05/2019)") | "<página> \| <grupo>" |
| `url` | a página HTML (sem âncora) ou o PDF com `#page=N` | a página (o Balcão com `#duvidasfrequentes`) ou o PDF |

Guardas: cada página e PDF com ao menos uma pergunta (a TEF fica de fora pela lista, não por exceção); pergunta sem
resposta é erro; sequência de numeração com aviso; piso por bloco.

## 5. Decisões propostas (D-BAFAQ-*)

- **D-BAFAQ-1 — Escopo.**
  - **A — o Plantão Fiscal para contribuintes (Inspetoria, Legislação, Contencioso) e as três do site novo: ~830.**
  - **B — A + Finanças Públicas: ~1.035.**
  - **C — só o HTML** (Inspetoria, Contencioso, Balcão Virtual, Cadastro Sincronizado), sem PDF: ~433. Deixa de fora os
    PDFs de legislação (onde está a alíquota defasada), mas também as perguntas de ICMS mais procuradas (antecipação
    parcial, ST, DIFAL).
  - **D — só o recente** (Balcão Virtual, ITD, IPVA): ~64.

  Recomendo **A**: é a FAQ que a SEFAZ-BA publica e linka no site novo, e a data na trilha (D-BAFAQ-2) avisa o leitor
  quando o texto é antigo. As Finanças Públicas são para órgãos do Estado e fornecedores, fora do público do Auli (como o
  Portal do Servidor no AM).
- **D-BAFAQ-2 — Data na trilha** quando há uma confiável: a do PDF (criação, ou a da capa) e a do ITD; as páginas HTML
  antigas ficam sem data (a do servidor é a da migração). Como o "(até 2022)" do GO e o "(Novembro de 2021)" do PI.
- **D-BAFAQ-3 — Segmentação.** HTML: as tabelas de layout viram blocos e o `kit::linhas` segmenta; a NF-e, por seção.
  PDF: a linha na margem que começa pelo número seguinte (recuo de até 3, salto com aviso). Balcão Virtual: o título do
  acordeão.
- **D-BAFAQ-4 — NFC-e:** só a versão 1.6 (2019); a 1.1 (2015) está superada e continua no menu.
- **D-BAFAQ-5 — Transporte.** `portal`: `native-tls` (o do `scrape.rs`), latin1, `robots` ignorado (D-FROTA-ROBOTS).
  `www`: o R36 como âncora (o mesmo do AC). PDFs pelo `pdftotext` (como o PI). UA AuliBot; ~1 s entre GETs; sempre à
  rede, cache só com `--usecache`. ~40 GETs por rodada (17 páginas do portal com o menu, 18 PDFs, 2 páginas do site novo).

## 6. Reprodução

```sh
UA='AuliBot/0.1 (+https://github.com/oxschellen/auli)'; P=https://portal.sefaz.ba.gov.br/default/perguntas_respostas
curl -sS -A "$UA" "$P/menu.htm" | iconv -f latin1 -t utf-8 | grep -o '<option[^>]*value="[^"]*"[^>]*>[^<]*'
curl -sS -A "$UA" "$P/perguntas_inspetoria_ipva.htm" | iconv -f latin1 -t utf-8 | grep -o '<strong>[^<]*?</strong>'
curl -sS -A "$UA" -o aliquota.pdf https://portal.sefaz.ba.gov.br/contribuinte/tributacao/duvidas/aliquota.pdf && pdftotext -layout aliquota.pdf - | head
# o site novo: o leaf é do Sectigo R36 sem o intermediário (o PEM está no auli-scraper-ac)
curl -sS --cacert r36.pem -A "$UA" "https://www.sefaz.ba.gov.br/wp-json/wp/v2/pages/13458" | jq -r .content.rendered | grep -c elementor-tab-title
```

## 7. Implementação (07/11/2026)

`auli-scraper-ba faqs`; `scripts/tools/update-ba-faqs.sh`; detalhes em
[REGISTRO-scrapers.md#ba-faqs](REGISTRO-scrapers.md#ba-faqs). A modelagem é a da §4, com estes ajustes:

- **Data (D-BAFAQ-2).** PDFs: o mês da criação (`pdfinfo`), "Alíquota (abr/2020)" — a NFC-e fica "(mai/2019)", o mês da
  capa (23/05/2019), e não "(Versão 1.6, 23/05/2019)". Páginas HTML: o `Last-Modified`, só quando é posterior à
  migração (08/2023) — hoje, só o ITD ("ITD (mai/2025)"); uma página que a SEFAZ editar passa a mostrar a data sozinha.
- **Segmentação (D-BAFAQ-3)** — o que a fonte pediu além do previsto: as páginas são windows-1252 rotulado de latin1; a
  pergunta vem também em `<span>` de classe CSS negrita e com só o número em negrito; no SINTEGRA, os subtítulos
  "Registro Tipo NN – …" viram o grupo das perguntas seguintes, e saem o "topo ˆ", o índice e o rodapé. Três regras
  novas no `kit::linhas` (a nota entre parênteses depois do "?" não conta; o número sozinho em negrito é item da
  resposta; o número sem separador, "49 Deve…?", vale na pergunta marcada que continua a numeração), com o PA, a PB, o
  AC, o RO e o PI idênticos à produção. Nos PDFs, o número seguido de dígito ("13.780/12") não abre pergunta, o romano
  em caixa alta só abre seção seguido da pergunta esperada, e o inciso e a alínea no começo da linha abrem parágrafo.
- **`kit::pdf`** (novo): o `pdftotext` do PI subiu para o kit e ganhou o `pdfinfo` (a data de criação). O PI, coletado
  pela rede com o kit, ficou idêntico à produção (134).

**Resultado da rodada de 07/11.** **830** perguntas (a estimativa era ~830): Inspetoria 383 em HTML e 106 em PDF,
Legislação 275, Contencioso 10, site novo 56. Contra a §2–3: AIDF 18 (est. 19), NFC-e 44 (vai até a 45 e não tem a 6 —
aviso), taxa de incêndio 11 (o PDF tem 11), NF-e 140 (est. 136). As irregularidades de numeração restantes são da fonte
e ficam: DPD 10.1/10.2, SINTEGRA 18.1, uma seção da NF-e sem a 14 e com duas 15. 37 GETs. `--usecache` reproduz a árvore
byte a byte e o snapshot a menos do `coletado_em`. Smoke (`/v1/retrieve`, faqs): o alvo em 1º em 5 de 6 (SINTEGRA 64 e
49, taxa de incêndio, ITD, Balcão Virtual); no AIDF, em 2º, atrás da pergunta vizinha que dá a mesma resposta.

**Testes.** 22 no `faqs.rs` (fontes reais: o menu, o SINTEGRA, o texto de 3 PDFs, o Cadastro Sincronizado; sintéticos:
a NF-e, o Balcão Virtual, as guardas) e 3 no `kit::linhas`; 40 mutações (cada regra e cada guarda), todas pegas. Uma
condição que nenhum dado exercitava e que não mudava o resultado saiu (a seção do PDF dentro de uma pergunta aberta).
