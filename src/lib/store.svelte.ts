import { api, type Client, type Contract, type ImportTemplate, type TimeEntry, type TrackingCode, type UserProfile } from './api'

const EMPTY_PROFILE: UserProfile = {
  firstName: null,
  lastName: null,
  fullName: null,
  email: null,
  outputFolder: null,
  outputType: 'xlsx',
  windowWidth: null,
  windowHeight: null,
  windowX: null,
  windowY: null,
  defaultStartPage: 'timer',
  launchPosition: 'default',
  theme: 'system',
  customBg: null,
  customText: null,
  customButtonBg: null,
  customButtonText: null,
}

export const store = $state({
  clients: [] as Client[],
  contracts: [] as Contract[],
  entries: [] as TimeEntry[],
  deletedEntries: [] as TimeEntry[],
  activeTimers: [] as TimeEntry[],
  trackingCodesByClient: {} as Record<number, TrackingCode[]>,
  archivedTrackingCodes: [] as TrackingCode[],
  importTemplates: [] as ImportTemplate[],
  userProfile: { ...EMPTY_PROFILE } as UserProfile,
  loading: false,
  error: '',
})

/** Set by the currently-open detail/edit page so navigation can warn before discarding
 * unsaved changes. Cleared automatically whenever navigation actually proceeds. */
export const navGuard = $state({ isDirty: false })

export type EntriesFilterPeriod = 'lastweek' | 'last2weeks' | 'month' | '3months' | 'all'

/** Entries page filter state, lifted out of the component so it survives navigating
 * away (e.g. into an entry's detail page, or anywhere else) and back — otherwise
 * Svelte tears the page down and recreates it fresh, losing local component state. */
export const entriesFilter = $state({
  period: 'lastweek' as EntriesFilterPeriod,
  contractId: '' as number | '',
  trackingCodeId: '' as number | '',
})

export async function refreshArchivedTrackingCodes() {
  store.archivedTrackingCodes = await api.listArchivedTrackingCodes()
}

export async function refreshImportTemplates() {
  store.importTemplates = await api.listImportTemplates()
}

export async function refreshUserProfile() {
  store.userProfile = (await api.getUserProfile()) ?? { ...EMPTY_PROFILE }
}

/** Fetches ALL clients, including archived ones — archived clients must stay resolvable
 * (e.g. for Trash's Archived Clients section). Use `activeClients()` for lists/dropdowns
 * that should exclude archived clients. */
export async function refreshClients() {
  store.clients = await api.listClients(true)
}

export function activeClients(): Client[] {
  return store.clients.filter((c) => !c.archivedAt)
}

/** Fetches ALL contracts, including archived ones — archived contracts must stay
 * resolvable so entries referencing them can still show the (struck-through) name.
 * Use `activeContracts()` for lists/dropdowns that should exclude archived contracts. */
export async function refreshContracts() {
  store.contracts = await api.listContracts(true)
}

export function activeContracts(): Contract[] {
  return store.contracts.filter((c) => !c.archivedAt)
}

export function isContractArchived(contractId: number): boolean {
  return !!store.contracts.find((c) => c.id === contractId)?.archivedAt
}

export async function refreshEntries() {
  store.entries = await api.listEntries()
}

export async function refreshActiveTimers() {
  store.activeTimers = await api.getActiveTimers()
}

export async function refreshDeletedEntries() {
  store.deletedEntries = await api.listDeletedEntries()
}

export async function refreshAll() {
  store.loading = true
  try {
    await Promise.all([
      refreshClients(),
      refreshContracts(),
      refreshEntries(),
      refreshActiveTimers(),
      refreshUserProfile(),
      refreshImportTemplates(),
    ])
    store.error = ''
  } catch (e) {
    store.error = String(e)
  } finally {
    store.loading = false
  }
}

export function contractLabel(contractId: number): string {
  const contract = store.contracts.find((c) => c.id === contractId)
  if (!contract) return `#${contractId}`
  const client = store.clients.find((c) => c.id === contract.clientId)
  return client ? `${client.name} — ${contract.name}` : contract.name
}

export function clientIdForContract(contractId: number): number | undefined {
  return store.contracts.find((c) => c.id === contractId)?.clientId
}

/** Loads (and caches) a client's active tracking codes. Re-fetches every call so newly
 * added codes show up; callers that just need the cached value can read the store directly. */
export async function loadTrackingCodesForClient(clientId: number): Promise<TrackingCode[]> {
  const codes = await api.listTrackingCodes(clientId)
  store.trackingCodesByClient[clientId] = codes
  return codes
}
