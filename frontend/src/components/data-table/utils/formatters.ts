import * as React from "react";
import { cn } from "@/lib/utils";

export function formatCellValue(
  value: any,
  dataType?: string,
  format?: string,
  currency?: string,
  currencyDisplay?: "symbol" | "code" | "name"
): React.ReactNode {
  if (value === null || value === undefined)
    return React.createElement("span", { className: "text-muted-foreground" }, "—");

  // Handle NaN specifically
  if (typeof value === "number" && isNaN(value))
    return React.createElement("span", { className: "text-muted-foreground" }, "—");

  switch (dataType) {
    case "date":
      try {
        const date = new Date(value);
        if (isNaN(date.getTime())) {
          return React.createElement("span", { className: "text-muted-foreground" }, "—");
        }
        return date.toLocaleDateString("en-US", {
          year: "numeric",
          month: "short",
          day: "numeric",
        });
      } catch {
        return React.createElement("span", { className: "text-muted-foreground" }, "—");
      }
    case "currency":
      const numValue = Number(value);
      if (isNaN(numValue)) {
        return React.createElement("span", { className: "text-muted-foreground" }, "—");
      }

      // Determine locale based on currency
      const getLocale = (curr: string) => {
        switch (curr) {
          case "USD":
            return "en-US";
          case "EUR":
            return "de-DE";
          case "GBP":
            return "en-GB";
          case "JPY":
            return "ja-JP";
          case "CNY":
            return "zh-CN";
          case "IDR":
            return "id-ID";
          case "SGD":
            return "en-SG";
          case "MYR":
            return "ms-MY";
          case "THB":
            return "th-TH";
          case "VND":
            return "vi-VN";
          case "PHP":
            return "en-PH";
          default:
            return "en-US";
        }
      };

      const locale = getLocale(currency || "USD");
      const currencyCode = currency || "USD";

      try {
        return new Intl.NumberFormat(locale, {
          style: "currency",
          currency: currencyCode,
          currencyDisplay: currencyDisplay || "symbol",
          minimumFractionDigits: ["IDR", "JPY", "VND"].includes(currencyCode)
            ? 0
            : 2,
          maximumFractionDigits: ["IDR", "JPY", "VND"].includes(currencyCode)
            ? 0
            : 2,
        }).format(numValue);
      } catch {
        // Fallback to simple formatting if Intl fails
        return `${currencyCode} ${numValue.toLocaleString()}`;
      }
    case "number":
      const num = Number(value);
      if (isNaN(num)) {
        return React.createElement("span", { className: "text-muted-foreground" }, "—");
      }
      if (format === "percentage") {
        return `${(num * 100).toFixed(2)}%`;
      }
      return num.toLocaleString();
    case "boolean":
      return React.createElement(
        "div",
        {
          className: cn(
            "inline-flex items-center justify-center rounded-full px-2 py-1 font-medium",
            value
              ? "bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400"
              : "bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400"
          )
        },
        value ? "Yes" : "No"
      );
    default:
      return String(value);
  }
}