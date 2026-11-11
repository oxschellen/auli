# Estudo-sidebar-direita — "Minhas perguntas": as perguntas enviadas e as favoritas, num painel à direita do chat (Etapa 1)

**Data:** 10/11/2026 · **Status:** aplicado em 11/11/2026, com os portões da §8 aprovados (ver a nota abaixo).
**Base:** `main` em `242dec4` (10/11/2026, PR #220).
**Leitura prévia:** `auli-frontend/src/pages/chat/Chat.tsx`, `src/pages/home/Home.tsx`,
`src/App.tsx`, `src/shared/layout.ts`, `src/pages/chat/utils/useQuestionType.ts` e a seção
Privacidade de `public/about.md`.

> Documento único para o Claude Code: o estudo, o passo a passo e, no Anexo, o patch completo
> (código e testes). Trabalhe **um passo de cada vez** e confirme comigo (Carlos) nos portões
> marcados. Branch nova a partir do `main`; nada de push ou merge sem confirmação.

> **Aplicado em 11/11/2026**, sobre o `072dbe0` (v0.1.70). O `main` tinha andado três entregas
> desde o `242dec4`: as abas de estado (#221), o alinhamento delas com a barra lateral e o spinner na
> aba (#222). O `git am -3` parou em quatro conflitos de import e de fim de arquivo, resolvidos
> mantendo os dois lados. Três ajustes:
>
> - a escolha do painel passou a valer para todas as abas de estado
>   (`utils/usePainelHistorico.ts`), em vez de um `useState` por `Chat`;
> - a frase do Sobre diz "os estados abertos" no lugar de "o estado";
> - um teste novo grava antes de o aviso `storage` chegar: a mutação "a gravação parte da lista da
>   tela" (§6) passava nos testes do patch, porque o teste existente dispara o `storage` antes de
>   gravar.

---

## 1. O pedido e a resposta curta

Guardar as perguntas que a pessoa faz no chat e deixá-la marcar as prediletas, num painel à direita
da tela.

Esta é a **Etapa 1**: sem login e sem servidor. A lista mora no navegador de quem perguntou. A
Etapa 2 (a lista no servidor, depois que houver controle de usuários) fica fora deste estudo; a §10
diz o que ela reaproveita.

A Etapa 1 cabe inteira no frontend: 776 linhas novas, das quais 260 são testes. O servidor, a API,
os prompts e os packs não mudam. O `Home.tsx` e o `App.tsx` também não — tudo fica dentro do `Chat`.
A única prosa que muda é uma frase do Sobre, que passaria a estar errada (§8).

O patch do Anexo está pronto e verificado (§6). As escolhas que ainda dependem de aprovação estão na §8 e na §9.

## 2. O que o repositório já oferece, e o que ele impõe

| O que existe | Onde | O que isso decide aqui |
| --- | --- | --- |
| Seções são abas, não rotas; cada seção é um componente **sem props** na tabela `TABS` | `Home.tsx` | O `Home` não tem como passar nada ao `Chat`. Um painel desenhado pelo `Home` precisaria de um canal novo para devolver a pergunta à caixa (D-HIST-9) |
| Um único container de scroll, e o `Chat` mora dentro dele | `App.tsx`, `Home.tsx` | Um painel dentro do `Chat` rola junto com as mensagens, a menos que seja `sticky` |
| A barra de composição é `position: fixed` e recua a largura da barra lateral | `Chat.tsx`, `shared/layout.ts` | O que entrar à direita tem de fazer a barra recuar também |
| Preferências no navegador: `auli.entity`, `auli.questionType`, `auli.sidebar`, sempre com `try/catch` e "sem persistência, vale nesta sessão" | `EntityContext.tsx`, `useQuestionType.ts`, `App.tsx` | Mesmo prefixo e mesma tolerância para as chaves novas |
| Barra lateral esquerda: coluna a partir de `md` com a escolha salva; abaixo, drawer feito à mão, com Esc devolvendo o foco | `App.tsx`, `Home.tsx` | O painel da direita segue o mesmo par, um degrau acima (`xl`) |
| O exemplo do estado inicial vai para a caixa **sem enviar**; o balão do usuário tem "Reutilizar a pergunta" | `EstadoInicial.tsx` (D-UI-10), `UserMessage.tsx` | Clicar numa pergunta guardada faz o mesmo: preenche, não envia |
| O registro de auditoria do servidor já grava cada pergunta, sem ligá-la a uma pessoa | `about.md`, D-LOG-\* | Nada disso muda. A lista nova não vai ao servidor |
| "O navegador guarda apenas a sua preferência de estado" | `public/about.md`, Privacidade | Já está impreciso (o navegador guarda também o tipo de consulta e a barra lateral) e ficaria falso |

## 3. As decisões

| | O quê | Por quê |
| --- | --- | --- |
| **D-HIST-1** | A lista mora no `localStorage`, uma chave por entidade: `auli.historico.<id>` | Sem login não há outro lugar. Por entidade porque a pergunta feita no RS não serve em SP |
| **D-HIST-2** | Guarda a pergunta, o tipo de consulta, a data do último envio e a estrela. **Não** guarda a resposta nem o `log_id` | A resposta é derivada do acervo, que muda a cada coleta; reenviar a refaz. Abrir o log a partir da lista fica fora de escopo (§10) |
| **D-HIST-3** | Registra no **envio**, não na chegada | A pergunta que deu erro ou estourou o tempo é a que a pessoa vai querer reenviar |
| **D-HIST-4** | Pergunta repetida no mesmo tipo não duplica: sobe ao topo com a data nova e mantém a estrela. A mesma frase em outro tipo é outra entrada | Caixa e espaços não fazem outra pergunta; outra coleção faz outra consulta |
| **D-HIST-5** | Teto de 50 recentes. Favorita nunca é descartada sozinha. O teto só é aplicado ao registrar | 50 perguntas pesam poucas dezenas de kB, num limite de cerca de 5 MB. Tirar a estrela não pode fazer a pergunta sumir na hora |
| **D-HIST-6** | Clicar numa pergunta a devolve à caixa **com o tipo em que foi feita**, sem enviar | Sem o tipo, a pergunta de Pareceres iria para Serviços + FAQs. Sem enviar pelo critério da D-UI-10: quem pergunta ainda pode ajustar |
| **D-HIST-7** | O painel existe só no Chat. A partir de `xl` (1280px) é uma coluna de 288px, aberta por padrão, com a escolha salva em `auli.historicoPainel`. Abaixo disso é um drawer vindo da direita | A 1280px sobram 768px para a coluna de leitura de 760px; a 1024px sobrariam 512px (§4.4) |
| **D-HIST-8** | Um botão "Minhas perguntas" na linha acima da caixa de mensagem, em toda largura | O botão flutuante no canto, testado antes, cobria o balão da pergunta no celular (§5) |
| **D-HIST-9** | Tudo dentro do `Chat`; o painel é `sticky` no container de scroll. `Home` e `App` não mudam | Sem canal novo entre `Home` e `Chat`. Em `fixed`, o painel cobriria a barra de rolagem do conteúdo |
| **D-HIST-10** | `useHistorico` é a única porta para o armazenamento | Na Etapa 2 troca-se esse arquivo; quem usa o hook não sabe onde a lista mora |
| **D-HIST-11** | A frase do Sobre é corrigida no mesmo patch; o painel diz "Guardadas só neste navegador" e oferece remover uma pergunta e limpar as recentes | A pergunta pode trazer dado de contribuinte, e em computador compartilhado outra pessoa a vê |

## 4. O desenho

### 4.1 O dado

```ts
export interface ItemHistorico {
  id: string;
  pergunta: string;
  tipo: QuestionType; // "1" | "2" | "3" | "4" — o contrato com o backend
  em: number;         // epoch em ms do último envio
  favorita: boolean;
}
```

Gravado como um objeto versionado, para o formato poder mudar sem quebrar quem já tem lista:

```json
{ "v": 1, "itens": [ { "id": "…", "pergunta": "Como pagar o IPVA?", "tipo": "1", "em": 1794300000000, "favorita": true } ] }
```

A lista fica sempre da mais recente para a mais antiga. "Favoritas" e "Recentes" são dois filtros da
mesma lista, não duas listas. A leitura descarta o que não tiver a forma esperada (texto que não é
JSON, versão desconhecida, item com tipo inválido) em vez de derrubar o chat: o `localStorage` é
texto que qualquer pessoa edita.

As operações são funções puras em `utils/historico.ts`. A que tem regra é `registrar` (aqui sem os tipos dos parâmetros; o código inteiro está no Anexo):

```ts
export function registrar(itens, pergunta, tipo, em, id): ItemHistorico[] {
  const chave = normalizar(pergunta);
  const antiga = itens.find((i) => i.tipo === tipo && normalizar(i.pergunta) === chave);
  const nova = {
    id: antiga?.id ?? id,
    pergunta: pergunta.trim(),
    tipo,
    em,
    favorita: antiga?.favorita ?? false,
  };
  let recentes = 0;
  return [nova, ...itens.filter((i) => i !== antiga)].filter(
    (i) => i.favorita || ++recentes <= MAX_RECENTES,
  );
}
```

### 4.2 O armazenamento

`utils/useHistorico.ts` usa `useSyncExternalStore` sobre o `localStorage`, e não o par `useState` +
efeito das outras preferências. O motivo são duas abas abertas: com `useState`, cada aba grava a sua
cópia e a última apaga o que a outra registrou. Aqui, toda gravação **relê** a lista antes de
alterá-la, e o evento `storage` avisa as outras abas.

```ts
export function useHistorico(entityId: string) {
  const chave = PREFIXO + entityId; // "auli.historico.rs"
  const itens = useSyncExternalStore(assinar, () => ler(chave));

  return {
    itens,
    registrar: (pergunta: string, tipo: QuestionType) =>
      gravar(chave, historico.registrar(ler(chave), pergunta, tipo, Date.now(), crypto.randomUUID())),
    alternarFavorita: (id: string) => gravar(chave, historico.alternarFavorita(ler(chave), id)),
    remover: (id: string) => gravar(chave, historico.remover(ler(chave), id)),
    limparRecentes: () => gravar(chave, historico.limparRecentes(ler(chave))),
  };
}
```

Dois comportamentos de borda:

- **`localStorage` bloqueado pelo navegador:** a lista vale só na sessão, em memória.
- **`localStorage` cheio (a leitura funciona, a gravação falha):** a lista volta ao que está gravado.
  A pergunta nova não entra, e a tela não mostra algo que não foi guardado.

No `Chat`, o registro é uma linha, antes da chamada ao servidor:

```ts
isSubmittingRef.current = true;
historico.registrar(trimmedPrompt, questionType);
```

### 4.3 O painel e o layout

```text
a partir de 1280px (xl)                              abaixo de 1280px
┌────────────── cabeçalho, 52px ───────────────┐     ┌──── cabeçalho ────┐
│ seções │ chat (rola)          │ Minhas       │     │ chat (rola)       │
│ 224px  │                      │ perguntas    │     │       ┌───────────┤
│        │                      │ 288px        │     │       │ drawer,   │
│        │                      │ parado       │     │       │ vem da    │
│        │ ▯ Minhas perguntas   │              │     │       │ direita   │
│        │ [ caixa de mensagem ]│              │     │ ▯ Minhas perguntas│
└────────┴──────────────────────┴──────────────┘     └───────────────────┘
```

A raiz do `Chat` deixa de ser uma coluna e vira uma linha: a conversa à esquerda, o painel à direita. Em esqueleto:

```tsx
<Flex w="100%" flex={1} bg="bg.app">
  <Flex flexDirection="column" flex={1} minW={0} pb={`${composerHeight + 16}px`}>
    {/* EstadoInicial ou Messages, como hoje */}
  </Flex>

  {painelAberto && (
    <Box as="aside" aria-label="Minhas perguntas" hideBelow="xl"
         position="sticky" top={0} alignSelf="flex-start" flexShrink={0}
         w={HISTORICO_WIDTH} h={`calc(100dvh - ${HEADER_HEIGHT})`} overflowY="auto" zIndex={21}>
      {painel(() => setPainelAberto(false))}
    </Box>
  )}

  {drawerAberto && (
    <Box hideFrom="xl" position="fixed" inset={0} zIndex={110}>
      {/* overlay + o MESMO painel, à direita — feito à mão, como o drawer de seções */}
    </Box>
  )}

  {/* a barra de composição, fixed como hoje, agora recuando também à direita: */}
  <Box position="fixed" right={painelAberto ? { base: 0, xl: HISTORICO_WIDTH } : 0} … />
</Flex>
```

Três pontos do layout:

- **`sticky`, e não `fixed`.** O painel fica dentro do fluxo, então o chat encolhe em vez de ser
  coberto, e a barra de rolagem do conteúdo continua à vista. A altura é a da área que rola, que
  nessa largura é a tela menos o cabeçalho. Daí o `HEADER_HEIGHT` novo em `shared/layout.ts`, que o
  `AppHeader` passa a ler para os dois não divergirem.
- **`zIndex` 21, acima da barra de composição (20).** A barra é `fixed` e não desconta a barra de
  rolagem, então avança alguns pixels sobre a borda do painel; o painel fica por cima.
- **Dois estados, como na barra lateral esquerda.** `painelAberto` vale a partir de `xl` e fica
  salvo. `drawerAberto` vale abaixo e não deixa rastro. Quem escolhe o modo é o CSS
  (`hideBelow`/`hideFrom`), não o JavaScript.

O conteúdo do painel é `Historico.tsx`, que só desenha a lista e os botões: Favoritas em cima,
Recentes embaixo, cada linha com a pergunta (duas linhas no máximo), o tipo, a data, a estrela e o
"×". No rodapé, "Limpar recentes" e o aviso "Guardadas só neste navegador". O `Chat` o monta nos
dois lugares, a coluna e o drawer.

### 4.4 Quanto cabe

A coluna de leitura do chat tem 760px. O que sobra para ela, com o painel de 288px aberto:

| Largura da janela | Barra lateral aberta (224px) | Barra lateral recolhida (64px) |
| --- | --- | --- |
| 1024px (`lg`) | 512px | 672px |
| 1280px (`xl`) | 768px | 928px |
| 1366px (notebook comum) | 854px | 1014px |

Por isso o painel só vira coluna a partir de `xl`. Um notebook de 1366px com zoom de 125% tem 1093px
de largura útil e cai no drawer.

## 5. Caminhos descartados

| Caminho | Por que não |
| --- | --- |
| Painel desenhado pelo `Home`, como terceira coluna | As seções não recebem props, então clicar numa pergunta precisaria de um contexto ou barramento novo só para chegar à caixa do `Chat` |
| Painel `position: fixed` à direita | Cobre a barra de rolagem da área de conteúdo, que vai até a borda da janela |
| Botão flutuante no canto de cima do chat | Implementado e capturado: no celular ficava sobre o canto do balão da pergunta. Trocado pelo botão na linha acima da caixa |
| Botão no cabeçalho, espelhando o da barra lateral | O cabeçalho não sabe qual seção está aberta, e o painel só existe no Chat |
| `useState` + efeito, como em `auli.sidebar` | Duas abas se apagam: cada uma grava a própria cópia da lista |
| Guardar a resposta junto | Envelhece com o acervo e multiplica o tamanho gravado; reenviar a pergunta custa uma consulta |

## 6. O que já foi verificado fora daqui

O patch foi aplicado sobre o `242dec4`. Nesse estado: `tsc --noEmit` limpo, `npm run lint` com 0
avisos, `npm run build` ok, `vitest run` com **305 testes** (25 novos), `check-sem-navi.sh` ok. A
árvore resultante é `6bd5d6d91651af6382bad32133b94e6e1db715e9`, e um `git am` do patch numa árvore
limpa do `242dec4` reproduz esse mesmo hash.

Um teste existente foi ajustado: "depois do primeiro envio, não aparece saudação acima da pergunta"
procurava a pergunta com `findByText`, e ela agora aparece duas vezes (no balão e no painel). Passou
a contar as ocorrências fora do painel.

Verificação por mutação: quatro defeitos foram recolocados, um por vez, e cada um derrubou um teste.

| Defeito recolocado | Teste que caiu |
| --- | --- |
| O envio não registra a pergunta | "a pergunta enviada entra no painel e fica gravada no navegador, por entidade" |
| Clicar não restaura o tipo de consulta | "devolve a pergunta com o tipo de consulta em que ela foi feita" |
| `registrar` não deduplica | "pergunta repetida sobe ao topo sem duplicar, mantendo o id e a estrela" |
| A gravação parte da lista da tela, sem reler | "o que outra aba gravou aparece aqui e não é apagado pela próxima gravação" |

Conferido no Chrome headless, com a API simulada (nenhuma pergunta foi à produção):

- 1440×900: painel aberto com favoritas e recentes, conversa rolando, painel fechado, modo escuro e
  lista vazia. Medido: o painel ocupa de x=1152 a 1440 e de y=52 a 900 e não se move enquanto a
  conversa rola; a barra de composição termina em x=1152; fechado o painel, ela volta a `right: 0`
  e `auli.historicoPainel` fica `recolhido`.
- 1100×800: o botão abre o drawer pela direita.
- 390×800: o botão divide a linha com o "Recolher"; o drawer abre; usar uma pergunta preenche a
  caixa e fecha o drawer.

**Não verificado:**

- Windows com barra de rolagem clássica e Firefox. O Chrome headless usa barra sobreposta, então a
  faixa em que a barra de composição encosta no painel não foi observada com a barra visível. Com a
  lista longa, o painel ganha rolagem própria, ao lado da rolagem do conteúdo.
- Safari e iOS, leitor de tela e navegação só por teclado dentro do painel.
- O servidor real: as respostas foram simuladas.
- Janela anônima e `localStorage` bloqueado, fora do teste automatizado que simula o bloqueio.

## 7. Passo a passo

### Passo 1 — Extrair o patch

O patch está no Anexo, num bloco cercado por `~~~~~patch`. Extraia com o script (ajuste o caminho) e
confira o hash. Se divergir, pare e me avise.

```bash
mkdir -p /tmp/dhist && python3 - <<'PY'
import re, pathlib, hashlib
doc = pathlib.Path("Estudo-sidebar-direita.md").read_text(encoding="utf-8")
for nome, corpo in re.findall(r"^~~~~~patch (\S+)\n(.*?)^~~~~~$", doc, re.S | re.M):
    pathlib.Path("/tmp/dhist", nome).write_text(corpo, encoding="utf-8")
    print(hashlib.sha256(corpo.encode("utf-8")).hexdigest(), nome)
PY
```

Esperado:

```text
a4921d43395185164e5e9711a4332f8f45a068f7b05cebd1b850e0c8cc002cd8  0001-minhas-perguntas.patch
```

### Passo 2 — Aplicar numa branch

```bash
cd ~/Desktop/auli
git status --short                 # tem de estar limpo; se não estiver, pare e me pergunte
git switch -c minhas-perguntas
git am -3 /tmp/dhist/0001-minhas-perguntas.patch
git rev-parse HEAD^{tree}          # se a base era o 242dec4: 6bd5d6d91651af6382bad32133b94e6e1db715e9
```

Se o `main` andou e o `git am` parar em conflito, resolva **preservando as duas intenções**. O ponto
provável é o `Chat.tsx` (a raiz do componente e a linha acima da caixa de mensagem). Na dúvida sobre
a intenção, pare e me pergunte. Não use `git am --skip`.

### Passo 3 — Verificar

```bash
cd auli-frontend
npm ci && npm run typecheck && npm run lint && npm test && npm run build
cd .. && bash scripts/tools/check-sem-navi.sh
```

### Passo 4 — Conferir no navegador (portão humano)

Suba o frontend (`npm run dev`) e confira:

1. Janela de 1280px ou mais, primeira visita: o painel aparece aberto, com o texto de lista vazia.
   Enviar uma pergunta a põe em Recentes na hora.
2. A estrela leva a pergunta para Favoritas. Recarregar a página mantém a lista e a estrela.
3. Clicar numa pergunta feita em outro tipo: a caixa é preenchida, o seletor troca de tipo e nada é
   enviado.
4. Resposta longa: o painel fica parado enquanto a conversa rola, e a caixa de mensagem não invade o
   painel. **Conferir no Windows**, com a barra de rolagem visível (§6).
5. Fechar pelo ícone do painel e pelo botão "Minhas perguntas". Recarregar mantém fechado.
6. Janela estreita e celular: o botão abre o drawer; Esc e o clique fora fecham; usar uma pergunta
   fecha o drawer e põe o foco na caixa.
7. Modo escuro.
8. Duas abas no mesmo estado: perguntar numa e ver a pergunta aparecer na outra.
9. Trocar de estado: a lista é outra.
10. "Limpar recentes" preserva as favoritas; o "×" remove uma pergunta só.

### Passo 5 — Registrar e fechar

- Acrescente a tabela da §3 ao `docs/auli_code.md` como §4.3.2 (decisões D-HIST-\*), e os arquivos
  novos à lista da §4.3, no mesmo PR.
- `auli-frontend/DESCRIPTION.md` diz que a entidade selecionada é "the one piece of state that
  persists". Já não era verdade; corrija a frase no mesmo PR.
- Bump de versão do frontend num commit `chore(frontend): v0.1.69` separado, como é a convenção.
- **Não** faça push nem merge sem a minha confirmação.

## 8. Portões humanos — o que eu preciso aprovar

- **A frase do Sobre** (`public/about.md`, Privacidade). O patch troca *"O navegador guarda apenas a
  sua preferência de estado, que nunca é enviada ao servidor."* por:

  > O navegador guarda as suas preferências (o estado, o tipo de consulta e a posição dos painéis) e
  > a lista **Minhas perguntas** do chat: as perguntas que você enviou e as que marcou como
  > favoritas. Essa lista fica só neste navegador — o servidor recebe cada pergunta para
  > respondê-la, como sempre, mas não recebe a lista nem sabe de quem ela é. Você pode apagar cada
  > pergunta, ou todas as recentes, no próprio painel. Em computador compartilhado, quem usar o
  > mesmo navegador verá essas perguntas.

- **O nome** "Minhas perguntas" e os rótulos "Favoritas", "Recentes" e "Limpar recentes".
- **O painel aberto na primeira visita**, nas telas largas. É a regra da barra lateral esquerda; a
  alternativa é começar fechado e a pessoa descobri-lo pelo botão.
- **Computador compartilhado.** Quem atende em máquina de uso comum deixa as perguntas à vista de
  quem usar o mesmo navegador. O painel avisa e deixa apagar, mas não há opção de não guardar (§10).

## 9. Escolhas baratas de trocar

Cada uma é uma linha ou uma constante, e trocar qualquer uma antes do Passo 2 não mexe no resto.

| Escolha | Valor no patch | Onde |
| --- | --- | --- |
| Teto de recentes | 50 | `MAX_RECENTES`, `utils/historico.ts` |
| Largura do painel | 288px | `HISTORICO_WIDTH`, `shared/layout.ts` |
| A partir de que largura vira coluna | `xl` (1280px) | três props em `Chat.tsx` (`hideBelow`, `hideFrom`, `right`) |
| Painel na primeira visita | aberto | `lerPainelAberto`, `Chat.tsx` |
| "Limpar recentes" | sem confirmação (as favoritas ficam) | `Historico.tsx` |
| Cor da estrela marcada | `accent` (o azul da marca) | `Historico.tsx`; um amarelo pede token novo em `theme/system.js` |
| Identidade da pergunta repetida | mesmo tipo + mesmo texto, sem contar caixa e espaços | `registrar`, `utils/historico.ts` |

## 10. Fora de escopo, e o que a Etapa 2 reaproveita

Fica para depois:

- Busca dentro da lista.
- Uma opção de não guardar as perguntas (pausar o histórico).
- Abrir o log da consulta a partir da lista, ou guardar a resposta.
- O painel nas outras seções, e exportar a lista.

Na Etapa 2, com login:

- O painel, o botão, o layout e os testes de interface ficam como estão. Muda `utils/useHistorico.ts`,
  que passa a ler e gravar pela API com a mesma assinatura (D-HIST-10).
- No primeiro login há uma lista local para tratar. O `registrar` já deduplica, então importar é
  aplicar a lista local sobre a do servidor.
- O servidor passa a guardar perguntas ligadas a uma pessoa. Isso pede duas decisões que a Etapa 1
  não tem: texto original ou anonimizado, e por quanto tempo. A frase do Sobre muda outra vez.

---

## Anexo — o patch

Um arquivo `git format-patch` íntegro. Extraia com o script do Passo 1.

~~~~~patch 0001-minhas-perguntas.patch
From 45ade0d6e81f497002763c1e9ba6d0d74bb51965 Mon Sep 17 00:00:00 2001
From: oxschellen <oxschellen@gmail.com>
Date: Tue, 10 Nov 2026 08:38:33 -0300
Subject: [PATCH] =?UTF-8?q?feat(frontend):=20"Minhas=20perguntas"=20?=
 =?UTF-8?q?=E2=80=94=20as=20perguntas=20enviadas=20ficam=20guardadas=20no?=
 =?UTF-8?q?=20navegador,=20com=20favoritas,=20num=20painel=20=C3=A0=20dire?=
 =?UTF-8?q?ita=20do=20chat=20(D-HIST-1..11)?=
MIME-Version: 1.0
Content-Type: text/plain; charset=UTF-8
Content-Transfer-Encoding: 8bit

Etapa 1: sem login e sem servidor. A lista mora no localStorage, uma chave por entidade
(auli.historico.<id>), e o hook useHistorico é a única porta para ela.

- utils/historico.ts: registrar (sem duplicar, teto de 50 recentes), favoritar, remover, limpar,
  e a leitura tolerante do que está gravado
- utils/useHistorico.ts: useSyncExternalStore sobre o localStorage; relê antes de gravar e ouve
  o evento storage, para duas abas não se apagarem
- Historico.tsx: o painel — Favoritas em cima, Recentes embaixo
- Chat.tsx: registra no envio; coluna sticky à direita a partir de xl, drawer abaixo; o botão
  mora na linha acima da caixa; a barra de composição recua também à direita
- shared/layout.ts: HEADER_HEIGHT e HISTORICO_WIDTH
- public/about.md: a frase de privacidade passa a dizer o que o navegador guarda
---
 auli-frontend/public/about.md                 |   2 +-
 auli-frontend/src/pages/chat/Chat.test.tsx    | 110 +++++++++-
 auli-frontend/src/pages/chat/Chat.tsx         | 188 ++++++++++++++++--
 auli-frontend/src/pages/chat/Historico.tsx    | 155 +++++++++++++++
 .../src/pages/chat/utils/historico.test.ts    |  95 +++++++++
 .../src/pages/chat/utils/historico.ts         | 101 ++++++++++
 .../src/pages/chat/utils/useHistorico.test.ts |  57 ++++++
 .../src/pages/chat/utils/useHistorico.ts      |  73 +++++++
 .../src/pages/chat/utils/useQuestionType.ts   |   2 +-
 auli-frontend/src/shared/AppHeader.tsx        |   3 +-
 auli-frontend/src/shared/layout.ts            |   8 +
 11 files changed, 776 insertions(+), 18 deletions(-)
 create mode 100644 auli-frontend/src/pages/chat/Historico.tsx
 create mode 100644 auli-frontend/src/pages/chat/utils/historico.test.ts
 create mode 100644 auli-frontend/src/pages/chat/utils/historico.ts
 create mode 100644 auli-frontend/src/pages/chat/utils/useHistorico.test.ts
 create mode 100644 auli-frontend/src/pages/chat/utils/useHistorico.ts

diff --git a/auli-frontend/public/about.md b/auli-frontend/public/about.md
index 8dcc20e..c65142b 100644
--- a/auli-frontend/public/about.md
+++ b/auli-frontend/public/about.md
@@ -40,7 +40,7 @@ O conector expõe hoje o acervo de pareceres e respostas a consultas tributária
 
 ## Privacidade
 
-**A Auli não sabe quem você é.** Não há cadastro, login nem cookie de rastreamento, e o seu endereço IP não é gravado — nada liga uma consulta a uma pessoa, a um local ou a um histórico individual. O navegador guarda apenas a sua preferência de estado, que nunca é enviada ao servidor.
+**A Auli não sabe quem você é.** Não há cadastro, login nem cookie de rastreamento, e o seu endereço IP não é gravado — nada liga uma consulta a uma pessoa, a um local ou a um histórico individual. O navegador guarda as suas preferências (o estado, o tipo de consulta e a posição dos painéis) e a lista **Minhas perguntas** do chat: as perguntas que você enviou e as que marcou como favoritas. Essa lista fica só neste navegador — o servidor recebe cada pergunta para respondê-la, como sempre, mas não recebe a lista nem sabe de quem ela é. Você pode apagar cada pergunta, ou todas as recentes, no próprio painel. Em computador compartilhado, quem usar o mesmo navegador verá essas perguntas.
 
 **O que fica registrado:** a pergunta, o horário e os documentos consultados. Serve para auditar a qualidade das respostas e o mascaramento automático descrito abaixo. Como a pergunta é guardada exatamente como você a escreveu, **dados pessoais que você mesmo digitar ficam gravados** — se incluir um CPF ou CNPJ, ele fica no registro. Evite informar dados que não sejam necessários à dúvida.
 
diff --git a/auli-frontend/src/pages/chat/Chat.test.tsx b/auli-frontend/src/pages/chat/Chat.test.tsx
index eaaad4a..12659be 100644
--- a/auli-frontend/src/pages/chat/Chat.test.tsx
+++ b/auli-frontend/src/pages/chat/Chat.test.tsx
@@ -1,6 +1,6 @@
 // @vitest-environment jsdom
 import { beforeEach, describe, expect, it, vi } from "vitest";
-import { fireEvent, screen, waitFor } from "@testing-library/react";
+import { fireEvent, screen, waitFor, within } from "@testing-library/react";
 import { renderWithProvider } from "../../test/render";
 import { EntityProvider } from "../../shared/EntityContext";
 import { Chat } from "./Chat";
@@ -145,7 +145,113 @@ describe("a conversa começa vazia", () => {
     const pergunta = "Qual o prazo para pagar o ICMS da empresa?";
     fireEvent.change(screen.getByLabelText("Sua pergunta"), { target: { value: pergunta } });
     fireEvent.click(screen.getByLabelText("Enviar pesquisa"));
-    expect(await screen.findByText(pergunta)).toBeInTheDocument();
+    // A pergunta aparece no balão e, com o painel "Minhas perguntas", também na lista dele.
+    const ocorrencias = await screen.findAllByText(pergunta);
+    expect(ocorrencias.filter((el) => !el.closest("aside"))).toHaveLength(1);
     expect(screen.queryByText("Olá! Como posso ajudar?")).toBeNull();
   });
 });
+
+/**
+ * "Minhas perguntas" (D-HIST-*). Em jsdom o `hideBelow`/`hideFrom` não recorta nada, então o painel
+ * fixo e os dois botões renderizam juntos — o que se confere aqui é o comportamento, não o modo.
+ */
+describe("minhas perguntas", () => {
+  const CHAVE = "auli.historico.rs";
+  const pergunta = "Qual o prazo para pagar o ICMS da empresa?";
+  const enviar = (texto: string) => {
+    fireEvent.change(screen.getByLabelText("Sua pergunta"), { target: { value: texto } });
+    fireEvent.click(screen.getByLabelText("Enviar pesquisa"));
+  };
+  const painel = () => within(screen.getByRole("complementary", { name: "Minhas perguntas" }));
+
+  beforeEach(() => {
+    localStorage.removeItem(CHAVE);
+    localStorage.removeItem("auli.historicoPainel");
+    localStorage.setItem("auli.questionType", "1");
+  });
+
+  it("a pergunta enviada entra no painel e fica gravada no navegador, por entidade", async () => {
+    montar();
+    enviar(pergunta);
+    expect(await painel().findByText(pergunta)).toBeInTheDocument();
+    const gravado = JSON.parse(localStorage.getItem(CHAVE)!);
+    expect(gravado.itens).toHaveLength(1);
+    expect(gravado.itens[0]).toMatchObject({ pergunta, tipo: "1", favorita: false });
+  });
+
+  it("clicar numa pergunta do painel a devolve à caixa, sem enviar", () => {
+    localStorage.setItem(
+      CHAVE,
+      JSON.stringify({ v: 1, itens: [{ id: "a", pergunta, tipo: "1", em: 1, favorita: false }] }),
+    );
+    montar();
+    fireEvent.click(painel().getByText(pergunta));
+    expect(screen.getByLabelText("Sua pergunta")).toHaveValue(pergunta);
+    // Não enviou: a conversa segue vazia, com a apresentação do tipo.
+    expect(screen.getByRole("heading", { name: "Como posso ajudar?" })).toBeInTheDocument();
+  });
+
+  it("devolve a pergunta com o tipo de consulta em que ela foi feita", () => {
+    localStorage.setItem(
+      CHAVE,
+      JSON.stringify({ v: 1, itens: [{ id: "a", pergunta, tipo: "2", em: 1, favorita: false }] }),
+    );
+    montar();
+    expect(screen.getByRole("button", { name: /^Tipo de consulta: Serviços \+ FAQs/ })).toBeInTheDocument();
+    fireEvent.click(painel().getByText(pergunta));
+    expect(screen.getByRole("button", { name: /^Tipo de consulta: Pareceres/ })).toBeInTheDocument();
+  });
+
+  it("a estrela move a pergunta para Favoritas e a marca fica gravada", () => {
+    localStorage.setItem(
+      CHAVE,
+      JSON.stringify({ v: 1, itens: [{ id: "a", pergunta, tipo: "1", em: 1, favorita: false }] }),
+    );
+    montar();
+    expect(painel().queryByRole("region", { name: "Favoritas" })).toBeNull();
+    fireEvent.click(painel().getByRole("button", { name: "Marcar como favorita" }));
+    expect(painel().getByRole("region", { name: "Favoritas" })).toHaveTextContent(pergunta);
+    expect(JSON.parse(localStorage.getItem(CHAVE)!).itens[0].favorita).toBe(true);
+  });
+
+  it("limpar recentes preserva as favoritas, e remover tira uma pergunta só", () => {
+    localStorage.setItem(
+      CHAVE,
+      JSON.stringify({
+        v: 1,
+        itens: [
+          { id: "a", pergunta, tipo: "1", em: 2, favorita: false },
+          { id: "b", pergunta: "Como emitir a certidão de situação fiscal?", tipo: "1", em: 1, favorita: true },
+        ],
+      }),
+    );
+    montar();
+    fireEvent.click(painel().getByRole("button", { name: "Limpar recentes" }));
+    expect(painel().queryByText(pergunta)).toBeNull();
+    expect(painel().getByText("Como emitir a certidão de situação fiscal?")).toBeInTheDocument();
+    fireEvent.click(painel().getByRole("button", { name: "Remover esta pergunta" }));
+    expect(painel().getByText(/As perguntas que você enviar aparecem aqui/)).toBeInTheDocument();
+    expect(JSON.parse(localStorage.getItem(CHAVE)!).itens).toEqual([]);
+  });
+
+  it("fechar o painel fica salvo, e o botão o traz de volta", () => {
+    montar();
+    fireEvent.click(painel().getByRole("button", { name: "Fechar minhas perguntas" }));
+    expect(screen.queryByRole("complementary", { name: "Minhas perguntas" })).toBeNull();
+    expect(localStorage.getItem("auli.historicoPainel")).toBe("recolhido");
+    fireEvent.click(screen.getByRole("button", { name: "Mostrar minhas perguntas" }));
+    expect(screen.getByRole("complementary", { name: "Minhas perguntas" })).toBeInTheDocument();
+  });
+
+  it("o drawer abre pelo botão, e Esc o fecha devolvendo o foco", () => {
+    localStorage.setItem("auli.historicoPainel", "recolhido");
+    montar();
+    const botao = screen.getByRole("button", { name: "Abrir minhas perguntas" });
+    fireEvent.click(botao);
+    expect(screen.getByRole("complementary", { name: "Minhas perguntas" })).toBeInTheDocument();
+    fireEvent.keyDown(document, { key: "Escape" });
+    expect(screen.queryByRole("complementary", { name: "Minhas perguntas" })).toBeNull();
+    expect(document.activeElement).toBe(botao);
+  });
+});
diff --git a/auli-frontend/src/pages/chat/Chat.tsx b/auli-frontend/src/pages/chat/Chat.tsx
index d802c1c..71bab1b 100644
--- a/auli-frontend/src/pages/chat/Chat.tsx
+++ b/auli-frontend/src/pages/chat/Chat.tsx
@@ -1,5 +1,6 @@
 import { Box, Flex, Text, chakra } from "@chakra-ui/react";
 import { MdEdit, MdKeyboardArrowDown, MdKeyboardArrowUp } from "react-icons/md";
+import { LuPanelRight } from "react-icons/lu";
 import { useRef, useEffect, useState } from "react";
 import { useIsKeyboardVisible } from "./utils/useIsKeyboardVisible";
 import { Messages } from "./Messages";
@@ -11,13 +12,45 @@ import { callServerAPI } from "./utils/callServerAPI";
 import { isPromptValid } from "./utils/prompt";
 import { SelectQuestionType } from "./SelectQuestionType";
 import { EstadoInicial } from "./EstadoInicial";
+import { Historico } from "./Historico";
+import { useHistorico } from "./utils/useHistorico";
+import type { ItemHistorico } from "./utils/historico";
 import { TIPOS } from "./utils/tipos";
 import { useSelectedEntity } from "../../shared/EntityContext";
-import { SIDEBAR_VAR, SIDEBAR_WIDTH } from "../../shared/layout";
+import { HEADER_HEIGHT, HISTORICO_WIDTH, SIDEBAR_VAR, SIDEBAR_WIDTH } from "../../shared/layout";
 // O `API_URL` saiu daqui para `utils/api.ts` quando o modal do log passou a precisar da MESMA
 // base — ver `logUrl`, que é derivada dele justamente para não divergir.
 import { API_URL } from "./utils/api";
 
+const PAINEL_KEY = "auli.historicoPainel";
+
+/** O painel começa como foi deixado; sem preferência salva (ou sem localStorage), aberto — a mesma
+ *  regra da barra lateral esquerda (`App.tsx`). */
+function lerPainelAberto(): boolean {
+  try {
+    return localStorage.getItem(PAINEL_KEY) !== "recolhido";
+  } catch {
+    return true;
+  }
+}
+
+/** O botão de "Minhas perguntas", na linha acima da caixa de mensagem. São dois no DOM — um por
+ *  modo, escolhidos por CSS (`hideBelow`/`hideFrom`), como os dois botões da barra lateral
+ *  esquerda — com a mesma cara, que é a do "Recolher" vizinho. */
+const BOTAO_HISTORICO = {
+  type: "button",
+  display: "flex",
+  alignItems: "center",
+  gap: 1,
+  px: 2,
+  py: 1,
+  fontSize: "12px",
+  color: "fg.muted",
+  borderRadius: "full",
+  cursor: "pointer",
+  _hover: { bg: "bg.overlay" },
+} as const;
+
 export const Chat = () => {
   const entity = useSelectedEntity();
   const { prompt, setPrompt, updatePrompt } = usePrompt();
@@ -28,6 +61,33 @@ export const Chat = () => {
   const messagesEndRef = useRef<HTMLDivElement>(null);
   const isSubmittingRef = useRef(false);
 
+  const historico = useHistorico(entity.id);
+  // Dois estados, como na barra lateral esquerda: a partir de `xl` o painel é uma coluna e a
+  // escolha fica salva; abaixo dele é um drawer, que abre e fecha sem deixar rastro.
+  const [painelAberto, setPainelAberto] = useState(lerPainelAberto);
+  const [drawerAberto, setDrawerAberto] = useState(false);
+  const botaoDrawerRef = useRef<HTMLButtonElement>(null);
+
+  useEffect(() => {
+    try {
+      localStorage.setItem(PAINEL_KEY, painelAberto ? "aberto" : "recolhido");
+    } catch {
+      // Sem persistência, o botão segue valendo nesta sessão.
+    }
+  }, [painelAberto]);
+
+  // Esc fecha o drawer e devolve o foco ao botão que o abriu — o mesmo par do drawer de seções.
+  useEffect(() => {
+    if (!drawerAberto) return;
+    const aoTeclar = (e: KeyboardEvent) => {
+      if (e.key !== "Escape") return;
+      setDrawerAberto(false);
+      botaoDrawerRef.current?.focus();
+    };
+    document.addEventListener("keydown", aoTeclar);
+    return () => document.removeEventListener("keydown", aoTeclar);
+  }, [drawerAberto]);
+
   const { isKeyboardVisible: isKeyboardOpen, keyboardHeight } = useIsKeyboardVisible();
   const [loading, setLoading] = useState(false);
   // Só no celular: a caixa de mensagem ocupa ~220px da tela, e recolhê-la libera a leitura da
@@ -73,10 +133,34 @@ export const Chat = () => {
     requestAnimationFrame(() => textareaRef.current?.focus());
   };
 
+  // Devolve a pergunta à caixa COM o tipo em que foi feita (D-HIST-6): sem isso, a pergunta de
+  // Pareceres iria para Serviços + FAQs se o seletor tivesse mudado. O `updateQuestionType` já
+  // ignora o tipo que a entidade não tem. Não envia — quem pergunta ainda pode ajustar.
+  const usarDoHistorico = (item: ItemHistorico) => {
+    updateQuestionType(item.tipo);
+    setDrawerAberto(false);
+    setRecolhido(false);
+    usarExemplo(item.pergunta);
+  };
+
+  const painel = (aoFechar: () => void) => (
+    <Historico
+      itens={historico.itens}
+      onUsar={usarDoHistorico}
+      onFavoritar={historico.alternarFavorita}
+      onRemover={historico.remover}
+      onLimparRecentes={historico.limparRecentes}
+      onFechar={aoFechar}
+    />
+  );
+
   const handleCallServerAPI = async (prompt: string) => {
     const trimmedPrompt = prompt.trim();
     if (isSubmittingRef.current || loading || !isPromptValid(trimmedPrompt)) return;
     isSubmittingRef.current = true;
+    // No ENVIO, não na chegada (D-HIST-3): a pergunta que deu erro ou estourou o tempo é
+    // justamente a que a pessoa vai querer reenviar.
+    historico.registrar(trimmedPrompt, questionType);
     try {
       await callServerAPI({
         prompt: trimmedPrompt,
@@ -94,13 +178,63 @@ export const Chat = () => {
   };
 
   return (
-    <Flex flexDirection="column" w="100%" flex={1} bg="bg.app" pb={`${composerHeight + 16}px`}>
-      {vazio ? (
-        <EstadoInicial tipo={questionType} onExemplo={usarExemplo} />
-      ) : (
-        <Messages messages={messages} setPrompt={setPrompt} />
+    <Flex w="100%" flex={1} bg="bg.app">
+      <Flex flexDirection="column" flex={1} minW={0} pb={`${composerHeight + 16}px`}>
+        {vazio ? (
+          <EstadoInicial tipo={questionType} onExemplo={usarExemplo} />
+        ) : (
+          <Messages messages={messages} setPrompt={setPrompt} />
+        )}
+        <div ref={messagesEndRef} />
+      </Flex>
+
+      {/* Painel (>= xl): coluna à direita, DENTRO do fluxo — o chat encolhe em vez de ser coberto,
+          e a barra de rolagem da área de conteúdo segue à vista, à direita dele. `sticky` porque o
+          Chat mora dentro do único container de scroll (ver `Home`): sem isso o painel subiria
+          junto com as mensagens. A altura é a da área que rola, que nesta largura é a tela menos
+          o cabeçalho. O zIndex fica ACIMA da barra de composição (20): ela é `fixed` e não desconta
+          a barra de rolagem, então avança alguns pixels sobre a borda do painel. */}
+      {painelAberto && (
+        <Box
+          as="aside"
+          aria-label="Minhas perguntas"
+          hideBelow="xl"
+          position="sticky"
+          top={0}
+          alignSelf="flex-start"
+          flexShrink={0}
+          w={HISTORICO_WIDTH}
+          h={`calc(100dvh - ${HEADER_HEIGHT})`}
+          overflowY="auto"
+          bg="bg.app"
+          borderLeft="1px solid var(--chakra-colors-border)"
+          zIndex={21}
+        >
+          {painel(() => setPainelAberto(false))}
+        </Box>
+      )}
+
+      {/* Drawer (< xl): overlay + o MESMO painel, vindo da direita. Feito à mão e com o zIndex
+          acima do AppHeader, pelos motivos do drawer de seções no `Home`. */}
+      {drawerAberto && (
+        <Box hideFrom="xl" position="fixed" inset={0} zIndex={110}>
+          <Box position="absolute" inset={0} bg="blackAlpha.500" onClick={() => setDrawerAberto(false)} />
+          <Box
+            as="aside"
+            aria-label="Minhas perguntas"
+            position="absolute"
+            top={0}
+            bottom={0}
+            right={0}
+            w="min(320px, 88vw)"
+            overflowY="auto"
+            bg="bg.app"
+            borderLeft="1px solid var(--chakra-colors-border)"
+          >
+            {painel(() => setDrawerAberto(false))}
+          </Box>
+        </Box>
       )}
-      <div ref={messagesEndRef} />
 
       {/* Barra de composição. `fixed` é o que permite subi-la acima do teclado virtual — mas
           `fixed` se ancora na VIEWPORT, não na área de conteúdo, então ela precisa recuar a
@@ -120,22 +254,50 @@ export const Chat = () => {
         // device/UA check — desktops simply report 0.
         bottom={isKeyboardOpen ? `${keyboardHeight}px` : 0}
         left={{ base: 0, md: `var(${SIDEBAR_VAR}, ${SIDEBAR_WIDTH})` }}
-        right={0}
+        // Com o painel "Minhas perguntas" aberto (>= xl), recua também à direita.
+        right={painelAberto ? { base: 0, xl: HISTORICO_WIDTH } : 0}
         zIndex={20}
         // Mesma cor do fundo: a caixa é que se destaca (borda e sombra), não uma faixa branca de
         // ponta a ponta (D-UI-4).
         bg="bg.app"
       >
-        {/* Celular: o "Recolher" acima da caixa e, recolhida, a barrinha que a traz de volta. A
-            caixa segue filha DIRETA desta barra fixa (o teste da sidebar sobe por `parentElement`),
-            então quem a esconde é o próprio `Input`, pela prop `recolhido`. */}
-        <Flex display={{ base: recolhido ? "none" : "flex", md: "none" }} justify="flex-end" mx={3}>
+        {/* A linha acima da caixa, alinhada com ela: à esquerda, "Minhas perguntas" (em toda
+            largura); à direita, só no celular, o "Recolher" — e, recolhida, a barrinha que a traz
+            de volta. A caixa segue filha DIRETA desta barra fixa (o teste da sidebar sobe por
+            `parentElement`), então quem a esconde é o próprio `Input`, pela prop `recolhido`. */}
+        <Flex
+          display={{ base: recolhido ? "none" : "flex", md: "flex" }}
+          justify="space-between"
+          maxW="760px"
+          mx={{ base: 3, md: "auto" }}
+        >
+          <chakra.button
+            {...BOTAO_HISTORICO}
+            hideBelow="xl"
+            aria-label={painelAberto ? "Recolher minhas perguntas" : "Mostrar minhas perguntas"}
+            aria-expanded={painelAberto}
+            onClick={() => setPainelAberto((v) => !v)}
+          >
+            <LuPanelRight size={14} aria-hidden="true" />
+            Minhas perguntas
+          </chakra.button>
+          <chakra.button
+            {...BOTAO_HISTORICO}
+            ref={botaoDrawerRef}
+            hideFrom="xl"
+            aria-label="Abrir minhas perguntas"
+            aria-expanded={drawerAberto}
+            onClick={() => setDrawerAberto(true)}
+          >
+            <LuPanelRight size={14} aria-hidden="true" />
+            Minhas perguntas
+          </chakra.button>
           <chakra.button
             type="button"
             aria-label="Recolher a caixa de mensagem"
             aria-expanded={true}
             onClick={() => setRecolhido(true)}
-            display="flex"
+            display={{ base: "flex", md: "none" }}
             alignItems="center"
             gap={1}
             px={2}
diff --git a/auli-frontend/src/pages/chat/Historico.tsx b/auli-frontend/src/pages/chat/Historico.tsx
new file mode 100644
index 0000000..d20bccc
--- /dev/null
+++ b/auli-frontend/src/pages/chat/Historico.tsx
@@ -0,0 +1,155 @@
+import { Box, Flex, Text, chakra } from "@chakra-ui/react";
+import { MdClose, MdStar, MdStarBorder } from "react-icons/md";
+import { LuPanelRight } from "react-icons/lu";
+import { TIPOS } from "./utils/tipos";
+import type { ItemHistorico } from "./utils/historico";
+
+interface HistoricoProps {
+  itens: ItemHistorico[];
+  /** Devolve a pergunta à caixa de mensagem (não envia: quem pergunta ainda pode ajustar). */
+  onUsar: (item: ItemHistorico) => void;
+  onFavoritar: (id: string) => void;
+  onRemover: (id: string) => void;
+  onLimparRecentes: () => void;
+  onFechar: () => void;
+}
+
+const dia = (em: number) =>
+  new Date(em).toLocaleDateString("pt-BR", { day: "2-digit", month: "2-digit" });
+
+/** Botão de ícone das linhas e do cabeçalho: 32px, para o dedo acertar no celular. */
+const Acao = chakra("button", {
+  base: {
+    display: "flex",
+    alignItems: "center",
+    justifyContent: "center",
+    flex: "none",
+    w: "32px",
+    h: "32px",
+    borderRadius: "8px",
+    color: "fg.muted",
+    cursor: "pointer",
+    _hover: { bg: "bg.overlay", color: "fg" },
+  },
+});
+
+const Linha = ({
+  item,
+  onUsar,
+  onFavoritar,
+  onRemover,
+}: { item: ItemHistorico } & Pick<HistoricoProps, "onUsar" | "onFavoritar" | "onRemover">) => (
+  <Flex as="li" align="flex-start" gap="2px" borderRadius="8px" _hover={{ bg: "bg.subtle" }}>
+    <chakra.button
+      type="button"
+      onClick={() => onUsar(item)}
+      flex="1"
+      minW={0}
+      px="10px"
+      py="6px"
+      textAlign="left"
+      cursor="pointer"
+      borderRadius="8px"
+    >
+      <Text fontSize="0.85rem" lineHeight="1.35" color="fg" lineClamp={2}>
+        {item.pergunta}
+      </Text>
+      <Text fontSize="0.7rem" color="fg.muted" mt="2px">
+        {TIPOS[item.tipo].rotulo} · {dia(item.em)}
+      </Text>
+    </chakra.button>
+    {/* `aria-pressed` é o que diz ao leitor de tela que a estrela é um liga/desliga. */}
+    <Acao
+      type="button"
+      aria-pressed={item.favorita}
+      aria-label={item.favorita ? "Tirar das favoritas" : "Marcar como favorita"}
+      onClick={() => onFavoritar(item.id)}
+      color={item.favorita ? "accent" : "fg.muted"}
+    >
+      {item.favorita ? <MdStar size={18} /> : <MdStarBorder size={18} />}
+    </Acao>
+    <Acao type="button" aria-label="Remover esta pergunta" onClick={() => onRemover(item.id)}>
+      <MdClose size={16} />
+    </Acao>
+  </Flex>
+);
+
+const Grupo = ({
+  titulo,
+  itens,
+  ...acoes
+}: { titulo: string; itens: ItemHistorico[] } & Pick<
+  HistoricoProps,
+  "onUsar" | "onFavoritar" | "onRemover"
+>) =>
+  itens.length === 0 ? null : (
+    <Box as="section" aria-label={titulo}>
+      {/* O mesmo título de grupo da barra lateral esquerda. */}
+      <Text
+        fontSize="0.68rem"
+        color="fg.muted"
+        letterSpacing="0.06em"
+        textTransform="uppercase"
+        px="10px"
+        mt="14px"
+        mb="4px"
+      >
+        {titulo}
+      </Text>
+      <Flex as="ul" direction="column" gap="2px" listStyleType="none">
+        {itens.map((item) => (
+          <Linha key={item.id} item={item} {...acoes} />
+        ))}
+      </Flex>
+    </Box>
+  );
+
+/**
+ * O conteúdo do painel "Minhas perguntas": as favoritas em cima, as recentes embaixo.
+ *
+ * Só a lista e os botões — onde ela aparece (coluna fixa à direita ou drawer) é decisão do
+ * `Chat`, que a monta nos dois lugares.
+ */
+export const Historico = ({ itens, onLimparRecentes, onFechar, ...acoes }: HistoricoProps) => {
+  const favoritas = itens.filter((i) => i.favorita);
+  const recentes = itens.filter((i) => !i.favorita);
+
+  return (
+    <Flex direction="column" minH="100%" p="10px 8px">
+      <Flex align="center" justify="space-between" pl="10px">
+        <Text fontSize="0.9rem" fontWeight="600" color="fg">
+          Minhas perguntas
+        </Text>
+        <Acao type="button" aria-label="Fechar minhas perguntas" onClick={onFechar}>
+          <LuPanelRight size={18} aria-hidden="true" />
+        </Acao>
+      </Flex>
+
+      {itens.length === 0 && (
+        <Text fontSize="0.85rem" lineHeight="1.45" color="fg.muted" px="10px" mt="14px">
+          As perguntas que você enviar aparecem aqui. Marque com a estrela as que quiser manter.
+        </Text>
+      )}
+      <Grupo titulo="Favoritas" itens={favoritas} {...acoes} />
+      <Grupo titulo="Recentes" itens={recentes} {...acoes} />
+
+      <Box mt="auto" pt="14px" px="10px">
+        {recentes.length > 0 && (
+          <chakra.button
+            type="button"
+            onClick={onLimparRecentes}
+            fontSize="0.8rem"
+            color="fg.muted"
+            cursor="pointer"
+            _hover={{ color: "fg", textDecoration: "underline" }}
+          >
+            Limpar recentes
+          </chakra.button>
+        )}
+        <Text fontSize="0.7rem" lineHeight="1.4" color="fg.muted" mt="6px">
+          Guardadas só neste navegador.
+        </Text>
+      </Box>
+    </Flex>
+  );
+};
diff --git a/auli-frontend/src/pages/chat/utils/historico.test.ts b/auli-frontend/src/pages/chat/utils/historico.test.ts
new file mode 100644
index 0000000..392662a
--- /dev/null
+++ b/auli-frontend/src/pages/chat/utils/historico.test.ts
@@ -0,0 +1,95 @@
+import { describe, expect, it } from "vitest";
+import {
+  MAX_RECENTES,
+  alternarFavorita,
+  desserializar,
+  limparRecentes,
+  registrar,
+  remover,
+  serializar,
+  type ItemHistorico,
+} from "./historico";
+
+const item = (id: string, pergunta: string, extra: Partial<ItemHistorico> = {}): ItemHistorico => ({
+  id,
+  pergunta,
+  tipo: "1",
+  em: 1,
+  favorita: false,
+  ...extra,
+});
+
+describe("registrar", () => {
+  it("põe a pergunta nova no topo", () => {
+    const lista = registrar([item("a", "Como emitir a certidão?")], "Como pagar o IPVA?", "1", 5, "b");
+    expect(lista.map((i) => i.id)).toEqual(["b", "a"]);
+    expect(lista[0]).toEqual({ id: "b", pergunta: "Como pagar o IPVA?", tipo: "1", em: 5, favorita: false });
+  });
+
+  it("pergunta repetida sobe ao topo sem duplicar, mantendo o id e a estrela", () => {
+    const antes = [item("a", "Outra pergunta qualquer"), item("b", "Como pagar o IPVA?", { favorita: true })];
+    // Caixa e espaços não fazem dela outra pergunta.
+    const lista = registrar(antes, "  como pagar   o IPVA? ", "1", 9, "novo");
+    expect(lista.map((i) => i.id)).toEqual(["b", "a"]);
+    expect(lista[0]).toMatchObject({ id: "b", favorita: true, em: 9 });
+  });
+
+  it("a mesma frase em outro tipo de consulta é outra entrada", () => {
+    const lista = registrar([item("a", "Como pagar o IPVA?")], "Como pagar o IPVA?", "2", 9, "b");
+    expect(lista.map((i) => [i.id, i.tipo])).toEqual([
+      ["b", "2"],
+      ["a", "1"],
+    ]);
+  });
+
+  it("descarta a recente mais antiga ao passar do teto, e nunca uma favorita", () => {
+    const cheias = Array.from({ length: MAX_RECENTES }, (_, n) => item(`r${n}`, `Pergunta número ${n}`));
+    const antes = [...cheias, item("fav", "Favorita antiga", { favorita: true })];
+    const lista = registrar(antes, "Pergunta nova", "1", 9, "nova");
+    expect(lista.filter((i) => !i.favorita)).toHaveLength(MAX_RECENTES);
+    expect(lista.map((i) => i.id)).not.toContain(`r${MAX_RECENTES - 1}`);
+    expect(lista.map((i) => i.id)).toContain("fav");
+  });
+});
+
+describe("favoritar, remover e limpar", () => {
+  const lista = [item("a", "Primeira"), item("b", "Segunda", { favorita: true })];
+
+  it("a estrela liga e desliga só o item pedido", () => {
+    expect(alternarFavorita(lista, "a").map((i) => i.favorita)).toEqual([true, true]);
+    expect(alternarFavorita(lista, "b").map((i) => i.favorita)).toEqual([false, false]);
+  });
+
+  it("remover tira só o item pedido", () => {
+    expect(remover(lista, "a").map((i) => i.id)).toEqual(["b"]);
+  });
+
+  it("limpar recentes preserva as favoritas", () => {
+    expect(limparRecentes(lista).map((i) => i.id)).toEqual(["b"]);
+  });
+});
+
+describe("o que está gravado", () => {
+  it("vai e volta sem perda", () => {
+    const lista = [item("a", "Primeira"), item("b", "Segunda", { favorita: true, tipo: "3" })];
+    expect(desserializar(serializar(lista))).toEqual(lista);
+  });
+
+  it.each([
+    ["nada gravado", null],
+    ["texto que não é JSON", "{{{"],
+    ["JSON que não é objeto", "[]"],
+    ["versão desconhecida", JSON.stringify({ v: 2, itens: [item("a", "Primeira")] })],
+    ["itens que não são lista", JSON.stringify({ v: 1, itens: "x" })],
+  ])("%s vira lista vazia", (_, bruto) => {
+    expect(desserializar(bruto)).toEqual([]);
+  });
+
+  it("item fora da forma é descartado, os bons ficam", () => {
+    const bruto = JSON.stringify({
+      v: 1,
+      itens: [item("a", "Boa"), { id: "b", pergunta: "Tipo inválido", tipo: "9", em: 1, favorita: false }, null, { id: "c" }],
+    });
+    expect(desserializar(bruto).map((i) => i.id)).toEqual(["a"]);
+  });
+});
diff --git a/auli-frontend/src/pages/chat/utils/historico.ts b/auli-frontend/src/pages/chat/utils/historico.ts
new file mode 100644
index 0000000..19c225f
--- /dev/null
+++ b/auli-frontend/src/pages/chat/utils/historico.ts
@@ -0,0 +1,101 @@
+import { isQuestionType, type QuestionType } from "./useQuestionType";
+
+/**
+ * Uma pergunta enviada, como o painel "Minhas perguntas" a guarda (D-HIST-2).
+ *
+ * Só a pergunta: a resposta NÃO é guardada. Ela é derivada do acervo, que muda a cada coleta, e
+ * reenviar a pergunta a refaz com o acervo do dia.
+ */
+export interface ItemHistorico {
+  id: string;
+  pergunta: string;
+  tipo: QuestionType;
+  /** Quando foi enviada pela última vez (epoch em ms). */
+  em: number;
+  favorita: boolean;
+}
+
+/** Teto das NÃO favoritas (D-HIST-5). Favorita nunca é descartada sozinha. */
+export const MAX_RECENTES = 50;
+
+/** Versão do formato gravado. Mudou o formato, muda o número — e o que está gravado é ignorado. */
+const VERSAO = 1;
+
+/** A identidade de uma pergunta para a deduplicação: caixa e espaços não a tornam outra. */
+const normalizar = (texto: string) => texto.trim().replace(/\s+/g, " ").toLowerCase();
+
+/**
+ * Registra um envio (D-HIST-3/4). A lista fica sempre da mais recente para a mais antiga.
+ *
+ * Pergunta repetida NO MESMO TIPO não duplica: sobe para o topo com a data nova e mantém o `id` e
+ * a estrela. A mesma frase em outro tipo é outra consulta (outra coleção) e entra à parte.
+ * `em` e `id` vêm de fora para a função seguir pura.
+ */
+export function registrar(
+  itens: ItemHistorico[],
+  pergunta: string,
+  tipo: QuestionType,
+  em: number,
+  id: string,
+): ItemHistorico[] {
+  const chave = normalizar(pergunta);
+  const antiga = itens.find((i) => i.tipo === tipo && normalizar(i.pergunta) === chave);
+  const nova: ItemHistorico = {
+    id: antiga?.id ?? id,
+    pergunta: pergunta.trim(),
+    tipo,
+    em,
+    favorita: antiga?.favorita ?? false,
+  };
+  let recentes = 0;
+  return [nova, ...itens.filter((i) => i !== antiga)].filter(
+    (i) => i.favorita || ++recentes <= MAX_RECENTES,
+  );
+}
+
+export function alternarFavorita(itens: ItemHistorico[], id: string): ItemHistorico[] {
+  return itens.map((i) => (i.id === id ? { ...i, favorita: !i.favorita } : i));
+}
+
+export function remover(itens: ItemHistorico[], id: string): ItemHistorico[] {
+  return itens.filter((i) => i.id !== id);
+}
+
+/** Apaga o histórico e preserva as favoritas — que são o que a pessoa escolheu guardar. */
+export function limparRecentes(itens: ItemHistorico[]): ItemHistorico[] {
+  return itens.filter((i) => i.favorita);
+}
+
+export function serializar(itens: ItemHistorico[]): string {
+  return JSON.stringify({ v: VERSAO, itens });
+}
+
+function eItem(x: unknown): x is ItemHistorico {
+  if (typeof x !== "object" || x === null) return false;
+  const i = x as Record<string, unknown>;
+  return (
+    typeof i.id === "string" &&
+    typeof i.pergunta === "string" &&
+    typeof i.tipo === "string" &&
+    isQuestionType(i.tipo) &&
+    typeof i.em === "number" &&
+    typeof i.favorita === "boolean"
+  );
+}
+
+/**
+ * Lê o que está gravado. O localStorage é texto que qualquer um edita (e que uma versão futura
+ * pode ter escrito): o que não tiver a forma esperada é descartado, nunca derruba o chat.
+ */
+export function desserializar(bruto: string | null): ItemHistorico[] {
+  if (!bruto) return [];
+  try {
+    const dados: unknown = JSON.parse(bruto);
+    if (typeof dados !== "object" || dados === null) return [];
+    const { v, itens } = dados as { v?: unknown; itens?: unknown };
+    if (v !== VERSAO || !Array.isArray(itens)) return [];
+    return itens.filter(eItem);
+  } catch {
+    return [];
+  }
+}
diff --git a/auli-frontend/src/pages/chat/utils/useHistorico.test.ts b/auli-frontend/src/pages/chat/utils/useHistorico.test.ts
new file mode 100644
index 0000000..9e49a84
--- /dev/null
+++ b/auli-frontend/src/pages/chat/utils/useHistorico.test.ts
@@ -0,0 +1,57 @@
+// @vitest-environment jsdom
+import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
+import { act, renderHook } from "@testing-library/react";
+import { useHistorico } from "./useHistorico";
+
+beforeEach(() => localStorage.clear());
+afterEach(() => vi.restoreAllMocks());
+
+describe("useHistorico", () => {
+  it("grava numa chave por entidade, e uma não enxerga a outra", () => {
+    const rs = renderHook(() => useHistorico("rs"));
+    const sp = renderHook(() => useHistorico("sp"));
+    act(() => rs.result.current.registrar("Como pagar o IPVA no estado?", "1"));
+    expect(rs.result.current.itens.map((i) => i.pergunta)).toEqual(["Como pagar o IPVA no estado?"]);
+    expect(sp.result.current.itens).toEqual([]);
+    expect(localStorage.getItem("auli.historico.rs")).toContain("Como pagar o IPVA no estado?");
+    expect(localStorage.getItem("auli.historico.sp")).toBeNull();
+  });
+
+  it("a lista sobrevive a remontar (é o que acontece ao recarregar a página)", () => {
+    const primeiro = renderHook(() => useHistorico("rs"));
+    act(() => primeiro.result.current.registrar("Como pagar o IPVA no estado?", "1"));
+    primeiro.unmount();
+    const segundo = renderHook(() => useHistorico("rs"));
+    expect(segundo.result.current.itens).toHaveLength(1);
+  });
+
+  it("o que outra aba gravou aparece aqui e não é apagado pela próxima gravação", () => {
+    const { result } = renderHook(() => useHistorico("rs"));
+    act(() => result.current.registrar("Pergunta feita nesta aba", "1"));
+    // A outra aba releu, acrescentou a dela e gravou; o navegador avisa esta com `storage`.
+    const daOutra = JSON.parse(localStorage.getItem("auli.historico.rs")!);
+    daOutra.itens.unshift({ id: "x", pergunta: "Pergunta feita na outra aba", tipo: "2", em: 9, favorita: false });
+    act(() => {
+      localStorage.setItem("auli.historico.rs", JSON.stringify(daOutra));
+      window.dispatchEvent(new StorageEvent("storage", { key: "auli.historico.rs" }));
+    });
+    expect(result.current.itens).toHaveLength(2);
+    act(() => result.current.alternarFavorita("x"));
+    expect(result.current.itens.map((i) => i.pergunta)).toEqual([
+      "Pergunta feita na outra aba",
+      "Pergunta feita nesta aba",
+    ]);
+  });
+
+  it("sem localStorage, a lista vale nesta sessão", () => {
+    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
+      throw new Error("bloqueado");
+    });
+    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
+      throw new Error("bloqueado");
+    });
+    const { result } = renderHook(() => useHistorico("go"));
+    act(() => result.current.registrar("Como pagar o IPVA no estado?", "1"));
+    expect(result.current.itens).toHaveLength(1);
+  });
+});
diff --git a/auli-frontend/src/pages/chat/utils/useHistorico.ts b/auli-frontend/src/pages/chat/utils/useHistorico.ts
new file mode 100644
index 0000000..676eb99
--- /dev/null
+++ b/auli-frontend/src/pages/chat/utils/useHistorico.ts
@@ -0,0 +1,73 @@
+import { useSyncExternalStore } from "react";
+import * as historico from "./historico";
+import type { ItemHistorico } from "./historico";
+import type { QuestionType } from "./useQuestionType";
+
+/** Uma chave por entidade (D-HIST-1): a pergunta feita no RS não aparece em SP. */
+const PREFIXO = "auli.historico.";
+
+const VAZIO: ItemHistorico[] = [];
+const ouvintes = new Set<() => void>();
+/** Última leitura de cada chave. O `useSyncExternalStore` exige a MESMA referência enquanto nada
+ *  muda, então só se refaz o parse quando o texto gravado muda. */
+const cache = new Map<string, { bruto: string | null; itens: ItemHistorico[] }>();
+
+function ler(chave: string): ItemHistorico[] {
+  let bruto: string | null;
+  try {
+    bruto = localStorage.getItem(chave);
+  } catch {
+    // Sem localStorage (bloqueado pelo navegador): a lista vale só nesta sessão, na memória.
+    return cache.get(chave)?.itens ?? VAZIO;
+  }
+  const visto = cache.get(chave);
+  if (visto && visto.bruto === bruto) return visto.itens;
+  const itens = historico.desserializar(bruto);
+  cache.set(chave, { bruto, itens });
+  return itens;
+}
+
+function gravar(chave: string, itens: ItemHistorico[]) {
+  const bruto = historico.serializar(itens);
+  try {
+    localStorage.setItem(chave, bruto);
+  } catch {
+    // Não fatal, como nas outras preferências: segue valendo nesta sessão.
+  }
+  cache.set(chave, { bruto, itens });
+  ouvintes.forEach((avisar) => avisar());
+}
+
+function assinar(avisar: () => void) {
+  ouvintes.add(avisar);
+  // `storage` dispara nas OUTRAS abas quando uma grava: duas abas da Auli mostram a mesma lista.
+  window.addEventListener("storage", avisar);
+  return () => {
+    ouvintes.delete(avisar);
+    window.removeEventListener("storage", avisar);
+  };
+}
+
+/**
+ * As perguntas enviadas nesta entidade, guardadas só neste navegador (Etapa 1: sem login e sem
+ * servidor).
+ *
+ * Toda gravação relê a lista antes de alterá-la, em vez de partir da que está na tela: com duas
+ * abas abertas, a que grava depois não apaga o que a outra acabou de registrar.
+ *
+ * É a ÚNICA porta para o armazenamento. Na Etapa 2 (login), o que muda é este arquivo — quem usa
+ * o hook não sabe onde a lista mora.
+ */
+export function useHistorico(entityId: string) {
+  const chave = PREFIXO + entityId;
+  const itens = useSyncExternalStore(assinar, () => ler(chave));
+
+  return {
+    itens,
+    registrar: (pergunta: string, tipo: QuestionType) =>
+      gravar(chave, historico.registrar(ler(chave), pergunta, tipo, Date.now(), crypto.randomUUID())),
+    alternarFavorita: (id: string) => gravar(chave, historico.alternarFavorita(ler(chave), id)),
+    remover: (id: string) => gravar(chave, historico.remover(ler(chave), id)),
+    limparRecentes: () => gravar(chave, historico.limparRecentes(ler(chave))),
+  };
+}
diff --git a/auli-frontend/src/pages/chat/utils/useQuestionType.ts b/auli-frontend/src/pages/chat/utils/useQuestionType.ts
index 9cf32e5..e8ab607 100644
--- a/auli-frontend/src/pages/chat/utils/useQuestionType.ts
+++ b/auli-frontend/src/pages/chat/utils/useQuestionType.ts
@@ -37,7 +37,7 @@ export function tiposDisponiveis(entity: Entity): QuestionType[] {
   });
 }
 
-function isQuestionType(value: string | null): value is QuestionType {
+export function isQuestionType(value: string | null): value is QuestionType {
   return value === "1" || value === "2" || value === "3" || value === "4";
 }
 
diff --git a/auli-frontend/src/shared/AppHeader.tsx b/auli-frontend/src/shared/AppHeader.tsx
index 354a328..bf8ba50 100644
--- a/auli-frontend/src/shared/AppHeader.tsx
+++ b/auli-frontend/src/shared/AppHeader.tsx
@@ -4,6 +4,7 @@ import { LuPanelLeft } from "react-icons/lu";
 import { ColorModeButton } from "../pages/chat/ui/color-mode.jsx";
 import { Tooltip } from "../pages/chat/ui/tooltip";
 import type { Entity } from "./entities";
+import { HEADER_HEIGHT } from "./layout";
 
 interface AppHeaderProps {
   /** A versão do app. Saiu da vista (D-UI-4): vira a dica do logo — era informação de quem
@@ -37,7 +38,7 @@ export const AppHeader = ({
     <Box
       as="header"
       w="100%"
-      h="52px"
+      h={HEADER_HEIGHT}
       flex="none"
       px={{ base: 2, md: 4 }}
       bg="bg.canvas"
diff --git a/auli-frontend/src/shared/layout.ts b/auli-frontend/src/shared/layout.ts
index 9b9dd2a..bb532d4 100644
--- a/auli-frontend/src/shared/layout.ts
+++ b/auli-frontend/src/shared/layout.ts
@@ -20,3 +20,11 @@ export const SIDEBAR_RAIL_WIDTH = "64px";
  *  `SIDEBAR_RAIL_WIDTH` recolhida. O `Home` a define na sua raiz e a barra fixa do chat, que é
  *  descendente dela no DOM, a herda — `fixed` muda o bloco de contenção, não a herança. */
 export const SIDEBAR_VAR = "--auli-sidebar-w";
+
+/** Altura do cabeçalho. O painel "Minhas perguntas" gruda no topo da área que rola e precisa da
+ *  altura que sobra abaixo dele; o `AppHeader` lê daqui para os dois não divergirem. */
+export const HEADER_HEIGHT = "52px";
+
+/** Largura do painel "Minhas perguntas", à direita do chat (a partir do breakpoint `xl`; abaixo
+ *  dele vira drawer). A barra de composição, que é `fixed`, recua esta largura à direita. */
+export const HISTORICO_WIDTH = "288px";
-- 
2.43.0

~~~~~
