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

/** This machine's current offset east of UTC, in minutes — the sign convention the
 * backend's date-range bounds expect (UTC-5 is -300). `getTimezoneOffset` reports the
 * opposite sign, hence the negation. Read per call so it stays right across a DST
 * change without a restart. */
export function utcOffsetMinutes(): number {
  return -new Date().getTimezoneOffset()
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
