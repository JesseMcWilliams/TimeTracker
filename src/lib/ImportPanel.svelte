<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog'
  import { api, type ImportPreviewSheet, type ImportResult } from './api'
  import { store, activeClients, activeContracts } from './store.svelte'

  let { onGoToTemplate }: { onGoToTemplate: (templateId: number | null) => void } = $props()

  let clientId = $state<number | ''>('')
  let contractId = $state<number | ''>('')
  let filePaths = $state<string[]>([])
  let busy = $state(false)
  let result = $state<ImportResult | null>(null)
  let error = $state('')
  let templateId = $state<number | ''>('')
  let previewing = $state(false)
  let preview = $state<ImportPreviewSheet[] | null>(null)
  let previewError = $state('')

  let contractsForClient = $derived(clientId ? activeContracts().filter((c) => c.clientId === Number(clientId)) : [])

  $effect(() => {
    contractId = ''
  })

  // Keep templateId pointing at a real template at all times: pick the flagged default
  // (or the first one) whenever there's no selection yet, or the previously-selected
  // template has just been deleted — but otherwise leave the user's own choice alone.
  $effect(() => {
    if (store.importTemplates.length === 0) {
      templateId = ''
      return
    }
    if (templateId !== '' && store.importTemplates.some((t) => t.id === Number(templateId))) return
    const def = store.importTemplates.find((t) => t.isDefault) ?? store.importTemplates[0]
    templateId = def.id
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
    preview = null
    previewError = ''
  }

  function fileName(path: string): string {
    return path.split(/[\\/]/).pop() ?? path
  }

  function onTemplateChange() {
    preview = null
    previewError = ''
  }

  async function runPreview() {
    if (!templateId || filePaths.length === 0) return
    previewing = true
    previewError = ''
    preview = null
    try {
      preview = await api.previewImport(filePaths, Number(templateId))
    } catch (e) {
      previewError = String(e)
    } finally {
      previewing = false
    }
  }

  async function runImport() {
    if (!contractId || !templateId || filePaths.length === 0) return
    busy = true
    error = ''
    result = null
    try {
      result = await api.importTimeEntries(Number(contractId), filePaths, Number(templateId))
      filePaths = []
      preview = null
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
    <select id="template-select" bind:value={templateId} onchange={onTemplateChange}>
      {#each store.importTemplates as t (t.id)}
        <option value={t.id}>{t.name}{t.isDefault ? ' (default)' : ''}</option>
      {/each}
    </select>
    <button type="button" onclick={() => onGoToTemplate(null)}>Add…</button>
    <button type="button" onclick={() => onGoToTemplate(Number(templateId))} disabled={!templateId}>Edit…</button>
  </div>
  {#if selectedTemplate}
    <p class="muted template-summary">
      Columns: <strong>{selectedTemplate.dateColumns}</strong>, <strong>{selectedTemplate.startColumns}</strong>,
      <strong>{selectedTemplate.endColumns}</strong>{#if selectedTemplate.categoryColumns}, <strong
          >{selectedTemplate.categoryColumns}</strong
        > (category){/if}{#if selectedTemplate.notesColumns}, <strong>{selectedTemplate.notesColumns}</strong> (notes)
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

  <div class="row">
    <button onclick={runImport} disabled={busy || !contractId || !templateId || filePaths.length === 0}
      >Import</button
    >
    <button
      type="button"
      onclick={runPreview}
      disabled={previewing || !templateId || filePaths.length === 0}>Preview…</button
    >
  </div>

  {#if previewError}
    <div class="error">{previewError}</div>
  {/if}

  {#if preview}
    <div class="preview">
      {#each preview as sheet}
        <div class="preview-sheet">
          <p class="preview-label">{sheet.label}</p>
          {#if sheet.error}
            <p class="error inline">{sheet.error}</p>
          {:else}
            <p class="muted">
              Date → <strong>{sheet.matchedDateColumn}</strong>, Start → <strong>{sheet.matchedStartColumn}</strong>,
              End → <strong>{sheet.matchedEndColumn}</strong>, Category →
              <strong>{sheet.matchedCategoryColumn ?? '(not found)'}</strong>, Notes →
              <strong>{sheet.matchedNotesColumn ?? '(not found)'}</strong>
            </p>
            {#each sheet.warnings as w}
              <p class="warning inline">{w}</p>
            {/each}
            {#if sheet.sampleRows.length > 0}
              <table class="preview-table">
                <thead>
                  <tr>
                    <th>Date</th>
                    <th>Start</th>
                    <th>End</th>
                    <th>Category</th>
                    <th>Notes</th>
                  </tr>
                </thead>
                <tbody>
                  {#each sheet.sampleRows as row}
                    <tr>
                      <td>{row.date}</td>
                      <td>{row.start}</td>
                      <td>{row.end}</td>
                      <td>{row.category}</td>
                      <td>{row.notes}</td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            {/if}
          {/if}
        </div>
      {/each}
    </div>
  {/if}

  {#if error}
    <div class="error">{error}</div>
  {/if}

  {#if result}
    <div class="result">
      <p><strong>{result.imported}</strong> imported, <strong>{result.skipped}</strong> skipped.</p>
      {#if result.warnings.length > 0}
        <ul class="warnings">
          {#each result.warnings as w}
            <li>{w}</li>
          {/each}
        </ul>
      {/if}
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
  .error.inline {
    margin-top: 0.25rem;
    padding: 0.35rem 0.6rem;
    font-size: 0.85rem;
  }
  .warning {
    background: #fff3cd;
    color: #664d03;
    border: 1px solid #ffe69c;
    border-radius: 4px;
    padding: 0.35rem 0.6rem;
    font-size: 0.85rem;
    margin-top: 0.25rem;
  }
  .result {
    margin-top: 1rem;
  }
  .errors {
    color: #b91c1c;
    font-size: 0.85rem;
  }
  .warnings {
    color: #8a6500;
    font-size: 0.85rem;
  }
  .preview {
    margin-top: 0.75rem;
    border: 1px solid var(--border, #ddd);
    border-radius: 4px;
    padding: 0.75rem;
  }
  .preview-sheet + .preview-sheet {
    margin-top: 0.75rem;
    padding-top: 0.75rem;
    border-top: 1px solid var(--border, #ddd);
  }
  .preview-label {
    font-weight: 600;
    margin: 0 0 0.25rem;
  }
  .preview-table {
    width: 100%;
    border-collapse: collapse;
    margin-top: 0.5rem;
    font-size: 0.85rem;
  }
  .preview-table th,
  .preview-table td {
    text-align: left;
    padding: 0.2rem 0.5rem;
    border-bottom: 1px solid var(--border, #eee);
  }
</style>
