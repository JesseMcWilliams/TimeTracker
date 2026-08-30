<script lang="ts">
  import { onMount } from 'svelte'
  import { open } from '@tauri-apps/plugin-dialog'
  import {
    api,
    START_PAGES,
    LAUNCH_POSITIONS,
    THEMES,
    type StartPage,
    type LaunchPosition,
    type Theme,
    type BackupFile,
    type RestoreResult,
    type PurgeAllResult,
    type WindowGeometry,
  } from './api'
  import { store, navGuard, refreshUserProfile, refreshAll } from './store.svelte'
  import { isDirty, formatBytes } from './dateUtils'
  import { applyTheme } from './theme'

  const THEME_LABELS: Record<Theme, string> = {
    system: 'System (follow OS)',
    light: 'Light',
    dark: 'Dark',
    custom: 'Custom',
  }

  const START_PAGE_LABELS: Record<StartPage, string> = {
    timer: 'Timer',
    'quick-timer': 'Quick Timer',
    clients: 'Clients',
    contracts: 'Contracts',
    entries: 'Entries',
    trash: 'Trash',
    reports: 'Reports',
    import: 'Import',
    user: 'User',
  }

  const LAUNCH_POSITION_LABELS: Record<LaunchPosition, string> = {
    default: 'Default (OS decides)',
    'top-left': 'Upper Left',
    'top-center': 'Upper Center',
    'top-right': 'Upper Right',
    'middle-left': 'Middle Left',
    center: 'Center',
    'middle-right': 'Middle Right',
    'bottom-left': 'Lower Left',
    'bottom-center': 'Lower Center',
    'bottom-right': 'Lower Right',
    custom: 'Custom (remembered position)',
  }

  let firstName = $state('')
  let lastName = $state('')
  let fullName = $state('')
  let email = $state('')
  let outputFolder = $state('')
  let outputType = $state<'csv' | 'xlsx'>('xlsx')
  let windowWidth = $state<number | ''>('')
  let windowHeight = $state<number | ''>('')
  let windowX = $state<number | ''>('')
  let windowY = $state<number | ''>('')
  let defaultStartPage = $state<StartPage>('timer')
  let launchPosition = $state<LaunchPosition>('default')
  let theme = $state<Theme>('system')
  let customBg = $state('#ffffff')
  let customText = $state('#24292f')
  let customButtonBg = $state('#f0f0f0')
  let customButtonText = $state('#24292f')
  let busy = $state(false)
  let loadedFor = $state<string | null>(null)

  let currentGeometry = $state<WindowGeometry | null>(null)
  let dbSize = $state<number | null>(null)

  onMount(async () => {
    currentGeometry = await api.getWindowGeometry()
    dbSize = await api.getDatabaseSize()
  })

  $effect(() => {
    applyTheme({ theme, customBg, customText, customButtonBg, customButtonText })
  })

  async function useCurrentAsDefault() {
    const geo = await api.getWindowGeometry()
    currentGeometry = geo
    windowWidth = geo.width
    windowHeight = geo.height
    windowX = geo.x
    windowY = geo.y
    launchPosition = 'custom'
  }

  let backupBusy = $state(false)
  let backupResult = $state<BackupFile[] | null>(null)
  let backupError = $state('')

  let restoreBusy = $state(false)
  let restoreResult = $state<RestoreResult | null>(null)
  let restoreError = $state('')

  let purgeAllBusy = $state(false)
  let purgeAllResult = $state<PurgeAllResult | null>(null)
  let purgeAllError = $state('')

  $effect(() => {
    const key = JSON.stringify(store.userProfile)
    if (loadedFor !== key) {
      firstName = store.userProfile.firstName ?? ''
      lastName = store.userProfile.lastName ?? ''
      fullName = store.userProfile.fullName ?? ''
      email = store.userProfile.email ?? ''
      outputFolder = store.userProfile.outputFolder ?? ''
      outputType = store.userProfile.outputType
      windowWidth = store.userProfile.windowWidth ?? ''
      windowHeight = store.userProfile.windowHeight ?? ''
      windowX = store.userProfile.windowX ?? ''
      windowY = store.userProfile.windowY ?? ''
      defaultStartPage = store.userProfile.defaultStartPage
      launchPosition = store.userProfile.launchPosition
      theme = store.userProfile.theme
      customBg = store.userProfile.customBg ?? '#ffffff'
      customText = store.userProfile.customText ?? '#24292f'
      customButtonBg = store.userProfile.customButtonBg ?? '#f0f0f0'
      customButtonText = store.userProfile.customButtonText ?? '#24292f'
      loadedFor = key
      if (!outputFolder) {
        api.getDefaultOutputFolder().then((d) => {
          if (!outputFolder) outputFolder = d
        })
      }
    }
  })

  let dirty = $derived(
    isDirty(
      {
        firstName,
        lastName,
        fullName,
        email,
        outputFolder,
        outputType,
        windowWidth,
        windowHeight,
        windowX,
        windowY,
        defaultStartPage,
        launchPosition,
        theme,
        customBg,
        customText,
        customButtonBg,
        customButtonText,
      },
      {
        firstName: store.userProfile.firstName ?? '',
        lastName: store.userProfile.lastName ?? '',
        fullName: store.userProfile.fullName ?? '',
        email: store.userProfile.email ?? '',
        outputFolder: store.userProfile.outputFolder ?? '',
        outputType: store.userProfile.outputType,
        windowWidth: store.userProfile.windowWidth ?? '',
        windowHeight: store.userProfile.windowHeight ?? '',
        windowX: store.userProfile.windowX ?? '',
        windowY: store.userProfile.windowY ?? '',
        defaultStartPage: store.userProfile.defaultStartPage,
        launchPosition: store.userProfile.launchPosition,
        theme: store.userProfile.theme,
        customBg: store.userProfile.customBg ?? '#ffffff',
        customText: store.userProfile.customText ?? '#24292f',
        customButtonBg: store.userProfile.customButtonBg ?? '#f0f0f0',
        customButtonText: store.userProfile.customButtonText ?? '#24292f',
      },
    ),
  )

  $effect(() => {
    navGuard.isDirty = dirty
  })

  async function browseOutputFolder() {
    const selected = await open({ directory: true, multiple: false, defaultPath: outputFolder || undefined })
    if (typeof selected === 'string') {
      outputFolder = selected
    }
  }

  function suggestFullName() {
    if (!fullName.trim() && (firstName.trim() || lastName.trim())) {
      fullName = [lastName.trim(), firstName.trim()].filter(Boolean).join(', ')
    }
  }

  async function save() {
    busy = true
    try {
      await api.saveUserProfile({
        firstName: firstName.trim() || null,
        lastName: lastName.trim() || null,
        fullName: fullName.trim() || null,
        email: email.trim() || null,
        outputFolder: outputFolder.trim() || null,
        outputType,
        windowWidth: windowWidth === '' ? null : Number(windowWidth),
        windowHeight: windowHeight === '' ? null : Number(windowHeight),
        windowX: windowX === '' ? null : Number(windowX),
        windowY: windowY === '' ? null : Number(windowY),
        defaultStartPage,
        launchPosition,
        theme,
        customBg,
        customText,
        customButtonBg,
        customButtonText,
      })
      await refreshUserProfile()
      navGuard.isDirty = false
    } finally {
      busy = false
    }
  }

  async function runBackup() {
    backupBusy = true
    backupError = ''
    backupResult = null
    try {
      backupResult = await api.backupAllData()
    } catch (e) {
      backupError = String(e)
    } finally {
      backupBusy = false
    }
  }

  async function runPurgeAll() {
    if (
      !confirm(
        'This will back up everything, then permanently delete ALL clients, contracts, categories, and time entries. Your User settings are kept. This cannot be undone. Continue?',
      )
    )
      return
    purgeAllBusy = true
    purgeAllError = ''
    purgeAllResult = null
    try {
      purgeAllResult = await api.purgeAllData()
      await refreshAll()
    } catch (e) {
      purgeAllError = String(e)
    } finally {
      purgeAllBusy = false
    }
  }

  async function runRestore() {
    const selected = await open({
      multiple: true,
      filters: [{ name: 'TimeTracker backup', extensions: ['csv'] }],
    })
    const filePaths = Array.isArray(selected) ? selected : typeof selected === 'string' ? [selected] : []
    if (filePaths.length === 0) return

    restoreBusy = true
    restoreError = ''
    restoreResult = null
    try {
      restoreResult = await api.restoreFromBackups(filePaths)
    } catch (e) {
      restoreError = String(e)
    } finally {
      restoreBusy = false
    }
  }
</script>

<section class="panel">
  <h2>User</h2>

  <div class="field">
    <label for="first-name">First name</label>
    <input id="first-name" bind:value={firstName} onblur={suggestFullName} />
  </div>
  <div class="field">
    <label for="last-name">Last name</label>
    <input id="last-name" bind:value={lastName} onblur={suggestFullName} />
  </div>
  <div class="field">
    <label for="full-name">Full name</label>
    <input id="full-name" bind:value={fullName} placeholder="Used on generated timesheet filenames" />
  </div>
  <div class="field">
    <label for="email">Email</label>
    <input id="email" type="email" bind:value={email} />
  </div>
  <div class="field">
    <label for="output-folder">Output folder</label>
    <input
      id="output-folder"
      bind:value={outputFolder}
      placeholder="e.g. C:\Users\you\Documents\TimeTracker Exports"
      style="flex: 1"
    />
    <button type="button" onclick={browseOutputFolder}>Browse…</button>
  </div>
  <div class="field">
    <label for="output-type">Output type</label>
    <select id="output-type" bind:value={outputType}>
      <option value="xlsx">Excel (.xlsx)</option>
      <option value="csv">CSV</option>
    </select>
  </div>
  <div class="field">
    <label for="window-width">Window size</label>
    <input id="window-width" type="number" min="200" placeholder="Width" bind:value={windowWidth} style="width: 6rem; flex: 0 0 auto" />
    <span>&times;</span>
    <input id="window-height" type="number" min="200" placeholder="Height" bind:value={windowHeight} style="width: 6rem; flex: 0 0 auto" />
    <span class="muted">px (applied next launch)</span>
  </div>
  <div class="field">
    <span class="field-spacer" aria-hidden="true"></span>
    <span class="muted">
      {#if currentGeometry}
        Current size: {currentGeometry.width} &times; {currentGeometry.height}px at ({currentGeometry.x}, {currentGeometry.y})
      {:else}
        Loading current window info…
      {/if}
    </span>
  </div>
  <div class="field">
    <span class="field-spacer" aria-hidden="true"></span>
    <button type="button" onclick={useCurrentAsDefault}>Use current position &amp; size as default</button>
  </div>
  <div class="field">
    <label for="start-page">Start page</label>
    <select id="start-page" bind:value={defaultStartPage}>
      {#each START_PAGES as page}
        <option value={page}>{START_PAGE_LABELS[page]}</option>
      {/each}
    </select>
  </div>
  <div class="field">
    <label for="launch-position">Launch position</label>
    <select id="launch-position" bind:value={launchPosition}>
      {#each LAUNCH_POSITIONS as pos}
        <option value={pos}>{LAUNCH_POSITION_LABELS[pos]}</option>
      {/each}
    </select>
    {#if launchPosition === 'custom' && windowX !== '' && windowY !== ''}
      <span class="muted">({windowX}, {windowY})</span>
    {/if}
  </div>

  <div class="field">
    <span class="field-spacer" aria-hidden="true"></span>
    <span class="muted">Database size: {dbSize === null ? 'loading…' : formatBytes(dbSize)}</span>
  </div>

  <h3>Colors</h3>
  <div class="field">
    <label for="theme">Theme</label>
    <select id="theme" bind:value={theme}>
      {#each THEMES as t}
        <option value={t}>{THEME_LABELS[t]}</option>
      {/each}
    </select>
  </div>
  {#if theme === 'custom'}
    <div class="field">
      <label for="custom-bg">Background</label>
      <input id="custom-bg" type="color" bind:value={customBg} />
    </div>
    <div class="field">
      <label for="custom-text">Text</label>
      <input id="custom-text" type="color" bind:value={customText} />
    </div>
    <div class="field">
      <label for="custom-button-bg">Button background</label>
      <input id="custom-button-bg" type="color" bind:value={customButtonBg} />
    </div>
    <div class="field">
      <label for="custom-button-text">Button text</label>
      <input id="custom-button-text" type="color" bind:value={customButtonText} />
    </div>
  {/if}
  <button class:unchanged={!dirty} onclick={save} disabled={busy || !dirty}>Save</button>

  <h3>Backup &amp; Restore</h3>
  <p class="muted">
    Backup writes one CSV per data type (Clients, Contracts, Contract Rates, Categories,
    Time Entries) to your output folder. Restore reads those CSVs back in — it only adds
    rows that don't already exist (matched by id), so it's safe to run more than once,
    but it never overwrites or removes existing data.
  </p>
  <div class="row">
    <button onclick={runBackup} disabled={backupBusy}>Backup Data</button>
    <button onclick={runRestore} disabled={restoreBusy}>Restore Data…</button>
    <button onclick={runPurgeAll} disabled={purgeAllBusy} class="danger-btn">Purge All…</button>
  </div>
  <p class="muted">
    Purge All backs up everything (same as Backup Data), then permanently deletes every
    client, contract, category, and time entry. Your User settings are not touched.
  </p>

  {#if backupError}
    <p class="error">{backupError}</p>
  {:else if backupResult}
    <ul class="results">
      {#each backupResult as f}
        <li>{f.dataType}: {f.count} rows → {f.path}</li>
      {/each}
    </ul>
  {/if}

  {#if restoreError}
    <p class="error">{restoreError}</p>
  {:else if restoreResult}
    <ul class="results">
      {#each restoreResult.restored as r}
        <li>{r.dataType}: {r.inserted} new rows inserted</li>
      {/each}
      {#each restoreResult.errors as err}
        <li class="error-item">{err}</li>
      {/each}
    </ul>
  {/if}

  {#if purgeAllError}
    <p class="error">{purgeAllError}</p>
  {:else if purgeAllResult}
    <ul class="results">
      {#each purgeAllResult.backups as f}
        <li>{f.dataType}: {f.count} rows backed up → {f.path}</li>
      {/each}
      <li>All data purged (User settings kept).</li>
    </ul>
  {/if}
</section>

<style>
  .panel {
    margin-bottom: 2rem;
    max-width: 32rem;
  }
  .field {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.6rem;
  }
  .field label,
  .field-spacer {
    width: 7rem;
    flex-shrink: 0;
  }
  .field input,
  .field select {
    flex: 1;
  }
  button.unchanged {
    background: #e5e5e5;
    color: #888;
    border-color: #d5d5d5;
  }
  .muted {
    color: #666;
    font-size: 0.85rem;
  }
  h3 {
    margin-top: 2rem;
  }
  .row {
    display: flex;
    gap: 0.5rem;
    margin: 0.5rem 0;
  }
  .error {
    background: #f8d7da;
    color: #58151c;
    border: 1px solid #f1aeb5;
    border-radius: 4px;
    padding: 0.5rem 0.75rem;
    margin: 0.5rem 0;
  }
  .results {
    margin-top: 0.5rem;
    padding-left: 1.2rem;
    font-size: 0.9rem;
  }
  .error-item {
    color: #b91c1c;
  }
  .danger-btn {
    color: #b91c1c;
  }
</style>
