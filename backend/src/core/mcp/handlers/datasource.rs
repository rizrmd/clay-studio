use super::base::McpHandlers;
use crate::core::mcp::types::*;
use crate::utils::datasource::create_connector;
use serde_json::{json, Value};
use sqlx::Row;
use chrono::Utc;
use uuid;

impl McpHandlers {
    pub async fn add_datasource(&self, args: &serde_json::Map<String, Value>) -> Result<String, JsonRpcError> {
        self.execute_db_operation("add_datasource", async {
            // Extract required parameters
            let name = args.get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("Missing required parameter: name"))?;
            
            let source_type = args.get("source_type")
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("Missing required parameter: source_type"))?;
            
            let config = args.get("config")
                .ok_or_else(|| format!("Missing required parameter: config"))?;

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

            Ok(format!(
                "✅ **Datasource Added Successfully**\n\n\
                 🔗 **ID**: `{}`\n\
                 📝 **Name**: {}\n\
                 🔧 **Type**: {}\n\n\
                 The datasource has been added to your project and is now available for querying. \
                 Your CLAUDE.md file has been updated with the new datasource information.",
                datasource_id, name, source_type
            ))
        }).await
    }

    pub async fn list_datasources(&self, _args: &serde_json::Map<String, Value>) -> Result<String, JsonRpcError> {
        self.execute_db_operation("list_datasources", async {
            let data_sources = sqlx::query(
                "SELECT id, name, source_type, created_at FROM data_sources 
                 WHERE project_id = $1 AND deleted_at IS NULL 
                 ORDER BY created_at DESC"
            )
            .bind(&self.project_id)
            .fetch_all(&self.db_pool)
            .await?;

            if data_sources.is_empty() {
                return Ok("📋 **No Datasources Found**\n\nYou haven't added any datasources to this project yet. Use the `datasource_add` tool to add your first datasource.".to_string());
            }

            let mut result = String::from("📋 **Available Datasources**\n\n");
            for ds in data_sources {
                let id: String = ds.get("id");
                let name: String = ds.get("name");
                let source_type: String = ds.get("source_type");
                let created_at: chrono::DateTime<Utc> = ds.get("created_at");
                
                result.push_str(&format!(
                    "• **{}** ({})\n  📋 ID: `{}`\n  📅 Created: {}\n\n",
                    name,
                    source_type,
                    id,
                    created_at.format("%Y-%m-%d %H:%M UTC")
                ));
            }

            Ok(result)
        }).await
    }

    pub async fn remove_datasource(&self, args: &serde_json::Map<String, Value>) -> Result<String, JsonRpcError> {
        self.execute_db_operation("remove_datasource", async {
            let datasource_id = args.get("datasource_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("Missing required parameter: datasource_id"))?;

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

            Ok(format!(
                "✅ **Datasource Removed Successfully**\n\n\
                 🗑️ **Removed**: {} ({})\n\n\
                 The datasource has been removed from your project. \
                 Your CLAUDE.md file has been updated to reflect this change.",
                name, datasource_id
            ))
        }).await
    }

    pub async fn datasource_update(&self, args: &serde_json::Map<String, Value>) -> Result<String, JsonRpcError> {
        self.execute_db_operation("datasource_update", async {
            let datasource_id = args.get("datasource_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("Missing required parameter: datasource_id"))?;

            // Check if datasource exists and belongs to this project
            let existing = sqlx::query(
                "SELECT name, source_type, connection_config FROM data_sources WHERE id = $1 AND project_id = $2 AND deleted_at IS NULL"
            )
            .bind(datasource_id)
            .bind(&self.project_id)
            .fetch_optional(&self.db_pool)
            .await?
            .ok_or_else(|| format!("Datasource not found"))?;

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

            Ok(format!(
                "✅ **Datasource Updated Successfully**\n\n\
                 🔄 **Updated**: {} ({})\n\n\
                 The datasource has been updated. Your CLAUDE.md file has been refreshed.",
                datasource_id, 
                args.get("name").and_then(|v| v.as_str()).unwrap_or("name unchanged")
            ))
        }).await
    }

    pub async fn test_connection(&self, args: &serde_json::Map<String, Value>) -> Result<String, JsonRpcError> {
        self.execute_db_operation("test_connection", async {
            // Get datasource_id parameter
            let datasource_id = args.get("datasource_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("Missing required parameter: datasource_id"))?;

            // Get connector
            let source = self.get_datasource_connector(datasource_id).await?;
            let mut connector = create_connector(&source.source_type, &source.connection_config)
                .await
                .map_err(|e| format!("Failed to create connector: {}", e))?;

            // Test connection
            match connector.test_connection().await {
                Ok(_) => {
                    Ok(format!(
                        "✅ **Connection Test Successful**\n\n\
                         🔗 **Datasource**: {} ({})\n\
                         🟢 **Status**: Connected\n\n\
                         The connection to your datasource is working correctly!",
                        source.name, datasource_id
                    ))
                }
                Err(e) => {
                    Ok(format!(
                        "❌ **Connection Test Failed**\n\n\
                         🔗 **Datasource**: {} ({})\n\
                         🔴 **Error**: {}\n\n\
                         Please check your connection configuration and try again.",
                        source.name, datasource_id, e
                    ))
                }
            }
        }).await
    }

    pub async fn get_datasource_detail(&self, args: &serde_json::Map<String, Value>) -> Result<String, JsonRpcError> {
        self.execute_db_operation("get_datasource_detail", async {
            let datasource_id = args.get("datasource_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("Missing required parameter: datasource_id"))?;

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
            .ok_or_else(|| format!("Datasource not found"))?;

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

            let updated_str = updated_at
                .map(|dt| dt.format("%Y-%m-%d %H:%M UTC").to_string())
                .unwrap_or_else(|| "Never".to_string());

            let schema_str = schema_info.unwrap_or_else(|| "Not analyzed yet".to_string());

            Ok(format!(
                "🔗 **Datasource Details**\n\n\
                 📋 **ID**: `{}`\n\
                 📝 **Name**: {}\n\
                 🔧 **Type**: {}\n\
                 📅 **Created**: {}\n\
                 🔄 **Updated**: {}\n\n\
                 **Configuration** (sensitive data hidden):\n\
                 ```json\n{}\n```\n\n\
                 **Schema Info**:\n\
                 ```\n{}\n```",
                id, name, source_type,
                created_at.format("%Y-%m-%d %H:%M UTC"),
                updated_str,
                config_display,
                schema_str
            ))
        }).await
    }

    pub async fn query_datasource(&self, args: &serde_json::Map<String, Value>) -> Result<String, JsonRpcError> {
        self.execute_db_operation("query_datasource", async {
            let datasource_id = args.get("datasource_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("Missing required parameter: datasource_id"))?;
            
            let query = args.get("query")
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("Missing required parameter: query"))?;

            // Get limit parameter (default to 100, max 1000)
            let limit = args.get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(100)
                .min(1000) as usize;

            // Get connector
            let source = self.get_datasource_connector(datasource_id).await?;
            let connector = create_connector(&source.source_type, &source.connection_config)
                .await
                .map_err(|e| format!("Failed to create connector: {}", e))?;

            // Execute query
            let result = connector.execute_query(query, limit as i32)
                .await
                .map_err(|e| format!("Query execution failed: {}", e))?;

            // Format result for display
            let result_json = serde_json::to_string_pretty(&result)?;
            Ok(format!(
                "📊 **Query Results**\n\n\
                 🔗 **Datasource**: {} ({})\n\
                 📝 **Query**: ```sql\n{}\n```\n\n\
                 **Results**:\n\
                 ```json\n{}\n```",
                source.name, datasource_id, query, result_json
            ))
        }).await
    }

    pub async fn inspect_datasource(&self, args: &serde_json::Map<String, Value>) -> Result<String, JsonRpcError> {
        self.execute_db_operation("inspect_datasource", async {
            let datasource_id = args.get("datasource_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("Missing required parameter: datasource_id"))?;

            self.inspect_datasource_internal(datasource_id).await
        }).await
    }

    pub async fn inspect_datasource_internal(&self, datasource_id: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Get connector
        let source = self.get_datasource_connector(datasource_id).await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.message)) })?;
        let connector = create_connector(&source.source_type, &source.connection_config).await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("{}", e))) })?;

        // Run inspection
        let analysis = connector.analyze_database().await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("{}", e))) })?;

        // Store schema info in database for future reference
        let schema_info = serde_json::to_string(&analysis)?;
        sqlx::query("UPDATE data_sources SET schema_info = $1, updated_at = NOW() WHERE id = $2")
            .bind(&schema_info)
            .bind(datasource_id)
            .execute(&self.db_pool)
            .await?;

        Ok(self.format_inspection_result(&source.name, &analysis))
    }
}