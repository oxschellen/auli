# Auli Frontend — Project Description

## Overview

**Auli** is an open, cooperative pilot project: an AI-powered virtual assistant
that helps Brazilian public servants answer citizens' questions about **state
taxes** (impostos estaduais), with a focus on Rio Grande do Sul (RS).

This repository is the **frontend** — a single-page React application. The user
asks questions in natural language and receives AI-generated answers, plus
several browsable reference sections (Services, FAQs, Legal Opinions, and Tax
Notes). The UI is in **Brazilian Portuguese** and is **mobile-first**.

The app is **multi-tenant by state**: the user picks a state tax authority
(*entity*) on a map, and everything picked under it — every data file and the
chat — is scoped to that entity. Several states can be open at once, each in its
own **state tab** in the header, like browser tabs. All 27 UFs have an entity
today, each with the collections scraped for it so far. See
[Multi-tenant state selection](#multi-tenant-state-selection).

- **App name (package):** `auli-ui`
- **UI version:** taken from `package.json` `version` — injected at build time
  via the `__APP_VERSION__` define, so it can't drift from a hardcoded constant —
  and shown as the tooltip of the "Auli" logo.
- **Backend API:** `https://api.auli.com.br/v1/question` (POST `{ question, entity? }`
  → `{ answer }`) — `entity` is the selected state id, so the backend can query
  the right tenant's collections (override the URL with `VITE_API_URL`).

---

## Tech Stack

| Concern | Choice |
| --- | --- |
| Language | **TypeScript (strict)** — all app code is `.ts`/`.tsx`; the only JavaScript left is the color token system, `theme/system.js` |
| UI library | **React 19** |
| Build tool / dev server | **Vite 8** (Rolldown-based) with `@vitejs/plugin-react` |
| Component library | **Chakra UI v3** (+ `@emotion/react`) |
| Navigation | **None** — single-page **tabbed** shell (no router) |
| Data fetching | **SWR** + **Axios** |
| Animation | **Framer Motion** (`LazyMotion` / `domAnimation`) |
| Icons | **react-icons** (Material Design set) |
| Markdown | **react-markdown** |
| Theming | **next-themes** + a custom Chakra **semantic-token system** — full light/dark mode (see [THEME.md](THEME.md)) |
| Testing | **Vitest** + **@testing-library/react** + **jsdom** |

Linting via **ESLint 9** (flat config) with React, React Hooks, React Refresh,
and `typescript-eslint` plugins. `react-hooks/exhaustive-deps` is enforced as an
error. Formatting is **Prettier** ([.prettierrc.json](.prettierrc.json): defaults
plus `printWidth: 100`), checked in CI; the generated `entities.ts` and
`brazilPaths.ts` are in [.prettierignore](.prettierignore).

---

## How to Run

```bash
npm install       # install dependencies
npm run dev       # start Vite dev server
npm run build     # type-check (tsc --noEmit) then production build
npm run preview   # preview the production build
npm run lint      # ESLint over js/jsx/ts/tsx (fails on warnings)
npm run format    # Prettier --write over src/ and the root *.js configs
npm run format:check # what CI runs
npm run typecheck # tsc --noEmit only
npm test          # run the Vitest suite once
npm run test:watch# Vitest in watch mode
```

---

## Application Architecture

### Entry & shell

- [index.html](index.html) loads [src/main.tsx](src/main.tsx), which mounts
  `<App />` inside `React.StrictMode`.
- [src/app/App.tsx](src/app/App.tsx) wraps everything in (outermost first):
  - `Provider` (Chakra UI + color mode, from [src/ui/provider.tsx](src/ui/provider.tsx)),
  - a single global `Toaster`,
  - [`ErrorBoundary`](src/app/ErrorBoundary.tsx),
  - `LazyMotion` (Framer Motion),
  - then `AppShell`: a full-height `Flex` column with the shared
    [`AppHeader`](src/app/AppHeader.tsx), which carries the state tabs, pinned
    above one panel per tab. A tab shows the
    [`StateSelection`](src/secoes/mapa/StateSelection.tsx) map until it
    has a state, then that state's [`Home`](src/secoes/home/Home.tsx) inside its
    own [`EntityProvider`](src/contexto/EntityContext.tsx). Inactive tabs stay
    mounted and hidden, so each keeps its conversation, section and scroll.

There is **no router** — the app is a single page. Sections are tabs inside
`Home`, not routes, so the URL stays at `/`. Conversations, searches and scroll
live in memory only. What persists, in `localStorage` only: the open state tabs
(`auli.abas`), the question type, whether the sidebar and the "Minhas perguntas"
panel are open, and each state's "Minhas perguntas" list (`auli.historico.<id>`).
A reload returns to the same tabs. All of it goes through
[src/lib/storage.ts](src/lib/storage.ts), and other browser windows of the app
follow along (the `storage` event).

### Multi-tenant state selection

Each state tab is scoped to one **entity** (state tax authority), chosen once on
its map, so chat history, search and scroll never need re-scoping. To look at
another state, the user opens another tab; a state opens in one tab only.

- [src/contexto/entities.ts](src/contexto/entities.ts) — the entity registry (a
  frontend mirror of the scraper's `domain::entities`). Each `Entity` has `id`
  (also the `public/<id>/` data folder and the backend tenant key), `name`
  (`SEFAZ-RS`), `uf` (`RS`), `state` (`Rio Grande do Sul`), and **`collections`**
  — the list of sections it actually has data for. `hasCollection(entity, kind)`
  drives the per-section empty state. Generated from `config/registry.toml` by
  `scripts/tools/gen-frontend-entities.mjs` — do not hand-edit.
- [src/app/abas.ts](src/app/abas.ts) — the state tabs: open, close, pick a
  state, persisted to `localStorage` under `auli.abas` (the open states in order,
  and the active one). The pre-tabs `auli.entity` key is migrated into the first
  tab once. Ids the registry no longer knows, and repeats, are dropped.
- [src/app/AbasDeEstado.tsx](src/app/AbasDeEstado.tsx) — the tab strip next
  to the logo, `[RS SEFAZ-RS ×] [SP SEFAZ-SP ×] [+]`: WAI-ARIA tabs (arrow keys,
  Delete closes), "+" opens a tab on the map. A tab whose chat is waiting for an
  answer shows a spinner, and one whose answer arrived while it was hidden shows a
  dot until it is opened — both in the ×'s spot, so the tab keeps its width; with
  a mouse, hovering brings the × back. Reported through
  [RespostaDaAba.tsx](src/contexto/RespostaDaAba.tsx).
- [src/contexto/EntityContext.tsx](src/contexto/EntityContext.tsx) — `EntityProvider`,
  one per tab, with that tab's entity; `useSelectedEntity()` reads it.
- [src/secoes/mapa/StateSelection.tsx](src/secoes/mapa/StateSelection.tsx)
  — the map page ("Escolha o estado") of a tab with no state yet: an interactive
  **Brazil map**; picking a state calls the tab's `onSelect`.
- [src/secoes/mapa/BrazilMap.tsx](src/secoes/mapa/BrazilMap.tsx) —
  an outlined SVG map of all 27 states. States backed by an available entity (all
  27 today) are filled with the accent color and are clickable/keyboard-operable (`role=button`,
  `<title>` tooltip); every other state is greyed (`bg.mapInactive`) and inert.
  Colors resolve via `useToken` and native `<svg>`/`<path>` elements are used (Chakra's
  `Box as="path"` drops the `d`/`viewBox` attributes). Geometry comes from
  [brazilPaths.ts](src/secoes/mapa/brazilPaths.ts) — auto-generated compact
  per-UF path strings (GeoJSON simplified via Douglas-Peucker, projected to a
  1000×1000 viewBox; ~33 kB, do not hand-edit). The map lights up any UF that has an
  entity in the registry.

### Home tab shell

Three files, by what changes them:

- [src/secoes/registro.ts](src/secoes/registro.ts) — the sections as data: the
  17 `SECOES` (id, label, icon, component, required collection, group), the four
  `GRUPOS` (top, Acervo, Tributum, Integrações), and `secoesDe(entity)`, which
  keeps every section visible and disables the ones whose collection the entity
  lacks. A new section is an entry here.
- [src/secoes/home/Navegacao.tsx](src/secoes/home/Navegacao.tsx) — the list by
  groups (`NavList`), used by the sidebar and by the mobile drawer, following the
  WAI-ARIA vertical tabs pattern (roving `tabIndex`; arrow keys move to the next
  *enabled* section, wrapping around, via `proximaHabilitada`), and the scroll
  cue of both containers.
- [src/secoes/home/Home.tsx](src/secoes/home/Home.tsx) — the active section, the
  sidebar (collapsible into an icon rail from `md`), the drawer below `md`, and
  the panels. A panel is **mounted on first activation and then kept mounted**
  (toggled with `display: none`) so each section preserves its state — scroll
  position, search query, chat history — across switches, and megabyte-sized
  section data is fetched only once the section is first opened.

### Scrolling

`Home`'s content box is the **single scroll container** for the whole app
(App's outer box is `overflow: hidden`, and the chat transcript no longer has
its own scroll context). This keeps the chat's scroll-to-bottom deterministic
and gives list pages a single place for their `position: sticky` headers to
anchor.

---

## Feature Sections

### 1. Chat (`src/secoes/chat/`)

The conversational core.

- [Chat.tsx](src/secoes/chat/Chat.tsx) — composes the parts below and keeps
  what is the chat's own: sending (valid prompt, history entry and an empty box
  at send time, D-HIST-3/D-UI-2), scrolling to the answer that arrives (to its
  start, D-UI-2), and a history question going back to the box with its type.
- [Compositor.tsx](src/secoes/chat/Compositor.tsx) — the fixed composer bar:
  the "Minhas perguntas" buttons, the mobile collapse bar, the message box with
  the type selector inside, and the prototype notice. It lifts itself above the
  virtual keyboard (measured via `visualViewport`, no device sniffing), clears
  the sidebar, and measures its own height so the transcript is not hidden.
- [PainelHistorico.tsx](src/secoes/chat/PainelHistorico.tsx) — "Minhas
  perguntas" in its two forms: a column from `xl` and a drawer below.
- [Input.tsx](src/secoes/chat/Input.tsx) — multiline `Textarea` + send button.
  **Enter** submits (Shift+Enter = newline). A live character hint shows how
  many characters remain.
- [Messages.tsx](src/secoes/chat/Messages.tsx) — renders the transcript,
  delegating to `UserMessage` / `SystemMessage`.
- [UserMessage.tsx](src/secoes/chat/UserMessage.tsx) — user's bubble (right
  aligned, blue) with a copy-to-input button.
- [SystemMessage.tsx](src/secoes/chat/SystemMessage.tsx) — the answer card,
  rendered via `react-markdown` (shared renderers), with the type line, sources,
  log and copy buttons; an error is a plain bubble with no actions. What a
  message is lives in one field, `natureza` (`pergunta`, `espera`, `resposta`,
  `erro`), in [tipos.ts](src/secoes/chat/tipos.ts): it picks who draws it and
  where the scroll goes.
- [Historico.tsx](src/secoes/chat/Historico.tsx) — the "Minhas perguntas" panel:
  the questions sent in this state, starred ones on top, kept only in this
  browser (`lib/historico.ts`, `hooks/useHistorico.ts`; whether it is open,
  shared by all tabs, in `hooks/usePainelHistorico.ts`). A column from `xl`, a
  drawer below.

**Chat hooks and logic (`hooks/` and `lib/` under `src/secoes/chat/`):**

- [useConversa.ts](src/secoes/chat/hooks/useConversa.ts) — the conversation:
  messages (empty at first, D-UI-10), whether it is waiting, and `enviar`, which
  appends the question and an "Aguarde! Pensando..." placeholder, asks the API
  through `perguntar` ([api/chat.ts](src/api/chat.ts)) and swaps the placeholder
  for the answer or a friendly error (timeout, rate limit — with the server's own
  message —, or server unavailable). It answers at once whether it took the
  question (`false` while another is on its way: no double submit).
- [tipoDeConsulta.ts](src/secoes/chat/lib/tipoDeConsulta.ts) — the question
  type (`QuestionType`, a backend contract), the collection each one needs, and
  `tiposDisponiveis(entity)`; `hooks/useQuestionType.ts` persists the choice.
- [prompt.ts](src/secoes/chat/lib/prompt.ts) — the single source of truth for
  the prompt-length rule: `MIN_PROMPT_LENGTH` (10), `isPromptValid`, and
  `charsRemaining`, used by the send guard, the button's disabled state, and the
  character hint.
- [useIsKeyboardVisible.ts](src/secoes/chat/hooks/useIsKeyboardVisible.ts) —
  detects the on-screen keyboard and its height (via `visualViewport`) so the
  input bar can float above it.

### 2. Serviços (`src/secoes/servicos/`)

Browsable catalog of tax services, grouped by audience. The audience sub-tabs are
**manifest-driven**: `ServicosList` loads `<entity>/servicos-index.json`
(`{ tipo, filename }[]`, emitted by the scraper per entity) via SWR, falling back
to `getDefaultTipoServicos()` (the legacy RS list) in
[servicos.ts](src/secoes/servicos/servicos.ts) when the manifest is absent. The
selected sub-tab loads its `<entity>/<filename>.json`, groups by `classe`, and
renders through
[ServicosAccordion.tsx](src/secoes/servicos/ServicosAccordion.tsx). Includes
client-side search (deferred via `useDeferredValue`).

Because the manifest comes from the entity's own data, each state shows its own
audiences — RS's `Cidadãos / Empresas / Fornecedores / Agentes / Servidores`, or
SC's `Cidadão / Empresa / Servidor Público / Estudante / Prefeitura`.

### 3. FAQs (`src/secoes/faqs/`)

[FaqsList.tsx](src/secoes/faqs/FaqsList.tsx) loads `public/<entity>/faqs.json`
and builds an in-memory tree with the parsers in
[faqs.ts](src/secoes/faqs/faqs.ts) (`buildNodesFromJson`,
`buildAnswerMap`, `buildPageTypeMap`, `searchNodes`, `getEffectiveUrl`),
rendered as a searchable, keyboard-operable accordion
([FaqsAccordion.tsx](src/secoes/faqs/FaqsAccordion.tsx)). Search is deferred
with `useDeferredValue` since the dataset is large.

### 4. Pareceres (`src/secoes/jurisprudencia/`)

[PareceresList.tsx](src/secoes/jurisprudencia/PareceresList.tsx) loads the plain
text `public/<entity>/portal-pareceres.txt` and renders it with URLs turned into
links via [linkify.tsx](src/lib/linkify.tsx).

### 5. Notas (`src/secoes/notas/`)

[NotasList.tsx](src/secoes/notas/NotasList.tsx) — same pattern as Pareceres,
loading `public/<entity>/portal-notas.txt`.

### 6. Conteúdos (`src/secoes/conteudos/`)

[ConteudosList.tsx](src/secoes/conteudos/ConteudosList.tsx) — "Central de
Conteúdo", loads `public/<entity>/conteudo_site_tree.json` (a `categories` tree),
rendered with [ConteudosAccordion.tsx](src/secoes/conteudos/ConteudosAccordion.tsx)
and filtered by [conteudos.ts](src/secoes/conteudos/conteudos.ts).

> **Empty state:** every section gates on `hasCollection(entity, kind)`. When the
> selected state has no data for a section (e.g. SC has only Serviços), the tab
> renders [`CollectionEmpty`](src/ui/CollectionEmpty.tsx) — a friendly
> "*&lt;Section&gt; ainda não disponível para &lt;UF&gt;*" placeholder — and never
> fires the fetch, so there is no 404. Tabs reappear as that state's data is scraped.

### 7. Sobre (`src/secoes/sobre/About.tsx`)

Loads `public/about.md` and renders it as markdown.

---

## App Shell and Base Modules

`src/` has three layers, and dependencies go one way only:

- **`app/`** is the composition root: it mounts the app and may import from
  anywhere.
- **`secoes/`** holds the sections. A section never imports from another one;
  `home/`, which exists to compose them, is the exception.
- **The base** (`ui/`, `lib/`, `api/`, `contexto/`, `theme/`) never imports
  from `secoes/` or `app/`. What two sections need moves down here.

[src/test/camadas.test.ts](src/test/camadas.test.ts) reads every relative import
and fails when a rule breaks.

### `src/app/`

- [App.tsx](src/app/App.tsx) — providers and the single-page shell; see
  [Entry & shell](#entry--shell).
- [AppHeader.tsx](src/app/AppHeader.tsx) — sticky 52px header: the sidebar
  toggle (from `md`, also Ctrl+B), the "Auli" wordmark (the app version is its
  tooltip), the state tabs, and the light/dark **color-mode toggle**
  ([`ColorModeButton`](src/ui/color-mode.tsx)) on the right. With the
  sidebar open, the tabs start on the sidebar's edge.
- [abas.ts](src/app/abas.ts), [AbasDeEstado.tsx](src/app/AbasDeEstado.tsx) — the
  state tabs; see [Multi-tenant state selection](#multi-tenant-state-selection).
- [ErrorBoundary.tsx](src/app/ErrorBoundary.tsx) — app-level React error
  boundary wrapping the whole tree.

### `src/ui/`

- Chakra v3 snippet components, typed like Chakra's TypeScript snippets —
  [provider.tsx](src/ui/provider.tsx), [color-mode.tsx](src/ui/color-mode.tsx),
  [toaster.tsx](src/ui/toaster.tsx), [tooltip.tsx](src/ui/tooltip.tsx).
- [CollectionEmpty.tsx](src/ui/CollectionEmpty.tsx) — the "coming soon for
  this state" placeholder shown by a section the selected entity has no data for.
- [AsyncContent.tsx](src/ui/AsyncContent.tsx) — three-state gate
  (loading spinner / error alert / children) shared by every SWR-backed page.
- [Acordeao.tsx](src/ui/Acordeao.tsx) — `PainelRevelado` (the accordion panel:
  height 0 → `auto` once, 0.22 s, still under `prefers-reduced-motion`, with
  `contain: layout paint` — the only height animation in the app) and
  `comoBotao(acao, ativo)` (role, Tab focus, click, Enter and Space for an element
  that is not a `<button>`; inactive, it is plain text).
- [busca.tsx](src/ui/busca.tsx) — the pieces every searchable list uses:
  `useBusca` (the text, its deferred value, the normalized terms, and whether a
  search is active), `BarraDeBusca` (the sticky bar with the field and the
  counter line; it owns the input, so clearing refocuses it) and
  `SemResultados`. Each list keeps its own filter and body.
- [SearchInput.tsx](src/ui/SearchInput.tsx) — the search field inside
  `BarraDeBusca` (icon + input + accessible clear button).
- [LinkExterno.tsx](src/ui/LinkExterno.tsx) — the "open in a new tab" link
  (`target="_blank"` + `rel="noopener noreferrer"`, mandatory `aria-label`), used by
  the Tributum lists and the "Abrir no portal" of Pareceres and Legislação; and
  `abrirEmNovaAba(url)`, the same guarantee for rows that act as a link.
- [DocumentoMarkdown.tsx](src/ui/DocumentoMarkdown.tsx) — fetches a `.md` from
  `public/` and shows it as prose, through `AsyncContent`: the Sobre page, the
  Tributum Critérios and the "Conectar sua IA" manuals.
- [copiar.ts](src/ui/copiar.ts) — `utilsCopyTextToClipboard`, copies text and
  shows a Chakra toast (with an insecure-origin guard).
- [layout.ts](src/ui/layout.ts) — the sidebar widths (open, and collapsed into
  an icon rail), which the chat's `position: fixed` composer must also know, and
  **`Z`**, the app's layers in one place: sticky search bar 10, chat composer 20,
  "Minhas perguntas" column 21, question-type menu 30, header 100, overlays 110.
- [Sobreposicao.tsx](src/ui/Sobreposicao.tsx) — the skeleton of the two drawers
  (sections, "Minhas perguntas") and the two modals (question log, PDF reader):
  a veil that closes on click, Esc that closes (listened on the `document`) and,
  for the drawers, focus back to the button that opened them. It does not trap
  focus yet.

### `src/lib/`

- [markdown.tsx](src/lib/markdown.tsx) — shared `react-markdown` component
  maps (`compactMarkdownComponents` for chat/FAQ, `compactMarkdownComRealce` —
  the same plus the search highlight's `<mark>` — for Pareceres and Legislação,
  `proseMarkdownComponents` for authored documents), centralizing the link
  renderer's `target`/`rel`. The `<mark>` itself is `Marca`, in `highlight.tsx`.
- [linkify.tsx](src/lib/linkify.tsx) — turns bare URLs in plain text into
  Chakra `Link`s, trimming trailing sentence punctuation (and preserving
  balanced-paren URLs).
- [storage.ts](src/lib/storage.ts) — the only door to `localStorage`: every key
  (`CHAVES`), `lerTexto`/`gravarTexto` with one `try/catch` (blocked storage
  keeps the value in memory for the session), and `useTextoGravado`, which
  re-renders on writes from this window and from others.
- [textSearch.ts](src/lib/textSearch.ts), [highlight.tsx](src/lib/highlight.tsx),
  [rehypeHighlight.ts](src/lib/rehypeHighlight.ts) — the client-side search
  (accent- and case-insensitive) and the highlight of what it matched, in plain
  text and in markdown.

### `src/api/`

- [estaticos.ts](src/api/estaticos.ts) — Axios-based `textFetcher` and
  generic `jsonFetcher<T>` for SWR, `SWR_OPTS` that disable revalidation on
  focus/reconnect (the endpoints serve static files), `versioned()` for the
  per-build `?v=` cache-buster, and **`entityPath(entityId, file)`** which
  resolves an entity-scoped data file to its versioned `/<entityId>/<file>` path.
- [chat.ts](src/api/chat.ts) — the chat's way out: `perguntar` posts the
  question (`question`, `entity`, `type`) with a **70-second timeout** and returns
  a `Resultado` (the answer, or which failure: `tempo`, `limite`, `indisponivel`)
  instead of touching state; `buscarLog` fetches a response's audit log. The
  endpoint is `VITE_API_URL` (falling back to production), and the log URL is
  derived from it. `Fonte`, the server's source contract, lives here too.

### `src/contexto/`

What a state tab hands to the sections inside it.

- [entities.ts](src/contexto/entities.ts) — the entity registry (`Entity`,
  `ENTITIES`, `getEntity`, `hasCollection`); see
  [Multi-tenant state selection](#multi-tenant-state-selection).
- [EntityContext.tsx](src/contexto/EntityContext.tsx) — `EntityProvider` +
  `useSelectedEntity`; one provider per state tab.
- [RespostaDaAba.tsx](src/contexto/RespostaDaAba.tsx) — how a tab's chat reports
  that it is waiting for an answer, or that one arrived while hidden.

---

## Theming & Color Mode

Color is centralized in a single source of truth,
[src/theme/system.js](src/theme/system.js), built with Chakra v3's
`createSystem`/`defineConfig`. It defines a small set of **semantic tokens**
(`bg.app`, `fg.muted`, `accent`, `border`, …), each carrying a light (`base`)
and a `_dark` value, so color mode is a data change rather than per-component
conditionals. Shadow tokens live here too (`focusRing`, and `cardHover` for the
hoverable state-selection cards), keeping raw color values out of components. Components consume tokens only — never raw color literals — and a
`no-restricted-syntax` lint rule in [eslint.config.js](eslint.config.js) fails
the build on any hex/`rgb()`/`hsl()` literal in `src/**/*.{jsx,tsx}` (raw values
are allowed only in `system.js` and `.css` files, the latter as
`var(--chakra-colors-*)`).

`next-themes` (via `ColorModeProvider`) drives the mode with
`defaultTheme="system"` + `enableSystem`, honoring the OS preference on first
load and persisting the user's explicit choice. The toggle lives in `AppHeader`.
The token vocabulary is documented in [THEME.md](THEME.md).

---

## Data Sources

The app reads **static files served from [public/](public/)** — there is no
database in the frontend. Collection data is **namespaced per entity** under
`public/<entityId>/` (e.g. `public/rs/`, `public/sc/`); list/text pages build
their path with `entityPath(entity.id, file)`. Site-wide assets stay at the
`public/` root. Most files are fetched with a `?v=<build-id>` cache-buster
(see `versioned()` / `__BUILD_ID__`):

| File | Used by | Format |
| --- | --- | --- |
| `<entity>/servicos-index.json` | Serviços (audience tabs manifest) | JSON `{tipo,filename}[]` |
| `<entity>/servicos-*.json` | Serviços | JSON per audience |
| `<entity>/faqs.json` | FAQs | JSON tree |
| `<entity>/conteudo_site_tree.json` | Conteúdos | JSON categories |
| `<entity>/portal-pareceres.txt` | Pareceres | plain text |
| `<entity>/portal-notas.txt` | Notas | plain text |
| `about.md` (root) | Sobre | markdown |
| `favicon.ico`, `robots.txt`, `sitemap.xml` (root) | site metadata | — |

An entity only ships the files for the collections it has — e.g. `public/sc/` has
the Serviços files but no `faqs.json` (the FAQs tab shows the empty state).

The only **dynamic** call is the chat question endpoint at
`https://api.auli.com.br/v1/question` (POST `{ question, entity? }`; override with
`VITE_API_URL`).

---

## Build Configuration

[vite.config.js](vite.config.js):

- **`define`** injects `__BUILD_ID__` (a per-build timestamp for cache-busting)
  and `__APP_VERSION__` (read from `package.json` at config load).
- **Manual vendor chunking** via Rolldown to keep the main bundle small:
  - `vendor-react` — react, react-dom
  - `vendor-chakra` — @chakra-ui/react, @emotion/react
  - `vendor-icons` — react-icons
  - `vendor-utils` — axios, framer-motion, react-markdown, remark-gfm, swr
- **`test`** block configures Vitest (see Testing below).

A representative production build emits separate hashed chunks per group
(react ≈ 56 kB gzip, chakra ≈ 95 kB gzip, utils ≈ 77 kB gzip, app ≈ 16 kB gzip).

---

## TypeScript

The project is on **TypeScript with `strict: true`**. All application code is
typed `.ts`/`.tsx`; the only JavaScript file left is
[src/theme/system.js](src/theme/system.js).

- [tsconfig.json](tsconfig.json) — `strict: true`, `moduleResolution: "Bundler"`,
  `jsx: "react-jsx"`, `noEmit: true`. `allowJs: true` + `checkJs: false` lets
  `theme/system.js` be imported without being type-checked. There is no
  path alias: imports are relative.
- [src/vite-env.d.ts](src/vite-env.d.ts) — Vite client types plus the
  `__BUILD_ID__` / `__APP_VERSION__` / `VITE_API_URL` declarations.
- [src/secoes/chat/tipos.ts](src/secoes/chat/tipos.ts) — shared `Message` and `SetPrompt`
  types.
- Data shapes are typed at their source: `RawFaqNode`/`FaqNode` in `faqs.ts`,
  `ConteudoCategory`/`ConteudoTree` in `conteudos.ts`, `Servico`/`TipoServico`
  in `servicos.ts`; the list pages pass these as the `jsonFetcher<T>`
  generic so SWR data is typed end-to-end.

---

## Testing

[Vitest](https://vitest.dev) with two environments configured in
[vite.config.js](vite.config.js) `test`:

- Default **Node** environment for pure-logic tests.
- **jsdom** opted into per-file with a `// @vitest-environment jsdom` docblock
  for component/render tests.
- [src/test/setup.ts](src/test/setup.ts) registers `@testing-library/jest-dom`
  matchers, runs `cleanup()` after each test, and polyfills
  `matchMedia`/`ResizeObserver`/`scrollIntoView` (guarded to jsdom).
- [src/test/render.tsx](src/test/render.tsx) — `renderWithProvider`, which wraps
  a component in the Chakra + color-mode `Provider`.

Coverage:

- **Logic:** `prompt`, `faqs`, `conteudos`, `editLinks`, the `api/chat`
  contract and the `useConversa` flow (mocked Axios).
- **Structure:** [camadas.test.ts](src/test/camadas.test.ts) — the layer rules of
  [App Shell and Base Modules](#app-shell-and-base-modules).
- **Render:** `SearchInput`, `AsyncContent`, `Input`, `Messages`.

---

## Directory Layout

```text
auli-frontend/
├─ index.html
├─ vite.config.js
├─ eslint.config.js
├─ .prettierrc.json, .prettierignore
├─ tsconfig.json
├─ package.json
├─ public/                     # static assets
│  ├─ rs/, sc/, … (27)         # per-entity collection data (faqs, servicos-*, portal-*…)
│  ├─ tributum/, tributum.json # the Tributum catalog (shared by every entity)
│  └─ about.md, favicon.ico…   # site-wide assets (not entity-scoped)
└─ src/
   ├─ main.tsx                 # entry
   ├─ index.css                # global styles (consume Chakra color vars)
   ├─ vite-env.d.ts
   │
   ├─ app/                     # COMPOSITION ROOT: mounts the app, imports from anywhere
   │  ├─ App.tsx               # providers + single-page shell (no router)
   │  ├─ AppHeader.tsx
   │  ├─ AbasDeEstado.tsx      # the state-tab strip in the header
   │  ├─ abas.ts               # state tabs: open/close/pick, persisted (auli.abas)
   │  └─ ErrorBoundary.tsx
   │
   ├─ secoes/                  # SECTIONS: import from the base, never from each other
   │  ├─ registro.ts           # the 17 sections as data (used by home/)
   │  ├─ home/                 # sidebar shell that composes the sections (+ Navegacao)
   │  ├─ mapa/                 # "Escolha o estado" map of a new tab (+ BrazilMap, brazilPaths)
   │  ├─ chat/                 # components · hooks/ · lib/ · tipos.ts
   │  ├─ servicos/  faqs/  jurisprudencia/  legislacao/  notas/  conteudos/
   │  ├─ tributum/             # the six Tributum sections
   │  ├─ downloads/  mcp/
   │  └─ sobre/
   │
   ├─ ui/                      # BASE: generic components (+ the Chakra snippets)
   ├─ lib/                     # BASE: search, highlight, markdown, linkify
   ├─ api/                     # BASE: every network exit (static files, chat endpoint)
   ├─ contexto/                # BASE: what a state tab hands to its sections
   │  ├─ entities.ts           # entity registry (generated from config/registry.toml)
   │  ├─ EntityContext.tsx     # per-tab state context
   │  └─ RespostaDaAba.tsx     # a tab's chat reports when it waits (spinner, then dot)
   ├─ theme/
   │  └─ system.js             # color token system (single source of truth)
   └─ test/
      ├─ setup.ts              # Vitest setup (matchers, polyfills, cleanup)
      ├─ render.tsx            # renderWithProvider helper
      └─ camadas.test.ts       # the layer rules above, as a test
```
