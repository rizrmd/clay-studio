use super::base::McpHandlers;
use crate::core::mcp::types::*;
use serde_json::{json, Value};
use std::path::PathBuf;
use tokio::fs;
use rust_xlsxwriter::{Workbook, Format, Color};
use chrono::Utc;
use uuid;

impl McpHandlers {
    pub async fn handle_export_excel(&self, arguments: &serde_json::Map<String, Value>) -> Result<String, JsonRpcError> {
        let filename = arguments.get("filename")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing filename".to_string(),
                data: None,
            })?;

        let sheets = arguments.get("sheets")
            .and_then(|v| v.as_array())
            .ok_or_else(|| JsonRpcError {
                code: INVALID_PARAMS,
                message: "Missing sheets array".to_string(),
                data: None,
            })?;

        // Validate sheets structure
        if sheets.is_empty() {
            return Err(JsonRpcError {
                code: INVALID_PARAMS,
                message: "At least one sheet must be provided".to_string(),
                data: None,
            });
        }

        // Validate each sheet
        for (i, sheet) in sheets.iter().enumerate() {
            if !sheet.is_object() {
                return Err(JsonRpcError {
                    code: INVALID_PARAMS,
                    message: format!("Sheet {} must be an object", i),
                    data: None,
                });
            }

            let sheet_obj = sheet.as_object().unwrap();
            if !sheet_obj.contains_key("name") || !sheet_obj.contains_key("data") {
                return Err(JsonRpcError {
                    code: INVALID_PARAMS,
                    message: format!("Sheet {} must have 'name' and 'data' fields", i),
                    data: None,
                });
            }

            if !sheet_obj["data"].is_array() {
                return Err(JsonRpcError {
                    code: INVALID_PARAMS,
                    message: format!("Sheet {} 'data' field must be an array of rows", i),
                    data: None,
                });
            }
        }

        // Extract optional parameters
        let options = arguments.get("options");

        // Create Excel export interaction response
        let interaction_id = uuid::Uuid::new_v4().to_string();
        let export_id = uuid::Uuid::new_v4().to_string();

        // Create directory structure for Excel exports following existing pattern
        let export_dir = PathBuf::from(".clients")
            .join(&self.client_id)
            .join(&self.project_id)
            .join("excel_exports");

        fs::create_dir_all(&export_dir).await
            .map_err(|e| JsonRpcError {
                code: INTERNAL_ERROR,
                message: format!("Failed to create export directory: {}", e),
                data: None,
            })?;

        // Generate Excel file
        let file_path = export_dir.join(format!("{}_{}.xlsx", export_id, filename));
        let relative_path = format!(".clients/{}/{}/excel_exports/{}_{}.xlsx",
            self.client_id, self.project_id, export_id, filename);

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
                .set_border(rust_xlsxwriter::FormatBorder::Thin);

            // Write headers if provided
            let mut row_idx = 0;
            if let Some(headers_array) = headers {
                for (col_idx, header) in headers_array.iter().enumerate() {
                    if let Some(header_str) = header.as_str() {
                        worksheet.write_string_with_format(row_idx, col_idx as u16, header_str, &header_format)
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
                                worksheet.write_string(row_idx, col_idx as u16, s)
                                    .map_err(|e| JsonRpcError {
                                        code: INTERNAL_ERROR,
                                        message: format!("Failed to write string cell: {}", e),
                                        data: None,
                                    })?;
                            }
                            Value::Number(n) => {
                                if let Some(int_val) = n.as_i64() {
                                    worksheet.write_number(row_idx, col_idx as u16, int_val as f64)
                                        .map_err(|e| JsonRpcError {
                                            code: INTERNAL_ERROR,
                                            message: format!("Failed to write number cell: {}", e),
                                            data: None,
                                        })?;
                                } else if let Some(float_val) = n.as_f64() {
                                    worksheet.write_number(row_idx, col_idx as u16, float_val)
                                        .map_err(|e| JsonRpcError {
                                            code: INTERNAL_ERROR,
                                            message: format!("Failed to write number cell: {}", e),
                                            data: None,
                                        })?;
                                }
                            }
                            Value::Bool(b) => {
                                worksheet.write_boolean(row_idx, col_idx as u16, *b)
                                    .map_err(|e| JsonRpcError {
                                        code: INTERNAL_ERROR,
                                        message: format!("Failed to write boolean cell: {}", e),
                                        data: None,
                                    })?;
                            }
                            Value::Null => {
                                worksheet.write_string(row_idx, col_idx as u16, "")
                                    .map_err(|e| JsonRpcError {
                                        code: INTERNAL_ERROR,
                                        message: format!("Failed to write empty cell: {}", e),
                                        data: None,
                                    })?;
                            }
                            _ => {
                                // Convert other types to string
                                worksheet.write_string(row_idx, col_idx as u16, &cell_value.to_string())
                                    .map_err(|e| JsonRpcError {
                                        code: INTERNAL_ERROR,
                                        message: format!("Failed to write cell: {}", e),
                                        data: None,
                                    })?;
                            }
                        }
                    }
                }
                row_idx += 1;
            }

            // Apply formatting options
            if let Some(opts) = options {
                // Auto filter
                if opts.get("auto_filter").and_then(|v| v.as_bool()).unwrap_or(true) {
                    let last_col = if let Some(headers_array) = headers {
                        headers_array.len().saturating_sub(1) as u16
                    } else if !data.is_empty() {
                        data[0].as_array().map(|r| r.len().saturating_sub(1) as u16).unwrap_or(0)
                    } else {
                        0
                    };

                    if last_col > 0 {
                        worksheet.autofilter(0, 0, 0, last_col).map_err(|e| JsonRpcError {
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
                    worksheet.set_freeze_panes(row, col).map_err(|e| JsonRpcError {
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
                                worksheet.set_column_width(col_idx, width as f64).map_err(|e| JsonRpcError {
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

        // Calculate total rows across all sheets for summary
        let total_rows: usize = sheets.iter()
            .filter_map(|sheet| sheet.as_object())
            .filter_map(|sheet| sheet.get("data"))
            .filter_map(|data| data.as_array())
            .map(|rows| rows.len())
            .sum();

        let sheet_count = sheets.len();

        // Get file size
        let file_size = fs::metadata(&file_path).await
            .map(|m| m.len())
            .unwrap_or(0);

        // Build the interaction spec with download URL
        let download_url = format!("/api/files/excel/{}/{}/{}", self.client_id, self.project_id, export_id);
        let interaction_spec = json!({
            "interaction_id": interaction_id,
            "interaction_type": "excel_export",
            "filename": filename,
            "export_id": export_id,
            "download_url": download_url,
            "file_path": relative_path,
            "file_size": file_size,
            "sheets": sheets,
            "options": options.unwrap_or(&json!({
                "auto_filter": true,
                "freeze_panes": null,
                "column_widths": {}
            })),
            "status": "completed",
            "requires_response": false,
            "created_at": chrono::Utc::now().to_rfc3339(),
        });

        // Format response for Excel export
        let response_text = format!(
            "📊 **Excel Export Completed**: {}\n\n\
            📄 **Sheets**: {}\n\
            📊 **Total Rows**: {}\n\
            📁 **File Size**: {} bytes\n\
            🔗 **Download**: {}\n\n\
            ```json\n{}\n```\n\n\
            ✨ The Excel file has been generated and is ready for download!",
            filename,
            sheet_count,
            total_rows,
            file_size,
            download_url,
            serde_json::to_string_pretty(&interaction_spec).unwrap_or_default()
        );

        Ok(response_text)
    }

    pub async fn cleanup_old_excel_files(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
                if path.is_file() && path.extension().map_or(false, |ext| ext == "xlsx") {
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