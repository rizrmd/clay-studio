use crate::core::mcp::types::*;
use crate::core::mcp::handlers::base::McpHandlers;
use serde_json::{json, Value};

/// Get interaction tools for the MCP tools list
pub fn get_interaction_tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "ask_user".to_string(),
            description: "Ask the user a question and wait for their response".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "question": {
                        "type": "string",
                        "description": "The question to ask the user"
                    },
                    "options": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Optional list of predefined response options"
                    }
                },
                "required": ["question"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "export_excel".to_string(),
            description:
                "Export data to an Excel file with multiple sheets and formatting options"
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "filename": {
                        "type": "string",
                        "description": "Name for the Excel file (without extension)"
                    },
                    "sheets": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": {
                                    "type": "string",
                                    "description": "Name of the worksheet"
                                },
                                "data": {
                                    "type": "array",
                                    "items": {"type": "array"},
                                    "description": "2D array of data rows"
                                },
                                "headers": {
                                    "type": "array",
                                    "items": {"type": "string"},
                                    "description": "Optional column headers"
                                }
                            },
                            "required": ["name", "data"]
                        },
                        "description": "Array of worksheets to include in the Excel file"
                    },
                    "options": {
                        "type": "object",
                        "properties": {
                            "auto_filter": {
                                "type": "boolean",
                                "default": true,
                                "description": "Enable auto filter on headers"
                            },
                            "freeze_panes": {
                                "type": "object",
                                "properties": {
                                    "row": {"type": "integer", "minimum": 0},
                                    "col": {"type": "integer", "minimum": 0}
                                },
                                "description": "Freeze panes at specified row/column"
                            },
                            "column_widths": {
                                "type": "object",
                                "patternProperties": {
                                    "^\\d+$": {"type": "number", "minimum": 1}
                                },
                                "description": "Custom column widths by column index"
                            }
                        },
                        "description": "Formatting and display options"
                    }
                },
                "required": ["filename", "sheets"]
            }),
        },
        Tool {
            name: "show_table".to_string(),
            description: "Display tabular data in an interactive table format".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "data": {
                        "type": "object",
                        "properties": {
                            "columns": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "key": {"type": "string"},
                                        "label": {"type": "string"},
                                        "data_type": {"type": "string"},
                                        "filterable": {"type": "boolean"},
                                        "sortable": {"type": "boolean"},
                                        "currency": {"type": "string"}
                                    },
                                    "required": ["key", "label", "data_type"]
                                },
                                "description": "Array of column definitions"
                            },
                            "rows": {
                                "type": "array",
                                "items": {"type": "object"},
                                "description": "Array of row objects"
                            }
                        },
                        "required": ["columns", "rows"],
                        "description": "Object with columns and rows structure"
                    },
                    "title": {
                        "type": "string",
                        "description": "Optional title for the table"
                    }
                },
                "required": ["data"]
            }),
        },
        Tool {
            name: "show_chart".to_string(),
            description: "Display data in various chart formats".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "data": {
                        "type": "object",
                        "properties": {
                            "categories": {
                                "type": "array",
                                "items": {"type": "string"},
                                "description": "Array of category labels for the x-axis"
                            },
                            "series": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "name": {"type": "string"},
                                        "data": {
                                            "type": "array",
                                            "items": {"type": "number"}
                                        }
                                    },
                                    "required": ["name", "data"]
                                },
                                "description": "Array of data series with name and values"
                            }
                        },
                        "required": ["categories", "series"],
                        "description": "Chart data with categories and series"
                    },
                    "chart_type": {
                        "type": "string",
                        "enum": ["line", "bar", "pie", "scatter", "area", "column", "donut", "radar", "gauge"],
                        "description": "Type of chart to display"
                    },
                    "title": {
                        "type": "string",
                        "description": "Chart title"
                    }
                },
                "required": ["data", "chart_type"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "file_list".to_string(),
            description: "List all uploaded files in the current project and conversation".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "conversation_id": {
                        "type": "string",
                        "description": "Optional conversation ID to filter files (if not provided, lists all project files)"
                    }
                },
                "additionalProperties": false
            }),
        },
        Tool {
            name: "file_read".to_string(),
            description: "Read the content of an uploaded file by its ID".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "file_id": {
                        "type": "string",
                        "description": "ID of the file to read"
                    }
                },
                "required": ["file_id"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "file_search".to_string(),
            description: "Search for files by name or description (metadata only) in the project".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query (searches in file names, descriptions, and content)"
                    },
                    "file_type": {
                        "type": "string",
                        "description": "Optional file type filter (e.g., 'text', 'image', 'json', 'csv')"
                    },
                    "conversation_id": {
                        "type": "string",
                        "description": "Optional conversation ID to limit search scope"
                    }
                },
                "required": ["query"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "file_metadata".to_string(),
            description: "Get detailed metadata about an uploaded file".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "file_id": {
                        "type": "string",
                        "description": "ID of the file to get metadata for"
                    }
                },
                "required": ["file_id"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "file_peek".to_string(),
            description: "Peek into parts of a large file with different sampling strategies".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "file_id": {
                        "type": "string",
                        "description": "ID of the file to peek into"
                    },
                    "strategy": {
                        "type": "string",
                        "enum": ["overview", "head", "tail", "middle", "distributed", "smart"],
                        "default": "smart",
                        "description": "Sampling strategy: overview (metadata only), head (beginning), tail (end), middle, distributed (evenly spaced samples), smart (auto-detect based on file type)"
                    },
                    "sample_size": {
                        "type": "integer",
                        "minimum": 100,
                        "maximum": 100000,
                        "default": 5000,
                        "description": "Size of content to retrieve in characters (for text) or items (for structured data)"
                    },
                    "options": {
                        "type": "object",
                        "properties": {
                            "sheet": {
                                "type": "string",
                                "description": "[Excel only] Specific sheet name to peek"
                            },
                            "pages": {
                                "type": "array",
                                "items": {"type": "integer"},
                                "description": "[PDF only] Specific page numbers to sample"
                            },
                            "columns": {
                                "type": "array",
                                "items": {"type": "string"},
                                "description": "[CSV/Excel only] Specific columns to include"
                            },
                            "encoding": {
                                "type": "string",
                                "description": "Text encoding (utf-8, latin1, etc.)"
                            }
                        },
                        "additionalProperties": false
                    }
                },
                "required": ["file_id"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "file_search_content".to_string(),
            description: "Search within a specific file for patterns with context".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "file_id": {
                        "type": "string",
                        "description": "ID of the file to search within"
                    },
                    "pattern": {
                        "type": "string",
                        "description": "Text or regex pattern to search for"
                    },
                    "search_type": {
                        "type": "string",
                        "enum": ["text", "regex", "fuzzy"],
                        "default": "text",
                        "description": "Type of search: exact text, regex pattern, or fuzzy matching"
                    },
                    "context_lines": {
                        "type": "integer",
                        "minimum": 0,
                        "maximum": 50,
                        "default": 3,
                        "description": "Number of surrounding lines/rows to include with each match"
                    },
                    "max_results": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 100,
                        "default": 10,
                        "description": "Maximum number of search results to return"
                    },
                    "case_sensitive": {
                        "type": "boolean",
                        "default": false,
                        "description": "Whether the search is case-sensitive"
                    },
                    "options": {
                        "type": "object",
                        "properties": {
                            "sheet": {
                                "type": "string",
                                "description": "[Excel only] Search within specific sheet"
                            },
                            "column": {
                                "type": "string",
                                "description": "[CSV/Excel only] Search within specific column"
                            },
                            "date_range": {
                                "type": "object",
                                "properties": {
                                    "start": {"type": "string", "format": "date"},
                                    "end": {"type": "string", "format": "date"}
                                },
                                "description": "[Logs/CSV only] Filter by date range"
                            }
                        },
                        "additionalProperties": false
                    }
                },
                "required": ["file_id", "pattern"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "file_download_url".to_string(),
            description: "Download a file from a URL and store it for analysis".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "format": "uri",
                        "description": "URL of the file to download (HTTP/HTTPS only)"
                    },
                    "file_name": {
                        "type": "string",
                        "description": "Optional custom name for the downloaded file"
                    },
                    "auto_extract": {
                        "type": "boolean",
                        "default": true,
                        "description": "Automatically extract content from the file after download"
                    },
                    "conversation_id": {
                        "type": "string",
                        "description": "Optional conversation ID to associate the file with"
                    }
                },
                "required": ["url"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "file_range".to_string(),
            description: "Extract a specific range of content from a large file".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "file_id": {
                        "type": "string",
                        "description": "ID of the file to extract from"
                    },
                    "unit": {
                        "type": "string",
                        "enum": ["lines", "bytes", "characters", "rows", "pages", "cells", "auto"],
                        "default": "auto",
                        "description": "Unit for range specification (auto detects based on file type)"
                    },
                    "start": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "Starting position (0-indexed)"
                    },
                    "end": {
                        "type": "integer",
                        "description": "Ending position (exclusive). If omitted, reads to end or reasonable limit"
                    },
                    "options": {
                        "type": "object",
                        "properties": {
                            "sheet": {
                                "type": "string",
                                "description": "[Excel only] Sheet to read from"
                            },
                            "columns": {
                                "type": "array",
                                "items": {"type": "string"},
                                "description": "[CSV/Excel only] Specific columns to include"
                            },
                            "format": {
                                "type": "string",
                                "enum": ["raw", "json", "csv", "markdown"],
                                "default": "raw",
                                "description": "Output format for the extracted content"
                            }
                        },
                        "additionalProperties": false
                    }
                },
                "required": ["file_id", "start"],
                "additionalProperties": false
            }),
        },
    ]
}

/// Check if a tool name is an interaction tool
pub fn is_interaction_tool(tool_name: &str) -> bool {
    matches!(tool_name, "ask_user" | "export_excel" | "show_table" | "show_chart" | "file_list" | "file_read" | "file_search" | "file_metadata" | "file_peek" | "file_search_content" | "file_range" | "file_download_url")
}

/// Handle interaction tool calls
pub async fn handle_tool_call(
    handlers: &McpHandlers,
    tool_name: &str,
    arguments: Option<&Value>
) -> Result<Value, JsonRpcError> {
    match tool_name {
        "ask_user" => {
            let empty_map = serde_json::Map::new();
            let args = arguments.and_then(|v| v.as_object()).unwrap_or(&empty_map);
            handlers.handle_ask_user(args).await.map(serde_json::Value::String)
        },
        "export_excel" => {
            let empty_map = serde_json::Map::new();
            let args = arguments.and_then(|v| v.as_object()).unwrap_or(&empty_map);
            handlers.handle_export_excel(args).await.map(serde_json::Value::String)
        },
        "show_table" => {
            handlers.handle_show_table(arguments).await
        },
        "show_chart" => {
            handlers.handle_show_chart(arguments).await
        },
        "file_list" => {
            let empty_map = serde_json::Map::new();
            let args = arguments.and_then(|v| v.as_object()).unwrap_or(&empty_map);
            handlers.handle_file_list(args).await.map(serde_json::Value::String)
        },
        "file_read" => {
            let empty_map = serde_json::Map::new();
            let args = arguments.and_then(|v| v.as_object()).unwrap_or(&empty_map);
            handlers.handle_file_read(args).await.map(serde_json::Value::String)
        },
        "file_search" => {
            let empty_map = serde_json::Map::new();
            let args = arguments.and_then(|v| v.as_object()).unwrap_or(&empty_map);
            handlers.handle_file_search(args).await.map(serde_json::Value::String)
        },
        "file_metadata" => {
            let empty_map = serde_json::Map::new();
            let args = arguments.and_then(|v| v.as_object()).unwrap_or(&empty_map);
            handlers.handle_file_metadata(args).await.map(serde_json::Value::String)
        },
        "file_peek" => {
            let empty_map = serde_json::Map::new();
            let args = arguments.and_then(|v| v.as_object()).unwrap_or(&empty_map);
            handlers.handle_file_peek(args).await
                .map(serde_json::Value::String)
                .map_err(|e| JsonRpcError {
                    code: INVALID_PARAMS,
                    message: e,
                    data: None,
                })
        },
        "file_range" => {
            let empty_map = serde_json::Map::new();
            let args = arguments.and_then(|v| v.as_object()).unwrap_or(&empty_map);
            handlers.handle_file_range(args).await
                .map(serde_json::Value::String)
                .map_err(|e| JsonRpcError {
                    code: INVALID_PARAMS,
                    message: e,
                    data: None,
                })
        },
        "file_download_url" => {
            let empty_map = serde_json::Map::new();
            let args = arguments.and_then(|v| v.as_object()).unwrap_or(&empty_map);
            handlers.handle_file_download_url(args).await
                .map(serde_json::Value::String)
                .map_err(|e| JsonRpcError {
                    code: INVALID_PARAMS,
                    message: e.message,
                    data: None,
                })
        },
        "file_search_content" => {
            let empty_map = serde_json::Map::new();
            let args = arguments.and_then(|v| v.as_object()).unwrap_or(&empty_map);
            handlers.handle_file_search_content(args).await
                .map(serde_json::Value::String)
                .map_err(|e| JsonRpcError {
                    code: INVALID_PARAMS,
                    message: e,
                    data: None,
                })
        },
        _ => Err(JsonRpcError {
            code: METHOD_NOT_FOUND,
            message: format!("Unknown interaction tool: {}", tool_name),
            data: None,
        })
    }
}
// Placeholder exports for missing types
#[allow(dead_code)]
pub struct InteractionHandlers;
