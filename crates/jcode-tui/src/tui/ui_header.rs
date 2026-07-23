use super::box_utils::render_rounded_box;
use super::changelog::get_unseen_changelog_entries;
use super::{TuiState, dim_color, header_name_color, shorten_model_name};
use crate::auth::{AuthState, AuthStatus};
use crate::tui::color_support::rgb;
use ratatui::prelude::*;
#[cfg(test)]
use super::semver;
#[cfg(test)]
use std::sync::OnceLock;

#[cfg(test)]
fn unseen_changelog_entries_override() -> &'static std::sync::Mutex<Option<Vec<String>>> {
    static OVERRIDE: OnceLock<std::sync::Mutex<Option<Vec<String>>>> = OnceLock::new();
    OVERRIDE.get_or_init(|| std::sync::Mutex::new(None))
}

fn unseen_changelog_entries() -> Vec<String> {
    #[cfg(test)]
    {
        if let Ok(guard) = unseen_changelog_entries_override().lock()
            && let Some(entries) = guard.clone()
        {
            return entries;
        }
    }
    get_unseen_changelog_entries().clone()
}

#[cfg(test)]
pub(crate) fn set_unseen_changelog_entries_override_for_tests(entries: Option<Vec<String>>) {
    let mut guard = unseen_changelog_entries_override()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *guard = entries;
}

pub(crate) fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().chain(chars).collect(),
    }
}

/// Compact form of a full build version string: `v0.25.19-dev (abc1234, dirty)`
/// becomes `v0.25.19-dev`. Used for the per-line server/client version labels.
#[cfg(test)]
fn compact_version_label(version: &str) -> String {
    let trimmed = version.trim();
    match trimmed.split_once(" (") {
        Some((head, _)) => head.trim().to_string(),
        None => trimmed.to_string(),
    }
}

fn format_model_name(short: &str, provider_name: &str) -> String {
    if short.contains('/') {
        // Slashed model ids (e.g. `nvidia/nemotron-...`) are served by the
        // OpenRouter slot, which also fronts direct OpenAI-compatible profiles
        // such as NVIDIA NIM or DeepSeek. Label the line with the active
        // provider's display name instead of hard-coding "OpenRouter" so the
        // header matches the profile the user actually selected.
        let label = {
            let trimmed = provider_name.trim();
            if trimmed.is_empty() {
                "OpenRouter".to_string()
            } else {
                trimmed.to_string()
            }
        };
        return format!("{}: {}", label, short);
    }
    if short.contains("opus") {
        if short.contains("4.5") {
            return "Claude 4.5 Opus".to_string();
        }
        return "Claude Opus".to_string();
    }
    if short.contains("sonnet") {
        if short.contains("3.5") {
            return "Claude 3.5 Sonnet".to_string();
        }
        return "Claude Sonnet".to_string();
    }
    if short.contains("haiku") {
        return "Claude Haiku".to_string();
    }
    if short.starts_with("gpt") {
        // Only the numeric GPT families (gpt-4o, gpt-5.2-codex, ...) have a
        // curated form. Other gpt-prefixed ids (gpt-oss-120b) fall through to
        // the generic prettifier instead of producing "GPT-oss120b".
        let rest = short.trim_start_matches("gpt");
        if rest.is_empty() || rest.starts_with(|c: char| c.is_ascii_digit()) {
            return format_gpt_name(short);
        }
    }
    short.to_string()
}

fn format_gpt_name(short: &str) -> String {
    let rest = short.trim_start_matches("gpt");
    if rest.is_empty() {
        return "GPT".to_string();
    }

    if let Some(idx) = rest.find("codex") {
        let version = &rest[..idx];
        if version.is_empty() {
            return "GPT Codex".to_string();
        }
        return format!("GPT-{} Codex", version);
    }

    format!("GPT-{}", rest)
}

/// Generic fallback for model ids with no curated pretty name: title-case the
/// hyphen/underscore segments (`claude-fable-5` -> `Claude Fable 5`). Date or
/// snapshot suffixes (6+ digit runs) are dropped, vowel-less short segments are
/// treated as acronyms (`glm` -> `GLM`), and parameter sizes are uppercased
/// (`70b` -> `70B`). Placeholder labels with spaces/ellipses pass through.
fn prettify_model_id(model: &str) -> String {
    if model.contains(' ') || model.contains('…') || model.contains('/') {
        return model.to_string();
    }

    fn is_acronym(part: &str) -> bool {
        // Well-known initialisms that contain vowels and would otherwise be
        // title-cased as words.
        const KNOWN: &[&str] = &["oss", "ai", "moe", "vl", "it", "fp8", "awq", "exp"];
        if KNOWN.contains(&part.to_ascii_lowercase().as_str()) {
            return true;
        }
        // Short, all-alphabetic, and vowel-less segments read as initialisms:
        // glm, gpt, qwq, llm. Anything with a vowel (pro, max, mini, fable)
        // reads as a word and gets normal title-casing.
        part.len() <= 4
            && part.chars().all(|c| c.is_ascii_alphabetic())
            && !part
                .chars()
                .any(|c| matches!(c.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u' | 'y'))
    }

    fn is_param_size(part: &str) -> bool {
        // 70b / 8x7b / 32k style size or context markers.
        part.len() >= 2
            && part
                .chars()
                .last()
                .is_some_and(|c| matches!(c.to_ascii_lowercase(), 'b' | 'm' | 'k'))
            && part[..part.len() - 1]
                .chars()
                .all(|c| c.is_ascii_digit() || c == '.' || c == 'x')
            && part.chars().any(|c| c.is_ascii_digit())
    }

    let parts: Vec<String> = model
        .split(['-', '_'])
        .filter(|part| !part.is_empty())
        // Drop date/snapshot suffixes like 20241022.
        .filter(|part| !(part.len() >= 6 && part.chars().all(|c| c.is_ascii_digit())))
        .map(|part| {
            if is_acronym(part) || is_param_size(part) {
                return part.to_uppercase();
            }
            let mut chars = part.chars();
            match chars.next() {
                Some(first) if first.is_ascii_alphabetic() => {
                    first.to_uppercase().chain(chars).collect::<String>()
                }
                Some(first) => first.to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect();
    if parts.is_empty() {
        model.to_string()
    } else {
        parts.join(" ")
    }
}

/// Final display name for the header model line: curated pretty names first
/// (Claude 4.5 Opus, GPT-5.2 Codex), generic title-cased prettification otherwise.
fn header_model_display_name(model: &str, provider_name: &str) -> String {
    let raw = model.trim();

    // Claude family ids ("claude-opus-4-6", "claude-3-5-sonnet-latest",
    // "claude-haiku-4.5") render as "Claude <version> <Family>" for any
    // version, instead of only the hardcoded 3.5/4.5 cases.
    if raw.starts_with("claude") {
        for family in ["opus", "sonnet", "haiku"] {
            if raw.contains(family) {
                let family_pretty = capitalize(family);
                let version = claude_version_segment(raw, family);
                return match version {
                    Some(version) => format!("Claude {} {}", version, family_pretty),
                    None => format!("Claude {}", family_pretty),
                };
            }
        }
    }

    // GPT ids are formatted from the raw segments ("gpt-5.1-codex-max" ->
    // "GPT-5.1 Codex Max") rather than the legacy mashed short form, which
    // produced "GPT-5.1codexmax"-style names.
    if let Some(rest) = raw.strip_prefix("gpt-")
        && rest.starts_with(|c: char| c.is_ascii_digit())
    {
        let mut segments = rest.split('-');
        let version = segments.next().unwrap_or_default();
        let mut name = format!("GPT-{}", version);
        for segment in segments {
            if segment.is_empty() {
                continue;
            }
            let pretty = prettify_model_id(segment);
            name.push(' ');
            name.push_str(&pretty);
        }
        return name;
    }

    let short_model = shorten_model_name(raw);
    let curated = format_model_name(&short_model, provider_name);
    if curated == short_model {
        // No curated pretty name matched; title-case the raw model id
        // instead of showing the mangled short form (`claudefable5`).
        prettify_model_id(raw)
    } else {
        curated
    }
}

/// Extract the version from a Claude model id, e.g. "claude-opus-4-6" -> "4.6",
/// "claude-3-5-sonnet-latest" -> "3.5", "claude-haiku-4.5" -> "4.5". Snapshot
/// dates (6+ digit runs) are ignored.
fn claude_version_segment(raw: &str, family: &str) -> Option<String> {
    let digits: Vec<&str> = raw
        .split(['-', '_'])
        .filter(|part| *part != family)
        .filter(|part| {
            !part.is_empty()
                && part.len() < 6
                && part.chars().all(|c| c.is_ascii_digit() || c == '.')
                && part.chars().any(|c| c.is_ascii_digit())
        })
        .collect();
    match digits.as_slice() {
        [] => None,
        [single] => Some(single.to_string()),
        [major, minor, ..] => Some(format!(
            "{}.{}",
            major.trim_matches('.'),
            minor.trim_matches('.')
        )),
    }
}

pub(super) fn build_auth_status_line(auth: &AuthStatus, max_width: usize) -> Line<'static> {
    fn dot_color(state: AuthState) -> Color {
        match state {
            AuthState::Available => rgb(100, 200, 100),
            AuthState::Expired => rgb(255, 200, 100),
            AuthState::NotConfigured => rgb(80, 80, 80),
        }
    }

    fn dot_char(state: AuthState) -> &'static str {
        match state {
            AuthState::Available => "●",
            AuthState::Expired => "◐",
            AuthState::NotConfigured => "○",
        }
    }

    fn rendered_width(entries: &[&str]) -> usize {
        if entries.is_empty() {
            return 0;
        }

        entries.iter().map(|label| label.len() + 3).sum::<usize>() + (entries.len() - 1)
    }

    fn provider_label(name: &str, state: AuthState, method: Option<&str>) -> String {
        match (state, method) {
            (AuthState::NotConfigured, _) => name.to_string(),
            (_, Some(method)) if !method.is_empty() => format!("{}({})", name, method),
            _ => name.to_string(),
        }
    }

    // The auth line is a credential *inventory* (what is configured), while the
    // provider tag above reports the *active* route. When both credentials are
    // configured, mark the active one with `*` so the two surfaces read as one
    // consistent story ("oauth*+key" = both configured, OAuth in use) instead
    // of an ambiguous "oauth+key" that looks like both are being used at once.
    fn dual_method_label(
        provider: jcode_provider_core::ActiveProvider,
        auth: &AuthStatus,
    ) -> Option<&'static str> {
        use crate::auth::{ActiveCredential, resolve_dual_credential_auth};
        let runtime_provider = std::env::var("JCODE_RUNTIME_PROVIDER").ok();
        let resolved = resolve_dual_credential_auth(provider, auth, runtime_provider.as_deref())?;
        Some(match (resolved.has_oauth, resolved.has_api_key) {
            (true, true) => match resolved.active {
                ActiveCredential::OAuth => "oauth*+key",
                ActiveCredential::ApiKey => "oauth+key*",
            },
            (true, false) => "oauth",
            (false, true) => "key",
            (false, false) => return None,
        })
    }

    let anthropic_label = provider_label(
        "anthropic",
        auth.anthropic.state,
        dual_method_label(jcode_provider_core::ActiveProvider::Claude, auth),
    );

    let openai_label = provider_label(
        "openai",
        auth.openai,
        dual_method_label(jcode_provider_core::ActiveProvider::OpenAI, auth),
    );

    let gemini_label = if auth.gemini != AuthState::NotConfigured {
        provider_label("gemini", auth.gemini, Some("oauth"))
    } else {
        provider_label("gemini", auth.gemini, None)
    };

    let gemini_compact_label = if auth.gemini != AuthState::NotConfigured {
        provider_label("ge", auth.gemini, Some("oauth"))
    } else {
        provider_label("ge", auth.gemini, None)
    };

    let full_specs: Vec<(String, AuthState)> = vec![
        (anthropic_label, auth.anthropic.state),
        ("openrouter".to_string(), auth.openrouter),
        (openai_label, auth.openai),
        (provider_label("cursor", auth.cursor, None), auth.cursor),
        (provider_label("copilot", auth.copilot, None), auth.copilot),
        (gemini_label, auth.gemini),
        (
            provider_label("antigravity", auth.antigravity, None),
            auth.antigravity,
        ),
    ]
    .into_iter()
    .filter(|(_, state)| *state != AuthState::NotConfigured)
    .collect();

    let compact_specs: Vec<(String, AuthState)> = vec![
        (
            provider_label("an", auth.anthropic.state, None),
            auth.anthropic.state,
        ),
        ("or".to_string(), auth.openrouter),
        (provider_label("oa", auth.openai, None), auth.openai),
        (provider_label("cu", auth.cursor, None), auth.cursor),
        (provider_label("cp", auth.copilot, None), auth.copilot),
        (gemini_compact_label, auth.gemini),
        (
            provider_label("ag", auth.antigravity, None),
            auth.antigravity,
        ),
    ]
    .into_iter()
    .filter(|(_, state)| *state != AuthState::NotConfigured)
    .collect();

    let full: Vec<&str> = full_specs.iter().map(|(label, _)| label.as_str()).collect();
    let compact: Vec<&str> = compact_specs
        .iter()
        .map(|(label, _)| label.as_str())
        .collect();

    let provider_specs: Vec<&(String, AuthState)> = if rendered_width(&full) <= max_width {
        full_specs.iter().collect()
    } else if rendered_width(&compact) <= max_width {
        compact_specs.iter().collect()
    } else {
        compact_specs.iter().take(4).collect()
    };

    let mut spans = Vec::new();
    for (i, (label, state)) in provider_specs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" ", Style::default().fg(dim_color())));
        }

        spans.push(Span::styled(
            dot_char(*state),
            Style::default().fg(dot_color(*state)),
        ));
        spans.push(Span::styled(
            format!(" {} ", label),
            Style::default().fg(dim_color()),
        ));
    }

    Line::from(spans)
}

fn header_provider_auth_tag(name: &str, auth: &AuthStatus) -> &'static str {
    let runtime_provider = std::env::var("JCODE_RUNTIME_PROVIDER").ok();

    // Anthropic and OpenAI share one credential-resolution source of truth so
    // the header tag never drifts from the info widget / model-switch line. We
    // route through the canonical ActiveProvider rather than matching display
    // strings, which is how this surface previously broke (name == "claude"
    // never matched a "anthropic"-only arm and the tag silently vanished).
    if let Some(provider) = jcode_provider_core::parse_provider_hint(name) {
        use crate::auth::{ActiveCredential, resolve_dual_credential_auth};
        match resolve_dual_credential_auth(provider, auth, runtime_provider.as_deref()) {
            Some(resolved) => {
                // Report exactly the credential the next request will use. The
                // "both configured" inventory now lives in the auth status line
                // (`oauth*+key`), so this tag never claims two credentials at
                // once -- that ambiguity is how "Claude OAuth" and "API key"
                // used to contradict each other across surfaces.
                return match resolved.active {
                    ActiveCredential::OAuth => "oauth",
                    ActiveCredential::ApiKey => "api-key",
                };
            }
            // Provider recognized but no credentials configured: no tag.
            None if matches!(
                provider,
                jcode_provider_core::ActiveProvider::Claude
                    | jcode_provider_core::ActiveProvider::OpenAI
            ) =>
            {
                return "";
            }
            None => {}
        }
    }

    match name {
        "copilot" => {
            if auth.copilot_has_api_token {
                "oauth"
            } else {
                ""
            }
        }
        "openrouter" | "openai-compatible" => "api-key",
        other
            if crate::provider_catalog::resolve_openai_compatible_profile_selection(other)
                .is_some()
                || crate::provider_catalog::openai_compatible_profile_id_for_display_name(
                    other,
                )
                .is_some() =>
        {
            "api-key"
        }
        _ => "",
    }
}

fn header_provider_label(provider_name: &str, auth: &AuthStatus) -> String {
    let trimmed = provider_name.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let name = trimmed.to_lowercase();
    let auth_tag = header_provider_auth_tag(&name, auth);
    if auth_tag.is_empty() {
        name
    } else {
        format!("{}:{}", auth_tag, name)
    }
}

fn abbreviate_home(path: &str) -> String {
    if let Some(home) = dirs::home_dir() {
        let home_str = home.display().to_string();
        if path == home_str {
            return "~".to_string();
        }
        if let Some(rest) = path.strip_prefix(&home_str) {
            return format!("~{}", rest);
        }
    }
    path.to_string()
}

fn truncate_to_width(text: &str, width: usize) -> String {
    let char_count = text.chars().count();
    if char_count <= width {
        return text.to_string();
    }
    if width == 0 {
        return String::new();
    }
    if width == 1 {
        return "…".to_string();
    }

    let mut truncated = text
        .chars()
        .take(width.saturating_sub(1))
        .collect::<String>();
    truncated.push('…');
    truncated
}

#[cfg(test)]
fn choose_header_candidate(width: usize, candidates: Vec<String>) -> String {
    let mut last_non_empty = String::new();
    for candidate in candidates
        .into_iter()
        .filter(|candidate| !candidate.trim().is_empty())
    {
        if candidate.chars().count() <= width {
            return candidate;
        }
        last_non_empty = candidate;
    }

    truncate_to_width(&last_non_empty, width)
}

#[cfg(test)]
fn semver_core() -> String {
    semver()
        .split('-')
        .next()
        .unwrap_or_else(semver)
        .to_string()
}

#[cfg(test)]
fn semver_minor() -> String {
    let core = semver_core();
    let parts: Vec<&str> = core.split('.').collect();
    if parts.len() >= 2 {
        format!("{}.{}", parts[0], parts[1])
    } else {
        core
    }
}

#[cfg(test)]
fn version_display_candidates() -> Vec<String> {
    let full = format!("jcode {}", semver());
    let core = format!("jcode {}", semver_core());
    let minor = format!("jcode {}", semver_minor());
    let shortest = semver_minor();
    vec![full, core, minor, shortest]
}

#[cfg(test)]
fn configured_auth_count(auth: &AuthStatus) -> usize {
    [
        auth.jcode,
        auth.anthropic.state,
        auth.openrouter,
        auth.azure,
        auth.openai,
        auth.cursor,
        auth.copilot,
        auth.gemini,
        auth.antigravity,
        auth.google,
    ]
    .into_iter()
    .filter(|state| *state != AuthState::NotConfigured)
    .count()
}

const HOMEPAGE_CARD_MAX_WIDTH: usize = 56;
const HOMEPAGE_CARD_MIN_WIDTH: usize = 32;
const HOMEPAGE_LABEL_WIDTH: usize = 10;

struct HomepageCardRow {
    label: &'static str,
    value: String,
    value_style: Style,
}

fn homepage_plain_title(width: usize) -> Line<'static> {
    let title = if width >= "Mercury Core".chars().count() {
        "Mercury Core"
    } else if width >= "Mercury".chars().count() {
        "Mercury"
    } else {
        ""
    };

    Line::from(Span::styled(
        title.to_string(),
        Style::default().fg(header_name_color()).bold(),
    ))
    .alignment(Alignment::Center)
}

fn provider_display_label(provider_name: &str) -> String {
    let trimmed = provider_name.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    match trimmed.to_ascii_lowercase().as_str() {
        "openrouter" => "OpenRouter".to_string(),
        "openai" => "OpenAI".to_string(),
        "openai-compatible" => "OpenAI-compatible".to_string(),
        "anthropic" | "claude" => "Anthropic".to_string(),
        "gemini" | "google" => "Google".to_string(),
        "copilot" => "Copilot".to_string(),
        "cursor" => "Cursor".to_string(),
        other => other
            .split(['-', '_', ' '])
            .filter(|part| !part.is_empty())
            .map(capitalize)
            .collect::<Vec<_>>()
            .join(" "),
    }
}

fn homepage_model_value(model: &str, provider_name: &str, upstream: Option<&str>) -> String {
    let model = homepage_model_display_label(model);
    if model.is_empty() {
        return String::new();
    }

    let provider = upstream
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| provider_display_label(provider_name));
    if provider.is_empty() || model.contains(&provider) {
        model
    } else {
        format!("{} \u{00b7} {}", model, provider)
    }
}

fn homepage_model_display_label(model: &str) -> String {
    let trimmed = model.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let route_stripped = trimmed
        .rsplit_once(':')
        .map(|(_, value)| value.trim())
        .unwrap_or(trimmed);
    let model_id = route_stripped
        .rsplit_once('/')
        .map(|(_, value)| value.trim())
        .unwrap_or(route_stripped);

    match model_id.to_ascii_lowercase().as_str() {
        "claude-sonnet-4" | "claude-sonnet-4-20250514" | "claude-4-sonnet" => {
            "Claude Sonnet 4".to_string()
        }
        "claude-opus-4" | "claude-opus-4-20250514" | "claude-4-opus" => {
            "Claude Opus 4".to_string()
        }
        "claude-haiku-4" | "claude-haiku-4-20250514" | "claude-4-haiku" => {
            "Claude Haiku 4".to_string()
        }
        _ => header_model_display_name(model_id, ""),
    }
}

fn workspace_display_label(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    std::path::Path::new(trimmed)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| abbreviate_home(trimmed))
}

fn homepage_mode_label(app: &dyn TuiState) -> String {
    homepage_mode_label_from_flags(app.is_replay(), app.is_canary(), app.is_remote_mode())
}

fn homepage_mode_label_from_flags(is_replay: bool, is_canary: bool, is_remote: bool) -> String {
    if is_replay {
        return "replay".to_string();
    }
    if is_canary {
        return "self-dev".to_string();
    }
    if is_remote {
        return "remote".to_string();
    }
    "normal".to_string()
}

fn homepage_card_width(width: usize, rows: &[HomepageCardRow]) -> usize {
    let content_width = rows
        .iter()
        .map(|row| {
            HOMEPAGE_LABEL_WIDTH
                + 2
                + row.value.chars().count().min(HOMEPAGE_CARD_MAX_WIDTH)
        })
        .max()
        .unwrap_or(0)
        .max("/server details   /model switch".chars().count());
    let desired = (content_width + 6).clamp(HOMEPAGE_CARD_MIN_WIDTH, HOMEPAGE_CARD_MAX_WIDTH);
    desired.min(width)
}

fn homepage_card_border_top(card_width: usize) -> Line<'static> {
    let title = " Mercury Core ";
    let inner_width = card_width.saturating_sub(2);
    if inner_width <= title.chars().count() {
        return Line::from(Span::styled(
            format!("+{}+", "-".repeat(inner_width)),
            Style::default().fg(dim_color()),
        ))
        .alignment(Alignment::Center);
    }

    let right = inner_width.saturating_sub(1 + title.chars().count());
    Line::from(vec![
        Span::styled(
            "+-",
            Style::default().fg(dim_color()),
        ),
        Span::styled(title, Style::default().fg(header_name_color()).bold()),
        Span::styled(
            format!("{}+", "-".repeat(right)),
            Style::default().fg(dim_color()),
        ),
    ])
    .alignment(Alignment::Center)
}

fn homepage_card_row(row: &HomepageCardRow, card_width: usize) -> Line<'static> {
    let inner_width = card_width.saturating_sub(2);
    let value_width = inner_width.saturating_sub(HOMEPAGE_LABEL_WIDTH + 5);
    let value = truncate_to_width(&row.value, value_width);
    let used = HOMEPAGE_LABEL_WIDTH + 5 + value.chars().count();
    let trailing = inner_width.saturating_sub(used);

    Line::from(vec![
        Span::styled("|  ", Style::default().fg(dim_color())),
        Span::styled(
            format!("{:<width$}", row.label, width = HOMEPAGE_LABEL_WIDTH),
            Style::default().fg(dim_color()),
        ),
        Span::styled("  ", Style::default().fg(dim_color())),
        Span::styled(value, row.value_style),
        Span::styled(" ".repeat(trailing), Style::default().fg(dim_color())),
        Span::styled(" |", Style::default().fg(dim_color())),
    ])
    .alignment(Alignment::Center)
}

fn homepage_card_blank(card_width: usize) -> Line<'static> {
    let inner_width = card_width.saturating_sub(2);
    Line::from(Span::styled(
        format!("|{}|", " ".repeat(inner_width)),
        Style::default().fg(dim_color()),
    ))
    .alignment(Alignment::Center)
}

fn homepage_card_footer(card_width: usize) -> Line<'static> {
    let inner_width = card_width.saturating_sub(2);
    let hint = truncate_to_width("/server details   /model switch", inner_width.saturating_sub(3));
    let trailing = inner_width.saturating_sub(3 + hint.chars().count());
    Line::from(vec![
        Span::styled("|  ", Style::default().fg(dim_color())),
        Span::styled(hint, Style::default().fg(dim_color())),
        Span::styled(" ".repeat(trailing), Style::default().fg(dim_color())),
        Span::styled(" |", Style::default().fg(dim_color())),
    ])
    .alignment(Alignment::Center)
}

fn homepage_card_lines(rows: Vec<HomepageCardRow>, width: usize) -> Vec<Line<'static>> {
    if width < HOMEPAGE_CARD_MIN_WIDTH {
        return homepage_plain_lines(rows, width);
    }

    let card_width = homepage_card_width(width, &rows);
    let mut lines = Vec::new();
    lines.push(homepage_card_border_top(card_width));
    for row in &rows {
        lines.push(homepage_card_row(row, card_width));
    }
    lines.push(homepage_card_blank(card_width));
    lines.push(homepage_card_footer(card_width));
    lines.push(
        Line::from(Span::styled(
            format!("+{}+", "-".repeat(card_width.saturating_sub(2))),
            Style::default().fg(dim_color()),
        ))
        .alignment(Alignment::Center),
    );
    lines
}

fn homepage_plain_lines(rows: Vec<HomepageCardRow>, width: usize) -> Vec<Line<'static>> {
    let mut lines = vec![homepage_plain_title(width), Line::from("")];
    for row in rows {
        let text = format!("{} {}", row.label, row.value);
        lines.push(
            Line::from(Span::styled(
                truncate_to_width(&text, width as usize),
                row.value_style,
            ))
            .alignment(Alignment::Center),
        );
    }
    lines
}

fn homepage_rows(app: &dyn TuiState, model: &str, nice_model: String) -> Vec<HomepageCardRow> {
    let mut rows = vec![HomepageCardRow {
        label: "Status",
        value: "Ready".to_string(),
        value_style: Style::default().fg(rgb(130, 220, 170)).bold(),
    }];

    let session_name = app.session_display_name().unwrap_or_default();
    if !session_name.trim().is_empty() {
        rows.push(HomepageCardRow {
            label: "Session",
            value: capitalize(&session_name),
            value_style: Style::default().fg(header_name_color()).bold(),
        });
    }

    let model_is_placeholder = {
        let trimmed = model.trim();
        trimmed.is_empty()
            || trimmed == "connected"
            || trimmed.ends_with("...")
            || trimmed.chars().last().is_some_and(|c| c == '\u{2026}')
            || trimmed.starts_with("connecting")
    };
    let model_value = if model_is_placeholder {
        nice_model
    } else {
        homepage_model_value(&nice_model, &app.provider_name(), app.upstream_provider().as_deref())
    };
    if !model_value.trim().is_empty() {
        rows.push(HomepageCardRow {
            label: "Model",
            value: model_value,
            value_style: Style::default().fg(rgb(255, 150, 200)).bold(),
        });
    }

    if let Some(dir) = app.working_dir() {
        let workspace = workspace_display_label(&dir);
        if !workspace.trim().is_empty() {
            rows.push(HomepageCardRow {
                label: "Workspace",
                value: workspace,
                value_style: Style::default().fg(header_name_color()),
            });
        }
    }

    rows.push(HomepageCardRow {
        label: "Mode",
        value: homepage_mode_label(app),
        value_style: Style::default().fg(header_name_color()),
    });

    rows
}

pub(super) fn build_persistent_header(app: &dyn TuiState, width: u16) -> Vec<Line<'static>> {
    let model = app.provider_model();
    let nice_model = header_model_display_name(&model, &app.provider_name());
    homepage_card_lines(homepage_rows(app, &model, nice_model), width as usize)
}

pub(crate) fn build_header_lines(app: &dyn TuiState, width: u16) -> Vec<Line<'static>> {
    let mut lines: Vec<Line> = Vec::new();
    let align = ratatui::layout::Alignment::Center;
    let w = width as usize;

    if let Some(goal_badge) = crate::goal::header_badge(
        app.working_dir().as_deref().map(std::path::Path::new),
        app.side_panel(),
    ) {
        lines.push(
            Line::from(Span::styled(
                goal_badge,
                Style::default().fg(rgb(170, 200, 120)),
            ))
            .alignment(align),
        );
    }

    let new_entries = unseen_changelog_entries();
    if !new_entries.is_empty() && w > 20 {
        const MAX_LINES: usize = 8;
        let available_width = w.saturating_sub(2);
        let display_count = new_entries.len().min(MAX_LINES);
        let has_more = new_entries.len() > MAX_LINES;

        let mut content: Vec<Line> = Vec::new();
        for entry in new_entries.iter().take(display_count) {
            content.push(
                Line::from(Span::styled(
                    format!("\u{2022} {}", entry),
                    Style::default().fg(dim_color()),
                ))
                .alignment(align),
            );
        }
        if has_more {
            content.push(
                Line::from(Span::styled(
                    format!(
                        "  \u{2026}{} more \u{00b7} /changelog to see all",
                        new_entries.len() - MAX_LINES
                    ),
                    Style::default().fg(dim_color()),
                ))
                .alignment(align),
            );
        }

        let boxed = render_rounded_box(
            "Updates",
            content,
            available_width,
            Style::default().fg(dim_color()),
        );
        for line in boxed {
            lines.push(line.alignment(align));
        }
    }

    lines.push(Line::from(""));
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{AuthState, AuthStatus, ProviderAuth};
    use crate::message::Message;
    use crate::provider::{EventStream, Provider};
    use crate::tool::Registry;
    use anyhow::Result;
    use async_trait::async_trait;
    use std::sync::Arc;
    use std::sync::OnceLock;

    struct MockProvider;

    struct ChangelogOverrideGuard;

    impl Drop for ChangelogOverrideGuard {
        fn drop(&mut self) {
            set_unseen_changelog_entries_override_for_tests(None);
        }
    }

    fn changelog_override(entries: Vec<String>) -> ChangelogOverrideGuard {
        set_unseen_changelog_entries_override_for_tests(Some(entries));
        ChangelogOverrideGuard
    }

    #[async_trait]
    impl Provider for MockProvider {
        async fn complete(
            &self,
            _messages: &[Message],
            _tools: &[crate::message::ToolDefinition],
            _system: &str,
            _resume_session_id: Option<&str>,
        ) -> Result<EventStream> {
            Err(anyhow::anyhow!(
                "Mock provider should not be used for streaming completions in ui header tests"
            ))
        }

        fn name(&self) -> &str {
            "mock"
        }

        fn fork(&self) -> Arc<dyn Provider> {
            Arc::new(MockProvider)
        }
    }

    fn ensure_test_jcode_home_if_unset() {
        static TEST_HOME: OnceLock<std::path::PathBuf> = OnceLock::new();

        if std::env::var_os("JCODE_HOME").is_some() {
            return;
        }

        let path = TEST_HOME.get_or_init(|| {
            let path = std::env::temp_dir().join(format!("jcode-test-home-{}", std::process::id()));
            let _ = std::fs::create_dir_all(&path);
            path
        });
        crate::env::set_var("JCODE_HOME", path);
    }

    fn create_test_app() -> crate::tui::app::App {
        ensure_test_jcode_home_if_unset();

        let provider: Arc<dyn Provider> = Arc::new(MockProvider);
        let rt = tokio::runtime::Runtime::new().expect("test runtime");
        let registry = rt.block_on(Registry::new(provider.clone()));
        crate::tui::app::App::new_for_test_harness(provider, registry)
    }

    #[test]
    fn left_aligned_mode_keeps_persistent_header_centered() {
        let mut app = create_test_app();
        app.set_centered(false);

        let lines = build_persistent_header(&app, 80);
        let non_empty: Vec<&Line<'_>> = lines
            .iter()
            .filter(|line| !line.spans.iter().all(|span| span.content.trim().is_empty()))
            .collect();

        assert!(!non_empty.is_empty(), "expected persistent header lines");
        assert!(
            non_empty
                .iter()
                .all(|line| line.alignment == Some(Alignment::Center)),
            "persistent header should remain centered in left-aligned mode: {non_empty:?}"
        );
    }

    #[test]
    fn left_aligned_mode_keeps_secondary_header_centered() {
        let mut app = create_test_app();
        app.set_centered(false);
        let _guard = changelog_override(vec!["Card polish".to_string()]);

        let lines = build_header_lines(&app, 80);
        let non_empty: Vec<&Line<'_>> = lines
            .iter()
            .filter(|line| !line.spans.iter().all(|span| span.content.trim().is_empty()))
            .collect();

        assert!(!non_empty.is_empty(), "expected header detail lines");
        assert!(
            non_empty
                .iter()
                .all(|line| line.alignment == Some(Alignment::Center)),
            "header detail lines should remain centered in left-aligned mode: {non_empty:?}"
        );
    }

    #[test]
    fn version_display_candidates_compact_for_narrow_width() {
        let rendered = choose_header_candidate(8, version_display_candidates());
        // Version-agnostic: at width 8 only the bare minor semver fits.
        assert_eq!(rendered, semver_minor());
    }

    fn rendered_header_lines(app: &crate::tui::app::App, width: u16) -> Vec<String> {
        build_persistent_header(app, width)
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect()
    }

    #[test]
    fn persistent_header_includes_mercury_core_inside_centered_block() {
        let mut app = create_test_app();
        app.set_remote_server_identity_for_tests(
            Some("summit"),
            None,
            Some("v0.14.2-dev (old1234)"),
            Some("session_parrot_1705012345678"),
        );

        let lines = build_persistent_header(&app, 80);
        let rendered: Vec<String> = lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect();
        let mercury_idx = rendered
            .iter()
            .position(|line| line.contains("Mercury Core"))
            .expect("Mercury Core card title");
        let status_idx = rendered
            .iter()
            .position(|line| line.contains("Status") && line.contains("Ready"))
            .expect("Status card row");

        assert!(mercury_idx < status_idx);
        assert_eq!(lines[mercury_idx].alignment, Some(Alignment::Center));
        assert!(rendered[mercury_idx].starts_with('+'));
        assert!(rendered[mercury_idx].ends_with('+'));
        assert!(rendered.iter().any(|line| line.starts_with('+') && line.ends_with('+')));
    }

    #[test]
    fn persistent_header_mercury_core_does_not_render_as_left_edge_strip() {
        use ratatui::buffer::Buffer;
        use ratatui::layout::Rect;
        use ratatui::widgets::{Paragraph, Widget};

        let app = create_test_app();
        let lines = build_persistent_header(&app, 80);
        let mut buffer = Buffer::empty(Rect::new(0, 0, 80, lines.len() as u16));
        Paragraph::new(lines).render(buffer.area, &mut buffer);

        let mercury_row = (0..buffer.area.height)
            .find(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, *y)].symbol())
                    .collect::<String>()
                    .contains("Mercury Core")
            })
            .expect("Mercury Core rendered row");
        let row_text = (0..buffer.area.width)
            .map(|x| buffer[(x, mercury_row)].symbol())
            .collect::<String>();
        let first_non_space = row_text
            .chars()
            .position(|ch| ch != ' ')
            .expect("non-empty Mercury Core row");

        assert!(first_non_space > 0);
    }

    #[test]
    fn persistent_header_uses_user_facing_status_labels() {
        let mut app = create_test_app();
        app.set_remote_server_identity_for_tests(
            Some("summit"),
            None,
            Some("v0.14.2-dev (old1234)"),
            Some("session_parrot_1705012345678"),
        );

        let lines = rendered_header_lines(&app, 120);
        let expected_model = header_model_display_name(&app.provider_model(), &app.provider_name());

        assert!(lines.iter().any(|line| line.contains("Mercury Core")));
        assert!(lines.iter().any(|line| line.contains("Status") && line.contains("Ready")));
        assert!(lines.iter().any(|line| line.contains("Session")));
        assert!(lines.iter().any(|line| line.contains("Parrot")));
        assert!(lines.iter().any(|line| line.contains("Model")));
        assert!(lines.iter().any(|line| line.contains(&expected_model)));
        if app.working_dir().is_some() {
            assert!(lines.iter().any(|line| line.contains("Workspace")));
        }
        assert!(lines.iter().any(|line| line.contains("Mode")));
        assert!(lines.iter().any(|line| line.contains("/server details")));
        assert!(lines.iter().any(|line| line.contains("/model switch")));
        assert!(lines.iter().all(|line| !line.contains("[client-dev]")));
        assert!(lines.iter().all(|line| !line.contains("server:")));
        assert!(lines.iter().all(|line| !line.contains("client:")));
        assert!(lines.iter().all(|line| !line.contains("api-key:")));
        assert!(lines.iter().all(|line| !line.contains("remote - self-dev")));
    }

    #[test]
    fn persistent_header_mercury_core_degrades_without_panicking_on_tiny_widths() {
        let app = create_test_app();

        for width in [0, 1, 4, 8, 12] {
            let lines = build_persistent_header(&app, width);
            assert!(!lines.is_empty());
            let rendered: Vec<String> = lines
                .iter()
                .map(|line| {
                    line.spans
                        .iter()
                        .map(|span| span.content.as_ref())
                        .collect::<String>()
                })
                .collect();
            assert!(rendered.iter().all(|line| !line.starts_with('+')));
            assert!(rendered.iter().all(|line| !line.starts_with('|')));
        }
    }

    #[test]
    fn homepage_model_value_uses_display_provider_without_api_key_prefix() {
        let value = homepage_model_value("Claude Sonnet 4", "openrouter", None);

        assert_eq!(value, "Claude Sonnet 4 \u{00b7} OpenRouter");
        assert!(!value.contains("api-key:"));
    }

    #[test]
    fn homepage_model_value_cleans_openrouter_provider_prefixed_ids() {
        let value = homepage_model_value(
            "OpenRouter: anthropic/claude-sonnet-4",
            "openrouter",
            None,
        );

        assert_eq!(value, "Claude Sonnet 4 \u{00b7} OpenRouter");
        assert!(!value.contains("anthropic/"));
        assert!(!value.contains("OpenRouter:"));
    }

    #[test]
    fn homepage_mode_label_prefers_self_dev_over_remote_suffix() {
        let value = homepage_mode_label_from_flags(false, true, true);

        assert_eq!(value, "self-dev");
        assert_ne!(value, "remote - self-dev");
    }

    #[test]
    fn persistent_header_represents_remote_session_without_server_client_labels() {
        let mut app = create_test_app();
        app.set_remote_server_identity_for_tests(
            Some("blazing"),
            Some("fire"),
            Some("v0.14.2-dev (old1234)"),
            Some("session_fox_1705012345678"),
        );

        let lines = rendered_header_lines(&app, 120);
        assert!(lines.iter().any(|line| line.contains("Session") && line.contains("Fox")));
        assert!(lines.iter().any(|line| line.contains("Mode") && line.contains("remote")));
        assert!(lines.iter().all(|line| !line.contains("server:") && !line.contains("client:")));
        assert!(lines.iter().all(|line| !line.contains("v0.14.2") && !line.contains("old1234")));
    }

    #[test]
    fn persistent_header_hides_build_hashes_from_homepage() {
        let mut app = create_test_app();
        let client_semver = compact_version_label(jcode_build_meta::VERSION);
        let fake_server_version = format!("{} (0000000)", client_semver);
        app.set_remote_server_identity_for_tests(
            Some("blazing"),
            None,
            Some(&fake_server_version),
            Some("session_fox_1705012345678"),
        );

        let lines = rendered_header_lines(&app, 160);
        assert!(lines.iter().any(|line| line.contains("Session") && line.contains("Fox")));
        assert!(lines.iter().all(|line| !line.contains("(0000000)") && !line.contains(jcode_build_meta::VERSION)));
    }

    #[test]
    fn persistent_header_omits_versions_when_too_narrow() {
        let mut app = create_test_app();
        app.set_remote_server_identity_for_tests(
            Some("blazing"),
            Some("fire"),
            Some("v0.14.2-dev (old1234)"),
            Some("session_fox_1705012345678"),
        );

        let lines = rendered_header_lines(&app, 18);
        assert!(lines.iter().all(|line| !line.contains("server:")));
        assert!(lines.iter().all(|line| !line.contains("v0.14.2")));
    }

    #[test]
    fn persistent_header_local_mode_has_no_version_labels() {
        let app = create_test_app();
        let lines = rendered_header_lines(&app, 120);
        assert!(!lines.iter().any(|line| line.contains("server:")));
        assert!(!lines.iter().any(|line| line.contains("client:") && line.contains(" - v")));
    }

    #[test]
    fn persistent_header_hides_connection_hint_from_homepage() {
        let mut app = create_test_app();
        app.set_remote_server_identity_for_tests(
            Some("blazing"),
            Some("fire"),
            Some("v0.14.2-dev (old1234)"),
            Some("session_ram_1705012345678"),
        );

        let lines = rendered_header_lines(&app, 120);
        assert!(lines.iter().any(|line| line.contains("Session") && line.contains("Ram")));
        assert!(lines.iter().all(|line| !line.contains("client:") && !line.contains("https/sse")));
    }

    #[test]
    fn persistent_header_keeps_session_name_without_raw_client_label() {
        let mut app = create_test_app();
        app.set_remote_server_identity_for_tests(
            Some("blazing"),
            Some("fire"),
            Some("v0.14.2-dev (old1234)"),
            Some("session_fox_1705012345678"),
        );

        let lines = rendered_header_lines(&app, 120);
        assert!(lines.iter().any(|line| line.contains("Session") && line.contains("Fox")));
        assert!(lines.iter().all(|line| !line.contains("client:")));
    }

    #[test]
    fn prettify_model_id_title_cases_unknown_models() {
        assert_eq!(prettify_model_id("claude-fable-5"), "Claude Fable 5");
        assert_eq!(prettify_model_id("grok-code-fast-1"), "Grok Code Fast 1");
        assert_eq!(prettify_model_id("kimi_k2"), "Kimi K2");
        assert_eq!(
            prettify_model_id("gemini-3-pro-preview"),
            "Gemini 3 Pro Preview"
        );
        assert_eq!(prettify_model_id("deepseek-chat"), "Deepseek Chat");
        assert_eq!(
            prettify_model_id("mistral-large-2411"),
            "Mistral Large 2411"
        );
        assert_eq!(prettify_model_id("o3-mini"), "O3 Mini");
        // Vowel-less short segments read as acronyms.
        assert_eq!(prettify_model_id("glm-4.6"), "GLM 4.6");
        assert_eq!(prettify_model_id("qwq-32b"), "QWQ 32B");
        // Parameter sizes are uppercased.
        assert_eq!(prettify_model_id("llama-3.3-70b"), "Llama 3.3 70B");
        assert_eq!(prettify_model_id("mixtral-8x7b"), "Mixtral 8X7B");
        // Long digit runs (snapshot dates) are dropped.
        assert_eq!(
            prettify_model_id("claude-fable-5-20260101"),
            "Claude Fable 5"
        );
        // Placeholders and slashed ids pass through untouched.
        assert_eq!(prettify_model_id("loading session…"), "loading session…");
        assert_eq!(
            prettify_model_id("deepseek/deepseek-chat"),
            "deepseek/deepseek-chat"
        );
        // Degenerate inputs survive.
        assert_eq!(prettify_model_id(""), "");
        assert_eq!(prettify_model_id("-"), "-");
    }

    #[test]
    fn header_model_display_name_sweeps_real_model_catalog() {
        // End-to-end through shorten_model_name + format_model_name +
        // prettify_model_id, over the model ids jcode actually routes.
        let cases = [
            // Anthropic
            ("claude-opus-4-5-20251101", "Claude 4.5 Opus"),
            ("claude-opus-4.6", "Claude 4.6 Opus"),
            ("claude-opus-4-8", "Claude 4.8 Opus"),
            ("claude-sonnet-4-5", "Claude 4.5 Sonnet"),
            ("claude-sonnet-4", "Claude 4 Sonnet"),
            ("claude-3-5-sonnet-latest", "Claude 3.5 Sonnet"),
            ("claude-haiku-4-5", "Claude 4.5 Haiku"),
            ("claude-fable-5", "Claude Fable 5"),
            // OpenAI
            ("gpt-5.2-codex", "GPT-5.2 Codex"),
            ("gpt-5.1-codex-max", "GPT-5.1 Codex Max"),
            ("gpt-5.3-codex-spark", "GPT-5.3 Codex Spark"),
            ("gpt-5-mini", "GPT-5 Mini"),
            ("gpt-5.1-chat-latest", "GPT-5.1 Chat Latest"),
            ("gpt-4o", "GPT-4o"),
            ("gpt-4o-mini", "GPT-4o Mini"),
            ("gpt-oss-120b", "GPT OSS 120B"),
            ("o3-mini", "O3 Mini"),
            ("o4-mini", "O4 Mini"),
            // Google
            ("gemini-3-pro-preview", "Gemini 3 Pro Preview"),
            ("gemini-2.5-flash", "Gemini 2.5 Flash"),
            // xAI / Moonshot / Zhipu / DeepSeek / Minimax
            ("grok-code-fast-1", "Grok Code Fast 1"),
            ("kimi-k2.5", "Kimi K2.5"),
            ("kimi-k2p5-turbo", "Kimi K2p5 Turbo"),
            ("glm-4.6", "GLM 4.6"),
            ("deepseek-v4-flash", "Deepseek V4 Flash"),
            ("minimax-m2.7", "Minimax M2.7"),
            // Meta / Mistral / Qwen / community
            ("llama-3.3-70b", "Llama 3.3 70B"),
            ("mixtral-8x7b", "Mixtral 8X7B"),
            ("devstral-medium-2507", "Devstral Medium 2507"),
            ("qwen3-coder-plus", "Qwen3 Coder Plus"),
            ("composer-1.5", "Composer 1.5"),
            ("llama-3.1-8b-instant", "Llama 3.1 8B Instant"),
        ];
        for (input, expected) in cases {
            assert_eq!(
                header_model_display_name(input, ""),
                expected,
                "model id {input:?}"
            );
        }

        // Slashed ids keep the provider label form.
        assert_eq!(
            header_model_display_name("deepseek/deepseek-chat", "OpenRouter"),
            "OpenRouter: deepseek/deepseek-chat"
        );
        // Placeholders pass through untouched.
        assert_eq!(
            header_model_display_name("loading session…", ""),
            "loading session…"
        );
        assert_eq!(header_model_display_name("connected", ""), "Connected");
    }

    #[test]
    fn compact_version_label_strips_hash_suffix() {
        assert_eq!(
            compact_version_label("v0.25.19-dev (7e261bcc, dirty)"),
            "v0.25.19-dev"
        );
        assert_eq!(compact_version_label("v0.25.19 (abc1234)"), "v0.25.19");
        assert_eq!(compact_version_label(" v0.25.19 "), "v0.25.19");
    }

    #[test]
    fn configured_auth_count_includes_non_model_auth_surfaces() {
        let auth = AuthStatus {
            jcode: AuthState::Available,
            anthropic: ProviderAuth {
                state: AuthState::Expired,
                has_oauth: true,
                oauth_state: AuthState::Expired,
                has_api_key: false,
            },
            azure: AuthState::Available,
            google: AuthState::Available,
            ..AuthStatus::default()
        };

        assert_eq!(configured_auth_count(&auth), 4);
    }

    #[test]
    fn header_provider_auth_tag_reports_active_credential_for_openai() {
        let _guard = crate::storage::lock_test_env();
        let prev = std::env::var_os("JCODE_RUNTIME_PROVIDER");
        crate::env::remove_var("JCODE_RUNTIME_PROVIDER");
        let auth = AuthStatus {
            openai: AuthState::Available,
            openai_has_oauth: true,
            openai_has_api_key: true,
            ..AuthStatus::default()
        };

        // Auto mode prefers OAuth; the tag must report only the credential in
        // use (the auth inventory line carries the "both configured" detail).
        assert_eq!(header_provider_auth_tag("openai", &auth), "oauth");
        if let Some(value) = prev {
            crate::env::set_var("JCODE_RUNTIME_PROVIDER", value);
        }
    }

    #[test]
    fn header_provider_auth_tag_honors_runtime_selection_and_oauth_first() {
        let _guard = crate::storage::lock_test_env();
        let prev = std::env::var_os("JCODE_RUNTIME_PROVIDER");

        let both = AuthStatus {
            anthropic: ProviderAuth {
                has_oauth: true,
                has_api_key: true,
                ..Default::default()
            },
            ..AuthStatus::default()
        };

        // Explicit API-key selection wins even when OAuth is available.
        crate::env::set_var("JCODE_RUNTIME_PROVIDER", "claude-api");
        assert_eq!(header_provider_auth_tag("anthropic", &both), "api-key");

        // Explicit OAuth selection.
        crate::env::set_var("JCODE_RUNTIME_PROVIDER", "claude");
        assert_eq!(header_provider_auth_tag("anthropic", &both), "oauth");

        // Auto (unset) prefers OAuth when both credentials are present.
        crate::env::remove_var("JCODE_RUNTIME_PROVIDER");
        assert_eq!(header_provider_auth_tag("anthropic", &both), "oauth");

        // The "claude" display name resolves to the same Anthropic tagging.
        assert_eq!(header_provider_auth_tag("claude", &both), "oauth");
        crate::env::set_var("JCODE_RUNTIME_PROVIDER", "claude-api");
        assert_eq!(header_provider_auth_tag("claude", &both), "api-key");
        crate::env::remove_var("JCODE_RUNTIME_PROVIDER");

        // Auto falls back to the API key when no OAuth credential exists.
        let api_only = AuthStatus {
            anthropic: ProviderAuth {
                has_oauth: false,
                has_api_key: true,
                ..Default::default()
            },
            ..AuthStatus::default()
        };
        assert_eq!(header_provider_auth_tag("anthropic", &api_only), "api-key");

        if let Some(value) = prev {
            crate::env::set_var("JCODE_RUNTIME_PROVIDER", value);
        } else {
            crate::env::remove_var("JCODE_RUNTIME_PROVIDER");
        }
    }

    #[test]
    fn build_persistent_header_prefers_configured_model_during_remote_connect() {
        let _guard = crate::storage::lock_test_env();
        let prev_model = std::env::var_os("JCODE_MODEL");
        let prev_provider = std::env::var_os("JCODE_PROVIDER");
        crate::env::set_var("JCODE_MODEL", "gpt-5.4");
        crate::env::set_var("JCODE_PROVIDER", "openai");

        let app = crate::tui::app::App::new_for_remote(None);
        let lines = build_persistent_header(&app, 80);
        let rendered = lines
            .iter()
            .flat_map(|line| line.spans.iter())
            .map(|span| span.content.as_ref())
            .collect::<String>();

        assert!(rendered.contains("GPT-5.4"));
        assert!(!rendered.contains("connecting to server…"));

        if let Some(prev_model) = prev_model {
            crate::env::set_var("JCODE_MODEL", prev_model);
        } else {
            crate::env::remove_var("JCODE_MODEL");
        }
        if let Some(prev_provider) = prev_provider {
            crate::env::set_var("JCODE_PROVIDER", prev_provider);
        } else {
            crate::env::remove_var("JCODE_PROVIDER");
        }
    }

    #[test]
    fn build_header_lines_omits_placeholder_provider_label_when_unknown() {
        let mut app = crate::tui::app::App::new_for_remote(None);
        app.set_remote_startup_phase(crate::tui::app::RemoteStartupPhase::LoadingSession);

        // The model line lives in the persistent header now; the startup phase
        // label renders there without a bogus "(unknown)" provider tag.
        let lines = build_persistent_header(&app, 80);
        let rendered = lines
            .iter()
            .flat_map(|line| line.spans.iter())
            .map(|span| span.content.as_ref())
            .collect::<String>();

        assert!(rendered.contains("loading session…"), "{rendered}");
        assert!(!rendered.contains("(unknown)"));
        assert!(!rendered.contains("(remote)"));
    }

    #[test]
    fn build_header_lines_hides_secondary_placeholder_during_brief_connecting_phase() {
        let app = crate::tui::app::App::new_for_remote(None);

        let lines = build_header_lines(&app, 80);
        let rendered = lines
            .iter()
            .flat_map(|line| line.spans.iter())
            .map(|span| span.content.as_ref())
            .collect::<String>();

        assert!(
            !rendered.contains("connecting to server…"),
            "brief connecting placeholder should not render the secondary detail line"
        );
        assert!(!rendered.contains("(remote)"));
    }

    #[test]
    fn auth_status_line_hides_not_configured_providers() {
        let auth = AuthStatus {
            anthropic: ProviderAuth {
                state: AuthState::Expired,
                has_oauth: true,
                oauth_state: AuthState::Expired,
                has_api_key: false,
            },
            openai: AuthState::Available,
            openai_has_oauth: false,
            openai_has_api_key: true,
            ..AuthStatus::default()
        };

        let line = build_auth_status_line(&auth, 120);
        let rendered = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();

        assert!(
            rendered.contains("anthropic(oauth)"),
            "rendered: {rendered}"
        );
        assert!(rendered.contains("openai(key)"), "rendered: {rendered}");
        assert!(!rendered.contains("openrouter"), "rendered: {rendered}");
        assert!(!rendered.contains("copilot"), "rendered: {rendered}");
        assert!(!rendered.contains("cursor"), "rendered: {rendered}");
    }

    #[test]
    fn auth_status_line_is_empty_when_nothing_was_attempted() {
        let line = build_auth_status_line(&AuthStatus::default(), 120);
        assert!(line.spans.is_empty(), "line should be empty: {line:?}");
    }

    #[test]
    fn auth_status_line_marks_active_credential_when_both_configured() {
        let _guard = crate::storage::lock_test_env();
        let prev = std::env::var_os("JCODE_RUNTIME_PROVIDER");
        let auth = AuthStatus {
            anthropic: ProviderAuth {
                state: AuthState::Available,
                has_oauth: true,
                oauth_state: AuthState::Available,
                has_api_key: true,
            },
            ..AuthStatus::default()
        };

        let rendered_with = |runtime: Option<&str>| {
            match runtime {
                Some(value) => crate::env::set_var("JCODE_RUNTIME_PROVIDER", value),
                None => crate::env::remove_var("JCODE_RUNTIME_PROVIDER"),
            }
            build_auth_status_line(&auth, 120)
                .spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        };

        // Auto prefers OAuth: the star must sit on oauth, matching the header
        // provider tag's active-route answer.
        let rendered = rendered_with(None);
        assert!(
            rendered.contains("anthropic(oauth*+key)"),
            "rendered: {rendered}"
        );

        // Pinning the API key moves the star, keeping both surfaces consistent.
        let rendered = rendered_with(Some("claude-api"));
        assert!(
            rendered.contains("anthropic(oauth+key*)"),
            "rendered: {rendered}"
        );

        match prev {
            Some(value) => crate::env::set_var("JCODE_RUNTIME_PROVIDER", value),
            None => crate::env::remove_var("JCODE_RUNTIME_PROVIDER"),
        }
    }

    #[test]
    fn format_model_name_labels_slashed_models_with_active_provider() {
        // Regression for issue #329: a NVIDIA NIM model must be labeled with the
        // active provider's display name, not the fixed "OpenRouter" aggregator.
        assert_eq!(
            format_model_name("nvidia/nemotron-3-super-120b-a12b", "NVIDIA NIM"),
            "NVIDIA NIM: nvidia/nemotron-3-super-120b-a12b"
        );
        // The public aggregator still reads "OpenRouter".
        assert_eq!(
            format_model_name("anthropic/claude-sonnet-4", "OpenRouter"),
            "OpenRouter: anthropic/claude-sonnet-4"
        );
        // Missing provider name falls back to "OpenRouter" rather than an empty label.
        assert_eq!(
            format_model_name("deepseek/deepseek-chat", ""),
            "OpenRouter: deepseek/deepseek-chat"
        );
        // Non-slashed models are unaffected by the provider label.
        assert_eq!(
            format_model_name("claude-opus-4-6", "OpenRouter"),
            "Claude Opus"
        );
    }
}
