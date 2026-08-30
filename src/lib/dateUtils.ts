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

export function formatDateTime(iso: string): string {
  return new Date(iso).toLocaleString()
}

export function formatDuration(secs: number | null): string {
  if (secs === null || secs === undefined) return '—'
  const h = Math.floor(secs / 3600)
  const m = Math.floor((secs % 3600) / 60)
  return `${h}h ${m}m`
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
