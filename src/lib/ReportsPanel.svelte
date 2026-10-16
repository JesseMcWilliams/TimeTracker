<script lang="ts">
  import { api, type Report, type ReportPeriod, type TimesheetFile, type TimeEntry, type NoTimeEmailPreview } from './api'
  import { formatDuration, formatMoney, formatDateTime } from './dateUtils'
  import { store } from './store.svelte'

  let { onGoToEmailTemplate }: { onGoToEmailTemplate: (templateId: number | null) => void } = $props()

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

  let emailTemplateId = $state<number | ''>('')
  let emailPreview = $state<NoTimeEmailPreview | null>(null)
  let emailBusy = $state(false)
  let emailError = $state('')
  let emailCopied = $state(false)

  // Keep emailTemplateId pointing at a real template whenever possible, without
  // overriding a selection the user already made.
  $effect(() => {
    if (store.emailTemplates.length === 0) {
      emailTemplateId = ''
      return
    }
    if (emailTemplateId !== '' && store.emailTemplates.some((t) => t.id === Number(emailTemplateId))) return
    emailTemplateId = store.emailTemplates[0].id
  })

  let selectedEmailTemplate = $derived(
    emailTemplateId ? store.emailTemplates.find((t) => t.id === Number(emailTemplateId)) : undefined,
  )

  let mailtoHref = $derived.by(() => {
    if (!emailPreview) return ''
    const to = emailPreview.to ?? ''
    const params = new URLSearchParams({ subject: emailPreview.subject, body: emailPreview.body })
    return `mailto:${encodeURIComponent(to)}?${params.toString()}`
  })

  function onEmailTemplateChange() {
    emailPreview = null
    emailError = ''
    emailCopied = false
  }

  async function generateEmail() {
    if (!emailTemplateId) return
    emailBusy = true
    emailError = ''
    emailPreview = null
    emailCopied = false
    try {
      emailPreview = await api.generateNoTimeEmail(Number(emailTemplateId), today())
    } catch (e) {
      emailError = String(e)
    } finally {
      emailBusy = false
    }
  }

  async function copyEmailToClipboard() {
    if (!emailPreview) return
    await navigator.clipboard.writeText(`Subject: ${emailPreview.subject}\n\n${emailPreview.body}`)
    emailCopied = true
  }
</script>

<section class="panel">
  <h2>Reports</h2>

  <div class="row">
    <select bind:value={period}>
      <option value="week">Week</option>
      <option value="month">Month</option>
      <option value="year">Year</option>
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

    {#if drillClientId === null && report.contractsWithoutTime.length > 0}
      <h3>No time logged this period</h3>
      <ul class="no-time-list">
        {#each report.contractsWithoutTime as noTimeContract (noTimeContract.contractId)}
          <li>{noTimeContract.clientName} — {noTimeContract.contractName}</li>
        {/each}
      </ul>
    {/if}
  {/if}

  <h3>Create Timesheet</h3>
  {#if timesheetScopeLabel}
    <p class="muted">
      Writes a .{store.userProfile.outputType} file for <strong>{timesheetScopeLabel}</strong> only, covering
      this period, to your configured output folder (set on the User page). Columns: Date, Start
      Time, End Time, HH:MM, Category, Notes.
    </p>
  {:else}
    <p class="muted">
      Writes one .{store.userProfile.outputType} file per contract with entries in this period to
      your configured output folder (set on the User page). Columns: Date, Start Time, End Time,
      HH:MM, Category, Notes. Drill into a client or contract above to limit this to just that
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

  <h3>No-Time Email</h3>
  <p class="muted">
    Generates the subject and body of an email listing contracts with no time logged in a
    template's date range. This app never sends email itself — copy the text or open it in your
    default mail client.
  </p>
  <div class="row">
    <label for="email-template-select">Template</label>
    <select id="email-template-select" bind:value={emailTemplateId} onchange={onEmailTemplateChange}>
      {#each store.emailTemplates as t (t.id)}
        <option value={t.id}>{t.name}</option>
      {/each}
    </select>
    <button type="button" onclick={() => onGoToEmailTemplate(null)}>Add…</button>
    <button type="button" onclick={() => onGoToEmailTemplate(Number(emailTemplateId))} disabled={!emailTemplateId}
      >Edit…</button
    >
    <button type="button" onclick={generateEmail} disabled={!emailTemplateId || emailBusy}>Generate email</button>
  </div>
  {#if selectedEmailTemplate?.description}
    <p class="muted">{selectedEmailTemplate.description}</p>
  {/if}

  {#if emailError}
    <div class="error">{emailError}</div>
  {:else if emailPreview}
    <div class="email-preview">
      <p><strong>Subject:</strong> {emailPreview.subject}</p>
      <pre class="email-body">{emailPreview.body}</pre>
      <div class="row">
        <button type="button" onclick={copyEmailToClipboard}>{emailCopied ? 'Copied!' : 'Copy to clipboard'}</button>
        {#if emailPreview.to}
          <a class="button-link" href={mailtoHref}>Open in email client</a>
        {/if}
      </div>
      {#if !emailPreview.to}
        <p class="muted">No recipient set on this template — add one to enable "Open in email client".</p>
      {/if}
    </div>
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
  .no-time-list {
    margin-top: 0.5rem;
    padding-left: 1.2rem;
    color: #666;
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
  .email-preview {
    margin-top: 0.75rem;
    border: 1px solid var(--border, #ddd);
    border-radius: 4px;
    padding: 0.75rem;
  }
  .email-body {
    white-space: pre-wrap;
    font: inherit;
    background: none;
    margin: 0.5rem 0;
  }
  .button-link {
    display: inline-flex;
    align-items: center;
    padding: 0.4rem 0.8rem;
    border: 1px solid var(--border, #ccc);
    border-radius: 4px;
    text-decoration: none;
    color: inherit;
  }
</style>
