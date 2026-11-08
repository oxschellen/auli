# ESTUDO-MT-Faqs — Perguntas Frequentes da SEFAZ-MT (coleção `faqs` da entidade `mt`)

**Data:** 2026-11-02 (descoberta) · revisado em 2026-11-02 após sondagem (§5.1). · **Estado:** **v1 implementada
— FAQ do Nota MT, API JSON anônima, 135 perguntas** (§2, §8). **Fase 2 sondada e implementada em 07/11/2026 (§9,
§10): as páginas de perguntas das áreas do site da SEFAZ (48) e o Portal MT (11), 194 perguntas no MT** — a
hipótese da §2b (os PDFs "FAQ <tema>") não se confirmou como fonte principal. · **Órgão:** Secretaria de Estado de Fazenda de Mato Grosso
(SEFAZ-MT). · **Hoje a `mt` tem só `servicos`** (X-Via, 27 itens, descrição rica; recoletado sem cache em 02/11).

> **TL;DR — duas fontes de FAQ de verdade, de formas opostas.**
>
> 1. **Nota MT** (`www.sefaz.mt.gov.br/notamt/duvida`, SPA Angular): a página lê **`GET /notamt/api/duvidas`** —
>    JSON anônimo, **uma chamada, 135 perguntas** em 9 grupos (Programa, Cadastro, Prêmios e Entidades, Bilhetes e
>    Sorteios, Pagamento dos Prêmios, Empresas, Menor Preço, Doe sua Nota, **Desconto IPVA**). O host não tem
>    `robots.txt` alcançável. É a fonte v1 — mais limpa que qualquer PDF; molde MS/AL (API JSON) em versão mínima.
> 2. **Site institucional Liferay** (`www5.sefaz.mt.gov.br`): hub "Perguntas frequentes" (`/-/perguntas-frequentes-1`)
>    e PDFs "FAQ Cadastro" e "FAQ ITCD" (Q&A numerado) na pasta `6071037/8241275`. `robots.txt` = `User-agent: *` /
>    `Disallow: /` (Google e Bing liberados) ⇒ **decisão do mantenedor** (D-MTFAQ-3) antes de qualquer coleta.
>    Tamanho desconhecido; scraper seria "`pdftotext` + parser de numeração" (precedente: pareceres do PR).
>
> O catálogo X-Via (`portal.mt.gov.br`) **não** tem FAQ (verificado na resposta de 02/11). O
> `conecta.sefaz.mt.gov.br` é um ServiceNow CSM (o Nota MT carrega dele o widget `sn_csm_ec.js`), com
> `Disallow: /` — candidato a base de conhecimento como a do MG, **não sondado**. Uma resposta do Nota MT linka
> um "Portal do Conhecimento" CITSmart em `sac.sefaz.mt.gov.br` — outro candidato, **não sondado**.

---

## 1. Hierarquia discovery-first — o que foi avaliado

| Fonte / rota | Veredito | Evidência (02/11/2026) |
|---|---|---|
| **Nota MT** `https://www.sefaz.mt.gov.br/notamt/api/duvidas` | **fonte v1** — JSON anônimo, 135 perguntas, 9 grupos, todas ativas (§2) | GET direto; bundle `main.*.js` (`fetchDuvidas` → `urlApi+"duvidas"`, `urlApi = …/notamt/api/`) e chunk lazy 22 (`groupBy` → painéis `titulo`/`resposta`) |
| **Hub Liferay** `https://www5.sefaz.mt.gov.br/-/perguntas-frequentes-1` | **fase 2** — web content "Perguntas frequentes"; conteúdo e links não vistos (robots, D-MTFAQ-3) | WebSearch (título indexado) |
| **Pasta de documentos** `https://www5.sefaz.mt.gov.br/documents/6071037/8241275/…` | **fase 2** — PDFs **"FAQ Cadastro"** e **"FAQ ITCD"**, texto indexado "PERGUNTAS FREQUENTES CADASTRO 1. Como …" ⇒ Q&A numerado em PDF | WebSearch |
| `https://www5.sefaz.mt.gov.br/documents/364148/413083/Perguntas+e+respostas+CIB/…` | outro documento de P&R (CIB — Cadastro Imobiliário Brasileiro), outra pasta; avaliar junto com a fase 2 | WebSearch |
| **ServiceNow** `https://conecta.sefaz.mt.gov.br` | candidato não sondado (possível base de conhecimento, como o CSM do MG); `robots.txt` = `Disallow: /` | `robots.txt`; `<script src=".../sn_csm_ec.js">` no Nota MT |
| **CITSmart** `https://sac.sefaz.mt.gov.br/citsmart/pages/knowledgeBasePortal/knowledgeBasePortal.load#/knowledge/<id>` | candidato não sondado — "Portal do Conhecimento" (base de conhecimento do SAC; o passo a passo do resgate do Desconto IPVA é o artigo 19156) | link na resposta da dúvida 540 do Nota MT |
| `https://www.sefaz.mt.gov.br/portal/nfce/arqDownloads/NFC-e-Perguntas-e-Respostas_SEFAZ_MT_12_03_15.pdf` | PDF de 2015 — provável desatualizado; fora salvo decisão | WebSearch |
| **Lotus Domino** `https://app1.sefaz.mt.gov.br/sistema/acesso/controledeacesso.nsf/…?OpenDocument`, `…/pqsp.nsf/…` | legado (Notes), conteúdo de sistemas internos/controle de acesso — fora | WebSearch |
| Web content avulso `…/-/sefaz-esclarece-duvidas-de-empreendedores-individuais`, `…/-/destda-1` | notícias/páginas em prosa — fora (não são FAQ estruturada) | WebSearch |
| **X-Via** `portal.mt.gov.br` (`POST /v1/search/department`, `departmentSlug` = `secretaria-de-estado-de-fazenda`) | **sem FAQ** — os itens têm 26 campos (`active`, `category`, `description`, `popularNames`, `tags`, `targets`…), nenhum de FAQ; "pergunta" só 2× em prosa sobre Consulta Pública | resposta da coleta de serviços de 02/11 (`data/mt/raw/cache/servicos/`) |
| Headless | desnecessário — o Nota MT tem API; o Liferay serve HTML e PDF | — |

**Robots por host (lidos em 02/11, UA AuliBot):**

| host | `robots.txt` | efeito |
|---|---|---|
| `www.sefaz.mt.gov.br` (Nota MT) | **inalcançável** — `https://…/robots.txt` → 302 `http://…` → 302 `https://…` em laço | RFC 9309 §2.3.1.2: mais de 5 redirecionamentos ⇒ arquivo indisponível ⇒ acesso liberado. Nada a decidir. |
| `www5.sefaz.mt.gov.br` (Liferay) | `Googlebot` e `Bingbot` liberados; `User-agent: *` → `Disallow: /`, `Disallow: /web/`, `Crawl-delay: 30` | bloqueia o AuliBot ⇒ D-MTFAQ-3 |
| `conecta.sefaz.mt.gov.br` (ServiceNow) | `User-agent: *` / `Disallow: /` | idem, se um dia for sondado |

## 2. Modelagem v1 — FAQ do Nota MT

Uma chamada: `GET https://www.sefaz.mt.gov.br/notamt/api/duvidas` (sem corpo, sem sessão, sem token) → array JSON.
Cada item: `{id, descDuvida, descResposta, descGrupoDuvida, statusDuvida, numrOrdemDuvida}` — `id` número, os
`desc*` strings, `statusDuvida` objeto `{ativo, valor, id}`, `numrOrdemDuvida` número ou `null` (1 caso, a 136).

| `FaqRaw` | origem |
|---|---|
| `pergunta` | `descDuvida` (trim — há pergunta com espaço à esquerda) |
| `resposta` | `descResposta` → texto. HTML simples em 46/135: `<br>` (134), `<li>` (56), `<b>` (12), `<ul>`/`<UL>` (8), `<a>` (2); sem entidades. Mesmo tratamento do `html_para_texto` das FAQs do MS (links preservados como texto + URL). |
| `origin` | `Nota MT \| <grupo sem o prefixo de ordem>`: `"9 - DESCONTO IPVA"` → `"Nota MT \| DESCONTO IPVA"` (caixa alta mantida, D-MTFAQ-7). O programa entra porque a trilha vai para o texto vetorizado e "PROGRAMA" ou "CADASTRO" sozinhos não dizem de quê. |
| `url` | `https://www.sefaz.mt.gov.br/notamt/duvida` para todas — o SPA só tem a rota `path:"duvida"`, sem rota nem âncora por pergunta |

**Números (02/11/2026):** 135 itens, 135 `id` distintos, 0 perguntas repetidas, 0 pares (grupo, ordem) repetidos,
135/135 ativos. Resposta: mín. 4 caracteres ("Sim."), mediana 251, máx. 992; 0 vazias. Grupos: 1 - PROGRAMA 27 ·
2 - CADASTRO 9 · 3 - PRÊMIOS E ENTIDADES 16 · 4 - BILHETES E SORTEIOS 8 · 5 - PAGAMENTO DOS PRÊMIOS 6 ·
6 - EMPRESAS 13 · 7 - MENOR PREÇO 17 · 8 - DOE SUA NOTA 15 · 9 - DESCONTO IPVA 24.

**Identidade.** O nome do `.md` é `slug(pergunta)` + hash8 de `url + "\n" + pergunta`
([`nome_faq_de`](../../auli-server/crates/auli-contract/src/mddoc.rs)). Como a `url` é a mesma para as 135, **a pergunta
é a identidade**: duas perguntas iguais gerariam o mesmo arquivo e uma sobrescreveria a outra em silêncio (aconteceu
no MS — 2 casos, resolvidos com `?idFaq=`; aqui o SPA não respeita parâmetro). Guarda `Err` em pergunta repetida.

**Ordem determinística:** grupo pelo número do prefixo → `numrOrdemDuvida` (sem ordem vai para o fim do grupo) →
`id`. Gravar na ordem da API gera falso "raw mudou" no `atualizar-servicos.sh` (aconteceu com o ES em 02/11).

**`statusDuvida.ativo`:** inativo fica fora, com aviso e contagem (hoje nenhum).

**Guardas (`Err`):** resposta fora do formato (campo ausente ou de outro tipo); lista sem nenhuma dúvida ativa; `id`
repetido; pergunta vazia ou repetida; grupo fora do formato `N - NOME`. Resposta muito curta ("Sim.", "Não.") é
legítima — não avisa.

**Árvore:** raiz → `Menu` "Nota MT" → um `Faq` por grupo (na ordem do prefixo), tudo com o link da página e os itens
sem `url`. A aba de FAQs começa pelos filhos da raiz (`parseFaqs.ts`): o nível do programa dá o contexto que os
grupos não têm, e é onde a fase 2 penduraria os PDFs. URL repetida entre nós já existe na frota (PR, SP, MG).

## 2b. Fase 2 — PDFs "FAQ <tema>" do Liferay (hipótese, depende da D-MTFAQ-3 e da §5.2)

**Fonte = os PDFs "FAQ <tema>" da pasta `6071037/8241275`, enumerados pelo hub** (ou pela listagem da pasta).

| `FaqRaw` | origem |
|---|---|
| `pergunta` | linha numerada `N. <pergunta>` do texto do PDF (`pdftotext -layout -enc UTF-8`), até o `?`/fim de linha |
| `resposta` | texto entre a pergunta e a próxima `N+1.` (parágrafos juntados; quebras de página e cabeçalho/rodapé do PDF removidos — medir na §5.2) |
| `origin` | `"<Tema>"` (do título do documento: "FAQ Cadastro" → `Cadastro`); sub-seções do PDF, se houver, como 2º nível |
| `url` | a URL do documento + `#page=N` (página da pergunta; o `pdftotext` separa páginas com `\f`). Os leitores de PDF dos navegadores abrem na página, e duas perguntas iguais em páginas diferentes deixam de colidir no nome do `.md`. Custo: repaginar o PDF muda o hash ⇒ re-embed do tema (aceito na D-MTFAQ-8). |

Identidade = `(documento, N)`. Guardas: numeração contígua 1..N sem buraco/repetição (`Err`); documento do hub que não
baixa (`Err`); zero no total (`Err`); PDF sem nenhuma linha `1.` (`Err` "layout mudou"); pergunta vazia (`Err`);
`(url, pergunta)` repetido (`Err`).

**Transporte:** não há chamada de PDF no kit. O precedente é local ao scraper de pareceres do PR
([`pareceres.rs`](../../auli-server/crates/scrapers/auli-scraper-pr/src/pareceres.rs): download por `curl`,
`pdftotext -layout -enc UTF-8 <pdf> -` como subprocesso). O MT seria o 2º uso: copiar as ~15 linhas; promover ao kit
só no 3º.

**Riscos próprios do PDF:** duas colunas ou tabelas quebram a ordem do texto; numeração de listas **dentro** das
respostas ("1) …, 2) …") confundindo o parser (exigir `N.` no início de linha **e** N == anterior + 1); cabeçalho
repetido a cada página ("PERGUNTAS FREQUENTES CADASTRO") entrando na resposta; acentos/hifenização.

## 3. Decisões propostas (D-MTFAQ-*)

- **D-MTFAQ-1** — Escopo v1 = **FAQ do Nota MT** (135, §2). Fase 2 = PDFs "FAQ <tema>" do Liferay (e o CIB, se
  estruturado), após a D-MTFAQ-3 e a §5.2. NFC-e 2015 e Domino fora. ServiceNow `conecta` não sondado.
  Precedente de FAQ de programa de cidadania fiscal no acervo: as 18 FAQs "Devolve ICMS" do RS vêm do site do Nota
  Fiscal Gaúcha.
- **D-MTFAQ-2** — Enumeração: v1 = a própria API (uma lista, nada no código). Fase 2 = pelo **hub** (links para
  `/documents/…`), nunca por lista no código; se o hub for pobre, pela **listagem da pasta** (JSON WS antigo ou
  `headless-delivery` do Liferay 7, se abertos a convidado — §5.2).
- **D-MTFAQ-3** — Robots: o Nota MT não depende disto (host sem `robots.txt` alcançável). O `www5` declara
  `Disallow: /` para o AuliBot ⇒ **decisão do mantenedor** antes da fase 2. Precedentes registrados:
  **D-PA-ROBOTS** (política: conteúdo público/LAI, baixo volume, AuliBot + cortesia ≥1 s, nunca autenticar),
  **D-AC-ROBOTS** e **§42** (MG, `atendimento2`, `Disallow: /`, sem autorização do órgão — o caso mais parecido). Se
  desconsiderado, honrar o `Crawl-delay: 30` declarado: com poucos PDFs custa minutos e é a mitigação natural.
  **✅ 07/11/2026 — robots decidido pela regra geral D-FROTA-ROBOTS** (`auli_pendencias.md` §44): o `www5` não é
  mais barrado pelo robots; a proposta do `Crawl-delay` segue para a fase 2.
- **D-MTFAQ-4** — Guardas: §2 (v1) e §2b (fase 2).
- **D-MTFAQ-5** — Rede sempre; cache só com `--usecache` (miss = erro). O `update-mt-faqs.sh` apaga `cache/faqs`
  antes de coletar (regra da frota, 02/11).
- **D-MTFAQ-6** — Subcomando `faqs` + `all` no `auli-scraper-mt`; default segue `servicos`.
- **D-MTFAQ-7** — Ingerir como está (caixa alta dos grupos, texto da fonte); só o prefixo de ordem (`9 -`) sai do
  `origin`, porque é ordenação, não nome. O `origin` ganha o programa na frente (`Nota MT | <GRUPO>`).
- **D-MTFAQ-8** — (fase 2) Versão do documento: no Liferay o `uuid` da URL `/documents/<grupo>/<pasta>/<título>/<uuid>`
  é da entrada do arquivo e, pelo que se sabe da plataforma, **se mantém quando sobe nova versão**; muda se o arquivo
  for apagado e reenviado, e a URL muda se o título mudar (a confirmar na §5.2). Em qualquer troca o `url` muda e o
  nome do `.md` muda com ele ⇒ re-embed do tema inteiro. Aceitável (poucos temas); registrar.
- **D-MTFAQ-9** — UA: AuliBot (D-PA-ROBOTS); a API do Nota MT respondeu 200 a ele.

## 4. Riscos / pendências

- Fase 2: tamanho desconhecido (2 PDFs ou 20). O que pesa é o custo do parser de PDF, não a contagem — a frota já
  aceita coleções pequenas.
- O `conecta.sefaz.mt.gov.br` (ServiceNow) pode ter base de conhecimento pública como o CSM do MG — e, como lá, exigir
  sessão anônima (`g_ck`) e UA de navegador. Não sondado; só com decisão.
- A API do Nota MT também atende `POST duvidas` (`salvarDuvida`, anexos em `NMTTB52_DUVIDA_MENSAGEM`) e
  `GET duvidas/{id}` — o scraper usa **só** o `GET duvidas` da lista.
- O "Portal do Conhecimento" CITSmart do SAC (`sac.sefaz.mt.gov.br`) pode ser uma base de conhecimento ampla —
  não sondado; `robots.txt` não lido.

## 5. Sondagem

### 5.1 Feita em 02/11/2026 (deste ambiente, UA AuliBot)

- `robots.txt` dos três hosts (§1).
- Nota MT: `notamt/duvida` = shell Angular de 1,7 KB (`<base href="/notamt/">`); `main.*.js` (1,9 MB) →
  `urlApi:i+"notamt/api/"` e `fetchDuvidas`/`salvarDuvida`/`buscarDuvida`; `runtime.*.js` → chunk 22
  (`22.3f4fece7a6c083f12a0d.js`, módulo da dúvida: `fetchDuvidas().subscribe(… groupBy …)`, painéis com
  `titulo`/`resposta`); `GET notamt/api/duvidas` → 200, 76 KB, `application/json`, 135 itens (números na §2).
- X-Via: chaves dos itens lidas da resposta da coleta de serviços do mesmo dia (§1).

### 5.2 A rodar na máquina do usuário — fase 2, **só depois da D-MTFAQ-3**

```bash
cd /tmp; UA="AuliBot/0.1 (+https://github.com/oxschellen/auli)"; W=https://www5.sefaz.mt.gov.br
# 1) o hub: o que ele lista
curl -s -A "$UA" "$W/-/perguntas-frequentes-1" -o hub.html -w 'hub: %{http_code} %{size_download}B\n'
grep -oE 'href="[^"]*(documents/|perguntas|faq)[^"]*"' hub.html | sort -u
grep -oE '<(h[1-4]|a)[^>]*>[^<]{3,100}' hub.html | grep -iE 'faq|pergunta|d[uú]vida' | head -20
# 2) a pasta de documentos: JSON WS antigo e, se fechado, a API headless-delivery do Liferay 7
curl -s -A "$UA" -H "Accept: application/json" "$W/api/jsonws/dlapp/get-file-entries/repository-id/6071037/folder-id/8241275" -o pasta.json -w 'jsonws: %{http_code} %{size_download}B\n'
curl -s -A "$UA" -H "Accept: application/json" "$W/o/headless-delivery/v1.0/document-folders/8241275/documents?pageSize=100" -o pasta-hd.json -w 'headless-delivery: %{http_code} %{size_download}B\n'
jq -r '(if type=="array" then . else (.items // []) end)[] | "\(.title)\t\(.contentUrl // .uuid)"' pasta.json pasta-hd.json 2>/dev/null | head -30
# 3) um PDF: baixa, converte e mostra a anatomia da numeração (só "N." no início de linha, como o parser)
curl -sL -A "$UA" "$W/documents/6071037/8241275/FAQ+Cadastro/12a9467b-f481-786a-450a-8a98887dbb1a" -o faq-cadastro.pdf -w 'pdf: %{http_code} %{size_download}B %{content_type}\n'
pdftotext -layout -enc UTF-8 faq-cadastro.pdf faq-cadastro.txt && grep -nE '^\s*[0-9]{1,3}\.\s' faq-cadastro.txt | head -40
echo "--- perguntas N.: $(grep -cE '^\s*[0-9]{1,3}\.\s' faq-cadastro.txt) · páginas: $(grep -c $'\f' faq-cadastro.txt)"; sed -n 1,60p faq-cadastro.txt
```

## 6. Plano v1 (Nota MT) — executado (§8)

F1 parser puro + fixtures (recorte do JSON real: resposta com `<ul><li>`, com `<a>`, "Sim.", pergunta com espaço à
esquerda; casos `Err`: `id` repetido, pergunta repetida, `statusDuvida` inválido, grupo fora do formato) → F2 transporte
(1 GET; cache como bytes; `write_faqs("mt", …)` + árvore) → F3 pipeline (`registry.toml` `collections = ["servicos",
"faqs"]`, `gen-frontend-entities`, `auli-collections mt`, `build-packs.sh mt faqs`, smoke) → F4 registro
(`REGISTRO-scrapers.md#mt-faqs`, `update-mt-faqs.sh`, README/SCRAPERS.md).

## 7. Evidência

- `https://www.sefaz.mt.gov.br/notamt/api/duvidas` — 200, 135 itens (02/11/2026, GET direto).
- `https://www5.sefaz.mt.gov.br/robots.txt`, `https://conecta.sefaz.mt.gov.br/robots.txt` — lidos em 02/11/2026.
- `https://www5.sefaz.mt.gov.br/-/perguntas-frequentes-1` — "Perguntas frequentes - SEFAZ - Site" (índice de busca).
- `https://www5.sefaz.mt.gov.br/documents/6071037/8241275/FAQ+Cadastro/12a9467b-f481-786a-450a-8a98887dbb1a` — "PERGUNTAS FREQUENTES CADASTRO 1. Como …" (índice de busca).
- `https://www5.sefaz.mt.gov.br/documents/6071037/8241275/FAQ+ITCD/0538c698-8840-a661-30a1-e52ae0c8f319` — "PERGUNTAS FREQUENTES ITCD 1. O que é ITCD …" (índice de busca).
- `https://www5.sefaz.mt.gov.br/documents/364148/413083/Perguntas+e+respostas+CIB/5e6cef11-9335-c7fb-6e76-d902626b750a` (índice de busca).
- `https://www.sefaz.mt.gov.br/portal/nfce/arqDownloads/NFC-e-Perguntas-e-Respostas_SEFAZ_MT_12_03_15.pdf` (2015).
- `https://app1.sefaz.mt.gov.br/sistema/acesso/controledeacesso.nsf/…?OpenDocument=` — Domino, legado.

## 8. Implementação (02/11/2026)

`auli-scraper-mt` ganhou `faqs.rs` e os subcomandos `faqs` e `all`; a `mt` ganhou a coleção `faqs` no registry;
`scripts/tools/update-mt-faqs.sh` (irmão do do MS: apaga o cache, guarda de encolhimento contra o `.anterior`).
Registro em [`REGISTRO-scrapers.md#mt-faqs`](../REGISTRO-scrapers.md#mt-faqs).

- **Rodada de 02/11:** 1 GET (200, 76 KB) → 135 perguntas, 9 grupos, 0 inativas, 0 repetidas, 0 respostas vazias;
  nenhuma tag nem entidade no texto. `--usecache`: árvore idêntica byte a byte, snapshot idêntico a menos do
  `coletado_em`. 135 `.md` com nomes únicos; 135 vetores. Os artefatos de serviços, refeitos pelo
  `auli-collections mt`, saíram idênticos.
- **Smoke** (`/v1/retrieve`, `mt`/`faqs`, top 3): o alvo em 1º em 4 de 5 perguntas (desconto do IPVA, prêmio
  sorteado, Simples Nacional, doação de notas); em "conta bancária", "Onde informar os meus dados bancários?" em 2º,
  0,003 atrás de "Como fazer o cadastro no Programa Nota MT?".
- **Testes:** 13 (recorte real de 7 dúvidas de 5 grupos, embaralhado de modo que o grupo 3 chegue de trás para
  frente). Mutação: sem a ordenação, com "sem ordem" primeiro, sem as guardas de pergunta e de `id` repetidos, com
  as inativas dentro, sem a quebra de linha dos blocos, sem o link, sem o `clean` da pergunta e sem o programa na
  trilha — cada uma derruba um teste.

## 9. Fase 2 — sondagem de 07/11/2026

Feita deste ambiente depois da D-FROTA-ROBOTS, com o UA AuliBot e **30 s entre as requisições** ao `www5` e ao
Portal do Conhecimento (o `Crawl-delay` que os dois declaram, como a D-MTFAQ-3 propunha). ~80 GETs ao todo.

> **TL;DR — as FAQs atuais do MT estão em HTML e numa API, não nos PDFs.**
>
> O hub `/-/perguntas-frequentes-1` não lista mais PDF nenhum: diz que as perguntas "estão disponíveis no Portal do
> Conhecimento" e linka `portaldoconhecimento.mt.gov.br/perguntas-frequentes7`, que dá **404** — o portal foi refeito
> (o sitemap declara 10 páginas: calculadoras, modelos, nenhuma de perguntas). Os PDFs "FAQ Cadastro" e "FAQ ITCD"
> ainda baixam, mas são de **28/09/2017** e nada os linka. O que há de atual:
>
> 1. **Três páginas "Perguntas frequentes" de áreas de serviço do `www5`**, em HTML: Contencioso Administrativo
>    Tributário (**21**), Declarações Fiscais — DIMP (**11**) e Documentos Fiscais — NFA-e (**15**): **47**. (A NFA-e
>    tem 16; a contagem certa é 48 — §10.)
> 2. **A FAQ do portal estadual X-Via** (`portal.mt.gov.br/perguntas-frequentes`), por API JSON aberta: `POST /v1/faq`
>    com o id da SEFAZ devolve **11** perguntas, todas alteradas em fev–mar/2026.

### 9.1 O que foi avaliado

| Fonte | Veredito | Evidência |
|---|---|---|
| `www5` — hub `/-/perguntas-frequentes-1` | **sem conteúdo**: aponta para o Portal do Conhecimento | o texto do web content; o link é `…/perguntas-frequentes7` |
| `portaldoconhecimento.mt.gov.br` (Liferay; robots `Disallow: /`, `Crawl-delay: 30`) | **sem FAQ**: `/perguntas-frequentes7`, `/mapa-do-site` e `/sobre-o-itcd` dão 404; o sitemap (32 sub-sitemaps) declara 10 páginas, nenhuma de perguntas | os 32 sub-sitemaps, lidos um a um |
| `www5` — listagem da pasta (JSON WS e `headless-delivery`) | **403** nas duas | a enumeração não pode ser pela pasta (D-MTFAQ-2) |
| `www5` — as 24 áreas de "Todos os Serviços" (`/servicos`) | **5 têm o item "Perguntas frequentes"** no menu: Contencioso, Declarações Fiscais e Documentos Fiscais com perguntas; Autorregularização vazia; ITCD → o hub (sem conteúdo) | as 24 páginas de área |
| **`www5` — Contencioso** (`/-/22563369-perguntas-frequentes`) | **fonte**: 21 perguntas, `<p><strong>N. …?</strong></p>` + `<p><strong>Resposta:</strong> …</p>` | atual (Plenária Virtual, DT-E, e-Process) |
| **`www5` — Declarações Fiscais** (`/servicos?c=75259024&e=75467035&s=75468011`) | **fonte**: 11 perguntas sobre a DIMP, `<strong>N. …?</strong><br>` + resposta no mesmo `<p>` | cita o Ato COTEPE/ICMS 38/2024 |
| **`www5` — Documentos Fiscais** (`/servicos?c=16773297&e=74923471&s=74924121`) | **fonte**: 15 perguntas sobre a NFA-e, `<ol start="N"><li><strong>…?</strong></li></ol>` + `<p>` | — |
| `www5` — PDF "FAQ Cadastro" (`documents/6071037/8241275/FAQ+Cadastro/…`) | **antigo**: 28/09/2017, 22 perguntas em 2 seções em caixa alta (14 + 8, a numeração recomeça), Portaria 005/2014 | baixa (200), mas nenhuma página o linka |
| `www5` — PDF "FAQ ITCD" (`…/FAQ+ITCD/0538c698-…`) | **antigo**: 28/09/2017, 14 perguntas, tabelas de faixas de alíquota | idem |
| `www5` — "Perguntas e respostas CIB" | **fora**: CIB é a Comissão Intergestores Bipartite, da assistência social (SETAS/MT) — não o "Cadastro Imobiliário" que a §1 supôs | índice de busca |
| **X-Via** `portal.mt.gov.br` (robots libera tudo) | **fonte**: `POST /v1/faq {"uo": [<id da SEFAZ>]}` → 11 perguntas (pergunta, resposta em HTML, `dataDaUltimaAlteracao`, categoria, serviço, status); página por pergunta, `/perguntas-frequentes/<id>`; o id vem de `GET /v1/department/secretaria-de-estado-de-fazenda` | o bundle `main.*.js` (`/v1/faq`, `/v1/faq/{id}`); a lista sem filtro tem 295 perguntas de 13 órgãos, nenhuma outra tributária |

As 11 do X-Via: 5 de "Finanças e Tributação" (contribuinte do ICMS, domicílio tributário, transferência e situação
da inscrição estadual, inscrição de produtor menor de idade — a primeira cita a Portaria 059/2025, que torna o PDF
de Cadastro de 2017 duvidoso), 5 de "Comunicação e Transparência" (orçamento, receita e despesa públicas, audiência e
consulta pública) e 1 institucional ("O que é Servidor Fazendário?").

### 9.2 Modelagem proposta

| `FaqRaw` | `www5` (HTML) | X-Via (JSON) |
|---|---|---|
| `pergunta` | a linha em negrito terminada em "?" (`kit::linhas`) | `pergunta` |
| `resposta` | até a próxima pergunta, sem o "Resposta:" (`kit::html`) | `resposta` (HTML → `kit::html`) |
| `origin` | `"SEFAZ-MT | <área> | Perguntas frequentes"` | `"Portal MT | <categoria>"` (+ o serviço, quando há) |
| `url` | a página da área (sem âncora) | `https://portal.mt.gov.br/perguntas-frequentes/<id>` |

Árvore: o Nota MT segue como está; entram os nós "Site da SEFAZ" (uma página por área) e "Portal MT". Guardas: cada
página listada com pelo menos uma pergunta; pergunta sem resposta, `(url, pergunta)` repetido, X-Via com `status`
diferente de "Ativo" ou `uo` de outro órgão → `Err`; item "Perguntas frequentes" novo numa área, ou que sumiu → aviso.

### 9.3 Decisões propostas

- **D-MTFAQ-10 — Escopo da fase 2.**
  - **A — as 3 páginas do `www5` e as 11 do X-Via: 58** (o MT vai de 135 a 193). Tudo atual e linkado pelos sites.
  - **B — A + os 2 PDFs de 2017: 94.** Os PDFs estão órfãos (nenhuma página os linka) e o de Cadastro cita normas
    que a FAQ do X-Via já dá por substituídas; exigiriam ainda uma regra nova no leitor de PDF (seção em caixa alta
    que recomeça a numeração).
  - **C — só o X-Via: 11.**

  Recomendo **A**: é o que a SEFAZ-MT publica hoje, nas duas formas mais limpas que a frota já lê (`kit::linhas`, API
  JSON). Os PDFs ficam registrados; entram se a SEFAZ voltar a linká-los.
- **D-MTFAQ-11 — Enumeração do `www5`:** pela página "Todos os Serviços" → as áreas → o item "Perguntas frequentes" de
  cada uma, nunca por lista no código (D-MTFAQ-2). Custo: ~30 GETs a 30 s, ~15 min por rodada. Área vazia
  (Autorregularização) ou que leva ao hub (ITCD) → aviso, não erro.
- **D-MTFAQ-12 — X-Via:** o id da SEFAZ derivado do `GET /v1/department/<slug>` (não fixo no código, como o órgão do
  AL); 1 s entre as chamadas (o robots libera tudo).
- **D-MTFAQ-13 — Cortesia no `www5`:** honrar o `Crawl-delay: 30` (a proposta da D-MTFAQ-3, aplicada na sondagem).
- **D-MTFAQ-14 — Trilha:** `SEFAZ-MT | <área> | Perguntas frequentes` e `Portal MT | <categoria>`; o Nota MT não
  muda.

### 9.4 Reprodução

```sh
UA='AuliBot/0.1 (+https://github.com/oxschellen/auli)'; W=https://www5.sefaz.mt.gov.br
curl -s -A "$UA" "$W/servicos" | grep -o 'href="/servicos?c=[^"]*"[^>]*>[^<]*'          # as 24 áreas
curl -s -A "$UA" "$W/-/22563369-perguntas-frequentes" | grep -c 'Resposta:'             # 21 (30 s entre GETs)
ID=$(curl -s -A "$UA" https://portal.mt.gov.br/v1/department/secretaria-de-estado-de-fazenda | jq -r .id)
curl -s -A "$UA" -H 'Content-Type: application/json' -X POST https://portal.mt.gov.br/v1/faq \
  -d "{\"uo\":[\"$ID\"]}" | jq 'length'                                                   # 11
```

## 10. Fase 2 — implementação (07/11/2026)

**Decidido pelo usuário em 07/11: D-MTFAQ-10 = A**; D-MTFAQ-11 a -14 como propostas, com os desvios abaixo.
Registro em [`REGISTRO-scrapers.md#mt-faqs`](../REGISTRO-scrapers.md#mt-faqs).

O `faqs.rs` passa a ter três fontes, cada uma um ramo da árvore (raiz → `Menu` por fonte → `Faq` por grupo): o Nota
MT, sem mudança; o site da SEFAZ (`faqs/site.rs`); e o Portal MT (`faqs/portal.rs`). As chamadas passam todas pela
`Rede`: cache por URL (no POST, URL e corpo) e cortesia por host, 30 s no `www5` e 1 s nos outros.

- **`www5`:** "Todos os Serviços" → as áreas (`a[href*="/servicos?c="]`, sem repetir o link; menos de 15 é o layout
  que mudou, `Err`) → o item "Perguntas frequentes" do menu da área (`div.mosaic-relateds-list`; o texto comparado
  sem diferença de maiúscula e com os espaços normalizados) → a página, lida dentro do único `div.mosaic-content`
  com o `kit::linhas`; o "Resposta:" do começo da resposta sai. Duas áreas que levem à mesma página: ela entra uma
  vez. Página sem pergunta: aviso. Dependência nova do crate: `scraper` (html5ever).
- **Portal MT:** `GET /v1/department/<slug>` → o id → `POST /v1/faq {"uo": [<id>]}`. Grupo = a categoria, ou o tipo
  quando não há ("Institucional"), na ordem do nome; dentro dele, na ordem do id (a API não garante ordem); link =
  a página da pergunta.

**Desvios em relação à §9:**

1. **A NFA-e tem 16 perguntas, não 15.** A sondagem contou errado: a página numera de 1 a 16 (`<ol start>`). A 6ª
   vem partida em dois negritos ("…da NFA-" + "e?"), e o `kit::linhas` junta as duas. O `www5` dá **48**, a fase 2
   **59** e o MT **194**.
2. **Trilha do `www5` sem o sufixo:** `SEFAZ-MT | <área>`, e não `… | Perguntas frequentes` (D-MTFAQ-14). Toda página
   da fonte é de perguntas frequentes, e a raiz da árvore já tem esse nome; o sufixo repetiria as mesmas duas
   palavras nos 48 textos vetorizados.
3. **Trilha do Portal MT só com a categoria** (`Portal MT | <categoria>`, como a D-MTFAQ-14), sem o serviço que a
   tabela da §9.2 sugeria. 10 das 11 têm serviço, mas é o serviço do catálogo a que a pergunta foi pendurada
   ("Inscrever no Cadastro de Contribuintes Eletrônicos…" para "Quem é considerado contribuinte do ICMS em MT?"),
   não o assunto dela.
4. **Pergunta inativa no Portal MT = aviso e fora**, e não `Err` (§9.2): o mesmo tratamento das dúvidas inativas do
   Nota MT. Pergunta de outro órgão continua `Err`.
5. **Página de área sem pergunta = aviso** (D-MTFAQ-11), e não `Err` como a lista de guardas da §9.2 dizia. Área que
   ganhe ou perca o item "Perguntas frequentes" não tem aviso próprio: pergunta a mais só aumenta o total, e a menos
   cai na guarda de encolhimento do `update-mt-faqs.sh`.
6. O nó da árvore se chama "SEFAZ-MT", e não "Site da SEFAZ": o mesmo nome da trilha.

**Rodada de laboratório** (07/11, rede, diretório isolado): 30 GETs ao `www5` com 30 s entre eles ("Todos os
Serviços", as 24 áreas, as 5 páginas de perguntas), 1 GET e 1 POST ao Portal MT, 1 GET ao Nota MT; **194
perguntas em 15 grupos**. Avisos: Autorregularização e ITCD sem pergunta.

**Testes:** 21 → 40. O `www5` com 6 páginas reais de 07/11, sem `<script>`, `<style>` e comentários ("Todos os
Serviços", uma área com o item, uma sem, a NFA-e, o Contencioso e a página vazia da Autorregularização), e um site
de mentira de 15 áreas para o `coletar`, servido por uma `Rede` só de cache; o Portal MT com as duas respostas reais;
as três fontes juntas na consolidação e na árvore. Mutação, 31 defeitos plantados um a um: 30 derrubam
um teste. Entre eles: sem o mínimo de áreas, sem a deduplicação das páginas e das áreas, sem o filtro do link de ícone,
com o item procurado fora do menu, com a comparação sensível a maiúscula ou a espaço, sem a exigência de um só bloco
de conteúdo, sem o corte do "Resposta:", com link relativo tratado como absoluto, sem a guarda de órgão, com as
inativas dentro, sem a ordem do id, sem o link da pergunta, com o id do órgão fixo, com a chave de cache do POST sem o
corpo, e com o link e a trilha errados na consolidação e na árvore. O que sobrevive é tirar uma fonte da lista do
`run`, que é a entrada de rede e não tem teste unitário (como na fase 1); a rodada de produção a cobre, com a
contagem por fonte no log.

**`--usecache`** sobre o cache da rodada de laboratório: árvore idêntica byte a byte; snapshot idêntico a menos do
`coletado_em`.

**Rodada de produção** (07/11, `update-mt-faqs.sh --no-serve`, cache apagado antes): **194 perguntas** em 15 grupos
— Nota MT 135 (os mesmos 9 grupos), `www5` 48 (Contencioso 21, Declarações Fiscais 11, Documentos Fiscais 16),
Portal MT 11 (Comunicação e Transparência 5, Finanças e Tributação 5, Institucional 1). A guarda de encolhimento
passou (194 contra 135). O pack reaproveitou os 135 vetores do Nota MT e vetorizou as 59 novas; os serviços,
rederivados pelo `auli-collections mt`, seguem com 27.

**Smoke** (`/v1/retrieve`, `mt`/`faqs`, top 3, servidor de laboratório): o alvo em 1º em 5 de 6 — "O que é a NFA-e e
quem é obrigado a usar?" (as duas perguntas da NFA-e em 1º e 2º), "Como contestar o lançamento…" (Contencioso, 0,282),
"Quem deve entregar a DIMP?", "Quem é considerado contribuinte do ICMS em MT?" (Portal MT, 0,235) e o desconto do
IPVA (Nota MT, sem regressão). Em "prazos para cancelar uma nota fiscal avulsa eletrônica", as duas de cancelamento
da NFA-e vêm em 2º e 3º, atrás de "Consultei minha nota fiscal eletrônica e ela aparece como cancelada…" do Nota MT
(0,364 contra 0,400).
