<script lang="ts">
  import { store } from './store.svelte'

  let {
    onBack,
    onGoToEmailTemplate,
    onGoToImportTemplate,
  }: {
    onBack: () => void
    onGoToEmailTemplate: (templateId: number | null) => void
    onGoToImportTemplate: (templateId: number | null) => void
  } = $props()

  let emailTemplates = $derived([...store.emailTemplates].sort((a, b) => a.name.localeCompare(b.name)))
  let importTemplates = $derived([...store.importTemplates].sort((a, b) => a.name.localeCompare(b.name)))
</script>

<section class="panel">
  <button onclick={onBack}>&larr; Back to Admin</button>

  <h2>Templates</h2>
  <p class="muted">
    The same templates reachable from Reports and Import, collected in one place. Editing one here
    is the same edit — there is only ever one copy of a template.
  </p>

  <div class="section-head">
    <h3>Email templates</h3>
    <button type="button" onclick={() => onGoToEmailTemplate(null)}>Add…</button>
  </div>
  <p class="muted">Build the subject and body of a "which contracts have no time logged" email.</p>
  {#if emailTemplates.length === 0}
    <p class="muted empty">No email templates yet.</p>
  {:else}
    <table>
      <thead>
        <tr>
          <th>Name</th>
          <th>Date range</th>
          <th>Recipient</th>
          <th>Description</th>
        </tr>
      </thead>
      <tbody>
        {#each emailTemplates as t (t.id)}
          <tr class="clickable" onclick={() => onGoToEmailTemplate(t.id)}>
            <td>{t.name}{t.isDefault ? ' (default)' : ''}</td>
            <td>{t.dateRange.replaceAll('_', ' ')}</td>
            <td>{t.email ?? '—'}</td>
            <td>{t.description ?? ''}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}

  <div class="section-head">
    <h3>Import templates</h3>
    <button type="button" onclick={() => onGoToImportTemplate(null)}>Add…</button>
  </div>
  <p class="muted">Map the columns of a spreadsheet you import time entries from.</p>
  {#if importTemplates.length === 0}
    <p class="muted empty">No import templates yet.</p>
  {:else}
    <table>
      <thead>
        <tr>
          <th>Name</th>
          <th>Date column</th>
          <th>Start column</th>
          <th>End column</th>
          <th>Notes</th>
        </tr>
      </thead>
      <tbody>
        {#each importTemplates as t (t.id)}
          <tr class="clickable" onclick={() => onGoToImportTemplate(t.id)}>
            <td>{t.name}{t.isDefault ? ' (default)' : ''}</td>
            <td>{t.dateColumns}</td>
            <td>{t.startColumns}</td>
            <td>{t.endColumns}</td>
            <td>{t.notes ?? ''}</td>
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
  .section-head {
    display: flex;
    align-items: baseline;
    gap: 0.75rem;
    margin-top: 2rem;
  }
  .section-head h3 {
    margin: 0;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    margin-top: 0.5rem;
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
  .muted {
    color: #666;
    font-size: 0.9rem;
  }
  .empty {
    margin-top: 0.5rem;
  }
</style>
