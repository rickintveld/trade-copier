import React, { useState } from 'react';
import { useTradingData } from '@/hooks/useTradingData';
import { useDependencyStatus } from '@/hooks/useDependencyStatus';
import DashboardHeader from '@/components/DashboardHeader';
import WorkersPanel from '@/components/WorkersPanel';
import MetricsPanel from '@/components/MetricsPanel';
import PositionTable from '@/components/PositionTable';
import ErrorLogPanel from '@/components/ErrorLogPanel';
import PerformanceChart from '@/components/PerformanceChart';
import ProfitChart from '@/components/ProfitChart';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { TableProperties, AlertCircle, BarChart3, TrendingUp } from 'lucide-react';
import { Toaster } from '@/components/ui/sonner';
import { toast } from 'sonner';

const Dashboard: React.FC = () => {
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const { workers, positions, errors, metrics, accountBalances, isConnected, lastUpdate } = useTradingData();
  const { status: dependencyStatus } = useDependencyStatus();
  const previousErrorsRef = React.useRef<typeof errors>([]);

  // Show toast for critical errors
  React.useEffect(() => {
    // Skip on initial mount - don't show toasts for existing errors
    if (previousErrorsRef.current.length === 0 && errors.length > 0) {
      previousErrorsRef.current = errors;
      return;
    }

    const criticalErrors = errors.filter(e => e.severity === 'critical');
    const previousCriticalErrors = previousErrorsRef.current.filter(e => e.severity === 'critical');
    
    // Find errors that exist in current but not in previous
    const newErrors = criticalErrors.filter(error => {
      const errorId = `${error.timestamp}-${error.message}-${error.workerName || ''}`;
      return !previousCriticalErrors.some(prevError => {
        const prevErrorId = `${prevError.timestamp}-${prevError.message}-${prevError.workerName || ''}`;
        return prevErrorId === errorId;
      });
    });

    if (newErrors.length > 0) {
      newErrors.forEach(error => {
        toast.error(error.message, {
          description: error.workerName 
            ? `Worker: ${error.workerName}` 
            : undefined,
        });
      });
    }

    // Update ref to current errors for next comparison
    previousErrorsRef.current = errors;
  }, [errors]);

  return (
    <div className="flex flex-col h-screen bg-background">
      <Toaster 
        position="top-right" 
        theme="dark"
        toastOptions={{
          classNames: {
            toast: 'glass-card border-border/50',
            error: 'border-l-4 border-l-destructive',
          },
        }}
      />
      
      <DashboardHeader 
        isConnected={isConnected} 
        onMenuClick={() => setSidebarCollapsed(!sidebarCollapsed)}
      />

      <div className="flex flex-1 overflow-hidden">
        <WorkersPanel
          workers={workers}
          isCollapsed={sidebarCollapsed}
          onToggleCollapse={() => setSidebarCollapsed(!sidebarCollapsed)}
        />

        <main className="flex-1 flex flex-col overflow-hidden">
          <MetricsPanel metrics={metrics} lastUpdate={lastUpdate} />

          <Tabs defaultValue="signals" className="flex-1 flex flex-col overflow-hidden">
            <div className="px-4 border-b border-border/50 bg-card/30">
              <TabsList className="h-12 bg-transparent border-0 gap-1">
                <TabsTrigger 
                  value="signals"
                  className="data-[state=active]:bg-primary/10 data-[state=active]:text-primary data-[state=active]:shadow-none gap-2 px-4"
                >
                  <TableProperties className="w-4 h-4" />
                  Signals
                </TabsTrigger>
                <TabsTrigger 
                  value="performance"
                  className="data-[state=active]:bg-primary/10 data-[state=active]:text-primary data-[state=active]:shadow-none gap-2 px-4"
                >
                  <BarChart3 className="w-4 h-4" />
                  Performance
                </TabsTrigger>
                <TabsTrigger 
                  value="profits"
                  className="data-[state=active]:bg-primary/10 data-[state=active]:text-primary data-[state=active]:shadow-none gap-2 px-4"
                >
                  <TrendingUp className="w-4 h-4" />
                  Profits
                </TabsTrigger>
                <TabsTrigger 
                  value="errors"
                  className="data-[state=active]:bg-primary/10 data-[state=active]:text-primary data-[state=active]:shadow-none gap-2 px-4 relative"
                >
                  <AlertCircle className="w-4 h-4" />
                  Errors
                  {errors.filter(e => e.severity === 'critical').length > 0 && (
                    <span className="absolute -top-1 -right-1 w-4 h-4 rounded-full bg-destructive text-[10px] font-bold flex items-center justify-center">
                      {errors.filter(e => e.severity === 'critical').length}
                    </span>
                  )}
                </TabsTrigger>
              </TabsList>
            </div>

            <TabsContent value="signals" className="flex-1 m-0 overflow-hidden">
              <PositionTable 
                positions={positions} 
                workers={workers.map(w => ({ id: w.id, name: w.name }))}
              />
            </TabsContent>

            <TabsContent value="errors" className="flex-1 m-0 overflow-hidden">
              <ErrorLogPanel errors={errors} />
            </TabsContent>

            <TabsContent value="performance" className="flex-1 m-0 overflow-hidden">
              <PerformanceChart 
                workers={workers}
                positions={positions}
                errors={errors}
                metrics={metrics}
                dependencyStatus={dependencyStatus}
              />
            </TabsContent>

            <TabsContent value="profits" className="flex-1 m-0 overflow-hidden">
              <ProfitChart balances={accountBalances} />
            </TabsContent>
          </Tabs>
        </main>
      </div>
    </div>
  );
};

export default Dashboard;
