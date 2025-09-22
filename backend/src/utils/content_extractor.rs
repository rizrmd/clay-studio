use std::fs;
use std::path::Path;
use serde_json::{json, Value};
use calamine::{Reader, Xlsx, Xls, open_workbook, Data};
use docx_rs::*;
use pdf_extract::extract_text;

/// Content extraction service for different file types
pub struct ContentExtractor;

/// Configuration for content extraction limits
pub struct ExtractionLimits {
    /// Maximum file size to attempt full parsing (in bytes)
    pub max_full_parse_size: u64,
    /// Maximum text content to store in database (in characters)
    pub max_text_content: usize,
    /// Maximum preview length (in characters)
    pub max_preview_length: usize,
    /// Maximum rows to extract from Excel sheets
    pub max_excel_rows: usize,
    /// Maximum sheets to process in Excel files
    pub max_excel_sheets: usize,
}

impl Default for ExtractionLimits {
    fn default() -> Self {
        Self {
            max_full_parse_size: std::env::var("MAX_FILE_PARSE_SIZE")
                .ok()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(10 * 1024 * 1024), // 10MB default
            max_text_content: std::env::var("MAX_TEXT_CONTENT")
                .ok()
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(1_000_000), // 1M characters default
            max_preview_length: std::env::var("MAX_PREVIEW_LENGTH")
                .ok()
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(5000), // 5K characters default
            max_excel_rows: std::env::var("MAX_EXCEL_ROWS")
                .ok()
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(10_000), // 10K rows default
            max_excel_sheets: std::env::var("MAX_EXCEL_SHEETS")
                .ok()
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(20), // 20 sheets default
        }
    }
}

impl ContentExtractor {
    /// Get current extraction limits configuration
    pub fn get_limits() -> ExtractionLimits {
        ExtractionLimits::default()
    }

    /// Extract meaningful content and metadata from any file type
    pub async fn extract_content(
        file_path: &Path,
        original_name: &str,
        mime_type: &str,
    ) -> Result<ExtractedContent, ContentExtractionError> {
        Self::extract_content_with_limits(file_path, original_name, mime_type, &ExtractionLimits::default()).await
    }

    /// Extract content with custom limits for large file handling
    pub async fn extract_content_with_limits(
        file_path: &Path,
        original_name: &str,
        mime_type: &str,
        limits: &ExtractionLimits,
    ) -> Result<ExtractedContent, ContentExtractionError> {
        // Check file size first
        let metadata = fs::metadata(file_path)
            .map_err(|e| ContentExtractionError::IoError(e.to_string()))?;
        
        let file_size = metadata.len();
        
        // If file is too large, return basic metadata only
        if file_size > limits.max_full_parse_size {
            return Self::extract_large_file_metadata(file_path, original_name, mime_type, file_size).await;
        }
        let file_extension = Path::new(original_name)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();

        let result = match mime_type {
            // Text files - read directly
            t if t.starts_with("text/") || Self::is_text_file(&file_extension) => {
                Self::extract_text_content_with_limits(file_path, original_name, limits).await
            }
            
            // Images - extract metadata and OCR text if possible
            t if t.starts_with("image/") => {
                Self::extract_image_content(file_path, original_name).await
            }
            
            // Spreadsheets - extract data as structured text
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" |
            "application/vnd.ms-excel" |
            "text/csv" => {
                Self::extract_spreadsheet_content_with_limits(file_path, original_name, &file_extension, limits).await
            }
            
            // PDFs - extract text content
            "application/pdf" => {
                Self::extract_pdf_content_with_limits(file_path, original_name, limits).await
            }
            
            // Word documents
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document" |
            "application/msword" => {
                Self::extract_document_content_with_limits(file_path, original_name, limits).await
            }
            
            // JSON files - validate and describe structure
            "application/json" => {
                Self::extract_json_content_with_limits(file_path, original_name, limits).await
            }
            
            // Default - basic file info
            _ => {
                Self::extract_basic_metadata(file_path, original_name, mime_type).await
            }
        };

        // Apply content truncation if needed
        result.map(|content| Self::apply_content_limits(content, limits))
    }

    /// Handle large files that exceed processing limits
    async fn extract_large_file_metadata(
        _file_path: &Path,
        _original_name: &str,
        mime_type: &str,
        file_size: u64,
    ) -> Result<ExtractedContent, ContentExtractionError> {
        let file_size_mb = file_size as f64 / (1024.0 * 1024.0);
        
        let description = format!(
            "Large file ({:.1} MB) - Too large for full content extraction. Basic metadata only.",
            file_size_mb
        );

        let file_type = Self::get_file_type_from_mime(mime_type);

        Ok(ExtractedContent {
            text_content: None,
            structured_data: Some(json!({
                "type": file_type,
                "size_bytes": file_size,
                "size_mb": file_size_mb,
                "large_file": true,
                "content_extraction_skipped": true,
                "suggested_actions": [
                    "File is too large for automatic processing",
                    "Consider splitting into smaller files",
                    "Manual processing may be required"
                ]
            })),
            description: Some(description),
            preview: None,
        })
    }

    /// Apply content limits to extracted content
    fn apply_content_limits(mut content: ExtractedContent, limits: &ExtractionLimits) -> ExtractedContent {
        // Truncate text content if too long
        if let Some(ref mut text) = content.text_content {
            if text.chars().count() > limits.max_text_content {
                let truncated: String = text.chars().take(limits.max_text_content).collect();
                *text = format!("{}... [Content truncated - original length: {} characters]", 
                               truncated, text.chars().count());
                
                // Update metadata to indicate truncation
                if let Some(ref mut data) = content.structured_data {
                    data["content_truncated"] = json!(true);
                    data["original_length"] = json!(text.chars().count());
                }
            }
        }

        // Truncate preview if too long
        if let Some(ref mut preview) = content.preview {
            if preview.chars().count() > limits.max_preview_length {
                let truncated: String = preview.chars().take(limits.max_preview_length).collect();
                *preview = format!("{}...", truncated);
            }
        }

        content
    }

    /// Get file type string from MIME type
    fn get_file_type_from_mime(mime_type: &str) -> &'static str {
        match mime_type {
            t if t.starts_with("text/") => "text",
            t if t.starts_with("image/") => "image",
            "application/pdf" => "pdf",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => "word_document",
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => "excel",
            "application/vnd.ms-excel" => "excel",
            "text/csv" => "csv",
            "application/json" => "json",
            _ => "binary_file"
        }
    }

    fn is_text_file(extension: &str) -> bool {
        matches!(extension, 
            "txt" | "md" | "markdown" | "log" | "conf" | "config" | 
            "js" | "ts" | "jsx" | "tsx" | "json" | "yaml" | "yml" |
            "html" | "htm" | "css" | "sql" | "py" | "rs" | "go" |
            "java" | "cpp" | "c" | "h" | "php" | "rb" | "sh" | "bash"
        )
    }

    #[allow(dead_code)]
    async fn extract_text_content(
        file_path: &Path,
        original_name: &str,
    ) -> Result<ExtractedContent, ContentExtractionError> {
        Self::extract_text_content_with_limits(file_path, original_name, &ExtractionLimits::default()).await
    }

    async fn extract_text_content_with_limits(
        file_path: &Path,
        _original_name: &str,
        limits: &ExtractionLimits,
    ) -> Result<ExtractedContent, ContentExtractionError> {
        let content = fs::read_to_string(file_path)
            .map_err(|e| ContentExtractionError::IoError(e.to_string()))?;
        
        let line_count = content.lines().count();
        let word_count = content.split_whitespace().count();
        let char_count = content.chars().count();
        
        // Extract preview (respecting limits)
        let preview_len = std::cmp::min(char_count, limits.max_preview_length);
        let preview = if char_count > preview_len {
            format!("{}...", &content.chars().take(preview_len).collect::<String>())
        } else {
            content.clone()
        };

        Ok(ExtractedContent {
            text_content: Some(content),
            structured_data: Some(json!({
                "type": "text_file",
                "stats": {
                    "lines": line_count,
                    "words": word_count,
                    "characters": char_count
                }
            })),
            description: Some(format!(
                "Text file with {} lines, {} words, {} characters",
                line_count, word_count, char_count
            )),
            preview: Some(preview),
        })
    }

    async fn extract_image_content(
        file_path: &Path,
        _original_name: &str,
    ) -> Result<ExtractedContent, ContentExtractionError> {
        // Get basic image metadata
        let metadata = fs::metadata(file_path)
            .map_err(|e| ContentExtractionError::IoError(e.to_string()))?;
        
        let file_size = metadata.len();
        
        // TODO: Add actual image processing here
        // For now, provide basic info and suggest what could be done
        let description = format!(
            "Image file ({} bytes). Content analysis available through Vision AI.", 
            file_size
        );

        Ok(ExtractedContent {
            text_content: None,
            structured_data: Some(json!({
                "type": "image",
                "size_bytes": file_size,
                "analysis_available": true,
                "suggested_actions": [
                    "Use Claude Vision API to analyze image content",
                    "Extract text using OCR if image contains text",
                    "Describe visual elements, charts, or diagrams"
                ]
            })),
            description: Some(description),
            preview: None,
        })
    }

    #[allow(dead_code)]
    async fn extract_spreadsheet_content(
        file_path: &Path,
        original_name: &str,
        extension: &str,
    ) -> Result<ExtractedContent, ContentExtractionError> {
        Self::extract_spreadsheet_content_with_limits(file_path, original_name, extension, &ExtractionLimits::default()).await
    }

    async fn extract_spreadsheet_content_with_limits(
        file_path: &Path,
        original_name: &str,
        extension: &str,
        limits: &ExtractionLimits,
    ) -> Result<ExtractedContent, ContentExtractionError> {
        match extension {
            "csv" => Self::extract_csv_content_with_limits(file_path, original_name, limits).await,
            "xlsx" | "xls" => Self::extract_excel_content_with_limits(file_path, original_name, limits).await,
            _ => Self::extract_basic_metadata(file_path, original_name, "spreadsheet").await,
        }
    }

    #[allow(dead_code)]
    async fn extract_csv_content(
        file_path: &Path,
        original_name: &str,
    ) -> Result<ExtractedContent, ContentExtractionError> {
        Self::extract_csv_content_with_limits(file_path, original_name, &ExtractionLimits::default()).await
    }

    async fn extract_csv_content_with_limits(
        file_path: &Path,
        _original_name: &str,
        limits: &ExtractionLimits,
    ) -> Result<ExtractedContent, ContentExtractionError> {
        let content = fs::read_to_string(file_path)
            .map_err(|e| ContentExtractionError::IoError(e.to_string()))?;
        
        let lines: Vec<&str> = content.lines().collect();
        let row_count = lines.len();
        
        if row_count == 0 {
            return Ok(ExtractedContent {
                text_content: Some(content),
                structured_data: Some(json!({"type": "csv", "rows": 0})),
                description: Some("Empty CSV file".to_string()),
                preview: None,
            });
        }

        // Get headers (first row)
        let headers = if let Some(first_line) = lines.first() {
            first_line.split(',').map(|h| h.trim()).collect::<Vec<_>>()
        } else {
            vec![]
        };

        let column_count = headers.len();
        
        // Create preview with limited rows
        let max_preview_rows = std::cmp::min(20, limits.max_excel_rows / 100); // Reasonable preview size
        let preview_rows = lines.iter().take(max_preview_rows).map(|line| line.to_string()).collect::<Vec<_>>();
        let preview = preview_rows.join("\n");
        
        // Truncate content if too many rows
        let processed_content = if row_count > limits.max_excel_rows {
            let truncated_lines: Vec<&str> = lines.iter().take(limits.max_excel_rows).cloned().collect();
            format!("{}... [CSV truncated - original had {} rows]", 
                   truncated_lines.join("\n"), row_count)
        } else {
            content.clone()
        };
        
        // Create structured description
        let description = format!(
            "CSV file with {} rows and {} columns. Headers: {}",
            row_count,
            column_count,
            headers.join(", ")
        );

        let mut structured_data = json!({
            "type": "csv",
            "rows": row_count,
            "columns": column_count,
            "headers": headers,
            "sample_data": preview_rows
        });

        // Add truncation info if applicable
        if row_count > limits.max_excel_rows {
            structured_data["truncated"] = json!(true);
            structured_data["original_rows"] = json!(row_count);
            structured_data["processed_rows"] = json!(limits.max_excel_rows);
        }

        Ok(ExtractedContent {
            text_content: Some(processed_content),
            structured_data: Some(structured_data),
            description: Some(description),
            preview: Some(preview),
        })
    }

    #[allow(dead_code)]
    async fn extract_excel_content(
        file_path: &Path,
        original_name: &str,
    ) -> Result<ExtractedContent, ContentExtractionError> {
        Self::extract_excel_content_with_limits(file_path, original_name, &ExtractionLimits::default()).await
    }

    async fn extract_excel_content_with_limits(
        file_path: &Path,
        _original_name: &str,
        limits: &ExtractionLimits,
    ) -> Result<ExtractedContent, ContentExtractionError> {
        // Try to extract content based on file extension
        let extension = file_path.extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();

        let result = if extension == "xlsx" {
            Self::extract_xlsx_content_with_limits(file_path, limits).await
        } else if extension == "xls" {
            Self::extract_xls_content_with_limits(file_path, limits).await
        } else {
            Err(ContentExtractionError::UnsupportedFormat(
                "Unsupported Excel format".to_string()
            ))
        };

        result.or_else(|_| {
            // Fall back to basic metadata if parsing fails
            let metadata = fs::metadata(file_path)
                .map_err(|e| ContentExtractionError::IoError(e.to_string()))?;
            
            let description = format!(
                "Excel file ({} bytes). Parsing failed, basic metadata only.",
                metadata.len()
            );

            Ok(ExtractedContent {
                text_content: None,
                structured_data: Some(json!({
                    "type": "excel",
                    "size_bytes": metadata.len(),
                    "parsing_failed": true
                })),
                description: Some(description),
                preview: None,
            })
        })
    }

    #[allow(dead_code)]
    async fn extract_xlsx_content(file_path: &Path) -> Result<ExtractedContent, ContentExtractionError> {
        Self::extract_xlsx_content_with_limits(file_path, &ExtractionLimits::default()).await
    }

    async fn extract_xlsx_content_with_limits(file_path: &Path, limits: &ExtractionLimits) -> Result<ExtractedContent, ContentExtractionError> {
        let mut workbook: Xlsx<_> = open_workbook(file_path)
            .map_err(|e| ContentExtractionError::ParseError(format!("XLSX error: {}", e)))?;
        
        Self::process_xlsx_workbook_with_limits(&mut workbook, limits).await
    }

    #[allow(dead_code)]
    async fn extract_xls_content(file_path: &Path) -> Result<ExtractedContent, ContentExtractionError> {
        Self::extract_xls_content_with_limits(file_path, &ExtractionLimits::default()).await
    }

    async fn extract_xls_content_with_limits(file_path: &Path, limits: &ExtractionLimits) -> Result<ExtractedContent, ContentExtractionError> {
        let mut workbook: Xls<_> = open_workbook(file_path)
            .map_err(|e| ContentExtractionError::ParseError(format!("XLS error: {}", e)))?;
        
        Self::process_xls_workbook_with_limits(&mut workbook, limits).await
    }

    #[allow(dead_code)]
    async fn process_xlsx_workbook(workbook: &mut Xlsx<std::io::BufReader<std::fs::File>>) -> Result<ExtractedContent, ContentExtractionError> {
        Self::process_xlsx_workbook_with_limits(workbook, &ExtractionLimits::default()).await
    }

    async fn process_xlsx_workbook_with_limits(workbook: &mut Xlsx<std::io::BufReader<std::fs::File>>, _limits: &ExtractionLimits) -> Result<ExtractedContent, ContentExtractionError> {
        let sheet_names = workbook.sheet_names().to_owned();
        let mut full_text = String::new();
        let mut total_rows = 0;
        let mut total_cols = 0;
        let mut sheets_data = Vec::new();

        // Process each sheet
        for sheet_name in &sheet_names {
            if let Ok(range) = workbook.worksheet_range(sheet_name) {
                let (rows, cols) = range.get_size();
                total_rows += rows;
                if cols > total_cols {
                    total_cols = cols;
                }

                // Extract data from this sheet
                let mut sheet_text = format!("=== Sheet: {} ===\n", sheet_name);
                let mut row_data = Vec::new();
                
                // Get first 20 rows for preview
                let preview_rows = std::cmp::min(rows, 20);
                for row_idx in 0..preview_rows {
                    let mut row_values = Vec::new();
                    for col_idx in 0..cols {
                        if let Some(cell) = range.get_value((row_idx as u32, col_idx as u32)) {
                            let cell_str = match cell {
                                Data::Empty => String::new(),
                                Data::String(s) => s.clone(),
                                Data::Float(f) => f.to_string(),
                                Data::Int(i) => i.to_string(),
                                Data::Bool(b) => b.to_string(),
                                Data::Error(e) => format!("#ERROR: {:?}", e),
                                Data::DateTime(dt) => dt.to_string(),
                                Data::DateTimeIso(dt) => dt.clone(),
                                Data::DurationIso(d) => d.clone(),
                            };
                            row_values.push(cell_str);
                        } else {
                            row_values.push(String::new());
                        }
                    }
                    row_data.push(row_values.clone());
                    sheet_text.push_str(&format!("{}\n", row_values.join("\t")));
                }

                full_text.push_str(&sheet_text);
                full_text.push('\n');

                sheets_data.push(json!({
                    "name": sheet_name,
                    "rows": rows,
                    "columns": cols,
                    "preview_data": row_data
                }));
            }
        }

        let char_count = full_text.chars().count();
        let preview = if char_count > 2000 {
            format!("{}...", &full_text.chars().take(2000).collect::<String>())
        } else {
            full_text.clone()
        };

        let description = format!(
            "Excel workbook with {} sheets, {} total rows, {} max columns",
            sheet_names.len(), total_rows, total_cols
        );

        Ok(ExtractedContent {
            text_content: Some(full_text),
            structured_data: Some(json!({
                "type": "excel",
                "stats": {
                    "sheets": sheet_names.len(),
                    "total_rows": total_rows,
                    "max_columns": total_cols,
                    "sheet_names": sheet_names
                },
                "sheets": sheets_data,
                "parsed_successfully": true
            })),
            description: Some(description),
            preview: Some(preview),
        })
    }

    #[allow(dead_code)]
    async fn process_xls_workbook(workbook: &mut Xls<std::io::BufReader<std::fs::File>>) -> Result<ExtractedContent, ContentExtractionError> {
        Self::process_xls_workbook_with_limits(workbook, &ExtractionLimits::default()).await
    }

    async fn process_xls_workbook_with_limits(workbook: &mut Xls<std::io::BufReader<std::fs::File>>, _limits: &ExtractionLimits) -> Result<ExtractedContent, ContentExtractionError> {
        let sheet_names = workbook.sheet_names().to_owned();
        let mut full_text = String::new();
        let mut total_rows = 0;
        let mut total_cols = 0;
        let mut sheets_data = Vec::new();

        // Process each sheet
        for sheet_name in &sheet_names {
            if let Ok(range) = workbook.worksheet_range(sheet_name) {
                let (rows, cols) = range.get_size();
                total_rows += rows;
                if cols > total_cols {
                    total_cols = cols;
                }

                // Extract data from this sheet
                let mut sheet_text = format!("=== Sheet: {} ===\n", sheet_name);
                let mut row_data = Vec::new();
                
                // Get first 20 rows for preview
                let preview_rows = std::cmp::min(rows, 20);
                for row_idx in 0..preview_rows {
                    let mut row_values = Vec::new();
                    for col_idx in 0..cols {
                        if let Some(cell) = range.get_value((row_idx as u32, col_idx as u32)) {
                            let cell_str = match cell {
                                Data::Empty => String::new(),
                                Data::String(s) => s.clone(),
                                Data::Float(f) => f.to_string(),
                                Data::Int(i) => i.to_string(),
                                Data::Bool(b) => b.to_string(),
                                Data::Error(e) => format!("#ERROR: {:?}", e),
                                Data::DateTime(dt) => dt.to_string(),
                                Data::DateTimeIso(dt) => dt.clone(),
                                Data::DurationIso(d) => d.clone(),
                            };
                            row_values.push(cell_str);
                        } else {
                            row_values.push(String::new());
                        }
                    }
                    row_data.push(row_values.clone());
                    sheet_text.push_str(&format!("{}\n", row_values.join("\t")));
                }

                full_text.push_str(&sheet_text);
                full_text.push('\n');

                sheets_data.push(json!({
                    "name": sheet_name,
                    "rows": rows,
                    "columns": cols,
                    "preview_data": row_data
                }));
            }
        }

        let char_count = full_text.chars().count();
        let preview = if char_count > 2000 {
            format!("{}...", &full_text.chars().take(2000).collect::<String>())
        } else {
            full_text.clone()
        };

        let description = format!(
            "Excel workbook with {} sheets, {} total rows, {} max columns",
            sheet_names.len(), total_rows, total_cols
        );

        Ok(ExtractedContent {
            text_content: Some(full_text),
            structured_data: Some(json!({
                "type": "excel",
                "stats": {
                    "sheets": sheet_names.len(),
                    "total_rows": total_rows,
                    "max_columns": total_cols,
                    "sheet_names": sheet_names
                },
                "sheets": sheets_data,
                "parsed_successfully": true
            })),
            description: Some(description),
            preview: Some(preview),
        })
    }

    #[allow(dead_code)]
    async fn extract_pdf_content(
        file_path: &Path,
        original_name: &str,
    ) -> Result<ExtractedContent, ContentExtractionError> {
        Self::extract_pdf_content_with_limits(file_path, original_name, &ExtractionLimits::default()).await
    }

    async fn extract_pdf_content_with_limits(
        file_path: &Path,
        _original_name: &str,
        _limits: &ExtractionLimits,
    ) -> Result<ExtractedContent, ContentExtractionError> {
        // Extract text from PDF using pdf-extract
        match extract_text(file_path) {
            Ok(text) => {
                let line_count = text.lines().count();
                let word_count = text.split_whitespace().count();
                let char_count = text.chars().count();
                
                // Create preview (first 2000 chars for PDFs as they tend to be longer)
                let preview = if char_count > 2000 {
                    format!("{}...", &text.chars().take(2000).collect::<String>())
                } else {
                    text.clone()
                };
                
                let description = format!(
                    "PDF document with {} lines, {} words, {} characters (text extracted)",
                    line_count, word_count, char_count
                );

                // Check if the text seems meaningful (not just whitespace/gibberish)
                let meaningful_content = word_count > 0 && char_count > 50;

                Ok(ExtractedContent {
                    text_content: Some(text),
                    structured_data: Some(json!({
                        "type": "pdf",
                        "stats": {
                            "lines": line_count,
                            "words": word_count,
                            "characters": char_count
                        },
                        "text_extraction_successful": true,
                        "meaningful_content": meaningful_content
                    })),
                    description: Some(description),
                    preview: Some(preview),
                })
            }
            Err(e) => {
                // Fall back to basic metadata if text extraction fails
                let metadata = fs::metadata(file_path)
                    .map_err(|e| ContentExtractionError::IoError(e.to_string()))?;
                
                let description = format!(
                    "PDF document ({} bytes). Text extraction failed: {}",
                    metadata.len(), e
                );

                Ok(ExtractedContent {
                    text_content: None,
                    structured_data: Some(json!({
                        "type": "pdf",
                        "size_bytes": metadata.len(),
                        "text_extraction_failed": true,
                        "error": e.to_string(),
                        "suggested_actions": [
                            "PDF may be image-based or encrypted",
                            "Try OCR for image-based PDFs",
                            "Check if PDF requires password"
                        ]
                    })),
                    description: Some(description),
                    preview: None,
                })
            }
        }
    }

    #[allow(dead_code)]
    async fn extract_document_content(
        file_path: &Path,
        original_name: &str,
    ) -> Result<ExtractedContent, ContentExtractionError> {
        Self::extract_document_content_with_limits(file_path, original_name, &ExtractionLimits::default()).await
    }

    async fn extract_document_content_with_limits(
        file_path: &Path,
        _original_name: &str,
        limits: &ExtractionLimits,
    ) -> Result<ExtractedContent, ContentExtractionError> {
        // Read and parse the .docx file
        let file_data = fs::read(file_path)
            .map_err(|e| ContentExtractionError::IoError(e.to_string()))?;
        
        match read_docx(&file_data) {
            Ok(docx) => {
                // Extract text content from all paragraphs
                let mut full_text = String::new();
                let mut paragraph_count = 0;
                let mut word_count = 0;
                
                for child in &docx.document.children {
                    if let DocumentChild::Paragraph(para) = child {
                        paragraph_count += 1;
                        for run_child in &para.children {
                            if let ParagraphChild::Run(run) = run_child {
                                for text_child in &run.children {
                                    if let RunChild::Text(text) = text_child {
                                        full_text.push_str(&text.text);
                                        word_count += text.text.split_whitespace().count();
                                    }
                                }
                            }
                        }
                        full_text.push('\n'); // Add newline after each paragraph
                    }
                }
                
                let char_count = full_text.chars().count();
                
                // Create preview (respecting limits)
                let preview_len = std::cmp::min(char_count, limits.max_preview_length);
                let preview = if char_count > preview_len {
                    format!("{}...", &full_text.chars().take(preview_len).collect::<String>())
                } else {
                    full_text.clone()
                };
                
                let description = format!(
                    "Word document with {} paragraphs, {} words, {} characters",
                    paragraph_count, word_count, char_count
                );

                Ok(ExtractedContent {
                    text_content: Some(full_text),
                    structured_data: Some(json!({
                        "type": "word_document",
                        "stats": {
                            "paragraphs": paragraph_count,
                            "words": word_count,
                            "characters": char_count
                        },
                        "parsed_successfully": true
                    })),
                    description: Some(description),
                    preview: Some(preview),
                })
            }
            Err(e) => {
                // Fall back to basic metadata if parsing fails
                let metadata = fs::metadata(file_path)
                    .map_err(|e| ContentExtractionError::IoError(e.to_string()))?;
                
                let description = format!(
                    "Word document ({} bytes). Failed to parse: {}",
                    metadata.len(), e
                );

                Ok(ExtractedContent {
                    text_content: None,
                    structured_data: Some(json!({
                        "type": "word_document",
                        "size_bytes": metadata.len(),
                        "parsing_failed": true,
                        "error": e.to_string()
                    })),
                    description: Some(description),
                    preview: None,
                })
            }
        }
    }

    #[allow(dead_code)]
    async fn extract_json_content(
        file_path: &Path,
        original_name: &str,
    ) -> Result<ExtractedContent, ContentExtractionError> {
        Self::extract_json_content_with_limits(file_path, original_name, &ExtractionLimits::default()).await
    }

    async fn extract_json_content_with_limits(
        file_path: &Path,
        _original_name: &str,
        limits: &ExtractionLimits,
    ) -> Result<ExtractedContent, ContentExtractionError> {
        let content = fs::read_to_string(file_path)
            .map_err(|e| ContentExtractionError::IoError(e.to_string()))?;
        
        // Parse JSON to validate and analyze structure
        match serde_json::from_str::<Value>(&content) {
            Ok(json_value) => {
                let structure_info = Self::analyze_json_structure(&json_value);
                let preview_len = std::cmp::min(content.len(), limits.max_preview_length);
                let preview = if content.len() > preview_len {
                    format!("{}...", &content[..preview_len])
                } else {
                    content.clone()
                };

                Ok(ExtractedContent {
                    text_content: Some(content),
                    structured_data: Some(json!({
                        "type": "json",
                        "valid": true,
                        "structure": structure_info
                    })),
                    description: Some(format!("Valid JSON file: {}", structure_info.description)),
                    preview: Some(preview),
                })
            }
            Err(e) => {
                Ok(ExtractedContent {
                    text_content: Some(content.clone()),
                    structured_data: Some(json!({
                        "type": "json",
                        "valid": false,
                        "error": e.to_string()
                    })),
                    description: Some(format!("Invalid JSON file: {}", e)),
                    preview: Some(content.chars().take(500).collect()),
                })
            }
        }
    }

    async fn extract_basic_metadata(
        file_path: &Path,
        _original_name: &str,
        mime_type: &str,
    ) -> Result<ExtractedContent, ContentExtractionError> {
        let metadata = fs::metadata(file_path)
            .map_err(|e| ContentExtractionError::IoError(e.to_string()))?;
        
        let description = format!(
            "File of type {} ({} bytes)",
            mime_type,
            metadata.len()
        );

        Ok(ExtractedContent {
            text_content: None,
            structured_data: Some(json!({
                "type": "binary_file",
                "mime_type": mime_type,
                "size_bytes": metadata.len()
            })),
            description: Some(description),
            preview: None,
        })
    }

    fn analyze_json_structure(value: &Value) -> JsonStructureInfo {
        match value {
            Value::Object(obj) => JsonStructureInfo {
                json_type: "object".to_string(),
                description: format!("Object with {} keys", obj.len()),
                details: Some(json!({
                    "keys": obj.keys().collect::<Vec<_>>(),
                    "key_count": obj.len()
                })),
            },
            Value::Array(arr) => JsonStructureInfo {
                json_type: "array".to_string(),
                description: format!("Array with {} items", arr.len()),
                details: Some(json!({
                    "length": arr.len(),
                    "item_types": arr.iter().map(Self::get_json_type).collect::<Vec<_>>()
                })),
            },
            _ => JsonStructureInfo {
                json_type: Self::get_json_type(value),
                description: format!("Single {} value", Self::get_json_type(value)),
                details: None,
            },
        }
    }

    fn get_json_type(value: &Value) -> String {
        match value {
            Value::Null => "null".to_string(),
            Value::Bool(_) => "boolean".to_string(),
            Value::Number(_) => "number".to_string(),
            Value::String(_) => "string".to_string(),
            Value::Array(_) => "array".to_string(),
            Value::Object(_) => "object".to_string(),
        }
    }
}

/// Extracted content from a file
#[derive(Debug, Clone)]
pub struct ExtractedContent {
    /// Text content if file is readable as text
    pub text_content: Option<String>,
    /// Structured metadata and analysis
    pub structured_data: Option<Value>,
    /// Human-readable description
    pub description: Option<String>,
    /// Preview of content (first N chars/lines)
    pub preview: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct JsonStructureInfo {
    json_type: String,
    description: String,
    details: Option<Value>,
}

#[derive(Debug)]
pub enum ContentExtractionError {
    IoError(String),
    ParseError(String),
    UnsupportedFormat(String),
    DocumentParseError(String),
}

impl std::fmt::Display for ContentExtractionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContentExtractionError::IoError(msg) => write!(f, "IO Error: {}", msg),
            ContentExtractionError::ParseError(msg) => write!(f, "Parse Error: {}", msg),
            ContentExtractionError::UnsupportedFormat(msg) => write!(f, "Unsupported Format: {}", msg),
            ContentExtractionError::DocumentParseError(msg) => write!(f, "Document Parse Error: {}", msg),
        }
    }
}

impl std::error::Error for ContentExtractionError {}