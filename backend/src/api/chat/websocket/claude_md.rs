use crate::utils::AppState;
use sqlx::Row;

/// Update CLAUDE.md template if it's outdated or missing the latest template features
pub async fn update_claude_md_if_needed(
    state: &AppState,
    project_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use crate::core::projects::manager::ProjectManager;
    use crate::utils::claude_md_template;
    use std::fs;

    // Get project information and client_id
    let project_row = sqlx::query(
        "SELECT name, client_id FROM projects WHERE id = $1"
    )
    .bind(project_id)
    .fetch_optional(&state.db_pool)
    .await?;

    let (project_name, client_id) = match project_row {
        Some(row) => {
            let name: String = row.get("name");
            let client_uuid: uuid::Uuid = row.get("client_id");
            (name, client_uuid)
        }
        None => {
            tracing::warn!("Project {} not found, skipping CLAUDE.md update", project_id);
            return Ok(());
        }
    };

    // Check if CLAUDE.md file exists and contains the latest template features
    let claude_md_path = format!(".clients/{}/{}/CLAUDE.md", client_id, project_id);
    
    let should_update = match fs::read_to_string(&claude_md_path) {
        Ok(content) => {
            let has_validation = content.contains("MCP Interaction Parameter Validation");
            let has_breaking_change = content.contains("BREAKING CHANGE: show_table Parameter Format");
            let needs_update = !has_validation || !has_breaking_change;
            
            tracing::debug!(
                "CLAUDE.md check for project {}: path={}, has_validation={}, has_breaking_change={}, needs_update={}",
                project_id, claude_md_path, has_validation, has_breaking_change, needs_update
            );
            
            needs_update
        }
        Err(e) => {
            tracing::debug!("CLAUDE.md file not readable for project {}: {}, will create/update", project_id, e);
            true // File doesn't exist or unreadable, create/update it
        }
    };

    if should_update {
        tracing::info!("Updating CLAUDE.md template for project {} ({})", project_name, project_id);

        // Get datasources for this project (exclude soft deleted ones)
        let datasources = sqlx::query(
            "SELECT id, name, source_type, connection_config, schema_info, is_active 
             FROM data_sources WHERE project_id = $1 AND deleted_at IS NULL"
        )
        .bind(project_id)
        .fetch_all(&state.db_pool)
        .await?;

        let datasource_values: Vec<serde_json::Value> = datasources
            .into_iter()
            .map(|row| {
                serde_json::json!({
                    "id": row.get::<String, _>("id"),
                    "name": row.get::<String, _>("name"),
                    "source_type": row.get::<String, _>("source_type"),
                    "connection_config": row.get::<serde_json::Value, _>("connection_config"),
                    "schema_info": row.get::<Option<serde_json::Value>, _>("schema_info"),
                    "is_active": row.get::<bool, _>("is_active"),
                })
            })
            .collect();

        // Generate updated CLAUDE.md content
        let claude_md_content = if !datasource_values.is_empty() {
            claude_md_template::generate_claude_md_with_datasources(
                project_id,
                &project_name,
                datasource_values,
            )
            .await
        } else {
            claude_md_template::generate_claude_md(project_id, &project_name)
        };

        // Save the updated content
        let project_manager = ProjectManager::new();
        project_manager.save_claude_md_content(client_id, project_id, &claude_md_content)?;

        tracing::info!(
            "Successfully updated CLAUDE.md template for project {} ({})",
            project_name,
            project_id
        );
    } else {
        tracing::debug!(
            "CLAUDE.md template for project {} ({}) is up to date",
            project_name,
            project_id
        );
    }

    Ok(())
}