use super::base::DataSourceConnector;
use serde_json::{json, Value};
use sqlx::{sqlite::SqlitePool, Row as SqlxRow, Column};
use std::error::Error;
use async_trait::async_trait;

pub struct SQLiteConnector {
    connection_string: String,
}

impl SQLiteConnector {
    pub fn new(config: &Value) -> Result<Self, Box<dyn Error>> {
        // Prefer URL if provided, otherwise construct from path
        let connection_string = if let Some(url) = config.get("url").and_then(|v| v.as_str()) {
            url.to_string()
        } else {
            // Fallback to path for backward compatibility
            let path = config.get("path").and_then(|v| v.as_str()).ok_or("Missing database path")?;
            format!("sqlite://{}", path)
        };
        
        Ok(Self { connection_string })
    }
}

#[async_trait]
impl DataSourceConnector for SQLiteConnector {
    async fn test_connection(&mut self) -> Result<bool, Box<dyn Error>> {
        let pool = SqlitePool::connect(&self.connection_string).await?;
        sqlx::query("SELECT 1").fetch_one(&pool).await?;
        Ok(true)
    }
    
    async fn execute_query(&self, query: &str, limit: i32) -> Result<Value, Box<dyn Error>> {
        let pool = SqlitePool::connect(&self.connection_string).await?;
        
        let query_with_limit = if query.to_lowercase().contains("limit") {
            query.to_string()
        } else {
            format!("{} LIMIT {}", query, limit)
        };
        
        let start = std::time::Instant::now();
        let rows = sqlx::query(&query_with_limit).fetch_all(&pool).await?;
        let execution_time_ms = start.elapsed().as_millis() as i64;
        
        if rows.is_empty() {
            return Ok(json!({
                "columns": [],
                "rows": [],
                "row_count": 0,
                "execution_time_ms": execution_time_ms
            }));
        }
        
        // Get column names from the first row
        let first_row = &rows[0];
        let columns: Vec<String> = first_row.columns().iter().map(|c| c.name().to_string()).collect();
        
        // Convert rows to JSON
        let mut result_rows = Vec::new();
        for row in rows.iter() {
            let mut row_data = Vec::new();
            for (i, _col) in columns.iter().enumerate() {
                // Try to get value as different types
                if let Ok(val) = row.try_get::<String, _>(i) {
                    row_data.push(val);
                } else if let Ok(val) = row.try_get::<i32, _>(i) {
                    row_data.push(val.to_string());
                } else if let Ok(val) = row.try_get::<i64, _>(i) {
                    row_data.push(val.to_string());
                } else if let Ok(val) = row.try_get::<f64, _>(i) {
                    row_data.push(val.to_string());
                } else if let Ok(val) = row.try_get::<f32, _>(i) {
                    row_data.push(val.to_string());
                } else if let Ok(val) = row.try_get::<bool, _>(i) {
                    row_data.push(if val { "true" } else { "false" }.to_string());
                } else {
                    row_data.push("NULL".to_string());
                }
            }
            result_rows.push(row_data);
        }
        
        Ok(json!({
            "columns": columns,
            "rows": result_rows,
            "row_count": result_rows.len(),
            "execution_time_ms": execution_time_ms
        }))
    }
    
    async fn fetch_schema(&self) -> Result<Value, Box<dyn Error>> {
        let pool = SqlitePool::connect(&self.connection_string).await?;
        
        let tables = sqlx::query("SELECT name FROM sqlite_master WHERE type='table'")
            .fetch_all(&pool)
            .await?;
        
        let mut schema = json!({
            "tables": {},
            "refreshed_at": chrono::Utc::now().to_rfc3339()
        });
        
        for table_row in tables {
            let table_name: String = table_row.get("name");
            let columns = sqlx::query(&format!("PRAGMA table_info({})", table_name))
                .fetch_all(&pool)
                .await?;
            
            let mut column_info = Vec::new();
            for col in columns {
                column_info.push(json!({
                    "column_name": col.get::<String, _>("name"),
                    "data_type": col.get::<String, _>("type"),
                    "is_nullable": col.get::<i32, _>("notnull") == 0,
                }));
            }
            schema["tables"][table_name] = json!(column_info);
        }
        
        Ok(schema)
    }
    
    async fn list_tables(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let pool = SqlitePool::connect(&self.connection_string).await?;
        
        let tables = sqlx::query("SELECT name FROM sqlite_master WHERE type='table'")
            .fetch_all(&pool)
            .await?;
        
        let table_names: Vec<String> = tables.iter().map(|row| row.get("name")).collect();
        Ok(table_names)
    }
    
    async fn analyze_database(&self) -> Result<Value, Box<dyn Error>> {
        let pool = SqlitePool::connect(&self.connection_string).await?;
        
        // Get all tables
        let tables = sqlx::query("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'")
            .fetch_all(&pool)
            .await?;
        
        let table_count = tables.len() as i64;
        let mut table_names = Vec::new();
        let mut table_info = Vec::new();
        let mut key_tables = Vec::new();
        
        // Get info for each table
        for table_row in &tables {
            let table_name: String = table_row.get("name");
            table_names.push(table_name.clone());
            
            // Get row count for this table
            let count_query = format!("SELECT COUNT(*) as count FROM \"{}\"", table_name.replace('"', "\"\""));
            let row_count: i64 = sqlx::query(&count_query)
                .fetch_one(&pool)
                .await
                .map(|r| r.get("count"))
                .unwrap_or(0);
            
            // Get column count
            let col_count_query = format!("SELECT COUNT(*) as count FROM pragma_table_info('{}')", table_name.replace('\'', "''"));
            let col_count: i64 = sqlx::query(&col_count_query)
                .fetch_one(&pool)
                .await
                .map(|r| r.get("count"))
                .unwrap_or(0);
            
            // Get foreign key count to determine importance
            let fk_query = format!("SELECT COUNT(*) as count FROM pragma_foreign_key_list('{}')", table_name.replace('\'', "''"));
            let fk_count: i64 = sqlx::query(&fk_query)
                .fetch_one(&pool)
                .await
                .map(|r| r.get("count"))
                .unwrap_or(0);
            
            table_info.push(json!({
                "name": table_name,
                "row_count": row_count,
                "column_count": col_count,
                "foreign_key_count": fk_count,
            }));
            
            // Consider it a key table if it has foreign keys or many rows
            if fk_count > 0 || row_count > 1000 {
                key_tables.push(json!({
                    "name": table_name,
                    "row_count": row_count,
                    "connections": fk_count,
                }));
            }
        }
        
        // Sort tables by row count for largest_tables
        table_info.sort_by_key(|t| -t["row_count"].as_i64().unwrap_or(0));
        let largest_tables: Vec<Value> = table_info.iter()
            .take(10)
            .map(|t| json!({
                "name": t["name"],
                "row_count": t["row_count"],
                "size_human": format!("{} rows", t["row_count"]),
            }))
            .collect();
        
        // Sort key tables by importance
        key_tables.sort_by_key(|t| {
            let connections = t["connections"].as_i64().unwrap_or(0);
            let rows = t["row_count"].as_i64().unwrap_or(0);
            -(connections * 1000 + rows)
        });
        
        Ok(json!({
            "statistics": {
                "table_count": table_count,
                "total_size": 0,
                "total_size_human": "N/A",
                "total_rows": table_info.iter().map(|t| t["row_count"].as_i64().unwrap_or(0)).sum::<i64>(),
            },
            "table_names": table_names,
            "key_tables": key_tables.into_iter().take(10).collect::<Vec<_>>(),
            "largest_tables": largest_tables,
            "analyzed_at": chrono::Utc::now().to_rfc3339(),
        }))
    }
    
    async fn get_tables_schema(&self, tables: Vec<&str>) -> Result<Value, Box<dyn Error>> {
        let pool = SqlitePool::connect(&self.connection_string).await?;
        let mut result = json!({});
        
        for table_name in tables {
            // Get column information
            let columns_query = format!("PRAGMA table_info('{}')", table_name.replace('\'', "''"));
            let columns = sqlx::query(&columns_query)
                .fetch_all(&pool)
                .await?;
            
            if columns.is_empty() {
                continue; // Table doesn't exist
            }
            
            // Get foreign keys
            let fk_query = format!("PRAGMA foreign_key_list('{}')", table_name.replace('\'', "''"));
            let foreign_keys = sqlx::query(&fk_query)
                .fetch_all(&pool)
                .await?;
            
            // Get row count
            let count_query = format!("SELECT COUNT(*) as count FROM \"{}\"", table_name.replace('"', "\"\""));
            let row_count: i64 = sqlx::query(&count_query)
                .fetch_one(&pool)
                .await
                .map(|r| r.get("count"))
                .unwrap_or(0);
            
            // Get sample data
            let sample_query = format!("SELECT * FROM \"{}\" LIMIT 5", table_name.replace('"', "\"\""));
            let sample_rows = sqlx::query(&sample_query)
                .fetch_all(&pool)
                .await
                .unwrap_or_default();
            
            // Build column info
            let column_info: Vec<Value> = columns.iter().map(|col| {
                json!({
                    "name": col.get::<String, _>("name"),
                    "type": col.get::<String, _>("type"),
                    "nullable": col.get::<i32, _>("notnull") == 0,
                    "default": col.get::<Option<String>, _>("dflt_value"),
                    "primary_key": col.get::<i32, _>("pk") > 0,
                })
            }).collect();
            
            // Get primary keys
            let primary_keys: Vec<String> = columns.iter()
                .filter(|col| col.get::<i32, _>("pk") > 0)
                .map(|col| col.get::<String, _>("name"))
                .collect();
            
            // Build foreign key info
            let fk_info: Vec<Value> = foreign_keys.iter().map(|fk| {
                json!({
                    "column": fk.get::<String, _>("from"),
                    "references_table": fk.get::<String, _>("table"),
                    "references_column": fk.get::<String, _>("to"),
                })
            }).collect();
            
            // Build sample data
            let mut sample_data = Vec::new();
            for row in sample_rows {
                let mut row_data = json!({});
                for (i, col) in columns.iter().enumerate() {
                    let col_name: String = col.get("name");
                    // SQLite is more flexible with types
                    if let Ok(val) = row.try_get::<Option<String>, _>(i) {
                        row_data[&col_name] = json!(val);
                    } else if let Ok(val) = row.try_get::<Option<i64>, _>(i) {
                        row_data[&col_name] = json!(val);
                    } else if let Ok(val) = row.try_get::<Option<f64>, _>(i) {
                        row_data[&col_name] = json!(val);
                    } else {
                        row_data[&col_name] = json!(null);
                    }
                }
                sample_data.push(row_data);
            }
            
            result[table_name] = json!({
                "columns": column_info,
                "primary_keys": primary_keys,
                "foreign_keys": fk_info,
                "row_count": row_count,
                "sample_data": sample_data,
            });
        }
        
        Ok(result)
    }
    
    async fn search_tables(&self, pattern: &str) -> Result<Value, Box<dyn Error>> {
        let pool = SqlitePool::connect(&self.connection_string).await?;
        
        // Use LIKE in the query directly for SQLite
        let query = "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name LIKE ?";
        let tables = sqlx::query(query)
            .bind(pattern)
            .fetch_all(&pool)
            .await?;
        
        let mut results = Vec::new();
        for table_row in tables {
            let table_name: String = table_row.get("name");
            
            // Get column count
            let col_count_query = format!("SELECT COUNT(*) as count FROM pragma_table_info('{}')", table_name.replace('\'', "''"));
            let col_count: i64 = sqlx::query(&col_count_query)
                .fetch_one(&pool)
                .await
                .map(|r| r.get("count"))
                .unwrap_or(0);
            
            results.push(json!({
                "name": table_name,
                "description": null,
                "column_count": col_count,
            }));
        }
        
        Ok(json!({
            "matches": results,
            "total_matches": results.len(),
        }))
    }
    
    async fn get_related_tables(&self, table: &str) -> Result<Value, Box<dyn Error>> {
        let pool = SqlitePool::connect(&self.connection_string).await?;
        
        // Get the main table schema
        let main_schema = self.get_tables_schema(vec![table]).await?;
        
        if main_schema.get(table).is_none() {
            return Err("Table not found".into());
        }
        
        // Get foreign keys from this table (outgoing)
        let fk_query = format!("PRAGMA foreign_key_list('{}')", table.replace('\'', "''"));
        let foreign_keys = sqlx::query(&fk_query)
            .fetch_all(&pool)
            .await?;
        
        let mut related_table_names = Vec::new();
        for fk in foreign_keys {
            let referenced_table: String = fk.get("table");
            if !related_table_names.contains(&referenced_table) {
                related_table_names.push(referenced_table);
            }
        }
        
        // For incoming foreign keys, we need to check all tables
        let all_tables = self.list_tables().await?;
        for other_table in all_tables {
            if other_table == table {
                continue;
            }
            
            let fk_check = format!("PRAGMA foreign_key_list('{}')", other_table.replace('\'', "''"));
            let fks = sqlx::query(&fk_check)
                .fetch_all(&pool)
                .await?;
            
            for fk in fks {
                let referenced_table: String = fk.get("table");
                if referenced_table == table && !related_table_names.contains(&other_table) {
                    related_table_names.push(other_table.clone());
                }
            }
        }
        
        // Get schemas for related tables
        let related_schemas = if !related_table_names.is_empty() {
            let refs: Vec<&str> = related_table_names.iter().map(|s| s.as_str()).collect();
            self.get_tables_schema(refs).await?
        } else {
            json!({})
        };
        
        Ok(json!({
            "main_table": main_schema[table],
            "related_tables": related_schemas,
            "relationship_count": related_table_names.len(),
        }))
    }
    
    async fn get_database_stats(&self) -> Result<Value, Box<dyn Error>> {
        let pool = SqlitePool::connect(&self.connection_string).await?;
        
        // Get all tables
        let tables = sqlx::query("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'")
            .fetch_all(&pool)
            .await?;
        
        let mut table_stats = Vec::new();
        let mut total_rows = 0i64;
        
        for table_row in &tables {
            let table_name: String = table_row.get("name");
            
            // Get row count
            let count_query = format!("SELECT COUNT(*) as count FROM \"{}\"", table_name.replace('"', "\"\""));
            let row_count: i64 = sqlx::query(&count_query)
                .fetch_one(&pool)
                .await
                .map(|r| r.get("count"))
                .unwrap_or(0);
            
            total_rows += row_count;
            
            // Get foreign key count
            let fk_query = format!("SELECT COUNT(*) as count FROM pragma_foreign_key_list('{}')", table_name.replace('\'', "''"));
            let fk_count: i64 = sqlx::query(&fk_query)
                .fetch_one(&pool)
                .await
                .map(|r| r.get("count"))
                .unwrap_or(0);
            
            table_stats.push(json!({
                "name": table_name,
                "row_count": row_count,
                "foreign_keys": fk_count,
            }));
        }
        
        // Sort by row count for largest tables
        table_stats.sort_by_key(|t| -t["row_count"].as_i64().unwrap_or(0));
        let largest_tables: Vec<Value> = table_stats.iter()
            .take(10)
            .map(|t| json!({
                "name": t["name"],
                "row_count": t["row_count"],
                "size_human": format!("{} rows", t["row_count"]),
            }))
            .collect();
        
        // Sort by foreign key count for most connected
        table_stats.sort_by_key(|t| -t["foreign_keys"].as_i64().unwrap_or(0));
        let most_connected: Vec<Value> = table_stats.iter()
            .filter(|t| t["foreign_keys"].as_i64().unwrap_or(0) > 0)
            .take(10)
            .map(|t| json!({
                "name": t["name"],
                "references_count": t["foreign_keys"],
                "referenced_by_count": 0, // Would need to scan all tables
                "total_connections": t["foreign_keys"],
            }))
            .collect();
        
        Ok(json!({
            "summary": {
                "total_tables": tables.len(),
                "total_rows": total_rows,
                "total_size_bytes": 0,
                "total_size_human": "N/A",
            },
            "largest_tables": largest_tables,
            "most_connected_tables": most_connected,
        }))
    }
}