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
| 10 | kit | Camada 2 do kit (`validar_contagem`, `absolutize`, `cli::parse_args`; conversor HTML das FAQs) | TAREFA curta, não feita |
| 11 | frota | User-Agent: AuliBot por padrão (D-FROTA-UA) | ✅ decidida e aplicada; restam os usos legados, a testar quando alguém tocar o scraper |
| 12 | frota | Serviços sem atualização desde 25/07 (AP desde 02/08) | ✅ feita: as 27 atualizadas entre 31/10 e 03/11 (AP com o fix de TLS do #190) |
| 13 | ES | O scraper não ordena os itens | ajuste pequeno |
| 14 | SP | FAQs: dois pontos da revisão do #179 | na próxima vez que alguém tocar o `faqs.rs` |
| 15 | kit | Cache: validação de completude na leitura | medir antes de virar TAREFA |
| 16 | AM | Perguntas e Respostas do GCC (D-AMFAQ-6) | sondar o XHR da casca Struts2 |
| 17 | AM | Serviço da listagem sem nenhum público (3223) | ✅ decidida: PF e PJ (02/11) |
| 18 | CE | Os 332 conteúdos do `conteudo-geral` que não são FAQ (Parecer CECON, Comunicados COTRI, Notas Explicativas…) | candidatos a coleção própria (D-CEFAQ-7) |
| 19 | CE | FAQ nova fora da convenção de nome (aviso do D-CEFAQ-2) | medir: `getItem` dos 332 |
| 20 | RS | FAQs: o índice novo `/perguntas-frequentes` como guarda e plano B (D-RSFAQ-2, -6); duas páginas que nunca entraram | D-RSFAQ-3..5 feitas (06/11); guarda e plano B em aberto |
| 21 | AL | FAQs: não existe FAQ pública da SEFAZ-AL — o site institucional está em manutenção (D-ALFAQ-1..5) | ✅ decidida: adiar (06/11); retomar quando o título da raiz deixar de ser "Site em manutenção" |

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
qualquer scraper atual. **(02/11/2026)** As FAQs do AM já leem esses detalhes (`auli-scraper-am/src/faqs.rs`,
[ESTUDO-AM-Faqs.md](ESTUDO-AM-Faqs.md)): o portão `visibleSections`, o valor único de um campo do
`serviceDetails` (`valor_unico`) e a referência `$x` → bloco de texto, lida em bytes de linha em linha
(`bloco_de_texto`). A v2 reusa os três e o cache de detalhes de `data/am/raw/cache/faqs/` — rodando junto com as
FAQs, não custa rede. A árvore tem hoje 243 serviços com detalhe (eram 239). Os **39 serviços link-only** (externo/submenu) NÃO têm página de detalhe → sem
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

**Candidata nova (02/11/2026):** o conversor HTML → texto das FAQs (`html_para_texto`: `<style>`/comentários fora,
link → `texto "url"`, bloco → linha, entidades no fim) tem **cinco cópias** — SP, GO, DF, AM e CE (a do DF com o filtro de
`http(s)`/`mailto` e sem duplicar a URL quando ela é o texto do link; a do AM é a do DF, sem mudança; a do CE é a do AM
com o modo `markdown`, em que a quebra de linha do fonte também vira quebra). Já passou do 2º consumidor; ao extrair,
levar as melhorias do DF e o modo do CE e provar SP, GO, AM e CE byte a byte com `--usecache`.

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
o RJ responde ao AuliBot com a página de "IP bloqueado" (§4 acima).

**Decisão D-FROTA-UA (mantenedor, 02/11/2026): a regra que existe vira oficial — AuliBot por padrão, UA de
navegador só onde o portal exigir, com o motivo comentado no código.** Sem migração em massa. Aplicada no
mesmo dia: a regra está no doc do `kit::USER_AGENT` e no `SCRAPERS.md` ("Regra do UA"), e cada call site do
UA de navegador diz o porquê:

| Motivo | Onde |
|---|---|
| o portal exige | RJ (o AuliBot recebe "IP bloqueado"); CSM do MG (o ServiceNow omite o texto do artigo para o AuliBot) |
| mesmo host do caso acima, sem teste próprio | serviços do MG |
| decisão registrada — robots restritivo a crawlers genéricos | BA (D-BA4), CE |
| **legado, nunca testado com o AuliBot** | serviços de AM, GO (token), MS, MT, PI, PR, RS, SC e SP; FAQs do RS |
| **legado, caminho curl** (o call site passa o `kit::USER_AGENT` explícito) | serviços de GO, SE e RS; FAQs do RS (AJAX) |

**O que fica aberto:** as duas linhas "legado". Não são violação a corrigir já, mas candidatas: quem tocar um
desses scrapers testa o AuliBot no portal e, se o portal responder igual (recoleta com `--usecache` desligado
e diff limpo contra o backup), troca e apaga o comentário de legado. O caminho curl recebe o UA como parâmetro
desde as FAQs do DF (02/11/2026).

**06/11/2026 — serviços de PE no AuliBot (D-PE4).** O motivo da linha "robots restritivo" caducou (o
robots da SEFAZ-PE libera `*`); a troca veio com recoleta verificada — snapshot idêntico ao de 03/11, fora o
`coletado_em` — e PE saiu da tabela.

**06/11/2026 — serviços do DF no AuliBot.** Era o candidato mais barato da linha "caminho curl" (o host já
aceitava o AuliBot nas FAQs). A recoleta com o AuliBot devolveu os 475 serviços idênticos aos de 02/11 (UA de
navegador) e 1 novo do portal (Consultar NFS-e, `codServico=1453`). O HTML mudou também nos links do cabeçalho e
dos anexos (`http://mobile.receita…`, `http://static…`), que o scraper não extrai — mudança do host inteiro desde
02/11, não do UA: a casca das FAQs, pedida com o AuliBot nos dois dias, mostra a mesma troca. DF saiu da tabela.

## 12. Serviços sem atualização desde julho (✅ feita — 2026-11-03)

Data da última coleta no snapshot de serviços (`coleta.coletado_em`):

| Coleta | Entidades |
|---|---|
| 31/10–02/11/2026 | sc, pr, rs, sp, df, es, go, mg, ms, mt, rj, am |
| 03/11/2026 | pi, ba, ce, ac, al, ma, pa, pb, pe, rn, ro, rr, se, to, ap |

**Como atualizar.** `scripts/tools/atualizar-servicos.sh <id>` por entidade (faz backup, apaga o cache, coleta,
deriva e refaz os vetores). Comparar com o backup em `/tmp/auli-atualizacao/<id>` **por link e título**, não
pela posição. Depois, `./start_server.sh` e `scripts/deploy-frontend.sh`.

**AM (02/11/2026):** 278 → 282 — 4 novos, nenhum removido, 8 renomeados (mesmo link), 4 descrições editadas,
nenhuma mudança de público ou classe. O 283º, sem rota de perfil, entrou depois com PF e PJ (§17) → 283.

**PI (03/11/2026):** 30 → 28 — saíram da Carta de Serviços os três de IPVA ("Notificações de débitos de IPVA",
"Notificações de lançamento IPVA", "Pagamento IPVA"), que a busca anônima não acha mais no CMS, com nenhum pai; entrou
"SIAFE-PI"; o "Acervo Normativo" foi recriado (mesmo texto, `_id` novo, sem `friendlyUrl` → o link passa a ser
o `_id`); "Serviço de Atendimento ao Consumidor (SAC)" virou "…ao Cidadão (SAC)". O IPVA agora vive em "Temas e
categorias - Portal Institucional" (`69653110…`, 14 temas, o do IPVA com 12 subtemas) — taxonomia do portal,
fora do escopo Carta-only.

**BA (03/11/2026):** 205 → 208 — 3 novos ("Celebração de Transação por Adesão e Emissão de DAE", "Combustíveis -
Autorização para Emissão de NF-e de Ajuste de Estoque", "Senha de Acesso aos Sistemas Sefaz - Recuperação da Frase
Lembrete"), nenhum removido nem renomeado, 10 descrições editadas (as três fichas de Senha passam a orientar o
acesso pelo Gov.br), nenhuma mudança de público ou classe. A classe do primeiro novo, "por Adesão e Emissão de
DAE", é o `<small>` do próprio portal, no padrão do irmão "…por Adesão ou Individual…".

**CE (03/11/2026):** 383 → 374 — 11 removidos (os 9 de MFE/software house e os 2 de Difal: "Emitir DAE referente
ao ICMS Difal e FECOP", "Realizar Cálculo do Diferencial de Alíquotas (Difal)"), 2 novos ("Consulta de Ata das
Sessões de Julgamento…", "Contestação de Notificação - Simples Nacional"), 3 renomeados com o mesmo `_id` (o slug do
link muda junto, então parear pelo `_id` do fim do link): "Recepcionar Documentos Processuais no Contencioso" →
"Impugnar o auto de infração", "Solicitar Consulta sobre Aplicação de Normas e Processos Tributários" → "Solicitar
Consulta Tributária", "…Declaração de Livre Trânsito (DLT)" → "…Declaração de Conteúdo Eletrônica (DCe)"; 3
descrições editadas. O `hits` caiu junto (393 → 384) e o gap do índice segue 10, então a queda é do catálogo, não
da paginação.

**Os 12 de julho (03/11/2026):**

- **AL, PA, PB, AP:** idênticos byte a byte (60, 34, 101, 49). O AP só passou depois do fix de TLS (#190): o
  certificado novo do portal (09/2026) é do Soluti V2 GR46, e o servidor manda intermediários que não são o
  emissor (`UnknownIssuer`) → o Soluti entrou como trust anchor, como no AC.
- **AC:** 17 → 19 — o post da "Certidão Negativa de Débitos" (`?p=322`) foi trocado por um novo, "Certidão Negativa
  de Débito" (`?p=24450`); entraram "Certidão Negativa de Débito Inventário" e "Certidão da Dívida Ativa".
- **MA:** 38 = 38 — 5 descrições editadas (ITCD ganhou o aviso do módulo novo).
- **PE:** 35 = 35 — "ICD - Causa Mortis e Doações" → "ITCMD - Causa Mortis e Doação"; o "IPVA" passou a apontar
  para o e-Fisco (`PRConsultarResumoIPVA`).
- **RN:** 15 = 15 — 2 links de `uvt.set.rn.gov.br` → `uvt.sefaz.rn.gov.br`; o card "Carta de Serviços" agora
  aponta para um design no Canva (é o `acf.link` do próprio portal).
- **RO:** 194 → 193 — 4 removidos ("Adesão ao REFAZ ICMS 2026 (até 31/07/2026)", "Solicitação da Inclusão" e "…da
  Alteração de Produtos na Pauta Fiscal", "Protocolo de entrada e trânsito"), 3 novos ("Dúvidas sobre Indeferimento
  da Opção pelo Simples Nacional", "Calculadora de Rendimentos Recebidos Acumuladamente (RRA)", "Sefin App"), 5
  renomeados (4 com slug novo, pareados pelo `_id`), 4 descrições editadas. Os 3 novos vêm com a descrição de
  enchimento do portal (`ㅤ`, U+3164), como 3 antigos — é o portal, não o scraper.
- **RR:** 17 = 17 — saiu "Gerar CSC da NFC-e", entrou "Consultar NFC-e" (Cidadão e Empresa).
- **SE:** 91 = 91 — 20 descrições editadas, quase todas por links que migraram de `www.sefaz.se.gov.br` para
  `portais-fazendario.apps.sefaz.se.gov.br` (26 de 47). A Carta (`servicos_cidadao.aspx`) ainda está no `www`; se ela
  migrar também, o scraper do SE muda de host.
- **TO:** 45 → 47 — 2 novos: "Autorizar Envio de Escrituração Fiscal Digital Substitutiva (Retificadora)" em 1ª e
  em 2ª Instância (classe "Novos Serviços Fiscais").

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

## 16. AM — Perguntas e Respostas do GCC (D-AMFAQ-6)

_Aberta em 02/11/2026, no estudo das FAQs do AM ([ESTUDO-AM-Faqs.md](ESTUDO-AM-Faqs.md))._

`sistemas.sefaz.am.gov.br/gcc/menucontribuinte/publico/perguntasRespostas.action` responde 200 (12.830 B), mas é
só a casca Struts2-jQuery (`jQuery.struts2_jquery…`, 1 link, texto vazio): o conteúdo chega por JS. As FAQs do AM
que entraram são as do Portfólio de Serviços.

**Próximo passo.** Abrir a página num navegador, capturar o XHR que traz as perguntas e ver se ele é um GET/POST
anônimo que o `ureq` repete. Se for, é a 2ª fonte do `faqs.rs` do AM, nos moldes da 2ª fonte do DF.

## 17. AM — serviço da listagem sem nenhum público (✅ decidida — 2026-11-02)

Na atualização dos serviços de 02/11/2026, o 3223 "Regularização de Pagamentos Efetuados IPVA"
(`/portfolio-servicos/detalhes/3223`) está na listagem `/portfolio-servicos/todos`, mas em nenhuma das 3 rotas de
perfil (PF, PJ, Órgãos Públicos). O público do AM sai do pertencimento às rotas, e o `build_servicos` do `am.rs`
pula serviço sem público, com aviso — por isso o snapshot tem 282 dos 283. Em julho não havia caso assim (o comentário
do `am.rs` ainda diz "não observado"). O inverso também segue: o 1436 está numa rota de perfil e não na listagem.

A própria página de detalhe também não declara público (`"perfis":[]` no `serviceDetails`).

**Decisão do mantenedor (02/11/2026): o 3223 é de Pessoa Física e Pessoa Jurídica.** Está no `am.rs` como
`PUBLICO_SEM_ROTA`, aplicado só quando o serviço não está em nenhuma rota — se o portal o puser numa, vale o portal.
Outro serviço sem rota continua pulado, com aviso. Com ele, 283 serviços.

## 18. CE — o resto do catálogo `conteudo-geral` (D-CEFAQ-7)

_Aberta em 03/11/2026, na implantação das FAQs do CE ([ESTUDO-CE-Faqs.md](ESTUDO-CE-Faqs.md))._

As FAQs leem 96 dos 428 conteúdos do `conteudo-geral`. Os outros 332, pela mesma API (`getChildren` + `getItem`), são
na maioria textos interpretativos: **78 "Parecer CECON"** (e mais alguns "Parecer nº …"), 71 Comunicados COTRI, 38
Notas Explicativas, 28 Comunicados NUPAF, 12 Normas de Execução e ~104 avulsos (Instruções Normativas, portarias,
páginas de tema como "Prazos relacionados ao ICMS", itens da Malha Fiscal). O estudo chamava todos de "COMUNICADO
COTRI"; a contagem por nome é de 03/11/2026.

**Próximo passo.** Os Pareceres CECON são o candidato mais forte: é a coleção `pareceres` que SP, PR, SC e RS já têm,
sem custo de descoberta (o transporte é o das FAQs). Estudo próprio para o shape do `contentMarkdown` deles e para
separar o que é parecer do que é comunicado.

## 19. CE — FAQ nova fora da convenção de nome (D-CEFAQ-2)

_Aberta em 03/11/2026, na implantação das FAQs do CE ([ESTUDO-CE-Faqs.md](ESTUDO-CE-Faqs.md))._

A seleção dos artigos de FAQ é pelo nome (`d[úu]vidas|pergunta|faq`), o critério editorial da SEFAZ-CE. O estudo
previa avisar quando um conteúdo de nome fora do padrão tivesse ≥ 3 blocos `section-wrapper` — sinal de FAQ nova fora
da convenção —, mas isso pede o `getItem` dos 332 (~2,5 min por rodada), e a recomendação do estudo foi ficar só nos
96. Não implementado.

**Próximo passo.** Medir uma vez: `getItem` dos 332 e contar os blocos. Se nenhum tiver seções, a pendência fecha; se
algum tiver, decidir entre ampliar o filtro de nome e ler todos a cada rodada.

## 20. RS — o índice novo das FAQs e três defeitos da coleta atual (D-RSFAQ-1..6)

_Aberta em 06/11/2026, depois da troca de raiz do #201. Estudo: [ESTUDO-RS-Faqs-Indice.md](ESTUDO-RS-Faqs-Indice.md)._

O `/perguntas-frequentes` novo é uma página só (5 MB, 1 GET, passa com o AuliBot) com 1.854 perguntas canônicas de
233 páginas, com o mesmo `panel-N` do AJAX de cada página. Todo painel dele está na coleta pela raiz `-antigo`, com o
mesmo texto (1.845 de 1.846); a coleta tem a mais 107 cópias (o mesmo painel em duas URLs) e as 29 do Piloto RTC –
IBS, que o índice não lista. A trilha sai dele igual ao breadcrumb em 94% das páginas. Contra: é gerado, não ao vivo
(publicação mais recente 30/10 × 06/11 nas páginas).

**Proposta (decisão do mantenedor):**

- **D-RSFAQ-1** — a caminhada pela raiz `-antigo` segue sendo a fonte.
- **D-RSFAQ-2** — o índice vira guarda: 1 GET por rodada; página inteira do índice ausente da coleta é `Err` (pega a
  queda silenciosa de subárvore), painel isolado é aviso, índice indisponível é aviso.
- **D-RSFAQ-3** — tirar das respostas o texto dos botões "Copiar link" / "Copiar resposta": hoje está nas **1.990**
  (e já estava nas 1.947 de agosto), e vai para os `.md` e para os vetores.
- **D-RSFAQ-4** — deduplicar por `panel-N` (−107).
- **D-RSFAQ-5** — link por pergunta (`<página>#panel-N`), junto com a D-RSFAQ-3 para re-vetorizar uma vez só.
- **D-RSFAQ-6** — plano B se o `-antigo` sair: o índice + a caminhada do menu `/faq-piloto-rtc-ibs`; antes, medir a
  defasagem do índice (1 GET semanal por 2–3 semanas).

**06/11/2026 — D-RSFAQ-3..5 feitas** (botões fora das respostas, uma ocorrência por painel, link por pergunta):
1.990 → 1.883 FAQs, respostas iguais às antigas menos o texto dos botões — [ESTUDO-RS-Faqs-Indice.md
§11](ESTUDO-RS-Faqs-Indice.md).

**Achado da mesma rodada:** `/faq-nfcom` e `/manual-para-declaracao-de-itcd-faq` nunca entraram no acervo (nem em
agosto): o ureq recusa o corpo ("stream did not contain valid UTF-8"), e o índice também não as lista. Sondar a
codificação (e se são FAQ) antes de decidir.

**Próximo passo.** Decidir a D-RSFAQ-2 (guarda pelo índice); a D-RSFAQ-6 só se o `-antigo` sair do ar.

## 21. AL — FAQs adiadas: o site da SEFAZ-AL está em manutenção (D-ALFAQ-1..5)

_Aberta e decidida em 06/11/2026. Estudo: [ESTUDO-AL-Faqs.md](ESTUDO-AL-Faqs.md)._

**Conclusão: não há FAQ pública da SEFAZ-AL para coletar hoje.**

- A FAQ morava no site institucional (`www.sefaz.al.gov.br/perguntas-frequentes`, Joomla/K2), que está em manutenção
  sem data. O host devolve **a mesma página para qualquer caminho** — `/`, `/perguntas-frequentes`, `/robots.txt`,
  `/sitemap.xml`, um caminho inventado: HTTP 200, 46.523 bytes, md5 `dd07faf6…`, título "SEFAZ-AL | Site em
  manutenção", "Notícias e conteúdos voltam em breve". Nenhum host de site novo (`novo.`, `portal.`, `www2.`…) existe.
- Fora dele não há FAQ: o Portal do Contribuinte (38 rotas no bundle, nenhuma de FAQ), o ITCD, o IPVA on-line, a Nota
  Fiscal Cidadã (só um redirecionamento — diferente da Nota MT), o `gcs2` (só legislação: `CAT001` e `CAT017`) e a
  API do Portal Alagoas Digital (26 campos por serviço, nenhum de pergunta).
- O canal de dúvidas é a **Nise**, assistente de WhatsApp `(82) 3216-9999`, sem base de conhecimento pública.
- A última captura da FAQ no Wayback é de 18/07/2025; o arquivo não é fonte (o link oficial precisa abrir) e o
  conteúdo da captura não foi lido. A expectativa para quando voltar é modesta (dezenas a poucas centenas de
  perguntas), mas é hipótese.

**Decisão do mantenedor (06/11/2026): adiar** (D-ALFAQ-1) — sem TAREFA, sem código, sem cache. As outras propostas do
estudo seguem como estão: sem Wayback como fonte (D-ALFAQ-2), sonda manual (D-ALFAQ-4), cartilhas em PDF fora
(D-ALFAQ-5).

**Gatilho de retomada (D-ALFAQ-3):** o título da raiz deixar de ser "SEFAZ-AL | Site em manutenção" — o status já é
200 e não vai mudar; o título vai. 1 GET:

```bash
curl -sS -A 'AuliBot/0.1 (+https://github.com/oxschellen/auli)' https://www.sefaz.al.gov.br/ | grep -o '<title>[^<]*'
```

**Próximo passo.** Nenhum até o gatilho. Quando disparar: salvar o HTML, sondar `/perguntas-frequentes` e
`/robots.txt` e reabrir o estudo pelo checklist do §4.

**À parte:** o `gcs2.sefaz.al.gov.br` é o gestor de legislação da SEFAZ-AL — ponto de partida se a `al` um dia ganhar
uma coleção `legislacao` (estudo §6).
