// Admin functionality
pub mod crud;
pub mod debug;
pub mod analysis;

use salvo::prelude::*;

pub fn admin_routes() -> Router {
    Router::new()
        .push(Router::with_path("/debug/connections").get(debug::get_active_connections))
}