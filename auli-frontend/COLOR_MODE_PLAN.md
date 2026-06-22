# Color Management & Dark Mode — Implementation Plan

> Status: Phases 0–3 executed (token system live, all components/CSS migrated, dark mode + header toggle working, lint guardrail + token docs in place). Complete.
> Scope: consolidate all color usage behind a single token system built on Chakra UI v3, then add a real light/dark mode on top of it.

---

## Progress (executed)

- **Phase 0 ✓** — `src/theme/system.js` defines primitives + semantic tokens; `provider.jsx` uses the custom `system`. Verified pixel-identical in light mode.
- **Phase 1 ✓** — every component migrated off raw hex to semantic tokens (token names in props; `var(--chakra-colors-*)` inside shorthand strings / inline `style`). Live globals in `index.css` and the `body`/scrollbar rules in `chat.css` now reference Chakra vars so they follow the mode. `grep` confirms no raw hex/named-default colors remain in `src/**/*.{jsx,tsx}`.
- **Phase 2 ✓** — `ColorModeProvider` set to `defaultTheme="system"` + `enableSystem`; `useColorMode`/`ColorModeButton` added to `color-mode.jsx` and wired into `AppHeader`'s right slot. Verified light **and** dark via forced-theme screenshots.
- **Phase 3 ✓** — ESLint `no-restricted-syntax` guard in `eslint.config.js` bans raw hex / `rgb()` / `hsl()` literals (both string `Literal`s and `TemplateElement`s) in `src/**/*.{jsx,tsx}`; raw values are allowed only in `src/theme/system.js` and `.css` files. Token vocabulary documented in `THEME.md`. Verified: lint clean, build clean, grep gate clean, and a negative test (injected hex + rgba) fires both selectors.
  - **The guard caught real Phase 1 gaps.** Phase 1's "no hex remains" check only grepped for `#…`, so `rgba()` literals had survived in `AppHeader.jsx`, `color-mode.jsx`, `Input.jsx`, `SystemMessage.jsx`, and `UserMessage.tsx`, and `ConteudosAccordion.jsx` had **never been migrated at all** (14 hex literals — it was missing from the Phase 1 file list). All now on tokens.
  - **New semantic tokens added** for cases the original set didn't cover: `bg.overlay` (translucent hover overlay, replaces ad-hoc `rgba(0,0,0,.06)` / brown copy-button hovers), `fg.inverted.muted` (header subtitle on black), `border.inverted` (header hairline + toggle hover on black), and a `focusRing` shadow token (input focus ring). The brownish `rgba(200,160,140,.5)` user-bubble copy-hover was legacy from the old sage/tan palette — neutralized to `bg.overlay`.

### Deviations from the original plan

- Bubble tokens simplified to a single `bubble.user` (= `#dbeafe`, blue). The plan's green/blue split came from `DESIGN.md`, but in the actual app the user bubble is blue and bot/system replies are plain `bg.canvas` — tokens now match reality, not `DESIGN.md`.
- `ColorModeButton` is a plain `Box as="button"` rather than Chakra `IconButton`: the v3 IconButton recipe's base styles override style props at equal specificity, which made the icon `color` resolve to the gray color-palette (invisible on the black header).

### Findings (both resolved)

- **~~Horizontal overflow at mobile widths~~ — not a real bug (verified).** Measured via CDP at a true 420px mobile viewport (`Emulation.setDeviceMetricsOverride`): `document.scrollWidth === innerWidth === 420` and zero elements extend past the right edge, on both Home and `/listservicos`. The apparent "cut off" content in earlier screenshots was a headless-Chrome artifact — `--window-size=420` actually lays out at ~500px (headless min window width) and clips the capture to 420, so right-aligned elements (toggle, send button) only *looked* off-screen. No code change needed; the header toggle, tabs (with intended ellipsis), and chat send button all fit at real 420px.
- **`chat.css` deleted.** It was ~90% dead — only `*`/`html`/`body`/scrollbar rules were live, and those duplicated `index.css`. Removed the file and its import from `Chat.jsx`; the global scrollbar now comes from `index.css` (mode-aware). Build/lint/doctor stay green and the chat renders identically.

---

## 1. Why this is needed (current state)

A quick audit of `src/` turned up the problems this plan solves:

| Finding | Detail |
| --- | --- |
| **No custom theme** | `src/pages/chat/ui/provider.jsx` passes Chakra's stock `defaultSystem`. There is no `createSystem`/`defineConfig`, so we have no project token vocabulary. |
| **Dark mode is wired but dead** | `next-themes` is mounted via `ColorModeProvider` (`attribute="class"`), but there are **zero** `_dark` style props, **zero** `useColorMode` calls, no toggle UI, and no dark CSS. Toggling the theme today changes nothing visible. |
| **Color literals everywhere** | ~30 distinct hex values + ~11 `rgba()` literals spread across **16 files** — inline Chakra props *and* two hand-written CSS files (`src/index.css`, `src/pages/chat/chat.css`). |
| **Heavy redundancy** | Borders use `#e2e8f0`, `#e0e0e0`, `#e9ecef`, `#ced4da`, `#cbd5e1` interchangeably. Muted text uses `#64748b`, `#4a5568`, `#6c757d`, `#7a7a7a`, `#94a3b8`. These are the same intent expressed five ways. |
| **Token set exists but is ignored** | `src/index.css` `:root` already defines CSS variables (`--color-primary`, `--bg-app`, `--text-primary`, …) sourced from `DESIGN.md`, but components inline raw hex instead of referencing them. |
| **`DESIGN.md` is the source of truth** | It defines a named palette (`primary #0066cc`, `ink #1d1d1f`, `canvas-parchment #f5f5f7`, plus **dark** `surface-tile-*`, `surface-black`, `body-on-dark`, `primary-on-dark`). The design language already anticipates dark surfaces. |

**Goal:** one place defines color; every component consumes semantic tokens; flipping color mode "just works" and meets WCAG AA contrast.

---

## 2. Principles

1. **Single source of truth** — colors are defined once, in the Chakra system config, seeded from `DESIGN.md`.
2. **Semantic over literal** — components reference *roles* (`fg.muted`, `bg.subtle`, `border`) not values (`#64748b`). A component should never know which mode it's in.
3. **`DESIGN.md` stays canonical** — the token palette must match it; if they diverge, `DESIGN.md` wins and we update tokens.
4. **No raw hex in components** — enforced by lint after migration. CSS files reference Chakra-emitted CSS vars.
5. **Dark mode is a data change, not a code change** — adding `_dark` values to semantic tokens, never per-component conditionals.

---

## 3. Target architecture (Chakra UI v3)

Chakra v3 handles modes through **semantic tokens** that carry per-mode values. We build a custom system once:

```js
// src/theme/system.js  (new)
import { createSystem, defaultConfig, defineConfig } from "@chakra-ui/react";

const config = defineConfig({
  theme: {
    tokens: {
      // ── primitives: the raw palette, mode-agnostic ──
      colors: {
        brand: {
          50:  { value: "#dbeafe" },
          400: { value: "#2997ff" }, // primary-on-dark / sky link
          500: { value: "#0066cc" }, // primary / Action Blue
          600: { value: "#0071e3" }, // primary-focus
          700: { value: "#004fa3" }, // pressed
        },
        ink:    { value: "#1d1d1f" },
        canvas: { value: "#ffffff" },
        parchment: { value: "#f5f5f7" },
        // neutral scale consolidating the 10+ ad-hoc grays
        neutral: {
          100: { value: "#f1f5f9" },
          200: { value: "#e2e8f0" }, // was also e0e0e0 / e9ecef / ced4da
          400: { value: "#94a3b8" },
          500: { value: "#64748b" }, // was also 4a5568 / 6c757d / 7a7a7a
          700: { value: "#334155" }, // was also 2d3748 / 475569
        },
        bubbleUser: { value: "#d1fae5" },
        bubbleBot:  { value: "#dbeafe" },
      },
    },
    semanticTokens: {
      // ── semantic: what components actually use; light + dark values ──
      colors: {
        "bg.canvas":   { value: { base: "{colors.canvas}",    _dark: "#1d1d1f" } },
        "bg.app":      { value: { base: "{colors.parchment}", _dark: "#000000" } },
        "bg.subtle":   { value: { base: "{colors.neutral.100}", _dark: "#272729" } },
        "bg.inverted": { value: { base: "#000000",            _dark: "#000000" } }, // header stays black in both
        "fg":          { value: { base: "{colors.ink}",       _dark: "#ffffff" } },
        "fg.muted":    { value: { base: "{colors.neutral.500}", _dark: "#cccccc" } },
        "fg.inverted": { value: { base: "#ffffff",            _dark: "#ffffff" } },
        "border":      { value: { base: "{colors.neutral.200}", _dark: "#2a2a2c" } },
        "accent":      { value: { base: "{colors.brand.500}", _dark: "{colors.brand.400}" } },
        "accent.fg":   { value: { base: "#ffffff",            _dark: "#ffffff" } },
        "bubble.user.bg": { value: { base: "{colors.bubbleUser}", _dark: "#14532d" } },
        "bubble.bot.bg":  { value: { base: "{colors.bubbleBot}",  _dark: "#1e3a5f" } },
      },
    },
  },
});

export const system = createSystem(defaultConfig, config);
```

```jsx
// src/pages/chat/ui/provider.jsx  (edit)
import { ChakraProvider } from "@chakra-ui/react";
import { system } from "../../../theme/system";
import { ColorModeProvider } from "./color-mode";

export function Provider(props) {
  return (
    <ChakraProvider value={system}>
      <ColorModeProvider {...props} />
    </ChakraProvider>
  );
}
```

Components then read tokens directly: `bg="bg.app"`, `color="fg.muted"`, `borderColor="border"`. Chakra also emits these as CSS custom properties (`var(--chakra-colors-fg-muted)`), which the two `.css` files consume so they track the mode too.

---

## 4. Token mapping (migration cheat-sheet)

Every literal currently in the code maps to one semantic token:

| Current literal(s) | Semantic token | Notes |
| --- | --- | --- |
| `#0066cc`, `var(--color-primary)` | `accent` | Action Blue |
| `#0071e3` | `accent` (hover/focus) | focus blue |
| `#2997ff` | `brand.400` / `accent` in dark | on-dark link |
| `#004fa3` | `brand.700` | pressed |
| `#2563eb`, `#dbeafe` | `accent` / `bubble.bot.bg` | dedupe blues |
| `#1d1d1f` | `fg` | ink |
| `#ffffff`, `"white"` | `bg.canvas` / `fg.inverted` | role decides which |
| `#f5f5f7` | `bg.app` | parchment canvas |
| `#f8fafc`, `#f8f9fa`, `#fafafc` | `bg.canvas` or `bg.subtle` | pick by role |
| `#f1f5f9` | `bg.subtle` | scrollbar track |
| `#e2e8f0`, `#e0e0e0`, `#e9ecef`, `#ced4da`, `#cbd5e1` | `border` | collapse 5 → 1 |
| `#64748b`, `#4a5568`, `#6c757d`, `#7a7a7a`, `#94a3b8` | `fg.muted` | collapse 5 → 1 |
| `#2d3748`, `#334155`, `#475569` | `fg` / `neutral.700` | strong text |
| `#000000`, `"black"` | `bg.inverted` | header / nav |
| `#d1fae5` | `bubble.user.bg` | user bubble |
| `#8b7d6b`, `#a09484` | `border` / `fg.muted` | scrollbar thumb → neutralize |
| `rgba(0,0,0,0.04–0.08)` | shadow tokens (`shadow.sm/md/lg`) | move to `theme.tokens.shadows` |
| `rgba(255,255,255,0.12)` | `border` on dark surfaces | header hairline |

> Exact dark values are first-pass; finalize against `DESIGN.md`'s `surface-tile-*` set and a contrast check (§7).

---

## 5. Dark mode wiring

`next-themes` is already mounted, so most of the runtime is in place. Remaining work:

1. **Default & system preference** — pass `defaultTheme="system"` + `enableSystem` in `ColorModeProvider`, so we honour OS preference on first load.
2. **Toggle UI** — add a color-mode button to the shared `AppHeader` (right slot at `src/shared/AppHeader.jsx:39` is currently an empty `Box` reserved for exactly this). Use Chakra's `useColorMode()` / a small `ColorModeButton`.
3. **Persisted choice** — `next-themes` already persists to `localStorage`; verify the storage key and SSR/flash behavior (`disableTransitionOnChange` is already set, which avoids the transition flash).
4. **Header special case** — the global nav is black in *both* modes (`bg.inverted`), matching `DESIGN.md`'s `surface-black`. Its hairline switches from `rgba(0,0,0,.12)`-on-light intent to `rgba(255,255,255,.12)` — already the value in `AppHeader`, so it's correct on black.
5. **Global CSS** (`index.css`, `chat.css`):
   - `::selection`, `:focus-visible`, scrollbar track/thumb, and the `linear-gradient` chat background all use raw hex → swap to `var(--chakra-colors-*)` so they follow the mode.
   - `body { background / color }` → reference `--chakra-colors-bg-app` / `--chakra-colors-fg`.
6. **Images / shadows** — `DESIGN.md`'s signature product drop-shadow assumes a light surface; verify it reads acceptably on dark or swap to a mode-aware shadow token.

---

## 6. Migration phases

**Phase 0 — Foundation (no visual change)**
- [ ] Add `src/theme/system.js` with primitives + semantic tokens (§3).
- [ ] Swap `defaultSystem` → `system` in `provider.jsx`.
- [ ] Verify app looks identical in light mode (tokens default to today's values).

**Phase 1 — Migrate components to tokens (light only)**
Replace inline hex with semantic tokens, file by file. Suggested order (cheapest → richest):
- [ ] `src/shared/AppHeader.jsx`, `src/shared/ErrorBoundary.jsx`
- [ ] `src/pages/home/Home.jsx` (tab bar: active/inactive colors → `accent` / `fg.muted` / `border`)
- [ ] List pages: `ServicosList.jsx`, `FaqsList.jsx`, `ConteudosList.jsx`, `NotasList.jsx`, `PareceresList.jsx`, `About.jsx`
- [ ] Accordions: `FaqsAccordion.jsx`, `ServicosAccordion.jsx`
- [ ] Chat: `Input.jsx`, `Messages.jsx`, `SystemMessage.jsx`, `UserMessage.tsx`
- [ ] CSS files: `index.css` `:root` becomes a thin alias layer over `--chakra-colors-*`; `chat.css` literals → CSS vars.

**Phase 2 — Enable dark mode**
- [ ] Confirm every semantic token has a sensible `_dark` value.
- [ ] Add the toggle to `AppHeader` + `defaultTheme="system"`.
- [ ] Migrate global CSS hex (selection, scrollbar, gradient, body) to CSS vars.

**Phase 3 — Guardrails**
- [ ] Add an ESLint rule (or a CI `grep`) failing on raw hex / `rgba()` inside `src/**/*.{jsx,tsx}` (allow only in `theme/` and `*.css`).
- [ ] Document the token vocabulary in `DESIGN.md` (or a `THEME.md`) so new code uses tokens.

---

## 7. Verification

- **Contrast** — check every `fg`×`bg` pairing in both modes against WCAG AA (4.5:1 body, 3:1 large). The muted-text and bubble tokens are the risky ones.
- **Visual diff** — screenshot Home + each tab + a standalone route in light *and* dark (the `/run` skill already drives headless Chrome for this; we used it to verify the header refactor).
- **No regressions** — `npm run lint`, `npm run build`, and `npx react-doctor@latest --diff` clean.
- **Flash check** — hard-reload in dark mode; confirm no white flash before hydration.
- **Grep gate** — `grep -rE '#[0-9a-fA-F]{3,8}' src --include=*.jsx --include=*.tsx` returns nothing after Phase 3.

---

## 8. Risks & notes

- **Two parallel CSS files** (`index.css`, `chat.css`) duplicate base resets/typography. Worth merging while we're in here, but out of strict scope — flag separately.
- **`chat.css` uses `100vh`** and gradients tuned for light; the gradient (`#ffffff → #f8f9fa`) needs a dark equivalent or removal.
- **Bubble colors** (sage green / light blue) are brand-meaningful; pick dark variants that preserve the user-vs-bot distinction rather than just darkening.
- **Don't over-tokenize** — keep the semantic set small (~12 tokens). Too many roles is as unmaintainable as too few.
- Estimated effort: Phase 0 ~0.5 day, Phase 1 ~1–2 days (mechanical), Phase 2 ~0.5 day, Phase 3 ~0.5 day.
