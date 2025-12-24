# rund

**Run CLI apps in detached terminal popup window with configurable size, position, and smart behavior.**

A lightweight, cross-platform terminal launcher that opens CLI applications in separate terminal with automatic positioning, smart pause detection, and per-app configuration. Perfect for creating temporary workspaces, viewing files, or running quick scripts without cluttering your main terminal.

## Features

- 🚀 **Detached Terminal Windows** - Launch apps in separate terminal windows that don't block your workflow
- 📐 **Configurable Geometry** - Set custom window size and position (per-app or globally)
- 🎯 **Smart Pause Detection** - Automatically determines when to pause based on app type and file size
- 💾 **Automatic Backups** - Creates backups when files are modified
- 📋 **Clipboard Integration** - Edit clipboard content directly
- 🎨 **Per-App Configuration** - Different geometry settings for different applications
- 🪟 **Windows Terminal Support** - Full support for both cmd.exe and Windows Terminal (wt)
- 🐧 **Cross-Platform** - Works on Windows, macOS, and Linux
- 🔧 **Highly Customizable** - Configure app classifications, pause behavior, and more

## Installation

### From Source

```bash
git clone https://github.com/cumulus13/rund
cd rund
cargo build --release
```

The binary will be at `target/release/rund` (or `rund.exe` on Windows).

### Add to PATH

**Windows:**
```powershell
# Add the directory containing rund.exe to your PATH
$env:Path += ";C:\path\to\rund"
```

**Linux/macOS:**
```bash
# Copy to a directory in PATH
sudo cp target/release/rund /usr/local/bin/
```

### From Crates.io
```bash
cargo install rund
```

you may need to add Cargo's bin directory to your PATH:
```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

## Quick Start

```bash
# View a file with bat
rund bat file.txt

# Edit a file with nvim
rund nvim document.md

# Edit clipboard content
rund -c -o temp.txt nvim

# Run a Python script
rund "python -m http.server 8000"

# Use specific output file
rund -o C:\temp\notes.txt nvim
```

## Usage

```
rund [OPTIONS] [APP] [ARGS...]
```

### Options

- `-c, --clipboard` - Read clipboard content to file before launching
- `-o, --output FILE` - Specify output file path
- `-b, --backup DIR` - Override backup directory
- `-t, --top` - Always-on-top window (macOS/Linux only)
- `--config` - Show config file location
- `-h, --help` - Show help message

### Examples

```bash
# View file with bat (auto-size detection)
rund bat README.md

# Edit file with specific geometry (if configured)
rund nvim config.toml

# Edit clipboard and save to specific path
rund -c -o C:\temp\script.py bat

# Run command with arguments (relative paths work!)
rund bat ..\README.md

# Python REPL with auto-pause
rund python

# Node script with output
rund "node script.js --verbose"
```

## Configuration

Configuration file is automatically created at:
- **Windows**: `%APPDATA%\rund\config.toml` or `<exe_dir>\config.toml`
- **Linux**: `~/.config/rund/config.toml`
- **macOS**: `~/Library/Application Support/rund/config.toml`

### Basic Configuration

```toml
[terminal]
# Default window geometry
width = 800
height = 600
x = 100
y = 100
auto_position = false

# Windows only: Terminal type
terminal = "cmd"  # or "wt" for Windows Terminal

# Pause behavior: "never", "always", or "auto"
pause_behavior = "auto"

# Backup directory
backup_dir = "backups"

# Default app (optional)
# default_app = "nvim"
```

### App Classifications

Customize which apps are editors, viewers, or always need pause:

```toml
# Editors: NEVER pause (they're interactive)
editor_apps = "vim, nvim, nano, emacs, micro, helix, hx, code, subl"

# Viewers: Pause ONLY for small files (<30 lines)
viewer_apps = "bat, less, more, cat, type"

# Always pause: For scripts/interpreters that produce output
always_pause_apps = "python, python3, node, ruby, perl, php"
```

### Per-App Geometry

Configure specific geometry for individual apps:

```toml
[bat]
width = 1200
height = 800
x = 200
y = 150
auto_position = false

[nvim]
width = 1000
height = 700
auto_position = true  # Let system decide position

[python]
width = 900
height = 600
x = 300
y = 200
```

**Note:** Per-app geometry works with:
- ✅ Windows Terminal (wt)
- ✅ Linux terminals (alacritty, kitty, etc.)
- ✅ macOS Terminal.app
- ⚠️ Windows cmd.exe (position via registry, less reliable)

## Smart Pause Behavior

The `auto` pause behavior intelligently determines when to pause:

| App Type | Small File (<30 lines) | Large File (≥30 lines) |
|----------|------------------------|------------------------|
| **Editors** (vim, nvim, nano) | No pause | No pause |
| **Viewers** (bat, less, cat) | Pause → close | Interactive pager → close |
| **Always Pause** (python, node) | Pause → close | Pause → close |
| **Unknown** | Pause → close | Pause → close |

**No manual terminal closing required!** All windows auto-close after use.

## Platform-Specific Features

### Windows

- **cmd.exe**: Uses registry for position control
- **Windows Terminal (wt)**: Full position and size control
  ```toml
  terminal = "wt"  # Enable Windows Terminal
  ```
- **Auto-position**: When `auto_position = true`, omits position parameters
- **Type command**: Automatically pipes large files through `more`

### Linux

Supports multiple terminal emulators (auto-detected):
- alacritty (full geometry control)
- kitty (full geometry control)
- gnome-terminal
- konsole
- xterm

### macOS

- Uses AppleScript to control Terminal.app
- Full geometry control with `bounds`

## Advanced Features

### Clipboard Editing

Edit your clipboard content in your favorite editor:

```bash
# Edit clipboard in nvim
rund -c nvim

# Edit clipboard in bat (view-only)
rund -c bat

# Save to specific file
rund -c -o C:\temp\clipboard.txt bat
```

### Automatic Backups

When editing files with `-o` or `-c` flags, rund automatically:
1. Calculates initial file hash
2. Monitors the process
3. Creates timestamped backup if file changed

Backups are saved to `./backups/` (or configured directory):
```
backups/
  ├── file_1703001234.txt
  ├── file_1703005678.txt
  └── ...
```

### Relative Path Support

Relative paths are automatically converted to absolute paths:

```bash
# These work from any directory!
rund bat ..\README.md
rund nvim ..\..\config.toml
rund less ./docs/guide.md
```

### Custom App Detection

Add your own apps to classifications:

```toml
# Add custom editor
editor_apps = "vim, nvim, nano, kak, joe, micro"

# Add custom viewer
viewer_apps = "bat, less, glow, mdcat"

# Add custom script interpreter
always_pause_apps = "python, python3, node, deno, bun"
```

## Use Cases

### Quick File Viewing
```bash
# View with syntax highlighting
rund bat document.md

# View log files
rund less app.log
```

### Temporary Editing
```bash
# Edit config quickly
rund nvim config.yaml

# Edit from clipboard
rund -c -o temp.py nvim
```

### Script Running
```bash
# Run Python script with visible output
rund "python script.py --verbose"

# Start local server
rund "python -m http.server 8000"

# Run Node script
rund "node build.js"
```

### Code Review
```bash
# Different geometry for different tools
rund bat large_file.rs    # Wide window (if configured)
rund nvim small_fix.rs     # Standard editor window
```

## Troubleshooting

### Windows Terminal Issues

If you get errors with Windows Terminal:
```toml
# Try cmd.exe instead
terminal = "cmd"
```

### Relative Paths Not Working

Make sure you're using the latest version - older versions didn't support relative path conversion.

### Terminal Not Closing

Check your `pause_behavior` setting:
```toml
# For apps like bat/less, use:
pause_behavior = "auto"  # or "never"

# NOT:
pause_behavior = "always"  # This requires manual close
```

### File Not Found with Type Command

For large files, use a proper pager:
```bash
# Instead of: rund type largefile.txt
rund bat largefile.txt  # Better paging support
```

## Building from Source

### Prerequisites

- Rust 1.70 or later
- Cargo

### Dependencies

- `arboard` - Clipboard support
- `sha2` - File hashing for backups

### Compile

```bash
cargo build --release
```

For smaller binary size:
```bash
cargo build --release
strip target/release/rund  # Linux/macOS
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

Areas for improvement:
- Additional terminal emulator support
- More smart detection heuristics
- GUI configuration editor
- Plugin system for custom behaviors

## License

MIT License - see [LICENSE](LICENSE) file for details

## 💻 Author

[**Hadi Cahyadi**](mailto:cumulus13@gmail.com)

- GitHub: [@cumulus13](https://github.com/cumulus13)
- Email: cumulus13@gmail.com

[![Buy Me a Coffee](https://www.buymeacoffee.com/assets/img/custom_images/orange_img.png)](https://www.buymeacoffee.com/cumulus13)

[![Donate via Ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/cumulus13)
 
[Support me on Patreon](https://www.patreon.com/cumulus13)

## Links

- **Repository**: https://github.com/cumulus13/rund
- **Issues**: https://github.com/cumulus13/rund/issues
- **Documentation**: https://docs.rs/rund

**Star ⭐ this repo if you find it useful!**