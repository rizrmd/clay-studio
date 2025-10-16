// Module organization for datasources API
pub mod types;
pub mod crud;
pub mod connection;
pub mod schema;
pub mod query;
pub mod mutations;
pub mod upload;

use salvo::prelude::*;

// Re-export all public types

// Re-export all handler functions

pub fn datasource_routes() -> Router {
    Router::new()
        // Project-scoped routes
        .push(Router::with_path("/projects/{project_id}/datasources").get(crud::list_datasources).post(crud::create_datasource))
        // File upload routes
        .push(Router::with_path("/projects/{project_id}/datasources/upload").post(upload::upload_file_datasource))
        .push(Router::with_path("/projects/{project_id}/datasources/preview").post(upload::preview_file))
        // Datasource-specific routes
        .push(Router::with_path("/datasources/{datasource_id}").put(crud::update_datasource).delete(crud::delete_datasource))
        .push(Router::with_path("/datasources/{datasource_id}/test").post(connection::test_connection))
        .push(Router::with_path("/datasources/{datasource_id}/schema").get(schema::get_schema))
        // Data browser routes
        .push(Router::with_path("/datasources/{datasource_id}/query").post(query::execute_query))
        .push(Router::with_path("/datasources/{datasource_id}/tables").get(schema::get_tables))
        .push(Router::with_path("/datasources/{datasource_id}/tables/{table_name}/data").post(query::get_table_data))
        .push(Router::with_path("/datasources/{datasource_id}/tables/{table_name}/structure").get(schema::get_table_structure))
        .push(Router::with_path("/datasources/{datasource_id}/tables/{table_name}/distinct").post(query::get_distinct_values))
        .push(Router::with_path("/datasources/{datasource_id}/tables/{table_name}/row-ids").post(query::get_table_row_ids))
        .push(Router::with_path("/datasources/{datasource_id}/tables/{table_name}/rows").delete(mutations::delete_rows).put(mutations::update_rows).post(mutations::insert_rows))
        // Test arbitrary config
        .push(Router::with_path("/test-connection").post(connection::test_connection_with_config))
}




