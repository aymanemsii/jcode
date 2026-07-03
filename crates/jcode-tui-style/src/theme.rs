use crate::color;
use crate::color::rgb;
use ratatui::prelude::*;
use std::sync::atomic::{AtomicU32, Ordering};

const DEFAULT_ACCENT_RGB: (u8, u8, u8) = (186, 139, 255);
const DARK_THEME_ACCENT_RGB: (u8, u8, u8) = (125, 211, 252);
const HIGH_CONTRAST_THEME_ACCENT_RGB: (u8, u8, u8) = (255, 255, 0);
const NO_CONFIGURED_ACCENT: u32 = 0x0100_0000;
static CONFIGURED_ACCENT_RGB: AtomicU32 = AtomicU32::new(NO_CONFIGURED_ACCENT);
static THEME_ACCENT_RGB: AtomicU32 = AtomicU32::new(pack_rgb(DEFAULT_ACCENT_RGB));

pub fn user_color() -> Color {
    rgb(138, 180, 248)
}
pub fn ai_color() -> Color {
    rgb(129, 199, 132)
}
pub fn tool_color() -> Color {
    rgb(120, 120, 120)
}
pub fn file_link_color() -> Color {
    rgb(180, 200, 255)
}
pub fn dim_color() -> Color {
    rgb(80, 80, 80)
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
    let theme_accent = pack_rgb(theme_default_accent_rgb(theme));
    let configured_accent = value.and_then(parse_hex_rgb).unwrap_or(NO_CONFIGURED_ACCENT);
    THEME_ACCENT_RGB.store(theme_accent, Ordering::Relaxed);
    CONFIGURED_ACCENT_RGB.store(configured_accent, Ordering::Relaxed);
}
pub fn theme_default_accent_rgb(theme: Option<&str>) -> (u8, u8, u8) {
    match canonical_theme_name(theme).unwrap_or("default") {
        "dark" => DARK_THEME_ACCENT_RGB,
        "high-contrast" => HIGH_CONTRAST_THEME_ACCENT_RGB,
        _ => DEFAULT_ACCENT_RGB,
    }
}
pub fn canonical_theme_name(theme: Option<&str>) -> Option<&'static str> {
    match theme.map(str::trim).unwrap_or("").to_ascii_lowercase().as_str() {
        "" | "default" => Some("default"),
        "dark" => Some("dark"),
        "high-contrast" => Some("high-contrast"),
        _ => None,
    }
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
fn unpack_rgb(packed: u32) -> (u8, u8, u8) {
    (
        ((packed >> 16) & 0xff) as u8,
        ((packed >> 8) & 0xff) as u8,
        (packed & 0xff) as u8,
    )
}
pub fn system_message_color() -> Color {
    rgb(255, 170, 220)
}
pub fn queued_color() -> Color {
    rgb(255, 193, 7)
}
pub fn asap_color() -> Color {
    rgb(110, 210, 255)
}
pub fn pending_color() -> Color {
    rgb(140, 140, 140)
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
        set_accent_color_from_config(None);
        assert_eq!(accent_color(), rgb(186, 139, 255));

        set_accent_color_from_config(Some("not-a-color"));
        assert_eq!(accent_color(), rgb(186, 139, 255));
    }

    #[test]
    fn configured_accent_overrides_default_when_valid() {
        set_accent_color_from_config(Some("#123456"));
        assert_eq!(accent_color(), rgb(18, 52, 86));
        set_accent_color_from_config(None);
    }

    #[test]
    fn theme_accent_is_used_when_configured_accent_is_missing_or_invalid() {
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
        assert_eq!(canonical_theme_name(Some("unknown")), None);
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
