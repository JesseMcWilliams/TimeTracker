<script lang="ts">
  import { onMount } from 'svelte'
  import { open, confirm } from '@tauri-apps/plugin-dialog'
  import { api, type BackupFile, type RestoreResult, type PurgeAllResult } from './api'
  import { store, navGuard, refreshUserProfile, refreshAll } from './store.svelte'
  import { isDirty, formatBytes } from './dateUtils'

  let { onBack }: { onBack: () => void } = $props()

  let firstName = $state('')
  let lastName = $state('')
  let fullName = $state('')
  let email = $state('')
  let outputFolder = $state('')
  let outputType = $state<'csv' | 'xlsx'>('xlsx')
  let busy = $state(false)
  let loadedFor = $state<string | null>(null)

  let dbSize = $state<number | null>(null)

  onMount(async () => {
    dbSize = await api.getDatabaseSize()
  })

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
      { firstName, lastName, fullName, email, outputFolder, outputType },
      {
        firstName: store.userProfile.firstName ?? '',
        lastName: store.userProfile.lastName ?? '',
        fullName: store.userProfile.fullName ?? '',
        email: store.userProfile.email ?? '',
        outputFolder: store.userProfile.outputFolder ?? '',
        outputType: store.userProfile.outputType,
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
        ...store.userProfile,
        firstName: firstName.trim() || null,
        lastName: lastName.trim() || null,
        fullName: fullName.trim() || null,
        email: email.trim() || null,
        outputFolder: outputFolder.trim() || null,
        outputType,
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
      !(await confirm(
        'This will back up everything, then permanently delete ALL clients, contracts, categories, and time entries. Your User settings are kept. This cannot be undone. Continue?',
      ))
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
  <button onclick={onBack}>&larr; Back to Admin</button>

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
    <span class="field-spacer" aria-hidden="true"></span>
    <span class="muted">Database size: {dbSize === null ? 'loading…' : formatBytes(dbSize)}</span>
  </div>

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
