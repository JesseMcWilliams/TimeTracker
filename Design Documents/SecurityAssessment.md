# TimeTracker — Vulnerability Assessment (2026-08-29)

## 1. Threat model

TimeTracker is a single-user, fully offline desktop application. It makes no outbound
network requests, has no accounts, and stores everything in one local SQLite file that
only the OS-level user account running the app can already read/write directly. The
relevant threat model is therefore narrow:

- **In scope**: data corruption or loss from the app's own logic (handled by the
  backup-before-purge design, reviewed in `CodeReview.md`); injection or code-execution
  bugs that could be triggered by data the app itself parses (imported CSV/XLSX files,
  restored backup CSVs); dependency vulnerabilities; unintended IPC surface exposed to
  the webview.
- **Out of scope**: multi-user authorization, remote attackers, network transport
  security, and privilege escalation *within* the user's own account — a local user who
  can already read/write their own files gains nothing by attacking their own local app
  that they don't already have.

## 2. Dependency vulnerability scan

### Frontend (`npm audit`)
```
found 0 vulnerabilities
```
Clean, including dev dependencies.

### Backend (`cargo audit`, via `cargo-audit` installed for this assessment)

Initial scan found **2 high-severity (7.5) advisories**, both against `quick-xml
0.37.5`, pulled in transitively by `calamine 0.30.1` (used to read `.xlsx` files on
import):

- `RUSTSEC-2026-0194` — quadratic run time when checking a start tag for duplicate
  attribute names (denial-of-service via a crafted XML/XLSX file).
- `RUSTSEC-2026-0195` — unbounded namespace-declaration allocation in `NsReader`
  (memory-exhaustion denial of service via a crafted XML/XLSX file).

**Relevance**: the app parses `.xlsx` files during Bulk Import, chosen by the user via
a native file picker. Practical exploitability is low (the only person who could feed
the app a malicious file is the same person who already runs it), but a corrupted or
oddly-formed real-world spreadsheet could still trigger degraded performance/memory
use, and the fix was a straightforward, available dependency bump — not something to
leave in place just because the practical risk is low.

**Fix applied**: bumped `calamine` from `0.30.1` to `0.36.1` in `src-tauri/Cargo.toml`
(compatible with the project's existing `rust-version = "1.77.2"`; no MSRV bump
needed), which pulls in `quick-xml 0.41.0`. Re-running `cargo audit` afterward shows
**0 vulnerabilities** (only the unmaintained-crate warnings below remain). Verified:
`cargo check` builds clean and all 14 `cargo test --lib` tests still pass after the
bump.

**Remaining `cargo audit` output**: 17 *warnings* (not vulnerabilities — `cargo audit`
only reports an `error:` when an actual vulnerability advisory matches, which no
longer happens here). These are all "unmaintained" notices for the `gtk`/`gdk`/`atk`/
`glib` GTK3-binding family (`gtk-rs`) and a few `unic-*` Unicode-data crates, plus one
"unsound" notice for `glib`'s `VariantStrIter` iterator impl. All of these are
transitive dependencies of Tauri's Linux (`webkit2gtk`/GTK3) backend, pulled in so the
project *can* build for Linux — they are not exercised at all when building or running
on Windows, and there is currently no drop-in maintained replacement upstream in
Tauri's own dependency tree. **Recommendation**: no action needed today; re-check with
`cargo audit` periodically (e.g. before each release) in case Tauri's Linux backend
dependencies get updated upstream.

## 3. Injection / code-execution review

- **SQL injection**: every data-carrying SQL parameter across the entire backend is
  bound via `rusqlite::params![]` or a `Vec<Box<dyn ToSql>>`, never string-interpolated
  into query text. The handful of places that use `format!()` to build SQL only
  interpolate compile-time-constant table names or static query fragments (full detail
  in `CodeReview.md` finding #4). No injection vector found.
- **Shell/command injection**: no use of `std::process::Command` (or any other process-
  spawning API) anywhere in the codebase. There is no code path that shells out to the
  OS.
- **XSS / HTML injection**: no use of `{@html ...}` in any Svelte component and no use
  of `innerHTML`/`eval` anywhere in the frontend. All rendered data goes through
  Svelte's default (auto-escaping) text interpolation. No injection vector found.
- **Unsafe Rust**: zero `unsafe` blocks anywhere in the codebase (confirmed by a full-
  tree grep). Memory safety is fully backed by the Rust compiler's guarantees; there is
  no hand-rolled unsafe code to audit.
- **Path handling**: file paths the app writes to (backups, purge-backups, timesheets)
  are joined under a folder the user explicitly chose (via the native folder picker,
  or the pre-filled OS Documents folder), with generated filenames sanitized to
  alphanumerics/`-`/`_` (`sanitize()` in `timesheets.rs`) and collision-avoided via
  `unique_path()`. The User page's output-folder field is a free-text input rather than
  being locked to only what the native picker returns, meaning a user *could* type an
  arbitrary path — this is expected, intentional flexibility (a local user pointing the
  app at a folder they already know the path to) rather than a privilege boundary
  crossing, since no other account or trust level is involved. No fix applied; no
  vulnerability found under this app's threat model.
- **Import/restore of attacker-controlled data**: bulk import (CSV/XLSX) and backup
  restore (CSV) both parse files the user selects themselves. All parsed values flow
  into parameterized queries (see above); the XLSX-parsing DoS risk is covered in §2.
  No other decoding/deserialization vulnerability (e.g. unsafe deserialization of
  arbitrary types) was found — imported values are read as plain strings/numbers and
  explicitly parsed into known field types.

## 4. IPC / Tauri configuration review

- **Capability manifest** (`src-tauri/capabilities/default.json`): grants only
  `core:default`, `dialog:default`, and (added by this review) `dialog:allow-confirm`.
  This is a narrow, explicit allowlist — no filesystem-plugin, shell-plugin, or HTTP-
  plugin permissions are granted at all, and none of those plugins are even declared as
  dependencies in `Cargo.toml`. The webview cannot reach the filesystem or spawn
  processes except through the specific `#[tauri::command]` functions this app defines
  itself.
- **`dialog:allow-confirm` gap**: see `CodeReview.md` finding #2 — this was a
  functional bug more than a security bug (it silently broke confirmation prompts
  rather than allowing anything unintended), but is included here because it touches
  the IPC permission surface. Fixed as described there.
- **Content Security Policy**: `tauri.conf.json` has `app.security.csp: null`, i.e. no
  CSP is applied. **Not changed by this assessment** — the app has zero remote content,
  no `eval`, and no `{@html}`/`innerHTML` usage (confirmed above), which substantially
  limits what a CSP would even be defending against here; but this is still a
  recognized Tauri hardening recommendation left in its default (permissive) state.
  I did not attempt to write a specific CSP value because an incorrect one could
  silently break the app's rendering (e.g. Svelte/Vite's injected `<style>` tags) in a
  way I cannot visually verify from this environment — a real risk when guessing at
  security configuration is worse than a documented gap. **Recommendation**: if
  tightening this further is desired, introduce a CSP (e.g. `default-src 'self'`) in a
  change that can be visually verified in a running window, ideally as its own isolated
  change so any breakage is easy to attribute.
- **Secrets**: grepped the frontend for hardcoded credentials/API keys/tokens — none
  found (the app has no external API integration to hold credentials for in the first
  place).

## 5. Summary

| Area | Result |
|---|---|
| npm audit (frontend deps) | 0 vulnerabilities |
| cargo audit (backend deps) | 2 high-severity found → **fixed** (calamine bump); 0 remain; 17 unrelated "unmaintained" warnings on Linux-only GTK bindings, accepted/monitored |
| SQL injection | None found |
| Shell/command injection | None found (no process-spawning code exists) |
| XSS / HTML injection | None found (no `{@html}`/`innerHTML`/`eval`) |
| Unsafe Rust | None (zero `unsafe` blocks) |
| Path handling | Reviewed; no privilege-boundary issue under this app's single-user local threat model |
| IPC capability surface | Narrow, explicit allowlist; one gap found and fixed (`dialog:allow-confirm`) |
| CSP | Not configured; documented as an accepted, low-impact hardening gap pending a visually-verifiable follow-up change |
| Hardcoded secrets | None found |

**Overall**: no exploitable vulnerability found within this app's actual threat model
(single local user, no network, no multi-tenant boundary). Two real issues were found
and fixed during this pass (the outdated `calamine`/`quick-xml` dependency, and the
missing `dialog:allow-confirm` capability); the CSP gap is a deliberate, documented
non-fix pending a change that can be visually verified rather than guessed at.
