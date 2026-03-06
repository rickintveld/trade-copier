import React, { useState, useEffect, useMemo, useRef, useCallback } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Skeleton } from '@/components/ui/skeleton';
import { AlertTriangle, Calendar, Clock, Ban, Droplet, Flame } from 'lucide-react';
import { cn } from '@/lib/utils';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';

interface EconomicEvent {
  title: string;
  impact: 'low' | 'medium' | 'high';
  instrument: string;
  restriction: boolean;
  eventType: 'normal' | 'all-day';
  date: string;
  forecast: string | null;
  previous: string | null;
  actual: string | null;
  youtubeLink: string | null;
  articleLink: string | null;
}

interface ApiResponse {
  items: EconomicEvent[];
}

const getWeekDates = (date: Date = new Date()) => {
  const isSunday = date.getDay() === 0;
  const day = date.getDay();
  const diff = date.getDate() - day + (day === 0 ? -6 : 1);
  const monday = new Date(date.setDate(diff));
  monday.setHours(0, 0, 0, 0);

  const totalDays = isSunday ? 8 : 7; // Include next Monday on Sundays
  const days: Date[] = [];
  for (let i = 0; i < totalDays; i++) {
    const d = new Date(monday);
    d.setDate(monday.getDate() + i);
    days.push(d);
  }
  return days;
};

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

const formatTime = (dateString: string) => {
  const date = new Date(dateString);
  return date.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit', hour12: false });
};

const formatDayShort = (date: Date) => {
  return date.toLocaleDateString('en-US', { weekday: 'short' });
};

const formatDayNumber = (date: Date) => {
  return date.getDate();
};

const isSameDay = (date1: Date, date2: Date) => {
  return (
    date1.getFullYear() === date2.getFullYear() &&
    date1.getMonth() === date2.getMonth() &&
    date1.getDate() === date2.getDate()
  );
};

const isToday = (date: Date) => {
  return isSameDay(date, new Date());
};

const ImpactIndicator: React.FC<{ impact: 'low' | 'medium' | 'high' }> = ({ impact }) => {
  const colors = {
    low: 'bg-emerald-500',
    medium: 'bg-amber-500',
    high: 'bg-red-500',
  };

  return (
    <div className="flex gap-0.5 items-center">
      {[0, 1, 2].map((i) => (
        <div
          key={i}
          className={cn(
            'w-1.5 rounded-full transition-all',
            i === 0 ? 'h-2' : i === 1 ? 'h-3' : 'h-4',
            (impact === 'low' && i === 0) ||
            (impact === 'medium' && i <= 1) ||
            (impact === 'high')
              ? colors[impact]
              : 'bg-muted-foreground/20'
          )}
        />
      ))}
    </div>
  );
};

const CurrencyFlag: React.FC<{ currency: string }> = ({ currency }) => {
  const currencyFlags: Record<string, string> = {
    USD: '🇺🇸',
    EUR: '🇪🇺',
    GBP: '🇬🇧',
    JPY: '🇯🇵',
    AUD: '🇦🇺',
    CAD: '🇨🇦',
    CHF: '🇨🇭',
    NZD: '🇳🇿',
    CNY: '🇨🇳',
  };

  // Extract first currency if combined with +
  const displayCurrency = currency.includes('+') 
    ? currency.split('+')[0].trim() 
    : currency;

  const flag = currencyFlags[displayCurrency];
  
  // If we have a flag emoji, display it with a border
  if (flag) {
    return (
      <div 
        className="w-10 h-6 rounded border border-border/50 flex items-center justify-center text-xl bg-card/50"
        title={currency}
      >
        {flag}
      </div>
    );
  }

  // Fallback for non-country instruments using icons
  const getInstrumentIcon = () => {
    const upper = displayCurrency.toUpperCase();
    
    // Oil
    if (upper.includes('OIL') || upper.includes('CRUDE')) {
      return { icon: Droplet, color: 'bg-stone-700' };
    }
    // Natural gas
    if (upper.includes('GAS') || upper === 'NATGAS') {
      return { icon: Flame, color: 'bg-sky-600' };
    }
    
    return null;
  };

  const iconConfig = getInstrumentIcon();
  
  if (iconConfig) {
    const Icon = iconConfig.icon;
    return (
      <div 
        className={cn(
          'w-10 h-6 rounded flex items-center justify-center text-white',
          iconConfig.color
        )}
        title={currency}
      >
        <Icon className="w-4 h-4" />
      </div>
    );
  }

  // Final fallback: show first 3-4 chars in gray badge
  return (
    <div 
      className="w-10 h-6 rounded bg-gray-500 flex items-center justify-center text-[10px] font-bold text-white"
      title={currency}
    >
      <span className="truncate">{displayCurrency.slice(0, 4)}</span>
    </div>
  );
};

const isEventPast = (event: EconomicEvent): boolean => {
  const now = new Date();
  if (event.eventType === 'all-day') {
    const eventDate = new Date(event.date);
    const endOfDay = new Date(eventDate);
    endOfDay.setHours(23, 59, 59, 999);
    return endOfDay < now;
  }
  return new Date(event.date) < now;
};

const EventCard: React.FC<{ event: EconomicEvent }> = ({ event }) => {
  const isPast = isEventPast(event);
  const getValueColor = (actual: string | null, forecast: string | null, previous: string | null) => {
    if (!actual) return null;
    
    const parseValue = (val: string) => {
      const num = parseFloat(val.replace(/[^-\d.]/g, ''));
      return isNaN(num) ? null : num;
    };

    const actualNum = parseValue(actual);
    const forecastNum = forecast ? parseValue(forecast) : null;
    const previousNum = previous ? parseValue(previous) : null;

    if (actualNum === null) return null;
    
    const compareVal = forecastNum ?? previousNum;
    if (compareVal === null) return null;

    if (actualNum > compareVal) return 'text-emerald-400';
    if (actualNum < compareVal) return 'text-red-400';
    return 'text-muted-foreground';
  };

  const actualColor = getValueColor(event.actual, event.forecast, event.previous);

  return (
    <div className={cn(
      'group relative px-4 py-3 rounded-lg border transition-all',
      isPast
        ? 'opacity-40 border-border/30 bg-card/20'
        : 'hover:bg-card/80',
      !isPast && event.restriction 
        ? 'border-red-500/30 bg-red-500/5' 
        : !isPast && 'border-border/50 bg-card/50'
    )}>
      <div className="flex items-center gap-4">
        {/* Time */}
        <div className="flex items-center gap-2 w-20 shrink-0">
          <Clock className="w-3.5 h-3.5 text-muted-foreground" />
          <span className="text-sm text-muted-foreground font-mono">
            {event.eventType === 'all-day' ? 'All Day' : formatTime(event.date)}
          </span>
        </div>

        {/* Currency & Impact */}
        <div className="flex items-center gap-2 shrink-0">
          <CurrencyFlag currency={event.instrument} />
          <ImpactIndicator impact={event.impact} />
        </div>

        {/* Title */}
        <div className="flex-1 min-w-0 flex items-center gap-2">
          <h4 className={cn(
            'font-medium text-sm leading-tight truncate',
            event.restriction && 'text-red-200'
          )}>
            {event.title}
          </h4>
          {/* Restriction badge */}
          {event.restriction && (
            <Badge variant="destructive" className="gap-1 text-[10px] shrink-0">
              <Ban className="w-3 h-3" />
              No Trading
            </Badge>
          )}
        </div>

        {/* Values */}
        <div className="flex items-center gap-6 text-xs shrink-0">
          <div className="flex flex-col items-center w-16">
            <span className="text-muted-foreground text-[10px]">Previous</span>
            <span className="font-mono">{event.previous || '—'}</span>
          </div>
          <div className="flex flex-col items-center w-16">
            <span className="text-muted-foreground text-[10px]">Forecast</span>
            <span className="font-mono">{event.forecast || '—'}</span>
          </div>
          <div className="flex flex-col items-center w-16">
            <span className="text-muted-foreground text-[10px]">Actual</span>
            <span className={cn('font-mono font-semibold', actualColor)}>
              {event.actual || '—'}
            </span>
          </div>
        </div>
      </div>
    </div>
  );
};

const EconomicCalendar: React.FC = () => {
  const [events, setEvents] = useState<EconomicEvent[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selectedDate, setSelectedDate] = useState<Date | null>(null);

  const weekDates = useMemo(() => getWeekDates(), []);

  useEffect(() => {
    const fetchEvents = async () => {
      setLoading(true);
      setError(null);

      const startDate = weekDates[0];
      const endDate = weekDates[weekDates.length - 1];

      const dateFrom = formatDateForApi(startDate);
      const dateTo = formatDateForApi(endDate, true);

      try {
        const response = await invoke<{ success: boolean; data: ApiResponse }>(
          'fetch_economic_calendar',
          { dateFrom, dateTo }
        );

        if (response.success && response.data?.items) {
          setEvents(response.data.items);
        } else {
          throw new Error('Invalid response from API');
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        setLoading(false);
      }
    };

    fetchEvents();
  }, [weekDates]);

  const filteredEvents = useMemo(() => {
    if (!selectedDate) return events;
    return events.filter((event) => {
      const eventDate = new Date(event.date);
      return isSameDay(eventDate, selectedDate);
    });
  }, [events, selectedDate]);

  const groupedEvents = useMemo(() => {
    const groups: Record<string, EconomicEvent[]> = {};
    
    filteredEvents.forEach((event) => {
      const eventDate = new Date(event.date);
      const dateKey = eventDate.toISOString().split('T')[0];
      
      if (!groups[dateKey]) {
        groups[dateKey] = [];
      }
      groups[dateKey].push(event);
    });

    // Sort events within each group by time
    Object.keys(groups).forEach((key) => {
      groups[key].sort((a, b) => new Date(a.date).getTime() - new Date(b.date).getTime());
    });

    return groups;
  }, [filteredEvents]);

  const eventRefs = useRef<Map<string, HTMLDivElement>>(new Map());
  const lastScrolledKey = useRef<string | null>(null);
  const notifiedEventsRef = useRef<Set<string>>(new Set());

  // Show toast for upcoming high-impact or restriction events (within 15 minutes)
  useEffect(() => {
    if (loading || events.length === 0) return;

    const checkUpcomingEvents = () => {
      const now = new Date();
      const threshold = new Date(now.getTime() + 15 * 60 * 1000);

      events.forEach((event) => {
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
    };

    checkUpcomingEvents();
    const interval = setInterval(checkUpcomingEvents, 60_000);
    return () => clearInterval(interval);
  }, [loading, events]);

  const setEventRef = useCallback((key: string, el: HTMLDivElement | null) => {
    if (el) {
      eventRefs.current.set(key, el);
    } else {
      eventRefs.current.delete(key);
    }
  }, []);

  // Reset scroll tracking when the date filter changes
  useEffect(() => {
    lastScrolledKey.current = null;
  }, [selectedDate]);

  // Auto-scroll to the first upcoming (non-past) event
  // Only scroll when viewing "All" days or today — future days have no past
  // events so scrolling would jump past the day header.
  const shouldAutoScroll = selectedDate === null || isToday(selectedDate);

  useEffect(() => {
    if (!shouldAutoScroll || loading || Object.keys(groupedEvents).length === 0) return;

    const scrollToUpcoming = () => {
      const sortedEntries = Object.entries(groupedEvents)
        .sort(([a], [b]) => a.localeCompare(b));

      for (const [, dayEvents] of sortedEntries) {
        for (let i = 0; i < dayEvents.length; i++) {
          const event = dayEvents[i];
          if (!isEventPast(event)) {
            const key = `${event.date}-${event.title}-${i}`;
            if (key !== lastScrolledKey.current) {
              lastScrolledKey.current = key;
              const el = eventRefs.current.get(key);
              if (el) {
                el.scrollIntoView({ behavior: 'smooth', block: 'start' });
              }
            }
            return;
          }
        }
      }
    };

    // Small delay on initial load so DOM refs are populated
    const timeout = setTimeout(scrollToUpcoming, 200);

    // Re-check every 30 seconds so we scroll when an event becomes past
    const interval = setInterval(scrollToUpcoming, 30_000);

    return () => {
      clearTimeout(timeout);
      clearInterval(interval);
    };
  }, [loading, groupedEvents]);

  const restrictionCount = useMemo(() => {
    return filteredEvents.filter((e) => e.restriction).length;
  }, [filteredEvents]);

  const highImpactCount = useMemo(() => {
    return filteredEvents.filter((e) => e.impact === 'high').length;
  }, [filteredEvents]);

  if (error) {
    return (
      <div className="flex flex-col items-center justify-center h-full text-muted-foreground gap-4 p-8">
        <AlertTriangle className="w-12 h-12 text-destructive" />
        <p className="text-lg">Failed to load economic calendar</p>
        <p className="text-sm">{error}</p>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      {/* Header with date filters */}
      <div className="p-4 border-b border-border/50 bg-card/30">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-2">
            <Calendar className="w-5 h-5 text-primary" />
            <h2 className="text-lg font-semibold">Economic Calendar</h2>
          </div>
          <div className="flex items-center gap-4 text-sm">
            {highImpactCount > 0 && (
              <div className="flex items-center gap-1.5 text-red-400">
                <AlertTriangle className="w-4 h-4" />
                <span>{highImpactCount} High Impact</span>
              </div>
            )}
            {restrictionCount > 0 && (
              <div className="flex items-center gap-1.5 text-amber-400">
                <Ban className="w-4 h-4" />
                <span>{restrictionCount} Restrictions</span>
              </div>
            )}
          </div>
        </div>

        {/* Day filter buttons */}
        <div className="flex gap-2">
          <Button
            variant={selectedDate === null ? 'default' : 'outline'}
            size="sm"
            onClick={() => setSelectedDate(null)}
            className="flex flex-col items-center min-w-[60px] h-auto py-1.5 relative"
          >
            All
          </Button>
          {weekDates.map((date, index) => {
            const dayEvents = events.filter((e) => isSameDay(new Date(e.date), date));
            const hasRestriction = dayEvents.some((e) => e.restriction);
            const hasHighImpact = dayEvents.some((e) => e.impact === 'high');
            const isSelected = selectedDate && isSameDay(selectedDate, date);

            return (
              <Button
                key={index}
                variant={isSelected ? 'default' : 'outline'}
                size="sm"
                onClick={() => setSelectedDate(isSelected ? null : date)}
                className={cn(
                  'flex flex-col items-center min-w-[60px] h-auto py-1.5 relative',
                  isToday(date) && !isSelected && 'border-primary/50 bg-primary/5',
                  hasRestriction && !isSelected && 'border-red-500/30'
                )}
              >
                <span className="text-[10px] text-muted-foreground">{formatDayShort(date)}</span>
                <span className="text-sm font-semibold">{formatDayNumber(date)}</span>
                {(hasHighImpact || hasRestriction) && (
                  <div className="absolute -top-1 -right-1 flex gap-0.5">
                    {hasHighImpact && (
                      <div className="w-2 h-2 rounded-full bg-red-500" />
                    )}
                    {hasRestriction && (
                      <div className="w-2 h-2 rounded-full bg-amber-500" />
                    )}
                  </div>
                )}
              </Button>
            );
          })}
        </div>
      </div>

      {/* Events list */}
      <ScrollArea className="flex-1">
        <div className="p-4 space-y-6">
          {loading ? (
            <div className="space-y-4">
              {[...Array(6)].map((_, i) => (
                <Skeleton key={i} className="h-32 w-full rounded-lg" />
              ))}
            </div>
          ) : Object.keys(groupedEvents).length === 0 ? (
            <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
              <Calendar className="w-12 h-12 mb-4 opacity-50" />
              <p>No events for the selected period</p>
            </div>
          ) : (
            Object.entries(groupedEvents)
              .sort(([a], [b]) => a.localeCompare(b))
              .map(([dateKey, dayEvents]) => {
                const date = new Date(dateKey + 'T00:00:00');
                return (
                  <div key={dateKey}>
                    <div className="flex items-center gap-2 mb-3">
                      <h3 className={cn(
                        'text-sm font-semibold',
                        isToday(date) && 'text-primary'
                      )}>
                        {date.toLocaleDateString('en-US', {
                          weekday: 'long',
                          month: 'short',
                          day: 'numeric',
                        })}
                      </h3>
                      {isToday(date) && (
                        <Badge variant="default" className="text-[10px]">
                          Today
                        </Badge>
                      )}
                      <span className="text-xs text-muted-foreground">
                        ({dayEvents.length} events)
                      </span>
                    </div>
                    <div className="flex flex-col gap-3">
                      {dayEvents.map((event, index) => {
                        const eventKey = `${event.date}-${event.title}-${index}`;
                        return (
                          <div
                            key={eventKey}
                            ref={(el) => setEventRef(eventKey, el)}
                          >
                            <EventCard event={event} />
                          </div>
                        );
                      })}
                    </div>
                  </div>
                );
              })
          )}
        </div>
      </ScrollArea>
    </div>
  );
};

export default EconomicCalendar;
