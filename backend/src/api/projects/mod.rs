// Project management
pub mod crud;
pub mod datasources;
pub mod context;
pub mod members;

use salvo::prelude::*;
use crate::utils::middleware::auth::auth_required;
use crate::utils::middleware::client_scoped;
use crate::api::admin::analysis;

pub fn project_routes() -> Router {
    Router::new()
        .hoop(auth_required)
        .hoop(client_scoped)
        .push(Router::with_path("/projects").get(crud::list_projects).post(crud::create_project))
        .push(Router::with_path("/projects/{project_id}").get(crud::get_project).delete(crud::delete_project))
        .push(Router::with_path("/projects/{project_id}/context")
            .get(context::get_project_context)
            .put(context::update_project_context))
        .push(Router::with_path("/projects/{project_id}/context/compile").post(context::compile_project_context))
        .push(Router::with_path("/projects/{project_id}/context/preview").get(context::preview_project_context))
        .push(Router::with_path("/projects/{project_id}/context/cache").delete(context::clear_context_cache))
        .push(Router::with_path("/projects/{project_id}/queries").get(crud::list_queries).post(crud::save_query))
        .push(Router::with_path("/projects/{project_id}/members")
            .get(members::list_project_members)
            .post(members::add_project_member))
        .push(Router::with_path("/projects/{project_id}/members/{user_id}")
            .delete(members::remove_project_member)
            .patch(members::update_project_member_role))
        .push(Router::with_path("/projects/{project_id}/transfer").post(members::transfer_project_ownership))
        .push(datasources::datasource_routes())
        .push(analysis::configure_analysis_routes())
}
