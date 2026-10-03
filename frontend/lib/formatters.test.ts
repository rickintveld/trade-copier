import { describe, expect, it } from 'vitest';
import { formatDateTime, formatLatency, formatTimestamp } from '@/lib/formatters';

describe('formatters', () => {
  it('formats SQLite UTC strings in UTC (TZ=UTC in tests)', () => {
    expect(formatTimestamp('2026-01-15 08:30:05')).toBe('08:30:05');
    expect(formatDateTime('2026-01-15 08:30:05')).toBe('2026-01-15 08:30:05');
  });

  it('accepts Date instances', () => {
    expect(formatDateTime(new Date('2026-01-15T08:30:05Z'))).toBe('2026-01-15 08:30:05');
  });

  it('formats latency in microseconds', () => {
    expect(formatLatency(250)).toBe('250μs');
  });
});
