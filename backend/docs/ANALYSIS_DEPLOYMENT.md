# Analysis System Deployment Guide

This guide covers deploying the Analysis Sandbox system in production environments.

## Prerequisites

### System Requirements
- **CPU**: 4+ cores recommended for concurrent analysis execution
- **Memory**: 8GB+ RAM (analyses can be memory-intensive)
- **Storage**: SSD recommended for DuckDB performance
- **Network**: Stable internet for DuckDB executable download

### Software Dependencies
- **PostgreSQL 14+**: Main database for analysis metadata
- **Rust 1.70+**: For compilation
- **Operating System**: Linux, macOS, or Windows

## Environment Setup

### 1. Database Configuration

Create the analysis database and user:

```sql
-- Create database
CREATE DATABASE clay_studio_analysis;

-- Create user with appropriate permissions
CREATE USER analysis_user WITH PASSWORD 'secure_password_here';
GRANT ALL PRIVILEGES ON DATABASE clay_studio_analysis TO analysis_user;

-- Connect to the database and create extensions if needed
\c clay_studio_analysis
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
```

### 2. Environment Variables

Set the following environment variables:

```bash
# Database connection
export DATABASE_URL="postgres://analysis_user:secure_password_here@localhost:5432/clay_studio_analysis"

# Analysis system configuration
export ANALYSIS_DATA_DIR="/opt/clay-studio/analysis-data"
export ANALYSIS_MAX_CONCURRENT_JOBS="10"
export ANALYSIS_JOB_TIMEOUT_SECONDS="1800"  # 30 minutes
export ANALYSIS_RESULT_RETENTION_DAYS="90"

# Security settings
export ANALYSIS_SANDBOX_MEMORY_LIMIT="256MB"
export ANALYSIS_SANDBOX_TIMEOUT_SECONDS="300"  # 5 minutes

# Monitoring (optional)
export ANALYSIS_METRICS_ENABLED="true"
export ANALYSIS_METRICS_INTERVAL_SECONDS="30"
```

### 3. Directory Structure

Create necessary directories:

```bash
# Main data directory
sudo mkdir -p /opt/clay-studio/analysis-data
sudo mkdir -p /opt/clay-studio/analysis-data/databases
sudo mkdir -p /opt/clay-studio/analysis-data/results
sudo mkdir -p /opt/clay-studio/analysis-data/bin

# Set appropriate permissions
sudo chown -R analysis-user:analysis-user /opt/clay-studio/analysis-data
sudo chmod 755 /opt/clay-studio/analysis-data
```

## Database Migration

### Run Analysis System Migrations

```bash
# Navigate to backend directory
cd backend/

# Run migrations to create analysis tables
sqlx migrate run --source ./migrations

# Verify tables were created
psql $DATABASE_URL -c "\dt analysis*"
```

Expected tables:
- `analyses` - Analysis definitions
- `analysis_versions` - Version history
- `analysis_jobs` - Execution tracking
- `analysis_schedules` - Cron schedules
- `analysis_dependencies` - Dependencies
- `analysis_result_storage` - Result metadata

## Application Integration

### 1. Service Initialization

Add to your main application startup:

```rust
use clay_studio_backend::core::analysis::*;
use std::path::PathBuf;

async fn initialize_analysis_system(db: sqlx::PgPool) -> anyhow::Result<Arc<AnalysisService>> {
    let data_dir = PathBuf::from(
        std::env::var("ANALYSIS_DATA_DIR")
            .unwrap_or_else(|_| "./analysis-data".to_string())
    );
    
    let service = AnalysisService::new(db, data_dir).await?;
    service.start().await?;
    
    tracing::info!("Analysis system initialized successfully");
    Ok(Arc::new(service))
}
```

### 2. API Route Integration

Add analysis routes to your router:

```rust
use clay_studio_backend::api::analyses;

// In your router setup
let analysis_routes = analyses::configure_analysis_routes();
router = router.push(Router::with_path("/api").push(analysis_routes));
```

### 3. Background Services

Ensure the scheduler is running:

```rust
// The scheduler starts automatically with AnalysisService::start()
// Monitor its health with periodic checks
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5 minutes
    
    loop {
        interval.tick().await;
        
        let stats = analysis_service.get_system_stats().await;
        match stats {
            Ok(stats) => {
                tracing::info!("Analysis system stats: {} running jobs, {} total results", 
                             stats.running_jobs_count, stats.total_results);
            }
            Err(e) => {
                tracing::error!("Failed to get analysis system stats: {}", e);
            }
        }
    }
});
```

## Production Configuration

### 1. Performance Tuning

**Database Optimization:**
```sql
-- Optimize PostgreSQL for analysis workloads
ALTER SYSTEM SET shared_buffers = '2GB';
ALTER SYSTEM SET work_mem = '256MB';
ALTER SYSTEM SET maintenance_work_mem = '512MB';
ALTER SYSTEM SET effective_cache_size = '6GB';

-- Analysis-specific indexes
CREATE INDEX CONCURRENTLY idx_analysis_jobs_status_created 
    ON analysis_jobs(status, created_at) 
    WHERE status IN ('pending', 'running');

CREATE INDEX CONCURRENTLY idx_analysis_schedules_next_run_enabled 
    ON analysis_schedules(next_run_at) 
    WHERE enabled = true;
```

**Application Configuration:**
```rust
// Configure connection pool for analysis workloads
let db_pool = sqlx::postgres::PgPoolOptions::new()
    .max_connections(20)
    .min_connections(5)
    .acquire_timeout(Duration::from_secs(30))
    .idle_timeout(Duration::from_secs(600))
    .max_lifetime(Duration::from_secs(1800))
    .connect(&database_url)
    .await?;
```

### 2. Security Configuration

**Firewall Rules:**
```bash
# Only allow necessary ports
sudo ufw allow 5432/tcp  # PostgreSQL
sudo ufw allow 8080/tcp  # Application port
sudo ufw deny 3000/tcp   # Block development ports
```

**File Permissions:**
```bash
# Restrict access to analysis data
sudo chmod 700 /opt/clay-studio/analysis-data
sudo chmod 600 /opt/clay-studio/analysis-data/databases/*
```

### 3. Monitoring Setup

**System Monitoring:**
```yaml
# Prometheus metrics example
analysis_jobs_total{status="completed"} 1234
analysis_jobs_total{status="failed"} 56
analysis_execution_duration_seconds_bucket{le="30"} 890
analysis_memory_usage_bytes 536870912
```

**Health Checks:**
```rust
// Add health check endpoint
#[handler]
async fn analysis_health_check(
    analysis_service: Data<&Arc<AnalysisService>>
) -> Result<Json<Value>, AppError> {
    let stats = analysis_service.get_system_stats().await?;
    
    let health_status = if stats.running_jobs_count < 50 { // threshold
        "healthy"
    } else {
        "degraded"
    };
    
    Ok(Json(serde_json::json!({
        "status": health_status,
        "running_jobs": stats.running_jobs_count,
        "total_results": stats.total_results,
        "storage_mb": stats.storage_size_bytes / 1_048_576
    })))
}
```

## Backup and Recovery

### 1. Database Backup

```bash
#!/bin/bash
# backup-analysis-db.sh

BACKUP_DIR="/opt/backups/analysis"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")

# Create backup directory
mkdir -p "$BACKUP_DIR"

# Backup analysis tables
pg_dump "$DATABASE_URL" \
    --table="analyses*" \
    --table="analysis_*" \
    --data-only \
    --file="$BACKUP_DIR/analysis_data_$TIMESTAMP.sql"

# Backup schema
pg_dump "$DATABASE_URL" \
    --schema-only \
    --table="analyses*" \
    --table="analysis_*" \
    --file="$BACKUP_DIR/analysis_schema_$TIMESTAMP.sql"

# Compress backups older than 1 day
find "$BACKUP_DIR" -name "*.sql" -mtime +1 -exec gzip {} \;

# Delete backups older than 30 days
find "$BACKUP_DIR" -name "*.sql.gz" -mtime +30 -delete
```

### 2. DuckDB Backup

```bash
#!/bin/bash
# backup-duckdb.sh

ANALYSIS_DATA_DIR="/opt/clay-studio/analysis-data"
BACKUP_DIR="/opt/backups/duckdb"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")

# Create backup directory
mkdir -p "$BACKUP_DIR"

# Backup all DuckDB databases
tar -czf "$BACKUP_DIR/duckdb_databases_$TIMESTAMP.tar.gz" \
    -C "$ANALYSIS_DATA_DIR" databases/

# Delete backups older than 14 days
find "$BACKUP_DIR" -name "*.tar.gz" -mtime +14 -delete
```

### 3. Recovery Procedures

**Database Recovery:**
```bash
# Stop the application
sudo systemctl stop clay-studio

# Restore from backup
psql "$DATABASE_URL" < "/opt/backups/analysis/analysis_schema_TIMESTAMP.sql"
psql "$DATABASE_URL" < "/opt/backups/analysis/analysis_data_TIMESTAMP.sql"

# Restart application
sudo systemctl start clay-studio
```

**DuckDB Recovery:**
```bash
# Stop the application
sudo systemctl stop clay-studio

# Restore DuckDB databases
cd /opt/clay-studio/analysis-data
tar -xzf "/opt/backups/duckdb/duckdb_databases_TIMESTAMP.tar.gz"

# Fix permissions
chown -R analysis-user:analysis-user databases/

# Restart application
sudo systemctl start clay-studio
```

## Maintenance

### 1. Regular Cleanup

```bash
#!/bin/bash
# cleanup-analysis-system.sh

# Clean up old job results (older than retention period)
RETENTION_DAYS=${ANALYSIS_RESULT_RETENTION_DAYS:-90}

psql "$DATABASE_URL" -c "
DELETE FROM analysis_result_storage 
WHERE created_at < NOW() - INTERVAL '$RETENTION_DAYS days';
"

# Clean up old job records (keep metadata for 1 year)
psql "$DATABASE_URL" -c "
DELETE FROM analysis_jobs 
WHERE created_at < NOW() - INTERVAL '365 days' 
AND status IN ('completed', 'failed', 'cancelled');
"

# Vacuum and analyze tables
psql "$DATABASE_URL" -c "
VACUUM ANALYZE analyses;
VACUUM ANALYZE analysis_jobs;
VACUUM ANALYZE analysis_result_storage;
"
```

### 2. Performance Monitoring

```bash
#!/bin/bash
# monitor-analysis-performance.sh

# Check for long-running jobs
psql "$DATABASE_URL" -c "
SELECT id, analysis_id, 
       EXTRACT(EPOCH FROM (NOW() - started_at))/60 as runtime_minutes
FROM analysis_jobs 
WHERE status = 'running' 
AND started_at < NOW() - INTERVAL '30 minutes'
ORDER BY started_at;
"

# Check for frequently failing analyses
psql "$DATABASE_URL" -c "
SELECT analysis_id, 
       COUNT(*) as total_runs,
       COUNT(*) FILTER (WHERE status = 'failed') as failures,
       ROUND(COUNT(*) FILTER (WHERE status = 'failed') * 100.0 / COUNT(*), 2) as failure_rate
FROM analysis_jobs 
WHERE created_at > NOW() - INTERVAL '7 days'
GROUP BY analysis_id
HAVING COUNT(*) FILTER (WHERE status = 'failed') * 100.0 / COUNT(*) > 20
ORDER BY failure_rate DESC;
"
```

## Troubleshooting

### Common Issues

**1. DuckDB Download Failures**
```bash
# Manually download DuckDB if auto-download fails
mkdir -p /opt/clay-studio/analysis-data/bin
cd /opt/clay-studio/analysis-data/bin

# Download for your platform
wget https://github.com/duckdb/duckdb/releases/download/v1.1.3/duckdb_cli-linux-amd64.zip
unzip duckdb_cli-linux-amd64.zip
chmod +x duckdb
```

**2. High Memory Usage**
```bash
# Monitor memory usage per job
ps aux | grep clay-studio | awk '{print $4, $11}' | sort -nr

# Check DuckDB database sizes
du -sh /opt/clay-studio/analysis-data/databases/*

# Clean up large temporary files
find /opt/clay-studio/analysis-data -name "*.tmp" -size +100M -delete
```

**3. Job Queue Backlog**
```sql
-- Check job queue status
SELECT status, COUNT(*) 
FROM analysis_jobs 
WHERE created_at > NOW() - INTERVAL '1 hour'
GROUP BY status;

-- Cancel stuck jobs
UPDATE analysis_jobs 
SET status = 'cancelled', 
    error_message = 'Cancelled due to timeout',
    completed_at = NOW()
WHERE status = 'running' 
AND started_at < NOW() - INTERVAL '2 hours';
```

### Log Analysis

**Key Log Patterns:**
```bash
# Analysis execution errors
grep "Analysis.*failed" /var/log/clay-studio/application.log

# Performance warnings
grep "execution.*timeout\|memory.*limit" /var/log/clay-studio/application.log

# Database connection issues
grep "database.*error\|connection.*failed" /var/log/clay-studio/application.log
```

## Scaling Considerations

### Horizontal Scaling

**Multiple Instance Setup:**
- Use Redis for job queue coordination
- Share DuckDB storage via NFS or object storage
- Implement job distribution algorithm

**Load Balancing:**
```nginx
upstream analysis_backend {
    server analysis1.internal:8080;
    server analysis2.internal:8080;
    server analysis3.internal:8080;
}

location /api/analysis {
    proxy_pass http://analysis_backend;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
}
```

### Vertical Scaling

**Resource Limits:**
```rust
// Adjust based on available resources
const MAX_CONCURRENT_JOBS: usize = num_cpus::get() * 2;
const MAX_MEMORY_PER_JOB: usize = 512 * 1024 * 1024; // 512MB
const JOB_TIMEOUT_SECONDS: u64 = 1800; // 30 minutes
```

This deployment guide provides a comprehensive foundation for running the Analysis Sandbox system in production environments.