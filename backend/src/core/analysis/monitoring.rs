use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use sysinfo::{System, Disks, Networks};
use tokio::sync::RwLock;
use uuid::Uuid;

/// Performance metrics for the analysis system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisMetrics {
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub average_execution_time_ms: f64,
    pub longest_execution_time_ms: u64,
    pub shortest_execution_time_ms: u64,
    pub active_jobs: u64,
    pub queued_jobs: u64,
    pub memory_usage_bytes: u64,
    pub last_updated: DateTime<Utc>,
}

/// Real-time system monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealth {
    pub cpu_usage_percent: f32,
    pub memory_usage_percent: f32,
    pub disk_usage_percent: f32,
    pub network_io: NetworkIO,
    pub uptime_seconds: u64,
    pub last_check: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkIO {
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

/// Job performance tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobMetrics {
    pub job_id: Uuid,
    pub analysis_id: Uuid,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub memory_peak_bytes: u64,
    pub cpu_time_ms: u64,
    pub status: String,
    pub error_count: u32,
}

/// Performance monitoring service
pub struct MonitoringService {
    metrics: Arc<RwLock<AnalysisMetrics>>,
    job_metrics: Arc<RwLock<HashMap<Uuid, JobMetrics>>>,
    system_health: Arc<RwLock<SystemHealth>>,
    system: Arc<RwLock<System>>,
    disks: Arc<RwLock<Disks>>,
    networks: Arc<RwLock<Networks>>,
    start_time: Instant,
}

impl Default for AnalysisMetrics {
    fn default() -> Self {
        Self {
            total_executions: 0,
            successful_executions: 0,
            failed_executions: 0,
            average_execution_time_ms: 0.0,
            longest_execution_time_ms: 0,
            shortest_execution_time_ms: u64::MAX,
            active_jobs: 0,
            queued_jobs: 0,
            memory_usage_bytes: 0,
            last_updated: Utc::now(),
        }
    }
}

impl Default for SystemHealth {
    fn default() -> Self {
        Self {
            cpu_usage_percent: 0.0,
            memory_usage_percent: 0.0,
            disk_usage_percent: 0.0,
            network_io: NetworkIO {
                bytes_sent: 0,
                bytes_received: 0,
            },
            uptime_seconds: 0,
            last_check: Utc::now(),
        }
    }
}

impl MonitoringService {
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        
        let disks = Disks::new_with_refreshed_list();
        let networks = Networks::new_with_refreshed_list();
        
        Self {
            metrics: Arc::new(RwLock::new(AnalysisMetrics::default())),
            job_metrics: Arc::new(RwLock::new(HashMap::new())),
            system_health: Arc::new(RwLock::new(SystemHealth::default())),
            system: Arc::new(RwLock::new(system)),
            disks: Arc::new(RwLock::new(disks)),
            networks: Arc::new(RwLock::new(networks)),
            start_time: Instant::now(),
        }
    }

    /// Start a new job monitoring session
    pub async fn start_job_monitoring(&self, job_id: Uuid, analysis_id: Uuid) {
        let job_metric = JobMetrics {
            job_id,
            analysis_id,
            start_time: Utc::now(),
            end_time: None,
            duration_ms: None,
            memory_peak_bytes: 0,
            cpu_time_ms: 0,
            status: "running".to_string(),
            error_count: 0,
        };

        let mut job_metrics = self.job_metrics.write().await;
        job_metrics.insert(job_id, job_metric);

        // Update active jobs count
        let mut metrics = self.metrics.write().await;
        metrics.active_jobs += 1;
        metrics.last_updated = Utc::now();
    }

    /// Complete job monitoring
    pub async fn complete_job_monitoring(&self, job_id: Uuid, success: bool) {
        let mut job_metrics = self.job_metrics.write().await;
        let mut metrics = self.metrics.write().await;

        if let Some(job_metric) = job_metrics.get_mut(&job_id) {
            let end_time = Utc::now();
            let duration_ms = (end_time - job_metric.start_time).num_milliseconds() as u64;

            job_metric.end_time = Some(end_time);
            job_metric.duration_ms = Some(duration_ms);
            job_metric.status = if success { "completed" } else { "failed" }.to_string();

            // Update global metrics
            metrics.total_executions += 1;
            if success {
                metrics.successful_executions += 1;
            } else {
                metrics.failed_executions += 1;
            }

            // Update timing statistics
            if duration_ms > metrics.longest_execution_time_ms {
                metrics.longest_execution_time_ms = duration_ms;
            }
            if duration_ms < metrics.shortest_execution_time_ms {
                metrics.shortest_execution_time_ms = duration_ms;
            }

            // Update average execution time
            let total_time = metrics.average_execution_time_ms * (metrics.total_executions - 1) as f64 + duration_ms as f64;
            metrics.average_execution_time_ms = total_time / metrics.total_executions as f64;
        }

        metrics.active_jobs = metrics.active_jobs.saturating_sub(1);
        metrics.last_updated = Utc::now();
    }

    /// Record memory usage for a job
    pub async fn record_memory_usage(&self, job_id: Uuid, memory_bytes: u64) {
        let mut job_metrics = self.job_metrics.write().await;
        if let Some(job_metric) = job_metrics.get_mut(&job_id) {
            if memory_bytes > job_metric.memory_peak_bytes {
                job_metric.memory_peak_bytes = memory_bytes;
            }
        }
    }

    /// Get current process memory usage
    pub async fn get_current_process_memory(&self) -> u64 {
        let system = self.system.read().await;
        let pid = sysinfo::Pid::from(std::process::id() as usize);
        
        if let Some(process) = system.process(pid) {
            process.memory()
        } else {
            0
        }
    }

    /// Get analysis system statistics
    pub async fn get_system_stats(&self) -> Result<SystemStats> {
        let metrics = self.get_metrics().await;
        let health = self.get_system_health().await;
        let active_jobs = self.get_active_job_metrics().await;
        
        Ok(SystemStats {
            running_jobs_count: active_jobs.len() as u64,
            total_results: metrics.total_executions,
            storage_size_bytes: self.estimate_storage_size().await,
            cpu_usage_percent: health.cpu_usage_percent,
            memory_usage_percent: health.memory_usage_percent,
            uptime_seconds: health.uptime_seconds,
            last_updated: health.last_check,
        })
    }

    /// Estimate storage size used by analysis results
    async fn estimate_storage_size(&self) -> u64 {
        // This would calculate actual storage used by DuckDB databases and results
        // For now, provide an estimate based on job count
        let metrics = self.get_metrics().await;
        metrics.total_executions * 1024 * 1024 // Estimate 1MB per result
    }

    /// Get current metrics
    pub async fn get_metrics(&self) -> AnalysisMetrics {
        self.metrics.read().await.clone()
    }

    /// Get job metrics
    pub async fn get_job_metrics(&self, job_id: Uuid) -> Option<JobMetrics> {
        let job_metrics = self.job_metrics.read().await;
        job_metrics.get(&job_id).cloned()
    }

    /// Get all active job metrics
    pub async fn get_active_job_metrics(&self) -> Vec<JobMetrics> {
        let job_metrics = self.job_metrics.read().await;
        job_metrics
            .values()
            .filter(|m| m.status == "running")
            .cloned()
            .collect()
    }

    /// Update system health metrics
    pub async fn update_system_health(&self) -> Result<()> {
        // Refresh system information
        {
            let mut system = self.system.write().await;
            system.refresh_cpu();
            system.refresh_memory();
        }
        
        // Refresh disks and networks
        {
            let mut disks = self.disks.write().await;
            disks.refresh();
        }
        
        {
            let mut networks = self.networks.write().await;
            networks.refresh();
        }
        
        let mut health = self.system_health.write().await;
        
        // Get real system metrics
        health.uptime_seconds = self.start_time.elapsed().as_secs();
        health.last_check = Utc::now();
        health.cpu_usage_percent = self.get_cpu_usage().await;
        health.memory_usage_percent = self.get_memory_usage().await;
        health.disk_usage_percent = self.get_disk_usage().await;
        health.network_io = self.get_network_io().await;

        Ok(())
    }

    /// Get system health
    pub async fn get_system_health(&self) -> SystemHealth {
        self.system_health.read().await.clone()
    }

    /// Start monitoring background task
    pub async fn start_monitoring(&self) -> Result<()> {
        let monitoring_service = self.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            
            loop {
                interval.tick().await;
                
                if let Err(e) = monitoring_service.update_system_health().await {
                    tracing::error!("Failed to update system health: {}", e);
                }
                
                // Cleanup old job metrics (keep last 1000 jobs)
                monitoring_service.cleanup_old_metrics().await;
            }
        });

        tracing::info!("Monitoring service started");
        Ok(())
    }

    /// Clean up old job metrics to prevent memory leaks
    async fn cleanup_old_metrics(&self) {
        let mut job_metrics = self.job_metrics.write().await;
        
        if job_metrics.len() > 1000 {
            // Keep only the most recent 1000 job metrics
            let mut metrics_vec: Vec<_> = job_metrics.drain().collect();
            metrics_vec.sort_by(|a, b| b.1.start_time.cmp(&a.1.start_time));
            metrics_vec.truncate(1000);
            
            *job_metrics = metrics_vec.into_iter().collect();
        }
    }

    /// Get performance insights
    pub async fn get_performance_insights(&self) -> PerformanceInsights {
        let metrics = self.get_metrics().await;
        let active_jobs = self.get_active_job_metrics().await;
        let system_health = self.get_system_health().await;

        let success_rate = if metrics.total_executions > 0 {
            (metrics.successful_executions as f64 / metrics.total_executions as f64) * 100.0
        } else {
            0.0
        };

        PerformanceInsights {
            success_rate_percent: success_rate,
            average_execution_time_ms: metrics.average_execution_time_ms,
            current_load: active_jobs.len() as u32,
            memory_pressure: system_health.memory_usage_percent > 80.0,
            cpu_pressure: system_health.cpu_usage_percent > 80.0,
            recommendations: self.generate_recommendations(&metrics, &system_health).await,
        }
    }

    async fn generate_recommendations(&self, metrics: &AnalysisMetrics, health: &SystemHealth) -> Vec<String> {
        let mut recommendations = Vec::new();

        if health.memory_usage_percent > 80.0 {
            recommendations.push("High memory usage detected. Consider increasing memory limits or optimizing analysis scripts.".to_string());
        }

        if health.cpu_usage_percent > 80.0 {
            recommendations.push("High CPU usage detected. Consider reducing concurrent job limits.".to_string());
        }

        if metrics.average_execution_time_ms > 300000.0 { // 5 minutes
            recommendations.push("Average execution time is high. Review analysis complexity and optimize queries.".to_string());
        }

        if metrics.total_executions > 0 {
            let failure_rate = (metrics.failed_executions as f64 / metrics.total_executions as f64) * 100.0;
            if failure_rate > 10.0 {
                recommendations.push(format!("High failure rate ({:.1}%). Review analysis scripts and dependencies.", failure_rate));
            }
        }

        if recommendations.is_empty() {
            recommendations.push("System is performing well. No immediate recommendations.".to_string());
        }

        recommendations
    }

    // Real system metrics using sysinfo
    async fn get_cpu_usage(&self) -> f32 {
        let system = self.system.read().await;
        let cpus = system.cpus();
        
        if cpus.is_empty() {
            return 0.0;
        }
        
        // Calculate average CPU usage across all cores
        let total_usage: f32 = cpus.iter().map(|cpu| cpu.cpu_usage()).sum();
        total_usage / cpus.len() as f32
    }

    async fn get_memory_usage(&self) -> f32 {
        let system = self.system.read().await;
        let total_memory = system.total_memory();
        let used_memory = system.used_memory();
        
        if total_memory == 0 {
            return 0.0;
        }
        
        (used_memory as f32 / total_memory as f32) * 100.0
    }

    async fn get_disk_usage(&self) -> f32 {
        let disks = self.disks.read().await;
        
        if disks.is_empty() {
            return 0.0;
        }
        
        // Calculate average disk usage across all disks
        let mut total_usage = 0.0;
        let mut disk_count = 0;
        
        for disk in disks.iter() {
            let total_space = disk.total_space();
            let available_space = disk.available_space();
            
            if total_space > 0 {
                let used_space = total_space - available_space;
                let usage_percent = (used_space as f32 / total_space as f32) * 100.0;
                total_usage += usage_percent;
                disk_count += 1;
            }
        }
        
        if disk_count > 0 {
            total_usage / disk_count as f32
        } else {
            0.0
        }
    }

    async fn get_network_io(&self) -> NetworkIO {
        let networks = self.networks.read().await;
        
        let mut total_received = 0;
        let mut total_transmitted = 0;
        
        for (_interface_name, network) in networks.iter() {
            total_received += network.received();
            total_transmitted += network.transmitted();
        }
        
        NetworkIO {
            bytes_received: total_received,
            bytes_sent: total_transmitted,
        }
    }
}

impl Clone for MonitoringService {
    fn clone(&self) -> Self {
        Self {
            metrics: Arc::clone(&self.metrics),
            job_metrics: Arc::clone(&self.job_metrics),
            system_health: Arc::clone(&self.system_health),
            system: Arc::clone(&self.system),
            disks: Arc::clone(&self.disks),
            networks: Arc::clone(&self.networks),
            start_time: self.start_time,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PerformanceInsights {
    pub success_rate_percent: f64,
    pub average_execution_time_ms: f64,
    pub current_load: u32,
    pub memory_pressure: bool,
    pub cpu_pressure: bool,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemStats {
    pub running_jobs_count: u64,
    pub total_results: u64,
    pub storage_size_bytes: u64,
    pub cpu_usage_percent: f32,
    pub memory_usage_percent: f32,
    pub uptime_seconds: u64,
    pub last_updated: DateTime<Utc>,
}

/// Performance optimization helpers
pub struct PerformanceOptimizer;

impl PerformanceOptimizer {
    /// Suggest optimal concurrency limits based on system resources
    pub fn suggest_concurrency_limit(system_health: &SystemHealth) -> u32 {
        let base_limit = if system_health.memory_usage_percent < 50.0 {
            10
        } else if system_health.memory_usage_percent < 70.0 {
            6
        } else {
            3
        };

        let cpu_factor = if system_health.cpu_usage_percent < 50.0 {
            1.5
        } else if system_health.cpu_usage_percent < 70.0 {
            1.0
        } else {
            0.5
        };

        std::cmp::max(1, (base_limit as f32 * cpu_factor) as u32)
    }

    /// Check if system is under pressure
    pub fn is_system_under_pressure(health: &SystemHealth) -> bool {
        health.cpu_usage_percent > 80.0 || health.memory_usage_percent > 85.0
    }

    /// Suggest script optimization opportunities
    pub fn analyze_script_performance(execution_time_ms: u64) -> Vec<String> {
        let mut suggestions = Vec::new();

        if execution_time_ms > 600000 { // 10 minutes
            suggestions.push("Consider breaking down long-running analysis into smaller chunks".to_string());
            suggestions.push("Use DuckDB for large data processing instead of in-memory operations".to_string());
        }

        if execution_time_ms > 120000 { // 2 minutes
            suggestions.push("Review query efficiency and add appropriate database indexes".to_string());
            suggestions.push("Consider using streaming operations for large datasets".to_string());
        }

        if execution_time_ms > 30000 { // 30 seconds
            suggestions.push("Optimize data filtering to reduce processing volume".to_string());
        }

        suggestions
    }
}