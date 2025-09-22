use serde_json::Value;

/// Common query building utilities
#[allow(dead_code)]
pub struct QueryBuilder {
    query: String,
    parameters: Vec<Value>,
}

#[allow(dead_code)]
impl QueryBuilder {
    pub fn new(base_query: &str) -> Self {
        Self {
            query: base_query.to_string(),
            parameters: Vec::new(),
        }
    }

    pub fn add_where_clause(&mut self, column: &str, operator: &str, value: Value) -> &mut Self {
        let param_placeholder = format!("${}", self.parameters.len() + 1);
        
        if self.query.to_lowercase().contains("where") {
            self.query.push_str(&format!(" AND {} {} {}", column, operator, param_placeholder));
        } else {
            self.query.push_str(&format!(" WHERE {} {} {}", column, operator, param_placeholder));
        }
        
        self.parameters.push(value);
        self
    }

    pub fn add_order_by(&mut self, column: &str, direction: Option<&str>) -> &mut Self {
        let direction = direction.unwrap_or("ASC");
        if self.query.to_lowercase().contains("order by") {
            self.query.push_str(&format!(", {} {}", column, direction));
        } else {
            self.query.push_str(&format!(" ORDER BY {} {}", column, direction));
        }
        self
    }

    pub fn add_limit(&mut self, limit: i32) -> &mut Self {
        self.query.push_str(&format!(" LIMIT {}", limit));
        self
    }

    pub fn add_offset(&mut self, offset: i32) -> &mut Self {
        self.query.push_str(&format!(" OFFSET {}", offset));
        self
    }

    pub fn build(self) -> (String, Vec<Value>) {
        (self.query, self.parameters)
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn parameters(&self) -> &[Value] {
        &self.parameters
    }
}

/// Build a pagination query with common patterns
#[allow(dead_code)]
pub struct PaginationQueryBuilder {
    base_query: String,
    count_query: String,
}

#[allow(dead_code)]
impl PaginationQueryBuilder {
    pub fn new(table_name: &str, columns: Option<&str>) -> Self {
        let columns = columns.unwrap_or("*");
        let base_query = format!("SELECT {} FROM {}", columns, table_name);
        let count_query = format!("SELECT COUNT(*) FROM {}", table_name);
        
        Self {
            base_query,
            count_query,
        }
    }

    pub fn with_filters(&mut self, filters: Option<&Value>) -> &mut Self {
        if let Some(filters) = filters {
            if let Some(filter_obj) = filters.as_object() {
                for (key, value) in filter_obj {
                    let where_clause = format!(" WHERE {} = '{}'", key, value.as_str().unwrap_or(""));
                    self.base_query.push_str(&where_clause);
                    self.count_query.push_str(&where_clause);
                }
            }
        }
        self
    }

    pub fn with_sorting(&mut self, sort_column: Option<&str>, sort_direction: Option<&str>) -> &mut Self {
        if let Some(column) = sort_column {
            let direction = sort_direction.unwrap_or("ASC");
            self.base_query.push_str(&format!(" ORDER BY {} {}", column, direction));
        }
        self
    }

    pub fn with_pagination(&mut self, page: i32, limit: i32) -> &mut Self {
        let offset = (page - 1) * limit;
        self.base_query.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));
        self
    }

    pub fn build(self) -> (String, String) {
        (self.base_query, self.count_query)
    }
}

/// Build common schema queries
#[allow(dead_code)]
pub struct SchemaQueryBuilder;

#[allow(dead_code)]
impl SchemaQueryBuilder {
    /// Build a query to get table information for PostgreSQL
    pub fn postgres_tables_query(schema: &str) -> String {
        format!(
            "SELECT table_name FROM information_schema.tables WHERE table_schema = '{}' AND table_type = 'BASE TABLE' ORDER BY table_name",
            schema
        )
    }

    /// Build a query to get column information for PostgreSQL
    pub fn postgres_columns_query(schema: &str, table: &str) -> String {
        format!(
            r#"
            SELECT 
                column_name,
                data_type,
                is_nullable,
                column_default,
                character_maximum_length,
                numeric_precision,
                numeric_scale
            FROM information_schema.columns 
            WHERE table_schema = '{}' AND table_name = '{}'
            ORDER BY ordinal_position
            "#,
            schema, table
        )
    }

    /// Build a query to get table information for MySQL
    pub fn mysql_tables_query(database: &str) -> String {
        format!(
            "SELECT table_name FROM information_schema.tables WHERE table_schema = '{}' AND table_type = 'BASE TABLE' ORDER BY table_name",
            database
        )
    }

    /// Build a query to get column information for MySQL
    pub fn mysql_columns_query(database: &str, table: &str) -> String {
        format!(
            r#"
            SELECT 
                column_name,
                data_type,
                is_nullable,
                column_default,
                character_maximum_length,
                numeric_precision,
                numeric_scale
            FROM information_schema.columns 
            WHERE table_schema = '{}' AND table_name = '{}'
            ORDER BY ordinal_position
            "#,
            database, table
        )
    }
}