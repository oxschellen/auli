# ESTUDO-PE-Faqs — Perguntas e Respostas da SEFAZ-PE (coleção `faqs` da entidade `pe`)

**Data:** 2026-11-06 (sondagem direta, UA AuliBot). Substitui o rascunho de 05/11, que leu as páginas mas não
testou a `_api`, o `robots.txt` nem o scraper de serviços (§7). · **Estado:** **v1 implementada — duas páginas de
publicação do SharePoint, 59 perguntas** (§8). · **Órgão:** Secretaria da Fazenda de Pernambuco (SEFAZ-PE). · **Hoje
a `pe` tem só `servicos`** (menu global da home, 1 GET por rodada, D-PE1).

> **TL;DR — duas FAQs tributárias vivas, em HTML server-side, sem API.**
>
> **IPVA** (`/Servicos/IPVA/Paginas/Perguntas-e-Respostas.aspx`, 40 perguntas) e **ITCMD**
> (`/Servicos/ICD/Paginas/Perguntas-e-Respostas.aspx`, 19), ambas atualizadas para 2025/2026 (EC 137/2025, LC
> 563/2025). Cada página é UMA página de publicação do SharePoint 2013 com o corpo num único campo rico
> (`…_RichHtmlField`); as perguntas são parágrafos numerados ("1. ", "5.1 - "). A `_api` e o `listdata.svc` dão 403
> (página "Acesso Negado"), então é HTML server-side — 2 GETs. O `robots.txt` (gerido pela Cloudflare) libera `*` com
> *content signals* `search=yes, ai-train=no, use=reference`: o Auli não treina e responde citando o link oficial.
>
> Fora da v1: os ~86 Informativos Fiscais em PDF (o acervo grande de orientação do PE, outra escala e outro gênero),
> a página do TeleSefaz (orientação de balcão), as rejeições de NF-e de 2018–2019 e as páginas de PERC.

---

## 1. Hierarquia discovery-first — o que foi avaliado

| Fonte / rota | Veredito | Evidência (06/11/2026) |
|---|---|---|
| `_api/web`, `Servicos/IPVA/_api/web/lists`, `_api/web/GetFileByServerRelativeUrl(…)/ListItemAllFields` | **fechada** — 403 com a página "403 - Acesso Negado" do portal | GET com `Accept: application/json;odata=verbose` |
| `Servicos/IPVA/_vti_bin/listdata.svc` | **fechada** — 403 | GET |
| **IPVA — Perguntas e Respostas** `/Servicos/IPVA/Paginas/Perguntas-e-Respostas.aspx` | **fonte v1** — 878 KB de página, corpo de 181 KB, 40 perguntas (1–39 + 5.1) | GET (`ureq`/`curl`, 200) |
| **ITCMD — Perguntas e Respostas** `/Servicos/ICD/Paginas/Perguntas-e-Respostas.aspx` | **fonte v1** — 746 KB de página, corpo de 52 KB, 19 perguntas (1–17 + 11.1, 11.2) | GET |
| NF-e — Erros/Rejeições mais Comuns `/Servicos/nota-fiscal-eletronica/Paginas/Noticias.aspx` | **fora** (D-PEFAQ-1) — ~10 pares rejeição→solução, 7,5 mil caracteres, NT 2018.005 e erros de Java 8, sem nenhuma "?" | GET |
| PERC ICD 2025 / "Programa Dívida Zero 2.0" `/Servicos/ICD/Paginas/perc2025icd.aspx` | **fora** — procedimento de adesão (prazo 01/07 a 26/12/2025), não P&R | GET |
| DeSTDA `/Servicos/SEDIF/Paginas/Informativos-Perguntas-e-Respostas.aspx` | **fora** — biblioteca de documentos com 1 PDF de 30/01/2017 | GET |
| TeleSefaz `/Servicos/Paginas/TeleSefaz.aspx` e o resumo `Telesefaz-resumo.aspx` | **fora** (D-PEFAQ-3) — orientação de balcão / canais de atendimento | GET (resumo); rascunho de 05/11 (página cheia, 264 mil caracteres) |
| Informativos Fiscais (lista em `/Publicacoes/Paginas/Novo-regulamento-do-ICMS.aspx`) | **fora da v1** (D-PEFAQ-2) — ~86 PDFs, itens numerados, alguns "Perguntas e Respostas" explícitos | rascunho de 05/11 (não recontado) |
| Transparência Ativa — Perguntas Frequentes | **fora** — 10 respostas da Ouvidoria de 2008 (Folio Views, implantação do e-Fisco) | rascunho de 05/11 |
| Headless | desnecessário — tudo vem no HTML | — |

**`robots.txt`** (`www.sefaz.pe.gov.br`, "Cloudflare Managed content"): `User-agent: *` → `Allow: /` com
`Content-Signal: search=yes,ai-train=no,use=reference` (`ai-input` sem sinal: "nem concede nem restringe"). Seguem
`Disallow: /` para ~35 robôs nomeados (GPTBot, ClaudeBot, CCBot, Google-Extended…); o AuliBot não está entre eles. A
D-PE4 do scraper de serviços ("robots restritivo a crawlers genéricos", UA de navegador) descreve um arquivo que não é
mais este. Ver D-PEFAQ-5.

**Sobreposição com `servicos`:** nenhuma. O scraper de serviços só lê o menu global; os itens dele apontam para as
páginas de entrada `/Servicos/IPVA` e `/Servicos/ICD`, sem descrição — as páginas de P&R não estão no acervo.

## 2. Modelagem v1

Uma lista curada de 2 páginas (molde PA/PR): sem API, não há como enumerar as bibliotecas "Páginas", e o total é uma
página de FAQ por tributo. Para cada uma: GET → o único `div[id$="_RichHtmlField"].ms-rtestate-field` da página
(guarda: exatamente um) → texto em linhas → perguntas.

**Texto.** Árvore DOM (`scraper`/html5ever — o HTML traz `&#58;` 2,6 mil vezes, `&#160;`, `&quot;`, que o
`decode_entities` do kit não cobre) percorrida com quebra de linha nos blocos (`p`, `div`, `li`, `br`, `blockquote`,
`tr`…), células de tabela separadas por ` | ` e link como `texto "url"` (54 dos 81 links do IPVA são relativos →
absolutizados com o host; o ITCMD não tem nenhum `<a>`, as URLs vêm escritas no texto). `U+200B` (em rajadas no IPVA)
e nbsp saem pelo `clean`.

**Tabelas são resposta.** IPVA: 8 tabelas — 7 caixas "IMPORTANTE" de uma célula e o calendário de vencimentos (6 × 11);
ITCMD: as duas tabelas de alíquotas (a partir de 01/01/2026; de 01/01/2016 a 31/12/2025). Uma linha por `<tr>`, células com ` | `,
vazias fora. Sem tabela, a resposta da 10 do ITCMD ("Qual a alíquota?") ficaria vazia de conteúdo.

**Onde começa cada pergunta.** Bloco cujo texto começa por um número que **continua a sequência**:

- `N. ` (ponto + espaço) com N = última + 1 → pergunta de 1º nível;
- `N.M - ` (ou `N.M. `) com N = a atual, M = último subitem + 1 **e** texto terminado em "?" → pergunta própria
  (IPVA 5.1; ITCMD 11.1, 11.2). Os 4.1/4.2 do ITCMD ("Na capital…: Formalizar o pedido…") são partes da resposta 4 e
  ficam nela.

As listas numeradas dentro das respostas recomeçam em 1 (IPVA: 14 blocos; ITCMD: 2) e nunca alcançam a próxima
pergunta. O espaço depois do ponto é obrigatório: na tabela do ITCMD, "19.600,00" e "34.600,00" casariam "19." e
"34." sem ele. **Guarda:** número de 1º nível acima do esperado (a numeração pulou) é `Err` — hoje 0.

**Onde termina.** Na próxima pergunta, ou no `<hr>`: o IPVA fecha com um `<hr>` seguido do rodapé da Gerência de IPVA
("O órgão que normatiza… Estamos prontos a lhe prestar serviços"), 10 blocos que não são resposta da 39. O que vem
antes da 1ª pergunta também sai (ITCMD: 1 bloco, "Alterações trazidas no ANEXO 6 da LC 563/2025…").

| `FaqRaw` | origem |
|---|---|
| `pergunta` | o bloco da pergunta, como na página — com o número ("29. Como fica o pagamento…"), porque as respostas se referem umas às outras por ele ("verifique o item 29"); ITCMD em caixa alta, mantida. Duas do IPVA têm o enunciado em mais de um bloco (38 "…desde 20XX OU" / "…como sucata." / "O que fazer?"; 39 "… é bronze." / "O que fazer?"): o título é o 1º bloco, o resto abre a resposta. |
| `resposta` | os blocos até a próxima pergunta ou o `<hr>`, uma linha por bloco |
| `origin` | `IPVA` ou `ITCMD` — o tributo; as perguntas não têm subdivisão |
| `url` | a página; com `#N` quando a página tem a âncora `<a name="N">` (16 das 39 de 1º nível do IPVA; o ITCMD não tem nenhuma) |

**Números (06/11/2026):** IPVA 40, ITCMD 19 — 59. Resposta: IPVA mín. 73 caracteres, mediana 883, máx. 3.810; ITCMD
250 / 846 / 1.844; 0 vazias.

**Guardas (`Err`):** página sem o campo rico ou com mais de um; nenhuma pergunta na página; numeração que pula;
resposta vazia (a próxima pergunta colada na anterior é sinal de parse errado); título repetido na página (com o mesmo
link, o `.md` teria o mesmo nome). A coleta das duas páginas roda antes de gravar qualquer coisa.

## 3. Fora do escopo v1

- **Informativos Fiscais** — ver D-PEFAQ-2.
- **TeleSefaz** — ver D-PEFAQ-3.
- **NF-e, PERC, DeSTDA, Transparência** — §1.

## 4. Decisões

- **D-PEFAQ-1 — Escopo v1 = IPVA + ITCMD.** São as duas FAQs tributárias do portal, vivas e no formato P&R. A página
  de rejeições da NF-e é operacional e útil, mas é de 2018–2019, mistura erros de Java 8 com rejeições ainda válidas e
  pede um parser próprio — fica registrada como candidata.
- **D-PEFAQ-2 — Informativos Fiscais fora da v1.** São a fonte oficial de orientação do PE (as próprias FAQs remetem a
  eles item por item), mas são ~86 PDFs com versões sobrepostas por vigência e milhares de itens — outra escala e um
  parser de PDF. Merecem um estudo próprio.
- **D-PEFAQ-3 — TeleSefaz fora.** É orientação de balcão em tópicos (o "como fazer" de cada serviço), não P&R; o lugar
  natural é uma fase 2 do scraper de serviços.
- **D-PEFAQ-4 — Tabelas na resposta**, uma linha por `<tr>` com ` | ` entre as células (§2).
- **D-PEFAQ-5 — UA AuliBot** (D-FROTA-UA), 1 s entre os 2 GETs. O `robots.txt` atual libera `*`; os *content signals*
  pedem `ai-train=no` (o Auli não treina) e `use=reference` (o Auli responde com o link oficial). A D-PE4 dos serviços
  (UA de navegador) não muda aqui.
- **D-PEFAQ-6 — Subcomando** `auli-scraper-pe faqs` / `all`, sempre à rede, cache só com `--usecache`, como as FAQs da
  frota.

## 5. Plano de implementação

1. `auli-scraper-pe/src/faqs.rs`; `main.rs` aceita `servicos | faqs | all`. → 2 GETs por rodada.
2. Extração do campo, texto em linhas (DOM), segmentação por sequência, corte no `<hr>`, link com âncora, guardas. →
   testes sobre o corpo real das duas páginas (o ITCMD inteiro; um recorte do IPVA com tabelas, âncoras, subitem,
   listas numeradas nas respostas, as perguntas 38–39 e o rodapé); cada regra desligada uma a uma tem de fazer o teste
   dela falhar.
3. Registry + `entities.ts` (pelo gerador); `scripts/tools/update-pe-faqs.sh`; REGISTRO#pe-faqs, SCRAPERS.md, README.
4. `auli-collections pe` → 59 `.md` com nomes únicos, artefatos de serviços idênticos; `build-packs.sh pe faqs`; smoke
   `/v1/retrieve`.

## 6. Reprodução

```sh
UA='AuliBot/0.1 (+https://github.com/oxschellen/auli)'; H=https://www.sefaz.pe.gov.br
curl -sS -A "$UA" $H/robots.txt | grep -A2 '^User-agent: \*'
curl -sS -A "$UA" -o /dev/null -w '%{http_code}\n' "$H/_api/web"                       # 403
curl -sS -A "$UA" -o ipva.html "$H/Servicos/IPVA/Paginas/Perguntas-e-Respostas.aspx"     # 200, ~878 KB
curl -sS -A "$UA" -o icd.html  "$H/Servicos/ICD/Paginas/Perguntas-e-Respostas.aspx"      # 200, ~746 KB
grep -c '_ControlWrapper_RichHtmlField" class="ms-rtestate-field' ipva.html icd.html     # 1 e 1
```

## 7. O que o rascunho de 05/11 dizia e o que a sondagem mostrou

| rascunho | 06/11 |
|---|---|
| IPVA "39 + 3 subitens (5.1, 11.1, 11.2)" e "uma pergunta sem número (… roubado → item 29)" | IPVA tem 39 + 5.1; 11.1 e 11.2 são do ITCMD; a "pergunta sem número" é uma nota "OBS:" dentro de uma resposta |
| ITCMD "17 + 2" | confere (11.1, 11.2); os 4.1/4.2 são partes da resposta 4 |
| `_api` "provavelmente responde, não testado" | 403, e o `listdata.svc` também |
| robots: "o fetch passou, não há bloqueio" | o arquivo existe (Cloudflare): libera `*` com *content signals*; bloqueia ~35 robôs nomeados |
| D-PEFAQ-4: "conferir se o PE-serviços já colhe A/B" | não colhe: o scraper de serviços só lê o menu |
| corpo em `PublishingPageContent` | no HTML o campo é o `div` `…_RichHtmlField` (o nome interno não aparece) |
| v1 recomendada: IPVA + ITCMD + NF-e | IPVA + ITCMD; a NF-e fica como candidata (D-PEFAQ-1) |

## 8. Implementação (06/11/2026)

`auli-scraper-pe/src/faqs.rs` (`auli-scraper-pe faqs`, ou `all`). A `pe` ganha `faqs` no registry e no `entities.ts`
(pelo gerador); `scripts/tools/update-pe-faqs.sh`; REGISTRO#pe-faqs, SCRAPERS.md, README.

**Rodada de 06/11:** 2 GETs (`ureq`, ~880 KB e ~746 KB) → IPVA 40, ITCMD 19 = 59 perguntas; 10 linhas de rodapé do
IPVA e 1 nota do ITCMD fora; 0 respostas vazias; no texto, 0 tags, 0 entidades, 0 zero-width/nbsp. 16 perguntas com
link próprio (`#N`). Resposta: mín. 73 caracteres, mediana 873, máx. 3.922.

**Reprodução:** `--usecache` refaz a árvore idêntica byte a byte e o snapshot idêntico a menos do `coletado_em`.
`auli-collections pe` → 59 `.md` com nomes únicos; os 43 artefatos de serviços (raw, docs, packs) saíram com o mesmo
md5. `build-packs.sh pe faqs` → 59 vetores; o servidor sobe com `pe-faqs — 59 registros`.

**Smoke `/v1/retrieve`** (`entity=pe`, `kind=faqs`, 5 perguntas escritas à mão, em linguagem de contribuinte): o alvo
em 1º nas 5 — vencimento pela placa → 5; alíquota de herança de R$ 500 mil → 10 do ITCMD; cobrança de carro vendido →
38; IPVA pago duas vezes → 22; parcelar o imposto de herança → 11.2 do ITCMD.

**Testes:** 20 novos sobre o corpo real (o ITCMD inteiro e o recorte 1–6 + rodapé do IPVA em
`tests/fixtures/pe-faqs-*.html`). Cada regra — a sequência de 1º nível, o subitem e o "?" dele, as guardas de pulo,
repetição, resposta vazia, campo ausente ou duplicado, página vazia e par repetido, o corte no `<hr>` e o reinício dele
na pergunta seguinte, a âncora só onde existe, o link absoluto, a URL que não se repete, a tabela em uma linha, o
espaço obrigatório depois do ponto e o `clean` — foi desligada uma a uma e o teste dela falhou.
