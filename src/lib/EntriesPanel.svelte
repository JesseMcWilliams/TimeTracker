<script lang="ts">
  import { confirm } from '@tauri-apps/plugin-dialog'
  import { api, type TimeEntry } from './api'
  import {
    store,
    entriesFilter,
    type EntriesFilterPeriod,
    contractLabel,
    clientIdForContract,
    activeContracts,
    isContractArchived,
    loadTrackingCodesForClient,
    refreshAfterEntryChange,
    hasHoursBudget,
    remainingHours,
    payWeekStartFor,
    LOW_HOURS_THRESHOLD,
  } from './store.svelte'
  import {
    toRfc3339,
    formatDateTime,
    formatDuration,
    formatHours,
    formatMoney,
    roundToNearest5Minutes,
    toLocalDateString,
    payWeekRange,
    shiftWeeks,
  } from './dateUtils'

  let { onSelectEntry, onGoToImport }: { onSelectEntry: (entryId: number) => void; onGoToImport: () => void } = $props()

  let newContractId = $state<number | ''>('')
  let newStart = $state('')
  let newEnd = $state('')
  let newNotes = $state('')
  let newTrackingCodeId = $state<number | ''>('')
  let busy = $state(false)
  let showAmounts = $state(false)

  let localEntries = $state<TimeEntry[]>([])
  let loading = $state(false)

  let selectedIds = $state<Set<number>>(new Set())
  let bulkCategoryId = $state<number | ''>('')
  let bulkBusy = $state(false)
  let bulkMessage = $state('')

  const PERIOD_DAYS: Record<'month' | '3months', number> = {
    month: 30,
    '3months': 90,
  }

  function daysAgo(days: number): Date {
    const d = new Date()
    d.setDate(d.getDate() - days)
    return d
  }

  /** Inclusive `YYYY-MM-DD` bounds. Entries record the day they belong to, so these are
   * calendar days rather than instants — no timezone conversion anywhere in the path. */
  function periodRange(period: EntriesFilterPeriod): { from?: string; to?: string } {
    if (period === 'all') return {}

    const today = toLocalDateString(new Date())
    if (period === 'today') return { from: today, to: today }
    if (period === 'yesterday') {
      const yesterday = toLocalDateString(daysAgo(1))
      return { from: yesterday, to: yesterday }
    }

    // The week options follow whole pay periods, not a rolling N days back, so "last
    // week" is the previous complete pay week rather than "the last 7×24 hours".
    if (period === 'thisweek' || period === 'lastweek' || period === 'last2weeks') {
      const thisWeek = payWeekRange(new Date(), payWeekStartFor(entriesFilter.contractId))
      const range =
        period === 'thisweek'
          ? thisWeek
          : period === 'lastweek'
            ? shiftWeeks(thisWeek, 1)
            : shiftWeeks(thisWeek, 2, 2)
      return { from: toLocalDateString(range.from), to: toLocalDateString(range.to) }
    }

    return { from: toLocalDateString(daysAgo(PERIOD_DAYS[period])) }
  }

  async function loadEntries() {
    loading = true
    try {
      localEntries = await api.listEntries({
        ...periodRange(entriesFilter.period),
        contractId: entriesFilter.contractId ? Number(entriesFilter.contractId) : undefined,
      })
      selectedIds = new Set()
    } finally {
      loading = false
    }
  }

  $effect(() => {
    entriesFilter.period
    entriesFilter.contractId
    store.userProfile.payWeekStart
    loadEntries()
  })

  let filterCategoryOptions = $derived(
    [
      ...new Map(
        localEntries
          .filter((e) => e.trackingCodeId !== null)
          .map((e) => [e.trackingCodeId as number, e.trackingCode as string]),
      ).entries(),
    ].sort((a, b) => a[1].localeCompare(b[1])),
  )

  $effect(() => {
    if (entriesFilter.trackingCodeId && !filterCategoryOptions.some(([id]) => id === entriesFilter.trackingCodeId)) {
      entriesFilter.trackingCodeId = ''
    }
  })

  let displayedEntries = $derived(
    entriesFilter.trackingCodeId
      ? localEntries.filter((e) => e.trackingCodeId === Number(entriesFilter.trackingCodeId))
      : localEntries,
  )

  let totalDurationSecs = $derived(displayedEntries.reduce((sum, e) => sum + (e.durationSecs ?? 0), 0))

  let totalsByCurrency = $derived.by(() => {
    const totals: Record<string, number> = {}
    for (const e of displayedEntries) {
      if (e.durationSecs === null) continue
      const currency = store.contracts.find((c) => c.id === e.contractId)?.currency ?? 'USD'
      totals[currency] = (totals[currency] ?? 0) + (e.durationSecs / 3600) * e.rateSnapshot
    }
    return totals
  })

  let totalAmountText = $derived(
    Object.entries(totalsByCurrency)
      .map(([currency, amount]) => formatMoney(amount, currency))
      .join(', ') || '—',
  )

  let availableCodes = $derived(
    newContractId ? (store.trackingCodesByClient[clientIdForContract(Number(newContractId)) ?? -1] ?? []) : [],
  )

  let endBeforeStart = $derived(!!newStart && !!newEnd && newEnd <= newStart)

  /** The contract picked in the add-entry row, when it has an hours budget worth
   * warning about. Contracts with no budget set have nothing to run out of. */
  let lowHoursContract = $derived.by(() => {
    if (!newContractId) return undefined
    const contract = store.contracts.find((c) => c.id === Number(newContractId))
    if (!contract || !hasHoursBudget(contract)) return undefined
    return remainingHours(contract) < LOW_HOURS_THRESHOLD ? contract : undefined
  })

  $effect(() => {
    newTrackingCodeId = ''
    if (!newContractId) return
    const clientId = clientIdForContract(Number(newContractId))
    if (clientId !== undefined) loadTrackingCodesForClient(clientId)
  })

  let selectedEntries = $derived(localEntries.filter((e) => selectedIds.has(e.id)))
  let selectedClientIds = $derived([
    ...new Set(selectedEntries.map((e) => clientIdForContract(e.contractId)).filter((id) => id !== undefined)),
  ] as number[])

  $effect(() => {
    for (const clientId of selectedClientIds) {
      if (!store.trackingCodesByClient[clientId]) loadTrackingCodesForClient(clientId)
    }
  })

  let bulkCategoryOptions = $derived(
    [
      ...new Map(
        selectedClientIds
          .flatMap((clientId) => store.trackingCodesByClient[clientId] ?? [])
          .map((tc) => [tc.id, tc]),
      ).values(),
    ].sort((a, b) => a.code.localeCompare(b.code)),
  )

  function toggleSelected(id: number) {
    const next = new Set(selectedIds)
    if (next.has(id)) next.delete(id)
    else next.add(id)
    selectedIds = next
  }

  function toggleSelectAll() {
    selectedIds = selectedIds.size === displayedEntries.length ? new Set() : new Set(displayedEntries.map((e) => e.id))
  }

  function amountFor(entry: TimeEntry): string {
    if (entry.durationSecs === null) return '—'
    const contract = store.contracts.find((c) => c.id === entry.contractId)
    const currency = contract?.currency ?? 'USD'
    const hours = entry.durationSecs / 3600
    return formatMoney(hours * entry.rateSnapshot, currency)
  }

  async function addManualEntry() {
    if (!newContractId || !newStart || !newEnd) return
    if (endBeforeStart) return
    busy = true
    try {
      await api.createManualEntry(
        Number(newContractId),
        toRfc3339(newStart),
        toRfc3339(newEnd),
        newNotes.trim() || undefined,
        newTrackingCodeId ? Number(newTrackingCodeId) : undefined,
      )
      newContractId = ''
      newStart = ''
      newEnd = ''
      newNotes = ''
      newTrackingCodeId = ''
      await Promise.all([loadEntries(), refreshAfterEntryChange()])
    } finally {
      busy = false
    }
  }

  async function remove(event: MouseEvent, entryId: number) {
    event.stopPropagation()
    if (!(await confirm('Delete this time entry? You can restore it later from Trash.'))) return
    busy = true
    try {
      await api.deleteEntry(entryId)
      await Promise.all([loadEntries(), refreshAfterEntryChange()])
    } finally {
      busy = false
    }
  }

  async function bulkDelete() {
    const count = selectedEntries.length
    if (count === 0) return
    if (!(await confirm(`Delete ${count} selected entr${count === 1 ? 'y' : 'ies'}? You can restore them later from Trash.`)))
      return
    bulkBusy = true
    bulkMessage = ''
    try {
      await Promise.all(selectedEntries.map((e) => api.deleteEntry(e.id)))
      bulkMessage = `Deleted ${count} entr${count === 1 ? 'y' : 'ies'}.`
      await Promise.all([loadEntries(), refreshAfterEntryChange()])
    } finally {
      bulkBusy = false
    }
  }

  async function bulkSetCategory() {
    if (!bulkCategoryId || selectedEntries.length === 0) return
    bulkBusy = true
    bulkMessage = ''
    let applied = 0
    let skipped = 0
    const errors: string[] = []
    for (const entry of selectedEntries) {
      if (!entry.endedAt) {
        skipped += 1
        continue
      }
      try {
        await api.updateEntry(entry.id, entry.startedAt, entry.endedAt, entry.notes ?? undefined, Number(bulkCategoryId))
        applied += 1
      } catch (e) {
        errors.push(`Entry #${entry.id}: ${e}`)
      }
    }
    bulkMessage = `Updated category on ${applied} entr${applied === 1 ? 'y' : 'ies'}${skipped > 0 ? `, skipped ${skipped} running timer(s)` : ''}.${errors.length > 0 ? ' Errors: ' + errors.join('; ') : ''}`
    await Promise.all([loadEntries(), refreshAfterEntryChange()])
    bulkBusy = false
  }
</script>

<section class="panel">
  <div class="header-row">
    <h2>Time Entries</h2>
    <button onclick={onGoToImport}>Import…</button>
  </div>

  <div class="row">
    <select bind:value={newContractId}>
      <option value="">Select contract…</option>
      {#each activeContracts() as contract (contract.id)}
        <option value={contract.id}>{contractLabel(contract.id)}</option>
      {/each}
    </select>
    <input
      type="datetime-local"
      bind:value={newStart}
      onchange={() => (newStart = roundToNearest5Minutes(newStart))}
    />
    <input
      type="datetime-local"
      bind:value={newEnd}
      min={newStart || undefined}
      onchange={() => (newEnd = roundToNearest5Minutes(newEnd))}
    />
    {#if availableCodes.length > 0}
      <select bind:value={newTrackingCodeId}>
        <option value="">No category</option>
        {#each availableCodes as tc (tc.id)}
          <option value={tc.id}>{tc.code}</option>
        {/each}
      </select>
    {/if}
    <input placeholder="Notes (optional)" bind:value={newNotes} style="flex: 1" />
    <button onclick={addManualEntry} disabled={busy || endBeforeStart}>Add entry</button>
  </div>
  {#if endBeforeStart}
    <p class="error">End time must be after the start time.</p>
  {/if}
  {#if lowHoursContract}
    {@const left = remainingHours(lowHoursContract)}
    <p class="warning" class:overdrawn={left < 0}>
      {#if left < 0}
        <strong>{contractLabel(lowHoursContract.id)}</strong> is over budget by {formatHours(Math.abs(left))} —
        {formatHours(lowHoursContract.totalHoursBudgeted)} budgeted, {formatHours(lowHoursContract.hoursLogged)} logged.
      {:else}
        <strong>{contractLabel(lowHoursContract.id)}</strong> has only {formatHours(left)} remaining of
        {formatHours(lowHoursContract.totalHoursBudgeted)} budgeted.
      {/if}
    </p>
  {/if}

  <div class="row">
    <label for="filter-period">Show</label>
    <select id="filter-period" bind:value={entriesFilter.period}>
      <option value="today">Today</option>
      <option value="yesterday">Yesterday</option>
      <option value="thisweek">This week</option>
      <option value="lastweek">Last week</option>
      <option value="last2weeks">Last 2 weeks</option>
      <option value="month">Last month</option>
      <option value="3months">Last 3 months</option>
      <option value="all">All</option>
    </select>
    <select bind:value={entriesFilter.contractId} aria-label="Filter by contract">
      <option value="">All contracts</option>
      {#each activeContracts() as contract (contract.id)}
        <option value={contract.id}>{contractLabel(contract.id)}</option>
      {/each}
    </select>
    <select bind:value={entriesFilter.trackingCodeId} aria-label="Filter by category">
      <option value="">All categories</option>
      {#each filterCategoryOptions as [id, code] (id)}
        <option value={id}>{code}</option>
      {/each}
    </select>
    <label class="show-amounts">
      <input type="checkbox" bind:checked={showAmounts} />
      Show amounts
    </label>
  </div>

  {#if selectedIds.size > 0}
    <div class="bulk-bar">
      <span>{selectedIds.size} selected</span>
      <button onclick={bulkDelete} disabled={bulkBusy} class="danger">Delete selected</button>
      {#if bulkCategoryOptions.length > 0}
        <select bind:value={bulkCategoryId}>
          <option value="">Set category to…</option>
          {#each bulkCategoryOptions as tc (tc.id)}
            <option value={tc.id}>{tc.code}</option>
          {/each}
        </select>
        <button onclick={bulkSetCategory} disabled={bulkBusy || !bulkCategoryId}>Apply</button>
      {/if}
    </div>
  {/if}
  {#if bulkMessage}
    <p class="muted">{bulkMessage}</p>
  {/if}

  <table>
    <thead>
      <tr>
        <th
          ><input
            type="checkbox"
            checked={displayedEntries.length > 0 && selectedIds.size === displayedEntries.length}
            onclick={toggleSelectAll}
          /></th
        >
        <th>Contract</th>
        <th>Category</th>
        <th>Start</th>
        <th>End</th>
        <th>Duration</th>
        {#if showAmounts}
          <th>Amount</th>
        {/if}
        <th>Notes</th>
        <th>Source</th>
        <th></th>
      </tr>
    </thead>
    <tbody>
      {#if loading}
        <tr><td colspan="9" class="muted">Loading…</td></tr>
      {:else if displayedEntries.length === 0}
        <tr><td colspan="9" class="muted">No entries match this filter.</td></tr>
      {/if}
      {#each displayedEntries as entry (entry.id)}
        <tr class="clickable" onclick={() => onSelectEntry(entry.id)}>
          <td onclick={(e) => e.stopPropagation()}
            ><input type="checkbox" checked={selectedIds.has(entry.id)} onclick={() => toggleSelected(entry.id)} /></td
          >
          <td class:archived={isContractArchived(entry.contractId)}>{contractLabel(entry.contractId)}</td>
          <td>{entry.trackingCode ?? ''}</td>
          <td>{formatDateTime(entry.startedAt)}</td>
          <td>{entry.endedAt ? formatDateTime(entry.endedAt) : '—'}</td>
          <td>{formatDuration(entry.durationSecs)}</td>
          {#if showAmounts}
            <td>{amountFor(entry)}</td>
          {/if}
          <td>{entry.notes ?? ''}</td>
          <td>{entry.source}</td>
          <td><button onclick={(e) => remove(e, entry.id)} disabled={busy}>Delete</button></td>
        </tr>
      {/each}
      {#if displayedEntries.length > 0}
        <tr class="totals-row">
          <td colspan="4"></td>
          <td>Total</td>
          <td>{formatDuration(totalDurationSecs)}</td>
          {#if showAmounts}
            <td>{totalAmountText}</td>
          {/if}
          <td colspan="2"></td>
          <td></td>
        </tr>
      {/if}
    </tbody>
  </table>
</section>

<style>
  .panel {
    margin-bottom: 2rem;
  }
  .header-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 1rem;
  }
  .header-row h2 {
    margin: 0;
  }
  .row {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 0.75rem;
    align-items: center;
  }
  .show-amounts {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.9rem;
  }
  .bulk-bar {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background: rgba(13, 110, 253, 0.08);
    border: 1px solid rgba(13, 110, 253, 0.25);
    border-radius: 4px;
    padding: 0.4rem 0.6rem;
    margin-bottom: 0.5rem;
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
  tr.clickable {
    cursor: pointer;
  }
  tr.clickable:hover {
    background: rgba(128, 128, 128, 0.1);
  }
  .totals-row td {
    font-weight: 600;
    border-bottom: none;
    border-top: 2px solid var(--border, #ddd);
  }
  .archived {
    text-decoration: line-through;
    color: #888;
  }
  .error {
    color: #b91c1c;
    font-size: 0.9rem;
    margin: -0.4rem 0 0.75rem;
  }
  /* Amber by default: low hours is something to notice before logging, not an error.
     It escalates to the error red once the contract is actually over budget. */
  .warning {
    color: #92400e;
    background: #fef3c7;
    border: 1px solid #fcd34d;
    border-radius: 4px;
    font-size: 0.9rem;
    padding: 0.4rem 0.6rem;
    margin: -0.4rem 0 0.75rem;
  }
  .warning.overdrawn {
    color: #58151c;
    background: #f8d7da;
    border-color: #f1aeb5;
  }
  .muted {
    color: #666;
    font-size: 0.9rem;
  }
  .danger {
    color: #b91c1c;
  }
</style>
