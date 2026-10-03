import { formatDistanceToNow, format } from 'date-fns';
import { parseUTCTimestamp } from '@/lib/time';

export const formatTimestamp = (date: Date | string): string => {
  const localDate = typeof date === 'string' ? parseUTCTimestamp(date) : date;
  return format(localDate, 'HH:mm:ss');
};

export const formatDateTime = (date: Date | string): string => {
  const localDate = typeof date === 'string' ? parseUTCTimestamp(date) : date;
  return format(localDate, 'yyyy-MM-dd HH:mm:ss');
};

export const formatRelativeTime = (date: Date | string): string => {
  const localDate = typeof date === 'string' ? parseUTCTimestamp(date) : date;
  return formatDistanceToNow(localDate, { addSuffix: true });
};

export const formatPrice = (price: number, _symbol: string): number => {
  return price;
};

export const formatLots = (lots: number): number => {
  return lots;
};

export const formatLatency = (ms: number): string => {
  return `${ms}μs`;
};

export const cn = (...classes: (string | undefined | false)[]): string => {
  return classes.filter(Boolean).join(' ');
};
