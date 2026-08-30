<script lang="ts">
  import { onMount } from 'svelte'
  import { api } from './api'
  import {
    store,
    contractLabel,
    refreshDeletedEntries,
    refreshEntries,
    refreshContracts,
    refreshClients,
    refreshArchivedTrackingCodes,
  } from './store.svelte'
  import { formatDateTime, formatDuration } from './dateUtils'

  let busy = $state(false)
  let purgeMessage = $state('')
  let purgeError = $state('')

  type TrashType = 'entries' | 'contracts' | 'clients' | 'categories'
  let selectedType = $state<TrashType | null>(null)

  type FilterPeriod = 'day' | 'week' | 'month' | 'days' | 'all'
  let filterPeriod = $state<FilterPeriod>('all')
  let customDays = $state(14)

  const PERIOD_DAYS: Record<Exclude<FilterPeriod, 'days' | 'all'>, number> = {
    day: 1,
    week: 7,
    month: 30,
  }

  let cutoffMs = $derived.by(() => {
    if (filterPeriod === 'all') return null
    const days = filterPeriod === 'days' ? customDays : PERIOD_DAYS[filterPeriod]
    return Date.now() - days * 24 * 60 * 60 * 1000
  })

  let cutoffIso = $derived(cutoffMs === null ? undefined : new Date(cutoffMs).toISOString())

  function withinFilter(timestamp: string | null): boolean {
    if (cutoffMs === null) return true
    if (!timestamp) return false
    return new Date(timestamp).getTime() >= cutoffMs
  }

  let filteredDeletedEntries = $derived(store.deletedEntries.filter((e) => withinFilter(e.deletedAt)))
  let archivedContracts = $derived(store.contracts.filter((c) => c.archivedAt && withinFilter(c.archivedAt)))
  let archivedClients = $derived(store.clients.filter((c) => c.archivedAt && withinFilter(c.archivedAt)))
  let filteredArchivedTrackingCodes = $derived(store.archivedTrackingCodes.filter((tc) => withinFilter(tc.archivedAt)))

  function clientName(clientId: number): string {
    return store.clients.find((c) => c.id === clientId)?.name ?? `#${clientId}`
  }

  onMount(() => {
    refreshDeletedEntries()
    refreshArchivedTrackingCodes()
  })

  async function restoreEntry(entryId: number) {
    busy = true
    try {
      await api.restoreEntry(entryId)
      await Promise.all([refreshDeletedEntries(), refreshEntries()])
    } finally {
      busy = false
    }
  }

  async function restoreContract(contractId: number) {
    busy = true
    try {
      await api.restoreContract(contractId)
      await refreshContracts()
    } finally {
      busy = false
    }
  }

  async function restoreClient(clientId: number) {
    busy = true
    try {
      await api.restoreClient(clientId)
      await refreshClients()
    } finally {
      busy = false
    }
  }

  async function restoreTrackingCode(trackingCodeId: number) {
    busy = true
    try {
      await api.restoreTrackingCode(trackingCodeId)
      await refreshArchivedTrackingCodes()
    } finally {
      busy = false
    }
  }

  async function purge(
    label: string,
    count: number,
    action: () => Promise<{ count: number; backupPath: string; blocked: string[] }>,
    refresh: () => Promise<void>,
  ) {
    if (count === 0) return
    if (
      !confirm(
        `Permanently delete ${count} ${label}? A CSV backup will be written to your output folder first. This cannot be undone.`,
      )
    )
      return
    busy = true
    purgeMessage = ''
    purgeError = ''
    try {
      const result = await action()
      purgeMessage = `Backed up and purged ${result.count} ${label} — backup saved to ${result.backupPath}`
      if (result.blocked.length > 0) {
        purgeMessage += ` — ${result.blocked.length} skipped: ${result.blocked.join('; ')}`
      }
      await refresh()
    } catch (e) {
      purgeError = String(e)
    } finally {
      busy = false
    }
  }

  const purgeEntries = () =>
    purge('deleted entries', filteredDeletedEntries.length, () => api.purgeDeletedEntries(cutoffIso), refreshDeletedEntries)
  const purgeContracts = () =>
    purge('archived contracts', archivedContracts.length, () => api.purgeArchivedContracts(cutoffIso), refreshContracts)
  const purgeClients = () =>
    purge('archived clients', archivedClients.length, () => api.purgeArchivedClients(cutoffIso), refreshClients)
  const purgeCategories = () =>
    purge('archived categories', filteredArchivedTrackingCodes.length, () => api.purgeArchivedCategories(cutoffIso), refreshArchivedTrackingCodes)

  function openType(type: TrashType) {
    selectedType = type
    purgeMessage = ''
    purgeError = ''
  }

  const TYPE_LABELS: Record<TrashType, string> = {
    entries: 'Deleted Time Entries',
    contracts: 'Archived Contracts',
    clients: 'Archived Clients',
    categories: 'Archived Categories',
  }
</script>

<section class="panel">
  <h2>Trash</h2>

  <div class="row">
    <label for="filter-period">Show</label>
    <select id="filter-period" bind:value={filterPeriod}>
      <option value="day">Last day</option>
      <option value="week">Last week</option>
      <option value="month">Last month</option>
      <option value="days">Last N days…</option>
      <option value="all">All</option>
    </select>
    {#if filterPeriod === 'days'}
      <input type="number" min="1" bind:value={customDays} style="width: 5rem" />
      <span>days</span>
    {/if}
  </div>

  {#if purgeMessage}
    <p class="success">{purgeMessage}</p>
  {/if}
  {#if purgeError}
    <p class="error">{purgeError}</p>
  {/if}

  {#if selectedType === null}
    <ul class="type-list">
      <li>
        <button class="type-row" onclick={() => openType('entries')}>
          <span>{TYPE_LABELS.entries}</span>
          <span class="count">{filteredDeletedEntries.length}</span>
        </button>
      </li>
      <li>
        <button class="type-row" onclick={() => openType('contracts')}>
          <span>{TYPE_LABELS.contracts}</span>
          <span class="count">{archivedContracts.length}</span>
        </button>
      </li>
      <li>
        <button class="type-row" onclick={() => openType('clients')}>
          <span>{TYPE_LABELS.clients}</span>
          <span class="count">{archivedClients.length}</span>
        </button>
      </li>
      <li>
        <button class="type-row" onclick={() => openType('categories')}>
          <span>{TYPE_LABELS.categories}</span>
          <span class="count">{filteredArchivedTrackingCodes.length}</span>
        </button>
      </li>
    </ul>
  {:else}
    <button class="back" onclick={() => (selectedType = null)}>&larr; Back to Trash</button>

    {#if selectedType === 'entries'}
      <div class="section-head">
        <h3>{TYPE_LABELS.entries}</h3>
        <button onclick={purgeEntries} disabled={busy || filteredDeletedEntries.length === 0} class="danger"
          >Purge shown ({filteredDeletedEntries.length})</button
        >
      </div>
      {#if filteredDeletedEntries.length === 0}
        <p class="muted">No deleted entries in this range.</p>
      {:else}
        <table>
          <thead>
            <tr>
              <th>Contract</th>
              <th>Start</th>
              <th>End</th>
              <th>Duration</th>
              <th>Notes</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {#each filteredDeletedEntries as entry (entry.id)}
              <tr>
                <td>{contractLabel(entry.contractId)}</td>
                <td>{formatDateTime(entry.startedAt)}</td>
                <td>{entry.endedAt ? formatDateTime(entry.endedAt) : '—'}</td>
                <td>{formatDuration(entry.durationSecs)}</td>
                <td>{entry.notes ?? ''}</td>
                <td><button onclick={() => restoreEntry(entry.id)} disabled={busy}>Restore</button></td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    {:else if selectedType === 'contracts'}
      <div class="section-head">
        <h3>{TYPE_LABELS.contracts}</h3>
        <button onclick={purgeContracts} disabled={busy || archivedContracts.length === 0} class="danger"
          >Purge shown ({archivedContracts.length})</button
        >
      </div>
      {#if archivedContracts.length === 0}
        <p class="muted">No archived contracts in this range.</p>
      {:else}
        <table>
          <thead>
            <tr>
              <th>Client</th>
              <th>Contract</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {#each archivedContracts as contract (contract.id)}
              <tr>
                <td>{clientName(contract.clientId)}</td>
                <td>{contract.name}</td>
                <td><button onclick={() => restoreContract(contract.id)} disabled={busy}>Restore</button></td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    {:else if selectedType === 'clients'}
      <div class="section-head">
        <h3>{TYPE_LABELS.clients}</h3>
        <button onclick={purgeClients} disabled={busy || archivedClients.length === 0} class="danger"
          >Purge shown ({archivedClients.length})</button
        >
      </div>
      {#if archivedClients.length === 0}
        <p class="muted">No archived clients in this range.</p>
      {:else}
        <table>
          <thead>
            <tr>
              <th>Client</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {#each archivedClients as client (client.id)}
              <tr>
                <td>{client.name}</td>
                <td><button onclick={() => restoreClient(client.id)} disabled={busy}>Restore</button></td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    {:else if selectedType === 'categories'}
      <div class="section-head">
        <h3>{TYPE_LABELS.categories}</h3>
        <button onclick={purgeCategories} disabled={busy || filteredArchivedTrackingCodes.length === 0} class="danger"
          >Purge shown ({filteredArchivedTrackingCodes.length})</button
        >
      </div>
      {#if filteredArchivedTrackingCodes.length === 0}
        <p class="muted">No archived categories in this range.</p>
      {:else}
        <table>
          <thead>
            <tr>
              <th>Client</th>
              <th>Category</th>
              <th>Description</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {#each filteredArchivedTrackingCodes as tc (tc.id)}
              <tr>
                <td>{clientName(tc.clientId)}</td>
                <td>{tc.code}</td>
                <td>{tc.description ?? ''}</td>
                <td><button onclick={() => restoreTrackingCode(tc.id)} disabled={busy}>Restore</button></td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    {/if}
  {/if}
</section>

<style>
  .panel {
    margin-bottom: 2rem;
  }
  h3 {
    margin: 0;
  }
  .section-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 1rem;
    margin-bottom: 0.35rem;
  }
  .row {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    margin-bottom: 0.5rem;
  }
  .type-list {
    list-style: none;
    margin: 0.5rem 0 0;
    padding: 0;
  }
  .type-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    background: none;
    border: 1px solid var(--border, #ddd);
    border-radius: 4px;
    padding: 0.6rem 0.9rem;
    margin-bottom: 0.5rem;
    font-size: 1rem;
    cursor: pointer;
    text-align: left;
  }
  .type-row:hover {
    background: rgba(128, 128, 128, 0.1);
  }
  .type-row .count {
    font-weight: 600;
    color: #666;
  }
  .back {
    background: none;
    border: none;
    padding: 0;
    margin-bottom: 0.75rem;
    cursor: pointer;
    color: inherit;
    font-size: 0.95rem;
  }
  .back:hover {
    text-decoration: underline;
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
  .muted {
    color: #666;
    font-size: 0.9rem;
  }
  .danger {
    color: #b91c1c;
  }
  .success {
    background: #d1e7dd;
    color: #0a3622;
    border: 1px solid #a3cfbb;
    border-radius: 4px;
    padding: 0.5rem 0.75rem;
    margin-bottom: 0.75rem;
  }
  .error {
    background: #f8d7da;
    color: #58151c;
    border: 1px solid #f1aeb5;
    border-radius: 4px;
    padding: 0.5rem 0.75rem;
    margin-bottom: 0.75rem;
  }
</style>
