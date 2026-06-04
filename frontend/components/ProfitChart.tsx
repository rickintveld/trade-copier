import React, { useMemo, useState } from 'react';
import { Profit } from '@/types/trading';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { ChevronLeft, ChevronRight, Download } from 'lucide-react';
import { save } from '@tauri-apps/plugin-dialog';
import { writeTextFile } from '@tauri-apps/plugin-fs';
import {
  format,
  startOfMonth,
  endOfMonth,
  eachDayOfInterval,
  isSameMonth,
  addMonths,
  subMonths,
  startOfWeek,
  endOfWeek,
  isToday,
} from 'date-fns';
import { cn } from '@/lib/utils';

interface ProfitChartProps {
  profits: Profit[];
}

interface DayData {
  date: Date;
  trades: number;
  profit: number;
}

const WEEKDAYS = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];

const ProfitChart: React.FC<ProfitChartProps> = ({ profits }) => {
  const [currentMonth, setCurrentMonth] = useState(new Date());

  // Calculate daily profit data from profit records
  const dailyData = useMemo(() => {
    if (profits.length === 0) return new Map<string, DayData>();

    const dailyMap = new Map<string, DayData>();

    profits.forEach((profitRecord) => {
      const dateKey = format(profitRecord.createdAt, 'yyyy-MM-dd');

      if (!dailyMap.has(dateKey)) {
        dailyMap.set(dateKey, {
          date: new Date(dateKey),
          trades: 1,
          profit: profitRecord.profit,
        });
      } else {
        const existing = dailyMap.get(dateKey)!;
        existing.trades += 1;
        existing.profit += profitRecord.profit;
      }
    });

    return dailyMap;
  }, [profits]);

  // Get calendar days for the current month view
  const calendarDays = useMemo(() => {
    const monthStart = startOfMonth(currentMonth);
    const monthEnd = endOfMonth(currentMonth);
    const calendarStart = startOfWeek(monthStart);
    const calendarEnd = endOfWeek(monthEnd);

    return eachDayOfInterval({ start: calendarStart, end: calendarEnd });
  }, [currentMonth]);

  // Calculate monthly totals
  const monthlyTotals = useMemo(() => {
    let totalTrades = 0;
    let totalProfit = 0;

    calendarDays.forEach((day) => {
      if (isSameMonth(day, currentMonth)) {
        const dateKey = format(day, 'yyyy-MM-dd');
        const dayData = dailyData.get(dateKey);
        if (dayData) {
          totalTrades += dayData.trades;
          totalProfit += dayData.profit;
        }
      }
    });

    return { totalTrades, totalProfit };
  }, [calendarDays, currentMonth, dailyData]);

  const goToPreviousMonth = () => setCurrentMonth(subMonths(currentMonth, 1));
  const goToNextMonth = () => setCurrentMonth(addMonths(currentMonth, 1));
  const goToToday = () => setCurrentMonth(new Date());

  const handleExport = async () => {
    try {
      const rows = calendarDays
        .filter((day) => isSameMonth(day, currentMonth))
        .map((day) => {
          const dateKey = format(day, 'yyyy-MM-dd');
          const dayData = dailyData.get(dateKey);
          return [
            dateKey,
            dayData?.trades ?? 0,
            dayData?.profit?.toFixed(2) ?? '0.00',
          ].join(',');
        });

      const csv = ['Date,Trades,Profit', ...rows].join('\n');
      const monthLabel = format(currentMonth, 'yyyy-MM');

      const filePath = await save({
        defaultPath: `profits-${monthLabel}.csv`,
        filters: [{ name: 'CSV', extensions: ['csv'] }],
      });

      if (filePath) {
        await writeTextFile(filePath, csv);
      }
    } catch (error) {
      if (import.meta.env.DEV) {
        console.error('Failed to export CSV:', error);
      }
    }
  };

  if (profits.length === 0) {
    return (
      <div className="flex flex-col h-full items-center justify-center p-8">
        <div className="text-center space-y-2">
          <p className="text-lg text-muted-foreground">No profit data available</p>
          <p className="text-sm text-muted-foreground">
            Profits will appear here when trades are closed
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      <div className="flex-1 overflow-auto scrollbar-thin p-4">
        <Card className="glass-card border-border/50 max-w-8xl mx-auto">
          <CardHeader className="pb-2">
            <div className="flex items-center justify-between">
              <CardTitle className="text-xl">Profit Calendar</CardTitle>
              <div className="flex items-center gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleExport}
                  className="h-8 text-xs"
                >
                  <Download className="w-3 h-3 mr-1" />
                  Export
                </Button>
                <Button
                  variant="outline"
                  size="icon"
                  onClick={goToPreviousMonth}
                  className="h-8 w-8"
                >
                  <ChevronLeft className="h-4 w-4" />
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={goToToday}
                  className="h-8 px-3"
                >
                  Today
                </Button>
                <Button
                  variant="outline"
                  size="icon"
                  onClick={goToNextMonth}
                  className="h-8 w-8"
                >
                  <ChevronRight className="h-4 w-4" />
                </Button>
              </div>
            </div>
            <div className="flex items-center justify-between pt-2">
              <span className="text-lg font-medium">
                {format(currentMonth, 'MMMM yyyy')}
              </span>
              <div className="flex gap-4 text-sm">
                <span className="text-muted-foreground">
                  Trades: <span className="font-medium text-foreground">{monthlyTotals.totalTrades}</span>
                </span>
                <span className="text-muted-foreground">
                  Profit:{' '}
                  <span
                    className={cn(
                      'font-medium',
                      monthlyTotals.totalProfit >= 0 ? 'text-green-500' : 'text-red-500'
                    )}
                  >
                    ${monthlyTotals.totalProfit.toFixed(2)}
                  </span>
                </span>
              </div>
            </div>
          </CardHeader>
          <CardContent>
            {/* Weekday headers */}
            <div className="grid grid-cols-7 mb-2">
              {WEEKDAYS.map((day) => (
                <div
                  key={day}
                  className="text-center text-sm font-medium text-muted-foreground py-2"
                >
                  {day}
                </div>
              ))}
            </div>

            {/* Calendar grid */}
            <div className="grid grid-cols-7 gap-1">
              {calendarDays.map((day) => {
                const dateKey = format(day, 'yyyy-MM-dd');
                const dayData = dailyData.get(dateKey);
                const isCurrentMonth = isSameMonth(day, currentMonth);
                const isDayToday = isToday(day);
                const hasData = dayData && (dayData.trades > 0 || dayData.profit !== 0);
                const isProfit = dayData && dayData.profit > 0;
                const isLoss = dayData && dayData.profit < 0;

                return (
                  <div
                    key={dateKey}
                    className={cn(
                      'min-h-[100px] p-3 rounded-md border transition-colors',
                      !isCurrentMonth && 'opacity-30',
                      isDayToday && 'ring-2 ring-primary',
                      hasData && isProfit && 'bg-green-500/20 border-green-500/50',
                      hasData && isLoss && 'bg-red-500/20 border-red-500/50',
                      !hasData && 'border-border/50 bg-card/50'
                    )}
                  >
                    <div className="flex flex-col h-full">
                      <span
                        className={cn(
                          'text-base font-medium',
                          !isCurrentMonth && 'text-muted-foreground',
                          isDayToday && 'text-primary'
                        )}
                      >
                        {format(day, 'd')}
                      </span>
                      {hasData && isCurrentMonth && (
                        <div className="mt-auto space-y-1">
                          <div className="text-sm text-muted-foreground">
                            {dayData.trades} trade{dayData.trades !== 1 ? 's' : ''}
                          </div>
                          <div
                            className={cn(
                              'text-sm font-semibold',
                              isProfit && 'text-green-500',
                              isLoss && 'text-red-500'
                            )}
                          >
                            {dayData.profit >= 0 ? '+' : ''}
                            ${dayData.profit.toFixed(2)}
                          </div>
                        </div>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>

            {/* Legend */}
            <div className="flex items-center justify-center gap-6 mt-4 pt-4 border-t border-border/50">
              <div className="flex items-center gap-2">
                <div className="w-4 h-4 rounded bg-green-500/20 border border-green-500/50" />
                <span className="text-sm text-muted-foreground">Profit</span>
              </div>
              <div className="flex items-center gap-2">
                <div className="w-4 h-4 rounded bg-red-500/20 border border-red-500/50" />
                <span className="text-sm text-muted-foreground">Loss</span>
              </div>
              <div className="flex items-center gap-2">
                <div className="w-4 h-4 rounded border-2 border-primary" />
                <span className="text-sm text-muted-foreground">Today</span>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
};

export default ProfitChart;
