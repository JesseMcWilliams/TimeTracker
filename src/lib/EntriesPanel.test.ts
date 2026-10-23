// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

/**
 * Drives the real add-entry flow on the Entries page and asserts the app refetches
 * contracts as a result.
 *
 * The earlier tests for this bug proved less than they looked like they did: one called
 * the shared refresh directly, and the source-level guard only proved the function name
 * appears somewhere in the file. Neither shows that clicking Add entry reaches it. This
 * one does.
 */

vi.mock('./api', () => ({
  api: {
    listEntries: vi.fn(async () => []),
    listContracts: vi.fn(async () => []),
    listClients: vi.fn(async () => []),
    listTrackingCodes: vi.fn(async () => []),
    createManualEntry: vi.fn(async () => 1),
    deleteEntry: vi.fn(async () => undefined),
    updateEntry: vi.fn(async () => undefined),
  },
}))

vi.mock('@tauri-apps/plugin-dialog', () => ({ confirm: vi.fn(async () => true) }))

const { api } = await import('./api')
const { store } = await import('./store.svelte')
const EntriesPanel = (await import('./EntriesPanel.svelte')).default

const CONTRACT = {
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
  hoursLogged: 183.75,
}

const CLIENT = { id: 1, name: 'SAIA', weekStart: 'monday', weekEnd: 'sunday', archivedAt: null }

function renderPanel() {
  return render(EntriesPanel, { props: { onSelectEntry: () => {}, onGoToImport: () => {} } })
}

describe('Entries page add-entry flow', () => {
  afterEach(cleanup)

  beforeEach(() => {
    vi.clearAllMocks()
    store.clients = [CLIENT] as never
    store.contracts = [CONTRACT] as never
    store.entries = [] as never
  })

  it('refetches contracts after adding an entry, so hours figures are not left stale', async () => {
    const { container } = renderPanel()

    const [contractSelect] = container.querySelectorAll('select')
    await fireEvent.change(contractSelect, { target: { value: '1' } })

    const [start, end] = container.querySelectorAll('input[type="datetime-local"]')
    await fireEvent.input(start, { target: { value: '2026-10-23T09:00' } })
    await fireEvent.input(end, { target: { value: '2026-10-23T13:00' } })

    await fireEvent.click(screen.getByText('Add entry'))

    await waitFor(() => expect(api.createManualEntry).toHaveBeenCalledTimes(1))
    await waitFor(() =>
      expect(
        api.listContracts,
        'adding an entry must refetch contracts, or every hours display stays stale',
      ).toHaveBeenCalled(),
    )
  })
})
