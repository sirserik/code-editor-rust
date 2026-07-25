use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub theme: Theme,
    pub tab_size: usize,
    pub show_line_numbers: bool,
    pub word_wrap: bool,
    #[serde(default = "default_font_size")]
    pub font_size: f32,
    #[serde(default)]
    pub recent_projects: Vec<RecentProject>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentProject {
    pub path: String,
    pub name: String,
    pub timestamp: u64, // unix seconds
}

fn default_font_size() -> f32 { 14.0 }

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Theme {
    SystemDefault,
    /// JetBrains IntelliJ IDEA "Darcula". The old `TokyoNight` variant was
    /// already *labelled* Darcula but carried a Zed One Dark palette; the alias
    /// keeps existing `settings.json` files loading (without it, serde fails and
    /// silently resets every setting, recent projects included).
    #[serde(alias = "TokyoNight")]
    Darcula,
    Ferrite,
    Dracula,
    OneDark,
    GruvboxDark,
    Nord,
    Catppuccin,
    SolarizedDark,
    MonokaiPro,
    /// JetBrains IntelliJ IDEA Light.
    Light,
}

impl Theme {
    pub const ALL: &'static [Theme] = &[
        Theme::SystemDefault,
        Theme::Darcula,
        Theme::Light,
        Theme::Ferrite,
        Theme::Dracula,
        Theme::OneDark,
        Theme::GruvboxDark,
        Theme::Nord,
        Theme::Catppuccin,
        Theme::SolarizedDark,
        Theme::MonokaiPro,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Theme::SystemDefault => "System",
            Theme::Darcula => "Darcula",
            Theme::Ferrite => "Ferrite",
            Theme::Dracula => "Dracula",
            Theme::OneDark => "One Dark",
            Theme::GruvboxDark => "Gruvbox Dark",
            Theme::Nord => "Nord",
            Theme::Catppuccin => "Catppuccin Mocha",
            Theme::SolarizedDark => "Solarized Dark",
            Theme::MonokaiPro => "Monokai Pro",
            Theme::Light => "IntelliJ Light",
        }
    }

    /// Is this a light-background theme? Used for the handful of places that
    /// still need a coarse light/dark decision (egui `Visuals`, file icons).
    pub fn is_light(&self) -> bool {
        matches!(self.resolved(), Theme::Light)
    }

    /// Which bundled editor colour scheme paints the code for this UI theme.
    pub fn syntax_theme(&self) -> SyntaxTheme {
        match self.resolved() {
            Theme::Darcula => SyntaxTheme::Darcula,
            Theme::Light => SyntaxTheme::IntelliJLight,
            _ => SyntaxTheme::Ferrite,
        }
    }

    /// Detect macOS system dark mode
    pub fn system_is_dark() -> bool {
        // Cache the result: spawning `defaults read` costs ~5-15ms on macOS, and was being
        // called 10+ times per frame via Theme::resolved(). With ~12 callers, that single
        // subprocess invocation was costing 60-180ms per frame. Refresh at most every 5s.
        use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
        use std::sync::OnceLock;
        use std::time::Instant;
        static CACHED: AtomicBool = AtomicBool::new(false);
        static LAST_CHECK_MS: AtomicU64 = AtomicU64::new(0);
        static START: OnceLock<Instant> = OnceLock::new();
        let start = *START.get_or_init(Instant::now);
        let now_ms = start.elapsed().as_millis() as u64;
        let last = LAST_CHECK_MS.load(Ordering::Relaxed);
        if last == 0 || now_ms.saturating_sub(last) > 5_000 {
            let is_dark = std::process::Command::new("defaults")
                .args(["read", "-g", "AppleInterfaceStyle"])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
            CACHED.store(is_dark, Ordering::Relaxed);
            LAST_CHECK_MS.store(now_ms.max(1), Ordering::Relaxed);
            return is_dark;
        }
        CACHED.load(Ordering::Relaxed)
    }

    /// Resolve SystemDefault to actual theme
    pub fn resolved(&self) -> Theme {
        if *self == Theme::SystemDefault {
            if Self::system_is_dark() { Theme::Darcula } else { Theme::Light }
        } else {
            *self
        }
    }
}

/// The bundled tmTheme that paints code. Separate from the UI theme because
/// several UI themes share one editor scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntaxTheme {
    Darcula,
    IntelliJLight,
    Ferrite,
}

use egui::Color32;

#[derive(Debug, Clone, Copy)]
pub struct ThemeColors {
    pub bg: Color32,
    pub sidebar_bg: Color32,
    pub status_bg: Color32,
    pub tab_bar_bg: Color32,
    pub fg: Color32,
    pub fg_dim: Color32,
    pub gutter_fg: Color32,
    pub accent: Color32,
    pub selection_bg: Color32,
    pub current_line_bg: Color32,
    pub cursor_color: Color32,
    pub border: Color32,
    pub bracket_match_bg: Color32,
    pub red: Color32,
    pub green: Color32,
    pub orange: Color32,
    pub fold_fg: Color32,
    pub bracket_colors: [Color32; 6],

    // ── Chrome details ──
    // These used to be `if dark { … } else { … }` literals scattered through
    // `gui/`, which meant every theme except the two defaults got colours picked
    // for a different palette. They live here so a theme owns its whole look.
    /// Text on the status bar.
    pub status_fg: Color32,
    /// 2px underline marking the active editor tab (IntelliJ New UI).
    pub tab_underline: Color32,
    /// Background of the active editor tab.
    pub tab_active_bg: Color32,
    /// Vertical indent guides in the editor and file tree.
    pub indent_guide: Color32,
    /// Pill behind the "⋯ N" marker on a folded line.
    pub folded_bg: Color32,
    /// Row hover highlight in lists (file tree, results).
    pub hover_bg: Color32,
    /// Selected row in lists — file tree, search results, command palette.
    /// Deliberately softer than `selection_bg`: the editor's text selection is
    /// tuned to sit under glyphs for a few characters, and stretching that same
    /// tone across a full-width row reads as a shouting bar.
    pub list_selection_bg: Color32,
    /// Translucent viewport box drawn over the minimap.
    pub minimap_viewport_bg: Color32,
    pub minimap_viewport_border: Color32,
    /// Drag-and-drop target highlight in the file tree.
    pub drop_bg: Color32,
    /// Floating surfaces: command palette, popups, autocomplete.
    pub popup_bg: Color32,
    pub popup_border: Color32,
    /// Editor scrollbar.
    pub scrollbar_track: Color32,
    pub scrollbar_thumb: Color32,
    /// Welcome-screen cards.
    pub card_bg: Color32,
    pub card_bg_hover: Color32,
}

/// Linear mix of two colours; `t` = 0 gives `a`, `t` = 1 gives `b`.
fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgb(f(a.r(), b.r()), f(a.g(), b.g()), f(a.b(), b.b()))
}

impl ThemeColors {
    /// Sensible chrome derived from a theme's core palette. Themes spell out
    /// their own values for anything they care about and inherit the rest with
    /// `..ThemeColors::chrome(bg, fg, accent, panel)`.
    fn chrome(bg: Color32, fg: Color32, accent: Color32, panel: Color32) -> ThemeColors {
        let light = bg.r() as u32 + bg.g() as u32 + bg.b() as u32 > 3 * 128;
        ThemeColors {
            bg,
            sidebar_bg: panel,
            status_bg: panel,
            tab_bar_bg: panel,
            fg,
            fg_dim: mix(bg, fg, 0.6),
            gutter_fg: mix(bg, fg, 0.42),
            accent,
            selection_bg: mix(bg, accent, 0.35),
            current_line_bg: mix(bg, fg, 0.06),
            cursor_color: fg,
            border: mix(panel, fg, 0.14),
            bracket_match_bg: mix(bg, accent, 0.3),
            red: Color32::from_rgb(0xdb, 0x3b, 0x4b),
            green: Color32::from_rgb(0x06, 0x7d, 0x17),
            orange: Color32::from_rgb(0x9e, 0x88, 0x0d),
            fold_fg: mix(bg, fg, 0.45),
            bracket_colors: [accent; 6],
            status_fg: mix(panel, fg, 0.75),
            tab_underline: accent,
            tab_active_bg: bg,
            indent_guide: mix(bg, fg, 0.16),
            folded_bg: mix(bg, fg, 0.13),
            hover_bg: mix(panel, fg, 0.09),
            list_selection_bg: mix(panel, accent, 0.4),
            minimap_viewport_bg: Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 26),
            minimap_viewport_border: Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 64),
            drop_bg: mix(panel, accent, 0.28),
            popup_bg: if light { mix(panel, Color32::WHITE, 0.7) } else { mix(panel, Color32::BLACK, 0.25) },
            popup_border: mix(panel, fg, 0.2),
            scrollbar_track: Color32::TRANSPARENT,
            scrollbar_thumb: mix(bg, fg, 0.28),
            card_bg: mix(bg, fg, 0.05),
            card_bg_hover: mix(bg, fg, 0.1),
        }
    }
}

impl Theme {
    pub fn colors(&self) -> ThemeColors {
        match self {
            // System Default — resolves to dark or light based on macOS setting
            Theme::SystemDefault => Theme::resolved(&Theme::SystemDefault).colors(),

            // ── Darcula — JetBrains IntelliJ IDEA dark ──
            // Editor surface is the classic Darcula #2B2B2B / #A9B7C6; the chrome
            // around it follows the 2023+ "New UI": panels a shade lighter than
            // the editor, hairline borders, one blue accent (#3574F0), a status
            // bar in the panel colour, and a 2px underline on the active tab.
            Theme::Darcula => ThemeColors {
                bg: Color32::from_rgb(0x2b, 0x2b, 0x2b),          // editor
                sidebar_bg: Color32::from_rgb(0x3c, 0x3f, 0x41),  // tool windows
                status_bg: Color32::from_rgb(0x3c, 0x3f, 0x41),
                tab_bar_bg: Color32::from_rgb(0x3c, 0x3f, 0x41),
                fg: Color32::from_rgb(0xa9, 0xb7, 0xc6),          // default text
                fg_dim: Color32::from_rgb(0x87, 0x89, 0x8b),      // secondary label
                gutter_fg: Color32::from_rgb(0x60, 0x63, 0x66),   // line numbers
                accent: Color32::from_rgb(0x35, 0x74, 0xf0),      // New UI blue
                selection_bg: Color32::from_rgb(0x21, 0x42, 0x83),
                current_line_bg: Color32::from_rgb(0x32, 0x32, 0x32),
                cursor_color: Color32::from_rgb(0xbb, 0xbb, 0xbb),
                border: Color32::from_rgb(0x39, 0x3b, 0x40),
                bracket_match_bg: Color32::from_rgb(0x3b, 0x51, 0x4d), // Darcula brace match
                red: Color32::from_rgb(0xff, 0x6b, 0x68),
                green: Color32::from_rgb(0x6a, 0x87, 0x59),
                orange: Color32::from_rgb(0xcc, 0x78, 0x32),
                fold_fg: Color32::from_rgb(0x78, 0x7d, 0x82),
                bracket_colors: [
                    Color32::from_rgb(0xcc, 0x78, 0x32),   // keyword orange
                    Color32::from_rgb(0x98, 0x76, 0xaa),   // field purple
                    Color32::from_rgb(0x68, 0x97, 0xbb),   // number blue
                    Color32::from_rgb(0xff, 0xc6, 0x6d),   // function yellow
                    Color32::from_rgb(0x6a, 0x87, 0x59),   // string green
                    Color32::from_rgb(0xe8, 0xbf, 0x6a),   // tag gold
                ],
                status_fg: Color32::from_rgb(0xbc, 0xbe, 0xc4),
                tab_underline: Color32::from_rgb(0x35, 0x74, 0xf0),
                tab_active_bg: Color32::from_rgb(0x2b, 0x2b, 0x2b),
                indent_guide: Color32::from_rgb(0x4b, 0x4b, 0x4b),
                folded_bg: Color32::from_rgb(0x3a, 0x3a, 0x3a),
                hover_bg: Color32::from_rgb(0x4c, 0x50, 0x52),
                list_selection_bg: Color32::from_rgb(0x2f, 0x65, 0xca), // IntelliJ tree selection
                minimap_viewport_bg: Color32::from_rgba_unmultiplied(0xa9, 0xb7, 0xc6, 20),
                minimap_viewport_border: Color32::from_rgba_unmultiplied(0xa9, 0xb7, 0xc6, 50),
                drop_bg: Color32::from_rgb(0x2f, 0x40, 0x5f),
                popup_bg: Color32::from_rgb(0x3c, 0x3f, 0x41),
                popup_border: Color32::from_rgb(0x55, 0x55, 0x55),
                scrollbar_track: Color32::TRANSPARENT,
                scrollbar_thumb: Color32::from_rgba_premultiplied(0x59, 0x5b, 0x5d, 160),
                card_bg: Color32::from_rgb(0x33, 0x33, 0x33),
                card_bg_hover: Color32::from_rgb(0x3d, 0x3f, 0x41),
            },
            // Ferrite — flagship dark theme: deep navy backgrounds with indigo accents.
            // Palette extracted from the Ferrite design mockup.
            Theme::Ferrite => ThemeColors {
                bg: Color32::from_rgb(0x0b, 0x0d, 0x13),         // #0b0d13 editor surface
                sidebar_bg: Color32::from_rgb(0x11, 0x13, 0x1b), // #11131b project pane
                status_bg: Color32::from_rgb(0x0d, 0x0f, 0x16),  // #0d0f16 status strip
                tab_bar_bg: Color32::from_rgb(0x0d, 0x0f, 0x16), // #0d0f16 chrome
                fg: Color32::from_rgb(0xc8, 0xcf, 0xe0),         // #c8cfe0 primary text
                fg_dim: Color32::from_rgb(0x69, 0x71, 0x8a),     // #69718a secondary text
                gutter_fg: Color32::from_rgb(0x38, 0x3f, 0x52),  // #383f52 line numbers
                accent: Color32::from_rgb(0x7c, 0x8c, 0xff),     // #7c8cff indigo accent
                selection_bg: Color32::from_rgba_unmultiplied(0x7c, 0x8c, 0xff, 56), // ~22%
                current_line_bg: Color32::from_rgb(0x16, 0x19, 0x22), // #161922
                cursor_color: Color32::from_rgb(0x7c, 0x8c, 0xff),
                border: Color32::from_rgb(0x1c, 0x22, 0x33),     // #1c2233 hairline
                bracket_match_bg: Color32::from_rgb(0x2a, 0x33, 0x4a),
                red: Color32::from_rgb(0xff, 0x6b, 0x88),
                green: Color32::from_rgb(0xb5, 0xe0, 0x8e),     // #b5e08e strings
                orange: Color32::from_rgb(0xfb, 0xbf, 0x24),
                fold_fg: Color32::from_rgb(0x4a, 0x52, 0x66),
                bracket_colors: [
                    Color32::from_rgb(0xc7, 0x9b, 0xff),   // purple (keywords)
                    Color32::from_rgb(0x82, 0xaa, 0xff),   // blue (functions)
                    Color32::from_rgb(0x7f, 0xdc, 0xf0),   // cyan
                    Color32::from_rgb(0xb5, 0xe0, 0x8e),   // green (strings)
                    Color32::from_rgb(0xfb, 0xbf, 0x24),   // yellow
                    Color32::from_rgb(0xff, 0x6b, 0x88),   // pink/red
                ],
                ..ThemeColors::chrome(Color32::from_rgb(0x0b,0x0d,0x13), Color32::from_rgb(0xc8,0xcf,0xe0), Color32::from_rgb(0x7c,0x8c,0xff), Color32::from_rgb(0x11,0x13,0x1b))
            },
            Theme::Dracula => ThemeColors {
                bg: Color32::from_rgb(40, 42, 54),
                sidebar_bg: Color32::from_rgb(33, 34, 44),
                status_bg: Color32::from_rgb(50, 52, 66),
                tab_bar_bg: Color32::from_rgb(30, 31, 41),
                fg: Color32::from_rgb(248, 248, 242),
                fg_dim: Color32::from_rgb(148, 150, 160),
                gutter_fg: Color32::from_rgb(80, 82, 96),
                accent: Color32::from_rgb(189, 147, 249),
                selection_bg: Color32::from_rgb(68, 71, 90),
                current_line_bg: Color32::from_rgb(50, 52, 66),
                cursor_color: Color32::from_rgb(248, 248, 242),
                border: Color32::from_rgb(68, 71, 90),
                bracket_match_bg: Color32::from_rgb(80, 82, 100),
                red: Color32::from_rgb(255, 85, 85),
                green: Color32::from_rgb(80, 250, 123),
                orange: Color32::from_rgb(255, 184, 108),
                fold_fg: Color32::from_rgb(100, 102, 120),
                bracket_colors: [
                    Color32::from_rgb(255, 184, 108),
                    Color32::from_rgb(189, 147, 249),
                    Color32::from_rgb(139, 233, 253),
                    Color32::from_rgb(255, 85, 85),
                    Color32::from_rgb(80, 250, 123),
                    Color32::from_rgb(255, 121, 198),
                ],
                ..ThemeColors::chrome(Color32::from_rgb(40,42,54), Color32::from_rgb(248,248,242), Color32::from_rgb(189,147,249), Color32::from_rgb(33,34,44))
            },
            Theme::OneDark => ThemeColors {
                bg: Color32::from_rgb(40, 44, 52),
                sidebar_bg: Color32::from_rgb(33, 37, 43),
                status_bg: Color32::from_rgb(48, 52, 62),
                tab_bar_bg: Color32::from_rgb(30, 33, 39),
                fg: Color32::from_rgb(171, 178, 191),
                fg_dim: Color32::from_rgb(120, 126, 138),
                gutter_fg: Color32::from_rgb(75, 80, 92),
                accent: Color32::from_rgb(97, 175, 239),
                selection_bg: Color32::from_rgb(55, 60, 72),
                current_line_bg: Color32::from_rgb(44, 48, 58),
                cursor_color: Color32::from_rgb(171, 178, 191),
                border: Color32::from_rgb(55, 60, 72),
                bracket_match_bg: Color32::from_rgb(75, 80, 95),
                red: Color32::from_rgb(224, 108, 117),
                green: Color32::from_rgb(152, 195, 121),
                orange: Color32::from_rgb(209, 154, 102),
                fold_fg: Color32::from_rgb(90, 95, 108),
                bracket_colors: [
                    Color32::from_rgb(209, 154, 102),
                    Color32::from_rgb(198, 120, 221),
                    Color32::from_rgb(86, 182, 194),
                    Color32::from_rgb(224, 108, 117),
                    Color32::from_rgb(152, 195, 121),
                    Color32::from_rgb(97, 175, 239),
                ],
                ..ThemeColors::chrome(Color32::from_rgb(40,44,52), Color32::from_rgb(171,178,191), Color32::from_rgb(97,175,239), Color32::from_rgb(33,37,43))
            },
            Theme::GruvboxDark => ThemeColors {
                bg: Color32::from_rgb(40, 40, 40),
                sidebar_bg: Color32::from_rgb(30, 30, 30),
                status_bg: Color32::from_rgb(50, 48, 47),
                tab_bar_bg: Color32::from_rgb(28, 28, 28),
                fg: Color32::from_rgb(235, 219, 178),
                fg_dim: Color32::from_rgb(168, 153, 132),
                gutter_fg: Color32::from_rgb(100, 94, 80),
                accent: Color32::from_rgb(250, 189, 47),
                selection_bg: Color32::from_rgb(60, 56, 54),
                current_line_bg: Color32::from_rgb(50, 48, 47),
                cursor_color: Color32::from_rgb(235, 219, 178),
                border: Color32::from_rgb(60, 56, 54),
                bracket_match_bg: Color32::from_rgb(80, 73, 69),
                red: Color32::from_rgb(251, 73, 52),
                green: Color32::from_rgb(184, 187, 38),
                orange: Color32::from_rgb(254, 128, 25),
                fold_fg: Color32::from_rgb(120, 110, 100),
                bracket_colors: [
                    Color32::from_rgb(250, 189, 47),
                    Color32::from_rgb(211, 134, 155),
                    Color32::from_rgb(142, 192, 124),
                    Color32::from_rgb(254, 128, 25),
                    Color32::from_rgb(131, 165, 152),
                    Color32::from_rgb(184, 187, 38),
                ],
                ..ThemeColors::chrome(Color32::from_rgb(40,40,40), Color32::from_rgb(235,219,178), Color32::from_rgb(250,189,47), Color32::from_rgb(30,30,30))
            },
            Theme::Nord => ThemeColors {
                bg: Color32::from_rgb(46, 52, 64),
                sidebar_bg: Color32::from_rgb(39, 44, 54),
                status_bg: Color32::from_rgb(59, 66, 82),
                tab_bar_bg: Color32::from_rgb(36, 40, 50),
                fg: Color32::from_rgb(216, 222, 233),
                fg_dim: Color32::from_rgb(150, 158, 172),
                gutter_fg: Color32::from_rgb(76, 86, 106),
                accent: Color32::from_rgb(136, 192, 208),
                selection_bg: Color32::from_rgb(67, 76, 94),
                current_line_bg: Color32::from_rgb(55, 62, 76),
                cursor_color: Color32::from_rgb(216, 222, 233),
                border: Color32::from_rgb(59, 66, 82),
                bracket_match_bg: Color32::from_rgb(76, 86, 106),
                red: Color32::from_rgb(191, 97, 106),
                green: Color32::from_rgb(163, 190, 140),
                orange: Color32::from_rgb(208, 135, 112),
                fold_fg: Color32::from_rgb(96, 106, 126),
                bracket_colors: [
                    Color32::from_rgb(235, 203, 139),
                    Color32::from_rgb(180, 142, 173),
                    Color32::from_rgb(136, 192, 208),
                    Color32::from_rgb(208, 135, 112),
                    Color32::from_rgb(163, 190, 140),
                    Color32::from_rgb(129, 161, 193),
                ],
                ..ThemeColors::chrome(Color32::from_rgb(46,52,64), Color32::from_rgb(216,222,233), Color32::from_rgb(136,192,208), Color32::from_rgb(39,44,54))
            },
            Theme::Catppuccin => ThemeColors {
                bg: Color32::from_rgb(30, 30, 46),
                sidebar_bg: Color32::from_rgb(24, 24, 37),
                status_bg: Color32::from_rgb(39, 39, 55),
                tab_bar_bg: Color32::from_rgb(20, 20, 33),
                fg: Color32::from_rgb(205, 214, 244),
                fg_dim: Color32::from_rgb(147, 153, 178),
                gutter_fg: Color32::from_rgb(73, 77, 100),
                accent: Color32::from_rgb(137, 180, 250),
                selection_bg: Color32::from_rgb(49, 50, 68),
                current_line_bg: Color32::from_rgb(39, 39, 55),
                cursor_color: Color32::from_rgb(205, 214, 244),
                border: Color32::from_rgb(49, 50, 68),
                bracket_match_bg: Color32::from_rgb(73, 77, 100),
                red: Color32::from_rgb(243, 139, 168),
                green: Color32::from_rgb(166, 227, 161),
                orange: Color32::from_rgb(250, 179, 135),
                fold_fg: Color32::from_rgb(88, 91, 112),
                bracket_colors: [
                    Color32::from_rgb(249, 226, 175),
                    Color32::from_rgb(203, 166, 247),
                    Color32::from_rgb(137, 220, 235),
                    Color32::from_rgb(250, 179, 135),
                    Color32::from_rgb(166, 227, 161),
                    Color32::from_rgb(137, 180, 250),
                ],
                ..ThemeColors::chrome(Color32::from_rgb(30,30,46), Color32::from_rgb(205,214,244), Color32::from_rgb(137,180,250), Color32::from_rgb(24,24,37))
            },
            Theme::SolarizedDark => ThemeColors {
                bg: Color32::from_rgb(0, 43, 54),
                sidebar_bg: Color32::from_rgb(0, 36, 46),
                status_bg: Color32::from_rgb(7, 54, 66),
                tab_bar_bg: Color32::from_rgb(0, 30, 38),
                fg: Color32::from_rgb(131, 148, 150),
                fg_dim: Color32::from_rgb(88, 110, 117),
                gutter_fg: Color32::from_rgb(58, 80, 87),
                accent: Color32::from_rgb(38, 139, 210),
                selection_bg: Color32::from_rgb(7, 54, 66),
                current_line_bg: Color32::from_rgb(7, 54, 66),
                cursor_color: Color32::from_rgb(131, 148, 150),
                border: Color32::from_rgb(7, 54, 66),
                bracket_match_bg: Color32::from_rgb(30, 75, 88),
                red: Color32::from_rgb(220, 50, 47),
                green: Color32::from_rgb(133, 153, 0),
                orange: Color32::from_rgb(203, 75, 22),
                fold_fg: Color32::from_rgb(68, 95, 102),
                bracket_colors: [
                    Color32::from_rgb(181, 137, 0),
                    Color32::from_rgb(211, 54, 130),
                    Color32::from_rgb(42, 161, 152),
                    Color32::from_rgb(203, 75, 22),
                    Color32::from_rgb(133, 153, 0),
                    Color32::from_rgb(38, 139, 210),
                ],
                ..ThemeColors::chrome(Color32::from_rgb(0,43,54), Color32::from_rgb(131,148,150), Color32::from_rgb(38,139,210), Color32::from_rgb(0,36,46))
            },
            Theme::MonokaiPro => ThemeColors {
                bg: Color32::from_rgb(45, 42, 46),
                sidebar_bg: Color32::from_rgb(37, 34, 38),
                status_bg: Color32::from_rgb(55, 52, 56),
                tab_bar_bg: Color32::from_rgb(32, 30, 33),
                fg: Color32::from_rgb(252, 252, 250),
                fg_dim: Color32::from_rgb(150, 148, 146),
                gutter_fg: Color32::from_rgb(90, 88, 92),
                accent: Color32::from_rgb(120, 220, 232),
                selection_bg: Color32::from_rgb(68, 64, 70),
                current_line_bg: Color32::from_rgb(55, 52, 56),
                cursor_color: Color32::from_rgb(252, 252, 250),
                border: Color32::from_rgb(68, 64, 70),
                bracket_match_bg: Color32::from_rgb(90, 86, 92),
                red: Color32::from_rgb(255, 97, 136),
                green: Color32::from_rgb(169, 220, 118),
                orange: Color32::from_rgb(252, 152, 103),
                fold_fg: Color32::from_rgb(110, 107, 112),
                bracket_colors: [
                    Color32::from_rgb(255, 216, 102),
                    Color32::from_rgb(171, 157, 242),
                    Color32::from_rgb(120, 220, 232),
                    Color32::from_rgb(252, 152, 103),
                    Color32::from_rgb(169, 220, 118),
                    Color32::from_rgb(255, 97, 136),
                ],
                ..ThemeColors::chrome(Color32::from_rgb(45,42,46), Color32::from_rgb(252,252,250), Color32::from_rgb(120,220,232), Color32::from_rgb(37,34,38))
            },
            // ── IntelliJ Light — JetBrains IntelliJ IDEA light ──
            // White editor with the signature #FCFAED caret row, New UI chrome
            // (#F7F8FA panels, #EBECF0 hairlines, #3574F0 accent). The status bar
            // is panel-coloured with dark text — the old solid-blue VS Code strip
            // forced white-on-blue text that clashed with everything else.
            Theme::Light => ThemeColors {
                bg: Color32::from_rgb(0xff, 0xff, 0xff),          // editor
                sidebar_bg: Color32::from_rgb(0xf7, 0xf8, 0xfa),  // tool windows
                status_bg: Color32::from_rgb(0xf7, 0xf8, 0xfa),
                tab_bar_bg: Color32::from_rgb(0xf7, 0xf8, 0xfa),
                fg: Color32::from_rgb(0x08, 0x08, 0x08),          // default text
                fg_dim: Color32::from_rgb(0x6c, 0x70, 0x7e),      // secondary label
                gutter_fg: Color32::from_rgb(0x9b, 0x9b, 0x9b),   // line numbers
                accent: Color32::from_rgb(0x35, 0x74, 0xf0),      // New UI blue
                selection_bg: Color32::from_rgb(0xa6, 0xd2, 0xff),
                current_line_bg: Color32::from_rgb(0xfc, 0xfa, 0xed), // the IntelliJ caret row
                cursor_color: Color32::from_rgb(0x00, 0x00, 0x00),
                border: Color32::from_rgb(0xeb, 0xec, 0xf0),
                bracket_match_bg: Color32::from_rgb(0x93, 0xd9, 0xd9), // IntelliJ brace match
                red: Color32::from_rgb(0xdb, 0x3b, 0x4b),
                green: Color32::from_rgb(0x06, 0x7d, 0x17),
                orange: Color32::from_rgb(0x9e, 0x88, 0x0d),
                fold_fg: Color32::from_rgb(0x8c, 0x8c, 0x8c),
                bracket_colors: [
                    Color32::from_rgb(0x00, 0x33, 0xb3),   // keyword blue
                    Color32::from_rgb(0x87, 0x10, 0x94),   // field purple
                    Color32::from_rgb(0x00, 0x62, 0x7a),   // function teal
                    Color32::from_rgb(0x9e, 0x88, 0x0d),   // annotation gold
                    Color32::from_rgb(0x06, 0x7d, 0x17),   // string green
                    Color32::from_rgb(0x17, 0x50, 0xeb),   // number blue
                ],
                status_fg: Color32::from_rgb(0x3c, 0x3f, 0x41),
                tab_underline: Color32::from_rgb(0x35, 0x74, 0xf0),
                tab_active_bg: Color32::from_rgb(0xff, 0xff, 0xff),
                indent_guide: Color32::from_rgb(0xd6, 0xd6, 0xd6),
                folded_bg: Color32::from_rgb(0xe8, 0xea, 0xef),
                hover_bg: Color32::from_rgb(0xea, 0xed, 0xf2),
                list_selection_bg: Color32::from_rgb(0xd4, 0xe2, 0xff), // IntelliJ tree selection
                minimap_viewport_bg: Color32::from_rgba_unmultiplied(0x35, 0x74, 0xf0, 18),
                minimap_viewport_border: Color32::from_rgba_unmultiplied(0x35, 0x74, 0xf0, 45),
                drop_bg: Color32::from_rgb(0xd4, 0xe2, 0xff),
                popup_bg: Color32::from_rgb(0xff, 0xff, 0xff),
                popup_border: Color32::from_rgb(0xd3, 0xd5, 0xdb),
                scrollbar_track: Color32::TRANSPARENT,
                scrollbar_thumb: Color32::from_rgba_premultiplied(0xa0, 0xa4, 0xad, 150),
                card_bg: Color32::from_rgb(0xf2, 0xf4, 0xf8),
                card_bg_hover: Color32::from_rgb(0xe8, 0xeb, 0xf2),
            },
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: Theme::Darcula,
            tab_size: 4,
            show_line_numbers: true,
            word_wrap: true,
            font_size: 14.0,
            recent_projects: Vec::new(),
        }
    }
}

impl Settings {
    pub fn add_recent_project(&mut self, path: &str) {
        // Normalize: remove trailing slash
        let path = path.trim_end_matches('/');
        let name = std::path::Path::new(path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string());
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        // Remove if already exists (with or without trailing slash)
        self.recent_projects.retain(|p| {
            p.path.trim_end_matches('/') != path
        });

        // Add to front
        self.recent_projects.insert(0, RecentProject {
            path: path.to_string(),
            name,
            timestamp,
        });

        // Keep max 10
        self.recent_projects.truncate(10);
        self.save();
    }

    pub fn remove_recent_project(&mut self, path: &str) {
        self.recent_projects.retain(|p| p.path != path);
        self.save();
    }
}

impl Settings {
    fn config_path() -> PathBuf {
        // Tests must not pollute the real user config (we previously had stray
        // `/test/project*` entries in recent_projects from `cargo test`).
        #[cfg(test)]
        {
            return std::env::temp_dir()
                .join(format!("code-editor-rust-test-{}.json", std::process::id()));
        }
        #[cfg(not(test))]
        {
            let config_dir = dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("code-editor-rust");
            fs::create_dir_all(&config_dir).ok();
            config_dir.join("settings.json")
        }
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if let Ok(data) = fs::read_to_string(&path) {
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Ok(data) = serde_json::to_string_pretty(self) {
            if let Err(e) = fs::write(&path, data) {
                eprintln!("Failed to save settings to {}: {}", path.display(), e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings() {
        let s = Settings::default();
        assert_eq!(s.theme, Theme::Darcula);
        assert_eq!(s.tab_size, 4);
        assert!(s.show_line_numbers);
        assert!(s.word_wrap);
        assert_eq!(s.font_size, 14.0);
        assert!(s.recent_projects.is_empty());
    }

    #[test]
    fn theme_names() {
        assert_eq!(Theme::Light.name(), "IntelliJ Light");
        assert_eq!(Theme::Darcula.name(), "Darcula");
        assert_eq!(Theme::SystemDefault.name(), "System");
    }

    #[test]
    fn theme_all_contains_system() {
        assert!(Theme::ALL.contains(&Theme::SystemDefault));
        assert!(Theme::ALL.contains(&Theme::Light));
        assert!(Theme::ALL.contains(&Theme::Darcula));
    }

    #[test]
    fn theme_resolved_system() {
        let resolved = Theme::SystemDefault.resolved();
        // The IntelliJ pair is what System resolves to.
        assert!(resolved == Theme::Light || resolved == Theme::Darcula);
    }

    #[test]
    fn old_tokyonight_settings_still_load() {
        // Existing settings.json files name the theme "TokyoNight". Without the
        // serde alias, deserialization fails and `load()` silently resets every
        // setting — including the recent-projects list.
        let json = r#"{"theme":"TokyoNight","tab_size":2,"show_line_numbers":true,
                       "word_wrap":false,"font_size":16.0,"recent_projects":[]}"#;
        let s: Settings = serde_json::from_str(json).expect("legacy settings must load");
        assert_eq!(s.theme, Theme::Darcula);
        assert_eq!(s.tab_size, 2);
        assert_eq!(s.font_size, 16.0);
    }

    #[test]
    fn intellij_themes_use_jetbrains_palette() {
        let d = Theme::Darcula.colors();
        assert_eq!(d.bg, Color32::from_rgb(0x2b, 0x2b, 0x2b));
        assert_eq!(d.fg, Color32::from_rgb(0xa9, 0xb7, 0xc6));
        assert_eq!(d.current_line_bg, Color32::from_rgb(0x32, 0x32, 0x32));
        assert_eq!(d.selection_bg, Color32::from_rgb(0x21, 0x42, 0x83));

        let l = Theme::Light.colors();
        assert_eq!(l.bg, Color32::WHITE);
        // The signature IntelliJ caret row, not a grey tint.
        assert_eq!(l.current_line_bg, Color32::from_rgb(0xfc, 0xfa, 0xed));
        assert_eq!(l.selection_bg, Color32::from_rgb(0xa6, 0xd2, 0xff));
        // Status bar text must be readable on a panel-coloured (not blue) strip.
        assert_ne!(l.status_bg, l.status_fg);
        assert!(l.status_fg.r() < 128, "light status text should be dark");
    }

    #[test]
    fn every_theme_has_usable_chrome() {
        for theme in Theme::ALL {
            let c = theme.colors();
            let name = theme.name();
            assert_ne!(c.fg, c.bg, "{}: text invisible on editor bg", name);
            assert_ne!(c.status_fg, c.status_bg, "{}: status text invisible", name);
            assert_ne!(c.fg, c.sidebar_bg, "{}: sidebar text invisible", name);
            assert_ne!(c.indent_guide, c.bg, "{}: indent guides invisible", name);
            assert_ne!(c.popup_bg, c.popup_border, "{}: popup edge invisible", name);
            assert_ne!(c.hover_bg, c.sidebar_bg, "{}: hover state invisible", name);
            assert_ne!(c.list_selection_bg, c.sidebar_bg, "{}: list selection invisible", name);
            assert_ne!(c.list_selection_bg, c.hover_bg, "{}: selection == hover", name);
        }
    }

    #[test]
    fn list_selection_is_softer_than_editor_selection() {
        // A full-width row painted in the editor's text-selection tone shouts.
        // The two must be distinct so tuning one doesn't drag the other along.
        for theme in [Theme::Darcula, Theme::Light] {
            let c = theme.colors();
            assert_ne!(c.list_selection_bg, c.selection_bg, "{}", theme.name());
        }
    }

    #[test]
    fn theme_resolved_non_system() {
        assert_eq!(Theme::Dracula.resolved(), Theme::Dracula);
        assert_eq!(Theme::Light.resolved(), Theme::Light);
    }

    #[test]
    fn theme_colors_not_panic() {
        // Ensure all themes produce valid colors
        for theme in Theme::ALL {
            let colors = theme.colors();
            assert_ne!(colors.fg, colors.bg); // fg should differ from bg
        }
    }

    #[test]
    fn add_recent_project() {
        let mut s = Settings::default();
        s.recent_projects.clear();
        s.add_recent_project("/test/project");
        assert_eq!(s.recent_projects.len(), 1);
        assert_eq!(s.recent_projects[0].path, "/test/project");
        assert_eq!(s.recent_projects[0].name, "project");
    }

    #[test]
    fn add_recent_project_dedup() {
        let mut s = Settings::default();
        s.recent_projects.clear();
        s.add_recent_project("/test/project");
        s.add_recent_project("/test/project/");
        assert_eq!(s.recent_projects.len(), 1); // no duplicate
    }

    #[test]
    fn add_recent_project_limit() {
        let mut s = Settings::default();
        s.recent_projects.clear();
        for i in 0..15 {
            s.add_recent_project(&format!("/test/project{}", i));
        }
        assert_eq!(s.recent_projects.len(), 10); // max 10
    }

    #[test]
    fn remove_recent_project() {
        let mut s = Settings::default();
        s.recent_projects.clear();
        s.add_recent_project("/test/a");
        s.add_recent_project("/test/b");
        s.remove_recent_project("/test/a");
        assert_eq!(s.recent_projects.len(), 1);
        assert_eq!(s.recent_projects[0].path, "/test/b");
    }
}
