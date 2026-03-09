# Code Editor

A fast, lightweight native code editor for macOS built from scratch with **Rust** and **egui**. Designed with a JetBrains-inspired UI that feels premium and familiar.

![macOS](https://img.shields.io/badge/platform-macOS_12+-blue)
![Rust](https://img.shields.io/badge/language-Rust-orange)
![License](https://img.shields.io/badge/license-MIT-green)

## Features

### Editor
- **Syntax highlighting** for 20+ languages — Rust, TypeScript, JavaScript, Python, Go, PHP, Java, Swift, C/C++, HTML, CSS, SQL, and more
- **Syntax error detection** — bracket matching and unclosed string detection with wavy underlines
- **Multi-tab editing** with drag-and-drop reorder
- **Find & Replace** with regex support (`⌘F`)
- **Font zoom** — `⌘+` / `⌘-` / `⌘0` and `⌘+scroll`
- **Indent guides** — subtle vertical guides for code structure
- **Line numbers** with current line highlight
- **Auto-indent** and smart bracket handling

### File Management
- **File tree sidebar** with recursive directory navigation
- **Quick Open** (`⌘T`) — fuzzy file search across the project
- **Project-wide search** (`⌘⇧F`) — search text across all files with instant results
- **New file / folder creation** from the sidebar context
- **File type indicators** — colored dots for instant file type recognition

### Themes
9 built-in themes with JetBrains-quality color palettes:

| Theme | Style |
|-------|-------|
| **Darcula** | JetBrains Darcula (default) |
| **IntelliJ Light** | JetBrains IntelliJ |
| **Dracula** | Classic Dracula |
| **One Dark** | Atom One Dark |
| **Gruvbox Dark** | Retro warm |
| **Nord** | Arctic blue |
| **Catppuccin Mocha** | Pastel dark |
| **Solarized Dark** | Ethan Schoonover |
| **Monokai Pro** | Monokai-inspired |

### Git Integration
- Git status indicators in the file tree (modified, added, untracked)
- Branch display in the status bar

### Other
- **Integrated terminal** (PTY-based)
- **Persistent settings** — theme, font size, tab size, window state saved automatically
- **Native macOS `.app` bundle** with custom icon
- **CLI launcher** — `code-editor .` from terminal
- **Open With** support — right-click any source file in Finder

## Installation

### Prerequisites

- macOS 12.0+
- [Rust toolchain](https://rustup.rs/) (1.70+)

### Quick Install

```bash
git clone https://github.com/sirserik/code-editor-rust.git
cd code-editor-rust
chmod +x install.sh
./install.sh
```

This will:
1. Build an optimized release binary
2. Create `/Applications/Code Editor.app`
3. Install `code-editor` CLI command to `/usr/local/bin`
4. Register with macOS Launch Services

### Manual Build

```bash
cargo build --release
./target/release/code-editor-rust [path]
```

## Usage

### Launch

```bash
# From Spotlight
# ⌘+Space → "Code Editor"

# From terminal
code-editor .
code-editor ~/projects/my-app

# From Finder
# Right-click any source file → Open With → Code Editor
```

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `⌘S` | Save file |
| `⌘T` | Quick Open (fuzzy file search) |
| `⌘F` | Find / Replace |
| `⌘⇧F` | Search across project |
| `⌘W` | Close tab |
| `⌘+` / `⌘-` | Zoom in / out |
| `⌘0` | Reset zoom |
| `⌘Z` | Undo |
| `⌘⇧Z` | Redo |
| `⌘C` / `⌘V` / `⌘X` | Copy / Paste / Cut |
| `⌘A` | Select all |
| `⌘,` | Settings |
| `` ⌘` `` | Toggle terminal |

## Architecture

```
src/
├── main.rs          # Entry point, eframe setup
├── app.rs           # Application state, file I/O, tab management
├── gui.rs           # UI rendering (sidebar, tabs, editor, popups)
├── editor/
│   ├── mod.rs       # Editor logic, cursor, selection, undo/redo
│   ├── buffer.rs    # Text buffer (rope-based via ropey)
│   └── cursor.rs    # Cursor types
├── syntax.rs        # Syntax highlighting engine + error detection
├── file_tree.rs     # File tree data structure and directory scanning
├── search.rs        # Project-wide text search
├── git.rs           # Git status integration (libgit2)
├── settings.rs      # Theme definitions, settings persistence
├── terminal.rs      # Integrated PTY terminal
└── ui/
    ├── mod.rs
    └── render.rs    # Additional UI rendering helpers
```

**~5,700 lines of Rust** — no Electron, no web views, no JavaScript. Pure native GPU-accelerated rendering via OpenGL (glow).

### Key Dependencies

| Crate | Purpose |
|-------|---------|
| `eframe` / `egui` | Immediate-mode GUI framework |
| `ropey` | Efficient rope-based text buffer |
| `git2` | Git integration (libgit2 bindings) |
| `portable-pty` | Terminal emulation |
| `rfd` | Native file dialogs |
| `arboard` | System clipboard |
| `walkdir` | Recursive directory traversal |
| `regex` | Search and syntax patterns |

## Supported Languages

Syntax highlighting is available for:

Rust, JavaScript, TypeScript, JSX/TSX, Python, Go, PHP, Ruby, Java, Kotlin, Swift, C, C++, HTML, CSS/SCSS, Vue, Svelte, SQL, Shell (Bash/Zsh), JSON, YAML, TOML, XML, Markdown

## License

MIT
