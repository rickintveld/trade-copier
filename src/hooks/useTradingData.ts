import { Worker, ErrorLog, SystemMetrics, Position, Profit } from '@/types/trading';
import { tradeCopierApi, ApiWorker, ApiErrorLog, ApiTrade, ApiProfit } from '@/lib/api';
import { useState, useEffect, useCallback } from 'react';

// Helper to parse UTC timestamp from database
const parseUTCTimestamp = (dateString: string): Date => {
  // SQLite CURRENT_TIMESTAMP returns UTC, append 'Z' to ensure proper parsing
  const utcDateString = dateString.endsWith('Z') ? dateString : `${dateString}Z`;
  return new Date(utcDateString);
};

// Transform API worker to dashboard worker
function transformWorker(apiWorker: ApiWorker): Worker {
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
    latency: apiWorker.latency_us > 0 ? Math.round(apiWorker.latency_us / 1000) : 0,
    symbolPrefix: apiWorker.symbol_prefix
  };
}

// Transform API error to dashboard error
function transformError(apiError: ApiErrorLog): ErrorLog {
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
function transformTradeToPosition(apiTrade: ApiTrade): Position | null {

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
function transformProfit(apiProfit: ApiProfit): Profit {
  return {
    id: apiProfit.id,
    workerId: apiProfit.worker_id,
    workerName: apiProfit.worker_name,
    workerAddress: apiProfit.worker_address,
    profit: apiProfit.profit,
    createdAt: parseUTCTimestamp(apiProfit.created_at),
  };
}

interface UseTradingDataReturn {
  workers: Worker[];
  positions: Position[];
  errors: ErrorLog[];
  metrics: SystemMetrics;
  profits: Profit[];
  isConnected: boolean;
  lastUpdate: Date;
}

export const useTradingData = (): UseTradingDataReturn => {
  const [workers, setWorkers] = useState<Worker[]>([]);
  const [positions, setPositions] = useState<Position[]>([]);
  const [errors, setErrors] = useState<ErrorLog[]>([]);
  const [profits, setProfits] = useState<Profit[]>([]);
  const [metrics, setMetrics] = useState<SystemMetrics>({
    routerStatus: 'offline',
    listeningPort: 5000,
    activeConnections: 0,
    totalTrades: 0,
    avgLatency: 0,
    totalWorkers: 0,
    uptime: 0,
    providerConnected: false,
  });
  const [isConnected, setIsConnected] = useState(false);
  const [lastUpdate, setLastUpdate] = useState(new Date());

  const refreshData = useCallback(async () => {
    try {
      // Fetch workers from API
      const apiWorkers = await tradeCopierApi.getWorkers();
      const transformedWorkers = apiWorkers.map(transformWorker);
      setWorkers(transformedWorkers);
      
      // Fetch errors from API
      const apiErrors = await tradeCopierApi.getErrors(50);
      const transformedErrors = apiErrors.map(transformError);
      setErrors(transformedErrors);
      
      // Fetch trades from API and convert open trades to positions
      const apiTrades = await tradeCopierApi.getTrades(100);
      const transformedPositions = apiTrades
        .map(transformTradeToPosition)
        .filter((pos): pos is Position => pos !== null);
      setPositions(transformedPositions);
      
      // Fetch profit history
      try {
        const apiProfits = await tradeCopierApi.getProfitHistory();
        const transformedProfits = apiProfits.map(transformProfit);
        setProfits(transformedProfits);
      } catch (profitError) {
        if (import.meta.env.DEV) {
          console.warn('Failed to fetch profit history:', profitError);
        }
      }
      
      // Fetch system metrics from API
      try {
        const apiMetrics = await tradeCopierApi.getSystemMetrics();
        setMetrics({
          routerStatus: apiMetrics.router_status,
          listeningPort: apiMetrics.router_port,
          activeConnections: apiMetrics.active_workers,
          totalTrades: apiMetrics.total_trades,
          avgLatency: apiMetrics.avg_latency_ms,
          uptime: apiMetrics.uptime_seconds,
          totalWorkers: apiMetrics.total_workers,
          providerConnected: apiMetrics.provider_connected,
        });
      } catch (metricsError) {
        if (import.meta.env.DEV) {
          console.warn('Failed to fetch system metrics:', metricsError);
        }
      }
      
      setIsConnected(true);
      setLastUpdate(new Date());
    } catch (error) {
      if (import.meta.env.DEV) {
        console.error('Failed to fetch trading data:', error);
      }
      setIsConnected(false);
      
      // Clear all data when API is offline
      setWorkers([]);
      setPositions([]);
      setErrors([]);
      setMetrics({
        routerStatus: 'offline',
        listeningPort: 5000,
        activeConnections: 0,
        totalTrades: 0,
        avgLatency: 0,
        uptime: 0,
        totalWorkers: 0,
        providerConnected: false,
      });
    }
  }, []);

  useEffect(() => {
    refreshData();
    const interval = setInterval(refreshData, 2000);
    return () => clearInterval(interval);
  }, [refreshData]);

  return {
    workers,
    positions,
    errors,
    metrics,
    profits,
    isConnected,
    lastUpdate,
  };
};
