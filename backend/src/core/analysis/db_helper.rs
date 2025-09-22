use anyhow::Result;
use sqlx::PgPool;

/// Helper functions for database operations with proper error handling
pub struct DatabaseHelper {
    pool: PgPool,
}

impl DatabaseHelper {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Check if the analysis tables exist
    pub async fn analysis_tables_exist(&self) -> bool {
        let result = sqlx::query!(
            r#"
            SELECT EXISTS (
                SELECT FROM information_schema.tables 
                WHERE table_schema = 'public' 
                AND table_name = 'analyses'
            ) as table_exists
            "#
        )
        .fetch_one(&self.pool)
        .await;

        match result {
            Ok(row) => row.table_exists.unwrap_or(false),
            Err(_) => false,
        }
    }

    /// Safe wrapper for analysis operations
    pub async fn with_analysis_tables<F, T>(&self, operation: F) -> Result<Option<T>>
    where
        F: FnOnce(&PgPool) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send + '_>>,
    {
        if !self.analysis_tables_exist().await {
            tracing::warn!("Analysis tables do not exist. Please run migrations first.");
            return Ok(None);
        }

        let result = operation(&self.pool).await?;
        Ok(Some(result))
    }

    /// Initialize analysis tables if they don't exist
    pub async fn ensure_analysis_tables(&self) -> Result<()> {
        if !self.analysis_tables_exist().await {
            tracing::info!("Analysis tables do not exist. Creating basic structure...");
            
            // Create a minimal structure for development
            // In production, this should be done via proper migrations
            let _ = sqlx::query!(
                r#"
                CREATE TABLE IF NOT EXISTS analyses (
                    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                    title VARCHAR NOT NULL,
                    script_content TEXT NOT NULL,
                    project_id UUID NOT NULL,
                    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                    created_by UUID,
                    version INTEGER DEFAULT 1,
                    is_active BOOLEAN DEFAULT TRUE,
                    metadata JSONB DEFAULT '{}'
                );
                "#
            )
            .execute(&self.pool)
            .await;
        }
        
        Ok(())
    }
}

/// Macro to safely execute database operations that depend on analysis tables
#[macro_export]
macro_rules! safe_db_operation {
    ($db_helper:expr, $operation:expr) => {{
        match $db_helper.with_analysis_tables(Box::pin($operation)).await {
            Ok(Some(result)) => Ok(result),
            Ok(None) => Err(anyhow!("Analysis system not initialized. Please run migrations.")),
            Err(e) => Err(e),
        }
    }};
}

/// Safe wrapper for operations that can return empty results when tables don't exist
pub async fn safe_list_operation<T, F>(db_helper: &DatabaseHelper, operation: F) -> Result<Vec<T>>
where
    F: FnOnce(&PgPool) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<T>>> + Send + '_>>,
{
    match db_helper.with_analysis_tables(Box::pin(operation)).await {
        Ok(Some(result)) => Ok(result),
        Ok(None) => {
            tracing::info!("Analysis tables not available, returning empty list");
            Ok(Vec::new())
        }
        Err(e) => Err(e),
    }
}