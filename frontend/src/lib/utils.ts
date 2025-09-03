import { type ClassValue, clsx } from "clsx"
import { twMerge } from "tailwind-merge"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

/**
 * Format processing time in milliseconds to a user-friendly string
 * @param processingTimeMs - Processing time in milliseconds
 * @returns Formatted time string (e.g., "2.3s", "1.2m", "850ms")
 */
export function formatProcessingTime(processingTimeMs?: number): string {
  if (!processingTimeMs || processingTimeMs <= 0) {
    return "";
  }

  const seconds = processingTimeMs / 1000;

  if (seconds >= 60) {
    // Show in minutes for longer times
    const minutes = seconds / 60;
    return `${minutes.toFixed(1)}m`;
  } else if (seconds >= 1) {
    // Show in seconds for times >= 1 second
    return `${seconds.toFixed(1)}s`;
  } else {
    // Show in milliseconds for very quick responses
    return `${processingTimeMs}ms`;
  }
}