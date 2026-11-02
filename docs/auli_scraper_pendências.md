# Pendências — scrapers

Frentes de coleta em aberto. Criado em **02/11/2026**, depois da rodada que pôs no ar as FAQs de MS, MT, GO
e MG, os serviços de MS, ES, MT, GO e DF e o catálogo do RJ. Reúne também as pendências de scraper que
estavam espalhadas no [auli_pendencias.md](auli_pendencias.md) — movidas para cá na mesma data; lá ficou um
ponteiro no lugar de cada uma, para as citações do código continuarem válidas. Nada abaixo está rodando.

| # | Entidade | Pendência | Estado |
|---|---|---|---|
| 1 | MT | FAQs fase 2 — PDFs e hub do site institucional (`www5`) | decisão do mantenedor sobre o `robots.txt` (D-MTFAQ-3) |
| 2 | MT | Outras bases de conhecimento não sondadas (`conecta`, CITSmart) | nenhuma sondagem feita |
| 3 | GO | Biblioteca **Pareceres** (1.266 documentos) como coleção `pareceres` | estudo próprio — são arquivos |
| 4 | RJ | FAQs | em espera — proteção anti-robô; a D-RJ-WAF cobre só o catálogo |
| 5 | AM | Descrição rica do detalhe (D-AM-V2) | especificada, não feita |
| 6 | PA | `portal-digital` da SEFA (D-PA-PORTALDIGITAL) | reavaliar se voltar do ar |
| 7 | RN | Catálogo só de cards (D-RN-ACERVO) | reavaliar se o RN publicar uma Carta |
| 8 | PA, ES, AL | Scraper estadual genérico, multi-órgão (D-PA-ACERVO, D-AL-1) | registrado, não implementado |
| 9 | PI, RO | Scraper Sydle conecta-360 compartilhado (D-XX-SYDLE-COMPARTILHADO) | decisão de arquitetura em aberto |
| 10 | kit | Camada 2 do kit (`validar_contagem`, `absolutize`, `cli::parse_args`) | TAREFA curta, não feita |
| 11 | frota | User-Agent institucional para a frota toda | parcial — por entidade |
| 12 | 16 entidades | Serviços sem atualização desde 25/07 (AP desde 02/08) | falta rodar |
| 13 | ES | O scraper não ordena os itens | ajuste pequeno |
| 14 | SP | FAQs: dois pontos da revisão do #179 | na próxima vez que alguém tocar o `faqs.rs` |
| 15 | kit | Cache: validação de completude na leitura | medir antes de virar TAREFA |

---

## 1. MT — FAQs fase 2 no `www5` (D-MTFAQ-3)

A fase 1 (Nota MT, 135 perguntas, PR #185) está no ar. A fase 2 é o site institucional Liferay
`www5.sefaz.mt.gov.br`:

- hub "Perguntas frequentes" (`/-/perguntas-frequentes-1`);
- PDFs "FAQ Cadastro" e "FAQ ITCD" (perguntas numeradas), na mesma pasta de documentos;
- "Perguntas e respostas CIB", em outra pasta.

**Bloqueio.** O `robots.txt` do `www5` libera Googlebot e Bingbot e, para `User-agent: *`, declara
`Disallow: /` e `Crawl-delay: 30`. Até a decisão, o `www5` não é acessado.

**Próximo passo.** Se a decisão for coletar, rodar a sondagem da §5.2 do estudo na máquina do mantenedor e
avaliar os PDFs (PDF ficou fora em todas as outras entidades até aqui).

Referência: [ESTUDO-MT-Faqs.md](ESTUDO-MT-Faqs.md) §2b, §3 (D-MTFAQ-3) e §5.2.

## 2. MT — bases de conhecimento não sondadas

- **`conecta.sefaz.mt.gov.br`** — ServiceNow CSM (o Nota MT carrega dele um widget). Pode ter base de
  conhecimento pública, como o CSM do MG. O `robots.txt` declara `Disallow: /` para todos.
- **"Portal do Conhecimento" CITSmart** em `sac.sefaz.mt.gov.br` — base de conhecimento do SAC; uma resposta
  do Nota MT aponta para um artigo dela. Possivelmente ampla.

**Próximo passo.** Ler o `robots.txt` do `sac` e sondar se há listagem anônima, antes de qualquer estudo.

Referência: [ESTUDO-MT-Faqs.md](ESTUDO-MT-Faqs.md) §1 e §4.

## 3. GO — biblioteca Pareceres (D-GOFAQ-7)

O mesmo SharePoint das FAQs do GO (`orientacaotributaria.sefaz.go.gov.br`, `_api` anônima) tem a biblioteca
**Pareceres** com 1.266 documentos — seria a 5ª fonte de pareceres do acervo.

**Próximo passo.** Estudo próprio: formatos dos arquivos, extração de texto, metadados disponíveis na `_api`
(número, data, assunto) e se o conteúdo também para em 2022, como as FAQs.

Referência: [ESTUDO-GO-Faqs.md](ESTUDO-GO-Faqs.md), D-GOFAQ-7.

## 4. RJ: o portal da SEFAZ-RJ barra coleta automatizada — serviços de volta pela D-RJ-WAF, FAQs em espera (aberta — 2026-11-02)

_Era a §43 do [auli_pendencias.md](auli_pendencias.md)._

**Fato (02/11/2026, 9 requisições com 1 s entre elas).** Os três hosts — `portal.`, `portal2.` e
`www.fazenda.rj.gov.br` — respondem 200 com uma página que não é a pedida, em qualquer caminho, inclusive o
`robots.txt` (não há como lê-lo):

| UA | Resposta |
|---|---|
| AuliBot | página de "IP bloqueado" (1,6 KB, "bloqueia acessos provenientes desses endereços IP"). A mensagem engana: com outro UA, o mesmo IP recebe outra coisa |
| o do kit (Firefox) | desafio JavaScript da proteção anti-robô na frente do portal (~45 KB, scripts `/TSPD/`, "Please enable JavaScript to view the page content") |
| Chrome/130 | o mesmo desafio (50,6 KB: `TSPD` 3×, nenhum `wp-content`) — não é o Firefox/124 envelhecido do kit que dispara a proteção |

Em 25/07/2026 a mesma URL do catálogo (`portal2.fazenda.rj.gov.br/nossos-servicos/`) veio normal com o UA do
kit — é o cache do [`rj.rs`](auli-server/crates/scrapers/auli-scraper-rj/src/rj.rs), 62 KB.

**Efeito.**

1. **Serviços:** a coleta pelo `ureq` recebe o desafio e para com `Err` antes de gravar — confirmado em
   02/11/2026 com o cache tirado do caminho: "menu de categorias não encontrado (nenhuma âncora interna
   válida) — markup mudou?"; snapshot e cache de 25/07 intactos. Resolvido pela D-RJ-WAF (abaixo).
2. **FAQs:** a sondagem não chegou a nenhuma página — em espera (a D-RJ-WAF cobre só o catálogo).

**O que se sabe das FAQs sem tocar o portal** (busca na web, 02/11/2026): não há página central; o conteúdo
parece espalhado pelo WordPress multisite (subsites conhecidos: `/icms/`, `/consulta-tributaria/`,
`/ouvidoria/`, `/junta-revisao-fiscal/`):

- uma busca prévia de perguntas frequentes no "Fale Conosco" → "Legislação Tributária", por setor, tributo e
  assunto ([gov.br, serviço 17187](https://www.gov.br/en/state-services/servico_estadual?id=17187));
- um link "Dúvidas Frequentes" no bloco de "Fale Conosco" das portarias da SUPTRIB;
- perguntas e respostas em páginas de subsite (`/icms/gia-icms/`);
- PDFs em perguntas e respostas (cartilha do MEI, Manual NF-e) — PDF ficou fora nas outras entidades.

**Decisão D-RJ-WAF (mantenedor, 02/11/2026): o catálogo de serviços do RJ vem por um Chromium headless.**
Sondagem do mantenedor com o Playwright: o Chromium completo, em modo headless novo e com UA de Chrome,
executa o desafio e recebe o catálogo em ~3,5 s, idêntico ao modo com janela. Sem pedido à SEFAZ-RJ.

- **Só o `auli-scraper-rj`, só a página do catálogo, uma navegação por rodada** — a única exceção da frota à
  regra "sem navegador headless". O `check-scraper-boundary.sh` barra `headless_chrome` em qualquer outro crate.
- **Critério para desfazer, automático:** cada rodada tenta antes um GET simples pelo `ureq`; se o portal voltar
  a servir a página, o Chromium nem abre (e o scraper avisa que a exceção pode cair).
- **O parser decide se a página serve** (os guards D-RJ5 não mudaram). `TSPD` não é critério: a proteção injeta
  o script dela também na página boa.
- **Limite:** o corpo do desafio traz um captcha ("Qual é o Código da Imagem?") para quem não executa
  JavaScript. Se o portal passar a exigi-lo, a exceção para ali — captcha não se resolve.
- **Chromium:** o do Playwright (`npx playwright install chromium`), a versão mais nova em
  `~/.cache/ms-playwright/`, ou `AULI_CHROME_PATH`.
- **Operação:** a coleta roda pelo `atualizar-servicos.sh rj`, liberado numa regra de permissão local do
  mantenedor (o modo automático do Claude Code barra o comando sem ela).
- **Rodada de 02/11/2026:** o GET simples recebeu a página de bloqueio; o Chromium recebeu o catálogo — 14
  categorias, 91 serviços. Em relação a 25/07, só o link do "SCDI – Representante Legal" mudou
  (`scdi.fazenda.rj.gov.br/projetoCDI/` → `portal.fazenda.rj.gov.br/scdi/`). `--usecache` reproduz o snapshot.

**Próximo passo (FAQs).** Depende de estender (ou não) a exceção da D-RJ-WAF além da página do catálogo.

## 5. AM — D-AM-V2: descrição rica do detalhe (pendência aberta, NÃO feita)

_Era subseção da §13 do [auli_pendencias.md](auli_pendencias.md) (entidade `am`)._

A v1 usa só o `description` curto da listagem (≈1 frase). O AM publica, por serviço, uma **descrição
rica** (o diferencial do portal) que hoje NÃO entra no snapshot. Recuperá-la é a **v2 do `am`** — sem
navegador (a descoberta provou zero-XHR: todo o conteúdo é server-rendered no RSC do detalhe).

**Fonte:** `GET https://www.sefaz.am.gov.br/portfolio-servicos/detalhes/{id}?profile=todos` com header
`RSC: 1` (mesmo transporte da v1). No flight, o objeto **`serviceDetails`** (extrair por âncora
`"serviceDetails":{` + balanceamento) traz o mapa de seções:

| campo em `serviceDetails` | seção | forma |
|---|---|---|
| `resumo` | O que é | texto plano (inline) |
| `perfis[]` | A quem se destina | array de strings (= o público, já temos via rotas) |
| `comoProcederHtml` | Passo a passo / Como proceder | **ref de chunk `$a`** |
| `documentacaoHtml` | Documentação necessária | **ref de chunk `$b`** |
| `legislacaoHtml` | Legislação Aplicada | HTML inline |
| `perguntasRespostasHtml` | Perguntas Frequentes | **ref de chunk `$c`** |
| `setorResponsavel{nome,sigla}` + `email` + `phone` + `tempoMedioEmDias` | Contato | campos escalares |
| `visibleSections[]` | quais seções existem | enum (ex.: id 63 não tem `PERGUNTAS_FREQUENTES`) |

**Trabalho técnico da v2 (o que a v1 evitou):**

1. **Resolver refs de chunk do flight:** `"$a"/"$b"/"$c"` apontam para chunks-texto no MESMO payload,
   no formato `a:T<hexlen>,<html>` — **alguns colados ao chunk anterior sem `\n`** (o parser tem de
   varrer por `<ref>:T<hexlen>,` e ler `hexlen` bytes, não confiar em quebra de linha).
2. **Decodificar entidades HTML** (`&ccedil;`, `&atilde;`, `&ordm;`…) e limpar as tags — a tabela fixa
   do `kit::decode_entities` NÃO cobre tudo; usar **html5ever (crate `scraper`)**, mesma lição do GO.
3. Montar a `descricao` concatenando as seções presentes (`visibleSections`) num texto uniforme.

**Custo/risco:** **~278 GETs** (um por serviço, cacheáveis; cortesia entre eles) — mais pesado que
qualquer scraper atual. Os **39 serviços link-only** (externo/submenu) NÃO têm página de detalhe → sem
`serviceDetails`; a v2 mantém para eles só o `resumo` da listagem. Guard sugerido: um piso de seções
não-vazias por serviço para pegar regressão de parser. Evidência e amostras em `docs/REGISTRO-scrapers.md#am` (Fase 3).

## 6. PA — D-PA-PORTALDIGITAL

_Era item da §14 do [auli_pendencias.md](auli_pendencias.md) (entidade `pa`)._

- **⏳ D-PA-PORTALDIGITAL (aberta):** `portal-digital.sefa.pa.gov.br` (SPA SEFA-específica, ids Mongo
  ObjectId) estava em 522 na coleta. Se voltar, reavaliar (pode ter conteúdo próprio).

## 7. RN — D-RN-ACERVO

_Era item da §22 do [auli_pendencias.md](auli_pendencias.md) (entidade `rn`)._

- **D-RN-ACERVO (aberta):** RN é intrinsecamente menu-only (molde RJ/PE, mas via API JSON limpa).
  Reavaliar se/quando o RN publicar uma Carta descritiva ou a UVT expor um catálogo público.

## 8. Scraper estadual genérico — D-PA-ACERVO e D-AL-1

_Eram itens das §14 (`pa`), §15 (`es`) e §24 (`al`) do [auli_pendencias.md](auli_pendencias.md)._

- **⏳ D-PA-ACERVO (aberta, NÃO feita):** o paradigital é um catálogo estadual **multi-tenant com 63
  órgãos** sob o MESMO contrato (`/orgao/{idOrgao}` + `/servico/{id}`). Um **scraper estadual genérico**
  parametrizado por `idOrgao` cobriria os 63 órgãos sem novo código — oportunidade forte para o Acervo.
  Registrado; não implementado (foge do modelo "1 entidade = 1 órgão" atual).
- **ES:** o X-Via do ES tem **48 órgãos** sob a mesma API → **D-PA-ACERVO** ganha 2º caso (scraper
  estadual genérico serviria PA/Prodepa **e** ES/X-Via).
- **D-AL-1 (escopo SEFAZ-only, aberta):** o portal é multi-órgão (1664 serviços/71 órgãos) → **D-PA-ACERVO
  puro**; basta relaxar o filtro `organ_id` para virar scraper estadual genérico. Não implementado.

## 9. D-XX-SYDLE-COMPARTILHADO (aberta — a decisão de arquitetura mais importante)

_Era subseção da §16 do [auli_pendencias.md](auli_pendencias.md) (entidade `ro`)._

**PI e RO são a MESMA geração Sydle ONE (conecta-360):** mesmo contrato `_search`, mesmas classes de
plataforma (conteúdo `5cd32901…`, catálogo `5ca3bca7…`, classification `5d66ec59…`). Diferem só em
`{BASE_API_URL host, app, catálogo _id, prefixo-de-link}`. → oportunidade de **um scraper parametrizável
cobrindo PI + RO** (e futuros conecta-360). **O CE NÃO entra** — é a geração antiga (`getChildren`,
classes diferentes, tenant por header `X-Explorer-Account-Token`). **Não decidir/refatorar sem alinhar:**
o trade-off é DRY (1 crate, N estados) × acoplar 2 estados a um contrato de terceiros que evolui. O
"3º tenant PA/portal-digital" ficou **inconclusivo** (estava em 522 na descoberta do PA; usamos o
paradigital/Prodepa — outra plataforma). Correlato ao D-AM-V2: RO/PI têm `contentHtml` inline para uma
eventual descrição rica v2.

## 10. Kit — Camada 2

_Era item da §10 do [auli_pendencias.md](auli_pendencias.md) (extração das funções comuns para o kit)._

- **Camada 2 (2º-consumidor já satisfeito, extrair em TAREFA curta):** `kit::validar_contagem(unicos,
  total, min, dica)` — o invariante dinâmico de ce (`hits`) e mt (`resultTotal`); `kit::absolutize(base,
  href)` — o miolo comum de `canonical` de rj/sp/pe; `kit::cli::parse_args()` — o parse de
  `[--usecache] <cmd>` repetido nos `main.rs`. (Nota: `ms` foi re-ancorado em `união⊆Todos`, então o
  `validar_contagem` tem 2 consumidores reais — ce, mt —, não 3.)

Conferido em 02/11/2026: nenhuma das três existe no kit.

## 11. User-Agent institucional para a frota

_Era item da §10 do [auli_pendencias.md](auli_pendencias.md)._

- **`AuliBot/x.y (+url)` institucional:** avaliar um User-Agent identificável para a frota toda (hoje
  todos usam o Firefox/124 de navegador). Decisão de projeto — muda a identidade de rede em todos os
  portais, exige recoleta verificada.

**Revisão 02/11/2026 — o "hoje" acima envelheceu.** Desde o PA (D-PA-ROBOTS) o AuliBot entrou entidade por
entidade, cada crate com a sua constante (o kit não tem uma): nos serviços de AC, AL, AP, ES, MA, PA, PB, RN,
RO, RR e TO, nas FAQs de GO, MG (SPED), MS, MT, PR, SC e SP, nos pareceres de PR, RS, SC e SP e no TARF do RS.
Seguem com o UA de navegador do kit os serviços de AM, BA, CE, DF, GO, MG, MS, MT, PE, PI, PR, RJ, RS, SC, SE e
SP, as FAQs do RS e as do CSM do MG. Há casos em que o AuliBot
**não** serve: o ServiceNow do MG marca o AuliBot como robô e omite o texto do artigo (auli_pendencias §42), e
o RJ responde ao AuliBot com a página de "IP bloqueado" (§4 acima). O que resta decidir é se a frota inteira
migra (com recoleta verificada) ou se a regra fica por entidade, registrada.

## 12. Serviços sem atualização desde julho

Data da última coleta no snapshot de serviços (`coleta.coletado_em`):

| Coleta | Entidades |
|---|---|
| 25/07/2026 | ac, al, am, ba, ce, ma, pa, pb, pe, pi, rn, ro, rr, se, to |
| 02/08/2026 | ap |
| 31/10–02/11/2026 | as demais (sc, pr, rs, sp, df, es, go, mg, ms, mt, rj) |

**Próximo passo.** `scripts/tools/atualizar-servicos.sh <id>` por entidade (faz backup, apaga o cache, coleta,
deriva e refaz os vetores). Comparar com o backup em `/tmp/auli-atualizacao/<id>` **por link e título**, não
pela posição. Depois, `./start_server.sh` e `scripts/deploy-frontend.sh`.

## 13. ES — ordem dos itens

A API X-Via devolve os serviços em ordem diferente a cada rodada, e o scraper do ES não ordena. Como os `id`
do `raw/` são posicionais, o `atualizar-servicos.sh` acusa "raw/ mudou" mesmo sem mudança de conteúdo
(aconteceu em 02/11: 46 = 46, mesmos links).

**Próximo passo.** Ordenar os itens no scraper por uma chave estável (o link) antes de gravar o snapshot.

## 14. FAQs do SP: dois pontos deixados pela revisão do #179 (aberta — 2026-11-02)

_Era a §41 do [auli_pendencias.md](auli_pendencias.md)._

Registro em [REGISTRO-scrapers.md#sp-faqs](REGISTRO-scrapers.md#sp-faqs); estudo em
[ESTUDO-SP-Faqs.md](ESTUDO-SP-Faqs.md) §9. Nenhum dos dois justifica PR sozinho.

1. **Lista ausente derruba a rodada inteira.** O `buscar_todas` do
   [`faqs.rs`](auli-server/crates/scrapers/auli-scraper-sp/src/faqs.rs) propaga qualquer falha de chamada,
   e um subsite sem a lista `Perguntas Frequentes` responde 404 — então um Home novo criado sem FAQ
   (plausível: 31 das 95 listas já estão vazias) para a coleta toda por um subsite irrelevante. Em
   01/11/2026 nenhum dos 95 subsites com `URLHome` carecia da lista, por isso não morde hoje. O 404 da
   `_api` para `getByTitle` é inequívoco ("a lista não existe", nunca "lista vazia", que é 200 com
   `results: []`): **rebaixar esse caso a aviso + pular, mantendo `Err` para qualquer outro status.**
   Pede um GET que veja o status (o `get_string` do kit retenta o 404 três vezes e devolve `Err`), um
   marcador no cache para o `--usecache` reproduzir o pulo, e um teste. Fazer na próxima vez que alguém
   tocar o arquivo.
2. **Observar nos logs de auditoria as FAQs de público interno (D-SPFAQ-7).** Entrou "ingerir todos",
   como o portal mostra. Pela categoria do Home na `Homes 360`, as claramente internas são **62 das
   2.665** (planejamento e execução orçamentária e financeira 41, trabalho a distância 12, entidades da
   administração 4, segurança da informação 3, RH 1, financiamento público 1; "previsão orçamentária e
   arrecadação", 58, é duvidosa). Se elas começarem a aparecer no top-10 de perguntas de contribuinte,
   a evidência para filtrar pela categoria estará na seção ADERÊNCIA dos logs — e o filtro é uma lista
   fechada de categorias no `faqs.rs`, com teste.

## 15. Cache dos scrapers: o remédio é **validação na leitura**, não atomicidade (aberta — 2026-08-11)

_Era a §36 do [auli_pendencias.md](auli_pendencias.md)._

Levantada pela auditoria de escrita não atômica do item 7 (PR #143), e deliberadamente **não
corrigida ali** — a correção óbvia resolveria o sintoma menor e deixaria o maior de pé.

O [`cache::write`](auli-server/crates/scrapers/auli-scraper-kit/src/cache.rs) grava HTML por URL com
`fs::write` direto. Uma queda no meio deixa HTML **parcial** — que **parseia**, produzindo coleta
silenciosamente incompleta a partir de um arquivo que ninguém suspeita. Mas tornar a escrita atômica
não resolve o caso maior: um cache **válido porém obsoleto** passa em qualquer verificação de
integridade estrutural, e nenhum `rename` o pega.

Há um segundo argumento contra a atomicidade aqui, do desenho do próprio kit: o `write` é
_best-effort_ por contrato — se falha, loga e a coleta segue. Escrita atômica só mudaria o modo de
falha de "arquivo pela metade" para "arquivo ausente", e **ausente o `read` já sabe tratar** (ele
devolve `None` para arquivo vazio ou ilegível, e o scraper refaz a requisição).

**O desenho, se virar TAREFA:** validação de completude na LEITURA — um sentinela escrito por último,
ou tamanho/hash no nome do arquivo. E merece **medição antes**: com que frequência o cache é lido
depois de uma interrupção? Se for raro, a dívida pode ficar registrada e não ser paga.
