<script lang="ts">
  import { api } from './api'
  import {
    store,
    contractLabel,
    clientIdForContract,
    activeContracts,
    isContractArchived,
    loadTrackingCodesForClient,
    refreshActiveTimers,
    refreshEntries,
  } from './store.svelte'
  import { formatDateTime } from './dateUtils'

  let { onSelectEntry }: { onSelectEntry: (entryId: number) => void } = $props()

  let selectedContractId = $state<number | ''>('')
  let notes = $state('')
  let trackingCodeId = $state<number | ''>('')
  let busy = $state(false)
  let now = $state(Date.now())

  $effect(() => {
    const id = setInterval(() => {
      now = Date.now()
    }, 1000)
    return () => clearInterval(id)
  })

  let availableCodes = $derived(
    selectedContractId ? (store.trackingCodesByClient[clientIdForContract(Number(selectedContractId)) ?? -1] ?? []) : [],
  )

  $effect(() => {
    trackingCodeId = ''
    if (!selectedContractId) return
    const clientId = clientIdForContract(Number(selectedContractId))
    if (clientId !== undefined) loadTrackingCodesForClient(clientId)
  })

  function elapsed(startedAt: string): string {
    const secs = Math.max(0, Math.floor((now - new Date(startedAt).getTime()) / 1000))
    const h = Math.floor(secs / 3600)
    const m = Math.floor((secs % 3600) / 60)
    const s = secs % 60
    return `${String(h).padStart(2, '0')}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`
  }

  async function start() {
    if (!selectedContractId) return
    busy = true
    try {
      await api.startTimer(
        Number(selectedContractId),
        notes.trim() || undefined,
        trackingCodeId ? Number(trackingCodeId) : undefined,
      )
      notes = ''
      trackingCodeId = ''
      await Promise.all([refreshActiveTimers(), refreshEntries()])
    } finally {
      busy = false
    }
  }

  async function stop(event: MouseEvent, entryId: number) {
    event.stopPropagation()
    busy = true
    try {
      await api.stopTimer(entryId)
      await Promise.all([refreshActiveTimers(), refreshEntries()])
    } finally {
      busy = false
    }
  }
</script>

<section class="panel">
  <h2>Timer</h2>

  {#if store.activeTimers.length > 0}
    <div class="warning">
      {store.activeTimers.length} timer{store.activeTimers.length > 1 ? 's' : ''} already running —
      starting another is fine if this work benefits multiple contracts at once.
    </div>
  {/if}

  <div class="row">
    <select bind:value={selectedContractId}>
      <option value="">Select contract…</option>
      {#each activeContracts() as contract (contract.id)}
        <option value={contract.id}>{contractLabel(contract.id)}</option>
      {/each}
    </select>
    {#if availableCodes.length > 0}
      <select bind:value={trackingCodeId}>
        <option value="">No category</option>
        {#each availableCodes as tc (tc.id)}
          <option value={tc.id}>{tc.code}</option>
        {/each}
      </select>
    {/if}
    <input placeholder="Notes (optional)" bind:value={notes} style="flex: 1" />
    <button onclick={start} disabled={busy || !selectedContractId}>Start timer</button>
  </div>

  {#if store.activeTimers.length > 0}
    <table>
      <thead>
        <tr>
          <th>Contract</th>
          <th>Category</th>
          <th>Started</th>
          <th>Elapsed</th>
          <th>Notes</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        {#each store.activeTimers as timer (timer.id)}
          <tr class="clickable" onclick={() => onSelectEntry(timer.id)}>
            <td class:archived={isContractArchived(timer.contractId)}>{contractLabel(timer.contractId)}</td>
            <td>{timer.trackingCode ?? ''}</td>
            <td>{formatDateTime(timer.startedAt)}</td>
            <td>{elapsed(timer.startedAt)}</td>
            <td>{timer.notes ?? ''}</td>
            <td><button onclick={(e) => stop(e, timer.id)} disabled={busy}>Stop</button></td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</section>

<style>
  .panel {
    margin-bottom: 2rem;
  }
  .row {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 0.75rem;
    align-items: center;
  }
  .warning {
    background: #fff3cd;
    color: #664d03;
    border: 1px solid #ffe69c;
    border-radius: 4px;
    padding: 0.5rem 0.75rem;
    margin-bottom: 0.75rem;
  }
  table {
    width: 100%;
    border-collapse: collapse;
  }
  th,
  td {
    text-align: left;
    padding: 0.35rem 0.5rem;
    border-bottom: 1px solid #ddd;
  }
  .archived {
    text-decoration: line-through;
    color: #888;
  }
  tr.clickable {
    cursor: pointer;
  }
  tr.clickable:hover {
    background: rgba(128, 128, 128, 0.1);
  }
</style>
