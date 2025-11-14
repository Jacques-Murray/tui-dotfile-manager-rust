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
    assert!(dry_run_logs.iter().any(|s| s.contains("[DRY RUN]")));
    assert!(dry_run_logs
        .iter()
        .any(|s| s.contains("[BACKUP] Moving existing file")));
    assert!(dry_run_logs
        .iter()
        .any(|s| s.contains("[LINK] Creating symlink")));

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
