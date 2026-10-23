<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog'
  import { api, WEEKDAYS, type Weekday } from './api'
  import { store, navGuard, refreshUserProfile } from './store.svelte'
  import { capitalize, isDirty } from './dateUtils'

  let { onBack }: { onBack: () => void } = $props()

  let firstName = $state('')
  let lastName = $state('')
  let fullName = $state('')
  let email = $state('')
  let outputFolder = $state('')
  let outputType = $state<'csv' | 'xlsx'>('xlsx')
  let payWeekStart = $state<Weekday>('monday')
  let busy = $state(false)
  let loadedFor = $state<string | null>(null)

  $effect(() => {
    const key = JSON.stringify(store.userProfile)
    if (loadedFor !== key) {
      firstName = store.userProfile.firstName ?? ''
      lastName = store.userProfile.lastName ?? ''
      fullName = store.userProfile.fullName ?? ''
      email = store.userProfile.email ?? ''
      outputFolder = store.userProfile.outputFolder ?? ''
      outputType = store.userProfile.outputType
      payWeekStart = store.userProfile.payWeekStart
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
      { firstName, lastName, fullName, email, outputFolder, outputType, payWeekStart },
      {
        firstName: store.userProfile.firstName ?? '',
        lastName: store.userProfile.lastName ?? '',
        fullName: store.userProfile.fullName ?? '',
        email: store.userProfile.email ?? '',
        outputFolder: store.userProfile.outputFolder ?? '',
        outputType: store.userProfile.outputType,
        payWeekStart: store.userProfile.payWeekStart,
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
        payWeekStart,
      })
      await refreshUserProfile()
      navGuard.isDirty = false
    } finally {
      busy = false
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
    <label for="pay-week-start">Pay week starts</label>
    <select id="pay-week-start" bind:value={payWeekStart}>
      {#each WEEKDAYS as day (day)}
        <option value={day}>{capitalize(day)}</option>
      {/each}
    </select>
  </div>
  <p class="hint">
    Used by the Entries page's week filters when no contract is selected. With one
    selected, that client's own week start applies instead.
  </p>
  <div class="field">
    <label for="output-type">Output type</label>
    <select id="output-type" bind:value={outputType}>
      <option value="xlsx">Excel (.xlsx)</option>
      <option value="csv">CSV</option>
    </select>
  </div>
  <button class:unchanged={!dirty} onclick={save} disabled={busy || !dirty}>Save</button>
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
  .field label {
    width: 7rem;
    flex-shrink: 0;
  }
  .field input,
  .field select {
    flex: 1;
  }
  .hint {
    color: #666;
    font-size: 0.85rem;
    margin: -0.2rem 0 0.9rem 7.5rem;
    line-height: 1.45;
  }
  button.unchanged {
    background: #e5e5e5;
    color: #888;
    border-color: #d5d5d5;
  }
</style>
