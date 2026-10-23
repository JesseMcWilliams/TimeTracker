# Handoff — 2026-10-23

Snapshot of where TimeTracker stands after this working session, for whoever
(including future-you) picks this up next.

## Current state

- **`main`** is at commit `b4d813b`, clean working tree.
- **No open PRs.** Everything from the October request has landed.
- **`v0.6.1` is released and is the latest** — two releases came out of this
  session: `v0.6.0` with the October batch, then `v0.6.1` the same day fixing a
  bug found by using it (see below). `main` has nothing unreleased on it.
- The app was launched and exercised by hand against the real dev database,
  which is also what first applied migrations 0017–0019 and first exercised the
  pre-upgrade backup outside a test.

## What shipped

| # | What | PR | Release |
|---|---|---|---|
| 1 | Today filter leaked future-dated entries; added Yesterday option | #3 | v0.5.0 |
| 2 | Reports page lists contracts with no time logged in the period | #4 | v0.5.0 |
| 3 | EXE icon blank on taskbar | **Not a code bug** — see below | — |
| 4 | Contracts: lifetime hours budget + remaining-hours tracking | #5 | v0.5.0 |
| 5 | "No time logged" email template generator on Reports | #6 | v0.5.0 |
| 6 | 2 high-severity `npm audit` findings | #7 | v0.5.0 |
| 7 | CycloneDX SBOMs for npm + Rust deps (`sbom/`) | #8 | v0.5.0 |
| 8 | DB backed up before an upgrade applies new migrations | #9 | v0.6.0 |
| 9 | Entries add-row time pickers snap to 5-minute increments | #10 | v0.6.0 |
| 10 | `rust_xlsxwriter` + `calamine` bumps, clearing a yanked and three vulnerable `zip` versions | #11 | v0.6.0 |
| 11 | Email template defaults, placeholder subject/body, date formats | #12 | v0.6.0 |
| 12 | Hours totals on Clients and Contracts; low-hours warning on Entries | #12 | v0.6.0 |
| 13 | Entries week filters follow whole pay weeks; "This week" added | #12 | v0.6.0 |
| 14 | Admin → Templates page (email + import templates) | #12 | v0.6.0 |
| 15 | Archive button separated from Apply adjustment | #12 | v0.6.0 |
| 16 | vitest added; pay-week maths covered; vite bumped off a dev-server CVE | #12 | v0.6.0 |
| 17 | Entries store the day they belong to (`local_date`) | #13 | v0.6.0 |
| 18 | Remaining hours did not update after logging time | #14 | v0.6.1 |

## Things worth knowing before you continue

**A whole class of bug lives in the store, not the backend — and there is now a
test for it.** The v0.6.1 fix: a contract's logged hours are computed by the
backend per contract and cached on the contract objects in `store.contracts`.
Every UI path that changed a time entry refreshed only the *entries*, so every
hours display kept the figure loaded at app start. Nothing was individually
wrong — it compiled, types checked, and both the Rust and frontend suites stayed
green while the wiring was broken, which is exactly why it shipped.

The rule now lives in one function, `refreshAfterEntryChange`, and
`tests/entry-refresh.guard.test.ts` reads the component sources and asserts every
file calling an entry-mutating api routes through it. **If you add a new
entry-mutating call to `api.ts`, add it to that test's `ENTRY_MUTATIONS` list** —
the test checks its own list against `api.ts` and fails if one goes missing, so
it tells you rather than silently stopping covering it.

That guard typechecks under `tsconfig.node.json`, not `tsconfig.app.json`,
because it reads files and the app config deliberately withholds Node types from
browser code. Anything else that inspects source at test time belongs in
`tests/` for the same reason.

**The date handling was rebuilt, twice, and the second supersedes the first.**
PR #12 made period bounds respect the viewer's local midnight, which fixed the
reported symptom. PR #13 went further: entries now store `local_date` (the
calendar day they belong to) and `utc_offset_minutes`, decided once at write time
and never recomputed. Period queries are a plain string range over an indexed
column — no timezone arithmetic at query time, and no offset parameter threaded
through callers. If you find `to_rfc3339_bounds` in an old diff, it no longer
exists.

**The 0019 backfill was a one-time reconstruction for existing rows.** It derived
`local_date` from the machine's timezone, applying the rules in effect on each
entry's own date. For any entry logged while in a *different* timezone, that is a
guess — the offset that actually applied was never recorded. Every row written
since records its own, so this never has to be guessed again.

**Report totals near period edges may have shifted** for the same reason. That is
the bug being corrected, not a regression, but a report run on v0.6.x will not
always match the same report run on v0.5.0.

**Icon item (#3) needs a user action, not a code fix.** The blank taskbar icon
traced to a *pinned* taskbar shortcut pointing at a stale
`C:\Windows\Installer\{old-GUID}\ProductIcon` left over from an earlier MSI
version — not a bug in the app's icon files or build config. Unpin and re-pin
TimeTracker to fix it. This recurs on every version upgrade for anyone who pins
via the MSI-installed shortcut, since each version gets a new MSI ProductCode by
default.

**Migration-numbering hazard, already hit once.** `src-tauri/src/db/mod.rs`
applies migrations by *array position/count*, not by filename. PR #5 and #6 both
added a migration at the same position; whichever merged second needed renumbering
via a rebase. If you ever develop two migration-adding branches in parallel,
expect this and rebase the second before merging. The October batch deliberately
kept 0017–0019 sequential on single branches to avoid it. The same root cause can
bite your **local dev AppData database** if you test several such branches against
it in sequence — you may see a `table X already exists` panic on launch. Fix:
delete `%APPDATA%\dev.jbannerman.timetracker.dev\timetracker.sqlite` (safe,
disposable dev data) and relaunch.

**Backup files identify themselves by their header row.** `backup.rs` matches a
restore file purely by its CSV header, so *adding a column to a backed-up table
silently breaks every previously written backup file* — it stops matching any
known type, and rows restored without the new column can end up invisible to
queries. PR #13 hit this and handled it with a `legacy_headers` list plus
name-based (not position-based) value mapping. Do the same if you add another.

**`src-tauri/target/release/bundle/` accumulates every version you have ever
built.** When staging a release, select the new version's files by name and check
the timestamp — copying whatever is in the folder will happily publish the
previous release's binaries under the new version's name.

**Tests, and what each is for.** `cargo test` (62) covers the domain and
migrations. `npm test` (36) covers the pay-week date maths, the store refresh
helper, and the source-level guard above. `npm run check` covers 134 files. Run
all three before cutting a release; `npm audit` and `cd src-tauri && cargo audit`
are the dependency equivalents, both clean as of this session.

**The dev database currently holds seeded sample data** — two clients with
different pay-week start days, four contracts covering healthy / low / over /
no-budget hour states, and an entry whose local day and UTC day differ. A
pre-seed copy sits beside it as `timetracker.sqlite.pre-seed.bak`. Delete either
freely; it is disposable.

## Where things are documented

- `Design Documents/ReleaseProcess.md` — branching convention, versioning, and
  the exact steps to cut a release (bump → tag → build → package → publish).
- `Design Documents/TimeTracker_New_Features_Request.md` — the running backlog.
  Empty as of this session; everything in it shipped.
- `sbom/README.md` — how to regenerate the CycloneDX SBOMs when dependencies
  change meaningfully. Both were regenerated this session.
- This file — a point-in-time snapshot, not a living doc. Overwrite it at the end
  of a session rather than appending to it.
