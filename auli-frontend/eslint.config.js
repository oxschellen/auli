import react from "eslint-plugin-react";
import reactHooks from "eslint-plugin-react-hooks";
import reactRefresh from "eslint-plugin-react-refresh";
import tseslint from "typescript-eslint";

export default tseslint.config(
  { ignores: ["dist"] },
  {
    files: ["**/*.{js,jsx,ts,tsx}"],
    languageOptions: {
      ecmaVersion: "latest",
      sourceType: "module",
      parserOptions: {
        ecmaFeatures: {
          jsx: true,
        },
      },
    },
    settings: {
      react: {
        version: "detect",
      },
    },
    plugins: {
      react,
      "react-hooks": reactHooks,
      "react-refresh": reactRefresh,
    },
    rules: {
      ...react.configs.recommended.rules,
      ...react.configs["jsx-runtime"].rules,
      ...reactHooks.configs.recommended.rules,
      "react/prop-types": "off",
      "react-hooks/exhaustive-deps": "error",
      "react-refresh/only-export-components": "off",
    },
  },
  // TypeScript-only rules (sets the @typescript-eslint parser for .ts/.tsx).
  {
    files: ["**/*.{ts,tsx}"],
    extends: [...tseslint.configs.recommended],
  },
  // Color guardrail: components must consume semantic tokens (bg.app, fg.muted,
  // accent, …) — never raw color literals. Raw values live only in the token
  // source of truth (src/theme/system.js) and in .css files (as Chakra vars).
  // See COLOR_MODE_PLAN.md §3–4 for the token vocabulary and literal→token mapping.
  {
    files: ["src/**/*.{jsx,tsx}"],
    rules: {
      "no-restricted-syntax": [
        "error",
        {
          selector:
            "Literal[value=/#[0-9a-fA-F]{3,8}\\b|\\b(?:rgba?|hsla?)\\(/]",
          message:
            "No raw color literals in components. Use a semantic token (e.g. bg.app, fg.muted, accent, border) from src/theme/system.js.",
        },
        {
          selector:
            "TemplateElement[value.raw=/#[0-9a-fA-F]{3,8}\\b|\\b(?:rgba?|hsla?)\\(/]",
          message:
            "No raw color literals in components. Use a semantic token or var(--chakra-colors-*) from src/theme/system.js.",
        },
      ],
    },
  },
);
