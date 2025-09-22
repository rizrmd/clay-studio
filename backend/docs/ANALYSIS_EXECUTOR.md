# Analysis Executor - Deployment Guide

## Overview

The Analysis Executor is a dedicated HTTP service that processes analysis jobs independently from the main backend API. It uses database polling for job coordination and HTTP for health checks.

## Architecture

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Backend   │     │  MCP Server │     │  Analysis   │
│   :8001     │     │             │     │  Executor   │
│             │     │             │     │  :8002      │
│  Submit Jobs│────▶│ Tool Calls  │────▶│ Poll & Exec │
│  Get Status │     │ Get Results │     │ Update DB   │
└─────────────┘     └─────────────┘     └─────────────┘
       │                   │                   │
       └───────────────────┼───────────────────┘
                           │
                  ┌────────▼─────────┐
                  │   PostgreSQL    │
                  │   Database      │
                  │                 │
                  │ • analyses      │
                  │ • analysis_jobs │
                  │ • results       │
                  └─────────────────┘
```

## Building

```bash
# Build the analysis executor
cd backend
cargo build --release --bin analysis_executor

# The binary will be at:
# target/release/analysis_executor
```

## Configuration

### Environment Variables

```bash
# Required
DATABASE_URL=postgres://user:pass@localhost:5432/clay_studio

# Optional
ANALYSIS_EXECUTOR_PORT=8002
ANALYSIS_EXECUTOR_URL=http://localhost:8002
RUST_LOG=info
```

### Database Setup

The analysis executor requires the analysis tables to be created:

```bash
# Run migrations
cd backend
sqlx migrate run --source ./migrations

# Verify tables exist
psql $DATABASE_URL -c "SELECT tablename FROM pg_tables WHERE tablename LIKE 'analysis%';"
```

Expected tables:
- `analyses` - Analysis script definitions
- `analysis_jobs` - Job execution tracking
- `analysis_versions` - Version history
- `analysis_schedules` - Scheduled execution
- `analysis_dependencies` - Datasource dependencies
- `analysis_result_storage` - Large result metadata

## Running

### Development

```bash
# Terminal 1: Start main backend
cd backend
cargo run

# Terminal 2: Start analysis executor
cd backend
ANALYSIS_EXECUTOR_PORT=8002 cargo run --bin analysis_executor

# Terminal 3: Test health check
curl http://localhost:8002/health
```

### Production

```bash
# Start analysis executor as a service
./target/release/analysis_executor \
  --port 8002 \
  --database-url $DATABASE_URL \
  --log-level info
```

### Docker

```dockerfile
# Add to your existing Dockerfile
COPY target/release/analysis_executor /usr/local/bin/
EXPOSE 8002

# Run both services
CMD ["sh", "-c", "analysis_executor & ./clay-studio-backend"]
```

## Testing

### Health Check

```bash
curl http://localhost:8002/health
# Expected response:
{
  "status": "healthy",
  "version": "0.1.0",
  "jobs_processed": 0
}
```

### Submit Analysis Job via MCP

```bash
# Through Claude CLI with MCP server
# The MCP server will call:
# POST /analysis_run with analysis_id and parameters
```

### Direct Database Testing

```sql
-- Create a test analysis
INSERT INTO analyses (id, title, script_content, project_id, is_active, version)
VALUES (
  gen_random_uuid(),
  'Test Analysis',
  'export default function analyze(data) { return {result: "success"}; }',
  'your-project-id',
  true,
  1
);

-- Submit a job
INSERT INTO analysis_jobs (id, analysis_id, status, parameters, triggered_by)
VALUES (
  gen_random_uuid(),
  'your-analysis-id',
  'pending',
  '{"test": true}',
  'manual_test'
);

-- Watch for job execution
SELECT * FROM analysis_jobs ORDER BY created_at DESC LIMIT 5;
```

## Monitoring

### Health Endpoints

- `GET /health` - Service health and job count
- `POST /jobs` - Future direct job submission

### Database Queries

```sql
-- Check job queue status
SELECT status, COUNT(*) 
FROM analysis_jobs 
WHERE created_at > NOW() - INTERVAL '1 hour'
GROUP BY status;

-- Find long-running jobs
SELECT id, analysis_id, 
       EXTRACT(EPOCH FROM (NOW() - started_at))/60 as runtime_minutes
FROM analysis_jobs 
WHERE status = 'running' 
AND started_at < NOW() - INTERVAL '30 minutes';

-- Recent job success rate
SELECT 
  COUNT(*) as total_jobs,
  COUNT(*) FILTER (WHERE status = 'completed') as successful,
  COUNT(*) FILTER (WHERE status = 'failed') as failed,
  ROUND(COUNT(*) FILTER (WHERE status = 'completed') * 100.0 / COUNT(*), 2) as success_rate
FROM analysis_jobs 
WHERE created_at > NOW() - INTERVAL '24 hours';
```

### Logs

The executor outputs structured logs:

```bash
# Follow logs
tail -f /var/log/analysis-executor.log

# Key log messages:
# - "Analysis Executor starting..."
# - "Job poller started"
# - "Found N pending jobs"
# - "Job {id} completed successfully"
# - "Job {id} failed: {error}"
```

## Troubleshooting

### Common Issues

1. **Executor can't connect to database**
   ```bash
   # Check DATABASE_URL
   echo $DATABASE_URL
   
   # Test connection
   psql $DATABASE_URL -c "SELECT 1;"
   ```

2. **Jobs stuck in pending**
   ```bash
   # Check if executor is running
   curl http://localhost:8002/health
   
   # Check executor logs
   tail -f executor.log
   ```

3. **Jobs failing immediately**
   ```sql
   -- Check error messages
   SELECT id, error_message, created_at 
   FROM analysis_jobs 
   WHERE status = 'failed' 
   ORDER BY created_at DESC 
   LIMIT 10;
   ```

### Performance Tuning

```bash
# Increase polling frequency (default: 1s)
POLL_INTERVAL_MS=500 ./analysis_executor

# Increase concurrent job limit (default: 5)
MAX_CONCURRENT_JOBS=10 ./analysis_executor
```

## Security

- Executor runs on localhost by default
- No external API endpoints exposed
- Database connection uses existing credentials
- JavaScript execution will be sandboxed (future)

## Scaling

### Single Machine
- Run multiple executor instances on different ports
- Each polls the same job queue
- Database handles concurrency with SKIP LOCKED

### Multiple Machines
- Deploy executor on each machine
- Point to same database
- Use load balancer for health checks
- Jobs automatically distributed

## Future Enhancements

1. **JavaScript Sandbox Integration** - QuickJS execution environment
2. **Datasource Query Integration** - Reuse existing connection pools
3. **Result File Storage** - Large result handling
4. **Scheduled Execution** - Cron-based job scheduling
5. **Resource Limits** - Memory/CPU constraints per job