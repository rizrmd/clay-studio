use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use sqlx::PgPool;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::fs;
use tokio::process::Command;
use uuid::Uuid;

use super::mcp_bridge;

/// Bun runtime for executing analysis scripts
pub struct BunRuntime {
    bun_path: PathBuf,
    clients_dir: PathBuf,
    db_pool: Option<PgPool>,
}

impl BunRuntime {
    pub fn new(clients_dir: PathBuf) -> Result<Self> {
        // Use the bun from .clients/bun/bin/bun or system bun
        let bun_path = clients_dir
            .join("bun")
            .join("bin")
            .join("bun");

        let bun_path = if bun_path.exists() {
            bun_path
        } else {
            // Fall back to system bun
            PathBuf::from("bun")
        };

        Ok(Self {
            bun_path,
            clients_dir,
            db_pool: None,
        })
    }

    pub fn with_db_pool(mut self, db_pool: PgPool) -> Self {
        self.db_pool = Some(db_pool);
        self
    }

    /// Get or create the analysis directory for a project
    pub async fn get_project_analysis_dir(&self, project_id: Uuid) -> Result<PathBuf> {
        let project_dir = self.clients_dir.join(project_id.to_string());
        let analysis_dir = project_dir.join("analysis");

        // Create directory structure if it doesn't exist
        fs::create_dir_all(&analysis_dir).await
            .context("Failed to create analysis directory")?;

        // Create subdirectories
        fs::create_dir_all(analysis_dir.join("scripts")).await?;
        fs::create_dir_all(analysis_dir.join("temp")).await?;

        // Create package.json if it doesn't exist
        let package_json_path = analysis_dir.join("package.json");
        if !package_json_path.exists() {
            self.create_package_json(&package_json_path).await?;
        }

        Ok(analysis_dir)
    }

    async fn create_package_json(&self, path: &Path) -> Result<()> {
        let package_json = serde_json::json!({
            "name": "clay-analysis-runtime",
            "version": "1.0.0",
            "type": "module",
            "dependencies": {
                "duckdb": "^1.1.3",
                "csv-parse": "^5.6.0",
                "postgres": "^3.4.5",
                "mysql2": "^3.11.5"
            }
        });

        let content = serde_json::to_string_pretty(&package_json)?;
        fs::write(path, content).await?;

        Ok(())
    }

    /// Install dependencies for a project's analysis directory
    pub async fn install_dependencies(&self, project_id: Uuid) -> Result<()> {
        let analysis_dir = self.get_project_analysis_dir(project_id).await?;

        let output = Command::new(&self.bun_path)
            .arg("install")
            .current_dir(&analysis_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to install dependencies: {}", stderr));
        }

        Ok(())
    }

    /// Execute an analysis script with the given context
    pub async fn execute_analysis(
        &self,
        project_id: Uuid,
        job_id: Uuid,
        script_content: &str,
        parameters: Value,
        context: Value,
        backend_url: Option<String>,
        auth_token: Option<String>,
    ) -> Result<Value> {
        let analysis_dir = self.get_project_analysis_dir(project_id).await?;
        let temp_dir = analysis_dir.join("temp");

        // Write the analysis script
        let script_path = temp_dir.join(format!("{}.ts", job_id));
        fs::write(&script_path, script_content).await?;

        // Generate or use provided auth token
        let token = auth_token.unwrap_or_else(|| {
            // Generate a short-lived token for this job
            format!("job-{}-{}", project_id, job_id)
        });

        // Write the context data
        let context_path = temp_dir.join(format!("{}_context.json", job_id));
        let context_json = serde_json::json!({
            "projectId": project_id.to_string(),
            "jobId": job_id.to_string(),
            "parameters": parameters,
            "context": context,
            "backendUrl": backend_url.unwrap_or_else(|| "http://localhost:8000".to_string()),
            "authToken": token
        });
        fs::write(&context_path, serde_json::to_string(&context_json)?).await?;

        // Create the execution wrapper
        let wrapper_path = temp_dir.join(format!("{}_wrapper.ts", job_id));
        self.create_execution_wrapper(&wrapper_path, &script_path, &context_path).await?;

        // Execute with timeout
        let timeout_secs = 300; // 5 minutes
        let result = tokio::time::timeout(
            tokio::time::Duration::from_secs(timeout_secs),
            self.run_bun_script(&wrapper_path, &analysis_dir, project_id, job_id),
        )
        .await
        .map_err(|_| anyhow!("Analysis execution timeout after {} seconds", timeout_secs))??;

        // Clean up temp files
        let _ = fs::remove_file(&script_path).await;
        let _ = fs::remove_file(&context_path).await;
        let _ = fs::remove_file(&wrapper_path).await;

        Ok(result)
    }

    async fn create_execution_wrapper(
        &self,
        wrapper_path: &Path,
        script_path: &Path,
        context_path: &Path,
    ) -> Result<()> {
        let wrapper_content = format!(
            r#"
import {{ readFileSync }} from 'fs';
import {{ Database }} from 'duckdb';

// Load context
const contextData = JSON.parse(readFileSync('{}', 'utf-8'));

// Initialize DuckDB
const db = new Database(':memory:');
const conn = db.connect();

// Helper to promisify DuckDB queries
function queryDuckDB(sql: string, params: any[] = []): Promise<any> {{
    return new Promise((resolve, reject) => {{
        conn.all(sql, ...params, (err: any, rows: any) => {{
            if (err) {{
                reject(err);
            }} else {{
                // Extract column names from first row if available
                const columns = rows.length > 0 ? Object.keys(rows[0]) : [];
                resolve({{ rows, columns }});
            }}
        }});
    }});
}}

// Helper to query external datasources
async function queryDatasource(datasource: any, sql: string, params: any[] = []): Promise<any> {{
    const {{ type, config }} = datasource;

    switch (type) {{
        case 'postgres': {{
            const {{ default: postgres }} = await import('postgres');
            const sql_fn = postgres({{
                host: config.host,
                port: config.port,
                database: config.database,
                username: config.username || config.user,
                password: config.password,
            }});

            try {{
                const rows = await sql_fn.unsafe(sql, params);
                const columns = rows.length > 0 ? Object.keys(rows[0]) : [];
                await sql_fn.end();
                return {{ rows, columns }};
            }} catch (error) {{
                await sql_fn.end();
                throw error;
            }}
        }}

        case 'mysql': {{
            const {{ default: mysql }} = await import('mysql2/promise');
            const connection = await mysql.createConnection({{
                host: config.host,
                port: config.port,
                database: config.database,
                user: config.username || config.user,
                password: config.password,
            }});

            try {{
                const [rows] = await connection.execute(sql, params);
                const columns = Array.isArray(rows) && rows.length > 0 ? Object.keys(rows[0]) : [];
                await connection.end();
                return {{ rows, columns }};
            }} catch (error) {{
                await connection.end();
                throw error;
            }}
        }}

        default:
            throw new Error(`Unsupported datasource type: ${{type}}`);
    }}
}}

// Create context API
const ctx = {{
    projectId: contextData.projectId,
    jobId: contextData.jobId,
    datasources: contextData.context.datasources || {{}},
    metadata: contextData.context.metadata || {{}},

    // DuckDB query function - default queries go to in-memory DuckDB
    query: async (sql: string, params: any[] = []) => {{
        console.error('[query:duckdb]', sql);
        return await queryDuckDB(sql, params);
    }},

    // Query a specific external datasource via MCP (uses connection pooling!)
    queryDatasource: async (datasourceName: string, sql: string, params: any[] = []) => {{
        console.error('[query:datasource]', datasourceName, sql);
        try {{
            const result = await ctx._rpc('datasource.query', {{
                datasource_name: datasourceName,
                query: sql,
                params: params,
                limit: 10000
            }});
            return result;
        }} catch (error) {{
            console.error('[queryDatasource] Error:', error);
            throw error;
        }}
    }},

    // Load data into DuckDB from various sources
    loadData: async (tableName: string, data: any[]) => {{
        if (!Array.isArray(data) || data.length === 0) {{
            throw new Error('Data must be a non-empty array');
        }}

        // Create table from data structure
        const columns = Object.keys(data[0]);
        const placeholders = columns.map(() => '?').join(', ');

        // Create table (simple approach - all columns as VARCHAR)
        const createTableSQL = `CREATE TABLE ${{tableName}} (${{columns.join(' VARCHAR, ')}} VARCHAR)`;
        await queryDuckDB(createTableSQL);

        // Insert data
        const insertSQL = `INSERT INTO ${{tableName}} (${{columns.join(', ')}}) VALUES (${{placeholders}})`;
        for (const row of data) {{
            const values = columns.map(col => row[col]);
            await queryDuckDB(insertSQL, values);
        }}

        console.error(`[loadData] Loaded ${{data.length}} rows into ${{tableName}}`);
    }},

    log: (...args: any[]) => {{
        console.error('[log]', ...args);
    }},

    getDatasource: (name: string) => {{
        return ctx.datasources[name] || null;
    }},

    setMetadata: (key: string, value: any) => {{
        ctx.metadata[key] = value;
    }},

    getMetadata: (key: string) => {{
        return ctx.metadata[key];
    }},

    // MCP RPC call helper
    _rpc: async (method: string, params: any = {{}}) => {{
        const requestId = `req-${{Date.now()}}-${{Math.random().toString(36).substr(2, 9)}}`;

        return new Promise((resolve, reject) => {{
            // Store pending request
            if (!globalThis._pendingRpcRequests) {{
                globalThis._pendingRpcRequests = new Map();
            }}
            globalThis._pendingRpcRequests.set(requestId, {{ resolve, reject }});

            // Send RPC request to Rust via stdout (prefixed with "RPC:")
            console.log(`RPC:${{JSON.stringify({{ id: requestId, method, params }})}}`);

            // Timeout after 30s
            setTimeout(() => {{
                const pending = globalThis._pendingRpcRequests.get(requestId);
                if (pending) {{
                    globalThis._pendingRpcRequests.delete(requestId);
                    reject(new Error(`RPC timeout: ${{method}}`));
                }}
            }}, 30000);
        }});
    }},

    // HTTP helper with retry and auth (fallback)
    _fetch: async (url: string, options: any = {{}}, retries = 3) => {{
        const headers = {{
            'Content-Type': 'application/json',
            'Authorization': `Bearer ${{contextData.authToken}}`,
            'X-Project-ID': contextData.projectId,
            'X-Job-ID': contextData.jobId,
            ...options.headers
        }};

        for (let attempt = 0; attempt < retries; attempt++) {{
            try {{
                const response = await fetch(url, {{
                    ...options,
                    headers,
                    signal: AbortSignal.timeout(30000) // 30s timeout
                }});

                if (!response.ok) {{
                    throw new Error(`HTTP ${{response.status}}: ${{response.statusText}}`);
                }}

                return response;
            }} catch (error) {{
                if (attempt === retries - 1) throw error;

                // Exponential backoff: 100ms, 200ms, 400ms
                const delay = 100 * Math.pow(2, attempt);
                console.error(`[http] Retry ${{attempt + 1}}/${{retries}} after ${{delay}}ms:`, error.message);
                await new Promise(resolve => setTimeout(resolve, delay));
            }}
        }}
    }},

    files: {{
        list: async (conversationId?: string) => {{
            try {{
                const result = await ctx._rpc('files.list', {{ conversationId }});
                return result.files || [];
            }} catch (error) {{
                console.error('[files.list] Error:', error);
                throw error;
            }}
        }},

        read: async (fileId: string) => {{
            try {{
                const result = await ctx._rpc('files.read', {{ fileId }});
                return result.content || '';
            }} catch (error) {{
                console.error('[files.read] Error:', error);
                throw error;
            }}
        }},

        search: async (query: string, options?: any) => {{
            try {{
                const result = await ctx._rpc('files.search', {{
                    query,
                    conversationId: options?.conversationId
                }});
                return result.results || [];
            }} catch (error) {{
                console.error('[files.search] Error:', error);
                throw error;
            }}
        }},

        getMetadata: async (fileId: string) => {{
            try {{
                return await ctx._rpc('files.metadata', {{ fileId }});
            }} catch (error) {{
                console.error('[files.getMetadata] Error:', error);
                throw error;
            }}
        }},

        peek: async (fileId: string, options?: any) => {{
            try {{
                const result = await ctx._rpc('files.peek', {{
                    fileId,
                    ...options
                }});
                return result.content || '';
            }} catch (error) {{
                console.error('[files.peek] Error:', error);
                throw error;
            }}
        }},

        range: async (fileId: string, start: number, end: number) => {{
            try {{
                const result = await ctx._rpc('files.range', {{
                    fileId,
                    start,
                    end
                }});
                return result.content || '';
            }} catch (error) {{
                console.error('[files.range] Error:', error);
                throw error;
            }}
        }},

        searchContent: async (fileId: string, pattern: string, options?: any) => {{
            try {{
                return await ctx._rpc('files.searchContent', {{
                    fileId,
                    pattern,
                    ...options
                }});
            }} catch (error) {{
                console.error('[files.searchContent] Error:', error);
                throw error;
            }}
        }}
    }},

    datasource: {{
        list: async () => {{
            try {{
                const result = await ctx._rpc('datasource.list', {{}});
                return result.datasources || [];
            }} catch (error) {{
                console.error('[datasource.list] Error:', error);
                throw error;
            }}
        }},

        detail: async (name: string) => {{
            try {{
                return await ctx._rpc('datasource.detail', {{ name }});
            }} catch (error) {{
                console.error('[datasource.detail] Error:', error);
                throw error;
            }}
        }},

        inspect: async (name: string) => {{
            try {{
                return await ctx._rpc('datasource.inspect', {{ name }});
            }} catch (error) {{
                console.error('[datasource.inspect] Error:', error);
                throw error;
            }}
        }},

        query: async (name: string, query: string, params: any[] = [], limit: number = 10000) => {{
            try {{
                return await ctx._rpc('datasource.query', {{
                    datasource_name: name,
                    query,
                    params,
                    limit
                }});
            }} catch (error) {{
                console.error('[datasource.query] Error:', error);
                throw error;
            }}
        }}
    }}
}};

// Set up stdin listener for RPC responses
process.stdin.setEncoding('utf-8');
process.stdin.on('data', (data) => {{
    try {{
        const response = JSON.parse(data.toString().trim());
        if (response.id && globalThis._pendingRpcRequests) {{
            const pending = globalThis._pendingRpcRequests.get(response.id);
            if (pending) {{
                globalThis._pendingRpcRequests.delete(response.id);
                if (response.error) {{
                    pending.reject(new Error(response.error));
                }} else {{
                    pending.resolve(response.result);
                }}
            }}
        }}
    }} catch (err) {{
        console.error('[stdin] Failed to parse RPC response:', err);
    }}
}});

// Load and execute the analysis script
async function main() {{
    try {{
        const module = await import('{}');
        const analysis = module.default;

        if (!analysis || typeof analysis.run !== 'function') {{
            throw new Error('Analysis script must export default object with run function');
        }}

        const result = await analysis.run(ctx, contextData.parameters);

        // Validate result size (10MB limit)
        const resultStr = JSON.stringify(result);
        if (resultStr.length > 10 * 1024 * 1024) {{
            throw new Error('Result exceeds 10MB limit. Use DuckDB for large datasets.');
        }}

        // Output result to stdout
        console.log(JSON.stringify({{
            success: true,
            result: result
        }}));

        // Note: Closing DuckDB connections can cause Bun segfault
        // Let the process exit naturally to clean up
    }} catch (error) {{
        // Don't close connections on error, just exit

        console.log(JSON.stringify({{
            success: false,
            error: error instanceof Error ? error.message : String(error),
            stack: error instanceof Error ? error.stack : undefined
        }}));
        process.exit(1);
    }}
}}

main();
"#,
            context_path.display(),
            script_path.display()
        );

        fs::write(wrapper_path, wrapper_content).await?;
        Ok(())
    }

    async fn run_bun_script(&self, script_path: &Path, working_dir: &Path, project_id: Uuid, job_id: Uuid) -> Result<Value> {
        use tokio::io::{AsyncBufReadExt, BufReader};

        let mut child = Command::new(&self.bun_path)
            .arg("run")
            .arg(script_path)
            .current_dir(working_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdin = child.stdin.take().expect("Failed to open stdin");
        let stdout = child.stdout.take().expect("Failed to open stdout");
        let stderr = child.stderr.take().expect("Failed to open stderr");

        let mut stdout_reader = BufReader::new(stdout).lines();
        let stderr_reader = BufReader::new(stderr);

        // Collect stderr for logging
        let stderr_handle = tokio::spawn(async move {
            let mut stderr_lines = BufReader::new(stderr_reader.into_inner()).lines();
            let mut all_stderr = String::new();
            while let Ok(Some(line)) = stderr_lines.next_line().await {
                if !line.is_empty() {
                    tracing::debug!("Analysis: {}", line);
                    all_stderr.push_str(&line);
                    all_stderr.push('\n');
                }
            }
            all_stderr
        });

        // Handle stdout - either with MCP bridge or simple result collection
        let db_pool = self.db_pool.clone();
        let project_id_str = project_id.to_string();
        let job_id_str = job_id.to_string();

        let result_json = if let Some(pool) = db_pool {
            // MCP bridge handles both RPC and result collection
            match mcp_bridge::run_mcp_bridge(
                stdin,
                &mut stdout_reader,
                project_id_str,
                job_id_str,
                pool,
            ).await {
                Ok(result) => Some(result),
                Err(e) => {
                    tracing::error!("MCP bridge error: {}", e);
                    None
                }
            }
        } else {
            // No MCP, just collect result from stdout
            drop(stdin);
            let mut result: Option<Value> = None;
            while let Ok(Some(line)) = stdout_reader.next_line().await {
                if let Ok(json) = serde_json::from_str::<Value>(&line) {
                    result = Some(json);
                }
            }
            result
        };

        // Wait for process to exit
        let _status = child.wait().await?;

        // Wait for stderr collection
        let stderr = stderr_handle.await?;

        if !stderr.is_empty() {
            tracing::debug!("Analysis stderr: {}", stderr);
        }

        // Get the result
        let result = result_json
            .ok_or_else(|| anyhow!("Failed to parse analysis result from output"))?;

        // Check if execution was successful
        if result.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
            Ok(result.get("result").cloned().unwrap_or(Value::Null))
        } else {
            let error = result.get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error");
            let stack = result.get("stack")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            Err(anyhow!("Analysis execution failed: {}\n{}", error, stack))
        }
    }

    /// Validate an analysis script syntax
    pub async fn validate_script(
        &self,
        project_id: Uuid,
        script_content: &str,
    ) -> Result<Vec<String>> {
        let analysis_dir = self.get_project_analysis_dir(project_id).await?;
        let temp_dir = analysis_dir.join("temp");

        // Write a temporary validation script
        let validation_id = Uuid::new_v4();
        let script_path = temp_dir.join(format!("validate_{}.ts", validation_id));
        fs::write(&script_path, script_content).await?;

        // Try to parse the script with Bun
        let output = Command::new(&self.bun_path)
            .arg("build")
            .arg(&script_path)
            .arg("--target=node")
            .arg("--format=esm")
            .current_dir(&analysis_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        // Clean up
        let _ = fs::remove_file(&script_path).await;

        let mut errors = Vec::new();

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            errors.push(format!("Syntax error: {}", stderr));
        }

        // Basic validation checks
        if !script_content.contains("export default") {
            errors.push("Script must export a default object".to_string());
        }

        if !script_content.contains("run") {
            errors.push("Analysis must have a run function".to_string());
        }

        Ok(errors)
    }
}
