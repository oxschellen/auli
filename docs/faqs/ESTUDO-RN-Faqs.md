# ESTUDO-RN-Faqs — Perguntas Frequentes da SEFAZ-RN (coleção `faqs` da entidade `rn`)

**Data:** 2026-11-06 (sondagem direta, UA AuliBot, ~25 GETs). Substitui o rascunho do mesmo dia, que varreu só as
`postagem` do WordPress e fechou a descoberta (§7). · **Estado:** **v1 implementada (06/11/2026) — a FAQ do programa
Nota Potiguar, 23 perguntas** (§3, §9). · **Órgão:** Secretaria de Estado da Fazenda do Rio
Grande do Norte (SEFAZ-RN; host `sefaz.rn.gov.br`). · **Hoje a `rn` tem só `servicos`** (15 cards do CPT `servicos`
pela WP REST — [REGISTRO-scrapers.md#rn](../REGISTRO-scrapers.md#rn)).

> **TL;DR — a FAQ tributária do RN não está no site da SEFAZ; está no Nota Potiguar.**
>
> O site da SEFAZ-RN (WordPress + React, `wp-json` aberto, `robots.txt` permissivo) não tem tipo de conteúdo de FAQ.
> Varrendo o corpo dos **179** itens de conteúdo (154 `postagem`, 18 `materia`, 6 `page`, 1 `central-servico`), a
> única FAQ de verdade é a da **Loteria Estadual** (29 perguntas — não é tributo); o resto são 2–6 perguntas soltas em
> páginas de contato, e as "FAQs" de IPVA e ITCD são **PDFs** num repositório de documentos. O site legado ASP
> (`set_v2/…_faq.asp`) não existe mais.
>
> Mas o **Nota Potiguar** (`np.sefaz.rn.gov.br/portal`, o programa de cidadania fiscal — o par da Nota MT, de onde
> vieram as FAQs do MT) tem uma tela **FAQ (Perguntas Frequentes)** com **23 perguntas** sobre o programa: participar,
> CPF na nota, reclamação, nota que não aparece, contingência, **desconto no IPVA**, cupom × nota. O texto está
> **escrito no próprio JavaScript** do portal (`js/templates.js`, templates AngularJS no `$templateCache`) — **1 GET**,
> molde AP; o link oficial é a rota `#/o-aplicativo/faq`.
>
> Recomendação: **v1 = a FAQ do Nota Potiguar** (D-RNFAQ-1). Fora: Loteria, PDFs, Wayback. À parte: o **Portal da
> Reforma Tributária** da SEFAZ-RN (lançado em 12/06/2026) estava fora do ar (502) nas duas tentativas — sondar de
> novo; e as **Respostas de Consultas** de 2023 e 2024 (59 ementas com PDF) são candidatas a `pareceres`.

---

## 1. O que foi avaliado (06/11/2026)

Todas as sondagens com `AuliBot/0.1 (+https://github.com/oxschellen/auli)`, pelo `curl`, ~1 s entre requisições.

| Fonte / rota | Veredito | Evidência |
|---|---|---|
| `www.sefaz.rn.gov.br/robots.txt` | permissivo | `User-agent: *`, `Disallow: /wp-admin/`, `Allow: /wp-admin/admin-ajax.php`, `Sitemap: /wp-sitemap.xml` |
| WP REST `/wp-json/wp/v2/types` | **sem tipo de FAQ** | `postagem` (Páginas internas), `servicos`, `programas`, `central-servico`, `materia`, `link-inicial`, `social-media`, `informacoes-do-orgao`, `banners-fixos`, `destaque-orgaos` + nativos |
| **Todo o conteúdo do WP** — `X-WP-Total`: `postagem` 154, `materia` 18, `page` 6, `central-servico` 1, `posts` 0, `programas` 0 | varrido o corpo dos **179** (`content` + todos os campos `acf`): linhas terminadas em "?", termos de FAQ, marcação de acordeão | §2 |
| `postagem` 1274 "Perguntas e respostas – LOTERIA ESTADUAL" | **FAQ de verdade, fora do escopo** (loteria, não tributo) | 29 perguntas |
| `postagem` 408 "FAQ (Dúvidas frequentes)" (IPVA) | **só um link** para `/documentos/Impostos/FAQs/IPVA/` | 227 caracteres |
| `/documentos/Impostos/FAQs/IPVA/` e `/ITCD/` (o "Mapa do Site" linka os dois como "FAQ (Dúvidas frequentes)") | **PDFs** — um por pasta: "Perguntas Frequentes de Isenção de IPVA.pdf" e "Perguntas Frequentes do ITCD.pdf", servidos pelo Nextcloud (`/wp-json/nextcloud/v1/download?filePath=SEFAZ-GOVRN%2FImpostos%2FFAQs%2F…`) | página "Documentos", 268 KB cada |
| Site legado ASP (`contentProducao/aplicacao/set_v2/…`: `ipva_faq.asp`, `itcd_faq.asp`, `plantaofiscal/…faq.asp`) | extinto — 404 do WordPress | sondagem do rascunho; os índices de busca ainda listam as rotas |
| **Nota Potiguar** (`np.sefaz.rn.gov.br` → `/portal`, IIS/ASP.NET, "Programa de Cidadania Fiscal do Rio Grande do Norte") | **FAQ estruturada e pública — a fonte da v1** | AngularJS com 80 templates em `js/templates.js` (1,06 MB); duas telas de FAQ: `components/oaplicativo/views/faq.html` (estado `o-aplicativo-faq`, rota `/o-aplicativo/faq`, **23 perguntas**) e `components/comunicacao/views/faq.html` (`comunicacao-faq`, `/comunicacao/faq`, 22); sem `robots.txt` (404) |
| `np…/portal/downloads/manuais/faq-np.pdf` ("Perguntas e Respostas — NOVO!") | PDF, fora | 4,9 MB, `Last-Modified` 20/07/2023 |
| **Portal da Reforma Tributária** (`portalreformatributaria.sefaz.rn.gov.br`, lançado em 12/06/2026 — `materia` 1869) | **não avaliado — fora do ar** | HTTP 502 "Aplicação indisponível" (IIS) na raiz e no `robots.txt`, às ~21h e às 21h14 |
| UVT (`uvt.sefaz.rn.gov.br`) e `usuarios-api` | transacional, sem FAQ | já avaliado para serviços (`REGISTRO-scrapers.md#rn`) |
| Headless | desnecessário | o WP é REST; o Nota Potiguar entrega os templates num `.js` estático |

## 2. O que a varredura do WordPress achou

| id | tipo | "?" | título | juízo |
|---|---|---|---|---|
| 1274 | `postagem` | 29 | Perguntas e respostas – LOTERIA ESTADUAL DO RIO GRANDE DO NORTE | FAQ, fora do escopo (D-RNFAQ-2) |
| 562 | `postagem` | 6 | Contatos e Dúvidas do Cadastro Sincronizado | 6 perguntas + contatos |
| 571 | `postagem` | 2–3 | O Fronteira Rápida | página institucional |
| 265 | `postagem` | 2 | SIGAT | página institucional |
| 408 | `postagem` | 0 | FAQ (Dúvidas frequentes) | link para a pasta do PDF do IPVA |
| 439 | `postagem` | 0 | Eventos da NF-e | 54 marcações de acordeão, mas é tabela de eventos, não pergunta |
| 442–578 (10) | `postagem` | 0–1 | Contatos e Dúvidas da NF-e / NFC-e / CT-e / MDF-e / BP-e / NFA / EFD… | páginas de contato |
| 1708, 1070 | `postagem` | — | Respostas de Consultas 2023 e 2024 | **pareceres**, não FAQ (§6) |

Fora a Loteria, o WordPress tem **~10 perguntas tributárias soltas** em páginas que não são FAQ. Não sustentam uma
coleção por conta própria (D-RNFAQ-5).

## 3. A v1 proposta — a FAQ do Nota Potiguar

**Fonte.** `GET https://np.sefaz.rn.gov.br/portal/js/templates.js` (1 GET, ~1 MB, nome fixo — sem hash de build). Cada
tela é um módulo `angular.module("<template>", []).run(["$templateCache", …put("<template>", "…" + "…" + …)])`: o
HTML vem como concatenação de literais de string JS, que se juntam e desescapam (`\"`, `\n`) antes do parse. A tela é
`components/oaplicativo/views/faq.html`: `<h1>FAQ (Perguntas Frequentes)</h1>` e, para cada pergunta, um
`<h3 class="titulo-pagina-h3">` seguido dos `<p>` da resposta até o próximo `<h3>`.

**As duas telas.** A de "O Aplicativo" tem as 23 perguntas; a de "Comunicação" tem 22, todas também na outra, com 10
respostas idênticas e 12 em redação mais curta — a de "O Aplicativo" é a versão completa (tem a pergunta do desconto
no IPVA e respostas atualizadas). A v1 lê só ela (D-RNFAQ-1); a outra serve de guarda: pergunta da "Comunicação" que não
esteja em "O Aplicativo" vira aviso (sinal de que as telas divergiram).

**Link e identidade.** Não há âncora por pergunta: as 23 têm o link da tela,
`https://np.sefaz.rn.gov.br/portal/#/o-aplicativo/faq` (o app usa `hashPrefix("")`). O nome do `.md` é
`slug(pergunta)` + hash8 de "link\npergunta", então a pergunta é a identidade: repetida é `Err`.

**Árvore e trilha.** Raiz `Menu` "Perguntas frequentes" com um `Faq` "Nota Potiguar" (23 itens); trilha "Nota
Potiguar".

**Guardas.** `Err` antes de gravar: `templates.js` sem a tela; tela sem `<h3>`; pergunta sem resposta; pergunta
repetida; menos de 15 perguntas (piso, contra uma tela esvaziada). Sempre à rede, cache só com `--usecache`, como as
FAQs da frota; `update-rn-faqs.sh` com guarda de encolhimento contra o `.anterior`.

**Transporte.** `ureq` com o UA AuliBot do crate (`rn.rs` já usa); o host é outro (`np.`), IIS, sem WAF observado.

## 4. Decisões propostas (do mantenedor)

- **D-RNFAQ-1 — v1 = a FAQ "O Aplicativo" do Nota Potiguar** (23 perguntas), 1 GET do `templates.js`, link da tela.
  ✅ Decidida pelo mantenedor e implementada em 06/11/2026 (§9).
  O precedente é o MT: as FAQs da Nota MT estão no acervo.
- **D-RNFAQ-2 — Loteria fora.** As 29 perguntas da Loteria Estadual são FAQ de verdade, mas não orientam o
  contribuinte sobre tributo. Decisão de escopo — se o mantenedor quiser o RN completo, é a segunda fonte mais fácil
  (WP REST, 1 GET, `postagem` 1274).
- **D-RNFAQ-3 — PDFs fora**: a de isenção de IPVA, a do ITCD e a `faq-np.pdf` do Nota Potiguar (2023) — a classe
  recusada no MA (D-MAFAQ-4) e no AL (D-ALFAQ-5).
- **D-RNFAQ-4 — Sem Wayback** do site ASP: as respostas citam o link oficial, e as rotas `set_v2` são 404.
- **D-RNFAQ-5 — As perguntas soltas do WordPress ficam fora da v1** (Cadastro Sincronizado 6, Fronteira Rápida 2–3,
  SIGAT 2): são páginas de contato e institucionais, não FAQ; uma v2 pode juntá-las se o acervo pedir.
- **D-RNFAQ-6 — Portal da Reforma Tributária: sondar de novo.** É o lugar mais provável de uma FAQ viva da RTC no RN;
  estava em 502. Se tiver FAQ, é v2.
- **D-RNFAQ-7 — Respostas de Consultas → estudo próprio de `pareceres`** (§6), não esta coleção.

## 5. Sondas (UA da frota)

```bash
UA='AuliBot/0.1 (+https://github.com/oxschellen/auli)'
# A tela da v1 continua no bundle? (conta os <h3> da FAQ "O Aplicativo")
curl -sS -A "$UA" https://np.sefaz.rn.gov.br/portal/js/templates.js \
  | python3 -c 'import sys,re; t=sys.stdin.read(); i=t.find("put(\"components/oaplicativo/views/faq.html\""); print(t[i:i+60000].split("$templateCache.put(")[0].count("<h3"))'
# O Portal da Reforma Tributária voltou? (D-RNFAQ-6)
curl -sS -A "$UA" https://portalreformatributaria.sefaz.rn.gov.br/ -o /tmp/rn-rt.html -w '%{http_code}\n'
grep -o '<title>[^<]*' /tmp/rn-rt.html
```

## 6. Achado à parte — as Respostas de Consultas

`postagem` 1074 ("Respostas de Consultas") é o índice; **1708 (2023)** traz **36** consultas e **1070 (2024)**, **23**,
cada uma com a **ementa inline** — número, processo SEI, data, CNAE, assunto, ementa — e o texto integral num PDF (35 e
23 links); **1075 (2025)** está vazia. São pareceres no formato que o acervo já tem para SP, PR, SC e RS: candidatos a
uma coleção `pareceres` da `rn`, com estudo próprio (ementa da página × corpo do PDF).

## 7. O que o rascunho dizia e o que a sondagem mostrou

O rascunho sondou bem o WordPress (tipos, busca, `postagem`, repositório de documentos) e chegou a "adiar, descoberta
fechada". O que mudou:

- **O Nota Potiguar não tinha sido olhado** — e é onde está a FAQ (23 perguntas, §3). O par dele no MT é a fonte das
  FAQs de lá; foi a primeira coisa a procurar depois que o site da SEFAZ não deu nada.
- **A varredura cobria só as 154 `postagem`**; agora cobre os 179 itens de conteúdo. O resultado no WordPress é o
  mesmo, mas a `materia` 1869 revelou o Portal da Reforma Tributária (fora do ar hoje).
- **A pasta do ITCD** estava "não aberta, presumivelmente outro PDF": é — "Perguntas Frequentes do ITCD.pdf".
- **As Respostas de Consultas** tinham a pergunta em aberto (consultas inline ou só PDFs?): ementas inline + PDF,
  36 em 2023 e 23 em 2024, 2025 vazia (§6).
- **A sonda de retomada usava UA de Chrome**, e o texto dizia que a sondagem foi com UA de navegador; o `rn.rs` dos
  serviços usa o AuliBot (D-FROTA-UA), e as sondas do §5 também.
- **Numeração:** a D-RNFAQ-6 vinha antes da 5; as decisões foram renumeradas.

## 8. Fontes (06/11/2026)

- `https://www.sefaz.rn.gov.br/robots.txt`; `/wp-json/wp/v2/types`; `/wp-json/wp/v2/{postagem,pages,materia,central-servico}?per_page=100`
  (todas as páginas); `/wp-json/wp/v2/search?search=nota%20potiguar`; `/documentos/Impostos/FAQs/{IPVA,ITCD}/`.
- `https://np.sefaz.rn.gov.br/` → `/portal/`; `/portal/js/{templates,core,components,shared}.js`; `/robots.txt` (404);
  `HEAD /portal/downloads/manuais/faq-np.pdf`.
- `https://portalreformatributaria.sefaz.rn.gov.br/` (502).
- Sondagem do rascunho (rotas ASP legadas, buscas no `wp-json`): mantida onde não foi refeita.
- `docs/REGISTRO-scrapers.md#rn` (serviços: WordPress + React, CPT `servicos`, UVT).

## 9. Implementação (06/11/2026)

`auli-scraper-rn faqs` (`src/faqs.rs`), como descrito no §3; o UA AuliBot subiu para o crate (`crate::UA`), usado
pelas duas coleções. Registry: `rn` com `servicos` e `faqs`. Atualização: `scripts/tools/update-rn-faqs.sh`.

- **Coleta:** 1 GET do `js/templates.js` com o AuliBot → **23 perguntas**, trilha "Nota Potiguar", link
  `#/o-aplicativo/faq`; nenhum aviso da conferência (as 22 da tela "Comunicação" estão na v1).
- **`--usecache`** reproduz a árvore byte a byte e o snapshot a menos do `coletado_em`.
- **Packs:** `rn-faqs` 23 vetores (rodada parcial; os 15 serviços ficam como estão).
- **Busca** (servidor descartável): "desconto no IPVA com o Nota Potiguar", "esqueci a senha do aplicativo" e
  "diferença entre cupom e nota fiscal" trazem a pergunta certa em 1º. "A loja não colocou meu CPF na nota" traz as
  cinco perguntas sobre CPF na nota (outro CPF registrado em 3º, CPF depois da emissão em 4º).
- **Testes:** 11 novos sobre o recorte real do `templates.js` (as duas telas e os módulos vizinhos) — leitura dos
  literais e escapes, as 23 perguntas, `<br>`/`<ul>`, links, guardas, piso, conferência, árvore. 8 mutações (a
  resposta não parar no `<h3>`, `<br>` sem quebra, escape sem decodificar, sem piso, sem checar repetida, conferência
  invertida, relativo sem o portal, a tela errada), cada uma derruba ao menos um teste.

