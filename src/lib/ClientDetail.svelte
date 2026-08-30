<script lang="ts">
  import { api, WEEKDAYS, type Weekday } from './api'
  import { store, navGuard, refreshClients, loadTrackingCodesForClient } from './store.svelte'
  import { isDirty, capitalize } from './dateUtils'

  let { clientId, onBack }: { clientId: number; onBack: () => void } = $props()

  let client = $derived(store.clients.find((c) => c.id === clientId))
  let codes = $derived(store.trackingCodesByClient[clientId] ?? [])

  let editName = $state('')
  let editNotes = $state('')
  let editPrime = $state('')
  let editSub = $state('')
  let editExternalId = $state('')
  let editWeekStart = $state<Weekday>('monday')
  let editWeekEnd = $state<Weekday>('sunday')
  let editDefaultTrackingCodeId = $state<number | ''>('')
  let editMinimumIncrementMinutes = $state<number | ''>('')
  let busy = $state(false)
  let initializedFor = $state<number | null>(null)

  let newCode = $state('')
  let newCodeDescription = $state('')

  $effect(() => {
    if (client && initializedFor !== client.id) {
      editName = client.name
      editNotes = client.notes ?? ''
      editPrime = client.prime ?? ''
      editSub = client.sub ?? ''
      editExternalId = client.externalId ?? ''
      editWeekStart = client.weekStart
      editWeekEnd = client.weekEnd
      editDefaultTrackingCodeId = client.defaultTrackingCodeId ?? ''
      editMinimumIncrementMinutes = client.minimumIncrementMinutes ?? ''
      initializedFor = client.id
      loadTrackingCodesForClient(client.id)
    }
  })

  let dirty = $derived(
    client
      ? isDirty(
          {
            editName,
            editNotes,
            editPrime,
            editSub,
            editExternalId,
            editWeekStart,
            editWeekEnd,
            editDefaultTrackingCodeId,
            editMinimumIncrementMinutes,
          },
          {
            editName: client.name,
            editNotes: client.notes ?? '',
            editPrime: client.prime ?? '',
            editSub: client.sub ?? '',
            editExternalId: client.externalId ?? '',
            editWeekStart: client.weekStart,
            editWeekEnd: client.weekEnd,
            editDefaultTrackingCodeId: client.defaultTrackingCodeId ?? '',
            editMinimumIncrementMinutes: client.minimumIncrementMinutes ?? '',
          },
        )
      : false,
  )

  $effect(() => {
    navGuard.isDirty = dirty
  })

  async function save() {
    if (!client || !editName.trim()) return
    busy = true
    try {
      await api.updateClient(
        client.id,
        editName.trim(),
        editWeekStart,
        editWeekEnd,
        editNotes.trim() || undefined,
        editPrime.trim() || undefined,
        editSub.trim() || undefined,
        editExternalId.trim() || undefined,
        editDefaultTrackingCodeId ? Number(editDefaultTrackingCodeId) : undefined,
        editMinimumIncrementMinutes === '' ? undefined : Number(editMinimumIncrementMinutes),
      )
      await refreshClients()
      navGuard.isDirty = false
    } finally {
      busy = false
    }
  }

  async function addTrackingCode() {
    if (!client || !newCode.trim()) return
    busy = true
    try {
      await api.createTrackingCode(client.id, newCode.trim(), newCodeDescription.trim() || undefined)
      newCode = ''
      newCodeDescription = ''
      await loadTrackingCodesForClient(client.id)
    } finally {
      busy = false
    }
  }

  async function archiveCode(trackingCodeId: number) {
    if (!client) return
    busy = true
    try {
      await api.archiveTrackingCode(trackingCodeId)
      await loadTrackingCodesForClient(client.id)
    } finally {
      busy = false
    }
  }

  async function archiveClient() {
    if (!client) return
    if (!confirm('Archive this client? It will be hidden from the Clients list but can be restored from Trash.'))
      return
    busy = true
    try {
      await api.archiveClient(client.id)
      await refreshClients()
      onBack()
    } finally {
      busy = false
    }
  }
</script>

<section class="panel">
  <button onclick={onBack}>&larr; Back to clients</button>

  {#if !client}
    <p>This client no longer exists.</p>
  {:else}
    <h2>{client.name}</h2>

    <div class="field">
      <label for="edit-name">Name</label>
      <input id="edit-name" bind:value={editName} />
    </div>
    <div class="field">
      <label for="edit-id">ID</label>
      <input id="edit-id" bind:value={editExternalId} />
    </div>
    <div class="field">
      <label for="edit-prime">Prime</label>
      <input id="edit-prime" bind:value={editPrime} placeholder="Prime contractor name" />
    </div>
    <div class="field">
      <label for="edit-sub">Sub</label>
      <input id="edit-sub" bind:value={editSub} placeholder="Subcontractor name" />
    </div>
    <div class="field">
      <label for="edit-week-start">Week start</label>
      <select id="edit-week-start" bind:value={editWeekStart}>
        {#each WEEKDAYS as day}
          <option value={day}>{capitalize(day)}</option>
        {/each}
      </select>
    </div>
    <div class="field">
      <label for="edit-week-end">Week end</label>
      <select id="edit-week-end" bind:value={editWeekEnd}>
        {#each WEEKDAYS as day}
          <option value={day}>{capitalize(day)}</option>
        {/each}
      </select>
    </div>
    <div class="field">
      <label for="edit-min-increment">Min. increment</label>
      <input
        id="edit-min-increment"
        type="number"
        min="1"
        placeholder="e.g. 15 or 30"
        bind:value={editMinimumIncrementMinutes}
        style="width: 6rem; flex: 0 0 auto"
      />
      <span class="muted">minutes — entries round up to the nearest multiple; blank means no rounding</span>
    </div>
    <div class="field notes-row">
      <label for="edit-notes">Notes</label>
      <textarea id="edit-notes" class="notes-input" bind:value={editNotes} rows="2"></textarea>
    </div>
    <div class="row">
      <button class:unchanged={!dirty} onclick={save} disabled={busy || !editName.trim() || !dirty}>Save</button>
      <button onclick={archiveClient} disabled={busy} class="danger">Archive client</button>
    </div>

    <h3>Categories</h3>
    <div class="row">
      <input placeholder="Category" bind:value={newCode} style="width: 8rem" />
      <input placeholder="Description (optional)" bind:value={newCodeDescription} style="flex: 1" />
      <button onclick={addTrackingCode} disabled={busy}>Add category</button>
    </div>
    {#if codes.length === 0}
      <p class="muted">No categories for this client yet.</p>
    {:else}
      <table>
        <thead>
          <tr>
            <th>Category</th>
            <th>Description</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {#each codes as tc (tc.id)}
            <tr>
              <td>{tc.code}</td>
              <td>{tc.description ?? ''}</td>
              <td><button onclick={() => archiveCode(tc.id)} disabled={busy}>Archive</button></td>
            </tr>
          {/each}
        </tbody>
      </table>

      <div class="field">
        <label for="default-code">Default category</label>
        <select id="default-code" bind:value={editDefaultTrackingCodeId}>
          <option value="">None</option>
          {#each codes as tc (tc.id)}
            <option value={tc.id}>{tc.code}</option>
          {/each}
        </select>
        <button class:unchanged={!dirty} onclick={save} disabled={busy || !dirty}>Save</button>
      </div>
      <p class="muted">Used by Quick Timer to start this client's timers without prompting for a category.</p>
    {/if}
  {/if}
</section>

<style>
  .panel {
    margin-bottom: 2rem;
  }
  h3 {
    margin-top: 2rem;
  }
  .field {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
  }
  .field label {
    width: 4.5rem;
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
  .muted {
    color: #666;
    font-size: 0.9rem;
  }
  button.unchanged {
    background: #e5e5e5;
    color: #888;
    border-color: #d5d5d5;
  }
  .danger {
    color: #b91c1c;
  }
</style>
