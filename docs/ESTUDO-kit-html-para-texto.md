# ESTUDO-kit-html-para-texto — um conversor HTML → texto único para as FAQs da frota

**Data:** 2026-11-07 (inventário do código e experimento local com os caches; nenhuma requisição de rede). ·
**Estado:** **implementado (07/11/2026)** — D-KITHTML-1..8 aprovadas pelo mantenedor (§10). · **Escopo:** a função `html_para_texto` dos scrapers de FAQ.
Substitui a nota "Candidata nova (02/11/2026)" do [auli_scraper_pendências.md](auli_scraper_pendências.md) §10,
que contava cinco cópias.

> **TL;DR — não são cópias: são 10 variantes, e a extração corrige defeitos que estão em produção hoje.**
>
> Há `html_para_texto` em 12 crates de FAQ (SP, GO, DF, AM, CE, MA, MS, MT, PR, SC, MG, ES) e no `tarf.rs` do RS.
> Só três são idênticas (DF, AM, ES); o resto diverge em até nove regras (§1).
>
> Experimento num laboratório isolado: os 12 scrapers rodando com `--usecache` sobre cópias dos caches, primeiro
> com o código de hoje (a **base reproduz a produção byte a byte nas 12**), depois chamando uma função única no kit.
> A candidata final muda **395 das 13.449 FAQs** (2,9 %), e cada mudança tem causa conhecida (§4): 318 deixam de
> repetir a URL quando o texto do link é a própria URL; 61 deixam de pôr entre aspas um `href` que não é endereço;
> 11 são espaçamento (4 com palavras que hoje saem coladas); 4 **recuperam texto que a produção perde hoje**; 1
> ganha a URL de uma imagem. **Nenhuma palavra visível se perde.** DF, AM, MT e ES saem idênticos.
>
> O experimento achou cinco defeitos na própria candidata antes do resultado final (§3). Dois deles já estão hoje
> em produção: o `<` que é texto apaga tudo até o próximo `>` (3 respostas do SC truncadas), e o `<CRT>` do CE some no
> decodificador. A função única corrige os dois.
>
> Recomendação: **extrair** `auli_scraper_kit::html::html_para_texto(s, Opcoes { base, quebras_do_fonte })` com as
> regras da §6 e aceitar as 395 mudanças, num PR que regera os 12 snapshots pelos caches (D-KITHTML-1..8).

---

## 1. Inventário (07/11/2026)

| Regra | DF · AM · ES | CE | SP | GO | MG | MS | PR | SC | MA | MT |
|---|---|---|---|---|---|---|---|---|---|---|
| tira `<style>`, `<script>`, `<svg>`, comentários | sim | sim | sim | sim | — | — | — | — | — | — |
| compacta o espaço do fonte antes das quebras | sim | sim (ou `\n`→`<br>` no modo `markdown`) | sim | sim | — | — | — | — | — | — |
| `<img src>` → `"url"` | sim, só `http(s)`/`mailto` | sim | sim | sim | — | — | sim | — | — | — |
| link sem endereço útil (`#`, `javascript:`, vazio) → só o texto | por esquema: só `http(s)`, `mailto` e caminho da raiz | por esquema | por prefixo | por prefixo | por prefixo | por prefixo | por prefixo | por prefixo | — (qualquer `href` entre aspas) | — |
| URL uma vez só quando o texto do link é a URL | sim | sim | — | — | — | — | — | — | — | — |
| caminho da raiz (`/x`) ganha o host | sim | sim | sim | sim | sim | — | só imagem | — | — | — |
| tags que quebram linha | a lista longa: `p div br hr li ul ol tr table tbody thead h1-6 blockquote` | a longa | a longa | a longa | a longa sem `blockquote` | a longa sem `blockquote` | `br center div hr li ol p ul` | `p br div li ul ol tr table h1-6` | `br div li ol p ul` | `br div li ol p ul` |
| célula de tabela → espaço | sim | sim | sim | sim | sim | sim | — | — | — | — |
| tira as tags com | `<[^>]+>` | idem | idem | idem | idem | idem | lista de 19 tags | `<[^>]+>` | `<[^>]*>` | `<[^>]*>` |
| decodifica as entidades com | html5ever | html5ever | próprio (`decode_html`) | html5ever | próprio (numéricas + kit) | html5ever | kit (tabela fixa) | kit | kit | kit |
| especial | — | modo `markdown` | decodifica `href`/texto antes | — | — | — | — | — | tira U+FEFF | — |

Fora do estudo: o `tarf.rs` do RS. É outro contrato (mantém uma linha em branco entre parágrafos e troca tag por
espaço), de uma coleção fechada em 09/08/2026 (D-KITHTML-7).

## 2. Método

- **Laboratório isolado** no scratchpad: uma `git worktree` do `main` (`042d56a`), uma cópia dos caches de FAQ das 12
  entidades (165 MB) e os binários rodando de um diretório cujo `../data` aponta para a cópia. O `data/` real não foi
  tocado, e não houve rede.
- **Base:** os 12 scrapers como estão, `--usecache faqs`. Os 12 snapshots saíram **iguais aos de produção**, a menos
  do `coletado_em`. Os caches de hoje reproduzem o acervo publicado, então o diff do experimento é o diff que o
  acervo teria.
- **Candidata:** um `kit::html` com a função única; em cada crate, o corpo do `html_para_texto` vira uma chamada a
  ela com o host que o crate já usa. Diff campo a campo contra a base.

## 3. O que as seis rodadas ensinaram

A 1ª candidata era a variante do DF tal como está. Ela mudava 1.238 respostas; quase todas eram defeitos dela, não
diferenças reais.

| Rodada | Achado | Correção na candidata |
|---|---|---|
| 1 | O SharePoint de SP e de GO escreve o `:` do link como entidade (`href="https&#58;//…"`): o filtro de esquema não reconhecia, e a URL sumia (SP 351 e GO 162 respostas mudadas, quase todas por isso). | O esquema e a comparação texto × URL olham o `href` e o texto **decodificados**; o que sai é o original, que o passo final decodifica uma vez só (decodificar duas vezes corromperia `&amp;region=` → `®ion=`). |
| 1 | GO tem links `ftp://` (4 respostas). | `ftp://` entra entre os esquemas aceitos. |
| 1 | No SC (428), MT (8), MS (1) e PR (1), a quebra de linha do fonte é estrutura (alíneas "a)", "b)"). Compactar o espaço as juntava. | Opção `quebras_do_fonte` (o modo `markdown` do CE) para essas quatro. |
| 2 | Com a opção ligada, a quebra **dentro** de uma tag (`<span\nstyle=…normal">`) virava `<br>` e quebrava a tag: lixo `normal">` em 4 respostas do SC. O modo `markdown` do CE tem o mesmo risco latente. | A quebra vira `<br>` só fora das tags. |
| 2 | O PR quebra linha em `<center>` (1). | `center` entra entre as tags de bloco. |
| 3 | O texto do PR e do SC tem `<` cru ("Contribuintes < Alteração Cadastral <", "(0) < Soma…"). A regex `<[^>]+>` apaga tudo até o próximo `>`, e **o SC de produção já perde isso hoje** (3 respostas truncadas). | A tag precisa de letra depois do `<`. |
| 4–5 | Mesmo poupado pela regex, um `<Palavra>` cru (`<Consulta Pendencias>` no PR, `<CRT>` no CE) é lido pelo html5ever como elemento desconhecido e some na decodificação. **O CE perde o `<CRT>` em produção hoje.** | Uma lista de nomes de tag HTML (mais `prefixo:nome`, para o `<o:p>` do Word) e o `<` que sobrar é escapado antes de decodificar. |
| 6 | Sem a regra "URL uma vez só", GO e MT zeram e SP, MS, PR e SC caem para 1–6: a atribuição se confirma. | — |

## 4. Resultado final (candidata da rodada 5 contra a base)

| UF | FAQs | mudam | URL uma vez só | `href` que não é endereço | espaçamento | `<` recuperado | imagem |
|---|---|---|---|---|---|---|---|
| SP | 2.665 | 96 | 90 | — | 6 | — | — |
| GO | 1.298 | 5 | 5 | — | — | — | — |
| DF | 993 | 0 | — | — | — | — | — |
| AM | 176 | 0 | — | — | — | — | — |
| CE | 2.706 | 2 | 1 | — | — | 1 | — |
| MA | 336 | 13 | 10 | 3 | — | — | — |
| MS | 1.200 | 144 | 139 | — | 5 | — | — |
| MT | 135 | 0 | — | — | — | — | — |
| PR | 1.321 | 49 | 49 | — | — | — | — |
| SC | 1.306 | 15 | 11 | — | — | 3 | 1 |
| MG | 310 | 71 | 13 | 58 | — | — | — |
| ES | 1.003 | 0 | — | — | — | — | — |
| **total** | **13.449** | **395** | **318** | **61** | **11** | **4** | **1** |

Nenhuma pergunta, trilha ou URL muda; só respostas. O que cada coluna é:

- **URL uma vez só.** Hoje SP, GO, MS, PR, SC, MG e MA escrevem `http://www.fazenda.pr.gov.br "http://www.fazenda.pr.gov.br"`.
  A candidata escreve `"http://www.fazenda.pr.gov.br"`, como DF, AM, CE e ES já fazem. É ruído a menos no texto
  vetorizado.
- **`href` que não é endereço.** O MG tem 58 links com `href="www.nfe.fazenda.gov.br"` (sem esquema). Hoje saem
  `www.nfe.fazenda.gov.br "www.nfe.fazenda.gov.br"`; na candidata fica só o texto do link. O MA tem 3 (um
  `href="Lei nº 7.799/2002,"`, um e-mail sem `mailto:`, um domínio sem esquema), que hoje saem entre aspas como se
  fossem URL.
- **Espaçamento.** SP 6 e MS 5: espaços em volta de aspas e `>`. Em 4 do SP e 3 do MS, palavras que hoje saem
  coladas ("ConvênioICMS", "peloDecreto") passam a sair separadas.
- **`<` recuperado.** Texto que a produção perde hoje e a candidata devolve: o `<CRT>` do CE e 3 respostas do SC
  (107 palavras, como "(0) < Soma Qtde de Saídas…"). O PR, que tem os mesmos casos ("< Alteração Cadastral <",
  `<Consulta Pendencias>`), não perde: a lista de tags dele os protege, e a candidata precisou chegar a esse resultado
  para não piorar o PR (§3).
- **Imagem.** O SC não emitia `<img>`; a candidata emite a URL de uma (`lh7-rt.googleusercontent.com`).

**Nenhuma palavra visível se perde.** Comparando as palavras de cada resposta alterada, fora as URLs, as únicas
"perdas" são as palavras coladas que passam a sair separadas e o texto dos `href` falsos do MA, que o portal não
mostra.

## 5. Por que extrair agora

O argumento da §10 era manutenção (cópias divergindo). O experimento acrescenta um argumento de correção: três dos
defeitos que a candidata precisou resolver estão nas variantes de produção hoje — o `<` que é texto (SC, 3 respostas),
o `<Palavra>` que some no decodificador (CE, 1) e a quebra dentro da tag no modo `markdown` (CE, latente: 0
respostas hoje) —, e cada FAQ nova copia uma variante. Com a função
única, a correção é feita uma vez, com teste, e vale para as 12 entidades e para as que vierem (PB, PA, RO, PI…).

## 6. Proposta

```rust
// auli_scraper_kit::html
pub struct Opcoes<'a> {
    /// Host para o caminho da raiz (`/x`) de link e imagem. `None`: o caminho da raiz vira só o texto.
    pub base: Option<&'a str>,
    /// A quebra de linha do fonte é estrutura e vira quebra (SC, MT, MS, PR; o modo `markdown` do CE).
    pub quebras_do_fonte: bool,
}
pub fn html_para_texto(s: &str, o: Opcoes) -> String;
/// O endereço que vale pôr no texto (decide pelo href decodificado; devolve o original). Público para o anexo do ES.
pub fn endereco(href: &str, base: Option<&str>) -> Option<String>;
```

Regras, na ordem: tira `<style>`, `<script>`, `<svg>` e comentários → com `quebras_do_fonte`, `\n` → `<br>` fora das
tags → compacta o espaço → `<img src>` → `"url"` → `<a href>texto</a>` → `texto "url"`, ou `"url"` quando o texto
(decodificado) é a URL, ou só o texto quando o `href` não é `http(s)`, `ftp`, `mailto` nem caminho da raiz → a
lista longa de tags de bloco + `center` → quebra; `td`/`th` → espaço → tira as tags de nome HTML conhecido (e
`prefixo:nome`) → escapa o `<` que sobrou → decodifica as entidades (html5ever), uma vez → tira U+FEFF → `clean`
por linha, linhas vazias fora.

| Entidade | `base` | `quebras_do_fonte` |
|---|---|---|
| SP, GO, SC, MG | o `BASE` do crate | SC: sim; os outros: não |
| DF | `RECEITA` ou `ISS`, como hoje | não |
| AM, CE | `am::BASE`, `ce::BASE` | CE: o modo `markdown` de hoje |
| ES | `HOST` | não |
| PR | `HOST` | sim |
| MA, MS, MT | o host do portal (hoje não absolutizam; 0 caminhos da raiz no acervo) | MS e MT: sim; MA: não |

O kit ganha `regex` e `scraper` como dependências. Nove dos 12 crates já dependem dos dois; SP, MT e SC passam a
levar o `scraper` (html5ever) pelo kit, e o binário deles cresce por isso. A
fronteira (`check-scraper-boundary.sh`) não muda: o kit continua só de `crates/scrapers/`.

## 7. Decisões propostas (D-KITHTML-*)

- **D-KITHTML-1 — Extrair** a função única para `auli_scraper_kit::html`, com as regras da §6, e apagar as 12 cópias
  (com os regexes e auxiliares que só elas usam).
- **D-KITHTML-2 — URL uma vez só, na frota toda** (318 respostas em 8 entidades). Alternativa: uma opção para
  manter a repetição onde ela existe hoje. Proponho não ter a opção: a repetição é ruído, e DF, AM, CE e ES já não
  a têm.
- **D-KITHTML-3 — `href` que não é endereço vira só o texto, na frota toda** (MG 58, MA 3).
- **D-KITHTML-4 — `quebras_do_fonte` por entidade:** SC, MT, MS, PR e o modo `markdown` do CE. Sem a opção, o SC
  perderia a estrutura de 428 respostas.
- **D-KITHTML-5 — Tag só com nome HTML conhecido, e o `<` que sobrar é texto.** Recupera o que SC e CE perdem
  hoje (4 respostas) e mantém o que o PR já preserva (`<Consulta Pendencias>`, "< Alteração Cadastral <"). O custo é uma lista de 94 nomes no kit.
- **D-KITHTML-6 — Imagem vira `"url"` na frota toda** (o SC ganha 1).
- **D-KITHTML-7 — O `tarf.rs` do RS fica de fora:** outro contrato (parágrafos separados por linha em branco), coleção
  fechada.
- **D-KITHTML-8 — Entrega num PR**, que regera os 12 snapshots pelos caches (`--usecache`), refaz a árvore `.md` das
  8 entidades com mudança e re-vetoriza só os registros alterados (o `build-packs` reaproveita os iguais). A prova
  é esta tabela: DF, AM, MT e ES saem byte a byte iguais, e as outras 8 com o diff da §4, causa por causa. **Pré-requisito:**
  nenhuma das 12 entidades ser recoletada antes do PR, porque os scripts de atualização apagam o cache e a base
  deixaria de ser reproduzível. Se alguma for, a base dela se refaz com o cache novo.

## 8. Plano do PR

1. `auli-scraper-kit/src/html.rs` com os testes: as regras da §6, uma por teste, e os casos reais do experimento
   (`https&#58;//`, `ftp://`, `<center>`, quebra dentro da tag, `< Alteração`, `<Consulta Pendencias>`, `<CRT>`,
   `<o:p>`). Os testes de conversor que hoje vivem em cada crate migram para lá ou passam a chamar o kit.
2. Os 12 crates chamam o kit (§6) e perdem a cópia local.
3. Laboratório refeito sobre o `main` do dia: base = produção; com o PR, o diff tem de bater com a §4.
4. `auli-collections` nas 8 entidades com mudança e `build-packs.sh <id> faqs` (~395 vetores); os artefatos de
   serviços com o mesmo md5.
5. O §10 do `auli_scraper_pendências.md` aponta para este estudo; a nota das "cinco cópias" sai.

## 9. Reprodução

```sh
S=<scratchpad>; X=$S/exp
git worktree add --detach $X/wt main
for e in sp go df am ce ma ms mt pr sc mg es; do
  mkdir -p $X/data/$e/raw/cache && cp -r data/$e/raw/cache/faqs $X/data/$e/raw/cache/
done
mkdir -p $X/srv   # os binários rodam daqui: ../data/<id>/raw = a cópia
(cd $X/wt/auli-server && CARGO_TARGET_DIR=$X/target cargo build -p auli-scraper-sp …)
(cd $X/srv && $X/target/debug/auli-scraper-sp --usecache faqs)   # grava em $X/data/sp/
# compare $X/data/<id>/<id>-faqs-snapshot.json com data/<id>/<id>-faqs-snapshot.json, sem o coletado_em
```

## 10. Implementação (07/11/2026)

D-KITHTML-1..8 como propostas. `auli-scraper-kit/src/html.rs` traz `html_para_texto(s, Opcoes { base,
quebras_do_fonte })` e `endereco(href, base)`, com as regras da §6 sem mudança. Cada um dos 12 crates troca a sua
versão por um invólucro de uma linha com as opções da tabela da §6, que mantém a assinatura de antes; as chamadas e os
testes dos crates não mudam. O anexo do ES passa a usar o `endereco` do kit.

- **Código:** +125 −626 linhas nos 12 crates e no kit, mais o `html.rs` novo (345 linhas, das quais 205 de testes). Saíram as 12 funções e os regexes e
  auxiliares que só elas usavam (`OCULTO_RE`, `LINK_RE`, `BLOCO_RE`…, `endereco`, `decodificar`, `absoluta`), a
  lista fechada de tags do PR (a regra dela foi para o kit, D-KITHTML-5) e cinco dependências que ficaram sem uso
  (`scraper` no AM e no CE; `regex` no MA, MS e MT). O kit ganhou `regex` e `scraper`.
- **Testes:** 15 no kit, um por regra, com os casos reais do experimento. Quinze mutações, uma por regra: todas
  derrubam o teste delas. Quatro testes não pegavam a mutação na primeira versão e foram reforçados: a quebra dentro
  de tag de bloco (`<p<br>style=` é o que vaza; com `<span>` a tag se recompõe), o `href` com o `:` codificado e o
  texto sem, o `<center>` no meio do texto corrido e o `<o:p>`. Nos crates, um teste mudou de expectativa: o do
  caminho de menu do PR, que fixava a URL repetida (D-KITHTML-2). No total, 336 testes passam.
- **Equivalência:** os binários do branch, no laboratório da §2, produzem os mesmos 12 snapshots e as mesmas 12
  árvores da candidata da rodada 5, byte a byte. O diff contra a base é a tabela da §4.
- **Regeração local:** os 8 snapshots que mudam (SP, GO, CE, MA, MS, PR, SC, MG) foram refeitos com `--usecache` sobre
  os caches de produção, e saíram iguais aos do laboratório. Depois vieram `auli-collections` e `build-packs.sh <id>
  faqs`. Foram calculados 388 vetores, não 395: no MS, 7 das 144 respostas mudam depois dos 6.000 caracteres da
  chave, que é o teto do vetor (o `build-packs` avisa "20 de 1200 keys no teto"), e reaproveitam o vetor. Os 21.259
  artefatos de serviços e pareceres dessas entidades saíram com o mesmo md5.
