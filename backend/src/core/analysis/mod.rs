// pub mod analysis_manager; // Complex analysis functionality temporarily disabled
// pub mod datasource_service; // Temporarily disabled due to compilation issues
// pub mod db_helper;
// pub mod duckdb_manager;
// pub mod examples; // Temporarily disabled due to doctest issues
// pub mod integration_test;
// pub mod job_manager;
// pub mod monitoring;
// pub mod result_storage;
// pub mod sandbox;
// pub mod scheduler;
pub mod service; // Keep only basic service for route configuration
// #[cfg(test)]
// pub mod tests;

// Re-exports
pub use service::AnalysisService;