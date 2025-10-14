use clay_studio_backend::core::analysis::bun_runtime::BunRuntime;
use serde_json::json;
use tempfile::TempDir;
use uuid::Uuid;

#[tokio::test]
async fn test_bun_runtime_basic_execution() {
    let temp_dir = TempDir::new().unwrap();
    let clients_dir = temp_dir.path().to_path_buf();

    let runtime = BunRuntime::new(clients_dir).unwrap();
    let project_id = Uuid::new_v4();
    let job_id = Uuid::new_v4();

    // Create a simple analysis script
    let script = r#"
export default {
    async run(ctx, parameters) {
        ctx.log('Hello from Bun!');
        return {
            message: 'Success',
            parameters,
            projectId: ctx.projectId,
        };
    }
}
"#;

    let parameters = json!({
        "testParam": "testValue"
    });

    let context = json!({
        "datasources": {},
        "metadata": {}
    });

    let result = runtime
        .execute_analysis(project_id, job_id, script, parameters.clone(), context, None, None)
        .await
        .unwrap();

    assert_eq!(result["message"], "Success");
    assert_eq!(result["parameters"]["testParam"], "testValue");
    assert_eq!(result["projectId"], project_id.to_string());
}

#[tokio::test]
async fn test_duckdb_query() {
    let temp_dir = TempDir::new().unwrap();
    let clients_dir = temp_dir.path().to_path_buf();

    let runtime = BunRuntime::new(clients_dir).unwrap();
    let project_id = Uuid::new_v4();
    let job_id = Uuid::new_v4();

    // Create analysis script that uses DuckDB
    let script = r#"
export default {
    async run(ctx, parameters) {
        // Create table and insert data
        await ctx.query('CREATE TABLE users (id INTEGER, name VARCHAR)');
        await ctx.query('INSERT INTO users VALUES (1, ?)' , ['Alice']);
        await ctx.query('INSERT INTO users VALUES (2, ?)' , ['Bob']);

        // Query the data
        const result = await ctx.query('SELECT * FROM users ORDER BY id');

        return {
            rowCount: result.rows.length,
            users: result.rows
        };
    }
}
"#;

    let result = runtime
        .execute_analysis(project_id, job_id, script, json!({}), json!({}), None, None)
        .await
        .unwrap();

    assert_eq!(result["rowCount"], 2);
    assert_eq!(result["users"][0]["name"], "Alice");
    assert_eq!(result["users"][1]["name"], "Bob");
}

#[tokio::test]
async fn test_load_data() {
    let temp_dir = TempDir::new().unwrap();
    let clients_dir = temp_dir.path().to_path_buf();

    let runtime = BunRuntime::new(clients_dir).unwrap();
    let project_id = Uuid::new_v4();
    let job_id = Uuid::new_v4();

    // Test loadData functionality
    let script = r#"
export default {
    async run(ctx, parameters) {
        const data = [
            { id: 1, name: 'Product A', price: 10.50 },
            { id: 2, name: 'Product B', price: 25.00 },
            { id: 3, name: 'Product C', price: 15.75 }
        ];

        await ctx.loadData('products', data);

        const result = await ctx.query('SELECT * FROM products WHERE price > 15');

        return {
            filteredCount: result.rows.length,
            products: result.rows
        };
    }
}
"#;

    let result = runtime
        .execute_analysis(project_id, job_id, script, json!({}), json!({}), None, None)
        .await
        .unwrap();

    assert_eq!(result["filteredCount"], 2);
}

#[tokio::test]
async fn test_script_validation() {
    let temp_dir = TempDir::new().unwrap();
    let clients_dir = temp_dir.path().to_path_buf();

    let runtime = BunRuntime::new(clients_dir).unwrap();
    let project_id = Uuid::new_v4();

    // Valid script
    let valid_script = r#"
export default {
    async run(ctx, parameters) {
        return { success: true };
    }
}
"#;

    let errors = runtime.validate_script(project_id, valid_script).await.unwrap();
    assert!(errors.is_empty(), "Valid script should have no errors");

    // Invalid script - missing export
    let invalid_script = r#"
function run(ctx, parameters) {
    return { success: true };
}
"#;

    let errors = runtime.validate_script(project_id, invalid_script).await.unwrap();
    assert!(!errors.is_empty(), "Invalid script should have errors");
    assert!(errors.iter().any(|e| e.contains("export default")));
}

#[tokio::test]
async fn test_async_await_support() {
    let temp_dir = TempDir::new().unwrap();
    let clients_dir = temp_dir.path().to_path_buf();

    let runtime = BunRuntime::new(clients_dir).unwrap();
    let project_id = Uuid::new_v4();
    let job_id = Uuid::new_v4();

    // Test that async/await works correctly
    let script = r#"
export default {
    async run(ctx, parameters) {
        // Async operations
        const delay = (ms) => new Promise(resolve => setTimeout(resolve, ms));

        await delay(10);

        const result1 = await ctx.query('SELECT 1 as value');
        await delay(10);
        const result2 = await ctx.query('SELECT 2 as value');

        return {
            value1: result1.rows[0].value,
            value2: result2.rows[0].value,
            asyncSupported: true
        };
    }
}
"#;

    let result = runtime
        .execute_analysis(project_id, job_id, script, json!({}), json!({}), None, None)
        .await
        .unwrap();

    assert_eq!(result["value1"], 1);
    assert_eq!(result["value2"], 2);
    assert_eq!(result["asyncSupported"], true);
}

#[tokio::test]
async fn test_error_handling() {
    let temp_dir = TempDir::new().unwrap();
    let clients_dir = temp_dir.path().to_path_buf();

    let runtime = BunRuntime::new(clients_dir).unwrap();
    let project_id = Uuid::new_v4();
    let job_id = Uuid::new_v4();

    // Script that throws an error
    let script = r#"
export default {
    async run(ctx, parameters) {
        throw new Error('Test error message');
    }
}
"#;

    let result = runtime
        .execute_analysis(project_id, job_id, script, json!({}), json!({}), None, None)
        .await;

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Test error message"));
}

#[tokio::test]
async fn test_result_size_limit() {
    let temp_dir = TempDir::new().unwrap();
    let clients_dir = temp_dir.path().to_path_buf();

    let runtime = BunRuntime::new(clients_dir).unwrap();
    let project_id = Uuid::new_v4();
    let job_id = Uuid::new_v4();

    // Script that tries to return data larger than 10MB
    let script = r#"
export default {
    async run(ctx, parameters) {
        // Create a large array that will exceed 10MB when serialized
        const largeArray = new Array(2 * 1024 * 1024).fill('x'.repeat(10));
        return { data: largeArray };
    }
}
"#;

    let result = runtime
        .execute_analysis(project_id, job_id, script, json!({}), json!({}), None, None)
        .await;

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("10MB limit"));
}
