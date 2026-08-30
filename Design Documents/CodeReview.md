# TimeTracker — Code Review (2026-08-29)

Scope: full backend (`src-tauri/src/**/*.rs`) and frontend (`src/**/*.svelte`, `*.ts`).
Method: systematic grep sweep for known problem patterns (unbounded timers/listeners,
panics outside tests, string-built SQL, unsafe/shell/eval/`{@html}` usage, dangling
`.unwrap()`), followed by manual read-through of every flagged call site to confirm
whether it's a real defect or a false positive.

## Findings

### 1. Timer-page 1-second interval was never cleaned up (FIXED)

**File**: `src/lib/TimerPanel.svelte`

**Issue**: the elapsed-time ticker was started with a bare top-level `setInterval(...)`
call in the component's `<script>` body, with no matching `clearInterval`. Since the
app's router (`App.svelte`) mounts/destroys `TimerPanel` on every navigation to/from the
Timer page (`{#if view.name === 'timer'} <TimerPanel /> {/if}`), each visit to the
Timer page started a brand new interval that outlived the component — navigating away
and back N times left N intervals running concurrently forever, each one still ticking
and writing to a `$state` variable belonging to an already-destroyed component
instance. This is a genuine resource/memory leak (unbounded interval accumulation) and
a wasted-CPU issue, not just a cosmetic one.

**Fix**: moved the interval into a Svelte 5 `$effect` with a cleanup return, which
runs once on mount and is guaranteed to `clearInterval` on unmount:

```svelte
$effect(() => {
  const id = setInterval(() => { now = Date.now() }, 1000)
  return () => clearInterval(id)
})
```

**Verified**: `svelte-check` still reports 0 errors; the Timer page's elapsed-time
display still updates every second in a live `tauri dev` session.

### 2. `window.confirm()` silently broken by an incomplete dialog-plugin permission (FIXED)

**Files**: `src-tauri/capabilities/default.json`; all 8 call sites of `confirm(...)`
across `App.svelte`, `ClientDetail.svelte`, `ContractDetail.svelte`,
`EntryDetail.svelte`, `EntriesPanel.svelte` (×2), `UserPanel.svelte`, `TrashPanel.svelte`.

**Issue**: earlier dev-session logs captured a real, reproduced runtime error:
`[Unhandled rejection] Unknown Error: dialog.confirm not allowed. Command not found`.
Tauri v2 routes the webview's native `window.confirm()` through the dialog plugin's
`confirm` IPC command, which requires the `dialog:allow-confirm` permission. The
app's capability manifest only granted `dialog:default`, which (confirmed directly
against the plugin's own ACL manifest at
`src-tauri/gen/schemas/acl-manifests.json`) bundles `allow-message`, `allow-save`, and
`allow-open` — but **not** `allow-confirm` or `allow-ask`. Every `confirm(...)` call in
the app — every archive/delete/purge confirmation and the "leave without saving?"
navigation guard — depends on exactly this permission.

**Fix**: added `"dialog:allow-confirm"` to `src-tauri/capabilities/default.json`'s
permission list.

**Verified**: `cargo check` accepts the updated capability manifest (capabilities are
schema-validated at build time). This is a config-level fix I could confirm was
*necessary* (the exact missing permission matches the exact error text and the
plugin's own documented default set) but I could not personally click through the UI
to watch a dialog appear — **please exercise at least one delete/archive/purge action
and the "leave without saving" prompt in a real session to confirm the dialog now
shows**, per the checklist item added in `Testing.md` §3.

### 3. Reviewed, no defect found: `.unwrap()`/`.expect()` usage

Grepped for all `.unwrap()`/`.expect()`/`panic!`/`todo!` in `src-tauri/src`. Every
occurrence but three is inside a `#[cfg(test)] mod tests` block, where a panic-on-setup-
failure is the correct, intended behavior (it fails the test loudly rather than
silently passing on bad fixture data). The three outside tests:

- `lib.rs`: `.run(...).expect("error while running tauri application")` — standard
  Tauri bootstrap; a failure here means the app cannot start at all, so terminating
  with a message is the only reasonable behavior.
- `period.rs` (×2): `NaiveDate::from_ymd_opt(reference.year(), reference.month(), 1).unwrap()`
  and the same for `reference.year() + 1, 1, 1` / `reference.month() + 1`. These
  cannot panic in practice: `reference` is already a valid, successfully-parsed
  `NaiveDate`, so its year is a valid `i32` (chrono's range comfortably covers any
  realistic reference date) and its month is already known to be in `1..=12`; day `1`
  is always valid for every month. No code change made — flagging here so a future
  reader doesn't need to re-derive the same reasoning from scratch.

### 4. Reviewed, no defect found: string-built SQL

Grepped for `format!(...)` used to build SQL text (as opposed to parameter values).
Found in `time_entries.rs` (`list_entries`, `list_deleted_entries`), `contracts.rs`
(`list_clients`), and `purge.rs` (`delete_by_ids`, and a test-only row-count helper).
In every case, the interpolated pieces are either:
- a compile-time string literal chosen by the calling code (e.g. `table` in
  `delete_by_ids` is always one of the 4 literal table names at its 4 call sites,
  never derived from any external input), or
- a static, hard-coded SQL fragment selected by an `if`/`else` on a `bool` (e.g.
  `include_archived`), never by concatenating a value into the query text.

All actual **values** (ids, dates, names, notes, etc.) go through `rusqlite::params![]`
or a `Vec<Box<dyn ToSql>>` bound as real bind parameters — including in
`delete_by_ids`, where the `id IN (...)` list is built purely from `i64::to_string()`
on values that are never derived from raw user-typed text. No SQL injection vector
found; no change made.

### 5. Reviewed, no defect found: no unsafe code, shell execution, or HTML injection

Grepped the whole tree for `unsafe `, `std::process::Command`/`Command::new`, `eval(`,
`innerHTML`, and Svelte's `{@html ...}`. Zero matches outside of two unrelated,
harmless uses of `std::process::id()` (used only to namespace temp-file names in test
fixtures, not `std::process::Command`). There is no code path that executes an
external process or renders unsanitized HTML/JS from any data source, so there is no
XSS or command-injection surface to begin with.

### 6. Reviewed, no defect found: Mutex-poisoning resilience

`AppState.conn` is a `Mutex<Connection>` locked once per command via
`state.conn.lock().map_err(|e| e.to_string())?`. A panic while holding this lock would
poison the mutex and fail every subsequent command until restart. Since no command or
domain function panics on any reachable path (confirmed by finding #3 — the only
non-test `.unwrap()`s are provably infallible, and every other error path returns
`Result` instead of panicking), this risk has no known trigger today. No change made;
noting it here as an invariant worth preserving — any future change that introduces a
panicking call inside a command handler should be treated as a regression against this
review.

## Summary

| # | Finding | Severity | Outcome |
|---|---|---|---|
| 1 | Uncleaned `setInterval` on the Timer page | Medium (resource leak, grows unbounded with navigation) | Fixed |
| 2 | Missing `dialog:allow-confirm` permission breaks every `confirm()` dialog | High (silently defeats every destructive-action confirmation and the unsaved-changes guard) | Fixed |
| 3 | Non-test `.unwrap()`/`.expect()` usage | None (all reviewed as provably safe or intentional) | No change |
| 4 | String-built SQL fragments | None (values always parameterized; interpolated text always compile-time constant) | No change |
| 5 | Unsafe code / shell exec / HTML injection surface | None (no matches found) | No change |
| 6 | Mutex-poisoning resilience | None (no panic path exists today) | No change, documented as an invariant |
