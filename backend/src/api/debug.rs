use crate::api::websocket::WS_CONNECTIONS;
use salvo::prelude::*;
use serde_json::json;

#[handler]
pub async fn get_active_connections(res: &mut Response) -> Result<(), salvo::Error> {
    let connections = WS_CONNECTIONS.read().await;

    let mut connection_info = Vec::new();
    for (connection_id, conn) in connections.iter() {
        connection_info.push(json!({
            "connection_id": connection_id,
            "user_id": conn.user_id,
            "project_id": conn.project_id,
            "conversation_id": conn.conversation_id,
        }));
    }

    res.render(Json(json!({
        "total_connections": connection_info.len(),
        "connections": connection_info
    })));

    Ok(())
}
