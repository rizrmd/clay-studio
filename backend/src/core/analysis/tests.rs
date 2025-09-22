#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;
    use tempfile::TempDir;
    use uuid::Uuid;

    async fn setup_test_db() -> PgPool {
        // In real tests, this would connect to a test database
        // For now, return a mock that won't actually connect
        // TODO: Setup actual test database for integration tests
        let database_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://test:test@localhost/test_clay_studio".to_string());
        
        match PgPool::connect(&database_url).await {
            Ok(pool) => pool,
            Err(_) => {
                // Skip database tests if no test DB available
                panic!("Test database not available. Set TEST_DATABASE_URL environment variable.");
            }
        }
    }

    fn create_test_analysis_script() -> String {
        r#"
        export default {
            title: "Test Analysis",
            dependencies: {
                datasources: ["test_db"],
                analyses: []
            },
            parameters: {
                test_param: {
                    type: "text",
                    required: true,
                    default: "test_value"
                }
            },
            run: async function(ctx, params) {
                return {
                    result: "success",
                    param_value: params.test_param,
                    timestamp: new Date().toISOString()
                };
            }
        }
        "#.to_string()
    }

    #[tokio::test]
    async fn test_analysis_validation() {
        let temp_dir = TempDir::new().unwrap();
        let sandbox = AnalysisSandbox::new(temp_dir.path().to_path_buf()).await.unwrap();

        // Test valid script
        let valid_script = create_test_analysis_script();
        let result = sandbox.validate_analysis(&valid_script).await.unwrap();
        assert!(result.valid, "Valid script should pass validation");

        // Test invalid script (missing export default)
        let invalid_script = "function test() { return 'invalid'; }";
        let result = sandbox.validate_analysis(invalid_script).await.unwrap();
        assert!(!result.valid, "Invalid script should fail validation");
        assert!(!result.errors.is_empty(), "Should have validation errors");
    }

    #[tokio::test]
    async fn test_duckdb_manager() {
        let temp_dir = TempDir::new().unwrap();
        let duckdb = DuckDBManager::new(temp_dir.path().to_path_buf()).await.unwrap();
        
        let project_id = "test_project";
        let db_path = duckdb.get_database_path(project_id).await;
        
        // Test basic query
        let result = duckdb.execute_query(&db_path, "SELECT 42 as answer").await;
        assert!(result.is_ok(), "Basic query should work");
        
        // Test table operations
        let create_result = duckdb.execute_query(
            &db_path, 
            "CREATE TABLE test_table (id INTEGER, name VARCHAR)"
        ).await;
        assert!(create_result.is_ok(), "Table creation should work");
        
        let tables = duckdb.list_tables(&db_path).await.unwrap();
        assert!(tables.contains(&"test_table".to_string()), "Created table should be listed");
    }

    #[tokio::test]
    async fn test_sandbox_execution() {
        let temp_dir = TempDir::new().unwrap();
        let sandbox = AnalysisSandbox::new(temp_dir.path().to_path_buf()).await.unwrap();
        
        let script = create_test_analysis_script();
        let analysis = Analysis {
            id: Uuid::new_v4(),
            title: "Test Analysis".to_string(),
            script_content: script,
            project_id: Uuid::new_v4(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            created_by: None,
            version: 1,
            is_active: true,
            metadata: serde_json::json!({}),
        };
        
        let parameters = serde_json::json!({
            "test_param": "hello_world"
        });
        
        let result = sandbox.execute_analysis(
            &analysis,
            parameters,
            Uuid::new_v4(),
            std::collections::HashMap::new(),
        ).await.unwrap();
        
        // Check mock result structure
        assert!(result.is_object(), "Result should be an object");
        assert_eq!(result["status"], "completed", "Should return completed status");
    }

    #[tokio::test] 
    async fn test_result_storage() {
        let temp_dir = TempDir::new().unwrap();
        let db = setup_test_db().await;
        let storage = ResultStorage::new(temp_dir.path().to_path_buf(), db).await.unwrap();
        
        let job_id = Uuid::new_v4();
        let test_result = serde_json::json!({
            "test": "data",
            "number": 42,
            "array": [1, 2, 3]
        });
        
        // Test storing result
        let store_result = storage.store_result(job_id, &test_result).await;
        assert!(store_result.is_ok(), "Should be able to store result");
        
        // Test retrieving result
        let retrieved = storage.retrieve_result(job_id).await.unwrap();
        assert_eq!(retrieved, test_result, "Retrieved result should match stored result");
        
        // Test storage stats
        let stats = storage.get_storage_stats().await.unwrap();
        assert!(stats.total_results >= 1, "Should have at least one result");
    }

    #[tokio::test]
    async fn test_job_lifecycle() {
        let temp_dir = TempDir::new().unwrap();
        let db = setup_test_db().await;
        let sandbox = Arc::new(AnalysisSandbox::new(temp_dir.path().to_path_buf()).await.unwrap());
        let job_manager = JobManager::new(db, sandbox).await;
        
        let analysis_id = Uuid::new_v4();
        let parameters = serde_json::json!({"test": "value"});
        
        // Test job creation and execution
        let job_id = job_manager.execute_analysis(
            analysis_id,
            parameters,
            "test".to_string(),
        ).await.unwrap();
        
        // Test job status retrieval
        let job = job_manager.get_job_status(job_id).await.unwrap();
        assert_eq!(job.id, job_id, "Job ID should match");
        assert_eq!(job.analysis_id, analysis_id, "Analysis ID should match");
    }

    #[tokio::test]
    async fn test_analysis_manager_operations() {
        let temp_dir = TempDir::new().unwrap();
        let db = setup_test_db().await;
        let sandbox = Arc::new(AnalysisSandbox::new(temp_dir.path().to_path_buf()).await.unwrap());
        let manager = AnalysisManager::new(db, sandbox).await;
        
        // Test creating analysis
        let create_request = CreateAnalysisRequest {
            title: "Test Analysis".to_string(),
            script_content: create_test_analysis_script(),
            project_id: Uuid::new_v4(),
        };
        
        let analysis = manager.create_analysis(create_request).await.unwrap();
        assert_eq!(analysis.title, "Test Analysis", "Title should match");
        assert_eq!(analysis.version, 1, "Version should be 1");
        
        // Test updating analysis
        let update_request = UpdateAnalysisRequest {
            title: Some("Updated Test Analysis".to_string()),
            script_content: None,
            change_description: Some("Updated title".to_string()),
        };
        
        let updated = manager.update_analysis(analysis.id, update_request).await.unwrap();
        assert_eq!(updated.title, "Updated Test Analysis", "Title should be updated");
        
        // Test listing analyses
        let analyses = manager.list_analyses(Some(analysis.project_id)).await.unwrap();
        assert!(!analyses.is_empty(), "Should have at least one analysis");
    }

    #[tokio::test]
    async fn test_scheduler_cron_parsing() {
        let temp_dir = TempDir::new().unwrap();
        let db = setup_test_db().await;
        let sandbox = Arc::new(AnalysisSandbox::new(temp_dir.path().to_path_buf()).await.unwrap());
        let job_manager = Arc::new(JobManager::new(db.clone(), sandbox).await);
        let scheduler = AnalysisScheduler::new(db, job_manager).await;
        
        // Test cron expression validation
        let valid_expressions = vec![
            "0 9 * * *",      // Daily at 9 AM
            "0 */4 * * *",    // Every 4 hours
            "0 0 1 * *",      // Monthly
            "*/15 * * * *",   // Every 15 minutes
        ];
        
        for expr in valid_expressions {
            let result = cron::Schedule::from_str(expr);
            assert!(result.is_ok(), "Cron expression '{}' should be valid", expr);
        }
    }

    #[tokio::test]
    async fn test_database_helper() {
        let db = setup_test_db().await;
        let helper = DatabaseHelper::new(db);
        
        // Test table existence check
        let tables_exist = helper.analysis_tables_exist().await;
        // This will be false if migrations haven't been run, which is expected
        println!("Analysis tables exist: {}", tables_exist);
        
        // Test safe operation wrapper
        let result = helper.with_analysis_tables(Box::pin(async move |_pool| {
            Ok("test_result".to_string())
        })).await;
        
        // Should return None if tables don't exist, Some(result) if they do
        match result {
            Ok(Some(value)) => assert_eq!(value, "test_result"),
            Ok(None) => println!("Tables don't exist, operation skipped (expected)"),
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    #[tokio::test]
    async fn test_analysis_service_integration() {
        let temp_dir = TempDir::new().unwrap();
        let db = setup_test_db().await;
        
        let service = AnalysisService::new(db, temp_dir.path().to_path_buf()).await.unwrap();
        
        // Test service startup
        let start_result = service.start().await;
        assert!(start_result.is_ok(), "Service should start successfully");
        
        // Test system stats
        let stats = service.get_system_stats().await.unwrap();
        assert_eq!(stats.running_jobs_count, 0, "Should start with no running jobs");
        
        // Test service shutdown
        let stop_result = service.stop().await;
        assert!(stop_result.is_ok(), "Service should stop successfully");
    }

    #[tokio::test]
    async fn test_error_handling() {
        let temp_dir = TempDir::new().unwrap();
        let sandbox = AnalysisSandbox::new(temp_dir.path().to_path_buf()).await.unwrap();
        
        // Test validation with completely invalid script
        let invalid_scripts = vec![
            "",                                    // Empty script
            "invalid javascript syntax {{}{{",    // Syntax errors
            "export default 'not an object';",    // Wrong export type
        ];
        
        for script in invalid_scripts {
            let result = sandbox.validate_analysis(script).await.unwrap();
            assert!(!result.valid, "Invalid script should fail validation: '{}'", script);
        }
    }
}

/// Integration test utilities
pub mod test_utils {
    use super::*;
    use tempfile::TempDir;
    
    pub async fn create_test_service() -> (AnalysisService, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        
        // Try to create service with test database
        let db_result = if let Ok(db_url) = std::env::var("TEST_DATABASE_URL") {
            sqlx::PgPool::connect(&db_url).await
        } else {
            // Skip integration tests if no database available
            panic!("Set TEST_DATABASE_URL for integration tests");
        };
        
        let db = db_result.expect("Failed to connect to test database");
        let service = AnalysisService::new(db, temp_dir.path().to_path_buf()).await
            .expect("Failed to create analysis service");
        
        (service, temp_dir)
    }
    
    pub fn sample_analysis_request() -> CreateAnalysisRequest {
        CreateAnalysisRequest {
            title: "Sample Test Analysis".to_string(),
            script_content: r#"
                export default {
                    title: "Sample Analysis",
                    dependencies: { datasources: [], analyses: [] },
                    run: async function(ctx, params) {
                        return { success: true, timestamp: new Date().toISOString() };
                    }
                }
            "#.to_string(),
            project_id: Uuid::new_v4(),
        }
    }
}