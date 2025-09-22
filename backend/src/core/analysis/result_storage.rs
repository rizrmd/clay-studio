use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::path::{Path, PathBuf};
use tokio::fs;
use uuid::Uuid;

const MAX_RESULT_SIZE: usize = 10 * 1024 * 1024; // 10MB

pub struct ResultStorage {
    storage_dir: PathBuf,
    db: PgPool,
}

impl ResultStorage {
    pub async fn new(storage_dir: PathBuf, db: PgPool) -> Result<Self> {
        // Create storage directory structure
        fs::create_dir_all(&storage_dir).await?;
        fs::create_dir_all(storage_dir.join("results")).await?;
        
        Ok(Self { storage_dir, db })
    }

    pub async fn store_result(&self, job_id: Uuid, result: &Value) -> Result<()> {
        // Serialize and validate size
        let json_bytes = serde_json::to_vec(result)?;
        if json_bytes.len() > MAX_RESULT_SIZE {
            return Err(anyhow!(
                "Result size {}MB exceeds 10MB limit. Use DuckDB for large datasets.",
                json_bytes.len() / 1_048_576
            ));
        }

        // Compress the result
        let compressed = self.compress_data(&json_bytes)?;
        
        // Generate storage path
        let storage_path = self.generate_storage_path(job_id);
        
        // Ensure parent directory exists
        if let Some(parent) = storage_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        // Write compressed data to file
        fs::write(&storage_path, &compressed).await?;

        // Calculate checksum
        let checksum = self.calculate_checksum(&compressed);

        // Store metadata in database
        self.store_result_metadata(job_id, &storage_path, compressed.len() as i64, &checksum).await?;

        Ok(())
    }

    pub async fn retrieve_result(&self, job_id: Uuid) -> Result<Value> {
        // Get storage metadata
        let metadata = self.get_result_metadata(job_id).await?;
        
        // Read compressed data
        let compressed_data = fs::read(&metadata.storage_path).await?;
        
        // Verify checksum if available
        if let Some(expected_checksum) = &metadata.checksum {
            let actual_checksum = self.calculate_checksum(&compressed_data);
            if &actual_checksum != expected_checksum {
                return Err(anyhow!("Result file corrupted: checksum mismatch"));
            }
        }

        // Decompress and parse
        let json_bytes = self.decompress_data(&compressed_data)?;
        let result: Value = serde_json::from_slice(&json_bytes)?;

        Ok(result)
    }

    pub async fn delete_result(&self, job_id: Uuid) -> Result<()> {
        // Get storage metadata
        if let Ok(metadata) = self.get_result_metadata(job_id).await {
            // Delete file if it exists
            if Path::new(&metadata.storage_path).exists() {
                fs::remove_file(&metadata.storage_path).await?;
            }
        }

        // Delete metadata from database
        sqlx::query!(
            "DELETE FROM analysis_result_storage WHERE job_id = $1",
            job_id
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }

    pub async fn cleanup_old_results(&self, older_than_days: i64) -> Result<usize> {
        let cutoff_date = Utc::now() - chrono::Duration::days(older_than_days);
        
        // Get old result files
        let old_results = sqlx::query!(
            r#"
            SELECT rs.job_id, rs.storage_path
            FROM analysis_result_storage rs
            JOIN analysis_jobs j ON rs.job_id = j.id
            WHERE j.created_at < $1
            "#,
            cutoff_date
        )
        .fetch_all(&self.db)
        .await?;

        let mut deleted_count = 0;

        for result in old_results {
            // Delete file
            if Path::new(&result.storage_path).exists() {
                if let Ok(()) = fs::remove_file(&result.storage_path).await {
                    deleted_count += 1;
                }
            }

            // Delete metadata
            let _ = sqlx::query!(
                "DELETE FROM analysis_result_storage WHERE job_id = $1",
                result.job_id
            )
            .execute(&self.db)
            .await;
        }

        Ok(deleted_count)
    }

    pub async fn get_storage_stats(&self) -> Result<StorageStats> {
        let stats = sqlx::query!(
            r#"
            SELECT 
                COUNT(*) as total_results,
                SUM(size_bytes) as total_size_bytes,
                AVG(size_bytes) as avg_size_bytes,
                MIN(created_at) as oldest_result,
                MAX(created_at) as newest_result
            FROM analysis_result_storage
            "#
        )
        .fetch_one(&self.db)
        .await?;

        Ok(StorageStats {
            total_results: stats.total_results.unwrap_or(0) as u64,
            total_size_bytes: stats.total_size_bytes.unwrap_or(0) as u64,
            avg_size_bytes: stats.avg_size_bytes.unwrap_or(0.0) as u64,
            oldest_result: stats.oldest_result,
            newest_result: stats.newest_result,
        })
    }

    fn generate_storage_path(&self, job_id: Uuid) -> PathBuf {
        let job_id_str = job_id.to_string();
        let year = Utc::now().format("%Y").to_string();
        let month = Utc::now().format("%m").to_string();
        let day = Utc::now().format("%d").to_string();

        self.storage_dir
            .join("results")
            .join(year)
            .join(month)
            .join(day)
            .join(format!("{}.json.gz", job_id_str))
    }

    fn compress_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data)?;
        let compressed = encoder.finish()?;
        
        Ok(compressed)
    }

    fn decompress_data(&self, compressed_data: &[u8]) -> Result<Vec<u8>> {
        use flate2::read::GzDecoder;
        use std::io::Read;

        let mut decoder = GzDecoder::new(compressed_data);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        
        Ok(decompressed)
    }

    fn calculate_checksum(&self, data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hex::encode(hasher.finalize())
    }

    async fn store_result_metadata(
        &self,
        job_id: Uuid,
        storage_path: &Path,
        size_bytes: i64,
        checksum: &str,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO analysis_result_storage (id, job_id, storage_path, size_bytes, checksum)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            Uuid::new_v4(),
            job_id,
            storage_path.to_string_lossy(),
            size_bytes,
            checksum
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }

    async fn get_result_metadata(&self, job_id: Uuid) -> Result<ResultMetadata> {
        let row = sqlx::query!(
            r#"
            SELECT storage_path, size_bytes, checksum, created_at
            FROM analysis_result_storage
            WHERE job_id = $1
            "#,
            job_id
        )
        .fetch_one(&self.db)
        .await?;

        Ok(ResultMetadata {
            storage_path: row.storage_path,
            size_bytes: row.size_bytes,
            checksum: row.checksum,
            created_at: row.created_at,
        })
    }
}

#[derive(Debug)]
pub struct ResultMetadata {
    pub storage_path: String,
    pub size_bytes: i64,
    pub checksum: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct StorageStats {
    pub total_results: u64,
    pub total_size_bytes: u64,
    pub avg_size_bytes: u64,
    pub oldest_result: Option<DateTime<Utc>>,
    pub newest_result: Option<DateTime<Utc>>,
}