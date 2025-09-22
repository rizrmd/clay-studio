use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::fs;
use tokio::process::Command as AsyncCommand;

const DUCKDB_VERSION: &str = "1.1.3";

pub struct DuckDBManager {
    executable_path: PathBuf,
    data_dir: PathBuf,
}

impl DuckDBManager {
    pub async fn new(data_dir: PathBuf) -> Result<Self> {
        let executable_path = Self::get_executable_path(&data_dir).await?;
        
        Ok(Self {
            executable_path,
            data_dir,
        })
    }

    async fn get_executable_path(data_dir: &Path) -> Result<PathBuf> {
        let bin_dir = data_dir.join("bin");
        fs::create_dir_all(&bin_dir).await?;

        let executable_name = if cfg!(target_os = "windows") {
            "duckdb.exe"
        } else {
            "duckdb"
        };
        
        let executable_path = bin_dir.join(executable_name);

        // Check if DuckDB executable exists and is working
        if executable_path.exists() {
            if let Ok(output) = Command::new(&executable_path)
                .arg("--version")
                .output()
            {
                if output.status.success() {
                    return Ok(executable_path);
                }
            }
        }

        // Download DuckDB if not available or not working
        Self::download_duckdb(&executable_path).await?;
        Ok(executable_path)
    }

    async fn download_duckdb(target_path: &Path) -> Result<()> {
        tracing::info!("Downloading DuckDB v{}", DUCKDB_VERSION);

        let platform = Self::get_platform_string()?;
        let download_url = format!(
            "https://github.com/duckdb/duckdb/releases/download/v{}/duckdb_cli-{}.zip",
            DUCKDB_VERSION, platform
        );

        // Download the zip file
        let response = reqwest::get(&download_url).await?;
        if !response.status().is_success() {
            return Err(anyhow!("Failed to download DuckDB: HTTP {}", response.status()));
        }

        let bytes = response.bytes().await?;
        
        // Create a temporary file for the zip
        let temp_dir = tempfile::tempdir()?;
        let zip_path = temp_dir.path().join("duckdb.zip");
        fs::write(&zip_path, &bytes).await?;

        // Extract the executable
        Self::extract_duckdb(&zip_path, target_path).await?;

        // Make executable on Unix systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(target_path).await?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(target_path, perms).await?;
        }

        tracing::info!("DuckDB downloaded and installed successfully");
        Ok(())
    }

    fn get_platform_string() -> Result<String> {
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;

        let platform = match (os, arch) {
            ("linux", "x86_64") => "linux-amd64",
            ("linux", "aarch64") => "linux-aarch64",
            ("macos", "x86_64") => "osx-universal",
            ("macos", "aarch64") => "osx-universal",
            ("windows", "x86_64") => "win-amd64",
            _ => return Err(anyhow!("Unsupported platform: {}-{}", os, arch)),
        };

        Ok(platform.to_string())
    }

    async fn extract_duckdb(zip_path: &Path, target_path: &Path) -> Result<()> {
        let file = std::fs::File::open(zip_path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        let executable_name = if cfg!(target_os = "windows") {
            "duckdb.exe"
        } else {
            "duckdb"
        };

        // Find and extract the executable
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            
            if file.name().ends_with(executable_name) || file.name() == executable_name {
                let mut contents = Vec::new();
                std::io::Read::read_to_end(&mut file, &mut contents)?;
                fs::write(target_path, contents).await?;
                return Ok(());
            }
        }

        Err(anyhow!("DuckDB executable not found in archive"))
    }

    pub async fn execute_query(&self, database_path: &Path, query: &str) -> Result<String> {
        let output = AsyncCommand::new(&self.executable_path)
            .arg(database_path.to_string_lossy().as_ref())
            .arg("-c")
            .arg(query)
            .output()
            .await?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("DuckDB query failed: {}", error));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    pub async fn execute_script(&self, database_path: &Path, script_path: &Path) -> Result<String> {
        let output = AsyncCommand::new(&self.executable_path)
            .arg(database_path.to_string_lossy().as_ref())
            .arg("-init")
            .arg(script_path.to_string_lossy().as_ref())
            .output()
            .await?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("DuckDB script execution failed: {}", error));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    pub async fn import_csv(&self, database_path: &Path, table_name: &str, csv_path: &Path) -> Result<()> {
        let query = format!(
            "CREATE OR REPLACE TABLE {} AS SELECT * FROM read_csv_auto('{}');",
            table_name,
            csv_path.to_string_lossy()
        );
        
        self.execute_query(database_path, &query).await?;
        Ok(())
    }

    pub async fn export_csv(&self, database_path: &Path, table_name: &str, csv_path: &Path) -> Result<()> {
        let query = format!(
            "COPY {} TO '{}' WITH (FORMAT CSV, HEADER);",
            table_name,
            csv_path.to_string_lossy()
        );
        
        self.execute_query(database_path, &query).await?;
        Ok(())
    }

    pub async fn get_database_path(&self, project_id: &str) -> PathBuf {
        self.data_dir.join("databases").join(format!("{}.duckdb", project_id))
    }

    pub async fn ensure_database_dir(&self) -> Result<()> {
        let db_dir = self.data_dir.join("databases");
        fs::create_dir_all(db_dir).await?;
        Ok(())
    }

    pub async fn list_tables(&self, database_path: &Path) -> Result<Vec<String>> {
        let output = self.execute_query(database_path, "SHOW TABLES;").await?;
        
        let tables = output
            .lines()
            .skip(1) // Skip header
            .filter_map(|line| {
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() >= 1 {
                    Some(parts[0].trim().to_string())
                } else {
                    None
                }
            })
            .collect();

        Ok(tables)
    }

    pub async fn get_table_info(&self, database_path: &Path, table_name: &str) -> Result<String> {
        let query = format!("DESCRIBE {};", table_name);
        self.execute_query(database_path, &query).await
    }

    pub async fn get_table_row_count(&self, database_path: &Path, table_name: &str) -> Result<u64> {
        let query = format!("SELECT COUNT(*) FROM {};", table_name);
        let output = self.execute_query(database_path, &query).await?;
        
        // Parse the count from the output
        let count_line = output.lines().nth(1).unwrap_or("0");
        let count_str = count_line.split('|').next().unwrap_or("0").trim();
        Ok(count_str.parse::<u64>().unwrap_or(0))
    }

    pub fn get_executable_path_ref(&self) -> &Path {
        &self.executable_path
    }
}