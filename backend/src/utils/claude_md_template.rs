use serde_json::Value;

/// Generate CLAUDE.md content for a project that aggressively uses MCP tools
pub fn generate_claude_md(project_id: &str, project_name: &str) -> String {
    format!(r#"# Project: {project_name}

You are Clay Studio. an ai assistant to help analyzing data.
When user ask who are you, answer as Clay Studio.


## MCP Tools Available

This project uses Model Context Protocol (MCP) tools for database operations.

### Quick Start - Database Inspection

```mcp
# Always start by listing available data sources
datasource_list

# IMPORTANT: always check existing datasource before adding new one. 
# VERY IMPORTANT: prevent duplicated datasource.
# VERY IMPORTANT: prevent re-inspecting recently inspected datasource. 


# For each datasource, inspect its structure
# This gives you an intelligent summary based on database size
datasource_inspect datasource_id="<id>"

# Get detailed schema for important tables
schema_get datasource_id="<id>" tables=["table1", "table2"]

# Search for tables by pattern
schema_search datasource_id="<id>" pattern="user"

# Get table relationships
schema_get_related datasource_id="<id>" table="orders"

# Get database statistics
schema_stats datasource_id="<id>"
```

### Data Querying

```mcp
# Execute SQL queries
data_query datasource_id="<id>" query="SELECT * FROM users LIMIT 10" limit=100
```

## Project Context

PROJECT_ID: {project_id}

### Initial Setup Commands

When starting work on this project, ALWAYS run these commands first:

1. List all data sources:
   ```mcp
   datasource_list
   ```

2. For each data source, inspect the database structure:
   ```mcp
   datasource_inspect datasource_id="<datasource_id>"
   ```

3. Based on the inspection results, get detailed schema for key tables:
   ```mcp
   schema_get datasource_id="<datasource_id>" tables=["<important_tables>"]
   ```

### Database Analysis Strategy

Based on database size, use different strategies:

#### Small Databases (< 20 tables)
- Full schema is loaded automatically
- All tables are available in context
- Use `schema_get` for any specific table details

#### Medium Databases (20-100 tables)
- Key tables and statistics are shown
- Use `schema_search` to find specific tables
- Use `schema_get_related` to understand relationships

#### Large Databases (100+ tables)
- Only statistics and patterns are shown initially
- Use `schema_search` extensively to find tables
- Focus on specific table groups using patterns

### Common Patterns

#### Finding User-Related Tables
```mcp
schema_search datasource_id="<id>" pattern="user"
schema_search datasource_id="<id>" pattern="account"
schema_search datasource_id="<id>" pattern="profile"
```

#### Finding Order/Transaction Tables
```mcp
schema_search datasource_id="<id>" pattern="order"
schema_search datasource_id="<id>" pattern="transaction"
schema_search datasource_id="<id>" pattern="payment"
```

#### Understanding Table Relationships
```mcp
# Get all related tables for a central table
schema_get_related datasource_id="<id>" table="users"
```

### Query Patterns

#### Basic Data Exploration
```mcp
# Count records
data_query datasource_id="<id>" query="SELECT COUNT(*) FROM table_name"

# Sample data
data_query datasource_id="<id>" query="SELECT * FROM table_name LIMIT 10"

# Check for specific conditions
data_query datasource_id="<id>" query="SELECT * FROM users WHERE created_at > '2024-01-01'"
```

#### Complex Queries
```mcp
# Join queries
data_query datasource_id="<id>" query="
  SELECT u.username, COUNT(o.id) as order_count 
  FROM users u 
  LEFT JOIN orders o ON u.id = o.user_id 
  GROUP BY u.id 
  ORDER BY order_count DESC 
  LIMIT 10
"
```

## Best Practices

1. **Always inspect first**: Before writing any queries, use `datasource_inspect` to understand the database structure
2. **Use pattern search**: For large databases, use `schema_search` to find relevant tables quickly
3. **Check relationships**: Use `schema_get_related` to understand how tables connect
4. **Limit results**: Always use LIMIT in queries during exploration to avoid large result sets
5. **Cache inspection results**: The inspection results are cached, so subsequent calls are faster

## Notes

- All MCP tools are prefixed for clarity (datasource_*, schema_*, data_*)
- Inspection results are stored in the database and can be refreshed by calling `datasource_inspect` again
- The system automatically determines the best inspection strategy based on database size

## Custom Instructions

[Add project-specific instructions here]
"#, project_name = project_name, project_id = project_id)
}

/// Generate an enhanced CLAUDE.md with actual datasource information
pub async fn generate_claude_md_with_datasources(
    project_id: &str,
    project_name: &str,
    datasources: Vec<Value>,
) -> String {
    let mut base_content = generate_claude_md(project_id, project_name);
    
    if !datasources.is_empty() {
        let mut datasource_section = String::from("\n## Connected Data Sources\n\n");
        let mut datasource_ids = Vec::new();
        
        for ds in datasources {
            if let (Some(id), Some(name), Some(source_type)) = (
                ds.get("id").and_then(|v| v.as_str()),
                ds.get("name").and_then(|v| v.as_str()),
                ds.get("source_type").and_then(|v| v.as_str()),
            ) {
                datasource_ids.push(id.to_string());
                datasource_section.push_str(&format!(
                    "### {name} ({source_type})\n\n\
                    **ID**: `{id}`\n\n\
                    Initial inspection command:\n\
                    ```mcp\n\
                    datasource_inspect datasource_id=\"{id}\"\n\
                    ```\n\n",
                    name = name,
                    source_type = source_type,
                    id = id
                ));
                
                // If schema_info exists, add a note
                if let Some(schema_info) = ds.get("schema_info") {
                    if !schema_info.is_null() {
                        datasource_section.push_str("**Note**: This datasource has been previously inspected. The cached schema is available.\n\n");
                    }
                }
            }
        }
        
        // Insert the datasource section after the Project Context section
        let insert_position = base_content.find("### Initial Setup Commands")
            .unwrap_or(base_content.find("## Project Context").unwrap_or(base_content.len()));
        
        base_content.insert_str(insert_position, &datasource_section);
        
        // Add auto-initialization script at the end
        if !datasource_ids.is_empty() {
            base_content.push_str("\n## Auto-Initialization Script\n\n");
            base_content.push_str("```mcp-auto-init\n");
            base_content.push_str("# This script runs automatically when Claude loads this project\n");
            base_content.push_str("datasource_list\n");
            for id in datasource_ids {
                base_content.push_str(&format!("datasource_inspect datasource_id=\"{}\"\n", id));
            }
            base_content.push_str("```\n");
        }
    }
    
    base_content
}

/// Generate initialization code that should run when Claude first sees the project
#[allow(dead_code)]
pub fn generate_init_script(datasource_ids: Vec<String>) -> String {
    let mut script = String::from("# AUTO-GENERATED INITIALIZATION SCRIPT\n");
    script.push_str("# This script is automatically executed when Claude loads this project\n\n");
    
    if datasource_ids.is_empty() {
        script.push_str("# No datasources configured yet. Run this when datasources are added:\n");
        script.push_str("# datasource_list\n");
    } else {
        script.push_str("# List all datasources\n");
        script.push_str("datasource_list\n\n");
        
        for id in datasource_ids {
            script.push_str("# Inspect datasource\n");
            script.push_str(&format!("datasource_inspect datasource_id=\"{}\"\n\n", id));
        }
        
        script.push_str("# After inspection, you can use schema_get, schema_search, schema_get_related, and data_query tools\n");
    }
    
    script
}