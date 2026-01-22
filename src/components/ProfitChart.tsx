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
  
  // Prepare per-worker chart data with individual Y-axis configs
  const workerChartData = useMemo(() => {
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
    
    // Create data for each worker with optimized Y-axis
    return Array.from(balancesByWorker.entries()).map(([workerId, workerBalances]) => {
      const balanceValues = workerBalances.map(b => b.balance);
      const minBalance = Math.min(...balanceValues);
      const maxBalance = Math.max(...balanceValues);
      
      // Calculate optimal Y-axis range
      const range = maxBalance - minBalance;
      const padding = range * 0.1; // 10% padding
      const min = Math.floor((minBalance - padding) / 1000) * 1000;
      const max = Math.ceil((maxBalance + padding) / 1000) * 1000;
      
      // Generate ticks
      const tickCount = 6;
      const tickInterval = (max - min) / (tickCount - 1);
      const ticks = [];
      for (let i = 0; i < tickCount; i++) {
        ticks.push(Math.round(min + tickInterval * i));
      }
      
      // Create chart data points
      const data = workerBalances.map((balance, index) => ({
        index,
        label: `#${index + 1}`,
        balance: balance.balance,
        date: format(balance.createdAt, 'MMM dd, yyyy'),
        time: format(balance.createdAt, 'HH:mm:ss'),
      }));
      
      return {
        workerId,
        workerName: workerBalances[0].workerName,
        data,
        yAxisConfig: { min, max, ticks },
      };
    });
  }, [balances]);
  
  
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
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-4 max-w-7xl mx-auto">
          {/* Per-Worker Charts */}
          {workerChartData.map((workerData, idx) => {
            const chartConfig = {
              balance: {
                label: 'Balance',
                color: CHART_COLORS[idx % CHART_COLORS.length],
              },
            };
            
            return (
              <Card key={workerData.workerId} className="glass-card border-border/50">
                <CardHeader>
                  <CardTitle className="text-lg flex items-center gap-2">
                    <div 
                      className="w-3 h-3 rounded-full" 
                      style={{ backgroundColor: CHART_COLORS[idx % CHART_COLORS.length] }}
                    />
                    {workerData.workerName}
                  </CardTitle>
                  <div className="space-y-2 pt-2">
                    <div className="flex justify-between text-sm">
                      <span className="text-muted-foreground">Start Balance:</span>
                      <span className="font-medium">${workerData.data[0].balance.toFixed(2)}</span>
                    </div>
                    <div className="flex justify-between text-sm">
                      <span className="text-muted-foreground">Current Balance:</span>
                      <span className="font-medium">${workerData.data[workerData.data.length - 1].balance.toFixed(2)}</span>
                    </div>
                    <div className="flex justify-between text-sm border-t border-border/50 pt-2">
                      <span className="text-muted-foreground">Profit/Loss:</span>
                      <span 
                        className={`font-bold ${
                          (workerData.data[workerData.data.length - 1].balance - workerData.data[0].balance) >= 0 
                            ? 'text-green-500' 
                            : 'text-red-500'
                        }`}
                      >
                        ${(workerData.data[workerData.data.length - 1].balance - workerData.data[0].balance).toFixed(2)} 
                        ({(((workerData.data[workerData.data.length - 1].balance - workerData.data[0].balance) / workerData.data[0].balance) * 100).toFixed(2)}%)
                      </span>
                    </div>
                  </div>
                </CardHeader>
                <CardContent className="pt-6">
                  <ChartContainer config={chartConfig} className="h-[300px]">
                    <LineChart data={workerData.data}>
                      <CartesianGrid strokeDasharray="3 3" />
                      <XAxis 
                        dataKey="label" 
                        tick={{ fontSize: 12 }}
                        label={{ value: 'Trade Number', position: 'insideBottom', offset: -5 }}
                      />
                      <YAxis 
                        tick={{ fontSize: 12 }}
                        tickFormatter={(value) => `$${value.toLocaleString()}`}
                        domain={[workerData.yAxisConfig.min, workerData.yAxisConfig.max]}
                        ticks={workerData.yAxisConfig.ticks}
                      />
                      <ChartTooltip 
                        content={<ChartTooltipContent />}
                        labelFormatter={(label) => `Trade ${label}`}
                        formatter={(value: any, name: string, props: any) => {
                          return [
                            `$${parseFloat(value).toFixed(2)}`,
                            `${props.payload.date} ${props.payload.time}`
                          ];
                        }}
                      />
                      <Line
                        type="monotone"
                        dataKey="balance"
                        stroke={CHART_COLORS[idx % CHART_COLORS.length]}
                        strokeWidth={2}
                        dot={{ r: 3 }}
                        name="Balance"
                      />
                    </LineChart>
                  </ChartContainer>
                </CardContent>
              </Card>
            );
          })}
        </div>
      </div>
    </div>
  );
};

export default ProfitChart;
