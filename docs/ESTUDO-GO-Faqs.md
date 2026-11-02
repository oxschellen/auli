# ESTUDO-GO-Faqs — Perguntas Frequentes da Economia-GO (coleção `faqs` da entidade `go`)

**Data:** 2026-11-02 (descoberta) · revisado e implementado em 2026-11-02 (§5, §8). · **Estado:** **implementado —
1.298 perguntas** do SharePoint "Orientação Tributária" (§8). · **Órgão:** Secretaria de Estado da Economia de Goiás
(ex-SEFAZ-GO; o Auli registra `SEFAZ-GO`). · **Serviços** da `go`: Portal Expresso/WSO2, 103 itens (recoletado em
02/11), via `get_via_curl` por WAF JA3 (D-GO-WAF).

> **TL;DR — a FAQ tributária de Goiás vive num SharePoint dedicado, "Orientação Tributária"**
> (`https://orientacaotributaria.sefaz.go.gov.br/`; o alias `…economia.go.gov.br` que a página declara canônico
> responde 404 a tudo). A REST **`_api` é anônima**: a lista **"Perguntas Frequentes"** vem inteira num GET —
> **1.298 itens visíveis** (1.338 no `ItemCount`). A categoria é um termo de taxonomia e a REST só dá o **nome** do
> termo; "Informações Gerais" nomeia ~20 termos de temas diferentes. O **caminho** ("Decreto nº 4.852/97 -
> RCTE:Informações Gerais") só aparece no **formulário público do item** (`DispForm.aspx`, que também é o link
> por pergunta): a coleta lê **um formulário por termo** (115). **Ressalva de idade, central:** o site avisa
> (07/07/2021) que as FAQs estão atualizadas até 31/12/2019, mas há itens alterados até **26/08/2022**; a trilha de
> cada pergunta abre com **"Orientação Tributária (até 2022)"** (decisão do mantenedor, D-GOFAQ-3). Achado lateral:
> a biblioteca **`Pareceres`** do mesmo site tem **1.266 documentos** — candidata a 5ª fonte de pareceres, com
> estudo próprio.

---

## 1. Hierarquia discovery-first — o que foi avaliado

| Rota | Veredito | Evidência (02/11/2026) |
|---|---|---|
| `https://goias.gov.br/economia/orientacao-tributaria1/` (WordPress, Elementor) | só o link "Manuais/Pareceres/Perguntas e Respostas Frequentes" → o site de Orientação Tributária | WebFetch |
| `https://goias.gov.br/economia/perguntas-frequentes-da-ouvidoria-setorial/` | 7 perguntas sobre a Ouvidoria (institucional) — **fora** | WebFetch |
| **`https://orientacaotributaria.sefaz.go.gov.br/Paginas/default.aspx`** | **SharePoint 16** instalado no órgão (`MicrosoftSharePointTeamServices: 16.0.0.18526`); seções Perguntas Frequentes, Pareceres, Manuais, Informativos; aviso de 07/07/2021: FAQs atualizadas até **31/12/2019** | GET 200 |
| `https://orientacaotributaria.economia.go.gov.br` | **quebrado**: o https responde 404 (página de erro de outro servidor) em tudo, até na home; o http (BigIP) redireciona para o https. A home do `sefaz` declara `<link rel=canonical>` para `http://…economia…:80` — ignorar | GET 404 / 302 |
| `robots.txt` | **404 nos dois hosts** — sem restrição | GET |
| **`_api` REST** | **anônima**: inventário de listas, campos e itens em JSON (`odata=verbose`) | GET 200 |
| `Paginas/PerguntasFrequentes.aspx` (a página de FAQ do site) | **401** — exige login | GET |
| `Lists/PerguntasFrequentes/DispForm.aspx?ID=<Id>` | **público** (200, ~90 KB): mostra a pergunta e embute o item em JSON (`WPQ1FormCtx`), com a **categoria pelo caminho**; também expõe autor e editor (nome e e-mail de servidores) — o scraper não lê nem guarda esses campos | GET 200 |
| `TaxonomyHiddenList` (`getbytitle` e `GetList`) | 404 / **401** — a taxonomia não é legível por anônimo | GET |
| `$expand=TaxCatchAll`, `FieldValuesAsText/Categoria` | 400 / `null` — não dão o caminho | GET |
| `_vti_bin/owssvr.dll` (exportação XML) | **403 do WAF** ("Acesso Negado") — não usar | GET |
| WAF JA3 | o `ureq` da frota passa (116 GETs da rodada); o JA3 do GO é só do `api.go.gov.br` | rodada de 02/11 |
| Headless | não | — |

## 2. A lista "Perguntas Frequentes" (`_api`, confirmada na §5)

Inventário (`/_api/web/lists`, não ocultas): **Perguntas Frequentes** 1.338 (lista), **Pareceres** 1.266
(biblioteca de documentos), Imagens 118, Manuais 9, Informativos 1, outras de sistema.

Campos (não ocultos e editáveis): `Title` (texto — **vem `null`**), **`Pergunta0`** ("Pergunta", nota em HTML — a
pergunta de fato), **`Resposta`** (nota em HTML), **`Categoria`** (`TaxonomyFieldType`, **um termo só**:
`AllowMultipleValues=false`, `IsPathRendered=true`), `CategoriaId` (texto, o `TermGuid`), `Hits` (vazio). Oculto:
**`m611689e352b49adae244c5880ecdf12`** (o `Categoria_0`): `Nome|TermGuid`.

Na REST, o `Categoria` vem como `{Label: "3", TermGuid, WssId: 3}` — o `Label` é o índice na lista oculta da
taxonomia, não o nome. O nome sai do campo oculto; o caminho, do formulário do item.

**Números (02/11/2026):** 1.298 itens visíveis num GET (`$top=5000`, sem `__next`), `Id` únicos; 0 perguntas
vazias; 6 respostas sem texto — todas só imagem, que viram o link da imagem; 0 itens sem categoria. 115 termos, 67
nomes distintos; 44 temas no 1º nível; caminhos de 2 níveis (112 termos) e 3 (3). 49 textos de pergunta aparecem
mais de uma vez (99 itens). 291 perguntas começam com número ("4- Como…"). 271 respostas já trazem "Conteúdo
atualizado em <data>" no texto. Alteração (`Modified`): 169 itens em 2018, 626 em 2019, 183 em 2020, 276 em 2021,
44 em 2022 (a última em 26/08/2022).

**Um termo renomeado.** O campo oculto guarda o nome de quando o item foi salvo: o termo
`acb3f1b2-…` ("Venda para Entrega Futura") chama-se "Informações Gerais" no formulário e em 5 itens, e "Informações
Fiscais" em 3 itens salvos pela última vez em 31/12/2018 (771, 971, 1001).

## 3. Modelagem → `FaqRaw`

| `FaqRaw` | origem |
|---|---|
| `pergunta` | `Pergunta0` → texto numa linha (numeração da fonte mantida, D-GOFAQ-8) |
| `resposta` | `Resposta` → texto: o conversor das FAQs do SP (o mesmo editor) — link vira `texto "url"`, imagem vira `"url"`, bloco vira linha, célula vira espaço; links relativos (`/Pareceres/…`, `/Manuais/…`, imagens) ganham o host; entidades (`&#58;` no lugar de `:`, inclusive nos `href`; `&#119872;` dos símbolos matemáticos) decodificadas no fim pelo parser HTML |
| `origin` | `Orientação Tributária (até <ano>) \| <tema> \| <termo>` — o caminho do formulário, com o ano da alteração mais recente da lista na frente (D-GOFAQ-3) |
| `url` | `…/Lists/PerguntasFrequentes/DispForm.aspx?ID=<Id>` — público, um por pergunta (D-GOFAQ-2) |

Identidade = `Id`. Ordem: `Id`. Árvore: raiz → `Menu` "Orientação Tributária (até 2022)" → os níveis do caminho
(`Menu` com filhos, `Faq` na folha); os nós apontam para a home do site (a página de FAQ exige login) e cada item
leva o seu formulário.

## 4. Decisões (D-GOFAQ-*)

- **D-GOFAQ-1** — Fonte = a lista "Perguntas Frequentes" via `_api` (1 GET) + um `DispForm` por termo (115 GETs)
  para o caminho da categoria. O conversor HTML é uma **cópia** do das FAQs do SP: já há 7 `html_para_texto` na
  frota (MG, MS, MT, PR, SC, SP e o TARF do RS), com comportamentos diferentes — reuni-los no kit é refatoração à
  parte, não desta coleção.
- **D-GOFAQ-2** — Link = o `DispForm` do item, público. Separa as 49 perguntas de texto repetido (o nome do `.md`
  sai de link + pergunta).
- **D-GOFAQ-3** — **Idade na trilha** (decisão do mantenedor, 02/11/2026): `Orientação Tributária (até <ano>)`, o
  ano da alteração mais recente da lista (hoje 2022), calculado a cada coleta. A trilha entra no texto vetorizado e
  chega ao chat; o nó da árvore mostra a mesma marca na aba. Não há data por pergunta além das 271 que a fonte já
  traz. (O "aviso na UI pelo mecanismo `nota_tipo`" da versão anterior deste estudo não tem base: não existe esse
  mecanismo no registry nem no frontend.)
- **D-GOFAQ-4** — Guardas (`Err` antes de gravar): resposta fora do formato; `__next` (a lista passou de 5.000);
  lista vazia; `Id` repetido; pergunta vazia; item sem categoria ou sem o nome do termo; formulário sem
  exatamente um `"Categoria"`; caminho com nível vazio; caminho que não termina no nome que algum item dá ao termo
  (formulário mal lido). Avisos: itens com nome antigo do termo, respostas vazias.
- **D-GOFAQ-5** — `AuliBot`, cortesia 1 s, `ureq` (sem WAF neste host), rede sempre; cache só com `--usecache`
  (miss = erro). O `update-go-faqs.sh` apaga `cache/faqs` antes de coletar.
- **D-GOFAQ-6** — Subcomando `faqs` + `all` no `auli-scraper-go`; default segue `servicos`.
- **D-GOFAQ-7** — Ouvidoria (WordPress) e PDFs avulsos fora. A biblioteca **`Pareceres`** (1.266 documentos) vira
  pendência de coleção `pareceres` do GO — estudo próprio (são arquivos; exige extração de texto).
- **D-GOFAQ-8** — Ingerir como está: numeração das perguntas ("4- …", "1092-…"), caixa e texto da fonte.
- **D-GOFAQ-9** — **Termo renomeado: vale o formulário.** O caminho do formulário (nome atual) vale para todos os
  itens do termo; o nome antigo no campo oculto de um item só gera aviso.
- **D-GOFAQ-10** — **Sem dados pessoais.** O `DispForm` traz autor e editor (nome e e-mail); o scraper lê só o
  `"Categoria"` da página, e os fixtures não contêm esses campos.

## 5. Sondagem (feita em 02/11/2026, deste ambiente, UA AuliBot)

- `robots.txt` e home dos dois hosts; inventário de listas; campos visíveis e ocultos; `$top=5000` só com `Id` e
  `Modified`; 3 itens completos; o item inteiro com o campo oculto; `TaxonomyHiddenList`, `TaxCatchAll`,
  `FieldValuesAsText`, `owssvr.dll` (§1); `DispForm` de 4 itens (o caminho da categoria e a ocorrência única do
  `"Categoria"` por página); a lista inteira (1 GET, 2,4 MB).
- Na rodada do scraper, os 115 formulários: todos com exatamente um `"Categoria"`; o nome do termo bate com o do
  item lido em 115 de 115; a divergência só aparece em outros itens do termo renomeado.

## 6. Plano — executado (§8)

F1 parser puro + fixtures (10 itens reais e o JSON da categoria de um formulário, sem os campos de autor) → F2
transporte (`_api` + `DispForm` por termo, cache, `write_faqs("go", …)` + árvore) → F3 pipeline (`registry.toml`,
`gen-frontend-entities`, `auli-collections go`, `build-packs.sh go faqs`, smoke) → F4 registro
(`REGISTRO-scrapers.md#go-faqs`, `update-go-faqs.sh`, README/SCRAPERS.md).

## 7. Evidência (02/11/2026)

- `https://orientacaotributaria.sefaz.go.gov.br/Paginas/default.aspx` — SharePoint 16; aviso "atualizado até 31/12/2019" (07/07/2021).
- `https://orientacaotributaria.sefaz.go.gov.br/_api/web/lists` — Perguntas Frequentes 1.338, Pareceres 1.266.
- `https://orientacaotributaria.sefaz.go.gov.br/Lists/PerguntasFrequentes/DispForm.aspx?ID=66` — `"Categoria":"Decreto nº 4.852/97 - RCTE:Informações Gerais"`.
- `https://orientacaotributaria.economia.go.gov.br/Paginas/default.aspx` — 404.
- `https://goias.gov.br/economia/orientacao-tributaria1/` — link único para o site de Orientação Tributária.
- `https://goias.gov.br/economia/perguntas-frequentes-da-ouvidoria-setorial/` — 7 perguntas institucionais (fora).
- `auli-scraper-go/src/go.rs` — D-GO-WAF (JA3 em `api.go.gov.br`, `get_via_curl`).

## 8. Implementação (02/11/2026)

`auli-scraper-go` ganhou `faqs.rs` e os subcomandos `faqs` e `all`; a `go` ganhou a coleção `faqs` no registry;
`scripts/tools/update-go-faqs.sh` (irmão do do MT: apaga o cache, guarda de encolhimento contra o `.anterior`).
Registro em [`REGISTRO-scrapers.md#go-faqs`](REGISTRO-scrapers.md#go-faqs).

- **Rodada de 02/11:** 116 GETs (1 da lista + 115 formulários), todos 200 pelo `ureq`; 1.298 perguntas, 115
  termos, 3 itens com nome antigo do termo (aviso); nenhuma resposta vazia depois da conversão; nenhuma tag nem
  entidade no texto. 1.298 links e 1.298 `.md` com nomes únicos; trilha com 3 níveis em 1.286 perguntas e 4 em 12,
  todas abrindo com "Orientação Tributária (até 2022)". `--usecache`: árvore idêntica byte a byte, snapshot idêntico
  a menos do `coletado_em`. 1.298 vetores (3 respostas no teto de 6.000 caracteres). Os artefatos de serviços,
  refeitos pelo `auli-collections go`, saíram idênticos.
- **A primeira rodada parou na guarda** — a versão inicial exigia que o caminho terminasse no nome do campo oculto
  de cada item, e o termo renomeado a derrubou (item 771). Daí a D-GOFAQ-9.
- **Smoke** (`/v1/retrieve`, `go`/`faqs`, top 3): o alvo em 1º em 4 de 5 perguntas (PROTEGE por benefício fiscal,
  cheque moradia na EFD, ITCD na doação, transporte de carga frigorificada); em "DIFAL no Simples Nacional", a
  pergunta exata em 3º, atrás de duas do mesmo assunto (DIFAL de uso e consumo e do Convênio 236/21).
- **Testes:** 15 (recorte real de 10 itens em ordem inversa do `Id`, e o JSON da categoria de um formulário).
  Mutação: sem a ordenação, sem a conferência do nome do termo, sem as guardas de nível vazio, de `__next`, de
  `Id` repetido e de "exatamente um `Categoria`", com o link na home, sem o host nos links relativos, sem a
  imagem, sem o site na trilha, com o ano mais antigo, sem a quebra de linha dos blocos — cada uma derruba um
  teste. A decodificação dos `href` antes da final não derrubava nenhum (era redundante) e saiu; com isso uma
  pergunta cujo link tem só `&#160;` como texto ganhou o espaço antes do link.
