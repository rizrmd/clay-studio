use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgPool;
use tokio::io::{AsyncWriteExt, BufReader};
use tokio::process::ChildStdin;

use crate::core::mcp::handlers::base::McpHandlers;

/// RPC request from Bun to Rust MCP handlers
#[derive(Debug, Deserialize)]
pub struct RpcRequest {
    pub id: String,
    pub method: String,
    pub params: Value,
}

/// RPC response from Rust to Bun
#[derive(Debug, Serialize)]
pub struct RpcResponse {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl RpcResponse {
    pub fn success(id: String, result: Value) -> Self {
        Self {
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: String, error: String) -> Self {
        Self {
            id,
            result: None,
            error: Some(error),
        }
    }
}

/// MCP Bridge handles RPC calls from Bun to MCP handlers
pub struct McpBridge {
    mcp_handlers: McpHandlers,
}

impl McpBridge {
    pub fn new(project_id: String, job_id: String, db_pool: PgPool) -> Self {
        Self {
            mcp_handlers: McpHandlers {
                project_id,
                client_id: job_id,
                server_type: "analysis".to_string(),
                db_pool,
            },
        }
    }

    /// Handle a single RPC request
    pub async fn handle_request(&self, request: RpcRequest) -> RpcResponse {
        let result = match request.method.as_str() {
            // File operations
            "files.list" => self.handle_file_list(request.params).await,
            "files.read" => self.handle_file_read(request.params).await,
            "files.search" => self.handle_file_search(request.params).await,
            "files.metadata" => self.handle_file_metadata(request.params).await,
            "files.peek" => self.handle_file_peek(request.params).await,
            "files.range" => self.handle_file_range(request.params).await,
            "files.searchContent" => self.handle_file_search_content(request.params).await,

            // Datasource operations
            "datasource.list" => self.handle_datasource_list(request.params).await,
            "datasource.detail" => self.handle_datasource_detail(request.params).await,
            "datasource.query" => self.handle_datasource_query(request.params).await,
            "datasource.inspect" => self.handle_datasource_inspect(request.params).await,

            _ => Err(format!("Unknown method: {}", request.method)),
        };

        match result {
            Ok(value) => RpcResponse::success(request.id, value),
            Err(error) => RpcResponse::error(request.id, error),
        }
    }

    async fn handle_file_list(&self, params: Value) -> Result<Value, String> {
        let params_obj = params.as_object().cloned().unwrap_or_default();

        let result = self
            .mcp_handlers
            .handle_file_list(&params_obj)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::from_str(&result).unwrap_or(Value::Null))
    }

    async fn handle_file_read(&self, params: Value) -> Result<Value, String> {
        let params_obj = params.as_object().cloned().unwrap_or_default();

        let result = self
            .mcp_handlers
            .handle_file_read(&params_obj)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::from_str(&result).unwrap_or(Value::Null))
    }

    async fn handle_file_search(&self, params: Value) -> Result<Value, String> {
        let params_obj = params.as_object().cloned().unwrap_or_default();

        let result = self
            .mcp_handlers
            .handle_file_search(&params_obj)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::from_str(&result).unwrap_or(Value::Null))
    }

    async fn handle_file_metadata(&self, params: Value) -> Result<Value, String> {
        let params_obj = params.as_object().cloned().unwrap_or_default();

        let result = self
            .mcp_handlers
            .handle_file_metadata(&params_obj)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::from_str(&result).unwrap_or(Value::Null))
    }

    async fn handle_file_peek(&self, params: Value) -> Result<Value, String> {
        let params_obj = params.as_object().ok_or("Invalid params")?;

        let result = self
            .mcp_handlers
            .handle_file_peek(params_obj)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::json!({ "content": result }))
    }

    async fn handle_file_range(&self, params: Value) -> Result<Value, String> {
        let params_obj = params.as_object().ok_or("Invalid params")?;

        let result = self
            .mcp_handlers
            .handle_file_range(params_obj)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::json!({ "content": result }))
    }

    async fn handle_file_search_content(&self, params: Value) -> Result<Value, String> {
        let params_obj = params.as_object().ok_or("Invalid params")?;

        let result = self
            .mcp_handlers
            .handle_file_search_content(params_obj)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::from_str(&result).unwrap_or(Value::Null))
    }

    // Datasource operations

    async fn handle_datasource_list(&self, params: Value) -> Result<Value, String> {
        let params_obj = params.as_object().cloned().unwrap_or_default();

        let result = self
            .mcp_handlers
            .handle_datasource_list(&params_obj)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::from_str(&result).unwrap_or(Value::Null))
    }

    async fn handle_datasource_detail(&self, params: Value) -> Result<Value, String> {
        let params_obj = params.as_object().cloned().unwrap_or_default();

        let result = self
            .mcp_handlers
            .handle_datasource_detail(&params_obj)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::from_str(&result).unwrap_or(Value::Null))
    }

    async fn handle_datasource_query(&self, params: Value) -> Result<Value, String> {
        let params_obj = params.as_object().ok_or("Invalid params")?;

        let result = self
            .mcp_handlers
            .handle_datasource_query(params_obj)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::from_str(&result).unwrap_or(Value::Null))
    }

    async fn handle_datasource_inspect(&self, params: Value) -> Result<Value, String> {
        let params_obj = params.as_object().cloned().unwrap_or_default();

        let result = self
            .mcp_handlers
            .handle_datasource_inspect(&params_obj)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::from_str(&result).unwrap_or(Value::Null))
    }
}

/// Start MCP bridge RPC server
pub async fn run_mcp_bridge(
    mut stdin: ChildStdin,
    stdout_lines: &mut tokio::io::Lines<BufReader<tokio::process::ChildStdout>>,
    project_id: String,
    job_id: String,
    db_pool: PgPool,
) -> Result<Value> {
    let bridge = McpBridge::new(project_id, job_id, db_pool);
    let mut result_json: Option<Value> = None;

    // Read RPC requests from Bun's stdout
    while let Ok(Some(line)) = stdout_lines.next_line().await {
        // Check if this is an RPC request (starts with "RPC:")
        if let Some(json_str) = line.strip_prefix("RPC:") {
            if let Ok(request) = serde_json::from_str::<RpcRequest>(json_str) {
                tracing::debug!("MCP RPC request: {} {}", request.id, request.method);

                // Handle the request
                let response = bridge.handle_request(request).await;

                // Send response back to Bun via stdin
                let response_json = serde_json::to_string(&response)?;
                stdin.write_all(response_json.as_bytes()).await?;
                stdin.write_all(b"\n").await?;
                stdin.flush().await?;
            }
        } else {
            // Not an RPC request, check if it's the final result JSON
            if let Ok(json) = serde_json::from_str::<Value>(&line) {
                result_json = Some(json);
            }
        }
    }

    result_json.ok_or_else(|| anyhow::anyhow!("No result returned from analysis"))
}
