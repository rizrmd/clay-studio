import type { AggregationResult } from "../types";

export function calculateAggregation(values: any[], aggregationType: string): any {
  // For "display" aggregation, just return the first non-null value
  if (aggregationType === "display") {
    const firstValue = values.find((v) => v !== null && v !== undefined);
    return firstValue !== undefined ? firstValue : null;
  }

  // Filter out null and undefined values
  const nonNullValues = values.filter((v) => v !== null && v !== undefined);

  // For count, return the count of available rows (non-null values)
  if (aggregationType === "count") {
    return nonNullValues.length;
  }

  if (nonNullValues.length === 0) {
    return null;
  }

  // Try to extract numeric values and track both valid and problematic ones
  const problematicValues: any[] = [];
  const validExamples: any[] = [];
  const numericValues = nonNullValues
    .map((v, index) => {
      const num = Number(v);
      if (isNaN(num)) {
        problematicValues.push({ value: v, index });
        return null;
      }
      // Track some valid examples (up to 5)
      if (validExamples.length < 5) {
        validExamples.push({ value: num, index });
      }
      return num;
    })
    .filter((v): v is number => v !== null);

  // For numeric aggregations (sum, avg, min, max), use numeric values if any exist
  if (["sum", "avg", "min", "max"].includes(aggregationType)) {
    if (numericValues.length === 0) {
      // No numeric values at all
      return null;
    }

    let result: number;
    switch (aggregationType) {
      case "sum":
        result = numericValues.reduce((a, b) => a + b, 0);
        break;
      case "avg":
        result =
          numericValues.reduce((a, b) => a + b, 0) / numericValues.length;
        break;
      case "min":
        result = Math.min(...numericValues);
        break;
      case "max":
        result = Math.max(...numericValues);
        break;
      default:
        result = numericValues[0] || 0;
    }

    // If there were problematic values, return with error details
    if (problematicValues.length > 0) {
      return {
        __hasError: true,
        __errorDetails: {
          problematicValues,
          validExamples,
          validCount: numericValues.length,
          totalCount: nonNullValues.length,
        },
        value: result,
      } as AggregationResult;
    }

    return result;
  }

  // For string/mixed values, use frequency-based aggregations
  const frequencyMap = new Map<any, number>();
  nonNullValues.forEach((value) => {
    const key = String(value);
    frequencyMap.set(key, (frequencyMap.get(key) || 0) + 1);
  });

  // Sort by frequency
  const sortedByFrequency = Array.from(frequencyMap.entries()).sort(
    (a, b) => a[1] - b[1]
  ); // Sort by count ascending

  switch (aggregationType) {
    case "min":
      // Return the value with least occurrence
      return sortedByFrequency[0]?.[0] || null;
    case "max":
      // Return the value with most occurrence
      return sortedByFrequency[sortedByFrequency.length - 1]?.[0] || null;
    case "sum":
      // For strings, return count of unique values instead of concatenating
      return `${frequencyMap.size} unique`;
    case "avg":
      // For strings, return most common value (mode)
      return sortedByFrequency[sortedByFrequency.length - 1]?.[0] || null;
    default:
      return nonNullValues[0];
  }
}