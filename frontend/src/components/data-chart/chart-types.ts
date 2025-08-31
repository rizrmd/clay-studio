import type { EChartsOption } from "echarts";

export type ChartType = 
  | "line"
  | "bar"
  | "pie"
  | "scatter"
  | "radar"
  | "gauge"
  | "funnel"
  | "heatmap"
  | "boxplot"
  | "candlestick"
  | "map"
  | "graph"
  | "tree"
  | "treemap"
  | "sunburst"
  | "sankey"
  | "parallel"
  | "calendar"
  | "custom";

export interface ChartDataSeries {
  name: string;
  data: any[];
  type?: ChartType;
  [key: string]: any;
}

export interface ChartData {
  series?: ChartDataSeries[];
  categories?: string[];
  dataset?: any;
  [key: string]: any;
}

export interface ChartOptions extends Partial<EChartsOption> {
  theme?: "light" | "dark";
  animation?: boolean;
  interactive?: boolean;
  responsive?: boolean;
  [key: string]: any;
}

export interface ChartDisplayProps {
  interactionId: string;
  title: string;
  chartType: ChartType;
  data: ChartData;
  options?: ChartOptions;
  requiresResponse?: boolean;
  className?: string;
}

export interface ChartLoader {
  load: () => Promise<any>;
  dependencies?: string[];
}

export interface ChartConfig {
  type: ChartType;
  loader: ChartLoader;
  defaultOptions?: Partial<EChartsOption>;
  dataTransformer?: (data: ChartData) => any;
}