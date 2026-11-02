# ESTUDO-MT-Faqs — Perguntas Frequentes da SEFAZ-MT (coleção `faqs` da entidade `mt`)

**Data:** 2026-11-02 (descoberta) · revisado em 2026-11-02 após sondagem (§5.1). · **Estado:** **v1 implementada
— FAQ do Nota MT, API JSON anônima, 135 perguntas** (§2, §8). Os PDFs "FAQ <tema>" do site institucional ficam
para a fase 2 e dependem da D-MTFAQ-3 (robots), que o mantenedor deixou para depois. · **Órgão:** Secretaria de Estado de Fazenda de Mato Grosso
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
([`nome_faq_de`](../auli-server/crates/auli-contract/src/mddoc.rs)). Como a `url` é a mesma para as 135, **a pergunta
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
([`pareceres.rs`](../auli-server/crates/scrapers/auli-scraper-pr/src/pareceres.rs): download por `curl`,
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
Registro em [`REGISTRO-scrapers.md#mt-faqs`](REGISTRO-scrapers.md#mt-faqs).

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
