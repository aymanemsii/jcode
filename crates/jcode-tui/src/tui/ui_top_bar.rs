use super::super::custom_theme_palettes;
use super::theme_support::{accent_color, foreground_color, muted_color, panel_color};
use super::TuiState;
use ratatui::prelude::*;
use ratatui::widgets::Paragraph;
use std::path::Path;

pub(super) const TOP_BAR_SESSION_FALLBACK: &str = "main";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TopBarFields {
    pub app: String,
    pub session: String,
    pub theme: String,
    pub repo: String,
}

pub(super) fn top_bar_enabled() -> bool {
    crate::config::config().display.top_bar == Some(true)
}

pub(super) fn build_top_bar_fields(app: &dyn TuiState) -> TopBarFields {
    let config = crate::config::config();
    let custom_themes = custom_theme_palettes(config);
    let theme = jcode_tui_style::theme::resolve_theme_palette(
        config.display.theme.as_deref(),
        &custom_themes,
    )
    .name;

    TopBarFields {
        app: sanitize_field(config.app.name(), "jcode"),
        session: sanitize_field(app.session_display_name().as_deref(), TOP_BAR_SESSION_FALLBACK),
        theme: sanitize_field(Some(theme.as_str()), "default"),
        repo: repo_name(app.working_dir().as_deref()),
    }
}

pub(super) fn build_top_bar_text(fields: &TopBarFields) -> String {
    format!(
        "{} | session: {} | theme: {} | repo: {}",
        fields.app, fields.session, fields.theme, fields.repo
    )
}

pub(super) fn draw_top_bar(frame: &mut Frame<'_>, area: Rect, app: &dyn TuiState) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let fields = build_top_bar_fields(app);
    let text = build_top_bar_text(&fields);
    let text = truncate_ascii_line(&text, area.width as usize);
    let spans = styled_top_bar_spans(&text, &fields);
    let style = Style::default().fg(foreground_color()).bg(panel_color());
    frame.render_widget(
        Paragraph::new(Text::from(Line::from(spans))).style(style),
        area,
    );
}

fn styled_top_bar_spans(text: &str, fields: &TopBarFields) -> Vec<Span<'static>> {
    if text.len() < fields.app.len() {
        return vec![Span::styled(
            text.to_string(),
            Style::default().fg(accent_color()).bg(panel_color()),
        )];
    }

    let app_len = fields.app.len().min(text.len());
    let (app, rest) = text.split_at(app_len);
    let mut spans = vec![Span::styled(
        app.to_string(),
        Style::default().fg(accent_color()).bg(panel_color()),
    )];

    if !rest.is_empty() {
        spans.push(Span::styled(
            rest.to_string(),
            Style::default().fg(muted_color()).bg(panel_color()),
        ));
    }

    spans
}

fn sanitize_field(value: Option<&str>, fallback: &str) -> String {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

fn repo_name(path: Option<&str>) -> String {
    let Some(path) = path else {
        return "unknown".to_string();
    };
    let normalized = path.replace('\\', "/");
    Path::new(&normalized)
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown")
        .to_string()
}

fn truncate_ascii_line(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_string();
    }
    text.chars().take(width).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_top_bar_text_uses_safe_mvp_fields() {
        let fields = TopBarFields {
            app: "AymaneCode".to_string(),
            session: "main".to_string(),
            theme: "dracula".to_string(),
            repo: "jcode".to_string(),
        };

        assert_eq!(
            build_top_bar_text(&fields),
            "AymaneCode | session: main | theme: dracula | repo: jcode"
        );
    }

    #[test]
    fn sanitize_field_ignores_blank_values() {
        assert_eq!(sanitize_field(Some("  AymaneCode  "), "jcode"), "AymaneCode");
        assert_eq!(sanitize_field(Some("   "), "jcode"), "jcode");
        assert_eq!(sanitize_field(None, "jcode"), "jcode");
    }

    #[test]
    fn repo_name_uses_path_basename() {
        assert_eq!(repo_name(Some(r"C:\Users\Aymane\jcode")), "jcode");
        assert_eq!(repo_name(Some("/home/aymane/jcode")), "jcode");
        assert_eq!(repo_name(None), "unknown");
    }
}
