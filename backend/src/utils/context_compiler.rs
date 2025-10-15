use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use regex::Regex;
use serde_json::Value as JsonValue;
use sqlx::{Column, PgPool, Row};

pub struct ContextCompiler {
    db_pool: PgPool,
}

impl ContextCompiler {
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }

    /// Get compiled context for a project, using cache if valid
    pub async fn get_compiled_context(&self, project_id: &str) -> Result<String> {
        // Fetch project context data
        let project = sqlx::query!(
            r#"
            SELECT 
                context,
                context_compiled,
                context_compiled_at
            FROM projects 
            WHERE id = $1
            "#,
            project_id
        )
        .fetch_optional(&self.db_pool)
        .await
        .context("Failed to fetch project")?;

        let project = match project {
            Some(p) => p,
            None => return Ok(String::new()),
        };

        // No context defined
        let context_source = match project.context {
            Some(ctx) if !ctx.is_empty() => ctx,
            _ => return Ok(String::new()),
        };

        // Check cache validity (5 minutes)
        if let (Some(compiled), Some(compiled_at)) = (project.context_compiled, project.context_compiled_at) {
            // Convert time::OffsetDateTime to chrono::DateTime
            let compiled_at_chrono = DateTime::<Utc>::from_timestamp(
                compiled_at.unix_timestamp(),
                compiled_at.nanosecond()
            ).unwrap_or_else(Utc::now);
            
            let cache_age = Utc::now() - compiled_at_chrono;
            if cache_age < Duration::minutes(5) {
                tracing::debug!("Using cached context for project {} (age: {} seconds)", project_id, cache_age.num_seconds());
                return Ok(compiled);
            }
        }

        // Compile and cache
        tracing::info!("Compiling context for project {}", project_id);
        self.compile_and_cache(project_id, &context_source).await
    }

    /// Force compile context (ignoring cache)
    pub async fn compile_context(&self, project_id: &str) -> Result<String> {
        let context_source = sqlx::query_scalar!(
            "SELECT context FROM projects WHERE id = $1",
            project_id
        )
        .fetch_optional(&self.db_pool)
        .await?
        .flatten()
        .unwrap_or_default();

        if context_source.is_empty() {
            return Ok(String::new());
        }

        self.compile_and_cache(project_id, &context_source).await
    }

    /// Compile context and update cache
    async fn compile_and_cache(&self, project_id: &str, source: &str) -> Result<String> {
        let compiled = self.compile_markdown_with_js(source, project_id).await?;
        
        // Update cache in database
        sqlx::query!(
            r#"
            UPDATE projects 
            SET 
                context_compiled = $1,
                context_compiled_at = NOW()
            WHERE id = $2
            "#,
            &compiled,
            project_id
        )
        .execute(&self.db_pool)
        .await
        .context("Failed to update compiled context cache")?;

        Ok(compiled)
    }

    /// Compile markdown with embedded JavaScript blocks
    async fn compile_markdown_with_js(&self, content: &str, project_id: &str) -> Result<String> {
        let mut compiled = content.to_string();
        
        // Find all ```javascript blocks
        let js_block_regex = Regex::new(r"(?ms)```javascript\n(.*?)\n```")
            .expect("Invalid regex pattern for JavaScript blocks");
        
        let mut replacements = Vec::new();
        
        for cap in js_block_regex.captures_iter(content) {
            let full_match = cap.get(0).expect("Regex capture should have full match");
            let js_code = &cap[1];
            
            // Execute the JavaScript code
            match self.execute_context_script(js_code, project_id).await {
                Ok(result) => {
                    replacements.push((full_match.as_str().to_string(), result));
                }
                Err(e) => {
                    let error_msg = format!("<!-- Error executing code: {} -->\n", e);
                    replacements.push((full_match.as_str().to_string(), error_msg));
                    tracing::error!("Error executing context script: {}", e);
                }
            }
        }
        
        // Apply all replacements
        for (from, to) in replacements {
            compiled = compiled.replace(&from, &to);
        }
        
        Ok(compiled)
    }

    /// Execute a JavaScript-like context script
    async fn execute_context_script(&self, script: &str, project_id: &str) -> Result<String> {
        // For now, we'll implement a simple query executor
        // This parses basic ctx.query() calls and executes them
        
        // Extract SQL query from await ctx.query(`...`)
        let query_regex = Regex::new(r"(?s)await\s+ctx\.query\s*\(\s*`([^`]+)`\s*\)")
            .expect("Invalid regex pattern for query extraction");
        
        if let Some(cap) = query_regex.captures(script) {
            let sql = &cap[1];
            
            // Execute the query
            let result = self.execute_sql_query(sql, project_id).await?;
            
            // Check if script has custom formatting logic
            if script.contains("return") && (script.contains("join") || script.contains("push")) {
                // Script has formatting logic, try to apply basic formatting
                return self.format_with_script_logic(script, result).await;
            }
            
            // Default: format as markdown table
            return Ok(self.format_as_markdown_table(result));
        }
        
        // Handle simple return statements
        if script.trim().starts_with("return ") {
            let return_content = script.trim().strip_prefix("return ").unwrap_or("");
            // Remove quotes if it's a simple string
            let cleaned = return_content.trim().trim_matches('"').trim_matches('\'');
            return Ok(cleaned.to_string());
        }
        
        Ok("<!-- Unable to process script -->\n".to_string())
    }

    /// Execute SQL query and return results
    async fn execute_sql_query(&self, sql: &str, _project_id: &str) -> Result<Vec<serde_json::Map<String, JsonValue>>> {
        // Safety check - only allow SELECT queries
        let normalized = sql.trim().to_uppercase();
        if !normalized.starts_with("SELECT") && !normalized.starts_with("WITH") {
            return Err(anyhow::anyhow!("Only SELECT queries are allowed in context"));
        }

        let rows = sqlx::query(sql)
            .fetch_all(&self.db_pool)
            .await
            .context("Failed to execute context query")?;

        let mut results = Vec::new();
        for row in rows {
            let mut row_obj = serde_json::Map::new();
            
            for (i, column) in row.columns().iter().enumerate() {
                let name = column.name();
                let value = self.extract_column_value(&row, i);
                row_obj.insert(name.to_string(), value);
            }
            
            results.push(row_obj);
        }

        Ok(results)
    }

    /// Extract value from a database row column
    fn extract_column_value(&self, row: &sqlx::postgres::PgRow, index: usize) -> JsonValue {
        use sqlx::Column;
        use sqlx::TypeInfo;
        
        let column = &row.columns()[index];
        let type_info = column.type_info();
        let type_name = type_info.name();

        // Try different types based on PostgreSQL type
        match type_name {
            "TEXT" | "VARCHAR" | "CHAR" | "BPCHAR" | "NAME" => {
                row.try_get::<Option<String>, _>(index)
                    .ok()
                    .flatten()
                    .map(JsonValue::String)
                    .unwrap_or(JsonValue::Null)
            }
            "INT2" | "INT4" | "INT8" => {
                row.try_get::<Option<i64>, _>(index)
                    .ok()
                    .flatten()
                    .map(|v| JsonValue::Number(v.into()))
                    .unwrap_or(JsonValue::Null)
            }
            "FLOAT4" | "FLOAT8" | "NUMERIC" => {
                row.try_get::<Option<f64>, _>(index)
                    .ok()
                    .flatten()
                    .and_then(serde_json::Number::from_f64)
                    .map(JsonValue::Number)
                    .unwrap_or(JsonValue::Null)
            }
            "BOOL" => {
                row.try_get::<Option<bool>, _>(index)
                    .ok()
                    .flatten()
                    .map(JsonValue::Bool)
                    .unwrap_or(JsonValue::Null)
            }
            "TIMESTAMP" | "TIMESTAMPTZ" => {
                row.try_get::<Option<DateTime<Utc>>, _>(index)
                    .ok()
                    .flatten()
                    .map(|v| JsonValue::String(v.to_rfc3339()))
                    .unwrap_or(JsonValue::Null)
            }
            "JSON" | "JSONB" => {
                row.try_get::<Option<JsonValue>, _>(index)
                    .ok()
                    .flatten()
                    .unwrap_or(JsonValue::Null)
            }
            _ => {
                // Try as string for unknown types
                row.try_get::<Option<String>, _>(index)
                    .ok()
                    .flatten()
                    .map(JsonValue::String)
                    .unwrap_or(JsonValue::Null)
            }
        }
    }

    /// Format query results as markdown table
    fn format_as_markdown_table(&self, rows: Vec<serde_json::Map<String, JsonValue>>) -> String {
        if rows.is_empty() {
            return "No data available.".to_string();
        }

        // Get column names from first row
        let columns: Vec<String> = rows[0].keys().cloned().collect();
        
        let mut table = vec![
            format!("| {} |", columns.join(" | ")),
            format!("| {} |", columns.iter().map(|_| "---").collect::<Vec<_>>().join(" | "))
        ];

        // Add data rows
        for row in &rows {
            let values: Vec<String> = columns.iter().map(|col| {
                match row.get(col) {
                    Some(JsonValue::String(s)) => s.clone(),
                    Some(JsonValue::Number(n)) => n.to_string(),
                    Some(JsonValue::Bool(b)) => b.to_string(),
                    Some(JsonValue::Null) | None => "NULL".to_string(),
                    Some(v) => v.to_string(),
                }
            }).collect();
            
            table.push(format!("| {} |", values.join(" | ")));
        }

        table.join("\n")
    }

    /// Try to apply basic formatting from script logic
    async fn format_with_script_logic(&self, script: &str, rows: Vec<serde_json::Map<String, JsonValue>>) -> Result<String> {
        // This is a simplified interpreter for basic formatting patterns
        // In production, you'd use a proper JavaScript engine
        
        // Check for table formatting pattern
        if script.contains("table.push") || script.contains("markdown.push") {
            // Look for header row pattern
            if script.contains("| Schema | Table |") || script.contains("| Table | Size |") {
                return Ok(self.format_as_markdown_table(rows));
            }
            
            // Look for list pattern
            if script.contains("summary.push") || script.contains("metrics.push") {
                let mut lines = Vec::new();
                for row in rows {
                    // Format as a list item with first few fields
                    let parts: Vec<String> = row.iter()
                        .take(3)
                        .map(|(k, v)| format!("{}: {}", k, self.json_value_to_string(v)))
                        .collect();
                    lines.push(format!("- {}", parts.join(", ")));
                }
                return Ok(lines.join("\n"));
            }
        }
        
        // Default to table format
        Ok(self.format_as_markdown_table(rows))
    }

    /// Convert JSON value to string
    fn json_value_to_string(&self, value: &JsonValue) -> String {
        match value {
            JsonValue::String(s) => s.clone(),
            JsonValue::Number(n) => n.to_string(),
            JsonValue::Bool(b) => b.to_string(),
            JsonValue::Null => "NULL".to_string(),
            v => v.to_string(),
        }
    }
}

/// Update a project's context source
pub async fn update_project_context(
    db_pool: &PgPool,
    project_id: &str,
    context: &str,
) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE projects 
        SET 
            context = $1,
            context_compiled = NULL,
            context_compiled_at = NULL,
            updated_at = NOW()
        WHERE id = $2
        "#,
        context,
        project_id
    )
    .execute(db_pool)
    .await
    .context("Failed to update project context")?;

    Ok(())
}