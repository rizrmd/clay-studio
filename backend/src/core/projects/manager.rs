use crate::utils::AppError;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
    pub client_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfoWithStats {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
    pub client_id: Uuid,
    pub conversation_count: Option<i32>,
    pub datasource_count: Option<i32>,
}

pub struct ProjectManager {
    clients_base_dir: PathBuf,
}

impl Default for ProjectManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ProjectManager {
    pub fn new() -> Self {
        let clients_base = std::env::var("CLIENTS_DIR").unwrap_or_else(|_| ".clients".to_string());

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
            fs::create_dir_all(&project_dir).map_err(|e| {
                AppError::InternalServerError(format!("Failed to create project directory: {}", e))
            })?;

            self.initialize_project_files(&project_dir, client_id, project_id)?;

            tracing::info!("Created project directory: {:?}", project_dir);
        }

        Ok(project_dir)
    }

    pub fn get_project_directory(&self, client_id: Uuid, project_id: &str) -> PathBuf {
        self.clients_base_dir
            .join(client_id.to_string())
            .join(project_id)
    }

    fn initialize_project_files(&self, project_dir: &Path, client_id: Uuid, project_id: &str) -> Result<(), AppError> {
        let claude_dir = project_dir.join(".claude");
        if !claude_dir.exists() {
            fs::create_dir(&claude_dir).map_err(|e| {
                AppError::InternalServerError(format!("Failed to create .claude directory: {}", e))
            })?;
        }

        // Create MCP configuration for this project
        let mcp_config_file = claude_dir.join("settings.local.json");
        if !mcp_config_file.exists() {
            // Use the actual project_id and client_id parameters passed to the method
            let project_id_str = project_id.to_string();
            let client_id_str = client_id.to_string();

            let mcp_config = serde_json::json!({
                "mcpServers": {
                    "operation": {
                        "type": "http",
                        "url": format!("http://localhost:7670/operation/{}/{}", client_id_str, project_id_str)
                    },
                    "analysis": {
                        "type": "http",
                        "url": format!("http://localhost:7670/analysis/{}/{}", client_id_str, project_id_str)
                    },
                    "interaction": {
                        "type": "http",
                        "url": format!("http://localhost:7670/interaction/{}/{}", client_id_str, project_id_str)
                    }
                }
            });

            fs::write(&mcp_config_file, serde_json::to_string_pretty(&mcp_config)?).map_err(
                |e| AppError::InternalServerError(format!("Failed to create MCP config: {}", e)),
            )?;
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
            fs::write(&claude_md_path, default_content).map_err(|e| {
                AppError::InternalServerError(format!("Failed to create CLAUDE.md: {}", e))
            })?;
        }

        let queries_dir = project_dir.join("queries");
        if !queries_dir.exists() {
            fs::create_dir(&queries_dir).map_err(|e| {
                AppError::InternalServerError(format!("Failed to create queries directory: {}", e))
            })?;
        }

        let project_info = ProjectInfo {
            id: project_id.to_string(),
            name: format!("Project {}", project_id),
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
            client_id,
        };

        let project_info_path = project_dir.join(".project.json");
        let project_info_json = serde_json::to_string_pretty(&project_info).map_err(|e| {
            AppError::InternalServerError(format!("Failed to serialize project info: {}", e))
        })?;

        fs::write(&project_info_path, project_info_json).map_err(|e| {
            AppError::InternalServerError(format!("Failed to write project info: {}", e))
        })?;

        Ok(())
    }


    #[allow(dead_code)]
    pub fn list_projects(&self, client_id: Uuid) -> Result<Vec<ProjectInfo>, AppError> {
        let client_dir = self.clients_base_dir.join(client_id.to_string());

        if !client_dir.exists() {
            return Ok(Vec::new());
        }

        let mut projects = Vec::new();

        let entries = fs::read_dir(&client_dir).map_err(|e| {
            AppError::InternalServerError(format!("Failed to read client directory: {}", e))
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                AppError::InternalServerError(format!("Failed to read directory entry: {}", e))
            })?;

            let path = entry.path();
            if path.is_dir() {
                let project_info_path = path.join(".project.json");
                if project_info_path.exists() {
                    let content = fs::read_to_string(&project_info_path).map_err(|e| {
                        AppError::InternalServerError(format!("Failed to read project info: {}", e))
                    })?;

                    let project_info: ProjectInfo =
                        serde_json::from_str(&content).map_err(|e| {
                            AppError::InternalServerError(format!(
                                "Failed to parse project info: {}",
                                e
                            ))
                        })?;

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
            fs::create_dir(&queries_dir).map_err(|e| {
                AppError::InternalServerError(format!("Failed to create queries directory: {}", e))
            })?;
        }

        let safe_name = query_name
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect::<String>();

        let query_path = queries_dir.join(format!("{}.txt", safe_name));

        fs::write(&query_path, content)
            .map_err(|e| AppError::InternalServerError(format!("Failed to save query: {}", e)))?;

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
        let query_path = project_dir
            .join("queries")
            .join(format!("{}.txt", query_name));

        if !query_path.exists() {
            return Err(AppError::NotFound(format!(
                "Query '{}' not found",
                query_name
            )));
        }

        fs::read_to_string(&query_path)
            .map_err(|e| AppError::InternalServerError(format!("Failed to read query: {}", e)))
    }

    pub fn list_queries(&self, client_id: Uuid, project_id: &str) -> Result<Vec<String>, AppError> {
        let project_dir = self.get_project_directory(client_id, project_id);
        let queries_dir = project_dir.join("queries");

        if !queries_dir.exists() {
            return Ok(Vec::new());
        }

        let mut queries = Vec::new();

        let entries = fs::read_dir(&queries_dir).map_err(|e| {
            AppError::InternalServerError(format!("Failed to read queries directory: {}", e))
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                AppError::InternalServerError(format!("Failed to read directory entry: {}", e))
            })?;

            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("txt") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    queries.push(name.to_string());
                }
            }
        }

        Ok(queries)
    }

    pub fn get_claude_md_content(
        &self,
        client_id: Uuid,
        project_id: &str,
    ) -> Result<String, AppError> {
        let project_dir = self.get_project_directory(client_id, project_id);
        let claude_md_path = project_dir.join("CLAUDE.md");

        if !claude_md_path.exists() {
            return Err(AppError::NotFound(
                "CLAUDE.md not found for this project".to_string(),
            ));
        }

        fs::read_to_string(&claude_md_path)
            .map_err(|e| AppError::InternalServerError(format!("Failed to read CLAUDE.md: {}", e)))
    }

    pub fn save_claude_md_content(
        &self,
        client_id: Uuid,
        project_id: &str,
        content: &str,
    ) -> Result<(), AppError> {
        let project_dir = self.ensure_project_directory(client_id, project_id)?;
        let claude_md_path = project_dir.join("CLAUDE.md");

        fs::write(&claude_md_path, content).map_err(|e| {
            AppError::InternalServerError(format!("Failed to save CLAUDE.md: {}", e))
        })?;

        Ok(())
    }
}
