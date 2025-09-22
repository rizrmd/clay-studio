use anyhow::{anyhow, Result};
use quickjs_runtime::builder::QuickJsRuntimeBuilder;
use quickjs_runtime::facades::QuickJsRuntimeFacade;
use quickjs_runtime::jsutils::Script;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::models::analysis::*;
use crate::core::mcp::handlers::base::McpHandlers;
use crate::core::mcp::handlers::tools::interaction;
use crate::core::mcp::types::JsonRpcError;
use super::{duckdb_manager::DuckDBManager, datasource_service::AnalysisDatasourceService};

pub struct SandboxContext {
    pub runtime: Arc<QuickJsRuntimeFacade>,
    pub duckdb: Arc<DuckDBManager>,
    pub datasource_service: Arc<AnalysisDatasourceService>,
    pub mcp_handlers: Arc<McpHandlers>,
    pub project_id: Uuid,
    pub job_id: Uuid,
    pub datasources: HashMap<String, Value>,
    pub metadata: HashMap<String, Value>,
    pub logs: Vec<String>,
    pub should_stop: Arc<Mutex<bool>>,
}

pub struct AnalysisSandbox {
    duckdb: Arc<DuckDBManager>,
    datasource_service: AnalysisDatasourceService,
    db_pool: sqlx::PgPool,
}

impl AnalysisSandbox {
    pub async fn new(data_dir: PathBuf, db_pool: sqlx::PgPool) -> Result<Self> {
        let duckdb = Arc::new(DuckDBManager::new(data_dir).await?);
        duckdb.ensure_database_dir().await?;
        let datasource_service = AnalysisDatasourceService::new(db_pool.clone());

        Ok(Self { duckdb, datasource_service, db_pool })
    }

    pub async fn execute_analysis(
        &self,
        analysis: &Analysis,
        parameters: Value,
        job_id: Uuid,
        datasources: HashMap<String, Value>,
    ) -> Result<Value> {
        let context = self.create_context(analysis.project_id, job_id, datasources).await?;
        
        // Parse and validate the script
        let parsed_script = self.parse_analysis_script(&analysis.script_content)?;
        
        // Execute the analysis
        self.execute_in_sandbox(context, &parsed_script, parameters).await
    }

    async fn create_context(
        &self,
        project_id: Uuid,
        job_id: Uuid,
        datasources: HashMap<String, Value>,
    ) -> Result<SandboxContext> {
        let runtime = Arc::new(
            QuickJsRuntimeBuilder::new().build()
        );

        // Create MCP handlers for this project
        // Use a dummy client_id for analysis jobs since they're system-generated
        let mcp_handlers = Arc::new(McpHandlers {
            project_id: project_id.to_string(),
            client_id: job_id.to_string(), // Use job_id as client_id
            server_type: "analysis".to_string(),
            db_pool: self.db_pool.clone(),
        });

        Ok(SandboxContext {
            runtime,
            duckdb: Arc::clone(&self.duckdb),
            datasource_service: Arc::new(self.datasource_service.clone()),
            mcp_handlers,
            project_id,
            job_id,
            datasources,
            metadata: HashMap::new(),
            logs: Vec::new(),
            should_stop: Arc::new(Mutex::new(false)),
        })
    }

    fn parse_analysis_script(&self, script_content: &str) -> Result<ParsedAnalysisScript> {
        // For now, we'll do basic validation
        // In a full implementation, we'd parse the JS to extract metadata
        if !script_content.contains("export default") {
            return Err(anyhow!("Analysis script must export a default object"));
        }

        Ok(ParsedAnalysisScript {
            content: script_content.to_string(),
            title: "Parsed Analysis".to_string(),
            dependencies: AnalysisDependencies {
                datasources: vec![],
                analyses: vec![],
            },
            parameters: HashMap::new(),
        })
    }

    async fn execute_in_sandbox(
        &self,
        mut context: SandboxContext,
        script: &ParsedAnalysisScript,
        parameters: Value,
    ) -> Result<Value> {
        let runtime: Arc<QuickJsRuntimeFacade> = Arc::clone(&context.runtime);

        // Set up the execution timeout
        let timeout_ms = 30000; // 30 seconds default

        // Inject context APIs into the runtime
        self.inject_context_apis(&mut context).await?;

        // Create the main execution script
        let execution_script = format!(
            r#"
            const analysisScript = {};
            const parameters = {};
            
            async function executeAnalysis() {{
                if (!analysisScript.run || typeof analysisScript.run !== 'function') {{
                    throw new Error('Analysis script must have a run function');
                }}
                
                const result = await analysisScript.run(globalThis.ctx, parameters);
                
                // Validate result size (10MB limit)
                const resultStr = JSON.stringify(result);
                if (resultStr.length > 10 * 1024 * 1024) {{
                    throw new Error('Result exceeds 10MB limit. Use DuckDB for large datasets.');
                }}
                
                return result;
            }}
            
            executeAnalysis();
            "#,
            script.content,
            serde_json::to_string(&parameters)?
        );

        // Execute with timeout
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(timeout_ms),
            self.execute_script(&runtime, &execution_script),
        )
        .await
        .map_err(|_| anyhow!("Analysis execution timeout after {}ms", timeout_ms))??;

        Ok(result)
    }

    async fn inject_context_apis(
        &self,
        context: &mut SandboxContext,
    ) -> Result<()> {
        let runtime: Arc<QuickJsRuntimeFacade> = Arc::clone(&context.runtime);

        // Create the global context object
        let ctx_api = format!(
            r#"
            globalThis.ctx = {{
                projectId: "{}",
                jobId: "{}",
                datasources: {},
                metadata: {},
                
                // DuckDB query function
                query: async function(sql, params = []) {{
                    // In a full implementation, this would execute DuckDB queries
                    // For now, return a mock response
                    console.log("Executing query:", sql, "with params:", params);
                    return {{ rows: [], columns: [] }};
                }},
                
                // Logging function
                log: function(...args) {{
                    console.log(...args);
                }},
                
                // Get datasource metadata
                getDatasource: function(name) {{
                    return globalThis.ctx.datasources[name] || null;
                }},
                
                // Store analysis metadata
                setMetadata: function(key, value) {{
                    globalThis.ctx.metadata[key] = value;
                }},
                
                // Get analysis metadata
                getMetadata: function(key) {{
                    return globalThis.ctx.metadata[key];
                }}
            }};
            "#,
            context.project_id,
            context.job_id,
            serde_json::to_string(&context.datasources)?,
            serde_json::to_string(&context.metadata)?
        );

        // Execute the context setup using the facade
        runtime.eval(Script::new("context_setup.js", &ctx_api)).await
            .map_err(|e| anyhow!("Failed to inject context APIs: {:?}", e))?;

        // Set up the MCP tool calling bridge
        self.setup_mcp_tool_bridge(&runtime, &context.mcp_handlers).await?;

        Ok(())
    }

    async fn setup_mcp_tool_bridge(
        &self,
        runtime: &QuickJsRuntimeFacade,
        mcp_handlers: &Arc<McpHandlers>,
    ) -> Result<()> {
        // For now, we'll create a simplified version that directly handles file operations
        // In a full implementation, this would use proper Rust-JS function bindings
        let handlers = Arc::clone(mcp_handlers);
        
        // Create a simplified version using eval for now
        // This demonstrates the concept without requiring complex function bindings
        let bridge_code = format!(
            r#"
            // File operations implemented directly in JavaScript for now
            // In production, these would call Rust functions via proper bindings
            
            // Store MCP handlers reference (simplified approach)
            globalThis._mcpHandlersProjectId = "{}";
            globalThis._mcpHandlersClientId = "{}";
            
            // Mock file operations for demonstration
            globalThis.ctx.files = {{
                list: async function(conversationId) {{
                    console.log("File list called with conversationId:", conversationId);
                    // Return mock data for now
                    return [
                        {{
                            id: "file1",
                            name: "sales_data.csv",
                            size: 1024,
                            type: "text/csv",
                            created_at: new Date().toISOString()
                        }},
                        {{
                            id: "file2", 
                            name: "customer_data.json",
                            size: 2048,
                            type: "application/json",
                            created_at: new Date().toISOString()
                        }}
                    ];
                }},
                
                read: async function(fileId) {{
                    console.log("File read called for file:", fileId);
                    // Mock file content
                    if (fileId === "file1") {{
                        return "name,age,city\nJohn,25,NYC\nJane,30,LA";
                    }} else if (fileId === "file2") {{
                        return JSON.stringify([{{name: "John", age: 25}}, {{name: "Jane", age: 30}}], null, 2);
                    }}
                    throw new Error("File not found: " + fileId);
                }},
                
                search: async function(query, options = {{}}) {{
                    console.log("File search called with query:", query, "options:", options);
                    return [
                        {{
                            id: "file1",
                            name: "sales_data.csv",
                            snippet: "Contains sales data..."
                        }}
                    ];
                }},
                
                getMetadata: async function(fileId) {{
                    console.log("File metadata called for file:", fileId);
                    return {{
                        id: fileId,
                        name: "example.csv",
                        size: 1024,
                        type: "text/csv",
                        created_at: new Date().toISOString(),
                        updated_at: new Date().toISOString()
                    }};
                }},
                
                peek: async function(fileId, options = {{}}) {{
                    console.log("File peek called for file:", fileId, "options:", options);
                    return "First few lines of file content...";
                }},
                
                range: async function(fileId, start, end) {{
                    console.log("File range called for file:", fileId, "start:", start, "end:", end);
                    return "Lines " + start + " to " + end + " of the file";
                }},
                
                searchContent: async function(fileId, pattern, options = {{}}) {{
                    console.log("File content search called for file:", fileId, "pattern:", pattern);
                    return [
                        {{
                            line: 1,
                            content: "Line containing the pattern"
                        }}
                    ];
                }},
                
                getDownloadUrl: async function(fileId) {{
                    console.log("File download URL called for file:", fileId);
                    return "/api/files/" + fileId + "/download";
                }}
            }};
            "#,
            handlers.project_id,
            handlers.client_id
        );

        runtime.eval(Script::new("mcp_bridge.js", &bridge_code)).await
            .map_err(|e| anyhow!("Failed to set up MCP bridge: {:?}", e))?;

        Ok(())
    }

    async fn execute_script(&self, runtime: &QuickJsRuntimeFacade, script: &str) -> Result<Value> {
        // Execute the script and get the result
        let result_facade = runtime.eval(Script::new("analysis_execution.js", script)).await
            .map_err(|e| anyhow!("Script execution failed: {:?}", e))?;
        
        // Convert the result to JSON string for parsing
        let json_result = runtime.eval(Script::new("to_json.js", "JSON.stringify(result)")).await
            .map_err(|e| anyhow!("Failed to convert result to JSON: {:?}", e))?;
        
        // For now, return a success indicator since we need to implement proper value conversion
        // In a full implementation, you'd convert the JsValueFacade to serde_json::Value
        Ok(serde_json::json!({
            "status": "completed",
            "message": "Analysis executed successfully"
        }))
    }

    pub async fn validate_analysis(&self, script_content: &str) -> Result<ValidationResult> {
        let mut errors = Vec::new();

        // Basic validation - check for required patterns
        if !script_content.contains("export default") {
            errors.push("Script must export a default object".to_string());
        }

        if !script_content.contains("run:") && !script_content.contains("run ") {
            errors.push("Analysis must have a run function".to_string());
        }

        if errors.is_empty() {
            Ok(ValidationResult {
                valid: true,
                errors: Vec::new(),
                metadata: None,
            })
        } else {
            Ok(ValidationResult {
                valid: false,
                errors,
                metadata: None,
            })
        }
    }

    pub async fn get_parameter_options(
        &self,
        _analysis: &Analysis,
        _parameter_name: &str,
        _current_params: Value,
    ) -> Result<Vec<ParameterOption>> {
        // This would execute the parameter's options function in the sandbox
        // For now, return empty options
        Ok(vec![])
    }
}

#[derive(Debug, Clone)]
pub struct ParsedAnalysisScript {
    pub content: String,
    pub title: String,
    pub dependencies: AnalysisDependencies,
    pub parameters: HashMap<String, AnalysisParameter>,
}