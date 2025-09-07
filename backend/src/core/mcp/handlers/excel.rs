use super::base::McpHandlers;
use crate::core::mcp::types::*;
use chrono::Utc;
use rust_xlsxwriter::{Color, Format, FormatBorder, Workbook};
use serde_json::{json, Value};
use std::path::PathBuf;
use tokio::fs;
use uuid;

impl McpHandlers {
    pub async fn handle_export_excel(
        &self,
        arguments: &serde_json::Map<String, Value>,
    ) -> Result<String, JsonRpcError> {
        let filename = arguments
            .get("filename")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing filename".to_string(),
                data: None,
            })?;

        let sheets = arguments
            .get("sheets")
            .and_then(|v| v.as_array())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing sheets array".to_string(),
                data: None,
            })?;

        // Validate sheets structure
        if sheets.is_empty() {
            let error_response = json!({
                "status": "error",
                "error": "Invalid parameter format for export_excel",
                "message": "At least one sheet must be provided",
                "correct_format_example": {
                    "filename": "report",
                    "sheets": [
                        {
                            "name": "Sheet1",
                            "data": [
                                ["Header1", "Header2", "Header3"],
                                ["Value1", "Value2", "Value3"],
                                ["Value4", "Value5", "Value6"]
                            ],
                            "headers": ["Header1", "Header2", "Header3"]
                        }
                    ],
                    "options": {
                        "auto_filter": true,
                        "freeze_panes": {"row": 1, "col": 0},
                        "column_widths": {"0": 15, "1": 20, "2": 25},
                        "borders": {
                            "style": "thin",
                            "color": "black",
                            "sides": ["all"]
                        }
                    }
                }
            });
            return Ok(serde_json::to_string(&error_response).unwrap());
        }

        // Validate each sheet
        for (i, sheet) in sheets.iter().enumerate() {
            if !sheet.is_object() {
                let error_response = json!({
                    "status": "error",
                    "error": "Invalid sheet structure",
                    "message": format!("Sheet {} must be an object", i),
                    "correct_format_example": {
                        "name": "Sheet Name",
                        "data": [
                            ["Header1", "Header2"],
                            ["Value1", "Value2"]
                        ],
                        "headers": ["Header1", "Header2"]
                    }
                });
                return Ok(serde_json::to_string(&error_response).unwrap());
            }

            let sheet_obj = sheet.as_object().unwrap();
            if !sheet_obj.contains_key("name") || !sheet_obj.contains_key("data") {
                let error_response = json!({
                    "status": "error",
                    "error": "Missing required sheet fields",
                    "message": format!("Sheet {} must have 'name' and 'data' fields", i),
                    "correct_format_example": {
                        "name": "My Data Sheet",
                        "data": [
                            ["ID", "Name", "Email"],
                            ["1", "John Doe", "john@example.com"],
                            ["2", "Jane Smith", "jane@example.com"]
                        ],
                        "headers": ["ID", "Name", "Email"]
                    }
                });
                return Ok(serde_json::to_string(&error_response).unwrap());
            }

            if !sheet_obj["data"].is_array() {
                let error_response = json!({
                    "status": "error",
                    "error": "Invalid data field type",
                    "message": format!("Sheet {} 'data' field must be an array of rows", i),
                    "correct_format_example": {
                        "name": "Data Sheet",
                        "data": [
                            ["Column 1", "Column 2", "Column 3"],
                            ["Row 1 Col 1", "Row 1 Col 2", "Row 1 Col 3"],
                            ["Row 2 Col 1", "Row 2 Col 2", "Row 2 Col 3"]
                        ]
                    }
                });
                return Ok(serde_json::to_string(&error_response).unwrap());
            }
        }

        // Extract optional parameters
        let options = arguments.get("options");

        // Create Excel export response
        let export_id = uuid::Uuid::new_v4().to_string();

        // Create directory structure for Excel exports following existing pattern
        let export_dir = PathBuf::from(".clients")
            .join(&self.client_id)
            .join(&self.project_id)
            .join("excel_exports");

        fs::create_dir_all(&export_dir)
            .await
            .map_err(|e| JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Failed to create export directory: {}", e),
                data: None,
            })?;

        // Generate Excel file
        let file_path = export_dir.join(format!("{}_{}.xlsx", export_id, filename));
        let relative_path = format!(
            ".clients/{}/{}/excel_exports/{}_{}.xlsx",
            self.client_id, self.project_id, export_id, filename
        );

        // Create workbook
        let mut workbook = Workbook::new();

        // Process each sheet
        for sheet_value in sheets {
            let sheet_obj = sheet_value.as_object().unwrap();
            let sheet_name = sheet_obj.get("name").unwrap().as_str().unwrap();
            let data = sheet_obj.get("data").unwrap().as_array().unwrap();
            let headers = sheet_obj.get("headers").and_then(|h| h.as_array());

            // Create worksheet
            let worksheet = workbook.add_worksheet();

            // Set worksheet name (truncate if too long)
            let ws_name = if sheet_name.len() > 31 {
                &sheet_name[..31]
            } else {
                sheet_name
            };
            worksheet.set_name(ws_name).map_err(|e| JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Failed to set worksheet name: {}", e),
                data: None,
            })?;

            // Create header format
            let header_format = Format::new()
                .set_bold()
                .set_background_color(Color::RGB(0xE6E6FA)) // Light lavender
                .set_border(FormatBorder::Thin);

            // Create data format based on border options
            let data_format = if let Some(opts) = options {
                create_data_format_with_borders(opts)
            } else {
                None
            };

            // Write headers if provided
            let mut row_idx = 0;
            if let Some(headers_array) = headers {
                for (col_idx, header) in headers_array.iter().enumerate() {
                    if let Some(header_str) = header.as_str() {
                        worksheet
                            .write_string_with_format(
                                row_idx,
                                col_idx as u16,
                                header_str,
                                &header_format,
                            )
                            .map_err(|e| JsonRpcError {
                                code: INTERNAL_ERROR,
                                message: format!("Failed to write header: {}", e),
                                data: None,
                            })?;
                    }
                }
                row_idx += 1;
            }

            // Write data rows
            for row in data {
                if let Some(row_array) = row.as_array() {
                    for (col_idx, cell_value) in row_array.iter().enumerate() {
                        match cell_value {
                            Value::String(s) => {
                                if let Some(ref format) = data_format {
                                    worksheet.write_string_with_format(row_idx, col_idx as u16, s, format).map_err(
                                        |e| JsonRpcError {
                                            code: INTERNAL_ERROR,
                                            message: format!("Failed to write string cell: {}", e),
                                            data: None,
                                        },
                                    )?;
                                } else {
                                    worksheet.write_string(row_idx, col_idx as u16, s).map_err(
                                        |e| JsonRpcError {
                                            code: INTERNAL_ERROR,
                                            message: format!("Failed to write string cell: {}", e),
                                            data: None,
                                        },
                                    )?;
                                }
                            }
                            Value::Number(n) => {
                                if let Some(int_val) = n.as_i64() {
                                    if let Some(ref format) = data_format {
                                        worksheet
                                            .write_number_with_format(row_idx, col_idx as u16, int_val as f64, format)
                                            .map_err(|e| JsonRpcError {
                                                code: INTERNAL_ERROR,
                                                message: format!("Failed to write number cell: {}", e),
                                                data: None,
                                            })?;
                                    } else {
                                        worksheet
                                            .write_number(row_idx, col_idx as u16, int_val as f64)
                                            .map_err(|e| JsonRpcError {
                                                code: INTERNAL_ERROR,
                                                message: format!("Failed to write number cell: {}", e),
                                                data: None,
                                            })?;
                                    }
                                } else if let Some(float_val) = n.as_f64() {
                                    if let Some(ref format) = data_format {
                                        worksheet
                                            .write_number_with_format(row_idx, col_idx as u16, float_val, format)
                                            .map_err(|e| JsonRpcError {
                                                code: INTERNAL_ERROR,
                                                message: format!("Failed to write number cell: {}", e),
                                                data: None,
                                            })?;
                                    } else {
                                        worksheet
                                            .write_number(row_idx, col_idx as u16, float_val)
                                            .map_err(|e| JsonRpcError {
                                                code: INTERNAL_ERROR,
                                                message: format!("Failed to write number cell: {}", e),
                                                data: None,
                                            })?;
                                    }
                                }
                            }
                            Value::Bool(b) => {
                                if let Some(ref format) = data_format {
                                    worksheet
                                        .write_boolean_with_format(row_idx, col_idx as u16, *b, format)
                                        .map_err(|e| JsonRpcError {
                                            code: INTERNAL_ERROR,
                                            message: format!("Failed to write boolean cell: {}", e),
                                            data: None,
                                        })?;
                                } else {
                                    worksheet
                                        .write_boolean(row_idx, col_idx as u16, *b)
                                        .map_err(|e| JsonRpcError {
                                            code: INTERNAL_ERROR,
                                            message: format!("Failed to write boolean cell: {}", e),
                                            data: None,
                                        })?;
                                }
                            }
                            Value::Null => {
                                if let Some(ref format) = data_format {
                                    worksheet
                                        .write_string_with_format(row_idx, col_idx as u16, "", format)
                                        .map_err(|e| JsonRpcError {
                                            code: INTERNAL_ERROR,
                                            message: format!("Failed to write empty cell: {}", e),
                                            data: None,
                                        })?;
                                } else {
                                    worksheet
                                        .write_string(row_idx, col_idx as u16, "")
                                        .map_err(|e| JsonRpcError {
                                            code: INTERNAL_ERROR,
                                            message: format!("Failed to write empty cell: {}", e),
                                            data: None,
                                        })?;
                                }
                            }
                            _ => {
                                // Convert other types to string
                                if let Some(ref format) = data_format {
                                    worksheet
                                        .write_string_with_format(row_idx, col_idx as u16, &cell_value.to_string(), format)
                                        .map_err(|e| JsonRpcError {
                                            code: INTERNAL_ERROR,
                                            message: format!("Failed to write cell: {}", e),
                                            data: None,
                                        })?;
                                } else {
                                    worksheet
                                        .write_string(row_idx, col_idx as u16, &cell_value.to_string())
                                        .map_err(|e| JsonRpcError {
                                            code: INTERNAL_ERROR,
                                            message: format!("Failed to write cell: {}", e),
                                            data: None,
                                        })?;
                                }
                            }
                        }
                    }
                }
                row_idx += 1;
            }

            // Apply formatting options
            if let Some(opts) = options {
                // Auto filter
                if opts
                    .get("auto_filter")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true)
                {
                    let last_col = if let Some(headers_array) = headers {
                        headers_array.len().saturating_sub(1) as u16
                    } else if !data.is_empty() {
                        data[0]
                            .as_array()
                            .map(|r| r.len().saturating_sub(1) as u16)
                            .unwrap_or(0)
                    } else {
                        0
                    };

                    if last_col > 0 {
                        worksheet
                            .autofilter(0, 0, 0, last_col)
                            .map_err(|e| JsonRpcError {
                                code: INTERNAL_ERROR,
                                message: format!("Failed to set autofilter: {}", e),
                                data: None,
                            })?;
                    }
                }

                // Freeze panes
                if let Some(freeze) = opts.get("freeze_panes").and_then(|v| v.as_object()) {
                    let row = freeze.get("row").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
                    let col = freeze.get("col").and_then(|v| v.as_u64()).unwrap_or(0) as u16;
                    worksheet
                        .set_freeze_panes(row, col)
                        .map_err(|e| JsonRpcError {
                            code: INTERNAL_ERROR,
                            message: format!("Failed to freeze panes: {}", e),
                            data: None,
                        })?;
                }

                // Column widths
                if let Some(widths) = opts.get("column_widths").and_then(|v| v.as_object()) {
                    for (col_str, width_val) in widths {
                        if let Ok(col_idx) = col_str.parse::<u16>() {
                            if let Some(width) = width_val.as_f64() {
                                worksheet
                                    .set_column_width(col_idx, width)
                                    .map_err(|e| JsonRpcError {
                                        code: INTERNAL_ERROR,
                                        message: format!("Failed to set column width: {}", e),
                                        data: None,
                                    })?;
                            }
                        }
                    }
                }
            }
        }

        // Save the workbook
        workbook.save(&file_path).map_err(|e| JsonRpcError {
            code: INTERNAL_ERROR,
            message: format!("Failed to save Excel file: {}", e),
            data: None,
        })?;

        // Cleanup old Excel files in the background
        let cleanup_self = self.clone();
        tokio::spawn(async move {
            if let Err(e) = cleanup_self.cleanup_old_excel_files().await {
                eprintln!(
                    "[{}] [WARNING] Failed to cleanup old Excel files: {}",
                    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
                    e
                );
            }
        });

        // Get file size
        let file_size = fs::metadata(&file_path).await.map(|m| m.len()).unwrap_or(0);

        // Build success response with download URL
        let download_url = format!(
            "/api/files/excel/{}/{}/{}",
            self.client_id, self.project_id, export_id
        );
        let response = json!({
            "status": "success",
            "message": "Excel file created successfully",
            "export_id": export_id,
            "download_url": download_url,
            "filename": format!("{}.xlsx", filename),
            "file_size": file_size,
            "sheets_count": sheets.len(),
            "file_info": {
                "relative_path": relative_path,
                "options": options.unwrap_or(&json!({
                    "auto_filter": true,
                    "freeze_panes": null,
                    "column_widths": {}
                })),
                "created_at": chrono::Utc::now().to_rfc3339()
            }
        });

        Ok(serde_json::to_string(&response).unwrap())
    }

    pub async fn cleanup_old_excel_files(
        &self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let export_dir = PathBuf::from(".clients")
            .join(&self.client_id)
            .join(&self.project_id)
            .join("excel_exports");

        if !export_dir.exists() {
            return Ok(());
        }

        let cutoff_time = Utc::now() - chrono::Duration::hours(24);

        if let Ok(mut entries) = fs::read_dir(&export_dir).await {
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if path.is_file() && path.extension().is_some_and(|ext| ext == "xlsx") {
                    if let Ok(metadata) = fs::metadata(&path).await {
                        if let Ok(modified) = metadata.modified() {
                            let modified_time = chrono::DateTime::<Utc>::from(modified);
                            if modified_time < cutoff_time {
                                if let Err(e) = fs::remove_file(&path).await {
                                    eprintln!(
                                        "[{}] [WARNING] Failed to remove old Excel file {}: {}",
                                        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
                                        path.display(),
                                        e
                                    );
                                } else {
                                    eprintln!(
                                        "[{}] [INFO] Removed old Excel file: {}",
                                        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
                                        path.display()
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

// Helper function to create data format with border options
fn create_data_format_with_borders(options: &Value) -> Option<Format> {
    if let Some(border_opts) = options.get("borders").and_then(|v| v.as_object()) {
        let mut format = Format::new();
        
        // Parse border style
        let border_style = border_opts
            .get("style")
            .and_then(|v| v.as_str())
            .unwrap_or("thin");
            
        let format_border = match border_style {
            "none" => return None,
            "thin" => FormatBorder::Thin,
            "medium" => FormatBorder::Medium,
            "thick" => FormatBorder::Thick,
            "double" => FormatBorder::Double,
            "dotted" => FormatBorder::Dotted,
            "dashed" => FormatBorder::Dashed,
            _ => FormatBorder::Thin,
        };
        
        // Parse border color
        let border_color = if let Some(color_str) = border_opts.get("color").and_then(|v| v.as_str()) {
            parse_color(color_str).unwrap_or(Color::Black)
        } else {
            Color::Black
        };
        
        // Apply borders based on sides specified
        if let Some(sides) = border_opts.get("sides") {
            if let Some(sides_array) = sides.as_array() {
                // Apply specific sides
                for side in sides_array {
                    if let Some(side_str) = side.as_str() {
                        match side_str {
                            "top" => format = format.set_border_top_color(border_color).set_border_top(format_border),
                            "bottom" => format = format.set_border_bottom_color(border_color).set_border_bottom(format_border),
                            "left" => format = format.set_border_left_color(border_color).set_border_left(format_border),
                            "right" => format = format.set_border_right_color(border_color).set_border_right(format_border),
                            "all" => format = format.set_border_color(border_color).set_border(format_border),
                            _ => {}
                        }
                    }
                }
            } else if let Some(sides_str) = sides.as_str() {
                // Single side as string
                match sides_str {
                    "top" => format = format.set_border_top_color(border_color).set_border_top(format_border),
                    "bottom" => format = format.set_border_bottom_color(border_color).set_border_bottom(format_border),
                    "left" => format = format.set_border_left_color(border_color).set_border_left(format_border),
                    "right" => format = format.set_border_right_color(border_color).set_border_right(format_border),
                    "all" => format = format.set_border_color(border_color).set_border(format_border),
                    _ => {}
                }
            }
        } else {
            // Default to all sides if no specific sides specified
            format = format.set_border_color(border_color).set_border(format_border);
        }
        
        Some(format)
    } else {
        None
    }
}

// Helper function to parse color strings
fn parse_color(color_str: &str) -> Option<Color> {
    match color_str.to_lowercase().as_str() {
        "black" => Some(Color::Black),
        "white" => Some(Color::White),
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "blue" => Some(Color::Blue),
        "yellow" => Some(Color::Yellow),
        "magenta" => Some(Color::Magenta),
        "cyan" => Some(Color::Cyan),
        _ => {
            // Try to parse hex color (#RRGGBB or #RGB)
            if color_str.starts_with('#') {
                let hex = &color_str[1..];
                if hex.len() == 6 {
                    if let Ok(rgb) = u32::from_str_radix(hex, 16) {
                        return Some(Color::RGB(rgb));
                    }
                } else if hex.len() == 3 {
                    // Convert #RGB to #RRGGBB
                    let r = u32::from_str_radix(&hex[0..1], 16).ok()? * 17;
                    let g = u32::from_str_radix(&hex[1..2], 16).ok()? * 17;
                    let b = u32::from_str_radix(&hex[2..3], 16).ok()? * 17;
                    return Some(Color::RGB((r << 16) | (g << 8) | b));
                }
            }
            None
        }
    }
}
