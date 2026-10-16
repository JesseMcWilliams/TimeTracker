# Software Bill of Materials (SBOM)

CycloneDX SBOMs for TimeTracker's two dependency trees, generated at commit
time rather than automatically — regenerate them whenever dependencies
change meaningfully (e.g. before cutting a release) using the commands
below.

- `sbom-npm.json` — frontend/build-tooling dependencies (`package-lock.json`).
  Generated with:
  ```
  npx @cyclonedx/cyclonedx-npm --output-file sbom/sbom-npm.json --output-format JSON
  ```
- `sbom-rust.json` — the Tauri backend's Rust crate dependencies
  (`src-tauri/Cargo.lock`). Generated with:
  ```
  cargo install cargo-cyclonedx   # one-time
  cd src-tauri
  cargo cyclonedx -f json --spec-version 1.5 --override-filename sbom-rust
  mv sbom-rust.json ../sbom/sbom-rust.json
  ```

Both are CycloneDX JSON documents (npm: spec 1.6, Rust: spec 1.5 — the
newest each generator supports) and can be fed into standard SBOM tooling
(e.g. `grype sbom:sbom/sbom-rust.json` for vulnerability scanning).

For a live vulnerability check instead of a point-in-time SBOM, use:
```
npm audit
cd src-tauri && cargo audit
```
