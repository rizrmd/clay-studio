import React from "react";
import { cn } from "@/lib/utils";
import {
  formatTimestamp,
  formatTime,
  formatDate,
  formatFullDateTime,
  getRelativeTime,
} from "@/lib/utils/timestamp";

interface TimestampProps extends React.HTMLAttributes<HTMLTimeElement> {
  date: Date | string | undefined | null;
  format?: "time" | "date" | "datetime" | "relative" | "custom";
  showDate?: boolean;
  showTime?: boolean;
  showSeconds?: boolean;
  relative?: boolean;
  showTooltip?: boolean;
}

export function Timestamp({
  date,
  format = "time",
  showDate,
  showTime,
  showSeconds = false,
  relative,
  showTooltip = true,
  className,
  ...props
}: TimestampProps) {
  if (!date) return null;

  const dateObj = typeof date === "string" ? new Date(date) : date;

  let displayText = "";
  switch (format) {
    case "time":
      displayText = formatTime(dateObj);
      break;
    case "date":
      displayText = formatDate(dateObj);
      break;
    case "datetime":
      displayText = formatFullDateTime(dateObj);
      break;
    case "relative":
      displayText = getRelativeTime(dateObj);
      break;
    case "custom":
      displayText = formatTimestamp(dateObj, {
        showDate,
        showTime,
        showSeconds,
        relative,
      });
      break;
    default:
      displayText = formatTime(dateObj);
  }

  const tooltipText = showTooltip ? formatFullDateTime(dateObj) : undefined;

  return (
    <time
      dateTime={dateObj.toISOString()}
      title={tooltipText}
      className={cn("text-xs text-muted-foreground", className)}
      {...props}
    >
      {displayText}
    </time>
  );
}