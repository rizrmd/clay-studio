import type { TableColumn } from "../demo-data";
import type { PivotRow, PivotTotal, DateRange } from "../types";
import { calculateAggregation } from "./aggregation";

export function processPivotData(
  data: any[],
  pivotColumns: string[],
  columnDefs: TableColumn[],
  aggregations: Record<string, string>
): { processedData: any[]; totalRow: PivotTotal | null } {
  if (pivotColumns.length === 0) {
    return { processedData: data, totalRow: null };
  }

  // Helper function to parse group key back to individual values
  const parseGroupKey = (key: string) => {
    return key.split("|||");
  };

  // Recursive function to create nested groups
  const createNestedGroups = (
    rows: any[],
    pivotCols: string[],
    level: number = 0,
    parentKey: string = ""
  ): any[] => {
    if (pivotCols.length === 0 || level >= pivotCols.length) {
      return rows;
    }

    const currentPivotCol = pivotCols[level];
    const groups: Record<string, any[]> = {};

    // Group data by current pivot column
    rows.forEach((row) => {
      // Convert pivot value to string to avoid formatting issues
      const rawValue = row[currentPivotCol];
      const pivotValue =
        rawValue !== null && rawValue !== undefined
          ? String(rawValue)
          : "No Group";
      if (!groups[pivotValue]) {
        groups[pivotValue] = [];
      }
      groups[pivotValue].push(row);
    });

    const result: any[] = [];

    Object.entries(groups).forEach(([groupKey, groupRows]) => {
      const fullKey = parentKey ? `${parentKey}|||${groupKey}` : groupKey;

      // Create pivot row for this group
      const pivotRow: PivotRow = {
        id: `pivot-${fullKey}`,
        __isPivotRow: true,
        __pivotLevel: level,
        __rowCount: groupRows.length,
        __groupKey: fullKey,
      };

      // Set values for all pivot columns up to current level
      pivotCols.forEach((col, idx) => {
        if (idx < level) {
          // For parent pivot columns, use the value from parent key
          const parentValues = parseGroupKey(fullKey);
          (pivotRow as any)[col] = parentValues[idx];
        } else if (idx === level) {
          // Current pivot column
          (pivotRow as any)[col] = groupKey;
        } else {
          // Child pivot columns - leave empty
          (pivotRow as any)[col] = "";
        }
      });

      // Calculate aggregations for this group (all columns are aggregatable)
      columnDefs.forEach((col) => {
        if (!pivotCols.includes(col.key)) {
          const values = groupRows.map((r) => r[col.key]);
          // Default aggregation is "display" for all columns
          const defaultAggregation = "display";
          const aggregationType =
            aggregations[col.key] || defaultAggregation;

          // Special handling for dates - show date range
          if (col.data_type === "date") {
            const dateValues = groupRows
              .map((r) => r[col.key])
              .filter(Boolean)
              .map((v) => new Date(v))
              .filter((d) => !isNaN(d.getTime()));

            if (dateValues.length > 0) {
              const minDate = new Date(
                Math.min(...dateValues.map((d) => d.getTime()))
              );
              const maxDate = new Date(
                Math.max(...dateValues.map((d) => d.getTime()))
              );

              if (minDate.getTime() === maxDate.getTime()) {
                (pivotRow as any)[col.key] = minDate.toISOString();
              } else {
                // Store as object for special rendering
                (pivotRow as any)[col.key] = {
                  __isDateRange: true,
                  min: minDate.toISOString(),
                  max: maxDate.toISOString(),
                } as DateRange;
              }
            } else {
              (pivotRow as any)[col.key] = null;
            }
          } else {
            // All other types use standard aggregation
            (pivotRow as any)[col.key] = calculateAggregation(
              values,
              aggregationType
            );
          }
        }
      });

      result.push(pivotRow);

      // If there are more pivot levels, create nested groups
      if (level < pivotCols.length - 1) {
        const nestedRows = createNestedGroups(
          groupRows,
          pivotCols,
          level + 1,
          fullKey
        );
        result.push(...nestedRows);
      }
    });

    return result;
  };

  // Create the nested pivot structure
  const processedData = createNestedGroups(data, pivotColumns);

  // Create a total row separately (not added to result)
  let totalRow: PivotTotal | null = null;
  if (processedData.length > 0) {
    const totalRowData: PivotTotal = {
      id: "pivot-total",
      __isPivotTotal: true,
      __rowCount: data.length,
    };

    // Set "TOTAL" for the first pivot column
    pivotColumns.forEach((col, idx) => {
      (totalRowData as any)[col] = idx === 0 ? "TOTAL" : "";
    });

    columnDefs.forEach((col) => {
      if (!pivotColumns.includes(col.key)) {
        const values = data.map((r) => r[col.key]);

        // Special handling for date columns - show date range
        if (col.data_type === "date") {
          const dateValues = values
            .filter(Boolean)
            .map((v) => new Date(v))
            .filter((d) => !isNaN(d.getTime()));

          if (dateValues.length > 0) {
            const minDate = new Date(
              Math.min(...dateValues.map((d) => d.getTime()))
            );
            const maxDate = new Date(
              Math.max(...dateValues.map((d) => d.getTime()))
            );

            if (minDate.getTime() === maxDate.getTime()) {
              (totalRowData as any)[col.key] = minDate.toISOString();
            } else {
              // Store as object for special rendering
              (totalRowData as any)[col.key] = {
                __isDateRange: true,
                min: minDate.toISOString(),
                max: maxDate.toISOString(),
              } as DateRange;
            }
          } else {
            (totalRowData as any)[col.key] = null;
          }
        } else {
          // All other columns use standard aggregation
          const defaultAggregation = "display";
          const aggregationType =
            aggregations[col.key] || defaultAggregation;
          const calculationResult = calculateAggregation(
            values,
            aggregationType
          );
          // For total row, always use the calculated value even if there are errors
          (totalRowData as any)[col.key] = calculationResult?.__hasError
            ? calculationResult
            : calculationResult;
        }
      }
    });

    totalRow = totalRowData;
  }

  return { processedData, totalRow };
}