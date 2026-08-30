<script lang="ts">
  import { api } from './api'
  import { store, activeClients, refreshClients } from './store.svelte'
  import { capitalize } from './dateUtils'

  let { onSelectClient }: { onSelectClient: (clientId: number) => void } = $props()

  let clientName = $state('')
  let busy = $state(false)

  async function addClient() {
    if (!clientName.trim()) return
    busy = true
    try {
      await api.createClient(clientName.trim())
      clientName = ''
      await refreshClients()
    } finally {
      busy = false
    }
  }
</script>

<section class="panel">
  <h2>Clients</h2>

  <div class="row">
    <input placeholder="New client name" bind:value={clientName} onkeydown={(e) => e.key === 'Enter' && addClient()} />
    <button onclick={addClient} disabled={busy}>Add client</button>
  </div>

  <table>
    <thead>
      <tr>
        <th>Client</th>
        <th>Prime</th>
        <th>Sub</th>
        <th>Week Start</th>
        <th>Min Inc</th>
        <th>Notes</th>
      </tr>
    </thead>
    <tbody>
      {#each activeClients() as client (client.id)}
        <tr class="clickable" onclick={() => onSelectClient(client.id)}>
          <td>{client.name}</td>
          <td>{client.prime ?? ''}</td>
          <td>{client.sub ?? ''}</td>
          <td>{capitalize(client.weekStart)}</td>
          <td>{client.minimumIncrementMinutes ?? ''}</td>
          <td>{client.notes ?? ''}</td>
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
  tr.clickable {
    cursor: pointer;
  }
  tr.clickable:hover {
    background: rgba(128, 128, 128, 0.1);
  }
</style>
