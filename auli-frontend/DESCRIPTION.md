# Auli Frontend — Project Description

## Overview

**Auli** is an open, cooperative pilot project: an AI-powered virtual assistant
that helps Brazilian public servants answer citizens' questions about **state
taxes** (impostos estaduais), with a focus on Rio Grande do Sul (RS).

This repository is the **frontend** — a single-page React application. The user
asks questions in natural language and receives AI-generated answers, and can
browse the state's collection (Services, FAQs, Legal Opinions, TARF rulings,
Legislation, Notes, Contents), the curated **Tributum** shelf, and the
integrations (connecting one's own AI assistant through MCP, and downloading each
state's package). The UI is in **Brazilian Portuguese** and is **mobile-first**.

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
- **Backend API:** `https://api.auli.com.br/v1/question` (POST
  `{ question, entity?, type }` → `{ answer, log_id?, fontes? }`) — `entity` is
  the state of the tab, so the backend queries the right tenant's collections,
  and `type` is the question type (Serviços + FAQs, Pareceres, TARF, Legislação).
  An answer's audit log is `GET /v1/log/{log_id}`. Override the URL with
  `VITE_API_URL`; both calls live in [src/api/chat.ts](src/api/chat.ts).

---

## Tech Stack

| Concern | Choice |
| --- | --- |
| Language | **TypeScript (strict)** — all app code is `.ts`/`.tsx`; the only JavaScript left is the color token system, `theme/system.js` |
| UI library | **React 19** |
| Build tool / dev server | **Vite 8** (Rolldown-based) with `@vitejs/plugin-react` |
| Component library | **Chakra UI v3** (+ `@emotion/react`) |
| Navigation | **No router** — state tabs in the header and a sidebar of sections, all in memory; the URL stays at `/` |
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
  decides which sections are enabled (`secoesDe`, in `secoes/registro.ts`). Generated from `config/registry.toml` by
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

[FaqsList.tsx](src/secoes/faqs/FaqsList.tsx) loads `<entity>-faqs-tree.json`
and builds an in-memory tree with the parsers in
[faqs.ts](src/secoes/faqs/faqs.ts) (`buildNodesFromJson`,
`buildAnswerMap`, `buildPageTypeMap`, `searchNodes`, `getEffectiveUrl`),
rendered as a searchable, keyboard-operable accordion
([FaqsAccordion.tsx](src/secoes/faqs/FaqsAccordion.tsx)). The search runs in the
list, not in the accordion, so the counter and the results share one pass over
the tree; it is deferred with `useDeferredValue` since the dataset is large.

### 4. Pareceres and Acórdãos TARF (`src/secoes/jurisprudencia/`)

Two sections, one component:
[JurisprudenciaList.tsx](src/secoes/jurisprudencia/JurisprudenciaList.tsx) takes
the collection and its vocabulary ("parecer"/"pareceres", "acórdão"/"acórdãos"),
and [PareceresList.tsx](src/secoes/jurisprudencia/PareceresList.tsx) and
[TarfList.tsx](src/secoes/jurisprudencia/TarfList.tsx) only fill them in. Both
load `<entity>-<collection>-index.json` (derived by
`auli-collections <id> indice <collection>`): a flat list of `numero`, `assunto`,
`resumo` (markdown) and `link`. For TARF, `numero` is the ruling, `assunto` its
headnote and `resumo` the reasoning the ruling itself carries.

- The search index ([pareceres.ts](src/secoes/jurisprudencia/pareceres.ts)) is
  built on the **first** search and reused on the next keystrokes: browsing never
  pays for it, and these collections run to thousands of documents.
- [PareceresAccordion.tsx](src/secoes/jurisprudencia/PareceresAccordion.tsx)
  renders **progressively**, 100 items at a time as the scroll nears the end
  (`IntersectionObserver`; without it, everything renders — never truncated).
  An item opens to its `resumo`, with the search terms highlighted, and to
  "Abrir no portal".

### 5. Legislação (`src/secoes/legislacao/`)

[LegislacaoList.tsx](src/secoes/legislacao/LegislacaoList.tsx) loads
`<entity>-legislacao-index.json`: one entry per provision, with the question
(`titulo`), the provision (`ementa`, e.g. "Art. 28"), the trail in the law
(`trilha`, "Lei | Título | Capítulo"), the answer (`corpo`, markdown) and the
official `link`. Entries are grouped by law (the first segment of the trail, see
[legislacao.ts](src/secoes/legislacao/legislacao.ts)), in the order of the legal
text; groups start closed and are forced open while searching. A row opens to
the answer and "Abrir no portal".

It is its own component, not `JurisprudenciaList` (D-LEG-13): there the list is
flat and chronological and each item opens to a summary; here it is a hierarchy
by law, and the two would share nothing but the search box.

### 6. Notas (`src/secoes/notas/`)

[NotasList.tsx](src/secoes/notas/NotasList.tsx) loads the plain text
`<entity>-portal-notas.txt` and shows it as is, with URLs turned into links via
[linkify.tsx](src/lib/linkify.tsx).

### 7. Conteúdos (`src/secoes/conteudos/`)

[ConteudosList.tsx](src/secoes/conteudos/ConteudosList.tsx) — "Central de
Conteúdo", loads `<entity>-conteudo_site_tree.json` (a `categories` tree),
rendered with [ConteudosAccordion.tsx](src/secoes/conteudos/ConteudosAccordion.tsx)
and filtered by [conteudos.ts](src/secoes/conteudos/conteudos.ts).

> **Sections a state lacks:** a section whose collection the entity does not
> declare is shown but **disabled** in the sidebar (`secoesDe`), so it cannot be
> opened — it appears as that state's data is scraped. Each list still gates on
> `hasCollection(entity, kind)` and renders
> [`CollectionEmpty`](src/ui/CollectionEmpty.tsx) without firing the fetch, as a
> defense.

### 8. Sobre (`src/secoes/sobre/About.tsx`)

The project's manifesto: `public/about.md`, shown as prose through
[`DocumentoMarkdown`](src/ui/DocumentoMarkdown.tsx).

### 9. Tributum (`src/secoes/tributum/`)

An editorial shelf, not a collection (D-TRIB-*): nothing here enters the `.md`
trees, the packs or the prompts, and **one catalog, `public/tributum.json`, is
shared by every state** — so its six sections are always enabled.

- **Critérios** ([CriteriosTributum.tsx](src/secoes/tributum/CriteriosTributum.tsx))
  — what goes in, what does not and who curates: `public/tributum-criterios.md`,
  through `DocumentoMarkdown`. It opens the group: the editorial rule comes before
  the lists it governs.
- **Artigos e estudos, Instituições, Dados, Análises, Livros** — five lists over
  [TributumShell.tsx](src/secoes/tributum/TributumShell.tsx): the search bar with
  its counter (over the fields each section declares searchable), the
  loading/error gate, the cards (each section draws its own) and [RodapeCurador.tsx](src/secoes/tributum/RodapeCurador.tsx), which
  names the shelf's curator — the curation is personal and named, not an
  institutional endorsement. The catalog is fetched once
  ([useTributum.ts](src/secoes/tributum/useTributum.ts)) and typed in
  [types.ts](src/secoes/tributum/types.ts), whose test also checks the published
  catalog against the schema.
- Every item links out through `LinkExterno`. One not declared published wears
  an "exemplo" badge ([PlaceholderBadge.tsx](src/secoes/tributum/PlaceholderBadge.tsx);
  absent means placeholder, fail-safe). A PDF hosted with recorded authorization
  opens in [PdfModal.tsx](src/secoes/tributum/PdfModal.tsx), a native `<iframe>`
  with "Abrir em nova aba" always visible — Chrome on Android shows a blank frame
  instead of the PDF. Books carry an access badge (open, commercial, library).

### 10. Conectar sua IA (`src/secoes/mcp/`)

[McpList.tsx](src/secoes/mcp/McpList.tsx) — how to query the Auli collection from
inside one's own AI assistant, through the MCP connector: what can be asked (the
five tools: `listar_entidades`, `buscar_pareceres`, `obter_parecer`,
`consultar_servicos_faqs`, `buscar_legislacao`) and a table of the four
installation manuals (ChatGPT, Claude, GitHub Copilot, Copilot Studio). A manual
opens in an in-page reader (`DocumentoMarkdown` over `public/manuais/*.md`,
copied from the repository's `manuais/` by `build-frontend-public.sh`). The list
is hardcoded on purpose and mirrors `manuais/README.md`. Always enabled.

### 11. Downloads (`src/secoes/downloads/`)

[DownloadsList.tsx](src/secoes/downloads/DownloadsList.tsx) — the selected
state's `.zip` package highlighted, and below it a table of every state, for
whoever feeds their own RAG. It reads `public/downloads/downloads.json`, the
**only global data file** of the frontend, generated by `auli bundle` from the
same `.md` trees as the packs (state, file, size, sha256, last change, and file
counts per type; see [downloads.ts](src/secoes/downloads/downloads.ts)). Always
enabled: a state without a package shows an empty highlight, and the table stays.

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
  a veil that closes on click, Esc that closes (listened on the `document`), and
  a focus trap: on open, focus moves to the panel's first focusable; Tab and
  Shift+Tab wrap around; focus that escapes (the Tab out of the PDF iframe)
  comes back; on close, by any path, focus returns to whoever opened it.

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
conditionals. Shadow tokens live here too (e.g. `focusRing`), keeping raw color values out of
components. Components consume tokens only — never raw color literals — and a
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
database in the frontend. Collection data is **namespaced per entity**:
`entityPath(entity.id, file)` resolves a file to `/<id>/<id>-<file>` (every file in
`public/<id>/` carries the `<id>-` prefix, so names are globally unique). Shared
files stay at the `public/` root. The data files are fetched with a `?v=<build-id>`
cache-buster (`versioned()` / `__BUILD_ID__`); the PDFs and `.zip` packages are
plain links:

| File | Used by | Format |
| --- | --- | --- |
| `<id>/<id>-servicos-index.json` | Serviços (audience tabs manifest) | JSON `{tipo,filename}[]` |
| `<id>/<id>-servicos-*.json` | Serviços | JSON per audience |
| `<id>/<id>-faqs-tree.json` | FAQs | JSON tree |
| `<id>/<id>-pareceres-index.json` | Pareceres | JSON list |
| `<id>/<id>-tarf-index.json` | Acórdãos TARF | JSON list |
| `<id>/<id>-legislacao-index.json` | Legislação | JSON list |
| `<id>/<id>-portal-notas.txt` | Notas | plain text |
| `<id>/<id>-conteudo_site_tree.json` | Conteúdos | JSON categories |
| `tributum.json`, `tributum-criterios.md`, `tributum/*.pdf` | Tributum (all states) | catalog / markdown / hosted PDFs |
| `manuais/*.md` | Conectar sua IA | markdown |
| `downloads/downloads.json` | Downloads (all states) | JSON manifest |
| `about.md` | Sobre | markdown |
| `favicon.ico`, `robots.txt`, `sitemap.xml` | site metadata | — |

An entity only ships the files for the collections it has: one that declares
only `servicos` has the Serviços files and nothing else.

The only **dynamic** calls are the chat's, in [src/api/chat.ts](src/api/chat.ts):
the question (`POST /v1/question`, 70-second timeout) and an answer's audit log
(`GET /v1/log/{log_id}`). Override the base URL with `VITE_API_URL`.

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

A production build emits separate hashed chunks per group (measured at v0.1.87:
react ≈ 65 kB gzip, chakra ≈ 104 kB gzip, utils ≈ 103 kB gzip, icons ≈ 5 kB
gzip, app ≈ 48 kB gzip).

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
