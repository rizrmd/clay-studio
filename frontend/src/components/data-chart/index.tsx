import { useMemo, useState } from "react";
import {
  Line,
  Bar,
  Pie,
  Area,
  LineChart,
  BarChart,
  PieChart,
  AreaChart,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
  Cell,
} from "recharts";
import { Download, Maximize2, RefreshCw, Layers } from "lucide-react";

interface ChartDisplayProps {
  interactionId: string;
  title: string;
  chartType?: "line" | "bar" | "pie" | "area";
  data: {
    chart_type?: string;
    datasets?: Array<{
      label: string;
      data: number[];
      borderColor?: string;
      backgroundColor?: string;
    }>;
    labels?: string[];
    series?: any[];
    values?: any[];
  };
  options?: any;
  requiresResponse?: boolean;
}

// Color palette for charts
const COLORS = [
  "#8884d8",
  "#82ca9d",
  "#ffc658",
  "#ff7c7c",
  "#8dd1e1",
  "#d084d0",
  "#ffb347",
  "#67b7dc",
];

export function ChartDisplay({
  interactionId,
  title,
  chartType: propChartType,
  data,
  options,
  requiresResponse,
}: ChartDisplayProps) {
  // Determine chart type
  const chartType = propChartType || data.chart_type || "line";

  // Transform data for Recharts format
  const chartData = useMemo(() => {
    // If data is already in the right format (array of objects)
    if (Array.isArray(data.series)) {
      return data.series;
    }

    // Transform from Chart.js format
    if (data.labels && data.datasets) {
      return data.labels.map((label: string, index: number) => {
        const point: any = { name: label };
        data.datasets?.forEach((dataset) => {
          point[dataset.label] = dataset.data[index];
        });
        return point;
      });
    }

    // For pie charts with simple values array
    if (data.labels && data.values) {
      return data.labels.map((label: string, index: number) => ({
        name: label,
        value: data.values![index],
      }));
    }

    return [];
  }, [data]);

  // Get dataset colors
  const datasetColors = useMemo(() => {
    if (data.datasets) {
      return data.datasets.map(
        (dataset, index) =>
          dataset.borderColor ||
          dataset.backgroundColor ||
          COLORS[index % COLORS.length]
      );
    }
    return COLORS;
  }, [data]);

  const renderChart = () => {
    switch (chartType) {
      case "pie":
        return (
          <ResponsiveContainer width="100%" height={300}>
            <PieChart margin={{ top: 5, right: 30, left: 20, bottom: 5 }}>
              <Pie
                data={chartData}
                cx="50%"
                cy="50%"
                labelLine={false}
                label={({ name, percent }) =>
                  `${name}: ${(percent * 100).toFixed(0)}%`
                }
                outerRadius={80}
                fill="#8884d8"
                dataKey="value"
              >
                {chartData.map((entry: any, index: number) => (
                  <Cell
                    key={`cell-${index}`}
                    fill={COLORS[index % COLORS.length]}
                  />
                ))}
              </Pie>
              <Tooltip />
              <Legend />
            </PieChart>
          </ResponsiveContainer>
        );

      case "bar":
        return (
          <ResponsiveContainer width="100%" height={300}>
            <BarChart data={chartData} margin={{ top: 5, right: 30, left: 20, bottom: 5 }}>
              <CartesianGrid strokeDasharray="3 3" />
              <XAxis dataKey="name" />
              <YAxis />
              <Tooltip />
              <Legend />
              {data.datasets?.map((dataset, index) => (
                <Bar
                  key={dataset.label}
                  dataKey={dataset.label}
                  fill={datasetColors[index]}
                />
              ))}
            </BarChart>
          </ResponsiveContainer>
        );

      case "area":
        return (
          <ResponsiveContainer width="100%" height={300}>
            <AreaChart data={chartData} margin={{ top: 5, right: 30, left: 20, bottom: 5 }}>
              <CartesianGrid strokeDasharray="3 3" />
              <XAxis dataKey="name" />
              <YAxis />
              <Tooltip />
              <Legend />
              {data.datasets?.map((dataset, index) => (
                <Area
                  key={dataset.label}
                  type="monotone"
                  dataKey={dataset.label}
                  stroke={datasetColors[index]}
                  fill={datasetColors[index]}
                  fillOpacity={0.6}
                />
              ))}
            </AreaChart>
          </ResponsiveContainer>
        );

      case "line":
      default:
        return (
          <ResponsiveContainer width="100%" height={300}>
            <LineChart data={chartData} margin={{ top: 5, right: 30, left: 20, bottom: 5 }}>
              <CartesianGrid strokeDasharray="3 3" />
              <XAxis dataKey="name" />
              <YAxis />
              <Tooltip />
              <Legend />
              {data.datasets?.map((dataset, index) => (
                <Line
                  key={dataset.label}
                  type="monotone"
                  dataKey={dataset.label}
                  stroke={datasetColors[index]}
                  strokeWidth={2}
                  dot={{ r: 3 }}
                />
              ))}
            </LineChart>
          </ResponsiveContainer>
        );
    }
  };

  return (
    <div className="space-y-2">
      {/* Title as standalone heading */}
      <h1 className="text-base font-semibold text-gray-800">{title}</h1>
      
      {/* Chart container */}
      <div className="relative border rounded-lg bg-white shadow-sm p-4">
        <div className="w-full overflow-x-auto">
          {renderChart()}
        </div>
      </div>
    </div>
  );
}