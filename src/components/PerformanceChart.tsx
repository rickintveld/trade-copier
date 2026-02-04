import React, { useMemo } from 'react';
import { Worker, Position, ErrorLog, SystemMetrics } from '@/types/trading';
import { formatUptime } from '@/lib/utils';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { ChartContainer, ChartTooltip, ChartTooltipContent, ChartLegend, ChartLegendContent } from '@/components/ui/chart';
import { BarChart, Bar, XAxis, YAxis, CartesianGrid, Cell } from 'recharts';
import { formatDistanceToNow } from 'date-fns';
import { CheckCircle2, XCircle, Clock, AlertTriangle } from 'lucide-react';
import { DependencyStatus } from '@/hooks/useDependencyStatus';

interface PerformanceChartProps {
  workers: Worker[];
  positions: Position[];
  errors: ErrorLog[];
  metrics: SystemMetrics;
  dependencyStatus: DependencyStatus | null;
}

const PerformanceChart: React.FC<PerformanceChartProps> = ({ workers, positions, errors, metrics, dependencyStatus }) => {
  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'installed':
        return <CheckCircle2 className="w-4 h-4 text-green-500" />;
      case 'installing':
        return <Clock className="w-4 h-4 text-yellow-500 animate-pulse" />;
      case 'error':
        return <XCircle className="w-4 h-4 text-red-500" />;
      case 'pending':
      default:
        return <AlertTriangle className="w-4 h-4 text-orange-500" />;
    }
  };

  const getStatusText = (status: string) => {
    return status.charAt(0).toUpperCase() + status.slice(1);
  };
  // Worker status distribution
  const workerStatusData = useMemo(() => {
    const statusCounts = workers.reduce((acc, worker) => {
      acc[worker.status] = (acc[worker.status] || 0) + 1;
      return acc;
    }, {} as Record<string, number>);

    return Object.entries(statusCounts).map(([status, count]) => ({
      name: status.charAt(0).toUpperCase() + status.slice(1),
      value: count,
      status,
    }));
  }, [workers]);

  // Worker latency data - show all workers
  const workerLatencyData = useMemo(() => {
    return workers
      .map(worker => ({
        name: worker.name,
        latency: worker.latency,
        status: worker.status,
      }))
      .sort((a, b) => b.latency - a.latency);
  }, [workers]);

  // Errors by worker - include all workers
  const errorsByWorkerData = useMemo(() => {
    // Start with all workers at 0 errors
    const errorCounts: Record<string, number> = {};
    workers.forEach(worker => {
      errorCounts[worker.name] = 0;
    });

    // Add actual error counts
    errors.forEach(error => {
      const workerName = error.workerName || 'Unknown';
      errorCounts[workerName] = (errorCounts[workerName] || 0) + 1;
    });

    return Object.entries(errorCounts)
      .map(([worker, count]) => ({
        worker,
        errors: count,
      }))
      .sort((a, b) => b.errors - a.errors);
  }, [errors, workers]);

  // Trade type distribution
  const tradeTypeData = useMemo(() => {
    const typeCounts = positions.reduce((acc, position) => {
      if (position.type) {
        const type = position.type.charAt(0).toUpperCase() + position.type.slice(1);
        acc[type] = (acc[type] || 0) + 1;
      }
      return acc;
    }, {} as Record<string, number>);

    return Object.entries(typeCounts).map(([type, count]) => ({
      name: type,
      value: count,
    }));
  }, [positions]);

  // Symbol distribution
  const symbolData = useMemo(() => {
    const symbolCounts = positions.reduce((acc, position) => {
      if (position.symbol) {
        acc[position.symbol] = (acc[position.symbol] || 0) + 1;
      }
      return acc;
    }, {} as Record<string, number>);

    return Object.entries(symbolCounts)
      .map(([symbol, count]) => ({
        symbol,
        count,
      }))
      .sort((a, b) => b.count - a.count)
      .slice(0, 10); // Top 10 symbols
  }, [positions]);

  const statusColors: Record<string, string> = {
    active: 'hsl(var(--chart-1))',
    inactive: 'hsl(var(--chart-4))',
    error: 'hsl(var(--chart-3))',
    installing: 'hsl(var(--chart-5))',
  };

  const severityColors: Record<string, string> = {
    warning: 'hsl(var(--chart-1))',
    error: 'hsl(var(--chart-2))',
    critical: 'hsl(var(--chart-3))',
  };

  const chartConfig = {
    active: { label: 'Active', color: 'hsl(var(--chart-1))' },
    inactive: { label: 'Inactive', color: 'hsl(var(--chart-4))' },
    error: { label: 'Error', color: 'hsl(var(--chart-3))' },
    installing: { label: 'Installing', color: 'hsl(var(--chart-5))' },
    latency: { label: 'Latency (ms)', color: 'hsl(var(--chart-1))' },
    positions: { label: 'Positions', color: 'hsl(var(--chart-2))' },
    buy: { label: 'Buy', color: 'hsl(var(--chart-2))' },
    sell: { label: 'Sell', color: 'hsl(var(--chart-3))' },
  };

  return (
    <div className="flex flex-col h-full">
      <div className="flex-1 overflow-auto scrollbar-thin p-4">
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-4 max-w-7xl mx-auto">
        {/* System Metrics Card */}
        <Card className="glass-card border-border/50">
          <CardHeader>
            <CardTitle className="text-lg">System Overview</CardTitle>
            <CardDescription>Current system performance metrics</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-1">
                <p className="text-sm text-muted-foreground">Router Status</p>
                <p className="text-2xl font-bold">{metrics.routerStatus}</p>
              </div>
              <div className="space-y-1">
                <p className="text-sm text-muted-foreground">Active Workers</p>
                <p className="text-2xl font-bold">{metrics.activeConnections}/{metrics.totalWorkers}</p>
              </div>
              <div className="space-y-1">
                <p className="text-sm text-muted-foreground">Total Signals</p>
                <p className="text-2xl font-bold">{metrics.totalTrades}</p>
              </div>
              <div className="space-y-1">
                <p className="text-sm text-muted-foreground">Avg Latency</p>
                <p className="text-2xl font-bold">{metrics.avgLatency}ms</p>
              </div>
              <div className="space-y-1">
                <p className="text-sm text-muted-foreground">Port</p>
                <p className="text-2xl font-bold">{metrics.listeningPort}</p>
              </div>
              <div className="space-y-1">
                <p className="text-sm text-muted-foreground">Uptime</p>
                <p className="text-2xl font-bold">
                  {formatUptime(metrics.uptime).value}{formatUptime(metrics.uptime).suffix}
                </p>
              </div>
            </div>
            
            {dependencyStatus && (
              <>
                <div className="border-t border-border/50 pt-4">
                  <div className="space-y-2">
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-2">
                        {getStatusIcon(dependencyStatus.package_manager_status)}
                        <span className="text-sm text-muted-foreground">{dependencyStatus.package_manager_name}</span>
                      </div>
                      <span className="text-sm font-medium">{getStatusText(dependencyStatus.package_manager_status)}</span>
                    </div>
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-2">
                        {getStatusIcon(dependencyStatus.wine_status)}
                        <span className="text-sm text-muted-foreground">Wine</span>
                      </div>
                      <span className="text-sm font-medium">{getStatusText(dependencyStatus.wine_status)}</span>
                    </div>
                    {dependencyStatus.error_message && (
                      <div className="mt-2 p-2 bg-destructive/10 border border-destructive/20 rounded text-xs text-destructive">
                        {dependencyStatus.error_message}
                      </div>
                    )}
                  </div>
                </div>
              </>
            )}
          </CardContent>
        </Card>

        {/* Worker Status Distribution */}
        {workerStatusData.length > 0 && (
          <Card className="glass-card border-border/50">
            <CardHeader>
              <CardTitle className="text-lg">Worker Status Distribution</CardTitle>
              <CardDescription>{workers.length} total workers</CardDescription>
            </CardHeader>
            <CardContent>
              <ChartContainer config={chartConfig} className="h-[250px]">
                <BarChart data={workerStatusData}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis dataKey="name" />
                  <YAxis allowDecimals={false} />
                  <ChartTooltip content={<ChartTooltipContent />} />
                  <Bar dataKey="value" radius={[4, 4, 0, 0]} maxBarSize={60} fillOpacity={0.5}>
                    {workerStatusData.map((entry, index) => (
                      <Cell key={`cell-${index}`} fill={statusColors[entry.status] || 'hsl(var(--chart-5))'} />
                    ))}
                  </Bar>
                </BarChart>
              </ChartContainer>
            </CardContent>
          </Card>
        )}

        {/* Trade Type Distribution */}
        <Card className="glass-card border-border/50">
          <CardHeader>
            <CardTitle className="text-lg">Trade Type Distribution</CardTitle>
            <CardDescription>Buy vs Sell signals</CardDescription>
          </CardHeader>
          <CardContent>
            {tradeTypeData.length > 0 ? (
              <ChartContainer config={chartConfig} className="h-[250px]">
                <BarChart data={tradeTypeData}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis dataKey="name" />
                  <YAxis allowDecimals={false} />
                  <ChartTooltip content={<ChartTooltipContent />} />
                  <Bar dataKey="value" radius={[4, 4, 0, 0]} maxBarSize={60} fillOpacity={0.5}>
                    {tradeTypeData.map((entry, index) => (
                      <Cell key={`cell-${index}`} fill={entry.name === 'Buy' ? 'hsl(var(--chart-2))' : 'hsl(var(--chart-3))'} />
                    ))}
                  </Bar>
                </BarChart>
              </ChartContainer>
            ) : (
              <div className="h-[250px] flex items-center justify-center text-muted-foreground">
                No signal data available
              </div>
            )}
          </CardContent>
        </Card>

        {/* Top Symbols */}
        <Card className="glass-card border-border/50">
          <CardHeader>
            <CardTitle className="text-lg">Top Traded Symbols</CardTitle>
            <CardDescription>Most actively traded symbols</CardDescription>
          </CardHeader>
          <CardContent>
            {symbolData.length > 0 ? (
              <ChartContainer config={chartConfig} className="h-[250px]">
                <BarChart data={symbolData}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis dataKey="symbol" />
                  <YAxis allowDecimals={false} />
                  <ChartTooltip content={<ChartTooltipContent />} />
                  <Bar 
                    dataKey="count" 
                    fill="hsl(var(--chart-2))" 
                    radius={[4, 4, 0, 0]}
                    maxBarSize={60}
                    fillOpacity={0.5}
                  />
                </BarChart>
              </ChartContainer>
            ) : (
              <div className="h-[250px] flex items-center justify-center text-muted-foreground">
                No signal data available
              </div>
            )}
          </CardContent>
        </Card>

        {/* Worker Latency */}
        <Card className="glass-card border-border/50">
          <CardHeader>
            <CardTitle className="text-lg">Worker Latency</CardTitle>
            <CardDescription>Response time per worker (ms)</CardDescription>
          </CardHeader>
          <CardContent>
            <ChartContainer config={chartConfig} className="h-[250px]">
              <BarChart data={workerLatencyData}>
                <CartesianGrid strokeDasharray="3 3" />
                <XAxis dataKey="name" angle={-45} textAnchor="end" height={80} />
                <YAxis domain={[0, 'auto']} />
                <ChartTooltip content={<ChartTooltipContent />} />
                <Bar dataKey="latency" fill="hsl(var(--chart-1))" radius={[4, 4, 0, 0]} maxBarSize={60} fillOpacity={0.5} />
              </BarChart>
            </ChartContainer>
          </CardContent>
        </Card>

        {/* Errors by Worker */}
        <Card className="glass-card border-border/50">
          <CardHeader>
            <CardTitle className="text-lg">Errors by Worker</CardTitle>
            <CardDescription>Error count per worker</CardDescription>
          </CardHeader>
          <CardContent>
            <ChartContainer config={chartConfig} className="h-[250px]">
              <BarChart data={errorsByWorkerData}>
                <CartesianGrid strokeDasharray="3 3" />
                <XAxis dataKey="worker" angle={-45} textAnchor="end" height={80} />
                <YAxis allowDecimals={false} />
                <ChartTooltip content={<ChartTooltipContent />} />
                <Bar dataKey="errors" fill="hsl(var(--chart-3))" radius={[4, 4, 0, 0]} maxBarSize={60} fillOpacity={0.5} />
              </BarChart>
            </ChartContainer>
          </CardContent>
        </Card>
        </div>
      </div>
    </div>
  );
};

export default PerformanceChart;
