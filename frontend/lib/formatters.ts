import { formatDistanceToNow, format } from 'date-fns';

// Helper function to convert UTC date string to local timezone
const parseUTCDate = (dateString: string): Date => {
  // SQLite CURRENT_TIMESTAMP returns UTC, so we append 'Z' if not present
  const utcDateString = dateString.endsWith('Z') ? dateString : `${dateString}Z`;
  return new Date(utcDateString);
};

export const formatTimestamp = (date: Date | string): string => {
  const localDate = typeof date === 'string' ? parseUTCDate(date) : date;
  return format(localDate, 'HH:mm:ss');
};

export const formatDateTime = (date: Date | string): string => {
  const localDate = typeof date === 'string' ? parseUTCDate(date) : date;
  return format(localDate, 'yyyy-MM-dd HH:mm:ss');
};

export const formatRelativeTime = (date: Date | string): string => {
  const localDate = typeof date === 'string' ? parseUTCDate(date) : date;
  return formatDistanceToNow(localDate, { addSuffix: true });
};

export const formatPrice = (price: number, symbol: string): number => {
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
