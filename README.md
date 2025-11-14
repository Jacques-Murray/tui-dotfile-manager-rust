# TUI Dotfile Manager

A terminal-based user interface (TUI) application for managing dotfiles using symlinks. Written in Rust with the Ratatui framework.

## Features

- 📁 **Profile-based Management** - Organize different sets of dotfiles for different environments (work, personal, etc.)
- 🔗 **Symlink Creation** - Automatically create symlinks from your dotfile repository to target locations
- 💾 **Automatic Backups** - Backs up existing files before replacing them with symlinks
- 🔍 **Dry Run Mode** - Preview changes before applying them
- 🎨 **Interactive TUI** - Easy-to-use terminal interface for profile selection and sync operations
- ⚡ **Async Operations** - Non-blocking UI with background sync operations
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

### Key Bindings

Once the TUI is running:

- **`j` / `↓`** - Select next profile
- **`k` / `↑`** - Select previous profile
- **`s` / `Enter`** - Sync the selected profile (creates symlinks)
- **`d`** - Dry run (preview changes without applying)
- **`q` / `Esc`** - Quit the application

### Workflow

1. **Create your config** - Set up `config.toml` with your profiles
2. **Launch the TUI** - Run the application
3. **Select a profile** - Use arrow keys or `j/k` to navigate
4. **Preview changes** - Press `d` for a dry run
5. **Apply changes** - Press `s` to sync the selected profile
6. **Review logs** - Check the log panel for operation details

## How It Works

1. **Profile Selection**: Choose which set of dotfiles to sync
2. **Path Resolution**: Expands `~` and resolves relative paths
3. **Backup Creation**: If a file exists at the target location:
   - If it's already a correct symlink, skip it
   - If it's an incorrect symlink, remove it
   - If it's a regular file/directory, back it up with a timestamp
4. **Symlink Creation**: Creates symlinks from your repo to target locations
5. **Logging**: All operations are logged in the TUI

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

## Safety & Behavior

- **TOCTOU Protection**: Uses metadata checks to avoid race conditions
- **Backup Safety**: High-precision timestamps (microseconds) prevent overwrites
- **Validation**: Configuration is validated on load
- **Error Handling**: Graceful error handling with detailed messages
- **Memory Management**: Log rotation prevents unbounded memory growth

## Limitations

- Currently looks for `config.toml` in the current directory only
- No built-in rollback mechanism (backups must be restored manually)
- No diff preview for file contents

## Future Enhancements

- [ ] CLI arguments for config path and profile selection
- [ ] Restore from backup functionality in TUI
- [ ] Configuration reload without restart
- [ ] Progress indicators for large sync operations
- [ ] Diff preview before syncing

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
