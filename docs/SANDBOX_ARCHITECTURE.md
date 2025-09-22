# Analysis Sandbox - Architecture Summary

## Overview
A lightweight, secure JavaScript sandbox for executing data analysis scripts using QuickJS with Rust bindings. Scripts can query multiple datasources, use Polars for complex in-memory processing, and persist results to DuckDB for analytical querying.

## Core Design Principles
- **Simple API** - Focus on querying with Polars and DuckDB as complementary tools
- **Security** - Sandboxed execution with no file/network access
- **Performance** - QuickJS provides 1-5ms startup time
- **Modular** - Analyses can call other analyses as dependencies
- **Persistent Analytics** - DuckDB stores transformation results for future querying
- **Complementary Tools** - Polars for in-memory processing → DuckDB for persistence

## Analysis Script Structure

Every analysis must export an object with this structure:

```javascript
export default {
    title: "Monthly customer report",
    
    // Optional: Schedule for automatic execution
    schedule: {
        cron: "0 2 * * *",  // Daily at 2 AM
        timezone: "UTC",    // Optional, defaults to UTC
        enabled: true       // Can be toggled on/off
    },
    
    dependencies: {
        datasources: ["postgres_main", "clickhouse"],  // Required datasources
        analyses: ["customer_metrics", "sales_summary"] // Other analyses to call
    },
    
    parameters: {
        month: { 
            type: "date", 
            required: true,
            default: "current_month"  // Special value for scheduled runs
        },
        category: {
            type: "select",
            required: true,
            default: "all",  // Default for scheduled runs
            options: async (ctx, params) => {
                // Dynamic options from database
                const categories = await ctx.datasource.postgres_main.query(
                    "SELECT id as value, name as label FROM categories");
                return categories;
            }
        }
    },
    
    run: async function(ctx, params) {
        // Main execution logic
        const users = await ctx.datasource.postgres_main.query("SELECT * FROM users");
        const events = await ctx.datasource.clickhouse.query("SELECT * FROM events");
        
        // Call another analysis
        const metrics = await ctx.runAnalysis("customer_metrics", params);
        
        // Must return an object (not array)
        return {
            users: users.length,
            events: events.length,
            metrics: metrics.result
        };
    }
}
```

## Datasource Types

Each datasource type provides specific operations suited to its capabilities:

### SQL Databases (PostgreSQL, MySQL, SQLite, ClickHouse)
```javascript
// PostgreSQL
ctx.datasource.postgres_main.query(sql, params?)
ctx.datasource.postgres_main.stream(sql, params?, batchSize?)

// MySQL
ctx.datasource.mysql_legacy.query(sql, params?)
ctx.datasource.mysql_legacy.stream(sql, params?, batchSize?)

// ClickHouse (optimized for analytics)
ctx.datasource.clickhouse_events.query(sql, params?)
ctx.datasource.clickhouse_events.insert(table, data)

// SQLite (local file-based)
ctx.datasource.sqlite_local.query(sql, params?)
ctx.datasource.sqlite_local.execute(sql, params?)
```

### File-Based Datasources (CSV, Parquet, Excel)
Query files directly with SQL using DuckDB as the engine:
```javascript
// Local files
ctx.datasource.sales_csv.query(sql)              // Query CSV with SQL
ctx.datasource.analytics_parquet.query(sql)      // Query Parquet files
ctx.datasource.reports_excel.query(sql)          // Query Excel sheets
ctx.datasource.sales_csv.getSchema()             // Get column names and types
ctx.datasource.sales_csv.getRowCount()           // Get total row count
ctx.datasource.sales_csv.getSampleData(limit?)  // Get sample rows
ctx.datasource.sales_csv.refresh()               // Re-read file (useful if file is updated)

// Remote files (S3, HTTP) - with auto-refresh
ctx.datasource.s3_data_lake.query(sql)           // Query S3 CSV/Parquet directly
ctx.datasource.s3_data_lake.refresh()            // Force re-download from S3
ctx.datasource.s3_data_lake.setCacheTTL(600)    // Cache for 10 minutes
ctx.datasource.http_dataset.query(sql)           // Query remote files via HTTP
ctx.datasource.http_dataset.refresh()            // Force re-download
ctx.datasource.http_dataset.getLastModified()    // Check when file was last updated

// Advanced operations
ctx.datasource.sales_csv.query(
    "SELECT * FROM data WHERE amount > 1000"
)  // 'data' is the automatic table name

// Join multiple file datasources
ctx.datasource.sales_csv.query(`
    SELECT s.*, p.price 
    FROM data s
    JOIN read_parquet('${ctx.datasource.products_parquet.path}') p
    ON s.product_id = p.id
`)
```

### Cloud Spreadsheets (Google Sheets, Excel Online)
Query and sync with cloud-based spreadsheets:
```javascript
// Google Sheets
ctx.datasource.budget_sheet.query(sql)           // Query with SQL
ctx.datasource.budget_sheet.refresh()            // Force refresh from Google
ctx.datasource.budget_sheet.getRange("A1:Z100") // Get specific range
ctx.datasource.budget_sheet.getSheet("Q1")      // Get specific sheet

// Excel Online (Microsoft 365)
ctx.datasource.forecast_excel.query(sql)         // Query with SQL
ctx.datasource.forecast_excel.refresh()          // Sync from cloud
ctx.datasource.forecast_excel.getWorksheet(name) // Get specific worksheet

// Automatic caching with TTL
ctx.datasource.budget_sheet.setCacheTTL(300)     // Cache for 5 minutes
```

### Object Storage (S3, GCS, Azure Blob)
```javascript
// S3
ctx.datasource.s3_data_lake.list(prefix, options?)
ctx.datasource.s3_data_lake.get(key)
ctx.datasource.s3_data_lake.put(key, data, metadata?)
ctx.datasource.s3_data_lake.delete(key)
ctx.datasource.s3_data_lake.getSignedUrl(key, expiry?)
ctx.datasource.s3_data_lake.multipartUpload(key, parts)

// Stream large files
ctx.datasource.s3_data_lake.stream(key, {
    decompress: '7z',  // Auto-decompress: gzip, zip, 7z, tar, bz2
    format: 'excel',   // Parse format: csv, excel, json, parquet
    onChunk: async (data) => {
        // Process data chunk by chunk
    }
})
```

### REST APIs
```javascript
// REST with automatic pagination
ctx.datasource.api_customers.get(endpoint, params?)
ctx.datasource.api_customers.post(endpoint, body, params?)
ctx.datasource.api_customers.put(endpoint, body, params?)
ctx.datasource.api_customers.delete(endpoint, params?)
ctx.datasource.api_customers.paginate(endpoint, {
    pageParam: 'page',
    limitParam: 'limit',
    dataPath: 'data.items',
    nextPagePath: 'data.next'
})
```

### OpenAPI/Swagger
```javascript
// Type-safe OpenAPI calls
ctx.datasource.openapi_service.operations.listUsers({ limit: 10 })
ctx.datasource.openapi_service.operations.getUser({ userId: "123" })
ctx.datasource.openapi_service.operations.createUser({ 
    body: { name: "John", email: "john@example.com" }
})

// Direct path access
ctx.datasource.openapi_service.call('/users/{id}', {
    method: 'GET',
    pathParams: { id: "123" },
    queryParams: { include: "profile" }
})
```

### SOAP/WSDL
```javascript
// SOAP service calls
ctx.datasource.soap_billing.call('GetInvoice', {
    InvoiceId: '12345',
    IncludeItems: true
})

// With complex types
ctx.datasource.soap_billing.call('CreateOrder', {
    Order: {
        Customer: { Id: '123', Name: 'ACME Corp' },
        Items: [
            { ProductId: 'ABC', Quantity: 5 },
            { ProductId: 'XYZ', Quantity: 3 }
        ]
    }
})
```

### GraphQL
```javascript
// GraphQL queries
ctx.datasource.graphql_api.query({
    query: `
        query GetUser($id: ID!) {
            user(id: $id) {
                name
                email
                posts {
                    title
                    createdAt
                }
            }
        }
    `,
    variables: { id: "123" }
})

// GraphQL mutations
ctx.datasource.graphql_api.mutate({
    mutation: `
        mutation CreatePost($input: PostInput!) {
            createPost(input: $input) {
                id
                title
            }
        }
    `,
    variables: {
        input: {
            title: "New Post",
            content: "Content here"
        }
    }
})
```

## Context API

The `ctx` object provides these methods:

### Core Operations
```javascript
// Access datasources by name with type-specific operations
ctx.datasource.postgres_main.query(sql, params?)
ctx.datasource.s3_bucket.list(prefix)
ctx.datasource.api_service.get(endpoint)
// ... see Datasource Types section above for full list

// Call other analyses
await ctx.runAnalysis(analysisId, parameters)

// Access MCP data analysis tools (only mcp__data-analysis tools available)
await ctx.mcp.callTool('mcp__data-analysis__datasource_query', parameters)
await ctx.mcp.callTool('mcp__data-analysis__schema_get', parameters)
await ctx.mcp.listTools()  // Lists available data analysis tools
await ctx.mcp.getToolSchema(toolName)

// Utilities
ctx.log(...args)           // Debug output
await ctx.sleep(ms)         // Rate limiting
ctx.shouldStop()            // Check if cancelled (for long-running)
```

### DuckDB Operations
For heavy data processing and persistent analytical storage:

```javascript
ctx.duckdb = {
    // Execute SQL without returning results (DDL, large operations)
    exec: async (sql) => {},
    
    // Query with results (small results only, < 10MB)
    query: async (sql) => [],
    
    // Load data from datasource into DuckDB table (persistent)
    load: async (datasource, sql, tableName) => {},
    
    // Save DuckDB table to external datasource
    export: async (tableName, datasource, destinationTable) => {},
    
    // List tables in DuckDB (includes all persistent tables)
    tables: async () => [],
    
    // Get table info (row count, columns, size)
    describe: async (tableName) => {}
}
```

### Metadata Storage
Store references to large datasets or processing results:

```javascript
ctx.metadata = {
    set: (key, value) => {},  // Store metadata
    get: (key) => {}          // Retrieve metadata
}
```

### MCP Tool Integration
Access data analysis tools through MCP (Model Context Protocol):

```javascript
// Only mcp__data-analysis tools are available in sandbox
ctx.mcp = {
    // Call MCP data analysis tool
    callTool: async (toolName, parameters) => {},
    
    // List available MCP tools
    listTools: async () => [],
    
    // Get schema for a specific tool
    getToolSchema: async (toolName) => {},
    
    // Check if a tool is available
    hasTool: (toolName) => boolean
}

// Example: Using available MCP tools for datasource management
await ctx.mcp.callTool('mcp__data-analysis__datasource_add', {
    name: 'new_analytics_db',
    source_type: 'postgresql',
    config: 'postgres://user:pass@host:port/db'
});

await ctx.mcp.callTool('mcp__data-analysis__datasource_query', {
    datasource_id: 'analytics_db_id',
    query: 'SELECT * FROM users LIMIT 100',
    limit: 100
});

await ctx.mcp.callTool('mcp__data-analysis__schema_get', {
    datasource_id: 'analytics_db_id',
    table_name: 'users'
});
```

### Polars DataFrame Integration
For in-memory data processing that feeds into DuckDB:

```javascript
// Create and manipulate DataFrames in memory
const df = ctx.DataFrame(data);

// Process with Polars (fast in-memory operations)
const processed = df
    .filter(row => row.get("amount") > 100)
    .groupBy("category")
    .agg({ total: "sum", count: "count" });

// Save processed DataFrame to DuckDB for persistence
await ctx.duckdb.saveDataFrame(processed, "analysis_results_2024");
```

## Parameter Types

### Static Parameters
```javascript
parameters: {
    startDate: { type: "date", required: true },
    limit: { type: "number", required: false },
    includeArchived: { type: "boolean", required: false }
}
```

### Dynamic Select Options
```javascript
parameters: {
    category: {
        type: "select",
        required: true,
        options: async (ctx) => {
            const result = await ctx.datasource.postgres_main.query(
                "SELECT DISTINCT category as value, category as label FROM products");
            return result;
        }
    },
    
    // Parameter with dependencies
    city: {
        type: "select", 
        required: true,
        options: async (ctx, params) => {
            if (!params.country) {
                return [{ value: "", label: "Select country first" }];
            }
            const cities = await ctx.datasource.postgres_main.query(
                "SELECT id as value, name as label FROM cities WHERE country_id = $1",
                [params.country]);
            return cities;
        }
    }
}
```

## Backend API Endpoints

### User-Facing Operations
Users can only execute and manage analyses, not create or modify them:

#### Discovery & Metadata
```bash
# List all available analyses
GET /api/analysis
Response: [
    {
        "id": "monthly_report",
        "title": "Monthly Sales Report",
        "created_at": "2024-01-01T00:00:00Z",
        "last_run": "2024-01-15T10:30:00Z"
    },
    ...
]

# Get analysis details with parameter metadata
GET /api/analysis/{analysis_id}
Response: {
    "id": "monthly_report",
    "title": "Monthly Sales Report",
    "parameters": {
        "month": {
            "type": "date",
            "required": true,
            "description": "Report month"
        },
        "category": {
            "type": "select",
            "required": true,
            "has_dynamic_options": true,  # Indicates backend execution needed
            "description": "Product category"
        },
        "city": {
            "type": "select",
            "required": false,
            "has_dynamic_options": true,
            "depends_on": ["country"],    # Re-fetch when these params change
            "description": "Filter by city"
        }
    }
}

# Get dynamic parameter options (backend executes the options function)
POST /api/analysis/{analysis_id}/parameters/{param_name}/options
Body: {
    "current_params": {  # Current form values for dependent parameters
        "country": "USA",
        "month": "2024-01"
    }
}
Response: {
    "options": [
        { "value": "nyc", "label": "New York City" },
        { "value": "la", "label": "Los Angeles" },
        { "value": "chi", "label": "Chicago" }
    ]
}
# For grouped options:
Response: {
    "options": [
        {
            "label": "Premium Cities",
            "options": [
                { "value": "nyc", "label": "New York City" },
                { "value": "sf", "label": "San Francisco" }
            ]
        },
        {
            "label": "Standard Cities",
            "options": [
                { "value": "aus", "label": "Austin" },
                { "value": "den", "label": "Denver" }
            ]
        }
    ]
}
```

#### Execution
```bash
# Execute analysis (always async, returns job_id)
POST /api/analysis/{analysis_id}/execute
Body: { 
    "parameters": { 
        "month": "2024-01",
        "category": "electronics",
        "city": "nyc"
    } 
}
Response: { "job_id": "abc-123" }

# Check job status
GET /api/analysis/jobs/{job_id}
Response: {
    "status": "running",  // or "completed", "failed"
    "result": { ... },    // When completed
    "error": "...",       // If failed
    "logs": ["Processing batch 1000", ...]
}

# Stop running analysis
DELETE /api/analysis/jobs/{job_id}

# Delete analysis
DELETE /api/analysis/{analysis_id}
```

### MCP Operations (Backend Only)
MCP has full control over analyses, including all user operations plus creation/modification:

#### Core Operations
- `create_analysis(script_content, metadata)`
- `update_analysis(analysis_id, script_content, change_description)`
- `delete_analysis(analysis_id)`
- `get_analysis(analysis_id)`
- `list_analyses()`

#### File Datasource Operations
- `add_file_datasource(name, file_path, file_type)` - Add CSV/Parquet/Excel file as datasource
- `add_google_sheets_datasource(name, sheet_id, credentials)` - Connect Google Sheets
- `add_excel_online_datasource(name, file_url, auth_token)` - Connect Excel Online
- `refresh_cloud_datasource(datasource_id)` - Force refresh cloud spreadsheet data
- `update_file_datasource(datasource_id, new_path)` - Update file location
- `get_file_schema(datasource_id)` - Get column names and types for file

#### Scheduling Operations
- `set_schedule(analysis_id, cron_expression, timezone, enabled)` - Configure schedule
- `get_schedule(analysis_id)` - Get current schedule configuration
- `enable_schedule(analysis_id)` - Enable scheduled execution
- `disable_schedule(analysis_id)` - Disable scheduled execution
- `get_schedule_history(analysis_id)` - Get past scheduled runs
- `trigger_scheduled_run(analysis_id)` - Manually trigger a scheduled run with defaults

#### Version Management

**`get_analysis_versions(analysis_id)`** - List all versions:
```json
{
  "current_version": 5,
  "versions": [
    {
      "version": 5,
      "created_at": "2024-01-15T10:30:00Z",
      "change_description": "Added customer filter parameter",
      "created_by": "mcp"
    },
    {
      "version": 4,
      "created_at": "2024-01-14T15:20:00Z",
      "change_description": "Fixed date range query",
      "created_by": "mcp"
    }
  ]
}
```

**`get_analysis_version(analysis_id, version_number)`** - Get specific version:
```json
{
  "version": 3,
  "script": "export default { ... }",
  "metadata": {
    "dependencies": { "datasources": ["postgres_main"] },
    "parameters": { ... }
  },
  "created_at": "2024-01-13T09:15:00Z",
  "change_description": "Initial version with basic reporting"
}
```

**`restore_analysis_version(analysis_id, version_number, reason)`** - Restore version:
```json
{
  "success": true,
  "new_version": 6,
  "restored_from": 3,
  "reason": "Reverting filter changes that caused performance issues"
}
```

**`diff_analysis_versions(analysis_id, version1, version2)`** - Compare versions:
```json
{
  "version1": 3,
  "version2": 5,
  "changes": {
    "dependencies": {
      "datasources": {
        "added": ["clickhouse"],
        "removed": []
      }
    },
    "parameters": {
      "added": ["customer_id"],
      "removed": [],
      "modified": []
    },
    "script_lines": {
      "added": 15,
      "removed": 8,
      "modified": 23
    }
  }
}

#### Execution Operations
- `execute_analysis(analysis_id, parameters)`
- `get_job_status(job_id)`
- `stop_job(job_id)`

#### Validation
`validate_analysis(script_content)` validates the script before saving:

**Checks:**
1. **JavaScript Syntax** - Parses with QuickJS to detect syntax errors
2. **Required Structure** - Verifies:
   - Exports default object
   - Contains `dependencies.datasources[]` array
   - Contains `run` function
   - Valid parameter definitions (if present)
3. **Dependencies** - Ensures:
   - All referenced datasources exist
   - All referenced analyses exist
4. **Security** - Blocks:
   - `eval()`, `Function()` constructor
   - Forbidden imports
   - File/network access attempts

**Response:**
```json
// Success
{
  "valid": true,
  "metadata": {
    "title": "Sales Report",
    "datasources": ["postgres_main"],
    "analyses": ["customer_metrics"],
    "parameters": ["month", "category"]
  }
}

// Failure
{
  "valid": false,
  "errors": [
    "Missing required 'run' function",
    "Datasource 'unknown_db' not found",
    "Syntax error at line 15: unexpected token"
  ]
}
```

## Analysis Patterns

### Regular Analysis Pattern
For reports, aggregations, and small result sets:

```javascript
export default {
    title: "Monthly sales summary",
    
    run: async function(ctx, params) {
        const sales = await ctx.datasource.postgres_main.query(
            "SELECT SUM(amount) as total, COUNT(*) as count FROM sales WHERE month = $1",
            [params.month]
        );
        
        // Return small result (< 10MB)
        return {
            month: params.month,
            total_sales: sales[0].total,
            transaction_count: sales[0].count
        };
    }
}
```

### ETL Pattern with Persistent DuckDB
For large-scale data transformations with persistent storage:

```javascript
export default {
    title: "ETL: Clean and transform user events",
    
    run: async function(ctx, params) {
        // 1. Load large dataset into persistent DuckDB table
        await ctx.duckdb.load(
            "raw_data",
            "SELECT * FROM events WHERE date >= '2024-01-01'",
            "events_raw_2024"  // This table persists in DuckDB
        );
        
        // 2. Transform using DuckDB's SQL engine (creates persistent table)
        await ctx.duckdb.exec(`
            CREATE OR REPLACE TABLE events_clean_2024 AS
            SELECT 
                user_id,
                event_type,
                DATE_TRUNC('day', timestamp) as event_date,
                COUNT(*) as event_count,
                ANY_VALUE(metadata) as sample_metadata
            FROM events_raw_2024
            WHERE user_id IS NOT NULL
            GROUP BY user_id, event_type, event_date
        `);
        
        // 3. Optionally export to external datasource (or keep in DuckDB)
        // await ctx.duckdb.export(
        //     "events_clean_2024",
        //     "analytics_db",
        //     "events_aggregated"
        // );
        
        // 4. Create indexes for faster querying
        await ctx.duckdb.exec(`
            CREATE INDEX idx_events_user ON events_clean_2024(user_id);
            CREATE INDEX idx_events_date ON events_clean_2024(event_date);
        `);
        
        // 5. Store metadata about the operation
        ctx.metadata.set("duckdb_table", "events_clean_2024");
        ctx.metadata.set("processing_date", new Date().toISOString());
        
        // 6. Return summary only (not the actual data)
        const stats = await ctx.duckdb.query(`
            SELECT 
                COUNT(*) as total_rows,
                COUNT(DISTINCT user_id) as unique_users,
                MIN(event_date) as start_date,
                MAX(event_date) as end_date
            FROM events_clean_2024
        `);
        
        return {
            success: true,
            statistics: stats[0],
            stored_in_duckdb: "events_clean_2024",
            message: "Data available for querying in DuckDB table: events_clean_2024"
        };
    }
}
```

### Cross-Datasource Analysis Pattern
Combining data from multiple sources:

```javascript
export default {
    title: "Cross-datasource customer analysis",
    
    run: async function(ctx) {
        // Load from multiple datasources into DuckDB
        await ctx.duckdb.load("postgres_main", 
            "SELECT * FROM customers", "customers");
        await ctx.duckdb.load("clickhouse_events", 
            "SELECT * FROM user_events", "events");
        await ctx.duckdb.load("mysql_orders", 
            "SELECT * FROM orders", "orders");
        
        // Join and analyze across sources
        const analysis = await ctx.duckdb.query(`
            SELECT 
                c.customer_segment,
                COUNT(DISTINCT c.id) as customer_count,
                COUNT(DISTINCT e.user_id) as active_users,
                SUM(o.amount) as total_revenue
            FROM customers c
            LEFT JOIN events e ON c.id = e.user_id
            LEFT JOIN orders o ON c.id = o.customer_id
            GROUP BY c.customer_segment
        `);
        
        return { segments: analysis };
    }
}
```

### Polars + DuckDB Pattern
Using Polars for complex in-memory processing, then persisting to DuckDB:

```javascript
export default {
    title: "Advanced customer scoring with Polars",
    
    run: async function(ctx, params) {
        // 1. Query raw data
        const customers = await ctx.datasource.postgres_main.query( 
            "SELECT * FROM customers WHERE created_at > $1", [params.start_date]);
        const transactions = await ctx.datasource.postgres_main.query(
            "SELECT * FROM transactions WHERE date > $1", [params.start_date]);
        
        // 2. Use Polars for complex in-memory processing
        const customerDf = ctx.DataFrame(customers);
        const transDf = ctx.DataFrame(transactions);
        
        // Complex operations that Polars excels at
        const scored = customerDf
            .join(transDf, "customer_id")
            .groupBy("customer_id")
            .agg({
                total_spent: col("amount").sum(),
                transaction_count: col("amount").count(),
                avg_amount: col("amount").mean(),
                days_active: col("date").nUnique()
            })
            .withColumn("score", 
                col("total_spent") * 0.4 + 
                col("transaction_count") * 0.3 + 
                col("days_active") * 0.3
            )
            .sort("score", true);
        
        // 3. Persist results to DuckDB for future queries
        const tableName = `customer_scores_${params.start_date.replace(/-/g, '_')}`;
        await ctx.duckdb.saveDataFrame(scored, tableName);
        
        // 4. Create view for easy access
        await ctx.duckdb.exec(`
            CREATE OR REPLACE VIEW latest_customer_scores AS
            SELECT * FROM ${tableName}
        `);
        
        // 5. Return summary
        const topCustomers = scored.head(10).toJSON();
        const stats = await ctx.duckdb.query(`
            SELECT 
                COUNT(*) as total_customers,
                AVG(score) as avg_score,
                PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY score) as median_score
            FROM ${tableName}
        `);
        
        return {
            top_customers: topCustomers,
            statistics: stats[0],
            stored_table: tableName,
            message: "Customer scores calculated with Polars and stored in DuckDB"
        };
    }
}
```

### File-Based Data Analysis Pattern
Query CSV, Parquet, and Excel files directly with SQL:

```javascript
export default {
    title: "Analyze sales data from CSV and Parquet files",
    dependencies: {
        datasources: ["sales_csv", "customers_parquet", "products_excel"]
    },
    
    run: async function(ctx, params) {
        // Refresh if files might have been updated (e.g., daily exports)
        if (params.force_refresh) {
            await ctx.datasource.sales_csv.refresh();
            await ctx.datasource.products_excel.refresh();
        }
        
        // Query CSV file directly with SQL
        const salesSummary = await ctx.datasource.sales_csv.query(
            `SELECT 
                product_category,
                SUM(amount) as total_sales,
                COUNT(*) as transaction_count,
                AVG(amount) as avg_sale
             FROM data 
             WHERE sale_date >= '${params.start_date}'
             GROUP BY product_category
             ORDER BY total_sales DESC`
        );
        
        // Join CSV with Parquet file using DuckDB
        const enrichedSales = await ctx.datasource.sales_csv.query(`
            SELECT 
                s.*,
                c.customer_name,
                c.customer_segment
            FROM data s
            JOIN read_parquet('${ctx.datasource.customers_parquet.path}') c
            ON s.customer_id = c.id
            WHERE s.amount > 1000
        `);
        
        // Query Excel file for product data
        const productPrices = await ctx.datasource.products_excel.query(
            "SELECT product_id, list_price, cost FROM data"
        );
        
        // Process with Polars for complex calculations
        const salesDf = ctx.DataFrame(enrichedSales);
        const productDf = ctx.DataFrame(productPrices);
        
        const profitAnalysis = salesDf
            .join(productDf, "product_id")
            .withColumn("profit", 
                col("amount") - col("cost")
            )
            .groupBy("customer_segment")
            .agg({
                total_profit: col("profit").sum(),
                avg_profit_margin: col("profit").mean()
            });
        
        // Store results in DuckDB for future analysis
        await ctx.duckdb.saveDataFrame(profitAnalysis, "profit_analysis_" + params.start_date);
        
        return {
            sales_summary: salesSummary,
            top_customers: enrichedSales.slice(0, 10),
            profit_by_segment: profitAnalysis.collect()
        };
    }
}
```

### Auto-Refresh Pattern for Regularly Updated Files
Handle CSV/Excel files that are updated by external processes:

```javascript
export default {
    title: "Monitor daily export files with auto-refresh",
    schedule: {
        cron: "*/30 * * * *",  // Every 30 minutes
        timezone: "UTC"
    },
    dependencies: {
        datasources: ["daily_sales_csv", "inventory_excel", "s3_transactions"]
    },
    
    run: async function(ctx) {
        // Check if files have been updated
        const salesLastModified = await ctx.datasource.daily_sales_csv.getLastModified();
        const inventoryLastModified = await ctx.datasource.inventory_excel.getLastModified();
        
        // Refresh if files are newer than last check
        const lastCheck = ctx.metadata.get('last_check_time') || new Date(0);
        
        if (salesLastModified > lastCheck) {
            ctx.log("Sales CSV updated, refreshing...");
            await ctx.datasource.daily_sales_csv.refresh();
        }
        
        if (inventoryLastModified > lastCheck) {
            ctx.log("Inventory Excel updated, refreshing...");
            await ctx.datasource.inventory_excel.refresh();
        }
        
        // For S3 files, always refresh (or use cache TTL)
        ctx.datasource.s3_transactions.setCacheTTL(1800); // 30 min cache
        
        // Query refreshed data
        const dailySales = await ctx.datasource.daily_sales_csv.query(
            "SELECT SUM(amount) as total, COUNT(*) as transactions FROM data WHERE date = CURRENT_DATE"
        );
        
        const lowStock = await ctx.datasource.inventory_excel.query(
            "SELECT product_id, product_name, quantity FROM data WHERE quantity < reorder_point"
        );
        
        const hourlyTransactions = await ctx.datasource.s3_transactions.query(
            "SELECT DATE_TRUNC('hour', timestamp) as hour, COUNT(*) as count FROM data GROUP BY hour ORDER BY hour"
        );
        
        // Save checkpoint
        ctx.metadata.set('last_check_time', new Date());
        
        // Alert if significant changes
        const previousTotal = ctx.metadata.get('previous_sales_total') || 0;
        if (dailySales[0].total > previousTotal * 1.5) {
            await ctx.createAlert('high', 'Sales spike detected', {
                current: dailySales[0].total,
                previous: previousTotal
            });
        }
        
        ctx.metadata.set('previous_sales_total', dailySales[0].total);
        
        return {
            sales: dailySales[0],
            low_stock_items: lowStock.length,
            hourly_pattern: hourlyTransactions
        };
    }
}
```

### Google Sheets Integration Pattern
Sync and analyze data from Google Sheets:

```javascript
export default {
    title: "Process budget data from Google Sheets",
    schedule: {
        cron: "0 9 * * MON",  // Every Monday at 9 AM
        timezone: "America/New_York"
    },
    dependencies: {
        datasources: ["budget_sheet", "actuals_sheet", "postgres_main"]
    },
    
    run: async function(ctx) {
        // Force refresh to get latest data from Google Sheets
        await ctx.datasource.budget_sheet.refresh();
        await ctx.datasource.actuals_sheet.refresh();
        
        // Query budget data with SQL
        const budgetByDept = await ctx.datasource.budget_sheet.query(
            `SELECT 
                department,
                fiscal_quarter,
                SUM(budget_amount) as total_budget
             FROM data
             WHERE fiscal_year = 2024
             GROUP BY department, fiscal_quarter`
        );
        
        // Get specific range for detailed analysis
        const q1Details = await ctx.datasource.budget_sheet.getRange("Q1!A1:Z500");
        
        // Query actuals from another sheet
        const actualSpending = await ctx.datasource.actuals_sheet.query(
            `SELECT 
                department,
                SUM(amount) as total_spent
             FROM data
             WHERE date >= '2024-01-01' AND date < '2024-04-01'
             GROUP BY department`
        );
        
        // Compare budget vs actuals
        const comparison = await ctx.duckdb.query(`
            WITH budget AS (
                SELECT * FROM (${JSON.stringify(budgetByDept)})
            ),
            actuals AS (
                SELECT * FROM (${JSON.stringify(actualSpending)})
            )
            SELECT 
                b.department,
                b.total_budget,
                COALESCE(a.total_spent, 0) as total_spent,
                b.total_budget - COALESCE(a.total_spent, 0) as remaining,
                ROUND(COALESCE(a.total_spent, 0) * 100.0 / b.total_budget, 2) as pct_used
            FROM budget b
            LEFT JOIN actuals a ON b.department = a.department
            ORDER BY pct_used DESC
        `);
        
        // Write results back to PostgreSQL
        for (const row of comparison) {
            await ctx.datasource.postgres_main.query(
                `INSERT INTO budget_tracking 
                 (department, budget, spent, remaining, pct_used, updated_at)
                 VALUES ($1, $2, $3, $4, $5, NOW())
                 ON CONFLICT (department) DO UPDATE SET
                     budget = EXCLUDED.budget,
                     spent = EXCLUDED.spent,
                     remaining = EXCLUDED.remaining,
                     pct_used = EXCLUDED.pct_used,
                     updated_at = NOW()`,
                [row.department, row.total_budget, row.total_spent, 
                 row.remaining, row.pct_used]
            );
        }
        
        return {
            departments_analyzed: comparison.length,
            top_spenders: comparison.slice(0, 5),
            sync_timestamp: new Date().toISOString()
        };
    }
}
```

### S3 + Excel Processing Pattern
Processing large files from object storage:

```javascript
export default {
    title: "Process monthly Excel reports from S3",
    dependencies: {
        datasources: ["s3_reports", "postgres_main"]
    },
    
    run: async function(ctx, params) {
        // List all Excel files in S3
        const files = await ctx.datasource.s3_reports.list("/monthly/2024/");
        
        const results = [];
        for (const file of files) {
            if (file.key.endsWith('.xlsx')) {
                // Stream and process large Excel file
                await ctx.datasource.s3_reports.stream(file.key, {
                    format: 'excel',
                    decompress: 'gzip',  // If compressed
                    onChunk: async (rows) => {
                        // Process each chunk with Polars
                        const df = ctx.DataFrame(rows);
                        const summary = df
                            .groupBy("region")
                            .agg({ 
                                total_sales: col("amount").sum(),
                                order_count: col("order_id").count()
                            });
                        
                        // Append to DuckDB table
                        await ctx.duckdb.saveDataFrame(summary, 
                            `regional_sales_${params.month}`, 
                            { mode: 'append' }
                        );
                    }
                });
                
                results.push({ file: file.key, processed: true });
            }
        }
        
        // Aggregate final results
        const totals = await ctx.duckdb.query(`
            SELECT region, SUM(total_sales) as total, SUM(order_count) as orders
            FROM regional_sales_${params.month}
            GROUP BY region
        `);
        
        return { 
            processed_files: results.length,
            regional_summary: totals
        };
    }
}
```

### MCP Datasource Management Pattern
Using MCP tools for dynamic datasource management within analyses:

```javascript
export default {
    title: "Dynamic multi-datasource analysis with MCP",
    dependencies: {
        datasources: ["postgres_main"]
    },
    
    run: async function(ctx, params) {
        // Dynamically add a new datasource during analysis
        const newDatasourceResult = await ctx.mcp.callTool('mcp__data-analysis__datasource_add', {
            name: `temp_analysis_${Date.now()}`,
            source_type: 'postgresql',
            config: params.additional_db_url
        });
        
        const tempDatasourceId = JSON.parse(newDatasourceResult).datasource_id;
        
        // Test connection to ensure it's working
        const connectionTest = await ctx.mcp.callTool('mcp__data-analysis__connection_test', {
            datasource_id: tempDatasourceId
        });
        
        if (!JSON.parse(connectionTest).success) {
            throw new Error('Failed to connect to additional datasource');
        }
        
        // Inspect the new datasource schema
        const schemaInspection = await ctx.mcp.callTool('mcp__data-analysis__datasource_inspect', {
            datasource_id: tempDatasourceId
        });
        
        const schemaInfo = JSON.parse(schemaInspection);
        
        // Get detailed schema for specific tables
        const userTableSchema = await ctx.mcp.callTool('mcp__data-analysis__schema_get', {
            datasource_id: tempDatasourceId,
            table_name: 'users',
            use_cache: false
        });
        
        // Search for tables containing 'transaction' in the name
        const transactionTables = await ctx.mcp.callTool('mcp__data-analysis__schema_search', {
            datasource_id: tempDatasourceId,
            search_term: 'transaction'
        });
        
        // Query data from the new datasource
        const userData = await ctx.mcp.callTool('mcp__data-analysis__datasource_query', {
            datasource_id: tempDatasourceId,
            query: 'SELECT id, email, created_at FROM users WHERE created_at >= $1',
            limit: 1000
        });
        
        const users = JSON.parse(userData).data;
        
        // Query main datasource for comparison
        const mainUsers = await ctx.datasource.postgres_main.query(
            "SELECT id, email, created_at FROM users WHERE created_at >= $1", 
            [params.start_date]
        );
        
        // Cross-datasource analysis using DuckDB
        await ctx.duckdb.load('temp_datasource', 
            `SELECT * FROM (${JSON.stringify(users)}) AS temp_users`, 
            'temp_users');
        await ctx.duckdb.load('postgres_main', 
            `SELECT * FROM (${JSON.stringify(mainUsers)}) AS main_users`, 
            'main_users');
        
        const crossAnalysis = await ctx.duckdb.query(`
            SELECT 
                'main' as source, COUNT(*) as user_count, MIN(created_at) as earliest, MAX(created_at) as latest
            FROM main_users
            UNION ALL
            SELECT 
                'temp' as source, COUNT(*) as user_count, MIN(created_at) as earliest, MAX(created_at) as latest  
            FROM temp_users
        `);
        
        // Get statistics about the temp datasource
        const datasourceStats = await ctx.mcp.callTool('mcp__data-analysis__schema_stats', {
            datasource_id: tempDatasourceId
        });
        
        // Clean up: Remove temporary datasource
        await ctx.mcp.callTool('mcp__data-analysis__datasource_remove', {
            datasource_id: tempDatasourceId
        });
        
        return {
            schema_discovery: {
                total_tables: schemaInfo.table_count,
                transaction_tables: JSON.parse(transactionTables).tables,
                user_table_columns: JSON.parse(userTableSchema).columns?.length || 0
            },
            data_comparison: {
                cross_analysis: crossAnalysis,
                temp_datasource_stats: JSON.parse(datasourceStats)
            },
            users_analyzed: {
                main_db_count: mainUsers.length,
                temp_db_count: users.length,
                total_processed: mainUsers.length + users.length
            },
            cleanup_status: "Temporary datasource removed successfully"
        };
    }
}
```

### API Integration Pattern
Fetching and processing data from REST APIs:

```javascript
export default {
    title: "Sync customer data from CRM API",
    dependencies: {
        datasources: ["api_crm", "postgres_main"]
    },
    
    run: async function(ctx, params) {
        // Paginate through all customers from API
        const customers = await ctx.datasource.api_crm.paginate('/api/v2/customers', {
            pageParam: 'page',
            limitParam: 'per_page',
            limit: 100,
            dataPath: 'data',
            nextPagePath: 'links.next'
        });
        
        // Enrich with additional API calls
        const enriched = [];
        for (const customer of customers) {
            // Get detailed customer info
            const details = await ctx.datasource.api_crm.get(
                `/api/v2/customers/${customer.id}/details`
            );
            
            enriched.push({
                ...customer,
                ...details,
                sync_date: new Date().toISOString()
            });
        }
        
        // Store in DuckDB for analysis
        const df = ctx.DataFrame(enriched);
        await ctx.duckdb.saveDataFrame(df, "customers_synced");
        
        // Update PostgreSQL with latest data
        for (const customer of enriched) {
            await ctx.datasource.postgres_main.query(
                `INSERT INTO customers (id, name, email, synced_at) 
                 VALUES ($1, $2, $3, $4) 
                 ON CONFLICT (id) DO UPDATE 
                 SET name = EXCLUDED.name, 
                     email = EXCLUDED.email, 
                     synced_at = EXCLUDED.synced_at`,
                [customer.id, customer.name, customer.email, customer.sync_date]
            );
        }
        
        return {
            synced_count: enriched.length,
            last_sync: new Date().toISOString()
        };
    }
}
```

### Multi-Source Real-time Dashboard
Combining multiple datasource types for a real-time dashboard:

```javascript
export default {
    title: "Real-time operations dashboard",
    schedule: {
        cron: "*/5 * * * *",  // Every 5 minutes
        timezone: "UTC"
    },
    dependencies: {
        datasources: [
            "postgres_main",
            "clickhouse_events", 
            "s3_logs",
            "api_monitoring",
            "graphql_analytics"
        ]
    },
    
    run: async function(ctx) {
        const now = new Date();
        const fiveMinutesAgo = new Date(now - 5 * 60 * 1000);
        
        // 1. Get current system metrics from monitoring API
        const systemHealth = await ctx.datasource.api_monitoring.get('/metrics/current');
        
        // 2. Query real-time events from ClickHouse
        const events = await ctx.datasource.clickhouse_events.query(
            `SELECT event_type, COUNT(*) as count, AVG(duration_ms) as avg_duration
             FROM events 
             WHERE timestamp >= ? AND timestamp < ?
             GROUP BY event_type`,
            [fiveMinutesAgo, now]
        );
        
        // 3. Check for new error logs in S3
        const errorLogs = await ctx.datasource.s3_logs.list('/errors/', {
            maxKeys: 100
        });
        
        const recentErrors = errorLogs.filter(
            log => new Date(log.lastModified) >= fiveMinutesAgo
        );
        
        // 4. Get user activity from GraphQL
        const activity = await ctx.datasource.graphql_analytics.query({
            query: `
                query RecentActivity($since: DateTime!) {
                    activeUsers(since: $since) {
                        count
                        topActions {
                            action
                            count
                        }
                    }
                }
            `,
            variables: { since: fiveMinutesAgo.toISOString() }
        });
        
        // 5. Get business metrics from PostgreSQL
        const metrics = await ctx.datasource.postgres_main.query(
            `SELECT 
                COUNT(*) FILTER (WHERE created_at >= $1) as new_orders,
                SUM(amount) FILTER (WHERE created_at >= $1) as revenue,
                AVG(processing_time) as avg_processing_time
             FROM orders 
             WHERE created_at >= $1`,
            [fiveMinutesAgo]
        );
        
        // 6. Store snapshot in DuckDB
        const snapshot = {
            timestamp: now.toISOString(),
            system_health: systemHealth,
            event_counts: events,
            error_count: recentErrors.length,
            active_users: activity.activeUsers.count,
            business_metrics: metrics[0]
        };
        
        await ctx.duckdb.exec(`
            INSERT INTO dashboard_snapshots 
            VALUES ('${snapshot.timestamp}', '${JSON.stringify(snapshot)}')
        `);
        
        // 7. Check for alerts
        const alerts = [];
        if (systemHealth.cpu_usage > 80) {
            alerts.push({ type: 'high_cpu', value: systemHealth.cpu_usage });
        }
        if (recentErrors.length > 10) {
            alerts.push({ type: 'error_spike', count: recentErrors.length });
        }
        
        return {
            timestamp: now.toISOString(),
            interval: '5_minutes',
            metrics: snapshot,
            alerts: alerts
        };
    }
}
```

## Scheduling

### Cron Syntax
Analyses can be scheduled using standard cron expressions:

```
┌───────────── minute (0-59)
│ ┌───────────── hour (0-23)
│ │ ┌───────────── day of month (1-31)
│ │ │ ┌───────────── month (1-12)
│ │ │ │ ┌───────────── day of week (0-7, 0 and 7 are Sunday)
│ │ │ │ │
* * * * *
```

### Common Schedule Examples

```javascript
// Daily reports
schedule: {
    cron: "0 9 * * *",      // Every day at 9 AM
    timezone: "America/New_York"
}

// Weekly summaries
schedule: {
    cron: "0 10 * * 1",     // Every Monday at 10 AM
    timezone: "UTC"
}

// Monthly aggregations
schedule: {
    cron: "0 0 1 * *",      // First day of month at midnight
    timezone: "UTC"
}

// Every 4 hours
schedule: {
    cron: "0 */4 * * *",    // At minute 0 past every 4th hour
}

// Business days only
schedule: {
    cron: "0 9 * * 1-5",    // Mon-Fri at 9 AM
}
```

### Scheduled ETL Pattern

```javascript
export default {
    title: "Daily data sync and aggregation",
    
    schedule: {
        cron: "0 1 * * *",  // Daily at 1 AM
        timezone: "UTC",
        enabled: true,
        retry_on_failure: true,  // Optional: retry if fails
        max_retries: 3
    },
    
    parameters: {
        // Parameters can use special values for scheduled runs
        date: {
            type: "date",
            required: true,
            default: "yesterday"  // Special value
        },
        mode: {
            type: "select",
            required: true,
            default: "incremental",
            options: [
                { value: "full", label: "Full refresh" },
                { value: "incremental", label: "Incremental update" }
            ]
        }
    },
    
    dependencies: {
        datasources: ["production_db", "analytics_db"]
    },
    
    run: async function(ctx, params) {
        // Handle special parameter values
        const targetDate = params.date === "yesterday" 
            ? new Date(Date.now() - 86400000).toISOString().split('T')[0]
            : params.date;
        
        // Load yesterday's data
        await ctx.duckdb.load(
            "production_db",
            `SELECT * FROM events WHERE date = '${targetDate}'`,
            `events_${targetDate.replace(/-/g, '_')}`
        );
        
        // Process and aggregate
        await ctx.duckdb.exec(`
            CREATE OR REPLACE TABLE daily_summary_${targetDate.replace(/-/g, '_')} AS
            SELECT 
                user_segment,
                COUNT(*) as event_count,
                COUNT(DISTINCT user_id) as unique_users
            FROM events_${targetDate.replace(/-/g, '_')}
            GROUP BY user_segment
        `);
        
        // Update materialized view
        await ctx.duckdb.exec(`
            CREATE OR REPLACE VIEW latest_daily_summary AS
            SELECT * FROM daily_summary_${targetDate.replace(/-/g, '_')}
        `);
        
        return {
            date_processed: targetDate,
            status: "success",
            next_run: ctx.schedule.next_run  // Next scheduled execution
        };
    }
}
```

### Parameter Defaults for Scheduling

Special default values for scheduled runs:

```javascript
parameters: {
    date: {
        type: "date",
        default: "yesterday"     // Previous day
        // or: "today"           // Current day
        // or: "last_week"       // Previous week
        // or: "last_month"      // Previous month
        // or: "current_month"   // Current month
    },
    
    time_range: {
        type: "object",
        default: {
            start: "week_start",  // Start of current week
            end: "week_end"       // End of current week
        }
    }
}
```

## Advanced Patterns

### Solving Complex Scenarios with Simple Approaches

The system is more capable than it initially appears. Here's how to handle complex scenarios using the existing tools creatively:

#### 1. Conditional Execution
**Challenge:** Need conditional scheduling based on data conditions  
**Solution:** Schedule at the shortest needed interval, let the script decide whether to execute

```javascript
export default {
    schedule: {
        cron: "*/15 * * * *",  // Run every 15 minutes
    },
    
    run: async function(ctx) {
        // Check condition first
        const marketData = await ctx.datasource.postgres.query("SELECT volatility FROM market_stats");
        if (marketData[0].volatility < 0.2) {
            return { skipped: true, reason: "Volatility below threshold" };
        }
        
        // Continue with actual work
        // ...
    }
}
```

#### 2. Managing Dependencies Between Analyses
**Challenge:** Need to wait for other analyses to complete  
**Solution:** Poll until ready

```javascript
// Wait for dependent analysis
async function waitForAnalysis(ctx, analysisName, params, maxAttempts = 60) {
    for (let i = 0; i < maxAttempts; i++) {
        const result = await ctx.runAnalysis(analysisName, params);
        if (result.success) return result;
        await ctx.sleep(5000); // Wait 5 seconds
    }
    throw new Error(`Timeout waiting for ${analysisName}`);
}

// Use it
const marketData = await waitForAnalysis(ctx, "market_data_ingestion", params);
```

#### 3. Handling Large Result Sets
**Challenge:** Results exceed 10MB limit  
**Solution:** Store in DuckDB, return reference

```javascript
run: async function(ctx) {
    // Process large dataset
    await ctx.duckdb.exec(`
        CREATE OR REPLACE TABLE analysis_results_${Date.now()} AS
        SELECT * FROM large_dataset WHERE conditions...
    `);
    
    // Return reference and summary
    const stats = await ctx.duckdb.query("SELECT COUNT(*), AVG(value) FROM analysis_results_...");
    
    return {
        result_table: `analysis_results_${Date.now()}`,
        row_count: stats[0].count,
        summary: stats[0]
    };
}
```

#### 4. State Management Across Retries
**Challenge:** Need persistent state for long-running jobs  
**Solution:** Use ctx.metadata for state tracking

```javascript
// Save checkpoint to metadata
async function saveCheckpoint(ctx, key, value) {
    ctx.metadata.set(key, value);
}

// Get checkpoint from metadata
function getCheckpoint(ctx, key) {
    return ctx.metadata.get(key);
}

// Use in processing
const lastProcessed = getCheckpoint(ctx, 'last_batch') || 0;
for (let batch = lastProcessed; batch < totalBatches; batch++) {
    await processBatch(batch);
    saveCheckpoint(ctx, 'last_batch', batch);
}

// Metadata persists across retries and can be queried later
```

#### 5. Queue Processing Pattern
**Challenge:** Need to process items with rate limiting  
**Solution:** Use DuckDB as a queue

```javascript
// Create queue table
await ctx.duckdb.exec(`
    CREATE TABLE IF NOT EXISTS processing_queue (
        id INTEGER PRIMARY KEY,
        item_data VARCHAR,
        status VARCHAR DEFAULT 'pending',
        processed_at TIMESTAMP,
        result VARCHAR
    )
`);

// Track progress in metadata
const lastProcessedId = ctx.metadata.get('last_processed_id') || 0;

// Process queue with rate limiting
const batchSize = 100;
const items = await ctx.duckdb.query(`
    SELECT * FROM processing_queue 
    WHERE status = 'pending' AND id > ${lastProcessedId}
    LIMIT ${batchSize}
`);

for (const item of items) {
    // Process item
    const result = await processItem(item);
    
    // Update status
    await ctx.duckdb.exec(`
        UPDATE processing_queue 
        SET status = 'completed', processed_at = NOW(), result = '${result}'
        WHERE id = ${item.id}
    `);
    
    // Save progress
    ctx.metadata.set('last_processed_id', item.id);
    
    await ctx.sleep(600); // Rate limit: 100 per minute
}
```

#### 6. Transaction Patterns
**Challenge:** Need all-or-nothing operations  
**Solution:** Use DuckDB transactions

```javascript
run: async function(ctx) {
    try {
        await ctx.duckdb.exec("BEGIN TRANSACTION");
        
        await ctx.duckdb.exec("INSERT INTO table1 ...");
        await ctx.duckdb.exec("UPDATE table2 ...");
        await ctx.duckdb.exec("DELETE FROM table3 ...");
        
        await ctx.duckdb.exec("COMMIT");
        return { success: true };
    } catch (error) {
        await ctx.duckdb.exec("ROLLBACK");
        throw error;
    }
}
```

#### 7. External System Integration
**Challenge:** Need to integrate with external APIs/systems  
**Solution:** Use DuckDB tables as interface

```javascript
// Analysis 1: Write requests to queue
await ctx.duckdb.exec(`
    INSERT INTO external_api_queue (endpoint, payload, status)
    VALUES ('/api/credit-score', '${JSON.stringify(data)}', 'pending')
`);

// Separate service processes queue and writes results back
// Analysis 2: Read results
const results = await ctx.duckdb.query(`
    SELECT * FROM external_api_queue 
    WHERE status = 'completed' 
    AND created_at > NOW() - INTERVAL '1 hour'
`);
```

#### 8. Complex DAG Workflows
**Challenge:** Analyses depend on each other in complex ways  
**Solution:** Use metadata to track completion status

```javascript
// Each analysis marks its completion
ctx.metadata.set('credit_risk_completed', true);
ctx.metadata.set('credit_risk_run_id', ctx.metadata.get('run_id'));

// Store results in DuckDB
await ctx.duckdb.exec(`
    CREATE OR REPLACE TABLE credit_risk_results AS ...
`);

// Dependent analyses check prerequisites
async function waitForDependencies(ctx, dependencies) {
    for (const dep of dependencies) {
        while (!ctx.metadata.get(`${dep}_completed`)) {
            await ctx.sleep(10000); // Wait 10 seconds
        }
    }
}

// Use it
await waitForDependencies(ctx, ['credit_risk', 'market_risk']);
```

#### 9. Point-in-Time Consistency
**Challenge:** Need consistent snapshot across datasources  
**Solution:** Use timestamp parameter

```javascript
run: async function(ctx) {
    const snapshotTime = new Date().toISOString();
    
    // All queries use same timestamp
    const trades = await ctx.datasource.postgres.query( 
        "SELECT * FROM trades WHERE created_at <= $1", [snapshotTime]);
    const prices = await ctx.datasource.mysql.query( 
        "SELECT * FROM prices WHERE timestamp <= ?", [snapshotTime]);
    
    // Store snapshot reference in metadata
    ctx.metadata.set('snapshot_time', snapshotTime);
    ctx.metadata.set('trade_count', trades.length);
    ctx.metadata.set('price_count', prices.length);
    
    return { snapshot_time: snapshotTime };
}
```

#### 10. Alerting Pattern
**Challenge:** Need to send alerts on conditions  
**Solution:** Write to alert queue, external service handles delivery

```javascript
async function createAlert(ctx, severity, message, metadata = {}) {
    await ctx.duckdb.exec(`
        INSERT INTO alert_queue (
            severity, message, metadata, created_at, status
        ) VALUES (
            '${severity}', 
            '${message}', 
            '${JSON.stringify(metadata)}',
            NOW(),
            'pending'
        )
    `);
}

// Use it
if (riskScore > threshold) {
    await createAlert(ctx, 'critical', 'Risk threshold exceeded', {
        risk_score: riskScore,
        threshold: threshold,
        analysis_run: ctx.metadata.get('run_id')
    });
}
```

### Best Practices

1. **Use Each Tool for Its Strength**
   - `ctx.metadata` → State management, checkpoints, small values
   - DuckDB → Large data storage, historical data, transformed data, etc.
   - Return object → Summary and references only (< 10MB)

2. **Let Analyses Handle Their Own Logic**
   - Don't over-engineer orchestration
   - Use polling for dependencies
   - Self-throttle for rate limits
   - Check conditions in the script

3. **Data Storage Pattern**
   - Query data from datasources
   - Process and transform
   - Store results in DuckDB (persistent)
   - Track progress in metadata
   - Return summary/reference

4. **Incremental Processing**
   ```javascript
   // Use metadata to track where you left off
   const lastId = ctx.metadata.get('last_processed_id') || 0;
   
   // Process next batch
   const batch = await ctx.datasource.postgres.query(
       "SELECT * FROM items WHERE id > $1 ORDER BY id LIMIT 1000", 
       [lastId]);
   
   // Update checkpoint
   if (batch.length > 0) {
       ctx.metadata.set('last_processed_id', batch[batch.length-1].id);
   }
   ```

## Result Storage

### Result Size Limit
All analysis results must be **less than 10MB** when serialized to JSON. For larger datasets:
- Use DuckDB to save to a datasource
- Return only summaries and metadata
- Store references to output tables

### Storage Architecture

```
/data/analysis_results/
├── 2024/
│   ├── 01/
│   │   ├── 15/
│   │   │   ├── abc-123.json.gz    # Compressed result < 10MB
│   │   │   └── def-456.json.gz
│   │   └── 16/
│   │       └── ghi-789.json.gz
│   └── 02/
│       └── ...
└── metadata.db  # SQLite for quick lookups
```

### Result Storage Implementation

```rust
pub struct ResultStorage {
    max_size: usize = 10 * 1024 * 1024,  // 10MB limit
    
    async fn store_result(&self, run_id: Uuid, result: Value) -> Result<()> {
        // Validate size
        let json = serde_json::to_vec(&result)?;
        if json.len() > self.max_size {
            return Err(anyhow!(
                "Result size {}MB exceeds 10MB limit. Use DuckDB for large datasets.",
                json.len() / 1_048_576
            ));
        }
        
        // Compress and store
        let compressed = zstd::encode_all(&json[..], 3)?;
        let path = self.get_path(run_id);
        fs::write(path, compressed).await?;
        
        Ok(())
    }
}
```

## Security Model

1. **Sandboxed Execution** - QuickJS sandbox with no host access
2. **Read-only Queries** - Only SELECT queries allowed on external datasources
3. **Resource Limits** - Memory (256MB QuickJS, 8GB DuckDB), execution time (30s default), result size (10MB)
4. **Datasource Whitelist** - Only declared datasources accessible
5. **No External Access** - No file system or network access (except DuckDB storage)
6. **Project Isolation** - Each project has its own persistent DuckDB database
7. **DuckDB Permissions** - Full read/write within project's DuckDB, isolated from other projects

## Performance Characteristics

- **Script startup**: 1-5ms (QuickJS)
- **Simple queries**: Native database speed + minimal overhead
- **Polars operations**: 10-100x faster than pandas equivalent
- **DuckDB operations**: Columnar storage with excellent compression
- **Memory usage**: Typically under 10MB for simple queries
- **Large dataset processing**: DuckDB and streaming prevent OOM on large datasets
- **Schedule checking**: Every 60 seconds for all schedules
- **Scheduled execution**: Same performance as manual execution

## Key Benefits

1. **Simple API** - Just `query()` and `forEach()` for most use cases
2. **Fast Execution** - Sub-10ms startup for quick analyses
3. **Powerful When Needed** - Polars available for complex operations
4. **Modular** - Analyses can be composed and reused
5. **Safe** - Strong sandboxing with clear security boundaries
6. **User-Friendly** - Dynamic parameter options with dropdowns

## Frontend Integration

### User Workflow
1. **Browse** - User views list of available analyses
2. **Select** - User clicks on an analysis to view details
3. **Configure** - Frontend shows form with parameters
   - Static parameters render immediately
   - Dynamic parameters fetch options from backend
   - Dependent parameters refresh when dependencies change
4. **Execute** - User submits form, receives job_id
5. **Monitor** - Frontend polls job status
6. **View Results** - Display results when complete

### Frontend Implementation

```typescript
// Types
interface Analysis {
  id: string;
  title: string;
  parameters: Record<string, ParameterDef>;
}

interface ParameterDef {
  type: 'text' | 'number' | 'date' | 'select' | 'boolean';
  required: boolean;
  description?: string;
  has_dynamic_options?: boolean;
  depends_on?: string[];
  options?: SelectOption[];  // For static options
}

interface SelectOption {
  value: string;
  label: string;
  options?: SelectOption[];  // For grouped options
}

// API Service
class AnalysisService {
  async listAnalyses(): Promise<Analysis[]> {
    return fetch('/api/analysis').then(r => r.json());
  }

  async getAnalysis(id: string): Promise<Analysis> {
    return fetch(`/api/analysis/${id}`).then(r => r.json());
  }

  async getParameterOptions(
    analysisId: string, 
    paramName: string, 
    currentParams: any
  ): Promise<SelectOption[]> {
    const response = await fetch(
      `/api/analysis/${analysisId}/parameters/${paramName}/options`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ current_params: currentParams })
      }
    );
    const data = await response.json();
    return data.options;
  }

  async execute(analysisId: string, parameters: any): Promise<{ job_id: string }> {
    const response = await fetch(`/api/analysis/${analysisId}/execute`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ parameters })
    });
    return response.json();
  }

  async getJobStatus(jobId: string): Promise<JobStatus> {
    return fetch(`/api/analysis/jobs/${jobId}`).then(r => r.json());
  }
}

// React Component Example
function AnalysisForm({ analysisId }: { analysisId: string }) {
  const [analysis, setAnalysis] = useState<Analysis | null>(null);
  const [formValues, setFormValues] = useState<Record<string, any>>({});
  const [paramOptions, setParamOptions] = useState<Record<string, SelectOption[]>>({});
  const [loading, setLoading] = useState<Record<string, boolean>>({});

  // Load analysis metadata
  useEffect(() => {
    service.getAnalysis(analysisId).then(setAnalysis);
  }, [analysisId]);

  // Load dynamic options for parameters
  useEffect(() => {
    if (!analysis) return;

    Object.entries(analysis.parameters).forEach(async ([name, param]) => {
      if (param.has_dynamic_options) {
        // Check if we need to reload based on dependencies
        const shouldReload = !param.depends_on || 
          param.depends_on.some(dep => formValues[dep] !== undefined);

        if (shouldReload) {
          setLoading(prev => ({ ...prev, [name]: true }));
          
          const options = await service.getParameterOptions(
            analysisId,
            name,
            formValues
          );
          
          setParamOptions(prev => ({ ...prev, [name]: options }));
          setLoading(prev => ({ ...prev, [name]: false }));
        }
      }
    });
  }, [analysis, formValues]);

  // Render parameter based on type
  function renderParameter(name: string, param: ParameterDef) {
    switch (param.type) {
      case 'select':
        const options = param.has_dynamic_options 
          ? paramOptions[name] || []
          : param.options || [];

        return (
          <Select
            value={formValues[name]}
            onChange={(value) => setFormValues({ ...formValues, [name]: value })}
            disabled={loading[name]}
            required={param.required}
          >
            {options.map(opt => (
              <Option key={opt.value} value={opt.value}>
                {opt.label}
              </Option>
            ))}
          </Select>
        );

      case 'date':
        return (
          <DatePicker
            value={formValues[name]}
            onChange={(date) => setFormValues({ ...formValues, [name]: date })}
            required={param.required}
          />
        );

      // ... other types
    }
  }

  async function handleSubmit() {
    const { job_id } = await service.execute(analysisId, formValues);
    // Redirect to job monitoring view
    navigate(`/analysis/jobs/${job_id}`);
  }

  return (
    <form onSubmit={handleSubmit}>
      {analysis && Object.entries(analysis.parameters).map(([name, param]) => (
        <div key={name}>
          <label>{param.description || name}</label>
          {renderParameter(name, param)}
        </div>
      ))}
      <button type="submit">Run Analysis</button>
    </form>
  );
}

// Job Monitoring Component
function JobMonitor({ jobId }: { jobId: string }) {
  const [status, setStatus] = useState<JobStatus | null>(null);

  useEffect(() => {
    const interval = setInterval(async () => {
      const jobStatus = await service.getJobStatus(jobId);
      setStatus(jobStatus);
      
      if (jobStatus.status === 'completed' || jobStatus.status === 'failed') {
        clearInterval(interval);
      }
    }, 2000); // Poll every 2 seconds

    return () => clearInterval(interval);
  }, [jobId]);

  if (!status) return <div>Loading...</div>;

  return (
    <div>
      <h2>Status: {status.status}</h2>
      {status.logs && (
        <div>
          <h3>Logs:</h3>
          {status.logs.map((log, i) => <p key={i}>{log}</p>)}
        </div>
      )}
      {status.status === 'completed' && (
        <div>
          <h3>Results:</h3>
          <ResultsDisplay data={status.result} />
        </div>
      )}
      {status.status === 'failed' && (
        <div>Error: {status.error}</div>
      )}
    </div>
  );
}
```

## Implementation Stack

- **Runtime**: QuickJS via rquickjs
- **Data Processing**: DuckDB for ETL, Polars for in-memory operations
- **Backend**: Rust with existing DataSource trait
- **Databases**: PostgreSQL, MySQL, SQLite, ClickHouse
- **Security**: Sandboxed execution with resource limits
- **Frontend**: React with TypeScript
- **Result Storage**: Compressed JSON files with 10MB limit

## Backend Implementation

### Scheduler Service

Handles automatic execution of scheduled analyses:

```rust
use cron::Schedule;
use chrono::{DateTime, Utc};
use tokio::time::{interval, Duration};

pub struct SchedulerService {
    db: PgPool,
    executor: AnalysisExecutor,
}

impl SchedulerService {
    pub async fn start(self) {
        // Check every minute for scheduled analyses
        let mut ticker = interval(Duration::from_secs(60));
        
        loop {
            ticker.tick().await;
            self.check_and_execute_scheduled().await;
        }
    }
    
    async fn check_and_execute_scheduled(&self) {
        // Get all enabled schedules
        let schedules = sqlx::query!(
            "SELECT s.*, a.name as analysis_name
             FROM analysis_schedules s
             JOIN analyses a ON s.analysis_id = a.id
             WHERE s.enabled = true
               AND (s.next_run_at IS NULL OR s.next_run_at <= NOW())"
        )
        .fetch_all(&self.db)
        .await?;
        
        for schedule in schedules {
            // Parse cron expression
            let cron: Schedule = schedule.cron_expression.parse()?;
            
            // Calculate next run time
            let next = cron.upcoming(Utc).next().unwrap();
            
            // Should we run now?
            if schedule.next_run_at.is_none() || Utc::now() >= schedule.next_run_at {
                // Execute the analysis
                self.execute_scheduled_analysis(schedule).await;
                
                // Update next run time
                sqlx::query!(
                    "UPDATE analysis_schedules 
                     SET next_run_at = $1, last_run_at = NOW()
                     WHERE id = $2",
                    next.naive_utc(),
                    schedule.id
                ).execute(&self.db).await?;
            }
        }
    }
    
    async fn execute_scheduled_analysis(&self, schedule: Schedule) {
        // Get analysis with default parameters
        let analysis = self.get_analysis(schedule.analysis_id).await?;
        
        // Resolve parameter defaults (e.g., "yesterday" -> actual date)
        let params = self.resolve_parameter_defaults(analysis.parameters).await;
        
        // Create run record
        let run_id = sqlx::query!(
            "INSERT INTO analysis_runs 
             (analysis_id, analysis_version, status, parameters, 
              trigger_type, executed_by)
             VALUES ($1, $2, 'pending', $3, 'scheduled', 'scheduler')
             RETURNING id",
            schedule.analysis_id,
            analysis.version,
            params
        ).fetch_one(&self.db).await?.id;
        
        // Execute asynchronously
        tokio::spawn(async move {
            match self.executor.execute(analysis, params).await {
                Ok(result) => {
                    // Update run status
                    sqlx::query!(
                        "UPDATE analysis_runs 
                         SET status = 'completed', completed_at = NOW()
                         WHERE id = $1",
                        run_id
                    ).execute(&self.db).await?;
                    
                    // Reset consecutive failures
                    sqlx::query!(
                        "UPDATE analysis_schedules 
                         SET consecutive_failures = 0, last_status = 'completed'
                         WHERE id = $1",
                        schedule.id
                    ).execute(&self.db).await?;
                },
                Err(e) => {
                    // Handle failure with retry logic
                    self.handle_scheduled_failure(schedule, run_id, e).await;
                }
            }
        });
    }
    
    fn resolve_parameter_defaults(&self, params: Value) -> Value {
        // Convert special values like "yesterday", "last_week" to actual dates
        let today = Utc::now().date();
        
        params.map(|param| {
            match param.default {
                "yesterday" => (today - Duration::days(1)).to_string(),
                "today" => today.to_string(),
                "last_week" => (today - Duration::days(7)).to_string(),
                "last_month" => (today - Duration::days(30)).to_string(),
                "current_month" => format!("{}-01", today.format("%Y-%m")),
                "week_start" => /* Calculate week start */,
                "week_end" => /* Calculate week end */,
                other => other
            }
        })
    }
}
```

## Backend Implementation

### DuckDB Persistent Storage

DuckDB provides persistent analytical storage that can be queried anytime:

```rust
use duckdb::Connection;
use std::path::PathBuf;

pub struct DuckDBStorage {
    conn: Connection,
    db_path: PathBuf,
}

impl DuckDBStorage {
    pub fn new(project_id: &str) -> Result<Self> {
        // Each project gets its own persistent DuckDB database
        let db_path = PathBuf::from("/data/duckdb")
            .join(format!("{}.db", project_id));
        
        // Ensure directory exists
        std::fs::create_dir_all(db_path.parent().unwrap())?;
        
        let conn = Connection::open(&db_path)?;
        
        // Configure DuckDB
        conn.execute("SET memory_limit = '8GB'", [])?;
        conn.execute("SET threads = 8", [])?;
        
        // Enable persistence features
        conn.execute("SET preserve_insertion_order = true", [])?;
        conn.execute("SET checkpoint_threshold = '1GB'", [])?;
        
        Ok(Self { conn, db_path })
    }
    
    pub async fn load_from_datasource(
        &self, 
        datasource: &DataSource,
        query: &str,
        table_name: &str
    ) -> Result<()> {
        match datasource {
            DataSource::Postgres { conn_str, .. } => {
                // Use DuckDB's PostgreSQL scanner
                self.conn.execute(
                    "INSTALL postgres_scanner",
                    []
                )?;
                
                self.conn.execute(&format!(
                    "CREATE TABLE {} AS SELECT * FROM postgres_scan('{}', $${}$$)",
                    table_name, conn_str, query
                ), [])?;
            },
            DataSource::MySQL { .. } => {
                // Similar for MySQL
            },
            DataSource::ClickHouse { .. } => {
                // Load via Parquet export or direct query
            }
        }
        Ok(())
    }
    
    pub async fn export_to_datasource(
        &self,
        table_name: &str,
        datasource: &DataSource,
        dest_table: &str
    ) -> Result<()> {
        match datasource {
            DataSource::Postgres { conn_str, .. } => {
                self.conn.execute(&format!(
                    "COPY {} TO '{}' (FORMAT POSTGRES)",
                    table_name, dest_table
                ), [])?;
            },
            // Handle other datasource types
        }
        Ok(())
    }
    
    pub fn list_tables(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT table_name FROM information_schema.tables 
             WHERE table_schema = 'main'"
        )?;
        
        let tables = stmt.query_map([], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;
        
        Ok(tables)
    }
    
    pub fn get_table_info(&self, table_name: &str) -> Result<TableInfo> {
        // Get row count and size
        let count: i64 = self.conn.query_row(
            &format!("SELECT COUNT(*) FROM {}", table_name),
            [],
            |row| row.get(0)
        )?;
        
        Ok(TableInfo {
            name: table_name.to_string(),
            row_count: count,
            created_at: None,  // Can be tracked separately
        })
    }
}
```

### Analysis Execution Context

```rust
pub struct AnalysisContext {
    datasources: HashMap<String, Arc<DataSource>>,
    duckdb: DuckDBSession,
    metadata: HashMap<String, Value>,
    run_id: Uuid,
    start_time: Instant,
    cancelled: Arc<AtomicBool>,
}

impl AnalysisContext {
    // Core operations exposed to JavaScript
    pub async fn query(&self, datasource: &str, sql: &str, params: Vec<Value>) -> Result<Value> {
        let ds = self.datasources.get(datasource)
            .ok_or_else(|| anyhow!("Datasource '{}' not found", datasource))?;
        
        // Execute query with timeout
        let result = timeout(Duration::from_secs(30), 
            ds.execute_query(sql, params)
        ).await??;
        
        Ok(result)
    }
    
    pub async fn duckdb_query(&self, sql: &str) -> Result<Value> {
        // Execute in DuckDB and return as JSON
        let mut stmt = self.duckdb.conn.prepare(sql)?;
        let results = stmt.query_map([], |row| {
            // Convert row to JSON
        })?;
        
        // Collect results
        let mut data = Vec::new();
        for row in results {
            data.push(row?);
        }
        
        // Check size before returning
        let json = serde_json::to_value(&data)?;
        self.validate_result_size(&json)?;
        
        Ok(json)
    }
    
    fn validate_result_size(&self, result: &Value) -> Result<()> {
        const MAX_SIZE: usize = 10 * 1024 * 1024; // 10MB
        
        let size = serde_json::to_vec(result)?.len();
        if size > MAX_SIZE {
            return Err(anyhow!(
                "Result size {} exceeds 10MB limit. Use DuckDB to save large results to a datasource.",
                size / 1_048_576
            ));
        }
        Ok(())
    }
    
    pub fn should_stop(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
}
```

### File-Based DataSource Implementation

File datasources use DuckDB as the SQL query engine for all file types:

```rust
use duckdb::{Connection, params};
use reqwest::Client;
use std::path::PathBuf;

pub enum FileType {
    CSV,
    Parquet,
    Excel,
    GoogleSheets,
    ExcelOnline,
}

pub enum FileSource {
    Local(PathBuf),
    S3 { bucket: String, key: String },
    HTTP(String),
    GoogleSheets { sheet_id: String, credentials: GoogleCredentials },
    ExcelOnline { file_url: String, auth_token: String },
}

pub struct FileDataSource {
    name: String,
    file_type: FileType,
    source: FileSource,
    duckdb_conn: Connection,
    cache_ttl: Option<Duration>,
    last_refreshed: Option<Instant>,
}

impl FileDataSource {
    pub fn new(name: &str, file_type: FileType, source: FileSource) -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        
        // Configure DuckDB for file operations
        conn.execute("INSTALL httpfs", [])?;  // For S3/HTTP
        conn.execute("LOAD httpfs", [])?;
        conn.execute("INSTALL excel", [])?;   // For Excel files
        conn.execute("LOAD excel", [])?;
        
        Ok(Self {
            name: name.to_string(),
            file_type,
            source,
            duckdb_conn: conn,
            cache_ttl: Some(Duration::from_secs(300)), // 5 min default
            last_refreshed: None,
        })
    }
    
    pub async fn query(&self, sql: &str) -> Result<Value> {
        // Check if refresh needed for any source (local files might change too)
        if self.needs_refresh() {
            self.refresh().await?;
        }
        
        let query = match &self.file_type {
            FileType::CSV => {
                let path = self.get_path().await?;
                format!(
                    "WITH data AS (SELECT * FROM read_csv('{}', AUTO_DETECT=TRUE)) {}",
                    path, sql
                )
            },
            FileType::Parquet => {
                let path = self.get_path().await?;
                format!(
                    "WITH data AS (SELECT * FROM read_parquet('{}')) {}",
                    path, sql
                )
            },
            FileType::Excel => {
                let path = self.get_path().await?;
                format!(
                    "WITH data AS (SELECT * FROM st_read('{}')) {}",
                    path, sql
                )
            },
            FileType::GoogleSheets => {
                // Fetch as CSV and query
                let csv_data = self.fetch_google_sheets_as_csv().await?;
                self.load_csv_to_temp_table(&csv_data)?;
                format!("WITH data AS (SELECT * FROM temp_sheet) {}", sql)
            },
            FileType::ExcelOnline => {
                // Similar to Google Sheets
                let data = self.fetch_excel_online().await?;
                self.load_to_temp_table(&data)?;
                format!("WITH data AS (SELECT * FROM temp_excel) {}", sql)
            }
        };
        
        // Execute query
        let mut stmt = self.duckdb_conn.prepare(&query)?;
        let rows = stmt.query_map([], |row| {
            // Convert row to JSON
            Ok(row_to_json(row))
        })?;
        
        let results: Vec<Value> = rows.collect::<Result<Vec<_>, _>>()?;
        Ok(json!({ "rows": results }))
    }
    
    pub async fn get_schema(&self) -> Result<Schema> {
        let schema_query = match &self.file_type {
            FileType::CSV => {
                let path = self.get_path().await?;
                format!("DESCRIBE SELECT * FROM read_csv('{}', SAMPLE_SIZE=1000)", path)
            },
            FileType::Parquet => {
                let path = self.get_path().await?;
                format!("DESCRIBE SELECT * FROM read_parquet('{}')", path)
            },
            // ... other types
        };
        
        let schema = self.duckdb_conn.query(&schema_query)?;
        Ok(parse_schema(schema))
    }
    
    async fn fetch_google_sheets_as_csv(&self) -> Result<String> {
        if let FileSource::GoogleSheets { sheet_id, .. } = &self.source {
            // Export Google Sheets as CSV
            let export_url = format!(
                "https://docs.google.com/spreadsheets/d/{}/export?format=csv",
                sheet_id
            );
            
            let client = Client::new();
            let csv = client.get(&export_url)
                .bearer_auth(&self.get_google_token().await?)
                .send()
                .await?
                .text()
                .await?;
            
            Ok(csv)
        } else {
            Err("Not a Google Sheets datasource".into())
        }
    }
    
    pub async fn refresh(&mut self) -> Result<()> {
        // Force refresh for all sources
        match &self.source {
            FileSource::Local(path) => {
                // For local files, clear DuckDB's internal cache
                self.duckdb_conn.execute("PRAGMA disable_object_cache", [])?;
                self.last_refreshed = Some(Instant::now());
                eprintln!("Refreshed local file: {:?}", path);
            },
            FileSource::S3 { bucket, key } => {
                // Clear S3 cache and force re-download
                self.duckdb_conn.execute("SET s3_force_download = true", [])?;
                self.last_refreshed = Some(Instant::now());
                eprintln!("Refreshed S3 file: s3://{}/{}", bucket, key);
            },
            FileSource::HTTP(url) => {
                // Clear HTTP cache
                self.duckdb_conn.execute("PRAGMA http_cache_clear", [])?;
                self.last_refreshed = Some(Instant::now());
                eprintln!("Refreshed HTTP file: {}", url);
            },
            FileSource::GoogleSheets { .. } | FileSource::ExcelOnline { .. } => {
                self.last_refreshed = Some(Instant::now());
                // Clear any cached data
                self.duckdb_conn.execute("DROP TABLE IF EXISTS temp_sheet", [])?;
                self.duckdb_conn.execute("DROP TABLE IF EXISTS temp_excel", [])?;
            },
        }
        Ok(())
    }
    
    pub async fn get_last_modified(&self) -> Result<DateTime<Utc>> {
        match &self.source {
            FileSource::Local(path) => {
                let metadata = std::fs::metadata(path)?;
                let modified = metadata.modified()?;
                Ok(DateTime::<Utc>::from(modified))
            },
            FileSource::S3 { bucket, key } => {
                // Use S3 HEAD request to get last modified
                let query = format!(
                    "SELECT last_modified FROM s3_object_info('s3://{}/{}')",
                    bucket, key
                );
                let result = self.duckdb_conn.query_row(&query, [], |row| {
                    Ok(row.get(0)?)
                })?;
                Ok(result)
            },
            FileSource::HTTP(url) => {
                // Use HTTP HEAD request
                let client = Client::new();
                let response = client.head(url).send().await?;
                if let Some(last_modified) = response.headers().get("last-modified") {
                    let date = httpdate::parse_http_date(last_modified.to_str()?)?;
                    Ok(DateTime::<Utc>::from(date))
                } else {
                    Ok(Utc::now())
                }
            },
            _ => Ok(self.last_refreshed.map(|_| Utc::now()).unwrap_or_else(Utc::now))
        }
    }
    
    fn needs_refresh(&self) -> bool {
        if let Some(ttl) = self.cache_ttl {
            if let Some(last) = self.last_refreshed {
                return last.elapsed() > ttl;
            }
        }
        false
    }
}

// Make FileDataSource compatible with DataSourceConnector trait
#[async_trait]
impl DataSourceConnector for FileDataSource {
    async fn test_connection(&mut self) -> Result<bool, Box<dyn Error>> {
        // Test if file is accessible
        match &self.source {
            FileSource::Local(path) => Ok(path.exists()),
            FileSource::S3 { .. } => {
                // Test S3 access
                self.query("SELECT COUNT(*) FROM data LIMIT 1").await.is_ok()
            },
            FileSource::GoogleSheets { .. } => {
                // Test Google Sheets API
                self.fetch_google_sheets_as_csv().await.is_ok()
            },
            _ => Ok(true)
        }
    }
    
    async fn execute_query(&self, query: &str, limit: i32) -> Result<Value, Box<dyn Error>> {
        let limited_query = format!("{} LIMIT {}", query, limit);
        self.query(&limited_query).await
    }
    
    async fn fetch_schema(&self) -> Result<Value, Box<dyn Error>> {
        let schema = self.get_schema().await?;
        Ok(serde_json::to_value(schema)?)
    }
    
    async fn list_tables(&self) -> Result<Vec<String>, Box<dyn Error>> {
        // File datasources have a single "table" named "data"
        Ok(vec!["data".to_string()])
    }
    
    // Other trait methods...
}
```

### QuickJS Integration

```rust
use rquickjs::{Context, Runtime, Module, Function};

pub struct SandboxRuntime {
    runtime: Runtime,
}

impl SandboxRuntime {
    pub fn new() -> Result<Self> {
        let runtime = Runtime::new()?;
        
        // Configure memory limit
        runtime.set_memory_limit(256 * 1024 * 1024); // 256MB
        
        // Set up module loader
        runtime.set_loader(
            BuiltinResolver::default(),
            BuiltinLoader::default()
        );
        
        Ok(Self { runtime })
    }
    
    pub async fn execute_analysis(
        &self,
        script: &str,
        context: AnalysisContext,
        params: Value
    ) -> Result<Value> {
        let ctx = Context::full(&self.runtime)?;
        
        ctx.with(|ctx| {
            // Bind context methods to JavaScript
            let global = ctx.globals();
            
            // Create ctx object
            let js_ctx = Object::new(ctx)?;
            
            // Bind query function
            js_ctx.set("query", Function::new(ctx, move |datasource: String, sql: String| {
                // Call context.query()
            })?)?;
            
            // Bind DuckDB operations
            let duckdb = Object::new(ctx)?;
            duckdb.set("exec", Function::new(ctx, move |sql: String| {
                // Call context.duckdb_exec()
            })?)?;
            
            js_ctx.set("duckdb", duckdb)?;
            
            // Load and execute module
            let module = Module::evaluate(ctx, "analysis.js", script)?;
            let default_export = module.get_export("default")?;
            
            // Call run function
            let run_fn = default_export.get("run")?;
            let result = run_fn.call((js_ctx, params))?;
            
            // Validate result is object
            if !result.is_object() {
                return Err(anyhow!("Analysis must return an object"));
            }
            
            Ok(result)
        })
    }
}
```

## Storage & Versioning

### Database Schema

```sql
-- Main analyses table
CREATE TABLE analyses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) UNIQUE NOT NULL,      -- Unique identifier (e.g., "monthly_sales_report")
    title VARCHAR(255),                      -- Display name
    script TEXT NOT NULL,                    -- Current JavaScript code
    metadata JSONB NOT NULL,                 -- Dependencies, parameters schema, etc.
    version INTEGER DEFAULT 1,                
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    created_by VARCHAR(255) DEFAULT 'mcp',   -- Who created it (mcp or user_id)
    is_active BOOLEAN DEFAULT true           -- Soft delete
);

-- Version history table
CREATE TABLE analysis_versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    analysis_id UUID REFERENCES analyses(id) ON DELETE CASCADE,
    version INTEGER NOT NULL,
    script TEXT NOT NULL,                    -- Full script at this version
    metadata JSONB NOT NULL,                  -- Metadata at this version
    created_at TIMESTAMP DEFAULT NOW(),
    change_description TEXT,                  -- What changed in this version
    created_by VARCHAR(255) DEFAULT 'mcp',
    UNIQUE(analysis_id, version)
);

-- Analysis execution history
CREATE TABLE analysis_runs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    analysis_id UUID REFERENCES analyses(id) ON DELETE CASCADE,
    analysis_version INTEGER NOT NULL,
    
    -- Execution details
    status VARCHAR(50) NOT NULL,  -- 'pending', 'running', 'completed', 'failed', 'cancelled'
    parameters JSONB NOT NULL,
    trigger_type VARCHAR(50) DEFAULT 'manual',  -- 'manual', 'scheduled', 'api'
    
    -- Result storage
    result_path VARCHAR(500),      -- Path to compressed JSON file
    result_size_bytes BIGINT,
    result_preview JSONB,          -- First few rows/keys for quick view
    metadata JSONB,                -- Metadata set by ctx.metadata.set()
    
    -- Timing
    started_at TIMESTAMP DEFAULT NOW(),
    completed_at TIMESTAMP,
    execution_time_ms INTEGER,
    
    -- User tracking
    executed_by VARCHAR(255),      -- User ID or 'scheduler' for cron jobs
    
    CONSTRAINT idx_unique_running UNIQUE (analysis_id, status) WHERE status = 'running'
);

-- Scheduling configuration
CREATE TABLE analysis_schedules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    analysis_id UUID REFERENCES analyses(id) ON DELETE CASCADE,
    
    -- Schedule configuration
    cron_expression VARCHAR(100) NOT NULL,  -- e.g., "0 2 * * *"
    timezone VARCHAR(50) DEFAULT 'UTC',
    enabled BOOLEAN DEFAULT true,
    
    -- Retry configuration
    retry_on_failure BOOLEAN DEFAULT false,
    max_retries INTEGER DEFAULT 3,
    retry_delay_seconds INTEGER DEFAULT 300,
    
    -- Execution tracking
    last_run_at TIMESTAMP,
    last_run_id UUID REFERENCES analysis_runs(id),
    last_status VARCHAR(50),
    next_run_at TIMESTAMP,
    consecutive_failures INTEGER DEFAULT 0,
    
    -- Metadata
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    
    UNIQUE(analysis_id)  -- One schedule per analysis
);

-- Indexes for performance
CREATE INDEX idx_analyses_name ON analyses(name) WHERE is_active = true;
CREATE INDEX idx_analyses_datasources ON analyses USING GIN ((metadata->'dependencies'->'datasources'));
CREATE INDEX idx_analyses_analyses ON analyses USING GIN ((metadata->'dependencies'->'analyses'));
CREATE INDEX idx_versions_analysis ON analysis_versions(analysis_id, version DESC);
CREATE INDEX idx_runs_analysis ON analysis_runs(analysis_id, started_at DESC);
CREATE INDEX idx_runs_status ON analysis_runs(status) WHERE status IN ('pending', 'running');
```

### Usage Examples

```sql
-- Get current version of an analysis
SELECT * FROM analyses 
WHERE name = 'monthly_sales_report' AND is_active = true;

-- Get version history
SELECT version, created_at, change_description 
FROM analysis_versions 
WHERE analysis_id = $1 
ORDER BY version DESC;

-- Update analysis (with automatic versioning)
WITH current AS (
    SELECT id, version, script, metadata FROM analyses WHERE name = $1
)
-- Insert old version to history
INSERT INTO analysis_versions (analysis_id, version, script, metadata, change_description)
SELECT id, version, script, metadata, $4 FROM current;

-- Update main table
UPDATE analyses 
SET script = $2, 
    metadata = $3,
    version = version + 1,
    updated_at = NOW()
WHERE name = $1;

-- Rollback to specific version
WITH target_version AS (
    SELECT script, metadata 
    FROM analysis_versions 
    WHERE analysis_id = $1 AND version = $2
)
UPDATE analyses 
SET script = tv.script,
    metadata = tv.metadata,
    version = version + 1,
    updated_at = NOW()
FROM target_version tv
WHERE analyses.id = $1;

-- Find all analyses using a specific datasource
SELECT name, title 
FROM analyses 
WHERE is_active = true 
  AND metadata->'dependencies'->'datasources' ? 'postgres_main';

-- Find dependency graph (which analyses depend on others)
SELECT a1.name as analysis, a2.name as depends_on
FROM analyses a1, 
     jsonb_array_elements_text(a1.metadata->'dependencies'->'analyses') as dep_name
JOIN analyses a2 ON a2.name = dep_name
WHERE a1.is_active = true AND a2.is_active = true;
```

### Metadata Structure

```json
{
  "dependencies": {
    "datasources": ["postgres_main", "clickhouse"],
    "analyses": ["customer_metrics"]
  },
  "parameters": {
    "month": {
      "type": "date",
      "required": true
    },
    "category": {
      "type": "select",
      "required": false
    }
  },
  "execution": {
    "timeout_ms": 300000,
    "memory_limit_mb": 512
  }
}
```

## TypeScript Definitions

Complete type definitions for LLM code generation:

```typescript
// Analysis Definition
interface Analysis {
    title: string;
    schedule?: {
        cron: string;
        timezone?: string;
        enabled?: boolean;
        shouldRun?: (ctx: Context) => Promise<boolean>;
    };
    dependencies?: {
        datasources?: string[];
        analyses?: string[];
    };
    parameters?: Record<string, Parameter>;
    run: (ctx: Context, params: any) => Promise<object>;
}

// Parameter Types
interface Parameter {
    type: 'string' | 'number' | 'boolean' | 'date' | 'select' | 'multiselect';
    required?: boolean;
    default?: any;
    description?: string;
    options?: SelectOption[] | ((ctx: Context, params: any) => Promise<SelectOption[]>);
    min?: number;
    max?: number;
    pattern?: string;
}

interface SelectOption {
    value: string | number;
    label: string;
}

// Context API
interface Context {
    // Datasource access (dynamic based on configuration)
    datasource: {
        [name: string]: DataSource;
    };
    
    // DuckDB operations
    duckdb: {
        exec: (sql: string) => Promise<void>;
        query: (sql: string) => Promise<any[]>;
        load: (datasource: string, sql: string, tableName: string) => Promise<void>;
        export: (tableName: string, datasource: string, destination: string) => Promise<void>;
        saveDataFrame: (df: DataFrame, tableName: string, options?: SaveOptions) => Promise<void>;
        tables: () => Promise<string[]>;
        describe: (tableName: string) => Promise<TableInfo>;
    };
    
    // Metadata storage
    metadata: {
        get: (key: string) => any;
        set: (key: string, value: any) => void;
        has: (key: string) => boolean;
        delete: (key: string) => void;
        list: () => string[];
    };
    
    // Polars DataFrame
    DataFrame: (data: any[]) => DataFrame;
    
    // Analysis operations
    runAnalysis: (analysisId: string, params?: any) => Promise<any>;
    
    // Utilities
    log: (...args: any[]) => void;
    error: (...args: any[]) => void;
    sleep: (ms: number) => Promise<void>;
    shouldStop: () => boolean;
    
    // Data utilities
    utils: {
        compress: (data: any, format: 'gzip' | 'brotli' | 'zstd') => Buffer;
        decompress: (data: Buffer, format: 'gzip' | 'brotli' | 'zstd') => any;
        hash: (data: any, algorithm: 'sha256' | 'md5' | 'sha512') => string;
        parseExcel: (buffer: Buffer, sheet?: string) => any[][];
        parseCSV: (text: string, options?: CSVOptions) => any[];
    };
}

// DataSource Types
interface SQLDataSource {
    query: (sql: string, params?: any[]) => Promise<any[]>;
    stream: (sql: string, params?: any[], batchSize?: number) => AsyncIterator<any[]>;
    execute?: (sql: string, params?: any[]) => Promise<void>;
    insert?: (table: string, data: any[]) => Promise<void>;
}

interface FileDataSource extends SQLDataSource {
    query: (sql: string) => Promise<any[]>;  // SQL query via DuckDB
    getSchema: () => Promise<Schema>;         // Get column info
    getRowCount: () => Promise<number>;       // Total row count
    getSampleData: (limit?: number) => Promise<any[]>; // Sample rows
    refresh: () => Promise<void>;             // Re-read file (local) or re-download (remote)
    setCacheTTL?: (seconds: number) => void;  // Set cache duration for remote files
    getLastModified?: () => Promise<Date>;    // When file was last modified
    getFileSize?: () => Promise<number>;      // File size in bytes
    path?: string;  // File path for local files
    isStale?: () => boolean;  // Check if cache is stale
}

interface CloudSpreadsheetDataSource extends FileDataSource {
    refresh: () => Promise<void>;             // Force refresh from cloud
    getRange: (range: string) => Promise<any[][]>; // Get specific range
    getSheet: (name: string) => Promise<any[]>;    // Get specific sheet
    setCacheTTL: (seconds: number) => void;   // Set cache duration
    lastRefreshed: () => Date;                // When last synced
}

interface S3DataSource {
    list: (prefix: string, options?: S3ListOptions) => Promise<S3Object[]>;
    get: (key: string) => Promise<Buffer>;
    put: (key: string, data: Buffer | string, metadata?: any) => Promise<void>;
    delete: (key: string) => Promise<void>;
    getSignedUrl: (key: string, expiry?: number) => Promise<string>;
    stream: (key: string, options: S3StreamOptions) => Promise<void>;
    multipartUpload: (key: string, parts: Buffer[]) => Promise<void>;
}

interface RESTDataSource {
    get: (endpoint: string, params?: any) => Promise<any>;
    post: (endpoint: string, body: any, params?: any) => Promise<any>;
    put: (endpoint: string, body: any, params?: any) => Promise<any>;
    delete: (endpoint: string, params?: any) => Promise<any>;
    paginate: (endpoint: string, options: PaginateOptions) => Promise<any[]>;
}

interface OpenAPIDataSource {
    operations: Record<string, (...args: any[]) => Promise<any>>;
    call: (path: string, options: OpenAPICallOptions) => Promise<any>;
}

interface SOAPDataSource {
    call: (operation: string, params: any) => Promise<any>;
}

interface GraphQLDataSource {
    query: (options: GraphQLQueryOptions) => Promise<any>;
    mutate: (options: GraphQLMutationOptions) => Promise<any>;
}

// Helper Types
interface S3Object {
    key: string;
    size: number;
    lastModified: Date;
    etag: string;
}

interface S3StreamOptions {
    decompress?: 'gzip' | 'zip' | '7z' | 'tar' | 'bz2';
    format?: 'csv' | 'excel' | 'json' | 'parquet';
    onChunk: (data: any) => Promise<void>;
}

interface S3ListOptions {
    maxKeys?: number;
    delimiter?: string;
    continuationToken?: string;
}

interface PaginateOptions {
    pageParam?: string;
    limitParam?: string;
    limit?: number;
    dataPath?: string;
    nextPagePath?: string;
}

interface SaveOptions {
    mode?: 'overwrite' | 'append';
    partition?: string[];
}

interface TableInfo {
    name: string;
    columns: ColumnInfo[];
    rowCount: number;
    sizeBytes: number;
}

interface ColumnInfo {
    name: string;
    type: string;
    nullable: boolean;
}

interface CSVOptions {
    delimiter?: string;
    headers?: boolean;
    skipRows?: number;
    encoding?: string;
}

interface OpenAPICallOptions {
    method: 'GET' | 'POST' | 'PUT' | 'DELETE';
    pathParams?: Record<string, any>;
    queryParams?: Record<string, any>;
    body?: any;
    headers?: Record<string, string>;
}

interface GraphQLQueryOptions {
    query: string;
    variables?: Record<string, any>;
}

interface GraphQLMutationOptions {
    mutation: string;
    variables?: Record<string, any>;
}

interface DataFrame {
    filter: (predicate: (row: any) => boolean) => DataFrame;
    select: (...columns: string[]) => DataFrame;
    groupBy: (...columns: string[]) => GroupedDataFrame;
    join: (other: DataFrame, on: string | string[]) => DataFrame;
    sort: (column: string, ascending?: boolean) => DataFrame;
    withColumn: (name: string, expr: any) => DataFrame;
    drop: (...columns: string[]) => DataFrame;
    limit: (n: number) => DataFrame;
    collect: () => any[];
}

interface GroupedDataFrame {
    agg: (aggregations: Record<string, string | AggregateExpr>) => DataFrame;
    count: () => DataFrame;
}

// Column expressions for Polars
declare function col(name: string): ColumnExpr;
declare function lit(value: any): LiteralExpr;

interface ColumnExpr {
    sum(): AggregateExpr;
    mean(): AggregateExpr;
    count(): AggregateExpr;
    min(): AggregateExpr;
    max(): AggregateExpr;
    nUnique(): AggregateExpr;
}

interface LiteralExpr {
    // Literal expressions for constants
}

interface AggregateExpr {
    // Aggregate expressions for group operations
}

type DataSource = SQLDataSource | FileDataSource | CloudSpreadsheetDataSource |
                   S3DataSource | RESTDataSource | OpenAPIDataSource | 
                   SOAPDataSource | GraphQLDataSource;

// Schema type for file datasources
interface Schema {
    columns: Array<{
        name: string;
        type: string;  // 'INTEGER', 'VARCHAR', 'DOUBLE', 'DATE', etc.
        nullable: boolean;
    }>;
    rowCount?: number;
    sizeBytes?: number;
}
```