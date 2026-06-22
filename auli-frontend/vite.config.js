import { readFileSync } from 'node:fs'
import { fileURLToPath, URL } from 'node:url'
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// Single source of truth for the displayed app version: package.json. Read at
// config load and injected via `define`, so the version can't drift from a
// hardcoded constant and package.json isn't bundled into the client.
const pkg = JSON.parse(readFileSync(new URL('./package.json', import.meta.url), 'utf-8'))

const chunkMap = {
  'vendor-react': ['react', 'react-dom'],
  'vendor-chakra': ['@chakra-ui/react', '@emotion/react', '@emotion/styled'],
  'vendor-icons': ['react-icons'],
  'vendor-utils': ['axios', 'framer-motion', 'react-markdown'],
}

function manualChunks(id) {
  for (const [chunk, pkgs] of Object.entries(chunkMap)) {
    if (pkgs.some(pkg => id.includes(`/node_modules/${pkg}/`))) return chunk
  }
}

// Stable per-build identifier, evaluated once when the config loads (build
// start / dev server start). Used to cache-bust static assets only when a new
// build is deployed, so reloads of the same build hit the browser cache.
const buildId = String(Date.now())

// https://vitejs.dev/config/
export default defineConfig({
  define: {
    __BUILD_ID__: JSON.stringify(buildId),
    __APP_VERSION__: JSON.stringify(`v.${pkg.version}`),
  },
  plugins: [react()],
  resolve: {
    // Matches the "@/*" path alias in tsconfig.json / jsconfig.json so imports
    // like "@/shared/fetchers" resolve at build time, not just in the editor.
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  build: {
    rolldownOptions: {
      output: {
        manualChunks,
      },
    },
  },
  test: {
    // Pure-logic tests run in Node; component tests opt into jsdom per-file
    // with a `// @vitest-environment jsdom` docblock.
    environment: 'node',
    include: ['src/**/*.test.{ts,tsx}'],
    setupFiles: ['./src/test/setup.ts'],
  },
})
