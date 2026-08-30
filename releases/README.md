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
platform-specific bundles under:

```
src-tauri/target/release/bundle/
```

This `releases/` folder is where you copy the ones you want to keep, renamed per the
convention below.

## Naming convention

```
TimeTracker-v{version}-{platform}-{arch}.{ext}
```

Organize each release under its own version subfolder, e.g. `releases/v0.1.0/`:

| Platform | Example filename |
|---|---|
| Windows (NSIS installer) | `TimeTracker-v0.1.0-windows-x64-setup.exe` |
| Windows (MSI) | `TimeTracker-v0.1.0-windows-x64.msi` |
| macOS (Apple Silicon) | `TimeTracker-v0.1.0-macos-aarch64.dmg` |
| macOS (Intel) | `TimeTracker-v0.1.0-macos-x64.dmg` |
| macOS (universal, if built) | `TimeTracker-v0.1.0-macos-universal.dmg` |
| Linux (AppImage) | `TimeTracker-v0.1.0-linux-x86_64.AppImage` |
| Linux (Debian/Ubuntu) | `TimeTracker-v0.1.0-linux-x86_64.deb` |
| Linux (Fedora/RHEL) | `TimeTracker-v0.1.0-linux-x86_64.rpm` |

Rules of thumb:
- **`{version}`** always matches the `version` field in `src-tauri/tauri.conf.json`
  and `src-tauri/Cargo.toml` (keep those two in sync when you bump a version) —
  that's what ties a given installer back to an exact point in the source history.
- **`{platform}`** is always one of `windows` / `macos` / `linux`, lowercase.
- **`{arch}`** is the target CPU architecture (`x64`, `aarch64`, or `universal` for a
  macOS universal binary) — include it even when you only build for one architecture
  today, so filenames don't need to change later if you add another.
- Keep Tauri's own generated file *extension* and installer *type* as-is (`.exe`,
  `.msi`, `.dmg`, `.deb`, `.rpm`, `.AppImage`) — only the base filename is
  standardized, not the format.

## Suggested workflow

```
npx tauri build
mkdir -p releases/v0.1.0
cp src-tauri/target/release/bundle/nsis/*.exe releases/v0.1.0/TimeTracker-v0.1.0-windows-x64-setup.exe
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
