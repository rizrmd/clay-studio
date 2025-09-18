use crate::core::mcp::handlers::base::McpHandlers;
use crate::models::file_upload::FileUpload;
use serde_json::{json, Value};
use uuid::Uuid;

/// File type detection for parameter validation
#[derive(Debug, PartialEq)]
pub enum DetectedFileType {
    Excel,
    Csv,
    Pdf,
    Word,
    Text,
    Json,
    Xml,
    Log,
    Binary,
    Unknown,
}

impl DetectedFileType {
    fn from_mime_type(mime_type: &str, file_name: &str) -> Self {
        // Check MIME type first
        let mime_lower = mime_type.to_lowercase();
        
        if mime_lower.contains("excel") || mime_lower.contains("spreadsheet") {
            return DetectedFileType::Excel;
        }
        if mime_lower.contains("pdf") {
            return DetectedFileType::Pdf;
        }
        if mime_lower.contains("word") || mime_lower.contains("document") {
            return DetectedFileType::Word;
        }
        if mime_lower.contains("csv") {
            return DetectedFileType::Csv;
        }
        if mime_lower.contains("json") {
            return DetectedFileType::Json;
        }
        if mime_lower.contains("xml") {
            return DetectedFileType::Xml;
        }
        if mime_lower.contains("text") {
            return DetectedFileType::Text;
        }
        
        // Fallback to file extension
        let extension = file_name.split('.').next_back().unwrap_or("").to_lowercase();
        match extension.as_str() {
            "xlsx" | "xls" | "xlsm" | "xlsb" => DetectedFileType::Excel,
            "csv" | "tsv" => DetectedFileType::Csv,
            "pdf" => DetectedFileType::Pdf,
            "docx" | "doc" | "docm" => DetectedFileType::Word,
            "txt" | "md" | "rst" => DetectedFileType::Text,
            "json" | "jsonl" => DetectedFileType::Json,
            "xml" | "html" | "xhtml" => DetectedFileType::Xml,
            "log" => DetectedFileType::Log,
            _ => DetectedFileType::Unknown,
        }
    }
    
    /// Get valid units for this file type
    fn valid_units(&self) -> Vec<&'static str> {
        match self {
            DetectedFileType::Excel => vec!["rows", "cells", "lines", "auto"],
            DetectedFileType::Csv => vec!["rows", "lines", "bytes", "characters", "auto"],
            DetectedFileType::Pdf => vec!["pages", "lines", "characters", "bytes", "auto"],
            DetectedFileType::Word => vec!["pages", "lines", "characters", "bytes", "auto"],
            DetectedFileType::Text | DetectedFileType::Log => vec!["lines", "bytes", "characters", "auto"],
            DetectedFileType::Json | DetectedFileType::Xml => vec!["lines", "bytes", "characters", "auto"],
            _ => vec!["bytes", "auto"],
        }
    }
    
    /// Get the best unit for this file type when "auto" is specified
    fn best_unit(&self) -> &'static str {
        match self {
            DetectedFileType::Excel => "rows",
            DetectedFileType::Csv => "rows",
            DetectedFileType::Pdf => "pages",
            DetectedFileType::Word => "pages",
            DetectedFileType::Text | DetectedFileType::Log => "lines",
            DetectedFileType::Json | DetectedFileType::Xml => "lines",
            _ => "bytes",
        }
    }
    
    /// Check if a specific option is valid for this file type
    fn is_option_valid(&self, option_name: &str) -> bool {
        matches!((self, option_name), 
            (DetectedFileType::Excel, "sheet") | 
            (DetectedFileType::Excel, "columns") | 
            (DetectedFileType::Csv, "columns") | 
            (DetectedFileType::Pdf, "pages") | 
            (DetectedFileType::Log, "date_range") | 
            (DetectedFileType::Csv, "date_range")
        )
    }
}

/// Parameter validation result
struct ValidationResult {
    is_valid: bool,
    corrected_unit: Option<String>,
    warnings: Vec<String>,
    errors: Vec<String>,
}

impl McpHandlers {
    /// Handle file_search_content tool - search within a specific file
    pub async fn handle_file_search_content(&self, args: &serde_json::Map<String, Value>) -> Result<String, String> {
        // Extract parameters
        let file_id = args
            .get("file_id")
            .and_then(|v| v.as_str())
            .ok_or("Missing required parameter: file_id")?;
        
        let pattern = args
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or("Missing required parameter: pattern")?;
        
        let search_type = args
            .get("search_type")
            .and_then(|v| v.as_str())
            .unwrap_or("text");
        
        let _context_lines = args
            .get("context_lines")
            .and_then(|v| v.as_i64())
            .unwrap_or(3) as usize;
        
        let _max_results = args
            .get("max_results")
            .and_then(|v| v.as_i64())
            .unwrap_or(10) as usize;
        
        let _case_sensitive = args
            .get("case_sensitive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        // Get file info
        let file_info = self.get_file_info(file_id).await?;
        
        // Build response (placeholder for actual implementation)
        let response = json!({
            "file_id": file_id,
            "file_name": file_info.original_name,
            "pattern": pattern,
            "search_type": search_type,
            "matches_found": 0,
            "results": [],
            "note": "Search within file implementation placeholder"
        });
        
        Ok(serde_json::to_string_pretty(&response).unwrap_or_else(|_| "Failed to serialize response".to_string()))
    }
    
    /// Handle file_peek tool with parameter validation
    pub async fn handle_file_peek(&self, args: &serde_json::Map<String, Value>) -> Result<String, String> {
        // Extract parameters
        let file_id = args
            .get("file_id")
            .and_then(|v| v.as_str())
            .ok_or("Missing required parameter: file_id")?;
        
        let strategy = args
            .get("strategy")
            .and_then(|v| v.as_str())
            .unwrap_or("smart");
        
        let sample_size = args
            .get("sample_size")
            .and_then(|v| v.as_i64())
            .unwrap_or(5000) as usize;
        
        let options = args
            .get("options")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();
        
        // Get file metadata and validate parameters
        let file_info = self.get_file_info(file_id).await?;
        let file_type = DetectedFileType::from_mime_type(
            &file_info.mime_type.clone().unwrap_or_default(),
            &file_info.original_name
        );
        
        // Validate options for this file type
        let validation_results = self.validate_options(&file_type, &options);
        
        // Build response with appropriate warnings
        let mut response = json!({
            "file_id": file_id,
            "file_name": file_info.original_name,
            "file_type": format!("{:?}", file_type),
            "strategy_used": strategy,
            "sample_size": sample_size,
        });
        
        if !validation_results.warnings.is_empty() {
            response["warnings"] = json!(validation_results.warnings);
        }
        
        if !validation_results.errors.is_empty() {
            response["errors"] = json!(validation_results.errors);
            response["content"] = json!("Unable to peek file due to parameter errors");
        } else {
            // Perform actual file peek operation
            let content = self.peek_file_content(&file_info, strategy, sample_size, &options, &file_type).await?;
            response["content"] = json!(content);
        }
        
        Ok(serde_json::to_string_pretty(&response).unwrap_or_else(|_| "Failed to serialize response".to_string()))
    }
    
    /// Handle file_range tool with parameter validation and auto-correction
    pub async fn handle_file_range(&self, args: &serde_json::Map<String, Value>) -> Result<String, String> {
        // Extract parameters
        let file_id = args
            .get("file_id")
            .and_then(|v| v.as_str())
            .ok_or("Missing required parameter: file_id")?;
        
        let unit = args
            .get("unit")
            .and_then(|v| v.as_str())
            .unwrap_or("auto");
        
        let start = args
            .get("start")
            .and_then(|v| v.as_i64())
            .ok_or("Missing required parameter: start")? as usize;
        
        let end = args
            .get("end")
            .and_then(|v| v.as_i64())
            .map(|e| e as usize);
        
        let options = args
            .get("options")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();
        
        // Get file metadata
        let file_info = self.get_file_info(file_id).await?;
        let file_type = DetectedFileType::from_mime_type(
            &file_info.mime_type.clone().unwrap_or_default(),
            &file_info.original_name
        );
        
        // Validate and potentially correct the unit parameter
        let validation = self.validate_unit(&file_type, unit);
        let effective_unit = validation.corrected_unit.clone().as_deref().map(|s| s.to_string()).unwrap_or_else(|| unit.to_string());
        
        // Build response
        let mut response = json!({
            "file_id": file_id,
            "file_name": file_info.original_name,
            "file_type": format!("{:?}", file_type),
            "requested_unit": unit,
            "effective_unit": effective_unit,
            "start": start,
            "end": end,
        });
        
        // Add warnings if parameter was auto-corrected
        if let Some(ref corrected) = validation.corrected_unit {
            response["parameter_correction"] = json!({
                "reason": format!(
                    "Unit '{}' is not valid for {} files. Automatically corrected to '{}'",
                    unit,
                    format!("{:?}", file_type).to_lowercase(),
                    corrected
                ),
                "valid_units": file_type.valid_units(),
                "suggested_unit": file_type.best_unit(),
            });
        }
        
        if !validation.warnings.is_empty() {
            response["warnings"] = json!(validation.warnings);
        }
        
        if !validation.errors.is_empty() {
            response["errors"] = json!(validation.errors);
            response["content"] = json!("Unable to extract range due to parameter errors");
        } else {
            // Perform actual range extraction
            let content = self.extract_file_range(
                &file_info,
                &effective_unit,
                start,
                end,
                &options,
                &file_type
            ).await?;
            response["content"] = json!(content);
        }
        
        Ok(serde_json::to_string_pretty(&response).unwrap_or_else(|_| "Failed to serialize response".to_string()))
    }
    
    /// Validate unit parameter for file type
    fn validate_unit(&self, file_type: &DetectedFileType, unit: &str) -> ValidationResult {
        let valid_units = file_type.valid_units();
        let mut result = ValidationResult {
            is_valid: true,
            corrected_unit: None,
            warnings: Vec::new(),
            errors: Vec::new(),
        };
        
        // Handle "auto" unit
        if unit == "auto" {
            result.corrected_unit = Some(file_type.best_unit().to_string());
            return result;
        }
        
        // Check if unit is valid for this file type
        if !valid_units.contains(&unit) {
            result.is_valid = false;
            
            // Provide intelligent correction
            let best_unit = match (file_type, unit) {
                // Excel file with invalid unit
                (DetectedFileType::Excel, "pages") => "rows",
                (DetectedFileType::Excel, "lines") => "rows",
                
                // PDF with invalid unit  
                (DetectedFileType::Pdf, "cells") => "pages",
                (DetectedFileType::Pdf, "rows") => "lines",
                
                // CSV with invalid unit
                (DetectedFileType::Csv, "pages") => "rows",
                (DetectedFileType::Csv, "cells") => "rows",
                
                // Text files with invalid unit
                (DetectedFileType::Text, "pages") => "lines",
                (DetectedFileType::Text, "rows") => "lines",
                (DetectedFileType::Text, "cells") => "lines",
                
                // Default fallback
                _ => file_type.best_unit(),
            };
            
            result.corrected_unit = Some(best_unit.to_string());
            result.warnings.push(format!(
                "Unit '{}' is not applicable to {} files. Using '{}' instead. Valid units are: {:?}",
                unit,
                format!("{:?}", file_type).to_lowercase(),
                best_unit,
                valid_units
            ));
        }
        
        result
    }
    
    /// Validate options for file type
    fn validate_options(&self, file_type: &DetectedFileType, options: &serde_json::Map<String, Value>) -> ValidationResult {
        let mut result = ValidationResult {
            is_valid: true,
            corrected_unit: None,
            warnings: Vec::new(),
            errors: Vec::new(),
        };
        
        for (key, _value) in options.iter() {
            if !file_type.is_option_valid(key) {
                let warning = match (file_type, key.as_str()) {
                    (DetectedFileType::Pdf, "sheet") => {
                        "Option 'sheet' is only valid for Excel files, not PDF. This option will be ignored.".to_string()
                    },
                    (DetectedFileType::Pdf, "columns") => {
                        "Option 'columns' is only valid for CSV/Excel files, not PDF. This option will be ignored.".to_string()
                    },
                    (DetectedFileType::Text, "pages") => {
                        "Option 'pages' is only valid for PDF files, not text files. This option will be ignored.".to_string()
                    },
                    (DetectedFileType::Excel, "pages") => {
                        "Option 'pages' is only valid for PDF files. Use 'sheet' to specify Excel worksheet.".to_string()
                    },
                    _ => {
                        format!("Option '{}' is not applicable to {} files and will be ignored.", 
                            key, format!("{:?}", file_type).to_lowercase())
                    }
                };
                result.warnings.push(warning);
            }
        }
        
        result
    }
    
    /// Get file information from database
    async fn get_file_info(&self, file_id: &str) -> Result<FileUpload, String> {
        let file_uuid = Uuid::parse_str(file_id)
            .map_err(|e| format!("Invalid file_id format: {}", e))?;
        
        let file_info = sqlx::query_as::<_, FileUpload>(
            "SELECT * FROM file_uploads WHERE id = $1"
        )
        .bind(file_uuid)
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or_else(|| format!("File not found: {}", file_id))?;
        
        Ok(file_info)
    }
    
    /// Peek file content with validated parameters
    async fn peek_file_content(
        &self,
        _file_info: &FileUpload,
        strategy: &str,
        sample_size: usize,
        options: &serde_json::Map<String, Value>,
        file_type: &DetectedFileType,
    ) -> Result<String, String> {
        // Implementation would read file based on strategy and file type
        // This is a placeholder - actual implementation would use file readers
        
        let content_preview = match (file_type, strategy) {
            (DetectedFileType::Excel, "smart") => {
                format!("Excel file preview: {} rows sampled from {} sheets", 
                    sample_size.min(100), 
                    options.get("sheet").and_then(|v| v.as_str()).unwrap_or("all"))
            },
            (DetectedFileType::Pdf, "smart") => {
                format!("PDF file preview: First {} characters from {} pages",
                    sample_size,
                    options.get("pages").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(1))
            },
            _ => {
                format!("File preview using {} strategy: {} bytes", strategy, sample_size)
            }
        };
        
        Ok(content_preview)
    }
    
    /// Extract file range with validated parameters
    async fn extract_file_range(
        &self,
        file_info: &FileUpload,
        unit: &str,
        start: usize,
        end: Option<usize>,
        options: &serde_json::Map<String, Value>,
        file_type: &DetectedFileType,
    ) -> Result<String, String> {
        // Implementation would extract actual file content
        // This is a placeholder
        
        let range_description = match file_type {
            DetectedFileType::Excel => {
                format!("Extracted {} {} from row {} to {}", 
                    unit,
                    options.get("sheet").and_then(|v| v.as_str()).unwrap_or("Sheet1"),
                    start,
                    end.unwrap_or(start + 100))
            },
            DetectedFileType::Pdf => {
                format!("Extracted {} {} from page {} to {}",
                    unit,
                    file_info.original_name,
                    start,
                    end.unwrap_or(start + 1))
            },
            _ => {
                format!("Extracted {} {} from position {} to {}",
                    unit,
                    file_info.original_name,
                    start,
                    end.unwrap_or(start + 1000))
            }
        };
        
        Ok(range_description)
    }
}