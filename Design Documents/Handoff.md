# Handoff — 2026-10-23

Snapshot of where TimeTracker stands after this working session, for whoever
(including future-you) picks this up next. Overwrite it at the end of a session;
it is a point-in-time snapshot, not a changelog.

## Current state

- **`main`** is at commit `f329eb7`, clean working tree, no open PRs.
- **`v0.7.0` is released and is the latest.** Nothing on `main` is unreleased.
- Three releases came out of this session: `v0.6.0` (the October batch), `v0.6.1`
  (one bug found by using it), `v0.7.0` (two more found the same way).
- The app has been launched and exercised by hand against a real database, which
  is also what first applied migrations 0017–0020 and first exercised the
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
| 19 | Email templates get "This week"; week ranges follow the pay week start | #15 | v0.7.0 |
| 20 | Contract hours: "Set total to" alongside "Adjust by" | #17 | v0.7.0 |

## Things worth knowing before you continue

**Two of the three bugs reported this session were not where they looked.**
Worth remembering before fixing the next one from a description alone:

- *"Remaining hours don't go down"* was real in v0.6.0 (the store cached a
  contract's logged hours and only entries were refreshed). But the follow-up
  report that v0.6.1 "didn't fix it" was the fix never having been exercised —
  the database showed zero entries created since that version was installed. It
  was confirmed working once one was added. **Check whether the thing was
  actually tried before fixing it again.**
- *"The email used the previous week's dates and dropped a contract"* was not two
  bugs and not a staleness bug. The template's range was **Previous week**, which
  correctly means the last *completed* week; run mid-week it reports on a week you
  have already dealt with, and the idle-contract list differs accordingly. The
  real gap was that no current-week option existed. Fixed in #15.

**Verify a fix against the data, not the description.** Both diagnoses above came
from querying the real database directly — the production one lives at
`%APPDATA%\dev.jbannerman.timetracker\timetracker.sqlite`, and the dev build uses
a separate `…timetracker.dev\` database, so testing a release build does not touch
dev data and vice versa. `node --experimental-sqlite` reads either without any
tooling.

**Component tests only count from a cold cache.** `@testing-library/svelte` holds
state in its own `.svelte.js` module, and Vitest externalises dependencies by
default, so its runes throw `rune_outside_svelte` the moment a component mounts.
`vitest.config.ts` now inlines it. This was found only because a later test
happened to run with a cold cache — an earlier green run had been meaningless.
If component tests start failing inexplicably, suspect that config first.

**A test can assert less than it appears to.** The first two attempts at covering
the stale-hours bug both passed while the bug was live: one called the shared
refresh directly, and the source-level guard only proved the function *name*
appeared in the file. Only rendering the page and clicking the button proved the
path. `tests/entry-refresh.guard.test.ts` is still worth keeping — it catches a
whole class cheaply — but it is a net, not a proof.

**If you add an entry-mutating call to `api.ts`, add it to `ENTRY_MUTATIONS`** in
that guard test. It checks its own list against `api.ts` and fails if one goes
missing, so it will tell you rather than silently stopping covering it.

**Retarget stacked PRs before deleting a base branch.** Merging #15 with
`--delete-branch` auto-closed the PR stacked on it, and GitHub will not reopen a
PR whose base branch no longer exists. The work had to go up again as a new PR.

**`target/release/bundle/` keeps every version ever built.** Four versions' files
now sit there. Staging a release by folder contents rather than by name will
publish the previous release's binaries under the new version's name. The release
process now says so explicitly.

**The date handling was rebuilt twice, and the second supersedes the first.** PR
#12 made period bounds respect the viewer's local midnight; PR #13 went further —
entries store `local_date` (the day they belong to) and `utc_offset_minutes`,
decided once at write time and never recomputed. Period queries are a plain string
range over an indexed column. If you find `to_rfc3339_bounds` in an old diff, it
no longer exists.

**The 0019 backfill was a one-time reconstruction.** It derived `local_date` from
the machine's timezone for existing rows. For any entry logged while in a
*different* timezone that is a guess — the offset that applied was never recorded.
Every row written since records its own.

**Backup files identify themselves by their header row.** `backup.rs` matches a
restore file purely by its CSV header, so *adding a column to a backed-up table
silently breaks every previously written backup file*. PR #13 hit this and handled
it with a `legacy_headers` list plus name-based value mapping. Do the same if you
add another.

**Migration-numbering hazard, already hit once.** `src-tauri/src/db/mod.rs` applies
migrations by *array position/count*, not by filename. Two migration-adding branches
developed in parallel will collide; rebase the second before merging. The same root
cause can bite your local dev database if you test several such branches in
sequence — delete
`%APPDATA%\dev.jbannerman.timetracker.dev\timetracker.sqlite` and relaunch.

**Icon item (#3) needs a user action, not a code fix.** The blank taskbar icon is a
*pinned* shortcut pointing at a stale `C:\Windows\Installer\{old-GUID}\ProductIcon`
from an earlier MSI. Unpin and re-pin. It recurs on **every** version upgrade for
anyone who pins the MSI-installed shortcut, since each version gets a new
ProductCode.

**Tests, and what each is for.** `cargo test` (78) covers the domain and migrations.
`npm test` (49, seven files) covers date maths, the store, four components, and the
source-level guard. `npm run check` covers 164 files. All three are now step 1 of
the release process. `npm audit` and `cd src-tauri && cargo audit` are the
dependency equivalents, both clean as of this session.

**Left undone deliberately:** the dev database holds seeded sample data with a
pre-seed copy beside it as `timetracker.sqlite.pre-seed.bak` — both disposable.
`tags.rs::tags_for_entry` is dead code and has been warning for a while; it is
unrelated to anything here and was left alone.

## Where things are documented

- `Design Documents/ReleaseProcess.md` — branching convention, versioning, and the
  steps to cut a release. Updating this file is step 6 of that process.
- `Design Documents/TimeTracker_New_Features_Request.md` — the running backlog.
  Empty; everything in it shipped.
- `sbom/README.md` — regenerating the CycloneDX SBOMs when dependencies change.
  Both were regenerated this session.
