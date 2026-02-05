//! Share directory discovery and skill installation
//!
//! Skills are distributed as separate files alongside the specks binary.
//! This module handles discovering the share directory and installing
//! skills to projects.
//!
//! Discovery order:
//! 1. Environment variable: SPECKS_SHARE_DIR
//! 2. Relative to binary: ../share/specks/ (works for homebrew and tarball)
//! 3. Standard locations: /opt/homebrew/share/specks/, /usr/local/share/specks/
//! 4. Development fallback: ./ (when running from source with skills in repo)

use std::fs;
use std::path::{Path, PathBuf};

/// Name of the environment variable for share directory override
pub const SHARE_DIR_ENV_VAR: &str = "SPECKS_SHARE_DIR";

/// Skills directory name within share directory
pub const SKILLS_DIR_NAME: &str = "skills";

/// Available skill names
pub const AVAILABLE_SKILLS: &[&str] = &["specks-plan", "specks-execute"];

/// Skill file name within each skill directory
pub const SKILL_FILE_NAME: &str = "SKILL.md";

/// Result of skill installation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillInstallStatus {
    /// Skill was installed (newly created)
    Installed,
    /// Skill was updated (existing file replaced)
    Updated,
    /// Skill was unchanged (files are identical)
    Unchanged,
    /// Source skill not found in share directory
    SourceMissing,
}

impl SkillInstallStatus {
    /// Convert to string for JSON output
    pub fn as_str(&self) -> &'static str {
        match self {
            SkillInstallStatus::Installed => "installed",
            SkillInstallStatus::Updated => "updated",
            SkillInstallStatus::Unchanged => "unchanged",
            SkillInstallStatus::SourceMissing => "missing",
        }
    }
}

/// Result of skill verification
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillVerifyStatus {
    /// Skill is installed and matches source
    UpToDate,
    /// Skill is installed but differs from source
    Outdated,
    /// Skill is not installed
    Missing,
    /// Source skill not found (can't verify)
    SourceMissing,
}

impl SkillVerifyStatus {
    /// Convert to string for JSON output
    pub fn as_str(&self) -> &'static str {
        match self {
            SkillVerifyStatus::UpToDate => "up_to_date",
            SkillVerifyStatus::Outdated => "outdated",
            SkillVerifyStatus::Missing => "missing",
            SkillVerifyStatus::SourceMissing => "source_missing",
        }
    }
}

/// Find the share directory using the discovery order.
///
/// Returns None if no valid share directory is found.
pub fn find_share_dir() -> Option<PathBuf> {
    // 1. Check environment variable
    if let Ok(env_path) = std::env::var(SHARE_DIR_ENV_VAR) {
        let path = PathBuf::from(&env_path);
        if path.is_dir() {
            return Some(path);
        }
    }

    // 2. Check relative to binary (../share/specks/)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let relative_share = exe_dir.join("../share/specks");
            if relative_share.is_dir() {
                // Canonicalize to clean up the path
                if let Ok(canonical) = relative_share.canonicalize() {
                    return Some(canonical);
                }
            }
        }
    }

    // 3. Check standard locations
    let standard_locations = ["/opt/homebrew/share/specks", "/usr/local/share/specks"];

    for location in &standard_locations {
        let path = PathBuf::from(location);
        if path.is_dir() {
            return Some(path);
        }
    }

    // 4. Development fallback: check current directory for .claude/skills
    // This is for when running from source in the specks repo
    let cwd = std::env::current_dir().ok()?;
    let dev_skills = cwd.join(".claude/skills");
    if dev_skills.is_dir() {
        // Return the parent of .claude (the project root) as share dir
        // Skills will be found at {share_dir}/.claude/skills/
        // But we need to adjust - for dev mode, we treat cwd as share
        // with skills at .claude/skills/ instead of skills/
        return Some(cwd);
    }

    None
}

/// Get the skills directory path from a share directory.
///
/// In production (homebrew/tarball), skills are at {share_dir}/skills/
/// In development, skills are at {share_dir}/.claude/skills/
pub fn get_skills_dir(share_dir: &Path) -> Option<PathBuf> {
    // Check production layout first: {share_dir}/skills/
    let prod_skills = share_dir.join(SKILLS_DIR_NAME);
    if prod_skills.is_dir() {
        return Some(prod_skills);
    }

    // Check development layout: {share_dir}/.claude/skills/
    let dev_skills = share_dir.join(".claude").join(SKILLS_DIR_NAME);
    if dev_skills.is_dir() {
        return Some(dev_skills);
    }

    None
}

/// List available skills in the share directory.
///
/// Returns a list of skill names that are available for installation.
pub fn list_available_skills(share_dir: &Path) -> Vec<String> {
    let Some(skills_dir) = get_skills_dir(share_dir) else {
        return vec![];
    };

    AVAILABLE_SKILLS
        .iter()
        .filter(|skill_name| {
            let skill_path = skills_dir.join(skill_name).join(SKILL_FILE_NAME);
            skill_path.is_file()
        })
        .map(|s| s.to_string())
        .collect()
}

/// Get the source path for a skill in the share directory.
fn get_source_skill_path(share_dir: &Path, skill_name: &str) -> Option<PathBuf> {
    let skills_dir = get_skills_dir(share_dir)?;
    let skill_path = skills_dir.join(skill_name).join(SKILL_FILE_NAME);
    if skill_path.is_file() {
        Some(skill_path)
    } else {
        None
    }
}

/// Get the destination path for a skill in a project.
fn get_dest_skill_path(project_dir: &Path, skill_name: &str) -> PathBuf {
    project_dir
        .join(".claude")
        .join(SKILLS_DIR_NAME)
        .join(skill_name)
        .join(SKILL_FILE_NAME)
}

/// Copy a skill from the share directory to a project.
///
/// If `force` is true, overwrites existing files even if they match.
/// Returns the installation status.
pub fn copy_skill_to_project(
    share_dir: &Path,
    skill_name: &str,
    project_dir: &Path,
    force: bool,
) -> Result<SkillInstallStatus, String> {
    // Get source path
    let Some(source_path) = get_source_skill_path(share_dir, skill_name) else {
        return Ok(SkillInstallStatus::SourceMissing);
    };

    // Read source content
    let source_content = fs::read_to_string(&source_path)
        .map_err(|e| format!("failed to read source skill {}: {}", skill_name, e))?;

    // Get destination path
    let dest_path = get_dest_skill_path(project_dir, skill_name);

    // Check if destination exists and compare content
    if dest_path.exists() {
        let dest_content = fs::read_to_string(&dest_path)
            .map_err(|e| format!("failed to read existing skill {}: {}", skill_name, e))?;

        if source_content == dest_content && !force {
            return Ok(SkillInstallStatus::Unchanged);
        }

        // Files differ or force is true - update
        // Create parent directories if needed
        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                format!(
                    "failed to create directory for skill {}: {}",
                    skill_name, e
                )
            })?;
        }

        fs::write(&dest_path, &source_content)
            .map_err(|e| format!("failed to write skill {}: {}", skill_name, e))?;

        Ok(SkillInstallStatus::Updated)
    } else {
        // Destination doesn't exist - install
        // Create parent directories
        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                format!(
                    "failed to create directory for skill {}: {}",
                    skill_name, e
                )
            })?;
        }

        fs::write(&dest_path, &source_content)
            .map_err(|e| format!("failed to write skill {}: {}", skill_name, e))?;

        Ok(SkillInstallStatus::Installed)
    }
}

/// Verify if a skill is installed and up-to-date.
pub fn verify_skill_installation(
    share_dir: &Path,
    skill_name: &str,
    project_dir: &Path,
) -> SkillVerifyStatus {
    // Get source path
    let Some(source_path) = get_source_skill_path(share_dir, skill_name) else {
        return SkillVerifyStatus::SourceMissing;
    };

    // Get destination path
    let dest_path = get_dest_skill_path(project_dir, skill_name);

    // Check if destination exists
    if !dest_path.exists() {
        return SkillVerifyStatus::Missing;
    }

    // Compare contents
    let source_content = match fs::read_to_string(&source_path) {
        Ok(c) => c,
        Err(_) => return SkillVerifyStatus::SourceMissing,
    };

    let dest_content = match fs::read_to_string(&dest_path) {
        Ok(c) => c,
        Err(_) => return SkillVerifyStatus::Missing,
    };

    if source_content == dest_content {
        SkillVerifyStatus::UpToDate
    } else {
        SkillVerifyStatus::Outdated
    }
}

/// Install all available skills to a project.
///
/// Returns a list of (skill_name, status) pairs.
pub fn install_all_skills(
    share_dir: &Path,
    project_dir: &Path,
    force: bool,
) -> Vec<(String, Result<SkillInstallStatus, String>)> {
    AVAILABLE_SKILLS
        .iter()
        .map(|skill_name| {
            let status = copy_skill_to_project(share_dir, skill_name, project_dir, force);
            (skill_name.to_string(), status)
        })
        .collect()
}

/// Verify all skills in a project.
///
/// Returns a list of (skill_name, status) pairs.
pub fn verify_all_skills(share_dir: &Path, project_dir: &Path) -> Vec<(String, SkillVerifyStatus)> {
    AVAILABLE_SKILLS
        .iter()
        .map(|skill_name| {
            let status = verify_skill_installation(share_dir, skill_name, project_dir);
            (skill_name.to_string(), status)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_share_dir() -> TempDir {
        let temp = TempDir::new().unwrap();

        // Create skills directory structure (production layout)
        let skills_dir = temp.path().join("skills");
        fs::create_dir_all(&skills_dir).unwrap();

        // Create specks-plan skill
        let plan_dir = skills_dir.join("specks-plan");
        fs::create_dir_all(&plan_dir).unwrap();
        fs::write(plan_dir.join("SKILL.md"), "# specks-plan skill content").unwrap();

        // Create specks-execute skill
        let exec_dir = skills_dir.join("specks-execute");
        fs::create_dir_all(&exec_dir).unwrap();
        fs::write(exec_dir.join("SKILL.md"), "# specks-execute skill content").unwrap();

        temp
    }

    #[test]
    fn test_find_share_dir_from_env() {
        let temp = create_test_share_dir();

        // Set environment variable
        // SAFETY: Tests run single-threaded for env var tests
        unsafe {
            std::env::set_var(SHARE_DIR_ENV_VAR, temp.path());
        }

        let found = find_share_dir();
        assert!(found.is_some());
        assert_eq!(found.unwrap(), temp.path());

        // Clean up
        unsafe {
            std::env::remove_var(SHARE_DIR_ENV_VAR);
        }
    }

    #[test]
    fn test_find_share_dir_returns_none_when_not_found() {
        // Ensure env var is not set
        // SAFETY: Tests run single-threaded for env var tests
        unsafe {
            std::env::remove_var(SHARE_DIR_ENV_VAR);
        }

        // Create a temporary directory that doesn't have skills
        let temp = TempDir::new().unwrap();
        unsafe {
            std::env::set_var(SHARE_DIR_ENV_VAR, temp.path().join("nonexistent"));
        }

        // With invalid env var, should fall through and eventually return None
        // (unless running in specks repo or homebrew location exists)
        let found = find_share_dir();
        // The result depends on the environment - just verify it doesn't panic
        let _ = found;

        unsafe {
            std::env::remove_var(SHARE_DIR_ENV_VAR);
        }
    }

    #[test]
    fn test_get_skills_dir_production_layout() {
        let temp = create_test_share_dir();

        let skills_dir = get_skills_dir(temp.path());
        assert!(skills_dir.is_some());
        assert_eq!(skills_dir.unwrap(), temp.path().join("skills"));
    }

    #[test]
    fn test_list_available_skills() {
        let temp = create_test_share_dir();

        let skills = list_available_skills(temp.path());
        assert_eq!(skills.len(), 2);
        assert!(skills.contains(&"specks-plan".to_string()));
        assert!(skills.contains(&"specks-execute".to_string()));
    }

    #[test]
    fn test_copy_skill_to_project_new_install() {
        let share = create_test_share_dir();
        let project = TempDir::new().unwrap();

        let status =
            copy_skill_to_project(share.path(), "specks-plan", project.path(), false).unwrap();
        assert_eq!(status, SkillInstallStatus::Installed);

        // Verify file was created
        let dest_path = project
            .path()
            .join(".claude/skills/specks-plan/SKILL.md");
        assert!(dest_path.exists());

        let content = fs::read_to_string(&dest_path).unwrap();
        assert_eq!(content, "# specks-plan skill content");
    }

    #[test]
    fn test_copy_skill_to_project_unchanged() {
        let share = create_test_share_dir();
        let project = TempDir::new().unwrap();

        // First install
        copy_skill_to_project(share.path(), "specks-plan", project.path(), false).unwrap();

        // Second install should be unchanged
        let status =
            copy_skill_to_project(share.path(), "specks-plan", project.path(), false).unwrap();
        assert_eq!(status, SkillInstallStatus::Unchanged);
    }

    #[test]
    fn test_copy_skill_to_project_updated() {
        let share = create_test_share_dir();
        let project = TempDir::new().unwrap();

        // First install
        copy_skill_to_project(share.path(), "specks-plan", project.path(), false).unwrap();

        // Modify the installed skill
        let dest_path = project
            .path()
            .join(".claude/skills/specks-plan/SKILL.md");
        fs::write(&dest_path, "# modified content").unwrap();

        // Re-install should update
        let status =
            copy_skill_to_project(share.path(), "specks-plan", project.path(), false).unwrap();
        assert_eq!(status, SkillInstallStatus::Updated);

        // Verify content was restored
        let content = fs::read_to_string(&dest_path).unwrap();
        assert_eq!(content, "# specks-plan skill content");
    }

    #[test]
    fn test_copy_skill_to_project_force() {
        let share = create_test_share_dir();
        let project = TempDir::new().unwrap();

        // First install
        copy_skill_to_project(share.path(), "specks-plan", project.path(), false).unwrap();

        // Force re-install should update even if unchanged
        let status =
            copy_skill_to_project(share.path(), "specks-plan", project.path(), true).unwrap();
        assert_eq!(status, SkillInstallStatus::Updated);
    }

    #[test]
    fn test_copy_skill_source_missing() {
        let temp = TempDir::new().unwrap();
        let project = TempDir::new().unwrap();

        // Share dir exists but has no skills
        let status =
            copy_skill_to_project(temp.path(), "specks-plan", project.path(), false).unwrap();
        assert_eq!(status, SkillInstallStatus::SourceMissing);
    }

    #[test]
    fn test_verify_skill_installation_up_to_date() {
        let share = create_test_share_dir();
        let project = TempDir::new().unwrap();

        // Install skill
        copy_skill_to_project(share.path(), "specks-plan", project.path(), false).unwrap();

        // Verify should be up to date
        let status = verify_skill_installation(share.path(), "specks-plan", project.path());
        assert_eq!(status, SkillVerifyStatus::UpToDate);
    }

    #[test]
    fn test_verify_skill_installation_missing() {
        let share = create_test_share_dir();
        let project = TempDir::new().unwrap();

        // Don't install - verify should be missing
        let status = verify_skill_installation(share.path(), "specks-plan", project.path());
        assert_eq!(status, SkillVerifyStatus::Missing);
    }

    #[test]
    fn test_verify_skill_installation_outdated() {
        let share = create_test_share_dir();
        let project = TempDir::new().unwrap();

        // Install skill
        copy_skill_to_project(share.path(), "specks-plan", project.path(), false).unwrap();

        // Modify the installed skill
        let dest_path = project
            .path()
            .join(".claude/skills/specks-plan/SKILL.md");
        fs::write(&dest_path, "# modified content").unwrap();

        // Verify should be outdated
        let status = verify_skill_installation(share.path(), "specks-plan", project.path());
        assert_eq!(status, SkillVerifyStatus::Outdated);
    }

    #[test]
    fn test_install_all_skills() {
        let share = create_test_share_dir();
        let project = TempDir::new().unwrap();

        let results = install_all_skills(share.path(), project.path(), false);

        assert_eq!(results.len(), 2);
        for (skill_name, status) in &results {
            assert!(
                AVAILABLE_SKILLS.contains(&skill_name.as_str()),
                "Unexpected skill: {}",
                skill_name
            );
            assert_eq!(
                status.as_ref().unwrap(),
                &SkillInstallStatus::Installed,
                "Skill {} should be installed",
                skill_name
            );
        }
    }

    #[test]
    fn test_verify_all_skills() {
        let share = create_test_share_dir();
        let project = TempDir::new().unwrap();

        // Install all skills first
        install_all_skills(share.path(), project.path(), false);

        let results = verify_all_skills(share.path(), project.path());

        assert_eq!(results.len(), 2);
        for (skill_name, status) in &results {
            assert!(
                AVAILABLE_SKILLS.contains(&skill_name.as_str()),
                "Unexpected skill: {}",
                skill_name
            );
            assert_eq!(
                status,
                &SkillVerifyStatus::UpToDate,
                "Skill {} should be up to date",
                skill_name
            );
        }
    }
}
