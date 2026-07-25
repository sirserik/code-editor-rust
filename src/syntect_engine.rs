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
#[cfg(test)]
use syntect::easy::HighlightLines;
use syntect::highlighting::{HighlightIterator, HighlightState, Highlighter, Theme as SynTheme, ThemeSet};
use syntect::parsing::{ParseState, ScopeStack, SyntaxSet};
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

/// Highlight the whole buffer in one pass. Superseded in the editor by
/// `highlight_incremental`, but kept as the straightforward reference the
/// incremental path is checked against.
#[cfg(test)]
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

// ── Incremental re-highlighting ──────────────────────────────────────────────
//
// Syntect is stateful across lines, so the obvious implementation re-parses the
// whole buffer after every edit. With the pure-Rust `fancy-regex` backend that
// costs ~0.15 ms per line — 150 ms for a 1000-line file, on *every keystroke*,
// which is long enough to see freshly typed text sit in the default colour.
//
// So we snapshot syntect's state every `CHECKPOINT_EVERY` lines. An edit
// restarts from the checkpoint just above it, and stops as soon as the state
// matches the snapshot it had before — past that point nothing can have
// changed, and the previously computed spans are still valid. Editing anywhere
// in a normal file then costs a couple of checkpoints' worth of lines instead
// of the whole buffer.

/// Lines between saved parser snapshots. Smaller = faster re-parse after an
/// edit, but more cloned state per pass.
const CHECKPOINT_EVERY: usize = 64;

/// Syntect's state at the *start* of a line.
#[derive(Clone, Debug)]
pub struct Checkpoint {
    pub line: usize,
    parse: ParseState,
    highlight: HighlightState,
}

/// Outcome of an incremental pass. `lines` covers `start_line ..
/// converged_at.unwrap_or(end of file)`; the caller splices it into the cache it
/// already has.
pub struct IncrementalHighlight {
    pub start_line: usize,
    /// Line at which the new state matched the old one, so everything from here
    /// down is unchanged. `None` means the pass ran to the end of the buffer.
    pub converged_at: Option<usize>,
    pub lines: Vec<Vec<HighlightSpan>>,
    pub checkpoints: Vec<Checkpoint>,
}

/// Re-highlight `content` from `from_line` down, reusing `prev` when possible.
///
/// `prev` must have been produced for a buffer with the *same line count* —
/// checkpoints are keyed by line number, so an insertion or deletion shifts
/// them and the caller must fall back to a full pass. Pass an empty slice to
/// force a full re-parse.
pub fn highlight_incremental(
    content: &str,
    lang: &str,
    syntax_theme: SyntaxTheme,
    prev: &[Checkpoint],
    from_line: usize,
) -> Option<IncrementalHighlight> {
    let ext = lang_to_ext(lang)?;
    let ss = syntax_set();
    let syntax = ss.find_syntax_by_extension(ext)?;
    let theme = theme_for(syntax_theme)?;
    let highlighter = Highlighter::new(theme);

    // Resume from the last checkpoint at or before the edit.
    let resume = prev.iter().rev().find(|c| c.line <= from_line);
    let (start_line, mut parse, mut hl) = match resume {
        Some(c) => (c.line, c.parse.clone(), c.highlight.clone()),
        None => (
            0,
            ParseState::new(syntax),
            HighlightState::new(&highlighter, ScopeStack::new()),
        ),
    };

    let mut lines: Vec<Vec<HighlightSpan>> = Vec::new();
    let mut checkpoints: Vec<Checkpoint> = prev.iter().filter(|c| c.line < start_line).cloned().collect();

    for (i, raw_line) in LinesWithEndings::from(content).enumerate() {
        if i < start_line {
            continue;
        }
        if i % CHECKPOINT_EVERY == 0 {
            // Past the edit, has the state caught up with what it was before?
            // If so the rest of the file is untouched.
            if i > from_line {
                if let Some(old) = prev.iter().find(|c| c.line == i) {
                    if old.parse == parse && old.highlight == hl {
                        checkpoints.extend(prev.iter().filter(|c| c.line >= i).cloned());
                        return Some(IncrementalHighlight {
                            start_line,
                            converged_at: Some(i),
                            lines,
                            checkpoints,
                        });
                    }
                }
            }
            checkpoints.push(Checkpoint { line: i, parse: parse.clone(), highlight: hl.clone() });
        }

        let ops = parse.parse_line(raw_line, ss).ok()?;
        let regions = HighlightIterator::new(&mut hl, &ops, raw_line, &highlighter);
        lines.push(spans_for_line(raw_line, regions));
    }

    Some(IncrementalHighlight { start_line, converged_at: None, lines, checkpoints })
}

/// Turn one line's styled regions into our span representation, dropping the
/// trailing newline so ranges match how the editor measures line content.
fn spans_for_line<'a>(
    raw_line: &str,
    regions: impl Iterator<Item = (syntect::highlighting::Style, &'a str)>,
) -> Vec<HighlightSpan> {
    let trim = if raw_line.ends_with('\n') { 1 } else { 0 };
    let line_len = raw_line.len().saturating_sub(trim);
    let mut byte_col = 0;
    let mut spans = Vec::new();
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
        if byte_col >= line_len {
            break;
        }
    }
    spans
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

    // ── Incremental re-highlighting ──

    /// Run an incremental pass and splice it into `base` the way the editor does.
    fn apply(
        base: &mut Vec<Vec<HighlightSpan>>,
        checkpoints: &mut Vec<Checkpoint>,
        content: &str,
        from_line: usize,
    ) {
        let inc = highlight_incremental(content, "rust", SyntaxTheme::Darcula, checkpoints, from_line)
            .expect("incremental pass");
        let end = inc.converged_at.unwrap_or(base.len());
        base.splice(inc.start_line..end, inc.lines);
        *checkpoints = inc.checkpoints;
    }

    fn colors(spans: &[Vec<HighlightSpan>]) -> Vec<Vec<(usize, usize, Option<Color32>)>> {
        spans
            .iter()
            .map(|l| l.iter().map(|s| (s.start, s.end, s.override_color)).collect())
            .collect()
    }

    fn full(content: &str) -> Vec<Vec<HighlightSpan>> {
        highlight_incremental(content, "rust", SyntaxTheme::Darcula, &[], 0)
            .unwrap()
            .lines
    }

    #[test]
    fn incremental_matches_a_full_pass() {
        // A buffer long enough to span several checkpoints, with a multi-line
        // string so the parser really does carry state across lines.
        let mut src = String::new();
        for i in 0..400 {
            src.push_str(&format!("fn f{}() {{ let s = \"text {}\"; }}\n", i, i));
        }
        let mut cache = full(&src);
        let mut cps = highlight_incremental(&src, "rust", SyntaxTheme::Darcula, &[], 0)
            .unwrap()
            .checkpoints;

        // Edit a line in the middle, keeping the line count identical.
        let edited = src.replace("fn f200() {", "fn g200() {");
        apply(&mut cache, &mut cps, &edited, 200);

        assert_eq!(colors(&cache), colors(&full(&edited)), "incremental drifted from a full pass");
    }

    #[test]
    fn incremental_handles_an_edit_that_opens_a_block_comment() {
        // The state change has to propagate: everything below turns into a
        // comment, so convergence must NOT fire early.
        let mut src = String::new();
        for i in 0..300 {
            src.push_str(&format!("let v{} = {};\n", i, i));
        }
        let mut cache = full(&src);
        let mut cps = highlight_incremental(&src, "rust", SyntaxTheme::Darcula, &[], 0)
            .unwrap()
            .checkpoints;

        let edited = src.replace("let v100 = 100;", "let v100 = 100; /* open");
        apply(&mut cache, &mut cps, &edited, 100);
        assert_eq!(colors(&cache), colors(&full(&edited)), "block comment did not propagate");

        // …and closing it again must restore the original colouring.
        let closed = edited.replace("let v100 = 100; /* open", "let v100 = 100;");
        apply(&mut cache, &mut cps, &closed, 100);
        assert_eq!(colors(&cache), colors(&full(&closed)));
    }

    #[test]
    fn incremental_converges_instead_of_reparsing_everything() {
        let mut src = String::new();
        for i in 0..1_000 {
            src.push_str(&format!("let v{} = {};\n", i, i));
        }
        let cps = highlight_incremental(&src, "rust", SyntaxTheme::Darcula, &[], 0)
            .unwrap()
            .checkpoints;
        let edited = src.replace("let v500 = 500;", "let v500 = 501;");
        let inc = highlight_incremental(&edited, "rust", SyntaxTheme::Darcula, &cps, 500).unwrap();

        assert!(inc.converged_at.is_some(), "expected an early exit");
        assert!(
            inc.lines.len() <= 2 * CHECKPOINT_EVERY,
            "re-parsed {} lines for a one-character edit",
            inc.lines.len()
        );
    }

    #[test]
    fn incremental_from_scratch_equals_the_old_full_path() {
        let src = "fn main() {\n    let s = \"hi\";\n    // note\n}\n";
        let old = highlight_buffer(src, "rust", SyntaxTheme::Darcula).unwrap();
        let new = full(src);
        assert_eq!(colors(&old), colors(&new));
    }

    #[test]
    fn each_ui_theme_maps_to_a_loadable_scheme() {
        for t in crate::settings::Theme::ALL {
            assert!(theme_for(t.syntax_theme()).is_some(), "no scheme for {}", t.name());
        }
    }
}


