# Handoff — 2026-10-23

Snapshot of where TimeTracker stands after this working session, for whoever
(including future-you) picks this up next.

## Current state

- **`main`** is at commit `32f6276`, clean working tree.
- **No open PRs.** Everything from the October request has landed.
- **Version is still `v0.5.0`** (released 2026-10-16), with **18 commits on
  `main` that are not in any release**. Cut `v0.6.0` when you want these in
  users' hands — new functionality, so minor per `ReleaseProcess.md`.

## What shipped

| # | What | PR |
|---|---|---|
| 1 | Today filter leaked future-dated entries; added Yesterday option | #3 |
| 2 | Reports page lists contracts with no time logged in the period | #4 |
| 3 | EXE icon blank on taskbar | **Not a code bug** — see below |
| 4 | Contracts: lifetime hours budget + remaining-hours tracking | #5 |
| 5 | "No time logged" email template generator on Reports | #6 |
| 6 | 2 high-severity `npm audit` findings | #7 |
| 7 | CycloneDX SBOMs for npm + Rust deps (`sbom/`) | #8 |
| 8 | DB backed up before an upgrade applies new migrations | #9 |
| 9 | Entries add-row time pickers snap to 5-minute increments | #10 |
| 10 | `rust_xlsxwriter` + `calamine` bumps, clearing a yanked and three vulnerable `zip` versions | #11 |
| 11 | Email template defaults, placeholder subject/body, date formats | #12 |
| 12 | Hours totals on Clients and Contracts; low-hours warning on Entries | #12 |
| 13 | Entries week filters follow whole pay weeks; "This week" added | #12 |
| 14 | Admin → Templates page (email + import templates) | #12 |
| 15 | Archive button separated from Apply adjustment | #12 |
| 16 | vitest added; pay-week maths covered; vite bumped off a dev-server CVE | #12 |
| 17 | Entries store the day they belong to (`local_date`) | #13 |

## Things worth knowing before you continue

**The date handling was rebuilt, twice, and the second supersedes the first.**
PR #12 made period bounds respect the viewer's local midnight, which fixed the
reported symptom. PR #13 went further: entries now store `local_date` (the
calendar day they belong to) and `utc_offset_minutes`, decided once at write
time and never recomputed. Period queries are a plain string range over an
indexed column — no timezone arithmetic at query time, and no offset parameter
threaded through callers. If you find `to_rfc3339_bounds` in an old diff, it
no longer exists.

**The 0019 backfill is a one-time reconstruction for existing rows.** It
derives `local_date` from this machine's timezone, applying the rules in effect
on each entry's own date. For any entry logged while you were in a *different*
timezone, that is a guess — the offset that actually applied was never recorded
and nothing can recover it. Every row written from here on records its own, so
this never has to be guessed again.

**Report totals near period edges may have shifted** for the same reason. That
is the bug being corrected, not a regression, but if you compare against a
report generated before this batch, expect edge entries to have moved to the
period they actually belong to.

**Icon item (#3) needs a user action, not a code fix.** The blank taskbar icon
traced to a *pinned* taskbar shortcut pointing at a stale
`C:\Windows\Installer\{old-GUID}\ProductIcon` left over from an earlier MSI
version — not a bug in the app's icon files or build config. Unpin and re-pin
TimeTracker from the taskbar to fix it. This recurs on every version upgrade
for anyone who pins via the MSI-installed shortcut, since each version gets a
new MSI ProductCode by default — **expect it again after `v0.6.0`.**

**Migration-numbering hazard, already hit once.** `src-tauri/src/db/mod.rs`
applies migrations by *array position/count*, not by filename. PR #5 and #6
both added a migration at the same position; whichever merged second needed
renumbering via a rebase. If you ever develop two migration-adding branches in
parallel, expect this and rebase the second before merging. The October batch
deliberately kept 0017–0019 sequential on single branches to avoid it. The same
root cause can bite your **local dev AppData database** if you test several such
branches against it in sequence — you may see a `table X already exists` panic
on launch. Fix: delete
`%APPDATA%\dev.jbannerman.timetracker.dev\timetracker.sqlite` (safe, disposable
dev data) and relaunch.

**Backup files identify themselves by their header row.** `backup.rs` matches a
restore file purely by its CSV header, so *adding a column to a backed-up table
silently breaks every previously written backup file* — it stops matching any
known type, and rows restored without the new column can end up invisible to
queries. PR #13 hit this and handled it with a `legacy_headers` list plus
name-based (not position-based) value mapping. Do the same if you add another.

**Tests now exist on both sides.** `cargo test` (62) and `npm test` (24,
vitest). `npm run check` covers 133 files. Run all three before cutting a
release; `npm audit` and `cd src-tauri && cargo audit` are the dependency
equivalents, both clean as of this session.

## Where things are documented

- `Design Documents/ReleaseProcess.md` — branching convention, versioning, and
  the exact steps to cut a release (bump → tag → build → package → publish).
- `Design Documents/TimeTracker_New_Features_Request.md` — the running backlog.
  Emptied as of this session; everything in it shipped.
- `sbom/README.md` — how to regenerate the CycloneDX SBOMs when dependencies
  change meaningfully. Both were regenerated this session.
- This file — a point-in-time snapshot, not a living doc. Overwrite it at the
  end of a session rather than appending to it.
