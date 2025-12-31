import { formatDistanceToNow, format } from 'date-fns';

export const formatTimestamp = (date: Date): string => {
  return format(date, 'HH:mm:ss');
};

export const formatDateTime = (date: Date): string => {
  return format(date, 'yyyy-MM-dd HH:mm:ss');
};

export const formatRelativeTime = (date: Date): string => {
  return formatDistanceToNow(date, { addSuffix: true });
};

export const formatPrice = (price: number, symbol: string): number => {
  return price;
};

export const formatLots = (lots: number): number => {
  return lots;
};

export const formatLatency = (ms: number): string => {
  return `${ms}ms`;
};

export const cn = (...classes: (string | undefined | false)[]): string => {
  return classes.filter(Boolean).join(' ');
};
