"use client";

import { useEffect, useState, useRef, useMemo } from "react";
import ReactECharts from "echarts-for-react";
import { Maximize2, Minimize2, Download, RefreshCw } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { loadChartType, getEChartsCore } from "./chart-registry";
import { transformChartData } from "./utils/data-transformer";
import { getDefaultOptions } from "./utils/chart-config";
import type { ChartDisplayProps } from "./chart-types";
import type { EChartsOption } from "echarts";

export function ChartDisplay({
  interactionId,
  title,
  chartType,
  data,
  options = {},
  requiresResponse = false,
  className,
}: ChartDisplayProps) {
  const [isLoading, setIsLoading] = useState(true);
  const [isMaximized, setIsMaximized] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const chartRef = useRef<ReactECharts>(null);

  // Load the required chart type
  useEffect(() => {
    const loadChart = async () => {
      try {
        setIsLoading(true);
        setError(null);
        await loadChartType(chartType);
        setIsLoading(false);
      } catch (err) {
        console.error("Failed to load chart type:", err);
        setError(`Failed to load ${chartType} chart`);
        setIsLoading(false);
      }
    };

    loadChart();
  }, [chartType]);

  // Prepare chart options
  const chartOptions = useMemo<EChartsOption>(() => {
    try {
      const defaultOpts = getDefaultOptions(chartType, title);
      const transformedData = transformChartData(chartType, data);
      
      // Merge default options, transformed data, and custom options
      const finalOptions: EChartsOption = {
        ...defaultOpts,
        ...transformedData,
        ...options,
        // Ensure title is set
        title: {
          text: title,
          left: "center",
          ...((options.title as any) || {}),
        },
        // Add toolbox for export functionality
        toolbox: {
          show: true,
          feature: {
            dataZoom: {
              show: ["line", "bar", "scatter", "candlestick"].includes(chartType),
            },
            dataView: { show: true, readOnly: false },
            magicType: {
              show: ["line", "bar", "scatter"].includes(chartType),
              type: ["line", "bar", "stack"],
            },
            restore: { show: true },
            saveAsImage: { show: true },
          },
          ...((options.toolbox as any) || {}),
        },
        // Enable animation by default
        animation: options.animation !== false,
      };

      return finalOptions;
    } catch (err) {
      console.error("Failed to prepare chart options:", err);
      setError("Failed to prepare chart data");
      return {};
    }
  }, [chartType, data, options, title]);

  // Handle chart export
  const handleExport = () => {
    if (chartRef.current) {
      const echarts = chartRef.current.getEchartsInstance();
      const base64 = echarts.getDataURL({
        type: "png",
        pixelRatio: 2,
        backgroundColor: "#fff",
      });
      
      // Create download link
      const link = document.createElement("a");
      link.download = `${title.replace(/\s+/g, "_")}_chart.png`;
      link.href = base64;
      link.click();
    }
  };

  // Handle chart refresh
  const handleRefresh = () => {
    if (chartRef.current) {
      const echarts = chartRef.current.getEchartsInstance();
      echarts.clear();
      echarts.setOption(chartOptions);
    }
  };

  // Handle empty data case
  if (!data || (Array.isArray(data.series) && data.series.length === 0)) {
    return (
      <div className="border rounded-lg p-6 bg-muted/30">
        <h3 className="font-medium text-sm mb-2">📊 {title}</h3>
        <div className="text-sm text-muted-foreground">
          No data available to display
        </div>
      </div>
    );
  }

  // Handle error case
  if (error) {
    return (
      <div className="border rounded-lg p-6 bg-destructive/10">
        <h3 className="font-medium text-sm mb-2">📊 {title}</h3>
        <div className="text-sm text-destructive">
          {error}
        </div>
      </div>
    );
  }

  return (
    <div
      className={cn(
        "w-full space-y-2",
        isMaximized &&
          "fixed inset-0 flex flex-col z-[100] bottom-[70px] bg-background px-4",
        className
      )}
      data-interaction-id={interactionId}
    >
      {/* Title Header */}
      <div className={cn("flex items-center justify-between px-1", isMaximized && "pt-2")}>
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium">{title}</span>
          <span className="text-xs text-muted-foreground">
            ({chartType} chart)
          </span>
        </div>
        <div className="flex items-center gap-1">
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7"
            onClick={handleExport}
            title="Export chart"
            disabled={isLoading}
          >
            <Download className="h-4 w-4" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7"
            onClick={handleRefresh}
            title="Refresh chart"
            disabled={isLoading}
          >
            <RefreshCw className="h-4 w-4" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7"
            onClick={() => setIsMaximized(!isMaximized)}
            title={isMaximized ? "Minimize" : "Maximize"}
          >
            {isMaximized ? (
              <Minimize2 className="h-4 w-4" />
            ) : (
              <Maximize2 className="h-4 w-4" />
            )}
          </Button>
        </div>
      </div>

      {/* Chart Container */}
      <div className={cn(
        "border-2 rounded-md bg-background",
        !isMaximized && "min-h-[400px]",
        isMaximized && "flex-1"
      )}>
        {isLoading ? (
          <div className="flex items-center justify-center h-full min-h-[400px]">
            <div className="text-sm text-muted-foreground">
              Loading {chartType} chart...
            </div>
          </div>
        ) : (
          <ReactECharts
            ref={chartRef}
            echarts={getEChartsCore()}
            option={chartOptions}
            style={{
              height: isMaximized ? "100%" : "400px",
              width: "100%",
            }}
            opts={{ renderer: "canvas" }}
            notMerge={false}
            lazyUpdate={true}
            theme={options.theme || "light"}
          />
        )}
      </div>

      {/* Response UI if needed */}
      {requiresResponse && (
        <div className="mt-3 p-3 border rounded-lg bg-muted/30">
          <div className="text-xs text-muted-foreground">
            Interactive chart is ready. Click on data points to interact.
          </div>
        </div>
      )}
    </div>
  );
}