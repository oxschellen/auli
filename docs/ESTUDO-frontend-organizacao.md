# Auli frontend — relatório de organização do código

Nov 13, 2026 · @Carlos Schellenberger

## Resumo

O código do `auli-frontend` está saudável e não pede reescrita: os problemas são de localização de arquivos e de repetição entre as abas de lista. A base está verde (`tsc`, ESLint e 336 testes passam, sem ciclos de import), o que torna a reorganização segura de fazer em passos pequenos.

| Medida | Valor |
| --- | --- |
| Arquivos de produção | 93 arquivos, 9.028 linhas |
| Arquivos de teste | 38 arquivos, 3.823 linhas |
| Arquivos de fora que importam de dentro de `pages/chat` | 7 |
| Padrões de código repetidos em 3 ou mais arquivos | 8 |
| Maior arquivo | `Home.tsx`, 506 linhas |
| Dependências sem uso (`knip`) | 7 |
| Script quebrado | 1 (`npm run gen:entities`) |

Os achados caem em cinco grupos, do mais estrutural ao mais barato de corrigir:

- **A. Lugar errado.** Infraestrutura do app inteiro mora dentro de `pages/chat`, e `shared/` e `types/` importam de lá.
- **B. Redundância.** O esqueleto de busca aparece em 6 arquivos, o painel de acordeão em 5, o overlay feito à mão em 4, o acesso ao `localStorage` em 5.
- **C. Concentração.** `Home.tsx` e `Chat.tsx` reúnem responsabilidades que mudam por motivos diferentes.
- **D. Nomes e estilo.** Pastas e arquivos de lógica seguem cinco convenções, e há dois estilos de formatação.
- **E. Limpeza.** Um script quebrado, dependências e exports sem uso, configuração duplicada e documentação defasada.

A recomendação é executar em quatro etapas, cada uma com a suíte verde ao final: limpeza, mover sem alterar código, extrair as redundâncias uma por vez, e dividir os dois arquivos grandes.

## Revisão e decisões (13/11/2026)

O relatório foi conferido contra a `main` em `a57bb86` antes da etapa 1. Os achados de A, D6 e E batem com o código, e a árvore proposta é coerente: nenhum arquivo destinado à base importa de algo que vai para `app/` ou `secoes/`, e o único import entre seções é o do achado A2.

**Decisões do mantenedor sobre o que antecedia a etapa 2:**

| Decisão | Escolha |
| --- | --- |
| Escopo da etapa 2 | a árvore completa |
| Nomes das pastas | português (`secoes/`, `contexto/`, `mapa/`, `sobre/`) |
| Alias `@/` | **removido**. Os imports seguem relativos, e os esboços I3 e I9 que usam `@/` valem com caminho relativo: o teste de camadas só precisa tratar `./` e `../` |
| Prettier | adotado, num PR só de formatação antes da etapa 2, registrado no `.git-blame-ignore-revs` |

**Correções ao relatório:**

- O `deploy-frontend.sh` cita `src/shared/entities.ts` (linhas 224 e 247: um comentário e a mensagem de erro do gate de deploy), e o `config/registry.toml` também (linha 22). Os três mudam no commit 2b, junto com o gerador e o `check-registry-sync.sh`.
- O `config/registry.toml` citava o gerador no caminho antigo, `scripts/gen-frontend-entities.mjs` (linhas 12 e 22), o mesmo defeito do E1. Corrigido na etapa 1.
- A suíte tem 342 testes, todos passando e nenhum pulado, em cerca de 5 s.
- O `DESCRIPTION.md` tem 75 referências a caminhos de `src/`, não 67; o `docs/auli_code.md` tem 41.
- E6: o `.cursor/` saiu (decisão do mantenedor em 13/11/2026); o `.agents/` fica.

**A etapa 1 fez também, além do checklist:** `remark-gfm` e `swr` entraram no `vendor-utils` do `chunkMap` (E4), o que leva o pacote do app de 204 kB para 153 kB, sem mudar o total; a linha comentada do `vite.svg` saiu do `index.html`; as menções a `@emotion/styled`, ao `jsconfig.json` e ao alias saíram do `README.md`, do `DESCRIPTION.md` e do `docs/auli_code.md`. A `meta description` com o texto "Auli" ficou: trocá-la é decisão de produto, não de organização.

## Escopo e método

A análise cobre a pasta `auli-frontend/` do repositório `oxschellen/auli` no commit `a57bb86` (11/11/2026, v0.1.73). Não existe um repositório `auli-frontend` separado.

**O que foi lido.** Os arquivos de configuração, o `README.md`, a estrutura do `DESCRIPTION.md` e o código de produção em `src/`. Seis arquivos longos de marcação foram lidos em parte: `Historico.tsx`, `PdfModal.tsx`, `McpList.tsx` e as listas `DadosLista`, `AnalisesLista` e `LivrosLista` do Tributum (estas comparadas por `diff` com `ArtigosLista`). Os testes foram executados e examinados pela estrutura, não linha a linha.

**O que foi executado.** Tudo numa cópia limpa, depois de `npm ci`:

| Verificação | Resultado |
| --- | --- |
| `tsc --noEmit` | sem erros |
| `npm run lint` | sem avisos |
| `vitest run` | 336 passam, 6 pulados, 46 s |
| `madge --circular` | nenhum ciclo em 132 arquivos |
| `knip` | 7 dependências e 4 exports sem uso |
| `npm run gen:entities` | falha: módulo não encontrado |

As contagens de padrões repetidos vêm de `grep` sobre os arquivos que não são de teste, e o grafo de imports entre pastas vem de um script que resolve cada `import` relativo.

**Limites.** Nada foi alterado no repositório. Os esboços de código deste relatório não foram compilados contra o projeto, com uma exceção: o teste de camadas da seção de implementação foi rodado contra a árvore atual e acusou exatamente os imports listados no grupo A.

## Retrato atual

`src/` tem seis pastas de primeiro nível, e duas delas concentram quase tudo: `pages/` com 12 seções e `shared/` com 17 arquivos numa pasta plana.

| Pasta | Arquivos | Linhas de produção | Linhas de teste | O que guarda |
| --- | --- | --- | --- | --- |
| `src/` (raiz) | 4 | 166 | 317 | `main.tsx`, `App.tsx`, `index.css`, `vite-env.d.ts` |
| `shared/` | 17 | 1.560 | 870 | casca do app, entidade, busca, markdown, componentes genéricos |
| `pages/chat/` | 12 | 1.611 | 600 | componentes do chat |
| `pages/chat/utils/` | 15 | 740 | 560 | hooks e funções puras do chat |
| `pages/chat/ui/` | 4 | 136 | 0 | provider, toaster, tooltip e modo de cor do Chakra |
| `pages/home/` | 1 | 506 | 138 | sidebar, drawer e painéis das seções |
| `pages/faqslist/` | 3 | 530 | 191 | aba FAQs |
| `pages/servicoslist/` | 3 | 427 | 172 | aba Serviços |
| `pages/legislacaolist/` | 2 | 417 | 124 | aba Legislação |
| `pages/parecereslist/` | 5 | 399 | 140 | abas Pareceres e Acórdãos TARF |
| `pages/mcplist/` | 1 | 359 | 40 | aba Conectar sua IA |
| `pages/downloadslist/` | 2 | 305 | 135 | aba Downloads |
| `pages/conteudoslist/` | 3 | 288 | 66 | aba Conteúdos |
| `pages/stateselection/` | 3 | 231 | 0 | mapa do Brasil |
| `pages/about/` | 1 | 50 | 0 | aba Sobre |
| `pages/notaslist/` | 1 | 37 | 0 | aba Notas |
| `tributum/` | 13 | 1.021 | 470 | as seis abas do Tributum |
| `theme/` | 1 | 150 | 0 | tokens de cor |
| `types/` | 1 | 40 | 0 | tipos do chat |
| `test/` | 2 | 55 | 0 | setup do Vitest |

O fluxo geral de dependências está certo: as seções fazem 77 imports de `shared/` e o Tributum faz 12. Os testes ficam ao lado do código que testam (47 dos 70 imports relativos dos testes apontam para a própria pasta), o que barateia qualquer movimentação. As exceções a esse fluxo são o grupo A.

## Achados A: arquivos no lugar errado

Sete arquivos de fora importam de dentro de `pages/chat`, e isso faz `shared/` e `types/` dependerem de uma seção. Não há ciclo entre arquivos, mas a direção está invertida: quem deveria ser base depende de quem deveria ser folha.

&#91;embedded content: imports invertidos · 6 origens, 2 destinos\]

As quatro primeiras setas somem quando `ui/` sai do chat. As duas últimas somem quando a cópia de texto e o tipo `QuestionType` ganham endereço próprio.

| Arquivo de fora | O que importa | De onde |
| --- | --- | --- |
| `App.tsx:4-5` | `Provider`, `Toaster` | `pages/chat/ui/` |
| `shared/AppHeader.tsx:4-5` | `ColorModeButton`, `Tooltip` | `pages/chat/ui/` |
| `shared/AbasDeEstado.tsx:5` | `Tooltip` | `pages/chat/ui/` |
| `pages/home/Home.tsx:47` | `Tooltip` | `pages/chat/ui/` |
| `test/render.tsx:3` | `Provider` | `pages/chat/ui/` |
| `pages/faqslist/FaqsAccordion.tsx:11` | `utilsCopyTextToClipboard` | `pages/chat/utils/utils.ts` |
| `types/chat.ts:2` | `QuestionType` | `pages/chat/utils/useQuestionType.ts` |

### A1. `pages/chat/ui/` é infraestrutura do app inteiro

Os quatro arquivos (`provider`, `toaster`, `tooltip`, `color-mode`) são os snippets do Chakra que todo o app usa, e só três dos oito arquivos que os importam são do chat. São também os únicos `.jsx` do projeto, sem tipos, e três deles começam com `'use client'`, diretiva do Next.js que não tem efeito no Vite.

### A2. A cópia para a área de transferência mora no chat

`pages/chat/utils/utils.ts` exporta `utilsCopyTextToClipboard`, usada por três componentes do chat e pelo `FaqsAccordion`. O nome do arquivo (`utils/utils.ts`) e o prefixo da função não dizem o que ela faz.

### A3. Tipos e lógica pura dependem de um arquivo de hook

`types/chat.ts` importa `QuestionType` de `useQuestionType.ts`. A pasta `types/` tem esse arquivo só, e os sete importadores dele são do chat. O mesmo acontece dentro do chat: `utils/historico.ts`, que é lógica pura, importa `isQuestionType` do arquivo do hook.

### A4. `tributum/` está fora de `pages/`, e `pages/` não tem páginas

O Tributum é uma seção com seis abas registradas no `Home.tsx`, igual às outras, mas fica em `src/tributum/`. E o app não tem roteador: o que está em `pages/` são seções de uma página só.

### A5. `shared/` mistura quatro assuntos numa pasta plana

| Assunto | Arquivos |
| --- | --- |
| Casca do app | `AppHeader`, `AbasDeEstado`, `abas`, `RespostaDaAba`, `ErrorBoundary`, `layout` |
| Entidade e dados | `entities` (gerado), `EntityContext`, `fetchers` |
| Busca e texto | `textSearch`, `highlight`, `rehypeHighlight`, `markdown`, `linkify` |
| Componentes genéricos | `AsyncContent`, `SearchInput`, `CollectionEmpty` |

Com os testes, são 28 arquivos lado a lado. `AppHeader` e `AbasDeEstado` só são usados pelo `App.tsx`, então não são compartilhados.

### A6. `parecereslist/` guarda duas coleções com o nome de uma

A pasta contém `JurisprudenciaList` (o componente genérico), `PareceresList` e `TarfList`. O nome certo já está no código: jurisprudência.

### A7. O arquivo gerado tem o caminho fixado fora do frontend

`shared/entities.ts` é gerado a partir de `config/registry.toml`, e o caminho dele aparece em `scripts/tools/gen-frontend-entities.mjs` e em `scripts/tools/check-registry-sync.sh`. Movê-lo exige alterar os dois no mesmo commit, senão o CI de sincronia do registry quebra.

## Achados B: redundâncias

Oito padrões aparecem copiados em três ou mais arquivos. O Tributum já resolveu o primeiro deles para as suas cinco listas com o `TributumShell`; o Acervo não tem equivalente.

| Padrão | Arquivos | Linhas aproximadas por cópia |
| --- | --- | --- |
| B1. Esqueleto de busca | 6 | 35 a 45 |
| B2. Painel e cabeçalho de acordeão | 5 | 25 a 30 |
| B3. Documento `.md` renderizado como prosa | 3 | 20 a 40 |
| B4. Estilo do `<mark>` | 3 | 6 |
| B5. Link externo | 8 | 8 a 12 |
| B6. Overlay feito à mão | 4 | 15 a 25 |
| B7. Acesso ao `localStorage` | 5 | 15 a 40 |
| B8. Acesso HTTP | 3 | não se aplica |

### B1. Esqueleto de busca

Seis arquivos repetem a mesma sequência: estado do texto, `useDeferredValue`, `ref` do campo, função `clearSearch`, barra fixa com `SearchInput`, contador e a mensagem "Nenhum resultado encontrado".

| Arquivo | `useDeferredValue` | `clearSearch` | "Nenhum resultado" |
| --- | --- | --- | --- |
| `pages/faqslist/FaqsList.tsx` | linha 35 | linha 46 | `FaqsAccordion.tsx:250` |
| `pages/servicoslist/ServicosList.tsx` | linha 43 | linha 82 | linha 187 |
| `pages/parecereslist/JurisprudenciaList.tsx` | linha 57 | linha 76 | linha 125 |
| `pages/legislacaolist/LegislacaoList.tsx` | linha 243 | linha 259 | linha 306 |
| `pages/conteudoslist/ConteudosList.tsx` | linha 24 | linha 27 | `ConteudosAccordion.tsx:184` |
| `tributum/TributumShell.tsx` | linha 50 | linha 70 | linha 122 |

A barra fixa (`position="sticky"`, `zIndex={10}`, mesmo padding e mesma borda) é idêntica em cinco dos seis. A exceção é `ConteudosList`, onde a busca não é fixa e não há contador, o que parece desvio e não decisão.

### B2. Painel e cabeçalho de acordeão

O painel animado (`m.div` de `height: 0` a `auto`, com `overflow: hidden` e `contain: layout paint`) está em cinco arquivos, e em quatro deles o comentário que explica o `contain` foi copiado palavra por palavra.

- `pages/servicoslist/ServicosAccordion.tsx:60` (duração 0,25 s)
- `pages/faqslist/FaqsAccordion.tsx:151` (duração 0,25 s)
- `pages/parecereslist/PareceresAccordion.tsx:75` (duração 0,22 s)
- `pages/legislacaolist/LegislacaoList.tsx:109` (duração 0,22 s)
- `pages/conteudoslist/ConteudosAccordion.tsx:141` (duração 0,22 s)

O cabeçalho clicável (`role="button"`, `tabIndex`, e o `onKeyDown` que trata Enter e Espaço) aparece nove vezes em seis arquivos: os cinco acima, duas vezes em `FaqsAccordion`, `LegislacaoList` e `ConteudosAccordion`, e mais o `BrazilMap.tsx:128`. O `doctor.config.json` registra a dispensa de uma regra para "15 ocorrências do mesmo padrão", que é este.

### B3. Documento `.md` renderizado como prosa

Três lugares buscam um `.md` de `public/` e o renderizam com `proseMarkdownComponents` dentro da mesma caixa: `pages/about/About.tsx`, `tributum/CriteriosTributum.tsx` e o `LeitorManual` em `pages/mcplist/McpList.tsx:150`. O `About.tsx` ainda monta à mão o spinner e o alerta de erro que o `AsyncContent` substituiu, e o comentário do `CriteriosTributum` reconhece isso.

### B4. Estilo do `<mark>`

O realce da busca é definido em `shared/highlight.tsx:107` e repetido como override de `markdownComponents` em `PareceresAccordion.tsx:16` e `LegislacaoList.tsx:31`. Os dois overrides são idênticos.

### B5. Link externo

`tributum/LinkExterno.tsx` existe e diz, no próprio comentário, que copia o "Abrir no portal" da Legislação. Mas `LegislacaoList.tsx:132` e `PareceresAccordion.tsx:97` continuam com a versão inline. Ao todo, oito arquivos escrevem `target="_blank" rel="noopener noreferrer"` à mão, e `window.open(url, '_blank', 'noopener,noreferrer')` aparece quatro vezes em `FaqsAccordion` e `ConteudosAccordion`.

### B6. Overlay feito à mão

Quatro sobreposições usam o mesmo esqueleto (`Box` fixo com `inset={0}`, fundo escuro que fecha ao clicar, painel por cima), cada uma com o seu ouvinte de Esc:

| Overlay | Local | `zIndex` | Devolve o foco ao fechar |
| --- | --- | --- | --- |
| Drawer de seções | `pages/home/Home.tsx:453` | 110 | sim |
| Drawer "Minhas perguntas" | `pages/chat/Chat.tsx:208` | 110 | sim |
| Log da pergunta | `pages/chat/LogModal.tsx:59` | 100 | não |
| Leitor de PDF | `tributum/PdfModal.tsx:38` | 100 | não |

Nenhum dos quatro prende o foco dentro do painel, embora os dois modais declarem `aria-modal`. Os valores de `zIndex` (1, 10, 20, 21, 30, 100, 110) estão espalhados por 12 arquivos, e a relação entre eles só existe em comentários. Os dois modais usam 100, o mesmo valor do `AppHeader`.

### B7. Acesso ao `localStorage`

Cinco implementações leem e gravam com `try/catch` próprio, para seis chaves:

| Chave | Arquivo | Mecanismo |
| --- | --- | --- |
| `auli.sidebar` | `App.tsx:20-52` | `useState` e `useEffect` |
| `auli.abas`, `auli.entity` (legada) | `shared/abas.ts:26-105` | `useState` e `useEffect` |
| `auli.questionType` | `pages/chat/utils/useQuestionType.ts:14-82` | `useState` e gravação no setter |
| `auli.historicoPainel` | `pages/chat/utils/usePainelHistorico.ts` | store externo com `useSyncExternalStore` |
| `auli.historico.<id>` | `pages/chat/utils/useHistorico.ts` | store externo com `useSyncExternalStore` e cache |

Os dois últimos reconstroem o mesmo store (conjunto de ouvintes, leitura, gravação, assinatura). O comentário do `usePainelHistorico` diz que segue "a mesma regra da barra lateral esquerda", mas a barra esquerda usa outro mecanismo e não sincroniza entre abas do navegador.

### B8. Acesso HTTP

Há três pontos de saída para a rede, com dois clientes: `axios.get` em `shared/fetchers.ts:28-31`, `axios.post` em `pages/chat/utils/callServerAPI.ts:83` e `fetch` nativo em `pages/chat/LogModal.tsx:36`. Além disso, `callServerAPI` recebe `API_URL` por parâmetro (linha 12), embora `utils/api.ts` já a exporte, e recebe quatro setters de estado do `Chat`, o que mistura transporte com estado da conversa.

### B9. Repetições menores

- A regex de diacríticos está em `shared/textSearch.ts:31` (exportada como `DIACRITICOS`) e de novo em `pages/chat/utils/citacoes.ts:23`.
- `"1px solid var(--chakra-colors-border)"` aparece 37 vezes em 19 arquivos. O código mais novo usa `border="1px solid" borderColor="border"`, que passa pelo token.
- A pilha de fontes do corpo está em `index.css:24`, em `theme/system.js:38` e em `index.css:96`, esta última com uma pilha diferente (sem "SF Pro Text") e numa regra morta: a classe .chakra-ui-provider não existe no Chakra instalado. O `AppHeader.tsx:93` declara uma quarta, inline.
- `index.css` declara `html { }` duas vezes (linhas 17 e 44).
- As classes `markdown-body` (4 usos) e `faq-answer` (1 uso) não têm regra de CSS nem são consultadas pelos testes.
- O gate "a entidade tem a coleção?" (`hasCollection` seguido de `CollectionEmpty`) é repetido em seis listas, embora o `Home.tsx` já desative a aba pela mesma condição.

## Achados C: arquivos que concentram demais

Dois arquivos reúnem partes que mudam por motivos diferentes, e são justamente os dois que toda feature nova de navegação ou de conversa precisa editar.

### C1. `pages/home/Home.tsx` (506 linhas)

| Linhas | Conteúdo | Muda quando |
| --- | --- | --- |
| 50 a 93 e 112 a 119 | `TABS` e `GRUPOS`: o registro das 17 seções, com rótulo, ícone, componente, coleção e grupo | entra ou sai uma seção |
| 97 a 110 | `PISTA_DE_ROLAGEM`: o estilo da sombra de rolagem | muda o visual da sidebar |
| 129 a 143 | `proximaHabilitada`: navegação por setas, função pura e testada | raramente |
| 151 a 309 | `NavItem` e `NavList`: a lista de navegação, usada na sidebar e no drawer | muda a navegação |
| 311 a 506 | `Home`: estado da seção ativa, drawer, painéis montados sob demanda | muda o layout |

O registro das seções é dado, mas está no meio de um componente e importa as 17 seções de forma estática. O `Home` adia a montagem de cada painel até o primeiro clique, mas não o download do código: tudo entra no pacote inicial.

### C2. `pages/chat/Chat.tsx` (360 linhas)

O componente tem quatro `useEffect`, quatro `useState` e reúne cinco responsabilidades:

- o envio da pergunta e o registro no histórico (`handleCallServerAPI`, linha 145);
- a rolagem até a resposta que chega (linha 106);
- o painel "Minhas perguntas" em duas formas, coluna e drawer (linhas 178 a 228);
- a barra de composição fixa, com a medição de altura e o recuo do teclado virtual (linhas 94 a 104 e 230 a 356);
- o estado "recolhido" da caixa no celular.

### C3. Hooks finos demais e uma pasta `utils/` com três naturezas

`usePrompt.ts` (12 linhas) e `useMessages.ts` (11 linhas) só embrulham um `useState`. E `pages/chat/utils/` mistura, em 15 arquivos, hooks (`use*.ts`), funções puras (`citacoes`, `fontes`, `historico`, `prompt`, `secoes`), texto editorial (`tipos.ts`) e transporte (`api.ts`, `callServerAPI.ts`).

### C4. O campo `showButton` carrega três significados

Em `Message`, `showButton` decide se a mensagem tem botões, se é desenhada como cartão largo ou balão, e se a rolagem vai ao começo dela. As mensagens de erro são criadas com `showButton: true` e por isso também ocupam a coluna inteira, como o comentário em `SystemMessage.tsx` registra. São 13 usos.

## Achados D: nomes e estilo

Não há uma convenção única para nomear pastas de seção e arquivos de lógica, e o projeto tem dois estilos de formatação sem ferramenta que escolha entre eles.

### D1. Pastas de seção

As pastas usam minúsculas coladas com o sufixo `list` (`servicoslist`, `faqslist`, `parecereslist`, `legislacaolist`, `conteudoslist`, `notaslist`, `downloadslist`, `mcplist`), ao lado de `about`, `home`, `chat`, `stateselection` e de `tributum` fora de `pages/`. O sufixo não acrescenta informação e não vale para todas.

### D2. O arquivo de lógica pura de cada seção

Cada lista tem um arquivo com os tipos e as funções puras, e cada um segue um nome:

| Seção | Arquivo de lógica |
| --- | --- |
| Serviços | `utils.ts` |
| FAQs | `parseFaqs.ts` |
| Conteúdos | `parseConteudos.ts` |
| Pareceres | `pareceres.ts` |
| Legislação | `legislacao.ts` |
| Downloads | `downloads.ts` |
| Tributum | `types.ts` |
| Chat | `utils/utils.ts` e mais 14 arquivos |

O padrão mais claro é o das três mais novas: o nome da coleção (`legislacao.ts`).

### D3. Nome do arquivo diferente do que ele exporta

- `ServicosAccordion.tsx` exporta `AccordionItem`.
- `utils/utils.ts` exporta `utilsCopyTextToClipboard`.
- `shared/linkify.tsx` exporta `editLinks`.
- `useQuestionType.ts` exporta, além do hook, o tipo `QuestionType`, `tiposDisponiveis` e `isQuestionType`.

### D4. Português e inglês nos identificadores

O código antigo usa inglês (`searchQuery`, `clearSearch`, `isSearching`, `handleKeyDown`) e o novo usa português (`abas`, `aoTeclar`, `recolhida`, `registrar`). Os dois convivem no mesmo arquivo, como em `JurisprudenciaList` e `Chat`. Não é um defeito a corrigir em massa, mas falta a regra para o código novo.

### D5. Dois estilos de formatação

Nove arquivos usam aspas simples sem ponto e vírgula: os quatro `.jsx` de `chat/ui/`, e `FaqsList`, `FaqsAccordion`, `ConteudosList`, `ConteudosAccordion` e `ServicosAccordion`. Os outros 84 usam aspas duplas com ponto e vírgula. Não há Prettier nem `.editorconfig`, e o ESLint não tem regra de estilo.

### D6. Comentários que já não descrevem o código

- `pages/servicoslist/utils.ts:100` manda ver `getTipoServicos`, que não existe.
- `pages/stateselection/BrazilMap.tsx:12` diz que os estados disponíveis são "RS, SC"; hoje são 27. O bloco de documentação do componente também está acima de `type Point`, e não da função.
- `tributum/PlaceholderBadge.tsx:8` afirma que o `Badge` do Chakra não é usado no frontend, mas `ConteudosAccordion.tsx` o usa.
- `shared/entities.ts` diz que a entidade é "persistida no localStorage" pela landing; desde as abas de estado quem persiste é `abas.ts`.

## Achados E: limpeza de risco zero

Nenhum item deste grupo muda comportamento do app, e o primeiro é um defeito de fato.

### E1. `npm run gen:entities` está quebrado

O `package.json:15` chama `node ../scripts/gen-frontend-entities.mjs`, mas o arquivo está em `scripts/tools/gen-frontend-entities.mjs`. O comando falha com módulo não encontrado, e `npm run gen` falha junto, porque o encadeia. O cabeçalho do próprio `entities.ts` cita o caminho certo.

### E2. Dependências sem uso

| Pacote | Tipo | Observação |
| --- | --- | --- |
| `@emotion/styled` | dependência | nenhum import em `src/`; aparece só no `chunkMap` do `vite.config.js` |
| `@types/babel__core` | dev | sem uso |
| `@types/babel__generator` | dev | sem uso |
| `@types/babel__template` | dev | sem uso |
| `@types/babel__traverse` | dev | sem uso |
| `@types/estree` | dev | sem uso |
| `@types/json-schema` | dev | sem uso |

A lista vem do `knip`. Depois de remover, vale rodar `npm run build` e `npm test`: o `@emotion/styled` não aparece no package.json do Chakra instalado, então a remoção deve ser segura, e o build confirma.

### E3. Exports sem uso

- `DEFAULT_ENTITY_ID` em `shared/entities.ts:218`. Como o arquivo é gerado, a remoção é no gerador.
- `ETAPAS` em `pages/chat/Pensando.tsx:18`: só é usado dentro do próprio arquivo, basta tirar o `export`.
- O tipo `Aba` em `shared/abas.ts:15` e o tipo `MessageSender` em `types/chat.ts:5`.

### E4. Configuração duplicada ou sem efeito

- O alias `@/` está configurado em `tsconfig.json`, `jsconfig.json` e `vite.config.js`, e não é usado em nenhum import.
- O `jsconfig.json` é redundante: o `tsconfig.json` na mesma pasta já cobre os `.js` com `allowJs`.
- O `chunkMap` do `vite.config.js` separa `vendor-utils` com `axios`, `framer-motion` e `react-markdown`, mas não inclui `swr` nem `remark-gfm`, que por isso não ficam em nenhum pacote de fornecedor.
- `index.html` tem `<meta name="description" content="Auli">` e o mesmo texto em `og:description`, e uma linha comentada do `vite.svg`.
- A variável `--transition-fast` do `index.css` é usada uma vez, no próprio arquivo.

### E5. Documentação defasada

| Arquivo | O que diz | O que o código faz |
| --- | --- | --- |
| `README.md:7` | há RS e SC | 27 entidades |
| `README.md:13` | sete abas | 17 seções em 4 grupos, numa sidebar |
| `README.md:18` | timeout de 25 segundos | 70 segundos (`callServerAPI.ts`) |
| `README.md` | cita Pareceres Jurídicos e Notas Tributárias | não cita TARF, Legislação, Tributum, Downloads nem MCP |
| `DESCRIPTION.md`, layout de diretórios | 9 pastas em `pages/`, sem `tributum/` | 12 pastas em `pages/`, mais `tributum/` |
| `DESCRIPTION.md` | 67 referências a caminhos de `src/` | todas mudam na reorganização |

O `README.md` e o `DESCRIPTION.md` descrevem a mesma coisa em dois níveis de detalhe, e os dois envelheceram. O `DESIGN.md` (583 linhas) tem 275 linhas de front matter YAML de uma análise de design da Apple antes do primeiro título.

### E6. Restos de ferramentas

A pasta tem `.cursor/` (hooks do react-doctor) e `.agents/skills/react-doctor/`. Se o Cursor não é mais usado no projeto, são duas pastas a menos; se é, ficam.

## Estrutura proposta

A proposta troca as duas pastas grandes (`pages/` e `shared/`) por três camadas com direção única de dependência: a raiz de composição, as seções e a base.

```text
src/
├─ main.tsx  index.css  vite-env.d.ts
│
├─ app/              RAIZ DE COMPOSIÇÃO: monta o app, importa de tudo
│  ├─ App.tsx
│  ├─ AppHeader.tsx
│  ├─ AbasDeEstado.tsx
│  ├─ abas.ts
│  └─ ErrorBoundary.tsx
│
├─ secoes/           SEÇÕES: importam da base, nunca umas das outras
│  ├─ registro.ts    as 17 seções como dado (hoje dentro do Home.tsx)
│  ├─ home/          Home.tsx, Navegacao.tsx
│  ├─ mapa/          StateSelection, BrazilMap, brazilPaths
│  ├─ chat/          componentes · hooks/ · lib/ · tipos.ts
│  ├─ servicos/      ServicosList, ServicosAccordion, servicos.ts
│  ├─ faqs/          FaqsList, FaqsAccordion, faqs.ts
│  ├─ jurisprudencia/  JurisprudenciaList, PareceresList, TarfList, ...
│  ├─ legislacao/
│  ├─ conteudos/
│  ├─ notas/
│  ├─ tributum/
│  ├─ downloads/
│  ├─ mcp/
│  └─ sobre/
│
├─ ui/               BASE: componentes genéricos
│  ├─ provider.tsx  toaster.tsx  tooltip.tsx  color-mode.tsx
│  ├─ AsyncContent.tsx  SearchInput.tsx  CollectionEmpty.tsx
│  ├─ LinkExterno.tsx  copiar.ts  layout.ts
│  └─ (novos) busca.tsx  Acordeao.tsx  Sobreposicao.tsx  DocumentoMarkdown.tsx
├─ lib/              BASE: busca, texto e armazenamento
│  ├─ textSearch.ts  highlight.tsx  rehypeHighlight.ts
│  ├─ markdown.tsx  linkify.tsx
│  └─ (novo) storage.ts
├─ api/              BASE: toda saída para a rede
│  ├─ estaticos.ts   (hoje shared/fetchers.ts)
│  └─ chat.ts        pergunta e log
├─ contexto/         BASE: o que a aba de estado entrega a quem está dentro dela
│  ├─ entities.ts    (gerado)
│  ├─ EntityContext.tsx
│  └─ RespostaDaAba.tsx
├─ theme/
└─ test/
```

### As três regras

1. **A base não conhece quem a usa.** `ui/`, `lib/`, `api/`, `contexto/` e `theme/` nunca importam de `secoes/` nem de `app/`.
2. **Uma seção não importa de outra.** O que duas seções precisam sobe para a base. As exceções são `secoes/registro.ts` e `secoes/home/`, que existem para compor as demais.
3. **Só `app/` monta o todo.** É o único lugar que importa o `Home`, o mapa e os providers ao mesmo tempo.

As regras são verificadas por um teste, descrito na seção de implementação, para que a estrutura não se desfaça com o tempo.

### Decisões embutidas na árvore

- **`layout.ts` vai para `ui/`, e `RespostaDaAba` e `EntityContext` vão para `contexto/`**, e não para `app/`. O `Chat` e o `Home` importam esses arquivos, e se eles ficassem em `app/` as seções dependeriam da raiz, que por sua vez depende delas.
- **`copiar.ts` vai para `ui/`, e não para `lib/`**, porque dispara um toast e portanto depende do `toaster`.
- **Dentro do chat, três subpastas por natureza:** `hooks/` para os `use*.ts`, `lib/` para as funções puras e o texto editorial, e os componentes na raiz da seção. O tipo `QuestionType` sai do arquivo do hook e vai para `chat/lib/tipoDeConsulta.ts`, com `isQuestionType` e `tiposDisponiveis`, o que desfaz o achado A3.
- **Arquivo de lógica com o nome da coleção:** `servicos.ts`, `faqs.ts`, `conteudos.ts`, no padrão que `legislacao.ts` e `downloads.ts` já seguem.
- **Testes continuam ao lado do código.** Nenhuma pasta `__tests__`.

Em relação ao resumo que enviei antes no chat, há dois ajustes: a pasta `entidades/` virou `contexto/`, porque passou a abrigar também o `RespostaDaAba`, e a cópia para a área de transferência foi de `lib/` para `ui/`.

## Sugestões de implementação

Cada sugestão abaixo resolve um achado, mostra o esboço do código e diz o que muda nos testes. Os esboços são ponto de partida: não foram compilados contra o projeto, salvo o teste de camadas (I9).

| Sugestão | Resolve | Arquivos novos | Testes afetados |
| --- | --- | --- | --- |
| I1. Armazenamento único | B7 | `lib/storage.ts` | nenhum, se as chaves forem mantidas |
| I2. Saída única para a rede | B8 | `api/chat.ts` | nenhum nesta etapa |
| I3. Busca compartilhada | B1 | `ui/busca.tsx` | nenhum |
| I4. Acordeão compartilhado | B2 | `ui/Acordeao.tsx` | nenhum |
| I5. Documento markdown, realce e link | B3, B4, B5 | `ui/DocumentoMarkdown.tsx` | nenhum |
| I6. Sobreposição e escala de `zIndex` | B6 | `ui/Sobreposicao.tsx` | nenhum |
| I7. Registro de seções e divisão do `Home` | C1 | `secoes/registro.ts`, `home/Navegacao.tsx` | `Home.test.ts` (um import) |
| I8. Divisão do `Chat` | C2, C3, C4 | `useConversa.ts`, `Compositor.tsx`, `PainelHistorico.tsx` | `callServerAPI.test.ts`, `Chat.test.tsx` |
| I9. Teste de camadas | A1 a A6 | `test/camadas.test.ts` | é o próprio teste |

### I1. Armazenamento único (`lib/storage.ts`)

Um módulo passa a ser a única porta para o `localStorage`: as chaves ficam juntas, o `try/catch` existe uma vez, e todo valor gravado é reativo.

```ts
import { useSyncExternalStore } from "react";

/** Todas as chaves do app. Os valores NÃO mudam: é o que já está gravado nos navegadores. */
export const CHAVES = {
  abas: "auli.abas",
  entidadeLegada: "auli.entity",
  sidebar: "auli.sidebar",
  painelHistorico: "auli.historicoPainel",
  tipoDeConsulta: "auli.questionType",
  historico: (entityId: string) => `auli.historico.${entityId}`,
} as const;

const ouvintes = new Set<() => void>();
/** O que não coube no localStorage (bloqueado ou cheio): vale só nesta sessão. */
const soNaMemoria = new Map<string, string>();

export function lerTexto(chave: string): string | null {
  const local = soNaMemoria.get(chave);
  if (local !== undefined) return local;
  try {
    return localStorage.getItem(chave);
  } catch {
    return null;
  }
}

export function gravarTexto(chave: string, valor: string): void {
  try {
    localStorage.setItem(chave, valor);
    soNaMemoria.delete(chave);
  } catch {
    soNaMemoria.set(chave, valor);
  }
  ouvintes.forEach((avisar) => avisar());
}

function assinar(avisar: () => void) {
  ouvintes.add(avisar);
  window.addEventListener("storage", avisar); // gravações das OUTRAS abas do navegador
  return () => {
    ouvintes.delete(avisar);
    window.removeEventListener("storage", avisar);
  };
}

/** O texto gravado em `chave`, reativo. É string, então a referência é estável entre renders. */
export function useTextoGravado(chave: string): string | null {
  return useSyncExternalStore(assinar, () => lerTexto(chave));
}
```

Com isso, os consumidores encolhem. O `usePainelHistorico` vai de 45 para 6 linhas, e o `useHistorico` perde o cache manual, porque o `useMemo` sobre a string faz o mesmo papel:

```ts
export function usePainelHistorico() {
  const aberto = useTextoGravado(CHAVES.painelHistorico) !== "recolhido";
  const definir = (v: boolean) => gravarTexto(CHAVES.painelHistorico, v ? "aberto" : "recolhido");
  return [aberto, definir] as const;
}

export function useHistorico(entityId: string) {
  const chave = CHAVES.historico(entityId);
  const bruto = useTextoGravado(chave);
  const itens = useMemo(() => historico.desserializar(bruto), [bruto]);
  // Toda gravação relê antes de alterar, como hoje: duas abas não se sobrescrevem.
  const alterar = (f: (atual: ItemHistorico[]) => ItemHistorico[]) =>
    gravarTexto(chave, historico.serializar(f(historico.desserializar(lerTexto(chave)))));
  return {
    itens,
    registrar: (pergunta: string, tipo: QuestionType) =>
      alterar((a) => historico.registrar(a, pergunta, tipo, Date.now(), crypto.randomUUID())),
    alternarFavorita: (id: string) => alterar((a) => historico.alternarFavorita(a, id)),
    remover: (id: string) => alterar((a) => historico.remover(a, id)),
    limparRecentes: () => alterar(historico.limparRecentes),
  };
}
```

A sidebar do `App.tsx` troca o par `useState` e `useEffect` por `useTextoGravado(CHAVES.sidebar) !== "recolhida"`. O `abas.ts` e o `useQuestionType` mantêm a própria lógica e só trocam as chamadas diretas por `lerTexto` e `gravarTexto`.

**O que muda de comportamento.** A sidebar passa a sincronizar entre abas do navegador, como o painel direito já faz. Nada mais.

**Atenção nos testes.** O módulo guarda estado (`soNaMemoria`), como o `useHistorico` e o `usePainelHistorico` já guardam hoje. O teste que simula o `localStorage` bloqueado deixa valor na memória, então convém exportar uma função de limpeza só para os testes ou usar `vi.resetModules()` naquele arquivo.

**Por que vem primeiro.** O comentário do `useHistorico` já prevê que, com login, a lista sai do navegador e vai para o servidor. Com um módulo só, essa troca acontece num arquivo.

### I2. Saída única para a rede (`api/`)

`api/estaticos.ts` é o `fetchers.ts` de hoje, sem alteração. `api/chat.ts` reúne o endereço, a pergunta e o log, e devolve um resultado em vez de mexer em estado:

```ts
import axios from "axios";

/** Contrato do servidor: mora aqui e é exportado para o chat (hoje está em types/chat.ts). */
export interface Fonte {
  rotulo: string;
  titulo: string;
  link: string;
  triagem?: "aplica" | "condicional" | "descarta";
}

const URL_PERGUNTA = import.meta.env.VITE_API_URL ?? "https://api.auli.com.br/v1/question";
/** Derivada da URL da pergunta, como hoje: as duas não podem apontar para servidores diferentes. */
const urlDoLog = (id: string) => `${URL_PERGUNTA.replace(/\/question$/, "/log")}/${id}`;

const LIMITE_MS = 70_000;

export type Resultado =
  | { ok: true; texto?: string; logId?: string; fontes?: Fonte[] }
  | { ok: false; falha: "tempo" | "limite" | "indisponivel"; mensagem?: string };

export async function perguntar(pergunta: string, entityId: string, tipo: number): Promise<Resultado> {
  const controle = new AbortController();
  const relogio = setTimeout(() => controle.abort(), LIMITE_MS);
  try {
    const { data } = await axios.post<{ answer?: string; log_id?: string; fontes?: Fonte[] }>(
      URL_PERGUNTA,
      { question: pergunta, entity: entityId, type: tipo },
      { signal: controle.signal },
    );
    return { ok: true, texto: data?.answer, logId: data?.log_id, fontes: data?.fontes };
  } catch (e) {
    if (axios.isCancel(e)) return { ok: false, falha: "tempo" };
    const r = (e as { response?: { status?: number; data?: { error?: string } } }).response;
    if (r?.status === 429) return { ok: false, falha: "limite", mensagem: r.data?.error };
    return { ok: false, falha: "indisponivel" };
  } finally {
    clearTimeout(relogio);
  }
}

/** O registro de auditoria, como texto. Lança com a mensagem do servidor quando não há registro. */
export async function buscarLog(id: string): Promise<string> {
  const r = await fetch(urlDoLog(id));
  const texto = await r.text();
  if (!r.ok) throw new Error(texto);
  return texto;
}
```

Três cuidados:

- **Sem `axios.create()` por enquanto.** Os quatro testes que simulam o axios simulam só `post` e `isCancel`. A instância com interceptor entra junto com o login, que é quando ela passa a ser necessária.
- **`buscarLog` continua com `fetch`**, para o `LogModal.test.tsx` seguir passando com o `fetch` global simulado. O ganho aqui é de lugar: o modal deixa de conhecer o endereço.
- **Em duas etapas.** Primeiro o módulo nasce e o `callServerAPI` passa a chamá-lo por dentro, sem mudar de assinatura. A troca do `callServerAPI` pelo hook `useConversa` fica para a I8.

Os arquivos estáticos do acervo continuam sem credencial, mesmo depois do login. Só `api/chat.ts` passaria a enviar token.

## Plano de execução

São quatro etapas, da mais segura à mais invasiva, e cada pull request tem um propósito só. O critério de aceite é o mesmo em todas: `tsc`, ESLint, os 336 testes e `npm run build` verdes.

| Etapa | O que faz | Muda comportamento | Tamanho |
| --- | --- | --- | --- |
| 1. Limpeza | corrige o script, remove o que não é usado, atualiza a documentação | não | pequeno |
| 2. Mover | só `git mv` e linhas de import, mais o teste de camadas | não | médio, mecânico |
| 3. Extrair | I1 a I6, um pull request por sugestão | quase nada, listado em cada uma | médio |
| 4. Dividir | I7 (`Home`) e I8 (`Chat`) | não | médio, com reescrita de testes na I8 |

### Etapa 1: limpeza

- [x] Corrigir `gen:entities` no `package.json` para `../scripts/tools/gen-frontend-entities.mjs` e rodar: o `git diff` de `entities.ts` deve ficar vazio.
- [x] Remover `@emotion/styled` e os seis `@types/*` sem uso, e tirar `@emotion/styled` do `chunkMap`.
- [x] Tirar o `export` de `ETAPAS`, `Aba` e `MessageSender`; remover `DEFAULT_ENTITY_ID` no gerador.
- [x] Apagar o `jsconfig.json` e os comentários que o citam no `tsconfig.json` e no `vite.config.js`.
- [x] Usar `DIACRITICOS` de `textSearch` em `citacoes.ts`.
- [x] No `index.css`: unir as duas regras `html`, remover a regra `.chakra-ui-provider` e a variável `--transition-fast`.
- [x] Corrigir os quatro comentários do achado D6.
- [x] Atualizar o `README.md`: entidades, seções e o timeout de 70 segundos.

### Etapa 2: mover sem alterar código

A regra desta etapa é que o diff só tenha renomeações e linhas de `import`. Com `git mv`, o histórico de cada arquivo é preservado. A tabela arquivo a arquivo está no apêndice.

- [ ] Commit 2a: `pages/chat/ui/` vai para `ui/`, e `pages/chat/utils/utils.ts` vira `ui/copiar.ts`.
- [ ] Commit 2b: `shared/` se reparte em `app/`, `ui/`, `lib/`, `api/` e `contexto/`. O caminho de `entities.ts` muda no mesmo commit em `gen-frontend-entities.mjs` e em `check-registry-sync.sh`.
- [ ] Commit 2c: `pages/*` e `tributum/` vão para `secoes/*`, com os nomes novos; `types/chat.ts` vira `secoes/chat/tipos.ts`; `chat/utils/` se reparte em `hooks/` e `lib/`.
- [ ] Commit 2d: entra o `test/camadas.test.ts`, e o layout de diretórios do `DESCRIPTION.md` é reescrito.
- [ ] Commit 2e, à parte: os quatro `.jsx` de `ui/` viram `.tsx` com tipos, sem o `'use client'`. O `allowJs` só pode sair do `tsconfig.json` se `theme/system.js` também virar `.ts`.

Dois testes dependem de caminho e precisam de ajuste no commit 2c:

- `tributum/types.test.ts` lê `../../public/tributum.json` e `../../../tributum/` relativos ao próprio arquivo (linhas 75 e 128). Uma pasta mais fundo, cada caminho ganha um `../`.
- `McpList.test.tsx` simula `"../../shared/fetchers"` pelo caminho (linha 7).

### Etapa 3: extrair as redundâncias

A ordem põe primeiro o que encosta em login e permissão, e depois o que só reduz repetição.

- [ ] I1, armazenamento único. Conferir à mão: recolher a sidebar, o painel de perguntas e trocar o tipo de consulta sobrevivem ao recarregar.
- [ ] I2, saída única para a rede. Conferir à mão: uma pergunta, o log dela e uma pergunta com o servidor desligado.
- [ ] I3, busca compartilhada. Conferir à mão: buscar com acento e com duas palavras em cada lista.
- [ ] I4, acordeão compartilhado. Conferir à mão: abrir e fechar por clique, Enter e Espaço.
- [ ] I5, documento markdown, realce e link.
- [ ] I6, sobreposição e escala de `zIndex`. Conferir à mão no celular: os dois drawers, o log e o PDF, com Esc e com clique no fundo.

### Etapa 4: dividir os arquivos grandes

- [ ] I7, registro de seções e `Navegacao.tsx`.
- [ ] I8, `useConversa`, `Compositor` e `PainelHistorico`, com a repartição do `callServerAPI.test.ts`.

### Decisões que antecedem a etapa 2

Três escolhas mudam o diff da etapa 2 e convém fechá-las antes:

- **Nomes das pastas em português.** A árvore proposta usa `secoes/`, `contexto/`, `mapa/`, `sobre/`, seguindo o código mais novo. A alternativa em inglês seria `features/`, `context/`, `map/`, `about/`.
- **Alias `@/`.** Recomendo adotá-lo para imports entre pastas de primeiro nível (`@/ui/tooltip`) e manter o caminho relativo dentro da mesma pasta. Ele já está configurado, e torna a próxima movimentação mais barata. A alternativa é removê-lo da configuração.
- **Prettier.** Um commit único de formatação resolve o achado D5 e pode ser registrado no `.git-blame-ignore-revs` da raiz, que o projeto já usa para o `rustfmt`. Se for adotado, deve vir antes da etapa 2, para não misturar formatação com movimentação.

## O que não fazer e riscos

A reorganização é de baixo risco porque a suíte é boa, mas há cinco coisas que eu deixaria de fora e quatro pontos de atenção.

### O que deixar como está

- **Não fundir `LegislacaoList` e `JurisprudenciaList`.** O código documenta por que são componentes separados: uma é hierárquica por lei, a outra é plana. A sugestão I3 extrai peças comuns e mantém as duas.
- **Não renomear identificadores em massa** entre português e inglês. O diff seria enorme e o ganho, estético. Basta a regra para código novo.
- **Não introduzir roteador, gerenciador de estado ou carga sob demanda** como parte desta reorganização. Nenhum achado pede isso.
- **Não trocar as chaves do `localStorage` nem os ids `tab-` e `tabpanel-`.** As chaves são o que os usuários já têm gravado, e os ids ancoram os testes de acessibilidade.
- **Não reescrever as TAREFAs e os ESTUDOs antigos de `docs/`.** Há 237 referências a caminhos de `src/` em 10 arquivos, e a maioria é registro histórico. Só `docs/auli_code.md` e o `DESCRIPTION.md` descrevem o estado atual e precisam acompanhar.

### Pontos de atenção

- **Mover e refatorar no mesmo commit.** É o erro que torna a revisão impossível: o `git` deixa de reconhecer a renomeação e o diff vira arquivo apagado mais arquivo novo. Por isso a etapa 2 não altera código.
- **O gerador de `entities.ts`.** Esquecer de atualizar `check-registry-sync.sh` quebra o workflow `registry-sync.yml` no CI, não o build local.
- **Tarefas ainda não executadas.** Uma TAREFA escrita antes da movimentação e enviada ao Claude Code depois dela cita caminhos que não existem mais. Convém executar as pendentes antes da etapa 2 ou atualizar os caminhos nelas.
- **O `CLAUDE.md` da raiz** manda fazer mudanças cirúrgicas e não refatorar o que não está quebrado. Esta reorganização é uma tarefa explícita, mas a instrução vale dentro de cada pull request: cada um faz uma coisa só, e o que aparecer no caminho é anotado, não corrigido junto.

### O que não foi verificado

- Os esboços I1 a I8 não foram compilados nem testados contra o projeto.
- A ausência de regressão visual depende de conferência manual: os testes cobrem lógica e acessibilidade, não aparência.
- O deploy não foi exercitado. Pela leitura, `deploy-frontend.sh` e `build-frontend-public.sh` não citam caminhos de `src/`, e a saída do build não muda.

## Apêndice: mapa de movimentação

Cada arquivo de hoje e o destino proposto. Os testes acompanham o arquivo que testam e não estão listados, salvo quando mudam de nome.

### Raiz e casca do app

| Hoje | Destino |
| --- | --- |
| `main.tsx`, `index.css`, `vite-env.d.ts` | ficam em `src/` |
| `App.tsx` (com `App.test.tsx` e `App.abas.test.tsx`) | `app/App.tsx` |
| `shared/AppHeader.tsx` | `app/AppHeader.tsx` |
| `shared/AbasDeEstado.tsx` | `app/AbasDeEstado.tsx` |
| `shared/abas.ts` | `app/abas.ts` |
| `shared/ErrorBoundary.tsx` | `app/ErrorBoundary.tsx` |
| `theme/system.js` | fica |
| `test/render.tsx`, `test/setup.ts` | ficam |

### Base

| Hoje | Destino |
| --- | --- |
| `pages/chat/ui/provider.jsx` | `ui/provider.tsx` |
| `pages/chat/ui/toaster.jsx` | `ui/toaster.tsx` |
| `pages/chat/ui/tooltip.jsx` | `ui/tooltip.tsx` |
| `pages/chat/ui/color-mode.jsx` | `ui/color-mode.tsx` |
| `pages/chat/utils/utils.ts` | `ui/copiar.ts` |
| `shared/layout.ts` | `ui/layout.ts` |
| `shared/AsyncContent.tsx` | `ui/AsyncContent.tsx` |
| `shared/SearchInput.tsx` | `ui/SearchInput.tsx` |
| `shared/CollectionEmpty.tsx` | `ui/CollectionEmpty.tsx` |
| `tributum/LinkExterno.tsx` | `ui/LinkExterno.tsx` |
| `shared/textSearch.ts` | `lib/textSearch.ts` |
| `shared/highlight.tsx` | `lib/highlight.tsx` |
| `shared/rehypeHighlight.ts` | `lib/rehypeHighlight.ts` |
| `shared/markdown.tsx` | `lib/markdown.tsx` |
| `shared/linkify.tsx` | `lib/linkify.tsx` |
| `shared/fetchers.ts` | `api/estaticos.ts` |
| `pages/chat/utils/api.ts` | `api/chat.ts` (ganha `perguntar` e `buscarLog` na I2) |
| `shared/entities.ts` (gerado) | `contexto/entities.ts` |
| `shared/EntityContext.tsx` | `contexto/EntityContext.tsx` |
| `shared/RespostaDaAba.tsx` | `contexto/RespostaDaAba.tsx` |

### Seções

| Hoje | Destino |
| --- | --- |
| `pages/home/Home.tsx` | `secoes/home/Home.tsx` (dividido na I7) |
| `pages/stateselection/*` | `secoes/mapa/*` |
| `pages/about/About.tsx` | `secoes/sobre/About.tsx` |
| `pages/servicoslist/ServicosList.tsx`, `ServicosAccordion.tsx` | `secoes/servicos/` |
| `pages/servicoslist/utils.ts` (e `utils.test.ts`) | `secoes/servicos/servicos.ts` (e `servicos.test.ts`) |
| `pages/faqslist/FaqsList.tsx`, `FaqsAccordion.tsx` | `secoes/faqs/` |
| `pages/faqslist/parseFaqs.ts` (e teste) | `secoes/faqs/faqs.ts` (e teste) |
| `pages/parecereslist/*` | `secoes/jurisprudencia/*` |
| `pages/legislacaolist/*` | `secoes/legislacao/*` |
| `pages/conteudoslist/ConteudosList.tsx`, `ConteudosAccordion.tsx` | `secoes/conteudos/` |
| `pages/conteudoslist/parseConteudos.ts` (e teste) | `secoes/conteudos/conteudos.ts` (e teste) |
| `pages/notaslist/NotasList.tsx` | `secoes/notas/NotasList.tsx` |
| `pages/downloadslist/*` | `secoes/downloads/*` |
| `pages/mcplist/McpList.tsx` | `secoes/mcp/McpList.tsx` |
| `tributum/*` (menos `LinkExterno.tsx`) | `secoes/tributum/*` |

### Chat

| Hoje | Destino |
| --- | --- |
| `pages/chat/*.tsx` (12 componentes) | `secoes/chat/*.tsx` |
| `types/chat.ts` | `secoes/chat/tipos.ts` (`Fonte` vai para `api/chat.ts` na I2) |
| `utils/useHistorico.ts`, `usePainelHistorico.ts`, `useIsKeyboardVisible.ts`, `useQuestionType.ts` | `secoes/chat/hooks/` |
| `utils/usePrompt.ts`, `utils/useMessages.ts` | `secoes/chat/hooks/` na etapa 2; absorvidos na I8 |
| `utils/callServerAPI.ts` | `secoes/chat/hooks/` na etapa 2; substituído por `useConversa.ts` na I8 |
| `utils/citacoes.ts`, `fontes.ts`, `historico.ts`, `prompt.ts`, `secoes.ts`, `tipos.ts` | `secoes/chat/lib/` |

O arquivo `utils/tipos.ts` (o texto editorial de cada tipo de consulta) e o novo `tipos.ts` da seção teriam nomes iguais em pastas vizinhas. Sugiro renomear o primeiro para `lib/textoDosTipos.ts` na mesma movimentação.

### I3. Busca compartilhada (`ui/busca.tsx`)

A extração é de peças, não de um componente único: um hook com o estado, a barra fixa e a mensagem de vazio. Cada lista continua dona do seu filtro e do seu corpo, o que respeita a decisão registrada na `LegislacaoList` de não fundir listas diferentes.

```tsx
import { useDeferredValue, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { Box, Text } from "@chakra-ui/react";
import { parseQuery } from "@/lib/textSearch";
import { SearchInput } from "./SearchInput";
import { Z } from "./layout";

/** O estado de uma caixa de busca: o texto na hora, o texto adiado e os termos já normalizados. */
export function useBusca() {
  const [texto, setTexto] = useState("");
  const adiado = useDeferredValue(texto); // o campo responde na hora; o filtro usa este
  const inputRef = useRef<HTMLInputElement>(null);
  const termos = useMemo(() => parseQuery(adiado), [adiado]);
  return {
    texto,
    setTexto,
    adiado,
    termos,
    inputRef,
    buscando: adiado.trim().length > 0,
    limpar: () => {
      setTexto("");
      inputRef.current?.focus();
    },
  };
}
export type Busca = ReturnType<typeof useBusca>;

/** A barra fixa no topo da lista. `antes` recebe o que vem acima do campo (as abas de público
 *  dos Serviços); `children` é a linha do contador. */
export function BarraDeBusca({
  busca,
  placeholder,
  antes,
  children,
}: {
  busca: Busca;
  placeholder: string;
  antes?: ReactNode;
  children?: ReactNode;
}) {
  return (
    <Box position="sticky" top={0} zIndex={Z.barraFixa} bg="bg.app" px={4} pt={3} pb={2}
         borderBottom="1px solid" borderColor="border">
      {antes}
      <SearchInput
        ref={busca.inputRef}
        value={busca.texto}
        onChange={(e) => busca.setTexto(e.target.value)}
        onClear={busca.limpar}
        placeholder={placeholder}
      />
      {children && (
        <Text pt={2} fontSize="0.8rem" color="fg.muted" fontWeight="500">
          {children}
        </Text>
      )}
    </Box>
  );
}

export const SemResultados = ({ consulta }: { consulta: string }) => (
  <Box py={10} textAlign="center">
    <Text color="fg.muted" fontSize="0.95rem">
      Nenhum resultado encontrado para &ldquo;{consulta}&rdquo;
    </Text>
  </Box>
);
```

A `JurisprudenciaList` ficaria assim, com o que é dela (o índice preguiçoso e o filtro) preservado:

```tsx
const busca = useBusca();
const indice = useMemo(
  () => (busca.buscando ? buildPareceresIndex(documentos) : undefined),
  [documentos, busca.buscando],
);
const filtrados = useMemo(
  () => searchPareceres(documentos, busca.adiado, indice),
  [documentos, busca.adiado, indice],
);

return (
  <Flex direction="column" flex={1} w="100%" bg="bg.app">
    <BarraDeBusca busca={busca} placeholder={`Pesquisar ${plural}...`}>
      {documentos.length > 0 && (
        <>
          {filtrados.length} {filtrados.length !== 1 ? plural : singular}
          {busca.buscando ? ` de ${documentos.length}` : ""}
        </>
      )}
    </BarraDeBusca>
    <Box px={4} pt={3} pb={6}>
      <AsyncContent loading={isLoading} error={error} ...>
        {busca.buscando && filtrados.length === 0 ? (
          <SemResultados consulta={busca.adiado} />
        ) : (
          <PareceresAccordion pareceres={filtrados} termos={busca.termos} />
        )}
      </AsyncContent>
    </Box>
  </Flex>
);
```

Dois detalhes preservados de propósito. `buscando` continua vindo do texto, e não dos termos, porque uma letra isolada é descartada pelo `parseQuery` mas ainda conta como busca ativa nas listas de hoje. E os acordeões passam a receber `termos` prontos, em vez de cada um refazer o `parseQuery` a partir do texto, como fazem hoje `FaqsAccordion`, `ConteudosAccordion` e `PareceresAccordion`.

O `TributumShell` passa a usar as mesmas peças por dentro. A `ConteudosList` ganha a barra fixa, que é a única mudança visual desta sugestão.

### I4. Acordeão compartilhado (`ui/Acordeao.tsx`)

Duas peças pequenas cobrem as cinco cópias: o painel que se revela e as props de um elemento que se comporta como botão.

```tsx
import type { KeyboardEvent, ReactNode } from "react";
import { AnimatePresence, m, useReducedMotion } from "framer-motion";

/**
 * O painel de um acordeão: revela os filhos animando a altura de 0 a `auto`.
 *
 * `contain: layout paint` prende o custo da animação na fronteira do painel: animar `height`
 * refaz o layout a cada quadro. É o caso que a regra `no-layout-property-animation` manda conter
 * em vez de reescrever (ver `doctor.config.json`). Este é o ÚNICO lugar do app com essa animação.
 */
export function PainelRevelado({ aberto, children }: { aberto: boolean; children: ReactNode }) {
  const semMovimento = useReducedMotion();
  return (
    <AnimatePresence initial={false}>
      {aberto && (
        <m.div
          initial={{ height: 0, opacity: 0 }}
          animate={{ height: "auto", opacity: 1 }}
          exit={{ height: 0, opacity: 0 }}
          transition={semMovimento ? { duration: 0 } : { duration: 0.22, ease: "easeInOut" }}
          style={{ overflow: "hidden", contain: "layout paint" }}
        >
          {children}
        </m.div>
      )}
    </AnimatePresence>
  );
}

/** Props de um elemento que não é `<button>` mas age como um: clique, Enter e Espaço. */
export function comoBotao(acao: () => void, ativo = true) {
  if (!ativo) return {};
  return {
    role: "button" as const,
    tabIndex: 0,
    onClick: acao,
    onKeyDown: (e: KeyboardEvent) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        acao();
      }
    },
  };
}
```

O uso, no cabeçalho de um item de parecer:

```tsx
<Flex {...comoBotao(() => setAberto((a) => !a))} aria-expanded={aberto} ...>
  ...
</Flex>
<PainelRevelado aberto={aberto}>
  ...
</PainelRevelado>
```

A duração fica em 0,22 s para todos (hoje Serviços e FAQs usam 0,25 s). Depois da troca, a nota do `doctor.config.json` deve dizer "1 ocorrência" no lugar de "15".

### I5. Documento markdown, realce e link (`ui/` e `lib/markdown.tsx`)

Três extrações pequenas, que podem ir no mesmo commit.

```tsx
// ui/DocumentoMarkdown.tsx
export const Prosa = ({ children }: { children: string }) => (
  <Box fontSize="1rem" lineHeight="1.7" color="fg" fontFamily="body">
    <ReactMarkdown remarkPlugins={markdownPlugins} components={proseMarkdownComponents}>
      {children}
    </ReactMarkdown>
  </Box>
);

/** Busca um `.md` de `public/` e o mostra como prosa, com os estados de carga e erro. */
export function DocumentoMarkdown({
  arquivo,
  carregando = "Carregando…",
  erro = "Não foi possível carregar o texto agora.",
}: {
  arquivo: string;
  carregando?: string;
  erro?: string;
}) {
  const { data, error, isLoading } = useSWR(versioned(arquivo), textFetcher, SWR_OPTS);
  return (
    <AsyncContent loading={isLoading} error={error} loadingText={carregando} errorDescription={erro}>
      <Prosa>{data ?? ""}</Prosa>
    </AsyncContent>
  );
}
```

O `About.tsx` cai de 50 para cerca de 10 linhas, e o `CriteriosTributum` e o `LeitorManual` ficam iguais a ele:

```tsx
export const About = () => (
  <Flex direction="column" flex={1} w="100%" bg="bg.app">
    <Box w="100%" maxW="720px" mx="auto" px={5} pt={5} pb={10}>
      <DocumentoMarkdown arquivo="/about.md" erro="Não foi possível carregar o manifesto." />
    </Box>
  </Flex>
);
```

Para o realce, `lib/markdown.tsx` passa a exportar o mapa pronto, e os dois overrides locais somem:

```tsx
/** O `<mark>` do realce de busca: o mesmo visual no texto simples e no markdown. */
export const Marca = ({ children }: { children?: ReactNode }) => (
  <Box as="mark" bg="bg.highlight" color="fg.highlight" px="0.1em" borderRadius="2px">
    {children}
  </Box>
);

/** O mapa compacto, mais o `<mark>` que o `rehypeHighlight` injeta. */
export const compactMarkdownComRealce: Components = { ...compactMarkdownComponents, mark: Marca };
```

O `Highlight` de `lib/highlight.tsx` usa a mesma `Marca`. E o `LinkExterno` sobe de `tributum/` para `ui/` e substitui os dois "Abrir no portal" inline. Para os quatro `window.open`, basta uma função `abrirEmNovaAba(url)` no mesmo arquivo.

As classes `markdown-body` e `faq-answer` saem junto, porque não têm regra de CSS.

### I6. Sobreposição e escala de `zIndex` (`ui/Sobreposicao.tsx`, `ui/layout.ts`)

A escala põe num lugar a relação que hoje só existe em comentários espalhados:

```ts
/** As camadas do app, da mais baixa à mais alta. Quem precisa de `zIndex` lê daqui. */
export const Z = {
  barraFixa: 10, // barra de busca das listas
  compositor: 20, // caixa de mensagem do chat (fixed)
  painelHistorico: 21, // coluna "Minhas perguntas": o compositor avança sobre a borda dela
  menuSuspenso: 30, // seletor do tipo de consulta, que abre para cima
  cabecalho: 100, // AppHeader (sticky)
  sobreposicao: 110, // drawers e modais: acima do cabeçalho
} as const;
```

E um componente cobre o esqueleto comum dos quatro overlays:

```tsx
import { useEffect } from "react";
import type { ReactNode, RefObject } from "react";
import { Box } from "@chakra-ui/react";
import type { BoxProps } from "@chakra-ui/react";
import { Z } from "./layout";

interface SobreposicaoProps extends BoxProps {
  onFechar: () => void;
  /** Quem recebe o foco quando o Esc fecha: o botão que abriu. */
  devolverFocoA?: RefObject<HTMLElement | null>;
  fundo?: string;
  children: ReactNode;
}

/** Camada sobre o app: fundo que fecha ao clicar, Esc que fecha, e o painel por cima. */
export function Sobreposicao({
  onFechar,
  devolverFocoA,
  fundo = "blackAlpha.500",
  children,
  ...resto
}: SobreposicaoProps) {
  useEffect(() => {
    const aoTeclar = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      onFechar();
      devolverFocoA?.current?.focus();
    };
    document.addEventListener("keydown", aoTeclar);
    return () => document.removeEventListener("keydown", aoTeclar);
  }, [onFechar, devolverFocoA]);

  return (
    <Box position="fixed" inset={0} zIndex={Z.sobreposicao} {...resto}>
      <Box position="absolute" inset={0} bg={fundo} onClick={onFechar} />
      {children}
    </Box>
  );
}
```

O drawer do `Home` vira `<Sobreposicao hideFrom="md" onFechar={...} devolverFocoA={menuBtnRef}>`, e os dois `useEffect` de Esc do `Home` e do `Chat` somem.

**O que muda de comportamento.** O `LogModal` e o `PdfModal` sobem de `zIndex` 100 para 110, e passam a usar o ouvinte de Esc do `document` em vez do `window`.

**O que fica para depois.** Nenhum dos quatro overlays prende o foco. Com um componente só, isso se resolve uma vez: ou com uma armadilha de foco aqui dentro, ou trocando o miolo pelo `Dialog` do Chakra, decisão que o código de hoje adiou de propósito.

### I7. Registro de seções e divisão do `Home`

O registro sai do componente e vira um arquivo de dados:

```ts
// secoes/registro.ts
export type Grupo = null | "acervo" | "tributum" | "integracoes";

export interface Secao {
  /** Ancora os ids `tab-` e `tabpanel-`: não muda. */
  id: string;
  rotulo: string;
  grupo: Grupo;
  Icone: IconType;
  Componente: ComponentType;
  /** Coleção exigida da entidade; `null` = sempre disponível. */
  colecao: Collection | null;
}

export const SECOES: Secao[] = [
  { id: "chat", rotulo: "Chat", grupo: null, Icone: MdOutlineChat, Componente: Chat, colecao: null },
  { id: "servicos", rotulo: "Serviços", grupo: "acervo", Icone: MdOutlineWorkOutline, Componente: ServicosList, colecao: "servicos" },
  // ... as outras 15, na ordem de hoje
];

export const GRUPOS: { id: Grupo; titulo: string | null }[] = [
  { id: null, titulo: null },
  { id: "acervo", titulo: "Acervo" },
  { id: "tributum", titulo: "Tributum" },
  { id: "integracoes", titulo: "Integrações" },
];

/** As seções como a entidade as vê: todas visíveis, as sem coleção desativadas. */
export const secoesDe = (entity: Entity) =>
  SECOES.map((s) => ({ ...s, desativada: s.colecao !== null && !hasCollection(entity, s.colecao) }));
```

O `Home.tsx` de 506 linhas se reparte em três:

| Arquivo | Conteúdo | Linhas aproximadas |
| --- | --- | --- |
| `secoes/registro.ts` | `SECOES`, `GRUPOS`, `secoesDe` | 70 |
| `secoes/home/Navegacao.tsx` | `NavItem`, `NavList`, `proximaHabilitada`, a pista de rolagem | 230 |
| `secoes/home/Home.tsx` | seção ativa, drawer (com `Sobreposicao`), painéis | 180 |

O `Home.test.ts` passa a importar `proximaHabilitada` de `./Navegacao`. Os ids e os rótulos não mudam, então o `Home.drawer.test.tsx` e o `App.abas.test.tsx` não precisam de ajuste.

Três observações sobre o que não fazer agora:

- **Permissão por seção.** Quando houver controle de usuários, a regra entra como um campo a mais em `Secao` e um filtro em `secoesDe`. Não vale criar o campo antes da feature.
- **Carga sob demanda.** O registro permitiria `lazy(() => import(...))` por seção. O ganho é pequeno: o código do app tem 204 kB (66 kB comprimido) de um total de cerca de 1.075 kB de JavaScript, e o resto são bibliotecas.
- **O gate repetido nas listas.** Pelo que li, o `Home` não deixa selecionar uma aba desativada, nem pelo clique nem pelas setas. O `CollectionEmpty` dentro das seis listas seria então só defensivo, e poderia sair delas num segundo momento.

### I8. Divisão do `Chat`

O estado da conversa vira um hook, que usa `perguntar` (I2) e dispensa o `callServerAPI`, o `useMessages` e a referência `isSubmittingRef` do componente:

```ts
// secoes/chat/hooks/useConversa.ts
export function useConversa(entityId: string) {
  const [mensagens, setMensagens] = useState<Message[]>([]);
  const [esperando, setEsperando] = useState(false);
  const emEnvio = useRef(false); // trava síncrona: dois Enter seguidos não enviam duas vezes

  const enviar = async (pergunta: string, tipo: QuestionType) => {
    if (emEnvio.current) return;
    emEnvio.current = true;
    setEsperando(true);
    const idDaEspera = crypto.randomUUID();
    setMensagens((m) => [
      ...m,
      { id: crypto.randomUUID(), from: "user", text: pergunta, showButton: true },
      { id: idDaEspera, from: "server", text: TEXTOS.pensando, showButton: false, pendente: true },
    ]);
    const inicio = Date.now();
    const r = await perguntar(pergunta, entityId, Number(tipo));
    const resposta: Message = r.ok
      ? {
          id: crypto.randomUUID(), from: "server", showButton: true,
          text: r.texto || TEXTOS.semResposta,
          logId: r.logId, fontes: r.fontes, tipo, duracaoMs: Date.now() - inicio,
        }
      : { id: crypto.randomUUID(), from: "server", showButton: true, text: r.mensagem || TEXTOS[r.falha] };
    setMensagens((m) => m.map((x) => (x.id === idDaEspera ? resposta : x)));
    setEsperando(false);
    emEnvio.current = false;
  };

  return { mensagens, esperando, enviar };
}
```

O restante do `Chat.tsx` se reparte por responsabilidade:

| Arquivo | Conteúdo |
| --- | --- |
| `Chat.tsx` | compõe as partes, decide entre `EstadoInicial` e `Messages`, cuida da rolagem |
| `Compositor.tsx` | a barra fixa: botões, barrinha do celular, `Input`, aviso, medição de altura e recuo do teclado |
| `PainelHistorico.tsx` | "Minhas perguntas" nas duas formas, coluna e drawer (com `Sobreposicao`) |
| `hooks/useConversa.ts` | mensagens, espera e envio |
| `lib/tipoDeConsulta.ts` | o tipo `QuestionType`, `isQuestionType`, `tiposDisponiveis` e a tabela de coleções |
| `tipos.ts` | `Message` (hoje em `types/chat.ts`) |

O `usePrompt` (12 linhas) volta a ser um `useState` dentro do `Chat`.

**Testes.** Esta é a sugestão que mais mexe em teste. O `callServerAPI.test.ts` (154 linhas) cobre duas coisas que passam a morar em lugares diferentes: o mapeamento de erros vai para `api/chat.test.ts`, com a mesma simulação do axios, e a sequência de mensagens vai para `useConversa.test.ts`, com `renderHook`. O `Chat.test.tsx` simula `axios.post` e deve seguir passando sem alteração.

**Opcional, por último.** Trocar `showButton` por um campo que diga o que a mensagem é, por exemplo `natureza: "resposta" | "erro"`, e derivar dele os botões, a largura e a rolagem (achado C4).

### I9. Teste de camadas (`test/camadas.test.ts`)

Um teste do Vitest lê os imports de `src/` e falha quando uma das regras da estrutura é quebrada. Segue o costume do projeto de guardar invariantes em teste (como a conferência do catálogo do Tributum) e não exige dependência nova.

```ts
import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, normalize, relative, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const SRC = join(dirname(fileURLToPath(import.meta.url)), "..");
const BASE = ["ui", "lib", "api", "contexto", "theme"];
const SECOES = "secoes";
/** Seções que existem para compor as outras. */
const COMPOE = ["home"];

function arquivos(dir: string): string[] {
  return readdirSync(dir).flatMap((nome) => {
    const caminho = join(dir, nome);
    if (statSync(caminho).isDirectory()) return arquivos(caminho);
    return /\.(tsx?|jsx?)$/.test(nome) && !/\.test\./.test(nome) ? [caminho] : [];
  });
}

/** [arquivo, alvo] de cada import relativo ou `@/`, os dois relativos a `src/`. */
function imports(): [string, string][] {
  return arquivos(SRC).flatMap((arq) =>
    [...readFileSync(arq, "utf-8").matchAll(/from\s+["'](\.{1,2}\/[^"']*|@\/[^"']*)["']/g)].map(
      ([, alvo]): [string, string] => [
        relative(SRC, arq),
        alvo.startsWith("@/") ? normalize(alvo.slice(2)) : relative(SRC, join(dirname(arq), alvo)),
      ],
    ),
  );
}

const pasta = (p: string) => p.split(sep)[0];
const secao = (p: string) => (pasta(p) === SECOES ? p.split(sep)[1] : null);

describe("camadas", () => {
  it("a base não importa de seções nem de app/", () => {
    const fora = imports().filter(
      ([de, para]) => BASE.includes(pasta(de)) && [SECOES, "app"].includes(pasta(para)),
    );
    expect(fora).toEqual([]);
  });

  it("uma seção não importa de outra", () => {
    const fora = imports().filter(([de, para]) => {
      const a = secao(de);
      const b = secao(para);
      // `length > 2` deixa de fora os arquivos soltos em secoes/, como o registro.ts.
      return a && b && a !== b && !COMPOE.includes(a) && de.split(sep).length > 2;
    });
    expect(fora).toEqual([]);
  });
});
```

Rodei essa lógica contra a árvore atual, trocando só os nomes das pastas (`BASE` = `shared`, `types`, `theme`, `test`; `SECOES` = `pages`). Ela acusou cinco imports na primeira regra e um na segunda, que são os do grupo A fora do `App.tsx` e do `Home.tsx`, onde a regra permite. O teste entra no fim da etapa 2, quando a árvore nova já passa nele.
