use egui::{Color32, CornerRadius, Stroke, Visuals};
use serde::{Deserialize, Serialize};

/// 主题颜色定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeColors {
    /// 背景色
    pub bg_primary: Color32,
    pub bg_secondary: Color32,
    pub bg_tertiary: Color32,

    /// 前景/文字色
    pub fg_primary: Color32,
    pub fg_secondary: Color32,
    pub fg_muted: Color32,

    /// 强调色
    pub accent: Color32,
    pub accent_hover: Color32,

    /// 语义色
    pub success: Color32,
    pub warning: Color32,
    pub error: Color32,
    pub info: Color32,

    /// 边框色
    pub border: Color32,
    pub border_hover: Color32,

    /// 选中/高亮
    pub selection: Color32,
    pub highlight: Color32,
}

/// 预设主题枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ThemePreset {
    #[default]
    TokyoNight,
    TokyoNightStorm,
    TokyoNightLight,
    CatppuccinMocha,
    CatppuccinMacchiato,
    CatppuccinFrappe,
    CatppuccinLatte,
    OneDark,
    OneDarkVivid,
    OneLight,
    GruvboxDark,
    GruvboxLight,
    Dracula,
    Nord,
    SolarizedDark,
    SolarizedLight,
    MonokaiPro,
    GithubDark,
    GithubLight,
}

impl ThemePreset {
    /// 显示名称
    pub fn display_name(&self) -> &'static str {
        match self {
            ThemePreset::TokyoNight => "Tokyo Night",
            ThemePreset::TokyoNightStorm => "Tokyo Night Storm",
            ThemePreset::TokyoNightLight => "Tokyo Night Light",
            ThemePreset::CatppuccinMocha => "Catppuccin Mocha",
            ThemePreset::CatppuccinMacchiato => "Catppuccin Macchiato",
            ThemePreset::CatppuccinFrappe => "Catppuccin Frappé",
            ThemePreset::CatppuccinLatte => "Catppuccin Latte",
            ThemePreset::OneDark => "One Dark",
            ThemePreset::OneDarkVivid => "One Dark Vivid",
            ThemePreset::OneLight => "One Light",
            ThemePreset::GruvboxDark => "Gruvbox Dark",
            ThemePreset::GruvboxLight => "Gruvbox Light",
            ThemePreset::Dracula => "Dracula",
            ThemePreset::Nord => "Nord",
            ThemePreset::SolarizedDark => "Solarized Dark",
            ThemePreset::SolarizedLight => "Solarized Light",
            ThemePreset::MonokaiPro => "Monokai Pro",
            ThemePreset::GithubDark => "GitHub Dark",
            ThemePreset::GithubLight => "GitHub Light",
        }
    }

    /// 是否为暗色主题
    pub fn is_dark(&self) -> bool {
        !matches!(
            self,
            ThemePreset::TokyoNightLight
                | ThemePreset::CatppuccinLatte
                | ThemePreset::OneLight
                | ThemePreset::GruvboxLight
                | ThemePreset::SolarizedLight
                | ThemePreset::GithubLight
        )
    }

    /// 获取主题颜色
    pub fn colors(&self) -> ThemeColors {
        match self {
            // ===== Tokyo Night 系列 =====
            ThemePreset::TokyoNight => ThemeColors {
                bg_primary: Color32::from_rgb(26, 27, 38),
                bg_secondary: Color32::from_rgb(36, 40, 59),
                bg_tertiary: Color32::from_rgb(41, 46, 66),
                fg_primary: Color32::from_rgb(192, 202, 245),
                fg_secondary: Color32::from_rgb(169, 177, 214),
                fg_muted: Color32::from_rgb(86, 95, 137),
                accent: Color32::from_rgb(122, 162, 247),
                accent_hover: Color32::from_rgb(157, 184, 248),
                success: Color32::from_rgb(158, 206, 106),
                warning: Color32::from_rgb(224, 175, 104),
                error: Color32::from_rgb(247, 118, 142),
                info: Color32::from_rgb(125, 207, 255),
                border: Color32::from_rgb(41, 46, 66),
                border_hover: Color32::from_rgb(122, 162, 247),
                selection: Color32::from_rgba_unmultiplied(70, 100, 160, 120),
                highlight: Color32::from_rgba_unmultiplied(224, 175, 104, 40),
            },
            ThemePreset::TokyoNightStorm => ThemeColors {
                bg_primary: Color32::from_rgb(36, 40, 59),
                bg_secondary: Color32::from_rgb(52, 59, 88),
                bg_tertiary: Color32::from_rgb(59, 66, 97),
                fg_primary: Color32::from_rgb(192, 202, 245),
                fg_secondary: Color32::from_rgb(169, 177, 214),
                fg_muted: Color32::from_rgb(86, 95, 137),
                accent: Color32::from_rgb(122, 162, 247),
                accent_hover: Color32::from_rgb(157, 184, 248),
                success: Color32::from_rgb(158, 206, 106),
                warning: Color32::from_rgb(224, 175, 104),
                error: Color32::from_rgb(247, 118, 142),
                info: Color32::from_rgb(125, 207, 255),
                border: Color32::from_rgb(59, 66, 97),
                border_hover: Color32::from_rgb(122, 162, 247),
                selection: Color32::from_rgba_unmultiplied(70, 100, 160, 120),
                highlight: Color32::from_rgba_unmultiplied(224, 175, 104, 40),
            },
            ThemePreset::TokyoNightLight => ThemeColors {
                bg_primary: Color32::from_rgb(213, 214, 219),
                bg_secondary: Color32::from_rgb(223, 224, 229),
                bg_tertiary: Color32::from_rgb(233, 234, 239),
                fg_primary: Color32::from_rgb(52, 59, 88),
                fg_secondary: Color32::from_rgb(74, 79, 108),
                fg_muted: Color32::from_rgb(150, 153, 167),
                accent: Color32::from_rgb(52, 84, 138),
                accent_hover: Color32::from_rgb(62, 94, 148),
                success: Color32::from_rgb(72, 117, 62),
                warning: Color32::from_rgb(143, 95, 24),
                error: Color32::from_rgb(195, 64, 67),
                info: Color32::from_rgb(14, 108, 153),
                border: Color32::from_rgb(200, 201, 206),
                border_hover: Color32::from_rgb(52, 84, 138),
                selection: Color32::from_rgba_unmultiplied(52, 84, 138, 90),
                highlight: Color32::from_rgba_unmultiplied(143, 95, 24, 40),
            },

            // ===== Catppuccin 系列 =====
            ThemePreset::CatppuccinMocha => ThemeColors {
                bg_primary: Color32::from_rgb(30, 30, 46),
                bg_secondary: Color32::from_rgb(49, 50, 68),
                bg_tertiary: Color32::from_rgb(69, 71, 90),
                fg_primary: Color32::from_rgb(205, 214, 244),
                fg_secondary: Color32::from_rgb(186, 194, 222),
                fg_muted: Color32::from_rgb(108, 112, 134),
                accent: Color32::from_rgb(137, 180, 250),
                accent_hover: Color32::from_rgb(180, 190, 254),
                success: Color32::from_rgb(166, 227, 161),
                warning: Color32::from_rgb(249, 226, 175),
                error: Color32::from_rgb(243, 139, 168),
                info: Color32::from_rgb(137, 220, 235),
                border: Color32::from_rgb(69, 71, 90),
                border_hover: Color32::from_rgb(137, 180, 250),
                selection: Color32::from_rgba_unmultiplied(80, 110, 170, 120),
                highlight: Color32::from_rgba_unmultiplied(249, 226, 175, 40),
            },
            ThemePreset::CatppuccinMacchiato => ThemeColors {
                bg_primary: Color32::from_rgb(36, 39, 58),
                bg_secondary: Color32::from_rgb(54, 58, 79),
                bg_tertiary: Color32::from_rgb(73, 77, 100),
                fg_primary: Color32::from_rgb(202, 211, 245),
                fg_secondary: Color32::from_rgb(184, 192, 224),
                fg_muted: Color32::from_rgb(110, 115, 141),
                accent: Color32::from_rgb(138, 173, 244),
                accent_hover: Color32::from_rgb(183, 189, 248),
                success: Color32::from_rgb(166, 218, 149),
                warning: Color32::from_rgb(238, 212, 159),
                error: Color32::from_rgb(237, 135, 150),
                info: Color32::from_rgb(145, 215, 227),
                border: Color32::from_rgb(73, 77, 100),
                border_hover: Color32::from_rgb(138, 173, 244),
                selection: Color32::from_rgba_unmultiplied(80, 110, 170, 120),
                highlight: Color32::from_rgba_unmultiplied(238, 212, 159, 40),
            },
            ThemePreset::CatppuccinFrappe => ThemeColors {
                bg_primary: Color32::from_rgb(48, 52, 70),
                bg_secondary: Color32::from_rgb(65, 69, 89),
                bg_tertiary: Color32::from_rgb(81, 87, 109),
                fg_primary: Color32::from_rgb(198, 208, 245),
                fg_secondary: Color32::from_rgb(181, 191, 226),
                fg_muted: Color32::from_rgb(115, 121, 148),
                accent: Color32::from_rgb(140, 170, 238),
                accent_hover: Color32::from_rgb(186, 187, 241),
                success: Color32::from_rgb(166, 209, 137),
                warning: Color32::from_rgb(229, 200, 144),
                error: Color32::from_rgb(231, 130, 132),
                info: Color32::from_rgb(153, 209, 219),
                border: Color32::from_rgb(81, 87, 109),
                border_hover: Color32::from_rgb(140, 170, 238),
                selection: Color32::from_rgba_unmultiplied(80, 110, 170, 120),
                highlight: Color32::from_rgba_unmultiplied(229, 200, 144, 40),
            },
            ThemePreset::CatppuccinLatte => ThemeColors {
                bg_primary: Color32::from_rgb(239, 241, 245),
                bg_secondary: Color32::from_rgb(230, 233, 239),
                bg_tertiary: Color32::from_rgb(220, 224, 232),
                fg_primary: Color32::from_rgb(76, 79, 105),
                fg_secondary: Color32::from_rgb(92, 95, 119),
                fg_muted: Color32::from_rgb(140, 143, 161),
                accent: Color32::from_rgb(30, 102, 245),
                accent_hover: Color32::from_rgb(114, 135, 253),
                success: Color32::from_rgb(64, 160, 43),
                warning: Color32::from_rgb(223, 142, 29),
                error: Color32::from_rgb(210, 15, 57),
                info: Color32::from_rgb(4, 165, 229),
                border: Color32::from_rgb(204, 208, 218),
                border_hover: Color32::from_rgb(30, 102, 245),
                selection: Color32::from_rgba_unmultiplied(30, 102, 245, 90),
                highlight: Color32::from_rgba_unmultiplied(223, 142, 29, 40),
            },

            // ===== One Dark 系列 =====
            ThemePreset::OneDark => ThemeColors {
                bg_primary: Color32::from_rgb(40, 44, 52),
                bg_secondary: Color32::from_rgb(50, 56, 66),
                bg_tertiary: Color32::from_rgb(62, 68, 81),
                fg_primary: Color32::from_rgb(171, 178, 191),
                fg_secondary: Color32::from_rgb(152, 159, 172),
                fg_muted: Color32::from_rgb(92, 99, 112),
                accent: Color32::from_rgb(97, 175, 239),
                accent_hover: Color32::from_rgb(127, 195, 255),
                success: Color32::from_rgb(152, 195, 121),
                warning: Color32::from_rgb(229, 192, 123),
                error: Color32::from_rgb(224, 108, 117),
                info: Color32::from_rgb(86, 182, 194),
                border: Color32::from_rgb(62, 68, 81),
                border_hover: Color32::from_rgb(97, 175, 239),
                selection: Color32::from_rgba_unmultiplied(60, 100, 150, 120),
                highlight: Color32::from_rgba_unmultiplied(229, 192, 123, 40),
            },
            ThemePreset::OneDarkVivid => ThemeColors {
                bg_primary: Color32::from_rgb(40, 44, 52),
                bg_secondary: Color32::from_rgb(50, 56, 66),
                bg_tertiary: Color32::from_rgb(62, 68, 81),
                fg_primary: Color32::from_rgb(220, 223, 228),
                fg_secondary: Color32::from_rgb(171, 178, 191),
                fg_muted: Color32::from_rgb(92, 99, 112),
                accent: Color32::from_rgb(82, 139, 255),
                accent_hover: Color32::from_rgb(112, 159, 255),
                success: Color32::from_rgb(137, 202, 120),
                warning: Color32::from_rgb(239, 191, 107),
                error: Color32::from_rgb(239, 83, 80),
                info: Color32::from_rgb(42, 195, 222),
                border: Color32::from_rgb(62, 68, 81),
                border_hover: Color32::from_rgb(82, 139, 255),
                selection: Color32::from_rgba_unmultiplied(60, 100, 150, 120),
                highlight: Color32::from_rgba_unmultiplied(239, 191, 107, 40),
            },
            ThemePreset::OneLight => ThemeColors {
                bg_primary: Color32::from_rgb(250, 250, 250),
                bg_secondary: Color32::from_rgb(240, 240, 240),
                bg_tertiary: Color32::from_rgb(230, 230, 230),
                fg_primary: Color32::from_rgb(56, 58, 66),
                fg_secondary: Color32::from_rgb(80, 82, 90),
                fg_muted: Color32::from_rgb(160, 161, 167),
                accent: Color32::from_rgb(64, 120, 242),
                accent_hover: Color32::from_rgb(84, 140, 255),
                success: Color32::from_rgb(80, 161, 79),
                warning: Color32::from_rgb(152, 104, 1),
                error: Color32::from_rgb(228, 86, 73),
                info: Color32::from_rgb(1, 132, 188),
                border: Color32::from_rgb(218, 219, 220),
                border_hover: Color32::from_rgb(64, 120, 242),
                selection: Color32::from_rgba_unmultiplied(64, 120, 242, 90),
                highlight: Color32::from_rgba_unmultiplied(152, 104, 1, 40),
            },

            // ===== Gruvbox 系列 =====
            ThemePreset::GruvboxDark => ThemeColors {
                bg_primary: Color32::from_rgb(40, 40, 40),
                bg_secondary: Color32::from_rgb(60, 56, 54),
                bg_tertiary: Color32::from_rgb(80, 73, 69),
                fg_primary: Color32::from_rgb(235, 219, 178),
                fg_secondary: Color32::from_rgb(213, 196, 161),
                fg_muted: Color32::from_rgb(146, 131, 116),
                accent: Color32::from_rgb(131, 165, 152),
                accent_hover: Color32::from_rgb(142, 192, 124),
                success: Color32::from_rgb(184, 187, 38),
                warning: Color32::from_rgb(250, 189, 47),
                error: Color32::from_rgb(251, 73, 52),
                info: Color32::from_rgb(131, 165, 152),
                border: Color32::from_rgb(80, 73, 69),
                border_hover: Color32::from_rgb(131, 165, 152),
                selection: Color32::from_rgba_unmultiplied(80, 110, 100, 120),
                highlight: Color32::from_rgba_unmultiplied(250, 189, 47, 40),
            },
            ThemePreset::GruvboxLight => ThemeColors {
                bg_primary: Color32::from_rgb(251, 241, 199),
                bg_secondary: Color32::from_rgb(242, 229, 188),
                bg_tertiary: Color32::from_rgb(235, 219, 178),
                fg_primary: Color32::from_rgb(60, 56, 54),
                fg_secondary: Color32::from_rgb(80, 73, 69),
                fg_muted: Color32::from_rgb(146, 131, 116),
                accent: Color32::from_rgb(69, 133, 136),
                accent_hover: Color32::from_rgb(7, 102, 120),
                success: Color32::from_rgb(121, 116, 14),
                warning: Color32::from_rgb(181, 118, 20),
                error: Color32::from_rgb(157, 0, 6),
                info: Color32::from_rgb(69, 133, 136),
                border: Color32::from_rgb(213, 196, 161),
                border_hover: Color32::from_rgb(69, 133, 136),
                selection: Color32::from_rgba_unmultiplied(69, 133, 136, 90),
                highlight: Color32::from_rgba_unmultiplied(181, 118, 20, 40),
            },

            // ===== Dracula =====
            ThemePreset::Dracula => ThemeColors {
                bg_primary: Color32::from_rgb(40, 42, 54),
                bg_secondary: Color32::from_rgb(68, 71, 90),
                bg_tertiary: Color32::from_rgb(98, 114, 164),
                fg_primary: Color32::from_rgb(248, 248, 242),
                fg_secondary: Color32::from_rgb(230, 230, 225),
                fg_muted: Color32::from_rgb(98, 114, 164),
                accent: Color32::from_rgb(189, 147, 249),
                accent_hover: Color32::from_rgb(255, 121, 198),
                success: Color32::from_rgb(80, 250, 123),
                warning: Color32::from_rgb(241, 250, 140),
                error: Color32::from_rgb(255, 85, 85),
                info: Color32::from_rgb(139, 233, 253),
                border: Color32::from_rgb(68, 71, 90),
                border_hover: Color32::from_rgb(189, 147, 249),
                selection: Color32::from_rgba_unmultiplied(100, 80, 140, 120),
                highlight: Color32::from_rgba_unmultiplied(241, 250, 140, 40),
            },

            // ===== Nord =====
            ThemePreset::Nord => ThemeColors {
                bg_primary: Color32::from_rgb(46, 52, 64),
                bg_secondary: Color32::from_rgb(59, 66, 82),
                bg_tertiary: Color32::from_rgb(67, 76, 94),
                fg_primary: Color32::from_rgb(236, 239, 244),
                fg_secondary: Color32::from_rgb(229, 233, 240),
                fg_muted: Color32::from_rgb(76, 86, 106),
                accent: Color32::from_rgb(136, 192, 208),
                accent_hover: Color32::from_rgb(129, 161, 193),
                success: Color32::from_rgb(163, 190, 140),
                warning: Color32::from_rgb(235, 203, 139),
                error: Color32::from_rgb(191, 97, 106),
                info: Color32::from_rgb(136, 192, 208),
                border: Color32::from_rgb(67, 76, 94),
                border_hover: Color32::from_rgb(136, 192, 208),
                selection: Color32::from_rgba_unmultiplied(70, 110, 130, 120),
                highlight: Color32::from_rgba_unmultiplied(235, 203, 139, 40),
            },

            // ===== Solarized 系列 =====
            ThemePreset::SolarizedDark => ThemeColors {
                bg_primary: Color32::from_rgb(0, 43, 54),
                bg_secondary: Color32::from_rgb(7, 54, 66),
                bg_tertiary: Color32::from_rgb(88, 110, 117),
                fg_primary: Color32::from_rgb(131, 148, 150),
                fg_secondary: Color32::from_rgb(147, 161, 161),
                fg_muted: Color32::from_rgb(88, 110, 117),
                accent: Color32::from_rgb(38, 139, 210),
                accent_hover: Color32::from_rgb(108, 113, 196),
                success: Color32::from_rgb(133, 153, 0),
                warning: Color32::from_rgb(181, 137, 0),
                error: Color32::from_rgb(220, 50, 47),
                info: Color32::from_rgb(42, 161, 152),
                border: Color32::from_rgb(88, 110, 117),
                border_hover: Color32::from_rgb(38, 139, 210),
                selection: Color32::from_rgba_unmultiplied(40, 90, 130, 120),
                highlight: Color32::from_rgba_unmultiplied(181, 137, 0, 40),
            },
            ThemePreset::SolarizedLight => ThemeColors {
                bg_primary: Color32::from_rgb(253, 246, 227),
                bg_secondary: Color32::from_rgb(238, 232, 213),
                bg_tertiary: Color32::from_rgb(147, 161, 161),
                fg_primary: Color32::from_rgb(101, 123, 131),
                fg_secondary: Color32::from_rgb(88, 110, 117),
                fg_muted: Color32::from_rgb(147, 161, 161),
                accent: Color32::from_rgb(38, 139, 210),
                accent_hover: Color32::from_rgb(108, 113, 196),
                success: Color32::from_rgb(133, 153, 0),
                warning: Color32::from_rgb(181, 137, 0),
                error: Color32::from_rgb(220, 50, 47),
                info: Color32::from_rgb(42, 161, 152),
                border: Color32::from_rgb(147, 161, 161),
                border_hover: Color32::from_rgb(38, 139, 210),
                selection: Color32::from_rgba_unmultiplied(38, 139, 210, 90),
                highlight: Color32::from_rgba_unmultiplied(181, 137, 0, 40),
            },

            // ===== Monokai Pro =====
            ThemePreset::MonokaiPro => ThemeColors {
                bg_primary: Color32::from_rgb(45, 42, 46),
                bg_secondary: Color32::from_rgb(57, 53, 58),
                bg_tertiary: Color32::from_rgb(69, 65, 71),
                fg_primary: Color32::from_rgb(252, 252, 250),
                fg_secondary: Color32::from_rgb(200, 200, 198),
                fg_muted: Color32::from_rgb(114, 109, 118),
                accent: Color32::from_rgb(255, 216, 102),
                accent_hover: Color32::from_rgb(171, 157, 242),
                success: Color32::from_rgb(169, 220, 118),
                warning: Color32::from_rgb(255, 216, 102),
                error: Color32::from_rgb(255, 97, 136),
                info: Color32::from_rgb(120, 220, 232),
                border: Color32::from_rgb(69, 65, 71),
                border_hover: Color32::from_rgb(255, 216, 102),
                selection: Color32::from_rgba_unmultiplied(120, 100, 60, 120),
                highlight: Color32::from_rgba_unmultiplied(171, 157, 242, 40),
            },

            // ===== GitHub 系列 =====
            ThemePreset::GithubDark => ThemeColors {
                bg_primary: Color32::from_rgb(13, 17, 23),
                bg_secondary: Color32::from_rgb(22, 27, 34),
                bg_tertiary: Color32::from_rgb(33, 38, 45),
                fg_primary: Color32::from_rgb(230, 237, 243),
                fg_secondary: Color32::from_rgb(201, 209, 217),
                fg_muted: Color32::from_rgb(139, 148, 158),
                accent: Color32::from_rgb(88, 166, 255),
                accent_hover: Color32::from_rgb(121, 192, 255),
                success: Color32::from_rgb(63, 185, 80),
                warning: Color32::from_rgb(210, 153, 34),
                error: Color32::from_rgb(248, 81, 73),
                info: Color32::from_rgb(56, 139, 253),
                border: Color32::from_rgb(48, 54, 61),
                border_hover: Color32::from_rgb(88, 166, 255),
                selection: Color32::from_rgba_unmultiplied(50, 90, 140, 120),
                highlight: Color32::from_rgba_unmultiplied(210, 153, 34, 40),
            },
            ThemePreset::GithubLight => ThemeColors {
                bg_primary: Color32::from_rgb(255, 255, 255),
                bg_secondary: Color32::from_rgb(246, 248, 250),
                bg_tertiary: Color32::from_rgb(234, 238, 242),
                fg_primary: Color32::from_rgb(31, 35, 40),
                fg_secondary: Color32::from_rgb(87, 96, 106),
                fg_muted: Color32::from_rgb(139, 148, 158),
                accent: Color32::from_rgb(9, 105, 218),
                accent_hover: Color32::from_rgb(31, 136, 229),
                success: Color32::from_rgb(26, 127, 55),
                warning: Color32::from_rgb(154, 103, 0),
                error: Color32::from_rgb(207, 34, 46),
                info: Color32::from_rgb(9, 105, 218),
                border: Color32::from_rgb(208, 215, 222),
                border_hover: Color32::from_rgb(9, 105, 218),
                selection: Color32::from_rgba_unmultiplied(9, 105, 218, 90),
                highlight: Color32::from_rgba_unmultiplied(154, 103, 0, 40),
            },
        }
    }
}

/// 主题管理器
#[derive(Debug, Clone)]
pub struct ThemeManager {
    pub current: ThemePreset,
    pub colors: ThemeColors,
}

impl Default for ThemeManager {
    fn default() -> Self {
        let preset = ThemePreset::default();
        Self {
            current: preset,
            colors: preset.colors(),
        }
    }
}

impl ThemeManager {
    pub fn new(preset: ThemePreset) -> Self {
        Self {
            current: preset,
            colors: preset.colors(),
        }
    }

    pub fn set_theme(&mut self, preset: ThemePreset) {
        self.current = preset;
        self.colors = preset.colors();
    }

    /// 应用主题到 egui 上下文
    pub fn apply(&self, ctx: &egui::Context) {
        let colors = &self.colors;

        // 基于是否为暗色主题选择基础视觉样式
        let mut visuals = if self.current.is_dark() {
            Visuals::dark()
        } else {
            Visuals::light()
        };

        // 应用自定义颜色
        visuals.panel_fill = colors.bg_primary;
        visuals.window_fill = colors.bg_secondary;
        visuals.extreme_bg_color = colors.bg_tertiary;
        visuals.faint_bg_color = colors.bg_secondary;
        visuals.code_bg_color = colors.bg_tertiary;

        // 窗口样式 - 更现代的外观
        visuals.window_stroke = Stroke::new(1.0, colors.border);
        visuals.window_shadow = egui::epaint::Shadow {
            offset: [0, 4],
            blur: 12,
            spread: 0,
            color: Color32::from_black_alpha(60),
        };

        // 弹出菜单阴影
        visuals.popup_shadow = egui::epaint::Shadow {
            offset: [0, 2],
            blur: 8,
            spread: 0,
            color: Color32::from_black_alpha(40),
        };

        // 选择/高亮
        visuals.selection.bg_fill = colors.selection;
        visuals.selection.stroke = Stroke::new(1.0, colors.accent);

        // 超链接
        visuals.hyperlink_color = colors.accent;

        // 警告/错误颜色
        visuals.warn_fg_color = colors.warning;
        visuals.error_fg_color = colors.error;

        // 滚动条样式
        visuals.handle_shape = egui::style::HandleShape::Rect { aspect_ratio: 0.5 };

        // 控件样式
        // 非交互
        visuals.widgets.noninteractive.bg_fill = colors.bg_secondary;
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, colors.fg_secondary);
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(0.5, colors.border);
        visuals.widgets.noninteractive.corner_radius = CornerRadius::same(6);

        // 非激活
        visuals.widgets.inactive.bg_fill = colors.bg_secondary;
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, colors.fg_primary);
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, colors.border);
        visuals.widgets.inactive.corner_radius = CornerRadius::same(6);
        visuals.widgets.inactive.expansion = 0.0;

        // 悬停 - 轻微放大效果
        visuals.widgets.hovered.bg_fill = colors.bg_tertiary;
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.5, colors.fg_primary);
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, colors.accent);
        visuals.widgets.hovered.corner_radius = CornerRadius::same(6);
        visuals.widgets.hovered.expansion = 1.0;

        // 激活
        visuals.widgets.active.bg_fill = colors.accent;
        visuals.widgets.active.fg_stroke = Stroke::new(2.0, colors.bg_primary);
        visuals.widgets.active.bg_stroke = Stroke::new(1.0, colors.accent_hover);
        visuals.widgets.active.corner_radius = CornerRadius::same(6);
        visuals.widgets.active.expansion = 0.0;

        // 打开(下拉菜单等)
        visuals.widgets.open.bg_fill = colors.bg_tertiary;
        visuals.widgets.open.fg_stroke = Stroke::new(1.0, colors.fg_primary);
        visuals.widgets.open.bg_stroke = Stroke::new(1.0, colors.accent);
        visuals.widgets.open.corner_radius = CornerRadius::same(6);

        // 条纹表格背景
        visuals.striped = true;

        // 设置默认文字颜色（确保白天模式用深色字，黑夜模式用浅色字）
        visuals.override_text_color = Some(colors.fg_primary);

        ctx.set_visuals(visuals);

        // 应用样式
        let mut style = (*ctx.style()).clone();

        // 间距设置
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(14.0, 7.0);
        style.spacing.window_margin = egui::Margin::same(12);
        style.spacing.menu_margin = egui::Margin::same(8);
        style.spacing.indent = 18.0;

        // 滚动条设置
        style.spacing.scroll = egui::style::ScrollStyle {
            bar_width: 10.0,
            handle_min_length: 20.0,
            bar_inner_margin: 2.0,
            bar_outer_margin: 2.0,
            floating: true,
            ..Default::default()
        };

        // 交互设置
        style.interaction.selectable_labels = true;
        style.interaction.show_tooltips_only_when_still = false;

        // 动画设置
        style.animation_time = 0.1;

        ctx.set_style(style);
    }
}
