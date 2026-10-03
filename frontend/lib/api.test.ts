import { invoke } from '@tauri-apps/api/core';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { tradeCopierApi } from '@/lib/api';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

const invokeMock = vi.mocked(invoke);

// Tauri 2 maps camelCase JS argument keys to snake_case Rust parameters.
// Sending snake_case keys silently yields None/default on the Rust side.
describe('tradeCopierApi argument casing', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue({ success: true, data: [] });
  });

  it('sends symbolPrefix when creating a worker', async () => {
    await tradeCopierApi.createWorker({ name: 'a', address: '127.0.0.1:5051', multiplier: 1, symbol_prefix: '.m' });
    expect(invokeMock).toHaveBeenCalledWith('create_instance', expect.objectContaining({ symbolPrefix: '.m' }));
  });

  it('sends symbolPrefix when updating a worker', async () => {
    await tradeCopierApi.updateWorker(3, { name: 'a', address: '127.0.0.1:5051', multiplier: 1, symbol_prefix: '.m' });
    expect(invokeMock).toHaveBeenCalledWith('update_instance', expect.objectContaining({ id: 3, symbolPrefix: '.m' }));
  });

  it('sends workerId for profit history', async () => {
    await tradeCopierApi.getProfitHistory(5, 10);
    expect(invokeMock).toHaveBeenCalledWith('get_profit_history', { workerId: 5, limit: 10 });
  });

  it('never sends snake_case argument keys', async () => {
    await tradeCopierApi.createWorker({ name: 'a', address: 'h:1', multiplier: 1 });
    await tradeCopierApi.updateWorker(1, { name: 'a', address: 'h:1', multiplier: 1 });
    await tradeCopierApi.getProfitHistory();
    for (const [, args] of invokeMock.mock.calls) {
      for (const key of Object.keys((args ?? {}) as object)) {
        expect(key).not.toMatch(/_/);
      }
    }
  });
});
