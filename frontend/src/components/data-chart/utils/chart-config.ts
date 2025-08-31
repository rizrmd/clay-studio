import type { ChartType } from "../chart-types";
import type { EChartsOption } from "echarts";

export function getDefaultOptions(chartType: ChartType, title: string): Partial<EChartsOption> {
  const baseOptions: Partial<EChartsOption> = {
    title: {
      text: title,
      left: "center",
      textStyle: {
        fontSize: 14,
        fontWeight: 500,
      },
    },
    tooltip: {
      trigger: getTooltipTrigger(chartType),
      formatter: undefined, // Will use default formatter
    },
    legend: {
      bottom: 10,
      left: "center",
      type: "scroll",
    },
    grid: {
      left: "3%",
      right: "4%",
      bottom: "15%",
      top: "15%",
      containLabel: true,
    },
  };

  // Add chart-specific defaults
  switch (chartType) {
    case "line":
      return {
        ...baseOptions,
        xAxis: { type: "category", boundaryGap: false },
        yAxis: { type: "value" },
      };

    case "bar":
      return {
        ...baseOptions,
        xAxis: { type: "category" },
        yAxis: { type: "value" },
      };

    case "pie":
      return {
        ...baseOptions,
        tooltip: {
          trigger: "item",
          formatter: "{a} <br/>{b}: {c} ({d}%)",
        },
        legend: {
          orient: "vertical",
          left: "left",
        },
      };

    case "scatter":
      return {
        ...baseOptions,
        xAxis: { type: "value", scale: true },
        yAxis: { type: "value", scale: true },
      };

    case "radar":
      return {
        ...baseOptions,
        radar: {
          radius: "60%",
          center: ["50%", "50%"],
        },
      };

    case "gauge":
      return {
        ...baseOptions,
        series: [{
          type: "gauge",
          startAngle: 180,
          endAngle: 0,
          min: 0,
          max: 100,
          splitNumber: 5,
          itemStyle: {
            color: "#58D9F9",
            shadowColor: "rgba(0,138,255,0.45)",
            shadowBlur: 10,
            shadowOffsetX: 2,
            shadowOffsetY: 2,
          },
          progress: {
            show: true,
            roundCap: true,
            width: 18,
          },
          pointer: {
            icon: "path://M2090.36389,615.30999 L2090.36389,615.30999 C2091.48372,615.30999 2092.40383,616.194028 2092.44859,617.312956 L2096.90698,728.755929 C2097.05155,732.369577 2094.2393,735.416212 2090.62566,735.56078 C2090.53845,735.564269 2090.45117,735.566014 2090.36389,735.566014 L2090.36389,735.566014 C2086.74736,735.566014 2083.81557,732.63423 2083.81557,729.017692 C2083.81557,728.930412 2083.81732,728.84314 2083.82081,728.755929 L2088.2792,617.312956 C2088.32396,616.194028 2089.24407,615.30999 2090.36389,615.30999 Z",
            length: "75%",
            width: 16,
            offsetCenter: [0, "5%"],
          },
          axisLine: {
            roundCap: true,
            lineStyle: {
              width: 18,
            },
          },
          axisTick: {
            splitNumber: 2,
            lineStyle: {
              width: 2,
              color: "#999",
            },
          },
          splitLine: {
            length: 12,
            lineStyle: {
              width: 3,
              color: "#999",
            },
          },
          axisLabel: {
            distance: 30,
            color: "#999",
            fontSize: 12,
          },
          title: {
            show: false,
          },
          detail: {
            backgroundColor: "#fff",
            borderColor: "#999",
            borderWidth: 2,
            width: "60%",
            lineHeight: 40,
            height: 40,
            borderRadius: 8,
            offsetCenter: [0, "35%"],
            valueAnimation: true,
            formatter: "{value}%",
            color: "auto",
          },
        }],
      };

    case "funnel":
      return {
        ...baseOptions,
        tooltip: {
          trigger: "item",
          formatter: "{a} <br/>{b} : {c}%",
        },
      };

    case "heatmap":
      return {
        ...baseOptions,
        tooltip: {
          position: "top",
        },
        grid: {
          height: "50%",
          top: "10%",
        },
      };

    case "boxplot":
      return {
        ...baseOptions,
        tooltip: {
          trigger: "item",
          axisPointer: {
            type: "shadow",
          },
        },
      };

    case "candlestick":
      return {
        ...baseOptions,
        xAxis: {
          type: "category",
          boundaryGap: false,
          axisLine: { onZero: false },
          splitLine: { show: false },
          min: "dataMin",
          max: "dataMax",
        },
        yAxis: {
          scale: true,
          splitArea: {
            show: true,
          },
        },
      };

    case "map":
      return {
        ...baseOptions,
        tooltip: {
          trigger: "item",
        },
        visualMap: {
          min: 0,
          max: 1000,
          left: "left",
          top: "bottom",
          text: ["High", "Low"],
          calculable: true,
        },
      };

    case "graph":
      return {
        ...baseOptions,
        tooltip: {},
        animationDurationUpdate: 1500,
        animationEasingUpdate: "quinticInOut",
      };

    case "tree":
      return {
        ...baseOptions,
        tooltip: {
          trigger: "item",
          triggerOn: "mousemove",
        },
      };

    case "treemap":
      return {
        ...baseOptions,
        tooltip: {
          formatter: "{b}<br/>{c}",
        },
      };

    case "sunburst":
      return {
        ...baseOptions,
        tooltip: {
          trigger: "item",
          formatter: "{b}<br/>{c} ({d}%)",
        },
      };

    case "sankey":
      return {
        ...baseOptions,
        tooltip: {
          trigger: "item",
          triggerOn: "mousemove",
        },
      };

    case "parallel":
      return {
        ...baseOptions,
        parallelAxis: [],
        parallel: {
          left: "5%",
          right: "15%",
          bottom: "10%",
          top: "20%",
        },
      };

    case "calendar":
      return {
        ...baseOptions,
        tooltip: {
          position: "top",
          formatter: (p: any) => {
            const format = p.data[0] + ": " + p.data[1];
            return format;
          },
        },
        visualMap: {
          min: 0,
          max: 100,
          calculable: true,
          orient: "horizontal",
          left: "center",
          top: "top",
        },
      };

    case "custom":
    default:
      return baseOptions;
  }
}

function getTooltipTrigger(chartType: ChartType): "item" | "axis" | "none" {
  switch (chartType) {
    case "line":
    case "bar":
    case "candlestick":
      return "axis";
    case "pie":
    case "funnel":
    case "gauge":
    case "scatter":
    case "radar":
    case "map":
    case "graph":
    case "tree":
    case "treemap":
    case "sunburst":
    case "sankey":
      return "item";
    default:
      return "item";
  }
}