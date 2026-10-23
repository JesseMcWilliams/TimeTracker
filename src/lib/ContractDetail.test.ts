// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

/**
 * Covers the Hours section's two modes end to end: that each button press reaches the
 * right backend call with the right number. The previous bug in this area was a wiring
 * mistake no unit test could see, so these drive the real controls.
 */

vi.mock('./api', () => ({
  api: {
    addContractHours: vi.fn(async () => undefined),
    setContractHours: vi.fn(async () => undefined),
    updateContractDetails: vi.fn(async () => undefined),
    updateContractRate: vi.fn(async () => undefined),
    archiveContract: vi.fn(async () => undefined),
    listContracts: vi.fn(async () => []),
    listClients: vi.fn(async () => []),
    listEntries: vi.fn(async () => []),
  },
}))

vi.mock('@tauri-apps/plugin-dialog', () => ({ confirm: vi.fn(async () => true) }))

const { api } = await import('./api')
const { store } = await import('./store.svelte')
const ContractDetail = (await import('./ContractDetail.svelte')).default

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
  totalHoursBudgeted: 200,
  hoursLogged: 183.75,
}

function renderDetail() {
  return render(ContractDetail, { props: { contractId: 1, onBack: () => {} } })
}

/** The number input in the Hours section. */
function hoursInput(container: HTMLElement): HTMLInputElement {
  return container.querySelector('#hours-adjustment') as HTMLInputElement
}

describe('Contract detail Hours section', () => {
  afterEach(cleanup)

  beforeEach(() => {
    vi.clearAllMocks()
    store.clients = [{ id: 1, name: 'SAIA', archivedAt: null }] as never
    store.contracts = [CONTRACT] as never
  })

  it('adds to the running total in the default Adjust by mode', async () => {
    const { container } = renderDetail()

    await fireEvent.input(hoursInput(container), { target: { value: '35.25' } })
    await fireEvent.click(screen.getByText('Apply adjustment'))

    await waitFor(() => expect(api.addContractHours).toHaveBeenCalledWith(1, 35.25, undefined))
    expect(api.setContractHours).not.toHaveBeenCalled()
  })

  /// The point of the new mode: state the total, do not work out the gap by hand.
  it('sends the total as given in Set total to mode', async () => {
    const { container } = renderDetail()

    await fireEvent.click(screen.getByLabelText('Set total to'))
    await fireEvent.input(hoursInput(container), { target: { value: '235.25' } })
    await fireEvent.click(screen.getByText('Set total'))

    await waitFor(() => expect(api.setContractHours).toHaveBeenCalledWith(1, 235.25, undefined))
    expect(api.addContractHours).not.toHaveBeenCalled()
  })

  it('passes a note through when one is given', async () => {
    const { container } = renderDetail()

    await fireEvent.click(screen.getByLabelText('Set total to'))
    await fireEvent.input(hoursInput(container), { target: { value: '10' } })
    await fireEvent.input(container.querySelector('#hours-adjustment-note') as HTMLInputElement, {
      target: { value: 'PO amendment 3' },
    })
    await fireEvent.click(screen.getByText('Set total'))

    await waitFor(() => expect(api.setContractHours).toHaveBeenCalledWith(1, 10, 'PO amendment 3'))
  })

  /// Zero means "no change" when adjusting, but "clear the budget" when setting, so the
  /// button must stay usable in one mode and not the other.
  it('allows a total of zero but not an adjustment of zero', async () => {
    const { container } = renderDetail()

    await fireEvent.input(hoursInput(container), { target: { value: '0' } })
    expect((screen.getByText('Apply adjustment') as HTMLButtonElement).disabled).toBe(true)

    await fireEvent.click(screen.getByLabelText('Set total to'))
    expect((screen.getByText('Set total') as HTMLButtonElement).disabled).toBe(false)
  })

  it('previews the resulting budget and remaining hours before committing', async () => {
    const { container } = renderDetail()

    await fireEvent.click(screen.getByLabelText('Set total to'))
    await fireEvent.input(hoursInput(container), { target: { value: '235.25' } })

    // 235.25 budgeted against 183.75 logged leaves 51.5 = 51h 30m.
    await waitFor(() => expect(screen.getByText(/235h 15m/)).toBeTruthy())
    expect(screen.getByText(/51h 30m/)).toBeTruthy()
  })
})
