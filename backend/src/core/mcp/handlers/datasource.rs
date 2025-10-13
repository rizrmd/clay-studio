use super::base::McpHandlers;
use crate::core::datasources::shared_service;
use crate::core::mcp::types::*;
use crate::utils::datasource::create_connector;
use chrono::Utc;
use serde_json::{json, Value};
use sqlx::Row;
use uuid;

impl McpHandlers {
    #[allow(dead_code)]
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

            // Test the connection directly before adding (no ID required)
            if let Err(e) = shared_service::test_datasource_connection_direct(source_type, &parsed_config).await {
                return Err(format!("Connection test failed: {}", e).into());
            }

            // Verify that client and project exist before inserting datasource
            self.verify_client_and_project_exist().await?;

            // Generate UUID for the new datasource
            let datasource_id = uuid::Uuid::new_v4().to_string();

            // Add the generated ID to the config for future connection pooling support
            let mut config_with_id = parsed_config.clone();
            if let Some(obj) = config_with_id.as_object_mut() {
                obj.insert("id".to_string(), Value::String(datasource_id.clone()));
            }

            // Insert into database with config that includes the ID
            sqlx::query(
                "INSERT INTO data_sources (id, project_id, name, source_type, connection_config, created_at) 
                 VALUES ($1, $2, $3, $4, $5, NOW())"
            )
            .bind(&datasource_id)
            .bind(&self.project_id)
            .bind(name)
            .bind(source_type)
            .bind(&config_with_id)
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

    #[allow(dead_code)]
    pub async fn list_datasources(
        &self,
        _args: &serde_json::Map<String, Value>,
    ) -> Result<String, JsonRpcError> {
        self.execute_db_operation("list_datasources", async {
            let datasources = shared_service::list_datasources_for_project(
                &self.project_id, 
                &self.db_pool
            ).await.map_err(|e| format!("Failed to list datasources: {}", e))?;

            let count = datasources.len();
            let response_data = json!({
                "datasources": datasources.into_iter().map(|ds| json!({
                    "id": ds.id,
                    "name": ds.name,
                    "source_type": ds.source_type,
                    "created_at": ds.created_at.map(|dt| dt.to_rfc3339()).unwrap_or_else(|| chrono::Utc::now().to_rfc3339())
                })).collect::<Vec<Value>>(),
                "count": count
            });
            Ok(serde_json::to_string(&response_data)?)
        }).await
    }

    #[allow(dead_code)]
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

    #[allow(dead_code)]
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
            let mut config_update: Option<Value> = None;

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
                
                config_update = Some(parsed_config);
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

    #[allow(dead_code)]
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

            // Get datasource info using shared service (with caching)
            let datasource = shared_service::get_datasource_with_validation(
                datasource_id,
                &self.project_id,
                &self.db_pool
            ).await.map_err(|e| format!("Failed to get datasource: {}", e))?;
            
            // Add datasource ID to config for pooling support
            let mut config_with_id = datasource.connection_config.clone();
            if let Some(config_obj) = config_with_id.as_object_mut() {
                config_obj.insert("id".to_string(), Value::String(datasource_id.to_string()));
            }
            
            // Create connector using the same mechanism for consistency
            let mut connector = create_connector(&datasource.source_type, &config_with_id)
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
                            "name": datasource.name
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
                            "name": datasource.name
                        },
                        "error": e.to_string()
                    });
                    Ok(serde_json::to_string(&response_data)?)
                },
            }
        })
        .await
    }

    #[allow(dead_code)]
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

            // Parse full connection details including sensitive information
            let connection_details = match serde_json::from_str::<Value>(&connection_config) {
                Ok(config) => config,
                Err(_) => json!("Invalid connection configuration")
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
                    "connection_details": connection_details,
                    "schema_info": schema_json
                },
                "metadata": {
                    "schema_analyzed": schema_info.is_some()
                }
            });
            Ok(serde_json::to_string(&response_data)?)
        }).await
    }

    #[allow(dead_code)]
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

            // Note: limit parameter is not used when pooling as the pooling mechanism handles limits internally

            // Get datasource info first for the response
            let datasource = shared_service::get_datasource_with_validation(
                datasource_id,
                &self.project_id,
                &self.db_pool
            ).await.map_err(|e| format!("Failed to get datasource: {}", e))?;

            // Execute query using shared service with connection pooling
            let result = shared_service::execute_query_on_datasource(
                datasource_id,
                &self.project_id,
                query,
                &self.db_pool
            ).await.map_err(|e| format!("Query execution failed: {}", e))?;

            // Return JSON result with metadata
            let response_data = json!({
                "datasource": {
                    "id": datasource_id,
                    "name": datasource.name
                },
                "query": query,
                "execution_time_ms": result.get("execution_time_ms"),
                "columns": result.get("columns"),
                "rows": result.get("rows"),
                "row_count": result.get("row_count"),
                "using_connection_pool": true
            });
            Ok(serde_json::to_string(&response_data)?)
        })
        .await
    }

    #[allow(dead_code)]
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

    #[allow(dead_code)]
    pub async fn inspect_datasource_internal(
        &self,
        datasource_id: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Get datasource info using shared service (with caching)
        let datasource = shared_service::get_datasource_with_validation(
            datasource_id,
            &self.project_id,
            &self.db_pool
        ).await.map_err(|e| format!("Failed to get datasource: {}", e))?;
        
        // Add datasource ID to config for pooling support
        let mut config_with_id = datasource.connection_config.clone();
        if let Some(config_obj) = config_with_id.as_object_mut() {
            config_obj.insert("id".to_string(), Value::String(datasource_id.to_string()));
        }
        
        // Create connector using the same mechanism as datasource_query
        // This ensures we use pooling where available
        let connector = create_connector(&datasource.source_type, &config_with_id)
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                Box::new(std::io::Error::other(format!("Failed to create connector: {}", e)))
            })?;

        // Run inspection
        let analysis = connector.analyze_database().await.map_err(
            |e| -> Box<dyn std::error::Error + Send + Sync> {
                Box::new(std::io::Error::other(format!("Database analysis failed: {}", e)))
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
                "name": datasource.name
            },
            "analysis": analysis,
            "message": "Database inspection completed successfully",
            "metadata": {
                "schema_cached": true,
                "using_connection_pool": true
            }
        });
        Ok(serde_json::to_string(&response_data)?)
    }
}
