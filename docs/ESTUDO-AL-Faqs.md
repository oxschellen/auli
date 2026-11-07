# ESTUDO-AL-Faqs — Perguntas Frequentes da SEFAZ-AL (coleção `faqs` da entidade `al`)

**Data:** 2026-11-06 (sondagem direta, UA AuliBot, ~20 GETs). Substitui o rascunho do mesmo dia, que não alcançava os
hosts de Alagoas e leu o site pelo buscador (§7). · **Estado:** **BLOQUEADO — não existe FAQ pública da SEFAZ-AL
hoje.** O site institucional, onde ela morava, está em manutenção sem data; nenhum outro host a tem. Nada a
implementar. · **Órgão:** Secretaria de Estado da Fazenda de Alagoas (SEFAZ-AL). · **Hoje a `al` tem só `servicos`**
(Portal Alagoas Digital, API REST `/api/v1`, 60 serviços — [REGISTRO-scrapers.md#al](REGISTRO-scrapers.md#al)).

> **TL;DR — não há o que raspar; o gatilho de retomada é o título da página.**
>
> `www.sefaz.al.gov.br` (e `sefaz.al.gov.br`) devolve **a mesma página para qualquer caminho** — `/`,
> `/perguntas-frequentes`, `/robots.txt`, `/sitemap.xml` e um caminho inventado: HTTP 200, 46.523 bytes, o mesmo md5,
> `<title>SEFAZ-AL | Site em manutenção</title>`, "Notícias e conteúdos voltam em breve", sem data. Status e
> cabeçalhos não vão sinalizar a volta; o título vai.
>
> Fora do site, procurei a FAQ onde ela apareceu nos outros estados — no sistema do programa de nota fiscal (MT), no
> bundle do SPA (MA), na API dos serviços (AL) e no gestor de documentos — e não está em nenhum: os sistemas
> transacionais não têm rota nem texto de FAQ, a API do Portal Alagoas Digital não tem campo de pergunta, e o `gcs2`
> só tem legislação. O canal de dúvidas da SEFAZ-AL é a **Nise** (WhatsApp), sem base pública.
>
> Recomendação (D-ALFAQ-1..5): **adiar**; sem Wayback como fonte; retomar quando o título da página deixar de ser
> "Site em manutenção".

---

## 1. O que foi avaliado (06/11/2026)

Todas as sondagens com `AuliBot/0.1 (+https://github.com/oxschellen/auli)`, pelo `curl`, ~1 s entre requisições.

| Fonte / rota | Veredito | Evidência |
|---|---|---|
| **`www.sefaz.al.gov.br/perguntas-frequentes`** e `sefaz.al.gov.br/perguntas-frequentes` | **fora do ar** — página de manutenção | 200, 46.523 B, `nginx/1.24.0`, título "SEFAZ-AL \| Site em manutenção", H1 "Estamos melhorando o site da SEFAZ-AL"; sem `generator`, sem data |
| `www.sefaz.al.gov.br/`, `/robots.txt`, `/sitemap.xml`, `/caminho-que-nao-existe-auli` | a **mesma** página (md5 idêntico) — o host responde a manutenção para tudo | idem; logo não há `robots.txt` legível hoje |
| `novo.`, `portal.`, `site.`, `www2.`, `homologacao.sefaz.al.gov.br` | não existem | nenhum resolve no DNS |
| A página de manutenção em si | 20 links distintos: os sistemas transacionais (13 no grid "Serviços disponíveis" + "Nota Fiscal Cidadã" em destaque), o WhatsApp da Nise, o Portal do Governo e a Transparência; nenhum de FAQ | texto: "os principais serviços ao contribuinte continuam funcionando", "Portal institucional em ajuste — Notícias e conteúdos voltam em breve" |
| **Portal do Contribuinte** (`contribuinte.sefaz.al.gov.br`) | sem FAQ | micro-frontend webpack (`sfz_portal_contribuinte_gateway`, padrão JHipster); o `bootstrap` (6,2 MB) tem 38 rotas — `login`, `govbr`, `empresa`, `servico/ipva`, `noticias/:id`, `admin`… — e nenhuma de FAQ; todo "faq…" do bundle é ícone do Font Awesome (`faQ`, `faQuestion`) |
| `itcd-contribuinte`, `ipvaonline`, `nfa` (Nota Fiscal Cidadã), `portal-dar`, `nfae`, `comexdigital`, `consulta-termo`, `transportadoras` | sem FAQ nos sondados | as páginas iniciais de `itcd-contribuinte`, `ipvaonline` e `nfa` não citam FAQ, "perguntas frequentes" nem "dúvidas"; a da Nota Fiscal Cidadã é só um redirecionamento para o sistema — diferente da Nota MT, de onde vieram as FAQs do MT. Os outros cinco (emissão e consulta) não foram sondados |
| **`gcs2.sefaz.al.gov.br`** (gestor de documentos, link "Legislação" da página) | sem FAQ — só legislação | JHipster; o bundle conhece duas categorias: `CAT001` (legislação tributária) e `CAT017` (Constituição Estadual, regime jurídico, ementário, leis digitalizadas). Candidato a uma coleção `legislacao`, não a `faqs` |
| **Portal Alagoas Digital** (fonte dos serviços) | sem FAQ | os 60 serviços do cache têm 26 campos (`description`, `other_informations`, `steps`, `requirements`…), nenhum de pergunta, e nenhum texto com "pergunta" ou "dúvida"; a página inicial do portal também não cita FAQ |
| **Nise** (WhatsApp `(82) 3216-9999`, "atendimento 24h") | fora — chatbot, sem base publicada | a própria página de manutenção a apresenta como o canal ("certidões, protocolos, ITCD, mercadorias em averiguação e muito mais") |
| Wayback Machine | **não é fonte** (D-ALFAQ-2); serve só para calibrar | a API de disponibilidade do `archive.org` indica a última captura de `/perguntas-frequentes` em **18/07/2025**; o `web.archive.org` não respondeu daqui, então o conteúdo da captura não foi lido |
| FAQs de NF-e / NFC-e | fora — conteúdo nacional (`nfe.fazenda.gov.br`, `nfce.encat.org`), como no MA | — |
| Headless | não se aplica — não há página a desenhar | — |

## 2. O que se espera da FAQ quando voltar

Pouca coisa se sabe do conteúdo antigo. O site era Joomla com K2 (as rotas `/component/k2/…` e
`/artigo/itemlist/tag/…` seguem nos índices de busca), com "Perguntas frequentes" e "Contribuinte Arretado —
Perguntas e Respostas" como páginas; o volume é desconhecido. Segundo as notícias que o rascunho levantou (§8), desde
2021 a SEFAZ-AL leva as dúvidas para a Nise ("principal canal", serviços atendidos "exclusivamente pela atendente
virtual"), então é plausível que a FAQ
pública tenha encolhido antes da manutenção. A expectativa é **modesta** — da ordem de dezenas a poucas centenas de
perguntas —, mas é hipótese: a captura de 18/07/2025 é o jeito barato de medi-la, quando o `web.archive.org` estiver
acessível.

## 3. Decisões propostas (do mantenedor)

- **D-ALFAQ-1 — Adiar** (✅ decidida pelo mantenedor em 06/11/2026; pendências §21). Nenhuma coleção `faqs` para a `al` enquanto o site institucional estiver em manutenção: sem
  TAREFA, sem código, sem cache.
- **D-ALFAQ-2 — Sem Wayback como fonte.** As respostas do Auli citam o link oficial, que precisa abrir; uma FAQ
  reconstruída do arquivo apontaria para rotas K2 que o site novo provavelmente não terá, com conteúdo de antes da
  reforma. A captura pode calibrar o volume (§2), não alimentar o acervo.
- **D-ALFAQ-3 — Gatilho de retomada:** o título de `https://www.sefaz.al.gov.br/` deixar de ser "SEFAZ-AL | Site em
  manutenção". Como o host serve a manutenção para qualquer caminho, basta a raiz; e é o título que muda, não o
  status (que já é 200). O md5 da página (`dd07faf6…`) é sinal secundário: muda também se só a página de manutenção
  for editada.
- **D-ALFAQ-4 — Sonda manual, sem rotina nova.** O §5 é uma checagem de 1 GET. Não há rotina mensal na frota hoje; se
  o mantenedor quiser automatizá-la, o lugar natural é o `scripts/tools/atualizar-servicos.sh al`, que já roda o
  scraper do AL — mas isso é código e decisão à parte.
- **D-ALFAQ-5 — Cartilhas fora**, mesmo depois da volta: as 7 de out/2025 (Taxa de Serviços, ST, Comex Digital, Profis
  2025, parcelamento do SN, PET ICM/ICMS, DT-e) são PDF/roteiro por tema, a classe recusada no MA (D-MAFAQ-4).

## 4. Checklist da retomada (quando o gatilho disparar)

1. **Plataforma** do site novo (Joomla de novo? WordPress? gov.br? SPA?) e se a FAQ vem no HTML do servidor ou de uma
   API (olhar o bundle, como no MA).
2. **Estrutura:** página única, categorias, acordeão? Há **contagem** declarada que sirva de invariante?
3. **Link por pergunta** (âncora ou `id`) ou só por página. O nome do `.md` é `slug(pergunta)` (até 60 caracteres) +
   hash8 de "link\npergunta": com link só da página, a pergunta repetida vira colisão a tratar.
4. **`robots.txt`** de verdade (hoje o host responde a manutenção) e se há WAF — o AL dos serviços não tem bloqueio;
   lembrar GO e DF, que só passam pelo `curl`.
5. **Volume e atualidade** (data por pergunta?). Se for pequeno e velho, pesar contra o custo de uma coleção.
6. **Nise:** se ganhou uma base de conhecimento pública — seria a fonte melhor, no espírito do CSM do MG.

## 5. Sonda (1 GET, UA da frota)

```bash
UA='AuliBot/0.1 (+https://github.com/oxschellen/auli)'
curl -sS -A "$UA" https://www.sefaz.al.gov.br/ -o /tmp/al-home.html -w '%{http_code} %{size_download}B\n'
grep -o '<title>[^<]*' /tmp/al-home.html     # "SEFAZ-AL | Site em manutenção" ⇒ ainda bloqueado
md5sum /tmp/al-home.html                     # dd07faf6… em 06/11/2026
```

Se o título mudar: salvar o HTML como evidência, sondar `/perguntas-frequentes` e `/robots.txt`, e reabrir este
estudo pelo §4.

## 6. Achado à parte

O **`gcs2`** é o gestor de legislação da SEFAZ-AL (categorias `CAT001` e `CAT017`, busca por espécie legislativa e
número). Se um dia a `al` ganhar uma coleção `legislacao`, como a do RS, a descoberta começa por ele — a API vive no
mesmo gateway JHipster (`api/…`), não sondada.

## 7. O que o rascunho dizia e o que a sondagem mostrou

O rascunho chegou à mesma conclusão (adiar), e a página de manutenção é como ele descreveu. O que mudou:

- **"O ambiente não alcança os hosts de Alagoas"** — este alcança: o estudo agora é de sondagem direta, não de índice
  de busca.
- **"Os demais hosts são sistemas (JHipster/Angular atrás de login) sem FAQ pública"** era dedução do HTML sem JS;
  agora está verificado no bundle do Portal do Contribuinte (rotas e textos) e nas páginas iniciais dos sistemas. O
  `gcs2` foi descrito como "outra coleção se um dia interessar": é legislação (§6).
- **Hosts do site novo** (`novo.` etc.) estavam "não sondados": não existem no DNS.
- **O gatilho:** o rascunho mandava olhar `/perguntas-frequentes` e contar "Estamos"; como o host responde a mesma
  página para qualquer caminho, a raiz e o título bastam (D-ALFAQ-3).
- **Os comandos de retomada usavam um UA de Chrome.** A regra da frota é o AuliBot (D-FROTA-UA); o §5 usa ele.
- **"Contrato do acervo (D-FROTA)"** — não existe decisão D-FROTA; o argumento contra o Wayback é o link oficial que
  precisa abrir (D-ALFAQ-2).
- **"Rotina mensal ao lado da do RS"** — não existe essa rotina; a sonda fica manual (D-ALFAQ-4).
- **Volume:** o rascunho não tinha como medir; a captura de 18/07/2025 existe (§2), o conteúdo ficou por ler.

## 8. Fontes (06/11/2026)

- Página de manutenção: `https://www.sefaz.al.gov.br/` (qualquer caminho), título e links como no §1.
- Sistemas: `contribuinte.sefaz.al.gov.br` (`main.js`, `runtime.js`, `src_main_webapp_bootstrap_ts.js`),
  `itcd-contribuinte.sefaz.al.gov.br`, `ipvaonline.sefaz.al.gov.br`, `nfa.sefaz.al.gov.br/nfa`,
  `gcs2.sefaz.al.gov.br` (`app/main.*.bundle.js`).
- Portal Alagoas Digital: a página inicial e o cache dos 60 serviços (`data/al/raw/cache/servicos/`).
- `https://archive.org/wayback/available?url=sefaz.al.gov.br/perguntas-frequentes` (captura de 18/07/2025).
- Notícias sobre a Nise como canal principal (citadas pelo rascunho): COAD 103873, 104916, 105351; legisweb 33252.
- Notícia das cartilhas (10/10/2025): notagateway.com.br, "Sefaz-AL lança novas cartilhas…".
