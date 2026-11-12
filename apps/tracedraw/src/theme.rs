//! Visual tokens: the interface colours of the four themes (Options >
//! Customization > Appearance). Colours live here and nowhere else; the
//! theme in use is picked with [`set_theme`] and read by every token.

use egui::{Color32, Visuals};
use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};

/// The interface themes, lightest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Light,
    MediumLight,
    MediumDark,
    Dark,
    /// The desktop's high contrast colours (the classic black theme
    /// where the desktop has none).
    HighContrast,
}

impl Theme {
    pub const ALL: [Theme; 5] = [
        Theme::Light,
        Theme::MediumLight,
        Theme::MediumDark,
        Theme::Dark,
        Theme::HighContrast,
    ];

    /// The name kept in the preferences.
    pub fn id(self) -> &'static str {
        match self {
            Theme::Light => "light",
            Theme::MediumLight => "medium_light",
            Theme::MediumDark => "medium_dark",
            Theme::Dark => "dark",
            Theme::HighContrast => "high_contrast",
        }
    }

    /// The theme a preference names; Light for anything else.
    pub fn from_id(id: &str) -> Theme {
        Theme::ALL
            .into_iter()
            .find(|t| t.id() == id)
            .unwrap_or(Theme::Light)
    }

    /// The i18n key of its name.
    pub fn key(self) -> &'static str {
        match self {
            Theme::Light => "options.theme_light",
            Theme::MediumLight => "options.theme_medium_light",
            Theme::MediumDark => "options.theme_medium_dark",
            Theme::Dark => "options.theme_dark",
            Theme::HighContrast => "options.theme_high_contrast",
        }
    }

    /// Light text on dark panels.
    pub fn is_dark(self) -> bool {
        match self {
            Theme::MediumDark | Theme::Dark => true,
            Theme::HighContrast => contrast_colors().is_dark(),
            _ => false,
        }
    }

    /// The page under a tabbed dialog's tabs and its frame line in this
    /// theme: the colour of fields (white in the light themes), so the
    /// labels on it read in every theme (#904).
    pub fn dialog_page(self) -> (Color32, Color32) {
        (self.palette().field, self.palette().border)
    }

    fn palette(self) -> &'static Palette {
        match self {
            Theme::Light => &LIGHT,
            Theme::MediumLight => &MEDIUM_LIGHT,
            Theme::MediumDark => &MEDIUM_DARK,
            Theme::Dark => &DARK,
            Theme::HighContrast => contrast_palette(),
        }
    }
}

/// The high contrast colours: the desktop's when one of its high
/// contrast themes is on, else the classic black theme.
pub fn contrast_colors() -> crate::system_prefs::ContrastColors {
    crate::system_prefs::get()
        .high_contrast
        .unwrap_or(crate::system_prefs::ContrastColors::BLACK)
}

/// Text drawn on a hovered or picked row's highlight.
pub fn on_highlight() -> Color32 {
    if theme() == Theme::HighContrast {
        let [r, g, b] = contrast_colors().highlight_text;
        Color32::from_rgb(r, g, b)
    } else {
        Tokens::text()
    }
}

/// Links in panels (Hints and the like).
pub fn link_color() -> Color32 {
    match theme() {
        Theme::HighContrast => {
            let [r, g, b] = contrast_colors().hot_light;
            Color32::from_rgb(r, g, b)
        }
        t if t.is_dark() => rehue(Color32::from_rgb(0x4F, 0xD1, 0x7E)),
        _ => Tokens::accent(),
    }
}

// ----- the accent of the workspace in use (Arcano edition) ---------------

/// The program's own green, which every accent colour of the themes is
/// made around.
pub const GREEN: [u8; 3] = [0x1A, 0xA3, 0x50];

static ACCENT: AtomicU32 = AtomicU32::new(0x1A_A3_50);

/// The accent colour in use (the workspace's).
pub fn accent_rgb() -> [u8; 3] {
    let v = ACCENT.load(Ordering::Relaxed);
    [(v >> 16) as u8, (v >> 8) as u8, v as u8]
}

/// Use `rgb` as the accent from now on: the selected buttons, tools,
/// rows, menus, links and focus take it instead of the green. True when
/// it changed (the visuals must be made again).
pub fn set_accent(rgb: [u8; 3]) -> bool {
    let v = (rgb[0] as u32) << 16 | (rgb[1] as u32) << 8 | rgb[2] as u32;
    ACCENT.swap(v, Ordering::Relaxed) != v
}

fn to_hsl(c: [u8; 3]) -> (f32, f32, f32) {
    let [r, g, b] = c.map(|v| v as f32 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    let d = max - min;
    if d < 1e-6 {
        return (0.0, 0.0, l);
    }
    let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
    let h = if max == r {
        ((g - b) / d).rem_euclid(6.0)
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    } * 60.0;
    (h, s, l)
}

fn from_hsl(h: f32, s: f32, l: f32) -> [u8; 3] {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = h.rem_euclid(360.0) / 60.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r, g, b) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    [r, g, b].map(|v| ((v + m).clamp(0.0, 1.0) * 255.0).round() as u8)
}

/// A colour made around the green, made around the accent in use: its
/// hue turned and its saturation and lightness scaled as the accent's
/// differ from the green's (so the green itself becomes the accent and a
/// pale green tint a pale tint of the accent).
pub fn rehue(c: Color32) -> Color32 {
    let acc = accent_rgb();
    if acc == GREEN || theme() == Theme::HighContrast {
        return c;
    }
    let (gh, gs, gl) = to_hsl(GREEN);
    let (ah, as_, al) = to_hsl(acc);
    let (h, s, l) = to_hsl([c.r(), c.g(), c.b()]);
    let s = (s * as_ / gs.max(1e-3)).clamp(0.0, 1.0);
    let l = if l <= gl {
        l * al / gl.max(1e-3)
    } else {
        1.0 - (1.0 - l) * (1.0 - al) / (1.0 - gl).max(1e-3)
    };
    let [r, g, b] = from_hsl(h + (ah - gh), s, l.clamp(0.0, 1.0));
    Color32::from_rgba_unmultiplied(r, g, b, c.a())
}

/// The tokens made around the green.
const ACCENT_TOKENS: [&str; 5] = [
    "tool_active",
    "menu_hover",
    "menu_hover_line",
    "selected_row",
    "selected_row_line",
];

/// The high contrast palettes made so far, by the colours they were made
/// from: the system's colours can change while the editor runs (#1412),
/// and each set is made once (there are few).
static CONTRAST: std::sync::Mutex<Vec<(crate::system_prefs::ContrastColors, &'static Palette)>> =
    std::sync::Mutex::new(Vec::new());

/// The high contrast theme's palette, made from [`contrast_colors`].
fn contrast_palette() -> &'static Palette {
    let c = contrast_colors();
    let mut made = CONTRAST.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((_, p)) = made.iter().find(|(k, _)| *k == c) {
        return p;
    }
    let p: &'static Palette = Box::leak(Box::new(contrast_palette_of(c)));
    made.push((c, p));
    p
}

/// The colours of the high contrast palette seen last, so that the
/// interface's look is set again when the system's change.
static CONTRAST_SEEN: std::sync::Mutex<Option<crate::system_prefs::ContrastColors>> =
    std::sync::Mutex::new(None);

/// Whether the high contrast colours changed since the last call.
pub fn contrast_changed() -> bool {
    let c = contrast_colors();
    let mut seen = CONTRAST_SEEN.lock().unwrap_or_else(|e| e.into_inner());
    seen.replace(c).is_some_and(|before| before != c)
}

/// The high contrast palette of the colours `c`.
fn contrast_palette_of(c: crate::system_prefs::ContrastColors) -> Palette {
    let col = |v: [u8; 3]| Color32::from_rgb(v[0], v[1], v[2]);
    let (win, text, hi, face, btn_text, gray, hot) = (
        col(c.window),
        col(c.window_text),
        col(c.highlight),
        col(c.button_face),
        col(c.button_text),
        col(c.gray_text),
        col(c.hot_light),
    );
    Palette {
        panel: face,
        menu_bar: face,
        panel_dark: win,
        border: text,
        icon_off: gray,
        text: btn_text,
        text_dim: gray,
        page_shadow: gray,
        page_border: text,
        ruler_bg: face,
        tool_active: hi,
        tool_hover: hi.gamma_multiply(0.6),
        icon: btn_text,
        button: face,
        control_border: text,
        slider_rail: gray,
        field: win,
        menu: win,
        menu_gutter: win,
        menu_hover: hi,
        menu_hover_line: hi,
        menu_text_off: gray,
        title_bar: hi,
        window_border: hi,
        selected_row: hi,
        hovered_row: hi.gamma_multiply(0.5),
        selected_row_line: hot,
        control_fill: win,
        tab_strip: face,
        tab: face,
        tab_active: win,
        ruler_tick: text,
        ruler_label: text,
        ruler_marker: hot,
        desktop: win,
    }
}

/// The status bar's light / dark button: a light theme goes to Dark, a
/// dark one to Light.
pub fn toggled(t: Theme) -> Theme {
    if t.is_dark() {
        Theme::Light
    } else {
        Theme::Dark
    }
}

/// The theme in use, as an index into [`Theme::ALL`]. Tests run on
/// threads of their own and each keeps its own theme (Light until it sets
/// another), as with the button sizes: a test that switches the theme
/// never changes the colours, or the pointer bitmaps kept by theme,
/// another test is checking.
#[cfg(not(test))]
static CURRENT: AtomicU8 = AtomicU8::new(0);
#[cfg(test)]
thread_local! {
    static CURRENT: AtomicU8 = const { AtomicU8::new(0) };
}

/// Use `theme` for the interface from now on (the tokens and
/// [`visuals`]).
pub fn set_theme(theme: Theme) {
    let i = Theme::ALL.iter().position(|t| *t == theme).unwrap_or(0);
    #[cfg(not(test))]
    CURRENT.store(i as u8, Ordering::Relaxed);
    #[cfg(test)]
    CURRENT.with(|c| c.store(i as u8, Ordering::Relaxed));
}

static SHADOWS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

/// Customization > Appearance > Shadows under menus and dialogs.
pub fn set_shadows(on: bool) {
    SHADOWS.store(on, Ordering::Relaxed);
}

/// Menus and dialogs cast shadows.
pub fn shadows() -> bool {
    SHADOWS.load(Ordering::Relaxed)
}

/// `s`, or no shadow when shadows are off.
pub fn shadow(s: egui::epaint::Shadow) -> egui::epaint::Shadow {
    if shadows() {
        s
    } else {
        egui::epaint::Shadow::NONE
    }
}

/// The theme in use.
pub fn theme() -> Theme {
    #[cfg(not(test))]
    let i = CURRENT.load(Ordering::Relaxed);
    #[cfg(test)]
    let i = CURRENT.with(|c| c.load(Ordering::Relaxed));
    Theme::ALL.get(i as usize).copied().unwrap_or(Theme::Light)
}

const fn rgb(v: u32) -> Color32 {
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

/// One theme's interface colours.
struct Palette {
    panel: Color32,
    menu_bar: Color32,
    panel_dark: Color32,
    border: Color32,
    icon_off: Color32,
    text: Color32,
    text_dim: Color32,
    page_shadow: Color32,
    page_border: Color32,
    ruler_bg: Color32,
    tool_active: Color32,
    tool_hover: Color32,
    icon: Color32,
    button: Color32,
    control_border: Color32,
    slider_rail: Color32,
    field: Color32,
    menu: Color32,
    menu_gutter: Color32,
    menu_hover: Color32,
    menu_hover_line: Color32,
    menu_text_off: Color32,
    title_bar: Color32,
    window_border: Color32,
    selected_row: Color32,
    hovered_row: Color32,
    selected_row_line: Color32,
    control_fill: Color32,
    tab_strip: Color32,
    tab: Color32,
    tab_active: Color32,
    ruler_tick: Color32,
    ruler_label: Color32,
    ruler_marker: Color32,
    /// The desktop around the page, unless the user picked a colour.
    desktop: Color32,
}

const LIGHT: Palette = Palette {
    panel: rgb(0xF7F7F8),
    menu_bar: rgb(0xEEEFF1),
    panel_dark: rgb(0xEEEFF1),
    border: rgb(0xE2E3E6),
    icon_off: rgb(0xBEC0C5),
    text: rgb(0x1E1E1E),
    text_dim: rgb(0x6B6E75),
    page_shadow: rgb(0xCDCFD4),
    page_border: rgb(0xC6C8CD),
    ruler_bg: rgb(0xF7F7F8),
    tool_active: rgb(0xD5F2DF),
    tool_hover: rgb(0xE8E9EC),
    icon: rgb(0x2B2D31),
    button: rgb(0xEDEEF0),
    control_border: rgb(0xD4D6DA),
    slider_rail: rgb(0xD4D6DA),
    field: rgb(0xEEEFF1),
    menu: rgb(0xFFFFFF),
    menu_gutter: rgb(0xFFFFFF),
    menu_hover: rgb(0xE3F5E9),
    menu_hover_line: rgb(0xE3F5E9),
    menu_text_off: rgb(0xB6B8BD),
    title_bar: rgb(0xF7F7F8),
    window_border: rgb(0xD4D6DA),
    selected_row: rgb(0xD5F2DF),
    hovered_row: rgb(0xEEF0F3),
    selected_row_line: rgb(0xD5F2DF),
    control_fill: rgb(0xF1F2F4),
    tab_strip: rgb(0xEEEFF1),
    tab: rgb(0xEEEFF1),
    tab_active: rgb(0xFFFFFF),
    ruler_tick: rgb(0x9B9EA4),
    ruler_label: rgb(0x7A7D84),
    ruler_marker: rgb(0x1D1E21),
    desktop: rgb(0xE6E7EA),
};

const MEDIUM_LIGHT: Palette = Palette {
    panel: rgb(0xE6E7EA),
    menu_bar: rgb(0xDCDDE1),
    panel_dark: rgb(0xDCDDE1),
    border: rgb(0xCFD1D5),
    icon_off: rgb(0xAAADB3),
    text: rgb(0x1E1E1E),
    text_dim: rgb(0x60636A),
    page_shadow: rgb(0xB9BCC2),
    page_border: rgb(0xB3B6BC),
    ruler_bg: rgb(0xE6E7EA),
    tool_active: rgb(0xC6EBD3),
    tool_hover: rgb(0xD6D8DC),
    icon: rgb(0x2B2D31),
    button: rgb(0xDCDDE1),
    control_border: rgb(0xC2C5CA),
    slider_rail: rgb(0xC2C5CA),
    field: rgb(0xD8DADE),
    menu: rgb(0xF4F5F6),
    menu_gutter: rgb(0xF4F5F6),
    menu_hover: rgb(0xD6F0DF),
    menu_hover_line: rgb(0xD6F0DF),
    menu_text_off: rgb(0xA5A8AE),
    title_bar: rgb(0xE6E7EA),
    window_border: rgb(0xC2C5CA),
    selected_row: rgb(0xC6EBD3),
    hovered_row: rgb(0xDDDFE3),
    selected_row_line: rgb(0xC6EBD3),
    control_fill: rgb(0xDEE0E3),
    tab_strip: rgb(0xDCDDE1),
    tab: rgb(0xDCDDE1),
    tab_active: rgb(0xF4F5F6),
    ruler_tick: rgb(0x8C8F95),
    ruler_label: rgb(0x6E7178),
    ruler_marker: rgb(0x1D1E21),
    desktop: rgb(0xD3D5D9),
};

const MEDIUM_DARK: Palette = Palette {
    panel: rgb(0x3A3B3F),
    menu_bar: rgb(0x303135),
    panel_dark: rgb(0x333438),
    border: rgb(0x48494E),
    icon_off: rgb(0x6C6E74),
    text: rgb(0xEDEDED),
    text_dim: rgb(0xA8ABB1),
    page_shadow: rgb(0x232427),
    page_border: rgb(0x26272A),
    ruler_bg: rgb(0x333438),
    tool_active: rgb(0x2E7A4B),
    tool_hover: rgb(0x47484D),
    icon: rgb(0xE2E3E6),
    button: rgb(0x46474C),
    control_border: rgb(0x55565C),
    slider_rail: rgb(0x55565C),
    field: rgb(0x2E2F33),
    menu: rgb(0x3A3B3F),
    menu_gutter: rgb(0x3A3B3F),
    menu_hover: rgb(0x2E7A4B),
    menu_hover_line: rgb(0x2E7A4B),
    menu_text_off: rgb(0x74767C),
    title_bar: rgb(0x333438),
    window_border: rgb(0x505157),
    selected_row: rgb(0x2A6A43),
    hovered_row: rgb(0x44454A),
    selected_row_line: rgb(0x2A6A43),
    control_fill: rgb(0x414246),
    tab_strip: rgb(0x2E2F33),
    tab: rgb(0x3A3B3F),
    tab_active: rgb(0x46474C),
    ruler_tick: rgb(0x7B7E84),
    ruler_label: rgb(0xA8ABB1),
    ruler_marker: rgb(0xEDEEF0),
    desktop: rgb(0x28292C),
};

const DARK: Palette = Palette {
    panel: rgb(0x2E2F33),
    menu_bar: rgb(0x26272B),
    panel_dark: rgb(0x28292D),
    border: rgb(0x3B3C41),
    icon_off: rgb(0x5C5E63),
    text: rgb(0xE8E8E8),
    text_dim: rgb(0x9DA0A6),
    page_shadow: rgb(0x101113),
    page_border: rgb(0x141518),
    ruler_bg: rgb(0x28292D),
    tool_active: rgb(0x276F44),
    tool_hover: rgb(0x3C3D42),
    icon: rgb(0xD9DADD),
    button: rgb(0x3C3D42),
    control_border: rgb(0x48494F),
    slider_rail: rgb(0x48494F),
    field: rgb(0x222327),
    menu: rgb(0x2E2F33),
    menu_gutter: rgb(0x2E2F33),
    menu_hover: rgb(0x276F44),
    menu_hover_line: rgb(0x276F44),
    menu_text_off: rgb(0x66686D),
    title_bar: rgb(0x28292D),
    window_border: rgb(0x48494F),
    selected_row: rgb(0x22563A),
    hovered_row: rgb(0x37383D),
    selected_row_line: rgb(0x22563A),
    control_fill: rgb(0x35363A),
    tab_strip: rgb(0x26272B),
    tab: rgb(0x2E2F33),
    tab_active: rgb(0x3C3D42),
    ruler_tick: rgb(0x6B6E74),
    ruler_label: rgb(0x9DA0A6),
    ruler_marker: rgb(0xE8E9EB),
    desktop: rgb(0x1C1D20),
};

pub struct Tokens;

macro_rules! tokens {
    ($($(#[$doc:meta])* $name:ident),* $(,)?) => {
        impl Tokens {
            $(
                $(#[$doc])*
                pub fn $name() -> Color32 {
                    let c = theme().palette().$name;
                    if ACCENT_TOKENS.contains(&stringify!($name)) {
                        rehue(c)
                    } else {
                        c
                    }
                }
            )*
        }
    };
}

tokens!(
    /// Toolbars, dockers, palettes, the status bar and dialog bodies.
    panel,
    /// The menu bar.
    menu_bar,
    panel_dark,
    /// Light lines: panel separators, frames of lists and docker parts.
    border,
    /// Icons of commands that cannot run now.
    icon_off,
    text,
    text_dim,
    /// The page's drop shadow, offset right and down behind the page.
    page_shadow,
    page_border,
    ruler_bg,
    tool_active,
    tool_hover,
    icon,
    /// Push buttons.
    button,
    /// The border of buttons, check boxes and fields.
    control_border,
    /// A slider's groove.
    slider_rail,
    /// Fields, lists and other places to type or pick in (white when light).
    field,
    /// The body of an open menu.
    menu,
    /// The strip of a menu behind its icons and check marks.
    menu_gutter,
    /// The box behind a hovered menu row, and its frame.
    menu_hover,
    menu_hover_line,
    /// Menu rows that cannot run now.
    menu_text_off,
    /// Dialogs' title bars.
    title_bar,
    /// The frame of dialog windows.
    window_border,
    /// The picked row of a list, a hovered row, and the picked row's frame.
    selected_row,
    hovered_row,
    selected_row_line,
    /// Lists' and spin buttons' fill.
    control_fill,
    /// The document tab strip, its tabs and the tab in front.
    tab_strip,
    tab,
    tab_active,
    /// Ruler ticks, numbers and the pointer's marker.
    ruler_tick,
    ruler_label,
    ruler_marker,
    desktop,
);

impl Tokens {
    /// The program's green: the accent is [`Tokens::accent`], the
    /// workspace's colour made from it.
    pub const ACCENT: Color32 = Color32::from_rgb(0x1A, 0xA3, 0x50);

    /// The accent in use: the workspace's colour (the green at first),
    /// the same in every theme.
    pub fn accent() -> Color32 {
        rehue(Tokens::ACCENT)
    }
    pub const PAGE: Color32 = Color32::WHITE;
    /// Screen offset of the page shadow, pixels.
    pub const PAGE_SHADOW_OFFSET: egui::Vec2 = egui::vec2(6.0, 4.0);
    pub const HANDLE: Color32 = Color32::from_rgb(0x1E, 0x1E, 0x1E);
    pub const SELECTION: Color32 = Color32::from_rgb(0x00, 0x78, 0xD7);
    /// The outline that shows where a dragged selection would go.
    pub const DRAG_OUTLINE: Color32 = Color32::from_rgb(0x1E, 0x64, 0xFF);

    pub const RULER: f32 = 18.0;

    /// The side of a toolbox button at the toolbox's button size (the
    /// bars' menu > Customize): 32 px at Medium.
    pub fn tool_button() -> f32 {
        toolbox_size().metrics().button
    }

    /// The side of a toolbox icon's box (its 24 unit grid) in a button
    /// of the full side: 24 px at Medium.
    pub fn tool_icon() -> f32 {
        toolbox_size().metrics().icon
    }

    /// The side of a standard toolbar button at the default button size:
    /// 32 px at Medium.
    pub fn bar_button() -> f32 {
        bar_size().metrics().button
    }

    /// The side of a standard toolbar icon's box: 24 px at Medium, 28
    /// and 34 px at Large and Extra Large (smaller than the toolbox's,
    /// see `ButtonMetrics`).
    pub fn bar_icon() -> f32 {
        bar_size().metrics().bar_icon
    }

    /// The side of a property bar button at the default button size:
    /// 28 px at Medium.
    pub fn prop_button() -> f32 {
        bar_size().metrics().prop_button
    }

    /// The side of a property bar icon's box: 22 px at Medium, 24 and
    /// 28 px at Large and Extra Large.
    pub fn prop_icon() -> f32 {
        bar_size().metrics().prop_icon
    }

    /// The fill and frame of an icon button (toolbox, standard toolbar,
    /// property bar toggles) in each of its states, or None when it is
    /// plain. Hovering tints it blue with a blue frame; a button that is
    /// on (the active tool, a toggle pressed in) stands out from the panel
    /// with a sharp accent frame, on the field colour when light and on
    /// the picked row's blue when dark; hovering it too deepens the fill.
    pub fn icon_button(hovered: bool, on: bool) -> Option<(Color32, egui::Stroke)> {
        icon_button_in(theme(), hovered, on)
    }
}

/// [`Tokens::icon_button`] under theme `t`.
fn icon_button_in(t: Theme, hovered: bool, on: bool) -> Option<(Color32, egui::Stroke)> {
    let p = t.palette();
    let stroke = |c: Color32| egui::Stroke::new(1.0, c);
    if t == Theme::HighContrast {
        let [r, g, b] = contrast_colors().hot_light;
        let hot = Color32::from_rgb(r, g, b);
        return match (hovered, on) {
            (false, false) => None,
            (true, false) => Some((p.hovered_row, stroke(p.selected_row))),
            (false, true) => Some((p.selected_row, egui::Stroke::new(2.0, hot))),
            (true, true) => Some((p.hovered_row, egui::Stroke::new(2.0, hot))),
        };
    }
    let dark = t.is_dark();
    // The frame of a button that is on: the accent, a lighter blue
    // where the accent would not read on a dark panel.
    let on_line = if dark {
        Color32::from_rgb(0x5C, 0xB2, 0xFF)
    } else {
        Tokens::ACCENT
    };
    let hover_fill = if dark {
        p.panel.lerp_to_gamma(p.selected_row, 0.6)
    } else {
        p.selected_row
    };
    match (hovered, on) {
        (false, false) => None,
        (true, false) => Some((hover_fill, stroke(p.selected_row_line))),
        (false, true) if dark => Some((p.selected_row, stroke(on_line))),
        (false, true) => Some((p.field, stroke(on_line))),
        (true, true) if dark => Some((
            p.selected_row.lerp_to_gamma(Color32::WHITE, 0.15),
            stroke(on_line),
        )),
        (true, true) => Some((p.selected_row, stroke(on_line))),
    }
}

/// Paint an icon button's background and frame for its state (see
/// [`Tokens::icon_button`]) under the icon.
pub fn paint_icon_button(painter: &egui::Painter, rect: egui::Rect, hovered: bool, on: bool) {
    if let Some((fill, line)) = Tokens::icon_button(hovered, on) {
        painter.rect_filled(rect, 2.0, fill);
        painter.rect_stroke(rect, 2.0, line, egui::StrokeKind::Inside);
    }
}

/// The icon box for a button of side `side` whose full size is `button`
/// with an icon box `icon`: the full icon in a full button, shrunk in the
/// same proportion (to whole pixels) in a button squeezed to fit, as the
/// docked toolbox's column does on a short window.
pub fn icon_in(side: f32, button: f32, icon: f32) -> f32 {
    if side >= button || button <= 0.0 {
        icon
    } else {
        (side * icon / button).round().max(1.0)
    }
}

/// The default button size and the toolbox's, as an index into
/// `ButtonSize::ALL` plus one (0: not set yet, Medium). Tests run on
/// threads of their own and each keeps its own sizes, so a test that sizes
/// the buttons never changes the layout another test is checking.
#[cfg(not(test))]
static BAR_SIZE: AtomicU8 = AtomicU8::new(0);
#[cfg(not(test))]
static TOOLBOX_SIZE: AtomicU8 = AtomicU8::new(0);
#[cfg(test)]
thread_local! {
    static BAR_SIZE: AtomicU8 = const { AtomicU8::new(0) };
    static TOOLBOX_SIZE: AtomicU8 = const { AtomicU8::new(0) };
}

fn size_of_index(n: u8) -> crate::settings::ButtonSize {
    use crate::settings::ButtonSize;
    match n {
        0 => ButtonSize::Medium,
        n => ButtonSize::ALL
            .get(usize::from(n) - 1)
            .copied()
            .unwrap_or(ButtonSize::Medium),
    }
}

#[cfg(not(test))]
fn stored_size(cell: &AtomicU8) -> crate::settings::ButtonSize {
    size_of_index(cell.load(Ordering::Relaxed))
}

#[cfg(test)]
fn stored_size(cell: &'static std::thread::LocalKey<AtomicU8>) -> crate::settings::ButtonSize {
    size_of_index(cell.with(|c| c.load(Ordering::Relaxed)))
}

#[cfg(not(test))]
fn store_size(cell: &AtomicU8, n: u8) {
    cell.store(n, Ordering::Relaxed);
}

#[cfg(test)]
fn store_size(cell: &'static std::thread::LocalKey<AtomicU8>, n: u8) {
    cell.with(|c| c.store(n, Ordering::Relaxed));
}

fn size_index(size: crate::settings::ButtonSize) -> u8 {
    crate::settings::ButtonSize::ALL
        .iter()
        .position(|s| *s == size)
        .map_or(0, |i| i as u8 + 1)
}

/// The default button size the bars draw at now.
pub fn bar_size() -> crate::settings::ButtonSize {
    stored_size(&BAR_SIZE)
}

/// The button size the toolbox draws at now.
pub fn toolbox_size() -> crate::settings::ButtonSize {
    stored_size(&TOOLBOX_SIZE)
}

/// Held by the tests that set the button sizes in force to another size
/// than the default, or that count on them: the sizes are shared by the
/// whole process.
#[cfg(test)]
pub(crate) fn button_sizes_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Size the bars' buttons and icons from now on: `bars` for every bar,
/// `toolbox` for the toolbox (the bars' menu > Customize > Default Button
/// Size and Toolbox Toolbar > Button Size).
pub fn set_button_sizes(
    bars: crate::settings::ButtonSize,
    toolbox: Option<crate::settings::ButtonSize>,
) {
    store_size(&BAR_SIZE, size_index(bars));
    store_size(&TOOLBOX_SIZE, size_index(toolbox.unwrap_or(bars)));
}

pub fn visuals() -> Visuals {
    let mut v = if theme().is_dark() {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    // Indented groups have no guide line.
    v.indent_has_left_vline = false;
    if !shadows() {
        v.window_shadow = egui::epaint::Shadow::NONE;
        v.popup_shadow = egui::epaint::Shadow::NONE;
    }
    v.panel_fill = Tokens::panel();
    v.window_fill = Tokens::panel();
    v.extreme_bg_color = Tokens::field();
    v.faint_bg_color = Tokens::panel_dark();
    v.code_bg_color = Tokens::panel_dark();
    v.override_text_color = None;
    v.widgets.noninteractive.bg_fill = Tokens::panel();
    v.widgets.noninteractive.weak_bg_fill = Tokens::panel();
    v.widgets.noninteractive.fg_stroke.color = Tokens::text();
    v.widgets.noninteractive.bg_stroke.color = Tokens::border();
    // Framed controls look like the desktop's: check boxes, radio
    // buttons and fields in the field colour with a border, push buttons
    // a shade darker than the panel.
    v.widgets.inactive.bg_fill = Tokens::field();
    v.widgets.inactive.weak_bg_fill = Tokens::button();
    // Fields and buttons without frames at rest: a fill a shade off the
    // panel; a light frame on hover, the accent while pressed or typed in.
    v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, Color32::TRANSPARENT);
    v.widgets.inactive.fg_stroke.color = Tokens::text();
    v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, Tokens::control_border());
    v.widgets.active.bg_stroke = egui::Stroke::new(1.0, Tokens::accent());
    v.widgets.hovered.bg_fill = Tokens::tool_hover();
    v.widgets.hovered.weak_bg_fill = Tokens::tool_hover();
    v.widgets.hovered.fg_stroke.color = Tokens::text();
    v.widgets.active.bg_fill = Tokens::tool_active();
    v.widgets.active.weak_bg_fill = Tokens::tool_active();
    v.widgets.active.fg_stroke.color = Tokens::text();
    v.widgets.open.bg_fill = Tokens::tool_active();
    v.widgets.open.weak_bg_fill = Tokens::tool_active();
    v.widgets.open.fg_stroke.color = Tokens::text();
    v.selection.bg_fill = Tokens::tool_active();
    // Text on the picked fill: the accent on light themes, white on dark
    // ones (the accent would not read on the dark blue).
    v.selection.stroke.color = if theme().is_dark() {
        Color32::WHITE
    } else {
        Tokens::accent()
    };
    v.hyperlink_color = if theme().is_dark() {
        rehue(Color32::from_rgb(0x4F, 0xD1, 0x7E))
    } else {
        Tokens::accent()
    };
    if theme() == Theme::HighContrast {
        // Text on the highlight in the desktop's highlight text colour,
        // links and focus in its hot-track colour.
        let c = contrast_colors();
        let col = |v: [u8; 3]| Color32::from_rgb(v[0], v[1], v[2]);
        let on_hi = col(c.highlight_text);
        v.selection.stroke.color = on_hi;
        v.widgets.hovered.fg_stroke.color = on_hi;
        v.widgets.active.fg_stroke.color = on_hi;
        v.widgets.open.fg_stroke.color = on_hi;
        v.widgets.hovered.bg_stroke = egui::Stroke::new(2.0, col(c.hot_light));
        v.widgets.active.bg_stroke = egui::Stroke::new(2.0, col(c.hot_light));
        v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, col(c.window_text));
        v.hyperlink_color = col(c.hot_light);
        v.warn_fg_color = col(c.hot_light);
    }
    // Soft corners: controls 5 px, windows and menus 8 px.
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.corner_radius = 5.0.into();
    }
    v.window_corner_radius = 8.0.into();
    v.menu_corner_radius = 8.0.into();
    // Lines between parts drawn in the light border colour, never black.
    v.window_stroke = egui::Stroke::new(1.0, Tokens::window_border());
    v
}

/// The interface font, bundled: Inter (SIL Open Font License, see
/// `assets/fonts/Inter-OFL.txt`), with Selawik (`Selawik-OFL.txt`) behind it: regular for text, bold for headings and
/// emphasis. Scripts it lacks fall back to egui's fonts and the system's.
const UI_REGULAR: &[u8] = include_bytes!("../assets/fonts/Inter-Regular.ttf");
const UI_BOLD: &[u8] = include_bytes!("../assets/fonts/Inter-SemiBold.ttf");
/// The earlier interface font, kept behind Inter for the symbols it lacks.
const UI_FALLBACK: &[u8] = include_bytes!("../assets/fonts/Selawik-Regular.ttf");
const UI_FALLBACK_BOLD: &[u8] = include_bytes!("../assets/fonts/Selawik-Bold.ttf");

/// The font family name of the bold interface font.
pub const BOLD: &str = "ui-bold";

/// A bold interface font of the given size.
pub fn bold(size: f32) -> egui::FontId {
    egui::FontId::new(size, egui::FontFamily::Name(BOLD.into()))
}

/// Install the bundled interface fonts only (headless drawing).
#[cfg(test)]
pub fn install_fonts_bundled(ctx: &egui::Context) {
    ctx.set_fonts(bundled_font_definitions());
}

/// Install the interface fonts (see [`font_definitions`]).
pub fn install_fonts(ctx: &egui::Context) {
    ctx.set_fonts(font_definitions());
}

/// The full interface fonts ([`font_definitions`]) being read on a worker
/// thread while the window already draws with the bundled ones. Reading the
/// system fonts for Chinese, Japanese, Arabic, Hindi and Bengali copies
/// tens of megabytes, which the editor need not wait for when the
/// interface language is not one of those.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Default)]
pub struct FontLoad {
    handle: Option<std::thread::JoinHandle<egui::FontDefinitions>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl FontLoad {
    /// Read the fonts on a worker thread. Where no thread can be made the
    /// load is empty: [`FontLoad::finish`] then reads them on the caller.
    pub fn start() -> Self {
        FontLoad {
            handle: std::thread::Builder::new()
                .name("tracedraw-fonts".into())
                .spawn(font_definitions)
                .map_err(|e| log::warn!("font thread: {e}"))
                .ok(),
        }
    }

    /// Whether the fonts are read (or there is no thread to wait for).
    pub fn is_finished(&self) -> bool {
        self.handle.as_ref().is_none_or(|h| h.is_finished())
    }

    /// A load of a thread of the test's own.
    #[cfg(test)]
    pub fn from_thread(handle: std::thread::JoinHandle<egui::FontDefinitions>) -> Self {
        FontLoad {
            handle: Some(handle),
        }
    }

    /// Whether a load is still under way.
    #[cfg(test)]
    pub fn pending(&self) -> bool {
        self.handle.is_some()
    }

    /// The fonts, waiting for the thread if it is not done (read on the
    /// caller when there was none).
    pub fn finish(&mut self) -> egui::FontDefinitions {
        self.handle
            .take()
            .and_then(|h| h.join().ok())
            .unwrap_or_else(font_definitions)
    }

    /// Install the fonts once they are read; true while they are still
    /// being read (ask for a repaint a little later and call again).
    pub fn poll(&mut self, ctx: &egui::Context) -> bool {
        if self.handle.is_none() {
            return false;
        }
        if !self.is_finished() {
            return true;
        }
        ctx.set_fonts(self.finish());
        false
    }
}

/// The fonts of a window that opens at once: the bundled ones now and
/// the system ones read in the background, or all of them before it
/// opens when the interface language needs a system font (its texts
/// would show as boxes meanwhile). Call [`FontLoad::poll`] each frame.
#[cfg(not(target_arch = "wasm32"))]
pub fn install_fonts_in_background(ctx: &egui::Context) -> FontLoad {
    if needs_system_font(&crate::i18n::language()) {
        install_fonts(ctx);
        return FontLoad::default();
    }
    ctx.set_fonts(bundled_font_definitions());
    FontLoad::start()
}

/// The interface fonts: the bundled UI font first, then egui's own and
/// the system fonts covering the scripts of the supported UI languages
/// (CJK, Arabic, Devanagari, Bengali), so menus render in every language
/// on machines that have such fonts. Reads the system font list, which
/// can take a moment: the splash screen builds these on a worker thread.
pub fn font_definitions() -> egui::FontDefinitions {
    let mut defs = bundled_font_definitions();
    for key in fallback_fonts(&mut defs) {
        for family in [
            egui::FontFamily::Proportional,
            egui::FontFamily::Monospace,
            egui::FontFamily::Name(BOLD.into()),
        ] {
            if let Some(list) = defs.families.get_mut(&family) {
                list.push(key.clone());
            }
        }
    }
    defs
}

/// The interface fonts without the system ones: the bundled UI font
/// (Latin, Greek, Cyrillic) and egui's own, ready at once.
pub fn bundled_font_definitions() -> egui::FontDefinitions {
    let mut defs = egui::FontDefinitions::default();
    defs.font_data.insert(
        "ui-regular".into(),
        std::sync::Arc::new(egui::FontData::from_static(UI_REGULAR)),
    );
    defs.font_data.insert(
        "ui-bold".into(),
        std::sync::Arc::new(egui::FontData::from_static(UI_BOLD)),
    );
    defs.font_data.insert(
        "ui-fallback".into(),
        std::sync::Arc::new(egui::FontData::from_static(UI_FALLBACK)),
    );
    defs.font_data.insert(
        "ui-fallback-bold".into(),
        std::sync::Arc::new(egui::FontData::from_static(UI_FALLBACK_BOLD)),
    );
    let defaults = defs
        .families
        .get(&egui::FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    if let Some(list) = defs.families.get_mut(&egui::FontFamily::Proportional) {
        list.insert(0, "ui-regular".into());
        list.insert(1, "ui-fallback".into());
    }
    let mut bold = vec!["ui-bold".to_string(), "ui-fallback-bold".to_string()];
    bold.extend(defaults);
    defs.families
        .insert(egui::FontFamily::Name(BOLD.into()), bold);
    defs
}

/// True when the interface language `code` needs a system font for its
/// script (the bundled one covers Latin, Greek and Cyrillic).
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub fn needs_system_font(code: &str) -> bool {
    matches!(code, "zh-CN" | "zh-TW" | "ja" | "ar" | "hi" | "bn" | "lo")
}

/// The family of the font folder (browser build) that covers the script
/// of the interface language `code`, when no bundled font does.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub fn ui_script_family(code: &str) -> Option<&'static str> {
    match code {
        "zh-CN" => Some("Noto Sans SC"),
        "zh-TW" => Some("Noto Sans TC"),
        "ja" => Some("Noto Sans JP"),
        "lo" => Some("Noto Sans Lao"),
        _ => None,
    }
}

/// A context with the interface fonts installed (headless tests draw
/// the real interface, which uses the bold family).
#[cfg(test)]
pub fn ui_context() -> egui::Context {
    let ctx = egui::Context::default();
    install_fonts(&ctx);
    ctx
}

/// Per script, the system font families the interface tries in order
/// (the first installed one wins): the scripts of the interface
/// languages and those people write their file and drawing names in,
/// which the bundled font (Latin, Greek, Cyrillic) does not cover.
const UI_SCRIPT_FONTS: &[&[&str]] = &[
    // Arabic (also covered by many Latin system fonts).
    &[
        "Noto Sans Arabic",
        "Noto Naskh Arabic",
        "Segoe UI",
        "Arial",
        "DejaVu Sans",
        "Geeza Pro",
    ],
    // Hebrew: file names, layer and page names typed in it.
    &[
        "Noto Sans Hebrew",
        "Arial Hebrew",
        "Segoe UI",
        "Arial",
        "DejaVu Sans",
        "Liberation Sans",
        "FreeSans",
    ],
    // Devanagari (Hindi).
    &[
        "Noto Sans Devanagari",
        "Nirmala UI",
        "Mangal",
        "Kohinoor Devanagari",
        "Lohit Devanagari",
    ],
    // Bengali.
    &[
        "Noto Sans Bengali",
        "Nirmala UI",
        "Vrinda",
        "Kohinoor Bangla",
        "Lohit Bengali",
    ],
    // Simplified Chinese and Japanese.
    &[
        "Noto Sans CJK SC",
        "Noto Sans SC",
        "Microsoft YaHei",
        "PingFang SC",
        "Source Han Sans SC",
        "WenQuanYi Micro Hei",
        "Noto Sans CJK JP",
    ],
    // Traditional Chinese.
    &[
        "Noto Sans CJK TC",
        "Noto Sans TC",
        "Microsoft JhengHei",
        "PingFang TC",
        "Source Han Sans TC",
    ],
    &[
        "Noto Sans CJK JP",
        "Noto Sans JP",
        "Meiryo",
        "Yu Gothic UI",
        "Hiragino Sans",
        "Source Han Sans",
    ],
    // Lao.
    &[
        "Noto Sans Lao",
        "Lao Sangam MN",
        "Leelawadee UI",
        "Saysettha OT",
    ],
    // Thai: file names, layer and page names typed in it (#2168).
    &[
        "Noto Sans Thai",
        "Noto Sans Thai UI",
        "Leelawadee UI",
        "Leelawadee",
        "Tahoma",
        "Thonburi",
        "Sukhumvit Set",
        "Loma",
        "Garuda",
        "Waree",
        "Norasi",
        "FreeSerif",
    ],
];

/// Add the system fonts covering the non-Latin UI languages to `defs`;
/// returns their keys.
fn fallback_fonts(defs: &mut egui::FontDefinitions) -> Vec<String> {
    let groups = UI_SCRIPT_FONTS;
    let fs = tracedraw_text::fonts();
    let mut added = Vec::new();
    for group in groups {
        let Some((name, data, index)) = fs.first_face_data(group) else {
            continue;
        };
        let key = format!("fallback-{name}");
        if defs.font_data.contains_key(&key) {
            continue;
        }
        let mut fd = egui::FontData::from_owned(data.as_ref().clone());
        fd.index = index;
        // A variable font draws at its default weight, Thin in some (the
        // browser build's Noto Sans CJK faces): set it to Regular.
        if fd.variation_axes().iter().any(|a| a.tag == *b"wght") {
            fd.tweak.coords = egui::epaint::text::VariationCoords::new([(b"wght", 400.0)]);
        }
        defs.font_data.insert(key.clone(), std::sync::Arc::new(fd));
        added.push(key);
    }
    if !added.is_empty() {
        log::info!("UI fallback fonts: {}", added.join(", "));
    }
    added
}

#[cfg(test)]
mod theme_tests {
    use super::*;

    /// A test that switches the theme leaves the theme of the tests
    /// running alongside alone: the pointer bitmaps are kept by theme, and
    /// a switch between drawing one and asking for it again drew it anew.
    #[test]
    fn a_theme_switched_in_another_test_is_not_this_ones() {
        assert_eq!(theme(), Theme::Light);
        std::thread::spawn(|| {
            for t in Theme::ALL {
                set_theme(t);
                assert_eq!(theme(), t);
            }
        })
        .join()
        .expect("the other test");
        assert_eq!(theme(), Theme::Light);
    }

    /// Names typed in Hebrew show in the interface (the file name field,
    /// the recent files, the tabs) when the system has a font for it.
    #[test]
    fn hebrew_names_have_an_interface_font() {
        let hebrew = UI_SCRIPT_FONTS
            .iter()
            .find(|g| g.first() == Some(&"Noto Sans Hebrew"))
            .expect("a group of Hebrew fonts");
        let has_one = tracedraw_text::fonts().first_face_data(hebrew).is_some();
        if has_one {
            let ctx = ui_context();
            let mut out = ctx.run_ui(Default::default(), |_| {});
            out.textures_delta.clear();
            let font = egui::FontId::proportional(14.0);
            for c in "שלום".chars() {
                assert!(ctx.fonts_mut(|f| f.has_glyph(&font, c)), "{c}");
            }
        }
    }

    /// Names typed in Thai show in the interface (the file name field,
    /// the file list, the recent files, the tabs) when the system has a
    /// font for it, consonants and the marks above and below them
    /// (#2168: a Thai file name showed as boxes).
    #[test]
    fn thai_names_have_an_interface_font() {
        // The fonts Windows, macOS and the common Linux installs bring
        // for Thai.
        let thai = [
            "Noto Sans Thai",
            "Leelawadee UI",
            "Tahoma",
            "Thonburi",
            "Loma",
        ];
        let has_one = tracedraw_text::fonts().first_face_data(&thai).is_some();
        if has_one {
            let ctx = ui_context();
            let mut out = ctx.run_ui(Default::default(), |_| {});
            out.textures_delta.clear();
            let font = egui::FontId::proportional(14.0);
            for c in "สวัสดี ไฟล์งานพิมพ์".chars().filter(|c| *c != ' ')
            {
                assert!(ctx.fonts_mut(|f| f.has_glyph(&font, c)), "{c}");
            }
        }
    }

    /// The fonts Windows (Leelawadee UI, Tahoma), macOS (Thonburi) and
    /// the common Linux installs (Noto Sans Thai, Loma) bring for Thai
    /// are asked for as one script's, so a Thai name does not depend on
    /// the font of another script happening to hold Thai letters.
    #[test]
    fn the_thai_interface_fonts_cover_every_system() {
        let thai = UI_SCRIPT_FONTS
            .iter()
            .find(|g| g.first() == Some(&"Noto Sans Thai"));
        for family in ["Leelawadee UI", "Tahoma", "Thonburi", "Loma"] {
            assert!(thai.is_some_and(|g| g.contains(&family)), "{family}");
        }
    }

    /// WCAG contrast ratio of two opaque colours.
    fn contrast(a: Color32, b: Color32) -> f32 {
        let lum = |c: Color32| {
            let ch = |v: u8| {
                let v = v as f32 / 255.0;
                if v <= 0.04045 {
                    v / 12.92
                } else {
                    ((v + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * ch(c.r()) + 0.7152 * ch(c.g()) + 0.0722 * ch(c.b())
        };
        let (x, y) = (lum(a), lum(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    /// Text drawn in a field (a list, the Glyphs panel's recent glyphs)
    /// reads in every theme (#660).
    #[test]
    fn text_contrasts_with_fields_in_every_theme() {
        for t in Theme::ALL {
            let p = t.palette();
            assert!(
                contrast(p.field, p.text) >= 4.5,
                "{t:?}: {:?} on {:?}",
                p.text,
                p.field
            );
        }
    }

    /// The labels on a tabbed dialog's page (Color Management, Document
    /// Properties) read in every theme: the page was white in the dark
    /// themes too, under their light text (#904).
    #[test]
    fn dialog_pages_carry_readable_text_in_every_theme() {
        for t in Theme::ALL {
            let (page, _) = t.dialog_page();
            let p = t.palette();
            assert!(
                contrast(page, p.text) >= 4.5,
                "{t:?}: {:?} on {page:?}",
                p.text
            );
            if t.is_dark() {
                assert_ne!(page, Color32::WHITE, "{t:?}");
            }
        }
    }

    #[test]
    fn themes_have_their_names_and_dark_ones_light_text() {
        let lum = |c: Color32| c.r() as u32 + c.g() as u32 + c.b() as u32;
        for t in Theme::ALL {
            assert_eq!(Theme::from_id(t.id()), t);
            let p = t.palette();
            // Text stands out from the panels in every theme.
            assert!(lum(p.text).abs_diff(lum(p.panel)) > 300, "{t:?}");
            assert_eq!(t.is_dark(), lum(p.text) > lum(p.panel), "{t:?}");
        }
        assert_eq!(Theme::from_id(""), Theme::Light);
        assert_eq!(Theme::from_id("purple"), Theme::Light);
        assert_eq!(toggled(Theme::MediumLight), Theme::Dark);
        assert_eq!(toggled(Theme::MediumDark), Theme::Light);
    }

    /// The browser build fetches a font for the interface languages no
    /// bundled font covers (#561), Lao among them.
    #[test]
    fn cjk_interface_languages_have_a_font_to_fetch() {
        for code in ["zh-CN", "zh-TW", "ja", "lo"] {
            assert!(needs_system_font(code));
            assert!(ui_script_family(code).is_some(), "{code}");
        }
        assert_eq!(ui_script_family("pt-BR"), None);
    }

    /// Plain, hovered, on and on while hovered look different from each
    /// other in every theme, and a button that is on stands out from the
    /// panel with a frame unlike the hover's.
    #[test]
    fn icon_buttons_have_four_distinct_looks() {
        for t in Theme::ALL {
            assert_eq!(icon_button_in(t, false, false), None, "{t:?}");
            let hover = icon_button_in(t, true, false).expect("hover look");
            let on = icon_button_in(t, false, true).expect("on look");
            let both = icon_button_in(t, true, true).expect("on and hovered look");
            assert_ne!(hover, on, "{t:?}");
            assert_ne!(on, both, "{t:?}");
            assert_ne!(hover, both, "{t:?}");
            assert_ne!(on.1.color, hover.1.color, "{t:?}");
            assert_ne!(on.0, t.palette().panel, "{t:?}");
            assert_ne!(hover.0, t.palette().panel, "{t:?}");
        }
    }

    /// A button that is on has a frame that contrasts with the panel.
    #[test]
    fn the_on_frame_contrasts_with_the_panel() {
        let luma = |c: Color32| 0.299 * c.r() as f32 + 0.587 * c.g() as f32 + 0.114 * c.b() as f32;
        for t in [
            Theme::Light,
            Theme::MediumLight,
            Theme::MediumDark,
            Theme::Dark,
        ] {
            let (_, line) = icon_button_in(t, false, true).expect("on look");
            let gap = (luma(line.color) - luma(t.palette().panel)).abs();
            assert!(gap > 50.0, "{t:?}: {gap}");
        }
    }
}

#[cfg(test)]
mod shadow_tests {
    use super::*;

    #[test]
    fn shadows_can_be_turned_off() {
        let s = egui::epaint::Shadow {
            offset: [2, 2],
            blur: 4,
            spread: 0,
            color: Color32::BLACK,
        };
        set_shadows(false);
        assert_eq!(shadow(s), egui::epaint::Shadow::NONE);
        assert_eq!(visuals().window_shadow, egui::epaint::Shadow::NONE);
        set_shadows(true);
        assert_eq!(shadow(s), s);
    }
}

#[cfg(test)]
mod glyph_tests {
    use super::*;

    /// The symbol characters (`U+2010` to `U+2BFF`: punctuation, arrows,
    /// shapes, dingbats; and the pictographs from `U+1F000`) in the string
    /// and character literals of `src`, outside comments and the test
    /// module, with their line numbers. Letters of other scripts (language
    /// names, sample text) are left to the system fonts.
    fn symbols_in(src: &str) -> Vec<(usize, char)> {
        let code = src.split("#[cfg(test)]").next().unwrap_or(src);
        let symbol =
            |c: char| matches!(c as u32, 0x2010..=0x2027 | 0x2030..=0x2BFF | 0x1F000..=0x1FAFF);
        let mut out = Vec::new();
        for (n, line) in code.lines().enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            let line = line.split(" // ").next().unwrap_or(line);
            // Literal text: what lies between quotes, escapes decoded.
            let mut chars = line.chars().peekable();
            let mut quote: Option<char> = None;
            while let Some(c) = chars.next() {
                match quote {
                    None if c == '"' => quote = Some('"'),
                    // A character literal, not a lifetime: 'x' or '\u{..}'.
                    None if c == '\'' => {
                        let rest: String = chars.clone().take(12).collect();
                        if rest.chars().nth(1) == Some('\'') || rest.starts_with("\\u{") {
                            quote = Some('\'');
                        }
                    }
                    None => {}
                    Some(q) if c == q => quote = None,
                    Some(_) if c == '\\' => {
                        if chars.peek() == Some(&'u') {
                            chars.next();
                            let hex: String = chars
                                .by_ref()
                                .skip_while(|c| *c == '{')
                                .take_while(|c| *c != '}')
                                .collect();
                            if let Some(c) = u32::from_str_radix(&hex, 16)
                                .ok()
                                .and_then(char::from_u32)
                                .filter(|c| symbol(*c))
                            {
                                out.push((n + 1, c));
                            }
                        } else {
                            chars.next();
                        }
                    }
                    Some(_) if symbol(c) => out.push((n + 1, c)),
                    Some(_) => {}
                }
            }
        }
        out
    }

    #[test]
    fn symbols_are_found_in_literals_only() {
        let src = "// \u{25b4} in a comment\nlet a = \"x \\u{25BE}\"; let b = '\u{25b4}';\nfn f<'a>(x: &'a str) {} // \u{2192}\n";
        assert_eq!(symbols_in(src), [(2, '\u{25BE}'), (2, '\u{25B4}')]);
    }

    /// Every symbol the interface writes is in the bundled fonts (the UI
    /// font and egui's own), so no system font is needed to show it: the
    /// panels' and toolbox's scroll arrows were U+25B4 and U+25BE, which
    /// none of them has, and Windows showed empty boxes (#1780).
    #[test]
    fn every_interface_symbol_is_in_the_bundled_fonts() {
        // Not interface text: the bullet characters Bullets puts into the
        // drawing's text (its fonts draw them), and the quote marks a
        // printer name is escaped for.
        let in_the_drawing: &[(&str, char)] = &[
            ("ui/dialogs.rs", '\u{25E6}'),
            ("ui/dialogs.rs", '\u{25AA}'),
            ("ui/dialogs.rs", '\u{2713}'),
            ("ui/dialogs.rs", '\u{27A4}'),
            ("printers.rs", '\u{2018}'),
            ("printers.rs", '\u{2019}'),
            ("printers.rs", '\u{201A}'),
            ("printers.rs", '\u{201B}'),
        ];
        // Each family's faces, read from their character maps (egui's own
        // `has_glyph` calls every character of the face holding its
        // replacement mark missing).
        let defs = bundled_font_definitions();
        let families = [
            egui::FontFamily::Proportional,
            egui::FontFamily::Name(BOLD.into()),
        ];
        let has = |family: &egui::FontFamily, c: char| {
            defs.families.get(family).is_some_and(|names| {
                names.iter().any(|n| {
                    defs.font_data.get(n).is_some_and(|d| {
                        ttf_parser::Face::parse(&d.font, d.index)
                            .is_ok_and(|f| f.glyph_index(c).is_some())
                    })
                })
            })
        };
        assert!(has(&families[0], 'A') && !has(&families[0], '\u{25B4}'));
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = vec![root.clone()];
        let mut missing = Vec::new();
        let mut seen = 0;
        while let Some(path) = files.pop() {
            if path.is_dir() {
                for e in std::fs::read_dir(&path).expect("source folder").flatten() {
                    files.push(e.path());
                }
                continue;
            }
            let name = path
                .strip_prefix(&root)
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default();
            if !name.ends_with(".rs") || name.ends_with("tests.rs") {
                continue;
            }
            let src = std::fs::read_to_string(&path).expect("source file");
            for (line, c) in symbols_in(&src) {
                seen += 1;
                if in_the_drawing.contains(&(name.as_str(), c)) {
                    continue;
                }
                for f in &families {
                    if !has(f, c) {
                        missing.push(format!("{name}:{line}: U+{:04X} {c} ({f:?})", c as u32));
                    }
                }
            }
        }
        assert!(seen > 20, "the scan found the interface's symbols");
        missing.sort();
        missing.dedup();
        assert!(
            missing.is_empty(),
            "symbols no bundled font has:\n{}",
            missing.join("\n")
        );
    }
}
