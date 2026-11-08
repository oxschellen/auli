# ESTUDO-AC-Faqs — FAQs da SEFAZ-AC (coleção `faqs` da entidade `ac`)

**Data:** 2026-11-07 (sondagem direta, UA AuliBot, cortesia de 1 s, ~40 GETs). · **Estado:** decidido (07/11: D-ACFAQ-1
A) e implementado (§5). · **Órgão:** Secretaria de Estado da Fazenda do Acre (SEFAZ-AC). · **Hoje a `ac` tem só `servicos`**
(Carta de Serviços do WordPress, 19 fichas — [REGISTRO-scrapers.md#ac](../REGISTRO-scrapers.md#ac)).

> **TL;DR — a SEFAZ-AC não tem FAQ própria; a única é a de uma ficha de serviço que já está no acervo.**
>
> No site (`sefaz.ac.gov.br/2021`, WordPress + Elementor) só o post 9727, "Isenção de ICMS para PCD", tem perguntas e
> respostas: **17**, em negrito terminado em "?" — o formato que o `kit::linhas` já segmenta. Mas esse post é uma das 19
> fichas da Carta de Serviços: o texto inteiro dele, com as 17 perguntas, já está no acervo como a descrição do serviço
> (6.748 caracteres). Nenhum dos outros sistemas que a home linka tem FAQ (Nota Premiada Acreana, DEC, sefazonline,
> autorregularização, SINF, Escola Fazendária).
>
> A decisão é se as 17 entram também como FAQ (uma aba de FAQs para o AC, uma pergunta por documento, ao custo de o
> mesmo texto estar duas vezes no acervo) ou se o AC fica registrado como "sem FAQ própria".

---

## 1. O que foi avaliado (07/11/2026)

Todas as sondagens com `AuliBot/0.1 (+https://github.com/oxschellen/auli)`, pelo `curl`, ~1 s entre requisições. Os hosts
`sefaz.ac.gov.br`, `notapremiadaacreana.ac.gov.br` e `dec.sefaz.ac.gov.br` têm certificado emitido pelo Sectigo R36 e não
mandam o intermediário certo: as sondagens usaram o R36 embutido no scraper de serviços como `--cacert`.

| Fonte / rota | Veredito | Evidência |
|---|---|---|
| WordPress, busca `?rest_route=/wp/v2/search` | **só o post 9727** | "perguntas": o post 9727, a página 10383 (Manual do IRRF) e a 331 (DeSTDA) — as duas páginas só citam a palavra, nenhuma pergunta; "perguntas frequentes": só o 9727; "dúvidas": 12 leis e portarias; "faq": nada |
| WordPress, as 30 páginas (`/wp/v2/pages`) | sem FAQ | institucionais, legislação, Carta de Serviços, DeSTDA, Dam on-line |
| **Post 9727 "Isenção de ICMS para PCD"** | **17 perguntas** | `content.rendered` (Elementor) com as perguntas em negrito terminado em "?" ("Quem tem direito a solicitar veículo com isenção de ICMS?", "Qual o valor do veículo que posso adquirir?"…); alterado em 01/04/2026. **Já é a ficha `?p=9727` da Carta de Serviços** no acervo de serviços |
| Nota Premiada Acreana (`notapremiadaacreana.ac.gov.br`, Angular + Strapi) | sem FAQ | o conteúdo vem do Strapi pela mesma origem (`/api/telas`, anônimo): 10 telas (Conheça o Programa, Como Funciona?, Como Receber?, Gestão do Programa, Educação Fiscal, Legislação, Termo de Adesão e as versões mobile), nenhuma de perguntas; a rota `ajuda` é a tela "Gestão do Programa"; "Fale conosco" é um formulário |
| DEC (`dec.sefaz.ac.gov.br/dec/`) | sem FAQ | tela de login e um manual em PDF |
| `sefazonline.ac.gov.br`, `regularize.sefaz.ac.gov.br` | sem FAQ | sistemas com login |
| `sinf.sefaz.ac.gov.br`, `ead.sefaz.ac.gov.br` | sem FAQ | painel de boletins (Oracle APEX) e a Escola Fazendária |
| Busca na web | nada além do site | — |

## 2. O post 9727

O corpo é o mesmo que o scraper de serviços já lê (`.elementor-widget-theme-post-content`): "OBJETIVO DO SERVIÇO", as
regras do Convênio ICMS 38/2012 e, depois, as 17 perguntas, cada uma em `<strong>` numa linha própria, com a resposta nos
parágrafos seguintes. É o caso que o `kit::linhas` cobre sem regra nova (o mesmo da PB e da PA). Seria 1 GET (o post),
com o R36 como âncora, como nos serviços.

## 3. Modelagem, se as 17 entrarem

| `FaqRaw` | origem |
|---|---|
| `pergunta` | a linha em negrito terminada em "?" (`kit::linhas`) |
| `resposta` | os parágrafos até a próxima pergunta (`kit::html`) |
| `origin` (trilha) | "Isenção de ICMS para PCD" |
| `url` | o post (`https://sefaz.ac.gov.br/2021/?p=9727`) — as perguntas não têm âncora |

Árvore: raiz → o post → as 17 perguntas. Guardas: o post com o corpo; ao menos 15 perguntas; pergunta sem resposta é
erro. A introdução (antes da 1ª pergunta) fica de fora — ela já está no serviço.

## 4. Decisões propostas (D-ACFAQ-*)

- **D-ACFAQ-1 — Escopo.**
  - **A — as 17 do post 9727 como FAQ.** Dá ao AC uma aba de FAQs e põe cada pergunta num documento próprio — um alvo menor
    para a busca do que a ficha inteira (não medido). O custo é o texto das 17 estar duas vezes no acervo: na ficha de
    serviço e nas FAQs.
  - **B — não coletar.** O AC fica registrado como "sem FAQ própria"; as 17 perguntas continuam no acervo pela ficha do
    serviço, que a busca já devolve inteira.

  Recomendo **A**: o custo de código é pequeno (o molde PB/PA, 1 GET), e a aba de FAQs passa a existir para o AC. **B** é a escolha se o acervo não deve ter o mesmo texto em duas coleções.
- **D-ACFAQ-2 — Transporte.** UA AuliBot; o R36 como âncora rustls (o mesmo do `ac.rs`); sempre à rede, cache só com
  `--usecache`.

## 5. Implementação (07/11/2026)

`auli-scraper-ac faqs`; `scripts/tools/update-ac-faqs.sh`. A modelagem é a da §3, com dois ajustes:

- **Só a aba "PERGUNTAS FREQUENTES"** é lida (`.elementor-toggle-item` com esse título; exatamente uma). Hoje ela é a
  última das 4 abas do post, mas segmentar o post inteiro faria a última resposta depender da ordem das abas.
- **Uma pergunta vinha partida** por `<br>` entre dois negritos ("Obtive a autorização para compra do veículo com
  isenção mas não</strong><br /><strong>utilizei, posso solicitar novamente?"): a 1ª versão gravou a pergunta só com a
  2ª metade e a 1ª foi para a resposta anterior. Os negritos separados só por um `<br>` passam a ser um antes da
  segmentação (1 caso no post), sem mexer no `kit::linhas` (PB e PA ficam como estão).

**Resultado da rodada de 07/11.** 17 perguntas, nenhuma tag nem entidade no texto. `--usecache` reproduz a árvore byte a
byte e o snapshot a menos do `coletado_em`. Smoke (`/v1/retrieve`, faqs): o alvo em 1º nas 4 perguntas, entre elas a que
vinha partida.

**Testes.** 7, sobre o post real (sem o `<head>`, scripts e estilos) e acordeões sintéticos; 7 mutações (cada regra e cada
guarda), todas pegas.

