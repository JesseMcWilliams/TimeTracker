import { invoke } from '@tauri-apps/api/core'

export type Weekday = 'monday' | 'tuesday' | 'wednesday' | 'thursday' | 'friday' | 'saturday' | 'sunday'

export const WEEKDAYS: Weekday[] = [
  'monday',
  'tuesday',
  'wednesday',
  'thursday',
  'friday',
  'saturday',
  'sunday',
]

export interface Client {
  id: number
  name: string
  notes: string | null
  prime: string | null
  sub: string | null
  externalId: string | null
  weekStart: Weekday
  weekEnd: Weekday
  defaultTrackingCodeId: number | null
  minimumIncrementMinutes: number | null
  archivedAt: string | null
}

export type FilenameDate = 'start' | 'end'

export interface Contract {
  id: number
  clientId: number
  name: string
  currency: string
  externalId: string | null
  startDate: string | null
  notes: string | null
  filenameDate: FilenameDate
  archivedAt: string | null
  currentRate: number | null
}

export type StartPage =
  | 'timer'
  | 'quick-timer'
  | 'clients'
  | 'contracts'
  | 'entries'
  | 'trash'
  | 'reports'
  | 'import'
  | 'user'
  | 'admin'
  | 'backup'
  | 'appearance'

export const START_PAGES: StartPage[] = [
  'timer',
  'quick-timer',
  'clients',
  'contracts',
  'entries',
  'trash',
  'reports',
  'import',
  'user',
  'admin',
  'backup',
  'appearance',
]

export type LaunchPosition =
  | 'default'
  | 'top-left'
  | 'top-center'
  | 'top-right'
  | 'middle-left'
  | 'center'
  | 'middle-right'
  | 'bottom-left'
  | 'bottom-center'
  | 'bottom-right'
  | 'custom'

export const LAUNCH_POSITIONS: LaunchPosition[] = [
  'default',
  'top-left',
  'top-center',
  'top-right',
  'middle-left',
  'center',
  'middle-right',
  'bottom-left',
  'bottom-center',
  'bottom-right',
  'custom',
]

export type Theme = 'system' | 'light' | 'dark' | 'custom'

export const THEMES: Theme[] = ['system', 'light', 'dark', 'custom']

export interface UserProfile {
  firstName: string | null
  lastName: string | null
  fullName: string | null
  email: string | null
  outputFolder: string | null
  outputType: 'csv' | 'xlsx'
  windowWidth: number | null
  windowHeight: number | null
  windowX: number | null
  windowY: number | null
  defaultStartPage: StartPage
  launchPosition: LaunchPosition
  theme: Theme
  customBg: string | null
  customText: string | null
  customButtonBg: string | null
  customButtonText: string | null
}

export interface WindowGeometry {
  x: number
  y: number
  width: number
  height: number
}

export interface ContractBreakdown {
  contractId: number
  contractName: string
  totalSecs: number
  totalAmount: number
}

export interface ClientBreakdown {
  clientId: number
  clientName: string
  totalSecs: number
  totalAmount: number
  contracts: ContractBreakdown[]
}

export interface Report {
  from: string
  to: string
  totalSecs: number
  totalAmount: number
  clients: ClientBreakdown[]
}

export type ReportPeriod = 'week' | 'month' | 'year'

export interface TimesheetFile {
  path: string
  clientName: string
  contractName: string
  entryCount: number
}

export type EntrySource = 'manual' | 'timer' | 'obsidian_import' | 'import'

export interface TimeEntry {
  id: number
  contractId: number
  startedAt: string
  endedAt: string | null
  durationSecs: number | null
  rateSnapshot: number
  notes: string | null
  source: EntrySource
  externalRef: string | null
  trackingCodeId: number | null
  trackingCode: string | null
  deletedAt: string | null
}

export interface TrackingCode {
  id: number
  clientId: number
  code: string
  description: string | null
  archivedAt: string | null
}

export interface EntryFilter {
  contractId?: number
  from?: string
  to?: string
}

export interface Tag {
  id: number
  name: string
}

export interface ImportResult {
  imported: number
  skipped: number
  errors: string[]
  warnings: string[]
}

export interface ImportTemplate {
  id: number
  name: string
  notes: string | null
  isDefault: boolean
  dateColumns: string
  startColumns: string
  endColumns: string
  categoryColumns: string | null
  notesColumns: string | null
}

export interface ImportPreviewSample {
  date: string
  start: string
  end: string
  category: string
  notes: string
}

export interface ImportPreviewSheet {
  label: string
  error: string | null
  warnings: string[]
  matchedDateColumn: string | null
  matchedStartColumn: string | null
  matchedEndColumn: string | null
  matchedCategoryColumn: string | null
  matchedNotesColumn: string | null
  sampleRows: ImportPreviewSample[]
}

export interface BackupFile {
  dataType: string
  path: string
  count: number
}

export interface RestoreTypeCount {
  dataType: string
  inserted: number
}

export interface RestoreResult {
  restored: RestoreTypeCount[]
  errors: string[]
}

export interface PurgeResult {
  count: number
  backupPath: string
  blocked: string[]
}

export interface PurgeAllResult {
  backups: BackupFile[]
}

export const api = {
  createClient: (name: string, notes?: string) =>
    invoke<number>('create_client', { name, notes: notes ?? null }),
  updateClient: (
    clientId: number,
    name: string,
    weekStart: Weekday,
    weekEnd: Weekday,
    notes?: string,
    prime?: string,
    sub?: string,
    externalId?: string,
    defaultTrackingCodeId?: number,
    minimumIncrementMinutes?: number,
  ) =>
    invoke<void>('update_client', {
      clientId,
      name,
      notes: notes ?? null,
      prime: prime ?? null,
      sub: sub ?? null,
      externalId: externalId ?? null,
      weekStart,
      weekEnd,
      defaultTrackingCodeId: defaultTrackingCodeId ?? null,
      minimumIncrementMinutes: minimumIncrementMinutes ?? null,
    }),
  listClients: (includeArchived = false) =>
    invoke<Client[]>('list_clients', { includeArchived }),
  archiveClient: (clientId: number) => invoke<void>('archive_client', { clientId }),
  restoreClient: (clientId: number) => invoke<void>('restore_client', { clientId }),

  createContract: (clientId: number, name: string, currency: string, initialHourlyRate: number) =>
    invoke<number>('create_contract', { clientId, name, currency, initialHourlyRate }),
  listContracts: (includeArchived = false) =>
    invoke<Contract[]>('list_contracts', { includeArchived }),
  updateContractRate: (contractId: number, newHourlyRate: number) =>
    invoke<void>('update_contract_rate', { contractId, newHourlyRate }),
  updateContractDetails: (
    contractId: number,
    name: string,
    currency: string,
    filenameDate: FilenameDate,
    externalId?: string,
    startDate?: string,
    notes?: string,
  ) =>
    invoke<void>('update_contract_details', {
      contractId,
      name,
      currency,
      externalId: externalId ?? null,
      startDate: startDate ?? null,
      notes: notes ?? null,
      filenameDate,
    }),
  archiveContract: (contractId: number) =>
    invoke<void>('archive_contract', { contractId }),
  restoreContract: (contractId: number) =>
    invoke<void>('restore_contract', { contractId }),

  startTimer: (contractId: number, notes?: string, trackingCodeId?: number) =>
    invoke<number>('start_timer', { contractId, notes: notes ?? null, trackingCodeId: trackingCodeId ?? null }),
  stopTimer: (entryId: number) => invoke<void>('stop_timer', { entryId }),
  getActiveTimers: () => invoke<TimeEntry[]>('get_active_timers'),

  createManualEntry: (
    contractId: number,
    startedAt: string,
    endedAt: string,
    notes?: string,
    trackingCodeId?: number,
  ) =>
    invoke<number>('create_manual_entry', {
      contractId,
      startedAt,
      endedAt,
      notes: notes ?? null,
      trackingCodeId: trackingCodeId ?? null,
    }),
  updateEntry: (
    entryId: number,
    startedAt: string,
    endedAt: string,
    notes?: string,
    trackingCodeId?: number,
  ) =>
    invoke<void>('update_entry', {
      entryId,
      startedAt,
      endedAt,
      notes: notes ?? null,
      trackingCodeId: trackingCodeId ?? null,
    }),
  updateEntryMetadata: (entryId: number, notes?: string, trackingCodeId?: number) =>
    invoke<void>('update_entry_metadata', {
      entryId,
      notes: notes ?? null,
      trackingCodeId: trackingCodeId ?? null,
    }),
  deleteEntry: (entryId: number) => invoke<void>('delete_entry', { entryId }),
  restoreEntry: (entryId: number) => invoke<void>('restore_entry', { entryId }),
  listDeletedEntries: () => invoke<TimeEntry[]>('list_deleted_entries'),
  listEntries: (filter: EntryFilter = {}) => invoke<TimeEntry[]>('list_entries', { filter }),

  listTags: () => invoke<Tag[]>('list_tags'),
  addTagToEntry: (timeEntryId: number, tagName: string) =>
    invoke<void>('add_tag_to_entry', { timeEntryId, tagName }),
  removeTagFromEntry: (timeEntryId: number, tagName: string) =>
    invoke<void>('remove_tag_from_entry', { timeEntryId, tagName }),

  createTrackingCode: (clientId: number, code: string, description?: string) =>
    invoke<number>('create_tracking_code', { clientId, code, description: description ?? null }),
  listTrackingCodes: (clientId: number, includeArchived = false) =>
    invoke<TrackingCode[]>('list_tracking_codes', { clientId, includeArchived }),
  archiveTrackingCode: (trackingCodeId: number) =>
    invoke<void>('archive_tracking_code', { trackingCodeId }),
  restoreTrackingCode: (trackingCodeId: number) =>
    invoke<void>('restore_tracking_code', { trackingCodeId }),
  listArchivedTrackingCodes: () => invoke<TrackingCode[]>('list_archived_tracking_codes'),

  getUserProfile: () => invoke<UserProfile | null>('get_user_profile'),
  saveUserProfile: (profile: UserProfile) => invoke<void>('save_user_profile', { profile }),
  getWindowGeometry: () => invoke<WindowGeometry>('get_window_geometry'),
  getDatabaseSize: () => invoke<number>('get_database_size'),

  generateReport: (period: ReportPeriod, referenceDate: string) =>
    invoke<Report>('generate_report', { period, referenceDate }),
  generateTimesheets: (
    period: ReportPeriod,
    referenceDate: string,
    includeRateAmount: boolean,
    clientId?: number,
    contractId?: number,
  ) =>
    invoke<TimesheetFile[]>('generate_timesheets', {
      period,
      referenceDate,
      includeRateAmount,
      clientId: clientId ?? null,
      contractId: contractId ?? null,
    }),
  getDefaultOutputFolder: () => invoke<string>('get_default_output_folder'),

  importTimeEntries: (contractId: number, filePaths: string[], templateId: number) =>
    invoke<ImportResult>('import_time_entries', { contractId, filePaths, templateId }),
  previewImport: (filePaths: string[], templateId: number) =>
    invoke<ImportPreviewSheet[]>('preview_import', { filePaths, templateId }),

  listImportTemplates: () => invoke<ImportTemplate[]>('list_import_templates'),
  createImportTemplate: (
    name: string,
    isDefault: boolean,
    dateColumns: string,
    startColumns: string,
    endColumns: string,
    notes?: string,
    categoryColumns?: string,
    notesColumns?: string,
  ) =>
    invoke<number>('create_import_template', {
      name,
      notes: notes ?? null,
      isDefault,
      dateColumns,
      startColumns,
      endColumns,
      categoryColumns: categoryColumns ?? null,
      notesColumns: notesColumns ?? null,
    }),
  updateImportTemplate: (
    id: number,
    name: string,
    isDefault: boolean,
    dateColumns: string,
    startColumns: string,
    endColumns: string,
    notes?: string,
    categoryColumns?: string,
    notesColumns?: string,
  ) =>
    invoke<void>('update_import_template', {
      id,
      name,
      notes: notes ?? null,
      isDefault,
      dateColumns,
      startColumns,
      endColumns,
      categoryColumns: categoryColumns ?? null,
      notesColumns: notesColumns ?? null,
    }),
  deleteImportTemplate: (id: number) => invoke<void>('delete_import_template', { id }),

  backupAllData: () => invoke<BackupFile[]>('backup_all_data'),
  restoreFromBackups: (filePaths: string[]) => invoke<RestoreResult>('restore_from_backups', { filePaths }),

  purgeDeletedEntries: (cutoff?: string) => invoke<PurgeResult>('purge_deleted_entries', { cutoff: cutoff ?? null }),
  purgeArchivedContracts: (cutoff?: string) =>
    invoke<PurgeResult>('purge_archived_contracts', { cutoff: cutoff ?? null }),
  purgeArchivedClients: (cutoff?: string) => invoke<PurgeResult>('purge_archived_clients', { cutoff: cutoff ?? null }),
  purgeArchivedCategories: (cutoff?: string) =>
    invoke<PurgeResult>('purge_archived_categories', { cutoff: cutoff ?? null }),
  purgeAllData: () => invoke<PurgeAllResult>('purge_all_data'),
}
