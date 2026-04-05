import { describe, it, expect } from 'vitest'
import { formatSize, fileDiskSize, timeAgo, tempToColor } from '../utils'

describe('formatSize', () => {
  it('returns "0 B" for 0 bytes', () => {
    expect(formatSize(0)).toBe('0 B')
  })
  it('formats bytes', () => {
    expect(formatSize(500)).toBe('500 B')
  })
  it('formats KB', () => {
    expect(formatSize(1024)).toBe('1.0 KB')
  })
  it('formats MB', () => {
    expect(formatSize(1048576)).toBe('1.0 MB')
  })
  it('formats GB', () => {
    expect(formatSize(1073741824)).toBe('1.0 GB')
  })
  it('formats TB', () => {
    expect(formatSize(1099511627776)).toBe('1.0 TB')
  })
  it('formats partial values', () => {
    expect(formatSize(1536)).toBe('1.5 KB')
  })
})

describe('fileDiskSize', () => {
  it('returns apparent_size for non-sparse files', () => {
    expect(fileDiskSize({ is_sparse: false, actual_size: 500, apparent_size: 1000 })).toBe(1000)
  })
  it('returns actual_size for sparse files under threshold', () => {
    expect(fileDiskSize({ is_sparse: true, actual_size: 100, apparent_size: 1000 })).toBe(100)
  })
  it('returns apparent_size when actual is close to apparent', () => {
    expect(fileDiskSize({ is_sparse: true, actual_size: 900, apparent_size: 1000 })).toBe(1000)
  })
})

describe('timeAgo', () => {
  it('returns empty string for null', () => {
    expect(timeAgo(null)).toBe('')
  })
  it('returns "just now" for recent dates', () => {
    const now = new Date()
    expect(timeAgo(now)).toBe('just now')
  })
  it('returns original string for unparseable dates', () => {
    expect(timeAgo('not-a-date')).toBe('not-a-date')
  })
  it('returns "yesterday" for a date 1 day ago', () => {
    const d = new Date(Date.now() - 86400000 * 1.5)
    expect(timeAgo(d)).toBe('yesterday')
  })
  it('returns minutes ago', () => {
    const d = new Date(Date.now() - 5 * 60000)
    expect(timeAgo(d)).toBe('5m ago')
  })
  it('returns hours ago', () => {
    const d = new Date(Date.now() - 3 * 3600000)
    expect(timeAgo(d)).toBe('3h ago')
  })
})

describe('tempToColor', () => {
  it('returns critical color for >= 95', () => {
    expect(tempToColor(95)).toBe('hsla(0, 50%, 48%, 0.85)')
  })
  it('returns hot color for >= 80', () => {
    expect(tempToColor(80)).toBe('hsla(25, 55%, 45%, 0.85)')
  })
  it('returns warm color for >= 65', () => {
    expect(tempToColor(65)).toBe('hsla(40, 55%, 45%, 0.85)')
  })
  it('returns cool color for >= 45', () => {
    expect(tempToColor(45)).toBe('hsla(160, 35%, 42%, 0.85)')
  })
  it('returns cold color for < 45', () => {
    expect(tempToColor(30)).toBe('hsla(195, 35%, 42%, 0.85)')
  })
})
