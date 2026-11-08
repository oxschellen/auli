# ESTUDO-PA-Servicos-pis — o catálogo de serviços do site novo da SEFA-PA (coleção `servicos` da entidade `pa`)

**Data:** 2026-11-07 (sondagem direta, UA AuliBot, 3 GETs de dados + o bundle do site novo). · **Estado:** decidido
(07/11: D-PASERV-1 = C, D-PASERV-2..4 como propostas) e implementado (§6). · **Origem:** pendência §22 do [auli_scraper_pendências.md](auli_scraper_pendências.md),
aberta no [ESTUDO-PA-Faqs.md](faqs/ESTUDO-PA-Faqs.md) §1. · **Hoje a `pa` coleta 34 serviços do paradigital** (Prodepa,
órgão 48 — [REGISTRO-scrapers.md#pa](REGISTRO-scrapers.md#pa)).

> **TL;DR — o site novo tem um catálogo próprio, 3,7× maior e vivo, mas sem público-alvo.**
>
> O tipo `pis` do WordPress do site novo (`site-sefa-wordpress.sefa.pa.gov.br/wp-json/wp/v2/pis`, anônimo) tem 154
> páginas: **137 serviços** e 17 páginas-índice de tema. Tirando 12 serviços publicados duas vezes (um post por tema),
> ficam **125 serviços distintos**, contra os 34 do paradigital — que o `pis` cobre quase todos (29 pelo título, mais 2
> com outro nome). 102 das 154 páginas foram alteradas em 2026. Cada ficha tem uma descrição e, em geral, 4 blocos
> ("Como solicitar", "Documentos necessários", "Manual/Tutorial", "Base legal"), e 113 têm o link do serviço online.
>
> O custo: **o `pis` não diz para quem é o serviço**. Hoje o PA tem 3 abas (Cidadão 21, Empresa 30, Estado 3), que vêm
> das flags do paradigital. Com o `pis`, a classificação é só por tema (ICMS, IPVA, ITCD, Documentos Fiscais…).
>
> Recomendação: **o `pis` como fonte, com os 3 serviços que só o paradigital tem, num público único** (opção C, 128).

> **Correção (07/11/2026, na implementação — §6):** as páginas-índice são **15**, não 17. A regra "título = nome de uma
> das categorias do post" errava em três: "CADIN" e "Consulta Protocolo" são serviços (o site os abre como serviço, e eles
> têm o link do app e os 4 blocos), e "Consulta" é índice (o tema dela é "Consulta Protocolo"). O site lista as 15 no
> próprio bundle. Ficam **139 serviços, 127 distintos**, e a opção C dá **130**.

---

## 1. O catálogo `pis` (07/11/2026)

| O quê | Evidência |
|---|---|
| Acesso | `GET /wp-json/wp/v2/pis?per_page=100&page=1..2` (anônimo, `X-WP-Total: 154`); categorias em `/pis-categories?per_page=100` (19). `/search` e `/types` dão 401, mas não são usados. `robots.txt` do host: só `/wp-admin/` fechado |
| Estado | 154 `publish`; alteração: 102 em 2026, 19 em 2025, 33 em 2024 |
| Campos | `title`, `content` (HTML, 97–2.120 caracteres de texto, mediana 270), `pis-categories`, e o ACF: `titulo1..8`/`descricao1..8` (blocos), `quantidade`, `service_link` (113 têm), `texto_do_link`, `link_arquivo`, `titulo_acordeao` |
| Blocos | 125 dos serviços têm 4 blocos; os títulos variam na grafia ("Documentos necessários"/"Documentos Necessários", "Base legal do serviço"/"Base Legal") |
| Categorias | 19 temas: Documentos Fiscais 18, Home 16, Débitos e Parcelamentos 15, Cadastro de Contribuintes 14, IPVA 13, Benefício Imunidades e Isenções 13, Consulta Protocolo 12, ICMS 11, ITCD 11, Trânsito de Mercadorias 10… 127 páginas têm 1 categoria, 18 têm 2, 9 nenhuma. **"Home" é destaque na página inicial**, não tema (o "ALMA web" está nela e é serviço) |
| Páginas-índice | **17** cujo título é o nome de uma das suas categorias (IPVA, ICMS, ITCD, Cadastro de Contribuintes, Débitos e Parcelamentos, Documentos Fiscais, Trânsito de Mercadorias, Consulta Protocolo, CADIN, Comércio Exterior…). São as páginas de tema que o rodapé do site linka; não são serviços |
| Cópias | 12 títulos publicados duas vezes, um post por tema (ex.: "Isenção de IPVA - Pessoa com Deficiência" em IPVA e em Benefícios): 6 pares idênticos, 6 com textos que divergem (edições em datas diferentes) |
| Link público | o site novo abre `https://www.sefa.pa.gov.br/internal/services/<código>`, em que o código é o id do post com cada dígito trocado por uma letra (`a`=0 … `j`=9): `ifg` = 856 = "Cadastro de Contribuintes". A função está no bundle (`parseInt(s.split("").map(c => c.charCodeAt(0)-97).join(""))`), e o rodapé linka as páginas de tema assim |

## 2. Comparação com o paradigital

| | paradigital (hoje) | `pis` |
|---|---|---|
| Serviços | 34 | 125 (137 menos 12 cópias) |
| Conteúdo | finalidade + etapas + requisitos + contatos + link | descrição + "Como solicitar" + "Documentos necessários" + "Manual" + "Base legal" + link |
| Público | **Cidadão / Empresa / Estado** (flags por serviço) | **nenhum** |
| Classe | um tema só ("Tributos e empresas") | 19 temas |
| Atualização | — | 102 páginas alteradas em 2026 |
| Custo por rodada | 1 + 34 GETs | 3 GETs |

**Sobreposição.** Dos 34 do paradigital, 29 têm par no `pis` pelo título; 2 têm com outro nome ("Consulta Nota Fiscal do
Consumidor Eletrônica-Nfce" → "Consulta NFCe"; "Pedido de parcelamento/reparcelamento ICMS e ITCD" → os dois
"Parcelamento/Reparcelamento" separados); **3 só existem no paradigital**: "Dispensa de pagamento IPVA", "Pedido de
Ressarcimento do IPVA" e "Registro de veículo novo".

## 3. Modelagem proposta (para A ou C)

| `ServicoRaw` | origem |
|---|---|
| `titulo` | `title` |
| `descricao` | `content` + cada bloco preenchido como "Título: texto" + "Acessar o serviço: `service_link`" — tudo pelo conversor do kit (`kit::html`) |
| `link` | `https://www.sefa.pa.gov.br/internal/services/<id em letras>` |
| `ocorrencias` | público único × cada tema do post (sem "Home"; sem tema → "Geral") |

- **Páginas-índice fora** (título = nome de uma das categorias do post).
- **Cópias:** idênticas viram um serviço com os temas das duas; as que divergem também, ficando o texto da alterada
  por último.
- **Guardas:** `X-WP-Total` = itens lidos; descrição vazia é erro; piso (proposta: 110).

## 4. Decisões propostas (D-PASERV-*)

- **D-PASERV-1 — Fonte.** **Decidida (07/11): C.** As opções:
  - **A — trocar pelo `pis`: 125 serviços.** Mais simples (3 GETs, o paradigital sai), mais rico e mais atual. Perde as
    3 abas de público e os 3 serviços que só o paradigital tem.
  - **B — trocar pelo `pis`, tentando manter o público:** o público do paradigital nos 31 que casam e "Serviços" nos
    outros 94. Rejeito: as abas ficariam incoerentes (um serviço de IPVA em "Cidadão", outro em "Serviços").
  - **C — o `pis` mais os 3 só do paradigital: 128 serviços, público único.** O paradigital continua sendo lido, só para
    esses 3, que mantêm o conteúdo de lá.
  - **D — manter o paradigital (34).**

  Recomendo **C**: o `pis` é a fonte do próprio site da SEFA e 3,7× maior, e não perde serviço nenhum. **A** é a
  alternativa mais simples, se os 3 não pesarem.
- **D-PASERV-2 — Público único "Serviços"**, como PI, RO, AC, SE e RN: as 3 abas do PA viram uma (só em A e C).
- **D-PASERV-3 — Cópias e páginas-índice** como na §3.
- **D-PASERV-4 — Transporte:** UA AuliBot (D-PA-ROBOTS), 1 s entre os GETs, `ureq`; sempre à rede, cache só com
  `--usecache`.

## 5. Reprodução

```sh
UA='AuliBot/0.1 (+https://github.com/oxschellen/auli)'; W=https://site-sefa-wordpress.sefa.pa.gov.br/wp-json/wp/v2
curl -sS -I -A "$UA" "$W/pis?per_page=1" | grep -i x-wp-total                       # 154
curl -sS -A "$UA" "$W/pis?per_page=100&page=1" -o pis1.json; curl -sS -A "$UA" "$W/pis?per_page=100&page=2" -o pis2.json
curl -sS -A "$UA" "$W/pis-categories?per_page=100" -o cats.json                       # 19 categorias
jq '[.[] | {id, t: .title.rendered, c: ."pis-categories"}] | length' pis1.json pis2.json
```

## 6. Implementação (07/11/2026)

`auli-scraper-pa servicos`: o `pis.rs` lê o `pis`, e o `pa.rs` junta os 3 do paradigital. O que mudou em relação à
§3:

| | estudo | implementação |
|---|---|---|
| Páginas-índice | 17, pela regra do título = tema | **15**, a lista do próprio site: o `main.js` tem os títulos que abrem como página de tema (`Cadastro de Contribuintes`, `Consulta`, `Débitos e Parcelamentos`… `AINF - Auto de Infração`), e o `pis.rs` copia a lista (`INDICES`). "CADIN" e "Consulta Protocolo" voltam a ser serviços |
| Serviços | 137 → 125 distintos | **139 → 127 distintos**; + 3 do paradigital = **130** |
| Cópias | 6 idênticas, 6 que divergem | 3 idênticas e 9 que divergem, comparando `content` e ACF ao pé da letra. O tratamento é o mesmo (o texto da alterada por último, os temas das duas); a chave é o título só com letras e números ("DIEF - Comércio Exterior" = "DIEF Comércio Exterior") |
| `X-WP-Total` | do cabeçalho | do corpo: os GETs levam `_envelope`, que põe os cabeçalhos no JSON — a guarda vale também com `--usecache` |
| Os 3 do paradigital | — | `GET /servico/{1034,1115,1116}`, com a descrição montada como antes (finalidade + etapas + requisitos + acesso), o link do paradigital e a classe **IPVA** (os três são de IPVA). Se um deles aparecer no `pis` com o mesmo título, fica o do `pis` |

**Descrição.** `content`, cada bloco com texto ("Título:" e o texto; sem os dois-pontos quando o título já termina em
"?", ":", "." ou "!") e "Acessar o serviço: `service_link`" (o caminho da raiz ganha `https://www.sefa.pa.gov.br`).
Tudo pelo `kit::html`, com a base do site. Os blocos ACF vêm de dois jeitos: HTML formatado (a quebra do fonte cai no
meio da frase) ou texto puro (a quebra separa linhas): a quebra do fonte só conta no campo sem tag de bloco. Os 8
blocos são lidos; o `quantidade` não, porque duas páginas têm o 5º bloco ("Base Legal") preenchido com `quantidade` 4.

**Guardas** (erro antes de gravar): os temas e a soma das páginas iguais ao `X-WP-Total`; uma página-índice da lista que
sumiu do catálogo; tema fora de `pis-categories`; descrição vazia; menos de 110 serviços do `pis`.

**Transporte** (D-PASERV-4): `crate::UA`, 1,1 s entre os GETs, `ureq`/rustls, 6 GETs por rodada. Sempre à rede; o
cache em `data/pa/raw/cache/servicos/` só é lido com `--usecache`, e só é gravado depois das guardas. O `--usecache`
reproduz o snapshot a menos do `coletado_em`.

**Resultado da rodada de 07/11.** 130 serviços, 141 ocorrências, uma aba ("Serviços", slug `servicos-gerais` — o
`servicos` colidiria com o `pa-servicos.json` agregado). Classes: Documentos Fiscais 17, IPVA 15, Débitos e
Parcelamentos 14, Cadastro de Contribuintes 13, Benefício Imunidades e Isenções 12, Consulta Protocolo 11, ICMS 10, ITCD
10, Trânsito de Mercadorias 9, Geral 6, Portal de Serviços 5, Comércio Exterior 4, Comunicação Oficial 4, Solicitação 3,
Declarações 3, CADIN 3, AINF 2. Descrição de 171 a 14.698 caracteres (mediana 1.010); só a de "Cadastrar declaração
ITCD - Causa Mortis" passa dos 6.000 que entram no vetor. Os `pa-cidadao/empresa/estado.json` antigos saíram de
`data/pa/raw/` (o `process` não apaga aba que deixou de existir).

**Testes.** 13 no `pis.rs`, sobre 28 páginas reais (as 15 páginas-índice só com id, título e temas, e 13 serviços
escolhidos pelas regras), e 4 no `pa.rs`. 22 mutações (cada regra e cada guarda desligada), todas pegas.

