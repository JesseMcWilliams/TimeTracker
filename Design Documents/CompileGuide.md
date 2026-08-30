# TimeTracker — Compile Guide

This guide covers building TimeTracker from source on Windows, macOS, and Linux. It
assumes you've already cloned the repository:

```
git clone https://github.com/JesseMcWilliams/TimeTracker.git
cd TimeTracker
```

## 1. Common prerequisites (all platforms)

| Tool | Notes |
|---|---|
| **Rust** (via [rustup](https://rustup.rs/)) | `src-tauri/Cargo.toml` declares `rust-version = "1.77.2"` as the minimum; this project is developed and verified against a current stable toolchain (1.98.0 at time of writing). Installing via rustup and keeping it updated (`rustup update`) is the simplest way to stay compatible. |
| **Node.js + npm** | Needed for the Vite/Svelte frontend build. Verified against Node v26.8.1 / npm 11.19.0 in this project's own dev environment; any current LTS or newer release should work. |
| **Git** | To clone the repository. |

The Tauri CLI itself does **not** need to be installed globally — it's a
`devDependency` (`@tauri-apps/cli`) and is invoked via `npx tauri ...` after
`npm install`.

## 2. Windows

1. **Microsoft C++ Build Tools** — required so Rust can link against the MSVC
   toolchain. Install via the [Visual Studio Installer](https://visualstudio.microsoft.com/downloads/),
   either as part of a full Visual Studio install or the standalone "Build Tools for
   Visual Studio," selecting the **"Desktop development with C++"** workload.
2. **WebView2 Runtime** — Tauri renders the UI in Microsoft Edge WebView2. It's
   preinstalled by default on Windows 11 and on most up-to-date Windows 10 systems
   (delivered via Windows Update). If it's missing, install the Evergreen Bootstrapper
   from Microsoft's WebView2 downloads page.
3. Install Rust via [rustup](https://rustup.rs/) (choose the MSVC toolchain, which is
   the default on Windows).
4. From the project root, install frontend dependencies and run the dev build:
   ```
   npm install
   npm run tauri:dev
   ```
   (uses a separate app identifier from a release build, so it won't touch an
   installed release's database — see the README's Development section.)

## 3. macOS

1. **Xcode Command Line Tools**:
   ```
   xcode-select --install
   ```
   The full Xcode app (from the App Store) is not required for a desktop-only build.
2. Install Rust via [rustup](https://rustup.rs/).
3. From the project root:
   ```
   npm install
   npm run tauri:dev
   ```
   (uses a separate app identifier from a release build, so it won't touch an
   installed release's database — see the README's Development section.)

## 4. Linux

Package names below are for **Debian/Ubuntu**-based distributions; Fedora, Arch, and
others use equivalent packages under different names (e.g. `webkit2gtk4.1-devel` on
Fedora). If you're on a different distribution, check the
[official Tauri prerequisites documentation](https://tauri.app/) for the exact package
names for your package manager before proceeding.

1. Install system build dependencies:
   ```
   sudo apt update
   sudo apt install -y build-essential curl wget file libssl-dev \
     libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev \
     libwebkit2gtk-4.1-dev
   ```
   (Tauri v2 uses WebKitGTK 4.1 — note this differs from the `4.0` package name you
   may see referenced in older Tauri v1 guides.)
2. Install Rust via [rustup](https://rustup.rs/).
3. From the project root:
   ```
   npm install
   npm run tauri:dev
   ```
   (uses a separate app identifier from a release build, so it won't touch an
   installed release's database — see the README's Development section.)

## 5. Running the test suite

```
# Rust unit tests (business logic: rounding, purge, backup/restore, import parsing)
cd src-tauri
cargo test --lib

# Frontend type-checking
cd ..
npm run check
```

Both should report zero failures/errors before considering a change complete — see
`Testing.md` in this folder for what's covered and what isn't.

## 6. Building a release bundle

See `README.md`'s **Deploying** section for how to produce a distributable
installer/package for each platform once the app builds and runs correctly in dev
mode.

## 7. Troubleshooting

- **"link.exe not found" / MSVC linker errors on Windows**: the C++ Build Tools
  workload (step 2.1) wasn't installed, or Rust is using the wrong toolchain — run
  `rustup show` and confirm the active toolchain is `*-pc-windows-msvc`.
- **A blank/white window on launch**: usually a missing or outdated WebView2 runtime
  on Windows, or a missing WebKitGTK package on Linux — revisit the platform-specific
  prerequisites above.
- **`error: failed to run custom build command for 'app'` mentioning `webkit2gtk` or
  `glib` on Linux**: a system dependency from step 4.1 is missing; re-check the
  package list against what your distro actually calls those packages.
- **Port 1420 already in use**: a previous `tauri dev`/`vite` process is still running
  in the background — stop it before relaunching.
