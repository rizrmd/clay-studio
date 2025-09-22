/*!
# Integration Test for Analysis System

This module provides a complete integration test that verifies the analysis system
works end-to-end without requiring database connections.
*/

use anyhow::Result;
use tempfile::TempDir;
use uuid::Uuid;
use crate::models::analysis::*;

/// Comprehensive integration test for the analysis system
pub async fn run_integration_test() -> Result<()> {
    println!("🔬 Starting Analysis System Integration Test...");

    // Test 1: DuckDB Manager
    test_duckdb_manager().await?;
    println!("✅ DuckDB Manager test passed");

    // Test 2: Sandbox Validation
    test_sandbox_validation().await?;
    println!("✅ Sandbox validation test passed");

    // Test 3: Analysis Script Parsing
    test_analysis_parsing().await?;
    println!("✅ Analysis parsing test passed");

    // Test 4: Monitoring System
    test_monitoring_system().await?;
    println!("✅ Monitoring system test passed");

    // Test 5: Result Storage (without database)
    test_result_storage_logic().await?;
    println!("✅ Result storage logic test passed");

    println!("🎉 All integration tests passed!");
    Ok(())
}

async fn test_duckdb_manager() -> Result<()> {
    use super::DuckDBManager;

    let temp_dir = TempDir::new()?;
    let duckdb = DuckDBManager::new(temp_dir.path().to_path_buf()).await?;
    
    // Test database path generation
    let project_id = "test_project_123";
    let db_path = duckdb.get_database_path(project_id).await;
    
    assert!(db_path.to_string_lossy().contains("test_project_123"));
    assert!(db_path.to_string_lossy().ends_with(".duckdb"));
    
    // Test executable path
    let exe_path = duckdb.get_executable_path_ref();
    assert!(exe_path.to_string_lossy().contains("duckdb"));
    
    Ok(())
}

async fn test_sandbox_validation() -> Result<()> {
    use super::AnalysisSandbox;

    let temp_dir = TempDir::new()?;
    let sandbox = AnalysisSandbox::new(temp_dir.path().to_path_buf()).await?;

    // Test valid script validation
    let valid_script = r#"
    export default {
        title: "Test Analysis",
        dependencies: {
            datasources: ["test_db"],
            analyses: []
        },
        run: async function(ctx, params) {
            return { result: "success" };
        }
    }
    "#;

    let result = sandbox.validate_analysis(valid_script).await?;
    assert!(result.valid, "Valid script should pass validation");
    assert!(result.errors.is_empty(), "Should have no errors");

    // Test invalid script validation
    let invalid_script = "function test() { return 'invalid'; }";
    let result = sandbox.validate_analysis(invalid_script).await?;
    assert!(!result.valid, "Invalid script should fail validation");
    assert!(!result.errors.is_empty(), "Should have validation errors");

    Ok(())
}

async fn test_analysis_parsing() -> Result<()> {
    use super::AnalysisSandbox;

    let temp_dir = TempDir::new()?;
    let sandbox = AnalysisSandbox::new(temp_dir.path().to_path_buf()).await?;

    // Create a mock analysis
    let analysis = Analysis {
        id: Uuid::new_v4(),
        title: "Test Analysis".to_string(),
        script_content: r#"
        export default {
            title: "Test Analysis",
            dependencies: { datasources: ["test_db"], analyses: [] },
            run: async function(ctx, params) {
                return { result: "success", params: params };
            }
        }
        "#.to_string(),
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

    // Test analysis execution
    let result = sandbox.execute_analysis(
        &analysis,
        parameters.clone(),
        Uuid::new_v4(),
        std::collections::HashMap::new(),
    ).await?;

    assert!(result.is_object(), "Result should be an object");
    // The result should contain the executed analysis output
    assert!(result.get("result").is_some(), "Should have a result field");

    Ok(())
}

async fn test_monitoring_system() -> Result<()> {
    use super::monitoring::MonitoringService;

    let monitoring = MonitoringService::new();
    let job_id = Uuid::new_v4();
    let analysis_id = Uuid::new_v4();

    // Test job monitoring lifecycle
    monitoring.start_job_monitoring(job_id, analysis_id).await;
    
    let metrics = monitoring.get_metrics().await;
    assert_eq!(metrics.active_jobs, 1, "Should have one active job");

    // Test memory recording
    monitoring.record_memory_usage(job_id, 1024 * 1024).await; // 1MB

    let job_metrics = monitoring.get_job_metrics(job_id).await;
    assert!(job_metrics.is_some(), "Should have job metrics");
    
    let job_metric = job_metrics.expect("Job metrics should be available after recording");
    assert_eq!(job_metric.memory_peak_bytes, 1024 * 1024, "Should record memory usage");

    // Test real system monitoring
    monitoring.update_system_health().await?;
    let system_health = monitoring.get_system_health().await;
    
    assert!(system_health.cpu_usage_percent >= 0.0, "CPU usage should be non-negative");
    assert!(system_health.memory_usage_percent >= 0.0, "Memory usage should be non-negative");
    assert!(system_health.disk_usage_percent >= 0.0, "Disk usage should be non-negative");

    // Test system stats
    let system_stats = monitoring.get_system_stats().await?;
    assert_eq!(system_stats.running_jobs_count, 1, "Should show one running job");

    // Test performance insights
    let insights = monitoring.get_performance_insights().await;
    assert!(insights.current_load > 0, "Should show current load");

    // Test job completion
    monitoring.complete_job_monitoring(job_id, true).await;
    
    let updated_metrics = monitoring.get_metrics().await;
    assert_eq!(updated_metrics.active_jobs, 0, "Should have no active jobs");
    assert_eq!(updated_metrics.total_executions, 1, "Should have one total execution");
    assert_eq!(updated_metrics.successful_executions, 1, "Should have one successful execution");

    Ok(())
}

async fn test_result_storage_logic() -> Result<()> {
    // Test result size validation
    let small_result = serde_json::json!({
        "test": "data",
        "numbers": [1, 2, 3, 4, 5]
    });

    let json_bytes = serde_json::to_vec(&small_result)?;
    assert!(json_bytes.len() < 10 * 1024 * 1024, "Small result should be under 10MB");

    // Test compression logic
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&json_bytes)?;
    let compressed = encoder.finish()?;

    assert!(compressed.len() <= json_bytes.len(), "Compressed data should be smaller or equal");

    // Test checksum calculation
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(&compressed);
    let checksum = hex::encode(hasher.finalize());

    assert_eq!(checksum.len(), 64, "SHA256 checksum should be 64 characters");

    Ok(())
}

/// Test the analysis system types and serialization
pub fn test_analysis_types() -> Result<()> {
    println!("🔧 Testing analysis types and serialization...");

    // Test analysis metadata serialization
    let metadata = AnalysisMetadata {
        id: "test-123".to_string(),
        title: "Test Analysis".to_string(),
        parameters: std::collections::HashMap::new(),
        dependencies: AnalysisDependencies {
            datasources: vec!["db1".to_string(), "db2".to_string()],
            analyses: vec!["analysis1".to_string()],
        },
        schedule: Some(ScheduleConfig {
            cron: "0 9 * * *".to_string(),
            timezone: Some("UTC".to_string()),
            enabled: true,
        }),
        created_at: chrono::Utc::now(),
        last_run: None,
    };

    let serialized = serde_json::to_string(&metadata)?;
    let deserialized: AnalysisMetadata = serde_json::from_str(&serialized)?;
    
    assert_eq!(metadata.id, deserialized.id);
    assert_eq!(metadata.title, deserialized.title);
    assert_eq!(metadata.dependencies.datasources, deserialized.dependencies.datasources);

    // Test job status enum
    let statuses = vec![
        JobStatus::Pending,
        JobStatus::Running, 
        JobStatus::Completed,
        JobStatus::Failed,
        JobStatus::Cancelled,
    ];

    for status in statuses {
        let serialized = serde_json::to_string(&status)?;
        let deserialized: JobStatus = serde_json::from_str(&serialized)?;
        assert_eq!(format!("{:?}", status), format!("{:?}", deserialized));
    }

    // Test parameter types
    let param = AnalysisParameter {
        name: "test_param".to_string(),
        param_type: ParameterType::Select,
        required: true,
        description: Some("Test parameter".to_string()),
        default_value: Some(serde_json::json!("default")),
        options: Some(vec![
            ParameterOption {
                value: "option1".to_string(),
                label: "Option 1".to_string(),
                options: None,
            },
            ParameterOption {
                value: "option2".to_string(),
                label: "Option 2".to_string(),
                options: None,
            },
        ]),
        has_dynamic_options: false,
        depends_on: vec!["other_param".to_string()],
    };

    let serialized = serde_json::to_string(&param)?;
    let deserialized: AnalysisParameter = serde_json::from_str(&serialized)?;
    
    assert_eq!(param.name, deserialized.name);
    assert_eq!(param.required, deserialized.required);

    println!("✅ Analysis types test passed");
    Ok(())
}

/// Performance benchmark for key operations
pub async fn benchmark_analysis_operations() -> Result<()> {
    println!("⚡ Benchmarking analysis operations...");

    let start = std::time::Instant::now();

    // Benchmark validation
    let temp_dir = TempDir::new()?;
    let sandbox = super::AnalysisSandbox::new(temp_dir.path().to_path_buf()).await?;
    
    let script = r#"
    export default {
        title: "Benchmark Analysis",
        dependencies: { datasources: ["test"], analyses: [] },
        run: async function(ctx, params) { return { success: true }; }
    }
    "#;

    let validation_start = std::time::Instant::now();
    for _ in 0..100 {
        let _ = sandbox.validate_analysis(script).await?;
    }
    let validation_time = validation_start.elapsed();

    println!("📊 Validation: 100 operations in {:?} ({:?} per operation)", 
             validation_time, validation_time / 100);

    // Benchmark monitoring
    let monitoring = super::monitoring::MonitoringService::new();
    let monitoring_start = std::time::Instant::now();
    
    for i in 0..100 {
        let job_id = Uuid::new_v4();
        let analysis_id = Uuid::new_v4();
        monitoring.start_job_monitoring(job_id, analysis_id).await;
        monitoring.record_memory_usage(job_id, 1024 * 1024).await;
        monitoring.complete_job_monitoring(job_id, i % 10 != 0).await; // 90% success rate
    }
    
    let monitoring_time = monitoring_start.elapsed();
    println!("📊 Monitoring: 100 job lifecycles in {:?} ({:?} per job)", 
             monitoring_time, monitoring_time / 100);

    let final_metrics = monitoring.get_metrics().await;
    println!("📈 Final metrics: {} total, {} successful, {:.1}% success rate",
             final_metrics.total_executions,
             final_metrics.successful_executions,
             (final_metrics.successful_executions as f64 / final_metrics.total_executions as f64) * 100.0);

    let total_time = start.elapsed();
    println!("🏁 Total benchmark time: {:?}", total_time);

    Ok(())
}