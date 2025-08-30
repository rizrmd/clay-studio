use crate::models::{DataSourceContext, ToolContext};
use serde_json::json;

pub struct ToolApplicabilityChecker;

impl ToolApplicabilityChecker {
    pub fn determine_applicable_tools(_data_sources: &[DataSourceContext]) -> Vec<ToolContext> {
        let mut tools = vec![];
        
        // MCP Data Source Management Tools - Always Available
        tools.push(ToolContext {
            name: "Add Data Source".to_string(),
            category: "data_management".to_string(),
            description: "Add a new data source to the project".to_string(),
            parameters: json!({
                "name": "string",
                "source_type": "string",
                "connection_config": "object"
            }),
            applicable: true,
            usage_examples: vec![
                "Add PostgreSQL database connection".to_string(),
                "Connect to MySQL server".to_string(),
            ],
        });
        
        tools.push(ToolContext {
            name: "List Data Sources".to_string(),
            category: "data_management".to_string(),
            description: "List all data sources in the project".to_string(),
            parameters: json!({}),
            applicable: true,
            usage_examples: vec![
                "Show all connected databases".to_string(),
                "List available data sources".to_string(),
            ],
        });
        
        tools.push(ToolContext {
            name: "Remove Data Source".to_string(),
            category: "data_management".to_string(),
            description: "Remove a data source from the project".to_string(),
            parameters: json!({
                "datasource_id": "string"
            }),
            applicable: true,
            usage_examples: vec![
                "Remove old database connection".to_string(),
                "Delete unused data source".to_string(),
            ],
        });
        
        tools.push(ToolContext {
            name: "Test Connection".to_string(),
            category: "data_management".to_string(),
            description: "Test connection to a data source".to_string(),
            parameters: json!({
                "datasource_id": "string"
            }),
            applicable: true,
            usage_examples: vec![
                "Test database connectivity".to_string(),
                "Verify connection settings".to_string(),
            ],
        });
        
        tools.push(ToolContext {
            name: "Query Data Source".to_string(),
            category: "data_management".to_string(),
            description: "Execute read-only queries on a data source".to_string(),
            parameters: json!({
                "datasource_id": "string",
                "query": "string",
                "limit": "number"
            }),
            applicable: true,
            usage_examples: vec![
                "SELECT * FROM users LIMIT 10".to_string(),
                "Query customer data".to_string(),
            ],
        });
        
        // SQL Query tool
        tools.push(ToolContext {
            name: "SQL Query".to_string(),
            category: "sql".to_string(),
            description: "Execute SQL queries on connected databases".to_string(),
            parameters: json!({
                "query": "string",
                "database": "string"
            }),
            applicable: true,
            usage_examples: vec![
                "SELECT * FROM users LIMIT 10".to_string(),
                "SELECT COUNT(*) FROM transactions".to_string(),
            ],
        });
        
        // Time Series Analysis
        tools.push(ToolContext {
                name: "Time Series Analysis".to_string(),
                category: "time_series".to_string(),
                description: "Analyze time-based patterns and trends in data".to_string(),
                parameters: json!({
                    "data_source": "string",
                    "time_column": "string",
                    "value_column": "string",
                    "aggregation": "string"
                }),
                applicable: true,
                usage_examples: vec![
                    "Analyze monthly revenue trends".to_string(),
                    "Forecast next quarter sales".to_string(),
                ],
            });
            
            tools.push(ToolContext {
                name: "Seasonality Detection".to_string(),
                category: "time_series".to_string(),
                description: "Detect seasonal patterns in time series data".to_string(),
                parameters: json!({
                    "data_source": "string",
                    "time_column": "string",
                    "value_column": "string",
                    "period": "string"
                }),
                applicable: true,
                usage_examples: vec![
                    "Detect weekly patterns in user activity".to_string(),
                    "Find seasonal trends in sales data".to_string(),
                ],
            });
        
        // Statistical Analysis
        tools.push(ToolContext {
                name: "Statistical Analysis".to_string(),
                category: "statistics".to_string(),
                description: "Perform statistical calculations and analysis".to_string(),
                parameters: json!({
                    "operation": "string",
                    "columns": "array"
                }),
                applicable: true,
                usage_examples: vec![
                    "Calculate correlation between price and sales".to_string(),
                    "Get distribution statistics for revenue".to_string(),
                ],
            });
            
            tools.push(ToolContext {
                name: "Regression Analysis".to_string(),
                category: "statistics".to_string(),
                description: "Perform linear and non-linear regression analysis".to_string(),
                parameters: json!({
                    "dependent_variable": "string",
                    "independent_variables": "array",
                    "method": "string"
                }),
                applicable: true,
                usage_examples: vec![
                    "Predict sales based on marketing spend".to_string(),
                    "Analyze factors affecting customer churn".to_string(),
                ],
            });
        
        // Data Quality Check
        tools.push(ToolContext {
                name: "Data Quality Check".to_string(),
                category: "data_quality".to_string(),
                description: "Check data quality and identify issues".to_string(),
                parameters: json!({
                    "table": "string",
                    "checks": "array"
                }),
                applicable: true,
                usage_examples: vec![
                    "Check for null values in critical columns".to_string(),
                    "Identify duplicate records".to_string(),
                ],
            });
            
            tools.push(ToolContext {
                name: "Data Profiling".to_string(),
                category: "data_quality".to_string(),
                description: "Generate comprehensive data profile reports".to_string(),
                parameters: json!({
                    "source": "string",
                    "include_samples": "boolean"
                }),
                applicable: true,
                usage_examples: vec![
                    "Profile customer data for completeness".to_string(),
                    "Analyze data distribution patterns".to_string(),
                ],
            });
        
        // Data Explorer
        tools.push(ToolContext {
                name: "Data Explorer".to_string(),
                category: "data_exploration".to_string(),
                description: "Explore and visualize data interactively".to_string(),
                parameters: json!({
                    "source": "string",
                    "limit": "number"
                }),
                applicable: true,
                usage_examples: vec![
                    "Show sample data from users table".to_string(),
                    "Preview uploaded CSV file".to_string(),
                ],
            });
        
        // CSV tools
        tools.push(ToolContext {
                name: "CSV Import Wizard".to_string(),
                category: "data_import".to_string(),
                description: "Import and transform CSV data with custom mappings".to_string(),
                parameters: json!({
                    "file_path": "string",
                    "delimiter": "string",
                    "headers": "boolean",
                    "transformations": "array"
                }),
                applicable: true,
                usage_examples: vec![
                    "Import sales data with date formatting".to_string(),
                    "Load customer list with data validation".to_string(),
                ],
            });
        
        // Visualization tools
        tools.push(ToolContext {
                name: "Chart Builder".to_string(),
                category: "visualization".to_string(),
                description: "Create interactive charts and visualizations".to_string(),
                parameters: json!({
                    "chart_type": "string",
                    "data": "object",
                    "options": "object"
                }),
                applicable: true,
                usage_examples: vec![
                    "Create a line chart of revenue over time".to_string(),
                    "Build a pie chart of market share".to_string(),
                ],
            });
            
            tools.push(ToolContext {
                name: "Dashboard Creator".to_string(),
                category: "visualization".to_string(),
                description: "Build custom dashboards with multiple visualizations".to_string(),
                parameters: json!({
                    "widgets": "array",
                    "layout": "object",
                    "refresh_interval": "number"
                }),
                applicable: true,
                usage_examples: vec![
                    "Create KPI dashboard for executives".to_string(),
                    "Build real-time monitoring dashboard".to_string(),
                ],
            });
        
        // Machine Learning tools
        tools.push(ToolContext {
                name: "Clustering Analysis".to_string(),
                category: "machine_learning".to_string(),
                description: "Perform customer segmentation and clustering".to_string(),
                parameters: json!({
                    "features": "array",
                    "method": "string",
                    "num_clusters": "number"
                }),
                applicable: true,
                usage_examples: vec![
                    "Segment customers by behavior".to_string(),
                    "Group products by similarity".to_string(),
                ],
            });
            
            tools.push(ToolContext {
                name: "Anomaly Detection".to_string(),
                category: "machine_learning".to_string(),
                description: "Detect outliers and anomalies in data".to_string(),
                parameters: json!({
                    "data_source": "string",
                    "columns": "array",
                    "sensitivity": "number"
                }),
                applicable: true,
                usage_examples: vec![
                    "Detect fraudulent transactions".to_string(),
                    "Find unusual patterns in usage data".to_string(),
                ],
            });
        
        // Natural Language Processing - always available for text queries
        tools.push(ToolContext {
            name: "Natural Language Query".to_string(),
            category: "nlp".to_string(),
            description: "Convert natural language questions to data queries".to_string(),
            parameters: json!({
                "question": "string",
                "context": "object"
            }),
            applicable: true,
            usage_examples: vec![
                "Show me top customers by revenue last month".to_string(),
                "What's the average order value this year?".to_string(),
            ],
        });
        
        // Export tools
        tools.push(ToolContext {
                name: "Report Generator".to_string(),
                category: "export".to_string(),
                description: "Generate formatted reports in various formats".to_string(),
                parameters: json!({
                    "template": "string",
                    "format": "string",
                    "data_sources": "array"
                }),
                applicable: true,
                usage_examples: vec![
                    "Generate monthly sales report in PDF".to_string(),
                    "Create Excel workbook with multiple sheets".to_string(),
                ],
            });
        
        tools
    }
}