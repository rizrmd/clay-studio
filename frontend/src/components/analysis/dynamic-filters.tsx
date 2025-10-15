"use client";

import { useState, useEffect } from "react";
import { Button } from "@/components/ui/button";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Badge } from "@/components/ui/badge";
import { Loader2, X, RefreshCw } from "lucide-react";

export interface FilterOption {
  value: string;
  label: string;
  disabled?: boolean;
  group?: string;
}

export interface FilterConfig {
  name: string;
  label: string;
  type: "select" | "multiselect" | "text" | "date" | "number";
  options?: FilterOption[];
  placeholder?: string;
  required?: boolean;
  dependsOn?: string[];
  dependsOnValue?: Record<string, any>;
  apiEndpoint?: string;
  searchable?: boolean;
}

interface DynamicFiltersProps {
  analysisId: string;
  filters: FilterConfig[];
  values: Record<string, any>;
  onChange: (name: string, value: any) => void;
  onAddFilter: (filterName: string) => void;
  onRemoveFilter: (filterName: string) => void;
  className?: string;
}

export function DynamicFilters({
  analysisId,
  filters,
  values,
  onChange,
  onAddFilter,
  onRemoveFilter,
  className = "",
}: DynamicFiltersProps) {
  const [loadingOptions, setLoadingOptions] = useState<Set<string>>(new Set());
  const [optionsData, setOptionsData] = useState<Record<string, FilterOption[]>>({});

  // Load dynamic options for filters that have API endpoints
  const loadFilterOptions = async (filter: FilterConfig, dependencies?: Record<string, any>) => {
    if (!filter.apiEndpoint) return;

    setLoadingOptions(prev => new Set(prev).add(filter.name));

    try {
      // Mock API call - replace with actual API call
      // const response = await fetch(`${filter.apiEndpoint}?analysis_id=${analysisId}&dependencies=${JSON.stringify(dependencies || {})}`);
      // const data = await response.json();

      // Mock data for demonstration
      const mockData = await getMockFilterOptions(filter.name, dependencies);
      setOptionsData(prev => ({
        ...prev,
        [filter.name]: mockData
      }));
    } catch (error) {
      console.error(`Failed to load options for filter ${filter.name}:`, error);
    } finally {
      setLoadingOptions(prev => {
        const newSet = new Set(prev);
        newSet.delete(filter.name);
        return newSet;
      });
    }
  };

  // Check if dependencies are satisfied for a filter
  const areDependenciesSatisfied = (filter: FilterConfig): boolean => {
    if (!filter.dependsOn || filter.dependsOn.length === 0) return true;

    return filter.dependsOn.every(dep => {
      const requiredValue = filter.dependsOnValue?.[dep];
      const currentValue = values[dep];

      // If no specific value required, just check that dependency exists
      if (requiredValue === undefined) {
        return currentValue !== undefined && currentValue !== "";
      }

      // If specific value required, check that it matches
      return currentValue === requiredValue;
    });
  };

  // Get available options for a filter (static + dynamic + filtered by dependencies)
  const getFilterOptions = (filter: FilterConfig): FilterOption[] => {
    let options = filter.options || optionsData[filter.name] || [];

    // Filter options based on dependencies
    if (filter.dependsOn && filter.dependsOnValue) {
      options = options.filter(_option => {
        // Apply dependency-based filtering logic here
        // For example, only show certain regions when country = "US"
        return true; // Placeholder - implement actual filtering logic
      });
    }

    return options;
  };

  // Load options when dependencies change
  useEffect(() => {
    filters.forEach(filter => {
      if (filter.apiEndpoint && areDependenciesSatisfied(filter)) {
        const dependencies = filter.dependsOn?.reduce((acc, dep) => {
          acc[dep] = values[dep];
          return acc;
        }, {} as Record<string, any>);

        loadFilterOptions(filter, dependencies);
      }
    });
  }, [values, filters, analysisId]);

  // Handle filter value change
  const handleFilterChange = (filterName: string, value: any) => {
    onChange(filterName, value);

    // Reload dependent filters
    filters.forEach(filter => {
      if (filter.dependsOn?.includes(filterName) && filter.apiEndpoint) {
        const dependencies = filter.dependsOn?.reduce((acc, dep) => {
          acc[dep] = dep === filterName ? value : values[dep];
          return acc;
        }, {} as Record<string, any>);

        loadFilterOptions(filter, dependencies);
      }
    });
  };

  // Get filters that can be added (not already in use and dependencies satisfied)
  const getAddableFilters = () => {
    return availableFilters.filter(filter =>
      !values.hasOwnProperty(filter.name) && areDependenciesSatisfied(filter)
    );
  };

  return (
    <div className={`space-y-3 ${className}`}>
      {/* Active Filters */}
      {Object.keys(values).length > 0 && (
        <div className="space-y-2">
          <div className="flex items-center gap-2">
            <span className="text-sm font-medium">Active Filters:</span>
          </div>

          {availableFilters
            .filter(filter => values.hasOwnProperty(filter.name))
            .map(filter => (
              <div key={filter.name} className="flex items-center gap-2">
                <span className="text-sm font-medium min-w-0 flex-1">{filter.label}:</span>

                {filter.type === "select" && (
                  <Select
                    value={values[filter.name]}
                    onValueChange={(value) => handleFilterChange(filter.name, value)}
                    disabled={loadingOptions.has(filter.name)}
                  >
                    <SelectTrigger className="w-48">
                      {loadingOptions.has(filter.name) ? (
                        <div className="flex items-center gap-2">
                          <Loader2 className="h-4 w-4 animate-spin" />
                          Loading...
                        </div>
                      ) : (
                        <SelectValue placeholder={filter.placeholder || `Select ${filter.label}`} />
                      )}
                    </SelectTrigger>
                    <SelectContent>
                      {getFilterOptions(filter).map(option => (
                        <SelectItem key={option.value} value={option.value} disabled={option.disabled}>
                          {option.label}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                )}

                {filter.type === "multiselect" && (
                  <div className="flex items-center gap-1 flex-wrap">
                    {(Array.isArray(values[filter.name]) ? values[filter.name] : [values[filter.name]]).map((val: string) => (
                      <Badge key={val} variant="secondary" className="flex items-center gap-1">
                        {val}
                        <button
                          onClick={() => {
                            const currentValues = Array.isArray(values[filter.name])
                              ? values[filter.name].filter((v: string) => v !== val)
                              : "";
                            handleFilterChange(filter.name, currentValues);
                          }}
                          className="ml-1 hover:bg-secondary-80 rounded-full p-0.5"
                        >
                          <X className="h-3 w-3" />
                        </button>
                      </Badge>
                    ))}
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => {
                        // Open selection dialog
                      }}
                      disabled={loadingOptions.has(filter.name)}
                    >
                      {loadingOptions.has(filter.name) ? (
                        <Loader2 className="h-3 w-3 animate-spin" />
                      ) : (
                        "+"
                      )}
                    </Button>
                  </div>
                )}

                {(filter.type === "text" || filter.type === "number") && (
                  <input
                    type={filter.type}
                    value={values[filter.name] || ""}
                    onChange={(e) => handleFilterChange(filter.name, e.target.value)}
                    placeholder={filter.placeholder || `Enter ${filter.label}`}
                    className="px-2 py-1 border rounded text-sm w-48"
                  />
                )}

                <button
                  onClick={() => onRemoveFilter(filter.name)}
                  className="text-red-500 hover:text-red-700"
                >
                  <X className="h-4 w-4" />
                </button>
              </div>
            ))}
        </div>
      )}

      {/* Add Filter Button */}
      <div className="flex items-center gap-2">
        <Select onValueChange={onAddFilter}>
          <SelectTrigger className="w-48">
            <SelectValue placeholder="Add filter..." />
          </SelectTrigger>
          <SelectContent>
            {getAddableFilters().map(filter => (
              <SelectItem key={filter.name} value={filter.name}>
                {filter.label}
                {filter.required && <span className="text-red-500 ml-1">*</span>}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>

        <Button
          variant="outline"
          size="sm"
          onClick={() => {
            // Reload all filter options
            filters.forEach(filter => {
              if (filter.apiEndpoint && areDependenciesSatisfied(filter)) {
                const dependencies = filter.dependsOn?.reduce((acc, dep) => {
                  acc[dep] = values[dep];
                  return acc;
                }, {} as Record<string, any>);
                loadFilterOptions(filter, dependencies);
              }
            });
          }}
        >
          <RefreshCw className="h-4 w-4" />
        </Button>
      </div>

      {/* Available filters info */}
      {getAddableFilters().length === 0 && Object.keys(values).length === 0 && (
        <div className="text-sm text-gray-500">
          No filters available for this analysis
        </div>
      )}
    </div>
  );
}

// Mock function to simulate API calls - replace with real API
async function getMockFilterOptions(filterName: string, dependencies?: Record<string, any>): Promise<FilterOption[]> {
  // Simulate API delay
  await new Promise(resolve => setTimeout(resolve, 500));

  switch (filterName) {
    case "country":
      return [
        { value: "US", label: "United States" },
        { value: "CA", label: "Canada" },
        { value: "UK", label: "United Kingdom" },
        { value: "DE", label: "Germany" },
        { value: "FR", label: "France" }
      ];

    case "region":
      if (dependencies?.country === "US") {
        return [
          { value: "west", label: "West Coast" },
          { value: "east", label: "East Coast" },
          { value: "central", label: "Central" },
          { value: "south", label: "South" }
        ];
      } else if (dependencies?.country === "CA") {
        return [
          { value: "ontario", label: "Ontario" },
          { value: "quebec", label: "Quebec" },
          { value: "bc", label: "British Columbia" }
        ];
      }
      return [];

    case "city":
      if (dependencies?.region === "west") {
        return [
          { value: "seattle", label: "Seattle, WA" },
          { value: "portland", label: "Portland, OR" },
          { value: "san-francisco", label: "San Francisco, CA" },
          { value: "los-angeles", label: "Los Angeles, CA" }
        ];
      } else if (dependencies?.region === "east") {
        return [
          { value: "new-york", label: "New York, NY" },
          { value: "boston", label: "Boston, MA" },
          { value: "miami", label: "Miami, FL" },
          { value: "washington", label: "Washington, DC" }
        ];
      }
      return [];

    case "datasource":
      return [
        { value: "user_data", label: "User Data" },
        { value: "transactions", label: "Transactions" },
        { value: "events", label: "Events" },
        { value: "products", label: "Products" }
      ];

    case "metric":
      if (dependencies?.datasource === "user_data") {
        return [
          { value: "active_users", label: "Active Users" },
          { value: "new_signups", label: "New Signups" },
          { value: "retention", label: "Retention Rate" },
          { value: "engagement", label: "Engagement Score" }
        ];
      } else if (dependencies?.datasource === "transactions") {
        return [
          { value: "revenue", label: "Revenue" },
          { value: "order_count", label: "Order Count" },
          { value: "avg_order_value", label: "Average Order Value" },
          { value: "conversion_rate", label: "Conversion Rate" }
        ];
      }
      return [];

    default:
      return [];
  }
}