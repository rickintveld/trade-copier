export type WorkerStatus = 'active' | 'inactive' | 'error' | 'installing';
export type TradeCommand = 'OPEN' | 'CLOSE' | 'MODIFY';
export type TradeType = 'buy' | 'sell';
export type ErrorSeverity = 'warning' | 'error' | 'critical';

export interface Worker {
  id: string;
  name: string;
  status: WorkerStatus;
  tcpAddress: string;
  port: number;
  riskMultiplier: number;
  mt5Connected: boolean;
  symbolPrefix: string;
  lastConnectionTime: Date;
  lastActivity: Date;
  latency: number;
}

export interface Trade {
  id: string;
  symbol: string;
  type: TradeType;
  command: TradeCommand;
  originalLots: number;
  price: number;
  sl: number;
  tp: number;
  timestamp: Date;
  workerResults: WorkerTradeResult[];
}

export interface WorkerTradeResult {
  workerId: string;
  workerName: string;
  adjustedLots: number;
  status: 'success' | 'pending' | 'failed';
  positionId?: string;
}

export interface Position {
  id: number;
  masterPositionId: string;
  workerId: string;
  workerName: string;
  symbol: string;
  type: TradeType;
  cmd: 'open' | 'modify' | 'close' | 'cancel';
  entryPrice: number;
  currentLots: number;
  sl: number;
  tp: number;
  openTime: Date;
  slavePositionIds: string[];
}

export interface ErrorLog {
  id: string;
  timestamp: Date;
  severity: ErrorSeverity;
  type: 'connection' | 'parse' | 'send' | 'channel';
  workerId?: string;
  workerName?: string;
  message: string;
  details?: string;
}

export interface SystemMetrics {
  routerStatus: 'online' | 'offline';
  listeningPort: number;
  activeConnections: number;
  totalTrades: number;
  avgLatency: number;
  totalWorkers: number;
  uptime: number;
}
