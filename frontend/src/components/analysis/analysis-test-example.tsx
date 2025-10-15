// This is a test example to show how the AnalysisDisplay component works with parameters

import { AnalysisDisplay } from './analysis-display';

export function AnalysisTestExample() {
  const exampleParameters = {
    date_range: "2024-01-01 to 2024-12-31",
    source: "user_data",
    filters: {
      status: "active",
      type: "premium",
      region: "US"
    },
    aggregation: "daily",
    metrics: ["revenue", "users", "conversion"]
  };

  return (
    <div className="p-4 space-y-4">
      <h2 className="text-xl font-bold">Analysis Component Test</h2>

      <div className="space-y-4">
        <div>
          <h3 className="text-lg font-semibold mb-2">Example 1: Simple Analysis</h3>
          <AnalysisDisplay
            analysisId="test-analysis-1"
            title="User Activity Analysis"
            description="Analyze user activity patterns over time"
            parameters={{
              date_range: "2024-01-01 to 2024-12-31",
              metric: "page_views"
            }}
          />
        </div>

        <div>
          <h3 className="text-lg font-semibold mb-2">Example 2: Complex Analysis with Filters</h3>
          <AnalysisDisplay
            analysisId="test-analysis-2"
            title="Revenue Analysis Dashboard"
            description="Comprehensive revenue analysis with multiple filters"
            parameters={exampleParameters}
          />
        </div>

        <div>
          <h3 className="text-lg font-semibold mb-2">Example 3: Analysis with No Parameters</h3>
          <AnalysisDisplay
            analysisId="test-analysis-3"
            title="System Health Check"
            description="Basic system health monitoring"
          />
        </div>
      </div>
    </div>
  );
}

// This demonstrates how the component handles different parameter types:
// - Strings: date_range, source
// - Objects: filters with nested properties
// - Arrays: metrics
// - Different data types and structures