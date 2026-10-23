import { readdirSync, readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'

/**
 * A contract's logged hours are computed by the backend and cached on the contract
 * objects in the store, so any UI that changes a time entry has to refetch contracts
 * too — otherwise the Total Hrs / Remaining columns on Clients and Contracts, and the
 * low-hours warning on Entries, keep showing the figure from whenever contracts were
 * last fetched.
 *
 * That is a wiring mistake: the code compiles, the types check, and both the Rust and
 * store test suites stay green, because nothing is individually wrong — a component
 * simply calls the narrower refresh. It shipped once for exactly that reason. These
 * tests read the component sources and assert the rule directly, which is the only
 * level at which a missing call is visible without mounting every component.
 */

const UI_DIR = join(dirname(fileURLToPath(import.meta.url)), '..', 'src')

/** Every api call that creates, edits, removes, restores, or imports a time entry — so
 * every call after which a contract's logged hours may have moved. */
const ENTRY_MUTATIONS = [
  'createManualEntry',
  'updateEntry',
  'updateEntryMetadata',
  'deleteEntry',
  'restoreEntry',
  'startTimer',
  'stopTimer',
  'importTimeEntries',
  'purgeDeletedEntries',
]

const REQUIRED_REFRESH = 'refreshAfterEntryChange'

function uiSources(): { name: string; text: string }[] {
  const files: { name: string; text: string }[] = []
  const walk = (dir: string) => {
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      const full = join(dir, entry.name)
      if (entry.isDirectory()) walk(full)
      else if (entry.name.endsWith('.svelte')) files.push({ name: entry.name, text: readFileSync(full, 'utf8') })
    }
  }
  walk(UI_DIR)
  return files
}

function mutationsIn(text: string): string[] {
  return ENTRY_MUTATIONS.filter((m) => text.includes(`api.${m}(`))
}

describe('components that change time entries refresh the contracts behind them', () => {
  const sources = uiSources()

  it('finds the component sources to check', () => {
    expect(sources.length).toBeGreaterThan(10)
  })

  it('covers every entry-mutating api call the app actually exposes', () => {
    // If a new entry-mutating call is added to api.ts without being listed above, this
    // guard would silently stop covering it.
    const apiText = readFileSync(join(UI_DIR, 'lib', 'api.ts'), 'utf8')
    for (const mutation of ENTRY_MUTATIONS) {
      expect(apiText, `api.ts no longer defines ${mutation}; update ENTRY_MUTATIONS`).toContain(`${mutation}:`)
    }
  })

  it.each(uiSources().filter((f) => mutationsIn(f.text).length > 0).map((f) => [f.name, f.text] as const))(
    '%s routes its entry changes through the refresh that also refetches contracts',
    (name, text) => {
      expect(
        text.includes(REQUIRED_REFRESH),
        `${name} calls ${mutationsIn(text).join(', ')} but never ${REQUIRED_REFRESH}, ` +
          `so logged hours will be stale after that change`,
      ).toBe(true)
    },
  )

  /// refreshEntries on its own is the exact shape of the original bug: it updates the
  /// entry list and leaves every hours figure untouched. The store may call it; the UI
  /// must go through refreshAfterEntryChange.
  it('no component calls the entries-only refresh directly', () => {
    const offenders = sources
      .filter((f) => /\brefreshEntries\s*\(/.test(f.text))
      .map((f) => f.name)

    expect(offenders, `these call refreshEntries() directly instead of ${REQUIRED_REFRESH}`).toEqual([])
  })
})
