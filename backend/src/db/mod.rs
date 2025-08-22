use sea_orm::{Database, DatabaseConnection};
use sea_orm_migration::prelude::*;
use tracing::debug;

pub async fn connect(database_url: &str) -> Result<DatabaseConnection, sea_orm::DbErr> {
    debug!("Connecting to database...");
    let db = Database::connect(database_url).await?;
    debug!("Database connected successfully");
    
    // Run migrations
    debug!("Running database migrations...");
    migration::Migrator::up(&db, None).await?;
    debug!("Migrations completed successfully");
    
    Ok(db)
}