use super::base::McpHandlers;
use crate::core::mcp::types::*;
use crate::utils::datasource::create_connector;
use chrono::Utc;
use serde_json::{json, Value};
use sqlx::Row;
use uuid;

impl McpHandlers {
    pub async fn add_datasource(
        &self,
        args: &serde_json::Map<String, Value>,
    ) -> Result<String, JsonRpcError> {
        self.execute_db_operation("add_datasource", async {
            // Extract required parameters
            let name = args.get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter: name".to_string())?;
            
            let source_type = args.get("source_type")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter: source_type".to_string())?;
            
            let config = args.get("config")
                .ok_or_else(|| "Missing required parameter: config".to_string())?;

            // Parse and validate the connection config
            let parsed_config = self.parse_connection_config(config, source_type)?;

            // Test the connection before adding
            let mut connector = create_connector(source_type, &parsed_config).await
                .map_err(|e| format!("Failed to create connector: {}", e))?;
            if let Err(e) = connector.test_connection().await {
                return Err(format!("Connection test failed: {}", e).into());
            }

            // Generate UUID for the new datasource
            let datasource_id = uuid::Uuid::new_v4().to_string();

            // Insert into database
            sqlx::query(
                "INSERT INTO data_sources (id, project_id, name, source_type, connection_config, created_at) 
                 VALUES ($1, $2, $3, $4, $5, NOW())"
            )
            .bind(&datasource_id)
            .bind(&self.project_id)
            .bind(name)
            .bind(source_type)
            .bind(serde_json::to_string(&parsed_config)?)
            .execute(&self.db_pool)
            .await?;

            // Refresh CLAUDE.md in the background
            let refresh_self = self.clone();
            tokio::spawn(async move {
                if let Err(e) = refresh_self.refresh_claude_md().await {
                    eprintln!(
                        "[{}] [WARNING] Failed to refresh CLAUDE.md after adding datasource: {}", 
                        Utc::now().format("%Y-%m-%d %H:%M:%S UTC"), 
                        e
                    );
                }
            });

            let response_data = json!({
                "status": "success",
                "datasource": {
                    "id": datasource_id,
                    "name": name,
                    "type": source_type
                },
                "message": "Datasource added successfully"
            });
            Ok(serde_json::to_string(&response_data)?)
        }).await
    }

    pub async fn list_datasources(
        &self,
        _args: &serde_json::Map<String, Value>,
    ) -> Result<String, JsonRpcError> {
        self.execute_db_operation("list_datasources", async {
            let data_sources = sqlx::query(
                "SELECT id, name, source_type, created_at FROM data_sources 
                 WHERE project_id = $1 AND deleted_at IS NULL 
                 ORDER BY created_at DESC"
            )
            .bind(&self.project_id)
            .fetch_all(&self.db_pool)
            .await?;

            let datasources: Vec<Value> = data_sources.iter().map(|ds| {
                let id: String = ds.get("id");
                let name: String = ds.get("name");
                let source_type: String = ds.get("source_type");
                let created_at: chrono::DateTime<Utc> = ds.get("created_at");
                
                json!({
                    "id": id,
                    "name": name,
                    "source_type": source_type,
                    "created_at": created_at.to_rfc3339()
                })
            }).collect();

            let response_data = json!({
                "datasources": datasources,
                "count": datasources.len()
            });
            Ok(serde_json::to_string(&response_data)?)
        }).await
    }

    pub async fn remove_datasource(
        &self,
        args: &serde_json::Map<String, Value>,
    ) -> Result<String, JsonRpcError> {
        self.execute_db_operation("remove_datasource", async {
            let datasource_id = args.get("datasource_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter: datasource_id".to_string())?;

            // Check if datasource exists and belongs to this project
            let existing_count = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM data_sources WHERE id = $1 AND project_id = $2 AND deleted_at IS NULL"
            )
            .bind(datasource_id)
            .bind(&self.project_id)
            .fetch_one(&self.db_pool)
            .await?;

            if existing_count == 0 {
                return Err("Datasource not found or already deleted".into());
            }

            // Get the datasource name before deleting
            let name: String = sqlx::query_scalar(
                "SELECT name FROM data_sources WHERE id = $1"
            )
            .bind(datasource_id)
            .fetch_one(&self.db_pool)
            .await?;

            // Soft delete the datasource
            sqlx::query(
                "UPDATE data_sources SET deleted_at = NOW() WHERE id = $1 AND project_id = $2"
            )
            .bind(datasource_id)
            .bind(&self.project_id)
            .execute(&self.db_pool)
            .await?;

            // Refresh CLAUDE.md in the background
            let refresh_self = self.clone();
            tokio::spawn(async move {
                if let Err(e) = refresh_self.refresh_claude_md().await {
                    eprintln!(
                        "[{}] [WARNING] Failed to refresh CLAUDE.md after removing datasource: {}", 
                        Utc::now().format("%Y-%m-%d %H:%M:%S UTC"), 
                        e
                    );
                }
            });

            let response_data = json!({
                "status": "success",
                "datasource": {
                    "id": datasource_id,
                    "name": name
                },
                "message": "Datasource removed successfully",
                "metadata": {
                    "claude_md_updated": true
                }
            });
            Ok(serde_json::to_string(&response_data)?)
        }).await
    }

    pub async fn datasource_update(
        &self,
        args: &serde_json::Map<String, Value>,
    ) -> Result<String, JsonRpcError> {
        self.execute_db_operation("datasource_update", async {
            let datasource_id = args.get("datasource_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter: datasource_id".to_string())?;

            // Check if datasource exists and belongs to this project
            let existing = sqlx::query(
                "SELECT name, source_type, connection_config FROM data_sources WHERE id = $1 AND project_id = $2 AND deleted_at IS NULL"
            )
            .bind(datasource_id)
            .bind(&self.project_id)
            .fetch_optional(&self.db_pool)
            .await?
            .ok_or_else(|| "Datasource not found".to_string())?;

            let mut name_update: Option<String> = None;
            let mut config_update: Option<String> = None;

            // Handle updates
            if let Some(name) = args.get("name").and_then(|v| v.as_str()) {
                name_update = Some(name.to_string());
            }

            if let Some(config) = args.get("config") {
                let source_type: String = existing.get("source_type");
                let parsed_config = self.parse_connection_config(config, &source_type)?;
                
                // Test the connection before updating
                let mut connector = create_connector(&source_type, &parsed_config).await
                    .map_err(|e| format!("Failed to create connector: {}", e))?;
                if let Err(e) = connector.test_connection().await {
                    return Err(format!("Connection test failed: {}", e).into());
                }
                
                config_update = Some(serde_json::to_string(&parsed_config)?);
            }

            if name_update.is_none() && config_update.is_none() {
                return Err("No valid update fields provided".into());
            }

            // Capture the information before moving the values
            let has_name_update = name_update.is_some();
            let has_config_update = config_update.is_some();

            // Perform the update
            match (name_update, config_update) {
                (Some(name), None) => {
                    sqlx::query("UPDATE data_sources SET name = $1, updated_at = NOW() WHERE id = $2 AND project_id = $3")
                        .bind(name)
                        .bind(datasource_id)
                        .bind(&self.project_id)
                        .execute(&self.db_pool)
                        .await?;
                }
                (None, Some(config)) => {
                    sqlx::query("UPDATE data_sources SET connection_config = $1, updated_at = NOW() WHERE id = $2 AND project_id = $3")
                        .bind(config)
                        .bind(datasource_id)
                        .bind(&self.project_id)
                        .execute(&self.db_pool)
                        .await?;
                }
                (Some(name), Some(config)) => {
                    sqlx::query("UPDATE data_sources SET name = $1, connection_config = $2, updated_at = NOW() WHERE id = $3 AND project_id = $4")
                        .bind(name)
                        .bind(config)
                        .bind(datasource_id)
                        .bind(&self.project_id)
                        .execute(&self.db_pool)
                        .await?;
                }
                (None, None) => unreachable!(),
            }

            // Refresh CLAUDE.md in the background
            let refresh_self = self.clone();
            tokio::spawn(async move {
                if let Err(e) = refresh_self.refresh_claude_md().await {
                    eprintln!(
                        "[{}] [WARNING] Failed to refresh CLAUDE.md after updating datasource: {}", 
                        Utc::now().format("%Y-%m-%d %H:%M:%S UTC"), 
                        e
                    );
                }
            });

            let response_data = json!({
                "status": "success",
                "datasource": {
                    "id": datasource_id,
                    "name": args.get("name").and_then(|v| v.as_str()).unwrap_or("name unchanged")
                },
                "message": "Datasource updated successfully",
                "metadata": {
                    "claude_md_updated": true,
                    "updated_fields": {
                        "name": has_name_update,
                        "config": has_config_update
                    }
                }
            });
            Ok(serde_json::to_string(&response_data)?)
        }).await
    }

    pub async fn test_connection(
        &self,
        args: &serde_json::Map<String, Value>,
    ) -> Result<String, JsonRpcError> {
        self.execute_db_operation("test_connection", async {
            // Get datasource_id parameter
            let datasource_id = args
                .get("datasource_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter: datasource_id".to_string())?;

            // Get connector
            let source = self.get_datasource_connector(datasource_id).await?;
            let mut connector = create_connector(&source.source_type, &source.connection_config)
                .await
                .map_err(|e| format!("Failed to create connector: {}", e))?;

            // Test connection
            match connector.test_connection().await {
                Ok(_) => {
                    let response_data = json!({
                        "status": "success",
                        "connected": true,
                        "datasource": {
                            "id": datasource_id,
                            "name": source.name
                        },
                        "message": "Connection test successful"
                    });
                    Ok(serde_json::to_string(&response_data)?)
                },
                Err(e) => {
                    let response_data = json!({
                        "status": "error",
                        "connected": false,
                        "datasource": {
                            "id": datasource_id,
                            "name": source.name
                        },
                        "error": e.to_string()
                    });
                    Ok(serde_json::to_string(&response_data)?)
                },
            }
        })
        .await
    }

    pub async fn get_datasource_detail(
        &self,
        args: &serde_json::Map<String, Value>,
    ) -> Result<String, JsonRpcError> {
        self.execute_db_operation("get_datasource_detail", async {
            let datasource_id = args.get("datasource_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter: datasource_id".to_string())?;

            // Get datasource details
            let source = sqlx::query(
                "SELECT id, name, source_type, connection_config, created_at, updated_at, schema_info 
                 FROM data_sources 
                 WHERE id = $1 AND project_id = $2 AND deleted_at IS NULL"
            )
            .bind(datasource_id)
            .bind(&self.project_id)
            .fetch_optional(&self.db_pool)
            .await?
            .ok_or_else(|| "Datasource not found".to_string())?;

            let id: String = source.get("id");
            let name: String = source.get("name");
            let source_type: String = source.get("source_type");
            let connection_config: String = source.get("connection_config");
            let created_at: chrono::DateTime<chrono::Utc> = source.get("created_at");
            let updated_at: Option<chrono::DateTime<chrono::Utc>> = source.get("updated_at");
            let schema_info: Option<String> = source.get("schema_info");

            // Parse connection config for display (hide sensitive data)
            let config_display = match serde_json::from_str::<Value>(&connection_config) {
                Ok(config) => {
                    let mut display_config = config.clone();
                    // Hide sensitive fields
                    if let Some(obj) = display_config.as_object_mut() {
                        for sensitive_field in &["password", "token", "secret", "key"] {
                            if obj.contains_key(*sensitive_field) {
                                obj.insert(sensitive_field.to_string(), json!("***"));
                            }
                        }
                    }
                    serde_json::to_string_pretty(&display_config).unwrap_or_default()
                }
                Err(_) => "Invalid JSON configuration".to_string(),
            };

            // Parse the config_display back to JSON for structured response
            let config_json = match serde_json::from_str::<Value>(&config_display) {
                Ok(config) => config,
                Err(_) => json!("Invalid JSON configuration")
            };

            // Parse schema_info if it's JSON, otherwise keep as string
            let schema_json = match schema_info.as_ref() {
                Some(schema_str) => {
                    match serde_json::from_str::<Value>(schema_str) {
                        Ok(schema) => schema,
                        Err(_) => json!(schema_str)
                    }
                },
                None => json!(null)
            };

            let response_data = json!({
                "datasource": {
                    "id": id,
                    "name": name,
                    "source_type": source_type,
                    "created_at": created_at.to_rfc3339(),
                    "updated_at": updated_at.map(|dt| dt.to_rfc3339()),
                    "configuration": config_json,
                    "schema_info": schema_json
                },
                "metadata": {
                    "sensitive_data_hidden": true,
                    "schema_analyzed": schema_info.is_some()
                }
            });
            Ok(serde_json::to_string(&response_data)?)
        }).await
    }

    pub async fn query_datasource(
        &self,
        args: &serde_json::Map<String, Value>,
    ) -> Result<String, JsonRpcError> {
        self.execute_db_operation("query_datasource", async {
            let datasource_id = args
                .get("datasource_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter: datasource_id".to_string())?;

            let query = args
                .get("query")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter: query".to_string())?;

            // Get limit parameter (default to 100, max 1000)
            let limit = args
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(100)
                .min(1000) as usize;

            // Get connector
            let source = self.get_datasource_connector(datasource_id).await?;
            let connector = create_connector(&source.source_type, &source.connection_config)
                .await
                .map_err(|e| format!("Failed to create connector: {}", e))?;

            // Execute query
            let result = connector
                .execute_query(query, limit as i32)
                .await
                .map_err(|e| format!("Query execution failed: {}", e))?;

            // Return JSON result with metadata
            let response_data = json!({
                "datasource": {
                    "id": datasource_id,
                    "name": source.name
                },
                "query": query,
                "execution_time_ms": result.get("execution_time_ms"),
                "columns": result.get("columns"),
                "rows": result.get("rows"),
                "row_count": result.get("row_count")
            });
            Ok(serde_json::to_string(&response_data)?)
        })
        .await
    }

    pub async fn inspect_datasource(
        &self,
        args: &serde_json::Map<String, Value>,
    ) -> Result<String, JsonRpcError> {
        self.execute_db_operation("inspect_datasource", async {
            let datasource_id = args
                .get("datasource_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing required parameter: datasource_id".to_string())?;

            self.inspect_datasource_internal(datasource_id).await
        })
        .await
    }

    pub async fn inspect_datasource_internal(
        &self,
        datasource_id: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Get connector
        let source = self.get_datasource_connector(datasource_id).await.map_err(
            |e| -> Box<dyn std::error::Error + Send + Sync> {
                Box::new(std::io::Error::other(e.message))
            },
        )?;
        let connector = create_connector(&source.source_type, &source.connection_config)
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                Box::new(std::io::Error::other(
                    format!("{}", e),
                ))
            })?;

        // Run inspection
        let analysis = connector.analyze_database().await.map_err(
            |e| -> Box<dyn std::error::Error + Send + Sync> {
                Box::new(std::io::Error::other(
                    format!("{}", e),
                ))
            },
        )?;

        // Store schema info in database for future reference
        let schema_info = serde_json::to_string(&analysis)?;
        sqlx::query("UPDATE data_sources SET schema_info = $1, updated_at = NOW() WHERE id = $2")
            .bind(&schema_info)
            .bind(datasource_id)
            .execute(&self.db_pool)
            .await?;

        // Return JSON response instead of formatted text
        let response_data = json!({
            "datasource": {
                "id": datasource_id,
                "name": source.name
            },
            "analysis": analysis,
            "message": "Database inspection completed successfully",
            "metadata": {
                "schema_cached": true
            }
        });
        Ok(serde_json::to_string(&response_data)?)
    }
}
