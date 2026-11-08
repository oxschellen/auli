# ESTUDO-MA-Faqs — Perguntas Frequentes da SEFAZ-MA (coleção `faqs` da entidade `ma`)

**Data:** 2026-11-05 (1ª versão, só índice de busca) · **reescrito em 2026-11-06 após sondagem direta** (§1, §6).
· **Estado:** **v1 implementada — API JSON do próprio portal de serviços, 336 perguntas** (decisões em §4,
resultado em §8). · **Órgão:** Secretaria de Estado da Fazenda do Maranhão (SEFAZ-MA). · **Hoje a
`ma` tem só `servicos`** (portal-sgc, 38 itens, PR #34; cache de 03/11 em `data/ma/raw/cache/servicos/`).

> **TL;DR — a FAQ está no mesmo portal e na mesma API que o `auli-scraper-ma` já raspa.**
>
> A página `https://portal-sgc.sefaz.ma.gov.br/portal/perguntas-frequentes` (SPA Angular) lê
> **`GET /sgc/api/faq/perguntas/faq?publicado=true`** com o token anônimo de sempre (header `AuthorizationPortal`,
> login com as credenciais públicas do bundle — D-MA-AUTH).
> **Uma chamada, 336 perguntas** com a resposta inteira em HTML limpo, em **75 categorias / 20 raízes, 2 níveis**.
> Conteúdo **atual**: Reforma Tributária do Consumo (IBS/CBS), Portaria 212/2026 (ativação de empresas), REFIS
> 2025/2026, novo download de DF-e, SIFMA, EFD, SEFAZ.NET, IPVA. Nenhum host do MA tem `robots.txt` (404 nos quatro).
> Molde: FAQ do MT (1 GET JSON, link único) + o login que já está no `ma.rs`. É a 2ª coleção do `auli-scraper-ma`.
>
> A 1ª versão deste estudo descrevia o portal JSF legado (`portalsefaz`), cuja FAQ hoje **redireciona para o portal
> novo**, e partia de duas premissas falsas (MA sem scraper, robots bloqueando). §7 registra o que mudou.

---

## 1. Hierarquia discovery-first — o que foi avaliado

| Fonte / rota | Veredito | Evidência (06/11/2026) |
|---|---|---|
| **portal-sgc** `GET /sgc/api/faq/perguntas/faq?publicado=true` | **fonte v1** — 336 perguntas, JSON UTF-8, token anônimo (§2) | chunk lazy `src_app_portal_faq_faq_module_ts.745cb1121c1fb1509b4d.js` (nome via `runtime.js` `n.u`): `endPointFaq=`${baseUrl}/faq/perguntas``, `questions(e){const t={publicado:!0,...e};return this.client.get(this.endPointFaq+"/faq",{params:t})}`; o resolver da rota chama `questions()` sem parâmetro; `baseUrl` = `environment.api` = `/sgc/api`; rota `path:"perguntas-frequentes"` no chunk do `PortalModule`. Sem token → 401. |
| mesma rota **sem** `publicado=true` | **não usar** — 353 itens; os 17 a mais são a categoria 103 "DFE.02 - CANAL VERDE" (`ativo=false`), que a página não mostra | GET comparado; os 336 comuns vêm idênticos campo a campo |
| PDF "IPVA, ICMS, ITCD – Principais dúvidas dos contribuintes" (`/sgc/api/portal/arquivos/public/identificador?identificador=3affbbd3-…`) | **fora da v1** (D-MAFAQ-4) — AGCEN, jun/2023, 24 p., roteiro por tópicos (9 temas: malha, DF-e, IPVA, cadastro, ICMS, processos, trânsito, ITCD, emissão de documentos), **não** pergunta-resposta | link no conteúdo 3197 "Manuais e Notas de Orientação" (já no acervo de serviços); `pdfinfo` CreationDate 14/07/2023 |
| Demais links de "Manuais e Notas de Orientação" (38 arquivos em `/sgc/api/portal/arquivos/public/…`, 33 deles no host `ui-sgc`) | fora — manuais de sistema e notas de orientação, não FAQ | `data/ma/raw/cache/servicos/…conteudos_3197.html` |
| **Legado JSF** `sistemas1…/portalsefaz/jsp/faq/view.jsf` | **não existe mais** — redireciona para `https://portal-sgc.sefaz.ma.gov.br/portal` (0 `ViewState`, 0 `RESPOSTA`) | GET com `-L` |
| `portal.sefaz.ma.gov.br` | mesmo front do portal-sgc (`index.html` com md5 igual); `/portalsefaz/…` = 404 | GET |
| "Portal do Conhecimento" | fora — é o Moodle de cursos (`http://cursos.sefaz.ma.gov.br/moodlecursos/`) | item do acervo de serviços (`ma-servicos-cidadao.json`) |
| FAQ de NFC-e / CT-e / MDF-e | fora — links para `nfce.encat.org`, `cte.fazenda.gov.br`, `mdfe-portal.sefaz.rs.gov.br` (conteúdo nacional/de outra UF) | conteúdos 3567, 3542, 3532 |
| FAQ "Dívida Ativa" | fora — é da PGE-MA (`pge.ma.gov.br`), outra entidade | índice de busca (1ª versão) |
| Headless | desnecessário — a página só desenha o JSON | — |

**Robots por host (lidos em 06/11, UA AuliBot, com o intermediário):**

| host | `robots.txt` | o que é |
|---|---|---|
| `portal-sgc.sefaz.ma.gov.br` | **404** (nginx) | portal atual — serviços e FAQ |
| `portal.sefaz.ma.gov.br` | **404** (nginx) | o mesmo front |
| `sistemas1.sefaz.ma.gov.br` | **404** (Apache) | legado JSF e apps |
| `www.sefaz.ma.gov.br` | **404** (Apache) | raiz institucional |

RFC 9309 §2.3.1.3: 4xx ⇒ sem restrição. Nada a decidir.

**TLS.** `portal-sgc` e `portal` **passaram a mandar a cadeia inteira** (folha → GlobalSign GCC R3 DV TLS CA 2020 →
raiz R3; `openssl s_client` verifica ok) — o gotcha da D-MA-TLS sumiu ali, e o intermediário embutido no
[`ma.rs`](../../auli-server/crates/scrapers/auli-scraper-ma/src/ma.rs) continua valendo como âncora (mesmo emissor;
vence em 18/03/2029) caso o servidor regrida. `sistemas1` e `www` **ainda mandam só a folha** (curl exit 60; com
`-s`, falha calado e a saída sai vazia) — sondar esses hosts exige `--cacert` com aquele PEM.

## 2. Modelagem v1 — FAQ do portal-sgc

Uma chamada autenticada: `login()` do `ma.rs` → `GET {API_BASE}/faq/perguntas/faq?publicado=true` com
`AuthorizationPortal: Bearer …` → array JSON. Cada item:
`{id, pergunta, resposta, ativo, publicado, categoria}`, com `categoria = {id, nome, ativo, publico, ordem,
categorias, categoriaPai}` e `categoriaPai` do mesmo formato (ou `null`).

| `FaqRaw` | origem |
|---|---|
| `pergunta` | `pergunta` (trim — "Perguntas e Resposta " tem espaço no fim). Texto puro: 0 tags, 0 entidades. 260/336 começam por um código (`RTC.03.008 - …`, `MOT.02.001 – …`, `DTE.05 …`) — **manter**: as respostas citam umas às outras pelo código ("consulte em nosso FAQ o menu SEFAZNET > DTE.05"). |
| `resposta` | `resposta` → texto. HTML de editor rico: `<p>` 3.123, `<br>` 1.289, `<strong>` 1.120, `<li>` 627, `<a>` 370, `<ul>` 241, `<em>` 44, `<u>` 23, `<ol>` 10, `<span>` 4; **0** `<img>`, `<iframe>` ou `data:`. Mesmo tratamento das FAQs do MS/MT: link vira `texto "url"`, bloco vira quebra de linha, entidades decodificadas. |
| `origin` | trilha `<raiz> \| <subcategoria>` (só `<raiz>` para os 56 itens pendurados direto na raiz), espaços colapsados e aparados (`" REFIS - …"`, `"NOVO SISTEMA DE  DOWNLOAD …"`), caixa mantida (D-MAFAQ-3). |
| `url` | `https://portal-sgc.sefaz.ma.gov.br/portal/perguntas-frequentes` para todas — o template não tem `id`, âncora, `fragment` nem `queryParams` por pergunta (é um acordeão `ngbCollapse` com `innerHTML`). Host `portal-sgc`, o mesmo dos serviços e o que as próprias respostas linkam. |

**Números (06/11/2026):** 336 itens, 336 `id` distintos (31 a 742), **335 perguntas distintas** (a repetida é
"Sobre a seção", ids 698 e 682 — D-MAFAQ-2), 336/336 `ativo` e `publicado`. Resposta em texto: mín. 51 caracteres,
mediana 523, p90 1.275, máx. 3.189 (HTML máx. 3.491 bytes); 0 vazias. Dois GETs seguidos devolveram os mesmos bytes.

| raiz | itens | em subcat. | | raiz | itens | em subcat. |
|---|---|---|---|---|---|---|
| ESCRITURAÇÃO FISCAL DIGITAL - EFD ICMS IPI | 64 | 63 | | SUBSTITUIÇÃO TRIBUTÁRIA | 10 | 0 |
| DOCUMENTOS FISCAIS ELETRÔNICOS | 49 | 48 | | NOTA LEGAL | 10 | 0 |
| IPVA | 28 | 27 | | FISCALIZAÇÃO DE MERCADORIAS EM TRÂNSITO | 9 | 8 |
| SEFAZ.NET | 27 | 26 | | SIFMA (agronegócio e vegetação nativa) | 9 | 0 |
| REFIS – débitos de ICMS | 23 | 22 | | LICITAÇÕES E CONTRATOS | 8 | 0 |
| REFORMA TRIBUTÁRIA DO CONSUMO - RTC | 22 | 21 | | REFIS – débitos de IPVA | 6 | 5 |
| TELECOMUNICAÇÕES | 21 | 20 | | ÍNDICE DE PARTICIPAÇÃO DOS MUNICÍPIOS - IPM | 4 | 0 |
| ATIVAÇÃO DE NOVAS EMPRESAS – PORTARIA nº 212/2026 | 16 | 15 | | COMÉRCIO EXTERIOR | 1 | 0 |
| NOVO SISTEMA DE DOWNLOAD DE DF-e | 16 | 15 | | CADASTRO/ATIVAÇÃO DE EMPRESA | 1 | 0 |
| INSCRIÇÃO ESTADUAL | 11 | 10 | | SIMPLES NACIONAL | 1 | 0 |

**Como a página desenha (e por que isso importa).** O componente agrupa por `categoria.id`; raízes são os grupos
sem pai, ordenadas por `categoria.ordem`; cada grupo-filho pendura no grupo do pai; dentro do grupo, perguntas por
`pergunta` (`localeCompare`). Um grupo só existe se a categoria tem pergunta **direta** — por isso cada raiz com
subcategorias tem exatamente 1 pergunta própria, a de apresentação ("Sobre o IPVA", "Sobre a EFD", "Sobre a seção"…).
Hoje 0 categorias órfãs (filho cujo pai não tem pergunta direta, que a página não mostraria).

**A `categoriaPai` aninhada vem rasa.** Dentro de uma subcategoria, a mãe chega com `ordem: null` e `ativo: false` em
todas — valores de omissão, não dado. Na visão da própria categoria (a da pergunta pendurada nela) as 75 têm
`ativo=true`, as 20 raízes têm `ordem` (0 a 20, sem o 13) e as 55 subcategorias não têm. Nome e ordem de uma raiz,
portanto, só valem lidos da pergunta pendurada direto nela — que é de onde a página também os tira.

**Quem decide o que aparece é o filtro do servidor.** O 13 que falta é a raiz "DFE.02 - CANAL VERDE", `ativo=false`:
seus 17 itens dizem `publicado=true` no próprio item e só saem pelo filtro. `publico=false` nas 75, sem efeito
visível. ⇒ **sempre mandar `publicado=true`**, exatamente como a página, e não filtrar por flag nenhum do nosso lado.

**Identidade.** O nome do `.md` sai de `url + "\n" + pergunta`
([`nome_faq_de`](../../auli-server/crates/auli-contract/src/mddoc.rs)); com a `url` única, **a pergunta é a
identidade** e as duas "Sobre a seção" colidiriam — uma sobrescreveria a outra em silêncio. Ver D-MAFAQ-2.

**Ordem determinística:** raízes pela `ordem` da própria categoria (sem ordem vai para o fim; nome desempata);
subcategorias por `id` (é o que a página faz); perguntas por `pergunta`, `id` desempata. A API devolveu ordem estável
hoje, mas gravar na ordem dela já gerou falso "raw mudou" no ES.

**Itens que não são pergunta** (23 sem "?"): as 12 apresentações de raiz ("Sobre …", "Perguntas Frequentes –
Telecomunicações", "Benefício Fiscal de IPVA 2025…"), os 7 motivos de suspensão `MOT.02.00x` (verbetes com
"Situação:"/"Procedimento:" — conteúdo bom), 1 tópico de Licitações ("e) Definição do Sistema de Registro de
Preços…") e 3 ponteiros cuja resposta é só uma lista de links (247 Simples Nacional/MEI, 249 Cadastro "CLIQUE AQUI",
299 Comércio Exterior). Entram como estão: os ponteiros carregam as URLs no texto, e a apresentação é o que ancora a
raiz na página.

**Guardas (`Err`):** resposta fora do formato (não é array, campo ausente ou de outro tipo); lista vazia; `id`
repetido; pergunta vazia; pergunta repetida depois da D-MAFAQ-2; categoria sem nome; categoria órfã (não teria onde
aparecer). Resposta curta é legítima ("d) Qual o ano safra devo declarar?", 68 caracteres) — não avisa.

## 3. Fora do escopo v1

- **Canal Verde** (17 perguntas, categoria inativa) — o servidor tira, a página não mostra.
- **PDF da AGCEN** (2023) e os manuais — ver D-MAFAQ-4.
- **Links das respostas** para arquivos em `ui-sgc.sefaz.ma.gov.br` (128), CONFAZ (58), `sistemas1` (31) etc. —
  ficam como URL no texto; não seguimos.

## 4. Decisões

**Decididas em 06/11/2026: as quatro conforme a recomendação.**

- **D-MAFAQ-1 — Fonte v1 = a API do portal-sgc.** Recomendo **sim**: é a FAQ que o próprio portal mostra, atual,
  sem robots, sem sessão de usuário, e o crate já tem login, TLS e cache.
- **D-MAFAQ-2 — As duas "Sobre a seção".** (a) **qualificar só a pergunta repetida com a raiz**:
  `Sobre a seção (REFORMA TRIBUTÁRIA DO CONSUMO - RTC)` / `Sobre a seção (ATIVAÇÃO DE NOVAS EMPRESAS – PORTARIA nº
  212/2026)` — regra genérica, aplicada só quando houver colisão, sem perder conteúdo; (b) tirar as duas (texto de
  apresentação, valor de busca baixo) — regra editorial ad hoc, e uma colisão futura ainda pararia a coleta;
  (c) `?id=` no link — o SPA ignora, seria link inventado. **Recomendo (a).**
- **D-MAFAQ-3 — Trilha.** `<raiz> | <subcategoria>`, espaços normalizados, caixa mantida (como a D-MTFAQ-7). Sem
  prefixo de programa: as raízes já dizem de quê (IPVA, SEFAZ.NET, REFIS…). **Recomendo.**
- **D-MAFAQ-4 — PDF "Principais dúvidas" (AGCEN, 2023).** **Recomendo fora:** é roteiro por tópicos, não P&R,
  tem 3 anos e os temas dele (malha, DF-e, IPVA, cadastro, ITCD) estão cobertos pela FAQ. Reavaliar só se faltar
  algo na FAQ.

## 5. Plano de implementação (executado — §8)

Molde **MT** (`auli-scraper-mt/src/faqs.rs`, PR #185):

1. `auli-scraper-ma/src/faqs.rs` novo; `main.rs` passa a aceitar `servicos | faqs | all`. Reusa `login()`, o agente
   com o intermediário e o cache do `ma.rs` (namespace `faqs`). → verificar: 1 login + 1 GET por rodada.
2. Conversão, trilha, ordem, D-MAFAQ-2 e guardas da §2. → testes sobre um recorte real da resposta de 06/11; cada
   regra desligada uma a uma tem de fazer o teste dela falhar.
3. Registry: a `ma` ganha `faqs`; `auli-frontend/src/shared/entities.ts`; `scripts/tools/update-ma-faqs.sh`
   (apaga o cache antes, guarda de encolhimento contra a rodada anterior); REGISTRO#ma-faqs, SCRAPERS.md, README.
   → verificar: rodada sem cache = 336; `--usecache` = árvore e snapshot idênticos a menos do `coletado_em`.
4. `auli-collections ma` → 336 `.md` com nomes únicos e 336 vetores; os artefatos de serviços saem idênticos; smoke
   `/v1/retrieve` com 5 perguntas reais (ex.: "como consultar débitos da empresa", "ativação de inscrição Portaria
   212", "adesão ao REFIS de IPVA").

## 6. Evidência e reprodução

Coletado em 06/11/2026 (não versionado): `index.html` do portal-sgc e do `portal`, `env.js`, `runtime.js`,
`main.js`, os chunks `PortalModule`, `FaqModule` e `common`, `faq.json` (336, 469.791 bytes), a resposta sem filtro
(353, 477.131 bytes), o PDF da AGCEN. Para refazer:

```sh
# credenciais: ID_CLIENTE e SENHA_PUBLICA do ma.rs; inter.pem = GLOBALSIGN_INTERMEDIATE_PEM (hoje dispensável
# no portal-sgc, que manda a cadeia; obrigatório em sistemas1/www)
A=https://portal-sgc.sefaz.ma.gov.br/sgc/api; C='--cacert inter.pem'
TOK=$(curl -sS $C -H 'Content-Type: application/json' \
  -d '{"id_cliente":"41","senha":"<SENHA_PUBLICA>","portal":true}' $A/login | jq -r .authtoken)
curl -sS $C -H "AuthorizationPortal: $TOK" "$A/faq/perguntas/faq?publicado=true" | jq length   # 336
curl -sS $C -H "AuthorizationPortal: $TOK" "$A/faq/perguntas/faq" | jq length                  # 353 (com o Canal Verde)
```

## 7. O que a 1ª versão (05/11) dizia e o que a sondagem mostrou

| 1ª versão | 06/11 |
|---|---|
| "O MA ainda não tem scraper de serviços" (e AM/ES/RO/AP/AC/GO/MT "só descoberta parcial") | todos têm; o do MA desde o PR #34 (portal-sgc, 38 serviços) |
| Portal = aplicação JSF `portalsefaz` em `sistemas1` e `portal` | o portal é o SPA Angular + Spring do portal-sgc; `portal.sefaz` serve o mesmo front; o JSF é legado |
| FAQ = `jsp/faq/view.jsf`, perguntas de usuários de 2008–2010, baixa qualidade | essa página redireciona para o portal novo; a FAQ atual tem 336 perguntas escritas pelo fisco, mantidas em dia (cita a Portaria 212/2026, o REFIS 2025/2026 e a RTC) |
| "robots bloqueia o fetch automático nos dois hosts" | nenhum host tem `robots.txt` (404 nos quatro); o bloqueio era da ferramenta de pesquisa, não do site |
| "Carta de Serviços" e "Manuais de Orientação" como fonte de um futuro scraper | são conteúdos do portal-sgc já no acervo (Manuais = conteúdo 3197) |
| "Portal do Conhecimento" desconhecido, talvez FAQ | Moodle de cursos |
| Roteiros ASDIN de 2020 | o que está linkado hoje é o PDF da AGCEN de jun/2023 |
| Recomendação: não começar o MA pelas FAQs; antes, acesso + scraper de serviços | os dois já existem; a FAQ é a próxima coleção natural |

## 8. Implementação (06/11/2026)

`auli-scraper-ma/src/faqs.rs` (`auli-scraper-ma faqs`, ou `all`), reusando `login()`, o agente com o intermediário e
as bases do `ma.rs`. A `ma` ganha `faqs` no registry e no `entities.ts`; `scripts/tools/update-ma-faqs.sh`;
REGISTRO#ma-faqs, SCRAPERS.md, README.

**Rodada de 06/11:** 1 login + 1 GET → 336 perguntas em 20 raízes e 55 subcategorias, 0 repetidas (as duas "Sobre a
seção" qualificadas), 0 respostas vazias; no texto, 0 tags, 0 entidades, 0 `U+FEFF`/nbsp/zero-width. Resposta:
mín. 67 caracteres, mediana 616, máx. 3.189 (em linhas, com os links como `texto "url"`). Árvore: 12 raízes `Menu`
(apresentação + subcategorias) e 8 `Faq`, 336 itens.

**Reprodução:** `--usecache` refaz a árvore idêntica byte a byte e o snapshot idêntico a menos do `coletado_em`.
`auli-collections ma` → 336 `.md` com nomes únicos; os 47 artefatos de serviços (raw, docs, packs) saíram com o
mesmo md5. `build-packs.sh ma faqs` → 336 vetores; o servidor sobe com `ma-faqs — 336 registros`.

**Smoke `/v1/retrieve`** (`entity=ma`, `kind=faqs`, 5 perguntas escritas à mão): o alvo em 1º em 4 de 5 (débitos da
empresa → RTC.03.008; dispensa da ativação → AT.04.001; benefício do IPVA 2025 → IP.BENEF.01.001; correção da
competência do DARE de ST → a "b)" da Substituição Tributária). Na 5ª ("de quanto em quanto tempo se paga o IPVA") o
1º é "Sobre o IPVA", cuja resposta diz "cobrado anualmente" — responde; o alvo IP.01.001 veio em 5º.

**Testes:** 18 novos sobre um recorte real de 14 perguntas (`tests/fixtures/ma-faqs-perguntas.json`). Cada regra —
ordem da raiz pela própria categoria, ordenação de raízes e de perguntas, desambiguação, as sete guardas, os dois
`clean`, o `U+FEFF`, a raiz sem ordem no fim, o `Menu` da raiz com subcategorias, a trilha com a subcategoria, o
link no texto — foi desligada uma a uma e o teste dela falhou.
