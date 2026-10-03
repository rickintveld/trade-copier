import { describe, expect, it } from 'vitest';
import type { ApiErrorLog, ApiProfit, ApiTrade, ApiWorker } from '@/lib/api';
import { transformError, transformProfit, transformTradeToPosition, transformWorker } from '@/lib/transforms';

const apiWorker: ApiWorker = {
  id: 7,
  name: 'Slave A',
  address: '127.0.0.1:5051',
  multiplier: 1.5,
  mt5_connected: true,
  state: 'active',
  last_error: null,
  latency_us: 2600,
  symbol_prefix: '.m',
  created_at: '2026-01-15 08:00:00',
  updated_at: '2026-01-15 09:00:00',
};

describe('transformWorker', () => {
  it('maps snake_case API fields to the dashboard worker', () => {
    const worker = transformWorker(apiWorker);
    expect(worker).toMatchObject({
      id: 7,
      name: 'Slave A',
      status: 'active',
      tcpAddress: '127.0.0.1',
      port: 5051,
      riskMultiplier: 1.5,
      mt5Connected: true,
      symbolPrefix: '.m',
    });
    expect(worker.lastActivity.toISOString()).toBe('2026-01-15T09:00:00.000Z');
  });

  it('converts latency from microseconds to rounded milliseconds', () => {
    expect(transformWorker(apiWorker).latency).toBe(3);
  });

  it('reports zero latency when the backend has no measurement', () => {
    expect(transformWorker({ ...apiWorker, latency_us: null }).latency).toBe(0);
    expect(transformWorker({ ...apiWorker, latency_us: 0 }).latency).toBe(0);
  });

  it('falls back to port 0 when the address has no port', () => {
    expect(transformWorker({ ...apiWorker, address: 'localhost' }).port).toBe(0);
  });
});

describe('transformError', () => {
  it('builds stable ids from the database ids', () => {
    const apiError: ApiErrorLog = {
      id: 12,
      worker_id: 3,
      worker_name: 'Slave B',
      worker_address: '127.0.0.1:5052',
      severity: 'critical',
      error_message: 'ack timeout',
      created_at: '2026-01-15 10:00:00',
    };
    expect(transformError(apiError)).toMatchObject({
      id: 'ERR-12',
      workerId: 'worker-003',
      severity: 'critical',
      message: 'ack timeout',
    });
  });
});

describe('transformTradeToPosition', () => {
  it('maps a trade to a position with master/slave references', () => {
    const apiTrade: ApiTrade = {
      id: 1,
      trade_id: 9001,
      worker_id: 4,
      worker_name: 'Slave C',
      worker_address: '127.0.0.1:5053',
      symbol: 'eurusd.m',
      trade_type: 'buy',
      lots: 0.2,
      price: 1.0845,
      sl: 1.08,
      tp: 1.09,
      cmd: 'open',
      created_at: '2026-01-15 11:00:00',
    };
    expect(transformTradeToPosition(apiTrade)).toMatchObject({
      masterPositionId: 'MASTER-9001',
      workerId: 'worker-004',
      symbol: 'EURUSD.M',
      type: 'buy',
      currentLots: 0.2,
      slavePositionIds: ['SLAVE-4-9001'],
    });
  });
});

describe('transformProfit', () => {
  it('parses the creation timestamp as UTC', () => {
    const apiProfit: ApiProfit = {
      id: 2,
      worker_id: 4,
      worker_name: 'Slave C',
      worker_address: '127.0.0.1:5053',
      profit: -12.5,
      created_at: '2026-01-15 12:00:00',
    };
    const profit = transformProfit(apiProfit);
    expect(profit.profit).toBe(-12.5);
    expect(profit.createdAt.toISOString()).toBe('2026-01-15T12:00:00.000Z');
  });
});
