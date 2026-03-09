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
}

fn default_font_size() -> f32 { 14.0 }

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Theme {
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
            // JetBrains Darcula
            Theme::TokyoNight => ThemeColors {
                bg: Color32::from_rgb(43, 43, 43),              // Darcula editor bg
                sidebar_bg: Color32::from_rgb(43, 43, 43),      // Same as editor — JB style
                status_bg: Color32::from_rgb(60, 63, 65),       // Slightly lighter status
                tab_bar_bg: Color32::from_rgb(49, 51, 53),      // Subtle tab strip
                fg: Color32::from_rgb(187, 187, 187),           // Default text
                fg_dim: Color32::from_rgb(128, 128, 128),       // Secondary text
                gutter_fg: Color32::from_rgb(96, 99, 102),      // Line numbers
                accent: Color32::from_rgb(75, 110, 175),        // JB Darcula link blue
                selection_bg: Color32::from_rgb(33, 66, 131),   // Deep selection blue
                current_line_bg: Color32::from_rgb(50, 50, 50), // Subtle current line
                cursor_color: Color32::from_rgb(187, 187, 187),
                border: Color32::from_rgb(50, 50, 50),          // Very subtle borders
                bracket_match_bg: Color32::from_rgb(58, 87, 110),
                red: Color32::from_rgb(255, 107, 104),
                green: Color32::from_rgb(106, 171, 115),
                orange: Color32::from_rgb(204, 147, 89),
                fold_fg: Color32::from_rgb(96, 99, 102),
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
            // JetBrains IntelliJ Light
            Theme::Light => ThemeColors {
                bg: Color32::from_rgb(255, 255, 255),            // White editor bg — real IntelliJ
                sidebar_bg: Color32::from_rgb(255, 255, 255),    // Same as editor — JB style
                status_bg: Color32::from_rgb(62, 62, 62),        // Dark status bar
                tab_bar_bg: Color32::from_rgb(238, 238, 238),    // Subtle tab strip
                fg: Color32::from_rgb(0, 0, 0),                  // Black text — real IntelliJ
                fg_dim: Color32::from_rgb(120, 120, 120),        // Secondary text
                gutter_fg: Color32::from_rgb(153, 154, 158),     // Line numbers
                accent: Color32::from_rgb(55, 125, 207),         // JB blue
                selection_bg: Color32::from_rgb(166, 210, 255),  // Selection
                current_line_bg: Color32::from_rgb(252, 250, 237), // Subtle yellow tint
                cursor_color: Color32::from_rgb(0, 0, 0),        // Black cursor
                border: Color32::from_rgb(225, 225, 225),        // Soft borders
                bracket_match_bg: Color32::from_rgb(153, 204, 255),
                red: Color32::from_rgb(199, 37, 78),
                green: Color32::from_rgb(10, 132, 57),
                orange: Color32::from_rgb(199, 125, 10),
                fold_fg: Color32::from_rgb(153, 154, 158),
                bracket_colors: [
                    Color32::from_rgb(55, 125, 207),
                    Color32::from_rgb(140, 44, 180),
                    Color32::from_rgb(0, 140, 125),
                    Color32::from_rgb(199, 37, 78),
                    Color32::from_rgb(10, 132, 57),
                    Color32::from_rgb(199, 125, 10),
                ],
            },
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: Theme::TokyoNight,
            tab_size: 4,
            show_line_numbers: true,
            word_wrap: false,
            font_size: 14.0,
        }
    }
}

impl Settings {
    fn config_path() -> PathBuf {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("code-editor-rust");
        fs::create_dir_all(&config_dir).ok();
        config_dir.join("settings.json")
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
            fs::write(path, data).ok();
        }
    }
}
