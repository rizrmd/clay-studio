use std::path::{Path, PathBuf};
use std::fs;
use uuid::Uuid;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json;
use crate::utils::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
    pub client_id: Uuid,
}

pub struct ProjectManager {
    clients_base_dir: PathBuf,
}

impl ProjectManager {
    pub fn new() -> Self {
        let clients_base = std::env::var("CLIENTS_DIR")
            .unwrap_or_else(|_| ".clients".to_string());
        
        Self {
            clients_base_dir: PathBuf::from(clients_base),
        }
    }
    
    pub fn ensure_project_directory(
        &self,
        client_id: Uuid,
        project_id: &str,
    ) -> Result<PathBuf, AppError> {
        let project_dir = self.get_project_directory(client_id, project_id);
        
        if !project_dir.exists() {
            fs::create_dir_all(&project_dir)
                .map_err(|e| AppError::InternalServerError(
                    format!("Failed to create project directory: {}", e)
                ))?;
            
            self.initialize_project_files(&project_dir)?;
            
            tracing::info!("Created project directory: {:?}", project_dir);
        }
        
        Ok(project_dir)
    }
    
    pub fn get_project_directory(&self, client_id: Uuid, project_id: &str) -> PathBuf {
        self.clients_base_dir
            .join(client_id.to_string())
            .join(project_id)
    }
    
    fn initialize_project_files(&self, project_dir: &Path) -> Result<(), AppError> {
        let claude_dir = project_dir.join(".claude");
        if !claude_dir.exists() {
            fs::create_dir(&claude_dir)
                .map_err(|e| AppError::InternalServerError(
                    format!("Failed to create .claude directory: {}", e)
                ))?;
        }
        
        // Create MCP configuration for this project
        let mcp_config_file = claude_dir.join("settings.local.json");
        if !mcp_config_file.exists() {
            // Get MCP server path - look for the built binary
            let mcp_server_path = {
                let release_path = std::env::current_dir()
                    .map(|p| p.join("target/release/mcp_server"))
                    .unwrap_or_else(|_| PathBuf::from("target/release/mcp_server"));
                let debug_path = std::env::current_dir()
                    .map(|p| p.join("target/debug/mcp_server"))
                    .unwrap_or_else(|_| PathBuf::from("target/debug/mcp_server"));
                
                if release_path.exists() {
                    release_path.canonicalize().unwrap_or(release_path)
                } else if debug_path.exists() {
                    debug_path.canonicalize().unwrap_or(debug_path)
                } else {
                    // Default path where it should be after build
                    std::env::current_dir()
                        .map(|p| p.join("target/debug/mcp_server"))
                        .unwrap_or_else(|_| PathBuf::from("/usr/local/bin/mcp_server"))
                }
            };
            
            // Get database URL from environment
            let database_url = std::env::var("DATABASE_URL").unwrap_or_default();
            
            // Extract project ID from directory name
            let project_id = project_dir.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("default")
                .to_string();
            
            // Extract client ID from parent directory
            let client_id = self.extract_client_id_from_path(project_dir)
                .map(|id| id.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            
            let mcp_config = serde_json::json!({
                "mcpServers": {
                    "clay-studio": {
                        "type": "stdio",
                        "command": mcp_server_path.to_string_lossy(),
                        "args": [
                            "--project-id", project_id,
                            "--client-id", client_id
                        ],
                        "env": {
                            "DATABASE_URL": database_url
                        }
                    }
                }
            });
            
            fs::write(&mcp_config_file, serde_json::to_string_pretty(&mcp_config)?)
                .map_err(|e| AppError::InternalServerError(
                    format!("Failed to create MCP config: {}", e)
                ))?;
        }
        
        let claude_md_path = project_dir.join("CLAUDE.md");
        if !claude_md_path.exists() {
            let default_content = r#"# Project Context for Claude

## Project Overview
This is a Clay Studio project workspace.

## Available Tools
- Claude Code SDK integration
- File management
- Query execution

## Notes
Add any project-specific context or instructions here that Claude should be aware of.
"#;
            fs::write(&claude_md_path, default_content)
                .map_err(|e| AppError::InternalServerError(
                    format!("Failed to create CLAUDE.md: {}", e)
                ))?;
        }
        
        let queries_dir = project_dir.join("queries");
        if !queries_dir.exists() {
            fs::create_dir(&queries_dir)
                .map_err(|e| AppError::InternalServerError(
                    format!("Failed to create queries directory: {}", e)
                ))?;
        }
        
        let project_info = ProjectInfo {
            id: project_dir.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string(),
            name: format!("Project {}", project_dir.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")),
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
            client_id: self.extract_client_id_from_path(project_dir)
                .unwrap_or(Uuid::new_v4()),
        };
        
        let project_info_path = project_dir.join(".project.json");
        let project_info_json = serde_json::to_string_pretty(&project_info)
            .map_err(|e| AppError::InternalServerError(
                format!("Failed to serialize project info: {}", e)
            ))?;
        
        fs::write(&project_info_path, project_info_json)
            .map_err(|e| AppError::InternalServerError(
                format!("Failed to write project info: {}", e)
            ))?;
        
        Ok(())
    }
    
    fn extract_client_id_from_path(&self, project_dir: &Path) -> Option<Uuid> {
        project_dir.parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .and_then(|s| Uuid::parse_str(s).ok())
    }
    
    #[allow(dead_code)]
    pub fn list_projects(&self, client_id: Uuid) -> Result<Vec<ProjectInfo>, AppError> {
        let client_dir = self.clients_base_dir.join(client_id.to_string());
        
        if !client_dir.exists() {
            return Ok(Vec::new());
        }
        
        let mut projects = Vec::new();
        
        let entries = fs::read_dir(&client_dir)
            .map_err(|e| AppError::InternalServerError(
                format!("Failed to read client directory: {}", e)
            ))?;
        
        for entry in entries {
            let entry = entry.map_err(|e| AppError::InternalServerError(
                format!("Failed to read directory entry: {}", e)
            ))?;
            
            let path = entry.path();
            if path.is_dir() {
                let project_info_path = path.join(".project.json");
                if project_info_path.exists() {
                    let content = fs::read_to_string(&project_info_path)
                        .map_err(|e| AppError::InternalServerError(
                            format!("Failed to read project info: {}", e)
                        ))?;
                    
                    let project_info: ProjectInfo = serde_json::from_str(&content)
                        .map_err(|e| AppError::InternalServerError(
                            format!("Failed to parse project info: {}", e)
                        ))?;
                    
                    projects.push(project_info);
                }
            }
        }
        
        Ok(projects)
    }
    
    pub fn save_query(
        &self,
        client_id: Uuid,
        project_id: &str,
        query_name: &str,
        content: &str,
    ) -> Result<PathBuf, AppError> {
        let project_dir = self.ensure_project_directory(client_id, project_id)?;
        let queries_dir = project_dir.join("queries");
        
        if !queries_dir.exists() {
            fs::create_dir(&queries_dir)
                .map_err(|e| AppError::InternalServerError(
                    format!("Failed to create queries directory: {}", e)
                ))?;
        }
        
        let safe_name = query_name
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect::<String>();
        
        let query_path = queries_dir.join(format!("{}.txt", safe_name));
        
        fs::write(&query_path, content)
            .map_err(|e| AppError::InternalServerError(
                format!("Failed to save query: {}", e)
            ))?;
        
        Ok(query_path)
    }
    
    #[allow(dead_code)]
    pub fn load_query(
        &self,
        client_id: Uuid,
        project_id: &str,
        query_name: &str,
    ) -> Result<String, AppError> {
        let project_dir = self.get_project_directory(client_id, project_id);
        let query_path = project_dir.join("queries").join(format!("{}.txt", query_name));
        
        if !query_path.exists() {
            return Err(AppError::NotFound(format!("Query '{}' not found", query_name)));
        }
        
        fs::read_to_string(&query_path)
            .map_err(|e| AppError::InternalServerError(
                format!("Failed to read query: {}", e)
            ))
    }
    
    pub fn list_queries(
        &self,
        client_id: Uuid,
        project_id: &str,
    ) -> Result<Vec<String>, AppError> {
        let project_dir = self.get_project_directory(client_id, project_id);
        let queries_dir = project_dir.join("queries");
        
        if !queries_dir.exists() {
            return Ok(Vec::new());
        }
        
        let mut queries = Vec::new();
        
        let entries = fs::read_dir(&queries_dir)
            .map_err(|e| AppError::InternalServerError(
                format!("Failed to read queries directory: {}", e)
            ))?;
        
        for entry in entries {
            let entry = entry.map_err(|e| AppError::InternalServerError(
                format!("Failed to read directory entry: {}", e)
            ))?;
            
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("txt") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    queries.push(name.to_string());
                }
            }
        }
        
        Ok(queries)
    }
}