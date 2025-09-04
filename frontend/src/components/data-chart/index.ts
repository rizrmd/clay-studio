export type {
  ChartType,
  ChartData,
  ChartOptions,
  ChartDisplayProps,
} from "./chart-types";
export { loadChartType, preloadCommonCharts } from "./chart-registry";
export { transformChartData } from "./utils/data-transformer";
export { getDefaultOptions } from "./utils/chart-config";
import { lazy } from "react";

// Lazy load chart components for better code splitting
export const ChartDisplay = lazy(() =>
  import("./chart-display").then((module) => ({ default: module.ChartDisplay }))
);
