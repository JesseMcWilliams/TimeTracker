<script lang="ts">
  import { api } from './api'
  import { store, navGuard, refreshImportTemplates } from './store.svelte'
  import { isDirty } from './dateUtils'

  let { templateId, onBack }: { templateId: number | null; onBack: (savedId?: number) => void } = $props()

  let template = $derived(templateId === null ? undefined : store.importTemplates.find((t) => t.id === templateId))

  const BLANK = {
    name: '',
    notes: '',
    isDefault: false,
    dateColumn: 'Date',
    startColumn: 'Start Time',
    endColumn: 'End Time',
    categoryColumn: 'Category',
    notesColumn: 'Notes',
  }

  let name = $state(BLANK.name)
  let notes = $state(BLANK.notes)
  let isDefaultField = $state(BLANK.isDefault)
  let dateColumn = $state(BLANK.dateColumn)
  let startColumn = $state(BLANK.startColumn)
  let endColumn = $state(BLANK.endColumn)
  let categoryColumn = $state(BLANK.categoryColumn)
  let notesColumn = $state(BLANK.notesColumn)
  let busy = $state(false)
  let initializedFor = $state<number | null | 'new'>(null)

  $effect(() => {
    const key = templateId === null ? 'new' : templateId
    if (initializedFor === key) return
    if (templateId === null) {
      name = BLANK.name
      notes = BLANK.notes
      isDefaultField = BLANK.isDefault
      dateColumn = BLANK.dateColumn
      startColumn = BLANK.startColumn
      endColumn = BLANK.endColumn
      categoryColumn = BLANK.categoryColumn
      notesColumn = BLANK.notesColumn
      initializedFor = 'new'
    } else if (template) {
      name = template.name
      notes = template.notes ?? ''
      isDefaultField = template.isDefault
      dateColumn = template.dateColumn
      startColumn = template.startColumn
      endColumn = template.endColumn
      categoryColumn = template.categoryColumn ?? ''
      notesColumn = template.notesColumn ?? ''
      initializedFor = templateId
    }
  })

  let dirty = $derived(
    templateId === null
      ? isDirty(
          { name, notes, isDefaultField, dateColumn, startColumn, endColumn, categoryColumn, notesColumn },
          BLANK,
        )
      : template
        ? isDirty(
            { name, notes, isDefaultField, dateColumn, startColumn, endColumn, categoryColumn, notesColumn },
            {
              name: template.name,
              notes: template.notes ?? '',
              isDefaultField: template.isDefault,
              dateColumn: template.dateColumn,
              startColumn: template.startColumn,
              endColumn: template.endColumn,
              categoryColumn: template.categoryColumn ?? '',
              notesColumn: template.notesColumn ?? '',
            },
          )
        : false,
  )

  $effect(() => {
    navGuard.isDirty = dirty
  })

  let canSave = $derived(name.trim() !== '' && dateColumn.trim() !== '' && startColumn.trim() !== '' && endColumn.trim() !== '')

  async function save() {
    if (!canSave) return
    busy = true
    try {
      if (templateId === null) {
        const id = await api.createImportTemplate(
          name.trim(),
          isDefaultField,
          dateColumn.trim(),
          startColumn.trim(),
          endColumn.trim(),
          notes.trim() || undefined,
          categoryColumn.trim() || undefined,
          notesColumn.trim() || undefined,
        )
        await refreshImportTemplates()
        navGuard.isDirty = false
        onBack(id)
      } else {
        await api.updateImportTemplate(
          templateId,
          name.trim(),
          isDefaultField,
          dateColumn.trim(),
          startColumn.trim(),
          endColumn.trim(),
          notes.trim() || undefined,
          categoryColumn.trim() || undefined,
          notesColumn.trim() || undefined,
        )
        await refreshImportTemplates()
        navGuard.isDirty = false
        onBack(templateId)
      }
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
      Case-insensitive exact match against your file's header row. Category and Notes are
      optional — leave one blank to skip looking for that column entirely.
    </p>
    <div class="field">
      <label for="tpl-date">Date</label>
      <input id="tpl-date" bind:value={dateColumn} />
    </div>
    <div class="field">
      <label for="tpl-start">Start Time</label>
      <input id="tpl-start" bind:value={startColumn} />
    </div>
    <div class="field">
      <label for="tpl-end">End Time</label>
      <input id="tpl-end" bind:value={endColumn} />
    </div>
    <div class="field">
      <label for="tpl-category">Category</label>
      <input id="tpl-category" bind:value={categoryColumn} placeholder="(none)" />
    </div>
    <div class="field">
      <label for="tpl-notes-col">Notes</label>
      <input id="tpl-notes-col" bind:value={notesColumn} placeholder="(none)" />
    </div>

    <button class:unchanged={!dirty} onclick={save} disabled={busy || !dirty || !canSave}>Save</button>
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
  button.unchanged {
    background: #e5e5e5;
    color: #888;
    border-color: #d5d5d5;
  }
</style>
