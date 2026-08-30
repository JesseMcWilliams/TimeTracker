<script lang="ts">
  import { api, type Report, type ReportPeriod, type TimesheetFile, type TimeEntry } from './api'
  import { formatDuration, formatMoney, formatDateTime } from './dateUtils'

  function today(): string {
    const d = new Date()
    const pad = (n: number) => String(n).padStart(2, '0')
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`
  }

  let period = $state<ReportPeriod>('week')
  let referenceDate = $state(today())
  let showAmounts = $state(false)
  let report = $state<Report | null>(null)
  let loading = $state(false)
  let error = $state('')

  let drillClientId = $state<number | null>(null)
  let drillContractId = $state<number | null>(null)
  let contractEntries = $state<TimeEntry[] | null>(null)
  let contractEntriesLoading = $state(false)

  let timesheetBusy = $state(false)
  let timesheetResult = $state<TimesheetFile[] | null>(null)
  let timesheetError = $state('')
  let includeRateAmount = $state(false)

  async function loadReport() {
    loading = true
    error = ''
    try {
      report = await api.generateReport(period, referenceDate)
    } catch (e) {
      error = String(e)
    } finally {
      loading = false
    }
  }

  $effect(() => {
    // re-run whenever period or referenceDate changes
    period
    referenceDate
    drillClientId = null
    drillContractId = null
    contractEntries = null
    loadReport()
  })

  let displayedClients = $derived(
    drillClientId === null ? (report?.clients ?? []) : (report?.clients ?? []).filter((c) => c.clientId === drillClientId),
  )

  let drillClientName = $derived(report?.clients.find((c) => c.clientId === drillClientId)?.clientName ?? '')
  let drillContractName = $derived(
    report?.clients
      .flatMap((c) => c.contracts)
      .find((c) => c.contractId === drillContractId)?.contractName ?? '',
  )

  function selectClient(clientId: number) {
    drillClientId = clientId
    drillContractId = null
    contractEntries = null
  }

  async function selectContract(contractId: number) {
    drillContractId = contractId
    contractEntriesLoading = true
    contractEntries = null
    if (report) {
      try {
        contractEntries = await api.listEntries({
          contractId,
          from: `${report.from}T00:00:00Z`,
          to: `${report.to}T23:59:59Z`,
        })
      } finally {
        contractEntriesLoading = false
      }
    }
  }

  function backToClients() {
    drillClientId = null
    drillContractId = null
    contractEntries = null
  }

  function backToContracts() {
    drillContractId = null
    contractEntries = null
  }

  async function createTimesheets() {
    timesheetBusy = true
    timesheetError = ''
    timesheetResult = null
    try {
      timesheetResult = await api.generateTimesheets(
        period,
        referenceDate,
        includeRateAmount,
        drillClientId ?? undefined,
        drillContractId ?? undefined,
      )
    } catch (e) {
      timesheetError = String(e)
    } finally {
      timesheetBusy = false
    }
  }

  let timesheetScopeLabel = $derived(
    drillContractId !== null
      ? `${drillClientName} — ${drillContractName}`
      : drillClientId !== null
        ? drillClientName
        : null,
  )
</script>

<section class="panel">
  <h2>Reports</h2>

  <div class="row">
    <select bind:value={period}>
      <option value="week">Week</option>
      <option value="month">Month</option>
    </select>
    <input type="date" bind:value={referenceDate} />
    <label class="show-amounts">
      <input type="checkbox" bind:checked={showAmounts} />
      Show amounts
    </label>
  </div>

  {#if error}
    <div class="error">{error}</div>
  {:else if loading}
    <p>Loading…</p>
  {:else if report}
    <p class="range">{report.from} to {report.to}</p>
    <p class="total">
      Total: {formatDuration(report.totalSecs)}
      {#if showAmounts}
        &mdash; {formatMoney(report.totalAmount, 'USD')}
      {/if}
    </p>

    {#if drillClientId !== null}
      <p class="breadcrumb">
        <button class="link" onclick={backToClients}>All Clients</button>
        {#if drillContractId !== null}
          &rsaquo; <button class="link" onclick={backToContracts}>{drillClientName}</button>
          &rsaquo; {drillContractName}
        {:else}
          &rsaquo; {drillClientName}
        {/if}
      </p>
    {/if}

    {#if drillContractId !== null}
      {#if contractEntriesLoading}
        <p>Loading entries…</p>
      {:else if !contractEntries || contractEntries.length === 0}
        <p class="muted">No entries for this contract in this period.</p>
      {:else}
        <table>
          <thead>
            <tr>
              <th>Start</th>
              <th>End</th>
              <th>Duration</th>
              <th>Category</th>
              <th>Notes</th>
            </tr>
          </thead>
          <tbody>
            {#each contractEntries as entry (entry.id)}
              <tr>
                <td>{formatDateTime(entry.startedAt)}</td>
                <td>{entry.endedAt ? formatDateTime(entry.endedAt) : '—'}</td>
                <td>{formatDuration(entry.durationSecs)}</td>
                <td>{entry.trackingCode ?? ''}</td>
                <td>{entry.notes ?? ''}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    {:else if report.clients.length === 0}
      <p class="muted">No time entries in this period.</p>
    {:else}
      <table>
        <thead>
          <tr>
            <th>Client / Contract</th>
            <th>Hours</th>
            {#if showAmounts}
              <th>Amount</th>
            {/if}
          </tr>
        </thead>
        <tbody>
          {#each displayedClients as client (client.clientId)}
            <tr class="client-row">
              <td
                ><button class="link" onclick={() => selectClient(client.clientId)}>{client.clientName}</button></td
              >
              <td>{formatDuration(client.totalSecs)}</td>
              {#if showAmounts}
                <td>{formatMoney(client.totalAmount, 'USD')}</td>
              {/if}
            </tr>
            {#each client.contracts as contract (contract.contractId)}
              <tr>
                <td class="indent"
                  ><button class="link" onclick={() => selectContract(contract.contractId)}
                    >{contract.contractName}</button
                  ></td
                >
                <td>{formatDuration(contract.totalSecs)}</td>
                {#if showAmounts}
                  <td>{formatMoney(contract.totalAmount, 'USD')}</td>
                {/if}
              </tr>
            {/each}
          {/each}
        </tbody>
      </table>
    {/if}
  {/if}

  <h3>Create Timesheet</h3>
  {#if timesheetScopeLabel}
    <p class="muted">
      Writes a .xlsx file for <strong>{timesheetScopeLabel}</strong> only, covering this period, to
      your configured output folder (set on the User page). Columns: Date, Start Time, End Time,
      HH:MM, Category, Notes.
    </p>
  {:else}
    <p class="muted">
      Writes one .xlsx file per contract with entries in this period to your configured output
      folder (set on the User page). Columns: Date, Start Time, End Time, HH:MM,
      Category, Notes. Drill into a client or contract above to limit this to just that
      client/contract.
    </p>
  {/if}
  <label class="show-amounts">
    <input type="checkbox" bind:checked={includeRateAmount} />
    Include rate and amount columns
  </label>
  <button onclick={createTimesheets} disabled={timesheetBusy}
    >{timesheetScopeLabel ? `Create Timesheet for ${timesheetScopeLabel}` : 'Create Timesheet'}</button
  >

  {#if timesheetError}
    <div class="error">{timesheetError}</div>
  {:else if timesheetResult}
    {#if timesheetResult.length === 0}
      <p class="muted">No contracts had entries in this period — nothing to write.</p>
    {:else}
      <ul class="results">
        {#each timesheetResult as file}
          <li>{file.clientName} / {file.contractName} — {file.entryCount} entries → {file.path}</li>
        {/each}
      </ul>
    {/if}
  {/if}
</section>

<style>
  .panel {
    margin-bottom: 2rem;
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
  .range {
    color: #666;
    margin: 0 0 0.25rem;
  }
  .total {
    font-weight: 600;
    margin: 0 0 0.75rem;
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
  tr.client-row {
    font-weight: 600;
  }
  .indent {
    padding-left: 1.5rem;
    font-weight: normal;
  }
  .muted {
    color: #666;
    font-size: 0.9rem;
  }
  .error {
    background: #f8d7da;
    color: #58151c;
    border: 1px solid #f1aeb5;
    border-radius: 4px;
    padding: 0.5rem 0.75rem;
    margin: 0.5rem 0;
  }
  h3 {
    margin-top: 2rem;
  }
  .results {
    margin-top: 0.75rem;
    padding-left: 1.2rem;
  }
  .breadcrumb {
    color: #666;
    font-size: 0.9rem;
    margin: 0 0 0.5rem;
  }
  .link {
    background: none;
    border: none;
    padding: 0;
    color: #0d6efd;
    text-decoration: underline;
    cursor: pointer;
    font: inherit;
  }
</style>
