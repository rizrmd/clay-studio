// Comprehensive example demonstrating dynamic filters with dependencies

import { AnalysisDisplay } from './analysis-display';
import type { FilterConfig } from './dynamic-filters';

const exampleFilters: FilterConfig[] = [
  {
    name: 'datasource',
    label: 'Data Source',
    type: 'select',
    required: true,
    placeholder: 'Select data source',
    apiEndpoint: '/api/analysis/filter-options/datasource',
    options: [
      { value: 'user_data', label: 'User Data' },
      { value: 'transactions', label: 'Transactions' },
      { value: 'events', label: 'Events' },
      { value: 'products', label: 'Products' }
    ]
  },
  {
    name: 'metric',
    label: 'Metric',
    type: 'select',
    required: true,
    dependsOn: ['datasource'],
    placeholder: 'Select metric',
    apiEndpoint: '/api/analysis/filter-options/metric',
    // Options will be loaded dynamically based on datasource
  },
  {
    name: 'country',
    label: 'Country',
    type: 'select',
    placeholder: 'Select country',
    apiEndpoint: '/api/analysis/filter-options/country',
    options: [
      { value: 'US', label: 'United States' },
      { value: 'CA', label: 'Canada' },
      { value: 'UK', label: 'United Kingdom' },
      { value: 'DE', label: 'Germany' },
      { value: 'FR', label: 'France' }
    ]
  },
  {
    name: 'region',
    label: 'Region',
    type: 'select',
    dependsOn: ['country'],
    placeholder: 'Select region',
    apiEndpoint: '/api/analysis/filter-options/region',
    // Options depend on selected country
  },
  {
    name: 'city',
    label: 'City',
    type: 'multiselect',
    dependsOn: ['region'],
    placeholder: 'Select cities',
    apiEndpoint: '/api/analysis/filter-options/city',
    // Options depend on selected region
  },
  {
    name: 'date_range',
    label: 'Date Range',
    type: 'select',
    placeholder: 'Select date range',
    options: [
      { value: 'today', label: 'Today' },
      { value: 'yesterday', label: 'Yesterday' },
      { value: 'last_7_days', label: 'Last 7 Days' },
      { value: 'last_30_days', label: 'Last 30 Days' },
      { value: 'last_90_days', label: 'Last 90 Days' },
      { value: 'this_month', label: 'This Month' },
      { value: 'last_month', label: 'Last Month' },
      { value: 'this_year', label: 'This Year' }
    ]
  },
  {
    name: 'custom_start_date',
    label: 'Start Date',
    type: 'date',
    dependsOn: ['date_range'],
    dependsOnValue: { date_range: 'custom' },
    placeholder: 'Select start date'
  },
  {
    name: 'custom_end_date',
    label: 'End Date',
    type: 'date',
    dependsOn: ['date_range'],
    dependsOnValue: { date_range: 'custom' },
    placeholder: 'Select end date'
  },
  {
    name: 'user_type',
    label: 'User Type',
    type: 'multiselect',
    placeholder: 'Select user types',
    apiEndpoint: '/api/analysis/filter-options/user_type',
    options: [
      { value: 'free', label: 'Free Users' },
      { value: 'premium', label: 'Premium Users' },
      { value: 'enterprise', label: 'Enterprise Users' },
      { value: 'trial', label: 'Trial Users' }
    ]
  },
  {
    name: 'min_revenue',
    label: 'Min Revenue',
    type: 'number',
    placeholder: 'Enter minimum revenue'
  },
  {
    name: 'max_revenue',
    label: 'Max Revenue',
    type: 'number',
    placeholder: 'Enter maximum revenue'
  }
];

export function AnalysisFiltersExample() {
  return (
    <div className="p-6 space-y-6">
      <div>
        <h2 className="text-2xl font-bold mb-2">Dynamic Analysis Filters Example</h2>
        <p className="text-gray-600 mb-6">
          This example demonstrates dynamic filters with dependencies. Try selecting different combinations:
        </p>
        <ul className="text-sm text-gray-500 space-y-1 mb-6">
          <li>• <strong>Data Source</strong> → <strong>Metric</strong>: Available metrics depend on data source</li>
          <li>• <strong>Country</strong> → <strong>Region</strong> → <strong>City</strong>: Geographic hierarchy</li>
          <li>• <strong>Date Range</strong> → <strong>Custom Dates</strong>: Custom dates only appear when "custom" is selected</li>
          <li>• <strong>User Type</strong>: Multi-select with multiple selections</li>
          <li>• <strong>Revenue Range</strong>: Numeric input fields</li>
        </ul>
      </div>

      <div className="space-y-4">
        <div className="border rounded-lg p-4 bg-white">
          <h3 className="text-lg font-semibold mb-4">Revenue Analysis Dashboard</h3>
          <AnalysisDisplay
            analysisId="revenue-analysis-example"
            title="Revenue Analysis Dashboard"
            description="Analyze revenue trends across different dimensions"
            availableFilters={exampleFilters}
          />
        </div>

        <div className="border rounded-lg p-4 bg-white">
          <h3 className="text-lg font-semibold mb-4">User Behavior Analysis</h3>
          <AnalysisDisplay
            analysisId="user-behavior-example"
            title="User Behavior Analysis"
            description="Track user engagement and behavior patterns"
            availableFilters={[
              exampleFilters[0], // datasource
              exampleFilters[1], // metric
              exampleFilters[5], // date_range
              exampleFilters[8], // user_type
              exampleFilters[3], // region
              exampleFilters[4]  // city
            ]}
          />
        </div>

        <div className="border rounded-lg p-4 bg-white">
          <h3 className="text-lg font-semibold mb-4">Geographic Performance</h3>
          <AnalysisDisplay
            analysisId="geo-performance-example"
            title="Geographic Performance"
            description="Compare performance across different geographic regions"
            availableFilters={[
              exampleFilters[0], // datasource
              exampleFilters[1], // metric
              exampleFilters[2], // country
              exampleFilters[3], // region
              exampleFilters[4], // city
              exampleFilters[5], // date_range
              exampleFilters[9], // min_revenue
              exampleFilters[10] // max_revenue
            ]}
          />
        </div>
      </div>
    </div>
  );
}