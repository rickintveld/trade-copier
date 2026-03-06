import React, { useState, useEffect, useRef } from 'react';
import { useTradingData } from '@/hooks/useTradingData';
import { useDependencyStatus } from '@/hooks/useDependencyStatus';
import { useFeatureToggles } from '@/hooks/useFeatureToggles';
import DashboardHeader from '@/components/DashboardHeader';
import WorkersPanel from '@/components/WorkersPanel';
import MetricsPanel from '@/components/MetricsPanel';
import PositionTable from '@/components/PositionTable';
import ErrorLogPanel from '@/components/ErrorLogPanel';
import PerformanceChart from '@/components/PerformanceChart';
import ProfitChart from '@/components/ProfitChart';
import EconomicCalendar from '@/components/EconomicCalendar';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { TableProperties, AlertCircle, BarChart3, TrendingUp, CalendarDays } from 'lucide-react';
import { Toaster } from '@/components/ui/sonner';
import { toast } from 'sonner';
import { invoke } from '@tauri-apps/api/core';

interface EconomicEvent {
  title: string;
  impact: 'low' | 'medium' | 'high';
  instrument: string;
  restriction: boolean;
  eventType: 'normal' | 'all-day';
  date: string;
}

const formatDateForApi = (date: Date, isEnd: boolean = false) => {
  const d = new Date(date);
  if (isEnd) {
    d.setHours(23, 59, 59, 0);
  } else {
    d.setHours(0, 0, 0, 0);
  }

  const offset = -d.getTimezoneOffset();
  const sign = offset >= 0 ? '+' : '-';
  const hours = String(Math.floor(Math.abs(offset) / 60)).padStart(2, '0');
  const minutes = String(Math.abs(offset) % 60).padStart(2, '0');

  const year = d.getFullYear();
  const month = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  const hour = String(d.getHours()).padStart(2, '0');
  const min = String(d.getMinutes()).padStart(2, '0');
  const sec = String(d.getSeconds()).padStart(2, '0');

  return `${year}-${month}-${day}T${hour}:${min}:${sec}${sign}${hours}:${minutes}`;
};

const Dashboard: React.FC = () => {
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const { workers, positions, errors, metrics, profits, isConnected, lastUpdate } = useTradingData();
  const { status: dependencyStatus } = useDependencyStatus();
  const { isEnabled } = useFeatureToggles();
  const notifiedErrorsRef = useRef<Set<string>>(new Set());
  const initialErrorsLoadedRef = useRef(false);
  const notifiedEventsRef = useRef<Set<string>>(new Set());

  // Show toast for critical errors (guarded by feature toggle)
  useEffect(() => {
    if (!isEnabled('error_notifications')) return;
    if (errors.length === 0) return;

    // On initial load, mark all existing errors as already notified
    if (!initialErrorsLoadedRef.current) {
      initialErrorsLoadedRef.current = true;
      errors.filter(e => e.severity === 'critical').forEach(error => {
        const errorId = `${error.id}-${error.timestamp}-${error.message}`;
        notifiedErrorsRef.current.add(errorId);
      });
      return;
    }

    // Only show toasts for critical errors we haven't notified about yet
    errors.filter(e => e.severity === 'critical').forEach(error => {
      const errorId = `${error.id}-${error.timestamp}-${error.message}`;
      if (!notifiedErrorsRef.current.has(errorId)) {
        notifiedErrorsRef.current.add(errorId);
        toast.error(error.message, {
          description: error.workerName 
            ? `Worker: ${error.workerName}` 
            : undefined,
        });
      }
    });
  }, [errors, isEnabled]);

  // Show toast for upcoming high-impact economic events (guarded by feature toggle)
  useEffect(() => {
    if (!isEnabled('news_notifications')) return;

    const fetchAndCheck = async () => {
      try {
        const now = new Date();
        const endOfDay = new Date(now);
        endOfDay.setHours(23, 59, 59, 0);

        const dateFrom = formatDateForApi(now);
        const dateTo = formatDateForApi(endOfDay, true);

        const response = await invoke<{ success: boolean; data: { items: EconomicEvent[] } }>(
          'fetch_economic_calendar',
          { dateFrom, dateTo }
        );

        if (!response.success || !response.data?.items) return;

        const threshold = new Date(now.getTime() + 15 * 60 * 1000);

        response.data.items.forEach((event) => {
          if (event.eventType === 'all-day') return;
          if (event.impact !== 'high' && !event.restriction) return;

          const eventTime = new Date(event.date);
          if (eventTime <= now || eventTime > threshold) return;

          const eventId = `${event.date}-${event.title}`;
          if (notifiedEventsRef.current.has(eventId)) return;

          notifiedEventsRef.current.add(eventId);

          const minutesUntil = Math.round((eventTime.getTime() - now.getTime()) / 60000);

          if (event.restriction) {
            toast.error(event.title, {
              description: `${event.instrument} — No Trading in ${minutesUntil} min`,
            });
          } else {
            toast.error(event.title, {
              description: `${event.instrument} — High impact in ${minutesUntil} min`,
            });
          }
        });
      } catch (err) {
        if (import.meta.env.DEV) {
          console.warn('Failed to fetch calendar for notifications:', err);
        }
      }
    };

    fetchAndCheck();
    const interval = setInterval(fetchAndCheck, 60_000);
    return () => clearInterval(interval);
  }, [isEnabled]);

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
          <MetricsPanel metrics={metrics} profits={profits} lastUpdate={lastUpdate} />

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
                  value="profits"
                  className="data-[state=active]:bg-primary/10 data-[state=active]:text-primary data-[state=active]:shadow-none gap-2 px-4"
                >
                  <TrendingUp className="w-4 h-4" />
                  Profits
                </TabsTrigger>
                <TabsTrigger 
                  value="calendar"
                  className="data-[state=active]:bg-primary/10 data-[state=active]:text-primary data-[state=active]:shadow-none gap-2 px-4"
                >
                  <CalendarDays className="w-4 h-4" />
                  Calendar
                </TabsTrigger>
                <TabsTrigger 
                  value="performance"
                  className="data-[state=active]:bg-primary/10 data-[state=active]:text-primary data-[state=active]:shadow-none gap-2 px-4"
                >
                  <BarChart3 className="w-4 h-4" />
                  Performance
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
              <ProfitChart profits={profits} />
            </TabsContent>

            <TabsContent value="calendar" className="flex-1 m-0 overflow-hidden">
              <EconomicCalendar />
            </TabsContent>
          </Tabs>
        </main>
      </div>
    </div>
  );
};

export default Dashboard;
