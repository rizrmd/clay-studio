/*!
# Analysis Sandbox - Usage Examples

This module provides examples of how to use the Analysis Sandbox system.

## Basic Setup

```rust
use clay_studio_backend::core::analysis::*;
use sqlx::PgPool;
use std::path::PathBuf;

async fn setup_analysis_system(db: PgPool) -> Result<AnalysisService> {
    let data_dir = PathBuf::from("./data");
    let service = AnalysisService::new(db, data_dir).await?;
    service.start().await?;
    Ok(service)
}
```

## Creating an Analysis

```rust
async fn create_sample_analysis(manager: &AnalysisManager) -> Result<()> {
    let script = r#"
    export default {
        title: "Daily Sales Report",
        
        dependencies: {
            datasources: ["postgres_main"],
            analyses: []
        },
        
        parameters: {
            date: {
                type: "date",
                required: true,
                default: "yesterday"
            }
        },
        
        schedule: {
            cron: "0 9 * * *",  // Daily at 9 AM
            timezone: "UTC",
            enabled: true
        },
        
        run: async function(ctx, params) {
            const sales = await ctx.datasource.postgres_main.query(
                "SELECT SUM(amount) as total FROM sales WHERE date = $1",
                [params.date]
            );
            
            return {
                date: params.date,
                total_sales: sales[0].total,
                generated_at: new Date().toISOString()
            };
        }
    }
    "#;

    let request = CreateAnalysisRequest {
        title: "Daily Sales Report".to_string(),
        script_content: script.to_string(),
        project_id: uuid::Uuid::new_v4(),
    };

    let analysis = manager.create_analysis(request).await?;
    println!("Created analysis: {}", analysis.id);
    Ok(())
}
```

## Executing an Analysis

```rust
async fn execute_analysis_example(manager: &AnalysisManager, analysis_id: Uuid) -> Result<()> {
    let parameters = serde_json::json!({
        "date": "2024-01-15"
    });

    let job_id = manager.execute_analysis(analysis_id, parameters).await?;
    println!("Started job: {}", job_id);

    // Monitor job status
    loop {
        let job = manager.get_job_status(job_id).await?;
        match job.status {
            JobStatus::Completed => {
                println!("Job completed: {:?}", job.result);
                break;
            }
            JobStatus::Failed => {
                println!("Job failed: {:?}", job.error_message);
                break;
            }
            _ => {
                println!("Job status: {:?}", job.status);
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        }
    }

    Ok(())
}
```

## Setting up Scheduling

```rust,ignore
async fn setup_analysis_schedule(manager: &AnalysisManager, analysis_id: Uuid) -> Result<()> {
    let schedule = ScheduleConfig {
        cron: "0 */4 * * *".to_string(), // Every 4 hours
        timezone: Some("UTC".to_string()),
        enabled: true,
    };

    manager.set_schedule(analysis_id, schedule).await?;
    println!("Schedule configured for analysis: {}", analysis_id);
    Ok(())
}
```

## Working with DuckDB

```rust,ignore
async fn duckdb_example(duckdb_manager: &DuckDBManager, project_id: &str) -> Result<()> {
    let db_path = duckdb_manager.get_database_path(project_id).await;
    
    // Create a table
    duckdb_manager.execute_query(&db_path, 
        "CREATE TABLE sales_summary AS SELECT * FROM read_csv_auto(''sales.csv'')"
    ).await?;
    
    // Query data
    let results = duckdb_manager.execute_query(&db_path,
        "SELECT category, SUM(amount) as total FROM sales_summary GROUP BY category"
    ).await?;
    
    println!("Query results: {}", results);
    
    // List tables
    let tables = duckdb_manager.list_tables(&db_path).await?;
    println!("Available tables: {:?}", tables);
    
    Ok(())
}
```

## Complete Integration Example

```rust
use clay_studio_backend::core::analysis::*;

async fn complete_example() -> Result<()> {
    // 1. Setup database connection
    let database_url = std::env::var("DATABASE_URL")
        .map_err(|_| anyhow::anyhow!("DATABASE_URL environment variable must be set"))?;
    let pool = sqlx::postgres::PgPool::connect(&database_url).await?;
    
    // 2. Initialize analysis system
    let data_dir = PathBuf::from("./analysis_data");
    let service = AnalysisService::new(pool, data_dir).await?;
    service.start().await?;
    
    // 3. Create an analysis
    let analysis_request = CreateAnalysisRequest {
        title: "Customer Analysis".to_string(),
        script_content: include_str!("../../examples/customer_analysis.js").to_string(),
        project_id: uuid::Uuid::new_v4(),
    };
    
    let analysis = service.analysis_manager.create_analysis(analysis_request).await?;
    
    // 4. Execute the analysis
    let job_id = service.analysis_manager.execute_analysis(
        analysis.id,
        serde_json::json!({
            "start_date": "2024-01-01",
            "end_date": "2024-01-31"
        })
    ).await?;
    
    // 5. Wait for completion
    let job = service.analysis_manager.get_job_status(job_id).await?;
    println!("Analysis result: {:?}", job.result);
    
    // 6. Setup scheduling for future runs
    service.analysis_manager.set_schedule(
        analysis.id,
        ScheduleConfig {
            cron: "0 6 1 * *".to_string(), // Monthly at 6 AM
            timezone: Some("UTC".to_string()),
            enabled: true,
        }
    ).await?;
    
    // 7. System stats
    let stats = service.get_system_stats().await?;
    println!("System stats: {:?}", stats);
    
    Ok(())
}
```

## Error Handling Best Practices

```rust
async fn robust_analysis_execution(
    manager: &AnalysisManager, 
    analysis_id: Uuid
) -> Result<serde_json::Value> {
    // Validate analysis exists
    let analysis = manager.get_analysis(analysis_id).await
        .map_err(|e| anyhow!("Failed to get analysis {}: {}", analysis_id, e))?;
    
    // Execute with timeout
    let job_id = manager.execute_analysis(analysis_id, serde_json::json!({})).await?;
    
    // Poll with timeout
    let timeout = tokio::time::Duration::from_secs(300); // 5 minutes
    let start = tokio::time::Instant::now();
    
    loop {
        if start.elapsed() > timeout {
            manager.cancel_job(job_id).await?;
            return Err(anyhow!("Analysis execution timed out"));
        }
        
        let job = manager.get_job_status(job_id).await?;
        match job.status {
            JobStatus::Completed => {
                return Ok(job.result.unwrap_or_else(|| serde_json::json!({})));
            }
            JobStatus::Failed => {
                return Err(anyhow!("Analysis failed: {}", 
                    job.error_message.unwrap_or_else(|| "Unknown error".to_string())));
            }
            JobStatus::Cancelled => {
                return Err(anyhow!("Analysis was cancelled"));
            }
            _ => {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
        }
    }
}
```

## Performance Monitoring

```rust
async fn monitor_system_performance(service: &AnalysisService) -> Result<()> {
    let stats = service.get_system_stats().await?;
    
    println!("Running jobs: {}", stats.running_jobs_count);
    println!("Total results: {}", stats.total_results);
    println!("Storage size: {} MB", stats.storage_size_bytes / 1_048_576);
    
    // Cleanup old results (older than 30 days)
    let cleaned = service.cleanup_old_results(30).await?;
    println!("Cleaned up {} old results", cleaned);
    
    Ok(())
}
```
*/

use anyhow::Result;
use uuid::Uuid;
use crate::models::analysis::*;
use super::*;

// Re-export for documentation examples
pub use anyhow;
pub use serde_json;
pub use uuid;

/// Example analysis script for customer analysis
pub const CUSTOMER_ANALYSIS_SCRIPT: &str = r#"
export default {
    title: "Customer Analysis Report",
    
    dependencies: {
        datasources: ["postgres_main"],
        analyses: []
    },
    
    parameters: {
        start_date: {
            type: "date",
            required: true,
            description: "Analysis start date"
        },
        end_date: {
            type: "date", 
            required: true,
            description: "Analysis end date"
        },
        customer_segment: {
            type: "select",
            required: false,
            default: "all",
            options: [
                { value: "all", label: "All Customers" },
                { value: "premium", label: "Premium Customers" },
                { value: "standard", label: "Standard Customers" }
            ]
        }
    },
    
    run: async function(ctx, params) {
        // Query customer data
        const customers = await ctx.datasource.postgres_main.query(`
            SELECT 
                customer_id,
                customer_segment,
                total_orders,
                total_spent
            FROM customers 
            WHERE created_at BETWEEN $1 AND $2
            ${params.customer_segment !== 'all' ? 'AND customer_segment = $3' : ''}
        `, params.customer_segment !== 'all' ? 
            [params.start_date, params.end_date, params.customer_segment] :
            [params.start_date, params.end_date]
        );
        
        // Calculate metrics
        const totalCustomers = customers.length;
        const totalRevenue = customers.reduce((sum, c) => sum + c.total_spent, 0);
        const avgOrderValue = totalRevenue / customers.reduce((sum, c) => sum + c.total_orders, 0);
        
        // Store results in DuckDB for further analysis
        await ctx.duckdb.saveDataFrame(
            ctx.DataFrame(customers), 
            `customer_analysis_${params.start_date.replace(/-/g, '_')}`
        );
        
        return {
            period: {
                start: params.start_date,
                end: params.end_date
            },
            segment: params.customer_segment,
            metrics: {
                total_customers: totalCustomers,
                total_revenue: totalRevenue,
                average_order_value: avgOrderValue
            },
            generated_at: new Date().toISOString()
        };
    }
}
"#;

/// Example analysis script demonstrating file operations
pub const FILE_ANALYSIS_SCRIPT: &str = include_str!("file_analysis_example.js");