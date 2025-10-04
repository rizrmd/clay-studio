import * as echarts from "echarts/core";
import {
  TitleComponent,
  TooltipComponent,
  GridComponent,
  LegendComponent,
  DataZoomComponent,
  ToolboxComponent,
  VisualMapComponent,
  PolarComponent,
  RadarComponent,
  GeoComponent,
  DatasetComponent,
  TransformComponent,
  MarkLineComponent,
  MarkPointComponent,
  MarkAreaComponent,
  CalendarComponent,
} from "echarts/components";
import { CanvasRenderer, SVGRenderer } from "echarts/renderers";
import type { ChartType, ChartLoader } from "./chart-types";

// Register core components that are always needed
echarts.use([
  TitleComponent,
  TooltipComponent,
  GridComponent,
  LegendComponent,
  DataZoomComponent,
  ToolboxComponent,
  DatasetComponent,
  TransformComponent,
  CanvasRenderer,
  SVGRenderer,
]);

// Chart loaders with lazy loading
const chartLoaders: Record<ChartType, ChartLoader> = {
  line: {
    load: async () => {
      const { LineChart } = await import("echarts/charts");
      echarts.use([LineChart, MarkLineComponent, MarkPointComponent, MarkAreaComponent]);
      return LineChart;
    },
  },
  bar: {
    load: async () => {
      const { BarChart } = await import("echarts/charts");
      echarts.use([BarChart, MarkLineComponent, MarkPointComponent]);
      return BarChart;
    },
  },
  column: {
    load: async () => {
      const { BarChart } = await import("echarts/charts");
      echarts.use([BarChart, MarkLineComponent, MarkPointComponent]);
      return BarChart;
    },
  },
  pie: {
    load: async () => {
      const { PieChart } = await import("echarts/charts");
      echarts.use([PieChart]);
      return PieChart;
    },
  },
  donut: {
    load: async () => {
      const { PieChart } = await import("echarts/charts");
      echarts.use([PieChart]);
      return PieChart;
    },
  },
  scatter: {
    load: async () => {
      const { ScatterChart } = await import("echarts/charts");
      echarts.use([ScatterChart, VisualMapComponent]);
      return ScatterChart;
    },
  },
  radar: {
    load: async () => {
      const { RadarChart } = await import("echarts/charts");
      echarts.use([RadarChart, RadarComponent]);
      return RadarChart;
    },
  },
  gauge: {
    load: async () => {
      const { GaugeChart } = await import("echarts/charts");
      echarts.use([GaugeChart]);
      return GaugeChart;
    },
  },
  funnel: {
    load: async () => {
      const { FunnelChart } = await import("echarts/charts");
      echarts.use([FunnelChart]);
      return FunnelChart;
    },
  },
  heatmap: {
    load: async () => {
      const { HeatmapChart } = await import("echarts/charts");
      echarts.use([HeatmapChart, VisualMapComponent]);
      return HeatmapChart;
    },
  },
  boxplot: {
    load: async () => {
      const { BoxplotChart } = await import("echarts/charts");
      echarts.use([BoxplotChart]);
      return BoxplotChart;
    },
  },
  candlestick: {
    load: async () => {
      const { CandlestickChart } = await import("echarts/charts");
      echarts.use([CandlestickChart, MarkLineComponent]);
      return CandlestickChart;
    },
  },
  map: {
    load: async () => {
      const { MapChart } = await import("echarts/charts");
      echarts.use([MapChart, GeoComponent, VisualMapComponent]);
      return MapChart;
    },
  },
  graph: {
    load: async () => {
      const { GraphChart } = await import("echarts/charts");
      echarts.use([GraphChart]);
      return GraphChart;
    },
  },
  tree: {
    load: async () => {
      const { TreeChart } = await import("echarts/charts");
      echarts.use([TreeChart]);
      return TreeChart;
    },
  },
  treemap: {
    load: async () => {
      const { TreemapChart } = await import("echarts/charts");
      echarts.use([TreemapChart, VisualMapComponent]);
      return TreemapChart;
    },
  },
  sunburst: {
    load: async () => {
      const { SunburstChart } = await import("echarts/charts");
      echarts.use([SunburstChart]);
      return SunburstChart;
    },
  },
  sankey: {
    load: async () => {
      const { SankeyChart } = await import("echarts/charts");
      echarts.use([SankeyChart]);
      return SankeyChart;
    },
  },
  parallel: {
    load: async () => {
      const { ParallelChart } = await import("echarts/charts");
      echarts.use([ParallelChart, PolarComponent]);
      return ParallelChart;
    },
  },
  calendar: {
    load: async () => {
      const { HeatmapChart, ScatterChart } = await import("echarts/charts");
      echarts.use([HeatmapChart, ScatterChart, CalendarComponent, VisualMapComponent]);
      return { HeatmapChart, ScatterChart };
    },
  },
  custom: {
    load: async () => {
      const { CustomChart } = await import("echarts/charts");
      echarts.use([CustomChart]);
      return CustomChart;
    },
  },
};

// Cache for loaded chart types
const loadedCharts = new Set<ChartType>();

// Function to load a specific chart type
export async function loadChartType(chartType: ChartType): Promise<void> {
  if (loadedCharts.has(chartType)) {
    return;
  }

  const loader = chartLoaders[chartType];
  if (!loader) {
    throw new Error(`Unknown chart type: ${chartType}`);
  }

  await loader.load();
  loadedCharts.add(chartType);
}

// Function to preload commonly used charts
export async function preloadCommonCharts(): Promise<void> {
  const commonCharts: ChartType[] = ["line", "bar", "pie", "scatter"];
  await Promise.all(commonCharts.map(loadChartType));
}

// Get ECharts instance
export function getEChartsCore() {
  return echarts;
}

// Export chart loaders for advanced usage
export { chartLoaders };