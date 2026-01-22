import React, { useMemo } from 'react';
import { AccountBalance } from '@/types/trading';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { ChartContainer, ChartTooltip, ChartTooltipContent, ChartLegend, ChartLegendContent } from '@/components/ui/chart';
import { LineChart, Line, XAxis, YAxis, CartesianGrid, ResponsiveContainer } from 'recharts';
import { format } from 'date-fns';

interface ProfitChartProps {
  balances: AccountBalance[];
}

// Generate distinct colors for different workers
const CHART_COLORS = [
  'hsl(var(--chart-1))',
  'hsl(var(--chart-2))',
  'hsl(var(--chart-3))',
  'hsl(var(--chart-4))',
  'hsl(var(--chart-5))',
];

const ProfitChart: React.FC<ProfitChartProps> = ({ balances }) => {
  // Calculate Y-axis domain and ticks
  const yAxisConfig = useMemo(() => {
    if (balances.length === 0) return { min: 0, max: 100000, ticks: [] };
    
    const allBalances = balances.map(b => b.balance);
    const minBalance = Math.min(...allBalances);
    const maxBalance = Math.max(...allBalances);
    
    // Round down to nearest 10,000 for min
    const min = Math.floor(minBalance / 10000) * 10000;
    // Round up to nearest 10,000 for max
    const max = Math.ceil(maxBalance / 10000) * 10000;
    
    // Generate ticks every 10,000
    const ticks = [];
    for (let i = min; i <= max; i += 10000) {
      ticks.push(i);
    }
    
    return { min, max, ticks };
  }, [balances]);
  
  // Group balances by worker and prepare data for chart
  const chartData = useMemo(() => {
    if (balances.length === 0) return [];
    
    // Group balances by worker
    const balancesByWorker = new Map<number, AccountBalance[]>();
    balances.forEach(balance => {
      if (!balancesByWorker.has(balance.workerId)) {
        balancesByWorker.set(balance.workerId, []);
      }
      balancesByWorker.get(balance.workerId)!.push(balance);
    });
    
    // Sort each worker's balances by timestamp
    balancesByWorker.forEach((workerBalances) => {
      workerBalances.sort((a, b) => a.createdAt.getTime() - b.createdAt.getTime());
    });
    
    // Find the maximum number of data points any worker has
    let maxDataPoints = 0;
    balancesByWorker.forEach((workerBalances) => {
      maxDataPoints = Math.max(maxDataPoints, workerBalances.length);
    });
    
    // Create data points for each index (trade number)
    const data = [];
    for (let i = 0; i < maxDataPoints; i++) {
      const point: any = {
        index: i,
        label: `#${i + 1}`,
      };
      
      // Add balance for each worker at this index
      balancesByWorker.forEach((workerBalances, workerId) => {
        if (i < workerBalances.length) {
          const balance = workerBalances[i];
          point[`worker_${workerId}`] = balance.balance;
          point[`worker_${workerId}_date`] = format(balance.createdAt, 'MMM dd, yyyy');
          point[`worker_${workerId}_time`] = format(balance.createdAt, 'HH:mm:ss');
        } else if (i > 0 && workerBalances.length > 0) {
          // Carry forward the last known balance
          const lastBalance = workerBalances[workerBalances.length - 1];
          point[`worker_${workerId}`] = lastBalance.balance;
        }
      });
      
      data.push(point);
    }
    
    return data;
  }, [balances]);
  
  // Get unique workers for line rendering
  const workers = useMemo(() => {
    const workerMap = new Map<number, { id: number; name: string }>();
    balances.forEach(balance => {
      if (!workerMap.has(balance.workerId)) {
        workerMap.set(balance.workerId, {
          id: balance.workerId,
          name: balance.workerName,
        });
      }
    });
    return Array.from(workerMap.values());
  }, [balances]);
  
  // Calculate statistics per worker
  const workerStats = useMemo(() => {
    return workers.map((worker, idx) => {
      const workerBalances = balances.filter(b => b.workerId === worker.id);
      if (workerBalances.length === 0) return null;
      
      const startBalance = workerBalances[0].balance;
      const currentBalance = workerBalances[workerBalances.length - 1].balance;
      const profit = currentBalance - startBalance;
      const profitPercent = ((profit / startBalance) * 100).toFixed(2);
      
      return {
        workerName: worker.name,
        startBalance: startBalance.toFixed(2),
        currentBalance: currentBalance.toFixed(2),
        profit: profit.toFixed(2),
        profitPercent,
        color: CHART_COLORS[idx % CHART_COLORS.length],
      };
    }).filter(Boolean);
  }, [workers, balances]);
  
  const chartConfig = useMemo(() => {
    const config: any = {};
    workers.forEach((worker, idx) => {
      config[`worker_${worker.id}`] = {
        label: worker.name,
        color: CHART_COLORS[idx % CHART_COLORS.length],
      };
    });
    return config;
  }, [workers]);
  
  if (balances.length === 0) {
    return (
      <div className="flex flex-col h-full items-center justify-center p-8">
        <div className="text-center space-y-2">
          <p className="text-lg text-muted-foreground">No account balance data available</p>
          <p className="text-sm text-muted-foreground">
            Connect a MetaTrader 5 account to start tracking balance
          </p>
        </div>
      </div>
    );
  }
  
  return (
    <div className="flex flex-col h-full">
      <div className="flex-1 overflow-auto scrollbar-thin p-4">
        <div className="grid grid-cols-1 gap-4 max-w-7xl mx-auto">
          {/* Main Chart */}
          <Card className="glass-card border-border/50">
            <CardHeader>
              <CardTitle className="text-lg">Account Balance</CardTitle>
              <CardDescription>Track profit/loss for each connected account</CardDescription>
            </CardHeader>
            <CardContent>
              <ChartContainer config={chartConfig} className="h-[400px]">
                <LineChart data={chartData}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis 
                    dataKey="label" 
                    tick={{ fontSize: 12 }}
                    label={{ value: 'Trade Number', position: 'insideBottom', offset: -5 }}
                  />
                  <YAxis 
                    tick={{ fontSize: 12 }}
                    tickFormatter={(value) => `$${value.toLocaleString()}`}
                    domain={[yAxisConfig.min, yAxisConfig.max]}
                    ticks={yAxisConfig.ticks}
                  />
                  <ChartTooltip 
                    content={<ChartTooltipContent />}
                    labelFormatter={(label, payload) => {
                      if (payload && payload.length > 0) {
                        // Show the trade number as the label
                        return `Trade ${label}`;
                      }
                      return label;
                    }}
                    formatter={(value: any, name: string, props: any) => {
                      // Show balance with date/time for each worker
                      const workerId = name.replace('worker_', '');
                      const date = props.payload[`worker_${workerId}_date`];
                      const time = props.payload[`worker_${workerId}_time`];
                      return [
                        `$${parseFloat(value).toFixed(2)}`,
                        date && time ? `${props.name} (${date} ${time})` : props.name
                      ];
                    }}
                  />
                  <ChartLegend content={<ChartLegendContent />} />
                  {workers.map((worker, idx) => (
                    <Line
                      key={worker.id}
                      type="monotone"
                      dataKey={`worker_${worker.id}`}
                      stroke={CHART_COLORS[idx % CHART_COLORS.length]}
                      strokeWidth={2}
                      dot={false}
                      name={worker.name}
                    />
                  ))}
                </LineChart>
              </ChartContainer>
            </CardContent>
          </Card>
          
          {/* Worker Statistics */}
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {workerStats.map((stat: any) => (
              <Card key={stat.workerName} className="glass-card border-border/50">
                <CardHeader>
                  <CardTitle className="text-base flex items-center gap-2">
                    <div 
                      className="w-3 h-3 rounded-full" 
                      style={{ backgroundColor: stat.color }}
                    />
                    {stat.workerName}
                  </CardTitle>
                </CardHeader>
                <CardContent className="space-y-2">
                  <div className="flex justify-between text-sm">
                    <span className="text-muted-foreground">Start Balance:</span>
                    <span className="font-medium">${stat.startBalance}</span>
                  </div>
                  <div className="flex justify-between text-sm">
                    <span className="text-muted-foreground">Current Balance:</span>
                    <span className="font-medium">${stat.currentBalance}</span>
                  </div>
                  <div className="flex justify-between text-sm border-t border-border/50 pt-2">
                    <span className="text-muted-foreground">Profit/Loss:</span>
                    <span 
                      className={`font-bold ${
                        parseFloat(stat.profit) >= 0 
                          ? 'text-green-500' 
                          : 'text-red-500'
                      }`}
                    >
                      ${stat.profit} ({stat.profitPercent}%)
                    </span>
                  </div>
                </CardContent>
              </Card>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
};

export default ProfitChart;
