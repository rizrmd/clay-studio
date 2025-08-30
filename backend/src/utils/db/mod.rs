use sea_orm::{Database, DatabaseConnection, ConnectOptions};
use sea_orm_migration::prelude::*;
use tracing::{info, warn, error};
use std::time::Duration;

pub async fn connect(database_url: &str) -> Result<DatabaseConnection, sea_orm::DbErr> {
    info!("🔌 Initiating database connection to: {}", mask_database_url(database_url));
    
    let mut opt = ConnectOptions::new(database_url.to_owned());
    
    // Configure connection pool with logging
    opt.max_connections(10)
        .min_connections(2)
        .connect_timeout(Duration::from_secs(30))
        .idle_timeout(Duration::from_secs(300))
        .max_lifetime(Duration::from_secs(3600))
        .sqlx_logging(true);
    
    info!("📊 Database connection pool configuration:");
    info!("  - Max connections: 10");
    info!("  - Min connections: 2");
    info!("  - Connect timeout: 30s");
    info!("  - Idle timeout: 300s");
    info!("  - Max lifetime: 3600s");
    
    // Attempt connection with retry logic
    let mut attempts = 0;
    const MAX_ATTEMPTS: u8 = 3;
    
    let db = loop {
        attempts += 1;
        info!("🔄 Database connection attempt {}/{}", attempts, MAX_ATTEMPTS);
        
        match Database::connect(opt.clone()).await {
            Ok(db) => {
                info!("✅ Database connection established successfully");
                break db;
            }
            Err(e) => {
                error!("❌ Database connection attempt {} failed: {}", attempts, e);
                
                // Log specific error diagnostics
                let error_msg = e.to_string();
                if error_msg.contains("Connection refused") {
                    error!("🚨 Database server appears to be down or unreachable");
                    error!("💡 Check if the database server is running");
                } else if error_msg.contains("authentication") || error_msg.contains("password") {
                    error!("🚨 Database authentication failed");
                    error!("💡 Check database credentials and permissions");
                } else if error_msg.contains("database") && error_msg.contains("does not exist") {
                    error!("🚨 Target database does not exist");
                    error!("💡 Create the database first");
                } else if error_msg.contains("timeout") {
                    error!("🚨 Database connection timeout");
                    error!("💡 Check network connectivity and database performance");
                } else if error_msg.contains("too many connections") {
                    error!("🚨 Database has too many active connections");
                    error!("💡 Close unused connections or increase max_connections");
                }
                
                if attempts >= MAX_ATTEMPTS {
                    error!("💥 All database connection attempts exhausted");
                    return Err(e);
                }
                
                warn!("⏳ Retrying database connection in 2 seconds...");
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    };
    
    // Test the connection - SeaORM connections are tested during creation
    info!("✅ Database connection test successful");
    
    // Run migrations with proper logging
    info!("🔄 Running database migrations...");
    match migration::Migrator::up(&db, None).await {
        Ok(_) => info!("✅ Database migrations completed successfully"),
        Err(e) => {
            error!("❌ Database migration failed: {}", e);
            return Err(e);
        }
    }
    
    info!("🎉 Database initialization completed successfully");
    Ok(db)
}

/// Mask sensitive information in database URL for logging
fn mask_database_url(url: &str) -> String {
    if let Some(at_pos) = url.rfind('@') {
        if let Some(colon_pos) = url[..at_pos].rfind(':') {
            if let Some(scheme_pos) = url[..colon_pos].rfind("://") {
                let scheme_end = scheme_pos + 3;
                let username_part = &url[scheme_end..colon_pos];
                let after_at = &url[at_pos..];
                return format!("{}{}:****{}", &url[..scheme_end], username_part, after_at);
            }
        }
    }
    url.to_string()
}