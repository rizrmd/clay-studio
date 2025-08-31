import type { ChartType, ChartData } from "../chart-types";
import type { EChartsOption } from "echarts";

export function transformChartData(chartType: ChartType, data: ChartData): Partial<EChartsOption> {
  // Handle direct ECharts option format
  if (data.option) {
    return data.option;
  }

  // Transform based on chart type
  switch (chartType) {
    case "line":
    case "bar":
      return transformCartesianData(data, chartType);
    
    case "pie":
    case "funnel":
      return transformPieData(data, chartType);
    
    case "scatter":
      return transformScatterData(data);
    
    case "radar":
      return transformRadarData(data);
    
    case "gauge":
      return transformGaugeData(data);
    
    case "heatmap":
      return transformHeatmapData(data);
    
    case "boxplot":
      return transformBoxplotData(data);
    
    case "candlestick":
      return transformCandlestickData(data);
    
    case "map":
      return transformMapData(data);
    
    case "graph":
      return transformGraphData(data);
    
    case "tree":
    case "treemap":
    case "sunburst":
      return transformTreeData(data, chartType);
    
    case "sankey":
      return transformSankeyData(data);
    
    case "parallel":
      return transformParallelData(data);
    
    case "calendar":
      return transformCalendarData(data);
    
    case "custom":
    default:
      return transformCustomData(data);
  }
}

function transformCartesianData(data: ChartData, type: "line" | "bar"): Partial<EChartsOption> {
  const options: Partial<EChartsOption> = {};

  // Handle categories (x-axis)
  if (data.categories) {
    options.xAxis = {
      type: "category",
      data: data.categories,
    };
    options.yAxis = { type: "value" };
  }

  // Handle series
  if (data.series) {
    options.series = data.series.map(s => {
      const { type: _, name, data: seriesData, ...rest } = s; // Exclude the type property from spreading
      return {
        type: type as any,
        name: name,
        data: seriesData,
        ...rest, // Include any additional series options except type
      };
    });
  }

  // Handle dataset format
  if (data.dataset) {
    options.dataset = data.dataset;
    if (!options.xAxis) {
      options.xAxis = { type: "category" };
    }
    if (!options.yAxis) {
      options.yAxis = { type: "value" };
    }
  }

  return options;
}

function transformPieData(data: ChartData, type: "pie" | "funnel" = "pie"): Partial<EChartsOption> {
  const options: Partial<EChartsOption> = {};

  if (data.series) {
    options.series = data.series.map(s => {
      const { type: _, name, data: seriesData, ...rest } = s;
      return {
        type: type as any,
        name: name,
        radius: type === "pie" ? "50%" : undefined,
        data: seriesData.map((value: any, index: number) => ({
          value: typeof value === "object" ? value.value : value,
          name: typeof value === "object" ? value.name : (data.categories?.[index] || `Item ${index + 1}`),
        })),
        ...rest,
      };
    });
  } else if (data.data) {
    // Simple pie data format
    options.series = [{
      type: type as any,
      radius: type === "pie" ? "50%" : undefined,
      data: Array.isArray(data.data) ? data.data : [],
    }];
  }

  return options;
}

function transformScatterData(data: ChartData): Partial<EChartsOption> {
  const options: Partial<EChartsOption> = {
    xAxis: { type: "value" },
    yAxis: { type: "value" },
  };

  if (data.series) {
    options.series = data.series.map(s => {
      const { type: _, name, data: seriesData, ...rest } = s;
      return {
        type: "scatter" as any,
        name: name,
        data: seriesData,
        ...rest,
      };
    });
  }

  return options;
}

function transformRadarData(data: ChartData): Partial<EChartsOption> {
  const options: Partial<EChartsOption> = {};

  if (data.indicator) {
    options.radar = {
      indicator: data.indicator,
    };
  }

  if (data.series) {
    options.series = data.series.map(s => {
      const { type: _, name, data: seriesData, ...rest } = s;
      return {
        type: "radar" as any,
        name: name,
        data: seriesData,
        ...rest,
      };
    });
  }

  return options;
}

function transformGaugeData(data: ChartData): Partial<EChartsOption> {
  const options: Partial<EChartsOption> = {};

  if (data.value !== undefined) {
    const { type: _, ...rest } = data;
    options.series = [{
      type: "gauge" as any,
      data: [{ value: data.value, name: data.name || "Value" }],
      ...rest,
    }];
  } else if (data.series) {
    options.series = data.series.map(s => {
      const { type: _, ...rest } = s;
      return {
        type: "gauge" as any,
        ...rest,
      };
    });
  }

  return options;
}

function transformHeatmapData(data: ChartData): Partial<EChartsOption> {
  const options: Partial<EChartsOption> = {
    xAxis: {
      type: "category",
      data: data.xAxis || [],
    },
    yAxis: {
      type: "category",
      data: data.yAxis || [],
    },
    visualMap: {
      min: data.min || 0,
      max: data.max || 100,
      calculable: true,
      orient: "horizontal",
      left: "center",
      bottom: "15%",
    },
  };

  if (data.series) {
    options.series = data.series.map(s => {
      const { type: _, ...rest } = s;
      return {
        type: "heatmap" as any,
        ...rest,
      };
    });
  }

  return options;
}

function transformBoxplotData(data: ChartData): Partial<EChartsOption> {
  const options: Partial<EChartsOption> = {
    xAxis: {
      type: "category",
      data: data.categories || [],
    },
    yAxis: { type: "value" },
  };

  if (data.series) {
    options.series = data.series.map(s => {
      const { type: _, ...rest } = s;
      return {
        type: "boxplot" as any,
        ...rest,
      };
    });
  }

  return options;
}

function transformCandlestickData(data: ChartData): Partial<EChartsOption> {
  const options: Partial<EChartsOption> = {
    xAxis: {
      type: "category",
      data: data.categories || [],
    },
    yAxis: { type: "value" },
  };

  if (data.series) {
    options.series = data.series.map(s => {
      const { type: _, ...rest } = s;
      return {
        type: "candlestick" as any,
        ...rest,
      };
    });
  }

  return options;
}

function transformMapData(data: ChartData): Partial<EChartsOption> {
  const options: Partial<EChartsOption> = {};

  if (data.series) {
    options.series = data.series.map(s => {
      const { type: _, ...rest } = s;
      return {
        type: "map" as any,
        map: data.mapType || "world",
        ...rest,
      };
    });
  }

  if (data.visualMap) {
    options.visualMap = data.visualMap;
  }

  return options;
}

function transformGraphData(data: ChartData): Partial<EChartsOption> {
  const options: Partial<EChartsOption> = {};

  if (data.nodes && data.links) {
    const { type: _, ...rest } = data;
    options.series = [{
      type: "graph" as any,
      layout: data.layout || "force",
      data: data.nodes,
      links: data.links,
      roam: true,
      label: {
        show: true,
      },
      ...rest,
    }];
  } else if (data.series) {
    options.series = data.series.map(s => {
      const { type: _, ...rest } = s;
      return {
        type: "graph" as any,
        ...rest,
      };
    });
  }

  return options;
}

function transformTreeData(data: ChartData, chartType: "tree" | "treemap" | "sunburst"): Partial<EChartsOption> {
  const options: Partial<EChartsOption> = {};

  if (data.data) {
    const { type: _, ...rest } = data;
    options.series = [{
      type: chartType as any,
      data: Array.isArray(data.data) ? data.data : [data.data],
      ...rest,
    }];
  } else if (data.series) {
    options.series = data.series.map(s => {
      const { type: _, ...rest } = s;
      return {
        type: chartType as any,
        ...rest,
      };
    });
  }

  return options;
}

function transformSankeyData(data: ChartData): Partial<EChartsOption> {
  const options: Partial<EChartsOption> = {};

  if (data.nodes && data.links) {
    const { type: _, ...rest } = data;
    options.series = [{
      type: "sankey" as any,
      data: data.nodes,
      links: data.links,
      ...rest,
    }];
  } else if (data.series) {
    options.series = data.series.map(s => {
      const { type: _, ...rest } = s;
      return {
        type: "sankey" as any,
        ...rest,
      };
    });
  }

  return options;
}

function transformParallelData(data: ChartData): Partial<EChartsOption> {
  const options: Partial<EChartsOption> = {};

  if (data.parallelAxis) {
    options.parallelAxis = data.parallelAxis;
  }

  if (data.series) {
    options.series = data.series.map(s => {
      const { type: _, ...rest } = s;
      return {
        type: "parallel" as any,
        ...rest,
      };
    });
  }

  return options;
}

function transformCalendarData(data: ChartData): Partial<EChartsOption> {
  const options: Partial<EChartsOption> = {
    calendar: {
      range: data.range || new Date().getFullYear().toString(),
    },
  };

  if (data.series) {
    options.series = data.series.map(s => {
      const { type: _, ...rest } = s;
      return {
        type: "heatmap" as any,
        coordinateSystem: "calendar",
        ...rest,
      };
    });
  }

  return options;
}

function transformCustomData(data: ChartData): Partial<EChartsOption> {
  // For custom charts, return the data as-is or with minimal transformation
  if (data.series) {
    return { 
      series: data.series.map(s => {
        const { type: _, ...rest } = s;
        return {
          type: "custom" as any,
          ...rest,
        };
      })
    };
  }
  
  return data as Partial<EChartsOption>;
}