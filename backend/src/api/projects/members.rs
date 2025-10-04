use crate::models::*;
use crate::utils::middleware::{get_current_user_id, is_current_user_root};
use crate::utils::{get_app_state, AppError};
use salvo::prelude::*;
use serde::Serialize;
use sqlx::Row;
use uuid::Uuid;

/// Get all members of a project
#[handler]
pub async fn list_project_members(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let project_id = req
        .param::<String>("project_id")
        .ok_or(AppError::BadRequest("Missing project_id".to_string()))?;

    let current_user_id = get_current_user_id(depot)?;

    // Check if user is a member of the project (or is root)
    let is_member = if is_current_user_root(depot) {
        true
    } else {
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM project_members WHERE project_id = $1 AND user_id = $2)",
        )
        .bind(&project_id)
        .bind(current_user_id)
        .fetch_one(&state.db_pool)
        .await
        .unwrap_or(false)
    };

    if !is_member {
        return Err(AppError::Forbidden(
            "You don't have access to this project".to_string(),
        ));
    }

    // Fetch all project members with user information
    let member_rows = sqlx::query(
        "SELECT pm.id, pm.project_id, pm.user_id, pm.role, pm.joined_at, u.username
         FROM project_members pm
         JOIN users u ON pm.user_id = u.id
         WHERE pm.project_id = $1
         ORDER BY pm.role DESC, pm.joined_at ASC",
    )
    .bind(&project_id)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Database error: {}", e)))?;

    let mut members = Vec::new();
    for row in member_rows {
        let role_str: String = row.get("role");
        let role = ProjectMemberRole::from_str(&role_str)
            .map_err(|e| AppError::InternalServerError(format!("Invalid role: {}", e)))?;

        members.push(ProjectMemberWithUser {
            id: row.get("id"),
            project_id: row.get("project_id"),
            user_id: row.get("user_id"),
            username: row.get("username"),
            role,
            joined_at: row.get("joined_at"),
        });
    }

    res.render(Json(members));
    Ok(())
}

/// Add a member to a project (owner only)
#[handler]
pub async fn add_project_member(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let project_id = req
        .param::<String>("project_id")
        .ok_or(AppError::BadRequest("Missing project_id".to_string()))?;

    let add_req: AddProjectMemberRequest = req
        .parse_json()
        .await
        .map_err(|_| AppError::BadRequest("Invalid request body".to_string()))?;

    let current_user_id = get_current_user_id(depot)?;

    // Check if current user is an owner (or root)
    let is_owner = if is_current_user_root(depot) {
        true
    } else {
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM project_members WHERE project_id = $1 AND user_id = $2 AND role = 'owner')",
        )
        .bind(&project_id)
        .bind(current_user_id)
        .fetch_one(&state.db_pool)
        .await
        .unwrap_or(false)
    };

    if !is_owner {
        return Err(AppError::Forbidden(
            "Only project owners can add members".to_string(),
        ));
    }

    // Check if user exists
    let user_exists = sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM users WHERE id = $1)")
        .bind(add_req.user_id)
        .fetch_one(&state.db_pool)
        .await
        .unwrap_or(false);

    if !user_exists {
        return Err(AppError::NotFound("User not found".to_string()));
    }

    // Check if user is already a member
    let already_member = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM project_members WHERE project_id = $1 AND user_id = $2)",
    )
    .bind(&project_id)
    .bind(add_req.user_id)
    .fetch_one(&state.db_pool)
    .await
    .unwrap_or(false);

    if already_member {
        return Err(AppError::BadRequest(
            "User is already a member of this project".to_string(),
        ));
    }

    // Add the member
    let member_row = sqlx::query(
        "INSERT INTO project_members (project_id, user_id, role) VALUES ($1, $2, $3)
         RETURNING id, project_id, user_id, role, joined_at, created_at",
    )
    .bind(&project_id)
    .bind(add_req.user_id)
    .bind(add_req.role.as_str())
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Failed to add member: {}", e)))?;

    let member = ProjectMember {
        id: member_row.get("id"),
        project_id: member_row.get("project_id"),
        user_id: member_row.get("user_id"),
        role: ProjectMemberRole::from_str(&member_row.get::<String, _>("role"))
            .map_err(|e| AppError::InternalServerError(e))?,
        joined_at: member_row.get("joined_at"),
        created_at: member_row.get("created_at"),
    };

    res.render(Json(member));
    Ok(())
}

/// Remove a member from a project (owner only)
#[handler]
pub async fn remove_project_member(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let project_id = req
        .param::<String>("project_id")
        .ok_or(AppError::BadRequest("Missing project_id".to_string()))?;
    let user_id = req
        .param::<Uuid>("user_id")
        .ok_or(AppError::BadRequest("Missing user_id".to_string()))?;

    let current_user_id = get_current_user_id(depot)?;

    // Check if current user is an owner (or root)
    let is_owner = if is_current_user_root(depot) {
        true
    } else {
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM project_members WHERE project_id = $1 AND user_id = $2 AND role = 'owner')",
        )
        .bind(&project_id)
        .bind(current_user_id)
        .fetch_one(&state.db_pool)
        .await
        .unwrap_or(false)
    };

    if !is_owner {
        return Err(AppError::Forbidden(
            "Only project owners can remove members".to_string(),
        ));
    }

    // Check if the member to remove is the last owner
    let owner_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM project_members WHERE project_id = $1 AND role = 'owner'",
    )
    .bind(&project_id)
    .fetch_one(&state.db_pool)
    .await
    .unwrap_or(0);

    let is_owner_to_remove = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM project_members WHERE project_id = $1 AND user_id = $2 AND role = 'owner')",
    )
    .bind(&project_id)
    .bind(user_id)
    .fetch_one(&state.db_pool)
    .await
    .unwrap_or(false);

    if is_owner_to_remove && owner_count <= 1 {
        return Err(AppError::BadRequest(
            "Cannot remove the last owner. Transfer ownership first.".to_string(),
        ));
    }

    // Get conversations owned by the user being removed
    let owned_conversations = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM conversations WHERE project_id = $1 AND created_by_user_id = $2",
    )
    .bind(&project_id)
    .bind(user_id)
    .fetch_one(&state.db_pool)
    .await
    .unwrap_or(0);

    if owned_conversations > 0 {
        return Err(AppError::BadRequest(
            format!("User has {} conversation(s) in this project. Transfer or delete them first.", owned_conversations)
        ));
    }

    // Remove the member
    let result = sqlx::query("DELETE FROM project_members WHERE project_id = $1 AND user_id = $2")
        .bind(&project_id)
        .bind(user_id)
        .execute(&state.db_pool)
        .await
        .map_err(|e| AppError::InternalServerError(format!("Failed to remove member: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Member not found in project".to_string()));
    }

    #[derive(Serialize)]
    struct RemoveMemberResponse {
        message: String,
        user_id: Uuid,
    }

    res.render(Json(RemoveMemberResponse {
        message: "Member removed successfully".to_string(),
        user_id,
    }));
    Ok(())
}

/// Transfer project ownership to another member
#[handler]
pub async fn transfer_project_ownership(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let project_id = req
        .param::<String>("project_id")
        .ok_or(AppError::BadRequest("Missing project_id".to_string()))?;

    let transfer_req: TransferOwnershipRequest = req
        .parse_json()
        .await
        .map_err(|_| AppError::BadRequest("Invalid request body".to_string()))?;

    let current_user_id = get_current_user_id(depot)?;

    // Check if current user is an owner (or root)
    let is_owner = if is_current_user_root(depot) {
        true
    } else {
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM project_members WHERE project_id = $1 AND user_id = $2 AND role = 'owner')",
        )
        .bind(&project_id)
        .bind(current_user_id)
        .fetch_one(&state.db_pool)
        .await
        .unwrap_or(false)
    };

    if !is_owner {
        return Err(AppError::Forbidden(
            "Only project owners can transfer ownership".to_string(),
        ));
    }

    // Check if new owner is a member
    let is_member = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM project_members WHERE project_id = $1 AND user_id = $2)",
    )
    .bind(&project_id)
    .bind(transfer_req.new_owner_user_id)
    .fetch_one(&state.db_pool)
    .await
    .unwrap_or(false);

    if !is_member {
        return Err(AppError::BadRequest(
            "New owner must be a member of the project".to_string(),
        ));
    }

    // Update the new owner's role
    sqlx::query("UPDATE project_members SET role = 'owner' WHERE project_id = $1 AND user_id = $2")
        .bind(&project_id)
        .bind(transfer_req.new_owner_user_id)
        .execute(&state.db_pool)
        .await
        .map_err(|e| {
            AppError::InternalServerError(format!("Failed to update new owner: {}", e))
        })?;

    // Optionally demote current owner to member (if not root and not the same user)
    if !is_current_user_root(depot) && current_user_id != transfer_req.new_owner_user_id {
        sqlx::query(
            "UPDATE project_members SET role = 'member' WHERE project_id = $1 AND user_id = $2",
        )
        .bind(&project_id)
        .bind(current_user_id)
        .execute(&state.db_pool)
        .await
        .map_err(|e| {
            AppError::InternalServerError(format!("Failed to update current owner: {}", e))
        })?;
    }

    #[derive(Serialize)]
    struct TransferOwnershipResponse {
        message: String,
        new_owner_user_id: Uuid,
    }

    res.render(Json(TransferOwnershipResponse {
        message: "Ownership transferred successfully".to_string(),
        new_owner_user_id: transfer_req.new_owner_user_id,
    }));
    Ok(())
}

/// Update member role
#[handler]
pub async fn update_project_member_role(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), AppError> {
    let state = get_app_state(depot)?;
    let project_id = req
        .param::<String>("project_id")
        .ok_or(AppError::BadRequest("Missing project_id".to_string()))?;
    let user_id = req
        .param::<Uuid>("user_id")
        .ok_or(AppError::BadRequest("Missing user_id".to_string()))?;

    let update_req: UpdateProjectMemberRequest = req
        .parse_json()
        .await
        .map_err(|_| AppError::BadRequest("Invalid request body".to_string()))?;

    let current_user_id = get_current_user_id(depot)?;

    // Check if current user is an owner (or root)
    let is_owner = if is_current_user_root(depot) {
        true
    } else {
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM project_members WHERE project_id = $1 AND user_id = $2 AND role = 'owner')",
        )
        .bind(&project_id)
        .bind(current_user_id)
        .fetch_one(&state.db_pool)
        .await
        .unwrap_or(false)
    };

    if !is_owner {
        return Err(AppError::Forbidden(
            "Only project owners can update member roles".to_string(),
        ));
    }

    // If demoting an owner, check if there's at least one other owner
    if update_req.role == ProjectMemberRole::Member {
        let is_currently_owner = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM project_members WHERE project_id = $1 AND user_id = $2 AND role = 'owner')",
        )
        .bind(&project_id)
        .bind(user_id)
        .fetch_one(&state.db_pool)
        .await
        .unwrap_or(false);

        if is_currently_owner {
            let owner_count = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM project_members WHERE project_id = $1 AND role = 'owner'",
            )
            .bind(&project_id)
            .fetch_one(&state.db_pool)
            .await
            .unwrap_or(0);

            if owner_count <= 1 {
                return Err(AppError::BadRequest(
                    "Cannot demote the last owner. Transfer ownership first.".to_string(),
                ));
            }
        }
    }

    // Update the role
    let result = sqlx::query(
        "UPDATE project_members SET role = $1 WHERE project_id = $2 AND user_id = $3",
    )
    .bind(update_req.role.as_str())
    .bind(&project_id)
    .bind(user_id)
    .execute(&state.db_pool)
    .await
    .map_err(|e| AppError::InternalServerError(format!("Failed to update role: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Member not found in project".to_string()));
    }

    #[derive(Serialize)]
    struct UpdateRoleResponse {
        message: String,
        user_id: Uuid,
        new_role: String,
    }

    res.render(Json(UpdateRoleResponse {
        message: "Member role updated successfully".to_string(),
        user_id,
        new_role: update_req.role.as_str().to_string(),
    }));
    Ok(())
}