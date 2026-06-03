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
    Ferrite,
    TokyoNight,
    Dracula,
    OneDark,
    GruvboxDark,
    Nord,
    Catppuccin,
    SolarizedDark,
    MonokaiPro,
    Light,
}

impl Theme {
    pub const ALL: &'static [Theme] = &[
        Theme::SystemDefault,
        Theme::Ferrite,
        Theme::TokyoNight,
        Theme::Dracula,
        Theme::OneDark,
        Theme::GruvboxDark,
        Theme::Nord,
        Theme::Catppuccin,
        Theme::SolarizedDark,
        Theme::MonokaiPro,
        Theme::Light,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Theme::SystemDefault => "System",
            Theme::Ferrite => "Ferrite",
            Theme::TokyoNight => "Darcula",
            Theme::Dracula => "Dracula",
            Theme::OneDark => "One Dark",
            Theme::GruvboxDark => "Gruvbox Dark",
            Theme::Nord => "Nord",
            Theme::Catppuccin => "Catppuccin Mocha",
            Theme::SolarizedDark => "Solarized Dark",
            Theme::MonokaiPro => "Monokai Pro",
            Theme::Light => "Light",
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
            if Self::system_is_dark() { Theme::Ferrite } else { Theme::Light }
        } else {
            *self
        }
    }
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
}

impl Theme {
    pub fn colors(&self) -> ThemeColors {
        match self {
            // System Default — resolves to dark or light based on macOS setting
            Theme::SystemDefault => Theme::resolved(&Theme::SystemDefault).colors(),
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
            },
            // Darcula — Zed One Dark inspired palette
            Theme::TokyoNight => ThemeColors {
                bg: Color32::from_rgb(40, 44, 51),              // #282c33
                sidebar_bg: Color32::from_rgb(59, 65, 77),      // #3b414d
                status_bg: Color32::from_rgb(59, 65, 77),       // #3b414d
                tab_bar_bg: Color32::from_rgb(47, 52, 62),      // #2f343e
                fg: Color32::from_rgb(208, 212, 218),           // #d0d4da
                fg_dim: Color32::from_rgb(148, 155, 168),       // muted text
                gutter_fg: Color32::from_rgb(78, 90, 95),       // #4e5a5f
                accent: Color32::from_rgb(116, 173, 232),       // #74ade8
                selection_bg: Color32::from_rgba_unmultiplied(116, 173, 232, 61), // #74ade8 at 24%
                current_line_bg: Color32::from_rgba_unmultiplied(47, 52, 62, 191), // #2f343e at 75%
                cursor_color: Color32::from_rgb(116, 173, 232), // accent as cursor
                border: Color32::from_rgb(70, 75, 87),          // #464b57
                bracket_match_bg: Color32::from_rgb(58, 87, 110),
                red: Color32::from_rgb(255, 107, 104),
                green: Color32::from_rgb(106, 171, 115),
                orange: Color32::from_rgb(204, 147, 89),
                fold_fg: Color32::from_rgb(110, 113, 116),
                bracket_colors: [
                    Color32::from_rgb(204, 147, 89),   // gold
                    Color32::from_rgb(179, 131, 191),   // purple
                    Color32::from_rgb(104, 151, 210),   // blue
                    Color32::from_rgb(255, 107, 104),   // red
                    Color32::from_rgb(106, 171, 115),   // green
                    Color32::from_rgb(86, 182, 194),    // cyan
                ],
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
            },
            // Light — JetBrains IntelliJ / VS Code inspired
            Theme::Light => ThemeColors {
                bg: Color32::from_rgb(255, 255, 255),            // White editor
                sidebar_bg: Color32::from_rgb(245, 246, 247),    // Subtle warm gray (IntelliJ-like)
                status_bg: Color32::from_rgb(0, 122, 204),       // Blue status bar (VS Code)
                tab_bar_bg: Color32::from_rgb(236, 237, 239),    // Tab strip
                fg: Color32::from_rgb(30, 30, 30),               // Near-black text
                fg_dim: Color32::from_rgb(110, 115, 125),        // Readable secondary
                gutter_fg: Color32::from_rgb(150, 155, 165),     // Line numbers
                accent: Color32::from_rgb(0, 122, 204),          // VS Code blue
                selection_bg: Color32::from_rgb(218, 230, 247),  // Soft, refined selection
                current_line_bg: Color32::from_rgb(248, 249, 250), // Very subtle
                cursor_color: Color32::from_rgb(0, 0, 0),        // Black cursor
                border: Color32::from_rgb(224, 226, 230),        // Visible borders
                bracket_match_bg: Color32::from_rgb(204, 222, 244),
                red: Color32::from_rgb(205, 49, 49),
                green: Color32::from_rgb(22, 130, 60),
                orange: Color32::from_rgb(191, 120, 12),
                fold_fg: Color32::from_rgb(145, 150, 160),
                bracket_colors: [
                    Color32::from_rgb(0, 122, 204),    // blue
                    Color32::from_rgb(150, 40, 190),   // purple
                    Color32::from_rgb(0, 140, 125),    // teal
                    Color32::from_rgb(205, 49, 49),    // red
                    Color32::from_rgb(22, 130, 60),    // green
                    Color32::from_rgb(191, 120, 12),   // orange
                ],
            },
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: Theme::Ferrite,
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
        assert_eq!(s.theme, Theme::Ferrite);
        assert_eq!(s.tab_size, 4);
        assert!(s.show_line_numbers);
        assert!(s.word_wrap);
        assert_eq!(s.font_size, 14.0);
        assert!(s.recent_projects.is_empty());
    }

    #[test]
    fn theme_names() {
        assert_eq!(Theme::Light.name(), "Light");
        assert_eq!(Theme::TokyoNight.name(), "Darcula");
        assert_eq!(Theme::SystemDefault.name(), "System");
    }

    #[test]
    fn theme_all_contains_system() {
        assert!(Theme::ALL.contains(&Theme::SystemDefault));
        assert!(Theme::ALL.contains(&Theme::Light));
        assert!(Theme::ALL.contains(&Theme::TokyoNight));
    }

    #[test]
    fn theme_resolved_system() {
        let resolved = Theme::SystemDefault.resolved();
        // Should resolve to either Light or Ferrite (the dark default)
        assert!(resolved == Theme::Light || resolved == Theme::Ferrite);
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
