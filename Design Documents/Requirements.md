# TimeTracker — Requirements

## 1. Purpose

TimeTracker is a self-contained desktop application for an independent consultant who
bills time against multiple clients and contracts. It replaces ad-hoc spreadsheets with
a local, single-user tool that tracks time (via a live timer or manual entry), applies
each client's own billing rules, and exports client-ready timesheets and internal
reports without depending on any external service.

## 2. Scope

- Single user, single machine, fully offline. No accounts, no server, no network calls.
- All data lives in one local SQLite database file; all exports are local files
  (.xlsx/.csv) written to a folder the user chooses.
- Windows, macOS, and Linux desktop (Tauri v2 + WebView).

Out of scope: multi-user collaboration, cloud sync, invoicing/payment processing,
mobile clients, remote import sources (e.g. OneNote/Graph).

## 3. Functional Requirements

### 3.1 Clients
- FR-1: Create, view, and edit a Client with: name, notes, Prime, Sub, external ID,
  week-start/week-end weekday, default category, minimum billable increment (minutes).
- FR-2: New clients default to a 15-minute minimum billable increment.
- FR-3: Archive a client (soft delete) and restore it later; archived clients are
  excluded from active pickers but still resolvable by name wherever historical data
  references them (e.g. struck-through in Entries).
- FR-4: The Clients list shows Client, Prime, Sub, Week Start, Min Increment, and Notes.

### 3.2 Contracts
- FR-5: Create, view, and edit a Contract under a Client: name, currency, external ID,
  start date, notes.
- FR-6: A contract's hourly rate has history (`contract_rates`) — updating the rate
  inserts a new effective-dated row rather than mutating the old one, so past entries
  keep the rate that was in effect when they were recorded.
- FR-7: Archive/restore a contract, independent of its client's archived state.
- FR-8: The Contracts list supports sorting and free-text filtering (client/contract
  name), an optional Rate column, and displays Notes.

### 3.3 Categories (internal name: tracking codes)
- FR-9: Categories are scoped to a client and are always optional on a time entry, even
  when the client has categories defined — never a hard requirement.
- FR-10: A category can be set as a client's default, auto-applied when starting a
  timer or quick-timer for that client.
- FR-11: Archive/restore a category.

### 3.4 Time entries
- FR-12: Start/stop one or more concurrent timers across different contracts. Starting
  a new timer while others are running is allowed and only produces an advisory
  warning (work can legitimately span multiple clients at once).
- FR-13: Create/edit manual entries with start time, end time, category, and notes.
  The UI blocks selecting an end time before the start time.
- FR-14: A still-running entry can have its category/notes edited and can be stopped
  from its detail view; start/end times only become editable once it has been stopped.
- FR-15: Soft-delete ("Trash") an entry with restore; deleting never removes the row
  outright.
- FR-16: An entry's billable duration rounds UP to the client's minimum increment
  (e.g. a 22-minute entry bills as 30 minutes for a 15-minute-increment client). A
  client with no minimum set bills the exact elapsed time.
- FR-17: The Entries list defaults to the last week, with Last 2 weeks/Month/3-months/
  All period filters, plus Contract and Category filters that narrow the same list, and
  a totals row (duration, and per-currency amount when amounts are shown). All three
  filters persist across navigation (e.g. into an entry's detail page and back, or to
  any other page and back) rather than resetting, since they're held in shared state
  rather than local to the Entries page component.
- FR-18: Bulk-select entries on the Entries page to bulk-delete or bulk-set category.
- FR-19: Bulk import time entries from CSV or XLSX (including multi-tab workbooks),
  targeted at one contract, with flexible date parsing and automatic creation of any
  category name encountered that doesn't already exist for that client.

### 3.5 Reports & exports
- FR-20: A Reports page shows total hours (and, optionally, amounts) for a selected
  Week or Month, broken down by client then contract, with click-to-drill-down from
  "all clients" → one client → one contract → that contract's raw entries for the
  period.
- FR-21: "Create Timesheet" writes one .xlsx per **client** (covering every one of
  that client's contracts in a single sheet) with entries in the selected period, to
  the user's configured output folder. Columns: Date, Start Time (24h), End Time
  (24h), HH:MM, Category, Notes, with Rate/Amount as an opt-in extra pair of columns,
  formatted as currency. Column widths are pre-sized so no manual resizing is needed.
- FR-22: Scoping behavior on the Reports page: with no drill-down at all ("all
  clients"), one combined workbook is produced per client that has any activity in
  the period — never one file per contract. Drilled into a specific **client**,
  output is scoped to just that client's combined workbook. Drilled into a specific
  **contract**, output is a single workbook for just that contract (named
  `{date}_{client}_{contract}_{yourFullName}.xlsx`, contract name included, no
  Contract column). A client-combined workbook (whether reached via "all clients" or
  by drilling into that one client) is named `{date}_{client}_{yourFullName}.xlsx`
  and gets a Contract column (rows interleaved chronologically across contracts) only
  when more than one of its contracts actually has entries in the period; with just
  one, it looks the same as a directly contract-scoped file.
- FR-23: Each contract's timesheet period is computed from its own client's
  week-start/week-end (a report's aggregate week view always uses Mon–Sun regardless
  of any one client's setting, since that view mixes multiple clients at once).
- FR-24: The `{date}` in a timesheet's filename is either the first or last day of
  that contract's resolved period, controlled by a per-contract "Timesheet filename
  date" setting (First day of period / Last day of period), defaulting to the last
  day.
- FR-25: In the exported timesheet, Start Time is always rounded DOWN to the nearest
  5-minute mark and End Time is always rounded UP to the nearest 5-minute mark (e.g.
  `:07` becomes `:05` as a start time, `:10` as an end time). This only affects the
  displayed Start/End Time columns — the billed HH:MM duration and any Rate/Amount
  columns still reflect the entry's actual billed duration (including the client's
  minimum-increment rounding), not the rounded display times.

### 3.6 Backup, restore, purge
- FR-26: "Backup Data" writes one timestamped CSV per data type (Clients, Contracts,
  Contract Rates, Categories, Time Entries) to the output folder.
- FR-27: "Restore Data" reads those CSVs back in, adding only rows that don't already
  exist by id — never overwriting or duplicating existing rows, so it is safe to run
  more than once.
- FR-28: Trash (soft-deleted entries; archived contracts/clients/categories) can be
  permanently purged. Every purge always writes a CSV backup of exactly what's being
  removed first, always asks for confirmation, and only purges past a "cannot purge —
  other records still reference this" gate: if some other row still depends on a
  candidate, that candidate is skipped (not purged) and reported by name/count, while
  everything else in the batch still proceeds.
- FR-29: "Purge All" backs up every data type, then permanently deletes all clients,
  contracts, categories, and time entries, while preserving the User profile.
- FR-30: The Trash page shows each type (Deleted Time Entries, Archived Contracts,
  Archived Clients, Archived Categories) as a count behind a clickable name; opening a
  type shows its list and is where that type's Purge action lives.

### 3.7 Admin, user profile & app behavior
- FR-31: The top nav has a single "Admin" entry (not "User") leading to a menu of
  four sub-pages: **User**, **Backup & Restore**, **Trash**, and **Appearance**.
  - **User**: first/last/full name (full name defaults to "Last, First"), email,
    output folder (defaulting to the OS Documents folder, with a native folder
    picker), output format preference, and database file size.
  - **Backup & Restore**: Backup Data, Restore Data, and Purge All — split out of
    User onto its own page, since it's a "your data" concern rather than a "who you
    are" one.
  - **Trash**: unchanged from FR-28/FR-30 — reached via Admin instead of its own
    top-nav entry.
  - **Appearance**: window size/position, default start page, and Colors (see
    FR-32/FR-33), grouped together since they're all "how the app looks/opens," not
    "who you are."
- FR-32: Launch position supports 9 named screen positions plus a "Custom" position
  that remembers an exact remembered (x, y), settable via "use current position & size
  as default".
- FR-33: A Colors section offers System/Light/Dark/Custom themes; Custom lets the user
  pick background, text, button-background, and button-text colors independently, live
  previewed as they're changed.
- FR-34: The Admin → User page shows the current on-disk database file size.
- FR-35: A dirty-state Save button is visually distinct (greyed out) whenever nothing
  has changed since it was loaded, on every editable form in the app.
- FR-36: Navigating away from a page with unsaved changes (including via the app's own
  back/nav buttons) prompts "Leave without saving?" before discarding them.

## 4. Non-Functional Requirements

- NFR-1 (Self-contained): No external services, accounts, or network access required
  at runtime. SQLite is bundled (no system dependency).
- NFR-2 (Cross-platform): Must build and run on Windows, macOS, and Linux via Tauri.
- NFR-3 (Data safety): Any operation that permanently deletes data (purge) must first
  write a recoverable backup and require explicit confirmation naming what will be
  lost.
- NFR-4 (Correctness of money): Rate changes must never retroactively alter the
  billing figures on already-recorded time entries (rate snapshotting).
- NFR-5 (Local-only trust model): The app assumes a single trusted local user with
  full OS-level access to their own files; it does not implement multi-user
  authentication/authorization.
- NFR-6 (Testability): Business-logic-bearing modules (rounding, rate snapshotting,
  import parsing, purge dependency chains, backup/restore round-tripping) are covered
  by automated Rust unit tests, not left to manual verification alone.
