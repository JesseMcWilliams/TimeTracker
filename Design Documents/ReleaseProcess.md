# TimeTracker — Branching & Release Process

## Why this exists

Every commit so far has gone straight to `main`. That's fine for a solo project, but
it means `main` is only as stable as whatever the most recent in-progress change
happens to be — there's no point to tag a release from without first checking nothing
half-finished snuck in. This document is the (lightweight) fix: keep `main` always
releasable, do new work on branches, and only tag a release from `main`.

## Branching convention

- **`main`** is always releasable. Anything merged into it should already pass
  `cargo test --lib` and `npm run check` (svelte-check + tsc).
- **New work** happens on a short-lived branch off `main`, named:
  - `feature/<short-kebab-case-description>` — new functionality
  - `fix/<short-kebab-case-description>` — bug fixes
  - e.g. `feature/quick-timer-stop-icon`, `fix/entry-detail-dirty-check`
- When the branch is done and verified (tests pass, svelte-check clean, manually
  exercised per `Testing.md` §3 if it touches something in that checklist), push it
  and open a pull request into `main` instead of merging locally:
  ```
  git push -u origin feature/your-branch
  gh pr create --title "..." --body "..."
  ```
  Merge the PR (via `gh pr merge --squash` or `--merge`, or on GitHub's web UI), then
  clean up both branches:
  ```
  git checkout main
  git pull origin main
  git branch -d feature/your-branch
  git push origin --delete feature/your-branch
  ```
- This is a solo project, so a PR here isn't a review gate — it exists for the diff
  view and a durable record of what shipped together. (Earlier work in this project's
  history merged branches directly into `main` locally, without a PR; the convention
  going forward is PR-based, per the above.)

## Versioning

The app version lives in two places that must always agree:
`src-tauri/tauri.conf.json`'s `version` field and `src-tauri/Cargo.toml`'s `version`
field. Use `npm run bump-version -- <patch|minor|major|X.Y.Z>` to update both at once
(see `scripts/bump-version.mjs`) — it only edits those two files and prints the
remaining steps; it never touches git or builds anything itself.

Rough semver guidance pre-1.0 (where breaking changes are expected occasionally):
- **patch** (`0.1.0` → `0.1.1`) — bug fixes only, no new functionality.
- **minor** (`0.1.0` → `0.2.0`) — new functionality, even if it changes existing
  behavior (e.g. the timesheet-grouping change) — this is the common case.
- **major** — reserved for once the app is stable enough that "1.0" means something;
  not expected to come up soon.

## Cutting a release

1. Make sure `main` is what you want to ship (merge in any finished feature/fix
   branches first).
2. Bump the version: `npm run bump-version -- <patch|minor|major|X.Y.Z>`, then follow
   the steps it prints (regenerate `Cargo.lock`, commit, tag `vX.Y.Z`, push).
3. Build: `npm run build && npx tauri build` (Windows only from this machine — see
   `CompileGuide.md` for building the macOS/Linux bundles on those OSes). On Windows
   this produces three usable files in one pass: the NSIS installer, the MSI
   installer, and — already sitting at `target/release/app.exe` as a side effect of
   the same build, no extra step — the portable no-install executable.
4. Copy the resulting installer(s) (and, on Windows, `target/release/app.exe` as the
   portable variant) into `releases/vX.Y.Z/`, renamed per `releases/README.md`'s
   naming convention.
5. Publish: `gh release create vX.Y.Z releases/vX.Y.Z/* --title "vX.Y.Z" --notes "..."`
   (or upload additional platform builds to the same release later with
   `gh release upload vX.Y.Z <file>`).

## Hotfixing an already-released version

Not yet needed (there's only been one release), but if a bug needs fixing in a
released version while `main` has since moved on to unreleased work: branch from the
release tag (`git checkout -b fix/urgent-thing v0.1.0`), fix and verify it there, tag
a new patch release from that branch, then merge the same fix into `main` too (via
`git cherry-pick` or a normal merge) so it isn't lost the next time `main` ships.
