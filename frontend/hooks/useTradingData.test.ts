import { renderHook, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { tradeCopierApi, type ApiSystemMetrics, type ApiWorker } from '@/lib/api';
import { useTradingData } from '@/hooks/useTradingData';

vi.mock('@/lib/api', () => ({
  tradeCopierApi: {
    getWorkers: vi.fn(),
    getErrors: vi.fn(),
    getTrades: vi.fn(),
    getProfitHistory: vi.fn(),
    getSystemMetrics: vi.fn(),
  },
}));

const api = vi.mocked(tradeCopierApi);

const worker: ApiWorker = {
  id: 1,
  name: 'Slave A',
  address: '127.0.0.1:5051',
  multiplier: 1,
  mt5_connected: true,
  state: 'active',
  last_error: null,
  latency_us: 1000,
  symbol_prefix: 'none',
  created_at: '2026-01-15 08:00:00',
  updated_at: '2026-01-15 08:00:00',
};

const metrics: ApiSystemMetrics = {
  id: 1,
  active_workers: 1,
  router_port: 5000,
  router_status: 'online',
  total_trades: 42,
  avg_latency_ms: 1.2,
  uptime_seconds: 60,
  total_workers: 1,
  provider_connected: true,
  created_at: '2026-01-15 08:00:00',
};

describe('useTradingData', () => {
  beforeEach(() => {
    api.getWorkers.mockResolvedValue([worker]);
    api.getErrors.mockResolvedValue([]);
    api.getTrades.mockResolvedValue([]);
    api.getProfitHistory.mockResolvedValue([]);
    api.getSystemMetrics.mockResolvedValue(metrics);
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('loads and transforms backend data', async () => {
    const { result } = renderHook(() => useTradingData());

    await waitFor(() => expect(result.current.isConnected).toBe(true));
    expect(result.current.workers).toHaveLength(1);
    expect(result.current.workers[0].port).toBe(5051);
    expect(result.current.metrics).toMatchObject({ routerStatus: 'online', totalTrades: 42, providerConnected: true });
  });

  it('still connects when only metrics fail', async () => {
    api.getSystemMetrics.mockRejectedValue(new Error('no metrics'));
    const { result } = renderHook(() => useTradingData());

    await waitFor(() => expect(result.current.isConnected).toBe(true));
    expect(result.current.metrics.routerStatus).toBe('offline');
  });

  it('clears data and reports disconnected when the backend is unreachable', async () => {
    api.getWorkers.mockRejectedValue(new Error('ipc down'));
    vi.spyOn(console, 'error').mockImplementation(() => {});
    const { result } = renderHook(() => useTradingData());

    await waitFor(() => expect(api.getWorkers).toHaveBeenCalled());
    expect(result.current.isConnected).toBe(false);
    expect(result.current.workers).toEqual([]);
  });
});
