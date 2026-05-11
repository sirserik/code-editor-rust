//! Whole-document syntax highlighting via syntect (Sublime Text grammars).
//!
//! Syntect is stateful (multi-line strings, etc.) so we parse a buffer in one pass and
//! return per-line span vectors. The result drops straight into `Editor.highlight_cache`.
//!
//! Syntect colors come from a bundled .tmTheme — we pick a dark one ("base16-mocha.dark")
//! for the dark UI themes and "InspiredGitHub" for the light theme. They're picked once
//! and cached behind `OnceLock`s so we don't reload on every render.

use std::sync::OnceLock;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme as SynTheme, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;
use egui::Color32;

use crate::syntax::{HighlightKind, HighlightSpan};

static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static THEME_SET: OnceLock<ThemeSet> = OnceLock::new();

fn syntax_set() -> &'static SyntaxSet {
    SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn theme_set() -> &'static ThemeSet {
    THEME_SET.get_or_init(ThemeSet::load_defaults)
}

/// Map our internal language id (from `syntax::detect_language`) to the file extension
/// syntect's `find_syntax_by_extension` expects. Returns `None` when syntect doesn't ship
/// a grammar for it — the caller should fall back to the keyword-based highlighter.
fn lang_to_ext(lang: &str) -> Option<&'static str> {
    Some(match lang {
        "rust" => "rs",
        "javascript" | "jsx" => "js",
        "typescript" | "tsx" => "ts",
        "python" => "py",
        "go" => "go",
        "swift" => "swift",
        "kotlin" => "kt",
        "java" => "java",
        "c" => "c",
        "cpp" => "cpp",
        "csharp" => "cs",
        "objc" => "m",
        "html" => "html",
        "xml" => "xml",
        "css" => "css",
        "scss" => "scss",
        "vue" => "html",     // close enough until a real vue grammar
        "svelte" => "html",
        "php" => "php",
        "ruby" => "rb",
        "json" => "json",
        "toml" => "toml",
        "yaml" => "yaml",
        "sh" => "sh",
        "powershell" => "ps1",
        "batch" => "bat",
        "lua" => "lua",
        "perl" => "pl",
        "r" => "r",
        "markdown" => "md",
        "latex" => "tex",
        "sql" => "sql",
        "diff" => "diff",
        "dart" => "dart",
        "elixir" => "ex",
        "erlang" => "erl",
        "haskell" => "hs",
        "ocaml" => "ml",
        "scala" => "scala",
        "clojure" => "clj",
        "dockerfile" => "Dockerfile",
        "makefile" => "Makefile",
        _ => return None,
    })
}

fn theme_for(dark: bool) -> Option<&'static SynTheme> {
    let ts = theme_set();
    if dark {
        // Tokyo-Night-ish palette from the bundled set
        ts.themes.get("base16-mocha.dark")
            .or_else(|| ts.themes.get("base16-ocean.dark"))
            .or_else(|| ts.themes.get("Solarized (dark)"))
    } else {
        ts.themes.get("InspiredGitHub")
            .or_else(|| ts.themes.get("base16-ocean.light"))
            .or_else(|| ts.themes.get("Solarized (light)"))
    }
}

/// Highlight the whole buffer for `lang` (our internal id). On success returns one
/// `Vec<HighlightSpan>` per line. Returns `None` when syntect can't handle the language
/// or the syntax set/theme is missing — the editor falls back to the keyword highlighter.
pub fn highlight_buffer(content: &str, lang: &str, dark: bool) -> Option<Vec<Vec<HighlightSpan>>> {
    let ext = lang_to_ext(lang)?;
    let ss = syntax_set();
    let syntax = ss.find_syntax_by_extension(ext)?;
    let theme = theme_for(dark)?;
    let mut h = HighlightLines::new(syntax, theme);

    let mut out: Vec<Vec<HighlightSpan>> = Vec::new();
    let mut buffered_line: Option<Vec<HighlightSpan>> = None;
    for raw_line in LinesWithEndings::from(content) {
        let regions = match h.highlight_line(raw_line, ss) {
            Ok(r) => r,
            Err(_) => return None,
        };
        // raw_line may end in '\n'; strip the trailing newline byte from span ranges so
        // they match how the editor measures line content (no newline included).
        let trim = if raw_line.ends_with('\n') { 1 } else { 0 };
        let line_len = raw_line.len().saturating_sub(trim);
        let mut byte_col = 0;
        let mut spans = Vec::with_capacity(regions.len());
        for (style, text) in regions {
            let span_len = text.len();
            let end = (byte_col + span_len).min(line_len);
            if byte_col < end {
                let c = style.foreground;
                spans.push(HighlightSpan {
                    start: byte_col,
                    end,
                    kind: HighlightKind::Normal,
                    override_color: Some(Color32::from_rgb(c.r, c.g, c.b)),
                });
            }
            byte_col += span_len;
            if byte_col >= line_len { break; }
        }
        out.push(spans);
        let _ = buffered_line.take();
    }
    Some(out)
}
