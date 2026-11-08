# ESTUDO-PB-Faqs — Perguntas frequentes da SEFAZ-PB (coleção `faqs` da entidade `pb`)

**Data:** 2026-11-07 (sondagem direta, UA AuliBot, cortesia de 1 s, ~30 GETs). · **Estado:** **v1 implementada (07/11/2026) — 9 páginas, 200 perguntas** (§8); D-PBFAQ-1 decidida na opção C
(sem a DeSTDA), D-PBFAQ-2..8 como propostas. · **Órgão:** Secretaria de Estado da Fazenda da Paraíba (SEFAZ-PB; o portal se chama "SER/PB"). ·
**Hoje a `pb` tem só `servicos`** (Carta de Serviços em `cartaservico.sefaz.pb.gov.br`, 101 fichas —
[REGISTRO-scrapers.md#pb](../REGISTRO-scrapers.md#pb)).

> **TL;DR — a FAQ da PB está espalhada em 10 páginas do Joomla, cada uma marcando a pergunta de um jeito.**
>
> O portal institucional (`www.sefaz.pb.gov.br`, Joomla) não tem uma página central de FAQ. Ela está em 10 páginas de
> informação (`/info/…`): a de Cadastro (10 abas, 64 perguntas) e as de SPED/EFD, Substituição Tributária, MDF-e,
> NFA-e, Simples Nacional, GIM, DeSTDA, AIDF e Cobrança Automática. São **205 perguntas** ao todo, em HTML servido
> pronto, **10 GETs** por rodada (1,3 MB), `ureq`/rustls.
>
> A pergunta aparece como `<h4>`, `<strong>`, `<b>`, `<li><strong>` ou `<span>` em negrito, numerada de quatro jeitos
> ("1.", "1 -", "P1.", sem número), e muitas vezes divide o `<p>` com a resposta, separada por `<br>`. A regra que dá
> conta das 10 páginas sem configuração por página é uma só (§3): **pergunta = linha toda em destaque terminada em
> "?"**, só as numeradas quando a maioria é numerada.
>
> Quatro tratamentos ficam por conta do Joomla: os 36 e-mails protegidos por script anti-spam ("Este endereço de email
> está sendo protegido de spambots…") viram o endereço visível; a tabela de anexos sai; o prefixo "R –" de 42 respostas
> sai; e os subgrupos da Substituição Tributária ("COMÉRCIO EXTERIOR - PERGUNTAS FREQUENTES"…) vão para a trilha.
>
> Decisão do mantenedor (07/11/2026): **v1 = 9 páginas, 200 perguntas** — as 10 menos a DeSTDA, que a PB deixou de
> aceitar desde a competência de janeiro de 2026 (D-PBFAQ-1, opção C). Texto pelo conversor do kit.

---

## 1. O que foi avaliado (07/11/2026)

Todas as sondagens com `AuliBot/0.1 (+https://github.com/oxschellen/auli)`, pelo `curl`, ~1 s entre requisições.

| Fonte / rota | Veredito | Evidência |
|---|---|---|
| `www.sefaz.pb.gov.br/robots.txt` | padrão do Joomla: bloqueia só as pastas do sistema (`/administrator/`, `/cache/`, `/components/`, `/images/`…) | conteúdo (`/info/…`) liberado |
| Busca do site (`/component/search/?searchword=…&searchphrase=exact&limit=0`) | **o mapa das FAQs** | "perguntas frequentes" 8 resultados, "perguntas e respostas" 17, "faq" 5, "dúvidas frequentes" 0; as páginas de FAQ estão entre eles, o resto são notícias, portarias e acórdãos que só citam o termo |
| Menu do site (todas as páginas `/info`, `/cidadao`, `/itcd-2` linkadas) | confere a busca | só a AIDF (aba "Dúvidas Frequentes") não aparecia na busca; IPVA, ITCD, MEI, ECF, GIA-ST, PED e a página da isenção para PcD não têm FAQ |
| **As 10 páginas de FAQ** (§2) | **fonte v1** | 200, 79–239 KB cada; corpo em `div.item-page > div.content-text` |
| NF-e e NFC-e (`/info/documentos-fiscais/nf-e`, `…/nfc-e`) | **quebradas no próprio menu** | 302 para `error?r=NDA0` (o "404" do Joomla) |
| CT-e (`/info/documentos-fiscais/ct-e`) | fora — só links | 4 abas (Comunicados, Consultas, Legislação, Orientações Técnicas), nenhuma pergunta |
| Portal Mercadorias em Trânsito (notícia de 21/05/2026) | fora — passo a passo em abas, não P&R | 6 "?" em 7 abas de procedimento |
| Carta de Serviços (`cartaservico.sefaz.pb.gov.br`) | já é a coleção `servicos` | — |
| TLS | `ureq`/rustls serve | TLS 1.2, `ECDHE-RSA-AES128-GCM-SHA256`, cadeia completa (2 certificados), verificação ok |
| Versão de impressão (`?tmpl=component&print=1`) | não testada | o estudo foi feito sobre a página normal, que é o que o link da FAQ abre |
| Headless | desnecessário | as abas (`nn_tabs`, Regular Labs) vêm todas no HTML; só a exibição é por JavaScript |

## 2. As 10 páginas

| Página (trilha) | Caminho | Onde está a FAQ | Marca da pergunta | Perguntas | Ano mais recente citado |
|---|---|---|---|---|---|
| Cadastro de Contribuintes | `/info/cadastro/8-perguntas-frequentes` | a página, 10 abas por assunto (FAC, MEI, REDESIM, Contador, Senha, Não Habilitado, DT-e, Subst. Tributário, E-Commerce, Geral) | `<h4>` "1." | 64 | 2026 |
| SPED - EFD-ICMS/IPI | `/info/declaracoes/sped-fiscal` | a página, sem abas | `<strong>` "P1." | 42 | 2026 |
| Substituição Tributária | `/info/produtos-substituicao-tributaria` | aba "Perguntas e Respostas", 4 grupos | `<strong>` "1 -" | 30 | 2026 |
| MDF-e | `/info/documentos-fiscais/mdf-e` | a página | `<b>` "1 -" (a própria página não tem a 6) | 17 | — |
| NFA-e | `/info/notas-fiscais/nfa-e` | a página | `<strong>`, sem número | 16 | — |
| Simples Nacional | `/info/2016-03-21-17-23-19/simples-nacional` | aba "Perguntas Frequentes" (com um índice antes) | `<li><strong>` "1 –" | 12 | 2026 |
| GIM | `/info/declaracoes/gim` | aba "Perguntas e Respostas" | `<h4>` "1." | 10 | 2020 |
| DeSTDA | `/info/2016-03-21-17-23-19/declaracao-de-substituicao-tributaria-diferencial-de-aliquota-e-antecipacao-destda` | aba "Perguntas Frequentes" | `<strong>` "1." | 5 | 2016 |
| AIDF | `/info/documentos-fiscais/aidf` | aba "Dúvidas Frequentes" | `<strong>` "1." | 5 | — |
| Cobrança Automática | `/info/cobranca-automatica` | aba "DIFAL EC 87/2015" | `<span>` em negrito "1 –" | 4 | 2015 |
| **total** | | | | **205** | |

Resposta: mín. 4 caracteres, mediana 527, máx. 4.272; 0 vazias. Nenhum par (página, aba, pergunta) repetido. Seis
seções têm `<table>`.

## 3. Modelagem v1

Lista curada de 10 entradas `(caminho, aba da FAQ ou todas, nome da página)` — molde PE/PA: o portal não tem índice
de FAQ, e a busca do site não serve de enumeração (não achou a AIDF). Para cada página: GET → `div.item-page >
div.content-text` → a aba indicada (o painel `.tab-pane` cujo `.nn_tabs-title` é o nome dado) ou todas as abas, ou o
corpo inteiro quando a página não tem abas.

**Pré-processamento do HTML:**

- **E-mail protegido do Joomla.** O plugin troca cada endereço por `<span id="cloak…">Este endereço de email está sendo
  protegido de spambots…</span><script>…</script>`, com o endereço montado em JavaScript (`var addy_text… = 'c&#97;d…'
  + '&#64;' + …`). A coleta junta os pedaços de `addy_text…`, decodifica as entidades e põe o **texto visível** no
  lugar. Decodificam-se os 36 das 10 páginas, e o texto visível é o que vale: num deles o `mailto` do próprio portal
  tem erro de digitação (`cadastro@sefaz.pg.gov.br`), e o texto mostrado está certo (`cadastro@sefaz.pb.gov.br`).
- **Tabela de anexos** do componente `com_attachments` (`div.attachmentsContainer`) sai: na SPED ela grudava na resposta
  da P42 ("… 215 kB").

**Segmentação por linha.** O HTML da seção vira linhas: tag de bloco e `<br>` quebram a linha. Cada linha guarda se
**todo o texto com letras** está em destaque: dentro de `<strong>`, `<b>` ou `<h1>`–`<h6>` (o `<span>` em negrito da
Cobrança Automática está dentro de um `<strong>`). Trecho sem letra fora do negrito não desfaz o destaque. Isso cobre o "1" de "13" que o editor
deixou fora do `<strong>` na ST e o `“` solto depois do "?" na DeSTDA.

- **Pergunta** = linha em destaque que termina em "?" (sem contar aspas e espaços finais).
- **Só as numeradas quando a maioria é numerada** (a regra do AM): no Cadastro, "Sim. Mas o que fazer?" e "Como saber
  se a Junta Comercial enviou solicitação…?" são subtítulos em negrito dentro de respostas do REDESIM, não perguntas.
  A NFA-e não numera nenhuma, e todas valem.
- **Grupo** = linha em destaque sem "?" com "PERGUNTAS FREQUENTES" ou "PERGUNTAS E RESPOSTAS", depois da 1ª pergunta. Vale
  para as perguntas seguintes e não entra na resposta. Na ST são 3 grupos depois das 16 perguntas gerais: "COMÉRCIO
  EXTERIOR" (9), "DIFAL NÃO CONTRIBUINTE – ICMS – EC 87/15" (3), "COMBUSTÍVEIS" (2).
- **Resposta** = as linhas até a próxima pergunta ou grupo. O que vem antes da 1ª pergunta sai: o título da aba, a
  introdução e o índice do Simples, cujas linhas não estão todas em negrito (`<strong>1 - </strong>Não sou…`).

**Texto pelo conversor do kit** (`kit::html::html_para_texto`, `base` = o host). O percurso do DOM serve só para
segmentar: cada linha é reserializada com o texto (reescapado) e os `<a>`/`<img>`, e uma tabela entra inteira, como um
bloco. A pergunta é a sua linha convertida; a resposta, as suas linhas unidas por `<br>` e convertidas. O prefixo
"R –", "R." ou "R:" do começo da resposta sai: são 42, em NFA-e 15, Simples 12, GIM 10 e AIDF 5.

| `FaqRaw` | origem |
|---|---|
| `pergunta` | a linha da pergunta, como na página — com o número ("P1.", "1 -") |
| `resposta` | as linhas da resposta, convertidas pelo kit, sem o prefixo "R" |
| `origin` | `Página \| Aba` quando a aba é o assunto (Cadastro e Cobrança Automática); `Página [\| Grupo]` nas outras (a aba de FAQ única não entra). O grupo perde o sufixo " - PERGUNTAS FREQUENTES" |
| `url` | a página, com `#<id da aba>` quando a FAQ está numa aba (`#fac`, `#perguntas-e-respostas`, `#n%C3%A3o-habilitado`). O portal não tem âncora por pergunta |

Ex.: `Cadastro de Contribuintes | MEI` · `Substituição Tributária | COMÉRCIO EXTERIOR` · `SPED - EFD-ICMS/IPI` ·
link `https://www.sefaz.pb.gov.br/info/cadastro/8-perguntas-frequentes#mei`.

**Árvore (`faqs-tree.json`):** raiz → um nó por página → no Cadastro e na Cobrança, um nó por aba; na ST, um nó por
grupo.

**Guardas (`Err`, antes de gravar):** página sem `div.item-page > div.content-text`; aba da FAQ não encontrada; seção
sem pergunta; pergunta sem resposta; par (link, pergunta) repetido; "protegido de spambots" que sobrou no texto (o
plugin mudou); total abaixo do piso (proposta: 180). **Aviso** (não `Err`): numeração que pula ou repete dentro de uma
seção. A MDF-e não tem a 6, e isso é da página.

## 4. Fora do escopo v1

- **PDFs** linkados nas respostas (exemplos de FAC, informativos da EFD) — o link fica na resposta, o conteúdo não é
  baixado.
- **CT-e**, **Mercadorias em Trânsito** e as abas que não são FAQ (Comunicados, Legislação, Downloads, "Dúvidas &
  Contatos" da GIM) — §1.
- **NF-e e NFC-e** — as páginas estão quebradas no próprio portal.

## 5. Decisões propostas (D-PBFAQ-*)

- **D-PBFAQ-1 — Escopo.** ✅ **Decidida em 07/11/2026: opção C — 9 páginas, 200 perguntas, sem a DeSTDA.** As três
  opções que estavam na mesa:
  - **A** — as 10 páginas (205), fiel ao portal; o risco é responder sobre a DeSTDA como se ela valesse.
  - **B** — só as que citam 2025–2026 (Cadastro, SPED, ST, Simples — 148). Critério fraco: "não cita ano" não é
    "obsoleta" (MDF-e, NFA-e e AIDF são procedimentos sem data).
  - **C** — todas menos o que se sabe obsoleto: a **DeSTDA (5)**. Segundo a SEFAZ-PB (notícias de dez/2025 sobre o
    Decreto 47.732/2025), o módulo EFD/OIE substitui a DeSTDA e a GIA-ST, que deixaram de ser aceitas a partir da
    competência de janeiro de 2026. A GIM continua (para quem não é obrigado à EFD), e a Cobrança Automática (DIFAL
    da EC 87/2015, de 2015) fica, porque não se verificou que as 4 respostas estejam erradas. Reincluir é uma linha da
    lista curada.
- **D-PBFAQ-2 — Uma regra de segmentação para as 10** (linha em destaque + "?", só as numeradas quando a maioria é
  numerada), sem configuração por página além de `(caminho, aba, nome)`.
- **D-PBFAQ-3 — Grupos da ST na trilha**, sem o sufixo " - PERGUNTAS FREQUENTES".
- **D-PBFAQ-4 — Trilha** `Página | Aba` onde a aba é o assunto; `Página [| Grupo]` onde a FAQ é uma aba única ou a
  página inteira.
- **D-PBFAQ-5 — Link** = página + `#<id da aba>` quando há aba; senão a página.
- **D-PBFAQ-6 — Texto pelo conversor do kit** (D-KITHTML), com três tratamentos próprios: o e-mail protegido vira o
  texto visível (36), a tabela de anexos sai e o prefixo "R" da resposta sai (42).
- **D-PBFAQ-7 — Guardas** da §3: `Err` para estrutura que sumiu; só aviso para numeração que pula.
- **D-PBFAQ-8 — Transporte:** UA AuliBot (D-FROTA-UA), 1 s entre os 10 GETs, `ureq`/rustls. Subcomando
  `auli-scraper-pb faqs` / `all`, sempre à rede, cache em `data/pb/raw/cache/faqs/` só com `--usecache`.

## 6. Plano de implementação

1. `auli-scraper-pb/src/faqs.rs`; `main.rs` aceita `servicos | faqs | all`. → 10 GETs.
2. Pré-processamento (e-mail, anexos) → seção → linhas com destaque → perguntas, grupos e respostas → texto pelo kit →
   trilha, link, árvore, guardas. → testes sobre as páginas reais (o Cadastro inteiro, com os subtítulos do REDESIM e
   os e-mails; a aba da ST com os grupos e a 13 partida; a DeSTDA com o `“`; o índice do Simples; a SPED com os
   anexos; a MDF-e sem a 6). Cada regra desligada tem de derrubar o teste dela.
3. Registry (`collections = ["servicos", "faqs"]`) + `entities.ts` (pelo gerador); `scripts/tools/update-pb-faqs.sh`;
   REGISTRO#pb-faqs, SCRAPERS.md, README.
4. `auli-collections pb` → 205 `.md` com nomes únicos, artefatos de serviços idênticos; `build-packs.sh pb faqs`; smoke
   `/v1/retrieve`.

## 7. Reprodução

```sh
UA='AuliBot/0.1 (+https://github.com/oxschellen/auli)'; H=https://www.sefaz.pb.gov.br
curl -sS -A "$UA" $H/robots.txt                                                  # padrão Joomla
curl -sS -A "$UA" -o busca.html "$H/component/search/?searchword=perguntas%20e%20respostas&searchphrase=exact&limit=0"
curl -sS -A "$UA" -o cad.html "$H/info/cadastro/8-perguntas-frequentes"          # 200, ~239 KB, 10 abas
grep -c 'class="tab-pane' cad.html                                               # 10
grep -c 'id="cloak' cad.html                                                     # 18 e-mails protegidos
curl -sS -A "$UA" -o /dev/null -w '%{http_code} %{redirect_url}\n' "$H/info/documentos-fiscais/nf-e"   # 302 → error?r=NDA0
```

## 8. Implementação (07/11/2026)

`auli-scraper-pb/src/faqs.rs` (`auli-scraper-pb faqs` ou `all`), D-PBFAQ-1 na opção C e D-PBFAQ-2..8 como propostas,
com dois ajustes que a implementação mostrou:

- **Link aberto fecha na quebra de linha.** O portal deixa `<a href="mailto:cacgr1@…">` aberto até o endereço da linha
  seguinte; levado adiante, o conversor colava as duas linhas no texto do link ("…gov.brAv. Presidente…"). O percurso
  fecha o link na quebra.
- **Sem a regra do `<span>` em negrito.** O `<span>` em negrito da Cobrança Automática está dentro de um `<strong>`; sem
  a regra, as 200 perguntas saem iguais, e ela saiu.
- **A pergunta é o texto puro da linha**, sem a URL dos links que alguns `<h4>` têm ("enviar uma FAC"); a resposta passa
  pelo conversor do kit.

Números da rodada à rede (9 GETs): 200 perguntas — Cadastro 64, SPED 42, ST 30, MDF-e 17, NFA-e 16, Simples 12, GIM 10,
AIDF 5, Cobrança 4 —, os mesmos títulos, na mesma ordem, do protótipo do estudo. Respostas: 0 vazias, nenhuma tag,
entidade, e-mail protegido ou tabela de anexos no texto, nenhum prefixo "R"; mín. 4 caracteres, mediana 583, máx.
4.718. Aviso único: a numeração da MDF-e (sem a 6). `--usecache`: árvore idêntica byte a byte, snapshot idêntico a menos
do `coletado_em`. 200 `.md` com nomes únicos e 200 vetores; os 106 artefatos de serviços saíram com o mesmo md5. Smoke
`/v1/retrieve` (5 perguntas parafraseadas): o alvo em 1º em 4; na da EFD ("quais empresas são obrigadas a enviar…")
em 5º, atrás de vizinhas da mesma página. Testes: 21 novos sobre as páginas reais e acordeões mínimos; 15 mutações,
uma por regra, todas pegas.

**07/11/2026 — no PR das FAQs do PA:** a segmentação foi para o kit (`kit::linhas`) e ganhou a regra da continuação da
numeração (a linha em negrito que continua a sequência abre pergunta mesmo sem "?"). Na PB ela recuperou a 3 de
Combustíveis da Substituição Tributária, que estava na resposta da 2: **201 perguntas** ([ESTUDO-PA-Faqs.md](ESTUDO-PA-Faqs.md) §8).

