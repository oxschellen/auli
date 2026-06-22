# Theme & Color Tokens

This project has **one source of truth for color**: [`src/theme/system.js`](src/theme/system.js).
Components never use raw color literals — they consume **semantic tokens**, which
carry both a light (`base`) and a `_dark` value, so color mode is a data change,
not a code change.

A lint rule (`no-restricted-syntax` in [`eslint.config.js`](eslint.config.js))
**fails the build** on any raw hex / `rgb()` / `hsl()` literal inside
`src/**/*.{jsx,tsx}`. Raw values are allowed only in `src/theme/system.js` (the
token definitions) and in `.css` files (as `var(--chakra-colors-*)`).

## How to use tokens

In Chakra style props, reference the token by name:

```jsx
<Box bg="bg.app" color="fg.muted" borderColor="border" />
<Button bg="accent" color="accent.fg" />
```

Inside shorthand strings or plain CSS, use the emitted CSS variable
(`var(--chakra-colors-<token-with-dashes>)`):

```jsx
<Box css={{ borderBottom: "1px solid var(--chakra-colors-border)" }} />
```

```css
/* src/index.css */
body { background: var(--chakra-colors-bg-app); color: var(--chakra-colors-fg); }
```

## Semantic token vocabulary

Keep this set small. If you need a new role, add it here and to `system.js` —
don't reach for a raw value or an off-list Chakra default.

| Token | Role | Light | Dark |
| --- | --- | --- | --- |
| `bg.canvas` | Card / surface background | `#ffffff` | `#1d1d1f` |
| `bg.app` | App / page background | `#f5f5f7` | `#000000` |
| `bg.subtle` | Subtle fill (scrollbar track, row hover) | `#f1f5f9` | `#272729` |
| `bg.inverted` | Header / nav (black in **both** modes) | `#000000` | `#000000` |
| `bg.overlay` | Translucent hover overlay on any surface | `rgba(0,0,0,.06)` | `rgba(255,255,255,.1)` |
| `fg` | Primary text | `#1d1d1f` | `#ffffff` |
| `fg.muted` | Secondary / muted text | `#64748b` | `#cccccc` |
| `fg.inverted` | Text on inverted/black surfaces | `#ffffff` | `#ffffff` |
| `fg.invertedMuted` | Muted text on inverted surfaces (header subtitle) | `rgba(255,255,255,.6)` | `rgba(255,255,255,.6)` |
| `border` | All borders / hairlines | `#e2e8f0` | `#2a2a2c` |
| `border.inverted` | Hairline on inverted/black surfaces | `rgba(255,255,255,.12)` | `rgba(255,255,255,.12)` |
| `accent` | Action blue (links, primary buttons) | `#0066cc` | `#2997ff` |
| `accent.fg` | Text/icon on an accent fill | `#ffffff` | `#ffffff` |
| `bubble.user` | Chat: the user's message bubble | `#dbeafe` | `#1e3a5f` |

There is also one **semantic shadow** token:

| Token | Role | Light | Dark |
| --- | --- | --- | --- |
| `focusRing` | Accent focus ring for text inputs (`boxShadow="focusRing"`) | `0 0 0 3px rgba(0,102,204,.15)` | `0 0 0 3px rgba(41,151,255,.25)` |

> The raw palette these are built from (`brand.*`, `ink`, `neutral.*`, …) lives
> in the `tokens.colors` block of `system.js`. Treat primitives as private —
> components should reference **semantic** tokens, not primitives.

## Color mode

`next-themes` (via `ColorModeProvider`) drives the mode with
`defaultTheme="system"` + `enableSystem`, so the OS preference wins on first
load and the user's explicit choice persists to `localStorage`. The toggle lives
in the shared `AppHeader`. Adding a new mode-aware color means giving its
semantic token a `_dark` value — never a per-component conditional.
