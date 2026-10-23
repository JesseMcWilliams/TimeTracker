import { svelte } from '@sveltejs/vite-plugin-svelte'
import { defineConfig } from 'vitest/config'

// Separate from vite.config.ts so the app build is untouched by test-only settings.
export default defineConfig({
  plugins: [svelte()],
  // Without this, importing a .svelte component under Vitest resolves Svelte's server
  // build, whose mount() throws "not available on the server". Component tests need the
  // client build; this is Svelte's documented workaround and is scoped to test runs.
  resolve: { conditions: ['browser'] },
  test: {
    // Most tests are plain logic and run fastest without a DOM. The ones that render a
    // component opt in with a `// @vitest-environment jsdom` comment at the top.
    environment: 'node',
    server: {
      deps: {
        // @testing-library/svelte holds component state in its own .svelte.js module.
        // Dependencies are externalised by default, which leaves that file uncompiled
        // and its runes throwing "rune_outside_svelte" the moment a component mounts.
        // Inlining routes it through the Svelte plugin like first-party code.
        inline: [/@testing-library\/svelte/, /svelte-core/],
      },
    },
  },
})
