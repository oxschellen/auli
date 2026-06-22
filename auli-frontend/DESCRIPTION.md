# Auli Frontend — Project Description

## Overview

**Auli** is an open, cooperative pilot project: an AI-powered virtual assistant
that helps Brazilian public servants answer citizens' questions about **state
taxes** (impostos estaduais), with a focus on Rio Grande do Sul (RS).

This repository is the **frontend** — a single-page React application. The user
asks questions in natural language and receives AI-generated answers, plus
several browsable reference sections (Services, FAQs, Legal Opinions, and Tax
Notes). The UI is in **Brazilian Portuguese** and is **mobile-first**.

The app is **multi-tenant by state**: the user first picks a state tax authority
(*entity*) on a landing page, and the whole session — every data file and the
chat — is scoped to that entity. Two entities ship today: **RS** (SEFAZ-RS, full
data) and **SC** (SEF-SC, Serviços only so far). See
[Multi-tenant state selection](#multi-tenant-state-selection).

- **App name (package):** `auli-ui`
- **UI version:** taken from `package.json` `version` (currently `0.1.43`) and
  shown in the header as `v.0.1.43` — injected at build time via the
  `__APP_VERSION__` define, so it can't drift from a hardcoded constant. When an
  entity is selected the header subtitle becomes `v.0.1.43 · SEF-SC`.
- **Backend API:** `https://api.auli.com.br/v1/question` (POST `{ question, entity? }`
  → `{ answer }`) — `entity` is the selected state id, so the backend can query
  the right tenant's collections (override the URL with `VITE_API_URL`).

---

## Tech Stack

| Concern | Choice |
| --- | --- |
| Language | **TypeScript (strict)** — all app code is `.ts`/`.tsx`; only the 4 vendored Chakra `ui/` snippets remain `.jsx` |
| UI library | **React 19** |
| Build tool / dev server | **Vite 8** (Rolldown-based) with `@vitejs/plugin-react` |
| Component library | **Chakra UI v3** (+ `@emotion/react`, `@emotion/styled`) |
| Navigation | **None** — single-page **tabbed** shell (no router) |
| Data fetching | **SWR** + **Axios** |
| Animation | **Framer Motion** (`LazyMotion` / `domAnimation`) |
| Icons | **react-icons** (Material Design set) |
| Markdown | **react-markdown** |
| Theming | **next-themes** + a custom Chakra **semantic-token system** — full light/dark mode (see [THEME.md](THEME.md)) |
| Testing | **Vitest** + **@testing-library/react** + **jsdom** |

Linting via **ESLint 9** (flat config) with React, React Hooks, React Refresh,
and `typescript-eslint` plugins. `react-hooks/exhaustive-deps` is enforced as an
error.

---

## How to Run

```bash
npm install       # install dependencies
npm run dev       # start Vite dev server
npm run build     # type-check (tsc --noEmit) then production build
npm run preview   # preview the production build
npm run lint      # ESLint over js/jsx/ts/tsx (fails on warnings)
npm run typecheck # tsc --noEmit only
npm test          # run the Vitest suite once
npm run test:watch# Vitest in watch mode
```

---

## Application Architecture

### Entry & shell

- [index.html](index.html) loads [src/main.tsx](src/main.tsx), which mounts
  `<App />` inside `React.StrictMode`.
- [src/App.tsx](src/App.tsx) wraps everything in (outermost first):
  - `Provider` (Chakra UI + color mode, from [src/pages/chat/ui/provider.jsx](src/pages/chat/ui/provider.jsx)),
  - a single global `Toaster`,
  - [`ErrorBoundary`](src/shared/ErrorBoundary.tsx),
  - `LazyMotion` (Framer Motion),
  - [`EntityProvider`](src/shared/EntityContext.tsx) — the selected-state context,
  - then `AppShell`: a full-height `Flex` column with the shared
    [`AppHeader`](src/shared/AppHeader.tsx) pinned above a single content region.
    The region renders the [`StateSelection`](src/pages/stateselection/StateSelection.tsx)
    landing page until an entity is chosen, then [`Home`](src/pages/home/Home.tsx).

There is **no router** — the app is a single page. Sections are tabs inside
`Home`, not routes, so all state lives in memory and the URL stays at `/`. The
selected entity is the one piece of state that persists (to `localStorage`), so a
reload returns straight to the chosen state's app.

### Multi-tenant state selection

The app is scoped to one **entity** (state tax authority) at a time, chosen up
front rather than toggled mid-session (so chat history, search, and scroll state
never need re-scoping).

- [src/shared/entities.ts](src/shared/entities.ts) — the entity registry (a
  frontend mirror of the scraper's `domain::entities`). Each `Entity` has `id`
  (also the `public/<id>/` data folder and the backend tenant key), `name`
  (`SEFAZ-RS`), `uf` (`RS`), `state` (`Rio Grande do Sul`), and **`collections`**
  — the list of sections it actually has data for. `hasCollection(entity, kind)`
  drives the per-tab empty state. Ships `rs` (all five collections) and `sc`
  (`servicos` only).
- [src/shared/EntityContext.tsx](src/shared/EntityContext.tsx) — `EntityProvider`
  + `useEntity()` / `useSelectedEntity()`. Holds the chosen entity (or `null` when
  none is picked yet), persisted to `localStorage` under `auli.entity`; a stored
  id that no longer maps to a known entity is ignored. `selectEntity(id)` /
  `clearEntity()` switch or reset the selection. `useSelectedEntity()` asserts one
  is selected (used inside the app shell, below the landing-page gate).
- [src/pages/stateselection/StateSelection.tsx](src/pages/stateselection/StateSelection.tsx)
  — the landing page ("Escolha o estado"): an interactive **Brazil map** beside one
  card per entity (UF badge, name, state, collection count). Both call `selectEntity`.
- [src/pages/stateselection/BrazilMap.tsx](src/pages/stateselection/BrazilMap.tsx) —
  an outlined SVG map of all 27 states. States backed by an available entity (RS, SC)
  are filled with the accent color and are clickable/keyboard-operable (`role=button`,
  `<title>` tooltip); every other state is greyed (`bg.mapInactive`) and inert.
  Colors resolve via `useToken` and native `<svg>`/`<path>` elements are used (Chakra's
  `Box as="path"` drops the `d`/`viewBox` attributes). Geometry comes from
  [brazilPaths.ts](src/pages/stateselection/brazilPaths.ts) — auto-generated compact
  per-UF path strings (GeoJSON simplified via Douglas-Peucker, projected to a
  1000×1000 viewBox; ~33 kB, do not hand-edit). A future map of more states only needs
  more entities in the registry — the map lights up any UF that has one.
- The active entity is surfaced in [`AppHeader`](src/shared/AppHeader.tsx) as a
  left-slot **`UF` chip + "trocar"** button that calls `clearEntity()` to return
  to the selector; the header subtitle gains `· <entity.name>`.

### Home tab shell

[src/pages/home/Home.tsx](src/pages/home/Home.tsx) renders a top tab bar
(Chat, Serviços, FAQs, Pareceres, Notas, Conteúdos, Sobre) implementing the
WAI-ARIA tabs pattern (roving `tabIndex`, arrow-key navigation). A tab panel is
**mounted on first activation and then kept mounted** (toggled with
`display: none`) so each section preserves its state — scroll position, search
query, chat history — across tab switches, and megabyte-sized section data is
fetched only once the tab is first opened.

### Scrolling

`Home`'s content box is the **single scroll container** for the whole app
(App's outer box is `overflow: hidden`, and the chat transcript no longer has
its own scroll context). This keeps the chat's scroll-to-bottom deterministic
and gives list pages a single place for their `position: sticky` headers to
anchor.

---

## Feature Sections

### 1. Chat (`src/pages/chat/`)

The conversational core.

- [Chat.tsx](src/pages/chat/Chat.tsx) — orchestrates state and layout. Holds
  message list, prompt, and loading state; auto-scrolls to the latest message;
  uses an `isSubmittingRef` guard to prevent double submits; pins the input bar
  to the bottom and lifts it above the virtual keyboard (measured via
  `visualViewport`, no device sniffing).
- [Input.tsx](src/pages/chat/Input.tsx) — multiline `Textarea` + send button.
  **Enter** submits (Shift+Enter = newline). A live character hint shows how
  many characters remain.
- [Messages.tsx](src/pages/chat/Messages.tsx) — renders the transcript,
  delegating to `UserMessage` / `SystemMessage`.
- [UserMessage.tsx](src/pages/chat/UserMessage.tsx) — user's bubble (right
  aligned, blue) with a copy-to-input button.
- [SystemMessage.tsx](src/pages/chat/SystemMessage.tsx) — assistant's bubble,
  rendered via `react-markdown` (shared renderers), with a copy button.

**Chat utilities (`src/pages/chat/utils/`):**

- [callServerAPI.ts](src/pages/chat/utils/callServerAPI.ts) — posts the question
  (plus the selected `entityId`, when present, as `entity` in the body) to the API
  with a **25-second timeout** (`AbortController`). Optimistically appends the user
  message and an "Aguarde! Pensando..." placeholder, then replaces the placeholder
  with the answer or a friendly error (distinguishing timeout/cancel from
  server-unavailable). `Chat` reads the active entity via `useSelectedEntity()`.
- [prompt.ts](src/pages/chat/utils/prompt.ts) — the single source of truth for
  the prompt-length rule: `MIN_PROMPT_LENGTH` (10), `isPromptValid`, and
  `charsRemaining`, used by the send guard, the button's disabled state, and the
  character hint.
- [useMessages.ts](src/pages/chat/utils/useMessages.ts) — seeds the transcript
  with a greeting message and exposes `{ messages, setMessages }`.
- [usePrompt.ts](src/pages/chat/utils/usePrompt.ts) — prompt input state.
- [useIsKeyboardVisible.ts](src/pages/chat/utils/useIsKeyboardVisible.ts) —
  detects the on-screen keyboard and its height (via `visualViewport`) so the
  input bar can float above it.
- [utils.ts](src/pages/chat/utils/utils.ts) — `utilsCopyTextToClipboard`,
  copies text and shows a Chakra toast (with an insecure-origin guard).

**Chat UI wrappers (`src/pages/chat/ui/`):** vendored Chakra v3 snippet
components, kept as `.jsx` — [provider.jsx](src/pages/chat/ui/provider.jsx),
[color-mode.jsx](src/pages/chat/ui/color-mode.jsx),
[toaster.jsx](src/pages/chat/ui/toaster.jsx),
[tooltip.jsx](src/pages/chat/ui/tooltip.jsx).

### 2. Serviços (`src/pages/servicoslist/`)

Browsable catalog of tax services, grouped by audience. The audience sub-tabs are
**manifest-driven**: `ServicosList` loads `<entity>/servicos-index.json`
(`{ tipo, filename }[]`, emitted by the scraper per entity) via SWR, falling back
to `getDefaultTipoServicos()` (the legacy RS list) in
[utils.ts](src/pages/servicoslist/utils.ts) when the manifest is absent. The
selected sub-tab loads its `<entity>/<filename>.json`, groups by `classe`, and
renders through
[ServicosAccordion.tsx](src/pages/servicoslist/ServicosAccordion.tsx). Includes
client-side search (deferred via `useDeferredValue`).

Because the manifest comes from the entity's own data, each state shows its own
audiences — RS's `Cidadãos / Empresas / Fornecedores / Agentes / Servidores`, or
SC's `Cidadão / Empresa / Servidor Público / Estudante / Prefeitura`.

### 3. FAQs (`src/pages/faqslist/`)

[FaqsList.tsx](src/pages/faqslist/FaqsList.tsx) loads `public/<entity>/faqs.json`
and builds an in-memory tree with the parsers in
[parseFaqs.ts](src/pages/faqslist/parseFaqs.ts) (`buildNodesFromJson`,
`buildAnswerMap`, `buildPageTypeMap`, `searchNodes`, `getEffectiveUrl`),
rendered as a searchable, keyboard-operable accordion
([FaqsAccordion.tsx](src/pages/faqslist/FaqsAccordion.tsx)). Search is deferred
with `useDeferredValue` since the dataset is large.

### 4. Pareceres (`src/pages/parecereslist/`)

[PareceresList.tsx](src/pages/parecereslist/PareceresList.tsx) loads the plain
text `public/<entity>/portal-pareceres.txt` and renders it with URLs turned into
links via [linkify.tsx](src/shared/linkify.tsx).

### 5. Notas (`src/pages/notaslist/`)

[NotasList.tsx](src/pages/notaslist/NotasList.tsx) — same pattern as Pareceres,
loading `public/<entity>/portal-notas.txt`.

### 6. Conteúdos (`src/pages/conteudoslist/`)

[ConteudosList.tsx](src/pages/conteudoslist/ConteudosList.tsx) — "Central de
Conteúdo", loads `public/<entity>/conteudo_site_tree.json` (a `categories` tree),
rendered with [ConteudosAccordion.tsx](src/pages/conteudoslist/ConteudosAccordion.tsx)
and filtered by [parseConteudos.ts](src/pages/conteudoslist/parseConteudos.ts).

> **Empty state:** every section gates on `hasCollection(entity, kind)`. When the
> selected state has no data for a section (e.g. SC has only Serviços), the tab
> renders [`CollectionEmpty`](src/shared/CollectionEmpty.tsx) — a friendly
> "*&lt;Section&gt; ainda não disponível para &lt;UF&gt;*" placeholder — and never
> fires the fetch, so there is no 404. Tabs reappear as that state's data is scraped.

### 7. Sobre (`src/pages/about/About.tsx`)

Loads `public/about.md` and renders it as markdown.

---

## Shared Modules (`src/shared/`)

- [AppHeader.tsx](src/shared/AppHeader.tsx) — sticky header (black in **both**
  color modes) with the "Auli" wordmark, an optional subtitle (the app version,
  suffixed with the active entity name), the light/dark **color-mode toggle**
  ([`ColorModeButton`](src/pages/chat/ui/color-mode.jsx)) in its right slot, and —
  when an entity is selected — the **`UF` chip + "trocar"** state switcher in its
  left slot.
- [entities.ts](src/shared/entities.ts) — the entity registry (`Entity`,
  `ENTITIES`, `getEntity`, `hasCollection`); see
  [Multi-tenant state selection](#multi-tenant-state-selection).
- [EntityContext.tsx](src/shared/EntityContext.tsx) — `EntityProvider` +
  `useEntity` / `useSelectedEntity`; the `localStorage`-persisted selected-state
  context.
- [CollectionEmpty.tsx](src/shared/CollectionEmpty.tsx) — the "coming soon for
  this state" placeholder shown by a section the selected entity has no data for.
- [AsyncContent.tsx](src/shared/AsyncContent.tsx) — three-state gate
  (loading spinner / error alert / children) shared by every SWR-backed page.
- [SearchInput.tsx](src/shared/SearchInput.tsx) — the sticky-bar search field
  shared by Serviços, FAQs, and Conteúdos (icon + input + accessible clear
  button).
- [markdown.tsx](src/shared/markdown.tsx) — shared `react-markdown` component
  maps (`compactMarkdownComponents` for chat/FAQ, `proseMarkdownComponents` for
  About), centralizing the link renderer's `target`/`rel`.
- [ErrorBoundary.tsx](src/shared/ErrorBoundary.tsx) — app-level React error
  boundary wrapping the whole tree.
- [fetchers.ts](src/shared/fetchers.ts) — Axios-based `textFetcher` and
  generic `jsonFetcher<T>` for SWR, `SWR_OPTS` that disable revalidation on
  focus/reconnect (the endpoints serve static files), `versioned()` for the
  per-build `?v=` cache-buster, and **`entityPath(entityId, file)`** which
  resolves an entity-scoped data file to its versioned `/<entityId>/<file>` path.
- [linkify.tsx](src/shared/linkify.tsx) — turns bare URLs in plain text into
  Chakra `Link`s, trimming trailing sentence punctuation (and preserving
  balanced-paren URLs).

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
- **`resolve.alias`** wires the `@/*` path alias to `./src` so it resolves in
  the editor, type-check, and build alike.
- **Manual vendor chunking** via Rolldown to keep the main bundle small:
  - `vendor-react` — react, react-dom
  - `vendor-chakra` — @chakra-ui/react, @emotion/react, @emotion/styled
  - `vendor-icons` — react-icons
  - `vendor-utils` — axios, framer-motion, react-markdown
- **`test`** block configures Vitest (see Testing below).

A representative production build emits separate hashed chunks per group
(react ≈ 56 kB gzip, chakra ≈ 95 kB gzip, utils ≈ 77 kB gzip, app ≈ 16 kB gzip).

---

## TypeScript

The project is on **TypeScript with `strict: true`**. All application code is
typed `.ts`/`.tsx`; the only remaining `.jsx` files are the four vendored Chakra
`ui/` snippets, kept as-is on purpose.

- [tsconfig.json](tsconfig.json) — `strict: true`, `moduleResolution: "Bundler"`,
  `jsx: "react-jsx"`, `noEmit: true`. `allowJs: true` + `checkJs: false` lets the
  Chakra `ui/*.jsx` snippets be imported without being type-checked. The `@/*`
  path alias is declared here and mirrored in `jsconfig.json` and the Vite
  `resolve.alias`.
- [src/vite-env.d.ts](src/vite-env.d.ts) — Vite client types plus the
  `__BUILD_ID__` / `__APP_VERSION__` / `VITE_API_URL` declarations.
- [src/types/chat.ts](src/types/chat.ts) — shared `Message`, `MessageSender`,
  `SetPrompt` types.
- Data shapes are typed at their source: `RawFaqNode`/`FaqNode` in `parseFaqs`,
  `ConteudoCategory`/`ConteudoTree` in `parseConteudos`, `Servico`/`TipoServico`
  in the servicos `utils`; the list pages pass these as the `jsonFetcher<T>`
  generic so SWR data is typed end-to-end.

> Note: Vite resolves a `.jsx`/`.js` import specifier to a `.tsx`/`.ts` file, so
> drop the extension (or keep it consistent) when renaming.

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

- **Logic:** `prompt`, `parseFaqs`, `parseConteudos`, `editLinks`, and the
  `callServerAPI` branches (mocked Axios).
- **Render:** `SearchInput`, `AsyncContent`, `Input`, `Messages`.

---

## Directory Layout

```text
auli-frontend/
├─ index.html
├─ vite.config.js
├─ eslint.config.js
├─ tsconfig.json
├─ jsconfig.json
├─ package.json
├─ public/                     # static assets
│  ├─ rs/                      # SEFAZ-RS collection data (faqs, servicos-*, portal-*…)
│  ├─ sc/                      # SEF-SC collection data (servicos only, so far)
│  └─ about.md, favicon.ico…   # site-wide assets (not entity-scoped)
└─ src/
   ├─ main.tsx                 # entry
   ├─ App.tsx                  # providers + single-page shell (no router)
   ├─ index.css                # global styles (consume Chakra color vars)
   ├─ vite-env.d.ts
   ├─ types/
   │  └─ chat.ts
   ├─ theme/
   │  └─ system.js             # color token system (single source of truth)
   ├─ test/
   │  ├─ setup.ts              # Vitest setup (matchers, polyfills, cleanup)
   │  └─ render.tsx            # renderWithProvider helper
   ├─ shared/
   │  ├─ AppHeader.tsx
   │  ├─ entities.ts           # entity registry (states + their collections)
   │  ├─ EntityContext.tsx     # selected-state context (localStorage)
   │  ├─ CollectionEmpty.tsx   # "coming soon for this state" placeholder
   │  ├─ AsyncContent.tsx
   │  ├─ SearchInput.tsx
   │  ├─ markdown.tsx
   │  ├─ ErrorBoundary.tsx
   │  ├─ fetchers.ts
   │  └─ linkify.tsx
   └─ pages/
      ├─ stateselection/       # "Escolha o estado" landing page (+ BrazilMap, brazilPaths)
      ├─ home/                 # tabbed shell
      ├─ chat/                 # chat + ui/ + utils/
      ├─ servicoslist/
      ├─ faqslist/
      ├─ parecereslist/
      ├─ notaslist/
      ├─ conteudoslist/
      └─ about/
```
