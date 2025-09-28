// Project management
pub mod crud;
pub mod shares;
pub mod datasources;
pub mod context;

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
        .push(datasources::datasource_routes())
        .push(shares::share_routes())
        .push(analysis::configure_analysis_routes())
}
