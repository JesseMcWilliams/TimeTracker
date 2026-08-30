// Bumps the app version consistently across every file that needs to agree on it, and
// prints the remaining manual steps (this script never touches git or builds anything
// itself — those stay explicit, deliberate actions).
//
// Usage:
//   node scripts/bump-version.mjs 0.2.0        # set an exact version
//   node scripts/bump-version.mjs patch        # 0.1.0 -> 0.1.1
//   node scripts/bump-version.mjs minor        # 0.1.0 -> 0.2.0
//   node scripts/bump-version.mjs major        # 0.1.0 -> 1.0.0

import { readFileSync, writeFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'

const rootDir = join(dirname(fileURLToPath(import.meta.url)), '..')
const tauriConfPath = join(rootDir, 'src-tauri', 'tauri.conf.json')
const cargoTomlPath = join(rootDir, 'src-tauri', 'Cargo.toml')

function currentVersion() {
  const conf = JSON.parse(readFileSync(tauriConfPath, 'utf8'))
  return conf.version
}

function bump(version, kind) {
  const parts = version.split('.').map(Number)
  if (parts.length !== 3 || parts.some(Number.isNaN)) {
    throw new Error(`current version "${version}" isn't a plain MAJOR.MINOR.PATCH — set an exact version instead`)
  }
  let [major, minor, patch] = parts
  if (kind === 'major') {
    major += 1
    minor = 0
    patch = 0
  } else if (kind === 'minor') {
    minor += 1
    patch = 0
  } else if (kind === 'patch') {
    patch += 1
  } else {
    throw new Error(`unknown bump kind "${kind}"`)
  }
  return `${major}.${minor}.${patch}`
}

function setJsonVersion(path, version) {
  const text = readFileSync(path, 'utf8')
  const updated = text.replace(/"version":\s*"[^"]*"/, `"version": "${version}"`)
  if (updated === text) throw new Error(`could not find a "version" field to update in ${path}`)
  writeFileSync(path, updated)
}

function setCargoVersion(path, version) {
  const text = readFileSync(path, 'utf8')
  const updated = text.replace(/^version = "[^"]*"/m, `version = "${version}"`)
  if (updated === text) throw new Error(`could not find a version = "..." line to update in ${path}`)
  writeFileSync(path, updated)
}

const arg = process.argv[2]
if (!arg) {
  console.error('Usage: node scripts/bump-version.mjs <patch|minor|major|X.Y.Z>')
  process.exit(1)
}

const from = currentVersion()
const to = /^\d+\.\d+\.\d+$/.test(arg) ? arg : bump(from, arg)

setJsonVersion(tauriConfPath, to)
setCargoVersion(cargoTomlPath, to)

console.log(`Version bumped: ${from} -> ${to}`)
console.log('')
console.log('Updated:')
console.log(`  - ${tauriConfPath}`)
console.log(`  - ${cargoTomlPath}`)
console.log('')
console.log('Next steps:')
console.log(`  1. cd src-tauri && cargo check   # regenerates Cargo.lock's version entry`)
console.log(`  2. git add -A -- src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock`)
console.log(`  3. git commit -m "Bump version to ${to}"`)
console.log(`  4. git tag -a v${to} -m "v${to}" && git push origin main --tags`)
console.log(`  5. npm run build && npx tauri build`)
console.log(`  6. Copy the built installer(s) into releases/v${to}/ per releases/README.md's naming convention`)
console.log(`  7. gh release create v${to} releases/v${to}/* --title "v${to}" --notes "..."`)
