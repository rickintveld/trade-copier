import { Worker, ErrorLog, Position, Profit } from '@/types/trading';
import { ApiWorker, ApiErrorLog, ApiTrade, ApiProfit } from '@/lib/api';
import { parseUTCTimestamp } from '@/lib/time';

// Transform API worker to dashboard worker
export function transformWorker(apiWorker: ApiWorker): Worker {
  const [address, port] = apiWorker.address.split(':');
  
  return {
    id: apiWorker.id,
    name: apiWorker.name,
    status: apiWorker.state,
    tcpAddress: address,
    port: parseInt(port, 10) || 0,
    riskMultiplier: apiWorker.multiplier,
    mt5Connected: apiWorker.mt5_connected,
    lastConnectionTime: parseUTCTimestamp(apiWorker.created_at),
    lastActivity: parseUTCTimestamp(apiWorker.updated_at),
    latency: apiWorker.latency_us && apiWorker.latency_us > 0 ? Math.round(apiWorker.latency_us / 1000) : 0,
    symbolPrefix: apiWorker.symbol_prefix
  };
}

// Transform API error to dashboard error
export function transformError(apiError: ApiErrorLog): ErrorLog {
  return {
    id: `ERR-${apiError.id}`,
    timestamp: parseUTCTimestamp(apiError.created_at),
    severity: apiError.severity,
    type: 'connection', // Default type, could be enhanced based on error message
    workerId: `worker-${apiError.worker_id.toString().padStart(3, '0')}`,
    workerName: apiError.worker_name,
    message: apiError.error_message,
  };
}

// Transform API trade (open command) to dashboard position
export function transformTradeToPosition(apiTrade: ApiTrade): Position | null {
  return {
    id: apiTrade.id,
    masterPositionId: `MASTER-${apiTrade.trade_id}`,
    workerId: `worker-${apiTrade.worker_id.toString().padStart(3, '0')}`,
    workerName: apiTrade.worker_name,
    symbol: apiTrade.symbol.toUpperCase(),
    type: apiTrade.trade_type,
    cmd: apiTrade.cmd,
    entryPrice: apiTrade.price,
    currentLots: apiTrade.lots,
    sl: apiTrade.sl,
    tp: apiTrade.tp,
    openTime: parseUTCTimestamp(apiTrade.created_at),
    slavePositionIds: [`SLAVE-${apiTrade.worker_id}-${apiTrade.trade_id}`],
  };
}

// Transform API profit to dashboard profit
export function transformProfit(apiProfit: ApiProfit): Profit {
  return {
    id: apiProfit.id,
    workerId: apiProfit.worker_id,
    workerName: apiProfit.worker_name,
    workerAddress: apiProfit.worker_address,
    profit: apiProfit.profit,
    createdAt: parseUTCTimestamp(apiProfit.created_at),
  };
}
