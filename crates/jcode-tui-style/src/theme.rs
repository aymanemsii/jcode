use crate::color;
use crate::color::rgb;
use ratatui::prelude::*;
use std::sync::atomic::{AtomicU32, Ordering};

const DEFAULT_ACCENT_RGB: (u8, u8, u8) = (186, 139, 255);
const NO_CONFIGURED_ACCENT: u32 = 0x0100_0000;
const THEME_DEFAULT: u8 = 0;
const THEME_DARK: u8 = 1;
const THEME_HIGH_CONTRAST: u8 = 2;
const THEME_DRACULA: u8 = 3;
const THEME_TOKYONIGHT: u8 = 4;
const THEME_GRUVBOX: u8 = 5;
const THEME_NORD: u8 = 6;
const THEME_CATPPUCCIN: u8 = 7;
const THEME_CATPPUCCIN_MACCHIATO: u8 = 8;
const THEME_KANAGAWA: u8 = 9;
const THEME_EVERFOREST: u8 = 10;
const THEME_AYU: u8 = 11;
const THEME_ONE_DARK: u8 = 12;
const THEME_MATRIX: u8 = 13;
const THEME_VERCEL: u8 = 14;
const THEME_CURSOR: u8 = 15;
static CONFIGURED_ACCENT_RGB: AtomicU32 = AtomicU32::new(NO_CONFIGURED_ACCENT);
static THEME_ACCENT_RGB: AtomicU32 = AtomicU32::new(pack_rgb(DEFAULT_ACCENT_RGB));
static ACTIVE_USER_RGB: AtomicU32 = AtomicU32::new(pack_rgb(DEFAULT_PALETTE.user));
static ACTIVE_AI_RGB: AtomicU32 = AtomicU32::new(pack_rgb(DEFAULT_PALETTE.ai));
static ACTIVE_TOOL_RGB: AtomicU32 = AtomicU32::new(pack_rgb(DEFAULT_PALETTE.tool));
static ACTIVE_SYSTEM_MESSAGE_RGB: AtomicU32 =
    AtomicU32::new(pack_rgb(DEFAULT_PALETTE.system_message));
static ACTIVE_QUEUED_RGB: AtomicU32 = AtomicU32::new(pack_rgb(DEFAULT_PALETTE.queued));
static ACTIVE_ASAP_RGB: AtomicU32 = AtomicU32::new(pack_rgb(DEFAULT_PALETTE.asap));
static ACTIVE_PENDING_RGB: AtomicU32 = AtomicU32::new(pack_rgb(DEFAULT_PALETTE.pending));
static ACTIVE_BACKGROUND_RGB: AtomicU32 = AtomicU32::new(pack_rgb(DEFAULT_PALETTE.background));
static ACTIVE_FOREGROUND_RGB: AtomicU32 = AtomicU32::new(pack_rgb(DEFAULT_PALETTE.foreground));
static ACTIVE_MUTED_RGB: AtomicU32 = AtomicU32::new(pack_rgb(DEFAULT_PALETTE.muted));
static ACTIVE_BORDER_RGB: AtomicU32 = AtomicU32::new(pack_rgb(DEFAULT_PALETTE.border));
static ACTIVE_ACTIVE_BORDER_RGB: AtomicU32 = AtomicU32::new(pack_rgb(DEFAULT_PALETTE.active_border));
static ACTIVE_PANEL_RGB: AtomicU32 = AtomicU32::new(pack_rgb(DEFAULT_PALETTE.panel));
static ACTIVE_INPUT_RGB: AtomicU32 = AtomicU32::new(pack_rgb(DEFAULT_PALETTE.input));
static ACTIVE_SELECTION_RGB: AtomicU32 = AtomicU32::new(pack_rgb(DEFAULT_PALETTE.selection));
static ACTIVE_SUCCESS_RGB: AtomicU32 = AtomicU32::new(pack_rgb(DEFAULT_PALETTE.success));
static ACTIVE_WARNING_RGB: AtomicU32 = AtomicU32::new(pack_rgb(DEFAULT_PALETTE.warning));
static ACTIVE_ERROR_RGB: AtomicU32 = AtomicU32::new(pack_rgb(DEFAULT_PALETTE.error));

pub const BUILT_IN_THEME_NAMES: &[&str] = &[
    "default",
    "dark",
    "high-contrast",
    "dracula",
    "tokyonight",
    "gruvbox",
    "nord",
    "catppuccin",
    "catppuccin-macchiato",
    "kanagawa",
    "everforest",
    "ayu",
    "one-dark",
    "matrix",
    "vercel",
    "cursor",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThemePalette {
    pub accent: (u8, u8, u8),
    pub user: (u8, u8, u8),
    pub ai: (u8, u8, u8),
    pub tool: (u8, u8, u8),
    pub system_message: (u8, u8, u8),
    pub queued: (u8, u8, u8),
    pub asap: (u8, u8, u8),
    pub pending: (u8, u8, u8),
    pub background: (u8, u8, u8),
    pub foreground: (u8, u8, u8),
    pub muted: (u8, u8, u8),
    pub border: (u8, u8, u8),
    pub active_border: (u8, u8, u8),
    pub panel: (u8, u8, u8),
    pub input: (u8, u8, u8),
    pub selection: (u8, u8, u8),
    pub success: (u8, u8, u8),
    pub warning: (u8, u8, u8),
    pub error: (u8, u8, u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeSource {
    BuiltIn,
    Custom,
    DefaultFallback,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedThemePalette {
    pub name: String,
    pub source: ThemeSource,
    pub palette: ThemePalette,
    pub invalid_custom_color_count: usize,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CustomThemePalette<'a> {
    pub name: &'a str,
    pub accent: Option<&'a str>,
    pub user: Option<&'a str>,
    pub assistant: Option<&'a str>,
    pub tool: Option<&'a str>,
    pub system: Option<&'a str>,
    pub queued: Option<&'a str>,
    pub asap: Option<&'a str>,
    pub pending: Option<&'a str>,
    pub background: Option<&'a str>,
    pub foreground: Option<&'a str>,
    pub muted: Option<&'a str>,
    pub border: Option<&'a str>,
    pub active_border: Option<&'a str>,
    pub panel: Option<&'a str>,
    pub input: Option<&'a str>,
    pub selection: Option<&'a str>,
    pub success: Option<&'a str>,
    pub warning: Option<&'a str>,
    pub error: Option<&'a str>,
}

const DEFAULT_PALETTE: ThemePalette = ThemePalette {
    accent: DEFAULT_ACCENT_RGB,
    user: (138, 180, 248),
    ai: (129, 199, 132),
    tool: (120, 120, 120),
    system_message: (255, 170, 220),
    queued: (255, 193, 7),
    asap: (110, 210, 255),
    pending: (140, 140, 140),
    background: (0, 0, 0),
    foreground: (245, 245, 245),
    muted: (80, 80, 80),
    border: (80, 80, 80),
    active_border: DEFAULT_ACCENT_RGB,
    panel: (18, 18, 22),
    input: (24, 24, 28),
    selection: (48, 44, 86),
    success: (129, 199, 132),
    warning: (255, 193, 7),
    error: (255, 110, 130),
};

const DARK_PALETTE: ThemePalette = ThemePalette {
    accent: (125, 211, 252),
    user: (147, 197, 253),
    ai: (134, 239, 172),
    tool: (148, 163, 184),
    system_message: (244, 114, 182),
    queued: (251, 191, 36),
    asap: (34, 211, 238),
    pending: (156, 163, 175),
    background: (9, 9, 11),
    foreground: (229, 231, 235),
    muted: (113, 113, 122),
    border: (39, 39, 42),
    active_border: (125, 211, 252),
    panel: (17, 17, 19),
    input: (24, 24, 27),
    selection: (30, 58, 138),
    success: (52, 211, 153),
    warning: (251, 191, 36),
    error: (251, 113, 133),
};

const HIGH_CONTRAST_PALETTE: ThemePalette = ThemePalette {
    accent: (255, 255, 0),
    user: (0, 255, 255),
    ai: (0, 255, 0),
    tool: (255, 255, 255),
    system_message: (255, 0, 255),
    queued: (255, 255, 0),
    asap: (0, 255, 255),
    pending: (192, 192, 192),
    background: (0, 0, 0),
    foreground: (255, 255, 255),
    muted: (192, 192, 192),
    border: (255, 255, 255),
    active_border: (255, 255, 0),
    panel: (0, 0, 0),
    input: (24, 24, 24),
    selection: (0, 0, 180),
    success: (0, 255, 0),
    warning: (255, 255, 0),
    error: (255, 0, 0),
};

const DRACULA_PALETTE: ThemePalette = ThemePalette {
    accent: (189, 147, 249),
    user: (139, 233, 253),
    ai: (80, 250, 123),
    tool: (248, 248, 242),
    system_message: (255, 121, 198),
    queued: (241, 250, 140),
    asap: (255, 184, 108),
    pending: (98, 114, 164),
    background: (40, 42, 54),
    foreground: (248, 248, 242),
    muted: (98, 114, 164),
    border: (68, 71, 90),
    active_border: (189, 147, 249),
    panel: (33, 34, 44),
    input: (45, 47, 59),
    selection: (68, 71, 90),
    success: (80, 250, 123),
    warning: (241, 250, 140),
    error: (255, 85, 85),
};

const TOKYONIGHT_PALETTE: ThemePalette = ThemePalette {
    accent: (122, 162, 247),
    user: (125, 207, 255),
    ai: (158, 206, 106),
    tool: (169, 177, 214),
    system_message: (247, 118, 142),
    queued: (224, 175, 104),
    asap: (187, 154, 247),
    pending: (86, 95, 137),
    background: (26, 27, 38),
    foreground: (192, 202, 245),
    muted: (86, 95, 137),
    border: (65, 72, 104),
    active_border: (122, 162, 247),
    panel: (22, 22, 30),
    input: (31, 35, 53),
    selection: (40, 52, 89),
    success: (158, 206, 106),
    warning: (224, 175, 104),
    error: (247, 118, 142),
};

const GRUVBOX_PALETTE: ThemePalette = ThemePalette {
    accent: (250, 189, 47),
    user: (131, 165, 152),
    ai: (184, 187, 38),
    tool: (168, 153, 132),
    system_message: (251, 73, 52),
    queued: (254, 128, 25),
    asap: (142, 192, 124),
    pending: (146, 131, 116),
    background: (40, 40, 40),
    foreground: (235, 219, 178),
    muted: (146, 131, 116),
    border: (80, 73, 69),
    active_border: (250, 189, 47),
    panel: (50, 48, 47),
    input: (60, 56, 54),
    selection: (69, 71, 52),
    success: (184, 187, 38),
    warning: (250, 189, 47),
    error: (251, 73, 52),
};

const NORD_PALETTE: ThemePalette = ThemePalette {
    accent: (136, 192, 208),
    user: (129, 161, 193),
    ai: (163, 190, 140),
    tool: (216, 222, 233),
    system_message: (191, 97, 106),
    queued: (235, 203, 139),
    asap: (180, 142, 173),
    pending: (143, 188, 187),
    background: (46, 52, 64),
    foreground: (216, 222, 233),
    muted: (129, 161, 193),
    border: (67, 76, 94),
    active_border: (136, 192, 208),
    panel: (59, 66, 82),
    input: (67, 76, 94),
    selection: (76, 86, 106),
    success: (163, 190, 140),
    warning: (235, 203, 139),
    error: (191, 97, 106),
};

const CATPPUCCIN_PALETTE: ThemePalette = ThemePalette {
    accent: (137, 180, 250),
    user: (116, 199, 236),
    ai: (166, 227, 161),
    tool: (205, 214, 244),
    system_message: (243, 139, 168),
    queued: (249, 226, 175),
    asap: (203, 166, 247),
    pending: (147, 153, 178),
    background: (30, 30, 46),
    foreground: (205, 214, 244),
    muted: (147, 153, 178),
    border: (69, 71, 90),
    active_border: (137, 180, 250),
    panel: (24, 24, 37),
    input: (49, 50, 68),
    selection: (69, 71, 90),
    success: (166, 227, 161),
    warning: (249, 226, 175),
    error: (243, 139, 168),
};

const CATPPUCCIN_MACCHIATO_PALETTE: ThemePalette = ThemePalette {
    accent: (138, 173, 244),
    user: (125, 196, 228),
    ai: (166, 218, 149),
    tool: (202, 211, 245),
    system_message: (237, 135, 150),
    queued: (238, 212, 159),
    asap: (198, 160, 246),
    pending: (128, 135, 162),
    background: (36, 39, 58),
    foreground: (202, 211, 245),
    muted: (128, 135, 162),
    border: (73, 77, 100),
    active_border: (138, 173, 244),
    panel: (30, 32, 48),
    input: (54, 58, 79),
    selection: (73, 77, 100),
    success: (166, 218, 149),
    warning: (238, 212, 159),
    error: (237, 135, 150),
};

const KANAGAWA_PALETTE: ThemePalette = ThemePalette {
    accent: (126, 156, 216),
    user: (126, 174, 194),
    ai: (152, 187, 108),
    tool: (220, 215, 186),
    system_message: (228, 104, 118),
    queued: (230, 195, 132),
    asap: (210, 126, 153),
    pending: (114, 144, 154),
    background: (31, 31, 40),
    foreground: (220, 215, 186),
    muted: (114, 144, 154),
    border: (84, 84, 109),
    active_border: (126, 156, 216),
    panel: (22, 22, 29),
    input: (42, 42, 55),
    selection: (54, 73, 99),
    success: (152, 187, 108),
    warning: (230, 195, 132),
    error: (228, 104, 118),
};

const EVERFOREST_PALETTE: ThemePalette = ThemePalette {
    accent: (167, 192, 128),
    user: (127, 187, 179),
    ai: (131, 192, 146),
    tool: (211, 198, 170),
    system_message: (224, 108, 117),
    queued: (219, 188, 127),
    asap: (226, 150, 117),
    pending: (133, 146, 137),
    background: (45, 53, 59),
    foreground: (211, 198, 170),
    muted: (133, 146, 137),
    border: (79, 89, 84),
    active_border: (167, 192, 128),
    panel: (39, 46, 51),
    input: (52, 63, 68),
    selection: (65, 76, 65),
    success: (131, 192, 146),
    warning: (219, 188, 127),
    error: (224, 108, 117),
};

const AYU_PALETTE: ThemePalette = ThemePalette {
    accent: (255, 204, 102),
    user: (95, 180, 180),
    ai: (180, 214, 130),
    tool: (191, 200, 217),
    system_message: (255, 51, 102),
    queued: (255, 163, 26),
    asap: (57, 186, 230),
    pending: (130, 139, 153),
    background: (15, 20, 25),
    foreground: (191, 200, 217),
    muted: (130, 139, 153),
    border: (54, 62, 70),
    active_border: (255, 204, 102),
    panel: (19, 26, 33),
    input: (28, 36, 45),
    selection: (38, 52, 71),
    success: (180, 214, 130),
    warning: (255, 204, 102),
    error: (255, 51, 102),
};

const ONE_DARK_PALETTE: ThemePalette = ThemePalette {
    accent: (97, 175, 239),
    user: (86, 182, 194),
    ai: (152, 195, 121),
    tool: (171, 178, 191),
    system_message: (224, 108, 117),
    queued: (229, 192, 123),
    asap: (198, 120, 221),
    pending: (130, 137, 151),
    background: (40, 44, 52),
    foreground: (171, 178, 191),
    muted: (130, 137, 151),
    border: (76, 82, 99),
    active_border: (97, 175, 239),
    panel: (33, 37, 43),
    input: (49, 54, 63),
    selection: (62, 68, 81),
    success: (152, 195, 121),
    warning: (229, 192, 123),
    error: (224, 108, 117),
};

const MATRIX_PALETTE: ThemePalette = ThemePalette {
    accent: (0, 255, 65),
    user: (0, 220, 110),
    ai: (80, 255, 120),
    tool: (140, 200, 150),
    system_message: (255, 80, 80),
    queued: (190, 255, 90),
    asap: (0, 255, 190),
    pending: (80, 130, 90),
    background: (0, 8, 0),
    foreground: (180, 255, 190),
    muted: (80, 130, 90),
    border: (0, 90, 35),
    active_border: (0, 255, 65),
    panel: (0, 18, 6),
    input: (0, 28, 10),
    selection: (0, 70, 30),
    success: (80, 255, 120),
    warning: (190, 255, 90),
    error: (255, 80, 80),
};

const VERCEL_PALETTE: ThemePalette = ThemePalette {
    accent: (255, 255, 255),
    user: (0, 112, 243),
    ai: (80, 220, 160),
    tool: (170, 170, 170),
    system_message: (255, 64, 96),
    queued: (245, 166, 35),
    asap: (121, 40, 202),
    pending: (136, 136, 136),
    background: (0, 0, 0),
    foreground: (255, 255, 255),
    muted: (136, 136, 136),
    border: (51, 51, 51),
    active_border: (255, 255, 255),
    panel: (17, 17, 17),
    input: (24, 24, 24),
    selection: (38, 38, 38),
    success: (80, 220, 160),
    warning: (245, 166, 35),
    error: (255, 64, 96),
};

const CURSOR_PALETTE: ThemePalette = ThemePalette {
    accent: (0, 136, 255),
    user: (84, 180, 255),
    ai: (102, 217, 170),
    tool: (176, 185, 197),
    system_message: (255, 106, 136),
    queued: (255, 197, 92),
    asap: (0, 212, 255),
    pending: (132, 142, 156),
    background: (12, 14, 18),
    foreground: (232, 237, 243),
    muted: (132, 142, 156),
    border: (43, 48, 58),
    active_border: (0, 136, 255),
    panel: (18, 21, 27),
    input: (26, 30, 38),
    selection: (22, 51, 87),
    success: (102, 217, 170),
    warning: (255, 197, 92),
    error: (255, 106, 136),
};

pub fn user_color() -> Color {
    color_from_rgb(active_palette().user)
}
pub fn ai_color() -> Color {
    color_from_rgb(active_palette().ai)
}
pub fn tool_color() -> Color {
    color_from_rgb(active_palette().tool)
}
pub fn file_link_color() -> Color {
    rgb(180, 200, 255)
}
pub fn dim_color() -> Color {
    muted_color()
}
pub fn accent_color() -> Color {
    let packed = CONFIGURED_ACCENT_RGB.load(Ordering::Relaxed);
    let (r, g, b) = if packed == NO_CONFIGURED_ACCENT {
        unpack_rgb(THEME_ACCENT_RGB.load(Ordering::Relaxed))
    } else {
        unpack_rgb(packed)
    };
    rgb(r, g, b)
}
pub fn set_accent_color_from_config(value: Option<&str>) {
    set_accent_color_and_theme_from_config(value, None);
}
pub fn set_accent_color_and_theme_from_config(value: Option<&str>, theme: Option<&str>) {
    set_accent_color_theme_and_custom_themes_from_config(value, theme, &[]);
}
pub fn set_accent_color_theme_and_custom_themes_from_config(
    value: Option<&str>,
    theme: Option<&str>,
    custom_themes: &[CustomThemePalette<'_>],
) {
    let resolved = resolve_theme_palette(theme, custom_themes);
    let theme_accent = pack_rgb(resolved.palette.accent);
    let configured_accent = value.and_then(parse_hex_rgb).unwrap_or(NO_CONFIGURED_ACCENT);
    THEME_ACCENT_RGB.store(theme_accent, Ordering::Relaxed);
    install_active_palette(resolved.palette);
    CONFIGURED_ACCENT_RGB.store(configured_accent, Ordering::Relaxed);
}
pub fn theme_default_accent_rgb(theme: Option<&str>) -> (u8, u8, u8) {
    palette_for_theme_id(theme_id(theme)).accent
}
pub fn resolve_theme_palette(
    theme: Option<&str>,
    custom_themes: &[CustomThemePalette<'_>],
) -> ResolvedThemePalette {
    let trimmed = theme.map(str::trim).unwrap_or("");
    if trimmed.is_empty() {
        return ResolvedThemePalette {
            name: "default".to_string(),
            source: ThemeSource::DefaultFallback,
            palette: DEFAULT_PALETTE,
            invalid_custom_color_count: 0,
        };
    }

    if let Some(name) = canonical_theme_name(theme) {
        return ResolvedThemePalette {
            name: name.to_string(),
            source: ThemeSource::BuiltIn,
            palette: palette_for_theme_id(theme_id(Some(name))),
            invalid_custom_color_count: 0,
        };
    }

    if !trimmed.is_empty()
        && let Some(custom) = custom_themes
            .iter()
            .find(|custom| custom.name.trim().eq_ignore_ascii_case(trimmed))
    {
        let (palette, invalid_custom_color_count) = custom_theme_palette(custom);
        return ResolvedThemePalette {
            name: custom.name.trim().to_ascii_lowercase(),
            source: ThemeSource::Custom,
            palette,
            invalid_custom_color_count,
        };
    }

    ResolvedThemePalette {
        name: "default".to_string(),
        source: ThemeSource::DefaultFallback,
        palette: DEFAULT_PALETTE,
        invalid_custom_color_count: 0,
    }
}
pub fn canonical_theme_name(theme: Option<&str>) -> Option<&'static str> {
    match theme.map(str::trim).unwrap_or("").to_ascii_lowercase().as_str() {
        "" | "default" => Some("default"),
        "dark" => Some("dark"),
        "high-contrast" => Some("high-contrast"),
        "dracula" => Some("dracula"),
        "tokyonight" => Some("tokyonight"),
        "gruvbox" => Some("gruvbox"),
        "nord" => Some("nord"),
        "catppuccin" => Some("catppuccin"),
        "catppuccin-macchiato" => Some("catppuccin-macchiato"),
        "kanagawa" => Some("kanagawa"),
        "everforest" => Some("everforest"),
        "ayu" => Some("ayu"),
        "one-dark" => Some("one-dark"),
        "matrix" => Some("matrix"),
        "vercel" => Some("vercel"),
        "cursor" => Some("cursor"),
        _ => None,
    }
}
fn theme_id(theme: Option<&str>) -> u8 {
    match canonical_theme_name(theme).unwrap_or("default") {
        "dark" => THEME_DARK,
        "high-contrast" => THEME_HIGH_CONTRAST,
        "dracula" => THEME_DRACULA,
        "tokyonight" => THEME_TOKYONIGHT,
        "gruvbox" => THEME_GRUVBOX,
        "nord" => THEME_NORD,
        "catppuccin" => THEME_CATPPUCCIN,
        "catppuccin-macchiato" => THEME_CATPPUCCIN_MACCHIATO,
        "kanagawa" => THEME_KANAGAWA,
        "everforest" => THEME_EVERFOREST,
        "ayu" => THEME_AYU,
        "one-dark" => THEME_ONE_DARK,
        "matrix" => THEME_MATRIX,
        "vercel" => THEME_VERCEL,
        "cursor" => THEME_CURSOR,
        _ => THEME_DEFAULT,
    }
}
fn palette_for_theme_id(theme_id: u8) -> ThemePalette {
    match theme_id {
        THEME_DARK => DARK_PALETTE,
        THEME_HIGH_CONTRAST => HIGH_CONTRAST_PALETTE,
        THEME_DRACULA => DRACULA_PALETTE,
        THEME_TOKYONIGHT => TOKYONIGHT_PALETTE,
        THEME_GRUVBOX => GRUVBOX_PALETTE,
        THEME_NORD => NORD_PALETTE,
        THEME_CATPPUCCIN => CATPPUCCIN_PALETTE,
        THEME_CATPPUCCIN_MACCHIATO => CATPPUCCIN_MACCHIATO_PALETTE,
        THEME_KANAGAWA => KANAGAWA_PALETTE,
        THEME_EVERFOREST => EVERFOREST_PALETTE,
        THEME_AYU => AYU_PALETTE,
        THEME_ONE_DARK => ONE_DARK_PALETTE,
        THEME_MATRIX => MATRIX_PALETTE,
        THEME_VERCEL => VERCEL_PALETTE,
        THEME_CURSOR => CURSOR_PALETTE,
        _ => DEFAULT_PALETTE,
    }
}
fn active_palette() -> ThemePalette {
    ThemePalette {
        accent: unpack_rgb(THEME_ACCENT_RGB.load(Ordering::Relaxed)),
        user: unpack_rgb(ACTIVE_USER_RGB.load(Ordering::Relaxed)),
        ai: unpack_rgb(ACTIVE_AI_RGB.load(Ordering::Relaxed)),
        tool: unpack_rgb(ACTIVE_TOOL_RGB.load(Ordering::Relaxed)),
        system_message: unpack_rgb(ACTIVE_SYSTEM_MESSAGE_RGB.load(Ordering::Relaxed)),
        queued: unpack_rgb(ACTIVE_QUEUED_RGB.load(Ordering::Relaxed)),
        asap: unpack_rgb(ACTIVE_ASAP_RGB.load(Ordering::Relaxed)),
        pending: unpack_rgb(ACTIVE_PENDING_RGB.load(Ordering::Relaxed)),
        background: unpack_rgb(ACTIVE_BACKGROUND_RGB.load(Ordering::Relaxed)),
        foreground: unpack_rgb(ACTIVE_FOREGROUND_RGB.load(Ordering::Relaxed)),
        muted: unpack_rgb(ACTIVE_MUTED_RGB.load(Ordering::Relaxed)),
        border: unpack_rgb(ACTIVE_BORDER_RGB.load(Ordering::Relaxed)),
        active_border: unpack_rgb(ACTIVE_ACTIVE_BORDER_RGB.load(Ordering::Relaxed)),
        panel: unpack_rgb(ACTIVE_PANEL_RGB.load(Ordering::Relaxed)),
        input: unpack_rgb(ACTIVE_INPUT_RGB.load(Ordering::Relaxed)),
        selection: unpack_rgb(ACTIVE_SELECTION_RGB.load(Ordering::Relaxed)),
        success: unpack_rgb(ACTIVE_SUCCESS_RGB.load(Ordering::Relaxed)),
        warning: unpack_rgb(ACTIVE_WARNING_RGB.load(Ordering::Relaxed)),
        error: unpack_rgb(ACTIVE_ERROR_RGB.load(Ordering::Relaxed)),
    }
}
fn install_active_palette(palette: ThemePalette) {
    ACTIVE_USER_RGB.store(pack_rgb(palette.user), Ordering::Relaxed);
    ACTIVE_AI_RGB.store(pack_rgb(palette.ai), Ordering::Relaxed);
    ACTIVE_TOOL_RGB.store(pack_rgb(palette.tool), Ordering::Relaxed);
    ACTIVE_SYSTEM_MESSAGE_RGB.store(pack_rgb(palette.system_message), Ordering::Relaxed);
    ACTIVE_QUEUED_RGB.store(pack_rgb(palette.queued), Ordering::Relaxed);
    ACTIVE_ASAP_RGB.store(pack_rgb(palette.asap), Ordering::Relaxed);
    ACTIVE_PENDING_RGB.store(pack_rgb(palette.pending), Ordering::Relaxed);
    ACTIVE_BACKGROUND_RGB.store(pack_rgb(palette.background), Ordering::Relaxed);
    ACTIVE_FOREGROUND_RGB.store(pack_rgb(palette.foreground), Ordering::Relaxed);
    ACTIVE_MUTED_RGB.store(pack_rgb(palette.muted), Ordering::Relaxed);
    ACTIVE_BORDER_RGB.store(pack_rgb(palette.border), Ordering::Relaxed);
    ACTIVE_ACTIVE_BORDER_RGB.store(pack_rgb(palette.active_border), Ordering::Relaxed);
    ACTIVE_PANEL_RGB.store(pack_rgb(palette.panel), Ordering::Relaxed);
    ACTIVE_INPUT_RGB.store(pack_rgb(palette.input), Ordering::Relaxed);
    ACTIVE_SELECTION_RGB.store(pack_rgb(palette.selection), Ordering::Relaxed);
    ACTIVE_SUCCESS_RGB.store(pack_rgb(palette.success), Ordering::Relaxed);
    ACTIVE_WARNING_RGB.store(pack_rgb(palette.warning), Ordering::Relaxed);
    ACTIVE_ERROR_RGB.store(pack_rgb(palette.error), Ordering::Relaxed);
}
fn color_from_rgb((r, g, b): (u8, u8, u8)) -> Color {
    rgb(r, g, b)
}
const fn pack_rgb((r, g, b): (u8, u8, u8)) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | b as u32
}
pub fn color_to_hex((r, g, b): (u8, u8, u8)) -> String {
    format!("#{r:02X}{g:02X}{b:02X}")
}
fn parse_hex_rgb(value: &str) -> Option<u32> {
    let hex = value.trim().strip_prefix('#').unwrap_or(value.trim());
    if hex.len() != 6 || !hex.as_bytes().iter().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    u32::from_str_radix(hex, 16).ok()
}
fn custom_theme_palette(custom: &CustomThemePalette<'_>) -> (ThemePalette, usize) {
    let mut invalid = 0;
    let mut parse = |value: Option<&str>, fallback: (u8, u8, u8)| match value {
        Some(value) => match parse_hex_rgb(value).map(unpack_rgb) {
            Some(rgb) => rgb,
            None => {
                invalid += 1;
                fallback
            }
        },
        None => fallback,
    };

    (
        ThemePalette {
            accent: parse(custom.accent, DEFAULT_PALETTE.accent),
            user: parse(custom.user, DEFAULT_PALETTE.user),
            ai: parse(custom.assistant, DEFAULT_PALETTE.ai),
            tool: parse(custom.tool, DEFAULT_PALETTE.tool),
            system_message: parse(custom.system, DEFAULT_PALETTE.system_message),
            queued: parse(custom.queued, DEFAULT_PALETTE.queued),
            asap: parse(custom.asap, DEFAULT_PALETTE.asap),
            pending: parse(custom.pending, DEFAULT_PALETTE.pending),
            background: parse(custom.background, DEFAULT_PALETTE.background),
            foreground: parse(custom.foreground, DEFAULT_PALETTE.foreground),
            muted: parse(custom.muted, DEFAULT_PALETTE.muted),
            border: parse(custom.border, DEFAULT_PALETTE.border),
            active_border: parse(custom.active_border, DEFAULT_PALETTE.active_border),
            panel: parse(custom.panel, DEFAULT_PALETTE.panel),
            input: parse(custom.input, DEFAULT_PALETTE.input),
            selection: parse(custom.selection, DEFAULT_PALETTE.selection),
            success: parse(custom.success, DEFAULT_PALETTE.success),
            warning: parse(custom.warning, DEFAULT_PALETTE.warning),
            error: parse(custom.error, DEFAULT_PALETTE.error),
        },
        invalid,
    )
}
fn unpack_rgb(packed: u32) -> (u8, u8, u8) {
    (
        ((packed >> 16) & 0xff) as u8,
        ((packed >> 8) & 0xff) as u8,
        (packed & 0xff) as u8,
    )
}
pub fn system_message_color() -> Color {
    color_from_rgb(active_palette().system_message)
}
pub fn queued_color() -> Color {
    color_from_rgb(active_palette().queued)
}
pub fn asap_color() -> Color {
    color_from_rgb(active_palette().asap)
}
pub fn pending_color() -> Color {
    color_from_rgb(active_palette().pending)
}
pub fn background_color() -> Color {
    color_from_rgb(active_palette().background)
}
pub fn foreground_color() -> Color {
    color_from_rgb(active_palette().foreground)
}
pub fn muted_color() -> Color {
    color_from_rgb(active_palette().muted)
}
pub fn border_color() -> Color {
    color_from_rgb(active_palette().border)
}
pub fn active_border_color() -> Color {
    color_from_rgb(active_palette().active_border)
}
pub fn panel_color() -> Color {
    color_from_rgb(active_palette().panel)
}
pub fn input_color() -> Color {
    color_from_rgb(active_palette().input)
}
pub fn selection_color() -> Color {
    color_from_rgb(active_palette().selection)
}
pub fn success_color() -> Color {
    color_from_rgb(active_palette().success)
}
pub fn warning_color() -> Color {
    color_from_rgb(active_palette().warning)
}
pub fn error_color() -> Color {
    color_from_rgb(active_palette().error)
}
pub fn user_text() -> Color {
    rgb(245, 245, 255)
}
pub fn user_bg() -> Color {
    rgb(35, 40, 50)
}
pub fn ai_text() -> Color {
    rgb(220, 220, 215)
}
pub fn header_icon_color() -> Color {
    rgb(120, 210, 230)
}
pub fn header_name_color() -> Color {
    rgb(190, 210, 235)
}
pub fn header_session_color() -> Color {
    rgb(255, 255, 255)
}

// Spinner frames for animated status. Keep these single-cell because the fast
// spinner-only renderer patches one status cell between full TUI redraws. This
// sequence should read as a circular spin, not a grow/recede pulse.
const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Frame rate for slow, full-line "liveness" indicators that can only be
/// repainted by a full TUI redraw (e.g. the running-tool progress bar) when
/// decorative animations are disabled (Minimal tier, SSH, WSL, etc.). These
/// ride the ~1 Hz passive-liveness redraw, so advancing them faster would just
/// skip frames. Keep this slow so they read as alive without forcing more
/// expensive full-frame redraws.
pub const LIVENESS_INDICATOR_FPS: f32 = 1.5;

/// Frame rate for the low-cost single-cell circular spinner when decorative
/// animations are disabled. Unlike the full-line indicators above, this spinner
/// is patched by the cheap one-cell fast path between full redraws, so it can
/// animate at a smooth, responsive cadence (well above ~1 Hz) while still
/// staying very light on resources. Keep this in sync with the spinner-only
/// tick interval in the TUI run loop (`STATUS_SPINNER_ONLY_INTERVAL`, 80ms) so
/// each tick lands on exactly one new frame.
pub const LIVENESS_SPINNER_FPS: f32 = 12.5;

pub fn spinner_frame_index(elapsed: f32, fps: f32) -> usize {
    ((elapsed * fps) as usize) % SPINNER_FRAMES.len()
}

pub fn spinner_frame(elapsed: f32, fps: f32) -> &'static str {
    SPINNER_FRAMES[spinner_frame_index(elapsed, fps)]
}

pub fn activity_indicator_frame_index(
    elapsed: f32,
    fps: f32,
    enable_decorative_animations: bool,
) -> usize {
    if enable_decorative_animations {
        spinner_frame_index(elapsed, fps)
    } else {
        // Keep ticking at the smooth liveness rate instead of freezing on a
        // single frame. The single-cell fast path repaints this cheaply, so it
        // can animate well above ~1 Hz without a full-frame redraw.
        spinner_frame_index(elapsed, LIVENESS_SPINNER_FPS)
    }
}

pub fn activity_indicator(
    elapsed: f32,
    fps: f32,
    enable_decorative_animations: bool,
) -> &'static str {
    SPINNER_FRAMES[activity_indicator_frame_index(elapsed, fps, enable_decorative_animations)]
}

/// Convert HSL to RGB (h in 0-360, s and l in 0-1)
/// Chroma color based on position and time - creates flowing rainbow wave
/// Calculate chroma color with fade-in from dim during startup
/// Calculate smooth animated color for the header (single color, no position)
pub fn color_to_floats(c: Color, fallback: (f32, f32, f32)) -> (f32, f32, f32) {
    match c {
        Color::Rgb(r, g, b) => (r as f32, g as f32, b as f32),
        Color::Indexed(n) => {
            let (r, g, b) = color::indexed_to_rgb(n);
            (r as f32, g as f32, b as f32)
        }
        _ => fallback,
    }
}

pub fn blend_color(from: Color, to: Color, t: f32) -> Color {
    let (fr, fg, fb) = color_to_floats(from, (80.0, 80.0, 80.0));
    let (tr, tg, tb) = color_to_floats(to, (200.0, 200.0, 200.0));
    let r = fr + (tr - fr) * t;
    let g = fg + (tg - fg) * t;
    let b = fb + (tb - fb) * t;
    rgb(
        r.clamp(0.0, 255.0) as u8,
        g.clamp(0.0, 255.0) as u8,
        b.clamp(0.0, 255.0) as u8,
    )
}

pub fn rainbow_prompt_color(distance: usize) -> Color {
    // Rainbow colors (hue progression): red -> orange -> yellow -> green -> cyan -> blue -> violet
    const RAINBOW: [(u8, u8, u8); 7] = [
        (255, 80, 80),   // Red (softened)
        (255, 160, 80),  // Orange
        (255, 230, 80),  // Yellow
        (80, 220, 100),  // Green
        (80, 200, 220),  // Cyan
        (100, 140, 255), // Blue
        (180, 100, 255), // Violet
    ];

    // Gray target (dim_color())
    const GRAY: (u8, u8, u8) = (80, 80, 80);

    // Exponential decay factor - how quickly we fade to gray
    // decay = e^(-distance * rate), rate of ~0.4 gives nice falloff
    let decay = (-0.4 * distance as f32).exp();

    // Select rainbow color based on distance (cycle through)
    let rainbow_idx = distance.min(RAINBOW.len() - 1);
    let (r, g, b) = RAINBOW[rainbow_idx];

    // Blend rainbow color with gray based on decay
    // At distance 0: 100% rainbow, as distance increases: approaches gray
    let blend = |rainbow: u8, gray: u8| -> u8 {
        (rainbow as f32 * decay + gray as f32 * (1.0 - decay)) as u8
    };

    rgb(blend(r, GRAY.0), blend(g, GRAY.1), blend(b, GRAY.2))
}

pub fn prompt_entry_color(base: Color, t: f32) -> Color {
    let peak = rgb(255, 230, 120);
    // Quick pulse in/out over the animation window.
    let phase = if t < 0.5 { t * 2.0 } else { (1.0 - t) * 2.0 };
    blend_color(base, peak, phase.clamp(0.0, 1.0) * 0.7)
}

pub fn prompt_entry_bg_color(base: Color, t: f32) -> Color {
    let spotlight = rgb(58, 66, 82);
    let ease_in = 1.0 - (1.0 - t).powi(3);
    let ease_out = (1.0 - t).powi(2);
    let phase = (ease_in * ease_out * 1.65).clamp(0.0, 1.0);
    blend_color(base, spotlight, phase * 0.85)
}

pub fn prompt_entry_shimmer_color(base: Color, pos: f32, t: f32) -> Color {
    let travel = (t * 1.15).clamp(0.0, 1.0);
    let width = 0.18;
    let dist = (pos - travel).abs();
    let shimmer = (1.0 - (dist / width).clamp(0.0, 1.0)).powf(2.2);
    let pulse = (1.0 - t).powf(0.55);
    let highlight = rgb(255, 248, 210);
    blend_color(base, highlight, shimmer * pulse * 0.7)
}

/// Generate an animated color that pulses between two colors
pub fn animated_tool_color(elapsed: f32, enable_decorative_animations: bool) -> Color {
    if !enable_decorative_animations {
        return tool_color();
    }

    // Cycle period of ~1.5 seconds
    let t = (elapsed * 2.0).sin() * 0.5 + 0.5; // 0.0 to 1.0

    // Interpolate between cyan and purple
    let r = (80.0 + t * 106.0) as u8; // 80 -> 186
    let g = (200.0 - t * 61.0) as u8; // 200 -> 139
    let b = (220.0 + t * 35.0) as u8; // 220 -> 255

    rgb(r, g, b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard};

    static THEME_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn theme_test_guard() -> MutexGuard<'static, ()> {
        THEME_TEST_LOCK.lock().unwrap()
    }

    #[test]
    fn spinner_frames_are_circular_braille_sequence() {
        assert_eq!(
            SPINNER_FRAMES,
            &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]
        );
    }

    #[test]
    fn spinner_frame_wraps_at_sequence_length() {
        let fps = 10.0;
        assert_eq!(spinner_frame(0.0, fps), "⠋");
        assert_eq!(spinner_frame(0.9, fps), "⠏");
        assert_eq!(spinner_frame(1.0, fps), "⠋");
    }

    #[test]
    fn activity_indicator_still_advances_without_decorative_animations() {
        // With decorative animations disabled the single-cell spinner must keep
        // ticking instead of freezing on one frame.
        let first = activity_indicator(0.0, 12.5, false);
        let later = activity_indicator(1.0, 12.5, false);
        assert!(SPINNER_FRAMES.contains(&first));
        assert_ne!(
            first, later,
            "liveness spinner should advance within one second"
        );
    }

    #[test]
    fn parse_hex_rgb_accepts_hash_prefixed_and_bare_values() {
        assert_eq!(parse_hex_rgb("#1A2b3C").map(unpack_rgb), Some((26, 43, 60)));
        assert_eq!(parse_hex_rgb("1A2b3C").map(unpack_rgb), Some((26, 43, 60)));
    }

    #[test]
    fn parse_hex_rgb_rejects_invalid_values() {
        assert_eq!(parse_hex_rgb("#12345"), None);
        assert_eq!(parse_hex_rgb("#1234567"), None);
        assert_eq!(parse_hex_rgb("#12xx56"), None);
        assert_eq!(parse_hex_rgb(""), None);
    }

    #[test]
    fn configured_accent_falls_back_to_default_for_missing_or_invalid_values() {
        let _guard = theme_test_guard();

        set_accent_color_from_config(None);
        assert_eq!(accent_color(), rgb(186, 139, 255));

        set_accent_color_from_config(Some("not-a-color"));
        assert_eq!(accent_color(), rgb(186, 139, 255));
    }

    #[test]
    fn configured_accent_overrides_default_when_valid() {
        let _guard = theme_test_guard();

        set_accent_color_from_config(Some("#123456"));
        assert_eq!(accent_color(), rgb(18, 52, 86));
        set_accent_color_from_config(None);
    }

    #[test]
    fn theme_accent_is_used_when_configured_accent_is_missing_or_invalid() {
        let _guard = theme_test_guard();

        set_accent_color_and_theme_from_config(None, Some("dark"));
        assert_eq!(accent_color(), rgb(125, 211, 252));

        set_accent_color_and_theme_from_config(Some("not-a-color"), Some("high-contrast"));
        assert_eq!(accent_color(), rgb(255, 255, 0));

        set_accent_color_and_theme_from_config(None, Some("unknown"));
        assert_eq!(accent_color(), rgb(186, 139, 255));

        set_accent_color_from_config(None);
    }

    #[test]
    fn configured_accent_overrides_theme_accent_when_valid() {
        let _guard = theme_test_guard();

        set_accent_color_and_theme_from_config(Some("#123456"), Some("high-contrast"));
        assert_eq!(accent_color(), rgb(18, 52, 86));
        set_accent_color_from_config(None);
    }

    #[test]
    fn canonical_theme_name_accepts_supported_names_case_insensitively() {
        assert_eq!(canonical_theme_name(Some("default")), Some("default"));
        assert_eq!(canonical_theme_name(Some("Dark")), Some("dark"));
        assert_eq!(
            canonical_theme_name(Some("HIGH-CONTRAST")),
            Some("high-contrast")
        );
        assert_eq!(canonical_theme_name(Some("Dracula")), Some("dracula"));
        assert_eq!(canonical_theme_name(Some("TokyoNight")), Some("tokyonight"));
        assert_eq!(
            canonical_theme_name(Some("CATPPUCCIN-MACCHIATO")),
            Some("catppuccin-macchiato")
        );
        assert_eq!(canonical_theme_name(Some("One-Dark")), Some("one-dark"));
        for name in BUILT_IN_THEME_NAMES {
            assert_eq!(canonical_theme_name(Some(name)), Some(*name));
        }
        assert_eq!(canonical_theme_name(Some("unknown")), None);
    }

    #[test]
    fn famous_themes_apply_to_central_semantic_colors() {
        let _guard = theme_test_guard();

        set_accent_color_and_theme_from_config(None, Some("dracula"));
        assert_eq!(accent_color(), rgb(189, 147, 249));
        assert_eq!(user_color(), rgb(139, 233, 253));
        assert_eq!(ai_color(), rgb(80, 250, 123));
        assert_eq!(tool_color(), rgb(248, 248, 242));
        assert_eq!(system_message_color(), rgb(255, 121, 198));
        assert_eq!(queued_color(), rgb(241, 250, 140));
        assert_eq!(asap_color(), rgb(255, 184, 108));
        assert_eq!(pending_color(), rgb(98, 114, 164));

        set_accent_color_and_theme_from_config(None, Some("cursor"));
        assert_eq!(accent_color(), rgb(0, 136, 255));
        assert_eq!(user_color(), rgb(84, 180, 255));
        assert_eq!(ai_color(), rgb(102, 217, 170));
        assert_eq!(tool_color(), rgb(176, 185, 197));
        assert_eq!(system_message_color(), rgb(255, 106, 136));
        assert_eq!(queued_color(), rgb(255, 197, 92));
        assert_eq!(asap_color(), rgb(0, 212, 255));
        assert_eq!(pending_color(), rgb(132, 142, 156));

        set_accent_color_from_config(None);
    }

    #[test]
    fn default_theme_preserves_existing_semantic_colors() {
        let _guard = theme_test_guard();

        set_accent_color_and_theme_from_config(None, Some("default"));

        assert_eq!(accent_color(), rgb(186, 139, 255));
        assert_eq!(user_color(), rgb(138, 180, 248));
        assert_eq!(ai_color(), rgb(129, 199, 132));
        assert_eq!(tool_color(), rgb(120, 120, 120));
        assert_eq!(system_message_color(), rgb(255, 170, 220));
        assert_eq!(queued_color(), rgb(255, 193, 7));
        assert_eq!(asap_color(), rgb(110, 210, 255));
        assert_eq!(pending_color(), rgb(140, 140, 140));

        set_accent_color_from_config(None);
    }

    #[test]
    fn named_themes_apply_to_central_semantic_colors() {
        let _guard = theme_test_guard();

        set_accent_color_and_theme_from_config(None, Some("dark"));
        assert_eq!(accent_color(), rgb(125, 211, 252));
        assert_eq!(user_color(), rgb(147, 197, 253));
        assert_eq!(ai_color(), rgb(134, 239, 172));
        assert_eq!(tool_color(), rgb(148, 163, 184));
        assert_eq!(system_message_color(), rgb(244, 114, 182));
        assert_eq!(queued_color(), rgb(251, 191, 36));
        assert_eq!(asap_color(), rgb(34, 211, 238));
        assert_eq!(pending_color(), rgb(156, 163, 175));

        set_accent_color_and_theme_from_config(None, Some("high-contrast"));
        assert_eq!(accent_color(), rgb(255, 255, 0));
        assert_eq!(user_color(), rgb(0, 255, 255));
        assert_eq!(ai_color(), rgb(0, 255, 0));
        assert_eq!(tool_color(), rgb(255, 255, 255));
        assert_eq!(system_message_color(), rgb(255, 0, 255));
        assert_eq!(queued_color(), rgb(255, 255, 0));
        assert_eq!(asap_color(), rgb(0, 255, 255));
        assert_eq!(pending_color(), rgb(192, 192, 192));

        set_accent_color_from_config(None);
    }

    #[test]
    fn invalid_theme_uses_default_palette() {
        let _guard = theme_test_guard();

        set_accent_color_and_theme_from_config(None, Some("unknown"));

        assert_eq!(accent_color(), rgb(186, 139, 255));
        assert_eq!(user_color(), rgb(138, 180, 248));
        assert_eq!(ai_color(), rgb(129, 199, 132));
        assert_eq!(tool_color(), rgb(120, 120, 120));
        assert_eq!(system_message_color(), rgb(255, 170, 220));
        assert_eq!(queued_color(), rgb(255, 193, 7));
        assert_eq!(asap_color(), rgb(110, 210, 255));
        assert_eq!(pending_color(), rgb(140, 140, 140));

        set_accent_color_from_config(None);
    }

    #[test]
    fn configured_accent_only_overrides_accent_in_theme_palette() {
        let _guard = theme_test_guard();

        set_accent_color_and_theme_from_config(Some("#123456"), Some("dark"));

        assert_eq!(accent_color(), rgb(18, 52, 86));
        assert_eq!(user_color(), rgb(147, 197, 253));
        assert_eq!(ai_color(), rgb(134, 239, 172));
        assert_eq!(tool_color(), rgb(148, 163, 184));

        set_accent_color_from_config(None);
    }

    #[test]
    fn custom_theme_applies_optional_fields_with_default_fallbacks() {
        let _guard = theme_test_guard();
        let themes = [CustomThemePalette {
            name: "aymane",
            accent: Some("#8B5CF6"),
            user: Some("#7DD3FC"),
            assistant: Some("#C084FC"),
            tool: Some("#FBBF24"),
            system: Some("#94A3B8"),
            queued: Some("#38BDF8"),
            asap: Some("#F97316"),
            pending: None,
            background: Some("#09090B"),
            foreground: None,
            muted: Some("#71717A"),
            border: Some("#27272A"),
            active_border: Some("#8B5CF6"),
            panel: None,
            input: None,
            selection: Some("#312E81"),
            success: None,
            warning: None,
            error: None,
        }];

        set_accent_color_theme_and_custom_themes_from_config(None, Some("Aymane"), &themes);

        assert_eq!(accent_color(), rgb(139, 92, 246));
        assert_eq!(user_color(), rgb(125, 211, 252));
        assert_eq!(ai_color(), rgb(192, 132, 252));
        assert_eq!(tool_color(), rgb(251, 191, 36));
        assert_eq!(system_message_color(), rgb(148, 163, 184));
        assert_eq!(queued_color(), rgb(56, 189, 248));
        assert_eq!(asap_color(), rgb(249, 115, 22));
        assert_eq!(pending_color(), rgb(140, 140, 140));
        assert_eq!(background_color(), rgb(9, 9, 11));
        assert_eq!(muted_color(), rgb(113, 113, 122));
        assert_eq!(border_color(), rgb(39, 39, 42));
        assert_eq!(active_border_color(), rgb(139, 92, 246));
        assert_eq!(selection_color(), rgb(49, 46, 129));
        assert_eq!(panel_color(), rgb(18, 18, 22));

        set_accent_color_from_config(None);
    }

    #[test]
    fn custom_theme_invalid_colors_fall_back_field_by_field() {
        let themes = [CustomThemePalette {
            name: "broken",
            accent: Some("not-a-color"),
            user: Some("#7DD3FC"),
            assistant: Some("#C084FC"),
            tool: Some("#bad"),
            system: None,
            queued: None,
            asap: None,
            pending: None,
            background: Some("#09090B"),
            foreground: None,
            muted: Some("nope"),
            border: None,
            active_border: None,
            panel: None,
            input: None,
            selection: None,
            success: None,
            warning: None,
            error: Some("#FB7185"),
        }];

        let resolved = resolve_theme_palette(Some("broken"), &themes);

        assert_eq!(resolved.name, "broken");
        assert_eq!(resolved.source, ThemeSource::Custom);
        assert_eq!(resolved.invalid_custom_color_count, 3);
        assert_eq!(resolved.palette.accent, DEFAULT_PALETTE.accent);
        assert_eq!(resolved.palette.user, (125, 211, 252));
        assert_eq!(resolved.palette.ai, (192, 132, 252));
        assert_eq!(resolved.palette.tool, DEFAULT_PALETTE.tool);
        assert_eq!(resolved.palette.background, (9, 9, 11));
        assert_eq!(resolved.palette.muted, DEFAULT_PALETTE.muted);
        assert_eq!(resolved.palette.error, (251, 113, 133));
    }

    #[test]
    fn built_in_theme_names_are_reserved_over_custom_themes() {
        let themes = [CustomThemePalette {
            name: "dark",
            accent: Some("#8B5CF6"),
            user: None,
            assistant: None,
            tool: None,
            system: None,
            queued: None,
            asap: None,
            pending: None,
            ..Default::default()
        }];

        let resolved = resolve_theme_palette(Some("dark"), &themes);

        assert_eq!(resolved.name, "dark");
        assert_eq!(resolved.source, ThemeSource::BuiltIn);
        assert_eq!(resolved.palette.accent, DARK_PALETTE.accent);
    }

    #[test]
    fn configured_accent_overrides_custom_theme_accent_only() {
        let _guard = theme_test_guard();
        let themes = [CustomThemePalette {
            name: "aymane",
            accent: Some("#8B5CF6"),
            user: Some("#7DD3FC"),
            assistant: Some("#C084FC"),
            tool: None,
            system: None,
            queued: None,
            asap: None,
            pending: None,
            ..Default::default()
        }];

        set_accent_color_theme_and_custom_themes_from_config(
            Some("#123456"),
            Some("aymane"),
            &themes,
        );

        assert_eq!(accent_color(), rgb(18, 52, 86));
        assert_eq!(user_color(), rgb(125, 211, 252));
        assert_eq!(ai_color(), rgb(192, 132, 252));

        set_accent_color_from_config(None);
    }

    #[test]
    fn liveness_spinner_advances_smoothly_within_a_few_frames() {
        // The single-cell fast path patches one status cell per 80ms tick, so the
        // non-decorative liveness spinner should advance well faster than ~1 Hz
        // (it should not still read as frozen between consecutive fast-path ticks).
        let frame_at = |elapsed: f32| activity_indicator(elapsed, 12.5, false);
        // One 80ms fast-path tick should already move to the next frame.
        assert_ne!(
            frame_at(0.0),
            frame_at(0.08),
            "liveness spinner should advance every fast-path tick (80ms)"
        );
        // It must be meaningfully faster than the old ~1.5 Hz cadence.
        assert!(
            LIVENESS_SPINNER_FPS >= 8.0,
            "liveness spinner should animate at a smooth, responsive rate"
        );
    }
}
