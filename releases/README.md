# Releases

This folder is a local, organized place to collect built installers when you cut a
release — e.g. before handing one to a client, uploading it somewhere, or attaching it
to a GitHub Release. It is **not** where `tauri build` writes its output, and its
actual contents are **not committed to git** (see `.gitignore` in this folder) —
only this README and the `.gitignore` itself are tracked, so the folder exists for
anyone who clones the repo, but nobody accumulates megabytes of installers in source
control over time.

## Where builds actually come from

Running `npx tauri build` (see the root `README.md`'s **Deploying** section) writes
platform-specific installer bundles under `src-tauri/target/release/bundle/`, and —
as a side effect of the same build, before it even gets to bundling — leaves the raw,
un-bundled executable at `src-tauri/target/release/app.exe` (or `app` with no
extension on macOS/Linux). That raw executable needs nothing installed to run: no
setup wizard, no registry entries, no admin rights — copy it anywhere and run it
directly. It still depends on the OS's native webview runtime being present (WebView2
on Windows, WebKitGTK on Linux — both already required for the installed version too;
practically always already present on a current OS), but that's the only external
dependency. This `releases/` folder is where you copy whichever of these you want to
keep, renamed per the convention below.

## Naming convention

```
TimeTracker-v{version}-{platform}-{arch}[-{variant}].{ext}
```

Organize each release under its own version subfolder, e.g. `releases/v0.1.0/`.
Windows gets **three** options; the other platforms currently ship the one installer
format built by default on that OS (add a `-portable` row for them too if you ever
build one — same idea, same source file, `target/release/app`):

| Platform | Example filename | Needs installing? |
|---|---|---|
| Windows (NSIS installer) | `TimeTracker-v0.1.0-windows-x64-setup.exe` | Yes |
| Windows (MSI) | `TimeTracker-v0.1.0-windows-x64.msi` | Yes |
| Windows (portable, no installer) | `TimeTracker-v0.1.0-windows-x64-portable.exe` | No — copy & run |
| macOS (Apple Silicon) | `TimeTracker-v0.1.0-macos-aarch64.dmg` | Yes |
| macOS (Intel) | `TimeTracker-v0.1.0-macos-x64.dmg` | Yes |
| macOS (universal, if built) | `TimeTracker-v0.1.0-macos-universal.dmg` | Yes |
| Linux (AppImage) | `TimeTracker-v0.1.0-linux-x86_64.AppImage` | Yes |
| Linux (Debian/Ubuntu) | `TimeTracker-v0.1.0-linux-x86_64.deb` | Yes |
| Linux (Fedora/RHEL) | `TimeTracker-v0.1.0-linux-x86_64.rpm` | Yes |

Rules of thumb:
- **`{version}`** always matches the `version` field in `src-tauri/tauri.conf.json`
  and `src-tauri/Cargo.toml` (keep those two in sync when you bump a version) —
  that's what ties a given installer back to an exact point in the source history.
- **`{platform}`** is always one of `windows` / `macos` / `linux`, lowercase.
- **`{arch}`** is the target CPU architecture (`x64`, `aarch64`, or `universal` for a
  macOS universal binary) — include it even when you only build for one architecture
  today, so filenames don't need to change later if you add another.
- **`{variant}`** is omitted for a normal installer; use `-portable` for the raw,
  no-install executable.
- Keep Tauri's own generated file *extension* and installer *type* as-is (`.exe`,
  `.msi`, `.dmg`, `.deb`, `.rpm`, `.AppImage`) — only the base filename is
  standardized, not the format.

## Suggested workflow

A single `npx tauri build` produces all three Windows files at once — the raw exe is
just sitting there as a side effect of the same build, no separate `--no-bundle` pass
needed:

```
npx tauri build
mkdir -p releases/v0.1.0
cp src-tauri/target/release/bundle/nsis/*.exe releases/v0.1.0/TimeTracker-v0.1.0-windows-x64-setup.exe
cp src-tauri/target/release/bundle/msi/*.msi releases/v0.1.0/TimeTracker-v0.1.0-windows-x64.msi
cp src-tauri/target/release/app.exe releases/v0.1.0/TimeTracker-v0.1.0-windows-x64-portable.exe
```

(adjust the source path/extension per platform, per the table above).

## Publishing further

If you want these attached somewhere other people can grab them from, GitHub Releases
is a natural fit — the `gh` CLI (already used for this repo) can create one and upload
files in a single step, e.g.:

```
gh release create v0.1.0 releases/v0.1.0/* --title "v0.1.0" --notes "…"
```

Tag the release (`v{version}`, matching the filename convention above) so the release
and the exact source snapshot it was built from stay traceable to each other.
