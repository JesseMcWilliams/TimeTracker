// @vitest-environment jsdom
import { cleanup, render, screen } from '@testing-library/svelte'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

/**
 * Renders the real Contracts page and checks that its Remaining column follows a change
 * to the underlying data. The v0.6.1 fix made every entry-change path call a refresh
 * that refetches contracts; this asserts the other half — that the rendered figure
 * actually tracks the refreshed store, rather than the page holding its own snapshot.
 */

vi.mock('./api', () => ({
  api: {
    listContracts: vi.fn(),
    listClients: vi.fn(async () => []),
    listEntries: vi.fn(async () => []),
    createContract: vi.fn(),
  },
}))

const { api } = await import('./api')
const { store, refreshAfterEntryChange } = await import('./store.svelte')
const ContractsPanel = (await import('./ContractsPanel.svelte')).default

const CLIENT = { id: 1, name: 'SAIA', archivedAt: null }

function contract(hoursLogged: number) {
  return {
    id: 1,
    clientId: 1,
    name: 'PS - Blueprint',
    currency: 'USD',
    externalId: null,
    startDate: null,
    notes: null,
    filenameDate: 'end',
    archivedAt: null,
    currentRate: 150,
    totalHoursBudgeted: 235.25,
    hoursLogged,
  }
}

describe('Contracts page Remaining column', () => {
  // Vitest runs without globals here, so the library's auto-cleanup hook never
  // registers and renders would otherwise accumulate across tests.
  afterEach(cleanup)

  beforeEach(() => {
    vi.clearAllMocks()
    store.clients = [CLIENT] as never
    store.contracts = [contract(183.75)] as never
  })

  it('shows remaining as budget minus logged', () => {
    render(ContractsPanel, { props: { onSelectContract: () => {} } })

    // 235.25 - 183.75 = 51.5
    expect(screen.getByText('51.5 h')).toBeTruthy()
  })

  /// The bug as reported: log more time, and the figure on screen does not come down.
  it('comes down when a time entry adds logged hours', async () => {
    render(ContractsPanel, { props: { onSelectContract: () => {} } })
    expect(screen.getByText('51.5 h')).toBeTruthy()

    // A 4-hour entry is logged elsewhere in the app, then the shared refresh runs.
    vi.mocked(api.listContracts).mockResolvedValueOnce([contract(187.75)] as never)
    await refreshAfterEntryChange()

    expect(screen.queryByText('51.5 h'), 'the stale figure is still on screen').toBeNull()
    expect(screen.getByText('47.5 h')).toBeTruthy()
  })

  it('renders a dash, not a zero, for a contract with no budget', () => {
    store.contracts = [{ ...contract(90.5), id: 2, name: 'Support', totalHoursBudgeted: 0 }] as never

    render(ContractsPanel, { props: { onSelectContract: () => {} } })

    expect(screen.queryByText('0 h')).toBeNull()
    expect(screen.getAllByText('—').length).toBeGreaterThan(0)
  })
})
