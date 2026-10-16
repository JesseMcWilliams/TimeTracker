<script lang="ts">
  import { confirm } from '@tauri-apps/plugin-dialog'
  import { api, type EmailDateRange } from './api'
  import { store, navGuard, refreshEmailTemplates, activeContracts, contractLabel } from './store.svelte'
  import { isDirty } from './dateUtils'

  let { templateId, onBack }: { templateId: number | null; onBack: (savedId?: number) => void } = $props()

  let template = $derived(templateId === null ? undefined : store.emailTemplates.find((t) => t.id === templateId))

  const BLANK = {
    name: '',
    description: '',
    notes: '',
    email: '',
    contractId: '' as number | '',
    dateRange: 'previous_week' as EmailDateRange,
  }

  let name = $state(BLANK.name)
  let description = $state(BLANK.description)
  let notes = $state(BLANK.notes)
  let email = $state(BLANK.email)
  let contractId = $state<number | ''>(BLANK.contractId)
  let dateRange = $state<EmailDateRange>(BLANK.dateRange)
  let busy = $state(false)
  let deleteError = $state('')
  let initializedFor = $state<number | null | 'new'>(null)

  $effect(() => {
    const key = templateId === null ? 'new' : templateId
    if (initializedFor === key) return
    if (templateId === null) {
      name = BLANK.name
      description = BLANK.description
      notes = BLANK.notes
      email = BLANK.email
      contractId = BLANK.contractId
      dateRange = BLANK.dateRange
      initializedFor = 'new'
    } else if (template) {
      name = template.name
      description = template.description ?? ''
      notes = template.notes ?? ''
      email = template.email ?? ''
      contractId = template.contractId ?? ''
      dateRange = template.dateRange
      initializedFor = templateId
    }
  })

  let dirty = $derived(
    templateId === null
      ? isDirty({ name, description, notes, email, contractId, dateRange }, BLANK)
      : template
        ? isDirty(
            { name, description, notes, email, contractId, dateRange },
            {
              name: template.name,
              description: template.description ?? '',
              notes: template.notes ?? '',
              email: template.email ?? '',
              contractId: template.contractId ?? '',
              dateRange: template.dateRange,
            },
          )
        : false,
  )

  $effect(() => {
    navGuard.isDirty = dirty
  })

  let canSave = $derived(name.trim() !== '')

  async function save() {
    if (!canSave) return
    busy = true
    try {
      const contractIdArg = contractId === '' ? undefined : Number(contractId)
      if (templateId === null) {
        const id = await api.createEmailTemplate(
          name.trim(),
          dateRange,
          description.trim() || undefined,
          notes.trim() || undefined,
          email.trim() || undefined,
          contractIdArg,
        )
        await refreshEmailTemplates()
        navGuard.isDirty = false
        onBack(id)
      } else {
        await api.updateEmailTemplate(
          templateId,
          name.trim(),
          dateRange,
          description.trim() || undefined,
          notes.trim() || undefined,
          email.trim() || undefined,
          contractIdArg,
        )
        await refreshEmailTemplates()
        navGuard.isDirty = false
        onBack(templateId)
      }
    } finally {
      busy = false
    }
  }

  async function remove() {
    if (templateId === null) return
    if (!(await confirm(`Delete the "${name}" email template? This can't be undone.`))) return
    busy = true
    deleteError = ''
    try {
      await api.deleteEmailTemplate(templateId)
      await refreshEmailTemplates()
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
  <button onclick={() => onBack()}>&larr; Back to Reports</button>

  <h2>{templateId === null ? 'Add Email Template' : 'Edit Email Template'}</h2>

  {#if templateId !== null && !template}
    <p>This template no longer exists.</p>
  {:else}
    <div class="field">
      <label for="tpl-name">Name</label>
      <input id="tpl-name" bind:value={name} style="flex: 1" />
    </div>
    <div class="field">
      <label for="tpl-description">Description</label>
      <input id="tpl-description" bind:value={description} style="flex: 1" />
    </div>
    <div class="field notes-row">
      <label for="tpl-notes">Notes</label>
      <textarea
        id="tpl-notes"
        class="notes-input"
        bind:value={notes}
        rows="2"
        placeholder="Optional message to prepend to the generated email"
      ></textarea>
    </div>
    <div class="field">
      <label for="tpl-email">E-Mail</label>
      <input id="tpl-email" type="email" bind:value={email} placeholder="recipient@example.com" style="flex: 1" />
    </div>
    <div class="field">
      <label for="tpl-contract">Contracts</label>
      <select id="tpl-contract" bind:value={contractId}>
        <option value="">All</option>
        {#each activeContracts() as contract (contract.id)}
          <option value={contract.id}>{contractLabel(contract.id)}</option>
        {/each}
      </select>
    </div>
    <div class="field">
      <label for="tpl-date-range">Date range</label>
      <select id="tpl-date-range" bind:value={dateRange}>
        <option value="previous_week">Previous week</option>
        <option value="previous_2_weeks">Previous 2 weeks</option>
        <option value="month">Month</option>
        <option value="previous_month">Previous month</option>
      </select>
    </div>

    {#if deleteError}
      <div class="error">{deleteError}</div>
    {/if}

    <div class="actions">
      <button class:unchanged={!dirty} onclick={save} disabled={busy || !dirty || !canSave}>Save</button>
      {#if templateId !== null}
        <button type="button" class="delete" onclick={remove} disabled={busy}>Delete</button>
      {/if}
    </div>
  {/if}
</section>

<style>
  .panel {
    margin-bottom: 2rem;
    max-width: 32rem;
  }
  .field {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.6rem;
  }
  .field label {
    width: 7rem;
    flex-shrink: 0;
  }
  .field input,
  .field select {
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
