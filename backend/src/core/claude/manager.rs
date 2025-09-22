use sqlx::Row;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex as StdMutex};
use tokio::sync::mpsc;
use uuid::Uuid;

use super::{
    sdk::ClaudeSDK,
    setup::ClaudeSetup,
    types::{ClaudeMessage, QueryOptions, QueryRequest},
};

#[derive(Debug)]
pub struct ClaudeManager;

static CLIENT_INSTANCES: LazyLock<StdMutex<HashMap<Uuid, Arc<ClaudeSetup>>>> =
    LazyLock::new(|| StdMutex::new(HashMap::new()));

// SDK Instance Manager
#[allow(dead_code)]
static SDK_INSTANCES: LazyLock<StdMutex<HashMap<Uuid, Arc<ClaudeSDK>>>> =
    LazyLock::new(|| StdMutex::new(HashMap::new()));

impl ClaudeManager {
    fn get_or_create_client(client_id: Uuid) -> Arc<ClaudeSetup> {
        let mut clients = match CLIENT_INSTANCES.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                tracing::error!("CLIENT_INSTANCES mutex poisoned, recovering");
                poisoned.into_inner()
            }
        };
        if let Some(client) = clients.get(&client_id) {
            client.clone()
        } else {
            let setup = Arc::new(ClaudeSetup::new(client_id));
            clients.insert(client_id, setup.clone());
            setup
        }
    }

    pub fn get_client_setup(client_id: Uuid) -> Option<Arc<ClaudeSetup>> {
        let clients = match CLIENT_INSTANCES.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                tracing::error!("CLIENT_INSTANCES mutex poisoned, recovering");
                poisoned.into_inner()
            }
        };
        clients.get(&client_id).cloned()
    }

    #[allow(dead_code)]
    pub fn is_input_ready(client_id: Uuid) -> bool {
        let clients = match CLIENT_INSTANCES.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                tracing::error!("CLIENT_INSTANCES mutex poisoned, recovering");
                poisoned.into_inner()
            }
        };
        if let Some(client) = clients.get(&client_id) {
            client.is_input_ready()
        } else {
            false
        }
    }

    pub async fn setup_client(
        client_id: Uuid,
        progress_tx: Option<mpsc::Sender<String>>,
    ) -> Result<ClaudeSetup, Box<dyn std::error::Error + Send + Sync>> {
        let setup = Self::get_or_create_client(client_id);
        setup.setup_environment(progress_tx).await?;
        Ok((*setup).clone())
    }

    pub async fn start_setup_token_stream(
        client_id: Uuid,
        progress_tx: mpsc::Sender<String>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let setup = Self::get_or_create_client(client_id);
        setup.start_setup_token_stream(progress_tx).await
    }

    pub async fn submit_token(
        client_id: Uuid,
        setup_token: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let setup = Self::get_or_create_client(client_id);
        setup.submit_setup_token(setup_token).await
    }

    // SDK Methods for programmatic Claude Code interaction

    #[allow(dead_code)]
    pub fn get_or_create_sdk(client_id: Uuid, oauth_token: Option<String>) -> Arc<ClaudeSDK> {
        let mut sdks = match SDK_INSTANCES.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                tracing::error!("SDK_INSTANCES mutex poisoned, recovering");
                poisoned.into_inner()
            }
        };
        if let Some(sdk) = sdks.get(&client_id) {
            sdk.clone()
        } else {
            let sdk = Arc::new(ClaudeSDK::new(client_id, oauth_token));
            sdks.insert(client_id, sdk.clone());
            sdk
        }
    }

    #[allow(dead_code)]
    pub async fn query_claude(
        client_id: Uuid,
        prompt: String,
        options: Option<QueryOptions>,
    ) -> Result<mpsc::Receiver<ClaudeMessage>, Box<dyn std::error::Error + Send + Sync>> {
        // First check if we have an OAuth token for this client
        let setup = Self::get_client_setup(client_id);
        let oauth_token = if let Some(setup) = setup {
            setup.get_oauth_token().await
        } else {
            None
        };

        if oauth_token.is_none() {
            return Err("Client not authenticated. Please complete setup first.".into());
        }

        let sdk = Self::get_or_create_sdk(client_id, oauth_token);
        let request = QueryRequest { prompt, options };
        sdk.query(request).await
    }

    #[allow(dead_code)]
    pub async fn query_claude_with_project(
        client_id: Uuid,
        project_id: &str,
        prompt: String,
        options: Option<QueryOptions>,
    ) -> Result<mpsc::Receiver<ClaudeMessage>, Box<dyn std::error::Error + Send + Sync>> {
        // First check if we have an OAuth token for this client
        let setup = Self::get_client_setup(client_id);
        let oauth_token = if let Some(setup) = setup {
            setup.get_oauth_token().await
        } else {
            None
        };

        if oauth_token.is_none() {
            return Err("Client not authenticated. Please complete setup first.".into());
        }

        // Create SDK with project directory
        let sdk = ClaudeSDK::new(client_id, oauth_token).with_project(project_id);
        let request = QueryRequest { prompt, options };
        sdk.query(request).await
    }

    pub async fn query_claude_with_project_and_db(
        client_id: Uuid,
        project_id: &str,
        prompt: String,
        options: Option<QueryOptions>,
        db_pool: &sqlx::PgPool,
    ) -> Result<mpsc::Receiver<ClaudeMessage>, Box<dyn std::error::Error + Send + Sync>> {
        tracing::info!(
            "ClaudeManager::query_claude_with_project_and_db - checking OAuth token for client: {}",
            client_id
        );

        // First check if we have an OAuth token for this client
        let setup = Self::get_client_setup(client_id);
        let oauth_token = if let Some(setup) = setup {
            tracing::info!("Found existing setup, getting token with DB");
            setup.get_oauth_token_with_db(db_pool).await
        } else {
            tracing::info!("No existing setup, checking database directly");
            // If no setup exists, try to get token directly from database
            if let Ok(row) = sqlx::query("SELECT claude_token FROM clients WHERE id = $1")
                .bind(client_id)
                .fetch_optional(db_pool)
                .await
            {
                if let Some(row) = row {
                    let token = row.get::<Option<String>, _>("claude_token");
                    tracing::info!("Found token in database: {}", token.is_some());
                    token
                } else {
                    tracing::info!("No client found in database");
                    None
                }
            } else {
                tracing::error!("Database query failed for client token");
                None
            }
        };

        if oauth_token.is_none() {
            tracing::error!("No OAuth token available for client: {}", client_id);
            return Err("Client not authenticated. Please complete setup first.".into());
        }

        tracing::info!("OAuth token found, creating SDK and querying Claude");
        // Create SDK with project directory
        let sdk = ClaudeSDK::new(client_id, oauth_token).with_project(project_id);
        let request = QueryRequest { prompt, options };
        tracing::info!("About to call sdk.query()");
        let result = sdk.query(request).await;
        tracing::info!("SDK query completed with result: {}", result.is_ok());
        result
    }

    pub async fn query_claude_with_project_and_token(
        client_id: Uuid,
        project_id: &str,
        prompt: String,
        options: Option<QueryOptions>,
        oauth_token: Option<String>,
    ) -> Result<mpsc::Receiver<ClaudeMessage>, Box<dyn std::error::Error + Send + Sync>> {
        if oauth_token.is_none() {
            return Err("Client not authenticated. Please complete setup first.".into());
        }

        // Create SDK with project directory
        let sdk = ClaudeSDK::new(client_id, oauth_token).with_project(project_id);
        let request = QueryRequest { prompt, options };
        sdk.query(request).await
    }

    #[allow(dead_code)]
    pub async fn query_claude_simple(
        client_id: Uuid,
        prompt: String,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let mut receiver = Self::query_claude(client_id, prompt, None).await?;
        let mut result = String::new();

        while let Some(message) = receiver.recv().await {
            match message {
                ClaudeMessage::Result { result: r } => {
                    result = r;
                    break;
                }
                ClaudeMessage::Error { error } => {
                    return Err(error.into());
                }
                _ => continue,
            }
        }

        Ok(result)
    }

    #[allow(dead_code)]
    pub async fn update_sdk_token(client_id: Uuid, oauth_token: String) {
        let sdk = Self::get_or_create_sdk(client_id, Some(oauth_token.clone()));
        sdk.set_oauth_token(oauth_token).await;
    }
}
