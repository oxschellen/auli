# ESTUDO-PA-Faqs — Perguntas e respostas da SEFA-PA (coleção `faqs` da entidade `pa`)

**Data:** 2026-11-07 (sondagem direta, UA AuliBot, cortesia de 1 s, ~50 GETs). · **Estado:** proposta v1, aguardando
decisão (§5). · **Órgão:** Secretaria de Estado da Fazenda do Pará (SEFA-PA). · **Hoje a `pa` tem só `servicos`**
(paradigital/Prodepa, 34 serviços — [REGISTRO-scrapers.md#pa](REGISTRO-scrapers.md#pa)).

> **TL;DR — a FAQ do PA está no site antigo, espalhada em 11 páginas, e é na maior parte de 2014–2018.**
>
> O site novo (`www.sefa.pa.gov.br`, Angular sobre um WordPress headless) não tem FAQ. O menu "Orientações" dele aponta
> para o **site antigo**, que continua no ar em `antigo.sefa.pa.gov.br` (Joomla) — o REGISTRO#pa o dava por extinto
> porque só testou o `www`. Lá, a página "Perguntas e Respostas" do Manual de Atendimento é só um índice de 30 links:
> 13 PDFs, 4 sites de outros órgãos, 5 itens sem link, 1 subsite fora do ar e 7 artigos, dos quais 5 têm perguntas. A
> busca do Joomla acha mais 6. São **11 páginas, ~152 perguntas** em HTML servido pronto, 11 GETs (~1 MB).
>
> A regra de segmentação da PB (linha toda em negrito terminada em "?") cobre 8 das 11. Nas outras 3 as perguntas são
> numeradas mas não estão em negrito: precisa de um complemento (§3). Com o PA, a regra passa a ter dois consumidores,
> e a proposta é levá-la para o kit (D-PAFAQ-3).
>
> A questão real é o escopo (D-PAFAQ-1): só 4 páginas foram criadas ou atualizadas de 2019 em diante (21 perguntas); a FAQ da DIEF 2016 (30) é
> sobre o programa daquela versão, e o Pará passou a dispensar a DIEF de quem entrega a EFD em 2023.

---

## 1. O que foi avaliado (07/11/2026)

Todas as sondagens com `AuliBot/0.1 (+https://github.com/oxschellen/auli)`, pelo `curl`, ~1 s entre requisições.

| Fonte / rota | Veredito | Evidência |
|---|---|---|
| **Site novo** `www.sefa.pa.gov.br` (Angular) | **sem FAQ** | rotas do bundle (`orientacoes`, `accordion-tables`, `internal-page`…); "Orientações" aponta para `http://antigo.sefa.pa.gov.br/orientacoes` |
| WordPress do site novo (`site-sefa-wordpress.sefa.pa.gov.br/wp-json/wp/v2`) | **sem FAQ** — `/search` e `/types` dão 401; `/posts`, `/pages` e `/pis` abrem | busca por "perguntas", "faq", "dúvidas": só notícias e o `pis` "FAQ" — que é a página de serviço do CADIN, com link para `app.sefa.pa.gov.br/cadin/faq` |
| `app.sefa.pa.gov.br/cadin/faq` | **bloqueia o AuliBot** | TLS ok (GlobalSign), mas "Empty reply from server"; não contornado com UA de navegador |
| **Site antigo** `antigo.sefa.pa.gov.br` (Joomla) | **a fonte** | `robots.txt` padrão Joomla + `/arquivos/`, `/legislacao/` e outros fechados; `/orientacoes`, `/noticias`, `/NN-…` abertos. TLS 1.3, cadeia completa |
| "Perguntas e Respostas" do Manual de Atendimento (`/orientacoes/27-orientacoes/manual-de-atendimento/11578-perguntas-e-respostas`, 14/01/2022) | **só um índice** | 30 itens: 13 PDFs, 4 de outros órgãos (GNRE de PE, portal nacional da NF-e, RECOPI de SP, Simples Nacional), 5 sem link, 1 subsite (`nfce.sefa.pa.gov.br`, timeout daqui), 7 artigos Joomla (5 com perguntas; os outros 2 são links para PDF) |
| Busca do Joomla antigo (`/component/search/?searchword=…&searchphrase=exact&limit=0`) | **acha mais 6** | "perguntas e respostas" 28, "perguntas frequentes" 8, "dúvidas frequentes" 7, "faq" 5; o resto são páginas de orientação que só linkam PDFs (MEI, ITCD, IPVA, DIFAL, PROREFIS, CT-e…) |
| Alíquota Modal 19% | **dois artigos idênticos** (ids 20715 e 20716) | o índice linka o 20716 por uma URL de edição (`/orientacoes?layout=edit&id=20716`) |
| Headless | desnecessário | o Joomla serve o HTML pronto |

**Achado à parte, para `servicos`:** o tipo `pis` do WordPress novo é o **catálogo de serviços do site novo** — 154
páginas (`X-WP-Total`), cada uma com blocos ACF ("Como solicitar este serviço", "Documentos necessários", "Base
legal"…). O acervo usa hoje os 34 do paradigital. Fica como pendência de serviços, fora deste estudo.

## 2. As 11 páginas

| Página (trilha) | Caminho (o link do título) | Criada | Ano mais recente citado | Marca da pergunta | Perguntas |
|---|---|---|---|---|---|
| Alíquota Modal 19% | `/orientacoes/27-orientacoes/manual-de-atendimento/20716-aliquota-modal` | 16/03/2023 | 2023 | negrito "01)" | 6 |
| PIX | `/27-orientacoes/manual-de-atendimento/21546-pix` | 16/03/2023 | — | numerada **sem negrito** "1." | 4 |
| Imunidades e Isenções | `/28-orientacoes/manual-de-atendimento/imunidades-e-isencoes/17525-duvidas-frequentes` | 08/04/2020 | 2019 | numerada **sem negrito** "1)" | 6 |
| Selo Fiscal de Controle e Qualidade (água) | `/38-orientacoes/18084-perguntas-e-respostas` | 21/05/2014 | 2021 | negrito, sem número | 5 |
| Parcelamento de débitos | `/noticias/13401-perguntas-frequentes-parcelamento-de-debitos` | 15/02/2018 | 2018 | numerada **sem negrito** "1.1 -" | 29 |
| DIEF 2016 | `/129-downloads/dief/11586-dief-2016-perguntas-e-respostas` | 05/02/2016 | 2018 | negrito "1)" | 30 |
| SINTEGRA | `/119-orientacoes/sintegra/1439-duvidas-frequentes` | 24/06/2014 | — | negrito "1 -" | 31 |
| Manifestação do Destinatário (NF-e) | `/orientacoes/115-orientacoes/sped/1575-faq-nfe` | 25/06/2014 | 2014 | negrito numerada | 20 |
| Denegação da NF-e | `/orientacoes/121-orientacoes/sped/nfe/1475-perguntas-respostas` | 24/06/2014 | 2014 | negrito numerada | 8 |
| Cruzamento EFD x NF-e | `/orientacoes/116-orientacoes/sped/efd/1427-efd-nfe` | 24/06/2014 | 2014 | negrito "1-" | 7 |
| EFD Estadual | `/orientacoes/116-orientacoes/sped/efd/1426-efd-estadual` | 24/06/2014 | 2014 | negrito, sem número | 6 |
| **total** | | | | | **152** |

"Criada" é a data do Joomla (a busca mostra a de criação, não a de alteração). As contagens são do protótipo do estudo,
com a regra da §3.

**Situação das duas maiores, pela web (notícias e legislação, não o texto das normas):** o Pará passou a dispensar a
DIEF de quem entrega a EFD (IN SEFA 14/2023, revogada pela IN 28/2024 a partir de 2025 — o texto desta não foi
verificado); a FAQ da "DIEF 2016" é sobre o programa daquela versão. O SINTEGRA segue obrigatório no RICMS/PA para quem
não entrega a EFD (o art. 389-T dispensa quem entrega).

## 3. Modelagem v1

Lista curada das páginas (molde PB): `(caminho, nome)`. Corpo = o `div.w3-card-12.internas` do artigo; o título (`h1`)
e a linha de data/categoria ficam antes da 1ª pergunta e saem.

**Segmentação = a da PB** (linhas; pergunta = linha toda em negrito que termina em "?"; só as numeradas quando a
maioria é numerada; grupo e prefixo "R" como lá), **mais um complemento**: numa seção **sem nenhuma** pergunta em
negrito, valem as linhas **numeradas** que terminam em "?". É o caso de PIX, Imunidades e Parcelamento. Na PB toda
seção tem pergunta em negrito, então o complemento não muda nada lá — a implementação tem de provar isso (§6).

| `FaqRaw` | origem |
|---|---|
| `pergunta` | a linha da pergunta, texto puro, como na página |
| `resposta` | as linhas até a próxima, pelo conversor do kit (`kit::html`) |
| `origin` | o nome da página (`Parcelamento de débitos`, `SINTEGRA`…) |
| `url` | o link do título da página (o portal não tem âncora por pergunta) |

**Guardas:** as da PB — página sem corpo, sem pergunta, pergunta sem resposta, par (link, pergunta) repetido, piso
(proposta: 135, ~90 %) —, mais a numeração que pula como aviso.

## 4. Fora do escopo v1

- **Os 13 PDFs** do índice (IPVA, ICMS Antecipado, AINF/ITCD, Cota Parte, EFD nacional…) — PDF fica fora na frota.
- **Os 4 links para outros órgãos** e os **5 itens sem link** do índice.
- **`nfce.sefa.pa.gov.br`** (FAQ da NFC-e) — não responde daqui (timeout de conexão).
- **CADIN** (`app.sefa.pa.gov.br/cadin/faq`) — o servidor fecha a conexão do AuliBot.
- **A cópia da Alíquota Modal 19%** (id 20715) — fica a 20716, a do índice.
- **O catálogo `pis` do WordPress novo** — é de serviços (§1).

## 5. Decisões propostas (D-PAFAQ-*)

- **D-PAFAQ-1 — Escopo.** As opções:
  - **A — as 11 páginas, 152 perguntas.** Fiel ao que a SEFA publica e linka (o site novo manda para o antigo em
    "Orientações"); é o critério que a PB adotou — tirar só o que se sabe obsoleto, e aqui nada está comprovadamente
    extinto. O risco é o envelhecimento: 7 das 11 são de 2014–2018, e as respostas não trazem data.
  - **B — só as criadas ou atualizadas de 2019 em diante: 21 perguntas** (Alíquota 19%, PIX, Imunidades e o Selo
    Fiscal, criado em 2014 mas que cita 2021). Seguro, mas descarta
    SINTEGRA, Parcelamento e NF-e, que podem continuar valendo.
  - **C — A sem a DIEF 2016: 122 perguntas.** A DIEF 2016 é a FAQ de um programa de 2016, e a obrigação mudou em
    2023–2025; é a mais provável de estar desatualizada, mas não há norma que a dê por extinta.
  - **D — não coletar o PA agora** e esperar o site novo ganhar FAQ.

  Recomendo **A**, pelo mesmo critério da PB; **C** é a alternativa se a dúvida sobre a DIEF pesar mais.
- **D-PAFAQ-2 — Segmentação = a da PB + o complemento** das numeradas sem negrito (§3).
- **D-PAFAQ-3 — A segmentação vai para o kit.** Com o PA, o percurso em linhas e o `segmentar` da PB ganham o 2º
  consumidor; em vez de copiar ~250 linhas, extrair para `auli_scraper_kit` (como o conversor, D-KITHTML). Prova: a PB
  sai byte a byte igual pelo `--usecache`.
- **D-PAFAQ-4 — Fonte e link:** lista curada das 11 no Joomla antigo, link = o do título da página, trilha = o nome da
  página.
- **D-PAFAQ-5 — Transporte:** UA AuliBot, 1 s entre os 11 GETs, `ureq`/rustls. Subcomando `auli-scraper-pa faqs` /
  `all`, sempre à rede, cache em `data/pa/raw/cache/faqs/` só com `--usecache`.
- **D-PAFAQ-6 — Correção do REGISTRO#pa:** a Fase 4 ("Manual de Atendimento — EXTINTO") passa a dizer que o Joomla vive
  em `antigo.sefa.pa.gov.br`; e o catálogo `pis` (154) entra como pendência de serviços.

## 6. Plano de implementação

1. Extrair para o kit o percurso em linhas e o `segmentar` da PB (D-PAFAQ-3), com o complemento; a PB passa a chamá-los.
   → a PB pelo `--usecache` sai idêntica (árvore e snapshot); os testes dela continuam passando.
2. `auli-scraper-pa/src/faqs.rs` com a lista curada (D-PAFAQ-1); `main.rs` aceita `servicos | faqs | all`. → 11 GETs.
3. Testes sobre as páginas reais: uma em negrito numerada (SINTEGRA), uma sem número (Selo Fiscal), uma sem negrito
   (Parcelamento, com o "1.1 -"), a do índice que é só links (para o "sem pergunta").
4. Registry, `entities.ts`, `update-pa-faqs.sh`, REGISTRO#pa-faqs (e a correção da Fase 4), SCRAPERS.md, README;
   `auli-collections pa`, `build-packs.sh pa faqs`, smoke `/v1/retrieve`.

## 7. Reprodução

```sh
UA='AuliBot/0.1 (+https://github.com/oxschellen/auli)'; A=https://antigo.sefa.pa.gov.br
curl -sS -A "$UA" -o idx.html "$A/orientacoes/27-orientacoes/manual-de-atendimento/11578-perguntas-e-respostas"
grep -o 'class="MsoNormal"><a href="[^"]*"' idx.html | wc -l                      # os links do índice
curl -sS -A "$UA" -o busca.html "$A/component/search/?searchword=perguntas%20e%20respostas&searchphrase=exact&limit=0"
grep -o '<p>[0-9]* resultados' busca.html                                          # 28 resultados
curl -sS -I -A "$UA" "https://site-sefa-wordpress.sefa.pa.gov.br/wp-json/wp/v2/pis?per_page=1" | grep -i x-wp-total   # 154
curl -sS -A "$UA" -o /dev/null https://app.sefa.pa.gov.br/cadin/faq               # Empty reply from server
```
