<script lang="ts">
  import { api } from './api'
  import {
    store,
    navGuard,
    contractLabel,
    clientIdForContract,
    isContractArchived,
    loadTrackingCodesForClient,
    refreshEntries,
    refreshActiveTimers,
  } from './store.svelte'
  import { toRfc3339, toDatetimeLocalValue, formatMoney, isDirty } from './dateUtils'

  let {
    entryId,
    onBack,
    backLabel = 'Back to entries',
  }: { entryId: number; onBack: () => void; backLabel?: string } = $props()

  let entry = $derived(store.entries.find((e) => e.id === entryId))
  let contract = $derived(entry ? store.contracts.find((c) => c.id === entry.contractId) : undefined)
  let availableCodes = $derived(
    entry ? (store.trackingCodesByClient[clientIdForContract(entry.contractId) ?? -1] ?? []) : [],
  )
  let running = $derived(!!entry && entry.endedAt === null)

  let editStart = $state('')
  let editEnd = $state('')
  let editNotes = $state('')
  let editTrackingCodeId = $state<number | ''>('')
  let busy = $state(false)
  let initializedFor = $state<number | null>(null)
  let endSyncedFor = $state<number | null>(null)

  $effect(() => {
    if (entry && initializedFor !== entry.id) {
      editStart = toDatetimeLocalValue(entry.startedAt)
      editEnd = entry.endedAt ? toDatetimeLocalValue(entry.endedAt) : ''
      editNotes = entry.notes ?? ''
      editTrackingCodeId = entry.trackingCodeId ?? ''
      initializedFor = entry.id
      endSyncedFor = entry.endedAt ? entry.id : null
      const clientId = clientIdForContract(entry.contractId)
      if (clientId !== undefined) loadTrackingCodesForClient(clientId)
    } else if (entry && entry.endedAt && endSyncedFor !== entry.id) {
      // The entry was still running when this page first loaded and has since been
      // stopped (e.g. via the "Stop timer" button below) — pick up its real end time
      // without touching any in-progress, unsaved notes/category edit.
      editEnd = toDatetimeLocalValue(entry.endedAt)
      endSyncedFor = entry.id
    }
  })

  let endBeforeStart = $derived(!!editStart && !!editEnd && editEnd <= editStart)

  let metadataDirty = $derived(
    entry
      ? isDirty(
          { editNotes, editTrackingCodeId },
          { editNotes: entry.notes ?? '', editTrackingCodeId: entry.trackingCodeId ?? '' },
        )
      : false,
  )

  let dirty = $derived(
    entry
      ? isDirty(
          { editStart, editEnd, editNotes, editTrackingCodeId },
          {
            editStart: toDatetimeLocalValue(entry.startedAt),
            editEnd: entry.endedAt ? toDatetimeLocalValue(entry.endedAt) : '',
            editNotes: entry.notes ?? '',
            editTrackingCodeId: entry.trackingCodeId ?? '',
          },
        )
      : false,
  )

  // A single derived (rather than branching between metadataDirty/dirty directly
  // inside the effect) so navGuard tracks one unambiguous reactive value, the same
  // shape every other Detail page uses for this.
  let formDirty = $derived(running ? metadataDirty : dirty)

  $effect(() => {
    navGuard.isDirty = formDirty
  })

  async function save() {
    if (!entry || !editStart || !editEnd) return
    if (endBeforeStart) return
    busy = true
    try {
      await api.updateEntry(
        entry.id,
        toRfc3339(editStart),
        toRfc3339(editEnd),
        editNotes.trim() || undefined,
        editTrackingCodeId ? Number(editTrackingCodeId) : undefined,
      )
      await refreshEntries()
      navGuard.isDirty = false
    } finally {
      busy = false
    }
  }

  async function saveMetadata() {
    if (!entry) return
    busy = true
    try {
      await api.updateEntryMetadata(
        entry.id,
        editNotes.trim() || undefined,
        editTrackingCodeId ? Number(editTrackingCodeId) : undefined,
      )
      await refreshEntries()
      navGuard.isDirty = false
    } finally {
      busy = false
    }
  }

  async function stopTimer() {
    if (!entry) return
    busy = true
    try {
      await api.stopTimer(entry.id)
      await Promise.all([refreshEntries(), refreshActiveTimers()])
    } finally {
      busy = false
    }
  }

  async function remove() {
    if (!entry) return
    if (!confirm('Delete this time entry? You can restore it later from Trash.')) return
    busy = true
    try {
      await api.deleteEntry(entry.id)
      await refreshEntries()
      onBack()
    } finally {
      busy = false
    }
  }
</script>

<section class="panel">
  <button onclick={onBack}>&larr; {backLabel}</button>

  {#if !entry}
    <p>This entry no longer exists.</p>
  {:else}
    <h2>Entry #{entry.id}</h2>
    <dl>
      <dt>Contract</dt>
      <dd class:archived={isContractArchived(entry.contractId)}>{contractLabel(entry.contractId)}</dd>
      <dt>Source</dt>
      <dd>{entry.source}</dd>
      <dt>Rate at time of entry</dt>
      <dd>{entry.rateSnapshot} {contract?.currency ?? ''} / hr</dd>
      {#if entry.durationSecs !== null}
        <dt>Amount</dt>
        <dd>{formatMoney((entry.durationSecs / 3600) * entry.rateSnapshot, contract?.currency ?? 'USD')}</dd>
      {/if}
    </dl>

    {#if running}
      <p class="muted">This timer is still running — start/end times can be edited once it's stopped.</p>
    {:else}
      <div class="field">
        <label for="edit-start">Start</label>
        <input id="edit-start" type="datetime-local" bind:value={editStart} />
      </div>
      <div class="field">
        <label for="edit-end">End</label>
        <input id="edit-end" type="datetime-local" bind:value={editEnd} min={editStart || undefined} />
      </div>
      {#if endBeforeStart}
        <p class="error">End time must be after the start time.</p>
      {/if}
    {/if}
    {#if availableCodes.length > 0}
      <div class="field">
        <label for="edit-code">Category</label>
        <select id="edit-code" bind:value={editTrackingCodeId}>
          <option value="">No category</option>
          {#each availableCodes as tc (tc.id)}
            <option value={tc.id}>{tc.code}</option>
          {/each}
        </select>
      </div>
    {/if}
    <div class="field notes-row">
      <label for="edit-notes">Notes</label>
      <textarea id="edit-notes" class="notes-input" bind:value={editNotes} rows="2"></textarea>
    </div>

    {#if running}
      <div class="row">
        <button class:unchanged={!metadataDirty} onclick={saveMetadata} disabled={busy || !metadataDirty}
          >Save</button
        >
        <button onclick={stopTimer} disabled={busy}>Stop timer</button>
        <button onclick={remove} disabled={busy} class="danger">Delete</button>
      </div>
    {:else}
      <div class="row">
        <button class:unchanged={!dirty} onclick={save} disabled={busy || !dirty || endBeforeStart}>Save</button>
        <button onclick={remove} disabled={busy} class="danger">Delete</button>
      </div>
    {/if}
  {/if}
</section>

<style>
  .panel {
    margin-bottom: 2rem;
  }
  dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 0.25rem 1rem;
    margin: 1rem 0;
  }
  dt {
    color: #666;
  }
  dd {
    margin: 0;
  }
  .field {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
  }
  .field label {
    width: 4.5rem;
  }
  .field.notes-row {
    align-items: flex-start;
  }
  .field.notes-row label {
    padding-top: 0.4rem;
  }
  .notes-input {
    flex: 1;
    width: 100%;
    box-sizing: border-box;
    resize: vertical;
    min-height: 2.5rem;
    height: clamp(2.5rem, 20vh, 12rem);
    font: inherit;
    padding: 0.4rem 0.5rem;
    border: 1px solid var(--border, #ccc);
    border-radius: 4px;
    background: var(--bg, #fff);
    color: var(--text, inherit);
  }
  .row {
    display: flex;
    gap: 0.5rem;
    margin-top: 1rem;
  }
  .danger {
    color: #b91c1c;
  }
  button.unchanged {
    background: #e5e5e5;
    color: #888;
    border-color: #d5d5d5;
  }
  .archived {
    text-decoration: line-through;
    color: #888;
  }
  .error {
    color: #b91c1c;
    font-size: 0.9rem;
    margin: -0.3rem 0 0.5rem;
  }
  .muted {
    color: #666;
    font-size: 0.9rem;
    margin: 0 0 0.75rem;
  }
</style>
