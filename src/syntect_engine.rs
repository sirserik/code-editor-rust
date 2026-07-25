//! Whole-document syntax highlighting via syntect (Sublime Text grammars).
//!
//! Syntect is stateful (multi-line strings, etc.) so we parse a buffer in one pass and
//! return per-line span vectors. The result drops straight into `Editor.highlight_cache`.
//!
//! Colours come from bundled .tmThemes, picked by the active UI theme via
//! `Theme::syntax_theme()`: JetBrains "Darcula" and "IntelliJ Light" for the two
//! IntelliJ themes, and "Ferrite" for the app's own palettes. All ship inside the
//! binary, are parsed once and cached behind `OnceLock`s, so switching themes
//! never reloads from disk.

use std::io::Cursor;
use std::sync::OnceLock;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme as SynTheme, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;
use egui::Color32;

use crate::settings::SyntaxTheme;
use crate::syntax::{HighlightKind, HighlightSpan};

static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static THEME_SET: OnceLock<ThemeSet> = OnceLock::new();
static FERRITE_THEME: OnceLock<Option<SynTheme>> = OnceLock::new();
static FERRITE_LIGHT_THEME: OnceLock<Option<SynTheme>> = OnceLock::new();
static DARCULA_THEME: OnceLock<Option<SynTheme>> = OnceLock::new();
static INTELLIJ_LIGHT_THEME: OnceLock<Option<SynTheme>> = OnceLock::new();

fn syntax_set() -> &'static SyntaxSet {
    SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

/// Force the lazy grammar/theme sets to load now (call from a background thread
/// at startup). Without this the first file open pays the load on the UI thread.
pub fn prewarm() {
    let _ = syntax_set();
    let _ = theme_set();
    let _ = ferrite_theme();
    let _ = ferrite_light_theme();
    let _ = darcula_theme();
    let _ = intellij_light_theme();
}

fn theme_set() -> &'static ThemeSet {
    THEME_SET.get_or_init(ThemeSet::load_defaults)
}

/// Parse one of the tmThemes compiled into the binary.
fn bundled(cell: &'static OnceLock<Option<SynTheme>>, bytes: &'static [u8]) -> Option<&'static SynTheme> {
    cell.get_or_init(|| ThemeSet::load_from_reader(&mut Cursor::new(bytes)).ok())
        .as_ref()
}

/// Custom tmTheme bundled in `assets/Ferrite.tmTheme`. Falls back to a bundled
/// theme if parsing fails (won't happen — the file ships with the binary).
fn ferrite_theme() -> Option<&'static SynTheme> {
    bundled(&FERRITE_THEME, include_bytes!("../assets/Ferrite.tmTheme"))
}

/// Custom vibrant light tmTheme bundled in `assets/FerriteLight.tmTheme`.
/// Saturated, high-contrast colors on white — replaces the washed-out
/// "InspiredGitHub" default. Falls back to bundled light themes if it fails.
fn ferrite_light_theme() -> Option<&'static SynTheme> {
    bundled(&FERRITE_LIGHT_THEME, include_bytes!("../assets/FerriteLight.tmTheme"))
}

/// JetBrains IntelliJ IDEA "Darcula" editor scheme.
fn darcula_theme() -> Option<&'static SynTheme> {
    bundled(&DARCULA_THEME, include_bytes!("../assets/Darcula.tmTheme"))
}

/// JetBrains IntelliJ IDEA Light editor scheme.
fn intellij_light_theme() -> Option<&'static SynTheme> {
    bundled(&INTELLIJ_LIGHT_THEME, include_bytes!("../assets/IntelliJLight.tmTheme"))
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

/// Pick the editor colour scheme for a UI theme, falling back to a bundled
/// syntect theme of the same polarity if a shipped tmTheme ever fails to parse.
fn theme_for(syntax_theme: SyntaxTheme) -> Option<&'static SynTheme> {
    let ts = theme_set();
    match syntax_theme {
        SyntaxTheme::Darcula => darcula_theme().or_else(|| {
            ts.themes.get("base16-mocha.dark").or_else(|| ts.themes.get("base16-ocean.dark"))
        }),
        SyntaxTheme::IntelliJLight => intellij_light_theme().or_else(|| {
            ts.themes.get("InspiredGitHub").or_else(|| ts.themes.get("base16-ocean.light"))
        }),
        SyntaxTheme::Ferrite => ferrite_theme().or_else(|| {
            ts.themes.get("base16-mocha.dark").or_else(|| ts.themes.get("base16-ocean.dark"))
        }),
    }
}

/// Highlight the whole buffer for `lang` (our internal id). On success returns one
/// `Vec<HighlightSpan>` per line. Returns `None` when syntect can't handle the language
/// or the syntax set/theme is missing — the editor falls back to the keyword highlighter.
pub fn highlight_buffer(content: &str, lang: &str, syntax_theme: SyntaxTheme) -> Option<Vec<Vec<HighlightSpan>>> {
    let ext = lang_to_ext(lang)?;
    let ss = syntax_set();
    let syntax = ss.find_syntax_by_extension(ext)?;
    let theme = theme_for(syntax_theme)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    use egui::Color32;

    #[test]
    fn bundled_themes_parse() {
        assert!(ferrite_theme().is_some(), "Ferrite.tmTheme failed to parse");
        assert!(ferrite_light_theme().is_some(), "FerriteLight.tmTheme failed to parse");
        assert!(darcula_theme().is_some(), "Darcula.tmTheme failed to parse");
        assert!(intellij_light_theme().is_some(), "IntelliJLight.tmTheme failed to parse");
    }

    #[test]
    fn light_sql_highlight_uses_bundled_light_theme() {
        // The IntelliJ light scheme must be selected for the light branch.
        let chosen = theme_for(SyntaxTheme::IntelliJLight).expect("a light theme");
        assert_eq!(chosen.name.as_deref(), Some("IntelliJ Light"));

        // And a real SQL buffer highlights with non-empty colored spans.
        let spans = highlight_buffer("CREATE TABLE t (id SERIAL);", "sql", SyntaxTheme::IntelliJLight)
            .expect("sql highlight");
        assert!(spans[0].iter().any(|s| s.override_color.is_some()));
    }

    /// Colour of the first span covering byte `at` on line 0.
    fn color_at(src: &str, lang: &str, theme: SyntaxTheme, at: usize) -> Color32 {
        let spans = highlight_buffer(src, lang, theme).expect("highlight");
        spans[0]
            .iter()
            .find(|s| s.start <= at && at < s.end)
            .and_then(|s| s.override_color)
            .unwrap_or_else(|| panic!("no coloured span at byte {} of {:?}", at, src))
    }

    #[test]
    fn darcula_uses_jetbrains_colors() {
        // `fn` is a keyword → Darcula orange #CC7832.
        assert_eq!(
            color_at("fn main() {}", "rust", SyntaxTheme::Darcula, 0),
            Color32::from_rgb(0xcc, 0x78, 0x32),
        );
        // String literal → Darcula green #6A8759.
        assert_eq!(
            color_at("let s = \"hi\";", "rust", SyntaxTheme::Darcula, 9),
            Color32::from_rgb(0x6a, 0x87, 0x59),
        );
        // Number → Darcula blue #6897BB.
        assert_eq!(
            color_at("let n = 42;", "rust", SyntaxTheme::Darcula, 8),
            Color32::from_rgb(0x68, 0x97, 0xbb),
        );
    }

    #[test]
    fn intellij_light_uses_jetbrains_colors() {
        // `fn` → IntelliJ Light dark blue #0033B3.
        assert_eq!(
            color_at("fn main() {}", "rust", SyntaxTheme::IntelliJLight, 0),
            Color32::from_rgb(0x00, 0x33, 0xb3),
        );
        // String literal → IntelliJ green #067D17.
        assert_eq!(
            color_at("let s = \"hi\";", "rust", SyntaxTheme::IntelliJLight, 9),
            Color32::from_rgb(0x06, 0x7d, 0x17),
        );
        // Number → IntelliJ blue #1750EB.
        assert_eq!(
            color_at("let n = 42;", "rust", SyntaxTheme::IntelliJLight, 8),
            Color32::from_rgb(0x17, 0x50, 0xeb),
        );
    }

    /// Colour of the span covering byte `at` on line 0, or a panic naming the input.
    fn hex_at(src: &str, lang: &str, theme: SyntaxTheme, at: usize) -> String {
        let c = color_at(src, lang, theme, at);
        format!("#{:02X}{:02X}{:02X}", c.r(), c.g(), c.b())
    }

    #[test]
    fn darcula_matches_intellij_semantic_colors() {
        use SyntaxTheme::Darcula as D;
        // Attributes are yellow-green throughout, including their arguments —
        // `meta.annotation` must beat the generic punctuation/identifier rules.
        assert_eq!(hex_at("#[derive(Debug)]", "rust", D, 0), "#BBB529", "# of attribute");
        assert_eq!(hex_at("#[derive(Debug)]", "rust", D, 2), "#BBB529", "attribute name");
        assert_eq!(hex_at("#[derive(Debug)]", "rust", D, 9), "#BBB529", "attribute argument");
        // Struct fields are purple; function names yellow — the two signatures
        // that make Darcula recognisable.
        assert_eq!(hex_at("struct S { field: u8 }", "rust", D, 11), "#9876AA", "field");
        assert_eq!(hex_at("fn go() {}", "rust", D, 3), "#FFC66D", "function name");
        // Doc comments are green, ordinary comments grey.
        assert_eq!(hex_at("/// docs", "rust", D, 4), "#629755", "doc comment");
        assert_eq!(hex_at("// note", "rust", D, 3), "#808080", "line comment");
        // Type names stay at the default foreground — authentic Darcula.
        assert_eq!(hex_at("struct S { field: u8 }", "rust", D, 7), "#A9B7C6", "type name");
    }

    #[test]
    fn intellij_light_matches_intellij_semantic_colors() {
        use SyntaxTheme::IntelliJLight as L;
        assert_eq!(hex_at("#[derive(Debug)]", "rust", L, 2), "#9E880D", "attribute name");
        assert_eq!(hex_at("struct S { field: u8 }", "rust", L, 11), "#871094", "field");
        assert_eq!(hex_at("fn go() {}", "rust", L, 3), "#00627A", "function name");
        assert_eq!(hex_at("// note", "rust", L, 3), "#8C8C8C", "line comment");
    }

    #[test]
    fn each_ui_theme_maps_to_a_loadable_scheme() {
        for t in crate::settings::Theme::ALL {
            assert!(theme_for(t.syntax_theme()).is_some(), "no scheme for {}", t.name());
        }
    }
}

