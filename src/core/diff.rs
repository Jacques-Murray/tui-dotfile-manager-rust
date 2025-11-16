// Author: Jacques Murray
//! Diff generation and preview logic for comparing dotfiles.

use similar::{ChangeTag, TextDiff};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Represents the result of comparing a source and target file.
#[derive(Debug, Clone)]
pub enum DiffResult {
    /// No diff needed - already a correct symlink
    NoDiff {
        path: PathBuf,
        reason: String,
    },
    /// Text file diff with unified diff output
    FileDiff {
        source: PathBuf,
        target: PathBuf,
        diff_lines: Vec<DiffLine>,
    },
    /// New file will be created (target doesn't exist)
    NewFile {
        source: PathBuf,
        target: PathBuf,
        content_preview: Vec<String>,
    },
    /// Binary files cannot be diffed
    BinaryFile {
        source: PathBuf,
        target: PathBuf,
    },
    /// Error reading files
    Error {
        source: PathBuf,
        target: PathBuf,
        error: String,
    },
}

/// Represents a single line in a diff with its change type.
#[derive(Debug, Clone, PartialEq)]
pub struct DiffLine {
    pub tag: DiffLineTag,
    pub content: String,
}

/// The type of change for a diff line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DiffLineTag {
    /// Line exists in both files (context)
    Equal,
    /// Line removed from target (only in target)
    Delete,
    /// Line added from source (only in source)
    Insert,
}

impl DiffResult {
    /// Gets the target path for this diff result.
    pub fn target_path(&self) -> &Path {
        match self {
            DiffResult::NoDiff { path, .. } => path,
            DiffResult::FileDiff { target, .. } => target,
            DiffResult::NewFile { target, .. } => target,
            DiffResult::BinaryFile { target, .. } => target,
            DiffResult::Error { target, .. } => target,
        }
    }

    /// Returns a summary description of the diff result.
    pub fn summary(&self) -> String {
        match self {
            DiffResult::NoDiff { reason, .. } => reason.clone(),
            DiffResult::FileDiff { diff_lines, .. } => {
                let additions = diff_lines
                    .iter()
                    .filter(|l| l.tag == DiffLineTag::Insert)
                    .count();
                let deletions = diff_lines
                    .iter()
                    .filter(|l| l.tag == DiffLineTag::Delete)
                    .count();
                format!("+{} -{} lines", additions, deletions)
            }
            DiffResult::NewFile { content_preview, .. } => {
                format!("New file ({} lines)", content_preview.len())
            }
            DiffResult::BinaryFile { .. } => "Binary file".to_string(),
            DiffResult::Error { error, .. } => format!("Error: {}", error),
        }
    }
}

/// Generates a diff between source and target files.
///
/// # Arguments
/// * `source` - Path to the source file in the dotfiles repository
/// * `target` - Path to the target file on the filesystem
///
/// # Returns
/// A DiffResult describing the differences between the files
pub fn generate_diff(source: &Path, target: &Path) -> DiffResult {
    // Check if target exists
    if !target.exists() {
        // Target doesn't exist - this will be a new file
        return match read_text_file(source) {
            Ok(content) => {
                let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
                DiffResult::NewFile {
                    source: source.to_path_buf(),
                    target: target.to_path_buf(),
                    content_preview: lines,
                }
            }
            Err(e) => {
                // Might be a binary file
                if is_binary_file(source) {
                    DiffResult::BinaryFile {
                        source: source.to_path_buf(),
                        target: target.to_path_buf(),
                    }
                } else {
                    DiffResult::Error {
                        source: source.to_path_buf(),
                        target: target.to_path_buf(),
                        error: format!("Failed to read source: {}", e),
                    }
                }
            }
        };
    }

    // Check if target is a symlink pointing to source
    if let Ok(link) = fs::read_link(target) {
        if link == source {
            return DiffResult::NoDiff {
                path: target.to_path_buf(),
                reason: format!("Already linked to {}", source.display()),
            };
        }
    }

    // Read both files
    let source_content = match read_text_file(source) {
        Ok(c) => c,
        Err(e) => {
            // Check if binary
            if is_binary_file(source) {
                return DiffResult::BinaryFile {
                    source: source.to_path_buf(),
                    target: target.to_path_buf(),
                };
            }
            return DiffResult::Error {
                source: source.to_path_buf(),
                target: target.to_path_buf(),
                error: format!("Failed to read source: {}", e),
            };
        }
    };

    let target_content = match read_text_file(target) {
        Ok(c) => c,
        Err(e) => {
            // Check if binary
            if is_binary_file(target) {
                return DiffResult::BinaryFile {
                    source: source.to_path_buf(),
                    target: target.to_path_buf(),
                };
            }
            return DiffResult::Error {
                source: source.to_path_buf(),
                target: target.to_path_buf(),
                error: format!("Failed to read target: {}", e),
            };
        }
    };

    // Generate diff using similar crate
    let diff = TextDiff::from_lines(&target_content, &source_content);

    let mut diff_lines = Vec::new();

    for change in diff.iter_all_changes() {
        let tag = match change.tag() {
            ChangeTag::Equal => DiffLineTag::Equal,
            ChangeTag::Delete => DiffLineTag::Delete,
            ChangeTag::Insert => DiffLineTag::Insert,
        };

        diff_lines.push(DiffLine {
            tag,
            content: change.to_string().trim_end().to_string(),
        });
    }

    DiffResult::FileDiff {
        source: source.to_path_buf(),
        target: target.to_path_buf(),
        diff_lines,
    }
}

/// Attempts to read a file as UTF-8 text.
fn read_text_file(path: &Path) -> io::Result<String> {
    fs::read_to_string(path)
}

/// Checks if a file appears to be binary by looking for null bytes.
fn is_binary_file(path: &Path) -> bool {
    if let Ok(bytes) = fs::read(path) {
        // Check first 8KB for null bytes
        let check_size = bytes.len().min(8192);
        bytes[..check_size].contains(&0)
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_diff_new_file() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source.txt");
        let target = temp.path().join("target.txt");

        fs::write(&source, "line1\nline2\n").unwrap();

        let result = generate_diff(&source, &target);

        match result {
            DiffResult::NewFile {
                content_preview, ..
            } => {
                assert_eq!(content_preview.len(), 2);
                assert_eq!(content_preview[0], "line1");
            }
            _ => panic!("Expected NewFile result"),
        }
    }

    #[test]
    fn test_diff_already_linked() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source.txt");
        let target = temp.path().join("target.txt");

        fs::write(&source, "content").unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(&source, &target).unwrap();

        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&source, &target).unwrap();

        let result = generate_diff(&source, &target);

        match result {
            DiffResult::NoDiff { reason, .. } => {
                assert!(reason.contains("Already linked"));
            }
            _ => panic!("Expected NoDiff result"),
        }
    }

    #[test]
    fn test_diff_text_files() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source.txt");
        let target = temp.path().join("target.txt");

        fs::write(&source, "line1\nline2\nline3\n").unwrap();
        fs::write(&target, "line1\nold_line\nline3\n").unwrap();

        let result = generate_diff(&source, &target);

        match result {
            DiffResult::FileDiff { diff_lines, .. } => {
                let inserts = diff_lines
                    .iter()
                    .filter(|l| l.tag == DiffLineTag::Insert)
                    .count();
                let deletes = diff_lines
                    .iter()
                    .filter(|l| l.tag == DiffLineTag::Delete)
                    .count();

                assert!(inserts > 0);
                assert!(deletes > 0);
            }
            _ => panic!("Expected FileDiff result"),
        }
    }

    #[test]
    fn test_binary_file() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source.bin");
        let target = temp.path().join("target.bin");

        // Write binary data (with null bytes)
        let mut file = fs::File::create(&source).unwrap();
        file.write_all(&[0u8, 1, 2, 3, 0, 255]).unwrap();

        let mut file = fs::File::create(&target).unwrap();
        file.write_all(&[0u8, 5, 6, 7, 0, 255]).unwrap();

        let result = generate_diff(&source, &target);

        match result {
            DiffResult::BinaryFile { .. } => {
                // Expected
            }
            _ => panic!("Expected BinaryFile result"),
        }
    }

    #[test]
    fn test_diff_result_summary() {
        let diff_result = DiffResult::NoDiff {
            path: PathBuf::from("/test"),
            reason: "Already linked".to_string(),
        };
        assert_eq!(diff_result.summary(), "Already linked");

        let diff_result = DiffResult::NewFile {
            source: PathBuf::from("/source"),
            target: PathBuf::from("/target"),
            content_preview: vec!["line1".to_string(), "line2".to_string()],
        };
        assert_eq!(diff_result.summary(), "New file (2 lines)");

        let diff_result = DiffResult::BinaryFile {
            source: PathBuf::from("/source"),
            target: PathBuf::from("/target"),
        };
        assert_eq!(diff_result.summary(), "Binary file");
    }
}
