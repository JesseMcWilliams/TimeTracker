# TimeTracker — Testing

## 1. Strategy

- **Backend business logic** (anything touching money, dates, or destructive
  operations) is covered by Rust unit tests in `#[cfg(test)] mod tests` blocks next to
  the code they test, run via `cargo test --lib`. Each test opens a throwaway SQLite
  file in the OS temp directory (named with the test name + process id to avoid
  collisions) and exercises the real migration path, not a mock.
- **Frontend type safety** is enforced with `svelte-check --tsconfig ./tsconfig.app.json`
  (0 errors required before any change is considered done), which catches prop/type
  mismatches between components and against `api.ts`'s typed `invoke()` wrappers, but
  it is a type checker, not a behavior test — it does not verify that a button click
  actually produces the right result.
- **Manual verification**: every UI-affecting change is exercised in a live
  `npx tauri dev` session (confirmed to build and launch cleanly) and, where a change
  touches the database schema, cross-checked directly against the live SQLite file
  (`schema_version` rows, new columns/values) rather than assumed from the migration
  file alone.
- **There is currently no automated frontend test suite** (no Vitest/Playwright).
  UI behavior (dialogs, drill-downs, dirty-state warnings, live theme preview, etc.)
  is verified manually against the checklist in §3. This is a known gap — see §4.

## 2. Automated backend test inventory (`cargo test --lib`, currently 14 passing)

| Test | Module | What it guards against |
|---|---|---|
| `rounds_duration_up_to_clients_minimum_increment` | `time_entries` | A 22-minute entry for a 15-minute-increment client must bill as 30 minutes; an exact multiple must not round further. |
| `no_rounding_when_client_has_no_minimum_set` | `time_entries` | A client with no minimum increment bills the exact elapsed time (regression test — new clients now default to a 15-minute minimum, so this test explicitly clears it first to still exercise the no-rounding path). |
| `imports_csv_and_xlsx_including_overnight_rollover` | `import` | CSV and XLSX rows both import correctly, including a shift that crosses midnight. |
| `imports_multi_tab_xlsx_with_activity_column_and_us_dates` | `import` | A workbook with multiple tabs and US-style `M/D/YYYY` dates imports every tab's rows. |
| `auto_creates_new_categories_and_allows_blank_category` | `import` | An unrecognized category name on import is auto-created for that client; a blank category imports fine (categories are never required). |
| `parses_dates_and_times_with_spurious_time_or_date_suffix` | `import` | Regression test for a real reported bug: an Excel date cell round-tripped as `"2026-08-03 00:00:00"` must still parse, and a time cell with a spurious date prefix must still parse. |
| `backup_then_restore_round_trips_all_data_into_a_fresh_database` | `backup` | A full backup, restored into an empty database, reproduces every client/contract/rate/category/entry. |
| `restore_does_not_duplicate_or_overwrite_existing_rows` | `backup` | Restoring into a database that already has some of the same rows (by id) only adds the missing ones — never duplicates or clobbers. |
| `purges_deleted_entries_after_writing_backup` | `purge` | A purge always writes a recoverable CSV backup before deleting, and the backup's contents match what was deleted. |
| `cutoff_only_purges_matching_rows` | `purge` | A cutoff date restricts purge to rows deleted/archived on or after it, leaving earlier ones in Trash. |
| `purges_archived_contract_with_no_referencing_entries` | `purge` | A contract with no time entries or rates left can be purged cleanly (regression for the "contract_rates blocks even zero-entry contracts" bug). |
| `purges_archived_client_only_after_its_contracts_and_categories_are_gone` | `purge` | A client can't be purged while its contracts or categories still exist; purging in the correct order (contract → category → client) succeeds. |
| `reports_blocked_contract_instead_of_failing_outright` | `purge` | When some candidates are blocked and others aren't, the unblocked ones are still purged and the blocked ones are named in the result, rather than the whole batch failing. |
| `purge_all_data_clears_everything_but_keeps_user_profile` | `purge` | Purge All empties every client/contract/category/entry/rate table but leaves the user profile row intact. |

Run with:
```
cd src-tauri
cargo test --lib
```

## 3. Manual verification checklist

Use this after any change that touches the areas below, since they aren't covered by
the automated suite:

- **Timers**: start a timer, confirm it appears in both Timer and Quick Timer pages;
  starting a second timer for a different contract shows the "already running"
  warning but still proceeds; Quick Timer's button swaps to a stop icon once a
  contract has a running timer, and clicking it stops that timer.
- **Running-timer editing**: from the Timer page, click a running timer's row → its
  category/notes can be edited and saved, and it can be stopped from there; once
  stopped, start/end times become editable.
- **Dirty-state guards**: edit a field on any Detail page, then navigate away via the
  nav bar — a "Leave without saving?" prompt appears; Save greys out again once
  nothing differs from what was loaded.
- **Rounding**: create an entry shorter than a client's minimum increment and confirm
  the billed duration rounds up correctly in both the Entries list and an exported
  timesheet.
- **Import**: import a CSV and a multi-tab XLSX against a real contract; confirm row
  counts, that unrecognized categories were created, and that dates near a client's
  timezone boundary land on the correct local calendar day (see the Rate/Amount
  Excel-export date bug below for why this specifically needs checking after any
  date-handling change).
- **Reports drill-down**: from Reports, click a client, then a contract, then back out
  via the breadcrumbs; confirm the "Create Timesheet" button and its label scope
  themselves to whatever is currently drilled into.
- **Trash**: confirm each type shows a count, drilling in shows the list, and Purge
  only appears in the drilled-in view; purge a type and confirm a backup CSV was
  written and named per the `{yyyy-mm-dd}_{Type}_{Archive|Deleted}_Backup_Purge.csv`
  convention.
- **Backup/Restore/Purge All**: back up, then restore into the same database (should
  report 0 new rows since everything already exists by id); confirm Purge All keeps
  the User profile.
- **Colors**: switch between System/Light/Dark/Custom and confirm the live preview
  updates immediately, and that it's still applied correctly after an app relaunch.
- **Window geometry**: set a custom window size/position via "use current position &
  size as default", relaunch, and confirm the window opens at that size/position.
- **Confirmation dialogs**: exercise at least one destructive action (archive, delete,
  purge) and one "leave without saving" prompt in a real built/dev app window — these
  depend on the Tauri dialog plugin's `confirm` permission, which is a config detail
  that can silently regress (see the Security Assessment document for the specific
  bug this checklist item exists to catch).

## 4. Known gaps

- No automated frontend tests (component or end-to-end). All Svelte behavior is
  verified manually per §3.
- No automated visual/screenshot regression testing; theme/color changes are verified
  by manual inspection only.
- No load/performance testing — the app is designed for one user's own data volume,
  not benchmarked at scale.
