import { TableColumn } from "@/components/data-table/demo-data";
import { dataBrowserStore } from "@/lib/store/data-browser-store";

// Map database types to display types for the data table
export const mapDbTypeToDisplayType = (dbType: string): TableColumn["data_type"] => {
  const type = dbType.toLowerCase();

  // Numeric types
  if (
    type.includes("int") ||
    type.includes("serial") ||
    type.includes("number") ||
    type.includes("decimal") ||
    type.includes("float") ||
    type.includes("double") ||
    type.includes("numeric") ||
    type.includes("real")
  ) {
    return "number";
  }

  // Date/time types
  if (
    type.includes("date") ||
    type.includes("time") ||
    type.includes("timestamp")
  ) {
    return "date";
  }

  // Boolean types
  if (type.includes("bool") || type.includes("bit")) {
    return "boolean";
  }

  // Default to string for all other types (varchar, text, char, etc.)
  return "string";
};

// Analyze column content to determine data type and width
export const analyzeColumnContent = (
  columnName: string,
  rows: any[][]
): {
  maxLength: number;
  avgLength: number;
  hasNulls: boolean;
  isNumeric: boolean;
  isBoolean: boolean;
  dataType: string;
} => {
  const columnIndex =
    dataBrowserStore.tableData?.columns.indexOf(columnName) ?? -1;
  if (columnIndex === -1 || !rows.length) {
    return {
      maxLength: 0,
      avgLength: 0,
      hasNulls: false,
      isNumeric: false,
      isBoolean: false,
      dataType: "string",
    };
  }

  let totalLength = 0;
  let maxLength = 0;
  let nullCount = 0;
  let numericCount = 0;
  let booleanCount = 0;
  let dateCount = 0;
  const sampleSize = Math.min(rows.length, 100);

  for (let i = 0; i < sampleSize; i++) {
    const value = rows[i][columnIndex];
    const strValue = String(value ?? "");

    if (value === null || value === undefined || value === "") {
      nullCount++;
      continue;
    }

    const length = strValue.length;
    totalLength += length;
    maxLength = Math.max(maxLength, length);

    // Check if numeric
    if (!isNaN(Number(value)) && value !== "") {
      numericCount++;
    }

    // Check if boolean-like
    if (
      ["true", "false", "1", "0", "yes", "no", "draft", "published"].includes(
        strValue.toLowerCase()
      )
    ) {
      booleanCount++;
    }

    // Check if date-like
    if (Date.parse(strValue) && strValue.match(/\d{4}-\d{2}-\d{2}/)) {
      dateCount++;
    }
  }

  const validValues = sampleSize - nullCount;
  const isNumeric = validValues > 0 && numericCount / validValues > 0.8;
  const isBoolean = validValues > 0 && booleanCount / validValues > 0.6;
  const isDate = validValues > 0 && dateCount / validValues > 0.5;

  let dataType = "string";
  if (isDate) dataType = "date";
  else if (isBoolean) dataType = "boolean";
  else if (isNumeric) dataType = "number";

  return {
    maxLength,
    avgLength: validValues > 0 ? totalLength / validValues : 0,
    hasNulls: nullCount > 0,
    isNumeric,
    isBoolean,
    dataType,
  };
};

// Get appropriate width for column based on content and context
export const getColumnWidth = (columnName: string): number => {
  const rows = dataBrowserStore.tableData?.rows ?? [];
  const contentAnalysis = analyzeColumnContent(columnName, rows as any);

  // If no data, use column name length as fallback
  if (rows.length === 0) {
    const baseWidth = Math.max(100, columnName.length * 8 + 40);
    return Math.min(200, baseWidth);
  }

  // Get column name suggestions for common patterns
  const lowerName = columnName.toLowerCase();

  // ID columns - narrow
  if (
    lowerName === "id" ||
    lowerName.endsWith("_id") ||
    lowerName.includes("id")
  ) {
    return 120;
  }

  // Status/boolean columns - narrow
  if (
    contentAnalysis.dataType === "boolean" ||
    ["status", "active", "enabled", "published"].includes(lowerName)
  ) {
    return 100;
  }

  // Long text content - wider but capped
  if (
    [
      "token",
      "access_token",
      "refresh_token",
      "password",
      "description",
      "content",
    ].includes(lowerName)
  ) {
    return 180;
  }

  // Calculate width based on average character length with stricter limits
  const { avgLength, maxLength } = contentAnalysis;

  // Use smaller multiplier and tighter constraints
  const baseCalc = Math.max(100, Math.min(avgLength * 6, maxLength * 4) + 20);

  // Much tighter cap to prevent overflow
  return Math.min(180, baseCalc);
};