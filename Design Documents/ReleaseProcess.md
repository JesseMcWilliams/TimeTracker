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
  exercised per `Testing.md` §3 if it touches something in that checklist), merge it
  into `main`:
  ```
  git checkout main
  git merge --no-ff feature/your-branch
  git branch -d feature/your-branch
  git push origin main
  ```
  `--no-ff` keeps a merge commit marking where the feature landed, which is useful
  later for `git log` / `git bisect` even without a PR review process.
- No pull request process is prescribed here — for a solo project, a branch is just a
  workspace to keep in-progress work out of `main` until it's actually done, not a
  review gate. Use PRs anyway if you want the diff view; it's not required.

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
