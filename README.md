# TimeTracker

A lightweight, self-contained desktop app for tracking billable time across multiple
clients and contracts — built for an independent consultant who needs to run timers,
log manual entries, and produce client-ready timesheets without depending on any
cloud service. Everything runs locally: one SQLite database file, local `.xlsx`/`.csv`
exports, no accounts, no network calls.

Built with [Tauri v2](https://tauri.app/) (Rust backend), [Svelte 5](https://svelte.dev/)
+ TypeScript (frontend), and [SQLite](https://www.sqlite.org/) (via the bundled,
dependency-free `rusqlite` crate).

## Features

- **Clients & Contracts** — track Prime/Sub relationships, per-client week
  boundaries, per-client minimum billable increments (e.g. round up to the nearest 15
  minutes), and a full contract rate *history* (changing a rate never rewrites past
  entries' billed amounts).
- **Timers** — a full Timer page and a one-click-per-contract Quick Timer page.
  Multiple concurrent timers across different contracts are allowed (with an advisory
  warning), since work can legitimately span more than one client at once.
- **Categories** — optional, per-client labels for entries (never a hard requirement,
  even when a client has categories defined).
- **Manual entries & bulk import** — add or edit entries by hand, or bulk-import from
  CSV or multi-tab XLSX files, with flexible date parsing and automatic creation of
  any new category encountered.
- **Reports** — Week/Month totals with a click-to-drill-down breakdown: all clients →
  one client → one contract → that contract's raw entries for the period.
- **Timesheet export** — one formatted `.xlsx` file per contract (or scoped to just
  the client/contract you've drilled into on the Reports page), with currency-
  formatted Rate/Amount columns as an opt-in extra.
- **Backup, restore, and gated purge** — CSV backup/restore for every data type, and
  a Trash page for soft-deleted/archived data that always backs up before any
  permanent purge, and reports (rather than silently failing on) anything that
  couldn't be purged yet because something else still references it.
- **Personalization** — configurable window size/launch position, a default start
  page, and System/Light/Dark/Custom color themes.

## Documentation

In-depth project documentation lives in [`Design Documents/`](./Design%20Documents):

- [`Requirements.md`](./Design%20Documents/Requirements.md) — functional and
  non-functional requirements.
- [`Design.md`](./Design%20Documents/Design.md) — architecture, data model, and module
  map.
- [`Testing.md`](./Design%20Documents/Testing.md) — test strategy, automated test
  inventory, and the manual verification checklist.
- [`CodeReview.md`](./Design%20Documents/CodeReview.md) — findings and fixes from the
  most recent code review pass.
- [`SecurityAssessment.md`](./Design%20Documents/SecurityAssessment.md) — the most
  recent vulnerability assessment.
- [`CompileGuide.md`](./Design%20Documents/CompileGuide.md) — detailed, per-OS
  build-from-source instructions (summarized below).

## Dependencies

### Runtime / backend (Rust — see `src-tauri/Cargo.toml`)

| Crate | Version | Purpose |
|---|---|---|
| `tauri` | 2.11.3 | Application shell / native webview host |
| `tauri-plugin-dialog` | 2.7.2 | Native file/folder picker dialogs |
| `tauri-plugin-log` | 2.x | Debug logging in development |
| `rusqlite` (feature: `bundled`) | 0.38 | SQLite access — bundled, no system SQLite required |
| `serde` / `serde_json` | 1.0 | (De)serialization across the Tauri IPC boundary |
| `chrono` (feature: `serde`) | 0.4 | Date/time handling |
| `csv` | 1.4 | Backup/restore CSV read/write |
| `rust_xlsxwriter` | 0.96 | Timesheet `.xlsx` generation |
| `calamine` (feature: `dates`) | 0.36.1 | Reading `.xlsx` files for bulk import |
| `log` | 0.4 | Logging facade |

### Frontend (npm — see `package.json`)

| Package | Version | Purpose |
|---|---|---|
| `svelte` | ^5.56.10 | UI framework (runes) |
| `vite` | ^8.2.2 | Dev server / build tool |
| `typescript` | ~6.0.2 | Type checking |
| `@tauri-apps/api` | ^2.11.1 | JS/TS bindings for invoking Rust commands |
| `@tauri-apps/plugin-dialog` | ^2.7.2 | JS bindings for the native folder picker |
| `@tauri-apps/cli` | ^2.11.4 (dev) | `tauri dev` / `tauri build` CLI |
| `svelte-check` | ^4.7.6 (dev) | Type/diagnostic checking for `.svelte` files |
| `@sveltejs/vite-plugin-svelte` | ^7.3.0 (dev) | Svelte support for Vite |

### System-level (OS toolchains, not installed via npm/cargo)

Building from source additionally requires a Rust toolchain, Node.js, and OS-specific
native build tools (MSVC Build Tools + WebView2 on Windows, Xcode Command Line Tools on
macOS, WebKitGTK + related packages on Linux) — see
[`Design Documents/CompileGuide.md`](./Design%20Documents/CompileGuide.md) for the full
per-OS list and exact commands.

## Building from source (quick start)

```
git clone https://github.com/JesseMcWilliams/TimeTracker.git
cd TimeTracker
npm install
npx tauri dev
```

See [`Design Documents/CompileGuide.md`](./Design%20Documents/CompileGuide.md) for
full per-OS prerequisites and troubleshooting.

## Development

```
# Run the app in dev mode (hot-reloading frontend + auto-rebuilding backend)
npx tauri dev

# Rust unit tests
cd src-tauri && cargo test --lib

# Frontend type-checking
npm run check
```

## Deploying

`tauri build` produces a native, distributable bundle for whichever OS you run it on
(Tauri does not cross-compile between OS families — build on the OS you're targeting,
or use a CI matrix if you need all three from one place):

```
npm install
npx tauri build
```

Bundle output lands under `src-tauri/target/release/bundle/`, in the format(s)
appropriate to the host OS (this project's `tauri.conf.json` sets `bundle.targets` to
`"all"`, so every bundle format supported on the current OS is produced):

- **Windows** → an NSIS installer (`.exe`) and an `.msi` under
  `bundle/nsis/` and `bundle/msi/`.
- **macOS** → an `.app` bundle and a `.dmg` disk image under `bundle/macos/` and
  `bundle/dmg/`.
- **Linux** → `.deb` and `.rpm` packages and/or an `.AppImage`, depending on what's
  available on the build machine, under `bundle/deb/`, `bundle/rpm/`, and
  `bundle/appimage/`.

**Code signing**: none of the build targets are currently signed. Unsigned builds will
trigger OS-level security warnings (e.g. Windows SmartScreen, macOS Gatekeeper) when an
end user first runs them — expected for personal/internal use, but something to revisit
before distributing more broadly.

**Database migrations ship with the app**: schema migrations live under
`src-tauri/migrations/` and are embedded into the binary at compile time
(`include_str!`), then applied automatically and in order the first time a built app
opens its database — there's no separate migration step to run at deploy time.

## Where your data lives

TimeTracker stores everything in one SQLite file in the OS-standard per-app data
directory (identifier: `dev.jbannerman.timetracker`), created automatically on first
run:

- **Windows**: `%APPDATA%\dev.jbannerman.timetracker\timetracker.sqlite`
- **macOS**: `~/Library/Application Support/dev.jbannerman.timetracker/timetracker.sqlite`
- **Linux**: `~/.local/share/dev.jbannerman.timetracker/timetracker.sqlite`

Use the in-app **Backup Data** action (User page) to export CSV copies of everything,
and **Restore Data** to read them back in — see `Requirements.md` FR-24/25 for exactly
how restore's add-only, never-overwrite behavior works.

## Project status

Actively developed, single-maintainer project. See `Testing.md`'s "Known gaps" section
for what isn't covered by automated tests yet, and `SecurityAssessment.md` for the most
recent vulnerability review.

## License

MIT — see [`LICENSE`](./LICENSE).
