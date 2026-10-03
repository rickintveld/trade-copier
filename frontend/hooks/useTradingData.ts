import { Worker, ErrorLog, SystemMetrics, Position, Profit } from '@/types/trading';
import { tradeCopierApi } from '@/lib/api';
import { transformWorker, transformError, transformTradeToPosition, transformProfit } from '@/lib/transforms';
import { useState, useEffect, useCallback } from 'react';

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
