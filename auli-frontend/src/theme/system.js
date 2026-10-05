import { createSystem, defaultConfig, defineConfig } from "@chakra-ui/react";

// Single source of truth for color. Primitives are the raw palette (seeded from
// DESIGN.md); semantic tokens are what components consume, and carry both a
// light (`base`) and `_dark` value so color mode is a data change, not a code
// change. See THEME.md for the token vocabulary and the literal→token mapping.
const config = defineConfig({
  theme: {
    tokens: {
      colors: {
        // ── primitives: mode-agnostic raw palette ──
        brand: {
          50: { value: "#dbeafe" },
          400: { value: "#2997ff" }, // primary-on-dark / sky link
          500: { value: "#0066cc" }, // primary / Action Blue
          600: { value: "#0071e3" }, // primary-focus
          700: { value: "#004fa3" }, // pressed
        },
        ink: { value: "#1d1d1f" },
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
        // Chat: the user's message bubble (system/bot replies are plain bg.canvas)
        bubble: { value: "#dbeafe" },
      },
      // One body font for the whole app — tabs, chat bubbles, inputs, lists.
      // Components reference fontFamily="body" instead of repeating this stack.
      fonts: {
        body: {
          value:
            '"SF Pro Text", system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
        },
      },
    },
    semanticTokens: {
      colors: {
        // ── semantic: what components reference; { light, dark } ──
        bg: {
          canvas: { value: { base: "{colors.canvas}", _dark: "#1d1d1f" } },
          // Quase-preto no escuro (era #000000): com o preto puro as superfícies (`canvas` #1d1d1f)
          // flutuavam sem degrau, e o contraste máximo cansava na leitura longa (D-UI-4).
          app: { value: { base: "{colors.parchment}", _dark: "#111113" } },
          // O bloco do Resumo da resposta do chat (D-UI-6): um tom do azul da marca, nos dois modos.
          resumo: { value: { base: "#f3f7fd", _dark: "#15253a" } },
          subtle: { value: { base: "{colors.neutral.100}", _dark: "#272729" } },
          inverted: { value: { base: "#000000", _dark: "#000000" } }, // header stays black both modes
          // translucent hover overlay that reads correctly on any surface
          overlay: { value: { base: "rgba(0,0,0,0.06)", _dark: "rgba(255,255,255,0.1)" } },
          // fill for inert/unavailable states on the Brazil selection map
          mapInactive: { value: { base: "{colors.neutral.200}", _dark: "#9aa0a6" } },
          // Estado SELECIONÁVEL do mapa: azul mais claro que o `accent` do app, específico do
          // mapa. Mesmo valor nos dois modos, como era quando o literal morava no componente —
          // o que separa os estados é o traço (`bg.canvas`), e esse já acompanha o modo.
          mapActive: { value: { base: "#3f9df2", _dark: "#3f9df2" } },
          // Realce de hover/foco no mapa: turquesa, para não confundir com o azul selecionável
          // nem com o cinza inerte.
          mapActiveHover: { value: { base: "#2dd4bf", _dark: "#2dd4bf" } },
          // search-term highlight (<mark>). Translucent like `overlay` so it reads on any surface
          // it lands on (canvas, app, subtle) without a per-surface variant. Amber on purpose:
          // `accent` is the link color here, and marking matches in it would read as clickable.
          highlight: { value: { base: "rgba(250,204,21,0.45)", _dark: "rgba(250,204,21,0.28)" } },
        },
        fg: {
          DEFAULT: { value: { base: "{colors.ink}", _dark: "#ffffff" } },
          muted: { value: { base: "{colors.neutral.500}", _dark: "#cccccc" } },
          // Text on inverted (always-black) surfaces, e.g. the header. Kept as
          // flat 2-level leaves on purpose: Chakra v3 does not reliably emit a
          // 3-level semantic var (fg.inverted.muted) when the parent carries a
          // DEFAULT, which left the header subtitle with no resolved color — it
          // fell back to `fg`, invisible on the black header in light mode.
          inverted: { value: { base: "#ffffff", _dark: "#ffffff" } },
          invertedMuted: { value: { base: "rgba(255,255,255,0.6)", _dark: "rgba(255,255,255,0.6)" } },
          // text inside a <mark>: the browser default forces near-black, unreadable over a dark
          // surface. Flat 2-level leaf for the same reason as `inverted` above.
          highlight: { value: { base: "{colors.ink}", _dark: "#ffffff" } },
          // Rótulo de controle INERTE (aba de coleção que a entidade não tem). Precisa ser um
          // degrau visível abaixo de `muted`, que já é a cor da aba não-selecionada mas clicável —
          // sem isso, "inativa" e "apenas não-escolhida" ficam indistinguíveis. Contraste baixo de
          // propósito (~2,5:1 sobre `bg.subtle`): a WCAG 1.4.3 dispensa componentes inativos, e a
          // dica de que o botão não responde é justamente o apagamento. Irmão de `bg.mapInactive`.
          disabled: { value: { base: "{colors.neutral.400}", _dark: "#7d7d82" } },
          // Sigla da UF desenhada sobre o realce turquesa do mapa (`bg.mapActiveHover`). Tinta
          // escura pelo contraste, e igual nos dois modos porque a superfície sob ela também é.
          // Folha plana de 2 níveis pelo mesmo motivo de `inverted` acima.
          mapLabel: { value: { base: "#083344", _dark: "#083344" } },
        },
        border: {
          DEFAULT: { value: { base: "{colors.neutral.200}", _dark: "#2a2a2c" } },
          // hairline on inverted (always-black) surfaces, e.g. header
          inverted: { value: { base: "rgba(255,255,255,0.12)", _dark: "rgba(255,255,255,0.12)" } },
        },
        accent: {
          DEFAULT: { value: { base: "{colors.brand.500}", _dark: "{colors.brand.400}" } },
          fg: { value: { base: "#ffffff", _dark: "#ffffff" } },
          // Texto pequeno em azul (o rótulo RESUMO): o brand.500 fica no limite do contraste em
          // 12px sobre `bg.resumo`; o 700 passa com folga.
          strong: { value: { base: "{colors.brand.700}", _dark: "{colors.brand.400}" } },
        },
        bubble: {
          user: { value: { base: "{colors.bubble}", _dark: "#1e3a5f" } },
        },
        // Selo "depende de condição" das fontes (D-UI-8).
        condicional: {
          bg: { value: { base: "#fff3e0", _dark: "#3a2a12" } },
          fg: { value: { base: "#8a4600", _dark: "#ffcc80" } },
        },
      },
      shadows: {
        // A caixa de mensagem descolada do fundo (D-UI-4) e o painel que abre sobre ela.
        composer: {
          value: { base: "0 2px 10px rgba(15,23,42,0.06)", _dark: "0 2px 10px rgba(0,0,0,0.35)" },
        },
        popover: {
          value: { base: "0 10px 30px rgba(15,23,42,0.16)", _dark: "0 10px 30px rgba(0,0,0,0.5)" },
        },
        // accent-colored focus ring for text inputs
        focusRing: {
          value: {
            base: "0 0 0 3px rgba(0,102,204,0.15)",
            _dark: "0 0 0 3px rgba(41,151,255,0.25)",
          },
        },
        // subtle lift used by hoverable cards (e.g. the state-selection cards)
        cardHover: {
          value: {
            base: "0 4px 16px rgba(0,0,0,0.08)",
            _dark: "0 4px 16px rgba(0,0,0,0.4)",
          },
        },
      },
    },
  },
});

export const system = createSystem(defaultConfig, config);
