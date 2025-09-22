// Authentication and user management
pub mod handlers;
pub mod user_management;
pub mod client_management;
pub mod clients;

use salvo::prelude::*;

pub fn auth_routes() -> Router {
    Router::new()
        .push(handlers::auth_routes())
        .push(clients::client_routes())
}