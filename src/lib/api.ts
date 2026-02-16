// API client for trade-copier backend using Tauri
import { invoke } from '@tauri-apps/api/tauri';

export interface ApiResponse<T> {
  success: boolean;
  data: T;
}

export interface ApiErrorResponse {
  success: false;
  error: string;
}

// API response types matching the backend
export interface ApiWorker {
  id: number;
  name: string;
  address: string;
  multiplier: number;
  mt5_connected: boolean;
  state: 'active' | 'error' | 'inactive' | 'installing';
  last_error: string | null;
  latency_us: number | null;
  symbol_prefix: string;
  created_at: string;
  updated_at: string;
}

export interface ApiTrade {
  id: number;
  trade_id: number;
  worker_id: number;
  worker_name: string;
  worker_address: string;
  symbol: string;
  trade_type: 'buy' | 'sell',
  lots: number;
  price: number;
  sl: number;
  tp: number;
  cmd: 'open' | 'close' | 'modify';
  created_at: string;
}

export interface ApiErrorLog {
  id: number;
  worker_id: number;
  worker_name: string;
  worker_address: string;
  severity: 'warning' | 'error' | 'critical';
  error_message: string;
  created_at: string;
}

export interface ApiSystemMetrics {
  id: number;
  active_workers: number;
  router_port: number;
  router_status: 'online' | 'offline';
  total_trades: number;
  avg_latency_ms: number;
  uptime_seconds: number;
  total_workers: number;
  provider_connected: boolean;
  created_at: string;
}

export interface ApiProfit {
  id: number;
  worker_id: number;
  worker_name: string;
  worker_address: string;
  profit: number;
  created_at: string;
}

async function tauriInvoke<T>(command: string, args?: any): Promise<ApiResponse<T>> {
  try {
    const response = await invoke<ApiResponse<T>>(command, args);
    return response;
  } catch (error) {
    throw new Error(typeof error === 'string' ? error : 'Unknown error occurred');
  }
}

export const tradeCopierApi = {
  async getWorkers(): Promise<ApiWorker[]> {
    const response = await tauriInvoke<ApiWorker[]>('get_workers');
    return response.data;
  },

  async getTrades(limit: number = 100): Promise<ApiTrade[]> {
    const response = await tauriInvoke<ApiTrade[]>('get_trades', { limit });
    return response.data;
  },

  async getErrors(limit: number = 100): Promise<ApiErrorLog[]> {
    const response = await tauriInvoke<ApiErrorLog[]>('get_errors', { limit });
    return response.data;
  },

  async getSystemMetrics(): Promise<ApiSystemMetrics> {
    const response = await tauriInvoke<ApiSystemMetrics>('get_system_metrics');
    return response.data;
  },

  async healthCheck(): Promise<{ status: string; message: string }> {
    const response = await tauriInvoke<{ status: string; message: string }>('health_check');
    return response.data;
  },

  async startWorker(workerId: number, force?: boolean): Promise<void> {
    await tauriInvoke('start_instance', { id: workerId, force });
  },

  async stopWorker(workerId: number): Promise<void> {
    await tauriInvoke('stop_instance', { id: workerId });
  },

  async createWorker(data: { name: string; address: string; multiplier: number; symbol_prefix?: string }): Promise<ApiWorker> {
    const response = await tauriInvoke<ApiWorker>('create_instance', {
      name: data.name,
      address: data.address,
      multiplier: data.multiplier,
      symbol_prefix: data.symbol_prefix || '',
    });
    return response.data;
  },

  async deleteWorker(workerId: number): Promise<void> {
    await tauriInvoke('delete_instance', { id: workerId, force: true });
  },

  async getProfitHistory(workerId?: number, limit?: number): Promise<ApiProfit[]> {
    const response = await tauriInvoke<ApiProfit[]>('get_profit_history', { 
      worker_id: workerId || null, 
      limit: limit || 1000 
    });
    return response.data;
  },
};
