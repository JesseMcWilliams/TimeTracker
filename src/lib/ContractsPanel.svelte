<script lang="ts">
  import { api } from './api'
  import { store, activeClients, activeContracts, refreshContracts } from './store.svelte'

  let { onSelectContract }: { onSelectContract: (contractId: number) => void } = $props()

  let contractClientId = $state<number | ''>('')
  let contractName = $state('')
  let contractCurrency = $state('USD')
  let contractRate = $state<number | ''>('')
  let busy = $state(false)
  let showRate = $state(false)
  let filterText = $state('')

  type SortColumn = 'client' | 'contract' | 'currency' | 'rate'
  let sortColumn = $state<SortColumn>('client')
  let sortDir = $state<'asc' | 'desc'>('asc')

  function clientName(clientId: number): string {
    return store.clients.find((c) => c.id === clientId)?.name ?? `#${clientId}`
  }

  function sortBy(column: SortColumn) {
    if (sortColumn === column) {
      sortDir = sortDir === 'asc' ? 'desc' : 'asc'
    } else {
      sortColumn = column
      sortDir = 'asc'
    }
  }

  function sortIndicator(column: SortColumn): string {
    if (sortColumn !== column) return ''
    return sortDir === 'asc' ? ' ▲' : ' ▼'
  }

  let filteredContracts = $derived(
    activeContracts().filter((c) => {
      const needle = filterText.trim().toLowerCase()
      if (!needle) return true
      return clientName(c.clientId).toLowerCase().includes(needle) || c.name.toLowerCase().includes(needle)
    }),
  )

  let sortedContracts = $derived(
    [...filteredContracts].sort((a, b) => {
      let cmp: number
      switch (sortColumn) {
        case 'client':
          cmp = clientName(a.clientId).localeCompare(clientName(b.clientId)) || a.name.localeCompare(b.name)
          break
        case 'contract':
          cmp = a.name.localeCompare(b.name)
          break
        case 'currency':
          cmp = a.currency.localeCompare(b.currency)
          break
        case 'rate':
          cmp = (a.currentRate ?? -Infinity) - (b.currentRate ?? -Infinity)
          break
      }
      return sortDir === 'asc' ? cmp : -cmp
    }),
  )

  async function addContract() {
    if (!contractClientId || !contractName.trim() || contractRate === '') return
    busy = true
    try {
      await api.createContract(Number(contractClientId), contractName.trim(), contractCurrency, Number(contractRate))
      contractName = ''
      contractRate = ''
      await refreshContracts()
    } finally {
      busy = false
    }
  }
</script>

<section class="panel">
  <h2>Contracts</h2>

  {#if activeClients().length === 0}
    <p class="muted">Add a client on the Clients page first.</p>
  {/if}

  <div class="row">
    <select bind:value={contractClientId}>
      <option value="">Select client…</option>
      {#each activeClients() as client (client.id)}
        <option value={client.id}>{client.name}</option>
      {/each}
    </select>
    <input placeholder="Contract name" bind:value={contractName} />
    <input placeholder="Currency" bind:value={contractCurrency} style="width: 4.5rem" />
    <input placeholder="Hourly rate" type="number" step="0.01" bind:value={contractRate} style="width: 7rem" />
    <button onclick={addContract} disabled={busy}>Add contract</button>
  </div>

  <div class="row">
    <input placeholder="Filter by client or contract…" bind:value={filterText} style="flex: 1" />
    <label class="show-rate">
      <input type="checkbox" bind:checked={showRate} />
      Show rate
    </label>
  </div>

  <table>
    <thead>
      <tr>
        <th class="sortable" onclick={() => sortBy('client')}>Client{sortIndicator('client')}</th>
        <th class="sortable" onclick={() => sortBy('contract')}>Contract{sortIndicator('contract')}</th>
        <th class="sortable" onclick={() => sortBy('currency')}>Currency{sortIndicator('currency')}</th>
        {#if showRate}
          <th class="sortable" onclick={() => sortBy('rate')}>Rate{sortIndicator('rate')}</th>
        {/if}
        <th>Notes</th>
      </tr>
    </thead>
    <tbody>
      {#if sortedContracts.length === 0}
        <tr><td colspan={showRate ? 5 : 4} class="muted">No contracts match this filter.</td></tr>
      {/if}
      {#each sortedContracts as contract (contract.id)}
        <tr class="clickable" onclick={() => onSelectContract(contract.id)}>
          <td>{clientName(contract.clientId)}</td>
          <td>{contract.name}</td>
          <td>{contract.currency}</td>
          {#if showRate}
            <td>{contract.currentRate ?? '—'}</td>
          {/if}
          <td>{contract.notes ?? ''}</td>
        </tr>
      {/each}
    </tbody>
  </table>
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
  .show-rate {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin-bottom: 0.5rem;
    font-size: 0.9rem;
  }
  tr.clickable {
    cursor: pointer;
  }
  tr.clickable:hover {
    background: rgba(128, 128, 128, 0.1);
  }
  th.sortable {
    cursor: pointer;
    user-select: none;
  }
  th.sortable:hover {
    background: rgba(128, 128, 128, 0.1);
  }
</style>
