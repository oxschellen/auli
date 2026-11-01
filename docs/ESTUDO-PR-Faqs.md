# ESTUDO-PR-Faqs — Perguntas Frequentes da SEFA-PR (coleção `faqs` da entidade `pr`)

**Data:** 2026-11-01 · **Escopo:** descoberta concluída + especificação para implementar a 2ª coleção
de atendimento da entidade `pr` (hoje só `servicos` + `pareceres`). · **Órgão:** Secretaria de Estado da
Fazenda do Paraná (SEFA-PR), Receita Estadual. · **Fonte:** `https://atendimento.fazenda.pr.gov.br/sacsefa/portal/`
(Portal de Atendimento do SAC).

> **TL;DR — fonte única, PHP server-side, HTML pronto, sem API e sem headless.** As 6 fichas "perguntas frequentes"
> da Carta de Serviços (`fazenda.pr.gov.br`) levam todas ao **Portal de Atendimento do SAC**, uma aplicação PHP à
> parte. O `index` lista **48 referentes** (temas). Cada `assuntosReferente/<id>` traz **todas** as perguntas do
> tema, com a resposta íntegra, agrupadas em cards de assunto. São **1.321 perguntas**, cada uma com um número
> próprio (`Pergunta Nº 0271`). Identidade = esse número. **Quatro armadilhas medidas:**
> (1) **TLS com cadeia incompleta**: o servidor manda só o certificado folha, emitido pelo Sectigo **DV** R36 (não o
> OV R36 do AC); curl e ureq falham sem o intermediário embutido;
> (2) **`<` literal no texto** ("Cadastro de Contribuintes < Alteração Cadastral", e um `<Consulta Pendencias>`), que
> um regex genérico de tag apaga; a conversão precisa de uma lista fechada de tags;
> (3) **não há link por pergunta**, e o nome do `.md` precisa de um: 17 pares de perguntas com o mesmo texto no mesmo
> referente colidiriam. A url leva a âncora `#heading<id>`;
> (4) **14 respostas trazem tabela em imagem**, e 3 delas são *só* a imagem.

---

## 1. Hierarquia discovery-first (o que foi avaliado)

| Rota | Veredito | Evidência |
|---|---|---|
| Cache dos serviços do PR (`data/pr/raw/cache/servicos/`) | ponto de partida: 6 fichas da Carta são "Consultar perguntas frequentes sobre…" | `grep` no cache, 01/11/2026 |
| Fichas IPVA / ITCMD / Receita PR / IPM | botão "Consultar" → `atendimento.fazenda.pr.gov.br/sacsefa/portal/assuntosReferente/{18,19,26,38}` | snapshot de serviços do PR |
| Ficha "Receita Estadual" (geral) | → `…/sacsefa/portal/index` | idem |
| Ficha "Simples Nacional" → `fazenda.pr.gov.br/Pagina/Perguntas-Simples-Nacional` (Drupal) | **só um hub**: aponta para os referentes `46`, `21` e `11` do mesmo SAC + o portal da RFB (fora de escopo) | `<article>` da página, 8 links |
| **`/sacsefa/portal/index` + `/sacsefa/portal/assuntosReferente/<id>`** | **✅ a fonte.** HTML server-side, sem sessão | abaixo |
| Rota por pergunta (`pergunta/271`, `perguntaFrequente/271`, `assuntosReferente/18/271`) | **não existe** — 302 para o `index` | curl, 01/11/2026 |
| `POST /sacsefa/portal/procurar` | busca por palavra-chave (`palavrasChave`, `tpProcurar` = qualquer/exata/inteiro, `idReferente`, `idAssunto`); não busca por número, não serve de link | form do `index` |
| `GET /sacsefa/procurar/listarAssuntoReferente/<id>` (o AJAX do `app.js`) | JSON dos assuntos de um referente (`idAssunto`, `assunto`, `ativo`). **Sem sessão: `{"status":403,"message":"Acesso negado"}`**; com o cookie `PHPSESSID` do `index` + `X-Requested-With: XMLHttpRequest`: 200. Desnecessário: os cards da própria página trazem os mesmos assuntos (§2.3) | curl, 01/11/2026 |

**Robots:** `robots.txt` = 404 (página "Object not found!" do Apache); nenhuma `meta robots`/`noindex` nas páginas.
Documentos públicos de atendimento ao contribuinte. UA `AuliBot` + 1 s entre requisições (padrão D-PA-ROBOTS, as
constantes do `pareceres.rs` do próprio crate).

## 2. Fonte

Base: `https://atendimento.fazenda.pr.gov.br`. Apache + PHP (`Set-Cookie: PHPSESSID`), jQuery + Bootstrap 3,
`text/html; charset=utf-8`. As páginas respondem 200 **sem cookie**; a sessão só importa para o AJAX, que não usamos.

### 2.1 TLS — cadeia incompleta (variante do AC e do MA)

```
0 s:CN=*.fazenda.pr.gov.br
  i:C=GB, O=Sectigo Limited, CN=Sectigo Public Server Authentication CA DV R36
Verify return code: 21 (unable to verify the first certificate)
```

O servidor manda **só o folha**. `curl` falha com `(60) unable to get local issuer certificate`, mesmo com o store do
sistema. O emissor é o **Sectigo Public Server Authentication CA DV R36**, que é **outro** intermediário, diferente
do **OV** R36 embutido no `auli-scraper-ac`. O PEM sai do AIA do folha:
`http://crt.sectigo.com/SectigoPublicServerAuthenticationCADVR36.crt` (DER; emitido pela Root R46, válido até
21/03/2036). Com ele + o store do sistema, `curl --cacert` dá 200.

- **O folha vence em 21/11/2026**, 20 dias depois desta descoberta. Se a renovação vier do mesmo DV R36, nada
  muda. Se vier de outro emissor, a coleta falha alta no handshake, o que é o comportamento certo, e o PEM tem de
  ser trocado. Conferir na primeira rodada depois de 21/11.
- `www.fazenda.pr.gov.br`, onde roda o scraper de serviços, manda a cadeia completa: o `ureq` padrão passa lá.
  Só o host `atendimento.` precisa do agent com âncora própria.

### 2.2 Índice dos referentes — `GET /sacsefa/portal/index`

48 links `href="/sacsefa/portal/assuntosReferente/<id>"`, com o nome longo do referente como texto
("IPVA - Imposto sobre a Propriedade de Veículos Automotores"). O `<select id="idReferente">` do formulário de busca
tem **as mesmas 48** opções. Os ids vão de 1 a 60 com lacunas, e **todas as lacunas (1–64 fora dos 48) dão 302
para o `index`**: são referentes desativados. Um caso real é o `19`, para onde a ficha do ITCMD da Carta ainda
aponta; o ITCMD de hoje é o `60` (§10). **A fonte de verdade dos referentes é o `index`.**

⚠️ O `index` reusa a classe `perguntaFrequentePanelHeader` e o padrão `id="heading<N>"` para os cards de
**referente**: ali `N` é o id do referente, não de pergunta. O parse de perguntas só roda nas páginas de referente.

### 2.3 Página do referente — `GET /sacsefa/portal/assuntosReferente/<id>`

Sem paginação: a página traz o referente inteiro, de 52 KB (`33`) a 594 KB (`60`, ITCMD, 185 perguntas); 6,3 MB os
48. A estrutura é **referente → cards de assunto → painéis de pergunta**:

```html
<div class="card-header" role="tab" id="headPanel76">            <!-- card do ASSUNTO (idAssunto = 76) -->
  <a class='linkActionPanel collapsed' … href="#collapsePanel76">
    <h4 referenteAssuntoPerguntaContrast …><b>Aspectos Gerais</b></h4></a></div>
<div id="collapsePanel76" class="collapsed collapse" …>           <!-- recolhido ao carregar -->
  <div class="panel-heading perguntaFrequentePanelHeader customPanelHeading" role="tab" id="heading271">
    … <h4 style="color: #0073e6" class='referenteAssuntoPerguntaContrast'>
        Arrematei um veículo em leilão e o mesmo possui débitos de IPVA. Como devo proceder?   </h4>
    … <small><b>Pergunta Nº 0271</b></small> …
  <div id="collapse271" class="panel-collapse collapse expandirRetrairAction" …>
    <div class="panel-body perguntaFrequentePanelBody">
      … <b> Referente: IPVA </b> … <b> Assunto: Arremate em Leilão </b> … <hr> …
      <p style="text-align: justify;">
        Solicitar a regularização … através do <A HREF="https://www.contribuinte.fazenda.pr.gov.br/ipva/faces/home"
        target=_new>Portal do IPVA</A> (…) -> menu "Serviços" -> opção …<br><br>Cabe ao arrematante …</p>
```

- **Pergunta**: `h4.referenteAssuntoPerguntaContrast` dentro do `panel-heading`. É texto puro: nenhuma tag em
  1.321, até 551 chars, com espaços de indentação em volta (`clean`).
- **Número**: `Pergunta Nº 0271` (4 dígitos, zero à esquerda) **== o `N` de `heading<N>`/`collapse<N>`** em 100% dos
  painéis. Ids de 1 a 2.881, esparsos.
- **Referente / Assunto**: as duas `<b>` do corpo. `Referente:` é o nome **curto** ("IPVA"); o nome longo está no
  `index` (difere em 23 dos 48). `Assunto:` == o título do card que contém o painel, em 100% dos 1.321.
- **Resposta**: o `<p style="text-align: justify;">` do corpo. É HTML em 819 das 1.321 (§4). Um `</p>` por
  painel: comparado com o corpo inteiro até o fim do `panel-body`, nenhum truncamento.

### 2.4 Link da pergunta

**Não existe rota por pergunta** (§1). O link é a página do referente mais a âncora do cabeçalho da pergunta:
`https://atendimento.fazenda.pr.gov.br/sacsefa/portal/assuntosReferente/<rid>#heading<id>`.

- **A âncora é obrigatória para a identidade, não só cosmética.** O nome do `.md` é
  `mddoc::nome_faq_de(titulo, link)`, com hash do par (url, pergunta). Há **17 pares** com o mesmo texto de
  pergunta no mesmo referente, sempre em assuntos diferentes ("Quais documentos devem ser apresentados e onde?" no
  `6`). Sem a âncora, a url seria a mesma, e um `.md` sobrescreveria o outro.
- **A âncora não abre a pergunta no navegador.** O painel fica dentro de um card de assunto recolhido
  (`collapsed collapse`), e a página não tem código que leia `location.hash`; só um botão "Expandir Perguntas
  Frequentes". O usuário cai no topo da página do referente certo. `#headPanel<aid>` (o card, que é visível) talvez
  rolasse até o assunto, mas não foi conferido num navegador e é único só por (assunto, pergunta) — ver D-PRFAQ-2.

## 3. Medições que fixam o desenho (01/11/2026)

| Medição | Valor | Leitura |
|---|---|---|
| Referentes no `index` = opções do `<select>` | **48 = 48** | fonte de verdade dos referentes |
| Ids fora do `index` (1–64) | todos **302** | desativados; nunca buscar por varredura |
| Páginas de referente | 48 × 200, sem cookie | sem sessão |
| Painéis no HTML (`perguntaFrequentePanelHeader`) vs. parseados | **1.321 = 1.321** | o parse não perde painel |
| Ids únicos / em mais de um referente | **1.321 / 0** | uma pergunta mora num referente só; sem dobro (≠ SC) |
| Cards de assunto / vazios / `Assunto:` ≠ card | **274 / 0 / 0** | árvore de 2 níveis limpa |
| `heading<N>` == `Pergunta Nº` | 100% | identidade conferível |
| Respostas com tag HTML | **819 / 1.321** | vocabulário: `br` 3.514, `b` 1.474, `a` 930, `u` 62, `hr` 16, `img` 14, `i` 14, `center` 6 |
| Entidades HTML / mojibake | **0 / 0** | UTF-8 puro; ainda assim decodificar |
| `href` | 465, todos entre aspas duplas e absolutos; 87 respostas com `<A HREF` maiúsculo; `target=_new` sem aspas | regex de link case-insensitive |
| `<` + espaço no texto / `<Palavra…>` literal | **12 / 1** (FAQ 1775: `<Consulta Pendencias>`) | §4 |
| Respostas com `<img>` / só imagem | **14 / 3** (1009, 1010, 1011, referente 15) | tabelas JPG em `/sacsefa/images/tabelas/` (200) |
| Textos de pergunta repetidos | 32 (17 no mesmo referente, **0 no mesmo assunto**); 7 pares P/R idênticos | identidade = número; a trilha desambigua |
| Tamanho da resposta | mediana 328, p95 1.384, máx. 3.784 chars | **nenhuma** passa do teto de 6.000 do vetor |
| Requisições de uma coleta | 1 (`index`) + 48 | ~1–2 min com 1 s de cortesia |

Comparado ao SC: tamanho parecido (1.321 × 1.306), mas sem o dobro, sem o 3º nível escondido e sem JSON. Em troca,
o HTML exige parse e a conversão para texto é mais delicada.

## 4. Conversão HTML → texto (as pegadinhas)

Reusar a **convenção** do `html_para_texto` do SC (`<a href="url">texto</a>` → `texto "url"`, tag de bloco → quebra
de linha, entidades decodificadas, `clean` por linha, linhas vazias fora). **Não** reusar o `TAG_RE = <[^>]+>` dele:

1. **`<` como seta de menu.** "…link "Cadastro de Contribuintes < Alteração Cadastral <  Dados Cadastrais"." O
   `<[^>]+>` casa do `<` até o próximo `>` (o da tag seguinte) e apaga o texto no meio. Há 12 casos.
2. **`<Consulta Pendencias>`** (FAQ 1775: "…`www.cadin.pr.gov.br</a> <Consulta Pendencias>`, basta digitar…"). Aqui
   até um regex mais estrito, `</?[A-Za-z][^>]*>`, apaga o texto.
3. **`->` e ` > `** (20 casos) como setas no texto: inofensivos para o regex de tag, mas não podem ser "limpos".

⇒ **Lista fechada de tags** (allowlist): só `a`, `b`, `br`, `center`, `hr`, `i`, `img`, `p`, `u` (+ `strong`, `em`,
`span`, `font`, `div`, `ul`, `ol`, `li`, `sup`, `sub` por folga) saem como tag; qualquer outro `<…>` fica como texto.
Bloco = `br`, `p`, `hr`, `center`, `div`, `li`. Link: `(?is)<a\b[^>]*?\shref\s*=\s*["']([^"']*)["'][^>]*>(.*?)</a>`
(o do SC, case-insensitive: cobre `<A HREF=… target=_new>`).

4. **Imagem.** `<img src="/sacsefa/images/tabelas/EESC_TCNAE1.JPG">` é a única coisa em 3 respostas e um pedaço
   da resposta em outras 11. Sem tratamento, as 3 saem vazias e as 11 perdem a referência à tabela. Recomendado
   (D-PRFAQ-5): `<img src>` → `"<url absoluta>"`, a mesma convenção da âncora vazia.

## 5. Modelagem → `FaqRaw` (snapshot `pr-faqs-snapshot.json`)

| `FaqRaw` | origem |
|---|---|
| `pergunta` | o `h4` do `panel-heading`, `clean` (já é texto) |
| `resposta` | o `<p style="text-align: justify;">` convertido (§4), parágrafos em linhas |
| `origin` | `<nome do referente no index> \| <título do card de assunto>` — ex.: `IPVA - Imposto sobre a Propriedade de Veículos Automotores \| Aspectos Gerais`. Dois níveis, sempre. O nome longo expande a sigla, o que ajuda a busca, e as perguntas genéricas dependem da trilha |
| `url` | `https://atendimento.fazenda.pr.gov.br/sacsefa/portal/assuntosReferente/<rid>#heading<id>` (§2.4) |

Nada mais é inventado: o `text_to_embed` é materializado pelo `auli-collections pr` (fórmula genérica
`origin + pergunta`, `P:/R:`). **Nenhuma mudança em `auli-contract`, `auli-collections` ou `auli-cli`.**
`config/prompts/pr.txt` já fala em FAQs (linhas 11 e 14), então não há o que mudar.

### 5.1 Árvore da aba de FAQs (`pr-faqs-tree.json`)

O `FaqNode` de `auli_scraper_kit::faqtree` (D-SCFAQ-5):

- raiz sintética `Menu` "Perguntas frequentes", `url` = `…/sacsefa/portal/index`;
- um `Menu` por referente, na ordem do `index`; `title` = nome longo; `url` = a página do referente;
- um `Faq` por card de assunto, na ordem da página; `origin` = a trilha; `url` = a página do referente;
- `faq_items` com **`url` por pergunta** (`…#heading<id>`), o campo que entrou no `FaqItem` no PR #176 (mergeado em 01/11/2026,
  `817641e`). Sem ela, a folha herdaria a `url` do nó, que aqui é a mesma página sem a âncora.
- Nada vazio para omitir hoje (0 cards vazios), mas manter a regra do SC: nó sem pergunta na subárvore sai.

## 6. Decisões (D-PRFAQ-*)

- **D-PRFAQ-1 — Fonte = `assuntosReferente/<id>` para cada referente do `index`.** Os 48 do `index`, nunca uma
  varredura de ids (as lacunas são 302). Uma página por referente, sem AJAX, sem sessão.
- **D-PRFAQ-2 — Identidade = `Pergunta Nº`; url = página do referente + `#heading<id>`.** A âncora é o que torna
  o nome do `.md` único (§2.4); o número é a identidade conferível (`heading<N>` == `Pergunta Nº`).
  **A decidir:** `#heading<id>` (recomendado: único por construção, não exige guarda) ou `#headPanel<aid>` (talvez
  role até o assunto certo — não conferido num navegador — e só é único se não houver texto repetido no mesmo
  assunto, que hoje é 0, mas exigiria uma guarda a mais).
- **D-PRFAQ-3 — Trilha = nome do `index` | assunto do card.** O `Referente:` curto do painel fica de fora: o nome
  longo é o mesmo dado, mais completo.
- **D-PRFAQ-4 — TLS: Sectigo DV R36 embutido como trust anchor do rustls**, num agent só para o host
  `atendimento.` (`RootCerts::new_with_certs`, o molde do `ac.rs`). Não usar `get_via_curl`: o curl também falha
  sem o intermediário. Consolidar AC/MA/PR num helper do kit fica para quando houver um 4º caso, sem mexer em AC e
  MA agora.
- **D-PRFAQ-5 — HTML → texto com lista fechada de tags; `<img>` vira `"<url absoluta>"`.** Implementar no crate do
  PR, adaptando o `html_para_texto` do SC (a troca é o `TAG_RE`). Mover para o kit obrigaria a re-verificar o SC byte
  a byte, sem ganho hoje. **A decidir:** o tratamento da imagem. A alternativa é descartar as 3 respostas que são só
  imagem, e aí a guarda de resposta vazia vira erro.
- **D-PRFAQ-6 — Guardas, todas `Err` antes de gravar, salvo a (g):**
  (a) referente do `index` que responde 302 ou vem sem nenhum painel. O `index` diz que ele existe, a página diz
  que não: é a coleta que falhou, não o portal que esvaziou;
  (b) nº de `perguntaFrequentePanelHeader` na página ≠ nº de painéis parseados (o parse perdeu algo);
  (c) `heading<N>` ≠ `Pergunta Nº`, ou `Assunto:` do painel ≠ título do card que o contém;
  (d) um número de pergunta em dois referentes (hoje 0; se aparecer, não é dobro conhecido, é reclassificação);
  (e) zero perguntas no total;
  (f) pergunta sem texto (a D-SCFAQ-3f);
  (g) **aviso**: resposta vazia depois da conversão (com a D-PRFAQ-5, hoje 0).
- **D-PRFAQ-7 — Rede sempre; cache só para `--usecache`** (a lição da D-SCFAQ-4): cache-first reproduziria a
  primeira coleta em silêncio. Cache sob `data/pr/raw/cache/faqs/`, chave = a url da página.
- **D-PRFAQ-8 — CLI:** `auli-scraper-pr faqs` e `all` (= serviços + FAQs). O padrão continua `servicos`, e
  `pareceres` fica fora do `all` (a D-SCFAQ-6).
- **D-PRFAQ-9 — Escopo = só a base do SAC.** A página do Simples Nacional é um hub para os referentes 46/21/11 do
  próprio SAC; o portal da RFB é federal. As FAQs do **Nota Paraná** (`notaparana.pr.gov.br`, programa da SEFA, sem
  referente no SAC) **não foram investigadas** — candidatas a uma coleta à parte, se interessar.

## 7. Plano de implementação (fases com portão humano)

**Pré-requisito:** PR #176 (`FaqItem.url`) — cumprido, mergeado em 01/11/2026 (`817641e`).

**F1 — módulo `auli-scraper-pr/src/faqs.rs` puro + fixtures.** Parse do `index` (48 links, nomes longos) e da
página do referente (cards → painéis → pergunta/número/assunto/resposta); `html_para_texto` com allowlist (§4);
`montar_faq_raw`; `montar_arvore`; guardas D-PRFAQ-6. Fixtures recortadas do HTML real: um referente com 2
cards; uma resposta com `<A HREF … target=_new>` e `<br><br>`; o `<` de menu (FAQ 1806, referente 8) e o `<Consulta Pendencias>`
(FAQ 1775, referente 34); uma resposta só imagem (1009); um painel com `heading` ≠ número ⇒ `Err`; uma página sem painel ⇒ `Err`;
um número em dois referentes ⇒ `Err`; o `index` com seus `heading<N>` de referente, que não podem virar pergunta.
Verificação por mutação: trocar a allowlist pelo `<[^>]+>` do SC tem de derrubar o teste da 1775. →
`cargo test -p auli-scraper-pr`.

**F2 — transporte.** Agent com o DV R36 (D-PRFAQ-4); rede sempre com cache para replay (D-PRFAQ-7); AuliBot + 1 s;
subcomandos `faqs`/`all` no `main.rs`; `write_faqs("pr", DATA_DIR, …)` + `pr-faqs-tree.json` via
`escrever_atomico`. → rodar `auli-scraper-pr faqs`; conferir no log 48 referentes, painéis = parseados por página,
total ≈ 1.321, guardas verdes. Conferir no snapshot: nenhuma tag do vocabulário, a FAQ 1775 com `<Consulta Pendencias>`
intacto, nenhuma resposta vazia. `--usecache` reproduz byte a byte. **Portão humano:** 3 perguntas ao acaso
comparadas com o portal (uma com link, uma com `<`, uma com imagem).

**F3 — pipeline.** `config/registry.toml`: `collections = ["servicos", "faqs", "pareceres"]` na `pr` →
`node scripts/tools/gen-frontend-entities.mjs` → `auli-collections pr` → `scripts/build-packs.sh pr faqs` →
`scripts/build-frontend-public.sh pr` → smoke `POST /v1/retrieve` (`entity=pr`, `kind=faqs`) numa instância
temporária: "Arrematei um veículo em leilão com débito de IPVA, o que faço?" deve trazer a 0271 no topo.

**F4 — registro.** Seção `#pr-faqs` no `docs/REGISTRO-scrapers.md`; README e `SCRAPERS.md` (FAQs = RS, SC e PR);
`scripts/tools/update-pr-faqs.sh` espelhando o `update-sc-faqs.sh` (sem apagar cache, guarda de encolhimento
contra o `.anterior`, `build-packs.sh pr faqs`).

## 8. Critérios de aceitação

- [ ] `cargo test -p auli-scraper-pr` verde com as fixtures da F1, incluindo cada `Err` da D-PRFAQ-6 e a mutação da
      allowlist.
- [ ] Rodada real: 48 referentes, painéis = parseados em cada um, total entre 1.250 e 1.400 (expectativa, não
      guarda), 0 números repetidos.
- [ ] Snapshot e árvore gravados atomicamente; `--usecache` roda offline e reproduz.
- [ ] Nenhuma tag do vocabulário no snapshot; a FAQ 1775 mantém `<Consulta Pendencias>`; as 1009–1011 têm a url da
      tabela.
- [ ] `auli-collections pr` materializa 1.321 `.md` (nenhuma colisão de nome: nº de `.md` == nº de perguntas).
- [ ] Servidor sobe com `pr-faqs`; aba FAQs do PR renderiza; o "Abrir no portal" da pergunta abre a página do
      referente certo.
- [ ] `check-scraper-boundary.sh` e `check-sem-navi.sh` verdes; nenhum código Rust fora de `crates/scrapers/`.

## 9. Evidência bruta (reprodutível)

```bash
# o intermediário que o servidor não manda
curl -s http://crt.sectigo.com/SectigoPublicServerAuthenticationCADVR36.crt | openssl x509 -inform DER > dvr36.pem
cat dvr36.pem /etc/ssl/certs/ca-certificates.crt > ca.pem
B=https://atendimento.fazenda.pr.gov.br/sacsefa/portal
curl -s -o /dev/null -w '%{http_code}\n' $B/index                                   # 000, curl (60) sem o PEM
curl -s --cacert ca.pem $B/index | grep -o 'assuntosReferente/[0-9]*' | sort -u | wc -l   # 48
curl -s --cacert ca.pem $B/assuntosReferente/18 | grep -c perguntaFrequentePanelHeader     # 70 (IPVA)
curl -s --cacert ca.pem $B/assuntosReferente/18 | grep -o 'Pergunta N[^<]*' | head -1      # Pergunta Nº 0271
curl -s --cacert ca.pem -o /dev/null -w '%{http_code}\n' $B/assuntosReferente/19         # 302 (o ITCMD da Carta)
curl -s --cacert ca.pem $B/assuntosReferente/34 | grep -o '<Consulta Pendencias>'           # FAQ 1775 (CADIN)
echo | openssl s_client -connect atendimento.fazenda.pr.gov.br:443 -servername atendimento.fazenda.pr.gov.br 2>/dev/null \
  | openssl x509 -noout -enddate                                                      # notAfter=Nov 21 23:59:59 2026 GMT
```

## 10. Observações fora do escopo

- **Link morto na Carta de Serviços.** A ficha "Ver as perguntas mais frequentes sobre o ITCMD" aponta para
  `assuntosReferente/19`, que hoje é 302 para o `index`; o ITCMD é o `60`. Está no snapshot de serviços do PR como
  veio da fonte. Corrigir é com a SEFA, não aqui.
- A ficha do IPM aponta para o referente `38`, "EFD - Valor Adicionado". É coerente (o IPM sai do valor adicionado),
  só o nome não diz.
