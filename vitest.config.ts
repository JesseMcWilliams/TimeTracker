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
    // Most tests are plain logic and run fastest without a DOM. The few that render a
    // component opt in with a `// @vitest-environment jsdom` comment at the top.
    environment: 'node',
  },
})
