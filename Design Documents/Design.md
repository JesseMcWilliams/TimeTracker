# TimeTracker — Design

## 1. Architecture overview

```
┌─────────────────────────────┐        Tauri IPC        ┌──────────────────────────────┐
│   Frontend (Svelte 5 + TS)   │ ───── invoke() calls ──▶ │   Backend (Rust / Tauri v2)   │
│   src/                       │ ◀──── JSON results ───── │   src-tauri/src/               │
└─────────────────────────────┘                          └──────────────────────────────┘
                                                                     │
                                                                     ▼
                                                          ┌──────────────────────┐
                                                          │  SQLite (rusqlite,    │
                                                          │  bundled, single file)│
                                                          └──────────────────────┘
```

- **Frontend**: Svelte 5 (runes) + TypeScript + Vite. A single-page app with a small
  hand-rolled router in `App.svelte` (a `view` state discriminated union + a
  `navigate()` guard). No routing library, no global state library beyond a plain
  `$state` store (`src/lib/store.svelte.ts`).
- **Backend**: Rust, organized as thin `#[tauri::command]` wrappers
  (`src-tauri/src/commands.rs`) over a `domain/` module that holds all business logic
  and SQL. Commands never contain business logic themselves — they lock the shared
  connection, call into `domain::*`, and map errors to `String` for IPC.
- **Storage**: one SQLite file (`timetracker.sqlite`) in the OS app-data directory,
  opened once at startup with `PRAGMA foreign_keys = ON`. Foreign keys are relied on
  deliberately as a real integrity backstop (e.g. purge naturally can't orphan rows).
- **No network**: the app makes no outbound requests. All exports are local files.

## 2. Why these choices

- **Tauri over Electron**: much smaller bundle/memory footprint, and lets business
  logic live in Rust rather than Node, which matters here because money math (rate
  snapshotting, rounding) needs to be verified by fast, deterministic unit tests, not
  re-derived in JS on every screen that touches it.
- **rusqlite direct, not `tauri-plugin-sql`**: keeps SQL and business rules in Rust
  behind typed function signatures, rather than letting the frontend construct SQL.
- **Svelte 5 runes, no framework router**: the app has a small, flat set of top-level
  pages; a `type View = {name:'x'} | {name:'y', someId:number} | ...` union plus one
  `navigate()` function is simpler and more transparent than pulling in a router for
  ~10 screens.
- **CSV via the `csv` crate for backup/restore, `rust_xlsxwriter` for timesheets,
  `calamine` for reading XLSX on import**: CSV is the plain, diffable, spreadsheet-safe
  format for full-fidelity backup/restore; XLSX is what a client-facing "timesheet"
  actually needs to look like (formatted columns, currency formatting, an .xlsx
  extension) and what real-world timesheets to import already arrive as.

## 3. Data model

```sql
clients(id, name, notes, prime, sub, external_id, week_start, week_end,
        default_tracking_code_id, minimum_increment_minutes, archived_at)

contracts(id, client_id, name, currency, external_id, start_date, notes,
          filename_date['start'|'end'], archived_at, created_at)
  -- filename_date picks which end of a resolved timesheet period ("start" or "end")
  -- names the exported file; defaults to "end" (the last day of the period).

contract_rates(id, contract_id, hourly_rate, effective_from, created_at)
  -- append-only history; "current rate" = latest effective_from/id per contract.

tracking_codes(id, client_id, code, description, archived_at)
  -- "categories" in the UI; always optional on a time entry.

time_entries(id, contract_id, started_at, ended_at, duration_secs, rate_snapshot,
             notes, source['manual'|'timer'|'obsidian_import'], external_ref,
             tracking_code_id, deleted_at, created_at, updated_at)
  -- rate_snapshot is fixed at creation time and never re-derived from contract_rates.

tags / time_entry_tags
  -- present in the schema; not currently exposed in the UI beyond the domain layer.

user_profile(id=1, first_name, last_name, full_name, email, output_folder,
             output_type, window_width, window_height, window_x, window_y,
             default_start_page, launch_position, theme, custom_bg, custom_text,
             custom_button_bg, custom_button_text)
  -- single row (id fixed at 1); upserted via ON CONFLICT DO UPDATE.

schema_version(version, name, applied_at)
  -- tracks which of the numbered migrations/*.sql files have been applied.
```

Key invariants enforced by design, not just convention:
- A time entry's `rate_snapshot` is copied from `contract_rates` at creation time.
  `update_contract_rate` only ever **inserts** a new row — it never updates or deletes
  an old one — so historical entries are provably unaffected by later rate changes.
- Billable `duration_secs` is computed once, centrally, in
  `time_entries::compute_duration_secs`, applied uniformly whether the entry came from
  stopping a timer, a manual entry, an edit, or a bulk import — so the rounding rule
  can't drift between code paths.
- Soft-delete (`deleted_at`) and archive (`archived_at`) are reversible; purge is the
  only actually-destructive operation, and it is gated behind an explicit backup.

## 4. Backend module map (`src-tauri/src/`)

| Module | Responsibility |
|---|---|
| `db/mod.rs` | Opens the SQLite connection, applies `migrations/*.sql` in order, tracked via `schema_version`. |
| `domain/contracts.rs` | Client/Contract CRUD, archive/restore, rate history, minimum-increment lookup. |
| `domain/time_entries.rs` | Timer start/stop, manual entry CRUD, running-entry metadata edit, soft delete/restore, duration rounding, filtered listing. |
| `domain/tracking_codes.rs` | Category CRUD, archive/restore, referential validation (optional-but-must-belong-to-client). |
| `domain/tags.rs` | Free-form tag support on entries (schema-level; not yet surfaced in the UI beyond this module). |
| `domain/reports.rs` | Aggregate Week/Month report grouped by client → contract. |
| `domain/timesheets.rs` | Per-contract `.xlsx` generation, honoring each client's own week boundaries, optional scoping to one client/contract. |
| `domain/period.rs` | Pure date-range math (week/month resolution for arbitrary start/end weekdays). |
| `domain/import.rs` | CSV/XLSX bulk import: multi-tab, flexible date/time parsing, auto-creates unknown categories. |
| `domain/backup.rs` | Table-driven CSV backup/restore (additive-only restore, matched by id). |
| `domain/purge.rs` | Per-row dependency-checked permanent deletion, always backup-first; `purge_all_data`. |
| `domain/user_profile.rs` | Single-row profile get/save. |
| `commands.rs` | Thin `#[tauri::command]` layer; owns `AppState` (the shared `Mutex<Connection>` and the resolved `db_path`). |
| `lib.rs` | Tauri builder setup: window sizing/positioning from the profile, command registration. |

## 5. Frontend structure (`src/`)

- `App.svelte` — top-level router (`view` union + `navigate()` dirty-check guard) and
  nav bar.
- `lib/api.ts` — every `invoke()` call, typed 1:1 against the Rust command signatures
  and `#[serde(rename_all = "camelCase")]` structs.
- `lib/store.svelte.ts` — a small reactive `$state` object holding the currently
  loaded clients/contracts/entries/etc., plus `refresh*()` loaders and derived helpers
  (`activeClients()`, `contractLabel()`, ...). `navGuard.isDirty` is the shared flag
  any detail page sets so `App.svelte`'s `navigate()` can prompt before discarding
  changes.
- `lib/theme.ts` — applies the user's chosen color theme as CSS custom properties on
  the document root (System clears overrides and lets `app.css`'s
  `prefers-color-scheme` media query decide; Light/Dark are fixed presets; Custom uses
  the four user-picked colors).
- `lib/dateUtils.ts` — local/UTC conversions, duration/money/byte formatting, the
  shallow-compare `isDirty()` used by every Save-button dirty state.
- One `*Panel.svelte` (list/landing) + one `*Detail.svelte` (edit) per entity
  (Clients, Contracts, Entries), following the same pattern: list page has an inline
  "add" form and navigates into the detail page on row click; detail page tracks
  dirty state locally and warns via `navGuard` before losing edits.
- `TimerPanel.svelte` / `QuickTimerPanel.svelte` — two different timer UIs: full page
  with category/notes input and a running-timers table (rows are clickable into
  `EntryDetail`); a "one click per contract" quick-start/stop grid that shows a stop
  icon in place of start once a contract has a running timer.
- `TrashPanel.svelte` — a two-level drill-down: a summary of counts per type, and a
  per-type list view where that type's Purge action lives.
- `ReportsPanel.svelte` — Week/Month report with click-to-drill (all clients → one
  client → one contract → that contract's entries) and the scoped "Create Timesheet"
  action described in Requirements FR-21/22.
- `UserPanel.svelte` — profile fields, window size/position, Colors, database size,
  Backup/Restore/Purge All.

## 6. Error handling convention

Every domain function returns `DomainResult<T> = Result<T, String>`. Errors are
human-readable strings by design (e.g. "this contract still has N time entries
referencing it") so they can be shown directly in the UI without a second translation
layer — `commands.rs` wrappers just propagate them (or `.map_err(|e| e.to_string())`
at the few `rusqlite::Error` call sites that don't already return `DomainResult`).

## 7. Known simplifications / deliberate non-goals

- No CSP is currently configured (`tauri.conf.json` → `app.security.csp: null`); the
  app has no remote content, `eval`, or `{@html}` usage, which limits the practical
  exposure, but this is a documented hardening gap rather than an evaluated-and-closed
  item (see the Security Assessment document).
- The `tags` / `time_entry_tags` tables and domain functions exist but aren't wired
  into any UI yet.
- Multi-currency reporting sums amounts per-currency (shown as separate totals) rather
  than converting to a single currency, since the app has no exchange-rate source.
