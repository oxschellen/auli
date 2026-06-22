/// <reference types="vite/client" />

/** Stable per-build identifier injected by vite.config.js `define`. */
declare const __BUILD_ID__: string;

/** App version (e.g. "v.0.1.43") sourced from package.json via vite.config.js. */
declare const __APP_VERSION__: string;

interface ImportMetaEnv {
  /** Chat question endpoint; defaults to production when unset. */
  readonly VITE_API_URL?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
