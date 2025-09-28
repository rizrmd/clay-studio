export function formatTimestamp(
  date: Date | string | undefined | null,
  options: {
    showDate?: boolean;
    showTime?: boolean;
    showSeconds?: boolean;
    relative?: boolean;
  } = {}
): string {
  const {
    showDate = false,
    showTime = true,
    showSeconds = false,
    relative = false,
  } = options;

  if (!date) return "";

  const dateObj = typeof date === "string" ? new Date(date) : date;

  if (isNaN(dateObj.getTime())) {
    return "";
  }

  if (relative) {
    return getRelativeTime(dateObj);
  }

  const parts: string[] = [];

  if (showDate) {
    parts.push(
      dateObj.toLocaleDateString("en-US", {
        year: "numeric",
        month: "short",
        day: "numeric",
      })
    );
  }

  if (showTime) {
    const timeOptions: Intl.DateTimeFormatOptions = {
      hour: "2-digit",
      minute: "2-digit",
      hour12: false,
    };

    if (showSeconds) {
      timeOptions.second = "2-digit";
    }

    parts.push(dateObj.toLocaleTimeString("en-US", timeOptions));
  }

  return parts.join(" ");
}

export function getRelativeTime(date: Date | string): string {
  const dateObj = typeof date === "string" ? new Date(date) : date;
  const now = new Date();
  const diffMs = now.getTime() - dateObj.getTime();
  const diffSec = Math.floor(diffMs / 1000);
  const diffMin = Math.floor(diffSec / 60);
  const diffHour = Math.floor(diffMin / 60);
  const diffDay = Math.floor(diffHour / 24);

  if (diffSec < 60) {
    return "just now";
  } else if (diffMin < 60) {
    return `${diffMin} minute${diffMin === 1 ? "" : "s"} ago`;
  } else if (diffHour < 24) {
    return `${diffHour} hour${diffHour === 1 ? "" : "s"} ago`;
  } else if (diffDay < 7) {
    return `${diffDay} day${diffDay === 1 ? "" : "s"} ago`;
  } else {
    return formatTimestamp(dateObj, { showDate: true, showTime: false });
  }
}

export function formatFullDateTime(date: Date | string | undefined | null): string {
  return formatTimestamp(date, { showDate: true, showTime: true });
}

export function formatTime(date: Date | string | undefined | null): string {
  return formatTimestamp(date, { showDate: false, showTime: true });
}

export function formatDate(date: Date | string | undefined | null): string {
  return formatTimestamp(date, { showDate: true, showTime: false });
}