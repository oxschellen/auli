# ESTUDO-frontend-dependencies-update — Atualização das dependências do `auli-frontend`

**Data:** 05/11/2026 · **Status:** medido em clone limpo do `main` (`62e5b73`, PR #192); patch pronto,
aguarda `git am` + conferência local.
**Escopo:** só `auli-frontend/package.json` e `package-lock.json`. Nenhuma linha de `src/` muda.

**Convenção:** como nos demais estudos, distingo **medido** (saiu de execução real neste clone),
**deduzido** (inferência com a evidência ao lado) e **não verificado** (fica para a §8).

---

## 1. A pergunta e a resposta curta

**Pergunta:** o que dá para atualizar hoje no frontend sem quebrar nada, e o que fica para depois?

**Resposta curta:** dá para atualizar **tudo menos dois pacotes**. As 14 atualizações compatíveis
(semver) e 4 das 6 majors passam em `tsc`, `eslint`, os 274 testes e o `vite build`, sem mudança
de código. O `npm audit` cai de **20 vulnerabilidades (9 altas)** para **7 moderadas**, todas no
mesmo `minimatch` interno do typescript-eslint (ferramenta de lint, nunca vai ao navegador).
Ficam de fora **ESLint 10** (o `eslint-plugin-react` ainda não o suporta — falha dura, §5.5) e
**TypeScript 7** (o compilador nativo em Go, publicado hoje; o typescript-eslint o recusa
explicitamente, §5.6). Nenhum dos dois é bloqueio de segurança ou de produto.

## 2. Linha de base (medido)

Clone limpo, `npm ci`, Node 22.23.4 / npm 10.9.9 (o CI usa `node-version: "22"`):

| Verificação | Resultado |
|---|---|
| `tsc --noEmit` | ok |
| `eslint` (`--max-warnings 0`) | ok |
| `vitest run` | 32 arquivos, **274 testes** passam, 6 pulados |
| `vite build` | ok, 1,08 s |
| `npm audit` | **20 vulnerabilidades** (11 moderadas, 9 altas) |
| `npm outdated` | 20 pacotes atrás do `latest`; 6 deles por **major** |

Diretas vulneráveis na base: `vite` 8.2.2 (alta — 3 CVEs do dev server + 3 do `launch-editor`),
`axios` 1.19.0 (alta, via `follow-redirects`), `vitest`/`@vitest/mocker`, `react-doctor` (via
`deslop-js`/`minimatch`), `typescript-eslint` (via `minimatch`). Transitivas altas: `zod`,
`undici`, `fast-uri`, `js-yaml`, `brace-expansion`, `source-map-js`.

Tamanho dos bundles na base (gzip): react 56,3 kB · chakra 100,1 kB · utils 79,4 kB · index 61,0 kB.

## 3. Método

Cada candidato a major foi testado **isolado**, sempre partindo do mesmo lockfile da Fase 1
(restaurado entre tentativas), com o ciclo completo `typecheck → lint → test → build → audit`.
Depois o conjunto aprovado foi testado **junto**.

**Armadilha encontrada (medido):** `npm update` e `npm install vitest@…` no npm **10.9.9** morrem
com `TypeError: Cannot read properties of null (reading 'edgesOut')` em `arborist/build-ideal-tree.js`
ao resolver o conjunto de peers do vitest. Com **npm 11** (`npx -y npm@11 install …`) funciona.
Rodar com `--legacy-peer-deps` no npm 10 "funciona" mas **remove `@testing-library/dom`** (peer do
`@testing-library/react`) e os 32 arquivos de teste falham com `Cannot find module` — portanto
**não** usar `--legacy-peer-deps` aqui. Se o Desktop Server estiver em npm 10, o caminho é
`npx -y npm@11 ci` ou simplesmente aplicar o patch (o lockfile já vem resolvido) e rodar `npm ci`.

## 4. Fase 1 — compatíveis (semver) + `npm audit fix`

### 4.1 O que sobe

| Pacote | De | Para | Tipo |
|---|---|---|---|
| `@chakra-ui/react` | 3.36.1 | **3.38.0** | runtime |
| `axios` | 1.19.0 | **1.21.0** | runtime |
| `react`, `react-dom` | 19.2.8 | **19.3.1** | runtime |
| `@testing-library/react` | 16.3.2 | 16.3.3 | dev |
| `@types/estree` | 1.0.9 | 1.0.10 | dev |
| `@types/node` | 26.2.0 | 26.6.6 | dev |
| `@types/react`, `@types/react-dom` | 19.2.x | 19.3.0 | dev |
| `@vitejs/plugin-react` | 6.1.0 | 6.2.1 | dev |
| `eslint-plugin-react-refresh` | 0.5.4 | 0.5.7 | dev |
| `react-doctor` | 0.9.12 | 0.9.22 | dev |
| `typescript-eslint` | 8.67.0 | 8.72.1 | dev |
| `vite` | 8.2.2 | **8.3.6** | dev (fecha as 6 CVEs do dev server) |
| `vitest` | 4.1.11 | 4.1.12 | dev |

`npm audit fix` (sem `--force`) por cima: `follow-redirects` 1.16.0→1.16.2, `undici` 7.29→7.30.2,
`fast-uri` 3.1.5→3.1.9, `js-yaml` 4.3.1→4.3.2, `zod` →4.6.7, `brace-expansion` 1.1.18→1.1.21.
Só lockfile; `package.json` não muda nesse passo.

### 4.2 Resultado (medido)

Tudo verde: tsc ok · lint ok · 274 testes · build 1,13 s · `npm ls` sem `invalid/missing`.
`npm audit`: **20 → 7 moderadas** — todas o `minimatch` 10.2.6 dentro de
`@typescript-eslint/typescript-estree` (GHSA-g3hv-75wg-m249, DoS por glob; a faixa corrigida é
≥11.0.1 e o typescript-estree ainda pede `^10`). Não há `fix available` sem trocar o
typescript-eslint de major, e a exposição é um processo de lint local lendo globs do próprio
`eslint.config.js`. **Aceitar.**

Bundles (gzip): react 56,3 → **64,8 kB** (+8,5; React 19.3) · chakra 100,1 → **104,3 kB** (+4,2) ·
utils 79,4 → 82,9 kB (+3,4) · index 61,0 kB (igual). Total gzip ≈ +16 kB (≈ +5%). É o preço de
React 19.3 e Chakra 3.38; não há como escolher só a parte de segurança.

## 5. Fase 2 — as seis majors

| Pacote | De → Para | Veredito | Evidência |
|---|---|---|---|
| `@testing-library/jest-dom` | 6.9.1 → **7.0.1** | **sobe** | §5.1 |
| `jsdom` | 29.1.1 → **30.1.3** | **sobe** (exige Node ≥ 22.22.2) | §5.2 |
| `vitest` | 4.1.12 → **5.1.0** | **sobe** | §5.3 |
| `framer-motion` | 12.43.0 → **14.9.1** | **sobe**, com smoke visual (§8) | §5.4 |
| `eslint` | 9.39.5 → 10.14.0 | **não agora** — falha dura | §5.5 |
| `typescript` | 6.0.3 → 7.0.2 | **não agora** — recusado pelo typescript-eslint | §5.6 |

### 5.1 `@testing-library/jest-dom` 7 — sobe

Peers: `vitest >= 0.32`, `@testing-library/dom >=10 <11`, Node ≥ 22 — todos satisfeitos. O
`src/test/setup.ts` já importa `@testing-library/jest-dom/vitest`, que é o caminho mantido na v7.
**Medido:** isolado, 274 testes passam, zero mudanças.

### 5.2 `jsdom` 30 — sobe, com uma exigência de ambiente

**Medido:** isolado, 274 testes passam. `jsdom@30.1.3` declara `engines.node = ^22.22.2 || ^24.15.0
|| >=26`. O CI (`node-version: "22"`) pega o 22 mais novo e passa. **Não verificado:** a versão de
Node do Desktop Server e do VPS — conferir `node -v` antes do `npm ci` (§8). Se for < 22.22.2, o
`npm ci` avisa (`EBADENGINE`) mas instala; a opção segura é subir o Node, não segurar o jsdom.
Efeito colateral bom: traz `undici` 7.30.2 (fecha as 3 CVEs de DoS do `undici` da base).

### 5.3 `vitest` 5 — sobe

Peers: `vite ^6.4||^7||^8` (temos 8.3.6), `@types/node ^22||>=24` (temos 26.6.6), Node ≥ 22.12.
**Medido:** isolado, 274 testes passam em 66,8 s (mesma ordem da v4: 66,9 s); `vite.config.js`
continua válido com o bloco `test:` (sem aviso de depreciação no log). Fecha a vulnerabilidade
moderada do `@vitest/mocker` (já fechada pela 4.1.12, mas a 5 a mantém fechada).

### 5.4 `framer-motion` 14 — sobe, único major de **runtime**

O que mudou nas majors ([guia oficial](https://motion.dev/docs/react-upgrade-guide)):

- **v13:** deixou de embutir `@emotion/is-prop-valid`; afeta quem compõe `styled(motion.div)` ou
  `chakra(m.div)`, que pode passar a vazar props para o DOM. **Medido:** o Auli usa só `<m.div>`
  direto (6 arquivos: `App.tsx` com `LazyMotion`/`domAnimation`, e os 5 accordions/listas com `m`,
  `AnimatePresence`, `useReducedMotion`); nenhum `styled(`/`chakra(`/`motion.create` por cima.
  **Deduzido:** não atinge.
- **v14:** "There are no breaking changes in Motion for React in version 14" — só depreciação de
  APIs internas e versionamento mais estrito entre `motion-dom`/`motion-utils`.

**Medido:** isolado, tsc ok, 274 testes, build ok; `vendor-utils` 82,9 → **86,8 kB** gzip (+3,9).
O que os testes **não** cobrem é o visual da animação (expandir/recolher accordion, `mode` do
`AnimatePresence`, redução de movimento) — por isso o smoke manual da §8.

Nota: o pacote `framer-motion` continua publicado e mantido (14.9.1 é de 04/11/2026); a migração
para `motion/react` é opcional e **não** faz parte deste estudo.

### 5.5 `eslint` 10 — não agora (falha dura, não é só peer)

Peers: `eslint-plugin-react@7.37.5` declara `eslint: … || ^9.7` — último release em **abr/2025**,
sem `next`/`canary` com suporte a 10. `eslint-plugin-react-hooks` 7.1.1, `eslint-plugin-react-refresh`
0.5.7 e `typescript-eslint` 8.72.1 já aceitam `^10`.

**Medido:** com peer estrito, ERESOLVE. Forçando (`--legacy-peer-deps`), o lint **quebra**:
`TypeError: Error while loading rule 'react/display-name': contextOrFilename.getFilename is not a
function` — o ESLint 10 removeu `context.getFilename()` e o plugin ainda o chama. Não é incompatibilidade
declarativa; é código.

**Tensão real:** o npm já marca `eslint@9.39.5` como "no longer supported". Três saídas, nenhuma
para este patch:

1. **Esperar** um `eslint-plugin-react` compatível (mais barato; o 9 continua funcionando — lint é
   ferramenta local e de CI, não vai ao navegador).
2. Trocar para `@eslint-react/eslint-plugin` (5.28.8, `eslint: *`, ativo) — muda o conjunto de
   regras; exige revisar `eslint.config.js` e re-rodar o lint em todo o `src/`. Estudo à parte.
3. Remover o `eslint-plugin-react` (projeto em TS estrito perde pouco: `jsx-key`,
   `no-unknown-property`, `display-name`…). Não recomendo sem medir o que ele pega hoje.

### 5.6 `typescript` 7 — não agora (publicado hoje; recusado explicitamente)

`typescript@7.0.2` (05/11/2026, 08:20 UTC) é o **compilador nativo** (porte em Go): o pacote npm
virou um *launcher* de 3,6 MB cujo `lib/` tem só `getExePath.js`/`tsc.js`, e o binário vem de
`@typescript/typescript-<plataforma>` (18 pacotes por plataforma como `dependencies`).

**Medido:** `typescript-eslint@8.72.1` declara `typescript: >=4.8.4 <6.1.0` → ERESOLVE. Forçando,
o `tsc --noEmit` roda (2,7 s) mas o `eslint` morre com `Error: typescript-eslint does not support
TS 7.0`. **Deduzido:** mesmo o TS **6.1+** está fora da faixa do typescript-eslint hoje; `6.0.3` é
o teto. Reavaliar quando o typescript-eslint publicar suporte (acompanhar o changelog; o 8.x saiu
em 02/11 ainda com `<6.1.0`).

### 5.7 Conjunto aprovado, testado junto (medido)

Fase 1 + jest-dom 7 + jsdom 30 + vitest 5 + framer-motion 14: tsc ok · lint ok · **274 testes**
passam · build 1,03–1,13 s · `npm ls` limpo · audit **7 moderadas** · `npm outdated` lista só
`eslint` e `typescript`.

Bundles finais (gzip): react **64,8** · chakra **104,3** · utils **86,8** · index **61,0** · icons 4,7
· runtime 0,4 — total ≈ **322 kB** contra ≈ 302 kB na base (**+20 kB, +6,6%**).

## 6. O patch

Um commit, dois arquivos: `auli-frontend/package.json` (abaixo) e `auli-frontend/package-lock.json`
(resolvido com npm 11; ~3.100 linhas de diff, é só o lock). Sem mudança em `src/`, `vite.config.js`,
`eslint.config.js` ou `tsconfig.json`.

```diff
-    "@chakra-ui/react": "^3.33.0",
+    "@chakra-ui/react": "^3.38.0",
-    "axios": "^1.13.5",
-    "framer-motion": "^12.34.1",
+    "axios": "^1.21.0",
+    "framer-motion": "^14.9.1",
-    "react": "^19.2.4",
-    "react-dom": "^19.2.4",
+    "react": "^19.3.1",
+    "react-dom": "^19.3.1",
-    "@testing-library/jest-dom": "^6.9.1",
-    "@testing-library/react": "^16.3.2",
+    "@testing-library/jest-dom": "^7.0.1",
+    "@testing-library/react": "^16.3.3",
-    "@types/estree": "^1.0.8",
+    "@types/estree": "^1.0.10",
-    "@types/node": "^26.2.0",
-    "@types/react": "^19.2.14",
-    "@types/react-dom": "^19.2.3",
-    "@vitejs/plugin-react": "^6.0.2",
+    "@types/node": "^26.6.6",
+    "@types/react": "^19.3.0",
+    "@types/react-dom": "^19.3.0",
+    "@vitejs/plugin-react": "^6.2.1",
-    "eslint-plugin-react-refresh": "^0.5.0",
-    "jsdom": "^29.1.1",
-    "react-doctor": "^0.9.12",
+    "eslint-plugin-react-refresh": "^0.5.7",
+    "jsdom": "^30.1.3",
+    "react-doctor": "^0.9.22",
-    "typescript-eslint": "^8.60.0",
-    "vite": "^8.0.13",
-    "vitest": "^4.1.8"
+    "typescript-eslint": "^8.72.1",
+    "vite": "^8.3.6",
+    "vitest": "^5.1.0"
```

Ficam como estão (deliberado): `eslint ^9.39.4`, `typescript ^6.0.3`, `eslint-plugin-react 7.37.5`
(pinado), `next-themes 0.4.6` (pinado), `@emotion/*`, `react-icons`, `react-markdown`, `remark-gfm`,
`swr`, `eslint-plugin-react-hooks`, `@types/babel__*`, `@types/json-schema` — nenhum estava atrás.

## 7. Como aplicar (no `~/Desktop/auli`)

```bash
git am 0001-frontend-atualizar-dependencias.patch
cd auli-frontend
node -v                      # §5.2: jsdom 30 quer ≥ 22.22.2
npm ci                       # instala exatamente o lock do patch; se o npm for 10 e reclamar: npx -y npm@11 ci
npm run lint && npm test && npm run build
npm audit                    # esperado: 7 moderate (minimatch do typescript-eslint)
```

Depois, PR normal — o workflow `frontend` roda os mesmos quatro passos. O `deploy-frontend.sh`
não muda (passo 5 continua `npm run build`). Bump de versão do `package.json` (`0.1.67` → `0.1.68`)
fica a seu critério; o `__APP_VERSION__` sai dali.

## 8. O que não foi verificado (fazer antes do deploy)

1. **Smoke visual do framer-motion 14** no `npm run dev`: abrir/fechar um item em cada accordion
   (Pareceres, Serviços, Conteúdos, FAQs, Legislação), ver se a saída (`AnimatePresence`) anima e
   se o `useReducedMotion` segue respeitado com "reduzir movimento" ativo no SO. É o único major de
   runtime e a única parte sem teste automatizado.
2. **`node -v` no Desktop Server e no VPS** (jsdom 30; §5.2). O CI já satisfaz.
3. **Dev server/HMR** com vite 8.3.6 + plugin-react 6.2.1: abrir, editar um `.tsx`, ver o refresh.
   O build de produção foi medido; o modo dev não.
4. `npm run doctor` usa `npx react-doctor@latest` (ignora a versão instalada) — não foi rodado e
   não é afetado pelo patch.
5. A **pasta `dist/`** gerada aqui não foi comparada visualmente com a atual (só tamanho/sucesso).

## 9. Observações laterais (não tocar neste patch)

- **`@types/babel__core` e os três `@types/babel__*`** não são exigidos por nada em `node_modules`
  (medido pelo lock: só a raiz os pede; os três menores são puxados pelo `__core`). Parecem resto da
  era do plugin-react em Babel (hoje é oxc). `@types/estree` e `@types/json-schema` ainda são pedidos
  por `eslint`/`@eslint/core`. Remover os quatro `babel__*` é tarefa separada, com `tsc` para provar.
- O npm 10.9.9 do Node 22 tem o bug do arborist descrito na §3; vale ter `npm@11` à mão
  (`npm i -g npm@11`) para as próximas atualizações.
- O `eslint@9` já consta como fora de suporte no registro. Não é urgente (ferramenta local), mas a
  decisão da §5.5 (esperar × `@eslint-react` × remover) vai se impor em algum momento.
