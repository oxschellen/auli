# ESTUDO-abas-por-estado — Vários estados abertos ao mesmo tempo, cada um na sua aba

**Data:** 10/11/2026 · **Status:** decidido em 10/11/2026 — **caminho A, sem o B** (§9);
implementado com D-ABAS-5..7.
**Leitura prévia:** [auli_code.md](auli_code.md) §4.2 (estrutura da aplicação), §4.5 (seleção de
estado) e §4.3.1 (D-UI-4, o cabeçalho com "SEFAZ-RS ▾").

**Convenção:** **medido** (saiu de execução real), **deduzido** (lido no código ou inferido, com a
evidência ao lado) e **não verificado** (fica para a §10).

**Termos:** neste estudo, **aba de estado** é a novidade (uma por estado aberto). As abas que já
existem — Chat, Serviços, FAQs, Pareceres… — o código chama de "abas", mas aqui são **seções**,
como no `aria-label="Seções"` da barra lateral, para os dois não se confundirem.

---

## 1. A pergunta e a resposta curta

**Pergunta:** como ter cada estado escolhido na sua própria aba, com o nome do estado na aba — do
jeito que a aba do navegador mostra o título da conversa ("Consultas formais scrapers code")?

**Resposta curta:** há dois caminhos, e eles não se excluem.

- **B — abas do navegador.** O estado vai para a URL (`?estado=rs`) e para o título da página
  ("SEFAZ-RS · Auli"). Cada aba do navegador fica num estado, sobrevive ao recarregar e vira link
  compartilhável. Pequeno: 3 a 4 arquivos.
- **A — abas dentro do Auli.** Uma faixa de abas no cabeçalho (`[RS SEFAZ-RS ×] [SP SEFAZ-SP ×]
  [+]`), cada aba com a sua conversa, a sua seção e a sua rolagem, trocando sem perder nada. Médio:
  7 a 8 arquivos e uma mudança obrigatória nos ids da página (§5.3).

**Recomendação:** **B primeiro**, porque entrega já o que a captura mostra — o nome na aba — e
corrige um defeito de hoje (§2). **A depois, só se o B não bastar**; o A reaproveita a URL do B.

## 2. Como é hoje

- **Um estado por vez.** O `EntityContext` guarda uma entidade, persistida em `auli.entity`
  (deduzido: [EntityContext.tsx](../auli-frontend/src/shared/EntityContext.tsx)).
- **Trocar de estado apaga a sessão.** "SEFAZ-RS ▾" chama `clearEntity`, o `AppShell` troca o
  `Home` pelo mapa, e o `Home` desmonta. Com ele vão a conversa (estado React em memória,
  [useMessages.ts](../auli-frontend/src/pages/chat/utils/useMessages.ts)), a seção aberta, a busca e
  a rolagem (deduzido: [App.tsx](../auli-frontend/src/App.tsx)).
- **O título é fixo.** `<title>Auli</title>` no `index.html`, sem `document.title` em lugar nenhum
  do `src/` (deduzido: grep). Duas abas do navegador com o Auli são indistinguíveis.
- **A URL não tem o estado, e as abas do navegador se atropelam.** O estado é lido do
  `localStorage` na partida, e o `localStorage` é compartilhado entre as abas do navegador. Abra o
  RS numa aba, escolha SP em outra, recarregue a primeira: ela abre no **SP** (deduzido:
  `readStoredEntity` na partida, `selectEntity` grava a chave).
- **O mapa não tem links.** As regiões são `<path role="button" onClick>`
  ([BrazilMap.tsx](../auli-frontend/src/pages/stateselection/BrazilMap.tsx)): Ctrl+clique e o botão
  do meio não abrem o estado em nova aba.
- **Os dados de cada estado visitado já ficam na memória.** O SWR usa o cache global padrão, sem
  `SWRConfig` com `provider` e sem descarte (deduzido: grep). Trocar RS → SP → RS hoje não baixa o
  RS de novo, mas também não solta o RS da memória.

## 3. Medições

**Memória por estado** (medido: build de produção no `vite preview`, Chrome headless 1280×720,
coleta de lixo forçada antes de cada leitura, `Performance.getMetrics`, um perfil novo por estado):

| Estado | Seções abertas | Heap JS | Nós do DOM |
|---|---|---|---|
| qualquer | só o Chat (partida) | 6,1 MB | 276 |
| RS | Serviços, FAQs, Pareceres, TARF, Legislação | 35,6 MB | 3.110 |
| SP | Serviços, FAQs, Pareceres | 44,4 MB | 2.353 |
| MG | Serviços, FAQs | 10,9 MB | 532 |

O peso é dado, não DOM: o `sp-pareceres-index.json` tem 16 MB e o `rs-tarf-index.json` 7,6 MB, mas
os pareceres desenham em lotes de 100
([PareceresAccordion.tsx](../auli-frontend/src/pages/parecereslist/PareceresAccordion.tsx)).

**A URL em produção** (medido: `curl -I` em `auli.com.br`, 10/11/2026):

| URL | Resposta |
|---|---|
| `/?estado=rs` | 200, o app |
| `/rs` | 301 → `/rs/` |
| `/rs/` | 200, o **mesmo** `index.html` da raiz (só muda o nonce do Rocket Loader do Cloudflare) |
| `/qualquer-coisa` | 200, o app (`FallbackResource /index.html`) |

## 4. Caminho B — abas do navegador

```text
[ SEFAZ-RS · Auli  × ][ SEFAZ-SP · Auli  × ][ Consultas formais… × ]  +
```

### 4.1 O que muda

- **Partida:** `?estado=<id>` na URL vale primeiro; sem ele, `auli.entity` (que passa a ser só "o
  último usado"); sem os dois, o mapa.
- **Escolher um estado** grava a URL com `history.replaceState`, sem criar entrada no Voltar
  (§9, item 4). **"SEFAZ-RS ▾"** volta ao mapa e tira o `estado` da URL.
- **Título:** `document.title` = `"<nome> · Auli"` com estado, `"Auli"` no mapa.
- **Mapa com links reais:** cada região vira `<a href="?estado=sp">` (o SVG aceita `<a>`). O clique
  simples segue dentro do app (`preventDefault` só no clique esquerdo sem modificador). Ctrl/⌘ +
  clique e o botão do meio abrem o estado numa aba nova do navegador, sem código a mais.

### 4.2 Por que query e não caminho

`/rs/` funciona em produção (§3), mas por coincidência: `rs` é também a pasta de dados
`public/rs/`, e o app só aparece porque a pasta não tem `index.html` e cai no `FallbackResource`.
Paga um 301 a cada `/rs` sem barra e quebra no dia em que uma pasta de dados ganhar um índice.
`?estado=rs` não depende de nada disso.

### 4.3 Custo e limites

- **Tamanho:** `EntityContext` (ler e gravar a URL), `App.tsx` (título), `BrazilMap.tsx` (links),
  testes.
- **Cada aba do navegador é uma página inteira:** baixa os dados de novo (o `?v=<build>` deixa o
  cache HTTP servir, §10) e ocupa a memória da §3 sozinha.
- **Não resolve:** trocar de estado **dentro** da mesma aba continua apagando a conversa, e não há,
  no Auli, a visão de "quais estados estão abertos".

## 5. Caminho A — abas dentro do Auli

```text
[▢] Auli   [RS SEFAZ-RS ×] [SP SEFAZ-SP ×] [+]                              ☾
```

### 5.1 O modelo

- `abas: { chave, entityId | null }[]` e a `ativa`. **"+"** abre uma aba nova com o mapa dentro,
  como a página de nova aba do navegador. Escolher no mapa um estado **já aberto** ativa a aba
  existente e fecha a nova (§9, item 2). Fechar a última aba volta ao mapa.
- **Cada aba é um `Home` próprio, mantido montado e escondido** — o mesmo que as seções já fazem
  ("montada na primeira ativação e mantida montada", auli_code §4.2). A conversa, a seção, a busca,
  a rolagem e até uma pergunta em andamento sobrevivem à troca.
- **Cada aba fornece a sua entidade.** `useSelectedEntity()` passa a ler de um provider por aba, em
  vez do global. Os 12 arquivos que leem a entidade (as listas, o Chat, o Home) não mudam.
- **Persistência:** a lista de abas em `auli.abas`, que substitui `auli.entity` (migração: se só
  houver `auli.entity`, abre aquele). As conversas não persistem, como hoje.
- **Com o B:** a URL e o título seguem a aba ativa.

### 5.2 Custo de memória

Cada aba aberta mantém a sua memória (§3): até ~40 MB num estado pesado com tudo aberto, quase todo
dado. Esse dado o SWR **já** guarda hoje para cada estado visitado (§2), então manter as abas
montadas acrescenta pouco além do DOM (~3 mil nós por aba pesada). **Fechar uma aba não devolve
memória** a menos que se apaguem do cache as chaves daquele estado (§9, item 6).

### 5.3 O que é obrigatório mudar: ids únicos por aba

Com dois `Home` montados, estes ids saem **duplicados**:

| id | Onde | Quem aponta para ele |
|---|---|---|
| `tab-<seção>`, `tabpanel-<seção>` | `Home.tsx` | `aria-labelledby` dos painéis, `aria-controls` das seções |
| `sidebar-secoes` | `Home.tsx` | `aria-controls` do botão de recolher a barra lateral (`AppHeader`) |
| `prompt-id` | `Input.tsx` | ninguém (a caixa se nomeia por `aria-label`), mas repetido é HTML inválido |

O navegador resolve um id repetido para o **primeiro** do documento, então o leitor de tela nomearia
o painel do SP com a seção do RS. A saída é prefixar com a chave da aba. Os ids das mensagens
(`msg-<uuid>`, usados pelo rolar até a resposta e pelas fontes) já são únicos (deduzido:
`crypto.randomUUID()` em `callServerAPI.ts`).

A barra fixa do chat de uma aba escondida não aparece: um `fixed` dentro de um ancestral
`display: none` não é desenhado (deduzido, §10).

### 5.4 O cabeçalho

- A faixa substitui o "SEFAZ-RS ▾" (D-UI-4) e fica ao lado do logo. É um `tablist` "Estados
  abertos": setas movem, Delete fecha. O × **não pode** ser um botão dentro do botão da aba (HTML
  inválido); fica ao lado, fora da ordem de Tab.
- **Celular (< md):** a faixa não cabe ao lado do logo. Proposta: um "RS ▾" que lista as abas
  abertas e "Abrir outro estado" (§9, item 5).
- **Resposta em segundo plano:** uma pergunta feita no RS continua enquanto se olha o SP. Um
  indicador na aba (girando, depois um ponto) exige subir o `loading` do Chat até a aba — fica para
  a fase 3. *Feito em 11/11/2026, só o girando:* um spinner enquanto a conversa espera, e
  "respondendo" no nome acessível da aba (`shared/RespostaDaAba.tsx`). O ponto depois, não.

### 5.5 Tamanho

`EntityContext` (vira o contexto das abas), `App.tsx`, `AppHeader.tsx` (a faixa), um componente
novo da faixa, `Home.tsx` e `Input.tsx` (ids), `StateSelection.tsx` (dentro da aba), testes.

## 6. Comparação

| | B — navegador | A — dentro do Auli |
|---|---|---|
| Vários estados ao mesmo tempo | sim, em abas do navegador | sim, na mesma página |
| Trocar de estado sem perder a conversa | só entre abas do navegador | sim |
| Nome do estado na aba | sim, na aba do navegador | sim, na faixa (e no navegador, com o B) |
| Sobrevive ao recarregar | sim (a URL) | a lista de abas sim; as conversas não |
| Link para um estado | sim | sim, com o B |
| Celular | as abas do navegador móvel | exige o menu da §5.4 |
| Memória | uma página inteira por aba | soma das abas (§5.2) |
| Tamanho | 3–4 arquivos | 7–8 arquivos, ids por aba |

## 7. Recomendação

1. **Fazer o B.** É pequeno, resolve o atropelo de abas da §2 e dá o nome do estado na aba.
2. **Usar o B por um tempo.** Se a necessidade for comparar estados lado a lado, o navegador já
   resolve.
3. **Fazer o A só se** o incômodo for trocar de estado **dentro** da mesma aba e perder a conversa.
   Nesse caso, A sobre o B: a URL passa a ser a da aba ativa.

## 8. Decisões propostas (D-ABAS-\*)

| Id | Decisão | Por quê |
|---|---|---|
| **D-ABAS-1** | O estado vai na URL como `?estado=<id>`, não no caminho | §4.2: o caminho funciona por coincidência com a pasta de dados e paga um 301 |
| **D-ABAS-2** | Título `"<nome> · Auli"`; `"Auli"` no mapa | É o que aparece na aba do navegador |
| **D-ABAS-3** | Na partida, a URL vale mais que `auli.entity` | Cada aba do navegador recarrega no seu estado |
| **D-ABAS-4** | As regiões do mapa viram links `<a href="?estado=…">` | Ctrl+clique e botão do meio abrem nova aba sem código a mais |
| **D-ABAS-5** | (A) Abas inativas ficam montadas e escondidas | Mesmo padrão das seções; custo na §5.2 |
| **D-ABAS-6** | (A) Ids prefixados pela chave da aba | §5.3: sem isso, os nomes acessíveis apontam para a aba errada |
| **D-ABAS-7** | (A) Persistir a lista de abas, não as conversas | As conversas já não persistem hoje |

## 9. Decisões em aberto

**Decidido em 10/11/2026:** item 1 = **A**, sem o B (a URL e o título seguem sem o estado);
itens 2 e 3 como propostos (sem estado repetido; o rótulo é o estado); item 5 = **a faixa rolando
na horizontal**, só com a sigla abaixo de `sm`, em vez do menu; item 6 fica para depois (fechar
não solta a memória). A partida sem nada salvo e a última aba fechada mostram o mapa numa aba
"Novo estado"; o "+" não empilha abas no mapa.

1. **B, A, ou B e depois A?** (recomendação na §7)
2. **(A) O mesmo estado em duas abas**, para duas conversas paralelas sobre o RS? A proposta é não
   na v1: escolher um estado aberto ativa a aba dele.
3. **O rótulo da aba:** só o estado, ou o estado mais a primeira pergunta, como o título de
   conversa da captura?
4. **(B) `replaceState` ou `pushState`?** O `pushState` faz o Voltar voltar ao estado anterior, mas
   a conversa daquele estado já se perdeu (§2), e exige tratar o `popstate`. A proposta é
   `replaceState`.
5. **(A) Celular:** o menu da §5.4, ou a faixa rolando na horizontal?
6. **(A) Fechar a aba solta a memória daquele estado?** Exige apagar do cache do SWR as chaves dele;
   sem isso, a memória só volta ao recarregar, como hoje.

## 10. Não verificado

- A §3 mediu cada estado num perfil separado, não várias abas somadas numa página.
- O cabeçalho de cache HTTP dos JSON versionados (`?v=<build>`), de que depende o custo do B por aba.
- O `vite dev` com URL de caminho (`/rs/`), já que a pasta existe em `public/`.
- Uma barra `fixed` dentro de um ancestral escondido, num navegador real (§5.3).
- Leitor de tela na faixa de abas com ×; Safari e Firefox.
- Quais atalhos de troca de aba (Ctrl+1…9, Ctrl+Tab) a página consegue interceptar — o v1 não usa
  atalho de teclado.

## 11. Fases sugeridas

1. **Fase 1 — B:** URL, título e links no mapa, com testes (partida pela URL, a URL vencendo o
   `localStorage`, título, clique com modificador sem `preventDefault`).
2. **Fase 2 — A básico:** faixa, abas montadas, provider por aba, ids prefixados, `auli.abas` com
   migração.
3. **Fase 3 — A completo:** indicador de resposta em segundo plano, celular, descarte do cache ao
   fechar.
