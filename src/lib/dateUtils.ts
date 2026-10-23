/** Converts a `<input type="datetime-local">` value (local time, no offset) to RFC3339 UTC. */
export function toRfc3339(datetimeLocal: string): string {
  return new Date(datetimeLocal).toISOString()
}

/** Converts a stored RFC3339 timestamp back to a `datetime-local` input value in local time. */
export function toDatetimeLocalValue(iso: string): string {
  const d = new Date(iso)
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`
}

/** Rounds a `<input type="datetime-local">` value to the nearest 5 minutes. The native
 * `step` attribute doesn't restrict the picker's minute list in this app's WebView2
 * runtime, so granularity is enforced here instead, on commit. */
export function roundToNearest5Minutes(datetimeLocal: string): string {
  if (!datetimeLocal) return datetimeLocal
  const d = new Date(datetimeLocal)
  d.setMinutes(Math.round(d.getMinutes() / 5) * 5, 0, 0)
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`
}

/** A Date as the local calendar day it falls on, `YYYY-MM-DD` — the form entries store
 * in `local_date` and that every date-range filter compares against. Built from local
 * components, never `toISOString()`, which would answer in UTC and shift the day. */
export function toLocalDateString(date: Date): string {
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`
}

export function formatDateTime(iso: string): string {
  return new Date(iso).toLocaleString()
}

export function formatDuration(secs: number | null): string {
  if (secs === null || secs === undefined) return '—'
  const sign = secs < 0 ? '-' : ''
  const abs = Math.abs(secs)
  const h = Math.floor(abs / 3600)
  const m = Math.floor((abs % 3600) / 60)
  return `${sign}${h}h ${m}m`
}

const WEEKDAY_INDEX: Record<string, number> = {
  sunday: 0,
  monday: 1,
  tuesday: 2,
  wednesday: 3,
  thursday: 4,
  friday: 5,
  saturday: 6,
}

export function startOfDay(date: Date): Date {
  const start = new Date(date)
  start.setHours(0, 0, 0, 0)
  return start
}

function endOfDay(date: Date): Date {
  const end = new Date(date)
  end.setHours(23, 59, 59, 999)
  return end
}

/** The most recent `weekday` on or before `date`, at local midnight. Mirrors
 * `period::most_recent_on_or_before` on the Rust side. */
function mostRecentOnOrBefore(date: Date, weekday: string): Date {
  const target = WEEKDAY_INDEX[weekday.toLowerCase()] ?? 1
  const d = startOfDay(date)
  const delta = (d.getDay() - target + 7) % 7
  d.setDate(d.getDate() - delta)
  return d
}

/** The pay week containing `reference`, as inclusive local-time bounds. `weekStart` is a
 * weekday name ("monday"); the week runs seven days from there, which is what every
 * weekStart/weekEnd pair in this app describes. */
export function payWeekRange(reference: Date, weekStart: string): { from: Date; to: Date } {
  const from = mostRecentOnOrBefore(reference, weekStart)
  const to = new Date(from)
  to.setDate(to.getDate() + 6)
  return { from, to: endOfDay(to) }
}

/** Shifts a pay-week range back by `weeks` whole weeks, optionally widening it to span
 * that many weeks — "last two weeks" is the two complete pay weeks before this one. */
export function shiftWeeks(range: { from: Date; to: Date }, weeks: number, spanWeeks = 1): { from: Date; to: Date } {
  const from = new Date(range.from)
  from.setDate(from.getDate() - weeks * 7)
  const to = new Date(range.to)
  to.setDate(to.getDate() - (weeks - spanWeeks + 1) * 7)
  return { from, to }
}

/** Hours as a short decimal — "7.5 h". Distinct from `formatDuration`, which renders
 * tracked time as "7h 30m"; budgets are negotiated and discussed in decimal hours. */
export function formatHours(hours: number): string {
  const rounded = Math.round(hours * 100) / 100
  return `${rounded} h`
}

export function formatMoney(amount: number, currency: string): string {
  try {
    return new Intl.NumberFormat(undefined, { style: 'currency', currency }).format(amount)
  } catch {
    return `${amount.toFixed(2)} ${currency}`
  }
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  const units = ['KB', 'MB', 'GB']
  let value = bytes / 1024
  let unitIndex = 0
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024
    unitIndex += 1
  }
  return `${value.toFixed(1)} ${units[unitIndex]}`
}

export function capitalize(s: string): string {
  return s.length ? s[0].toUpperCase() + s.slice(1) : s
}

/** Shallow-compares two plain field snapshots, used to grey out Save buttons when
 * nothing has actually changed from what was loaded. */
export function isDirty(current: Record<string, unknown>, original: Record<string, unknown>): boolean {
  const keys = new Set([...Object.keys(current), ...Object.keys(original)])
  for (const key of keys) {
    if (current[key] !== original[key]) return true
  }
  return false
}
