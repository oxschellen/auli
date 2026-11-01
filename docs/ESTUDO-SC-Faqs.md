# ESTUDO-SC-Faqs — Perguntas Frequentes da SEF-SC (coleção `faqs` da entidade `sc`)

**Data:** 2026-11-01 · **Escopo:** descoberta concluída + especificação para implementar a 2ª coleção
de atendimento da entidade `sc` (hoje só `servicos` + `pareceres`). · **Órgão:** Secretaria de Estado da
Fazenda de Santa Catarina (SEF-SC). · **Fonte:** `https://www.sef.sc.gov.br/perguntas`.

> **TL;DR — mesmo portal Next.js do scraper de serviços; API JSON em `/_next/data/<buildId>/perguntas/buscar.json`;
> sem headless.** A base antiga (`caf2.sef.sc.gov.br/.../ConsultarBaseConhecimento.aspx`, WebForms) hoje
> redireciona para `/perguntas`. A busca aceita `assunto=<AssuntoID>`, `texto=` e `numero=`. A lista já traz a
> **resposta íntegra** — não existe (nem é preciso) detalhe por pergunta. **≈ 1.305 FAQs únicas** nos 15 assuntos de
> topo, mais 1 numa raiz extra. Identidade = `BaseConhecimentoID`. Link oficial = `/perguntas/buscar?numero=<id>`.
> **Três armadilhas medidas:** (1) `assunto=<nó intermediário>` devolve cada descendente **em dobro** — dedup por
> raiz + guarda de interseção zero; (2) a árvore do `perguntas.json` tem **só 2 níveis** — o 3º vem no `Assunto` de
> cada pergunta; (3) `Questao`/`Resposta` vêm **ora em HTML, ora em texto puro** — conversão para texto.

> **Revisão de 01/11/2026** (contra o código e o portal ao vivo). Corrigido em relação à versão anterior: a árvore
> do JSON tem 2 níveis, não 3 (§2.1, D-SCFAQ-3c); `Questao`/`Resposta` trazem HTML em 1/3 dos casos (§2.2,
> D-SCFAQ-7); o cache do kit é cache-first sempre (D-SCFAQ-4); o AuliBot + 1 s é constante do `pareceres.rs`, não
> padrão do kit (D-SCFAQ-8); a F3 vetoriza pelo `build-packs.sh`; o critério de "nada fora de `crates/scrapers/`"
> vale para código Rust. Fechadas as três decisões em aberto: D-SCFAQ-3(e), D-SCFAQ-5 e D-SCFAQ-6.

---

## 1. Hierarquia discovery-first (o que foi avaliado)

| Rota | Veredito | Evidência |
|---|---|---|
| `caf2.sef.sc.gov.br/Views/Shared/ConsultarBaseConhecimento.aspx` | **morta** — redireciona (3xx) para `/perguntas` | WebFetch e curl 31/10/2026 |
| SSR de `/perguntas` | só formulário + árvore de assuntos (`noticias: []`) | `_next/data/.../perguntas.json` |
| SSR de `/perguntas/<AssuntoID>` | traz `perguntas[]`, mas **parcial**: o pai mostra uma fração do que os filhos têm (`2955` → 18 de 67) | `_next/data/.../perguntas/2955.json`, 31/10/2026 |
| `/perguntas/noticias/<id>` | é rota de **notícias**, não de FAQ — 404 para `1211` | `curl -o /dev/null -w %{http_code}` → 404 |
| chunks JS (`pages/perguntas-*.js`, `pages/perguntas/[id]-*.js`, `pages/perguntas/buscar-*.js`) | nenhuma chamada a API externa (nem `api-portal`); só `router.push` para `/perguntas/`, `/perguntas/noticias/`. Os nomes dos parâmetros são lidos no `getServerSideProps` (não aparecem no cliente) | grep nos chunks |
| **`/_next/data/<buildId>/perguntas/buscar.json?assunto=<id>`** | **✅ a fonte.** `gssp` (`"__N_SSP":true`); JSON com a lista completa | abaixo |

Força bruta dos parâmetros (`{buscar, n}`):

```
textoGeral=icms   {"buscar":false,"n":0}
texto=icms        {"buscar":true,"n":309}     ← busca livre (tem mínimo de caracteres: texto=a → 0)
termo=icms        {"buscar":false,"n":0}
assuntoId=166     {"buscar":false,"n":0}
AssuntoID=166     {"buscar":false,"n":0}
assunto=166       {"buscar":true,"n":7}       ← por assunto (o que usamos)
numero=5          {"buscar":true,"n":1}       ← por BaseConhecimentoID
numeroPergunta=5  {"buscar":false,"n":0}
```

`buscar:false` = nenhum parâmetro reconhecido (a página renderiza o formulário vazio). **Qualquer nome fora de
`assunto`/`texto`/`numero` é ignorado em silêncio** — o scraper deve tratar `buscar:false` como erro, não como
"zero resultados".

## 2. Endpoints e parâmetros

Base: `https://www.sef.sc.gov.br`. `buildId` lido do `"buildId":"…"` do HTML de qualquer página (reusar
`parse_build_id` do `sc.rs`; a página-fonte pode ser `/perguntas`). Valor em 31/10/2026: `lt_9XPrSVxKazhrBvLLq7`
(muda a cada deploy — **cache por URL lógica sem buildId**, como no `sc.rs`).

### 2.1 Árvore de assuntos — `GET /_next/data/<buildId>/perguntas.json`

```json
{ "pageProps": { "respostaApi": {
    "noticias": [],
    "assuntosBusca": [
      { "AssuntoID": 2955, "Nome": "BENEFÍCIOS FISCAIS", "ExigeCPF": true, "ExigeCNPJ": true, "ExigeIE": true,
        "IconeBase64": null,
        "Subassuntos": [
          { "AssuntoID": 90,   "Nome": "PIC - PROGRAMA DE INCENTIVO À CULTURA", "Subassuntos": [] },
          { "AssuntoID": 2969, "Nome": "PIE- PROGRAMA DE INCENTIVO AO ESPORTE", "Subassuntos": [] },
          { "AssuntoID": 26,   "Nome": "PRÓ-EMPREGO", "Subassuntos": [] },
          { "AssuntoID": 75,   "Nome": "TTD/FUNDOS", "Subassuntos": [] } ] },
      { "AssuntoID": 91, "Nome": "BP-E- BILHETE DE PASSAGEM ELETRÔNICO", "Subassuntos": [] },
      "…"
    ] } } }
```

- **A árvore do JSON tem só 2 níveis** (medido em 01/11/2026: 31 raízes, 81 filhos, **0 netos**, 112 IDs distintos,
  nenhum repetido). O 3º nível existe — Benefícios Fiscais › TTD/Fundos › Aspectos Gerais (`166`) —, mas **não está
  aqui**: `TTD/FUNDOS` vem com `Subassuntos: []`. Ele só aparece no `Assunto` de cada pergunta (§2.2), que traz
  `Nome` e `SuperiorHierarquicoID`. Em Benefícios Fiscais, **49 das 67 perguntas** pertencem a um assunto ausente
  da árvore, e o pai de todos eles está nela. Por isso o mapa de assuntos é **a árvore completada pelas perguntas**
  (D-SCFAQ-3c). Recursão genérica na árvore, sem assumir profundidade.
- `IconeBase64` pode vir com um PNG inteiro em base64 (dezenas de KB) — **não desserializar** (não declarar o campo).
- `ExigeCPF/ExigeCNPJ/ExigeIE` são da abertura de chamado da CAF (a árvore é compartilhada com o atendimento) — ignorar.
- **As raízes da árvore ≠ os 15 cards do HTML.** O HTML de `/perguntas` linka 15 assuntos
  (`5, 12, 13, 14, 2945, 2946, 2949–2957`), mas `assuntosBusca` tem 31 raízes. **A fonte de verdade das raízes é o
  JSON**, não o HTML. As 16 extras são quase vazias: `91`, `29`, `38`, `2974` e `351` devolvem 0 perguntas e `72`
  (DTEC) devolve 1 (`1170`). Raiz com zero perguntas não é erro (só o total zero é — D-SCFAQ-3d).

### 2.2 Perguntas de um assunto — `GET /_next/data/<buildId>/perguntas/buscar.json?assunto=<AssuntoID>`

```json
{ "pageProps": { "respostaApi": {
    "perguntas": [
      { "BaseConhecimentoID": 1211,
        "Questao": "Como efetuar um pedido de TTD?",
        "Resposta": "Os pedidos de TTD devem ser realizados pelo SAT, através da aplicação “TTD - Efetuar um\nPedido de Tratamento Tributário Diferenciado”.\nO requerente deve estar logado no SAT no perfil da empresa ou do seu profissional\ncontabilista.\nNão esquecer de cumprir o item 4 do protocolo e entregar os documentos na gerência\nregional a que jurisdicionado, se for o caso.",
        "Visibilidade": 2,
        "Assunto": { "AssuntoID": 166, "Nome": "ASPECTOS GERAIS", "SuperiorHierarquicoID": 75,
                     "ExigeCPF": false, "ExigeCNPJ": true, "ExigeIE": true },
        "AtendimentoID": null },
      { "BaseConhecimentoID": 1558,
        "Questao": "<p>O que é PIE?</p>",
        "Resposta": "<p>O Programa de Incentivo ao Esporte (PIE) é um programa de fomento a projetos esportivos …</p><p><br></p><p>O PIE permite …</p>",
        "Visibilidade": 2,
        "Assunto": { "AssuntoID": 2969, "Nome": "PIE- PROGRAMA DE INCENTIVO AO ESPORTE", "SuperiorHierarquicoID": 2955 },
        "AtendimentoID": null }
    ],
    "buscar": true,
    "errorCode": null } } }
```

- **Sem paginação**: `texto=icms` devolveu 309 numa resposta só; `assunto=2955` devolveu 134 linhas numa só.
- **`Resposta` já é íntegra, mas o formato varia.** Ora texto puro com `\n` (a `1211`), ora **HTML** (a `1558`):
  em Benefícios Fiscais, **22 das 67** perguntas trazem tags — `p`, `br`, `a`, `font`, `span`, `strong` — e
  entidades (`&nbsp;`, `&amp;`, `&quot;`, todas no `decode_entities` do kit). A `Questao` também pode vir em HTML.
  Os `<a>` são reais e às vezes trazem lixo de e-mail colado (`data-saferedirecturl`, `style`) — só o `href` importa.
  Conversão em D-SCFAQ-7. Links `[[url âncora]]` (o formato dos serviços) não apareceram na amostra (3 raízes): a F2
  conta no acervo inteiro e, se aparecerem, aplica `normalize_links` do `sc.rs`.
- `Visibilidade` = 2 em **100 %** dos 2.607 registros medidos → não filtra nada; só registrar e **alertar** se aparecer
  outro valor (D-SCFAQ-3e).
- `Assunto.SuperiorHierarquicoID` dá o pai; a trilha completa se resolve contra o mapa de assuntos (§2.1 completado).
- `errorCode` ≠ `null` ou `buscar == false` ⇒ **erro** (parâmetro não reconhecido / backend falhou), nunca "vazio".

### 2.3 Link oficial da pergunta

`https://www.sef.sc.gov.br/perguntas/buscar?numero=<BaseConhecimentoID>` — **200** e `numero` é o próprio id
(`numero=1211` devolve exatamente a pergunta 1211; conferido no HTML em 01/11/2026). É o `url` do `FaqRaw` e o
`link` do frontmatter do `.md`.

## 3. Medições que fixam o desenho

| Chamada | linhas / únicos | leitura |
|---|---|---|
| `assunto=166` (folha) | **7 / 7** | folha vem limpa |
| `assunto=75` (pai de 5 folhas) | **44 / 22** | nó intermediário **dobra** cada descendente |
| `assunto=2955` (raiz) | **134 / 67** | idem — Benefícios Fiscais tem 67 FAQs; multiplicidade máxima 2 |
| 15 raízes do HTML somadas | **2.607 / 1.305** | acervo ≈ 1.305 FAQs |
| 16 raízes extras do JSON | **1 / 1** | só a `72` (DTEC) tem pergunta |

(As três primeiras linhas foram re-medidas em 01/11/2026 e bateram.)

Consequências:

1. **Uma chamada por raiz** (31 requisições) cobre tudo — `assunto=<raiz>` agrega os descendentes. Não é
   preciso varrer folhas, e varrer só folhas **perderia** pergunta pendurada diretamente num nó intermediário.
2. O dobro é determinístico e vem da fonte. **Dedup por `BaseConhecimentoID` dentro de cada raiz**, registrando
   `"134 linhas → 67 únicas"` no log — e **não** inventar guarda `únicos == linhas` (ela falharia sempre).
3. A guarda real é estrutural: uma pergunta tem **um** `Assunto` e a árvore não repete ID, logo ela pertence a
   **uma** subárvore ⇒ **`Σ únicos por raiz == únicos no total`** (interseção entre raízes = ∅). Se um dia a fonte
   reclassificar perguntas em dois assuntos, isto falha alto, como deve.

## 4. Modelagem → `FaqRaw` (snapshot `sc-faqs-snapshot.json`)

| `FaqRaw` | origem |
|---|---|
| `pergunta` | `Questao` convertida para texto (D-SCFAQ-7), numa linha só |
| `resposta` | `Resposta` convertida para texto (D-SCFAQ-7), parágrafos em linhas |
| `origin` | trilha **da raiz à folha**, nomes separados por ` \| ` — ex.: `BENEFÍCIOS FISCAIS \| TTD/FUNDOS \| ASPECTOS GERAIS`. Resolvida subindo por `SuperiorHierarquicoID` no mapa de assuntos (árvore + `Assunto` das perguntas), **não** só o `Assunto.Nome` do item. Manter a caixa da fonte (CAIXA ALTA) — ingerir como está. |
| `url` | `https://www.sef.sc.gov.br/perguntas/buscar?numero=<BaseConhecimentoID>` |

Nada mais é inventado: `text_to_embed` é materializado pelo `auli-collections sc` (`derive_faqs::process`, fórmula
`origin + pergunta` com `P:/R:` — já existe e é genérica por entidade). A árvore `docs/faqs/*.md` sai do mesmo
`process` (`nome_faq` = slug da pergunta + hash8(url+pergunta)). **Nenhuma mudança em `auli-contract`,
`auli-collections` ou `auli-cli`.**

### 4.1 A árvore para a aba de FAQs do frontend (`sc-faqs-tree.json`)

A aba **FAQs** (`FaqsList.tsx`) lê `/<id>/<id>-faqs-tree.json` com a forma de `FaqNode`
(`title/url/page_type: Menu|Faq|Geral/origin/children/faq_items[{pergunta,resposta}]`). Para SC a árvore é o mapa
de assuntos (a `assuntosBusca` completada pelo 3º nível das perguntas):

- nó com filhos ⇒ `Menu` (`children` = subassuntos: os da árvore na ordem da fonte, depois os de 3º nível por nome);
- nó só com perguntas próprias ⇒ `Faq` (`faq_items` = perguntas cujo `Assunto.AssuntoID == este nó`, `origin` = trilha);
- **nó sem pergunta em toda a subárvore ⇒ omitido** (as 5 raízes extras vazias e afins não viram seção vazia na aba);
- `url` de cada nó = `…/perguntas/buscar?assunto=<AssuntoID>`, a página com a lista completa (a
  `/perguntas/<AssuntoID>` mostra só parte — §1); a F2 confere que ela renderiza;
- raiz sintética: `title: "Perguntas frequentes"`, `url: …/perguntas`, `page_type: Menu`.

**Um nó pode ser `Menu` E ter `faq_items`** (pergunta pendurada em nó intermediário). O `FaqNode` serializa os dois
campos independentes, e o `parseFaqs.ts` já renderiza os dois: o `convertJsonNode` empilha os `children` e depois os
`faq_items` como folhas (conferido em 01/11/2026).

## 5. Decisões (D-SCFAQ-*)

- **D-SCFAQ-1 — Fonte = `perguntas/buscar.json?assunto=<raiz>`**, uma chamada por raiz de `assuntosBusca` (do JSON,
  não dos cards do HTML). Nem `texto=` (tem mínimo de caracteres e não dá cobertura garantida) nem varredura de
  folhas (perderia pergunta de nó intermediário).
- **D-SCFAQ-2 — Identidade = `BaseConhecimentoID`.** Dedup por raiz; log de `linhas → únicos` por raiz.
- **D-SCFAQ-3 — Guardas dinâmicas (todas `Err`, salvo a (e)):**
  (a) `buscar != true` ou `errorCode != null` em qualquer chamada;
  (b) `Σ únicos por raiz != únicos totais` (pergunta em duas subárvores);
  (c) **cadeia de pais irresolúvel**: o mapa é a árvore completada pelo `Assunto` de cada pergunta (§2.1), e a trilha
  sobe por `SuperiorHierarquicoID` até uma raiz — se algum elo faltar (ex.: um 4º nível cujo pai não está nem na
  árvore nem em pergunta alguma), é `Err`. Exigir que o `Assunto` esteja na árvore, como a versão anterior pedia,
  abortaria toda rodada (49/67 em Benefícios Fiscais estão fora dela);
  (d) zero perguntas no total (é a raiz que caiu, não o portal vazio — mesma doutrina do RS: sai **antes** de gravar);
  (e) **decidido: aviso, não erro** — `Visibilidade != 2` em algum item imprime a contagem. O endpoint é público e não
  pede login, então o conteúdo é público; o aviso existe para alguém olhar;
  (f) **pergunta sem texto** (acrescentada em 01/11/2026, depois da revisão do PR #175) — `Questao` nula ou que o
  `html_para_texto` esvazia (ex.: `<p> </p>`) é `Err` com os `BaseConhecimentoID`. O `.md` sairia sem título e o vetor
  sem conteúdo. `Resposta` vazia não entra: título e link ainda são documento. Na rodada de 01/11, nenhuma.
- **D-SCFAQ-4 — Rede sempre; cache só para replay offline.** O `read_or_bail` do kit é **cache-first sempre** (devolve
  o hit mesmo sem `--usecache`), e reusar o `fetch_cached` do `sc.rs` faria a próxima atualização reproduzir em
  silêncio a primeira coleta — foi o que aconteceu com os serviços do SC em 31/10/2026 (cache de 25/07). Com 32
  requisições, não há o que economizar: sem `--usecache`, toda chamada vai à rede e grava o cache (URL lógica sem
  `buildId`, sob `data/sc/raw/cache/faqs/`); com `--usecache`, lê só do cache e miss é erro.
- **D-SCFAQ-5 — decidido: (a), `FaqNode/FaqItem/PageType` vão para o kit** (`auli_scraper_kit::faqtree`), e o RS passa
  a importar de lá (só o `pub use` do `faqs/mod.rs` muda). É contrato de fio com o frontend — duas cópias divergiriam.
  O kit é exclusivo dos scrapers e só o frontend consome o JSON, então cabe na fronteira D-C1. O SC emite
  `sc-faqs-tree.json` com essa forma (§4.1).
- **D-SCFAQ-6 — decidido: o padrão do `auli-scraper-sc` continua `servicos`**; entram os subcomandos `faqs` e `all`
  (= servicos + faqs). Trocar o padrão mudaria o comportamento de quem roda sem argumento, sem ganho. `pareceres` segue
  fora do `all`, como hoje.
- **D-SCFAQ-7 — Ingerir como está, convertendo só o formato.** Caixa alta dos assuntos, `\n` das respostas em texto
  puro, aspas tipográficas e grafias como `PIE- PROGRAMA` ficam como vieram. O que muda é estrutural: HTML vira texto
  — `<a href="url">texto</a>` → `texto "url"` (convenção do `normalize_links` e dos scrapers ba/pr; âncora vazia →
  só `"url"`; `#`/`javascript:` → só o texto), tags de bloco (`p`, `br`, `div`, `li`…) → quebra de linha, demais tags
  fora, entidades decodificadas, linhas vazias descartadas. Resposta em texto puro passa pela mesma função sem mudar.
- **D-SCFAQ-8 — `AuliBot` + ≥ 1 s entre requisições.** São as constantes `UA`/`COURTESY` do `pareceres.rs` do próprio
  crate (o `USER_AGENT` do kit é um Firefox, e os serviços do mesmo host rodam com ele, sem pausa). São 32 GETs; o
  censo completo é barato e **sempre completo** (sem marca de restart — mesma conclusão do TARF/SP/SC-pareceres).

## 6. Pegadinhas de serde / fonte

- `IconeBase64` **não declarar** (payload grande e inútil). `#[serde(default)]` em `Subassuntos`.
- `SuperiorHierarquicoID` pode ser `null` na raiz — `Option<i64>`.
- `Questao`/`Resposta` como `Option<String>` (um `null` derrubaria um `String` com `default`).
- `errorCode` como `Option<serde_json::Value>` (o tipo do erro não foi observado).
- `AtendimentoID` ignorar. `Visibilidade` como `i64`.
- Reusar `NextData<T>` / `PageProps<T>` do `sc.rs` (tornar `pub(crate)`).
- Caracteres: a fonte usa aspas tipográficas (`“TTD - …”`) e `\n` crus — o `.md` do contrato aceita; **não** converter.
- O HTML de `/perguntas` serve só para o `buildId`; não parsear mais nada dele.

## 7. Plano de implementação (fases com portão humano)

**F1 — módulo `faqs.rs` puro + fixtures.** Structs de desserialização (2.1, 2.2); `FaqNode` movido para o kit
(D-SCFAQ-5); mapa de assuntos = árvore + `Assunto` das perguntas; `trilha(mapa, id)`; dedup por raiz; `consolidar`
com as guardas D-SCFAQ-3; `html_para_texto`; `montar_faq_raw`; `montar_arvore`. Fixtures copiadas dos JSONs reais,
incluindo: **um caso de dobro**; **uma pergunta de 3º nível** (assunto fora da árvore, pai dentro); **uma resposta em
HTML** (`<p>`, `<br>`, `<a>`, entidades); **um caso de interseção** ⇒ `Err`; **um `buscar:false`** ⇒ `Err`; **um pai
fora do mapa** ⇒ `Err`; **zero perguntas** ⇒ `Err`. Verificação por mutação: tirar o dedup, a completação do mapa ou
a conversão de HTML tem de derrubar um teste. → `cargo test -p auli-scraper-sc` (e `-p auli-scraper-rs`, pelo
`FaqNode` movido).

**F2 — transporte.** `buildId` a partir de `/perguntas`; fetch rede-sempre com cache para replay (D-SCFAQ-4); laço
pelas raízes com `COURTESY`; `write_faqs("sc", DATA_DIR, …)` + `sc-faqs-tree.json` via
`auli_contract::arquivo::escrever_atomico`. Subcomandos `faqs` e `all` no `main.rs`. → rodar
`cargo run --release -p auli-scraper-sc -- faqs`; conferir no log: nº de raízes, `linhas → únicos` por raiz, total
único (≈ 1.306), guardas verdes, contagem de `[[` e de entidades que sobraram. **Portão humano:** abrir 3 `.md` ao
acaso e comparar com o portal.

**F3 — pipeline.** `config/registry.toml`: `collections = ["servicos", "faqs", "pareceres"]` na entidade `sc`
(o seletor do chat e a aba FAQs se auto-gateiam por isso) → `node scripts/tools/gen-frontend-entities.mjs` →
`auli-collections sc` (gera `docs/faqs/*.md`, `sc-faqs.json`, `sc-portal-faqs.txt`) → `scripts/build-packs.sh sc faqs`
(compila, vetoriza e deriva os índices — o caminho das rodadas de 31/10) → `scripts/build-frontend-public.sh sc`
(copia `sc-faqs-tree.json`) → smoke: chat tipo 1 na entidade SC com uma pergunta de TTD deve recuperar FAQ 1211.
`config/prompts/sc.txt` já fala em FAQs (linhas 6/12/14/15; nada a mudar). Os zips de download (`auli bundle`)
saem no `deploy-frontend.sh`.

**F4 — registro.** Seção `SC FAQs` em `docs/REGISTRO-scrapers.md` (índice + corpo com este estudo condensado:
endpoints, parâmetros, o dobro, os 2 níveis, o HTML, as guardas), linha nos totais (`downloads.json` já é fonte
única; README só se citar SC), e `docs/auli_pendencias.md` se sobrar algo (ex.: `Visibilidade` ≠ 2).
`scripts/tools/update-sc-faqs.sh` espelhando `update-rs-faqs.sh` (cópia `.anterior` + guard de encolhimento).

## 8. Critérios de aceitação

- [ ] `cargo test -p auli-scraper-sc` e `-p auli-scraper-rs` verdes com as fixtures da F1 (incluindo os 4 casos de `Err`).
- [ ] Rodada real: `buscar:true` em todas as raízes; `Σ únicos por raiz == total único`; total entre 1.250 e 1.400
      (se fora, investigar antes de gravar — não é guarda, é expectativa).
- [ ] `data/sc/sc-faqs-snapshot.json` + `data/sc/raw/sc-faqs-tree.json` gravados atomicamente; `--usecache` roda offline.
- [ ] Nenhuma tag HTML no snapshot (`grep -c '<p>'` = 0).
- [ ] `auli-collections sc` materializa `data/sc/docs/faqs/*.md` com `trilha` de 1–3 níveis e `link = …/buscar?numero=<id>`.
- [ ] Servidor sobe (`docs_hashes` batem), aba FAQs da SC renderiza a árvore e o chat tipo 1 cita `FAQ n` com link que abre a pergunta certa.
- [ ] Nenhum código Rust fora de `crates/scrapers/` mudou; `check-scraper-boundary.sh` e `check-sem-navi.sh` verdes.

## 9. Evidência bruta (reprodutível no Desktop Server)

```bash
B=$(curl -s https://www.sef.sc.gov.br/perguntas | grep -o '"buildId":"[^"]*"' | cut -d'"' -f4)
curl -s "https://www.sef.sc.gov.br/_next/data/$B/perguntas.json" | jq '.pageProps.respostaApi.assuntosBusca | length'   # 31
curl -s "https://www.sef.sc.gov.br/_next/data/$B/perguntas.json" \
  | jq '[.pageProps.respostaApi.assuntosBusca[].Subassuntos[].Subassuntos | length] | add'                         # 0 netos
curl -s "https://www.sef.sc.gov.br/_next/data/$B/perguntas/buscar.json?assunto=166" | jq '.pageProps.respostaApi.perguntas[0]'
for a in 166 75 2955; do curl -s "https://www.sef.sc.gov.br/_next/data/$B/perguntas/buscar.json?assunto=$a" | jq -r '.pageProps.respostaApi.perguntas[].BaseConhecimentoID' | awk '{t++} !s[$0]++{u++} END{print t"/"u}'; done   # 7/7, 44/22, 134/67
curl -s "https://www.sef.sc.gov.br/_next/data/$B/perguntas/buscar.json?assunto=2955" \
  | jq '[.pageProps.respostaApi.perguntas[] | select((.Questao + .Resposta) | test("<(p|br|a|span|strong|font)\\b"))] | length'   # 44 linhas = 22 únicas
curl -s -o /dev/null -w '%{http_code}\n' "https://www.sef.sc.gov.br/perguntas/buscar?numero=1211"   # 200
```

## 10. Execução

**F1 (01/11/2026)** — `auli-scraper-sc/src/faqs.rs` (módulo puro + 11 testes); `FaqNode/FaqItem/PageType` em
`auli_scraper_kit::faqtree`. Mutação conferida: tirar o dedup, a completação do mapa, a conversão de HTML, cada
guarda (a)/(b)/(d), a contagem de visibilidade ou a omissão de nó vazio derruba ao menos um teste.

**F2 (01/11/2026)** — subcomandos `faqs` e `all`; rodada real com 33 GETs (HTML para o `buildId`, a árvore e as 31
raízes):

- **1.306 perguntas únicas** (1.305 nos 15 assuntos de topo + 1 da raiz `72`); 15 das 31 raízes vazias; todas as
  guardas verdes; nenhuma `Visibilidade ≠ 2`.
- Trilha: 4 de 1 nível, 513 de 2, 789 de 3. Nenhuma pergunta ou resposta vazia; 1.306 URLs distintas.
- HTML: nenhuma tag nem entidade sobrando; os 2 `<cBenef>` que restam são texto (a resposta 1580 cita o XML da
  NF-e, que a fonte escapa como `&lt;cBenef&gt;`). Nenhum `[[` no acervo — o `normalize_links` não é necessário.
- Árvore do frontend: 49 `Menu` + 244 `Faq`, 1.306 `faq_items`; **14 `Menu` com `faq_items`** (o caso existe);
  16 raízes na aba.
- `--usecache` (33 arquivos em `data/sc/raw/cache/faqs/`) reproduz itens e árvore byte a byte.
- `…/perguntas/buscar?assunto=166` renderiza as 7 perguntas de ASPECTOS GERAIS — o `url` dos nós da aba serve.
- Portão humano: FAQs 523 e 529 (texto puro) e 1654 (veio em HTML) conferidas contra o portal — pergunta, resposta
  e assunto iguais; a 1654 sai com os dois parágrafos.

**F3 (01/11/2026)** — `faqs` no registry da `sc` → `entities.ts` regenerado (só a linha da SC muda) →
`auli-collections sc` materializou 1.306 `.md` em `data/sc/docs/faqs/` (trilha 4/513/789 de 1/2/3 níveis; 1.306
links `…/buscar?numero=<id>`, todos no padrão) → `build-packs.sh sc faqs`: 1.306 registros vetorizados, serviços e
pareceres preservados no manifesto (três `docs_hashes`); 1 registro passa do teto de 6.000 chars (FAQ 434, layout
do arquivo magnético do SINTEGRA, 7.047) e só a cauda fica fora do vetor → `build-frontend-public.sh sc`
(`sc-faqs-tree.json` idêntica à de `data/`).

- Smoke (instância temporária em `127.0.0.1:3999`, `POST /v1/retrieve`, `kind=faqs`): o boot valida o manifesto da
  `sc` e carrega `sc-faqs — 1306 registros`; "Como faço um pedido de TTD?" → **FAQ 1211 em 1º** (0,269); a
  pergunta de IPVA para filho autista → 1654 em 1º; "prazo da EFD em SC" → 523 em 2º, atrás de "quem é obrigado à
  EFD". O pack carrega `link …?numero=1211`.
- Falta conferir com o servidor de produção reiniciado: a aba FAQs da SC renderizando e o chat tipo 1 citando
  `FAQ n` com o link (o retrieve não chama o LLM).
