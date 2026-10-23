import {
  api,
  type Client,
  type Contract,
  type EmailTemplate,
  type ImportTemplate,
  type TimeEntry,
  type TrackingCode,
  type UserProfile,
} from './api'

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
  payWeekStart: 'monday',
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
  emailTemplates: [] as EmailTemplate[],
  userProfile: { ...EMPTY_PROFILE } as UserProfile,
  loading: false,
  error: '',
})

/** Set by the currently-open detail/edit page so navigation can warn before discarding
 * unsaved changes. Cleared automatically whenever navigation actually proceeds. */
export const navGuard = $state({ isDirty: false })

export type EntriesFilterPeriod =
  | 'today'
  | 'yesterday'
  | 'thisweek'
  | 'lastweek'
  | 'last2weeks'
  | 'month'
  | '3months'
  | 'all'

/** Which weekday a pay week starts on when no single client is in view. The Entries list
 * spans every client, and clients may each start their week differently, so there has to
 * be one fallback; a contract filter narrows it to that client's own setting. */
export function payWeekStartFor(contractId: number | ''): string {
  const fallback = store.userProfile.payWeekStart
  if (!contractId) return fallback
  const clientId = clientIdForContract(Number(contractId))
  if (clientId === undefined) return fallback
  return store.clients.find((c) => c.id === clientId)?.weekStart ?? fallback
}

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

export async function refreshEmailTemplates() {
  store.emailTemplates = await api.listEmailTemplates()
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

/** A contract with no hour adjustments recorded has a budget of 0, which means "no
 * budget set" rather than "no hours left" — every hours display has to distinguish the
 * two or an unbudgeted contract reads as permanently overdrawn. */
export function hasHoursBudget(contract: Contract): boolean {
  return contract.totalHoursBudgeted > 0
}

export function remainingHours(contract: Contract): number {
  return contract.totalHoursBudgeted - contract.hoursLogged
}

/** A client's hours across all of its active contracts, counting only the ones that
 * actually carry a budget. `budgeted` is null when none of them do. */
export function clientHoursTotals(clientId: number): { budgeted: number | null; logged: number; remaining: number } {
  const budgeted = activeContracts().filter((c) => c.clientId === clientId && hasHoursBudget(c))
  if (budgeted.length === 0) return { budgeted: null, logged: 0, remaining: 0 }
  const total = budgeted.reduce((sum, c) => sum + c.totalHoursBudgeted, 0)
  const logged = budgeted.reduce((sum, c) => sum + c.hoursLogged, 0)
  return { budgeted: total, logged, remaining: total - logged }
}

/** Contracts at or below this many remaining hours get a warning on the Entries page. */
export const LOW_HOURS_THRESHOLD = 10

export async function refreshEntries() {
  store.entries = await api.listEntries()
}

export async function refreshActiveTimers() {
  store.activeTimers = await api.getActiveTimers()
}

export async function refreshDeletedEntries() {
  store.deletedEntries = await api.listDeletedEntries()
}

/** Refreshes everything a time-entry change touches.
 *
 * Changing an entry also moves the logged-hours figure on its contract, but that figure
 * is computed per contract by the backend and cached in `store.contracts` — refreshing
 * the entries alone leaves every hours display stale: the Total Hrs / Remaining columns
 * on Clients and Contracts, and the low-hours warning on Entries. Call this rather than
 * `refreshEntries` on its own after any create, edit, delete, restore, or import. */
export async function refreshAfterEntryChange() {
  await Promise.all([refreshEntries(), refreshContracts()])
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
      refreshEmailTemplates(),
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
