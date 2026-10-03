import { describe, expect, it } from 'vitest';
import { parseUTCTimestamp } from '@/lib/time';

describe('parseUTCTimestamp', () => {
  it('treats SQLite CURRENT_TIMESTAMP values as UTC', () => {
    expect(parseUTCTimestamp('2026-01-15 08:30:00').toISOString()).toBe('2026-01-15T08:30:00.000Z');
  });

  it('does not append a second Z to ISO strings', () => {
    expect(parseUTCTimestamp('2026-01-15T08:30:00Z').toISOString()).toBe('2026-01-15T08:30:00.000Z');
  });
});
