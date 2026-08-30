<script lang="ts">
  import { api } from './api'
  import { store, activeClients, activeContracts, refreshActiveTimers, refreshEntries } from './store.svelte'
  import { formatDateTime } from './dateUtils'

  let { onGoToTimer }: { onGoToTimer: () => void } = $props()

  let busyContractId = $state<number | null>(null)
  let errorsByClient = $state<Record<number, string>>({})

  function activeTimersFor(contractId: number) {
    return store.activeTimers.filter((t) => t.contractId === contractId)
  }

  let clientsWithContracts = $derived(
    activeClients()
      .map((client) => ({
        client,
        contracts: activeContracts().filter((c) => c.clientId === client.id),
      }))
      .filter((entry) => entry.contracts.length > 0)
      .sort((a, b) => a.client.name.localeCompare(b.client.name)),
  )

  async function start(contractId: number, clientId: number, defaultTrackingCodeId: number | null) {
    busyContractId = contractId
    delete errorsByClient[clientId]
    try {
      await api.startTimer(contractId, undefined, defaultTrackingCodeId ?? undefined)
      await refreshActiveTimers()
    } catch (e) {
      errorsByClient[clientId] = String(e)
    } finally {
      busyContractId = null
    }
  }

  async function stop(entryId: number, contractId: number, clientId: number) {
    busyContractId = contractId
    delete errorsByClient[clientId]
    try {
      await api.stopTimer(entryId)
      await Promise.all([refreshActiveTimers(), refreshEntries()])
    } catch (e) {
      errorsByClient[clientId] = String(e)
    } finally {
      busyContractId = null
    }
  }

  function toggle(contractId: number, clientId: number, defaultTrackingCodeId: number | null) {
    const running = activeTimersFor(contractId)[0]
    if (running) {
      stop(running.id, contractId, clientId)
    } else {
      start(contractId, clientId, defaultTrackingCodeId)
    }
  }
</script>

<section class="panel">
  <h2>Quick Timer</h2>

  {#if store.activeTimers.length > 0}
    <div class="banner">
      {store.activeTimers.length} timer{store.activeTimers.length > 1 ? 's' : ''} running —
      <button class="link" onclick={onGoToTimer}>view/stop on the Timer page</button>
    </div>
  {/if}

  {#if clientsWithContracts.length === 0}
    <p class="muted">No active contracts yet — add a client and contract first.</p>
  {/if}

  {#each clientsWithContracts as { client, contracts } (client.id)}
    <div class="client-block">
      <h3>{client.name}</h3>
      {#if errorsByClient[client.id]}
        <p class="error">{errorsByClient[client.id]}</p>
      {/if}
      <div class="contracts">
        {#each contracts as contract (contract.id)}
          {@const runningTimer = activeTimersFor(contract.id)[0]}
          <div class="contract-row">
            <button
              class="contract-btn"
              class:stop-btn={!!runningTimer}
              onclick={() => toggle(contract.id, client.id, client.defaultTrackingCodeId)}
              disabled={busyContractId === contract.id}
            >
              {runningTimer ? '■' : '▶'} {contract.name}
            </button>
            {#if runningTimer}
              <span class="running">started {formatDateTime(runningTimer.startedAt)}</span>
            {/if}
          </div>
        {/each}
      </div>
    </div>
  {/each}
</section>

<style>
  .panel {
    margin-bottom: 2rem;
  }
  .banner {
    background: #d1e7dd;
    color: #0a3622;
    border: 1px solid #a3cfbb;
    border-radius: 4px;
    padding: 0.5rem 0.75rem;
    margin-bottom: 1rem;
  }
  .link {
    background: none;
    border: none;
    padding: 0;
    color: inherit;
    text-decoration: underline;
    cursor: pointer;
    font: inherit;
  }
  .client-block {
    margin-bottom: 1.25rem;
  }
  .client-block h3 {
    margin: 0 0 0.4rem;
    font-size: 1rem;
  }
  .contracts {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .contract-row {
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }
  .contract-btn {
    padding: 0.5rem 0.9rem;
    text-align: left;
  }
  .stop-btn {
    background: #b91c1c;
    color: #fff;
    border-color: #b91c1c;
  }
  .running {
    color: var(--text);
    opacity: 0.7;
    font-size: 0.85rem;
  }
  .muted {
    color: #666;
    font-size: 0.9rem;
  }
  .error {
    color: #b91c1c;
    font-size: 0.85rem;
    margin: 0 0 0.4rem;
  }
</style>
