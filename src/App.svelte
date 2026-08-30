<script lang="ts">
  import { onMount } from 'svelte'
  import { confirm } from '@tauri-apps/plugin-dialog'
  import { store, navGuard, refreshAll } from './lib/store.svelte'
  import { applyTheme } from './lib/theme'
  import ClientsPanel from './lib/ClientsPanel.svelte'
  import ClientDetail from './lib/ClientDetail.svelte'
  import ContractsPanel from './lib/ContractsPanel.svelte'
  import ContractDetail from './lib/ContractDetail.svelte'
  import TimerPanel from './lib/TimerPanel.svelte'
  import EntriesPanel from './lib/EntriesPanel.svelte'
  import EntryDetail from './lib/EntryDetail.svelte'
  import TrashPanel from './lib/TrashPanel.svelte'
  import UserPanel from './lib/UserPanel.svelte'
  import AdminPanel from './lib/AdminPanel.svelte'
  import AppearancePanel from './lib/AppearancePanel.svelte'
  import ReportsPanel from './lib/ReportsPanel.svelte'
  import QuickTimerPanel from './lib/QuickTimerPanel.svelte'
  import ImportPanel from './lib/ImportPanel.svelte'

  type View =
    | { name: 'timer' }
    | { name: 'quick-timer' }
    | { name: 'clients' }
    | { name: 'client-detail'; clientId: number }
    | { name: 'contracts' }
    | { name: 'contract-detail'; contractId: number }
    | { name: 'entries' }
    | { name: 'entry-detail'; entryId: number; returnTo?: 'entries' | 'timer' }
    | { name: 'admin' }
    | { name: 'trash' }
    | { name: 'user' }
    | { name: 'appearance' }
    | { name: 'reports' }
    | { name: 'import' }

  let view = $state<View>({ name: 'timer' })

  async function navigate(next: View) {
    if (navGuard.isDirty && !(await confirm('You have unsaved changes. Leave without saving?'))) return
    navGuard.isDirty = false
    view = next
  }

  onMount(async () => {
    await refreshAll()
    applyTheme(store.userProfile)
    const startPage = store.userProfile.defaultStartPage
    if (startPage && startPage !== 'timer') {
      view = { name: startPage }
    }
  })
</script>

<main>
  <header>
    <h1>TimeTracker</h1>
    <nav>
      <button class:active={view.name === 'quick-timer'} onclick={() => navigate({ name: 'quick-timer' })}
        >Quick Timer</button
      >
      <button class:active={view.name === 'timer'} onclick={() => navigate({ name: 'timer' })}>Timer</button>
      <button
        class:active={view.name === 'clients' || view.name === 'client-detail'}
        onclick={() => navigate({ name: 'clients' })}>Clients</button
      >
      <button
        class:active={view.name === 'contracts' || view.name === 'contract-detail'}
        onclick={() => navigate({ name: 'contracts' })}>Contracts</button
      >
      <button
        class:active={view.name === 'entries' || view.name === 'entry-detail'}
        onclick={() => navigate({ name: 'entries' })}>Entries</button
      >
      <button class:active={view.name === 'reports'} onclick={() => navigate({ name: 'reports' })}>Reports</button>
      <button
        class:active={view.name === 'admin' || view.name === 'user' || view.name === 'trash' || view.name === 'appearance'}
        onclick={() => navigate({ name: 'admin' })}>Admin</button
      >
    </nav>
  </header>

  {#if store.error}
    <div class="error">{store.error}</div>
  {/if}

  {#if view.name === 'timer'}
    <TimerPanel onSelectEntry={(entryId) => navigate({ name: 'entry-detail', entryId, returnTo: 'timer' })} />
  {:else if view.name === 'quick-timer'}
    <QuickTimerPanel onGoToTimer={() => navigate({ name: 'timer' })} />
  {:else if view.name === 'clients'}
    <ClientsPanel onSelectClient={(clientId) => navigate({ name: 'client-detail', clientId })} />
  {:else if view.name === 'client-detail'}
    <ClientDetail clientId={view.clientId} onBack={() => navigate({ name: 'clients' })} />
  {:else if view.name === 'contracts'}
    <ContractsPanel onSelectContract={(contractId) => navigate({ name: 'contract-detail', contractId })} />
  {:else if view.name === 'contract-detail'}
    <ContractDetail contractId={view.contractId} onBack={() => navigate({ name: 'contracts' })} />
  {:else if view.name === 'entries'}
    <EntriesPanel
      onSelectEntry={(entryId) => navigate({ name: 'entry-detail', entryId })}
      onGoToImport={() => navigate({ name: 'import' })}
    />
  {:else if view.name === 'entry-detail'}
    {@const returnTo = view.returnTo}
    <EntryDetail
      entryId={view.entryId}
      backLabel={returnTo === 'timer' ? 'Back to timer' : 'Back to entries'}
      onBack={() => navigate({ name: returnTo ?? 'entries' })}
    />
  {:else if view.name === 'admin'}
    <AdminPanel
      onGoToUser={() => navigate({ name: 'user' })}
      onGoToTrash={() => navigate({ name: 'trash' })}
      onGoToAppearance={() => navigate({ name: 'appearance' })}
    />
  {:else if view.name === 'trash'}
    <TrashPanel onBack={() => navigate({ name: 'admin' })} />
  {:else if view.name === 'user'}
    <UserPanel onBack={() => navigate({ name: 'admin' })} />
  {:else if view.name === 'appearance'}
    <AppearancePanel onBack={() => navigate({ name: 'admin' })} />
  {:else if view.name === 'reports'}
    <ReportsPanel />
  {:else if view.name === 'import'}
    <ImportPanel />
  {/if}
</main>

<style>
  main {
    max-width: 960px;
    margin: 0;
    padding: 1.5rem;
    font-family: system-ui, sans-serif;
  }
  header {
    display: flex;
    align-items: baseline;
    gap: 1.5rem;
    margin-bottom: 1.5rem;
  }
  h1 {
    margin: 0;
    font-size: 1.5rem;
  }
  nav {
    display: flex;
    gap: 0.5rem;
  }
  nav button {
    background: none;
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 0.3rem 0.7rem;
    cursor: pointer;
  }
  nav button.active {
    background: var(--border);
    font-weight: 600;
  }
  .error {
    background: #f8d7da;
    color: #58151c;
    border: 1px solid #f1aeb5;
    border-radius: 4px;
    padding: 0.5rem 0.75rem;
    margin-bottom: 1rem;
  }
</style>
