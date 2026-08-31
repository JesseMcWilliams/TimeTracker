<script lang="ts">
  import { confirm } from '@tauri-apps/plugin-dialog'
  import { api } from './api'
  import { store, navGuard, refreshImportTemplates } from './store.svelte'
  import { isDirty } from './dateUtils'

  let { templateId, onBack }: { templateId: number | null; onBack: (savedId?: number) => void } = $props()

  let template = $derived(templateId === null ? undefined : store.importTemplates.find((t) => t.id === templateId))

  const BLANK = {
    name: '',
    notes: '',
    isDefault: false,
    dateColumns: 'Date',
    startColumns: 'Start Time',
    endColumns: 'End Time',
    categoryColumns: 'Category',
    notesColumns: 'Notes',
  }

  let name = $state(BLANK.name)
  let notes = $state(BLANK.notes)
  let isDefaultField = $state(BLANK.isDefault)
  let dateColumns = $state(BLANK.dateColumns)
  let startColumns = $state(BLANK.startColumns)
  let endColumns = $state(BLANK.endColumns)
  let categoryColumns = $state(BLANK.categoryColumns)
  let notesColumns = $state(BLANK.notesColumns)
  let busy = $state(false)
  let deleteError = $state('')
  let initializedFor = $state<number | null | 'new'>(null)

  $effect(() => {
    const key = templateId === null ? 'new' : templateId
    if (initializedFor === key) return
    if (templateId === null) {
      name = BLANK.name
      notes = BLANK.notes
      isDefaultField = BLANK.isDefault
      dateColumns = BLANK.dateColumns
      startColumns = BLANK.startColumns
      endColumns = BLANK.endColumns
      categoryColumns = BLANK.categoryColumns
      notesColumns = BLANK.notesColumns
      initializedFor = 'new'
    } else if (template) {
      name = template.name
      notes = template.notes ?? ''
      isDefaultField = template.isDefault
      dateColumns = template.dateColumns
      startColumns = template.startColumns
      endColumns = template.endColumns
      categoryColumns = template.categoryColumns ?? ''
      notesColumns = template.notesColumns ?? ''
      initializedFor = templateId
    }
  })

  let dirty = $derived(
    templateId === null
      ? isDirty(
          { name, notes, isDefaultField, dateColumns, startColumns, endColumns, categoryColumns, notesColumns },
          BLANK,
        )
      : template
        ? isDirty(
            { name, notes, isDefaultField, dateColumns, startColumns, endColumns, categoryColumns, notesColumns },
            {
              name: template.name,
              notes: template.notes ?? '',
              isDefaultField: template.isDefault,
              dateColumns: template.dateColumns,
              startColumns: template.startColumns,
              endColumns: template.endColumns,
              categoryColumns: template.categoryColumns ?? '',
              notesColumns: template.notesColumns ?? '',
            },
          )
        : false,
  )

  $effect(() => {
    navGuard.isDirty = dirty
  })

  let canSave = $derived(
    name.trim() !== '' && dateColumns.trim() !== '' && startColumns.trim() !== '' && endColumns.trim() !== '',
  )
  let canDelete = $derived(templateId !== null && store.importTemplates.length > 1)

  async function save() {
    if (!canSave) return
    busy = true
    try {
      if (templateId === null) {
        const id = await api.createImportTemplate(
          name.trim(),
          isDefaultField,
          dateColumns.trim(),
          startColumns.trim(),
          endColumns.trim(),
          notes.trim() || undefined,
          categoryColumns.trim() || undefined,
          notesColumns.trim() || undefined,
        )
        await refreshImportTemplates()
        navGuard.isDirty = false
        onBack(id)
      } else {
        await api.updateImportTemplate(
          templateId,
          name.trim(),
          isDefaultField,
          dateColumns.trim(),
          startColumns.trim(),
          endColumns.trim(),
          notes.trim() || undefined,
          categoryColumns.trim() || undefined,
          notesColumns.trim() || undefined,
        )
        await refreshImportTemplates()
        navGuard.isDirty = false
        onBack(templateId)
      }
    } finally {
      busy = false
    }
  }

  async function remove() {
    if (templateId === null || !canDelete) return
    if (!(await confirm(`Delete the "${name}" import template? This can't be undone.`))) return
    busy = true
    deleteError = ''
    try {
      await api.deleteImportTemplate(templateId)
      await refreshImportTemplates()
      navGuard.isDirty = false
      onBack()
    } catch (e) {
      deleteError = String(e)
    } finally {
      busy = false
    }
  }
</script>

<section class="panel">
  <button onclick={() => onBack()}>&larr; Back to Import</button>

  <h2>{templateId === null ? 'Add Import Template' : 'Edit Import Template'}</h2>

  {#if templateId !== null && !template}
    <p>This template no longer exists.</p>
  {:else}
    <div class="field">
      <label for="tpl-name">Name</label>
      <input id="tpl-name" bind:value={name} style="flex: 1" />
    </div>
    <div class="field notes-row">
      <label for="tpl-notes">Notes</label>
      <textarea id="tpl-notes" class="notes-input" bind:value={notes} rows="2"></textarea>
    </div>
    <div class="field">
      <span class="field-spacer" aria-hidden="true"></span>
      <label class="checkbox-label">
        <input type="checkbox" bind:checked={isDefaultField} />
        Use as default template
      </label>
    </div>

    <h3>Column names</h3>
    <p class="muted">
      Case-insensitive exact match against your file's header row. Enter a comma-separated
      list to accept more than one header name for a field (e.g. "Start, Start Time") — the
      first one found in a given file wins. Category and Notes are optional — leave one
      blank to skip looking for that column entirely.
    </p>
    <div class="field">
      <label for="tpl-date">Date</label>
      <input id="tpl-date" bind:value={dateColumns} />
    </div>
    <div class="field">
      <label for="tpl-start">Start Time</label>
      <input id="tpl-start" bind:value={startColumns} />
    </div>
    <div class="field">
      <label for="tpl-end">End Time</label>
      <input id="tpl-end" bind:value={endColumns} />
    </div>
    <div class="field">
      <label for="tpl-category">Category</label>
      <input id="tpl-category" bind:value={categoryColumns} placeholder="(none)" />
    </div>
    <div class="field">
      <label for="tpl-notes-col">Notes</label>
      <input id="tpl-notes-col" bind:value={notesColumns} placeholder="(none)" />
    </div>

    {#if deleteError}
      <div class="error">{deleteError}</div>
    {/if}

    <div class="actions">
      <button class:unchanged={!dirty} onclick={save} disabled={busy || !dirty || !canSave}>Save</button>
      {#if templateId !== null}
        <button
          type="button"
          class="delete"
          onclick={remove}
          disabled={busy || !canDelete}
          title={canDelete ? '' : 'At least one import template must remain'}>Delete</button
        >
      {/if}
    </div>
  {/if}
</section>

<style>
  .panel {
    margin-bottom: 2rem;
    max-width: 32rem;
  }
  h3 {
    margin-top: 1.75rem;
  }
  .field {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.6rem;
  }
  .field label,
  .field-spacer {
    width: 7rem;
    flex-shrink: 0;
  }
  .field input {
    flex: 1;
  }
  .field.notes-row {
    align-items: flex-start;
  }
  .field.notes-row label {
    padding-top: 0.4rem;
  }
  .notes-input {
    flex: 1;
    width: 100%;
    box-sizing: border-box;
    resize: vertical;
    min-height: 2.5rem;
    height: clamp(2.5rem, 20vh, 12rem);
    font: inherit;
    padding: 0.4rem 0.5rem;
    border: 1px solid var(--border, #ccc);
    border-radius: 4px;
    background: var(--bg, #fff);
    color: var(--text, inherit);
  }
  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }
  .muted {
    color: #666;
    font-size: 0.85rem;
  }
  .actions {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    margin-top: 0.5rem;
  }
  button.unchanged {
    background: #e5e5e5;
    color: #888;
    border-color: #d5d5d5;
  }
  button.delete {
    color: #b91c1c;
    border-color: #f1aeb5;
  }
  .error {
    background: #f8d7da;
    color: #58151c;
    border: 1px solid #f1aeb5;
    border-radius: 4px;
    padding: 0.5rem 0.75rem;
    margin-top: 0.75rem;
  }
</style>
