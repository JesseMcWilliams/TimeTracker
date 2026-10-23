<script lang="ts">
  import { confirm } from '@tauri-apps/plugin-dialog'
  import { api, EMAIL_PLACEHOLDERS, type EmailDateFormat, type EmailDateRange } from './api'
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
    subjectTemplate: '',
    bodyTemplate: '',
    dateFormat: 'iso' as EmailDateFormat,
    isDefault: false,
  }

  let name = $state(BLANK.name)
  let description = $state(BLANK.description)
  let notes = $state(BLANK.notes)
  let email = $state(BLANK.email)
  let contractId = $state<number | ''>(BLANK.contractId)
  let dateRange = $state<EmailDateRange>(BLANK.dateRange)
  let subjectTemplate = $state(BLANK.subjectTemplate)
  let bodyTemplate = $state(BLANK.bodyTemplate)
  let dateFormat = $state<EmailDateFormat>(BLANK.dateFormat)
  let isDefault = $state(BLANK.isDefault)
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
      subjectTemplate = BLANK.subjectTemplate
      bodyTemplate = BLANK.bodyTemplate
      dateFormat = BLANK.dateFormat
      isDefault = BLANK.isDefault
      initializedFor = 'new'
    } else if (template) {
      name = template.name
      description = template.description ?? ''
      notes = template.notes ?? ''
      email = template.email ?? ''
      contractId = template.contractId ?? ''
      dateRange = template.dateRange
      subjectTemplate = template.subjectTemplate ?? ''
      bodyTemplate = template.bodyTemplate ?? ''
      dateFormat = template.dateFormat
      isDefault = template.isDefault
      initializedFor = templateId
    }
  })

  let edited = $derived({
    name,
    description,
    notes,
    email,
    contractId,
    dateRange,
    subjectTemplate,
    bodyTemplate,
    dateFormat,
    isDefault,
  })

  let dirty = $derived(
    templateId === null
      ? isDirty(edited, BLANK)
      : template
        ? isDirty(edited, {
            name: template.name,
            description: template.description ?? '',
            notes: template.notes ?? '',
            email: template.email ?? '',
            contractId: template.contractId ?? '',
            dateRange: template.dateRange,
            subjectTemplate: template.subjectTemplate ?? '',
            bodyTemplate: template.bodyTemplate ?? '',
            dateFormat: template.dateFormat,
            isDefault: template.isDefault,
          })
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
      const input = {
        name: name.trim(),
        dateRange,
        dateFormat,
        description: description.trim() || undefined,
        notes: notes.trim() || undefined,
        email: email.trim() || undefined,
        contractId: contractId === '' ? undefined : Number(contractId),
        subjectTemplate: subjectTemplate.trim() || undefined,
        bodyTemplate: bodyTemplate.trim() || undefined,
      }
      let savedId: number
      if (templateId === null) {
        savedId = await api.createEmailTemplate(input)
      } else {
        await api.updateEmailTemplate(templateId, input)
        savedId = templateId
      }

      // The default flag has its own command because setting it also has to clear
      // whichever template held it before.
      const wasDefault = template?.isDefault ?? false
      if (isDefault !== wasDefault) await api.setDefaultEmailTemplate(isDefault ? savedId : undefined)

      await refreshEmailTemplates()
      navGuard.isDirty = false
      onBack(savedId)
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
    <div class="field">
      <label for="tpl-date-format">Date format</label>
      <select id="tpl-date-format" bind:value={dateFormat}>
        <option value="iso">2026-08-17</option>
        <option value="us">08/17/2026</option>
        <option value="long">Aug 17, 2026</option>
      </select>
    </div>
    <div class="field">
      <span class="field-label">Default</span>
      <label class="checkbox">
        <input type="checkbox" bind:checked={isDefault} />
        Preselect this template on the Reports page
      </label>
    </div>

    <h3>Subject &amp; body</h3>
    <p class="hint">
      Leave either blank to use the built-in wording. Placeholders:
      {#each EMAIL_PLACEHOLDERS as placeholder, i (placeholder)}<code>{'{' + placeholder + '}'}</code
        >{i < EMAIL_PLACEHOLDERS.length - 1 ? ', ' : ''}{/each}
    </p>
    <div class="field">
      <label for="tpl-subject">Subject</label>
      <input id="tpl-subject" bind:value={subjectTemplate} placeholder="Timesheets: {'{StartDate}'} to {'{EndDate}'}" />
    </div>
    <div class="field notes-row">
      <label for="tpl-body">Body</label>
      <textarea
        id="tpl-body"
        class="notes-input"
        bind:value={bodyTemplate}
        rows="4"
        placeholder={'The following contracts have no time logged for {DateRange}:\n\n{Contracts}'}
      ></textarea>
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
  .field label,
  .field .field-label {
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
  h3 {
    margin-top: 1.5rem;
    margin-bottom: 0.4rem;
  }
  .hint {
    color: #666;
    font-size: 0.85rem;
    margin: 0 0 0.75rem;
    line-height: 1.5;
  }
  .hint code {
    background: rgba(128, 128, 128, 0.15);
    border-radius: 3px;
    padding: 0.05rem 0.25rem;
  }
  .field label.checkbox {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    width: auto;
    font-size: 0.9rem;
  }
  .checkbox input {
    flex: none;
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
