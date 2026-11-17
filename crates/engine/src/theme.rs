//! Markdown colours.
//!
//! Leaf's app-wide theme machinery is reduced to the palettes the markdown
//! engine paints with. There is no global theme state, no custom theme file,
//! and no UI or diff theme here.

use ratatui::style::Color;

/// Colours used by the markdown renderer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MarkdownTheme {
    pub search_highlight_bg: Color,
    pub search_match_bg: Color,
    pub code_gutter: Color,
    pub blockquote_marker: Color,
    pub list_level_1: Color,
    pub list_level_2: Color,
    pub list_level_3: Color,
    pub ordered_list: Color,
    pub table_border: Color,
    pub table_separator: Color,
    pub table_header: Color,
    pub table_cell: Color,
    pub heading_1: Color,
    pub heading_2: Color,
    pub heading_3: Color,
    pub heading_4: Color,
    pub heading_other: Color,
    pub heading_underline: Color,
    pub code_frame: Color,
    pub code_label: Color,
    pub inline_code_fg: Color,
    pub inline_code_bg: Color,
    pub rule: Color,
    pub link_icon: Color,
    pub link_text: Color,
    pub link_hover: Color,
    pub blockquote_text: Color,
    pub text: Color,
    pub strong_text: Color,
    pub latex_inline_fg: Color,
    pub latex_inline_bg: Color,
    pub latex_block_fg: Color,
    pub mermaid_keyword: Color,
    pub mermaid_arrow: Color,
    pub mermaid_label: Color,
    pub mermaid_block_fg: Color,
    pub mark_fg: Color,
    pub mark_bg: Color,
    pub task_checked: Color,
    pub task_unchecked: Color,
    pub alert_note: Color,
    pub alert_tip: Color,
    pub alert_important: Color,
    pub alert_warning: Color,
    pub alert_caution: Color,
    pub footnote_ref: Color,
    pub footnote_text: Color,
}

impl Default for MarkdownTheme {
    fn default() -> Self {
        OCEAN_DARK
    }
}

/// The four built-in markdown palettes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum ThemePreset {
    Arctic = 0,
    Forest = 1,
    #[default]
    OceanDark = 2,
    SolarizedDark = 3,
}

impl ThemePreset {
    /// The syntect theme leaf paired with this markdown palette.
    pub fn syntect_theme_name(self) -> &'static str {
        match self {
            Self::Arctic => "base16-ocean.light",
            Self::Forest => "InspiredGitHub",
            Self::OceanDark => "base16-ocean.dark",
            Self::SolarizedDark => "Solarized (dark)",
        }
    }

    /// Human-readable preset name.
    pub fn label(self) -> &'static str {
        match self {
            Self::Arctic => "Arctic",
            Self::Forest => "Forest",
            Self::OceanDark => "Ocean Dark",
            Self::SolarizedDark => "Solarized Dark",
        }
    }
}

pub const ARCTIC: MarkdownTheme = MarkdownTheme {
    search_highlight_bg: Color::Rgb(232, 223, 164),
    search_match_bg: Color::Rgb(255, 210, 70),
    code_gutter: Color::Rgb(132, 148, 164),
    blockquote_marker: Color::Rgb(124, 134, 184),
    list_level_1: Color::Rgb(48, 140, 98),
    list_level_2: Color::Rgb(90, 118, 188),
    list_level_3: Color::Rgb(128, 132, 148),
    ordered_list: Color::Rgb(48, 140, 98),
    table_border: Color::Rgb(138, 154, 172),
    table_separator: Color::Rgb(158, 170, 184),
    table_header: Color::Rgb(58, 108, 168),
    table_cell: Color::Rgb(58, 68, 78),
    heading_1: Color::Rgb(58, 108, 168),
    heading_2: Color::Rgb(48, 140, 98),
    heading_3: Color::Rgb(176, 128, 48),
    heading_4: Color::Rgb(58, 84, 110),
    heading_other: Color::Rgb(108, 116, 126),
    heading_underline: Color::Rgb(160, 176, 194),
    code_frame: Color::Rgb(132, 148, 164),
    code_label: Color::Rgb(92, 116, 140),
    inline_code_fg: Color::Rgb(170, 108, 76),
    inline_code_bg: Color::Rgb(232, 226, 222),
    rule: Color::Rgb(180, 192, 204),
    link_icon: Color::Rgb(62, 124, 188),
    link_text: Color::Rgb(62, 124, 188),
    link_hover: Color::Rgb(90, 160, 230),
    blockquote_text: Color::Rgb(114, 116, 158),
    text: Color::Rgb(58, 68, 78),
    strong_text: Color::Rgb(26, 32, 40),
    latex_inline_fg: Color::Rgb(128, 68, 148),
    latex_inline_bg: Color::Rgb(236, 226, 240),
    latex_block_fg: Color::Rgb(108, 58, 128),
    mermaid_keyword: Color::Rgb(0, 128, 128),
    mermaid_arrow: Color::Rgb(90, 120, 150),
    mermaid_label: Color::Rgb(28, 140, 120),
    mermaid_block_fg: Color::Rgb(68, 108, 118),
    mark_fg: Color::Rgb(58, 68, 78),
    mark_bg: Color::Rgb(255, 248, 140),
    task_checked: Color::Rgb(48, 140, 98),
    task_unchecked: Color::Rgb(158, 164, 172),
    alert_note: Color::Rgb(58, 108, 168),
    alert_tip: Color::Rgb(48, 140, 98),
    alert_important: Color::Rgb(128, 68, 148),
    alert_warning: Color::Rgb(176, 128, 48),
    alert_caution: Color::Rgb(188, 74, 74),
    footnote_ref: Color::Rgb(62, 124, 188),
    footnote_text: Color::Rgb(58, 68, 78),
};

pub const FOREST: MarkdownTheme = MarkdownTheme {
    search_highlight_bg: Color::Rgb(74, 78, 32),
    search_match_bg: Color::Rgb(148, 156, 60),
    code_gutter: Color::Rgb(50, 66, 60),
    blockquote_marker: Color::Rgb(98, 124, 118),
    list_level_1: Color::Rgb(120, 198, 148),
    list_level_2: Color::Rgb(127, 179, 255),
    list_level_3: Color::Rgb(184, 190, 170),
    ordered_list: Color::Rgb(120, 198, 148),
    table_border: Color::Rgb(74, 92, 82),
    table_separator: Color::Rgb(60, 76, 68),
    table_header: Color::Rgb(148, 204, 255),
    table_cell: Color::Rgb(212, 218, 212),
    heading_1: Color::Rgb(148, 204, 255),
    heading_2: Color::Rgb(120, 214, 170),
    heading_3: Color::Rgb(224, 190, 126),
    heading_4: Color::Rgb(184, 214, 196),
    heading_other: Color::Rgb(188, 194, 188),
    heading_underline: Color::Rgb(52, 68, 60),
    code_frame: Color::Rgb(50, 66, 60),
    code_label: Color::Rgb(128, 158, 142),
    inline_code_fg: Color::Rgb(224, 170, 132),
    inline_code_bg: Color::Rgb(42, 35, 31),
    rule: Color::Rgb(56, 70, 62),
    link_icon: Color::Rgb(102, 170, 255),
    link_text: Color::Rgb(110, 182, 255),
    link_hover: Color::Rgb(150, 210, 255),
    blockquote_text: Color::Rgb(160, 168, 188),
    text: Color::Rgb(212, 218, 212),
    strong_text: Color::Rgb(246, 248, 246),
    latex_inline_fg: Color::Rgb(192, 162, 218),
    latex_inline_bg: Color::Rgb(34, 28, 42),
    latex_block_fg: Color::Rgb(188, 158, 214),
    mermaid_keyword: Color::Rgb(80, 190, 170),
    mermaid_arrow: Color::Rgb(100, 170, 200),
    mermaid_label: Color::Rgb(90, 200, 160),
    mermaid_block_fg: Color::Rgb(150, 185, 170),
    mark_fg: Color::Rgb(212, 218, 212),
    mark_bg: Color::Rgb(72, 62, 18),
    task_checked: Color::Rgb(120, 198, 148),
    task_unchecked: Color::Rgb(90, 100, 92),
    alert_note: Color::Rgb(110, 182, 255),
    alert_tip: Color::Rgb(120, 198, 148),
    alert_important: Color::Rgb(192, 162, 218),
    alert_warning: Color::Rgb(224, 190, 126),
    alert_caution: Color::Rgb(224, 120, 120),
    footnote_ref: Color::Rgb(110, 182, 255),
    footnote_text: Color::Rgb(212, 218, 212),
};

pub const OCEAN_DARK: MarkdownTheme = MarkdownTheme {
    search_highlight_bg: Color::Rgb(72, 62, 16),
    search_match_bg: Color::Rgb(140, 120, 30),
    code_gutter: Color::Rgb(40, 48, 68),
    blockquote_marker: Color::Rgb(75, 80, 148),
    list_level_1: Color::Rgb(95, 200, 148),
    list_level_2: Color::Rgb(138, 155, 200),
    list_level_3: Color::Rgb(168, 168, 185),
    ordered_list: Color::Rgb(95, 200, 148),
    table_border: Color::Rgb(65, 75, 108),
    table_separator: Color::Rgb(55, 65, 95),
    table_header: Color::Rgb(140, 190, 255),
    table_cell: Color::Rgb(205, 208, 218),
    heading_1: Color::Rgb(140, 190, 255),
    heading_2: Color::Rgb(120, 210, 170),
    heading_3: Color::Rgb(210, 180, 120),
    heading_4: Color::Rgb(162, 192, 222),
    heading_other: Color::Rgb(180, 180, 190),
    heading_underline: Color::Rgb(40, 50, 75),
    code_frame: Color::Rgb(40, 48, 68),
    code_label: Color::Rgb(95, 110, 145),
    inline_code_fg: Color::Rgb(220, 150, 118),
    inline_code_bg: Color::Rgb(38, 32, 31),
    rule: Color::Rgb(48, 56, 76),
    link_icon: Color::Rgb(85, 148, 235),
    link_text: Color::Rgb(88, 152, 238),
    link_hover: Color::Rgb(130, 190, 255),
    blockquote_text: Color::Rgb(148, 148, 195),
    text: Color::Rgb(208, 210, 218),
    strong_text: Color::Rgb(245, 245, 255),
    latex_inline_fg: Color::Rgb(200, 160, 225),
    latex_inline_bg: Color::Rgb(38, 28, 48),
    latex_block_fg: Color::Rgb(195, 155, 220),
    mermaid_keyword: Color::Rgb(80, 200, 200),
    mermaid_arrow: Color::Rgb(120, 160, 200),
    mermaid_label: Color::Rgb(100, 210, 180),
    mermaid_block_fg: Color::Rgb(160, 190, 200),
    mark_fg: Color::Rgb(208, 210, 218),
    mark_bg: Color::Rgb(80, 68, 20),
    task_checked: Color::Rgb(95, 200, 148),
    task_unchecked: Color::Rgb(100, 100, 110),
    alert_note: Color::Rgb(88, 152, 238),
    alert_tip: Color::Rgb(95, 200, 148),
    alert_important: Color::Rgb(200, 160, 225),
    alert_warning: Color::Rgb(210, 180, 120),
    alert_caution: Color::Rgb(218, 95, 95),
    footnote_ref: Color::Rgb(88, 152, 238),
    footnote_text: Color::Rgb(208, 210, 218),
};

pub const SOLARIZED_DARK: MarkdownTheme = MarkdownTheme {
    search_highlight_bg: Color::Rgb(92, 74, 22),
    search_match_bg: Color::Rgb(180, 148, 40),
    code_gutter: Color::Rgb(88, 110, 117),
    blockquote_marker: Color::Rgb(108, 113, 196),
    list_level_1: Color::Rgb(133, 153, 0),
    list_level_2: Color::Rgb(38, 139, 210),
    list_level_3: Color::Rgb(147, 161, 161),
    ordered_list: Color::Rgb(133, 153, 0),
    table_border: Color::Rgb(88, 110, 117),
    table_separator: Color::Rgb(101, 123, 131),
    table_header: Color::Rgb(38, 139, 210),
    table_cell: Color::Rgb(147, 161, 161),
    heading_1: Color::Rgb(38, 139, 210),
    heading_2: Color::Rgb(42, 161, 152),
    heading_3: Color::Rgb(181, 137, 0),
    heading_4: Color::Rgb(147, 161, 161),
    heading_other: Color::Rgb(147, 161, 161),
    heading_underline: Color::Rgb(88, 110, 117),
    code_frame: Color::Rgb(88, 110, 117),
    code_label: Color::Rgb(131, 148, 150),
    inline_code_fg: Color::Rgb(190, 92, 48),
    inline_code_bg: Color::Rgb(22, 55, 60),
    rule: Color::Rgb(88, 110, 117),
    link_icon: Color::Rgb(38, 139, 210),
    link_text: Color::Rgb(38, 139, 210),
    link_hover: Color::Rgb(80, 175, 240),
    blockquote_text: Color::Rgb(131, 148, 150),
    text: Color::Rgb(147, 161, 161),
    strong_text: Color::Rgb(238, 232, 213),
    latex_inline_fg: Color::Rgb(108, 113, 196),
    latex_inline_bg: Color::Rgb(14, 48, 58),
    latex_block_fg: Color::Rgb(108, 113, 196),
    mermaid_keyword: Color::Rgb(42, 161, 152),
    mermaid_arrow: Color::Rgb(38, 139, 210),
    mermaid_label: Color::Rgb(88, 182, 172),
    mermaid_block_fg: Color::Rgb(131, 148, 150),
    mark_fg: Color::Rgb(147, 161, 161),
    mark_bg: Color::Rgb(62, 54, 8),
    task_checked: Color::Rgb(133, 153, 0),
    task_unchecked: Color::Rgb(88, 110, 117),
    alert_note: Color::Rgb(38, 139, 210),
    alert_tip: Color::Rgb(133, 153, 0),
    alert_important: Color::Rgb(108, 113, 196),
    alert_warning: Color::Rgb(181, 137, 0),
    alert_caution: Color::Rgb(220, 50, 47),
    footnote_ref: Color::Rgb(38, 139, 210),
    footnote_text: Color::Rgb(147, 161, 161),
};

/// Every built-in markdown preset, in leaf's order.
pub const PRESETS: [ThemePreset; 4] = [
    ThemePreset::Arctic,
    ThemePreset::Forest,
    ThemePreset::OceanDark,
    ThemePreset::SolarizedDark,
];

/// The palette for a preset.
pub fn theme_by_preset(preset: ThemePreset) -> &'static MarkdownTheme {
    match preset {
        ThemePreset::Arctic => &ARCTIC,
        ThemePreset::Forest => &FOREST,
        ThemePreset::OceanDark => &OCEAN_DARK,
        ThemePreset::SolarizedDark => &SOLARIZED_DARK,
    }
}

/// Parse a preset name the way leaf's theme picker did.
pub fn parse_theme_preset(name: &str) -> Option<ThemePreset> {
    match name {
        "arctic" => Some(ThemePreset::Arctic),
        "forest" => Some(ThemePreset::Forest),
        "ocean" | "ocean-dark" | "dark" => Some(ThemePreset::OceanDark),
        "solarized" | "solarized-dark" => Some(ThemePreset::SolarizedDark),
        _ => None,
    }
}
