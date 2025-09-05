use std::fs;
use std::path::{Path, PathBuf};
use regex::Regex;
use tracing::{info, warn, error};

/// Automatically organizes any loose log files in the root of claude_logs directory
pub fn auto_organize_logs(logs_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Only reorganize files that are directly in the root .claude_logs directory
    // This prevents reorganizing already organized files
    let entries = match std::fs::read_dir(logs_dir) {
        Ok(entries) => entries,
        Err(_) => return Ok(()), // Directory doesn't exist yet, nothing to organize
    };

    let filename_regex = Regex::new(r"^query_(\d{4})(\d{2})(\d{2})_\d{6}(?:_stderr)?\.log$")?;
    
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        
        // Only process files that are directly in the root directory
        if !path.is_file() || path.extension().is_none_or(|ext| ext != "log") {
            continue;
        }

        let filename = match path.file_name() {
            Some(name) => name.to_string_lossy(),
            None => continue,
        };

        // Check if it matches our log file pattern
        let captures = match filename_regex.captures(&filename) {
            Some(caps) => caps,
            None => continue,
        };

        let year = &captures[1];
        let month = &captures[2];
        let day = &captures[3];
        
        let month_year = format!("{}-{}", year, month);
        let target_dir = logs_dir.join(&month_year).join(day);
        let target_file = target_dir.join(&*filename);

        // Skip if already in organized location
        if path.parent() != Some(logs_dir) {
            continue;
        }

        // Skip if target already exists
        if target_file.exists() {
            warn!("Target log file already exists, skipping: {:?}", target_file);
            continue;
        }

        // Create target directory and move file
        if let Err(e) = std::fs::create_dir_all(&target_dir) {
            warn!("Failed to create log directory {:?}: {}", target_dir, e);
            continue;
        }

        if let Err(e) = std::fs::rename(&path, &target_file) {
            warn!("Failed to move log file {:?} to {:?}: {}", path, target_file, e);
        } else {
            info!("Auto-organized log: {} -> {}/{}/", filename, month_year, day);
        }
    }

    Ok(())
}

/// Reorganizes existing claude_logs from flat structure to organized month-year/day structure
#[allow(dead_code)]
pub fn reorganize_claude_logs(logs_dir: &Path, dry_run: bool) -> Result<(usize, usize), Box<dyn std::error::Error>> {
    if !logs_dir.exists() {
        return Err(format!("Directory does not exist: {:?}", logs_dir).into());
    }

    let mut moved_count = 0;
    let mut skipped_count = 0;

    info!("{}Processing: {:?}", if dry_run { "[DRY RUN] " } else { "" }, logs_dir);

    // Pattern to match query_YYYYMMDD_HHMMSS.log or query_YYYYMMDD_HHMMSS_stderr.log
    let filename_regex = Regex::new(r"^query_(\d{4})(\d{2})(\d{2})_\d{6}(?:_stderr)?\.log$")?;

    // Get all log files in the directory
    let entries = fs::read_dir(logs_dir)?;
    let mut log_files = Vec::new();

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "log") {
            log_files.push(path);
        }
    }

    if log_files.is_empty() {
        info!("  No log files found");
        return Ok((0, 0));
    }

    info!("  Found {} log files", log_files.len());

    for log_file in log_files {
        let filename = match log_file.file_name() {
            Some(name) => name.to_string_lossy(),
            None => {
                warn!("  ⚠️  Skipping file with no name: {:?}", log_file);
                skipped_count += 1;
                continue;
            }
        };

        // Parse filename to extract date components
        let captures = match filename_regex.captures(&filename) {
            Some(caps) => caps,
            None => {
                warn!("  ⚠️  Skipping unrecognized filename: {}", filename);
                skipped_count += 1;
                continue;
            }
        };

        let year = &captures[1];
        let month = &captures[2];
        let day = &captures[3];

        let month_year = format!("{}-{}", year, month);

        // Create target directory structure
        let target_dir = logs_dir.join(&month_year).join(day);
        let target_file = target_dir.join(&*filename);

        // Check if file is already in the right place
        if log_file.parent() == Some(&target_dir) {
            info!("  ✓  Already organized: {}", filename);
            skipped_count += 1;
            continue;
        }

        // Check if target file already exists
        if target_file.exists() {
            warn!("  ⚠️  Target exists, skipping: {}", filename);
            skipped_count += 1;
            continue;
        }

        if dry_run {
            info!("  📁 Would move: {} -> {}/{}/", filename, month_year, day);
        } else {
            // Create target directory if it doesn't exist
            if let Err(e) = fs::create_dir_all(&target_dir) {
                error!("  ❌ Error creating directory {:?}: {}", target_dir, e);
                skipped_count += 1;
                continue;
            }

            // Move the file
            match fs::rename(&log_file, &target_file) {
                Ok(()) => {
                    info!("  ✅ Moved: {} -> {}/{}/", filename, month_year, day);
                    moved_count += 1;
                }
                Err(e) => {
                    error!("  ❌ Error moving {}: {}", filename, e);
                    skipped_count += 1;
                }
            }
        }
    }

    Ok((moved_count, skipped_count))
}

/// Find all .claude_logs directories recursively from a base path
#[allow(dead_code)]
pub fn find_claude_logs_directories(base_path: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut claude_logs_dirs = Vec::new();

    // If the path itself is a .claude_logs directory
    if base_path.file_name().is_some_and(|name| name == ".claude_logs") && base_path.exists() {
        claude_logs_dirs.push(base_path.to_path_buf());
        return Ok(claude_logs_dirs);
    }

    // Search recursively for .claude_logs directories
    fn search_recursive(dir: &Path, results: &mut Vec<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
        if !dir.is_dir() {
            return Ok(());
        }

        let entries = fs::read_dir(dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                if path.file_name().is_some_and(|name| name == ".claude_logs") {
                    results.push(path);
                } else {
                    // Recursively search subdirectories, but skip hidden dirs except .claude_logs
                    if !path.file_name().unwrap().to_string_lossy().starts_with('.') {
                        search_recursive(&path, results)?;
                    }
                }
            }
        }
        Ok(())
    }

    search_recursive(base_path, &mut claude_logs_dirs)?;
    Ok(claude_logs_dirs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_find_claude_logs_directories() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();
        
        // Create a .claude_logs directory
        let claude_logs_dir = base_path.join("test_client").join(".claude_logs");
        fs::create_dir_all(&claude_logs_dir).unwrap();
        
        let found_dirs = find_claude_logs_directories(base_path).unwrap();
        assert_eq!(found_dirs.len(), 1);
        assert_eq!(found_dirs[0], claude_logs_dir);
    }

    #[test]
    fn test_reorganize_logs_dry_run() {
        let temp_dir = TempDir::new().unwrap();
        let logs_dir = temp_dir.path().join(".claude_logs");
        fs::create_dir_all(&logs_dir).unwrap();
        
        // Create a test log file
        let log_file = logs_dir.join("query_20250902_075519.log");
        fs::write(&log_file, "test log content").unwrap();
        
        let (moved, _skipped) = reorganize_claude_logs(&logs_dir, true).unwrap();
        
        // In dry run, nothing should be moved
        assert_eq!(moved, 0);
        // File should still exist in original location
        assert!(log_file.exists());
    }

    #[test]
    fn test_reorganize_logs_actual() {
        let temp_dir = TempDir::new().unwrap();
        let logs_dir = temp_dir.path().join(".claude_logs");
        fs::create_dir_all(&logs_dir).unwrap();
        
        // Create test log files
        let log_file1 = logs_dir.join("query_20250902_075519.log");
        let log_file2 = logs_dir.join("query_20250903_123456_stderr.log");
        fs::write(&log_file1, "test log content 1").unwrap();
        fs::write(&log_file2, "test log content 2").unwrap();
        
        let (moved, skipped) = reorganize_claude_logs(&logs_dir, false).unwrap();
        
        assert_eq!(moved, 2);
        assert_eq!(skipped, 0);
        
        // Check that files were moved to correct locations
        let expected_path1 = logs_dir.join("2025-09").join("02").join("query_20250902_075519.log");
        let expected_path2 = logs_dir.join("2025-09").join("03").join("query_20250903_123456_stderr.log");
        
        assert!(expected_path1.exists());
        assert!(expected_path2.exists());
        assert!(!log_file1.exists());
        assert!(!log_file2.exists());
    }

    #[test]
    fn test_auto_organize_logs() {
        let temp_dir = TempDir::new().unwrap();
        let logs_dir = temp_dir.path().join(".claude_logs");
        fs::create_dir_all(&logs_dir).unwrap();
        
        // Create test log files in root directory
        let log_file1 = logs_dir.join("query_20250902_075519.log");
        let log_file2 = logs_dir.join("query_20250903_123456_stderr.log");
        fs::write(&log_file1, "test log content 1").unwrap();
        fs::write(&log_file2, "test log content 2").unwrap();
        
        // Create an already organized file to ensure it's not moved
        let organized_dir = logs_dir.join("2025-08").join("15");
        fs::create_dir_all(&organized_dir).unwrap();
        let organized_file = organized_dir.join("query_20250815_120000.log");
        fs::write(&organized_file, "already organized").unwrap();
        
        // Run auto-organize
        auto_organize_logs(&logs_dir).unwrap();
        
        // Check that files were moved to correct locations
        let expected_path1 = logs_dir.join("2025-09").join("02").join("query_20250902_075519.log");
        let expected_path2 = logs_dir.join("2025-09").join("03").join("query_20250903_123456_stderr.log");
        
        assert!(expected_path1.exists());
        assert!(expected_path2.exists());
        assert!(!log_file1.exists());
        assert!(!log_file2.exists());
        
        // Ensure already organized file wasn't moved
        assert!(organized_file.exists());
    }

    #[test]
    fn test_auto_organize_logs_no_directory() {
        let temp_dir = TempDir::new().unwrap();
        let non_existent_dir = temp_dir.path().join("non_existent");
        
        // Should not error when directory doesn't exist
        auto_organize_logs(&non_existent_dir).unwrap();
    }
}