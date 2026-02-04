import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

/**
 * Formats uptime (in seconds) to a human-readable string.
 * - Less than 1 minute: displays seconds (e.g., "30/s")
 * - 1 minute to 1 hour: displays minutes (e.g., "5/m")
 * - 1 hour or more: displays hours (e.g., "2/h")
 */
export function formatUptime(uptime: number): { value: number; suffix: string } {
  const seconds = uptime;
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);

  if (hours >= 1) {
    return { value: hours, suffix: '/h' };
  } else if (minutes >= 1) {
    return { value: minutes, suffix: '/m' };
  } else {
    return { value: seconds, suffix: '/s' };
  }
}
