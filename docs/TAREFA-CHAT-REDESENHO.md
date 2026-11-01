# TAREFA — Chat redesenhado (D-UI-1..10)

> Documento único para o Claude Code: explicações, passo a passo e, no Anexo, o patch completo
> (código e testes). Trabalhe **um passo de cada vez** e confirme comigo (Carlos) nos portões
> marcados. Nada de push ou merge sem confirmação.

## 1. Contexto

A interface do chat foi capturada em navegador headless (desktop 1440×900 e celular 390×844, claro e
escuro) e analisada. O redesenho seguiu um mockup aprovado:
<https://claude.ai/artifact/FkKshDgq5ynaN4uJMbP5Em> (três pranchetas: resposta no desktop, estado
inicial com o seletor aberto, resposta no celular).

O que as capturas mostraram, em números: no desktop, cabeçalho (77px) e caixa de mensagem (~175px)
comiam 28% da altura; no celular, cabeçalho + barra do menu + caixa chegavam a ~370 de 844px (44%).
A resposta corria em ~980px de largura (~140 caracteres por linha). E seis defeitos visíveis: listas
sem marcador, "Resumo" colado no parágrafo, resposta que abria pelo fim no celular, pergunta que
ficava na caixa durante a espera, item ativo da barra lateral cortado, e um botão no balão do usuário
com ícone de "copiar" que na verdade reutiliza a pergunta.

**Escopo: só o frontend.** Nenhuma mudança no servidor, no contrato da API, nos prompts ou nos packs.

## 2. As decisões

| | O quê | Por quê |
| --- | --- | --- |
| **D-UI-1** | `ul`/`ol` dos dois mapas de markdown ganham `listStyleType` | O reset do Chakra zera o `list-style`; os itens viravam parágrafos recuados soltos |
| **D-UI-2** | A caixa esvazia no **envio** (não na chegada); a resposta que chega rola até o **começo** dela; o botão do balão do usuário vira "Reutilizar a pergunta" (ícone de lápis) | A pergunta ficava no campo com "Pronto para enviar" durante a espera, convidando a reenviar; a resposta longa abria na tabela e nas fontes, com o Resumo fora da vista; o ícone de cópia não dizia que a pergunta volta para a caixa |
| **D-UI-3** | Item ativo da barra lateral em peso 600 (era 700); `SIDEBAR_WIDTH` 210 → 224px | O negrito alargava o texto e cortava "Artigos e estu…" |
| **D-UI-4** | Cabeçalho de 52px na cor de superfície, com o estado à esquerda ("SEFAZ-RS ▾") e a versão como dica do logo; coluna de leitura de 760px; caixa numa linha só (seletor compacto + campo que cresce a partir de 1 linha + enviar), dica de mínimo só com texto curto; aviso de protótipo uma vez, sob a caixa; faixa da caixa na cor do fundo; fundo escuro `#111113` (era `#000`) | Devolve área de leitura e legibilidade (70–80 caracteres por linha). O aviso de 3 linhas repetido em toda resposta vira uma linha fixa |
| **D-UI-5** | `utils/secoes.ts` separa "Resumo" e "Detalhes" do texto — rótulo em negrito sozinho, com dois-pontos, ou como título | O modelo escreve `**Resumo**` colado ao parágrafo, e a estrutura que o prompt pede sumia no render |
| **D-UI-6** | O Resumo vira bloco de destaque (token `bg.resumo`, rótulo em `accent.strong`) | É a parte mais lida; tem de ser a primeira coisa que o olho encontra. Fundo, não borda lateral colorida (clichê que envelhece mal) |
| **D-UI-7** | `utils/citacoes.ts` + `CitacaoSelo`: `[Serviço 2]` vira selo clicável que leva à fonte e a realça (2,5 s); a dica mostra o título | Fecha visualmente a promessa da D-SF-7 e da D-SF-10: cada afirmação aponta para a origem. Só vira selo quando há lista de fontes para onde levar |
| **D-UI-8** | `utils/fontes.ts` + `Fontes.tsx`: três grupos — **citadas** (sempre à vista), **também consideradas** (com selo "depende de condição"; abrem sozinhas se o texto não cita nada) e **descartadas** (recolhidas, mas presentes) | O que sustenta o texto fica à vista; a alternativa `condicional` que o texto às vezes omite (REGISTRO-triagem §7) fica a um clique; o descarte errado continua visível |
| **D-UI-9** | Linha de contexto no topo da resposta ("Serviços + FAQs · respondida em 8 s"); `Message` ganha `tipo` e `duracaoMs`; `utils/tipos.ts` concentra rótulo, descrição, apresentação, placeholder e exemplos de cada tipo | Numa conversa que mistura tipos, não se sabia de que gaveta veio cada resposta. O texto de cada tipo, que estava espalhado, fica num lugar só |
| **D-UI-10** | `EstadoInicial.tsx` no lugar do "Olá! Como posso ajudar?": o que o tipo faz, três exemplos (vão para a caixa, não são enviados) e o que a Auli não responde | Reduz pergunta feita no tipo errado e deixa explícito que situação de contribuinte e erro de sistema estão fora do acervo público |

## 3. O que já foi verificado fora daqui

O patch foi aplicado sobre o `754060a` (`main` de 04/10/2026). Nesse estado: `tsc --noEmit` limpo,
`npm run lint` com 0 avisos, `npm run build` ok, `vitest run` com **268 testes** (22 novos),
`check-sem-navi.sh` ok. A árvore resultante é `e727074aca372c9588d49cfa56256b7e69afeb1e`.

Conferido no Chromium, com dados de exemplo e a API simulada: as três telas do mockup, nos modos
claro e escuro; e a **troca de tipo** pelo seletor (desktop e celular) — o `RadioGroup` do Chakra não
responde a clique no jsdom, então o teste automatizado cobre abrir, listar e fechar com Esc, e a troca
em si ficou conferida só no navegador.

**Não verificado:** com o LLM real (o formato da resposta foi simulado a partir do REGISTRO-triagem),
em Safari/iOS e com leitor de tela.

## 4. Passo a passo

### Passo 1 — Extrair o patch

O patch está no Anexo, num bloco cercado por `~~~~~patch`. Extraia com o script (ajuste o caminho) e
confira o hash — se divergir, pare e me avise:

```bash
mkdir -p /tmp/dui && python3 - <<'PY'
import re, pathlib, hashlib
doc = pathlib.Path("TAREFA-CHAT-REDESENHO.md").read_text(encoding="utf-8")
for nome, corpo in re.findall(r"^~~~~~patch (\S+)\n(.*?)^~~~~~$", doc, re.S | re.M):
    pathlib.Path("/tmp/dui", nome).write_text(corpo, encoding="utf-8")
    print(hashlib.sha256(corpo.encode("utf-8")).hexdigest(), nome)
PY
```

Esperado:

```text
d76bb85f6cdf0bab176921509b6c291538fdd84d1477846c659840636cfa640a  0001-chat-redesenho.patch
```

### Passo 2 — Aplicar numa branch

```bash
cd ~/Desktop/auli
git status --short                 # tem de estar limpo; se não estiver, pare e me pergunte
git switch -c chat-redesenho
git am -3 /tmp/dui/0001-chat-redesenho.patch
git rev-parse HEAD^{tree}          # se a base era o 754060a: e727074aca372c9588d49cfa56256b7e69afeb1e
```

Se o `main` andou e o `git am` parar em conflito, resolva **preservando as duas intenções**. Os pontos
prováveis são os arquivos do chat (`Chat.tsx`, `Input.tsx`, `SystemMessage.tsx`) e o `theme/system.js`.
Na dúvida sobre a intenção, pare e me pergunte — não use `git am --skip`.

### Passo 3 — Verificar

```bash
cd auli-frontend
npm ci && npm run typecheck && npm run lint && npm test && npm run build
cd .. && bash scripts/tools/check-sem-navi.sh
```

### Passo 4 — Conferir no navegador (portão humano)

Suba o frontend contra o servidor real (`npm run dev`, ou o staging) e confira, no desktop e no
celular:

1. Estado inicial: título, apresentação do tipo, três exemplos que vão para a caixa **sem enviar**.
2. Seletor de tipo: abre para cima, mostra a descrição, troca o tipo, o texto de exemplo do campo
   muda, fecha com Esc e com clique fora.
3. Pergunta de Serviços + FAQs no `rs` (por exemplo *"Empresa nova: como apurar e recolher o ICMS?"*):
   a caixa esvazia no envio; a resposta rola até o começo; Resumo em bloco; listas com marcador;
   selos clicáveis levando à fonte realçada; fontes em três grupos; "respondida em N s" coerente.
4. Pergunta de Pareceres: sem selos nem fontes (o tipo não as tem), sem quebra de layout.
5. Modo escuro e o botão de tema no cabeçalho novo.
6. Celular: a caixa numa linha; o "Recolher" e a barrinha "Escrever pergunta" continuam funcionando.

### Passo 5 — Registrar e fechar

- Acrescente a tabela da §2 ao `docs/auli_code.md`, na seção do frontend (decisões D-UI-\*), no mesmo
  PR.
- Bump de versão do frontend num commit `chore(frontend): v0.1.67` separado, como é a convenção.
- **Não** faça push nem merge sem a minha confirmação.

## 5. Portões humanos — textos que eu preciso aprovar

- **As perguntas de exemplo** de cada tipo e as frases de apresentação, em `utils/tipos.ts`. São
  sugestões genéricas, não frases do portal; é a primeira coisa que quem chega lê.
- **O aviso curto** sob a caixa: *"A Auli é experimental e pode errar — confira as fontes."* Ele
  substitui, nas respostas, o texto completo que tinha sido validado com quem atende. Proposta: levar
  o texto completo para o `public/about.md` (Sobre), para ele não se perder — confirme comigo antes.
- **O aviso do estado inicial** sobre o que a Auli não responde.

## 6. Fora de escopo (próximas rodadas)

- No celular, ainda há duas barras no topo (o cabeçalho e a barra "≡ Chat" do `Home`). Juntá-las
  numa só, com o menu no cabeçalho como no mockup, mexe no `Home.tsx` e no drawer — fica para depois.
- O grupo 4 da análise: Tributum com cara de revista, tela inicial com lista pesquisável de estados.
- Título e descrição de página nas listas do acervo, contagem por grupo e a ação "Perguntar ao chat".

---

## Anexo — o patch

Um arquivo `git format-patch` íntegro. Extraia com o script do Passo 1.

~~~~~patch 0001-chat-redesenho.patch
From 1b92e2d9c01f1489f110264f89594355c7a2b856 Mon Sep 17 00:00:00 2001
From: ubuntu <carlos.schellenberger@gmail.com>
Date: Mon, 5 Oct 2026 03:26:15 +0000
Subject: [PATCH] =?UTF-8?q?feat(chat):=20o=20chat=20redesenhado=20?=
 =?UTF-8?q?=E2=80=94=20leitura=20em=20coluna,=20resposta=20confer=C3=ADvel?=
 =?UTF-8?q?=20e=20caixa=20numa=20linha=20(D-UI-1..10)?=
MIME-Version: 1.0
Content-Type: text/plain; charset=UTF-8
Content-Transfer-Encoding: 8bit

A interface foi capturada em navegador (desktop 1440×900 e celular 390×844,
claro e escuro) e o redesenho seguiu o mockup aprovado. Três frentes:

Correções (D-UI-1..3)
- As listas da resposta voltam a ter marcador: o reset do Chakra zerava o
  `list-style`, e os itens viravam parágrafos recuados soltos.
- A caixa esvazia no ENVIO; a resposta que chega rola até o COMEÇO dela (o
  Resumo ficava fora da vista no celular); o botão do balão do usuário diz o
  que faz — reutilizar a pergunta, não "copiar".
- O item ativo da barra lateral não corta mais (600 em vez de 700; 224px).

Layout (D-UI-4)
- Cabeçalho de 52px na cor de superfície, com "SEFAZ-RS ▾" à esquerda; a
  versão vira a dica do logo.
- Coluna de leitura de 760px para mensagens e caixa.
- Caixa numa linha só: seletor de tipo compacto (abre a lista com descrição),
  campo que cresce a partir de 1 linha, dica de mínimo só quando há texto
  curto. O aviso de protótipo sai de cada resposta e fica uma vez, sob a caixa.
- Fundo escuro quase-preto (#111113 em vez de #000).

Resposta conferível (D-UI-5..10)
- "Resumo" e "Detalhes" separados do texto (`utils/secoes.ts`): o Resumo vira
  bloco de destaque — o rótulo em negrito colava no parágrafo.
- Citações viram selos clicáveis (`utils/citacoes.ts`, `CitacaoSelo`) que levam
  à fonte e a realçam.
- Fontes em três grupos (`utils/fontes.ts`): citadas à vista; também
  consideradas (com o selo "depende de condição") e descartadas, recolhidas.
- Linha de contexto no topo da resposta: tipo e tempo.
- Estado inicial no lugar do "Olá": apresentação do tipo, três exemplos que
  vão para a caixa, e o que a Auli não responde. Texto de cada tipo num lugar
  só (`utils/tipos.ts`).

Verificado: tsc, lint (0 avisos), build, 268 testes (22 novos), check-sem-navi;
a troca de tipo, que o jsdom não aciona por clique, conferida no Chromium.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>
---
 auli-frontend/src/pages/chat/Chat.test.tsx    |  36 ++-
 auli-frontend/src/pages/chat/Chat.tsx         |  44 +++-
 auli-frontend/src/pages/chat/CitacaoSelo.tsx  |  42 +++
 .../src/pages/chat/EstadoInicial.tsx          |  76 ++++++
 auli-frontend/src/pages/chat/Fontes.test.tsx  |  54 ++--
 auli-frontend/src/pages/chat/Fontes.tsx       | 204 ++++++++++----
 auli-frontend/src/pages/chat/Input.test.tsx   |  10 +-
 auli-frontend/src/pages/chat/Input.tsx        |  94 ++++---
 auli-frontend/src/pages/chat/Messages.tsx     |   5 +
 .../src/pages/chat/SelectQuestionType.tsx     | 170 +++++++-----
 .../src/pages/chat/SystemMessage.tsx          | 249 ++++++++++++------
 auli-frontend/src/pages/chat/UserMessage.tsx  |   6 +-
 .../src/pages/chat/utils/callServerAPI.ts     |  11 +-
 .../src/pages/chat/utils/citacoes.test.ts     |  28 ++
 .../src/pages/chat/utils/citacoes.ts          |  50 ++++
 .../src/pages/chat/utils/fontes.test.ts       |  21 ++
 auli-frontend/src/pages/chat/utils/fontes.ts  |  25 ++
 .../src/pages/chat/utils/secoes.test.ts       |  40 +++
 auli-frontend/src/pages/chat/utils/secoes.ts  |  56 ++++
 auli-frontend/src/pages/chat/utils/tipos.ts   |  64 +++++
 auli-frontend/src/pages/home/Home.tsx         |   2 +-
 auli-frontend/src/shared/AppHeader.tsx        |  95 +++----
 auli-frontend/src/shared/layout.ts            |   2 +-
 auli-frontend/src/shared/markdown.tsx         |  15 +-
 auli-frontend/src/theme/system.js             |  21 +-
 auli-frontend/src/types/chat.ts               |   5 +
 26 files changed, 1068 insertions(+), 357 deletions(-)
 create mode 100644 auli-frontend/src/pages/chat/CitacaoSelo.tsx
 create mode 100644 auli-frontend/src/pages/chat/EstadoInicial.tsx
 create mode 100644 auli-frontend/src/pages/chat/utils/citacoes.test.ts
 create mode 100644 auli-frontend/src/pages/chat/utils/citacoes.ts
 create mode 100644 auli-frontend/src/pages/chat/utils/fontes.test.ts
 create mode 100644 auli-frontend/src/pages/chat/utils/fontes.ts
 create mode 100644 auli-frontend/src/pages/chat/utils/secoes.test.ts
 create mode 100644 auli-frontend/src/pages/chat/utils/secoes.ts
 create mode 100644 auli-frontend/src/pages/chat/utils/tipos.ts

diff --git a/auli-frontend/src/pages/chat/Chat.test.tsx b/auli-frontend/src/pages/chat/Chat.test.tsx
index bde1b99..317ef22 100644
--- a/auli-frontend/src/pages/chat/Chat.test.tsx
+++ b/auli-frontend/src/pages/chat/Chat.test.tsx
@@ -23,23 +23,41 @@ describe("Chat", () => {
   it("monta o seletor de tipo de consulta DENTRO da caixa de mensagem", () => {
     montar();
     const caixa = screen.getByRole("group", { name: "Caixa de mensagem" });
-    expect(caixa).toContainElement(screen.getByRole("radiogroup"));
-    expect(caixa).toContainElement(screen.getByPlaceholderText("Digite sua pergunta..."));
+    const seletor = screen.getByRole("button", { name: /^Tipo de consulta:/ });
+    expect(caixa).toContainElement(seletor);
+    expect(caixa).toContainElement(screen.getByLabelText("Sua pergunta"));
   });
 
   it("o seletor vem ANTES do campo dentro da caixa", () => {
     montar();
-    const grupo = screen.getByRole("radiogroup");
-    const campo = screen.getByPlaceholderText("Digite sua pergunta...");
+    const seletor = screen.getByRole("button", { name: /^Tipo de consulta:/ });
+    const campo = screen.getByLabelText("Sua pergunta");
     // `DOCUMENT_POSITION_FOLLOWING` = o campo vem depois do seletor na ordem do documento, que é
     // a ordem que o teclado e o leitor de tela seguem.
-    expect(grupo.compareDocumentPosition(campo) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
+    expect(seletor.compareDocumentPosition(campo) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
   });
 
-  it("as duas opções de consulta continuam disponíveis", () => {
+  // A troca de tipo em si é do RadioGroup do Chakra, que o jsdom não aciona por clique; ela é
+  // conferida no navegador (ver a TAREFA-CHAT-REDESENHO, Fase 2). Aqui: abrir, listar, fechar.
+  it("o seletor abre a lista dos tipos (radiogroup) e fecha com Esc", () => {
     montar();
-    expect(screen.getByText("Serviços+FAQs")).toBeInTheDocument();
-    expect(screen.getByText("Pareceres")).toBeInTheDocument();
+    const botao = screen.getByRole("button", { name: /^Tipo de consulta:/ });
+    expect(botao).toHaveAttribute("aria-expanded", "false");
+    fireEvent.click(botao);
+    expect(botao).toHaveAttribute("aria-expanded", "true");
+    const grupo = screen.getByRole("radiogroup", { name: "Tipo de consulta" });
+    expect(grupo).toHaveTextContent("Serviços + FAQs");
+    expect(grupo).toHaveTextContent("Pareceres");
+    fireEvent.keyDown(grupo, { key: "Escape" });
+    expect(screen.queryByRole("radiogroup")).toBeNull();
+  });
+
+  it("antes da primeira pergunta, mostra a apresentação do tipo e os exemplos vão para a caixa", () => {
+    localStorage.setItem("auli.questionType", "1");
+    montar();
+    expect(screen.getByRole("heading", { name: "Como posso ajudar?" })).toBeInTheDocument();
+    fireEvent.click(screen.getByRole("button", { name: "Empresa nova: como apurar e recolher o ICMS?" }));
+    expect(screen.getByLabelText("Sua pergunta")).toHaveValue("Empresa nova: como apurar e recolher o ICMS?");
   });
 });
 
@@ -106,7 +124,7 @@ describe("recolher a caixa de mensagem (celular)", () => {
     fireEvent.click(screen.getByRole("button", { name: "Mostrar a caixa de mensagem" }));
     expect(regrasDe(caixa())).not.toMatch(/display:\s*none/);
     await waitFor(() =>
-      expect(document.activeElement).toBe(screen.getByPlaceholderText("Digite sua pergunta...")),
+      expect(document.activeElement).toBe(screen.getByLabelText("Sua pergunta")),
     );
   });
 });
diff --git a/auli-frontend/src/pages/chat/Chat.tsx b/auli-frontend/src/pages/chat/Chat.tsx
index 3527941..7812b09 100644
--- a/auli-frontend/src/pages/chat/Chat.tsx
+++ b/auli-frontend/src/pages/chat/Chat.tsx
@@ -1,4 +1,4 @@
-import { Box, Flex, chakra } from "@chakra-ui/react";
+import { Box, Flex, Text, chakra } from "@chakra-ui/react";
 import { MdEdit, MdKeyboardArrowDown, MdKeyboardArrowUp } from "react-icons/md";
 import { useRef, useEffect, useState } from "react";
 import { useIsKeyboardVisible } from "./utils/useIsKeyboardVisible";
@@ -10,6 +10,8 @@ import { useQuestionType } from "./utils/useQuestionType";
 import { callServerAPI } from "./utils/callServerAPI";
 import { isPromptValid } from "./utils/prompt";
 import { SelectQuestionType } from "./SelectQuestionType";
+import { EstadoInicial } from "./EstadoInicial";
+import { TIPOS } from "./utils/tipos";
 import { useSelectedEntity } from "../../shared/EntityContext";
 import { SIDEBAR_WIDTH } from "../../shared/layout";
 // O `API_URL` saiu daqui para `utils/api.ts` quando o modal do log passou a precisar da MESMA
@@ -50,12 +52,27 @@ export const Chat = () => {
     return () => ro.disconnect();
   }, []);
 
+  // Resposta que chega rola até o COMEÇO dela, não até o fim (D-UI-2): a resposta longa terminava
+  // com a tabela e as fontes na tela, e o Resumo — a parte mais lida — ficava acima, fora da vista
+  // (no celular, sempre). A pergunta e a espera seguem rolando até o fim, como antes.
   useEffect(() => {
+    const ultima = messages[messages.length - 1];
     requestAnimationFrame(() => {
-      messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
+      if (ultima?.from === "server" && ultima.showButton && !ultima.pendente) {
+        document.getElementById(`msg-${ultima.id}`)?.scrollIntoView({ behavior: "smooth", block: "start" });
+      } else {
+        messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
+      }
     });
   }, [messages]);
 
+  // Antes da primeira pergunta, a apresentação do tipo selecionado no lugar da saudação (D-UI-10).
+  const vazio = !messages.some((m) => m.from === "user");
+  const usarExemplo = (pergunta: string) => {
+    setPrompt(pergunta);
+    requestAnimationFrame(() => textareaRef.current?.focus());
+  };
+
   const handleCallServerAPI = async (prompt: string) => {
     const trimmedPrompt = prompt.trim();
     if (isSubmittingRef.current || loading || !isPromptValid(trimmedPrompt)) return;
@@ -78,7 +95,11 @@ export const Chat = () => {
 
   return (
     <Flex flexDirection="column" w="100%" flex={1} bg="bg.app" pb={`${composerHeight + 16}px`}>
-      <Messages messages={messages} setPrompt={setPrompt} />
+      {vazio ? (
+        <EstadoInicial tipo={questionType} onExemplo={usarExemplo} />
+      ) : (
+        <Messages messages={messages} setPrompt={setPrompt} />
+      )}
       <div ref={messagesEndRef} />
 
       {/* Barra de composição. `fixed` é o que permite subi-la acima do teclado virtual — mas
@@ -99,7 +120,9 @@ export const Chat = () => {
         left={{ base: 0, md: SIDEBAR_WIDTH }}
         right={0}
         zIndex={20}
-        bg="bg.canvas"
+        // Mesma cor do fundo: a caixa é que se destaca (borda e sombra), não uma faixa branca de
+        // ponta a ponta (D-UI-4).
+        bg="bg.app"
       >
         {/* Celular: o "Recolher" acima da caixa e, recolhida, a barrinha que a traz de volta. A
             caixa segue filha DIRETA desta barra fixa (o teste da sidebar sobe por `parentElement`),
@@ -159,6 +182,7 @@ export const Chat = () => {
           loading={loading}
           callServerAPI={handleCallServerAPI}
           recolhido={recolhido}
+          placeholder={TIPOS[questionType].placeholder}
         >
           <SelectQuestionType
             questionType={questionType}
@@ -166,6 +190,18 @@ export const Chat = () => {
             disponiveis={disponiveis}
           />
         </Input>
+        {/* O aviso de protótipo saiu de cada resposta (eram 3 linhas repetidas) e mora aqui, uma vez
+            (D-UI-4). O texto completo, validado com quem atende, está no Sobre. */}
+        <Text
+          display={{ base: recolhido ? "none" : "block", md: "block" }}
+          textAlign="center"
+          fontSize="12px"
+          color="fg.muted"
+          px={3}
+          pb={2}
+        >
+          A Auli é experimental e pode errar — confira as fontes.
+        </Text>
       </Box>
     </Flex>
   );
diff --git a/auli-frontend/src/pages/chat/CitacaoSelo.tsx b/auli-frontend/src/pages/chat/CitacaoSelo.tsx
new file mode 100644
index 0000000..f77fb37
--- /dev/null
+++ b/auli-frontend/src/pages/chat/CitacaoSelo.tsx
@@ -0,0 +1,42 @@
+import { chakra } from "@chakra-ui/react";
+
+interface CitacaoSeloProps {
+  rotulo: string;
+  /** Título da fonte, quando a lista de fontes a tem — vira a dica do selo. */
+  titulo?: string;
+  onClick: () => void;
+}
+
+/**
+ * O selo de uma citação no texto (`[Serviço 2]` → "Serviço 2" clicável, D-UI-7). Clicar leva à
+ * fonte na lista abaixo da resposta e a realça; a dica nativa (`title`) mostra o título. É um
+ * `<button>` de verdade — Tab chega nele e o leitor de tela anuncia o que ele faz.
+ */
+export const CitacaoSelo = ({ rotulo, titulo, onClick }: CitacaoSeloProps) => (
+  <chakra.button
+    type="button"
+    onClick={onClick}
+    title={titulo}
+    aria-label={titulo ? `${rotulo}: ${titulo}. Ver nas fontes` : `${rotulo}. Ver nas fontes`}
+    display="inline-flex"
+    alignItems="center"
+    h="20px"
+    px="6px"
+    mx="1px"
+    verticalAlign="1px"
+    borderRadius="6px"
+    border="1px solid"
+    borderColor="border"
+    bg="bg.subtle"
+    color="fg"
+    fontSize="12px"
+    fontWeight="600"
+    lineHeight="1"
+    whiteSpace="nowrap"
+    cursor="pointer"
+    transition="background-color 0.15s ease, color 0.15s ease"
+    _hover={{ bg: "accent", color: "accent.fg", borderColor: "accent" }}
+  >
+    {rotulo}
+  </chakra.button>
+);
diff --git a/auli-frontend/src/pages/chat/EstadoInicial.tsx b/auli-frontend/src/pages/chat/EstadoInicial.tsx
new file mode 100644
index 0000000..60e8131
--- /dev/null
+++ b/auli-frontend/src/pages/chat/EstadoInicial.tsx
@@ -0,0 +1,76 @@
+import { Box, Flex, Heading, Text, chakra } from "@chakra-ui/react";
+import { MdInfoOutline } from "react-icons/md";
+import type { QuestionType } from "./utils/useQuestionType";
+import { TIPOS } from "./utils/tipos";
+
+interface EstadoInicialProps {
+  tipo: QuestionType;
+  /** Põe a pergunta de exemplo na caixa (não envia: quem pergunta ainda pode ajustar). */
+  onExemplo: (pergunta: string) => void;
+}
+
+/**
+ * O que o chat mostra antes da primeira pergunta (D-UI-10), no lugar do "Olá! Como posso ajudar?":
+ * o que o tipo selecionado faz, três perguntas de exemplo e o que a Auli NÃO responde — situação de
+ * contribuinte e erro de processamento de sistema, as perguntas que mais chegam e que o acervo
+ * público não cobre. Muda com o tipo: é a apresentação dele.
+ */
+export const EstadoInicial = ({ tipo, onExemplo }: EstadoInicialProps) => {
+  const t = TIPOS[tipo];
+  return (
+    <Flex w="100%" maxW="760px" mx="auto" direction="column" gap={6} px={3} pt={{ base: 6, md: 14 }} pb={6}>
+      <Flex direction="column" gap={2.5}>
+        <Heading as="h1" fontSize={{ base: "24px", md: "30px" }} lineHeight="1.2" fontWeight="700" letterSpacing="-0.02em">
+          Como posso ajudar?
+        </Heading>
+        <Text fontSize="16.5px" lineHeight="1.6" color="fg.muted" maxW="640px">
+          <Text as="strong" color="fg">
+            {t.rotulo}
+          </Text>{" "}
+          {t.apresentacao}
+        </Text>
+      </Flex>
+
+      <Flex direction="column" gap={2.5}>
+        <Text fontSize="12px" fontWeight="700" letterSpacing="0.08em" color="fg.muted">
+          EXPERIMENTE
+        </Text>
+        <Flex direction="column" align="flex-start" gap={2}>
+          {t.exemplos.map((pergunta) => (
+            <chakra.button
+              key={pergunta}
+              type="button"
+              onClick={() => onExemplo(pergunta)}
+              minH="44px"
+              px={4}
+              py={2.5}
+              border="1px solid"
+              borderColor="border"
+              borderRadius="12px"
+              bg="bg.canvas"
+              color="fg"
+              fontSize="15px"
+              lineHeight="1.4"
+              textAlign="left"
+              cursor="pointer"
+              transition="border-color 0.15s ease"
+              _hover={{ borderColor: "accent" }}
+            >
+              {pergunta}
+            </chakra.button>
+          ))}
+        </Flex>
+      </Flex>
+
+      <Flex gap={2.5} align="flex-start" px={3.5} py={3} borderRadius="12px" bg="bg.subtle" fontSize="14px" lineHeight="1.5" color="fg.muted">
+        <Box flex="none" mt="2px">
+          <MdInfoOutline size={16} />
+        </Box>
+        <Text>
+          A Auli consulta só o acervo público. Situação de um contribuinte específico e erros de processamento dos
+          sistemas exigem acesso aos dados — ela não responde.
+        </Text>
+      </Flex>
+    </Flex>
+  );
+};
diff --git a/auli-frontend/src/pages/chat/Fontes.test.tsx b/auli-frontend/src/pages/chat/Fontes.test.tsx
index ce202b4..28e1eb8 100644
--- a/auli-frontend/src/pages/chat/Fontes.test.tsx
+++ b/auli-frontend/src/pages/chat/Fontes.test.tsx
@@ -1,6 +1,6 @@
 // @vitest-environment jsdom
 import { describe, it, expect } from "vitest";
-import { screen } from "@testing-library/react";
+import { fireEvent, screen } from "@testing-library/react";
 import { renderWithProvider } from "../../test/render";
 import { Fontes, resumoFontes } from "./Fontes";
 import { SystemMessage } from "./SystemMessage";
@@ -15,36 +15,52 @@ const fontes: Fonte[] = [
 
 describe("resumoFontes", () => {
   it("conta o total e, quando houver, os descartados — com o plural certo", () => {
-    expect(resumoFontes(fontes)).toBe("Fontes consultadas: 4 (1 descartada na triagem)");
-    expect(resumoFontes([fontes[0]])).toBe("Fontes consultadas: 1");
-    const duas = fontes.map((f) => ({ ...f, triagem: "descarta" as const }));
-    expect(resumoFontes(duas)).toBe("Fontes consultadas: 4 (4 descartadas na triagem)");
+    expect(resumoFontes(fontes)).toBe("4 recuperadas · 1 descartada na triagem");
+    expect(resumoFontes([fontes[0]])).toBe("1 recuperada");
   });
 });
 
 describe("Fontes", () => {
-  it("mostra rótulo da citação, título em link e o selo do veredito", () => {
-    const { container } = renderWithProvider(<Fontes fontes={fontes} />);
-    // Recolhida por padrão.
-    expect(container.querySelector("details")?.hasAttribute("open")).toBe(false);
-    expect(screen.getByText("[Serviço 2]")).toBeInTheDocument();
-    const link = screen.getByText("Emitir guia (Simples)").closest("a");
-    expect(link).toHaveAttribute("href", "https://s/2");
+  it("citadas à vista; consideradas e descartadas recolhidas, abrindo ao clicar", () => {
+    renderWithProvider(<Fontes fontes={fontes} citados={new Set(["Serviço 1"])} idBase="msg-x" />);
+    const link = screen.getByText("Emitir guia").closest("a");
+    expect(link).toHaveAttribute("href", "https://s/1");
     expect(link).toHaveAttribute("target", "_blank");
-    expect(screen.getByText(/depende de condição/)).toBeInTheDocument();
-    expect(screen.getByText(/descartada na triagem$/)).toBeInTheDocument();
+    expect(screen.queryByText("Emitir guia (Simples)")).toBeNull();
+    fireEvent.click(screen.getByRole("button", { name: /2 também consideradas/ }));
+    expect(screen.getByText("Emitir guia (Simples)")).toBeInTheDocument();
+    expect(screen.getByText("depende de condição")).toBeInTheDocument();
     // Sem link: o título aparece como texto, não como âncora.
     expect(screen.getByText("Prazo").closest("a")).toBeNull();
+    expect(screen.queryByText("Como pagar?")).toBeNull();
+    fireEvent.click(screen.getByRole("button", { name: /descartada na triagem/ }));
+    expect(screen.getByText("Como pagar?")).toBeInTheDocument();
+  });
+
+  it("sem citação no texto, as consideradas já abrem — são tudo o que há", () => {
+    renderWithProvider(<Fontes fontes={fontes} citados={new Set()} idBase="msg-x" />);
+    expect(screen.getByText("Emitir guia")).toBeInTheDocument();
+    expect(screen.getByText("Também consideradas")).toBeInTheDocument();
+  });
+
+  it("cada item tem id único por mensagem, alvo do selo de citação", () => {
+    const { container } = renderWithProvider(<Fontes fontes={fontes} citados={new Set(["Serviço 1"])} idBase="msg-42" />);
+    expect(container.querySelector("#msg-42-fonte-servico-1")).not.toBeNull();
   });
 });
 
 describe("SystemMessage + Fontes", () => {
-  it("desenha a lista só quando há fontes", () => {
+  it("desenha a lista só quando há fontes, e a citação vira selo", () => {
     const { rerender } = renderWithProvider(
-      <SystemMessage messageText="Resposta" showButton fontes={fontes} />,
+      <SystemMessage id="m1" messageText="**Resumo**\nUse a guia [Serviço 1]." showButton fontes={fontes} />,
     );
-    expect(screen.getByText(/Fontes consultadas: 4/)).toBeInTheDocument();
-    rerender(<SystemMessage messageText="Resposta" showButton />);
-    expect(screen.queryByText(/Fontes consultadas/)).toBeNull();
+    expect(screen.getByRole("heading", { name: "Fontes" })).toBeInTheDocument();
+    expect(screen.getByRole("button", { name: "Serviço 1: Emitir guia. Ver nas fontes" })).toBeInTheDocument();
+    expect(screen.getByRole("region", { name: "Resumo" })).toHaveTextContent("Use a guia");
+    rerender(<SystemMessage id="m1" messageText="Use a guia [Serviço 1]." showButton />);
+    expect(screen.queryByRole("heading", { name: "Fontes" })).toBeNull();
+    // Sem fontes, a citação fica como texto: não há para onde o selo levar.
+    expect(screen.queryByRole("button", { name: /Ver nas fontes/ })).toBeNull();
+    expect(screen.getByText(/\[Serviço 1\]/)).toBeInTheDocument();
   });
 });
diff --git a/auli-frontend/src/pages/chat/Fontes.tsx b/auli-frontend/src/pages/chat/Fontes.tsx
index 6b630a3..0d93254 100644
--- a/auli-frontend/src/pages/chat/Fontes.tsx
+++ b/auli-frontend/src/pages/chat/Fontes.tsx
@@ -1,72 +1,162 @@
-import { Box } from "@chakra-ui/react";
+import { Box, Flex, Heading, Text, chakra } from "@chakra-ui/react";
+import { useState } from "react";
+import type { ReactNode } from "react";
+import { MdChevronRight, MdExpandMore } from "react-icons/md";
 import type { Fonte } from "../../types/chat";
 import { toAbsoluteHref } from "../../shared/markdown";
+import { agruparFontes } from "./utils/fontes";
+import { ancora } from "./utils/citacoes";
 
-/** Como o analista lê o veredito da triagem (D-SF-9). `aplica` não ganha selo: é o caso normal. */
-const VEREDITO: Record<NonNullable<Fonte["triagem"]>, string> = {
-  aplica: "",
-  condicional: "depende de condição",
-  descarta: "descartada na triagem",
-};
-
-/** A linha do resumo: "Fontes consultadas: 15 (3 descartadas na triagem)". */
+/** A linha de contagem do cabeçalho: "8 recuperadas · 3 descartadas na triagem". */
 export function resumoFontes(fontes: Fonte[]): string {
   const descartadas = fontes.filter((f) => f.triagem === "descarta").length;
-  const base = `Fontes consultadas: ${fontes.length}`;
+  const base = `${fontes.length} ${fontes.length === 1 ? "recuperada" : "recuperadas"}`;
   if (descartadas === 0) return base;
-  return `${base} (${descartadas} ${descartadas === 1 ? "descartada" : "descartadas"} na triagem)`;
+  return `${base} · ${descartadas} ${descartadas === 1 ? "descartada" : "descartadas"} na triagem`;
 }
 
 interface FontesProps {
   fontes: Fonte[];
+  /** Rótulos que o texto da resposta cita (`rotulosCitados`). */
+  citados: Set<string>;
+  /** Prefixo dos ids dos itens — o id da mensagem —, porque o mesmo rótulo existe em toda resposta. */
+  idBase: string;
+  /** Rótulo realçado agora (o selo clicado no texto). */
+  destaque?: string | null;
 }
 
+const Item = ({ f, idBase, ativo }: { f: Fonte; idBase: string; ativo: boolean }) => {
+  const descartada = f.triagem === "descarta";
+  return (
+    <Flex
+      id={`${idBase}-${ancora(f.rotulo)}`}
+      align="center"
+      wrap="wrap"
+      columnGap={2.5}
+      rowGap={0.5}
+      px={2}
+      py={1}
+      mx={-2}
+      borderRadius="8px"
+      bg={ativo ? "bg.resumo" : "transparent"}
+      transition="background-color 0.3s ease"
+      color={descartada ? "fg.muted" : "fg"}
+    >
+      <Text as="span" flex="none" minW="64px" fontSize="12px" fontWeight="700" color={ativo ? "accent.strong" : "inherit"}>
+        {f.rotulo}
+      </Text>
+      {f.link ? (
+        <a
+          href={toAbsoluteHref(f.link)}
+          target="_blank"
+          rel="noopener noreferrer"
+          style={{ color: descartada ? "inherit" : "var(--chakra-colors-accent)", textDecoration: "underline", textUnderlineOffset: "3px" }}
+        >
+          {f.titulo}
+        </a>
+      ) : (
+        <span>{f.titulo}</span>
+      )}
+      {f.triagem === "condicional" && (
+        <Text as="span" fontSize="12px" fontWeight="600" px={1.5} lineHeight="20px" borderRadius="6px" bg="condicional.bg" color="condicional.fg">
+          depende de condição
+        </Text>
+      )}
+    </Flex>
+  );
+};
+
+const Grupo = ({ titulo, children }: { titulo: string; children: ReactNode }) => (
+  <Flex direction="column" gap={1}>
+    <Text fontSize="11px" fontWeight="700" letterSpacing="0.06em" color="fg.muted" textTransform="uppercase">
+      {titulo}
+    </Text>
+    {children}
+  </Flex>
+);
+
+const Alternar = ({ aberto, onClick, children }: { aberto: boolean; onClick: () => void; children: ReactNode }) => (
+  <chakra.button
+    type="button"
+    aria-expanded={aberto}
+    onClick={onClick}
+    alignSelf="flex-start"
+    display="flex"
+    alignItems="center"
+    gap={1}
+    h="32px"
+    px={2}
+    ml={-2}
+    borderRadius="8px"
+    fontSize="13.5px"
+    color="fg.muted"
+    _hover={{ bg: "bg.overlay" }}
+  >
+    {aberto ? <MdExpandMore size={16} /> : <MdChevronRight size={16} />}
+    {children}
+  </chakra.button>
+);
+
 /**
- * A lista FIXA dos documentos que a busca trouxe (D-SF-10), abaixo da resposta de Serviços + FAQs.
+ * As fontes da resposta de Serviços + FAQs, em três grupos (D-UI-8; antes era uma lista única
+ * recolhida, D-SF-10).
  *
- * Existe para que o analista veja sempre as fontes, independentemente do que o texto do LLM decidiu
- * citar — o núcleo do Auli é achar a informação, e o texto gerado é complemento. Os rótulos são os
- * mesmos da citação no texto (`[Serviço 2]`), na ordem do contexto. Recolhida por padrão: com até 15
- * itens, aberta empurraria a próxima pergunta para longe no celular.
+ * As **citadas** ficam sempre à vista: são poucas e são o que sustenta o texto. As **consideradas**
+ * — inclusive as `condicional`, a alternativa que o texto às vezes omite — abrem por padrão só
+ * quando o texto não cita nada (aí elas são tudo o que há). As **descartadas** ficam recolhidas, mas
+ * não somem: o descarte errado tem que ser visível.
  */
-export const Fontes = ({ fontes }: FontesProps) => (
-  <Box as="details" mt={3} fontSize="13px" lineHeight="1.5" color="fg">
-    <Box as="summary" cursor="pointer" color="fg.muted" userSelect="none">
-      {resumoFontes(fontes)}
-    </Box>
-    <Box as="ul" listStyleType="none" pl={0} mt={2} mb={0}>
-      {fontes.map((f) => {
-        const descartada = f.triagem === "descarta";
-        const selo = f.triagem ? VEREDITO[f.triagem] : "";
-        return (
-          <Box as="li" key={f.rotulo} py="2px" opacity={descartada ? 0.6 : 1}>
-            <Box as="span" fontWeight="600" mr={2} whiteSpace="nowrap">
-              [{f.rotulo}]
-            </Box>
-            {f.link ? (
-              <a
-                href={toAbsoluteHref(f.link)}
-                target="_blank"
-                rel="noopener noreferrer"
-                style={{
-                  color: "var(--chakra-colors-accent)",
-                  textDecoration: descartada ? "line-through" : "underline",
-                  textUnderlineOffset: "2px",
-                }}
-              >
-                {f.titulo}
-              </a>
-            ) : (
-              <span>{f.titulo}</span>
-            )}
-            {selo && (
-              <Box as="span" ml={2} color="fg.muted" fontSize="11px">
-                · {selo}
-              </Box>
-            )}
-          </Box>
-        );
-      })}
-    </Box>
-  </Box>
-);
+export const Fontes = ({ fontes, citados, idBase, destaque = null }: FontesProps) => {
+  const g = agruparFontes(fontes, citados);
+  const [consideradasAbertas, setConsideradasAbertas] = useState(g.citadas.length === 0);
+  const [descartadasAbertas, setDescartadasAbertas] = useState(false);
+
+  return (
+    <Flex as="section" aria-label="Fontes consultadas" direction="column" gap={3} mt={1} pt={3} borderTop="1px solid" borderColor="border" fontSize="14.5px" lineHeight="1.45">
+      <Flex align="baseline" justify="space-between" gap={3} wrap="wrap">
+        <Heading as="h3" fontSize="15px" fontWeight="700" m={0}>
+          Fontes
+        </Heading>
+        <Text fontSize="13px" color="fg.muted">
+          {resumoFontes(fontes)}
+        </Text>
+      </Flex>
+
+      {g.citadas.length > 0 && (
+        <Grupo titulo="Citadas na resposta">
+          {g.citadas.map((f) => (
+            <Item key={f.rotulo} f={f} idBase={idBase} ativo={destaque === f.rotulo} />
+          ))}
+        </Grupo>
+      )}
+
+      {g.consideradas.length > 0 &&
+        (consideradasAbertas ? (
+          <Grupo titulo="Também consideradas">
+            {g.consideradas.map((f) => (
+              <Item key={f.rotulo} f={f} idBase={idBase} ativo={destaque === f.rotulo} />
+            ))}
+          </Grupo>
+        ) : (
+          <Alternar aberto={false} onClick={() => setConsideradasAbertas(true)}>
+            {g.consideradas.length} também {g.consideradas.length === 1 ? "considerada" : "consideradas"}
+          </Alternar>
+        ))}
+
+      {g.descartadas.length > 0 && (
+        <Box>
+          <Alternar aberto={descartadasAbertas} onClick={() => setDescartadasAbertas((v) => !v)}>
+            {g.descartadas.length} {g.descartadas.length === 1 ? "descartada" : "descartadas"} na triagem
+          </Alternar>
+          {descartadasAbertas && (
+            <Flex direction="column" gap={1} mt={1}>
+              {g.descartadas.map((f) => (
+                <Item key={f.rotulo} f={f} idBase={idBase} ativo={destaque === f.rotulo} />
+              ))}
+            </Flex>
+          )}
+        </Box>
+      )}
+    </Flex>
+  );
+};
diff --git a/auli-frontend/src/pages/chat/Input.test.tsx b/auli-frontend/src/pages/chat/Input.test.tsx
index 4966544..2513bea 100644
--- a/auli-frontend/src/pages/chat/Input.test.tsx
+++ b/auli-frontend/src/pages/chat/Input.test.tsx
@@ -30,10 +30,16 @@ describe("Input", () => {
     expect(screen.getByText(/Mínimo \d+ caracteres/)).toBeInTheDocument();
   });
 
-  it("enables send and shows the ready hint for a valid prompt", () => {
+  it("enables send for a valid prompt, with no hint (D-UI-4: the lit button says it)", () => {
     setup(VALID);
     expect(screen.getByLabelText("Enviar pesquisa")).toBeEnabled();
-    expect(screen.getByText("Pronto para enviar")).toBeInTheDocument();
+    expect(screen.queryByText(/Mínimo \d+ caracteres/)).toBeNull();
+  });
+
+  it("shows no hint while the box is empty", () => {
+    setup("");
+    expect(screen.getByLabelText("Enviar pesquisa")).toBeDisabled();
+    expect(screen.queryByText(/Mínimo \d+ caracteres/)).toBeNull();
   });
 
   it("submits on click with a valid prompt", () => {
diff --git a/auli-frontend/src/pages/chat/Input.tsx b/auli-frontend/src/pages/chat/Input.tsx
index a953fa2..5d7f8b8 100644
--- a/auli-frontend/src/pages/chat/Input.tsx
+++ b/auli-frontend/src/pages/chat/Input.tsx
@@ -1,5 +1,5 @@
 import { Flex, Textarea, IconButton, Box } from "@chakra-ui/react";
-import { MdSend } from "react-icons/md";
+import { MdArrowUpward } from "react-icons/md";
 import type { ChangeEvent, ReactNode, RefObject } from "react";
 import { isPromptValid, charsRemaining } from "./utils/prompt";
 
@@ -16,6 +16,8 @@ interface InputProps {
   /** No celular (< md), a caixa some e quem aparece é a barrinha "Escrever pergunta" do `Chat`.
    *  No desktop é ignorado: ali não há barrinha para trazê-la de volta. */
   recolhido?: boolean;
+  /** Texto de exemplo do campo — muda com o tipo de consulta (D-UI-9). */
+  placeholder?: string;
 }
 
 /**
@@ -24,8 +26,12 @@ interface InputProps {
  *
  * A borda é do cartão, não do textarea — antes cada um tinha a sua, e aninhar o seletor deixaria
  * moldura dentro de moldura dentro de moldura. Pelo mesmo motivo o anel de foco subiu para o
- * cartão via `_focusWithin`: acende com o campo E com os chips, que é o que "esta caixa está
+ * cartão via `_focusWithin`: acende com o campo E com o seletor, que é o que "esta caixa está
  * ativa" quer dizer.
+ *
+ * Uma linha só (D-UI-4): seletor de tipo, campo e botão lado a lado; o campo começa com uma linha
+ * e cresce até ~6. A dica de tamanho mínimo só aparece quando há texto curto demais — vazia, a
+ * caixa não precisa explicar nada, e "Pronto para enviar" não informava (o botão aceso já diz).
  */
 export const Input = ({
   textareaRef,
@@ -35,50 +41,53 @@ export const Input = ({
   callServerAPI,
   children,
   recolhido = false,
+  placeholder = "Digite sua pergunta...",
 }: InputProps) => {
   const valid = isPromptValid(prompt);
   const remaining = charsRemaining(prompt);
+  const curto = prompt.trim().length > 0 && !valid;
   return (
     <Box
-      // O cartão passou a reunir controles heterogêneos (o seletor, o campo, o enviar), então
-      // agrupá-los é a marcação honesta — e é o que o dá nome para o leitor de tela.
       role="group"
       aria-label="Caixa de mensagem"
       display={{ base: recolhido ? "none" : "block", md: "block" }}
-      mx={3}
-      mb={2}
-      px={2}
-      pt={2}
-      pb={1}
+      // Coluna de leitura (D-UI-4): no desktop a caixa alinha com as mensagens, em 760px.
+      mx={{ base: 3, md: "auto" }}
+      maxW="760px"
+      mb={1.5}
+      p={1.5}
       bg="bg.canvas"
       border="1px solid"
       borderColor="border"
-      borderRadius="12px"
+      borderRadius="16px"
+      boxShadow="composer"
       _focusWithin={{ borderColor: "accent", boxShadow: "focusRing" }}
       transition="border-color 0.15s ease, box-shadow 0.15s ease"
     >
-      {children}
+      <Flex alignItems="flex-end" gap={2}>
+        {children}
 
-      <Flex mx="auto" flexDirection="row" alignItems="flex-start">
         <Textarea
           ref={textareaRef}
           id="prompt-id"
           name="prompt"
-          size="md"
-          fontSize="1rem"
+          aria-label="Sua pergunta"
+          rows={1}
+          autoresize
+          maxH="160px"
+          minH={{ base: "44px", md: "40px" }}
           px={2}
-          py={2}
-          rows={3}
-          minH="80px"
-          variant="outline"
-          placeholder="Digite sua pergunta..."
-          // Sem borda e sem anel próprios: quem os desenha é o cartão (ver o topo do arquivo).
+          py={{ base: "11px", md: "9px" }}
+          fontSize="16px"
+          lineHeight="1.4"
+          placeholder={placeholder}
           border="none"
           bg="bg.canvas"
           color="fg"
-          _placeholder={{ color: "fg.muted" }}
+          // Placeholder numa linha só: com `autoresize`, o texto de exemplo que quebrava no celular
+          // fazia a caixa VAZIA crescer para 5 linhas.
+          _placeholder={{ color: "fg.muted", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}
           _focus={{ outline: "none", boxShadow: "none" }}
-          borderRadius="8px"
           value={prompt}
           onChange={updatePrompt}
           onKeyDown={(event) => {
@@ -89,50 +98,37 @@ export const Input = ({
               }
             }
           }}
-          width="100%"
           flex={1}
+          minW={0}
           resize="none"
           fontFamily="body"
         />
 
         <IconButton
-          borderRadius="full"
-          ml={2}
-          mt={4}
-          px={4}
-          h="48px"
-          minW="48px"
+          flex="none"
+          borderRadius="10px"
+          h={{ base: "44px", md: "40px" }}
+          minW={{ base: "44px", md: "40px" }}
           variant="solid"
           aria-label="Enviar pesquisa"
           bg="accent"
           color="accent.fg"
-          _hover={{
-            bg: "brand.600",
-            transform: "translateY(-2px)",
-          }}
-          _disabled={{
-            bg: "border",
-            cursor: "not-allowed",
-            transform: "none",
-          }}
-          _active={{
-            bg: "brand.700",
-            transform: "translateY(0)"
-          }}
-          transition="all 0.2s ease"
+          _hover={{ bg: "brand.600" }}
+          _disabled={{ bg: "border", color: "fg.disabled", cursor: "not-allowed" }}
+          _active={{ bg: "brand.700" }}
+          transition="background 0.15s ease"
           disabled={loading || !valid}
           onClick={() => callServerAPI(prompt)}
         >
-          <MdSend size={20} />
+          <MdArrowUpward size={20} />
         </IconButton>
       </Flex>
 
-      {/* Character count hint */}
-      <Flex justify="flex-end" px={2}>
-        <Box fontSize="xs" color={valid ? "accent" : "fg.muted"} fontWeight={valid ? "500" : "400"}>
-          {valid ? "Pronto para enviar" : `Mínimo ${remaining} caracteres`}
+      {curto && (
+        <Box fontSize="xs" color="fg.muted" textAlign="right" px={2} pt={1}>
+          Mínimo {remaining} caracteres
         </Box>
-      </Flex>
+      )}
     </Box>
   );
 };
diff --git a/auli-frontend/src/pages/chat/Messages.tsx b/auli-frontend/src/pages/chat/Messages.tsx
index fa4a17b..34eaab5 100644
--- a/auli-frontend/src/pages/chat/Messages.tsx
+++ b/auli-frontend/src/pages/chat/Messages.tsx
@@ -13,6 +13,8 @@ export const Messages = ({ messages, setPrompt }: MessagesProps) => {
   return (
     <Flex
       w="100%"
+      maxW="760px"
+      mx="auto"
       flexDirection="column"
       position="relative"
       bg="bg.app"
@@ -31,10 +33,13 @@ export const Messages = ({ messages, setPrompt }: MessagesProps) => {
           return (
             <SystemMessage
               key={key}
+              id={item.id}
               messageText={messageText}
               showButton={item.showButton}
               logId={item.logId}
               fontes={item.fontes}
+              tipo={item.tipo}
+              duracaoMs={item.duracaoMs}
             />
           );
         }
diff --git a/auli-frontend/src/pages/chat/SelectQuestionType.tsx b/auli-frontend/src/pages/chat/SelectQuestionType.tsx
index 4aeb430..411cdf3 100644
--- a/auli-frontend/src/pages/chat/SelectQuestionType.tsx
+++ b/auli-frontend/src/pages/chat/SelectQuestionType.tsx
@@ -1,13 +1,8 @@
-import { Box, HStack, RadioGroup, Text, Flex } from "@chakra-ui/react";
+import { Box, Flex, RadioGroup, Text, chakra } from "@chakra-ui/react";
+import { useEffect, useRef, useState } from "react";
+import { MdExpandMore } from "react-icons/md";
 import type { QuestionType } from "./utils/useQuestionType";
-
-/** O rótulo de cada tipo. Os valores são enviados (como número) no campo `type` da requisição. */
-const ROTULOS: Record<QuestionType, string> = {
-  "1": "Serviços+FAQs",
-  "2": "Pareceres",
-  "3": "Acórdãos TARF",
-  "4": "Legislação",
-};
+import { TIPOS } from "./utils/tipos";
 
 interface SelectQuestionTypeProps {
   questionType: QuestionType;
@@ -17,68 +12,113 @@ interface SelectQuestionTypeProps {
 }
 
 /**
- * Radio selector shown at the top of the chat message box: pick the kind of query to run. Themed
- * with the app's semantic tokens so the selected chip follows the accent color in both light and
- * dark mode.
+ * O seletor do tipo de consulta (D-UI-4): um botão compacto DENTRO da caixa de mensagem, à
+ * esquerda do campo, que abre a lista dos tipos com uma linha de descrição cada. Era uma faixa de
+ * chips com rótulo "SELECIONE O TIPO DE CONSULTA" acima do campo — no celular, quebrava em duas
+ * linhas e a caixa passava de 220px.
  *
- * Não tem margem horizontal própria: quem espaça é o cartão que o contém (ver `Input`).
+ * A lista é um `radiogroup` de verdade (setas, Tab e leitor de tela continuam funcionando), num
+ * painel que abre para CIMA (a caixa fica no pé da tela). Fecha ao escolher, com Esc ou com clique
+ * fora.
  */
-export const SelectQuestionType = ({
-  questionType,
-  updateQuestionType,
-  disponiveis,
-}: SelectQuestionTypeProps) => {
-  // Com uma opção só não há o que escolher, e um rádio solto lê como controle quebrado.
+export const SelectQuestionType = ({ questionType, updateQuestionType, disponiveis }: SelectQuestionTypeProps) => {
+  const [aberto, setAberto] = useState(false);
+  const raiz = useRef<HTMLDivElement>(null);
+
+  useEffect(() => {
+    if (!aberto) return;
+    const fora = (e: MouseEvent) => {
+      if (raiz.current && !raiz.current.contains(e.target as Node)) setAberto(false);
+    };
+    document.addEventListener("mousedown", fora);
+    return () => document.removeEventListener("mousedown", fora);
+  }, [aberto]);
+
+  // Com uma opção só não há o que escolher, e um seletor solto lê como controle quebrado.
   if (disponiveis.length < 2) return null;
+  const atual = TIPOS[questionType];
+
   return (
-    <Box maxW="1440px" bg="bg.subtle" borderRadius="12px" px={2} py={1} mb={2}>
-      <Text
-        fontSize="0.525rem"
+    <Box ref={raiz} position="relative" flex="none" onKeyDown={(e) => e.key === "Escape" && setAberto(false)}>
+      <chakra.button
+        type="button"
+        aria-expanded={aberto}
+        aria-label={`Tipo de consulta: ${atual.rotulo}. Trocar`}
+        onClick={() => setAberto((v) => !v)}
+        display="flex"
+        alignItems="center"
+        gap={1}
+        h={{ base: "44px", md: "40px" }}
+        pl={3}
+        pr={2}
+        borderRadius="10px"
+        bg={aberto ? "bg.resumo" : "bg.subtle"}
+        boxShadow={aberto ? "inset 0 0 0 1.5px var(--chakra-colors-accent)" : "none"}
+        color="fg"
+        fontSize={{ base: "13.5px", md: "14px" }}
         fontWeight="600"
-        color="fg.muted"
-        mt={1}
-        mb={1}
-        textTransform="uppercase"
-        letterSpacing="0.05em"
-      >
-        Selecione o tipo de consulta
-      </Text>
-      <RadioGroup.Root
-        value={questionType}
-        onValueChange={({ value }) => updateQuestionType(value)}
+        whiteSpace="nowrap"
+        cursor="pointer"
+        _hover={{ bg: "bg.resumo" }}
       >
-        <HStack gap={2} flexWrap="wrap" justify="flex-start">
-          {disponiveis.map((tipo) => {
-            const selected = questionType === tipo;
-            return (
-              <RadioGroup.Item key={tipo} value={tipo} cursor="pointer">
-                <RadioGroup.ItemHiddenInput />
-                <RadioGroup.ItemIndicator display="none" />
-                <Flex
-                  direction="column"
-                  align="flex-start"
-                  bg={selected ? "accent" : "bg.canvas"}
-                  color={selected ? "accent.fg" : "fg"}
-                  px={2}
-                  py={0.5}
-                  borderRadius="7px"
-                  border="2px solid"
-                  borderColor={selected ? "accent" : "border"}
-                  transition="all 0.2s ease"
-                  _hover={{
-                    borderColor: "accent",
-                    transform: "translateY(-1px)",
-                  }}
-                >
-                  <RadioGroup.ItemText fontWeight={selected ? "600" : "500"} fontSize="0.665rem">
-                    {ROTULOS[tipo]}
-                  </RadioGroup.ItemText>
-                </Flex>
-              </RadioGroup.Item>
-            );
-          })}
-        </HStack>
-      </RadioGroup.Root>
+        {atual.rotulo}
+        <MdExpandMore size={16} color="var(--chakra-colors-fg-muted)" />
+      </chakra.button>
+
+      {aberto && (
+        <Box
+          position="absolute"
+          bottom="calc(100% + 10px)"
+          left={0}
+          w="min(340px, calc(100vw - 32px))"
+          p={1.5}
+          bg="bg.canvas"
+          border="1px solid"
+          borderColor="border"
+          borderRadius="12px"
+          boxShadow="popover"
+          zIndex={30}
+        >
+          <RadioGroup.Root
+            aria-label="Tipo de consulta"
+            value={questionType}
+            onValueChange={({ value }) => {
+              updateQuestionType(value);
+              setAberto(false);
+            }}
+          >
+            <Flex direction="column" gap={0.5}>
+              {disponiveis.map((tipo) => {
+                const selected = questionType === tipo;
+                return (
+                  <RadioGroup.Item key={tipo} value={tipo} cursor="pointer" w="100%">
+                    <RadioGroup.ItemHiddenInput />
+                    <RadioGroup.ItemIndicator display="none" />
+                    <Flex
+                      direction="column"
+                      align="flex-start"
+                      gap={0.5}
+                      w="100%"
+                      px={3}
+                      py={2}
+                      borderRadius="8px"
+                      bg={selected ? "bg.resumo" : "transparent"}
+                      _hover={{ bg: selected ? "bg.resumo" : "bg.overlay" }}
+                    >
+                      <RadioGroup.ItemText fontSize="14.5px" fontWeight="600" color={selected ? "accent.strong" : "fg"}>
+                        {TIPOS[tipo].rotulo}
+                      </RadioGroup.ItemText>
+                      <Text fontSize="13px" color="fg.muted">
+                        {TIPOS[tipo].descricao}
+                      </Text>
+                    </Flex>
+                  </RadioGroup.Item>
+                );
+              })}
+            </Flex>
+          </RadioGroup.Root>
+        </Box>
+      )}
     </Box>
   );
 };
diff --git a/auli-frontend/src/pages/chat/SystemMessage.tsx b/auli-frontend/src/pages/chat/SystemMessage.tsx
index 1360fde..79e4993 100644
--- a/auli-frontend/src/pages/chat/SystemMessage.tsx
+++ b/auli-frontend/src/pages/chat/SystemMessage.tsx
@@ -1,128 +1,207 @@
-import { Flex, Button, Box } from "@chakra-ui/react";
-import { useState } from "react";
+import { Flex, Button, Box, Heading, Text } from "@chakra-ui/react";
+import { useCallback, useEffect, useMemo, useState } from "react";
+import type { ReactNode } from "react";
 import { Tooltip } from "./ui/tooltip";
 import { MdCopyAll, MdReceiptLong } from "react-icons/md";
 import ReactMarkdown from "react-markdown";
+import type { Components } from "react-markdown";
 import { utilsCopyTextToClipboard } from "./utils/utils";
-import { compactMarkdownComponents, markdownPlugins } from "../../shared/markdown";
+import { compactMarkdownComponents, markdownPlugins, MarkdownLink } from "../../shared/markdown";
 import { LogModal } from "./LogModal";
 import { Fontes } from "./Fontes";
+import { CitacaoSelo } from "./CitacaoSelo";
+import { dividirSecoes } from "./utils/secoes";
+import { ancora, marcarCitacoes, rotuloDoHref, rotulosCitados } from "./utils/citacoes";
+import { TIPOS } from "./utils/tipos";
+import type { QuestionType } from "./utils/useQuestionType";
 import type { Fonte } from "../../types/chat";
 
 interface SystemMessageProps {
+  /** Id da mensagem: prefixa os ids das fontes (o mesmo rótulo existe em toda resposta) e é o
+   *  alvo da rolagem do `Chat` quando a resposta chega. */
+  id?: string;
   messageText: string;
   showButton: boolean;
   /** Quando presente, oferece o ícone que abre o registro de auditoria desta resposta. */
   logId?: string;
   /** Lista de fontes recuperadas (D-SF-10); ausente ou vazia = nada é desenhado. */
   fontes?: Fonte[];
+  /** Tipo de consulta e duração — a linha de contexto no topo da resposta (D-UI-9). */
+  tipo?: QuestionType;
+  duracaoMs?: number;
 }
 
-/** Aviso exibido no rodapé de toda resposta gerada (não aparece na saudação,
- *  que chega com showButton=false). Texto validado com quem atende — manter uma frase só. */
-const DISCLAIMER =
-  "A Auli é um protótipo experimental de inteligência artificial. As respostas são geradas " +
-  "automaticamente, podem variar e conter imprecisões, e têm caráter apenas informativo — " +
-  "confira sempre as fontes e os links indicados.";
+/** Quanto tempo o realce da fonte clicada dura. */
+const REALCE_MS = 2500;
 
-export const SystemMessage = ({ messageText, showButton, logId, fontes }: SystemMessageProps) => {
+const Markdown = ({ children, components }: { children: string; components: Components }) => (
+  <Box fontSize="16px" lineHeight="1.6" color="fg" fontFamily="body" className="markdown-body">
+    <ReactMarkdown remarkPlugins={markdownPlugins} components={components}>
+      {children}
+    </ReactMarkdown>
+  </Box>
+);
+
+const Secao = ({ rotulo, children }: { rotulo: string; children: ReactNode }) => (
+  <Box as="section" aria-label={rotulo}>
+    <Heading as="h3" fontSize="15px" fontWeight="700" mb={1.5}>
+      {rotulo}
+    </Heading>
+    {children}
+  </Box>
+);
+
+export const SystemMessage = ({
+  id,
+  messageText,
+  showButton,
+  logId,
+  fontes,
+  tipo,
+  duracaoMs,
+}: SystemMessageProps) => {
   const [logAberto, setLogAberto] = useState(false);
+  const [destaque, setDestaque] = useState<string | null>(null);
+  const idBase = `msg-${id ?? "saudacao"}`;
+  const comFontes = showButton && !!fontes && fontes.length > 0;
+
+  // Citações só viram selos quando há lista de fontes para onde levar (D-UI-7).
+  const texto = useMemo(() => (comFontes ? marcarCitacoes(messageText) : messageText), [comFontes, messageText]);
+  const citados = useMemo(() => (comFontes ? rotulosCitados(messageText) : new Set<string>()), [comFontes, messageText]);
+  const secoes = useMemo(() => dividirSecoes(texto), [texto]);
+  const titulos = useMemo(() => new Map((fontes ?? []).map((f) => [f.rotulo, f.titulo])), [fontes]);
+
+  const citar = useCallback(
+    (rotulo: string) => {
+      setDestaque(rotulo);
+      document.getElementById(`${idBase}-${ancora(rotulo)}`)?.scrollIntoView({ behavior: "smooth", block: "center" });
+    },
+    [idBase],
+  );
+  useEffect(() => {
+    if (!destaque) return;
+    const t = setTimeout(() => setDestaque(null), REALCE_MS);
+    return () => clearTimeout(t);
+  }, [destaque]);
+
+  const components = useMemo<Components>(
+    () => ({
+      ...compactMarkdownComponents,
+      a: ({ href, children }) => {
+        const rotulo = rotuloDoHref(href);
+        if (rotulo && comFontes) {
+          return <CitacaoSelo rotulo={rotulo} titulo={titulos.get(rotulo)} onClick={() => citar(rotulo)} />;
+        }
+        return <MarkdownLink href={href}>{children}</MarkdownLink>;
+      },
+    }),
+    [comFontes, titulos, citar],
+  );
+
+  // A saudação e as mensagens de erro seguem como balão simples; a resposta ocupa a coluna.
+  if (!showButton) {
+    return (
+      <Flex px={3} py={2} w="100%">
+        <Box bg="bg.canvas" border="1px solid" borderColor="border" borderRadius="16px" px={4} py={3} maxW="85%">
+          <Markdown components={compactMarkdownComponents}>{messageText}</Markdown>
+        </Box>
+      </Flex>
+    );
+  }
 
   return (
-    <Flex px={3} py={2} w="100%">
+    <Flex id={idBase} px={3} py={2} w="100%" scrollMarginTop="72px">
       <Flex
-        flexDirection="column"
+        as="article"
+        direction="column"
+        gap={4}
+        w="100%"
         bg="bg.canvas"
         color="fg"
-        borderRadius="18px 18px 18px 4px"
-        padding="14px 16px 8px"
-        minW="200px"
-        maxW="85%"
-        my={0}
-        border="1px solid var(--chakra-colors-border)"
+        borderRadius="16px"
+        border="1px solid"
+        borderColor="border"
+        px={{ base: 4, md: 6 }}
+        pt={{ base: 4, md: 5 }}
+        pb={2}
       >
-        <Box
-          fontSize="16px"
-          lineHeight="1.6"
-          color="fg"
-          fontFamily="body"
-          className="markdown-body"
-        >
-          <ReactMarkdown remarkPlugins={markdownPlugins} components={compactMarkdownComponents}>
-            {messageText}
-          </ReactMarkdown>
-        </Box>
+        {tipo && (
+          <Flex align="center" gap={2} fontSize="13px" color="fg.muted">
+            <Text as="span" px={2} lineHeight="22px" borderRadius="6px" bg="bg.subtle" color="fg" fontWeight="600">
+              {TIPOS[tipo].rotulo}
+            </Text>
+            {duracaoMs !== undefined && <Text as="span">respondida em {Math.max(1, Math.round(duracaoMs / 1000))} s</Text>}
+          </Flex>
+        )}
 
-        {showButton && fontes && fontes.length > 0 && <Fontes fontes={fontes} />}
-
-        {showButton && (
-          <Box
-            mt={2}
-            pt={2}
-            borderTop="1px solid var(--chakra-colors-border)"
-            fontSize="11px"
-            lineHeight="1.4"
-            color="fg.muted"
-          >
-            {DISCLAIMER}
+        {secoes.antes && <Markdown components={components}>{secoes.antes}</Markdown>}
+
+        {secoes.resumo !== null && (
+          <Box as="section" aria-label="Resumo" bg="bg.resumo" borderRadius="12px" px={{ base: 3.5, md: 4.5 }} pt={3} pb={1.5}>
+            <Text fontSize="12px" fontWeight="700" letterSpacing="0.08em" color="accent.strong" mb={1}>
+              RESUMO
+            </Text>
+            <Markdown components={components}>{secoes.resumo}</Markdown>
           </Box>
         )}
 
-        {showButton && (
-          <Flex justify="flex-end" mt={0} gap={2}>
-            {/* Só existe quando o backend devolveu `log_id` — sem registro gravado não há o que
-                abrir, e o botão some em vez de levar a um 404. Isso já o exclui da saudação.
-                Ícone de documento + rótulo visível: o relógio com seta (MdHistory) lia como
-                "reprocessar", e um botão só de ícone não dizia o que abre. */}
-            {logId && (
-              <Tooltip content="Ver o log desta resposta" bg="bg.inverted">
-                <Button
-                  borderRadius="full"
-                  aria-label="Ver o log de auditoria desta resposta"
-                  size="xs"
-                  h="26px"
-                  px={2}
-                  gap={1}
-                  fontSize="11px"
-                  fontWeight="normal"
-                  color="fg.muted"
-                  bg="transparent"
-                  _hover={{ bg: "bg.overlay" }}
-                  transition="all 0.15s ease"
-                  onClick={() => setLogAberto(true)}
-                >
-                  <MdReceiptLong size={16} color="var(--chakra-colors-fg-muted)" />
-                  Log da consulta
-                </Button>
-              </Tooltip>
-            )}
-            <Tooltip content="Copiar resposta" bg="bg.inverted">
+        {secoes.detalhes !== null && (
+          <Secao rotulo="Detalhes">
+            <Markdown components={components}>{secoes.detalhes}</Markdown>
+          </Secao>
+        )}
+
+        {comFontes && <Fontes fontes={fontes!} citados={citados} idBase={idBase} destaque={destaque} />}
+
+        <Flex justify="flex-end" gap={1} pt={2} borderTop="1px solid" borderColor="border">
+          {/* Só existe quando o backend devolveu `log_id` — sem registro gravado não há o que
+              abrir, e o botão some em vez de levar a um 404. */}
+          {logId && (
+            <Tooltip content="Ver o log desta resposta" bg="bg.inverted">
               <Button
-                borderRadius="full"
-                aria-label="Copiar resposta para a área de transferência"
+                borderRadius="8px"
+                aria-label="Ver o log de auditoria desta resposta"
                 size="xs"
-                h="26px"
-                px={2}
-                gap={1}
-                fontSize="11px"
+                h="32px"
+                px={2.5}
+                gap={1.5}
+                fontSize="13px"
                 fontWeight="normal"
                 color="fg.muted"
                 bg="transparent"
                 _hover={{ bg: "bg.overlay" }}
-                transition="all 0.15s ease"
-                onClick={() => {
-                  utilsCopyTextToClipboard(messageText, "A resposta foi copiada para a área de transferência");
-                }}
+                onClick={() => setLogAberto(true)}
               >
-                <MdCopyAll size={16} color="var(--chakra-colors-fg-muted)" />
-                Copiar resposta
+                <MdReceiptLong size={16} color="var(--chakra-colors-fg-muted)" />
+                Log da consulta
               </Button>
             </Tooltip>
-          </Flex>
-        )}
+          )}
+          <Tooltip content="Copiar resposta" bg="bg.inverted">
+            <Button
+              borderRadius="8px"
+              aria-label="Copiar resposta para a área de transferência"
+              size="xs"
+              h="32px"
+              px={2.5}
+              gap={1.5}
+              fontSize="13px"
+              fontWeight="normal"
+              color="fg.muted"
+              bg="transparent"
+              _hover={{ bg: "bg.overlay" }}
+              onClick={() => {
+                utilsCopyTextToClipboard(messageText, "A resposta foi copiada para a área de transferência");
+              }}
+            >
+              <MdCopyAll size={16} color="var(--chakra-colors-fg-muted)" />
+              Copiar resposta
+            </Button>
+          </Tooltip>
+        </Flex>
       </Flex>
 
       {logAberto && logId && <LogModal logId={logId} onClose={() => setLogAberto(false)} />}
     </Flex>
   );
-};
\ No newline at end of file
+};
diff --git a/auli-frontend/src/pages/chat/UserMessage.tsx b/auli-frontend/src/pages/chat/UserMessage.tsx
index 76885b5..b69f0af 100644
--- a/auli-frontend/src/pages/chat/UserMessage.tsx
+++ b/auli-frontend/src/pages/chat/UserMessage.tsx
@@ -1,6 +1,6 @@
 import { Flex, Text, Button } from "@chakra-ui/react";
 import { Tooltip } from "./ui/tooltip";
-import { MdCopyAll } from "react-icons/md";
+import { MdEdit } from "react-icons/md";
 import { utilsCopyTextToClipboard } from "./utils/utils";
 import type { SetPrompt } from "../../types/chat";
 
@@ -34,7 +34,7 @@ export const UserMessage = ({ messageText, setPrompt }: UserMessageProps) => {
         </Text>
 
         <Flex justify="flex-end" mt={0}>
-          <Tooltip content="Copiar pergunta" bg="bg.inverted">
+          <Tooltip content="Reutilizar a pergunta" bg="bg.inverted">
             <Button
               borderRadius="full"
               aria-label="Copiar a pergunta para a área de entrada"
@@ -53,7 +53,7 @@ export const UserMessage = ({ messageText, setPrompt }: UserMessageProps) => {
                 );
               }}
             >
-              <MdCopyAll size={16} color="var(--chakra-colors-fg-muted)" />
+              <MdEdit size={15} color="var(--chakra-colors-fg-muted)" />
             </Button>
           </Tooltip>
         </Flex>
diff --git a/auli-frontend/src/pages/chat/utils/callServerAPI.ts b/auli-frontend/src/pages/chat/utils/callServerAPI.ts
index fef050d..af1baf0 100644
--- a/auli-frontend/src/pages/chat/utils/callServerAPI.ts
+++ b/auli-frontend/src/pages/chat/utils/callServerAPI.ts
@@ -70,6 +70,11 @@ export const callServerAPI = async ({
   });
 
   setMessages([...messagesArray]);
+  // A caixa esvazia no ENVIO, não na chegada (D-UI-2): a pergunta já está no balão do usuário,
+  // e deixá-la no campo com "Pronto para enviar" durante a espera convidava a reenviar. Quem
+  // quiser reaproveitá-la usa o botão do próprio balão.
+  setPrompt("");
+  const inicio = Date.now();
 
   const controller = new AbortController();
   const timeoutId = setTimeout(() => controller.abort(), REQUEST_TIMEOUT_MS);
@@ -95,9 +100,9 @@ export const callServerAPI = async ({
       showButton: true,
       logId: res.data?.log_id,
       fontes: res.data?.fontes,
+      tipo: questionType,
+      duracaoMs: Date.now() - inicio,
     });
-
-    setPrompt("");
   } catch (e) {
     // Distinguish: timeout (aborted) → rate limited (HTTP 429, server sends a friendly pt-BR
     // `{ error }` body) → anything else (server down / network). Without this, a throttled user is
@@ -120,8 +125,6 @@ export const callServerAPI = async ({
       text: serverMessage,
       showButton: true,
     });
-
-    setPrompt("");
   } finally {
     clearTimeout(timeoutId);
     setMessages([...messagesArray]);
diff --git a/auli-frontend/src/pages/chat/utils/citacoes.test.ts b/auli-frontend/src/pages/chat/utils/citacoes.test.ts
new file mode 100644
index 0000000..06c271c
--- /dev/null
+++ b/auli-frontend/src/pages/chat/utils/citacoes.test.ts
@@ -0,0 +1,28 @@
+import { describe, it, expect } from "vitest";
+import { ancora, marcarCitacoes, rotuloDoHref, rotulosCitados } from "./citacoes";
+
+describe("citações", () => {
+  it("vira link para a âncora da fonte, com rótulo canônico", () => {
+    expect(marcarCitacoes("Use a GA [Serviço 1] e veja [FAQ 4].")).toBe(
+      "Use a GA [Serviço 1](#fonte-servico-1) e veja [FAQ 4](#fonte-faq-4).",
+    );
+    expect(marcarCitacoes("[Servico 2]")).toBe("[Serviço 2](#fonte-servico-2)");
+  });
+
+  it("não mexe em link que o modelo já escreveu nem em outros colchetes", () => {
+    const md = "[Serviço 1](https://portal/x) e [nota]";
+    expect(marcarCitacoes(md)).toBe(md);
+  });
+
+  it("âncora e href fazem o caminho de ida e volta", () => {
+    expect(ancora("Serviço 12")).toBe("fonte-servico-12");
+    expect(rotuloDoHref("#fonte-servico-12")).toBe("Serviço 12");
+    expect(rotuloDoHref("#fonte-faq-3")).toBe("FAQ 3");
+    expect(rotuloDoHref("https://portal")).toBeNull();
+    expect(rotuloDoHref(undefined)).toBeNull();
+  });
+
+  it("lista os rótulos citados, sem repetição", () => {
+    expect([...rotulosCitados("[Serviço 1] [FAQ 2] [Serviço 1]")]).toEqual(["Serviço 1", "FAQ 2"]);
+  });
+});
diff --git a/auli-frontend/src/pages/chat/utils/citacoes.ts b/auli-frontend/src/pages/chat/utils/citacoes.ts
new file mode 100644
index 0000000..f040a01
--- /dev/null
+++ b/auli-frontend/src/pages/chat/utils/citacoes.ts
@@ -0,0 +1,50 @@
+/**
+ * Citações por afirmação (`[Serviço 2]`, `[FAQ 4]` — D-SF-7) viram selos clicáveis (D-UI-7).
+ *
+ * O texto do modelo é markdown; a citação crua é um "link de referência" sem definição, que o
+ * react-markdown desenha como texto com colchetes. Aqui ela vira um link de verdade para uma âncora
+ * `#fonte-servico-2`, e o `SystemMessage` troca esse link pelo selo. Só os rótulos de serviços e
+ * FAQs: são os únicos que o prompt pede e os únicos que a lista de fontes (D-SF-10) tem.
+ */
+const CITACAO = /\[(Servi[çc]o|FAQ)\s+(\d+)\](?!\()/gi;
+const PREFIXO = "#fonte-";
+
+/** "Servico 2" / "faq 4" → o rótulo canônico da lista de fontes: "Serviço 2", "FAQ 4". */
+function canonico(tipo: string, n: string): string {
+  return `${/^faq$/i.test(tipo) ? "FAQ" : "Serviço"} ${Number(n)}`;
+}
+
+/** "Serviço 2" → "fonte-servico-2" (sem acento nem espaço: vale como id de elemento e como URL). */
+export function ancora(rotulo: string): string {
+  return (
+    "fonte-" +
+    rotulo
+      .normalize("NFD")
+      .replace(/[\u0300-\u036f]/g, "")
+      .toLowerCase()
+      .replace(/\s+/g, "-")
+  );
+}
+
+/** O inverso de `ancora`, a partir do href do link: "#fonte-servico-2" → "Serviço 2". */
+export function rotuloDoHref(href: string | undefined): string | null {
+  if (!href?.startsWith(PREFIXO)) return null;
+  const m = /^#fonte-(servico|faq)-(\d+)$/.exec(href);
+  return m ? canonico(m[1], m[2]) : null;
+}
+
+/** Troca cada citação por um link para a âncora da fonte. Citação já seguida de `(` é link do
+ *  próprio modelo e fica como está. */
+export function marcarCitacoes(md: string): string {
+  return md.replace(CITACAO, (_, tipo: string, n: string) => {
+    const rotulo = canonico(tipo, n);
+    return `[${rotulo}](#${ancora(rotulo)})`;
+  });
+}
+
+/** Os rótulos citados no texto, canônicos. */
+export function rotulosCitados(md: string): Set<string> {
+  const vistos = new Set<string>();
+  for (const m of md.matchAll(CITACAO)) vistos.add(canonico(m[1], m[2]));
+  return vistos;
+}
diff --git a/auli-frontend/src/pages/chat/utils/fontes.test.ts b/auli-frontend/src/pages/chat/utils/fontes.test.ts
new file mode 100644
index 0000000..b907ead
--- /dev/null
+++ b/auli-frontend/src/pages/chat/utils/fontes.test.ts
@@ -0,0 +1,21 @@
+import { describe, it, expect } from "vitest";
+import { agruparFontes } from "./fontes";
+import type { Fonte } from "../../../types/chat";
+
+const f = (rotulo: string, triagem?: Fonte["triagem"]): Fonte => ({ rotulo, titulo: rotulo, link: "", triagem });
+
+describe("agruparFontes", () => {
+  it("separa citadas, consideradas e descartadas, preservando a ordem", () => {
+    const fontes = [f("Serviço 1", "aplica"), f("Serviço 2", "condicional"), f("Serviço 3", "descarta"), f("FAQ 1"), f("FAQ 2", "aplica")];
+    const g = agruparFontes(fontes, new Set(["FAQ 2", "Serviço 1"]));
+    expect(g.citadas.map((x) => x.rotulo)).toEqual(["Serviço 1", "FAQ 2"]);
+    expect(g.consideradas.map((x) => x.rotulo)).toEqual(["Serviço 2", "FAQ 1"]);
+    expect(g.descartadas.map((x) => x.rotulo)).toEqual(["Serviço 3"]);
+  });
+
+  it("citada vence descartada (não deveria acontecer, mas o texto é a verdade do que foi dito)", () => {
+    const g = agruparFontes([f("Serviço 1", "descarta")], new Set(["Serviço 1"]));
+    expect(g.citadas).toHaveLength(1);
+    expect(g.descartadas).toHaveLength(0);
+  });
+});
diff --git a/auli-frontend/src/pages/chat/utils/fontes.ts b/auli-frontend/src/pages/chat/utils/fontes.ts
new file mode 100644
index 0000000..0426b59
--- /dev/null
+++ b/auli-frontend/src/pages/chat/utils/fontes.ts
@@ -0,0 +1,25 @@
+import type { Fonte } from "../../../types/chat";
+
+/**
+ * A lista de fontes em três grupos (D-UI-8), na ordem do contexto dentro de cada um:
+ *
+ * - **citadas**: as que o texto cita — o que sustenta a resposta;
+ * - **consideradas**: chegaram à redação mas não foram citadas — inclui as `condicional`, que são
+ *   justamente a alternativa que o texto às vezes omite (REGISTRO-triagem §7);
+ * - **descartadas**: a triagem tirou da redação (e o texto não cita).
+ */
+export interface FontesAgrupadas {
+  citadas: Fonte[];
+  consideradas: Fonte[];
+  descartadas: Fonte[];
+}
+
+export function agruparFontes(fontes: Fonte[], citados: Set<string>): FontesAgrupadas {
+  const g: FontesAgrupadas = { citadas: [], consideradas: [], descartadas: [] };
+  for (const f of fontes) {
+    if (citados.has(f.rotulo)) g.citadas.push(f);
+    else if (f.triagem === "descarta") g.descartadas.push(f);
+    else g.consideradas.push(f);
+  }
+  return g;
+}
diff --git a/auli-frontend/src/pages/chat/utils/secoes.test.ts b/auli-frontend/src/pages/chat/utils/secoes.test.ts
new file mode 100644
index 0000000..2fadda3
--- /dev/null
+++ b/auli-frontend/src/pages/chat/utils/secoes.test.ts
@@ -0,0 +1,40 @@
+import { describe, it, expect } from "vitest";
+import { dividirSecoes } from "./secoes";
+
+describe("dividirSecoes", () => {
+  it("separa rótulo em negrito sozinho na linha", () => {
+    const s = dividirSecoes("**Resumo**\nUse a GA [Serviço 1].\n\n**Detalhes**\n- item");
+    expect(s).toEqual({ antes: "", resumo: "Use a GA [Serviço 1].", detalhes: "- item" });
+  });
+
+  it("aceita rótulo com dois-pontos e texto na mesma linha", () => {
+    expect(dividirSecoes("**Resumo:** Use a GA.").resumo).toBe("Use a GA.");
+    expect(dividirSecoes("**Resumo**: Use a GA.").resumo).toBe("Use a GA.");
+    expect(dividirSecoes("Resumo: Use a GA.").resumo).toBe("Use a GA.");
+  });
+
+  it("aceita título markdown", () => {
+    const s = dividirSecoes("### Resumo\nTexto.\n## Detalhes\nMais.");
+    expect(s.resumo).toBe("Texto.");
+    expect(s.detalhes).toBe("Mais.");
+  });
+
+  it("prosa que começa com a palavra NÃO é rótulo", () => {
+    const s = dividirSecoes("Resumo do caso é este aqui.");
+    expect(s).toEqual({ antes: "Resumo do caso é este aqui.", resumo: null, detalhes: null });
+  });
+
+  it("sem seções devolve tudo em `antes`", () => {
+    expect(dividirSecoes("Só texto.\n\n- lista")).toEqual({
+      antes: "Só texto.\n\n- lista",
+      resumo: null,
+      detalhes: null,
+    });
+  });
+
+  it("não abre a mesma seção duas vezes, nem dentro de bloco de código", () => {
+    const s = dividirSecoes("**Resumo**\nA.\n```\n**Detalhes**\n```\n**Resumo**\nB.");
+    expect(s.detalhes).toBeNull();
+    expect(s.resumo).toContain("**Resumo**\nB.");
+  });
+});
diff --git a/auli-frontend/src/pages/chat/utils/secoes.ts b/auli-frontend/src/pages/chat/utils/secoes.ts
new file mode 100644
index 0000000..5823490
--- /dev/null
+++ b/auli-frontend/src/pages/chat/utils/secoes.ts
@@ -0,0 +1,56 @@
+/**
+ * As seções da resposta do chat — "Resumo" e "Detalhes" — separadas do texto corrido (D-UI-5).
+ *
+ * O prompt pede as duas seções com o rótulo em negrito, e o modelo escreve de jeitos diferentes:
+ * `**Resumo**` sozinho na linha, `**Resumo:** texto` na mesma linha, `### Resumo`. Renderizado como
+ * markdown puro, o rótulo em negrito COLA no parágrafo seguinte ("**Resumo** Para apurar…") e a
+ * estrutura que o prompt pediu some. Separar as seções aqui permite desenhar o Resumo como bloco
+ * de destaque e o "Detalhes" como título — sem depender de o modelo acertar o markdown.
+ *
+ * Só é rótulo a linha em que a palavra está MARCADA (negrito, título ou dois-pontos) ou sozinha:
+ * "Resumo do caso: …" no meio do texto é prosa, não seção.
+ */
+export interface Secoes {
+  /** O que vem antes da primeira seção (normalmente vazio). */
+  antes: string;
+  resumo: string | null;
+  detalhes: string | null;
+}
+
+const ROTULO =
+  /^[ \t]*(#{1,6}[ \t]*)?(\*\*|__)?[ \t]*(resumo|detalhes)[ \t]*(:)?[ \t]*(\*\*|__)?[ \t]*(:)?[ \t]*(.*)$/i;
+
+type Nome = "antes" | "resumo" | "detalhes";
+
+export function dividirSecoes(texto: string): Secoes {
+  const partes: Record<Nome, string[]> = { antes: [], resumo: [], detalhes: [] };
+  const vistas = new Set<Nome>();
+  let atual: Nome = "antes";
+  let emCodigo = false;
+
+  for (const linha of texto.split("\n")) {
+    if (/^\s*```/.test(linha)) emCodigo = !emCodigo;
+    const m = emCodigo ? null : ROTULO.exec(linha);
+    if (m) {
+      const [, titulo, negritoA, palavra, doisPontosA, negritoB, doisPontosB, resto] = m;
+      const negrito = Boolean(negritoA && negritoB);
+      const marcado = Boolean(titulo) || negrito || Boolean(doisPontosA || doisPontosB);
+      const nome = palavra.toLowerCase() as Nome;
+      // Negrito só abre de um lado ("**Resumo texto") não é rótulo; e a seção não se repete.
+      if ((marcado || resto.trim() === "") && Boolean(negritoA) === Boolean(negritoB) && !vistas.has(nome)) {
+        atual = nome;
+        vistas.add(nome);
+        if (resto.trim() !== "") partes[atual].push(resto);
+        continue;
+      }
+    }
+    partes[atual].push(linha);
+  }
+
+  const juntar = (n: Nome) => partes[n].join("\n").trim();
+  return {
+    antes: juntar("antes"),
+    resumo: vistas.has("resumo") ? juntar("resumo") : null,
+    detalhes: vistas.has("detalhes") ? juntar("detalhes") : null,
+  };
+}
diff --git a/auli-frontend/src/pages/chat/utils/tipos.ts b/auli-frontend/src/pages/chat/utils/tipos.ts
new file mode 100644
index 0000000..29d7c9b
--- /dev/null
+++ b/auli-frontend/src/pages/chat/utils/tipos.ts
@@ -0,0 +1,64 @@
+import type { QuestionType } from "./useQuestionType";
+
+/**
+ * O texto de cada tipo de consulta, num lugar só (D-UI-9): rótulo do seletor, descrição do menu,
+ * apresentação do estado inicial, texto de exemplo do campo e as perguntas de exemplo.
+ *
+ * **Texto editorial**: as perguntas de exemplo são sugestões genéricas, não frases do portal —
+ * revisar antes de cada mudança, porque é a primeira coisa que quem chega lê.
+ */
+export interface TextoDoTipo {
+  rotulo: string;
+  descricao: string;
+  apresentacao: string;
+  placeholder: string;
+  exemplos: string[];
+}
+
+export const TIPOS: Record<QuestionType, TextoDoTipo> = {
+  "1": {
+    rotulo: "Serviços + FAQs",
+    descricao: "Qual serviço do portal se aplica ao caso",
+    apresentacao:
+      "procura nos serviços e nas perguntas frequentes do portal e indica qual se aplica ao caso — cada afirmação vem com a fonte.",
+    placeholder: "Pergunte sobre serviços e FAQs do portal…",
+    exemplos: [
+      "Empresa nova: como apurar e recolher o ICMS?",
+      "Como solicitar isenção de IPVA para pessoa com deficiência?",
+      "Como emitir a certidão de situação fiscal?",
+    ],
+  },
+  "2": {
+    rotulo: "Pareceres",
+    descricao: "Respostas oficiais a consultas tributárias",
+    apresentacao: "procura nos pareceres — as respostas oficiais a consultas sobre a legislação tributária.",
+    placeholder: "Pergunte sobre o entendimento dos pareceres…",
+    exemplos: [
+      "Incide ICMS na transferência de mercadoria entre estabelecimentos do mesmo titular?",
+      "Como fica o crédito de ICMS na aquisição de energia elétrica?",
+      "Qual o tratamento do diferencial de alíquota na compra para uso e consumo?",
+    ],
+  },
+  "3": {
+    rotulo: "Acórdãos TARF",
+    descricao: "Decisões do Tribunal Administrativo de Recursos Fiscais",
+    apresentacao: "procura nos acórdãos do Tribunal Administrativo de Recursos Fiscais.",
+    placeholder: "Pergunte como o TARF tem decidido…",
+    exemplos: [
+      "Como o TARF decide sobre multa por falta de emissão de nota fiscal?",
+      "Há acórdãos sobre glosa de crédito presumido?",
+      "O que o TARF entende sobre decadência no lançamento por homologação?",
+    ],
+  },
+  "4": {
+    rotulo: "Legislação",
+    descricao: "Leis e regulamentos, em perguntas e respostas",
+    apresentacao: "procura nas leis e regulamentos, organizados em perguntas e respostas.",
+    placeholder: "Pergunte sobre leis e regulamentos…",
+    exemplos: [
+      "Quais as hipóteses de isenção do ITCD?",
+      "O que a Lei Complementar 123 define como microempresa?",
+      "Quais penalidades se aplicam à falta de recolhimento do ICMS?",
+    ],
+  },
+};
diff --git a/auli-frontend/src/pages/home/Home.tsx b/auli-frontend/src/pages/home/Home.tsx
index f95197d..06e661f 100644
--- a/auli-frontend/src/pages/home/Home.tsx
+++ b/auli-frontend/src/pages/home/Home.tsx
@@ -160,7 +160,7 @@ const NavItem = ({
     cursor={item.disabled ? "not-allowed" : "pointer"}
     fontSize="0.9rem"
     fontFamily="body"
-    fontWeight={isActive ? "700" : "500"}
+    fontWeight={isActive ? "600" : "500"}
     textAlign="left"
     color={item.disabled ? "fg.disabled" : isActive ? "accent" : "fg.muted"}
     bg={isActive ? "bg.subtle" : "transparent"}
diff --git a/auli-frontend/src/shared/AppHeader.tsx b/auli-frontend/src/shared/AppHeader.tsx
index 3f354e3..1f28cee 100644
--- a/auli-frontend/src/shared/AppHeader.tsx
+++ b/auli-frontend/src/shared/AppHeader.tsx
@@ -1,8 +1,11 @@
-import { Box, Button, Flex, Text, VStack } from "@chakra-ui/react";
+import { Box, Button, Flex, Text } from "@chakra-ui/react";
+import { MdExpandMore } from "react-icons/md";
 import { ColorModeButton } from "../pages/chat/ui/color-mode.jsx";
 import type { Entity } from "./entities";
 
 interface AppHeaderProps {
+  /** A versão do app. Saiu da vista (D-UI-4): vira a dica do logo — era informação de quem
+   *  desenvolve ocupando a área mais nobre da tela. */
   subtitle?: string;
   /** The active entity (state), if one is selected. */
   entity?: Entity | null;
@@ -10,79 +13,67 @@ interface AppHeaderProps {
   onChangeEntity?: () => void;
 }
 
+/**
+ * Cabeçalho compacto (D-UI-4): 52px na cor de superfície, com borda — era uma faixa preta de 77px
+ * com o logo centralizado e a versão embaixo. A marca e o estado vão para a esquerda, onde a
+ * leitura começa; o estado vira um seletor que diz o que é ("SEFAZ-RS ▾"), não "RS trocar".
+ */
 export const AppHeader = ({ subtitle, entity, onChangeEntity }: AppHeaderProps) => {
   return (
     <Box
       as="header"
       w="100%"
-      py={3}
-      px={3}
-      bg="bg.inverted"
-      borderBottom="1px solid var(--chakra-colors-border-inverted)"
+      h="52px"
+      flex="none"
+      px={{ base: 2, md: 4 }}
+      bg="bg.canvas"
+      borderBottom="1px solid"
+      borderColor="border"
       position="sticky"
       top={0}
       zIndex={100}
     >
-      <Flex maxW="1440px" mx="auto" alignItems="center" justifyContent="space-between">
-        {/* Left: active-state switcher (the rs/sc selector). */}
-        <Box minW="96px" display="flex" justifyContent="flex-start">
+      <Flex h="100%" alignItems="center" justifyContent="space-between" gap={3}>
+        <Flex alignItems="center" gap={3} minW={0}>
+          <Text
+            as="span"
+            fontSize="20px"
+            fontWeight="700"
+            letterSpacing="-0.02em"
+            color="fg"
+            pl={{ base: 1, md: 0 }}
+            title={subtitle}
+            fontFamily='"SF Pro Display", system-ui, -apple-system, BlinkMacSystemFont, sans-serif'
+          >
+            Auli
+          </Text>
           {entity && onChangeEntity && (
             <Button
-              size="xs"
+              size="sm"
               variant="outline"
               onClick={onChangeEntity}
               aria-label={`Estado atual: ${entity.name}. Trocar de estado.`}
               title="Trocar de estado"
-              color="fg.inverted"
-              borderColor="border.inverted"
-              _hover={{ bg: "whiteAlpha.200" }}
-              borderRadius="full"
-              px={3}
+              h="32px"
+              px={2}
               gap={1.5}
+              borderRadius="8px"
+              borderColor="border"
+              color="fg"
+              fontWeight="500"
+              _hover={{ bg: "bg.overlay" }}
             >
-              <Box
-                as="span"
-                fontWeight="700"
-                fontSize="0.7rem"
-                bg="accent"
-                color="accent.fg"
-                borderRadius="full"
-                px={1.5}
-                py={0.5}
-                lineHeight="1"
-              >
+              <Box as="span" fontWeight="700" fontSize="11px" bg="accent" color="accent.fg" borderRadius="5px" px={1.5} lineHeight="20px">
                 {entity.uf}
               </Box>
-              <Text as="span" fontSize="0.72rem" color="fg.invertedMuted" fontWeight="500">
-                trocar
+              <Text as="span" fontSize="14px" display={{ base: "none", sm: "inline" }}>
+                {entity.name}
               </Text>
+              <MdExpandMore size={16} color="var(--chakra-colors-fg-muted)" />
             </Button>
           )}
-        </Box>
-
-        {/* Center */}
-        <VStack gap={0} align="center">
-          <Text
-            fontSize="2rem"
-            fontWeight="600"
-            color="fg.inverted"
-            lineHeight="1.2"
-            letterSpacing="-0.01em"
-            fontFamily='"SF Pro Display", system-ui, -apple-system, BlinkMacSystemFont, sans-serif'
-          >
-            Auli
-          </Text>
-          {subtitle && (
-            <Text fontSize="0.75rem" color="fg.invertedMuted" lineHeight="1" fontWeight="400">
-              {entity ? `${subtitle} · ${entity.name}` : subtitle}
-            </Text>
-          )}
-        </VStack>
-
-        {/* Right */}
-        <Box minW="96px" display="flex" justifyContent="flex-end">
-          <ColorModeButton />
-        </Box>
+        </Flex>
+        <ColorModeButton color="fg" _hover={{ bg: "bg.overlay" }} borderRadius="8px" />
       </Flex>
     </Box>
   );
diff --git a/auli-frontend/src/shared/layout.ts b/auli-frontend/src/shared/layout.ts
index 4143f62..c5ca57b 100644
--- a/auli-frontend/src/shared/layout.ts
+++ b/auli-frontend/src/shared/layout.ts
@@ -10,4 +10,4 @@
  */
 
 /** Largura da sidebar de seções (a partir do breakpoint `md`; abaixo dele vira drawer). */
-export const SIDEBAR_WIDTH = "210px";
+export const SIDEBAR_WIDTH = "224px";
diff --git a/auli-frontend/src/shared/markdown.tsx b/auli-frontend/src/shared/markdown.tsx
index df9504c..0dea789 100644
--- a/auli-frontend/src/shared/markdown.tsx
+++ b/auli-frontend/src/shared/markdown.tsx
@@ -1,3 +1,4 @@
+import type { ReactNode } from "react";
 /**
  * Shared ReactMarkdown component maps. Centralizes the renderers that were
  * duplicated across the chat bubble, the FAQ answers, and the About page —
@@ -60,7 +61,9 @@ export function toAbsoluteHref(href?: string): string | undefined {
   return `http://${href.replace(/^\/+/, "")}`;
 }
 
-const MarkdownLink: Components["a"] = ({ href, children }) => (
+/** Exportado para o `SystemMessage`, que troca o `a` do mapa compacto pelo selo de citação e cai
+ *  neste para os demais links. */
+export const MarkdownLink = ({ href, children }: { href?: string; children?: ReactNode }) => (
   <a
     href={toAbsoluteHref(href)}
     target="_blank"
@@ -75,8 +78,10 @@ export const compactMarkdownComponents: Components = {
   ...tableComponents,
   a: MarkdownLink,
   p: ({ children }) => <p style={{ marginBottom: "0.5em" }}>{children}</p>,
-  ul: ({ children }) => <ul style={{ paddingLeft: "1.2em", marginBottom: "0.5em" }}>{children}</ul>,
-  ol: ({ children }) => <ol style={{ paddingLeft: "1.2em", marginBottom: "0.5em" }}>{children}</ol>,
+  // `listStyleType` explícito: o reset do Chakra zera o `list-style`, e sem ele os itens viravam
+  // parágrafos recuados soltos (D-UI-1).
+  ul: ({ children }) => <ul style={{ paddingLeft: "1.3em", marginBottom: "0.5em", listStyleType: "disc" }}>{children}</ul>,
+  ol: ({ children }) => <ol style={{ paddingLeft: "1.3em", marginBottom: "0.5em", listStyleType: "decimal" }}>{children}</ol>,
   li: ({ children }) => <li style={{ marginBottom: "0.25em" }}>{children}</li>,
 };
 
@@ -98,8 +103,8 @@ export const proseMarkdownComponents: Components = {
     </h3>
   ),
   p: ({ children }) => <p style={{ marginBottom: "0.85em" }}>{children}</p>,
-  ul: ({ children }) => <ul style={{ paddingLeft: "1.4em", marginBottom: "0.75em" }}>{children}</ul>,
-  ol: ({ children }) => <ol style={{ paddingLeft: "1.4em", marginBottom: "0.75em" }}>{children}</ol>,
+  ul: ({ children }) => <ul style={{ paddingLeft: "1.4em", marginBottom: "0.75em", listStyleType: "disc" }}>{children}</ul>,
+  ol: ({ children }) => <ol style={{ paddingLeft: "1.4em", marginBottom: "0.75em", listStyleType: "decimal" }}>{children}</ol>,
   li: ({ children }) => <li style={{ marginBottom: "0.3em" }}>{children}</li>,
   a: MarkdownLink,
   hr: () => <hr style={{ border: "none", borderTop: `1px solid var(--chakra-colors-border)`, margin: "1.5em 0" }} />,
diff --git a/auli-frontend/src/theme/system.js b/auli-frontend/src/theme/system.js
index 68c37d7..ffe2ec1 100644
--- a/auli-frontend/src/theme/system.js
+++ b/auli-frontend/src/theme/system.js
@@ -44,7 +44,11 @@ const config = defineConfig({
         // ── semantic: what components reference; { light, dark } ──
         bg: {
           canvas: { value: { base: "{colors.canvas}", _dark: "#1d1d1f" } },
-          app: { value: { base: "{colors.parchment}", _dark: "#000000" } },
+          // Quase-preto no escuro (era #000000): com o preto puro as superfícies (`canvas` #1d1d1f)
+          // flutuavam sem degrau, e o contraste máximo cansava na leitura longa (D-UI-4).
+          app: { value: { base: "{colors.parchment}", _dark: "#111113" } },
+          // O bloco do Resumo da resposta do chat (D-UI-6): um tom do azul da marca, nos dois modos.
+          resumo: { value: { base: "#f3f7fd", _dark: "#15253a" } },
           subtle: { value: { base: "{colors.neutral.100}", _dark: "#272729" } },
           inverted: { value: { base: "#000000", _dark: "#000000" } }, // header stays black both modes
           // translucent hover overlay that reads correctly on any surface
@@ -95,12 +99,27 @@ const config = defineConfig({
         accent: {
           DEFAULT: { value: { base: "{colors.brand.500}", _dark: "{colors.brand.400}" } },
           fg: { value: { base: "#ffffff", _dark: "#ffffff" } },
+          // Texto pequeno em azul (o rótulo RESUMO): o brand.500 fica no limite do contraste em
+          // 12px sobre `bg.resumo`; o 700 passa com folga.
+          strong: { value: { base: "{colors.brand.700}", _dark: "{colors.brand.400}" } },
         },
         bubble: {
           user: { value: { base: "{colors.bubble}", _dark: "#1e3a5f" } },
         },
+        // Selo "depende de condição" das fontes (D-UI-8).
+        condicional: {
+          bg: { value: { base: "#fff3e0", _dark: "#3a2a12" } },
+          fg: { value: { base: "#8a4600", _dark: "#ffcc80" } },
+        },
       },
       shadows: {
+        // A caixa de mensagem descolada do fundo (D-UI-4) e o painel que abre sobre ela.
+        composer: {
+          value: { base: "0 2px 10px rgba(15,23,42,0.06)", _dark: "0 2px 10px rgba(0,0,0,0.35)" },
+        },
+        popover: {
+          value: { base: "0 10px 30px rgba(15,23,42,0.16)", _dark: "0 10px 30px rgba(0,0,0,0.5)" },
+        },
         // accent-colored focus ring for text inputs
         focusRing: {
           value: {
diff --git a/auli-frontend/src/types/chat.ts b/auli-frontend/src/types/chat.ts
index 9eb6ef6..176c068 100644
--- a/auli-frontend/src/types/chat.ts
+++ b/auli-frontend/src/types/chat.ts
@@ -1,4 +1,5 @@
 import type { Dispatch, SetStateAction } from "react";
+import type { QuestionType } from "../pages/chat/utils/useQuestionType";
 
 /** Who authored a chat message. */
 export type MessageSender = "user" | "server";
@@ -29,6 +30,10 @@ export interface Message {
   /** O lugar da resposta enquanto a consulta roda: desenhado como a animação `Pensando`, não
    *  como texto. O `text` segue preenchido para quem não desenha (e para os testes). */
   pendente?: boolean;
+  /** Tipo de consulta que produziu esta resposta (D-UI-9) — a linha de contexto no topo dela. */
+  tipo?: QuestionType;
+  /** Quanto a resposta levou, medido no navegador do envio à chegada (D-UI-9). */
+  duracaoMs?: number;
 }
 
 /** Setter returned by `usePrompt`, matching React's `useState<string>` setter. */
-- 
2.43.0

~~~~~
