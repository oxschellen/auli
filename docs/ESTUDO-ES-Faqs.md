# ESTUDO-ES-Faqs — Receita Orienta da SEFAZ-ES (coleção `faqs` da entidade `es`)

**Data:** 2026-11-07 (sondagem direta, UA AuliBot, cortesia de 1 s, ~60 GETs). · **Estado:** **proposta v1 — as
1.003 perguntas da navegação do portal Receita Orienta** (§3), aguardando decisão. · **Órgão:** Secretaria de Estado
da Fazenda do Espírito Santo (SEFAZ-ES). · **Hoje a `es` tem só `servicos`** (X-Via, 45 itens —
[REGISTRO-scrapers.md#es](REGISTRO-scrapers.md#es)).

> **TL;DR — a FAQ do ES é um portal próprio, o Receita Orienta, em HTML servido pronto.**
>
> `s1-internet.sefaz.es.gov.br/ReceitaOrienta` (ASP.NET; o link "Perguntas Frequentes" do site institucional
> chega aqui por redirecionamento) organiza **1.003 perguntas** em 6 categorias → 46 assuntos → (151 subassuntos)
> → pergunta. Cada assunto é **uma página** com o acordeão inteiro: pergunta, resposta e um id numérico estável por
> pergunta, com "Atualizado em" no fim de 645 respostas (652 alteradas em 2026). Sem token, sem JavaScript para o
> conteúdo, TLS com cadeia completa: **52 GETs por rodada** (6 categorias + 46 assuntos, 3,7 MB), `ureq` puro.
>
> A busca do portal (`BuscaV2`) devolve um JSON com 1.176 perguntas, mas **não é a fonte**: 173 delas estão em
> assuntos e subassuntos que o portal tirou do ar (a página do assunto responde "Você não deveria estar aqui"), e o
> JSON traz o login do servidor que cadastrou cada pergunta (§4).
>
> Recomendação: **v1 = a navegação** (D-ESFAQ-1, -2), trilha `Categoria | Assunto | Subassunto` (D-ESFAQ-3), link por
> pergunta com âncora no cabeçalho (D-ESFAQ-4).

---

## 1. O que foi avaliado (07/11/2026)

Todas as sondagens com `AuliBot/0.1 (+https://github.com/oxschellen/auli)`, pelo `curl`, ~1 s entre requisições.

| Fonte / rota | Veredito | Evidência |
|---|---|---|
| `sefaz.es.gov.br` (institucional) | **só aponta** para o Receita Orienta | menu: "Perguntas Frequentes" → `s1-internet…/faleconosco` (→ `/ReceitaOrienta`); "Perguntas e Respostas" → `…/faleconosco/categoria/Assunto/781/Cidadão/21` (rota antiga, abre o assunto ITCMD) |
| `s1-internet.sefaz.es.gov.br/robots.txt` | **inexistente** — 404 do IIS | (o `sefaz.es.gov.br` também: 404). D-FROTA-ROBOTS de qualquer forma |
| **Home** `/ReceitaOrienta/` | lista as 6 categorias e o contador "Nossos Números" (**1.177 perguntas**, 44.517 atendimentos, 90,32 % de satisfação) | **17,8 MB** — imagens dos cards em base64 → **não entra na rodada** (D-ESFAQ-7) |
| **Categoria** `/ReceitaOrienta/categoria/{id}` | **fonte v1** — a barra de menu lista as 6 categorias; o corpo lista os assuntos (`href=Assunto/{id}/{Categoria}/{catId}`) | 6 páginas, 12–36 KB |
| **Assunto** `/ReceitaOrienta/categoria/Assunto/{id}/{Categoria}/{catId}` | **fonte v1** — acordeão completo, servido pronto | 46 páginas, 18–241 KB, todas 200 |
| Assunto fora da navegação (ex.: `Assunto/921/ICMS/61`, "CT-e") | **despublicado** | 200 com "Erro: Você não deveria estar aqui." e 0 perguntas |
| **Busca** `/ReceitaOrienta/BuscaV2?conteudo=…&start=&length=` | **não é fonte** (§4) — HTML com um JSON embutido (`var modelo = {…}`) | `recordsTotal` 2.076; `conteudo=a` → 1.176 (`flagInternet=1`) em 58 assuntos; vazio → 0 |
| Anexo `/ReceitaOrienta/Categoria/DownloadAnexo/{id}` | **link, não conteúdo** | ex.: 8256 = `.doc` de 48 KB (anexo do Convênio ICMS 38/2012), `Content-Disposition: attachment` |
| `/ReceitaOrienta/formulario` | fora — formulário "Fale conosco" | — |
| TLS | `ureq`/rustls serve | TLS 1.2, `ECDHE-RSA-AES128-GCM-SHA256`, o servidor manda leaf + Sectigo OV R36 (cadeia completa) |
| Headless | desnecessário | todo o conteúdo vem no HTML |

**Sobreposição com `servicos`:** nenhuma. Os serviços do ES vêm do catálogo X-Via (`portal.es.gov.br`). As respostas
do Receita Orienta às vezes linkam a Carta de Serviços, mas não estão no acervo.

## 2. Estrutura da página de assunto

```text
#accordion
├─ heading{ID}_faq   .pPergunta = pergunta            ← pergunta direto no assunto (328)
│  └─ collapse{ID}_faq  .card-body.user-content = resposta   [.card-footer → link "Anexo"]
└─ heading{SID}_sa   .pPergunta = nome do subassunto  ← seção (151; nenhuma vazia)
   └─ collapse{SID}_sa
      └─ #accordion_faq
         └─ heading{ID}_saf  .pPergunta = pergunta    ← pergunta dentro do subassunto (675)
            └─ collapse{ID}_saf  .card-body.user-content = resposta   [.card-footer → "Anexo"]
```

- **{ID} é o `idFaq` do banco** e é único no portal inteiro: 1.003 ids, nenhum repetido entre páginas. Batem 1:1 com
  o JSON da busca (`_faq` ↔ `idSubAssunto` nulo, 328 de 328; `_saf` ↔ `idSubAssunto` = o `{SID}` da página, 675 de
  675).
- Só dois níveis: nenhum `_saf` tem filho; cada resposta é exatamente um `.card-body.user-content` (1.003 corpos).
- As respostas são HTML simples: `<div>`/`<p>`, `<br>`, `<hr>` como separador (616), `<b>`, `<a>` (291). Não há tabela,
  imagem, lista nem iframe. "Atualizado em dd/mm/aaaa" aparece em 645 respostas; em 536 é a última linha.
- **Anexos** ficam fora do corpo, num `.card-footer` com `<a href='/ReceitaOrienta/Categoria/DownloadAnexo/{id}'>`
  (aspas simples, caminho relativo): **15**, em 10 assuntos. Uma resposta depende dele ("Em anexo está o laudo para
  preenchimento").

**Números (07/11/2026):**

| Categoria | Assuntos | Perguntas |
|---|---|---|
| ICMS | 41 | 874 |
| Nota Premiada Capixaba | 1 | 42 |
| IPVA | 1 | 35 |
| ITCMD | 1 | 32 |
| Reforma Tributária | 1 | 18 |
| Agendar Atendimento | 1 | 2 |
| **total** | **46** | **1.003** |

Resposta (texto, espaços comprimidos): mín. 4 caracteres ("Não."), mediana 473, máx. 4.757; 0 vazias. Títulos: 982
começam por número de ordem ("01.", "a)", "h.3)"); 166 não terminam em "?" (cabeçalhos de procedimento, ex.: "b) Para
empresas estabelecidas no ES e já registradas na JUCEES"); **10** repetem um título da mesma página, sempre em
subassunto diferente (ex.: "a) Quem pode requerer a isenção?" em "Da isenção para pessoas com deficiência" e em "Da
isenção para taxistas"). Última alteração (pelo JSON da busca): 652 em 2026, 48 em 2025, 75 em 2024, as demais entre
2012 e 2023.

## 3. Modelagem v1

Semente: a página de uma categoria (a barra de menu de qualquer categoria lista as 6 — D-ESFAQ-7) → as 6 categorias →
os assuntos de cada uma → cada página de assunto → acordeão. Árvore DOM (`scraper`/html5ever, que a crate já usa nos
serviços).

| `FaqRaw` | origem |
|---|---|
| `pergunta` | o `.pPergunta` do cabeçalho, como na página — com o número de ordem (D-ESFAQ-5) |
| `resposta` | o `.card-body.user-content` em texto (D-ESFAQ-6); com anexo, uma linha final `Anexo: <url absoluta>` |
| `origin` | `Categoria \| Assunto` ou `Categoria \| Assunto \| Subassunto`, com a categoria omitida quando tem o mesmo nome do assunto (D-ESFAQ-3) |
| `url` | a URL da página do assunto, como a categoria a escreve, + `#heading{ID}_faq` ou `#heading{ID}_saf` (D-ESFAQ-4) |

Ex.: `ICMS | AC- Aviso de Cobrança | 1- Aviso de Cobrança "ICMS declarado e não recolhido"` · `IPVA | 05. Da isenção
para pessoas com deficiência ou autismo` · link
`https://s1-internet.sefaz.es.gov.br/ReceitaOrienta/categoria/Assunto/101/ICMS/61#heading9000_saf`.

**Nomes dos `.md` únicos.** O nome é `slug(pergunta)-hash(link + pergunta)` (`mddoc::nome_faq`); com a âncora no link,
os 10 títulos repetidos caem em arquivos distintos sem regra própria.

**Árvore (`faqs-tree.json`).** Categoria → Assunto → Subassunto → perguntas, com o mesmo colapso da trilha (as 5
categorias de assunto único viram um nó só).

**Guardas (`Err`, antes de gravar qualquer coisa):** a semente não lista categoria nenhuma; categoria sem assunto;
página de assunto sem `#accordion`, com "Você não deveria estar aqui" ou sem pergunta; subassunto sem pergunta;
cabeçalho sem `collapse` correspondente ou sem `.user-content`; resposta vazia; `ID` repetido; total abaixo do piso
(proposta: 900, ~90 % de hoje). O `update-es-faqs.sh` guarda o snapshot `.anterior` e compara, como os irmãos.

## 4. Fora do escopo v1

- **A busca `BuscaV2` como fonte.** Seria 1 GET em vez de 52, mas: (1) só traz tudo com um termo que case quase todo
  texto (`a`), um truque que depende da semântica da busca; (2) inclui **155 perguntas de 12 assuntos despublicados**
  (versões antigas de AGV, MEI e DOT, COVID, Lei de Multas 2020, CT-e, BP-e, DIEF, PED, Documentos Fiscais, CSRT,
  Opção Simples Nacional 2027) e **18 de 4 subassuntos ocultos** em assuntos publicados (e-commerce no COMPETE-ES, 8;
  dois passo a passo do ITCMD, 6; um do Simples Nacional, 4) — o JSON não diz o que está fora do ar; (3) não traz o nome
  do subassunto; (4) traz `nomeLogin`, o login do servidor que cadastrou a pergunta. Fica como **referência de
  conferência** manual, não como guarda: a diferença 1.176 − 1.003 é editorial e muda sem aviso.
- **O contador da home (1.177).** Conta também o que está fora da navegação; e a home tem 17,8 MB.
- **Conteúdo dos anexos** (`.doc`/PDF) — só o link (D-ESFAQ-6).
- **Agendar Atendimento** entra (2 perguntas do próprio portal); o formulário "Fale conosco", não.

## 5. Decisões propostas (D-ESFAQ-*)

- **D-ESFAQ-1 — Fonte = a navegação em HTML** (categoria → assunto), não a `BuscaV2` (§4). É o que o contribuinte vê
  navegando, e cada página traz a estrutura inteira.
- **D-ESFAQ-2 — Escopo = o que a navegação mostra: 1.003 perguntas.** Assuntos despublicados e subassuntos ocultos
  ficam fora.
- **D-ESFAQ-3 — Trilha** `Categoria | Assunto [| Subassunto]`, omitindo a categoria quando o nome dela é o do assunto
  (IPVA, ITCMD, Nota Premiada Capixaba, Reforma Tributária, Agendar Atendimento). O subassunto é o que desambigua os
  títulos repetidos e situa os cabeçalhos sem "?".
- **D-ESFAQ-4 — Link** = página do assunto + `#heading{ID}_{faq|saf}`. A âncora vai no cabeçalho, que está visível; o
  `collapse` está fechado e o navegador não rola até ele (precedente: `#collapse<id>` do DF).
- **D-ESFAQ-5 — Pergunta como na página**, com o número de ordem (precedente: PE). Os 166 títulos sem "?" entram: são
  itens do acordeão com resposta própria.
- **D-ESFAQ-6 — Resposta em texto pelo `html_para_texto` na variante do DF** (link → `texto "url"` só para
  `http(s)`/`mailto`, sem repetir a URL quando ela é o próprio texto; bloco e `<hr>` → quebra de linha; entidades no
  fim). "Atualizado em" fica: é a única data visível da resposta. Anexo → linha final `Anexo: <url absoluta>`, sem
  baixar o arquivo. Seria a 7ª cópia do conversor (SP, GO, DF, AM, CE, MA); a extração para o kit segue como item
  próprio ([auli_scraper_pendências.md](auli_scraper_pendências.md) §10), fora deste PR.
- **D-ESFAQ-7 — Semente = `categoria/61` (ICMS)**, não a home: a barra de menu de qualquer categoria lista as 6
  (82, 61, 81, 62, 83, 63, na ordem da home), e a home custa 17,8 MB. Se a 61 sumir, a coleta falha alto.
- **D-ESFAQ-8 — Transporte:** UA AuliBot (D-FROTA-UA), 1 s entre os 52 GETs, sem paralelismo, `ureq`/rustls (cadeia
  completa, GCM). Subcomando `auli-scraper-es faqs` / `all`, sempre à rede, cache em `data/es/raw/cache/faqs/` só com
  `--usecache`, como as FAQs da frota.

## 6. Plano de implementação

1. `auli-scraper-es/src/faqs.rs`; `main.rs` aceita `servicos | faqs | all`. → 52 GETs por rodada.
2. Semente → categorias → assuntos → acordeão; texto da resposta; anexo; trilha com colapso; link com âncora; guardas.
   → testes sobre recortes reais: uma página com `_faq` e `_sa` misturados (Cooperação Fiscal, NFC-e), uma com título
   repetido em subassuntos (AC- Aviso de Cobrança), uma com anexo (Isenção - aquisição de veículo novo), a página
   "Você não deveria estar aqui" e uma categoria de assunto único. Cada regra desligada tem de fazer o teste dela falhar.
3. Registry (`collections = ["servicos", "faqs"]`) + `entities.ts` (pelo gerador); `scripts/tools/update-es-faqs.sh`;
   REGISTRO#es-faqs, SCRAPERS.md, README.
4. `auli-collections es` → 1.003 `.md` com nomes únicos, artefatos de serviços idênticos; `build-packs.sh es faqs`;
   smoke `/v1/retrieve`.

## 7. Reprodução

```sh
UA='AuliBot/0.1 (+https://github.com/oxschellen/auli)'; H=https://s1-internet.sefaz.es.gov.br/ReceitaOrienta
curl -sS -A "$UA" -o /dev/null -w '%{http_code}\n' https://s1-internet.sefaz.es.gov.br/robots.txt   # 404
curl -sS -A "$UA" -o cat61.html "$H/categoria/61"                                     # 200, ~36 KB, 41 assuntos
grep -o 'href="/ReceitaOrienta/categoria/[0-9]*"' cat61.html | sort -u                # as 6 categorias
curl -sS -A "$UA" -o cad.html "$H/categoria/Assunto/101/ICMS/61"                      # 200, ~142 KB
grep -oE 'id=heading[0-9]+_(faq|sa|saf)\b' cad.html | sed 's/.*_//' | sort | uniq -c  # 11 sa, 37 saf
curl -sS -A "$UA" "$H/categoria/Assunto/921/ICMS/61" | grep -o 'deveria estar aqui'    # assunto despublicado
```
