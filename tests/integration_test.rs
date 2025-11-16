// Author: Jacques Murray
//! Integration test for the core DotfileManager logic.
//! This test runs independently using `cargo test`.

use assert_fs::prelude::*;
use std::fs;
use tui_dotfile_manager::core::DotfileManager;

#[test]
fn test_full_sync_workflow() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Setup a temporary file system
    let temp = assert_fs::TempDir::new()?;
    let config_path = temp.child("config.toml");
    let repo_dir = temp.child("dotfiles");
    let backup_dir = temp.child("backups");
    let home_dir = temp.child("home");

    // 2. Create mock dotfiles
    repo_dir.child(".bashrc").write_str("REPO BASHRC")?;
    repo_dir.child(".vimrc").write_str("REPO VIMRC")?;

    // 3. Create mock existing files in 'home'
    home_dir.child(".bashrc").write_str("OLD BASHRC")?; // This should be backed up
    home_dir.child(".gitconfig").write_str("OLD GITCONFIG")?; // This should be ignored

    // 4. Create the config.toml
    let config_content = format!(
        r#"
[settings]
repo_dir = "{}"
backup_dir = "{}"

[profiles]
[profiles.test]
links = [
    {{ source = ".bashrc", target = "{}/.bashrc" }},
    {{ source = ".vimrc", target = "{}/.vimrc" }}
]
"#,
        repo_dir.path().display(),
        backup_dir.path().display(),
        home_dir.path().display(),
        home_dir.path().display()
    );
    config_path.write_str(&config_content)?;

    // 5. Initialize the manager
    let manager = DotfileManager::new(config_path.path())?;

    // 6. Run a dry run first
    let dry_run_logs = manager.execute_sync("test", true)?;
    assert!(dry_run_logs.iter().any(|s| s.contains("DRY RUN")));
    assert!(dry_run_logs
        .iter()
        .any(|s| s.contains("BACKUP") && s.contains("Moving existing file")));
    assert!(dry_run_logs
        .iter()
        .any(|s| s.contains("LINK") && s.contains("Creating symlink")));

    // Assert no changes were made
    assert_eq!(
        fs::read_to_string(home_dir.child(".bashrc").path())?,
        "OLD BASHRC"
    );
    assert!(!home_dir.child(".vimrc").exists());
    assert!(!backup_dir.exists());

    // 7. Run the actual sync
    let sync_logs = manager.execute_sync("test", false)?;
    assert!(sync_logs.iter().any(|s| s.contains("--- Executing Sync")));
    assert!(backup_dir.exists()); // Backup dir was created

    // 8. Assert changes
    // .bashrc was backed up and symlinked
    assert!(home_dir.child(".bashrc").path().is_symlink());
    assert_eq!(
        fs::read_to_string(home_dir.child(".bashrc").path())?,
        "REPO BASHRC"
    );

    // .vimrc was created
    assert!(home_dir.child(".vimrc").path().is_symlink());
    assert_eq!(
        fs::read_to_string(home_dir.child(".vimrc").path())?,
        "REPO VIMRC"
    );

    // Check that the backup file exists
    let backup_files: Vec<_> = fs::read_dir(backup_dir.path())?.collect();
    assert_eq!(backup_files.len(), 1);
    let backup_entry = backup_files.into_iter().next().unwrap()?;
    assert!(backup_entry
        .file_name()
        .to_str()
        .unwrap()
        .starts_with(".bashrc_"));

    // .gitconfig was untouched
    assert!(!home_dir.child(".gitconfig").path().is_symlink());
    assert_eq!(
        fs::read_to_string(home_dir.child(".gitconfig").path())?,
        "OLD GITCONFIG"
    );

    Ok(())
}

#[test]
fn test_missing_config_file() {
    use std::path::Path;
    let result = DotfileManager::new(Path::new("nonexistent.toml"));
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("Configuration file not found"));
    }
}

#[test]
fn test_invalid_toml() -> Result<(), Box<dyn std::error::Error>> {
    let temp = assert_fs::TempDir::new()?;
    let config_path = temp.child("config.toml");
    config_path.write_str("this is not valid toml {")?;

    let result = DotfileManager::new(config_path.path());
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("Failed to parse configuration"));
    }

    Ok(())
}

#[test]
fn test_profile_not_found() -> Result<(), Box<dyn std::error::Error>> {
    let temp = assert_fs::TempDir::new()?;
    let config_path = temp.child("config.toml");
    let config_content = r#"
[settings]
repo_dir = "dotfiles"
backup_dir = "backups"

[profiles.test]
links = [
    { source = ".bashrc", target = "~/.bashrc" }
]
"#;
    config_path.write_str(config_content)?;

    let manager = DotfileManager::new(config_path.path())?;
    let result = manager.execute_sync("nonexistent", false);

    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("Profile not found"));
    }

    Ok(())
}

#[test]
fn test_missing_source_file() -> Result<(), Box<dyn std::error::Error>> {
    let temp = assert_fs::TempDir::new()?;
    let config_path = temp.child("config.toml");
    let repo_dir = temp.child("dotfiles");
    let home_dir = temp.child("home");

    // Create repo but don't create the source file
    repo_dir.create_dir_all()?;

    let config_content = format!(
        r#"
[settings]
repo_dir = "{}"
backup_dir = "backups"

[profiles.test]
links = [
    {{ source = "missing.txt", target = "{}/.missing.txt" }}
]
"#,
        repo_dir.path().display(),
        home_dir.path().display()
    );
    config_path.write_str(&config_content)?;

    let manager = DotfileManager::new(config_path.path())?;
    let logs = manager.execute_sync("test", false)?;

    // Should warn about missing file but not fail
    assert!(logs
        .iter()
        .any(|s| s.contains("[WARN]") && s.contains("does not exist")));

    Ok(())
}

#[test]
fn test_symlink_already_correct() -> Result<(), Box<dyn std::error::Error>> {
    let temp = assert_fs::TempDir::new()?;
    let config_path = temp.child("config.toml");
    let repo_dir = temp.child("dotfiles");
    let home_dir = temp.child("home");

    // Create source file
    repo_dir.child(".bashrc").write_str("REPO BASHRC")?;

    // Create target directory
    home_dir.create_dir_all()?;

    let config_content = format!(
        r#"
[settings]
repo_dir = "{}"
backup_dir = "backups"

[profiles.test]
links = [
    {{ source = ".bashrc", target = "{}/.bashrc" }}
]
"#,
        repo_dir.path().display(),
        home_dir.path().display()
    );
    config_path.write_str(&config_content)?;

    let manager = DotfileManager::new(config_path.path())?;

    // First sync creates the link
    manager.execute_sync("test", false)?;

    // Second sync should skip (already correct)
    let logs = manager.execute_sync("test", false)?;
    assert!(logs
        .iter()
        .any(|s| s.contains("[SKIP]") && s.contains("already correct")));

    Ok(())
}

#[test]
fn test_get_profiles() -> Result<(), Box<dyn std::error::Error>> {
    let temp = assert_fs::TempDir::new()?;
    let config_path = temp.child("config.toml");
    let config_content = r#"
[settings]
repo_dir = "dotfiles"
backup_dir = "backups"

[profiles.work]
links = [
    { source = ".bashrc", target = "~/.bashrc" }
]

[profiles.personal]
links = [
    { source = ".bashrc", target = "~/.bashrc" }
]

[profiles.test]
links = [
    { source = ".bashrc", target = "~/.bashrc" }
]
"#;
    config_path.write_str(config_content)?;

    let manager = DotfileManager::new(config_path.path())?;
    let profiles = manager.get_profiles();

    // Should be sorted alphabetically
    assert_eq!(profiles, vec!["personal", "test", "work"]);

    Ok(())
}

#[test]
fn test_empty_profile_validation() -> Result<(), Box<dyn std::error::Error>> {
    let temp = assert_fs::TempDir::new()?;
    let config_path = temp.child("config.toml");
    let config_content = r#"
[settings]
repo_dir = "dotfiles"
backup_dir = "backups"

[profiles.empty]
links = []
"#;
    config_path.write_str(config_content)?;

    let result = DotfileManager::new(config_path.path());
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("no links"));
    }

    Ok(())
}

// ============================================================================
// CLI-specific integration tests
// ============================================================================

#[test]
fn test_cli_headless_mode_sync() -> Result<(), Box<dyn std::error::Error>> {
    // Test executing a sync via CLI in headless mode
    let temp = assert_fs::TempDir::new()?;
    let config_path = temp.child("config.toml");
    let repo_dir = temp.child("dotfiles");
    let backup_dir = temp.child("backups");
    let home_dir = temp.child("home");

    // Create dotfiles
    repo_dir.child(".bashrc").write_str("CLI BASHRC")?;

    let config_content = format!(
        r#"
[settings]
repo_dir = "{}"
backup_dir = "{}"

[profiles.cli_test]
links = [
    {{ source = ".bashrc", target = "{}/.bashrc" }}
]
"#,
        repo_dir.path().display(),
        backup_dir.path().display(),
        home_dir.path().display()
    );
    config_path.write_str(&config_content)?;

    let manager = DotfileManager::new(config_path.path())?;

    // Simulate CLI headless mode: execute_sync directly
    let logs = manager.execute_sync("cli_test", false)?;

    // Verify sync completed
    assert!(logs.iter().any(|s| s.contains("Executing Sync")));
    assert!(logs.iter().any(|s| s.contains("Sync Finished")));
    assert!(home_dir.child(".bashrc").path().is_symlink());

    Ok(())
}

#[test]
fn test_cli_headless_mode_dry_run() -> Result<(), Box<dyn std::error::Error>> {
    // Test executing a dry run via CLI in headless mode
    let temp = assert_fs::TempDir::new()?;
    let config_path = temp.child("config.toml");
    let repo_dir = temp.child("dotfiles");
    let backup_dir = temp.child("backups");
    let home_dir = temp.child("home");

    // Create dotfiles
    repo_dir.child(".bashrc").write_str("CLI BASHRC")?;
    home_dir.child(".bashrc").write_str("EXISTING BASHRC")?;

    let config_content = format!(
        r#"
[settings]
repo_dir = "{}"
backup_dir = "{}"

[profiles.dryrun_test]
links = [
    {{ source = ".bashrc", target = "{}/.bashrc" }}
]
"#,
        repo_dir.path().display(),
        backup_dir.path().display(),
        home_dir.path().display()
    );
    config_path.write_str(&config_content)?;

    let manager = DotfileManager::new(config_path.path())?;

    // Simulate CLI headless dry-run mode
    let logs = manager.execute_sync("dryrun_test", true)?;

    // Verify dry run output
    assert!(logs.iter().any(|s| s.contains("DRY RUN")));
    assert!(logs
        .iter()
        .any(|s| s.contains("BACKUP") && s.contains("Moving existing file")));

    // Verify no changes were made
    assert_eq!(
        fs::read_to_string(home_dir.child(".bashrc").path())?,
        "EXISTING BASHRC"
    );
    assert!(!home_dir.child(".bashrc").path().is_symlink());
    assert!(!backup_dir.exists());

    Ok(())
}

#[test]
fn test_cli_list_profiles() -> Result<(), Box<dyn std::error::Error>> {
    // Test listing profiles functionality
    let temp = assert_fs::TempDir::new()?;
    let config_path = temp.child("config.toml");

    let config_content = r#"
[settings]
repo_dir = "dotfiles"
backup_dir = "backups"

[profiles.alpha]
links = [
    { source = ".bashrc", target = "~/.bashrc" }
]

[profiles.beta]
links = [
    { source = ".vimrc", target = "~/.vimrc" }
]

[profiles.gamma]
links = [
    { source = ".zshrc", target = "~/.zshrc" }
]
"#;
    config_path.write_str(config_content)?;

    let manager = DotfileManager::new(config_path.path())?;

    // Simulate CLI --list-profiles functionality
    let profiles = manager.get_profiles();

    // Should be sorted alphabetically
    assert_eq!(profiles, vec!["alpha", "beta", "gamma"]);

    Ok(())
}

#[test]
fn test_cli_custom_config_path() -> Result<(), Box<dyn std::error::Error>> {
    // Test using a custom config path
    let temp = assert_fs::TempDir::new()?;
    let custom_config_dir = temp.child("custom");
    custom_config_dir.create_dir_all()?;
    let config_path = custom_config_dir.child("my-config.toml");
    let repo_dir = temp.child("dotfiles");
    let home_dir = temp.child("home");

    // Create dotfiles
    repo_dir
        .child(".bashrc")
        .write_str("CUSTOM CONFIG BASHRC")?;

    let config_content = format!(
        r#"
[settings]
repo_dir = "{}"
backup_dir = "backups"

[profiles.custom]
links = [
    {{ source = ".bashrc", target = "{}/.bashrc" }}
]
"#,
        repo_dir.path().display(),
        home_dir.path().display()
    );
    config_path.write_str(&config_content)?;

    // Load with custom config path
    let manager = DotfileManager::new(config_path.path())?;

    // Execute sync to verify it works
    let logs = manager.execute_sync("custom", false)?;

    assert!(logs.iter().any(|s| s.contains("Executing Sync")));
    assert!(home_dir.child(".bashrc").path().is_symlink());

    Ok(())
}

#[test]
fn test_cli_invalid_profile_error() -> Result<(), Box<dyn std::error::Error>> {
    // Test proper error handling for invalid profile name
    let temp = assert_fs::TempDir::new()?;
    let config_path = temp.child("config.toml");

    let config_content = r#"
[settings]
repo_dir = "dotfiles"
backup_dir = "backups"

[profiles.valid]
links = [
    { source = ".bashrc", target = "~/.bashrc" }
]
"#;
    config_path.write_str(config_content)?;

    let manager = DotfileManager::new(config_path.path())?;

    // Try to sync with invalid profile
    let result = manager.execute_sync("invalid_profile", false);

    assert!(result.is_err());
    if let Err(e) = result {
        let err_msg = e.to_string();
        assert!(err_msg.contains("Profile not found"));
        assert!(err_msg.contains("invalid_profile"));
    }

    Ok(())
}

// ============================================================================
// Restore functionality integration tests
// ============================================================================

#[test]
fn test_list_backups_empty() -> Result<(), Box<dyn std::error::Error>> {
    // Test listing backups when no backups exist
    let temp = assert_fs::TempDir::new()?;
    let config_path = temp.child("config.toml");
    let repo_dir = temp.child("dotfiles");
    let backup_dir = temp.child("backups");

    repo_dir.child(".bashrc").write_str("REPO BASHRC")?;

    let config_content = format!(
        r#"
[settings]
repo_dir = "{}"
backup_dir = "{}"

[profiles.test]
links = [
    {{ source = ".bashrc", target = "~/.bashrc" }}
]
"#,
        repo_dir.path().display(),
        backup_dir.path().display()
    );
    config_path.write_str(&config_content)?;

    let manager = DotfileManager::new(config_path.path())?;
    let backups = manager.list_backups()?;

    assert_eq!(backups.len(), 0);
    Ok(())
}

#[test]
fn test_restore_workflow() -> Result<(), Box<dyn std::error::Error>> {
    // Test full restore workflow: sync to create backup, list backups, restore
    let temp = assert_fs::TempDir::new()?;
    let config_path = temp.child("config.toml");
    let repo_dir = temp.child("dotfiles");
    let backup_dir = temp.child("backups");
    let home_dir = temp.child("home");

    // Create dotfiles
    repo_dir.child(".bashrc").write_str("REPO BASHRC")?;

    // Create existing file that will be backed up
    home_dir.child(".bashrc").write_str("OLD BASHRC")?;

    let config_content = format!(
        r#"
[settings]
repo_dir = "{}"
backup_dir = "{}"

[profiles.test]
links = [
    {{ source = ".bashrc", target = "{}/.bashrc" }}
]
"#,
        repo_dir.path().display(),
        backup_dir.path().display(),
        home_dir.path().display()
    );
    config_path.write_str(&config_content)?;

    let manager = DotfileManager::new(config_path.path())?;

    // Execute sync to create backup
    manager.execute_sync("test", false)?;

    // Verify symlink was created
    assert!(home_dir.child(".bashrc").path().is_symlink());
    assert_eq!(
        fs::read_to_string(home_dir.child(".bashrc").path())?,
        "REPO BASHRC"
    );

    // List backups
    let backups = manager.list_backups()?;
    assert_eq!(backups.len(), 1);

    let backup = &backups[0];
    assert_eq!(backup.original_name, ".bashrc");
    assert!(backup.backup_path.exists());

    // Read backup content to verify
    let backup_content = fs::read_to_string(&backup.backup_path)?;
    assert_eq!(backup_content, "OLD BASHRC");

    // Restore backup (dry run first)
    let dry_run_logs = manager.restore_backup(backup, true)?;
    assert!(dry_run_logs.iter().any(|s| s.contains("DRY RUN")));

    // Symlink should still exist after dry run
    assert!(home_dir.child(".bashrc").path().is_symlink());

    // Restore backup for real
    let restore_logs = manager.restore_backup(backup, false)?;
    assert!(restore_logs.iter().any(|s| s.contains("Restoring backup")));
    assert!(restore_logs.iter().any(|s| s.contains("RESTORE")));

    // Verify the old content was restored
    assert_eq!(
        fs::read_to_string(home_dir.child(".bashrc").path())?,
        "OLD BASHRC"
    );

    // Verify backup file was removed
    assert!(!backup.backup_path.exists());

    Ok(())
}

#[test]
fn test_delete_backup() -> Result<(), Box<dyn std::error::Error>> {
    // Test deleting a backup file
    let temp = assert_fs::TempDir::new()?;
    let config_path = temp.child("config.toml");
    let repo_dir = temp.child("dotfiles");
    let backup_dir = temp.child("backups");
    let home_dir = temp.child("home");

    // Create dotfiles
    repo_dir.child(".bashrc").write_str("REPO BASHRC")?;

    // Create existing file that will be backed up
    home_dir.child(".bashrc").write_str("OLD BASHRC")?;

    let config_content = format!(
        r#"
[settings]
repo_dir = "{}"
backup_dir = "{}"

[profiles.test]
links = [
    {{ source = ".bashrc", target = "{}/.bashrc" }}
]
"#,
        repo_dir.path().display(),
        backup_dir.path().display(),
        home_dir.path().display()
    );
    config_path.write_str(&config_content)?;

    let manager = DotfileManager::new(config_path.path())?;

    // Execute sync to create backup
    manager.execute_sync("test", false)?;

    // List backups
    let backups = manager.list_backups()?;
    assert_eq!(backups.len(), 1);

    let backup = &backups[0];
    assert!(backup.backup_path.exists());

    // Delete backup
    manager.delete_backup(backup)?;

    // Verify backup was deleted
    assert!(!backup.backup_path.exists());

    // List backups again - should be empty
    let backups = manager.list_backups()?;
    assert_eq!(backups.len(), 0);

    Ok(())
}

#[test]
fn test_restore_backup_before_restore() -> Result<(), Box<dyn std::error::Error>> {
    // Test that restoring creates a backup of the current file
    let temp = assert_fs::TempDir::new()?;
    let config_path = temp.child("config.toml");
    let repo_dir = temp.child("dotfiles");
    let backup_dir = temp.child("backups");
    let home_dir = temp.child("home");

    // Create dotfiles
    repo_dir.child(".bashrc").write_str("REPO BASHRC")?;

    // Create existing file that will be backed up
    home_dir.child(".bashrc").write_str("OLD BASHRC")?;

    let config_content = format!(
        r#"
[settings]
repo_dir = "{}"
backup_dir = "{}"

[profiles.test]
links = [
    {{ source = ".bashrc", target = "{}/.bashrc" }}
]
"#,
        repo_dir.path().display(),
        backup_dir.path().display(),
        home_dir.path().display()
    );
    config_path.write_str(&config_content)?;

    let manager = DotfileManager::new(config_path.path())?;

    // Execute sync to create backup
    manager.execute_sync("test", false)?;

    // Verify symlink was created
    assert!(home_dir.child(".bashrc").path().is_symlink());

    // Now modify the symlink target (simulate user changes)
    fs::remove_file(home_dir.child(".bashrc").path())?;
    home_dir.child(".bashrc").write_str("MODIFIED BASHRC")?;

    // List backups - should have one from the sync
    let backups = manager.list_backups()?;
    assert_eq!(backups.len(), 1);

    let original_backup = &backups[0];

    // Restore the original backup
    manager.restore_backup(original_backup, false)?;

    // Now there should be a new backup of "MODIFIED BASHRC"
    let backups = manager.list_backups()?;
    assert_eq!(backups.len(), 1);

    // The current file should be the old content
    assert_eq!(
        fs::read_to_string(home_dir.child(".bashrc").path())?,
        "OLD BASHRC"
    );

    Ok(())
}

#[test]
fn test_diff_preview() -> Result<(), Box<dyn std::error::Error>> {
    use tui_dotfile_manager::core::diff::DiffResult;

    // Setup a temporary file system
    let temp = assert_fs::TempDir::new()?;
    let config_path = temp.child("config.toml");
    let repo_dir = temp.child("dotfiles");
    let backup_dir = temp.child("backups");
    let home_dir = temp.child("home");

    // Create mock dotfiles
    repo_dir
        .child(".bashrc")
        .write_str("# New bashrc\necho 'Hello from new bashrc'\n")?;
    repo_dir
        .child(".vimrc")
        .write_str("set number\nset tabstop=4\n")?;

    // Create mock existing files in 'home' with different content
    home_dir
        .child(".bashrc")
        .write_str("# Old bashrc\necho 'Hello from old bashrc'\n")?;
    // .vimrc doesn't exist - should show as new file

    // Create the config.toml
    let config_content = format!(
        r#"
[settings]
repo_dir = "{}"
backup_dir = "{}"

[profiles.test]
links = [
    {{ source = ".bashrc", target = "{}/.bashrc" }},
    {{ source = ".vimrc", target = "{}/.vimrc" }}
]
"#,
        repo_dir.path().display(),
        backup_dir.path().display(),
        home_dir.path().display(),
        home_dir.path().display()
    );
    config_path.write_str(&config_content)?;

    // Initialize the manager
    let manager = DotfileManager::new(config_path.path())?;

    // Generate diff preview
    let diffs = manager.preview_diff("test")?;

    // Should have two diffs: one for .bashrc (modified) and one for .vimrc (new)
    assert_eq!(diffs.len(), 2);

    // Check first diff (.bashrc) - should be a FileDiff
    match &diffs[0] {
        DiffResult::FileDiff { diff_lines, .. } => {
            // Should have some diff lines
            assert!(!diff_lines.is_empty());
        }
        _ => panic!("Expected FileDiff for .bashrc"),
    }

    // Check second diff (.vimrc) - should be a NewFile
    match &diffs[1] {
        DiffResult::NewFile {
            content_preview, ..
        } => {
            // Should have content preview
            assert!(!content_preview.is_empty());
            assert!(content_preview.iter().any(|l| l.contains("set number")));
        }
        _ => panic!("Expected NewFile for .vimrc"),
    }

    Ok(())
}

#[test]
fn test_diff_preview_binary_file() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write;
    use tui_dotfile_manager::core::diff::DiffResult;

    // Setup a temporary file system
    let temp = assert_fs::TempDir::new()?;
    let config_path = temp.child("config.toml");
    let repo_dir = temp.child("dotfiles");
    let backup_dir = temp.child("backups");
    let home_dir = temp.child("home");

    // Create directories
    fs::create_dir_all(repo_dir.path())?;
    fs::create_dir_all(home_dir.path())?;

    // Create a binary file in repo
    let mut file = fs::File::create(repo_dir.child("image.bin").path())?;
    file.write_all(&[0u8, 1, 2, 3, 0, 255])?;
    drop(file);

    // Create a different binary file in home
    let mut file = fs::File::create(home_dir.child("image.bin").path())?;
    file.write_all(&[0u8, 5, 6, 7, 0, 255])?;
    drop(file);

    // Create the config.toml
    let config_content = format!(
        r#"
[settings]
repo_dir = "{}"
backup_dir = "{}"

[profiles.test]
links = [
    {{ source = "image.bin", target = "{}/image.bin" }}
]
"#,
        repo_dir.path().display(),
        backup_dir.path().display(),
        home_dir.path().display()
    );
    config_path.write_str(&config_content)?;

    // Initialize the manager
    let manager = DotfileManager::new(config_path.path())?;

    // Generate diff preview
    let diffs = manager.preview_diff("test")?;

    // Should have one diff for the binary file
    assert_eq!(diffs.len(), 1);

    // Check diff - should be BinaryFile
    match &diffs[0] {
        DiffResult::BinaryFile { .. } => {
            // Expected
        }
        _ => panic!("Expected BinaryFile for image.bin"),
    }

    Ok(())
}

