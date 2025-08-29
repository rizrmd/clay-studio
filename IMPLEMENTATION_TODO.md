# Analysis Sandbox Implementation Todo

This document outlines all the tasks needed to implement the Analysis Sandbox system described in SANDBOX_ARCHITECTURE.md.

## Phase 1: Core Infrastructure

### Database Schema Setup
- [ ] Create PostgreSQL tables for analyses, versions, runs, and schedules
- [ ] Add crash tracking tables:
  - [ ] analysis_crashes table (crash details, stack traces)
  - [ ] crash_recovery_attempts table
  - [ ] resource_exhaustion_events table
- [ ] Add indexes for performance optimization
- [ ] Set up database migrations
- [ ] Create seed data for testing

### QuickJS Runtime Integration
- [ ] Set up rquickjs dependency in Cargo.toml
- [ ] Implement basic QuickJS sandbox wrapper
- [ ] Configure per-analysis memory limits (from analysis metadata)
- [ ] Set up per-analysis execution timeout handling
- [ ] Implement module loading system
- [ ] Add security restrictions (no eval, file access, etc.)
- [ ] Create runtime configuration from analysis limits

### DuckDB Integration
- [ ] Add DuckDB dependency to Cargo.toml
- [ ] Implement per-project persistent DuckDB databases
- [ ] Create DuckDB connection pool/manager
- [ ] Implement basic SQL execution (exec, query)
- [ ] Add table management (list, describe, drop)
- [ ] Configure per-analysis DuckDB memory allocation
- [ ] Implement per-analysis query result size validation

## Phase 2: Context API Implementation

### Core Context Object
- [ ] Design AnalysisContext struct in Rust
- [ ] Implement metadata storage (HashMap<String, Value>)
- [ ] Add logging functionality (ctx.log, ctx.error)
- [ ] Implement sleep and cancellation (ctx.sleep, ctx.shouldStop)
- [ ] Add unique run ID generation and tracking

### Datasource Integration
- [ ] Extend existing DataSource trait for sandbox use
- [ ] Implement ctx.datasource.{name}.query() bindings
- [ ] Add streaming support for large datasets
- [ ] Implement type-specific operations for each datasource type:
  - [ ] SQL databases (PostgreSQL, MySQL, SQLite, ClickHouse)
  - [ ] S3 object storage (list, get, put, stream)
  - [ ] REST APIs (get, post, put, delete, paginate)
  - [ ] OpenAPI/Swagger integration
  - [ ] SOAP/WSDL support
  - [ ] GraphQL queries and mutations

### DuckDB Context Bindings
- [ ] Implement ctx.duckdb.exec() for DDL operations
- [ ] Add ctx.duckdb.query() with 10MB result size limit
- [ ] Create ctx.duckdb.load() for datasource importing
- [ ] Implement ctx.duckdb.export() to external datasources
- [ ] Add ctx.duckdb.tables() and ctx.duckdb.describe()
- [ ] Create ctx.duckdb.saveDataFrame() for Polars integration

### Analysis Execution
- [ ] Implement ctx.runAnalysis() for calling other analyses
- [ ] Add dependency resolution and validation
- [ ] Create parameter passing and validation system
- [ ] Implement circular dependency detection

## Phase 3: Polars Integration

### Polars Rust Bindings
- [ ] Add polars dependency to Cargo.toml
- [ ] Create JavaScript bindings for DataFrame creation
- [ ] Implement DataFrame operations (filter, select, groupBy)
- [ ] Add aggregation functions (sum, mean, count, etc.)
- [ ] Implement joins and sorting operations
- [ ] Add column expressions (col, lit) support
- [ ] Create Polars -> DuckDB integration

## Phase 4: Analysis Management (MCP)

### Core MCP Operations
- [ ] Implement create_analysis() function with resource limits
- [ ] Add update_analysis() with versioning
- [ ] Create delete_analysis() (soft delete)
- [ ] Implement get_analysis() and list_analyses()
- [ ] Add analysis validation system
- [ ] Implement set_analysis_limits() for per-analysis configuration

### Validation System
- [ ] JavaScript syntax validation using QuickJS
- [ ] Required structure validation (default export, run function)
- [ ] Dependencies validation (datasources and analyses exist)
- [ ] Security validation (block eval, file access, etc.)
- [ ] Parameter schema validation

### Version Management
- [ ] Implement automatic versioning on updates
- [ ] Create version history tracking
- [ ] Add rollback functionality (restore_analysis_version)
- [ ] Implement version comparison (diff_analysis_versions)
- [ ] Add change description tracking

### Resource Configuration
- [ ] Add per-analysis memory limits (default: 256MB, max: 2GB)
- [ ] Implement per-analysis timeout settings (default: 30s, max: 30min)
- [ ] Create per-analysis result size limits (default: 10MB, max: 50MB)
- [ ] Add per-analysis DuckDB memory allocation
- [ ] Implement MCP set_analysis_limits(analysis_id, limits) function
- [ ] Add MCP get_analysis_limits(analysis_id) function
- [ ] Store resource limits in analysis metadata
- [ ] Validate resource limit values against system maximums

### MCP Resource Management Functions
- [ ] Implement `set_analysis_limits(analysis_id, { memory_mb, timeout_ms, result_size_mb, duckdb_memory_mb })`
- [ ] Add `get_analysis_limits(analysis_id)` to retrieve current limits
- [ ] Create `reset_analysis_limits(analysis_id)` to restore defaults
- [ ] Implement `list_analysis_resource_usage()` for monitoring
- [ ] Add `get_system_resource_maximums()` to show available limits
- [ ] Create resource usage tracking and alerts

### MCP Retry Configuration Functions
- [ ] Implement `set_analysis_retry_config(analysis_id, { enabled: false, max_retries, retry_delay_ms, reduce_resources_on_retry, resource_reduction_factor })`
- [ ] Add `get_analysis_retry_config(analysis_id)` to retrieve current settings
- [ ] Create `enable_analysis_retry(analysis_id)` convenience function
- [ ] Add `disable_analysis_retry(analysis_id)` convenience function (default)

## Phase 5: Scheduling System

### Cron Scheduler
- [ ] Add cron dependency for expression parsing
- [ ] Implement SchedulerService background task
- [ ] Create schedule configuration API
- [ ] Add timezone support for scheduled runs
- [ ] Implement conditional execution logic

### Schedule Management
- [ ] Create set_schedule() MCP function
- [ ] Implement enable/disable schedule functionality
- [ ] Add schedule history tracking
- [ ] Implement manual trigger for scheduled analyses
- [ ] Add crash handling for scheduled analyses:
  - [ ] Track consecutive failures
  - [ ] Disable schedule after N consecutive crashes
  - [ ] Send alerts for any scheduled analysis failure
  - [ ] Log detailed failure reasons for investigation

### Parameter Defaults
- [ ] Implement special parameter values (yesterday, last_week, etc.)
- [ ] Add parameter resolution for scheduled runs
- [ ] Create default value calculation logic

## Phase 6: Parameter System

### Parameter Types
- [ ] Implement basic parameter types (string, number, boolean, date)
- [ ] Add select and multiselect support
- [ ] Create parameter validation system
- [ ] Add min/max/pattern validation for parameters

### Dynamic Parameters
- [ ] Implement options function execution
- [ ] Add parameter dependency tracking
- [ ] Create parameter refresh logic for dependent parameters
- [ ] Implement grouped select options
- [ ] Add parameter caching for performance

## Phase 7: API Endpoints

### User-Facing APIs
- [ ] Create GET /api/analysis endpoint (list analyses)
- [ ] Implement GET /api/analysis/{id} (get analysis details)
- [ ] Add POST /api/analysis/{id}/execute (execute analysis)
- [ ] Create GET /api/analysis/jobs/{job_id} (job status)
- [ ] Implement DELETE /api/analysis/jobs/{job_id} (stop job)
- [ ] Add DELETE /api/analysis/{id} (delete analysis)

### Parameter Options API
- [ ] Create POST /api/analysis/{id}/parameters/{param}/options
- [ ] Implement parameter dependency handling
- [ ] Add option caching and invalidation
- [ ] Create grouped options support

### Job Management
- [ ] Implement async job execution system
- [ ] Add job status tracking (pending, running, completed, failed)
- [ ] Create job cancellation functionality
- [ ] Implement job result storage and retrieval

## Phase 8: Result Storage

### File Storage System
- [ ] Create compressed JSON result storage
- [ ] Implement 10MB size limit validation
- [ ] Add result file organization by date
- [ ] Create result cleanup/archival system
- [ ] Implement result retrieval and decompression

### Large Result Handling
- [ ] Add result size validation before return
- [ ] Implement DuckDB reference storage for large datasets
- [ ] Create result preview generation for large results
- [ ] Add metadata storage for result references

## Phase 9: Security & Resource Management

### Security Implementation
- [ ] Implement QuickJS sandbox restrictions
- [ ] Add datasource access validation
- [ ] Create project isolation for DuckDB
- [ ] Implement per-analysis resource enforcement
- [ ] Add security audit logging
- [ ] Create resource limit monitoring and alerting

### Error Handling
- [ ] Create comprehensive error handling system
- [ ] Add detailed error messages with context
- [ ] Implement error logging and monitoring
- [ ] Create error recovery mechanisms

### Crash Recovery & Cleanup
- [ ] Implement crash detection (process monitoring, heartbeat)
- [ ] Add automatic cleanup for crashed analyses:
  - [ ] Release locked resources (memory, connections)
  - [ ] Close orphaned DuckDB connections
  - [ ] Clean up temporary files
  - [ ] Update job status to 'crashed' or 'failed'
- [ ] Implement post-crash diagnostics:
  - [ ] Capture stack traces
  - [ ] Save last known state from metadata
  - [ ] Record resource usage at crash time
  - [ ] Store partial results if available
- [ ] Add crash recovery options:
  - [ ] Implement configurable automatic retry (default: disabled)
  - [ ] Add retry configuration: max_retries, retry_delay, retry_on_crash (all default to 0/false)
  - [ ] Support retry with reduced resources (configurable reduction factor)
  - [ ] Allow manual resume from last checkpoint (using metadata)
  - [ ] Save crash state for debugging
  - [ ] Notify user/admin of crash with detailed report
- [ ] Create crash prevention mechanisms:
  - [ ] Graceful degradation when approaching limits
  - [ ] Preemptive resource checks before operations
  - [ ] Circuit breaker pattern for repeated failures

## Phase 10: Frontend Integration

### Analysis Browser
- [ ] Create analysis list view component
- [ ] Implement analysis detail view
- [ ] Add analysis execution form
- [ ] Create parameter input components

### Parameter Form System
- [ ] Implement dynamic parameter form generation
- [ ] Add dependent parameter handling
- [ ] Create select option loading and caching
- [ ] Implement form validation

### Job Monitoring
- [ ] Create job status polling system
- [ ] Implement real-time job progress display
- [ ] Add job cancellation UI
- [ ] Create result display components

### Result Visualization
- [ ] Implement JSON result viewer
- [ ] Add table display for array results
- [ ] Create chart generation for numeric data
- [ ] Implement result export functionality

## Phase 11: Advanced Features

### S3 File Processing
- [ ] Implement S3 file streaming
- [ ] Add compression support (gzip, 7z, tar, etc.)
- [ ] Create Excel/CSV parsing integration
- [ ] Implement chunk-based processing

### External API Integration
- [ ] Add REST API pagination support
- [ ] Implement OpenAPI specification parsing
- [ ] Create SOAP/WSDL client generation
- [ ] Add GraphQL introspection and validation

### Advanced Patterns
- [ ] Implement queue processing pattern examples
- [ ] Add transaction pattern support
- [ ] Create alerting system integration
- [ ] Implement point-in-time consistency helpers

## Phase 12: Performance & Monitoring

### Performance Optimization
- [ ] Add query result caching
- [ ] Implement connection pooling optimization
- [ ] Create DuckDB query optimization
- [ ] Add memory usage monitoring

### Monitoring & Logging
- [ ] Implement comprehensive logging system
- [ ] Add performance metrics collection
- [ ] Create health check endpoints
- [ ] Implement error rate monitoring
- [ ] Add crash monitoring and alerting:
  - [ ] Real-time crash detection
  - [ ] Crash rate metrics per analysis
  - [ ] Resource exhaustion alerts
  - [ ] Dead process cleanup scheduler
  - [ ] Crash report generation
  - [ ] Integration with monitoring systems (Prometheus, Grafana)

### Testing
- [ ] Create unit tests for all core functionality
- [ ] Add integration tests for analysis execution
- [ ] Implement performance benchmarks
- [ ] Create end-to-end test suite

## Phase 13: Documentation & Deployment

### Documentation
- [ ] Create API documentation
- [ ] Write user guides for analysis creation
- [ ] Create pattern examples and tutorials
- [ ] Add troubleshooting guides

### Deployment
- [ ] Create Docker containerization
- [ ] Add database migration scripts
- [ ] Implement configuration management
- [ ] Create deployment automation

### TypeScript Definitions
- [ ] Generate TypeScript definitions from Rust code
- [ ] Create comprehensive type definitions for LLM use
- [ ] Add JSDoc comments for better IDE support
- [ ] Implement type validation

## Estimated Timeline

- **Phase 1-3** (Core Infrastructure): 4-6 weeks
- **Phase 4-5** (Analysis Management): 3-4 weeks  
- **Phase 6-7** (Parameters & APIs): 3-4 weeks
- **Phase 8-9** (Storage & Security): 2-3 weeks
- **Phase 10** (Frontend): 4-5 weeks
- **Phase 11** (Advanced Features): 3-4 weeks
- **Phase 12-13** (Polish & Deploy): 2-3 weeks

**Total Estimated Time: 21-29 weeks (5-7 months)**

## Priority Levels

### High Priority (MVP)
- Phase 1: Core Infrastructure
- Phase 2: Context API Implementation  
- Phase 4: Basic Analysis Management
- Phase 7: Core API Endpoints
- Phase 8: Result Storage

### Medium Priority
- Phase 3: Polars Integration
- Phase 5: Scheduling System
- Phase 6: Parameter System
- Phase 10: Frontend Integration

### Low Priority (Nice to Have)
- Phase 9: Advanced Security
- Phase 11: Advanced Features
- Phase 12: Advanced Monitoring
- Phase 13: Documentation Polish

## Dependencies

- **Rust Crates**: rquickjs, duckdb, polars, sqlx, tokio, cron, chrono
- **Database**: PostgreSQL for metadata, DuckDB for analytics
- **Frontend**: React, TypeScript, existing UI components
- **Infrastructure**: Existing datasource implementations, MCP framework

## Risk Mitigation

- **QuickJS Integration**: Start with simple proof-of-concept
- **Performance**: Implement resource limits early
- **Security**: Regular security audits during development
- **Complexity**: Build incrementally with extensive testing