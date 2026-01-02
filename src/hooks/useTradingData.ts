import { Worker, ErrorLog, SystemMetrics, Position } from '@/types/trading';
import { tradeCopierApi, ApiWorker, ApiErrorLog, ApiTrade } from '@/lib/api';
import { useState, useEffect, useCallback } from 'react';

// Transform API worker to dashboard worker
function transformWorker(apiWorker: ApiWorker): Worker {
  const [address, port] = apiWorker.address.split(':');
  
  return {
    id: `worker-${apiWorker.id.toString().padStart(3, '0')}`,
    name: apiWorker.name,
    status: apiWorker.state,
    tcpAddress: address,
    port: parseInt(port, 10) || 0,
    riskMultiplier: apiWorker.multiplier,
    mt5Connected: apiWorker.mt5_connected,
    lastConnectionTime: new Date(apiWorker.created_at),
    lastActivity: new Date(apiWorker.updated_at),
    latency: apiWorker.latency_us > 0 ? Math.round(apiWorker.latency_us / 1000) : 0,
  };
}

// Transform API error to dashboard error
function transformError(apiError: ApiErrorLog): ErrorLog {
  return {
    id: `ERR-${apiError.id}`,
    timestamp: new Date(apiError.created_at),
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
    openTime: new Date(apiTrade.created_at),
    slavePositionIds: [`SLAVE-${apiTrade.worker_id}-${apiTrade.trade_id}`],
  };
}

interface UseTradingDataReturn {
  workers: Worker[];
  positions: Position[];
  errors: ErrorLog[];
  metrics: SystemMetrics;
  isConnected: boolean;
  lastUpdate: Date;
}

export const useTradingData = (): UseTradingDataReturn => {
  const [workers, setWorkers] = useState<Worker[]>([]);
  const [positions, setPositions] = useState<Position[]>([]);
  const [errors, setErrors] = useState<ErrorLog[]>([]);
  const [metrics, setMetrics] = useState<SystemMetrics>({
    routerStatus: 'offline',
    listeningPort: 5000,
    activeConnections: 0,
    totalTrades: 0,
    avgLatency: 0,
    totalWorkers: 0,
    uptime: 0,
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
        });
      } catch (metricsError) {
        console.warn('Failed to fetch system metrics:', metricsError);
      }
      
      setIsConnected(true);
      setLastUpdate(new Date());
    } catch (error) {
      console.error('Failed to fetch trading data:', error);
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
    isConnected,
    lastUpdate,
  };
};
