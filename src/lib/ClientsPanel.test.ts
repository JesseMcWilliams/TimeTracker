// @vitest-environment jsdom
import { cleanup, render, screen } from '@testing-library/svelte'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

/**
 * The Clients page totals hours across a client's contracts inside an `{@const}` in an
 * `{#each}`, which is the least obvious reactive construct in this codebase. These
 * assert it tracks a refreshed store rather than being computed once at mount.
 */

vi.mock('./api', () => ({
  api: {
    listClients: vi.fn(async () => []),
    listContracts: vi.fn(),
    listEntries: vi.fn(async () => []),
    createClient: vi.fn(),
  },
}))

const { api } = await import('./api')
const { store, refreshAfterEntryChange } = await import('./store.svelte')
const ClientsPanel = (await import('./ClientsPanel.svelte')).default

const CLIENT = {
  id: 1,
  name: 'SAIA',
  notes: null,
  prime: null,
  sub: null,
  externalId: null,
  weekStart: 'monday',
  weekEnd: 'sunday',
  defaultTrackingCodeId: null,
  minimumIncrementMinutes: 15,
  archivedAt: null,
}

/** Two contracts on one client, so the page has to sum them rather than show one. */
function contracts(loggedA: number, loggedB: number) {
  const base = {
    clientId: 1,
    currency: 'USD',
    externalId: null,
    startDate: null,
    notes: null,
    filenameDate: 'end',
    archivedAt: null,
    currentRate: 150,
  }
  return [
    { ...base, id: 1, name: 'Blueprint', totalHoursBudgeted: 100, hoursLogged: loggedA },
    { ...base, id: 2, name: 'Support', totalHoursBudgeted: 40, hoursLogged: loggedB },
  ]
}

describe('Clients page hours totals', () => {
  afterEach(cleanup)

  beforeEach(() => {
    vi.clearAllMocks()
    store.clients = [CLIENT] as never
    store.contracts = contracts(60, 10) as never
  })

  it('combines the budgets and remaining hours across the client contracts', () => {
    render(ClientsPanel, { props: { onSelectClient: () => {} } })

    expect(screen.getByText('140 h'), 'combined budget').toBeTruthy()
    expect(screen.getByText('70 h'), 'combined remaining').toBeTruthy()
  })

  it('comes down when a time entry adds logged hours', async () => {
    render(ClientsPanel, { props: { onSelectClient: () => {} } })
    expect(screen.getByText('70 h')).toBeTruthy()

    vi.mocked(api.listContracts).mockResolvedValueOnce(contracts(64, 10) as never)
    await refreshAfterEntryChange()

    expect(screen.queryByText('70 h'), 'the stale total is still on screen').toBeNull()
    expect(screen.getByText('66 h')).toBeTruthy()
  })

  it('counts only contracts that actually carry a budget', async () => {
    // One budgeted contract and one without: the unbudgeted one contributes neither
    // budget nor logged hours, so it cannot drag the remaining figure down.
    const mixed = contracts(60, 10)
    mixed[1].totalHoursBudgeted = 0
    store.contracts = mixed as never

    render(ClientsPanel, { props: { onSelectClient: () => {} } })

    expect(screen.getByText('100 h'), 'budget of the budgeted contract only').toBeTruthy()
    expect(screen.getByText('40 h'), 'remaining of the budgeted contract only').toBeTruthy()
  })

  it('shows a dash when no contract on the client has a budget', () => {
    const none = contracts(60, 10).map((c) => ({ ...c, totalHoursBudgeted: 0 }))
    store.contracts = none as never

    render(ClientsPanel, { props: { onSelectClient: () => {} } })

    expect(screen.getAllByText('—').length).toBe(2)
  })
})
