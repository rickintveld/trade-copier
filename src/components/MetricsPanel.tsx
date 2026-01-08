import React from 'react';
import { SystemMetrics } from '@/types/trading';
import { 
  Radio, 
  Network, 
  TrendingUp, 
  Clock, 
  Gauge, 
  Layers,
  Wifi
} from 'lucide-react';

interface MetricsPanelProps {
  metrics: SystemMetrics;
  lastUpdate: Date;
}

interface MetricCardProps {
  icon: React.ReactNode;
  label: string;
  value: string | number;
  suffix?: string;
  highlight?: boolean;
}

const MetricCard: React.FC<MetricCardProps> = ({ 
  icon, 
  label, 
  value, 
  suffix,
  highlight 
}) => (
  <div className="glass-card p-4 flex items-center gap-4 hover:bg-accent/20 transition-colors">
    <div className={`p-2.5 rounded-lg ${highlight ? 'bg-primary/20' : 'bg-muted'}`}>
      {icon}
    </div>
    <div>
      <p className="metric-label">{label}</p>
      <p className="metric-value">
        {value}
        {suffix && <span className="text-sm text-muted-foreground ml-1">{suffix}</span>}
      </p>
    </div>
  </div>
);

const MetricsPanel: React.FC<MetricsPanelProps> = ({ metrics, lastUpdate }) => {
  return (
    <div className="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-7 gap-3 p-4 bg-muted/10 border-b border-border/50">
      <div className="glass-card p-4 flex items-center gap-4 col-span-2 md:col-span-1">
        <div className={`relative p-2.5 rounded-lg ${
          metrics.routerStatus === 'online' ? 'bg-status-active/20' : 'bg-status-error/20'
        }`}>
          <Radio className={`w-5 h-5 ${
            metrics.routerStatus === 'online' ? 'text-status-active' : 'text-status-error'
          }`} />
          {metrics.routerStatus === 'online' && (
            <div className="absolute top-1 right-1 w-2 h-2 rounded-full bg-status-active animate-pulse" />
          )}
        </div>
        <div>
          <p className="metric-label">Router</p>
          <p className={`text-sm font-semibold ${
            metrics.routerStatus === 'online' ? 'text-status-active' : 'text-status-error'
          }`}>
            Port {metrics.listeningPort}
          </p>
        </div>
      </div>

      <MetricCard
        icon={<Layers className="w-5 h-5 text-muted-foreground" />}
        label="Workers"
        value={metrics.totalWorkers}
      />

      <MetricCard
        icon={<Network className="w-5 h-5 text-primary" />}
        label="Connections"
        value={metrics.activeConnections}
        highlight
      />

      <MetricCard
        icon={<TrendingUp className="w-5 h-5 text-success" />}
        label="Signals"
        value={metrics.totalTrades}
      />

      <MetricCard
        icon={<Gauge className="w-5 h-5 text-warning" />}
        label="Avg Latency"
        value={metrics.avgLatency}
        suffix="ms"
      />

      <MetricCard
        icon={<Clock className="w-5 h-5 text-primary" />}
        label="Uptime"
        value={metrics.uptime}
        suffix="/s"
      />

      <div className="glass-card p-4 flex items-center gap-4">
        <div className="p-2.5 rounded-lg bg-primary/10">
          <Wifi className="w-5 h-5 text-primary animate-pulse" />
        </div>
        <div>
          <p className="metric-label">Last Update</p>
          <p className="font-mono text-sm text-foreground">
            {lastUpdate.toLocaleTimeString()}
          </p>
        </div>
      </div>
    </div>
  );
};

export default MetricsPanel;
