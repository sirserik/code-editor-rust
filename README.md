<div align="center">

# Code Editor

**A fast, native macOS code editor — built from scratch in Rust.**

No Electron. No web views. No JavaScript. Pure native GPU‑accelerated rendering.

[![macOS](https://img.shields.io/badge/macOS-12.0+-000000?style=flat&logo=apple)](#installation)
[![Rust](https://img.shields.io/badge/rust-1.70+-CE422B?style=flat&logo=rust)](#building-from-source)
[![License](https://img.shields.io/badge/license-MIT-2BBC8A?style=flat)](#license)
![Lines](https://img.shields.io/badge/Rust-~15k_lines-orange?style=flat)

</div>

---

## Why

Native code editors on macOS are dominated by Electron (VS Code, Cursor) or platform‑specific behemoths (JetBrains, Xcode). This is a third option: **the JetBrains UI you know, the performance of a native Mac app, in a 7 MB binary**.

- **0.7 ms average frame time** on an Intel MacBook (~1400 FPS budget)
- **Sub‑pixel smooth scrolling** with momentum, identical feel to Zed
- **No telemetry. No accounts. No cloud.** Your code, your machine.

---

## Features

### Editing
- Syntax highlighting for **25+ languages** — Rust, TypeScript, JavaScript, Python, Go, PHP, Java, Swift, C/C++, HTML, CSS, SQL, Markdown, …
- Smart auto‑indent, bracket matching, rainbow brackets
- Multi‑cursor, multi‑selection (⌘D — select next occurrence)
- Code folding with `{…}` placeholders
- Inline syntax error detection with wavy underlines
- Indent guides, current line highlight, line numbers
- Auto‑detected indent style (tabs vs spaces, 2/4/8)
- Per‑file shebang language detection

### Navigation
- **Activity bar** in JetBrains New UI style — `P` Project · `G` Git · `F` Find
- **Quick Open** (⌘P) — fuzzy file search
- **Find / Replace** (⌘F) — in current file
- **Find / Replace in Project** (⌘⇧F) — across the whole tree, Unicode‑aware (works with Cyrillic, Kazakh, anything)
- **Go to Line** (⌘G)
- **Command Palette** (⌘⇧P)
- **Go to Top / Bottom of file** (⌘↑ / ⌘↓)
- **Word‑wise navigation** (⌥← / ⌥→)
- **Folder tree** with drag‑and‑drop reorder, context‑menu file ops, git status indicators

### Git
- **Branch switcher popup** — list local + remote branches, search, click to checkout
- **One‑click stage / unstage** per file with checkboxes
- **Inline commit message** + `Commit` and `Commit & Push`
- **Pull / Push / Fetch** buttons — shells out to system `git` so SSH / 2FA / credential helper all just work
- **Show diff in gutter** — added (green) / modified (blue) markers
- **Per‑line blame** in the editor margin

### Themes (9 built‑in)
Darcula (default) · IntelliJ Light · Dracula · One Dark · Gruvbox Dark · Nord · Catppuccin Mocha · Solarized Dark · Monokai Pro

Each tuned to JetBrains‑grade color quality. Switch instantly via the bottom‑right status bar.

### Other
- Integrated PTY terminal (⌘`)
- Persistent settings, font size, window state
- Native `.app` bundle with custom icon
- CLI launcher — `code-editor .` from any terminal
- Open With support — right‑click any source file in Finder

---

## Installation

### Easy (DMG)

```bash
./build-dmg.sh
open dist/CodeEditor-0.1.0.dmg
```

Drag the app onto the Applications symlink. Done.

### From source

```bash
git clone https://github.com/sirserik/code-editor-rust.git
cd code-editor-rust
chmod +x install.sh
./install.sh
```

Installs to `/Applications/Code Editor.app` and registers the `code-editor` CLI command.

### Manual

```bash
cargo build --release
./target/release/code-editor-rust [path]
```

---

## Usage

```bash
code-editor .              # current folder
code-editor ~/projects/x   # specific project
```

Right‑click any source file in Finder → **Open With** → **Code Editor**.

### Keyboard shortcuts

| Shortcut | Action |
|----------|--------|
| `⌘O` | Open folder |
| `⌘⇧W` | Close project |
| `⌘W` | Close tab |
| `⌘S` | Save |
| `⌘T` / `⌘P` | Quick Open (fuzzy file search) |
| `⌘F` | Find in file |
| `⌘⇧F` | Find / Replace in project |
| `⌘G` | Go to line |
| `⌘⇧P` | Command palette |
| `⌘Z` / `⌘⇧Z` | Undo / Redo |
| `⌘D` | Select next occurrence |
| `⌘K` | Delete line |
| `⌘/` | Toggle comment |
| `⌘B` | Toggle sidebar |
| `` ⌘` `` | Toggle terminal |
| `⌘\` | Split editor |
| `⌘↑` / `⌘↓` | Top / Bottom of file |
| `⌥↑` / `⌥↓` | Move line up / down |
| `⌥←` / `⌥→` | Word‑wise navigation |
| `⌘+` / `⌘-` / `⌘0` | Zoom in / out / reset (editor font only) |

---

## Architecture

```
src/
├── main.rs              Entry point, eframe setup, runtime icon embed
├── app.rs               App state, tab management, file/git operations
├── editor/
│   ├── mod.rs           Cursor, selection, undo/redo, fold ranges, multi-cursor
│   ├── buffer.rs        Rope-based text buffer (via ropey)
│   └── cursor.rs        Cursor types
├── gui/
│   ├── mod.rs           App shell, panel orchestration, perf HUD
│   ├── editor_view.rs   Main editor: scroll, paint, highlight, cursor, minimap
│   ├── sidebar.rs       Activity bar, Project tree, Git panel, Find panel
│   ├── menubar.rs       Menu bar, tab strip, status bar
│   ├── overlays.rs      Command palette, dialogs, popups
│   ├── terminal_view.rs PTY terminal renderer
│   └── keys.rs          Global keyboard shortcuts
├── syntax.rs            Tokenizer + highlight rules, shebang detection
├── file_tree.rs         File tree data structure, async scanner, fuzzy search
├── search.rs            Project-wide find/replace, Unicode-aware
├── git.rs               libgit2 + CLI shell-out (pull/push/fetch/branches)
├── settings.rs          Theme palettes, persistence, recent projects
├── terminal.rs          PTY backend, ANSI parser
├── snippets.rs          Per-language snippet library
└── templates.rs         File templates by extension
```

**Key crates:** `eframe` / `egui` 0.31 · `wgpu` (Metal on macOS) · `ropey` · `git2` · `portable-pty` · `regex` · `walkdir` · `notify` · `rfd` · `arboard`

---

## Performance

The editor renders at sub‑millisecond frame times because every per‑frame allocation, subprocess, and re‑layout has been audited. Notable wins on the journey from a 13 FPS first draft to 60+ FPS:

| Issue | Fix | Impact |
|---|---|---|
| `defaults read AppleInterfaceStyle` called ~12×/frame to resolve theme | Cache result with 5‑second TTL | ~80× speedup |
| `ctx.set_zoom_factor` called every frame invalidating glyph atlas | Apply only on change | Stable font cache |
| `Vec<FlatEntry>` of the file tree cloned every frame for the sidebar render | `ScrollArea::show_rows` + deferred mutations | No more per‑frame string allocations |
| `compute_fold_ranges` re‑scanned the whole file every frame for files without multi‑line brackets | Sticky `fold_ranges_computed` flag | One scan per file lifetime |
| `buffer.text()` cloning the entire buffer every frame for shebang detection | Read only the first line | O(file) → O(line) |
| Integer‑line scroll rounding caused micro‑stutter on each trackpad tick | Float `scroll_offset` + sub‑pixel offset in paint loop | Smooth Zed‑style motion |
| Auto‑save did synchronous `fs::write` on the render thread | Spawn background thread, fire‑and‑forget | No frame stalls |

Built‑in **frame‑time HUD** in the bottom status bar shows `avg` / `max` ms over the last 60 frames so regressions are visible at a glance.

---

## Building from source

Requires Rust 1.70+ and Xcode Command Line Tools.

```bash
cargo build --release
cargo test --release   # 146 tests, including Cyrillic/Kazakh Unicode coverage
```

To regenerate the app icon:

```bash
pip install pillow         # one‑time
bash resources/create_icon.sh
```

To build a distributable DMG:

```bash
./build-dmg.sh             # → dist/CodeEditor-<version>.dmg
```

The DMG is unsigned — first‑time users will see Gatekeeper. For public distribution, sign with a Developer ID:

```bash
codesign --deep --force --options runtime \
    --sign 'Developer ID Application: Your Name (TEAMID)' \
    'Code Editor.app'
```

---

## Supported languages

Rust, JavaScript, TypeScript, JSX, TSX, Python, Go, PHP, Ruby, Java, Kotlin, Swift, C, C++, C#, Objective‑C, Zig, Dart, HTML, CSS, SCSS, Vue, Svelte, SQL, Shell, JSON, YAML, TOML, XML, Markdown, Dockerfile, Makefile.

---

## License

MIT
