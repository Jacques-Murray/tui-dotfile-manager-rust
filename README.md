# TUI Dotfile Manager

A terminal-based user interface (TUI) application for managing dotfiles using symlinks. Written in Rust with the Ratatui framework.

## Features

- 📁 **Profile-based Management** - Organize different sets of dotfiles for different environments (work, personal, etc.)
- 🔗 **Symlink Creation** - Automatically create symlinks from your dotfile repository to target locations
- 💾 **Automatic Backups** - Backs up existing files before replacing them with symlinks
- 🔄 **Restore from Backups** - Browse, preview, and restore backed-up files directly from the TUI
- 🔍 **Dry Run Mode** - Preview changes before applying them (TUI and CLI)
- 🎨 **Interactive TUI** - Easy-to-use terminal interface for profile selection and sync operations
- ⚙️ **CLI Arguments** - Headless mode for automation, custom config paths, and direct profile selection
- ⚡ **Background Operations** - Non-blocking UI with background sync operations
- 🪟 **Cross-platform** - Supports both Unix-like systems and Windows

## Installation

### Prerequisites

- Rust 1.70+ (2021 edition)
- Cargo

### Building from Source

```bash
git clone https://github.com/Jacques-Murray/tui-dotfile-manager-rust.git
cd tui-dotfile-manager-rust
cargo build --release
```

The binary will be available at `target/release/tui-dotfile-manager`.

### Running

```bash
cargo run --release
```

Or directly execute the built binary:

```bash
./target/release/tui-dotfile-manager
```

## Configuration

The application looks for a `config.toml` file in the current directory. Create one with the following structure:

```toml
[settings]
repo_dir = "dotfiles"              # Directory containing your dotfiles
backup_dir = "~/.dotfile_backups"  # Where to store backups (~ expands to home)

[profiles.personal]
links = [
  { source = ".bashrc", target = "~/.bashrc" },
  { source = ".gitconfig_personal", target = "~/.gitconfig" },
  { source = "nvim/init.vim", target = "~/.config/nvim/init.vim" },
]

[profiles.work]
links = [
  { source = ".bashrc", target = "~/.bashrc" },
  { source = ".gitconfig_work", target = "~/.gitconfig" },
]
```

### Configuration Options

- **`repo_dir`**: Path to your dotfiles repository (relative to config file or absolute)
- **`backup_dir`**: Directory where existing files will be backed up (supports `~` for home directory)
- **`profiles`**: Named collections of symlink operations
  - **`source`**: Path to the file in your repo (relative to `repo_dir`)
  - **`target`**: Where to create the symlink (supports `~` for home directory)

## Usage

### Interactive TUI Mode

By default, running the application without arguments launches the interactive TUI:

```bash
# Launch TUI with default config.toml
tui-dotfile-manager

# Launch TUI with custom config
tui-dotfile-manager --config ~/.config/dotfiles/config.toml
```

### CLI Arguments (Headless Mode)

The application supports command-line arguments for automation and scripting:

```bash
# List available profiles
tui-dotfile-manager --list-profiles
tui-dotfile-manager -l

# Sync a specific profile (headless mode)
tui-dotfile-manager --profile personal
tui-dotfile-manager -p work

# Perform a dry run (preview changes without applying)
tui-dotfile-manager --profile work --dry-run
tui-dotfile-manager -p personal -d

# Use a custom config file
tui-dotfile-manager --config ~/.dotfiles/work.toml --profile work

# Combine options
tui-dotfile-manager -c ~/dotfiles/config.toml -p personal --dry-run
```

**Available Options:**
- `-c, --config <PATH>` - Path to configuration file (default: `config.toml`)
- `-p, --profile <NAME>` - Profile to sync (skips TUI if provided)
- `-d, --dry-run` - Perform a dry run without making changes
- `-l, --list-profiles` - List available profiles and exit
- `-h, --help` - Print help information
- `-V, --version` - Print version information

### TUI Key Bindings

Once the TUI is running:

#### Sync Mode (default)
- **`j` / `↓`** - Select next profile
- **`k` / `↑`** - Select previous profile
- **`s` / `Enter`** - Sync the selected profile (creates symlinks)
- **`d`** - Dry run (preview changes without applying)
- **`r`** - Enter restore mode
- **`q` / `Esc`** - Quit the application

#### Restore Mode
- **`j` / `↓`** - Select next backup
- **`k` / `↑`** - Select previous backup
- **`r` / `Enter`** - Restore the selected backup
- **`d`** - Dry run restore (preview without applying)
- **`Delete`** - Delete the selected backup
- **`b` / `Esc`** - Back to sync mode

### Workflow

#### Interactive (TUI) Workflow
1. **Create your config** - Set up `config.toml` with your profiles
2. **Launch the TUI** - Run the application
3. **Select a profile** - Use arrow keys or `j/k` to navigate
4. **Preview changes** - Press `d` for a dry run
5. **Apply changes** - Press `s` to sync the selected profile
6. **Review logs** - Check the log panel for operation details

#### Restore Workflow (TUI)
1. **Enter restore mode** - Press `r` from the main view
2. **Browse backups** - Use `j/k` to navigate the backup list
3. **Preview backup** - View metadata and content preview in the preview panel
4. **Dry run restore** - Press `d` to preview the restore operation
5. **Restore backup** - Press `r` or `Enter` to restore the selected backup
6. **Delete backups** - Press `Delete` to remove old backups
7. **Return to sync mode** - Press `b` or `Esc`

#### Headless (CLI) Workflow
1. **Create your config** - Set up configuration file
2. **List profiles** - Run `tui-dotfile-manager --list-profiles` to see available profiles
3. **Preview changes** - Run `tui-dotfile-manager -p <profile> --dry-run`
4. **Apply changes** - Run `tui-dotfile-manager -p <profile>`

### Use Cases

**Automation & Scripting:**
```bash
#!/bin/bash
# Auto-sync work profile on login
tui-dotfile-manager -c ~/.dotfiles/config.toml -p work
```

**Multiple Configurations:**
```bash
# Switch between different dotfile repos
tui-dotfile-manager -c ~/.dotfiles/personal.toml -p default
tui-dotfile-manager -c ~/.dotfiles/work.toml -p corporate
```

**CI/CD Integration:**
```bash
# Test dotfile sync in GitHub Actions
tui-dotfile-manager --config ./test-config.toml --profile test --dry-run
```

**Quick Profile Switching (Shell Aliases):**
```bash
# Add to your .bashrc or .zshrc
alias dots-work='tui-dotfile-manager -p work'
alias dots-personal='tui-dotfile-manager -p personal'
alias dots-list='tui-dotfile-manager -l'
```

## How It Works

### Sync Operation
1. **Profile Selection**: Choose which set of dotfiles to sync
2. **Path Resolution**: Expands `~` and resolves relative paths
3. **Backup Creation**: If a file exists at the target location:
   - If it's already a correct symlink, skip it
   - If it's an incorrect symlink, remove it
   - If it's a regular file/directory, back it up with a timestamp
4. **Symlink Creation**: Creates symlinks from your repo to target locations
5. **Logging**: All operations are logged in the TUI

### Restore Operation
1. **Backup Discovery**: Scans the backup directory for backed-up files
2. **Backup Parsing**: Extracts original filename and timestamp from backup filenames
3. **Target Resolution**: Matches backups to their original target locations from the config
4. **Preview**: Shows backup metadata, size, timestamp, and content preview
5. **Restoration**:
   - Removes or backs up the current file at the target location
   - Copies the backup file to the target location
   - Removes the backup file from the backup directory
6. **Logging**: All operations are logged in the TUI

### Backup Format

Backups are timestamped to prevent collisions:
```
.bashrc_20241114_143052.123456
```

## Project Structure

```
src/
├── main.rs           # Application entry point
├── lib.rs            # Library exports
├── core/             # Core business logic
│   ├── mod.rs
│   ├── config.rs     # Configuration parsing
│   ├── error.rs      # Error types
│   └── manager.rs    # Dotfile management logic
└── tui/              # Terminal UI
    ├── mod.rs
    ├── app.rs        # Application state
    ├── event.rs      # Event handling
    └── ui.rs         # UI rendering
```

## Testing

Run the test suite:

```bash
cargo test
```

Run with verbose output:

```bash
cargo test -- --nocapture
```

## Development

### Linting

```bash
cargo clippy --all-targets --all-features
```

### Formatting

```bash
cargo fmt
```

## Dependencies

- **ratatui** - TUI framework
- **crossterm** - Terminal manipulation
- **serde** & **toml** - Configuration parsing
- **anyhow** & **thiserror** - Error handling
- **chrono** - Timestamp generation
- **shellexpand** - Path expansion (`~` support)
- **clap** - Command-line argument parsing

## Safety & Behavior

- **TOCTOU Protection**: Uses metadata checks to avoid race conditions
- **Backup Safety**: High-precision timestamps (microseconds) prevent overwrites
- **Backup Before Restore**: Current files are backed up before restoration to prevent data loss
- **Validation**: Configuration is validated on load
- **Error Handling**: Graceful error handling with detailed messages
- **Memory Management**: Log rotation prevents unbounded memory growth

## Limitations

- No diff preview for file contents
- Backups are stored locally (no remote backup support)

## Future Enhancements

- [x] Restore from backup functionality in TUI ✅ **Completed**
- [ ] Configuration reload without restart
- [ ] Progress indicators for large sync operations
- [ ] Diff preview before syncing
- [ ] Backup compression
- [ ] Remote backup storage

## License

MIT License - See LICENSE file for details

## Author

Jacques Murray

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Troubleshooting

### "Configuration file not found"
Make sure `config.toml` exists in the directory where you run the application.

### "Source file does not exist"
Check that:
- The `repo_dir` path in your config is correct
- The `source` paths in your links are relative to `repo_dir`
- The files actually exist in your dotfiles repository

### Permission Errors
Ensure you have write permissions to:
- Target directories (where symlinks will be created)
- Backup directory

### Windows Symlink Issues
On Windows, creating symlinks may require administrator privileges or Developer Mode to be enabled.
