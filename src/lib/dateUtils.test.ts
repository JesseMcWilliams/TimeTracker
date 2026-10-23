import { describe, expect, it } from 'vitest'
import { payWeekRange, roundToNearest5Minutes, shiftWeeks, startOfDay, utcOffsetMinutes } from './dateUtils'

/** Local calendar date, written the way the assertions read. Built from local
 * components rather than an ISO string so these tests assert the same thing in every
 * timezone the suite might run in — which is the whole point of the code under test. */
function local(year: number, month: number, day: number, hours = 0, minutes = 0, seconds = 0, ms = 0): Date {
  return new Date(year, month - 1, day, hours, minutes, seconds, ms)
}

/** Renders a Date as local "YYYY-MM-DD HH:mm:ss.SSS", so a failure names the wall-clock
 * time that was actually produced instead of a UTC instant the reader has to convert. */
function show(date: Date): string {
  const pad = (n: number, width = 2) => String(n).padStart(width, '0')
  return (
    `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ` +
    `${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}.${pad(date.getMilliseconds(), 3)}`
  )
}

// 2026-08-17 is a Monday, so 08-23 is the Sunday that closes that week and 08-24 opens
// the next one. Every case below is anchored to that week.
const MONDAY = local(2026, 8, 17)
const THURSDAY = local(2026, 8, 20)
const SUNDAY = local(2026, 8, 23)
const NEXT_MONDAY = local(2026, 8, 24)

describe('payWeekRange', () => {
  it('runs from the given weekday through the following six days', () => {
    const { from, to } = payWeekRange(THURSDAY, 'monday')

    expect(show(from)).toBe('2026-08-17 00:00:00.000')
    expect(show(to)).toBe('2026-08-23 23:59:59.999')
  })

  it('treats the start weekday itself as inside the week it opens, not the one before', () => {
    expect(show(payWeekRange(MONDAY, 'monday').from)).toBe('2026-08-17 00:00:00.000')
    expect(show(payWeekRange(NEXT_MONDAY, 'monday').from)).toBe('2026-08-24 00:00:00.000')
  })

  it('includes the last moment of the final day', () => {
    expect(show(payWeekRange(SUNDAY, 'monday').to)).toBe('2026-08-23 23:59:59.999')
  })

  it('honours a week that does not start on Monday', () => {
    // A Sunday-start week containing Thursday 08-20 runs 08-16..08-22.
    const sundayWeek = payWeekRange(THURSDAY, 'sunday')
    expect(show(sundayWeek.from)).toBe('2026-08-16 00:00:00.000')
    expect(show(sundayWeek.to)).toBe('2026-08-22 23:59:59.999')

    // A Thursday-start week containing that same Thursday starts on it.
    const thursdayWeek = payWeekRange(THURSDAY, 'thursday')
    expect(show(thursdayWeek.from)).toBe('2026-08-20 00:00:00.000')
    expect(show(thursdayWeek.to)).toBe('2026-08-26 23:59:59.999')
  })

  it('falls back to Monday for an unrecognised weekday name', () => {
    expect(show(payWeekRange(THURSDAY, 'notaday').from)).toBe('2026-08-17 00:00:00.000')
  })

  it('accepts a weekday name in any case', () => {
    expect(show(payWeekRange(THURSDAY, 'SUNDAY').from)).toBe('2026-08-16 00:00:00.000')
  })

  it('ignores the time of day on the reference date', () => {
    const lateOnSunday = local(2026, 8, 23, 23, 59, 30)
    expect(show(payWeekRange(lateOnSunday, 'monday').from)).toBe('2026-08-17 00:00:00.000')
  })

  it('crosses a month boundary without losing a day', () => {
    // 2026-09-02 is a Wednesday; its Monday-start week opens on 08-31.
    const { from, to } = payWeekRange(local(2026, 9, 2), 'monday')
    expect(show(from)).toBe('2026-08-31 00:00:00.000')
    expect(show(to)).toBe('2026-09-06 23:59:59.999')
  })
})

describe('shiftWeeks', () => {
  const thisWeek = payWeekRange(THURSDAY, 'monday') // 08-17 .. 08-23

  it('shifts back one whole week for "last week"', () => {
    const { from, to } = shiftWeeks(thisWeek, 1)

    expect(show(from)).toBe('2026-08-10 00:00:00.000')
    expect(show(to)).toBe('2026-08-16 23:59:59.999')
  })

  it('spans the two complete weeks before this one for "last 2 weeks"', () => {
    const { from, to } = shiftWeeks(thisWeek, 2, 2)

    expect(show(from)).toBe('2026-08-03 00:00:00.000')
    expect(show(to)).toBe('2026-08-16 23:59:59.999')
  })

  it('stops short of the current week, so this week is never double-counted', () => {
    const lastWeek = shiftWeeks(thisWeek, 1)
    const lastTwo = shiftWeeks(thisWeek, 2, 2)

    expect(lastWeek.to.getTime()).toBeLessThan(thisWeek.from.getTime())
    expect(lastTwo.to.getTime()).toBeLessThan(thisWeek.from.getTime())
  })

  it('leaves the two-week span exactly 14 days long', () => {
    const { from, to } = shiftWeeks(thisWeek, 2, 2)
    const days = (to.getTime() - from.getTime()) / (24 * 60 * 60 * 1000)

    expect(days).toBeCloseTo(14, 3)
  })

  it('does not mutate the range it was given', () => {
    const original = payWeekRange(THURSDAY, 'monday')
    const before = [show(original.from), show(original.to)]

    shiftWeeks(original, 2, 2)

    expect([show(original.from), show(original.to)]).toEqual(before)
  })

  it('crosses a month boundary when shifting back', () => {
    const septemberWeek = payWeekRange(local(2026, 9, 9), 'monday') // 09-07 .. 09-13
    const { from, to } = shiftWeeks(septemberWeek, 2, 2)

    expect(show(from)).toBe('2026-08-24 00:00:00.000')
    expect(show(to)).toBe('2026-09-06 23:59:59.999')
  })
})

describe('startOfDay', () => {
  it('strips the time without moving the date', () => {
    expect(show(startOfDay(local(2026, 8, 20, 17, 45, 12, 500)))).toBe('2026-08-20 00:00:00.000')
  })

  it('does not mutate its argument', () => {
    const input = local(2026, 8, 20, 17, 45)
    startOfDay(input)

    expect(show(input)).toBe('2026-08-20 17:45:00.000')
  })
})

describe('utcOffsetMinutes', () => {
  it('reports minutes east of UTC, the sign the backend bounds expect', () => {
    expect(utcOffsetMinutes()).toBe(-new Date().getTimezoneOffset())
  })
})

describe('roundToNearest5Minutes', () => {
  it('rounds to the nearer 5-minute mark', () => {
    expect(roundToNearest5Minutes('2026-08-20T09:02')).toBe('2026-08-20T09:00')
    expect(roundToNearest5Minutes('2026-08-20T09:03')).toBe('2026-08-20T09:05')
  })

  it('rolls an upward round past the hour', () => {
    expect(roundToNearest5Minutes('2026-08-20T09:58')).toBe('2026-08-20T10:00')
  })

  it('rolls past midnight onto the next day', () => {
    expect(roundToNearest5Minutes('2026-08-20T23:59')).toBe('2026-08-21T00:00')
  })

  it('passes an empty value straight through', () => {
    expect(roundToNearest5Minutes('')).toBe('')
  })
})
