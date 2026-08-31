<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog'
  import { api, type ImportResult } from './api'
  import { store, activeClients, activeContracts } from './store.svelte'

  let { onGoToTemplate }: { onGoToTemplate: (templateId: number | null) => void } = $props()

  let clientId = $state<number | ''>('')
  let contractId = $state<number | ''>('')
  let filePaths = $state<string[]>([])
  let busy = $state(false)
  let result = $state<ImportResult | null>(null)
  let error = $state('')
  let templateId = $state<number | ''>('')
  let initializedTemplate = $state(false)

  let contractsForClient = $derived(clientId ? activeContracts().filter((c) => c.clientId === Number(clientId)) : [])

  $effect(() => {
    contractId = ''
  })

  // Pick the flagged default template once templates have loaded, but only the
  // first time — don't fight the user's own dropdown choice on every store update.
  $effect(() => {
    if (initializedTemplate || store.importTemplates.length === 0) return
    const def = store.importTemplates.find((t) => t.isDefault) ?? store.importTemplates[0]
    templateId = def.id
    initializedTemplate = true
  })

  let selectedTemplate = $derived(
    templateId ? store.importTemplates.find((t) => t.id === Number(templateId)) : undefined,
  )

  async function chooseFiles() {
    const selected = await open({
      multiple: true,
      filters: [{ name: 'Time entries', extensions: ['csv', 'xlsx', 'xls', 'xlsm'] }],
    })
    if (Array.isArray(selected)) {
      filePaths = selected
    } else if (typeof selected === 'string') {
      filePaths = [selected]
    }
  }

  function fileName(path: string): string {
    return path.split(/[\\/]/).pop() ?? path
  }

  async function runImport() {
    if (!contractId || !templateId || filePaths.length === 0) return
    busy = true
    error = ''
    result = null
    try {
      result = await api.importTimeEntries(Number(contractId), filePaths, Number(templateId))
      filePaths = []
    } catch (e) {
      error = String(e)
    } finally {
      busy = false
    }
  }
</script>

<section class="panel">
  <h2>Import</h2>
  <p class="muted">
    Import time entries from CSV or Excel files into a single contract. Every tab in an
    Excel workbook is imported. Dates accept YYYY-MM-DD or M/D/YYYY; times accept 24-hour
    or 12-hour with AM/PM. A row with a date but no times is skipped. Which column names
    to look for is controlled by the template selected below.
  </p>

  <div class="row">
    <select bind:value={clientId}>
      <option value="">Select client…</option>
      {#each activeClients() as client (client.id)}
        <option value={client.id}>{client.name}</option>
      {/each}
    </select>
    <select bind:value={contractId} disabled={!clientId}>
      <option value="">Select contract…</option>
      {#each contractsForClient as contract (contract.id)}
        <option value={contract.id}>{contract.name}</option>
      {/each}
    </select>
  </div>

  <div class="row">
    <label for="template-select">Template</label>
    <select id="template-select" bind:value={templateId}>
      {#each store.importTemplates as t (t.id)}
        <option value={t.id}>{t.name}{t.isDefault ? ' (default)' : ''}</option>
      {/each}
    </select>
    <button type="button" onclick={() => onGoToTemplate(null)}>Add…</button>
    <button type="button" onclick={() => onGoToTemplate(Number(templateId))} disabled={!templateId}>Edit…</button>
  </div>
  {#if selectedTemplate}
    <p class="muted template-summary">
      Columns: <strong>{selectedTemplate.dateColumn}</strong>, <strong>{selectedTemplate.startColumn}</strong>,
      <strong>{selectedTemplate.endColumn}</strong>{#if selectedTemplate.categoryColumn}, <strong
          >{selectedTemplate.categoryColumn}</strong
        > (category){/if}{#if selectedTemplate.notesColumn}, <strong>{selectedTemplate.notesColumn}</strong> (notes)
      {/if}{#if selectedTemplate.notes}<br />{selectedTemplate.notes}{/if}
    </p>
  {/if}

  <div class="row">
    <button onclick={chooseFiles}>Choose file(s)…</button>
    {#if filePaths.length > 0}
      <span class="muted">{filePaths.length} file{filePaths.length > 1 ? 's' : ''} selected</span>
    {/if}
  </div>

  {#if filePaths.length > 0}
    <ul class="file-list">
      {#each filePaths as path}
        <li>{fileName(path)}</li>
      {/each}
    </ul>
  {/if}

  <button onclick={runImport} disabled={busy || !contractId || !templateId || filePaths.length === 0}>Import</button>

  {#if error}
    <div class="error">{error}</div>
  {/if}

  {#if result}
    <div class="result">
      <p><strong>{result.imported}</strong> imported, <strong>{result.skipped}</strong> skipped.</p>
      {#if result.errors.length > 0}
        <ul class="errors">
          {#each result.errors as err}
            <li>{err}</li>
          {/each}
        </ul>
      {/if}
    </div>
  {/if}
</section>

<style>
  .panel {
    margin-bottom: 2rem;
    max-width: 40rem;
  }
  .muted {
    color: #666;
    font-size: 0.9rem;
  }
  .row {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    margin-bottom: 0.75rem;
  }
  .file-list {
    margin: 0 0 0.75rem;
    padding-left: 1.2rem;
    font-size: 0.9rem;
  }
  .error {
    background: #f8d7da;
    color: #58151c;
    border: 1px solid #f1aeb5;
    border-radius: 4px;
    padding: 0.5rem 0.75rem;
    margin-top: 0.75rem;
  }
  .result {
    margin-top: 1rem;
  }
  .errors {
    color: #b91c1c;
    font-size: 0.85rem;
  }
</style>
