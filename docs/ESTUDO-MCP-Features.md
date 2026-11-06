# ESTUDO-MCP-Features — Explicar as cinco ferramentas MCP na aba "Conectar sua IA"

**Data:** 03/11/2026 · **Status:** executado em 05/11/2026 (branch `feat/mcp-tab-ferramentas`), com
os ajustes da revisão registrados em §8 — os textos de §4 já são os finais.
**Base:** `main` em `20403c4` (03/11/2026).
**Leitura prévia:** `auli-server/crates/auli-cli/src/mcp.rs` (cabeçalho e as cinco `#[tool]`),
`auli-frontend/src/pages/mcplist/McpList.tsx`, `manuais/manual-auli-mcp-claude.md` e
`manuais/README.md`.

> Documento único para o Claude Code. Trabalhe **um passo de cada vez** e confirme comigo (Carlos)
> antes de qualquer push ou merge. Branch nova a partir do `main`; nada direto na `main`.

---

## 1. O problema

O servidor MCP (`/mcp`, via rmcp, no mesmo processo do chat) expõe **cinco ferramentas**, documentadas
no cabeçalho de `mcp.rs`:

| Ferramenta | Parâmetros | O que faz |
| --- | --- | --- |
| `listar_entidades` | — | UFs indexadas e totais por coleção (pareceres, TARF, legislação, serviços, FAQs); coleção com zero docs é omitida da linha |
| `buscar_pareceres` | `uf`, `pergunta`, `top_k` (1–20, padrão 5), `colecao` (`pareceres` padrão ou `tarf`) | Busca semântica na jurisprudência; devolve número + assunto + sinopse + link, **sem corpo** |
| `obter_parecer` | `uf`, `numero`, `colecao` (a mesma da busca) | Corpo integral de **um** documento, pelo número exato |
| `consultar_servicos_faqs` | `uf`, `pergunta` | Bloco único com até 5 serviços e 10 FAQs — o mesmo contexto que o chat usa |
| `buscar_legislacao` | `uf`, `pergunta` | Pares pergunta/resposta de lei, com dispositivo citado e link oficial |

Na UI, porém, a aba **Conectar sua IA** (`McpList.tsx`) só encaminha aos manuais de instalação: a
tela de lista fala do servidor e de quem configura, mas **não diz o que dá para pedir**. O manual do
Claude explica as três frentes (legislação / pareceres / serviços+FAQs) e dá bons exemplos, mas não
liga cada frase à ferramenta que ela aciona. O `manuais/README.md` lista as cinco ferramentas numa
linha — e não é servido pelo frontend.

Resultado: quem conecta o Auli ao seu assistente descobre as ferramentas por tentativa, ou lendo o
JSON de `tools/list`.

## 2. O que muda

Duas entregas, uma de código e uma de prosa:

- **(A) Bloco "O que dá para pedir" na tela de lista de `McpList.tsx`**, entre a tabela de manuais e
  "Os dois Copilot": parágrafo de abertura, tabela com as cinco ferramentas (Ferramenta / O que faz /
  Diga, por exemplo) e parágrafo de fechamento sobre o encadeamento e as três frentes. Mockup
  aprovado por Carlos em 03/11/2026 (canvas "Mockup — Conectar sua IA").
- **(B) Seção "As cinco ferramentas, e como pedi-las" em `manuais/manual-auli-mcp-claude.md`**, entre
  "Teste se funcionou" e "Como perguntar bem", com o **mesmo conteúdo** do bloco (A) em markdown.

Fora do escopo: os outros três manuais, o `README.md` da raiz, o `manuais/README.md` (já tem a lista
de uma linha; não mexer) e qualquer mudança no servidor. A seção pode ser espelhada nos demais
manuais depois, em tarefa própria.

## 3. Decisões

- **D-MCPUI-1 — Texto hardcoded no TSX.** Mesma doutrina do array `MANUAIS`: cinco itens não
  justificam um JSON gerado. Um array `FERRAMENTAS: Ferramenta[]` com `{ nome, oQueFaz, exemplo }`
  renderizado em `.map`, no mesmo arquivo.
- **D-MCPUI-2 — A fonte das descrições é `mcp.rs`.** O "O que faz" resume as `description` das
  `#[tool]`; não copia. Comentário no TSX apontando para `mcp.rs` como fonte, no padrão do comentário
  que já aponta para `manuais/README.md`.
- **D-MCPUI-3 — Uma frase-exemplo, dois lugares.** As frases do bloco (A) e da seção (B) são as
  mesmas, e as do manual "Como perguntar bem" permanecem. Comentário cruzado nos dois arquivos
  ("manter em sincronia com …"), como já se faz entre `MANUAIS` e `manuais/README.md`.
- **D-MCPUI-4 — Sem totais numéricos.** O bloco não cita quantidades nem quais UFs têm o quê (isso
  envelhece — ver a pendência dos totais mantidos à mão). Onde a cobertura importa, o texto aponta
  para `listar_entidades` ("disponível só onde `listar_entidades` o reporta").
- **D-MCPUI-5 — Estilo.** Fundo `bg.resumo`, borda pelo token `border` (era `brand.50`; ver §8),
  `borderRadius="10px"`,
  padding 16/18; tabela no padrão da tabela de manuais (`Table.Root size="sm"`, cabeçalho
  `bg.subtle`, caixa com borda e `overflowX="auto"`). Nomes de ferramenta em `<Text as="code">`. A
  coluna de exemplos em itálico com cor `fg` (não `fg.muted`: itálico + cinza cai abaixo de 4,5:1 em
  13px). O selo "Bloco novo" do mockup **não** entra.
- **D-MCPUI-6 — Sem o nome da unidade de atendimento.** Doutrina do repositório
  (`check-sem-navi.sh`): nenhuma menção ao órgão no código, comentários ou prosa; "o chat deste
  portal", "quem atende".
- **D-MCPUI-7 — Sem rota, sem estado novo.** O bloco é estático na tela `aberto === null`; não
  aparece no leitor de manual.

## 4. Conteúdo final

### 4.1 Bloco (A) — texto exato para o TSX

**Título (h2, mesmo estilo de "Manuais de instalação"):** O que dá para pedir

**Abertura:**

> Depois de conectado, você não chama ferramenta nenhuma: pergunta em português e o assistente
> escolhe qual usar. Mas ajuda saber o que existe por baixo — são cinco ferramentas, e a frase certa
> leva à ferramenta certa. Diga sempre **o estado**: quatro das cinco ferramentas exigem a UF, e sem
> ela o assistente tem de escolher uma por conta ou perguntar qual.

**Tabela** (colunas: Ferramenta · O que faz · Diga, por exemplo):

| Ferramenta | O que faz | Diga, por exemplo |
| --- | --- | --- |
| `listar_entidades` | Quais estados têm acervo e o que cada um tem (pareceres, TARF, legislação, serviços, FAQs), com os totais. Sem parâmetros. | *"Quais estados têm acervo no Auli, e o que cada um tem?"* |
| `buscar_pareceres` | Busca por sentido na jurisprudência de uma UF. Devolve número, assunto, sinopse e link — **sem o corpo**. Parâmetros: `uf`, `pergunta`, `top_k` (1 a 20, padrão 5) e `colecao`: `pareceres` (padrão) ou `tarf`. | *"Em Santa Catarina, o que já foi respondido sobre crédito de ICMS na energia elétrica?"* — ou, para os acórdãos: *"O que o TARF do RS já decidiu sobre…"*. Quer mais resultados? *"Traga os 15 mais próximos."* |
| `obter_parecer` | O texto integral de **um** documento, pelo número exato que a busca devolveu, na mesma coleção. O assistente só a usa se você pedir o texto completo. | *"Me mostre o texto completo do mais relevante."* — ou *"Abra a íntegra do segundo da lista."* |
| `consultar_servicos_faqs` | Para dúvidas de **como fazer**: devolve, num bloco só, até 5 serviços e 10 FAQs da UF mais próximos da pergunta — o mesmo contexto que o chat deste portal usa. | *"No Rio Grande do Sul, como faço para emitir a segunda via da guia do IPVA?"* |
| `buscar_legislacao` | Para o que a **lei** diz: pares pergunta/resposta sobre a norma, cada um com o dispositivo citado ("Art. 18-A, § 2º") e o link oficial. Disponível só onde `listar_entidades` o reporta. | *"O que a lei do RS exige de uma locadora para pagar a alíquota menor de IPVA?"* |

**Fechamento:**

> O encadeamento típico é `listar_entidades` → `buscar_pareceres` → `obter_parecer`: o assistente
> descobre o que existe, busca, e só então abre a íntegra do que você escolheu. Qual das três frentes
> responde decide-se pelo tipo de pergunta: a **legislação** diz o que a norma determina; os
> **pareceres** e acórdãos, como a Receita e o tribunal a interpretam num caso; os **serviços e
> FAQs**, como fazer a coisa no portal.

### 4.2 Seção (B) — markdown para o manual do Claude

Inserir após "## Teste se funcionou" e antes de "## Como perguntar bem":

```markdown
## As cinco ferramentas, e como pedi-las

Você não chama ferramenta nenhuma: pergunta em português e o Claude escolhe qual usar. Mas ajuda
saber o que existe por baixo — são cinco, e a frase certa leva à ferramenta certa. Diga sempre **o
estado**: quatro das cinco ferramentas exigem a UF, e sem ela o Claude tem de escolher uma por conta
ou perguntar qual.

[//]: # (manter em sincronia com auli-frontend/src/pages/mcplist/McpList.tsx — FERRAMENTAS)

| Ferramenta | O que faz | Diga, por exemplo |
|---|---|---|
| `listar_entidades` | Quais estados têm acervo e o que cada um tem, com os totais. Sem parâmetros. | *"Quais estados têm acervo no Auli, e o que cada um tem?"* |
| `buscar_pareceres` | Busca por sentido na jurisprudência de uma UF. Devolve número, assunto, sinopse e link — **sem o corpo**. Aceita `top_k` (1 a 20, padrão 5) e `colecao`: `pareceres` (padrão) ou `tarf`. | *"Em Santa Catarina, o que já foi respondido sobre crédito de ICMS na energia elétrica?"* — ou *"O que o TARF do RS já decidiu sobre…"*. Quer mais? *"Traga os 15 mais próximos."* |
| `obter_parecer` | O texto integral de **um** documento, pelo número exato que a busca devolveu, na mesma coleção. O Claude só a usa se você pedir o texto completo. | *"Me mostre o texto completo do mais relevante."* — ou *"Abra a íntegra do segundo da lista."* |
| `consultar_servicos_faqs` | Para dúvidas de **como fazer**: até 5 serviços e 10 FAQs da UF mais próximos da pergunta, num bloco só. | *"No Rio Grande do Sul, como faço para emitir a segunda via da guia do IPVA?"* |
| `buscar_legislacao` | Para o que a **lei** diz: pares pergunta/resposta com o dispositivo citado e o link oficial. Disponível só onde `listar_entidades` o reporta. | *"O que a lei do RS exige de uma locadora para pagar a alíquota menor de IPVA?"* |

O encadeamento típico é `listar_entidades` → `buscar_pareceres` → `obter_parecer`: o Claude descobre
o que existe, busca, e só então abre a íntegra do que você escolheu.
```

Observação: a tabela do manual é mais curta que a do TSX (omite a lista de parâmetros de
`buscar_pareceres` já explicada no prefácio do manual). As frases-exemplo são idênticas.

## 5. Passo a passo (portão humano a cada passo)

### Passo 1 — Bloco (A) em `McpList.tsx`

1. Interface `Ferramenta { nome: string; oQueFaz: ReactNode; exemplo: ReactNode }` e o array
   `FERRAMENTAS` com as cinco linhas de §4.1, logo abaixo de `MANUAIS`, com o comentário de fonte
   (D-MCPUI-2) e o de sincronia (D-MCPUI-3). `ReactNode` porque "O que faz" e "Diga, por exemplo"
   carregam `<Text as="code">` e `<Text as="strong">`.
2. No JSX da lista, novo `<Stack gap={2}>` entre o `<Stack>` da tabela de manuais e o de "Os dois
   Copilot", com a caixa de D-MCPUI-5 envolvendo: h2, parágrafo de abertura, tabela
   (`FERRAMENTAS.map`), parágrafo de fechamento.
3. Nada muda no `LeitorManual`, no estado `aberto` nem na tabela de manuais.

**Verificar:** `npm run lint`, `tsc --noEmit` e o teste do Passo 2 passando; visualmente, a tabela
rola na horizontal em 390px (mesmo comportamento da tabela de manuais).

### Passo 2 — Teste `McpList.test.tsx`

Novo arquivo em `auli-frontend/src/pages/mcplist/`, no padrão dos testes existentes
(`renderWithProvider` de `src/test/render.tsx`; `textFetcher` mockado para o leitor não ir à rede):

- renderiza `<McpList />` e espera o heading "O que dá para pedir";
- espera os cinco nomes de ferramenta, na ordem, na primeira coluna da tabela
  `aria-label="Ferramentas do servidor MCP"` — com escopo na tabela, porque três dos nomes também
  aparecem no texto do bloco (ver §8);
- espera **quatro** botões `Ler o manual do …` (a tabela de manuais não mudou);
- clica em "Ler o manual do Claude (web)" e espera que "O que dá para pedir" **não** esteja mais na
  tela (D-MCPUI-7).

**Verificar por mutação** (CLAUDE.md §4): remover uma linha de `FERRAMENTAS` e confirmar que o teste
falha; restaurar.

### Passo 3 — Seção (B) no manual

Inserir o markdown de §4.2 em `manuais/manual-auli-mcp-claude.md` no ponto indicado. Acrescentar ao
comentário do `FERRAMENTAS` no TSX a referência a esta seção, e no manual o comentário
`[//]: # (manter em sincronia com …McpList.tsx — FERRAMENTAS)` acima da tabela — **não** um
comentário HTML, que o leitor do portal exibe como texto (ver §8).

**Verificar:** markdownlint no arquivo; `scripts/tools/check-sem-navi.sh`.

### Passo 4 — Build e PR

- `npm run build` e `vitest run` no frontend; `scripts/tools/check-registry-sync.sh` (não deve ser
  afetado, mas roda no CI).
- Dois commits (um por entrega: `feat(mcp-tab): …` e `docs(manuais): …`) ou um só, a critério de
  Carlos.
- PR com título "Conectar sua IA: explicar as cinco ferramentas MCP" e o resumo desta seção no corpo.
  Não mergear sem confirmação.

## 6. Critérios de aceite

- [ ] A tela de lista mostra o bloco entre a tabela de manuais e "Os dois Copilot", com as cinco
      ferramentas e as frases de §4.1, byte a byte.
- [ ] O leitor de manual não mostra o bloco.
- [ ] O manual do Claude tem a seção de §4.2 no ponto indicado; as frases-exemplo são iguais às do
      TSX.
- [ ] Nenhum total numérico novo em prosa ou código (D-MCPUI-4).
- [ ] Nenhuma menção à unidade de atendimento (D-MCPUI-6).
- [ ] `lint`, `tsc`, `vitest`, `build`, `check-sem-navi` e markdownlint passam; teste novo falha sob
      mutação.
- [ ] Modo escuro: a caixa usa `bg.resumo` (que já tem `_dark`) e a borda o token `border`
      (decidido na revisão, §8).

## 7. Fora do escopo e pendências que ficam

- Espelhar a seção nos manuais do ChatGPT, GitHub Copilot e Copilot Studio (tarefa própria; o
  conteúdo de §4.2 serve de base trocando "o Claude" pelo assistente).
- Totais do acervo mantidos à mão nos manuais (pendência já aberta; este estudo não os toca nem os
  acrescenta).
- O `README.md` da raiz já descreve o MCP em linhas gerais; não é alterado.

## 8. Revisão antes da execução (05/11/2026)

As premissas foram conferidas contra o código antes de implementar. Confirmadas: as cinco
ferramentas e seus parâmetros; `top_k` 1–20, padrão 5 (`MAX_TOP_K`/`DEFAULT_TOP_K`); 5 serviços e
10 FAQs (`SERVICES`/`FAQS.n_results`); a paridade com o chat (`rag::blocos`); TARF e legislação só no
RS; `manuais/README.md` fora do frontend (`build-frontend-public.sh` copia só `manual-*.md`). O que
mudou em relação ao plano de 03/11, com o aval de Carlos:

- **A frase do estado estava errada.** "É o único parâmetro obrigatório em todas" — o
  `listar_entidades` não tem parâmetro, e `pergunta`/`numero` também são obrigatórios. Virou "quatro
  das cinco ferramentas exigem a UF, e sem ela o assistente tem de escolher uma por conta ou
  perguntar qual", no eco da dica que o manual já dava.
- **O exemplo "pelo número" falharia.** O `obter_parecer` casa o título inteiro
  (`documento_por_numero`, `trim().to_lowercase() ==`); o de SP é "RESPOSTA À CONSULTA TRIBUTÁRIA
  23333/2021", e a 1234/2024 nem existe (as de 2024 estão na casa dos 29 mil). Trocado por *"Abra a
  íntegra do segundo da lista."*, que usa o número que a busca devolveu.
- **Comentário HTML aparece no leitor.** O react-markdown v10 só com `remark-gfm` renderiza
  `<!-- … -->` como texto escapado (testado). O `[//]: # (…)` não renderiza nada (testado no
  pipeline do leitor) e o markdownlint o ignora (MD053 dispensa `//`).
- **O teste planejado quebraria.** `getByText("listar_entidades")` acha três nós (tabela, linha do
  `buscar_legislacao`, fechamento) e lança erro; com `getAllByText`, tirar a linha não falharia. O
  teste lê a primeira coluna da tabela. Mutação feita: sem a linha `listar_entidades`, falha.
- **Borda: token `border`.** `brand.50` é primitivo sem `_dark` (`system.js`) — no escuro seria um
  contorno `#dbeafe` sobre `#15253a`. O outro bloco em `bg.resumo` (o Resumo do chat) nem borda tem.
- **Posição: a do mockup**, entre a tabela de manuais e "Os dois Copilot" — reconfirmada por Carlos.
- **Padding explícito nas células** (`px={2} py={2}`, o valor do recipe `sm`). O reset
  `* { padding: 0 }` do `src/index.css` não tem camada e vence o recipe do Chakra, que vive em
  `@layer`; sem isso as colunas de texto corrido encostavam ("…coleção. Oou *Abra a…*"). A tabela de
  manuais tem o mesmo defeito ("Microsoft 365 CopilotUsuário…") e **não** foi tocada.
- **`minW="16rem"`** nas duas colunas de texto, para que em 390px a tabela role dentro da caixa
  (758 px de conteúdo em 316) em vez de espremer as colunas; a página não rola na horizontal.

Observados e deixados de fora por decisão de Carlos: as `instructions` do servidor (`get_info`) não
citam o `buscar_legislacao` e a `description` do `listar_entidades` omite a legislação; o manual
ainda diz que as FAQs são "só do Rio Grande do Sul".
