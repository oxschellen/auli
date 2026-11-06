# TAREFA — FAQs do RS: raiz da coleta passa a `/perguntas-frequentes-antigo`

> Documento único para o Claude Code. Contém o contexto, o passo a passo e, no fim (Anexo), o patch na
> íntegra. Trabalhe **um passo de cada vez** e confirme comigo (Carlos) antes de qualquer push ou merge.

## 1. Contexto

Em nov/2026 o Portal de Serviços da Receita (`atendimento.receita.rs.gov.br`) trocou a página
`/perguntas-frequentes`, que era a raiz da caminhada do `auli-scraper-rs faqs`. A página nova é um
**índice** com as perguntas inline (~5 MB, 1.854 perguntas de 233 páginas, agrupadas em seis acordeões), e
**não tem mais `data-matriz-source-uri`** — o atributo pelo qual `faqs::get_page_info` reconhece um menu
(`categoriafaq`) ou uma lista de FAQ (`sanfona.comnavegacao`). Sem ele a raiz é classificada como `Geral`,
a caminhada não desce, e `faqs::run` aborta com "nenhuma FAQ colhida" (a guarda de 09/10/2026, que
preserva snapshot e árvore — comportamento correto).

Medições feitas em 06/11/2026 com `curl` (UA de navegador, HTTP 200 nos dois casos):

| Página | Tamanho | `data-matriz-source-uri` | `panel panel-default` |
| ------ | ------- | ------------------------ | --------------------- |
| `/perguntas-frequentes` (nova) | 5.050.984 B | **ausente** | 0 |
| `/perguntas-frequentes-antigo` | 55.156 B | `/_service/conteudo/pagedlistfilho?id=108&templatename=pagina.listapagina.categoriafaq.comnavegacao` | — |

Ou seja: o portal **preservou a raiz antiga** em `-antigo`, com o template `categoriafaq` intacto, e as
páginas-filhas (`/faq-ted`, `/faq-dc-e`, `/ipva-isencoes-faq`…) não mudaram de URL. O conserto mínimo é
apontar a raiz para lá. O índice novo é candidato a fonte futura (um GET em vez de ~500 requisições, e um
invariante dinâmico de graça: âncoras `#panel-N` == perguntas colhidas), mas perde a trilha do breadcrumb —
fica como estudo à parte, **fora desta tarefa**.

## 2. O que o patch faz

Um commit, três arquivos, sobre o `f214f35` (último commit do `main` no GitHub em 06/11/2026):

| Arquivo | Mudança |
| ------- | ------- |
| `auli-server/crates/scrapers/auli-scraper-rs/src/main.rs` | `root_url` → `…/perguntas-frequentes-antigo`, com comentário datado explicando o índice novo e o papel da guarda de zero FAQs |
| `auli-server/crates/scrapers/SCRAPERS.md` | Linha da fonte das FAQs do RS |
| `docs/REGISTRO-scrapers.md` | Seção nova **`#rs-faqs`** (as FAQs do RS nunca tiveram uma — são anteriores ao REGISTRO): fonte, o F5 de out/2026, a troca de raiz de nov/2026, guardas e transporte; índice e cabeçalho (9 → 10 FAQs) atualizados |

Nenhuma mudança de lógica, de contrato ou de testes. A troca de URL cria um arquivo de cache novo para a
raiz (`url_to_filename`); o cache das filhas continua válido.

## 3. O que já foi verificado fora daqui

Num clone limpo, o patch aplicado sobre o `f214f35` com `git am` produz a árvore
`a1b80372cb046a4f622bcf1b67d24f847fd4a500`. Nesse estado passaram: `cargo check -p auli-scraper-rs`,
`cargo test -p auli-scraper-rs` (36 testes), `scripts/tools/check-sem-navi.sh` e o `markdownlint` do
REGISTRO (os únicos avisos restantes — MD040/MD029 — são preexistentes, em seções de outros estados).

**Não verificado:** a coleta real. O ambiente que preparou o patch não alcança o portal; o passo 5 é a
prova de que a raiz `-antigo` caminha de ponta a ponta.

## 4. Passo a passo

### Passo 1 — Extrair o patch deste documento

O patch está no Anexo, num bloco cercado por `~~~~~patch <nome>`. Não copie à mão: extraia com o script
abaixo (ajuste o caminho deste arquivo) e confira o hash.

```bash
mkdir -p /tmp/rsfaq && python3 - <<'PY'
import re, pathlib, hashlib
doc = pathlib.Path("TAREFA-RS-FAQS-RAIZ.md").read_text(encoding="utf-8")
for nome, corpo in re.findall(r"^~~~~~patch (\S+)\n(.*?)^~~~~~$", doc, re.S | re.M):
    p = pathlib.Path("/tmp/rsfaq") / nome
    p.write_text(corpo, encoding="utf-8")
    print(hashlib.sha256(corpo.encode("utf-8")).hexdigest(), nome)
PY
```

O hash tem de ser exatamente este — se divergir, pare e me avise:

```text
fd82c3b36046fa9e0ab3ed01513691e9677adcf64303d3b6252b78978413a385  0001-rs-faqs-raiz-antigo.patch
```

### Passo 2 — Ver onde o repositório local está

```bash
cd ~/Desktop/auli
git status --short          # tem de estar limpo; se não estiver, pare e me pergunte
git log --oneline -3        # esperado: f214f35 no topo (feat(scraper-ma): as FAQs da SEFAZ-MA…)
```

Se o topo não for o `f214f35`, rode `git apply --check /tmp/rsfaq/0001-rs-faqs-raiz-antigo.patch` antes de
seguir; se não aplicar limpo, pare e me avise.

### Passo 3 — Aplicar numa branch

```bash
git checkout -b fix/rs-faqs-raiz-antigo
git am /tmp/rsfaq/0001-rs-faqs-raiz-antigo.patch
git rev-parse 'HEAD^{tree}'   # esperado: a1b80372cb046a4f622bcf1b67d24f847fd4a500
git show --stat HEAD          # 3 arquivos, 63 inserções, 3 remoções
```

### Passo 4 — Verificações locais

```bash
cd auli-server
cargo fmt --all -- --check
cargo test -p auli-scraper-rs            # 36 testes
cargo clippy -p auli-scraper-rs -- -D warnings
cd ..
scripts/tools/check-sem-navi.sh
scripts/tools/check-scraper-boundary.sh
markdownlint docs/REGISTRO-scrapers.md auli-server/crates/scrapers/SCRAPERS.md   # só MD040/MD029 preexistentes
```

Qualquer falha nova: pare e me mostre a saída.

### Passo 5 — A prova: coleta real

Antes de tudo, anote a linha de base — quantas FAQs o snapshot atual tem:

```bash
jq '.coleta.items | length' data/rs/rs-faqs-snapshot.json
```

Depois rode o script oficial de atualização **sem subir o servidor** e **sem apagar o cache** (a raiz nova
será um miss e virá da rede; as filhas vêm do cache, então a rodada é rápida e poupa o portal):

```bash
scripts/tools/update-rs-faqs.sh --keep-cache --no-serve
```

O que esperar:

- A primeira linha da coleta é
  `Walking site from: https://atendimento.receita.rs.gov.br/perguntas-frequentes-antigo`.
- O log lista as categorias de 1º nível (`Piloto RTC - IBS`, `1. ICMS (PJ e Produtor)`, `IPVA ou Veículos`,
  `ITCD (Herança ou Doação)`, `4. Taxas`, `5. Certidão / Protesto / Serasa`, `6. Nota Fiscal Gaúcha e
  Devolve ICMS`, `7. e-CAC, site e PPF`, `8. Prefeituras`) — são as que o portal mostra na sidebar
  "Seções" das páginas de categoria. Categorias que não existiam na última coleta aparecem como páginas
  novas (miss → rede).
- `🔢 FAQs no snapshot: <DEPOIS> (antes: <ANTES>)` com `DEPOIS >= ANTES`. O script tem guarda de
  encolhimento: se encolher, ele para sozinho e mantém o `.anterior` — nesse caso **não** force; me mande
  o log (procure `Error walking` / `Error fetching body` / `o F5 devolveu`).
- Se aparecer `o F5 devolveu desafio/bloqueio no lugar da página` na raiz: o F5 passou a barrar também as
  páginas (até agora barrava só o `_service`). Isso muda o problema e a conversa — pare e me avise.

Se der tudo certo, o script já derivou a árvore `docs/faqs/*.md` e re-vetorizou os packs do RS. Me
reporte `ANTES` → `DEPOIS` e o número de vetores reaproveitados que o `build-packs.sh` imprimiu.

### Passo 6 — Subir o servidor e conferir a aba de FAQs

```bash
./start_server.sh --no-build
```

Na aba de FAQs da entidade `rs` no frontend, a árvore tem de mostrar as 9 categorias de 1º nível acima.
Depois, se quiser, uma pergunta de controle no chat (tipo 1, Serviços + FAQs) sobre DC-e ou GIA-Automática
— assuntos que estão entre os "artigos novos" do portal.

### Passo 7 — Push e PR (só depois do meu OK)

```bash
git push -u origin fix/rs-faqs-raiz-antigo
gh pr create --title "fix(scraper-rs): raiz das FAQs passa a /perguntas-frequentes-antigo" --body-file - <<'EOF'
Desde nov/2026 `/perguntas-frequentes` é um índice novo sem `data-matriz-source-uri` (a caminhada o
classificava como `Geral` e colhia zero FAQs — a guarda de 09/10 preservava snapshot e árvore). O portal
preservou a raiz `categoriafaq` em `/perguntas-frequentes-antigo`; as páginas-filhas não mudaram de URL.

- `main.rs`: `root_url` → `-antigo`, com comentário datado.
- `SCRAPERS.md`: linha da fonte.
- `REGISTRO-scrapers.md`: seção `#rs-faqs` (a primeira das FAQs do RS): fonte, F5 de out/2026, troca de
  raiz de nov/2026, guardas e transporte.

Coleta real: ANTES → DEPOIS FAQs (preencher).

Fora deste PR: o índice novo como fonte (1.854 perguntas inline em um GET) — estudo à parte.
EOF
```

Substitua "ANTES → DEPOIS" pelos números do passo 5. **Não faça merge**: eu reviso e mergeio.

## 5. Fora do escopo (não faça)

- Não adapte o scraper ao índice novo, nem adicione fallback automático raiz-nova → `-antigo`.
- Não troque o UA para `AuliBot` neste host (D-FROTA-UA: só com recoleta verificada, e não é esta).
- Não apague o cache de FAQs (`--keep-cache` é deliberado nesta rodada).
- Não mexa em serviços, pareceres ou TARF do RS.

## Anexo — o patch

~~~~~patch 0001-rs-faqs-raiz-antigo.patch
From 99f3fe460c98adc97ee47ec74ab1829d18e7678d Mon Sep 17 00:00:00 2001
From: Claude <noreply@anthropic.com>
Date: Fri, 6 Nov 2026 18:34:36 -0300
Subject: [PATCH] fix(scraper-rs): raiz das FAQs passa a
 /perguntas-frequentes-antigo
MIME-Version: 1.0
Content-Type: text/plain; charset=UTF-8
Content-Transfer-Encoding: 8bit

Desde nov/2026 /perguntas-frequentes é um índice novo sem data-matriz-source-uri
(a caminhada o classificava como Geral e colhia zero FAQs). O portal preservou a
raiz categoriafaq em -antigo; as páginas-filhas não mudaram de URL.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_0136kJBgDPKz97iyWMkVj7Zs
---
 auli-server/crates/scrapers/SCRAPERS.md       |  4 +-
 .../scrapers/auli-scraper-rs/src/main.rs      |  6 +-
 docs/REGISTRO-scrapers.md                     | 56 ++++++++++++++++++-
 3 files changed, 63 insertions(+), 3 deletions(-)

diff --git a/auli-server/crates/scrapers/SCRAPERS.md b/auli-server/crates/scrapers/SCRAPERS.md
index 8c6fad0..72fc83b 100644
--- a/auli-server/crates/scrapers/SCRAPERS.md
+++ b/auli-server/crates/scrapers/SCRAPERS.md
@@ -155,7 +155,9 @@ frequência (cortesia entre fetches). São catálogos públicos, coleta rara.
 - **FAQs e pareceres** além de serviços (`auli-scraper-rs [--usecache] faqs|servicos|pareceres`).
 - **Serviços:** API JSON do Tudo Fácil (`fazenda.rs.gov.br/_service/tudofacil/capaservicos`) — não
   precisa mais de headless Chrome (era o único que usava).
-- **FAQs:** `atendimento.receita.rs.gov.br/perguntas-frequentes`, via AJAX.
+- **FAQs:** `atendimento.receita.rs.gov.br/perguntas-frequentes-antigo`, via AJAX (raiz `categoriafaq`
+  preservada pelo portal; desde nov/2026 `/perguntas-frequentes` é um índice novo com as perguntas inline,
+  candidato a fonte futura).
 - **Pareceres** (Consultas Formais Respondidas): Portal de Legislação (`legislacao.sefaz.rs.gov.br`,
   ASP.NET WebForms/IIS, windows-1252) via **curl subprocess** (ureq recebe resposta degradada; cookie
   de sessão obrigatório, senão o servidor pendura). O filtro "Consultas Formais Respondidas" é **estado
diff --git a/auli-server/crates/scrapers/auli-scraper-rs/src/main.rs b/auli-server/crates/scrapers/auli-scraper-rs/src/main.rs
index 6bbf018..db33c40 100644
--- a/auli-server/crates/scrapers/auli-scraper-rs/src/main.rs
+++ b/auli-server/crates/scrapers/auli-scraper-rs/src/main.rs
@@ -76,7 +76,11 @@ fn run_faqs(use_cache: bool) -> errors::Result<()> {
     faqs::run(&faqs::FaqSource {
         id: ENTITY.to_string(),
         base_url: "https://atendimento.receita.rs.gov.br".to_string(),
-        root_url: "https://atendimento.receita.rs.gov.br/perguntas-frequentes".to_string(),
+        // Desde nov/2026 `/perguntas-frequentes` é um índice novo (sem `data-matriz-source-uri`, sem
+        // painéis: ~1.850 perguntas inline em ~230 páginas) — a caminhada o classifica como `Geral` e
+        // colhe zero. O portal preservou a raiz `categoriafaq` em `-antigo`; as páginas-filhas não
+        // mudaram de URL. Se o `-antigo` sumir, a guarda de zero FAQs em `faqs::run` é quem avisa.
+        root_url: "https://atendimento.receita.rs.gov.br/perguntas-frequentes-antigo".to_string(),
         root_title: "Perguntas Frequentes".to_string(),
         data_dir: DATA_DIR.to_string(),
         cache_dir: format!("{}/cache/faqs", DATA_DIR),
diff --git a/docs/REGISTRO-scrapers.md b/docs/REGISTRO-scrapers.md
index 4f81d00..421b318 100644
--- a/docs/REGISTRO-scrapers.md
+++ b/docs/REGISTRO-scrapers.md
@@ -1,7 +1,7 @@
 # REGISTRO-scrapers — como cada portal foi investigado
 
 > **Documento de MANUTENÇÃO, não de arquivo.** Um relatório por fonte: 14 de **serviços** por
-> entidade + 1 de **pareceres** (RS), o **TARF** e 9 de **FAQs** (SC, PR, SP, MG, MS, MT, GO, DF e AM). Cada seção registra como o portal foi investigado, o que ele
+> entidade + 1 de **pareceres** (RS), o **TARF** e 10 de **FAQs** (RS, SC, PR, SP, MG, MS, MT, GO, DF e AM). Cada seção registra como o portal foi investigado, o que ele
 > tem de peculiar e por que o scraper faz o que faz — coisas que o CÓDIGO usa mas não explica, e que
 > não estão documentadas em lugar nenhum do lado de lá.
 >
@@ -32,6 +32,7 @@
 - [TO — Carta de Serviços SEFAZ-TO (servicos.to.gov.br)](#to)
 - [RS Pareceres — Portal de Legislação / Consultas Formais Respondidas](#rs-pareceres)
 - [TARF — Tribunal Administrativo de Recursos Fiscais do RS (tarf.sefaz.rs.gov.br)](#tarf)
+- [RS FAQs — Perguntas Frequentes do Portal de Serviços da Receita (atendimento.receita.rs.gov.br)](#rs-faqs)
 - [SC FAQs — Perguntas Frequentes da SEF-SC (www.sef.sc.gov.br/perguntas)](#sc-faqs)
 - [PR FAQs — Perguntas Frequentes do SAC da SEFA-PR (atendimento.fazenda.pr.gov.br/sacsefa)](#pr-faqs)
 - [SP FAQs — Perguntas Frequentes da SEFAZ-SP (listas SharePoint por subsite)](#sp-faqs)
@@ -1719,6 +1720,59 @@ citável pelo chat; tirá-lo é decisão humana (apagar o `.md` e rodar o `build
 
 ---
 
+<a id="rs-faqs"></a>
+
+# RS FAQs — Perguntas Frequentes do Portal de Serviços da Receita (atendimento.receita.rs.gov.br)
+
+**Scraper:** coleção `faqs` do `auli-scraper-rs` (`auli-scraper-rs faqs`, ou `all` = FAQs + serviços), a
+primeira coleção de FAQs do acervo — anterior ao REGISTRO, e por isso sem seção até a troca de raiz de
+06/11/2026, registrada aqui. Módulo `src/faqs/` (`mod.rs` caminha, `fetch.rs` busca, `html.rs` extrai).
+
+## Fonte
+
+Portal "Matriz" da PROCERGS. Cada página declara em `data-matriz-source-uri` a chamada AJAX que traz o seu
+miolo (`/_service/conteudo/pagedlistfilho?id=<N>&templatename=<template>`), e o **template** diz o que a
+página é: `pagina.listapagina.categoriafaq[.comnavegacao]` = **menu** (filhos = categorias ou listas),
+`pagina.listapagina.sanfona.comnavegacao` = **lista de FAQ** (sanfona de pergunta/resposta); qualquer outro
+template é `Geral` (nó sem filhos e sem itens). O scraper lê a raiz, pede o AJAX com `currentPage=1&pageSize=100`,
+fatia o `body` (JSON) por `class="panel panel-default"` e desce recursivamente pelos menus; a trilha (`origin`)
+vem do breadcrumb da página-folha. ~250 páginas + ~250 chamadas AJAX por rodada.
+
+## As armadilhas
+
+1. **O F5 na frente do `_service/*` (desde out/2026)** rejeita o ureq pela impressão do cliente e devolve o
+   desafio JS do Bot Defense **com HTTP 200** (`window["bobcmn"]`/TSPD); o bloqueio do ASM é
+   `<title>Request Rejected</title>`. Por isso o AJAX vai por `get_via_curl` com o `Accept` de um `$.getJSON`
+   (`application/json, text/javascript, */*; q=0.01`), as páginas seguem no ureq, e `f5::bloqueio_f5` impede
+   que qualquer uma das duas páginas do F5 entre no cache (cache-first SEMPRE: gravada uma vez, envenenaria
+   todas as rodadas seguintes). As páginas de conteúdo ainda passam com UA de navegador (D-FROTA-UA: legado,
+   nunca testado com `AuliBot` neste host).
+2. **A raiz mudou de template em nov/2026.** `/perguntas-frequentes` virou um **índice novo**: sem
+   `data-matriz-source-uri`, sem `panel panel-default`, ~5 MB com **1.854 perguntas de 233 páginas inline**
+   (links `/<página>#panel-<N>`, 2.701 "Publicação:") agrupadas em seis acordeões (`#abx-icms`, `-ipva`,
+   `-itcd`, `-taxas`, `-nfg`, `-ecac`). A caminhada o classifica como `Geral` e colhe **zero** — o que a
+   guarda de `faqs::run` transforma em erro sem tocar snapshot nem árvore. O portal preservou a raiz antiga em
+   **`/perguntas-frequentes-antigo`** (`id=108&templatename=pagina.listapagina.categoriafaq.comnavegacao`), e
+   as páginas-filhas não mudaram de URL; a raiz do scraper passou a apontar para lá (06/11/2026). "-antigo" é
+   nome de página em despedida: se sumir, a mesma guarda avisa. O índice novo é candidato a fonte futura (um
+   GET em vez de ~500 requisições; invariante dinâmico de graça: âncoras `#panel` == perguntas colhidas), mas
+   perde a trilha do breadcrumb — é estudo à parte, não patch.
+3. **Queda parcial é silenciosa.** Erro ao caminhar uma página vira `eprintln!` e o laço segue; falha no AJAX
+   devolve nó vazio — uma categoria que caia leva a subárvore inteira. Quem pega isso é a guarda de
+   encolhimento de `scripts/tools/update-rs-faqs.sh` (compara com o `.anterior`).
+
+## Guardas e transporte
+
+- `Err` sem gravar nada: zero FAQs na caminhada; página ou AJAX que devolva o desafio/bloqueio do F5.
+- Cache-first sempre (`data/rs/raw/cache/faqs/`, página e `_body` por URL); `--usecache` = miss vira erro.
+  Trocar a raiz cria um arquivo de cache novo para ela; o das filhas continua válido.
+- `retry` 3× com backoff; AJAX por `curl`, páginas por ureq; UA de navegador nos dois.
+- Saída: `data/rs/raw/rs-faqs-tree.json` (a árvore que a aba de FAQs consome — único artefato sem outra
+  fonte) e `data/rs/rs-faqs-snapshot.json`, nessa ordem e atômicos.
+- Atualização: `scripts/tools/update-rs-faqs.sh` (apaga o cache antes; guarda de encolhimento).
+
+---
+
 <a id="sc-faqs"></a>
 
 # SC FAQs — Perguntas Frequentes da SEF-SC (www.sef.sc.gov.br/perguntas)
-- 
2.43.0

~~~~~
