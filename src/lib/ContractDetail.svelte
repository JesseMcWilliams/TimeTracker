<script lang="ts">
  import { api, type FilenameDate } from './api'
  import { store, navGuard, refreshContracts } from './store.svelte'
  import { isDirty } from './dateUtils'

  let { contractId, onBack }: { contractId: number; onBack: () => void } = $props()

  let contract = $derived(store.contracts.find((c) => c.id === contractId))
  let client = $derived(contract ? store.clients.find((c) => c.id === contract.clientId) : undefined)

  let editName = $state('')
  let editCurrency = $state('')
  let editExternalId = $state('')
  let editStartDate = $state('')
  let editNotes = $state('')
  let editFilenameDate = $state<FilenameDate>('end')
  let initializedFor = $state<number | null>(null)

  let newRate = $state('')
  let busy = $state(false)

  $effect(() => {
    if (contract && initializedFor !== contract.id) {
      editName = contract.name
      editCurrency = contract.currency
      editExternalId = contract.externalId ?? ''
      editStartDate = contract.startDate ?? ''
      editNotes = contract.notes ?? ''
      editFilenameDate = contract.filenameDate
      initializedFor = contract.id
    }
  })

  let dirty = $derived(
    contract
      ? isDirty(
          { editName, editCurrency, editExternalId, editStartDate, editNotes, editFilenameDate },
          {
            editName: contract.name,
            editCurrency: contract.currency,
            editExternalId: contract.externalId ?? '',
            editStartDate: contract.startDate ?? '',
            editNotes: contract.notes ?? '',
            editFilenameDate: contract.filenameDate,
          },
        )
      : false,
  )

  $effect(() => {
    navGuard.isDirty = dirty
  })

  async function saveDetails() {
    if (!contract || !editName.trim() || !editCurrency.trim()) return
    busy = true
    try {
      await api.updateContractDetails(
        contract.id,
        editName.trim(),
        editCurrency.trim(),
        editFilenameDate,
        editExternalId.trim() || undefined,
        editStartDate || undefined,
        editNotes.trim() || undefined,
      )
      await refreshContracts()
      navGuard.isDirty = false
    } finally {
      busy = false
    }
  }

  async function saveRate() {
    if (!contract || newRate === '') return
    busy = true
    try {
      await api.updateContractRate(contract.id, Number(newRate))
      newRate = ''
      await refreshContracts()
    } finally {
      busy = false
    }
  }

  async function archive() {
    if (!contract) return
    if (!confirm('Archive this contract?')) return
    busy = true
    try {
      await api.archiveContract(contract.id)
      await refreshContracts()
      onBack()
    } finally {
      busy = false
    }
  }
</script>

<section class="panel">
  <button onclick={onBack}>&larr; Back to contracts</button>

  {#if !contract}
    <p>This contract no longer exists.</p>
  {:else}
    <h2>{contract.name}</h2>
    <dl>
      <dt>Client</dt>
      <dd>{client?.name ?? `#${contract.clientId}`}</dd>
      <dt>Current rate</dt>
      <dd>{contract.currentRate ?? '—'} {contract.currency} / hr</dd>
    </dl>

    <div class="field">
      <label for="edit-name">Name</label>
      <input id="edit-name" bind:value={editName} />
    </div>
    <div class="field">
      <label for="edit-id">ID</label>
      <input id="edit-id" bind:value={editExternalId} />
    </div>
    <div class="field">
      <label for="edit-currency">Currency</label>
      <input id="edit-currency" bind:value={editCurrency} style="width: 5rem" />
    </div>
    <div class="field">
      <label for="edit-start-date">Start date</label>
      <input id="edit-start-date" type="date" bind:value={editStartDate} />
    </div>
    <div class="field">
      <label for="edit-notes">Notes</label>
      <input id="edit-notes" bind:value={editNotes} style="flex: 1" />
    </div>
    <div class="field">
      <label for="edit-filename-date">Timesheet filename date</label>
      <select id="edit-filename-date" bind:value={editFilenameDate}>
        <option value="end">Last day of period (default)</option>
        <option value="start">First day of period</option>
      </select>
    </div>
    <div class="row">
      <button
        class:unchanged={!dirty}
        onclick={saveDetails}
        disabled={busy || !editName.trim() || !editCurrency.trim() || !dirty}>Save</button
      >
    </div>

    <h3>Rate</h3>
    <div class="field">
      <label for="new-rate">New rate</label>
      <input id="new-rate" type="number" step="0.01" bind:value={newRate} style="width: 7rem" />
      {contract.currency}
    </div>
    <div class="row">
      <button onclick={saveRate} disabled={busy || newRate === ''}>Update rate</button>
      <button onclick={archive} disabled={busy} class="danger">Archive contract</button>
    </div>
  {/if}
</section>

<style>
  .panel {
    margin-bottom: 2rem;
  }
  h3 {
    margin-top: 2rem;
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
    margin-bottom: 0.6rem;
  }
  .field label {
    width: 5.5rem;
  }
  .row {
    display: flex;
    gap: 0.5rem;
  }
  .danger {
    color: #b91c1c;
  }
  button.unchanged {
    background: #e5e5e5;
    color: #888;
    border-color: #d5d5d5;
  }
</style>
