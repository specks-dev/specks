//! Implementation of log management commands
//!
//! Provides log rotation and prepend commands for managing the implementation log.

use clap::Subcommand;
use serde_json;

/// Threshold for log rotation by line count (per D01)
#[allow(dead_code)] // Will be used in step-1 implementation
pub const LOG_LINE_THRESHOLD: usize = 500;

/// Threshold for log rotation by byte size (per D01) - 100KB
#[allow(dead_code)] // Will be used in step-1 implementation
pub const LOG_BYTE_THRESHOLD: usize = 102400;

/// Log subcommands
#[derive(Subcommand, Debug)]
pub enum LogCommands {
    /// Rotate implementation log when over threshold
    ///
    /// Archives log when it exceeds 500 lines or 100KB.
    #[command(
        long_about = "Rotate implementation log when over threshold.\n\nRotation triggers when:\n  - Log exceeds 500 lines, OR\n  - Log exceeds 100KB (102400 bytes)\n\nArchives to .specks/archive/implementation-log-YYYY-MM-DD-HHMMSS.md\nCreates fresh log with header template.\n\nUse --force to rotate even when below thresholds."
    )]
    Rotate {
        /// Rotate even if below thresholds
        #[arg(long)]
        force: bool,
    },

    /// Prepend entry to implementation log
    ///
    /// Atomically adds a new entry to the log.
    #[command(
        long_about = "Prepend entry to implementation log.\n\nAdds a YAML frontmatter entry with:\n  - Step anchor\n  - Speck path\n  - Summary text\n  - Optional bead ID\n  - Timestamp\n\nEntry is inserted after the header section."
    )]
    Prepend {
        /// Step anchor (e.g., #step-0)
        #[arg(long)]
        step: String,

        /// Speck file path
        #[arg(long)]
        speck: String,

        /// One-line summary of completed work
        #[arg(long)]
        summary: String,

        /// Optional bead ID to record
        #[arg(long)]
        bead: Option<String>,
    },
}

/// Generate timestamp in YYYY-MM-DD-HHMMSS format for archive filenames
fn generate_archive_timestamp() -> Result<String, String> {
    use std::time::{SystemTime, UNIX_EPOCH};

    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("System time error: {}", e))?;

    let secs = duration.as_secs();

    // Convert to date/time components
    const SECONDS_PER_DAY: u64 = 86400;
    const DAYS_TO_EPOCH: i64 = 719162; // Days from 0000-01-01 to 1970-01-01

    let days_since_epoch = (secs / SECONDS_PER_DAY) as i64;
    let seconds_today = secs % SECONDS_PER_DAY;

    let hours = seconds_today / 3600;
    let minutes = (seconds_today % 3600) / 60;
    let seconds = seconds_today % 60;

    // Calculate year, month, day
    let total_days = DAYS_TO_EPOCH + days_since_epoch;

    let mut year = (total_days / 365) as i32;
    let mut remaining_days = total_days - year_to_days(year);

    while remaining_days < 0 {
        year -= 1;
        remaining_days = total_days - year_to_days(year);
    }
    while remaining_days >= days_in_year(year) {
        remaining_days -= days_in_year(year);
        year += 1;
    }

    let is_leap = is_leap_year(year);
    let mut month = 1;
    let mut day = remaining_days + 1;

    let days_in_months = if is_leap {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    for (m, &days) in days_in_months.iter().enumerate() {
        if day <= days as i64 {
            month = m + 1;
            break;
        }
        day -= days as i64;
    }

    Ok(format!(
        "{:04}-{:02}-{:02}-{:02}{:02}{:02}",
        year, month, day, hours, minutes, seconds
    ))
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn days_in_year(year: i32) -> i64 {
    if is_leap_year(year) { 366 } else { 365 }
}

fn year_to_days(year: i32) -> i64 {
    let y = year as i64;
    y * 365 + y / 4 - y / 100 + y / 400
}

/// Run the log rotate command
///
/// # Arguments
/// * `force` - Rotate even if below thresholds
/// * `json_output` - Output in JSON format
/// * `quiet` - Suppress non-error output
pub fn run_log_rotate(force: bool, json_output: bool, quiet: bool) -> Result<i32, String> {
    use crate::output::{JsonResponse, RotateData};
    use std::fs;
    use std::path::Path;

    let log_path = Path::new(".specks/specks-implementation-log.md");

    // Check if log file exists
    if !log_path.exists() {
        let data = RotateData {
            rotated: false,
            archived_path: None,
            original_lines: None,
            original_bytes: None,
            reason: "not_needed".to_string(),
        };

        if json_output {
            let response = JsonResponse::ok("log rotate", data);
            println!("{}", serde_json::to_string_pretty(&response).unwrap());
        } else if !quiet {
            println!("Implementation log does not exist - nothing to rotate");
        }
        return Ok(0);
    }

    // Read the log file to check thresholds
    let content = fs::read_to_string(log_path)
        .map_err(|e| format!("Failed to read implementation log: {}", e))?;

    let line_count = content.lines().count();
    let byte_count = content.len();

    // Determine if rotation is needed
    let should_rotate = force || line_count > LOG_LINE_THRESHOLD || byte_count > LOG_BYTE_THRESHOLD;

    if !should_rotate {
        let data = RotateData {
            rotated: false,
            archived_path: None,
            original_lines: Some(line_count),
            original_bytes: Some(byte_count),
            reason: "not_needed".to_string(),
        };

        if json_output {
            let response = JsonResponse::ok("log rotate", data);
            println!("{}", serde_json::to_string_pretty(&response).unwrap());
        } else if !quiet {
            println!(
                "Log below thresholds ({} lines, {} bytes) - no rotation needed",
                line_count, byte_count
            );
        }
        return Ok(0);
    }

    // Determine rotation reason
    let reason = if force {
        "forced"
    } else if line_count > LOG_LINE_THRESHOLD {
        "line_count_exceeded"
    } else {
        "byte_size_exceeded"
    };

    // Create archive directory if it doesn't exist
    let archive_dir = Path::new(".specks/archive");
    if !archive_dir.exists() {
        fs::create_dir_all(archive_dir)
            .map_err(|e| format!("Failed to create archive directory: {}", e))?;
    }

    // Generate archive filename with timestamp (D02)
    let timestamp = generate_archive_timestamp()?;
    let archive_filename = format!("implementation-log-{}.md", timestamp);
    let archive_path = archive_dir.join(&archive_filename);

    // Atomic rename from log to archive
    fs::rename(log_path, &archive_path)
        .map_err(|e| format!("Failed to move log to archive: {}", e))?;

    // Create fresh log with header template
    const IMPLEMENTATION_LOG_HEADER: &str = r#"# Specks Implementation Log

This file documents the implementation progress for this project.

**Format:** Each entry records a completed step with tasks, files, and verification results.

Entries are sorted newest-first.

---

"#;

    fs::write(log_path, IMPLEMENTATION_LOG_HEADER)
        .map_err(|e| format!("Failed to create fresh log file: {}", e))?;

    // Build response data
    let archived_path_str = format!(".specks/archive/{}", archive_filename);
    let data = RotateData {
        rotated: true,
        archived_path: Some(archived_path_str.clone()),
        original_lines: Some(line_count),
        original_bytes: Some(byte_count),
        reason: reason.to_string(),
    };

    if json_output {
        let response = JsonResponse::ok("log rotate", data);
        println!("{}", serde_json::to_string_pretty(&response).unwrap());
    } else if !quiet {
        println!("Log rotated successfully");
        println!("  Original: {} lines, {} bytes", line_count, byte_count);
        println!("  Archived to: {}", archived_path_str);
        println!("  Reason: {}", reason);
    }

    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_timestamp_generation() {
        let timestamp = generate_archive_timestamp().unwrap();
        // Should match YYYY-MM-DD-HHMMSS pattern (17 chars total)
        assert_eq!(timestamp.len(), 17); // YYYY-MM-DD-HHMMSS = 4+1+2+1+2+1+6 = 17
        assert_eq!(&timestamp[4..5], "-");
        assert_eq!(&timestamp[7..8], "-");
        assert_eq!(&timestamp[10..11], "-");
    }

    #[test]
    fn test_log_rotation_line_threshold() {
        let original_dir = std::env::current_dir().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Change to temp directory
        std::env::set_current_dir(temp_path).unwrap();

        // Create .specks directory
        fs::create_dir_all(".specks").unwrap();

        // Create log with > 500 lines (exceeds LOG_LINE_THRESHOLD)
        let log_path = temp_path.join(".specks/specks-implementation-log.md");
        let mut content = String::new();
        for i in 0..510 {
            content.push_str(&format!("Line {}\n", i));
        }
        fs::write(&log_path, content).unwrap();

        // Run rotation (should rotate)
        let result = run_log_rotate(false, false, true);
        assert!(result.is_ok());

        // Verify archive was created
        let archive_dir = temp_path.join(".specks/archive");
        assert!(archive_dir.exists());
        let archive_files: Vec<_> = fs::read_dir(&archive_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(archive_files.len(), 1);

        // Verify fresh log was created
        assert!(log_path.exists());
        let new_content = fs::read_to_string(&log_path).unwrap();
        assert!(new_content.contains("# Specks Implementation Log"));
        assert!(new_content.len() < 500); // Fresh log is much smaller

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_log_rotation_byte_threshold() {
        let original_dir = std::env::current_dir().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Change to temp directory
        std::env::set_current_dir(temp_path).unwrap();

        // Create .specks directory
        fs::create_dir_all(".specks").unwrap();

        // Create log with > 100KB (exceeds LOG_BYTE_THRESHOLD)
        let log_path = temp_path.join(".specks/specks-implementation-log.md");
        let line = "x".repeat(1000); // 1000 bytes per line
        let mut content = String::new();
        for i in 0..110 {
            // 110KB total
            content.push_str(&format!("Line {}: {}\n", i, line));
        }
        fs::write(&log_path, content).unwrap();

        // Run rotation (should rotate)
        let result = run_log_rotate(false, false, true);
        assert!(result.is_ok());

        // Verify archive was created
        let archive_dir = temp_path.join(".specks/archive");
        assert!(archive_dir.exists());

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_log_rotation_under_thresholds() {
        let original_dir = std::env::current_dir().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Change to temp directory
        std::env::set_current_dir(temp_path).unwrap();

        // Create .specks directory
        fs::create_dir_all(".specks").unwrap();

        // Create small log (under both thresholds)
        let log_path = temp_path.join(".specks/specks-implementation-log.md");
        let mut content = String::new();
        for i in 0..100 {
            // Only 100 lines
            content.push_str(&format!("Line {}\n", i));
        }
        fs::write(&log_path, content).unwrap();

        // Run rotation without force (should NOT rotate)
        let result = run_log_rotate(false, false, true);
        assert!(result.is_ok());

        // Verify archive was NOT created
        let archive_dir = temp_path.join(".specks/archive");
        assert!(!archive_dir.exists());

        // Verify log still exists and unchanged
        assert!(log_path.exists());
        let new_content = fs::read_to_string(&log_path).unwrap();
        assert!(new_content.contains("Line 99"));

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_log_rotation_force() {
        let original_dir = std::env::current_dir().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Change to temp directory
        std::env::set_current_dir(temp_path).unwrap();

        // Create .specks directory
        fs::create_dir_all(".specks").unwrap();

        // Create small log (under both thresholds)
        let log_path = temp_path.join(".specks/specks-implementation-log.md");
        fs::write(&log_path, "Small log\n").unwrap();

        // Run rotation WITH force (should rotate even though under thresholds)
        let result = run_log_rotate(true, false, true);
        assert!(result.is_ok());

        // Verify archive was created
        let archive_dir = temp_path.join(".specks/archive");
        assert!(archive_dir.exists());
        let archive_files: Vec<_> = fs::read_dir(&archive_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(archive_files.len(), 1);

        // Verify fresh log was created
        assert!(log_path.exists());
        let new_content = fs::read_to_string(&log_path).unwrap();
        assert!(new_content.contains("# Specks Implementation Log"));

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_log_rotation_nonexistent_log() {
        let original_dir = std::env::current_dir().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Change to temp directory
        std::env::set_current_dir(temp_path).unwrap();

        // Create .specks directory but NO log
        fs::create_dir_all(".specks").unwrap();

        // Run rotation (should succeed but not rotate)
        let result = run_log_rotate(false, false, true);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);

        // Verify no archive created
        let archive_dir = temp_path.join(".specks/archive");
        assert!(!archive_dir.exists());

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_archive_filename_format() {
        let original_dir = std::env::current_dir().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        std::env::set_current_dir(temp_path).unwrap();
        fs::create_dir_all(".specks").unwrap();

        // Create log that exceeds threshold
        let log_path = temp_path.join(".specks/specks-implementation-log.md");
        let mut content = String::new();
        for i in 0..510 {
            content.push_str(&format!("Line {}\n", i));
        }
        fs::write(&log_path, content).unwrap();

        // Run rotation
        run_log_rotate(false, false, true).unwrap();

        // Check archive filename matches pattern implementation-log-YYYY-MM-DD-HHMMSS.md
        let archive_dir = temp_path.join(".specks/archive");
        let archive_files: Vec<_> = fs::read_dir(&archive_dir)
            .unwrap()
            .filter_map(|e| e.ok().and_then(|e| e.file_name().into_string().ok()))
            .collect();

        assert_eq!(archive_files.len(), 1);
        let filename = &archive_files[0];
        assert!(filename.starts_with("implementation-log-"));
        assert!(filename.ends_with(".md"));
        // Check timestamp format: YYYY-MM-DD-HHMMSS (17 chars between prefix and .md)
        let timestamp_part = &filename["implementation-log-".len()..filename.len() - 3];
        assert_eq!(timestamp_part.len(), 17);

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_iso8601_timestamp_format() {
        let timestamp = generate_iso8601_timestamp().unwrap();
        // Should match YYYY-MM-DDTHH:MM:SSZ pattern (20 chars total)
        assert_eq!(timestamp.len(), 20);
        assert_eq!(&timestamp[4..5], "-");
        assert_eq!(&timestamp[7..8], "-");
        assert_eq!(&timestamp[10..11], "T");
        assert_eq!(&timestamp[13..14], ":");
        assert_eq!(&timestamp[16..17], ":");
        assert_eq!(&timestamp[19..20], "Z");
    }

    #[test]
    fn test_yaml_entry_generation() {
        let entry = generate_yaml_entry(
            "#step-0",
            ".specks/specks-13.md",
            "Test summary",
            Some("bd-123"),
            "2026-02-09T14:30:00Z",
        );

        assert!(entry.contains("step: #step-0"));
        assert!(entry.contains("date: 2026-02-09T14:30:00Z"));
        assert!(entry.contains("bead: bd-123"));
        assert!(entry.contains("## #step-0: Test summary"));
        assert!(entry.contains("- .specks/specks-13.md"));
    }

    #[test]
    fn test_yaml_entry_generation_no_bead() {
        let entry = generate_yaml_entry(
            "#step-1",
            ".specks/specks-13.md",
            "Test summary",
            None,
            "2026-02-09T14:30:00Z",
        );

        assert!(entry.contains("step: #step-1"));
        assert!(entry.contains("date: 2026-02-09T14:30:00Z"));
        assert!(!entry.contains("bead:"));
        assert!(entry.contains("## #step-1: Test summary"));
    }

    #[test]
    fn test_find_insertion_point() {
        let content = "# Specks Implementation Log\n\nHeader content.\n\n---\n\nExisting entry\n";
        let pos = find_insertion_point(content);

        // Should insert after the "---\n\n" separator
        let expected_pos = "# Specks Implementation Log\n\nHeader content.\n\n---\n\n".len();
        assert_eq!(pos, expected_pos);
    }

    #[test]
    fn test_find_insertion_point_empty_log() {
        let content = "# Specks Implementation Log\n\n---\n\n";
        let pos = find_insertion_point(content);

        // Should insert after the "---\n\n" separator
        let expected_pos = "# Specks Implementation Log\n\n---\n\n".len();
        assert_eq!(pos, expected_pos);
    }

    #[test]
    fn test_log_prepend_full_flow() {
        let original_dir = std::env::current_dir().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        std::env::set_current_dir(temp_path).unwrap();
        fs::create_dir_all(".specks").unwrap();

        // Create initial log with header
        let log_path = temp_path.join(".specks/specks-implementation-log.md");
        let initial_content = r#"# Specks Implementation Log

This file documents the implementation progress for this project.

**Format:** Each entry records a completed step with tasks, files, and verification results.

Entries are sorted newest-first.

---

"#;
        fs::write(&log_path, initial_content).unwrap();

        // Run prepend
        let result = run_log_prepend(
            "#step-0".to_string(),
            ".specks/specks-13.md".to_string(),
            "Test implementation".to_string(),
            Some("bd-123".to_string()),
            false,
            true,
        );
        assert!(result.is_ok());

        // Verify entry was added
        let new_content = fs::read_to_string(&log_path).unwrap();
        assert!(new_content.contains("step: #step-0"));
        assert!(new_content.contains("bead: bd-123"));
        assert!(new_content.contains("## #step-0: Test implementation"));
        assert!(new_content.contains("- .specks/specks-13.md"));

        // Verify entry is after the separator
        let separator_pos = new_content.find("---\n\n").unwrap();
        let entry_pos = new_content.find("step: #step-0").unwrap();
        assert!(entry_pos > separator_pos);

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_log_prepend_multiple_entries() {
        let original_dir = std::env::current_dir().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        std::env::set_current_dir(temp_path).unwrap();
        fs::create_dir_all(".specks").unwrap();

        // Create initial log
        let log_path = temp_path.join(".specks/specks-implementation-log.md");
        let initial_content = "# Specks Implementation Log\n\n---\n\n";
        fs::write(&log_path, initial_content).unwrap();

        // Add first entry
        run_log_prepend(
            "#step-0".to_string(),
            ".specks/specks-13.md".to_string(),
            "First entry".to_string(),
            None,
            false,
            true,
        )
        .unwrap();

        // Add second entry
        run_log_prepend(
            "#step-1".to_string(),
            ".specks/specks-13.md".to_string(),
            "Second entry".to_string(),
            None,
            false,
            true,
        )
        .unwrap();

        // Verify both entries exist and second is first
        let new_content = fs::read_to_string(&log_path).unwrap();
        let step0_pos = new_content.find("step: #step-0").unwrap();
        let step1_pos = new_content.find("step: #step-1").unwrap();
        assert!(step1_pos < step0_pos, "Newest entry should be first");

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_log_prepend_nonexistent_log() {
        let original_dir = std::env::current_dir().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        std::env::set_current_dir(temp_path).unwrap();
        fs::create_dir_all(".specks").unwrap();

        // Run prepend without log file
        let result = run_log_prepend(
            "#step-0".to_string(),
            ".specks/specks-13.md".to_string(),
            "Test".to_string(),
            None,
            false,
            true,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();
    }
}

/// Generate ISO 8601 timestamp (YYYY-MM-DDTHH:MM:SSZ)
fn generate_iso8601_timestamp() -> Result<String, String> {
    use std::time::{SystemTime, UNIX_EPOCH};

    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("System time error: {}", e))?;

    let secs = duration.as_secs();

    // Convert to date/time components
    const SECONDS_PER_DAY: u64 = 86400;
    const DAYS_TO_EPOCH: i64 = 719162; // Days from 0000-01-01 to 1970-01-01

    let days_since_epoch = (secs / SECONDS_PER_DAY) as i64;
    let seconds_today = secs % SECONDS_PER_DAY;

    let hours = seconds_today / 3600;
    let minutes = (seconds_today % 3600) / 60;
    let seconds = seconds_today % 60;

    // Calculate year, month, day
    let total_days = DAYS_TO_EPOCH + days_since_epoch;

    let mut year = (total_days / 365) as i32;
    let mut remaining_days = total_days - year_to_days(year);

    while remaining_days < 0 {
        year -= 1;
        remaining_days = total_days - year_to_days(year);
    }
    while remaining_days >= days_in_year(year) {
        remaining_days -= days_in_year(year);
        year += 1;
    }

    let is_leap = is_leap_year(year);
    let mut month = 1;
    let mut day = remaining_days + 1;

    let days_in_months = if is_leap {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    for (m, &days) in days_in_months.iter().enumerate() {
        if day <= days as i64 {
            month = m + 1;
            break;
        }
        day -= days as i64;
    }

    Ok(format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    ))
}

/// Generate YAML frontmatter entry for the implementation log
fn generate_yaml_entry(
    step: &str,
    speck: &str,
    summary: &str,
    bead: Option<&str>,
    timestamp: &str,
) -> String {
    let mut entry = String::new();
    entry.push_str("---\n");
    entry.push_str(&format!("step: {}\n", step));
    entry.push_str(&format!("date: {}\n", timestamp));
    if let Some(bead_id) = bead {
        entry.push_str(&format!("bead: {}\n", bead_id));
    }
    entry.push_str("---\n");
    entry.push('\n');
    entry.push_str(&format!("## {}: {}\n", step, summary));
    entry.push('\n');
    entry.push_str("**Files changed:**\n");
    entry.push_str(&format!("- {}\n", speck));
    entry.push('\n');
    entry.push_str("---\n");
    entry.push('\n');
    entry
}

/// Find the insertion point after the header section
fn find_insertion_point(content: &str) -> usize {
    // Look for the pattern "---\n\n" which marks the end of the header
    // The insertion point is right after the second newline

    // Search for "---\n\n" pattern
    if let Some(pos) = content.find("---\n\n") {
        // Return position after "---\n\n" (5 bytes)
        return pos + 5;
    }

    // Fallback: look for just "---\n" if double newline not found
    if let Some(pos) = content.find("---\n") {
        // Return position after "---\n" (4 bytes)
        return pos + 4;
    }

    // If no separator found, insert at the end
    content.len()
}

/// Run the log prepend command
///
/// # Arguments
/// * `step` - Step anchor (e.g., #step-0)
/// * `speck` - Speck file path
/// * `summary` - One-line summary of completed work
/// * `bead` - Optional bead ID to record
/// * `json_output` - Output in JSON format
/// * `quiet` - Suppress non-error output
pub fn run_log_prepend(
    step: String,
    speck: String,
    summary: String,
    bead: Option<String>,
    json_output: bool,
    quiet: bool,
) -> Result<i32, String> {
    use crate::output::{JsonResponse, PrependData};
    use std::fs;
    use std::path::Path;

    let log_path = Path::new(".specks/specks-implementation-log.md");

    // Check if log file exists
    if !log_path.exists() {
        return Err("Implementation log does not exist. Run 'specks init' first.".to_string());
    }

    // Read the current log
    let content = fs::read_to_string(log_path)
        .map_err(|e| format!("Failed to read implementation log: {}", e))?;

    // Generate timestamp
    let timestamp = generate_iso8601_timestamp()?;

    // Generate YAML entry
    let entry = generate_yaml_entry(&step, &speck, &summary, bead.as_deref(), &timestamp);

    // Find insertion point
    let insertion_point = find_insertion_point(&content);

    // Build new content
    let mut new_content = String::new();
    new_content.push_str(&content[..insertion_point]);
    new_content.push_str(&entry);
    new_content.push_str(&content[insertion_point..]);

    // Write atomically by writing to a temp file then renaming
    let temp_path = log_path.with_extension("md.tmp");
    fs::write(&temp_path, &new_content)
        .map_err(|e| format!("Failed to write temp log file: {}", e))?;
    fs::rename(&temp_path, log_path).map_err(|e| format!("Failed to update log file: {}", e))?;

    // Build response data
    let data = PrependData {
        entry_added: true,
        step: Some(step.clone()),
        speck: Some(speck.clone()),
        timestamp: Some(timestamp.clone()),
    };

    if json_output {
        let response = JsonResponse::ok("log prepend", data);
        println!("{}", serde_json::to_string_pretty(&response).unwrap());
    } else if !quiet {
        println!("Entry prepended to implementation log");
        println!("  Step: {}", step);
        println!("  Speck: {}", speck);
        println!("  Timestamp: {}", timestamp);
        if let Some(bead_id) = bead {
            println!("  Bead: {}", bead_id);
        }
    }

    Ok(0)
}
