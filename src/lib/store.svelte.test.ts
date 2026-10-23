import { beforeEach, describe, expect, it, vi } from 'vitest'

// The store talks to the backend through this module, so stubbing it lets the test
// observe exactly which fetches a refresh performs without a Tauri runtime.
vi.mock('./api', () => ({
  api: {
    listEntries: vi.fn(async () => []),
    listContracts: vi.fn(async () => []),
    listClients: vi.fn(async () => []),
  },
}))

const { api } = await import('./api')
const { refreshAfterEntryChange, store } = await import('./store.svelte')

describe('refreshAfterEntryChange', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  /// The bug this guards: a contract's logged hours are computed by the backend and
  /// cached on the contract objects, so refreshing only the entries leaves every hours
  /// display showing the figure from whenever contracts were last fetched.
  it('refetches contracts as well as entries, so logged hours are not left stale', async () => {
    await refreshAfterEntryChange()

    expect(api.listEntries).toHaveBeenCalledTimes(1)
    expect(api.listContracts).toHaveBeenCalledTimes(1)
  })

  it('writes both results into the store', async () => {
    const entries = [{ id: 1 }]
    const contracts = [{ id: 7, totalHoursBudgeted: 40, hoursLogged: 34.5 }]
    vi.mocked(api.listEntries).mockResolvedValueOnce(entries as never)
    vi.mocked(api.listContracts).mockResolvedValueOnce(contracts as never)

    await refreshAfterEntryChange()

    expect(store.entries).toEqual(entries)
    expect(store.contracts).toEqual(contracts)
  })

  it('issues both fetches concurrently rather than one after the other', async () => {
    const order: string[] = []
    vi.mocked(api.listEntries).mockImplementationOnce(async () => {
      order.push('entries:start')
      await Promise.resolve()
      order.push('entries:end')
      return [] as never
    })
    vi.mocked(api.listContracts).mockImplementationOnce(async () => {
      order.push('contracts:start')
      return [] as never
    })

    await refreshAfterEntryChange()

    expect(order.indexOf('contracts:start')).toBeLessThan(order.indexOf('entries:end'))
  })
})
