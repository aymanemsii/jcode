#![cfg_attr(test, allow(clippy::await_holding_lock))]

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::net::ToSocketAddrs;
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, Stdio};

use crate::{browser, gateway, memory, session, storage, tui};

use super::terminal::init_tui_runtime;

mod menubar;
mod provider_setup;
mod queue;
mod report_info;
mod restart;

pub(crate) use super::auth_test::run_post_login_validation;
#[cfg(test)]
pub(crate) use super::auth_test::{
    AuthTestChoicePlan, AuthTestTarget, ResolvedAuthTestTarget, auth_test_choice_plan,
    auth_test_error_is_retryable, configured_auth_test_targets, resolve_auth_test_targets,
};
pub use super::auth_test::{
    run_auth_test_command, run_auth_test_context_audit_command, run_auth_test_coverage_command,
};
pub use menubar::{ensure_menubar_helper_running, run_menubar_command};
pub(crate) use provider_setup::{ProviderAddOptions, run_provider_add_command};
pub(crate) use queue::{QueueSubcommand, run_queue_command};
pub use restart::{
    maybe_run_pending_restart_restore_on_startup, run_restart_clear_command,
    run_restart_restore_command, run_restart_save_command, run_restart_status_command,
};

pub enum AmbientSubcommand {
    Status,
    Log,
    Trigger,
    Stop,
    RunVisible,
}

pub enum CloudSubcommand {
    Sessions(CloudSessionsSubcommand),
}

const DEFAULT_ACCENT_COLOR_HEX: &str = "#BA8BFF";

pub enum CloudSessionsSubcommand {
    Configure {
        api_base: Option<String>,
        api_token: Option<String>,
        api_token_env: Option<String>,
        api_token_id: Option<String>,
        user_id: Option<String>,
        helper: Option<String>,
        clear: bool,
    },
    Status {
        json: bool,
    },
    Upload {
        session_file: String,
        raw: bool,
        user_id: String,
        profile: Option<String>,
        region: Option<String>,
        helper: Option<String>,
    },
    UploadLatest {
        sessions_dir: String,
        raw: bool,
        user_id: String,
        profile: Option<String>,
        region: Option<String>,
        helper: Option<String>,
    },
    Sync {
        sessions_dir: Option<String>,
        since_days: Option<u64>,
        all: bool,
        max: usize,
        min_interval_mins: Option<u64>,
        raw: bool,
        dry_run: bool,
        force: bool,
        json: bool,
        user_id: String,
        profile: Option<String>,
        region: Option<String>,
        helper: Option<String>,
    },
    List {
        limit: usize,
        json: bool,
        user_id: String,
        profile: Option<String>,
        region: Option<String>,
        helper: Option<String>,
    },
    Verify {
        session_id: String,
        user_id: String,
        profile: Option<String>,
        region: Option<String>,
        helper: Option<String>,
    },
    Dashboard {
        limit: usize,
        output: Option<String>,
        open: bool,
        with_view: bool,
        user_id: String,
        profile: Option<String>,
        region: Option<String>,
        helper: Option<String>,
    },
    View {
        session_id: String,
        format: String,
        output: Option<String>,
        open: bool,
        user_id: String,
        profile: Option<String>,
        region: Option<String>,
        helper: Option<String>,
    },
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct CloudSessionsConfig {
    api_base: Option<String>,
    api_token: Option<String>,
    api_token_id: Option<String>,
    helper: Option<String>,
    user_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct CloudSessionsConfigStatus {
    path: String,
    exists: bool,
    api_base: Option<String>,
    api_token_configured: bool,
    api_token_id: Option<String>,
    helper: Option<String>,
    user_id: Option<String>,
}

pub fn run_cloud_command(cmd: CloudSubcommand) -> Result<()> {
    match cmd {
        CloudSubcommand::Sessions(action) => run_cloud_sessions_command(action),
    }
}

fn run_cloud_sessions_command(action: CloudSessionsSubcommand) -> Result<()> {
    match action {
        CloudSessionsSubcommand::Configure {
            api_base,
            api_token,
            api_token_env,
            api_token_id,
            user_id,
            helper,
            clear,
        } => run_cloud_sessions_configure(
            api_base,
            api_token,
            api_token_env,
            api_token_id,
            user_id,
            helper,
            clear,
        ),
        CloudSessionsSubcommand::Status { json } => run_cloud_sessions_status(json),
        CloudSessionsSubcommand::Dashboard {
            limit,
            output,
            open,
            with_view,
            user_id,
            profile,
            region,
            helper,
        } => run_cloud_sessions_dashboard(CloudSessionsDashboardRequest {
            limit,
            output,
            open,
            with_view,
            user_id,
            profile,
            region,
            helper,
        }),
        CloudSessionsSubcommand::Sync {
            sessions_dir,
            since_days,
            all,
            max,
            min_interval_mins,
            raw,
            dry_run,
            force,
            json,
            user_id,
            profile,
            region,
            helper,
        } => run_cloud_sessions_sync(CloudSessionsSyncRequest {
            sessions_dir,
            since_days,
            all,
            max,
            min_interval_mins,
            raw,
            dry_run,
            force,
            json,
            user_id,
            profile,
            region,
            helper,
        }),
        other => run_cloud_sessions_helper_command(other),
    }
}

fn run_cloud_sessions_helper_command(action: CloudSessionsSubcommand) -> Result<()> {
    let config = load_cloud_sessions_config()?.unwrap_or_default();
    let helper_override = cloud_sessions_helper_override(&action).or_else(|| config.helper.clone());
    let helper = resolve_jade_sessions_helper(helper_override.as_deref())?;
    let helper_env = cloud_sessions_helper_env(&config);
    let args = build_jade_sessions_args_with_config(action, &config);
    let mut command = ProcessCommand::new(&helper);
    command
        .args(&args)
        .envs(helper_env)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    let status = command
        .status()
        .map_err(|err| anyhow::anyhow!("failed to run {}: {err}", helper.display()))?;

    if !status.success() {
        anyhow::bail!("{} exited with status {status}", helper.display());
    }
    Ok(())
}

fn cloud_sessions_config_path() -> Result<PathBuf> {
    Ok(crate::storage::jcode_dir()?.join("cloud_sessions.json"))
}

fn load_cloud_sessions_config() -> Result<Option<CloudSessionsConfig>> {
    let path = cloud_sessions_config_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|err| anyhow::anyhow!("failed to read {}: {err}", path.display()))?;
    let config = serde_json::from_str(&content)
        .map_err(|err| anyhow::anyhow!("failed to parse {}: {err}", path.display()))?;
    Ok(Some(config))
}

fn save_cloud_sessions_config(config: &CloudSessionsConfig) -> Result<PathBuf> {
    let path = cloud_sessions_config_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_vec_pretty(config)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .mode(0o600)
            .open(&path)?;
        file.write_all(&content)?;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }
    #[cfg(not(unix))]
    {
        std::fs::write(&path, &content)?;
    }
    Ok(path)
}

fn run_cloud_sessions_configure(
    api_base: Option<String>,
    api_token: Option<String>,
    api_token_env: Option<String>,
    api_token_id: Option<String>,
    user_id: Option<String>,
    helper: Option<String>,
    clear: bool,
) -> Result<()> {
    let path = cloud_sessions_config_path()?;
    if clear {
        match std::fs::remove_file(&path) {
            Ok(()) => println!("Removed Jade cloud sessions config at {}", path.display()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                println!("No Jade cloud sessions config found at {}", path.display());
            }
            Err(err) => return Err(err.into()),
        }
        return Ok(());
    }

    if api_base.is_none()
        && api_token.is_none()
        && api_token_env.is_none()
        && api_token_id.is_none()
        && user_id.is_none()
        && helper.is_none()
    {
        anyhow::bail!(
            "nothing to configure; pass --api-base, --api-token/--api-token-env, --api-token-id, --user-id, --helper, or --clear"
        );
    }

    let mut config = load_cloud_sessions_config()?.unwrap_or_default();
    if let Some(value) = non_empty(api_base) {
        config.api_base = Some(value);
    }
    if let Some(value) = non_empty(api_token) {
        config.api_token = Some(value);
    }
    if let Some(var) = non_empty(api_token_env) {
        let value = std::env::var(&var)
            .map_err(|err| anyhow::anyhow!("failed to read {var} for --api-token-env: {err}"))?;
        let value = non_empty(Some(value))
            .ok_or_else(|| anyhow::anyhow!("{var} for --api-token-env was empty"))?;
        config.api_token = Some(value);
    }
    if let Some(value) = non_empty(api_token_id) {
        config.api_token_id = Some(value);
    }
    if let Some(value) = non_empty(user_id) {
        config.user_id = Some(value);
    }
    if let Some(value) = non_empty(helper) {
        config.helper = Some(value);
    }

    let path = save_cloud_sessions_config(&config)?;
    println!("Saved Jade cloud sessions config to {}", path.display());
    println!("api_base: {}", configured_label(config.api_base.as_deref()));
    println!(
        "api_token: {}",
        if config.api_token.is_some() {
            "configured"
        } else {
            "not configured"
        }
    );
    println!(
        "api_token_id: {}",
        configured_label(config.api_token_id.as_deref())
    );
    println!("user_id: {}", configured_label(config.user_id.as_deref()));
    println!("helper: {}", configured_label(config.helper.as_deref()));
    Ok(())
}

fn run_cloud_sessions_status(json: bool) -> Result<()> {
    let path = cloud_sessions_config_path()?;
    let config = load_cloud_sessions_config()?.unwrap_or_default();
    let status = CloudSessionsConfigStatus {
        path: path.display().to_string(),
        exists: path.exists(),
        api_base: config.api_base,
        api_token_configured: config.api_token.is_some(),
        api_token_id: config.api_token_id,
        helper: config.helper,
        user_id: config.user_id,
    };
    if json {
        println!("{}", serde_json::to_string_pretty(&status)?);
    } else {
        println!("Jade cloud sessions config: {}", status.path);
        println!("exists: {}", status.exists);
        println!("api_base: {}", configured_label(status.api_base.as_deref()));
        println!(
            "api_token: {}",
            if status.api_token_configured {
                "configured"
            } else {
                "not configured"
            }
        );
        println!(
            "api_token_id: {}",
            configured_label(status.api_token_id.as_deref())
        );
        println!("user_id: {}", configured_label(status.user_id.as_deref()));
        println!("helper: {}", configured_label(status.helper.as_deref()));
    }
    Ok(())
}

fn non_empty(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn run_config_show_command() -> Result<()> {
    print!("{}", render_config_show(crate::config::config()));
    Ok(())
}

pub fn run_config_edit_command() -> Result<()> {
    let config_path = ensure_global_config_file()?;
    let editor = resolve_config_editor("jcode config edit")?;
    let mut command = ProcessCommand::new(&editor.program);
    command.args(&editor.args).arg(&config_path);

    let status = command.status().map_err(|err| {
        anyhow::anyhow!(
            "Failed to launch editor `{}` for {}: {}",
            editor.display(),
            config_path.display(),
            err
        )
    })?;

    if !status.success() {
        anyhow::bail!(
            "Editor `{}` exited unsuccessfully while editing {}{}",
            editor.display(),
            config_path.display(),
            status
                .code()
                .map(|code| format!(" (exit code {code})"))
                .unwrap_or_default()
        );
    }

    Ok(())
}

fn ensure_global_config_file() -> Result<PathBuf> {
    let config_path = crate::config::Config::path()
        .ok_or_else(|| anyhow::anyhow!("Could not determine jcode config path"))?;

    if config_path.exists() {
        return Ok(config_path);
    }

    crate::config::Config::create_default_config_file()
}

pub fn run_workspace_show_command() -> Result<()> {
    print!("{}", render_workspace_show()?);
    Ok(())
}

pub fn run_workspace_init_command() -> Result<()> {
    let workspace_path = create_workspace_config_file(false)?;
    println!("Created workspace config at {}", workspace_path.display());
    Ok(())
}

pub fn run_workspace_edit_command() -> Result<()> {
    let workspace_path = ensure_workspace_config_file()?;
    let editor = resolve_config_editor("jcode workspace edit")?;
    let mut command = ProcessCommand::new(&editor.program);
    command.args(&editor.args).arg(&workspace_path);

    let status = command.status().map_err(|err| {
        anyhow::anyhow!(
            "Failed to launch editor `{}` for {}: {}",
            editor.display(),
            workspace_path.display(),
            err
        )
    })?;

    if !status.success() {
        anyhow::bail!(
            "Editor `{}` exited unsuccessfully while editing {}{}",
            editor.display(),
            workspace_path.display(),
            status
                .code()
                .map(|code| format!(" (exit code {code})"))
                .unwrap_or_default()
        );
    }

    Ok(())
}

pub fn run_theme_list_command() -> Result<()> {
    print!("{}", render_theme_list(crate::config::config()));
    Ok(())
}

pub fn run_theme_current_command() -> Result<()> {
    print!("{}", render_theme_current(crate::config::config()));
    Ok(())
}

pub fn run_theme_preview_command(theme_name: Option<&str>) -> Result<()> {
    let config = crate::config::config();
    let requested = theme_name.and_then(|name| {
        let trimmed = name.trim();
        (!trimmed.is_empty()).then_some(trimmed)
    });

    if let Some(name) = requested
        && !theme_name_is_available(config, name)
    {
        anyhow::bail!(
            "Unknown theme `{}`.\nAvailable themes:\n{}",
            name,
            available_theme_names(config)
                .into_iter()
                .map(|name| format!("  {name}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    print!("{}", render_theme_preview(config, requested));
    Ok(())
}

fn ensure_workspace_config_file() -> Result<PathBuf> {
    for workspace_path in current_directory_workspace_config_paths()? {
        if workspace_path.exists() {
            return Ok(workspace_path);
        }
    }
    create_workspace_config_file(false)
}

fn create_workspace_config_file(overwrite: bool) -> Result<PathBuf> {
    let mercury_workspace_path = current_directory_mercury_workspace_config_path()?;
    let jcode_workspace_path = current_directory_jcode_workspace_config_path()?;

    if mercury_workspace_path.exists() && !overwrite {
        anyhow::bail!(
            "Workspace config already exists at {}. Edit it with `jcode workspace edit`.",
            mercury_workspace_path.display()
        );
    }

    if jcode_workspace_path.exists() && !mercury_workspace_path.exists() && !overwrite {
        anyhow::bail!(
            "Workspace config already exists at {}. Edit it with `jcode workspace edit`; not creating duplicate {}.",
            jcode_workspace_path.display(),
            mercury_workspace_path.display()
        );
    }

    if let Some(parent) = mercury_workspace_path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| {
            anyhow::anyhow!(
                "Failed to create workspace config directory {}: {err}",
                parent.display()
            )
        })?;
    }

    let contents = workspace_default_config_contents();
    std::fs::write(&mercury_workspace_path, contents).map_err(|err| {
        anyhow::anyhow!(
            "Failed to write workspace config file {}: {err}",
            mercury_workspace_path.display()
        )
    })?;
    Ok(mercury_workspace_path)
}

fn current_directory_workspace_config_paths() -> Result<[PathBuf; 2]> {
    Ok([
        current_directory_mercury_workspace_config_path()?,
        current_directory_jcode_workspace_config_path()?,
    ])
}

fn current_directory_mercury_workspace_config_path() -> Result<PathBuf> {
    current_directory_workspace_config_path(".mercury")
}

fn current_directory_jcode_workspace_config_path() -> Result<PathBuf> {
    current_directory_workspace_config_path(".jcode")
}

fn current_directory_workspace_config_path(workspace_dir_name: &str) -> Result<PathBuf> {
    Ok(std::env::current_dir()
        .map_err(|err| anyhow::anyhow!("Could not determine current directory: {err}"))?
        .join(workspace_dir_name)
        .join("workspace.toml"))
}

fn workspace_default_config_contents() -> String {
    let name = std::env::current_dir()
        .ok()
        .and_then(|path| path.file_name().map(|name| name.to_string_lossy().into_owned()))
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| "workspace".to_string());

    format!(
        "[workspace]\nname = \"{}\"\n\n[display]\ntheme = \"cursor\"\ntop_bar = true\ntop_bar_items = [\"app\", \"session\", \"theme\", \"repo\"]\n",
        toml_basic_string_escape(&name)
    )
}

fn toml_basic_string_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

#[derive(Debug)]
struct EditorCommand {
    program: String,
    args: Vec<String>,
}

impl EditorCommand {
    fn display(&self) -> String {
        self.program.clone()
    }
}

fn resolve_config_editor(retry_command: &'static str) -> Result<EditorCommand> {
    if let Some(editor) = non_empty(std::env::var("VISUAL").ok()) {
        return Ok(editor_command(editor));
    }
    if let Some(editor) = non_empty(std::env::var("EDITOR").ok()) {
        return Ok(editor_command(editor));
    }

    #[cfg(windows)]
    {
        Ok(EditorCommand {
            program: "notepad".to_string(),
            args: Vec::new(),
        })
    }

    #[cfg(not(windows))]
    {
        anyhow::bail!(
            "No editor configured. Set VISUAL or EDITOR, then run `{retry_command}` again."
        )
    }
}

fn editor_command(program: String) -> EditorCommand {
    EditorCommand {
        program,
        args: Vec::new(),
    }
}

fn render_config_show(config: &crate::config::Config) -> String {
    let app_label = app_identity_label(config);
    let home_env = crate::storage::config_home_env_resolution();
    let config_home_source_label = home_env.explicit_source.unwrap_or("default");
    let config_home_warning_label = if home_env.has_conflict() {
        "MERCURY_HOME and JCODE_HOME are both set; using MERCURY_HOME"
    } else {
        "none"
    };
    let workspace_config_label = config
        .workspace_config
        .path
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "not found".to_string());
    let workspace_config_source_label = config
        .workspace_config
        .source
        .or_else(|| {
            config
                .workspace_config
                .path
                .as_deref()
                .and_then(workspace_source_from_path)
        })
        .map(crate::config::WorkspaceConfigSource::label)
        .unwrap_or("not found");
    let workspace_config_location_label = config
        .workspace_config
        .path
        .as_ref()
        .map(|path| workspace_location_label(path))
        .unwrap_or("not found");
    let workspace_name_label = compact_optional_text_label(config.workspace.name.as_deref());
    let workspace_display_overrides_label = if config.workspace_config.display_overrides.is_empty()
    {
        "none".to_string()
    } else {
        config.workspace_config.display_overrides.join(", ")
    };
    let app_name_label = compact_optional_text_label(config.app.name.as_deref());
    let app_terminal_title_label =
        compact_optional_text_label(config.app.terminal_title.as_deref());
    let configured_theme = config.display.theme.as_deref();
    let custom_themes = custom_theme_palettes(config);
    let resolved_theme = jcode_tui_style::theme::resolve_theme_palette(
        configured_theme,
        &custom_themes,
    );
    let active_theme = resolved_theme.name.as_str();
    let theme_source = match resolved_theme.source {
        jcode_tui_style::theme::ThemeSource::BuiltIn => "built-in",
        jcode_tui_style::theme::ThemeSource::Custom => "custom",
        jcode_tui_style::theme::ThemeSource::DefaultFallback => "default/fallback",
    };
    let built_in_themes = jcode_tui_style::theme::BUILT_IN_THEME_NAMES.join(", ");
    let theme_accent = jcode_tui_style::theme::color_to_hex(resolved_theme.palette.accent);
    let configured_theme_label = match configured_theme.map(str::trim) {
        Some("") => "(empty)".to_string(),
        Some(_)
            if !matches!(
                resolved_theme.source,
                jcode_tui_style::theme::ThemeSource::DefaultFallback
            ) =>
        {
            active_theme.to_string()
        }
        Some(value) => value.to_string(),
        None => "not configured".to_string(),
    };
    let theme_valid_label = match configured_theme {
        Some(value)
            if !matches!(
                resolved_theme.source,
                jcode_tui_style::theme::ThemeSource::DefaultFallback
            ) && !value.trim().is_empty() =>
        {
            "true"
        }
        Some(_) => "false",
        None => "not configured",
    };
    let custom_theme_invalid_label = match resolved_theme.source {
        jcode_tui_style::theme::ThemeSource::Custom => {
            resolved_theme.invalid_custom_color_count.to_string()
        }
        _ => "not applicable".to_string(),
    };

    let configured_accent = config.display.accent_color.as_deref();
    let parsed_accent = configured_accent.and_then(parse_accent_color_hex);
    let active_accent = parsed_accent.as_deref().unwrap_or(&theme_accent);
    let configured_accent_label = match configured_accent.map(str::trim) {
        Some("") => "(empty)",
        Some(value) => value,
        None => "not configured",
    };
    let accent_valid_label = match configured_accent {
        Some(_) if parsed_accent.is_some() => "true",
        Some(_) => "false",
        None => "not configured",
    };
    let fallback_label = match configured_accent {
        Some(_) if parsed_accent.is_some() => {
            "not used (display.accent_color overrides active theme accent)"
        }
        Some(_) => {
            "active theme/default accent is being used (configured accent_color is invalid)"
        }
        None => "active theme/default accent is being used (no accent_color configured)",
    };
    let startup_splash_label = match config.display.startup_splash {
        Some(true) => "true",
        Some(false) => "false",
        None => "not configured",
    };
    let startup_splash_title_label =
        compact_optional_text_label(config.display.startup_splash_title.as_deref());
    let startup_splash_subtitle_label =
        compact_optional_text_label(config.display.startup_splash_subtitle.as_deref());
    let startup_splash_footer_label =
        compact_optional_text_label(config.display.startup_splash_footer.as_deref());
    let background_style_label =
        compact_background_style_label(config.display.background_style.as_deref());
    let background_style_valid_label =
        compact_background_style_valid_label(config.display.background_style.as_deref());
    let background_opacity_label =
        compact_background_opacity_label(config.display.background_opacity);
    let active_background_opacity_label =
        format!("{:.2}", resolved_background_opacity(config.display.background_opacity));
    let top_bar_label = compact_optional_bool_label(config.display.top_bar);
    let top_bar_items_label = compact_top_bar_items_label(config.display.top_bar_items.as_deref());
    let top_bar_items_ignored_label =
        compact_top_bar_items_ignored_label(config.display.top_bar_items.as_deref());

    format!(
        "{app_label} customization config:\n\
config home source: {config_home_source_label}\n\
config home warning: {config_home_warning_label}\n\
workspace config: {workspace_config_label}\n\
workspace config source: {workspace_config_source_label}\n\
workspace config location: {workspace_config_location_label}\n\
workspace.name: {workspace_name_label}\n\
project-local display overrides: {workspace_display_overrides_label}\n\
app.name: {app_name_label}\n\
app.terminal_title: {app_terminal_title_label}\n\
display.theme: {configured_theme_label}\n\
theme valid: {theme_valid_label}\n\
active theme: {active_theme}\n\
theme source: {theme_source}\n\
custom theme invalid colors: {custom_theme_invalid_label}\n\
theme accent color: {theme_accent}\n\
display.accent_color: {configured_accent_label}\n\
accent_color valid: {accent_valid_label}\n\
active accent color: {active_accent}\n\
display.startup_splash: {startup_splash_label}\n\
display.startup_splash_title: {startup_splash_title_label}\n\
display.startup_splash_subtitle: {startup_splash_subtitle_label}\n\
display.startup_splash_footer: {startup_splash_footer_label}\n\
display.background_style: {background_style_label}\n\
background_style valid: {background_style_valid_label}\n\
display.background_opacity: {background_opacity_label}\n\
active background_opacity: {active_background_opacity_label}\n\
display.top_bar: {top_bar_label}\n\
display.top_bar_items: {top_bar_items_label}\n\
display.top_bar_items_ignored: {top_bar_items_ignored_label}\n\
built-in themes: {built_in_themes}\n\
built-in default accent color: {DEFAULT_ACCENT_COLOR_HEX}\n\
fallback: {fallback_label}\n"
    )
}

fn render_workspace_show() -> Result<String> {
    let current_workspace_path = current_directory_mercury_workspace_config_path()?;
    let Some(workspace) = crate::config::Config::workspace_path_with_source() else {
        return Ok(format!(
            "Workspace config: not found\n\
current-directory path: {}\n\
\n\
Create one with `jcode workspace init`, or open a new local workspace file with `jcode workspace edit`.\n",
            current_workspace_path.display()
        ));
    };
    let workspace_source = workspace.source;
    let workspace_path = workspace.path;

    let content = std::fs::read_to_string(&workspace_path).map_err(|err| {
        anyhow::anyhow!(
            "Failed to read workspace config file {}: {err}",
            workspace_path.display()
        )
    })?;
    let value = toml::from_str::<toml::Value>(&content).map_err(|err| {
        anyhow::anyhow!(
            "Failed to parse workspace config file {}: {err}",
            workspace_path.display()
        )
    })?;

    Ok(render_workspace_show_from_value(
        &workspace_path,
        workspace_source.label(),
        workspace_location_label(&workspace_path),
        &value,
    ))
}

fn workspace_location_label(path: &Path) -> &'static str {
    if workspace_config_root(path)
        .and_then(|workspace_root| {
            std::env::current_dir()
                .ok()
                .map(|cwd| workspace_root == cwd.as_path())
        })
        .unwrap_or(false)
    {
        "current directory"
    } else {
        "parent directory"
    }
}

fn workspace_config_root(path: &Path) -> Option<&Path> {
    path.parent()?.parent()
}

fn workspace_source_from_path(path: &Path) -> Option<crate::config::WorkspaceConfigSource> {
    let source_dir = path.parent()?.file_name()?.to_str()?;
    match source_dir {
        ".mercury" => Some(crate::config::WorkspaceConfigSource::Mercury),
        ".jcode" => Some(crate::config::WorkspaceConfigSource::Jcode),
        _ => None,
    }
}

fn render_workspace_show_from_value(
    path: &Path,
    source: &str,
    location: &str,
    value: &toml::Value,
) -> String {
    format!(
        "Workspace config: found\n\
path: {}\n\
source: {}\n\
location: {}\n\
workspace.name: {}\n\
\n\
Supported project-local display fields:\n\
display.theme: {}\n\
display.accent_color: {}\n\
display.startup_splash_title: {}\n\
display.startup_splash_subtitle: {}\n\
display.startup_splash_footer: {}\n\
display.background_style: {}\n\
display.background_opacity: {}\n\
display.top_bar: {}\n\
display.top_bar_items: {}\n",
        path.display(),
        source,
        location,
        toml_string_label(value, &["workspace", "name"]),
        toml_string_label(value, &["display", "theme"]),
        toml_string_label(value, &["display", "accent_color"]),
        toml_string_label(value, &["display", "startup_splash_title"]),
        toml_string_label(value, &["display", "startup_splash_subtitle"]),
        toml_string_label(value, &["display", "startup_splash_footer"]),
        toml_string_label(value, &["display", "background_style"]),
        toml_float_label(value, &["display", "background_opacity"]),
        toml_bool_label(value, &["display", "top_bar"]),
        toml_string_array_label(value, &["display", "top_bar_items"]),
    )
}

fn render_theme_list(config: &crate::config::Config) -> String {
    let app_label = app_identity_label(config);
    let mut output = format!("{app_label} themes:\n\nBuilt-in themes:\n");
    for name in jcode_tui_style::theme::BUILT_IN_THEME_NAMES {
        output.push_str("  ");
        output.push_str(name);
        output.push('\n');
    }

    output.push_str("\nGlobal custom themes:\n");
    if config.themes.is_empty() {
        output.push_str("  none\n");
    } else {
        for name in config.themes.keys() {
            output.push_str("  ");
            output.push_str(name);
            if jcode_tui_style::theme::canonical_theme_name(Some(name)).is_some() {
                output.push_str(" (reserved built-in name; built-in wins)");
            }
            output.push('\n');
        }
    }

    output.push('\n');
    output.push_str(&workspace_theme_influence_summary(config));
    output
}

fn render_theme_current(config: &crate::config::Config) -> String {
    let custom_themes = custom_theme_palettes(config);
    let resolved =
        jcode_tui_style::theme::resolve_theme_palette(config.display.theme.as_deref(), &custom_themes);
    let theme_accent = jcode_tui_style::theme::color_to_hex(resolved.palette.accent);
    let active_accent = active_accent_color_hex(config, &theme_accent);
    let app_label = app_identity_label(config);

    format!(
        "{app_label} current theme:\n\
active theme: {}\n\
theme source: {}\n\
theme valid: {}\n\
active accent color: {}\n\
project-local workspace config: {}\n\
project-local theme overrides: {}\n",
        resolved.name,
        theme_source_label(resolved.source),
        theme_valid_label(config.display.theme.as_deref(), resolved.source),
        active_accent,
        workspace_config_presence_label(config),
        workspace_theme_override_label(config),
    )
}

fn render_theme_preview(config: &crate::config::Config, requested: Option<&str>) -> String {
    let custom_themes = custom_theme_palettes(config);
    let resolved = jcode_tui_style::theme::resolve_theme_palette(
        requested.or(config.display.theme.as_deref()),
        &custom_themes,
    );
    let mut palette = resolved.palette;
    let mut accent_note = "";
    if requested.is_none()
        && let Some(accent) = config
            .display
            .accent_color
            .as_deref()
            .and_then(parse_hex_rgb_tuple)
    {
        palette.accent = accent;
        accent_note = " (display.accent_color override)";
    }
    let app_label = app_identity_label(config);

    format!(
        "{app_label} theme preview: {}\n\
source: {}\n\
valid: {}\n\
\n\
accent: {}{}\n\
user: {}\n\
assistant: {}\n\
tool: {}\n\
system: {}\n\
muted: {}\n\
success: {}\n\
warning: {}\n\
error: {}\n\
border: {}\n\
active_border: {}\n\
background: {}\n\
panel: {}\n\
input: {}\n",
        resolved.name,
        theme_source_label(resolved.source),
        theme_valid_label(requested.or(config.display.theme.as_deref()), resolved.source),
        jcode_tui_style::theme::color_to_hex(palette.accent),
        accent_note,
        jcode_tui_style::theme::color_to_hex(palette.user),
        jcode_tui_style::theme::color_to_hex(palette.ai),
        jcode_tui_style::theme::color_to_hex(palette.tool),
        jcode_tui_style::theme::color_to_hex(palette.system_message),
        jcode_tui_style::theme::color_to_hex(palette.muted),
        jcode_tui_style::theme::color_to_hex(palette.success),
        jcode_tui_style::theme::color_to_hex(palette.warning),
        jcode_tui_style::theme::color_to_hex(palette.error),
        jcode_tui_style::theme::color_to_hex(palette.border),
        jcode_tui_style::theme::color_to_hex(palette.active_border),
        jcode_tui_style::theme::color_to_hex(palette.background),
        jcode_tui_style::theme::color_to_hex(palette.panel),
        jcode_tui_style::theme::color_to_hex(palette.input),
    )
}

fn app_identity_label(config: &crate::config::Config) -> &str {
    config.app.name().unwrap_or("jcode")
}

fn theme_source_label(source: jcode_tui_style::theme::ThemeSource) -> &'static str {
    match source {
        jcode_tui_style::theme::ThemeSource::BuiltIn => "built-in",
        jcode_tui_style::theme::ThemeSource::Custom => "custom",
        jcode_tui_style::theme::ThemeSource::DefaultFallback => "default/fallback",
    }
}

fn theme_valid_label(
    configured_theme: Option<&str>,
    source: jcode_tui_style::theme::ThemeSource,
) -> &'static str {
    match configured_theme {
        Some(value)
            if !matches!(
                source,
                jcode_tui_style::theme::ThemeSource::DefaultFallback
            ) && !value.trim().is_empty() =>
        {
            "true"
        }
        Some(_) => "false",
        None => "not configured",
    }
}

fn active_accent_color_hex(config: &crate::config::Config, theme_accent: &str) -> String {
    config
        .display
        .accent_color
        .as_deref()
        .and_then(parse_accent_color_hex)
        .unwrap_or_else(|| theme_accent.to_string())
}

fn parse_hex_rgb_tuple(value: &str) -> Option<(u8, u8, u8)> {
    let hex = value.trim().strip_prefix('#').unwrap_or(value.trim());
    if hex.len() != 6 || !hex.as_bytes().iter().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some((r, g, b))
}

fn workspace_theme_influence_summary(config: &crate::config::Config) -> String {
    let presence = workspace_config_presence_label(config);
    let overrides = workspace_theme_override_label(config);
    if overrides == "none" {
        format!(
            "Project-local workspace config: {presence}\nProject-local theme influence: none\n"
        )
    } else {
        format!(
            "Project-local workspace config: {presence}\nProject-local theme influence: {overrides} may affect current theme selection/accent\n"
        )
    }
}

fn workspace_config_presence_label(config: &crate::config::Config) -> String {
    config
        .workspace_config
        .path
        .as_ref()
        .map(|path| format!("found ({})", path.display()))
        .unwrap_or_else(|| "not found".to_string())
}

fn workspace_theme_override_label(config: &crate::config::Config) -> String {
    let overrides = config
        .workspace_config
        .display_overrides
        .iter()
        .filter(|name| matches!(name.as_str(), "display.theme" | "display.accent_color"))
        .cloned()
        .collect::<Vec<_>>();
    if overrides.is_empty() {
        "none".to_string()
    } else {
        overrides.join(", ")
    }
}

fn theme_name_is_available(config: &crate::config::Config, name: &str) -> bool {
    jcode_tui_style::theme::canonical_theme_name(Some(name)).is_some()
        || config
            .themes
            .keys()
            .any(|custom| custom.trim().eq_ignore_ascii_case(name.trim()))
}

fn available_theme_names(config: &crate::config::Config) -> Vec<String> {
    let mut names = BTreeSet::new();
    for name in jcode_tui_style::theme::BUILT_IN_THEME_NAMES {
        names.insert((*name).to_string());
    }
    for name in config.themes.keys() {
        if jcode_tui_style::theme::canonical_theme_name(Some(name)).is_none() {
            names.insert(name.to_string());
        }
    }
    names.into_iter().collect()
}

fn toml_string_label(value: &toml::Value, path: &[&str]) -> String {
    match toml_value_at_path(value, path) {
        Some(toml::Value::String(value)) if value.trim().is_empty() => {
            "(empty/fallback)".to_string()
        }
        Some(toml::Value::String(value)) => value.clone(),
        Some(_) => "(invalid type)".to_string(),
        None => "not configured".to_string(),
    }
}

fn toml_bool_label(value: &toml::Value, path: &[&str]) -> &'static str {
    match toml_value_at_path(value, path) {
        Some(toml::Value::Boolean(true)) => "true",
        Some(toml::Value::Boolean(false)) => "false",
        Some(_) => "(invalid type)",
        None => "not configured",
    }
}

fn toml_float_label(value: &toml::Value, path: &[&str]) -> String {
    match toml_value_at_path(value, path) {
        Some(toml::Value::Float(value)) => value.to_string(),
        Some(toml::Value::Integer(value)) => value.to_string(),
        Some(_) => "(invalid type)".to_string(),
        None => "not configured".to_string(),
    }
}

fn toml_string_array_label(value: &toml::Value, path: &[&str]) -> String {
    match toml_value_at_path(value, path) {
        Some(toml::Value::Array(values)) => {
            let strings = values
                .iter()
                .filter_map(toml::Value::as_str)
                .map(str::trim)
                .collect::<Vec<_>>();
            if strings.len() != values.len() {
                "(invalid type)".to_string()
            } else if strings.is_empty() {
                "(empty; renders no items)".to_string()
            } else {
                strings.join(", ")
            }
        }
        Some(_) => "(invalid type)".to_string(),
        None => "not configured".to_string(),
    }
}

fn toml_value_at_path<'a>(mut value: &'a toml::Value, path: &[&str]) -> Option<&'a toml::Value> {
    for key in path {
        value = value.as_table()?.get(*key)?;
    }
    Some(value)
}

fn compact_optional_text_label(value: Option<&str>) -> &str {
    match value.map(str::trim) {
        Some("") => "(empty/fallback)",
        Some(value) => value,
        None => "not configured",
    }
}

fn compact_optional_bool_label(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "true",
        Some(false) => "false",
        None => "not configured",
    }
}

fn compact_background_style_label(value: Option<&str>) -> String {
    match value.map(str::trim) {
        Some("") => "(empty; active none)".to_string(),
        Some(value) if supported_background_style(value).is_some() => {
            supported_background_style(value).unwrap_or("none").to_string()
        }
        Some(value) => format!("{value} (active none)"),
        None => "not configured (active none)".to_string(),
    }
}

fn compact_background_style_valid_label(value: Option<&str>) -> &'static str {
    match value.map(str::trim) {
        Some(value) if supported_background_style(value).is_some() && !value.is_empty() => "true",
        Some(_) => "false",
        None => "not configured",
    }
}

fn compact_background_opacity_label(value: Option<f32>) -> String {
    match value {
        Some(value) if value.is_finite() && (0.0..=1.0).contains(&value) => format!("{value:.2}"),
        Some(value) if value.is_finite() => {
            format!(
                "{value:.2} (clamped to {:.2})",
                resolved_background_opacity(Some(value))
            )
        }
        Some(_) => "invalid (fallback 0.15)".to_string(),
        None => "not configured (fallback 0.15)".to_string(),
    }
}

fn resolved_background_opacity(value: Option<f32>) -> f32 {
    value.filter(|value| value.is_finite())
        .unwrap_or(0.15)
        .clamp(0.0, 1.0)
}

fn supported_background_style(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "none" => Some("none"),
        "subtle-grid" => Some("subtle-grid"),
        "stars" => Some("stars"),
        "matrix" => Some("matrix"),
        _ => None,
    }
}

fn compact_top_bar_items_label(items: Option<&[String]>) -> String {
    let Some(items) = items else {
        return "not configured".to_string();
    };
    if items.is_empty() {
        return "(empty; renders no items)".to_string();
    }

    let supported = items
        .iter()
        .map(|item| item.trim())
        .filter(|item| is_supported_top_bar_item(item))
        .collect::<Vec<_>>();
    if supported.is_empty() {
        "(no supported items)".to_string()
    } else {
        supported.join(", ")
    }
}

fn compact_top_bar_items_ignored_label(items: Option<&[String]>) -> String {
    let Some(items) = items else {
        return "none".to_string();
    };
    let ignored = items
        .iter()
        .map(|item| item.trim())
        .filter(|item| !is_supported_top_bar_item(item))
        .map(|item| {
            if item.is_empty() {
                "(empty)".to_string()
            } else {
                item.to_string()
            }
        })
        .collect::<Vec<_>>();
    if ignored.is_empty() {
        "none".to_string()
    } else {
        ignored.join(", ")
    }
}

fn is_supported_top_bar_item(item: &str) -> bool {
    matches!(item, "app" | "session" | "theme" | "repo")
}

fn custom_theme_palettes(
    config: &crate::config::Config,
) -> Vec<jcode_tui_style::theme::CustomThemePalette<'_>> {
    config
        .themes
        .iter()
        .map(|(name, theme)| jcode_tui_style::theme::CustomThemePalette {
            name: name.as_str(),
            accent: theme.accent.as_deref(),
            user: theme.user.as_deref(),
            assistant: theme.assistant.as_deref(),
            tool: theme.tool.as_deref(),
            system: theme.system.as_deref(),
            queued: theme.queued.as_deref(),
            asap: theme.asap.as_deref(),
            pending: theme.pending.as_deref(),
            background: theme.background.as_deref(),
            foreground: theme.foreground.as_deref(),
            muted: theme.muted.as_deref(),
            border: theme.border.as_deref(),
            active_border: theme.active_border.as_deref(),
            panel: theme.panel.as_deref(),
            input: theme.input.as_deref(),
            selection: theme.selection.as_deref(),
            success: theme.success.as_deref(),
            warning: theme.warning.as_deref(),
            error: theme.error.as_deref(),
        })
        .collect()
}

fn parse_accent_color_hex(value: &str) -> Option<String> {
    let hex = value.trim().strip_prefix('#').unwrap_or(value.trim());
    if hex.len() != 6 || !hex.as_bytes().iter().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    Some(format!("#{}", hex.to_ascii_uppercase()))
}

struct CloudSessionsSyncRequest {
    sessions_dir: Option<String>,
    since_days: Option<u64>,
    all: bool,
    max: usize,
    min_interval_mins: Option<u64>,
    raw: bool,
    dry_run: bool,
    force: bool,
    json: bool,
    user_id: String,
    profile: Option<String>,
    region: Option<String>,
    helper: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct CloudSessionsSyncState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_sync_at: Option<String>,
    #[serde(default)]
    sessions: std::collections::BTreeMap<String, CloudSessionsSyncRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CloudSessionsSyncRecord {
    sha256: String,
    size: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    modified_unix: Option<i64>,
    uploaded_at: String,
}

#[derive(Debug, Serialize)]
struct CloudSessionsSyncEntry {
    session_id: String,
    path: String,
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct CloudSessionsSyncReport {
    sessions_dir: String,
    dry_run: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    throttled: bool,
    scanned: usize,
    uploaded: usize,
    skipped_unchanged: usize,
    failed: usize,
    reached_max: bool,
    entries: Vec<CloudSessionsSyncEntry>,
}

struct SyncCandidate {
    session_id: String,
    path: PathBuf,
    size: u64,
    modified_unix: Option<i64>,
}

fn cloud_sessions_sync_state_path() -> Result<PathBuf> {
    Ok(crate::storage::jcode_dir()?.join("cloud_sessions_sync.json"))
}

fn load_cloud_sessions_sync_state() -> Result<CloudSessionsSyncState> {
    let path = cloud_sessions_sync_state_path()?;
    if !path.exists() {
        return Ok(CloudSessionsSyncState::default());
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|err| anyhow::anyhow!("failed to read {}: {err}", path.display()))?;
    serde_json::from_str(&content)
        .map_err(|err| anyhow::anyhow!("failed to parse {}: {err}", path.display()))
}

fn save_cloud_sessions_sync_state(state: &CloudSessionsSyncState) -> Result<PathBuf> {
    let path = cloud_sessions_sync_state_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_vec_pretty(state)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .mode(0o600)
            .open(&path)?;
        file.write_all(&content)?;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }
    #[cfg(not(unix))]
    {
        std::fs::write(&path, &content)?;
    }
    Ok(path)
}

fn resolve_sync_sessions_dir(override_path: Option<&str>) -> Result<PathBuf> {
    if let Some(path) = override_path.map(str::trim).filter(|path| !path.is_empty()) {
        return Ok(expand_home_path(path));
    }
    Ok(crate::storage::jcode_dir()?.join("sessions"))
}

fn expand_home_path(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/")
        && let Some(home) = dirs::home_dir()
    {
        return home.join(stripped);
    }
    if path == "~"
        && let Some(home) = dirs::home_dir()
    {
        return home;
    }
    PathBuf::from(path)
}

fn is_syncable_session_stem(stem: &str) -> bool {
    (stem.starts_with("session_") || stem.starts_with("imported_")) && !stem.ends_with(".journal")
}

fn sha256_file(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    let mut file = std::fs::File::open(path)
        .map_err(|err| anyhow::anyhow!("failed to open {}: {err}", path.display()))?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)
        .map_err(|err| anyhow::anyhow!("failed to hash {}: {err}", path.display()))?;
    Ok(hex::encode(hasher.finalize()))
}

fn collect_sync_candidates(dir: &Path) -> Result<Vec<SyncCandidate>> {
    let mut candidates = Vec::new();
    if !dir.exists() {
        anyhow::bail!("sessions directory not found: {}", dir.display());
    }
    for entry in std::fs::read_dir(dir)
        .map_err(|err| anyhow::anyhow!("failed to read {}: {err}", dir.display()))?
        .flatten()
    {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        if !is_syncable_session_stem(stem) {
            continue;
        }
        let metadata = match entry.metadata() {
            Ok(metadata) if metadata.is_file() => metadata,
            _ => continue,
        };
        let modified_unix = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|dur| dur.as_secs() as i64);
        candidates.push(SyncCandidate {
            session_id: stem.to_string(),
            path,
            size: metadata.len(),
            modified_unix,
        });
    }
    Ok(candidates)
}

fn run_jade_upload(
    helper: &Path,
    helper_env: &[(&'static str, String)],
    file: &Path,
    user_id: &str,
    profile: Option<&str>,
    region: Option<&str>,
    raw: bool,
) -> Result<()> {
    let mut args = vec!["upload".to_string()];
    append_common_jade_args(
        &mut args,
        user_id.to_string(),
        profile.map(ToOwned::to_owned),
        region.map(ToOwned::to_owned),
    );
    if raw {
        args.push("--raw".to_string());
    }
    args.push(file.display().to_string());

    let output = ProcessCommand::new(helper)
        .args(&args)
        .envs(helper_env.iter().cloned())
        .stdin(Stdio::null())
        .output()
        .map_err(|err| anyhow::anyhow!("failed to run {}: {err}", helper.display()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = stderr.trim();
        let detail = if detail.is_empty() {
            format!("exited with status {}", output.status)
        } else {
            detail.lines().last().unwrap_or(detail).to_string()
        };
        anyhow::bail!(detail);
    }
    Ok(())
}

fn run_cloud_sessions_sync(request: CloudSessionsSyncRequest) -> Result<()> {
    let config = load_cloud_sessions_config()?.unwrap_or_default();
    let helper_override = request.helper.clone().or_else(|| config.helper.clone());
    let user_id = config_or_default_user_id(request.user_id.clone(), &config);
    let sessions_dir = resolve_sync_sessions_dir(request.sessions_dir.as_deref())?;
    let mut state = load_cloud_sessions_sync_state()?;

    // Self-throttle so the command is safe to call from cron/systemd timers without
    // re-uploading or even rescanning more often than requested.
    if !request.force
        && !request.dry_run
        && let Some(min_interval) = request.min_interval_mins
        && min_interval > 0
        && let Some(last) = state
            .last_sync_at
            .as_deref()
            .and_then(|ts| chrono::DateTime::parse_from_rfc3339(ts).ok())
    {
        let elapsed_mins = (chrono::Utc::now() - last.with_timezone(&chrono::Utc)).num_minutes();
        if elapsed_mins < min_interval as i64 {
            let report = CloudSessionsSyncReport {
                sessions_dir: sessions_dir.display().to_string(),
                dry_run: request.dry_run,
                throttled: true,
                scanned: 0,
                uploaded: 0,
                skipped_unchanged: 0,
                failed: 0,
                reached_max: false,
                entries: Vec::new(),
            };
            if request.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!(
                    "Jade cloud sessions sync skipped: last sync {elapsed_mins}m ago (< --min-interval-mins {min_interval})"
                );
            }
            return Ok(());
        }
    }

    let helper = resolve_jade_sessions_helper(helper_override.as_deref())?;
    let helper_env = cloud_sessions_helper_env(&config);
    let mut candidates = collect_sync_candidates(&sessions_dir)?;

    if !request.all {
        let since_days = request.since_days.unwrap_or(7);
        let cutoff = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|dur| dur.as_secs() as i64)
            .unwrap_or(0)
            - (since_days as i64) * 86_400;
        candidates.retain(|candidate| candidate.modified_unix.map(|m| m >= cutoff).unwrap_or(true));
    }

    // Newest first so --max keeps the most recent sessions.
    candidates.sort_by(|a, b| b.modified_unix.cmp(&a.modified_unix));

    let mut report = CloudSessionsSyncReport {
        sessions_dir: sessions_dir.display().to_string(),
        dry_run: request.dry_run,
        throttled: false,
        scanned: 0,
        uploaded: 0,
        skipped_unchanged: 0,
        failed: 0,
        reached_max: false,
        entries: Vec::new(),
    };
    let mut state_dirty = false;

    for candidate in candidates {
        if report.uploaded + report.failed >= request.max {
            report.reached_max = true;
            break;
        }
        report.scanned += 1;
        let sha = match sha256_file(&candidate.path) {
            Ok(sha) => sha,
            Err(err) => {
                report.failed += 1;
                report.entries.push(CloudSessionsSyncEntry {
                    session_id: candidate.session_id,
                    path: candidate.path.display().to_string(),
                    status: "failed",
                    error: Some(err.to_string()),
                });
                continue;
            }
        };

        if !request.force
            && let Some(record) = state.sessions.get(&candidate.session_id)
            && record.sha256 == sha
        {
            report.skipped_unchanged += 1;
            continue;
        }

        if request.dry_run {
            report.uploaded += 1;
            report.entries.push(CloudSessionsSyncEntry {
                session_id: candidate.session_id,
                path: candidate.path.display().to_string(),
                status: "would-upload",
                error: None,
            });
            continue;
        }

        match run_jade_upload(
            &helper,
            &helper_env,
            &candidate.path,
            &user_id,
            request.profile.as_deref(),
            request.region.as_deref(),
            request.raw,
        ) {
            Ok(()) => {
                report.uploaded += 1;
                state.sessions.insert(
                    candidate.session_id.clone(),
                    CloudSessionsSyncRecord {
                        sha256: sha,
                        size: candidate.size,
                        modified_unix: candidate.modified_unix,
                        uploaded_at: chrono::Utc::now().to_rfc3339(),
                    },
                );
                state_dirty = true;
                report.entries.push(CloudSessionsSyncEntry {
                    session_id: candidate.session_id,
                    path: candidate.path.display().to_string(),
                    status: "uploaded",
                    error: None,
                });
            }
            Err(err) => {
                report.failed += 1;
                report.entries.push(CloudSessionsSyncEntry {
                    session_id: candidate.session_id,
                    path: candidate.path.display().to_string(),
                    status: "failed",
                    error: Some(err.to_string()),
                });
            }
        }
    }

    // Record completion time for non-dry runs (even if nothing changed) so
    // --min-interval-mins throttling works for schedulers, and persist any
    // newly uploaded session records.
    if !request.dry_run {
        state.last_sync_at = Some(chrono::Utc::now().to_rfc3339());
        save_cloud_sessions_sync_state(&state)?;
    } else if state_dirty {
        save_cloud_sessions_sync_state(&state)?;
    }

    if request.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        let verb = if request.dry_run {
            "Would upload"
        } else {
            "Uploaded"
        };
        println!("Jade cloud sessions sync ({})", report.sessions_dir);
        println!(
            "scanned: {}  {}: {}  unchanged: {}  failed: {}",
            report.scanned, verb, report.uploaded, report.skipped_unchanged, report.failed
        );
        if report.reached_max {
            println!("note: reached --max {}; rerun to continue", request.max);
        }
        for entry in &report.entries {
            match entry.error.as_deref() {
                Some(error) => println!("  [{}] {} ({})", entry.status, entry.session_id, error),
                None => println!("  [{}] {}", entry.status, entry.session_id),
            }
        }
    }

    if report.failed > 0 {
        anyhow::bail!("{} session(s) failed to upload", report.failed);
    }
    Ok(())
}

struct CloudSessionsDashboardRequest {
    limit: usize,
    output: Option<String>,
    open: bool,
    with_view: bool,
    user_id: String,
    profile: Option<String>,
    region: Option<String>,
    helper: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct CloudSessionListItem {
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    short_name: Option<String>,
    #[serde(default)]
    message_count: Option<serde_json::Value>,
    #[serde(default)]
    uploaded_at: Option<String>,
}

fn fetch_cloud_session_list_json(
    helper: &Path,
    helper_env: &[(&'static str, String)],
    user_id: &str,
    profile: Option<&str>,
    region: Option<&str>,
    limit: usize,
) -> Result<Vec<CloudSessionListItem>> {
    let mut args = vec!["list".to_string()];
    append_common_jade_args(
        &mut args,
        user_id.to_string(),
        profile.map(ToOwned::to_owned),
        region.map(ToOwned::to_owned),
    );
    args.extend(["--limit".to_string(), limit.to_string()]);
    args.push("--json".to_string());

    let output = ProcessCommand::new(helper)
        .args(&args)
        .envs(helper_env.iter().cloned())
        .stdin(Stdio::null())
        .output()
        .map_err(|err| anyhow::anyhow!("failed to run {}: {err}", helper.display()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = stderr.trim();
        anyhow::bail!(
            "{} list failed: {}",
            helper.display(),
            if detail.is_empty() {
                format!("exited with status {}", output.status)
            } else {
                detail.to_string()
            }
        );
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_cloud_session_list_json(stdout.trim())
}

/// Parse the helper's `list --json` output.
///
/// The Jade helper prints a top-level JSON array, but we also accept an object
/// wrapper keyed by `items` or `sessions` so the dashboard keeps working if the
/// helper's output shape changes.
fn parse_cloud_session_list_json(raw: &str) -> Result<Vec<CloudSessionListItem>> {
    let value: serde_json::Value = serde_json::from_str(raw)
        .map_err(|err| anyhow::anyhow!("failed to parse Jade list JSON: {err}"))?;
    let array = match value {
        serde_json::Value::Array(items) => items,
        serde_json::Value::Object(mut map) => map
            .remove("items")
            .or_else(|| map.remove("sessions"))
            .and_then(|value| match value {
                serde_json::Value::Array(items) => Some(items),
                _ => None,
            })
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "failed to parse Jade list JSON: expected an array or an object with an `items`/`sessions` array"
                )
            })?,
        other => anyhow::bail!(
            "failed to parse Jade list JSON: expected an array, found {}",
            json_value_kind(&other)
        ),
    };
    array
        .into_iter()
        .map(|item| {
            serde_json::from_value(item)
                .map_err(|err| anyhow::anyhow!("failed to parse Jade list item: {err}"))
        })
        .collect()
}

fn json_value_kind(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "a boolean",
        serde_json::Value::Number(_) => "a number",
        serde_json::Value::String(_) => "a string",
        serde_json::Value::Array(_) => "an array",
        serde_json::Value::Object(_) => "an object",
    }
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn message_count_label(value: Option<&serde_json::Value>) -> String {
    match value {
        Some(serde_json::Value::Number(num)) => num.to_string(),
        Some(serde_json::Value::String(text)) => text.clone(),
        _ => "-".to_string(),
    }
}

fn render_cloud_sessions_dashboard_html(
    user_id: &str,
    items: &[CloudSessionListItem],
    view_links: &std::collections::BTreeMap<String, String>,
) -> String {
    let generated = chrono::Utc::now().to_rfc3339();
    let mut rows = String::new();
    for item in items {
        let session_id = item.session_id.as_deref().unwrap_or("(unknown)");
        let title = item
            .title
            .as_deref()
            .filter(|value| !value.is_empty())
            .or(item.short_name.as_deref())
            .unwrap_or("(untitled)");
        let uploaded = item.uploaded_at.as_deref().unwrap_or("-");
        // When a local per-session viewer was generated, link the session id to it.
        let id_cell = match item.session_id.as_deref().and_then(|id| view_links.get(id)) {
            Some(link) => format!(
                "<a href='{}'>{}</a>",
                html_escape(link),
                html_escape(session_id)
            ),
            None => html_escape(session_id),
        };
        rows.push_str(&format!(
            "<tr><td class='id'>{}</td><td>{}</td><td class='num'>{}</td><td class='ts'>{}</td></tr>\n",
            id_cell,
            html_escape(title),
            html_escape(&message_count_label(item.message_count.as_ref())),
            html_escape(uploaded),
        ));
    }
    if rows.is_empty() {
        rows.push_str("<tr><td colspan='4' class='empty'>No uploaded sessions found.</td></tr>\n");
    }
    format!(
        "<!doctype html><meta charset='utf-8'>\n\
<title>Jade Cloud Sessions Dashboard</title>\n\
<style>body{{font-family:system-ui,sans-serif;max-width:1100px;margin:2rem auto;padding:0 1rem;color:#1b1b1f}}\
h1{{margin-bottom:0.2rem}}.meta{{color:#666;margin-bottom:1.5rem}}\
table{{border-collapse:collapse;width:100%}}th,td{{text-align:left;padding:0.5rem 0.6rem;border-bottom:1px solid #e3e3e8}}\
th{{background:#f6f8fa;position:sticky;top:0}}td.id{{font-family:ui-monospace,monospace;font-size:0.85rem}}\
td.id a{{color:#0a58ca;text-decoration:none}}td.id a:hover{{text-decoration:underline}}\
td.num{{text-align:right}}td.ts{{white-space:nowrap;color:#555}}td.empty{{text-align:center;color:#888;padding:2rem}}\
tr:hover td{{background:#fafbff}}</style>\n\
<h1>Jade Cloud Sessions</h1>\n\
<div class='meta'>user: {user} &middot; {count} session(s) &middot; generated {generated}</div>\n\
<table><thead><tr><th>Session ID</th><th>Title</th><th>Messages</th><th>Uploaded</th></tr></thead>\n\
<tbody>\n{rows}</tbody></table>\n",
        user = html_escape(user_id),
        count = items.len(),
        generated = html_escape(&generated),
        rows = rows,
    )
}

fn run_cloud_sessions_dashboard(request: CloudSessionsDashboardRequest) -> Result<()> {
    let config = load_cloud_sessions_config()?.unwrap_or_default();
    let helper_override = request.helper.clone().or_else(|| config.helper.clone());
    let helper = resolve_jade_sessions_helper(helper_override.as_deref())?;
    let helper_env = cloud_sessions_helper_env(&config);
    let user_id = config_or_default_user_id(request.user_id.clone(), &config);

    let items = fetch_cloud_session_list_json(
        &helper,
        &helper_env,
        &user_id,
        request.profile.as_deref(),
        request.region.as_deref(),
        request.limit,
    )?;

    let output_path = match request
        .output
        .as_deref()
        .map(str::trim)
        .filter(|p| !p.is_empty())
    {
        Some(path) => expand_home_path(path),
        None => std::env::temp_dir().join(format!(
            "jade-cloud-dashboard-{}.html",
            chrono::Utc::now().format("%Y%m%d-%H%M%S")
        )),
    };
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Optionally download each session and render a local per-session viewer,
    // then link the dashboard rows to those files (relative to the dashboard).
    let mut view_links: std::collections::BTreeMap<String, String> =
        std::collections::BTreeMap::new();
    if request.with_view {
        let views_dir = dashboard_views_dir(&output_path);
        std::fs::create_dir_all(&views_dir)?;
        let total = items.len();
        let mut generated = 0usize;
        for (idx, item) in items.iter().enumerate() {
            let Some(session_id) = item.session_id.as_deref().filter(|id| !id.is_empty()) else {
                continue;
            };
            let view_file = views_dir.join(format!("{}.html", sanitize_filename(session_id)));
            eprintln!("[{}/{}] downloading {}", idx + 1, total, session_id);
            match generate_cloud_session_view_html(
                &helper,
                &helper_env,
                &user_id,
                request.profile.as_deref(),
                request.region.as_deref(),
                session_id,
                &view_file,
            ) {
                Ok(()) => {
                    if let Some(rel) = relative_link(&output_path, &view_file) {
                        view_links.insert(session_id.to_string(), rel);
                        generated += 1;
                    }
                }
                Err(err) => {
                    eprintln!("  warning: could not render viewer for {session_id}: {err}");
                }
            }
        }
        eprintln!(
            "Generated {generated}/{total} per-session viewer(s) in {}",
            views_dir.display()
        );
    }

    let html = render_cloud_sessions_dashboard_html(&user_id, &items, &view_links);
    std::fs::write(&output_path, html.as_bytes())
        .map_err(|err| anyhow::anyhow!("failed to write {}: {err}", output_path.display()))?;

    println!(
        "Wrote Jade cloud sessions dashboard ({} session(s)) to {}",
        items.len(),
        output_path.display()
    );
    if request.open {
        let _ = open::that(&output_path);
    }
    Ok(())
}

/// Directory that holds per-session viewer HTML files for a dashboard.
fn dashboard_views_dir(dashboard_path: &Path) -> PathBuf {
    let stem = dashboard_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "dashboard".to_string());
    let parent = dashboard_path.parent().unwrap_or_else(|| Path::new("."));
    parent.join(format!("{stem}-views"))
}

/// Make a filesystem-safe filename component from a session id.
fn sanitize_filename(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Build a link from the dashboard file to a viewer file, preferring a relative
/// path when both share a parent directory so the dashboard is portable.
fn relative_link(dashboard_path: &Path, view_file: &Path) -> Option<String> {
    let base = dashboard_path.parent()?;
    let rel = view_file.strip_prefix(base).ok()?;
    Some(rel.to_string_lossy().replace('\\', "/"))
}

/// Invoke the helper's `view --format html --output <file>` for one session.
fn generate_cloud_session_view_html(
    helper: &Path,
    helper_env: &[(&'static str, String)],
    user_id: &str,
    profile: Option<&str>,
    region: Option<&str>,
    session_id: &str,
    output_file: &Path,
) -> Result<()> {
    let mut args = vec!["view".to_string()];
    append_common_jade_args(
        &mut args,
        user_id.to_string(),
        profile.map(ToOwned::to_owned),
        region.map(ToOwned::to_owned),
    );
    args.extend(["--format".to_string(), "html".to_string()]);
    args.extend([
        "--output".to_string(),
        output_file.to_string_lossy().to_string(),
    ]);
    args.push(session_id.to_string());

    let output = ProcessCommand::new(helper)
        .args(&args)
        .envs(helper_env.iter().cloned())
        .stdin(Stdio::null())
        .output()
        .map_err(|err| anyhow::anyhow!("failed to run {}: {err}", helper.display()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = stderr.trim();
        anyhow::bail!(
            "{}",
            if detail.is_empty() {
                format!("view exited with status {}", output.status)
            } else {
                detail.to_string()
            }
        );
    }
    Ok(())
}

fn configured_label(value: Option<&str>) -> &str {
    value
        .filter(|value| !value.is_empty())
        .unwrap_or("not configured")
}

fn config_or_default_user_id(user_id: String, config: &CloudSessionsConfig) -> String {
    if user_id == "dev" {
        config.user_id.clone().unwrap_or(user_id)
    } else {
        user_id
    }
}

fn cloud_sessions_helper_env(config: &CloudSessionsConfig) -> Vec<(&'static str, String)> {
    let mut env = Vec::new();
    if let Some(api_base) = non_empty(config.api_base.clone()) {
        env.push(("JADE_API_BASE", api_base));
    }
    if let Some(api_token) = non_empty(config.api_token.clone()) {
        env.push(("JADE_API_TOKEN", api_token));
    }
    if let Some(api_token_id) = non_empty(config.api_token_id.clone()) {
        env.push(("JADE_API_TOKEN_ID", api_token_id));
    }
    env
}

fn cloud_sessions_helper_override(action: &CloudSessionsSubcommand) -> Option<String> {
    match action {
        CloudSessionsSubcommand::Configure { .. }
        | CloudSessionsSubcommand::Status { .. }
        | CloudSessionsSubcommand::Sync { .. } => None,
        CloudSessionsSubcommand::Upload { helper, .. }
        | CloudSessionsSubcommand::UploadLatest { helper, .. }
        | CloudSessionsSubcommand::List { helper, .. }
        | CloudSessionsSubcommand::Verify { helper, .. }
        | CloudSessionsSubcommand::Dashboard { helper, .. }
        | CloudSessionsSubcommand::View { helper, .. } => helper.clone(),
    }
}

fn append_common_jade_args(
    args: &mut Vec<String>,
    user_id: String,
    profile: Option<String>,
    region: Option<String>,
) {
    args.extend(["--user-id".to_string(), user_id]);
    if let Some(profile) = profile {
        args.extend(["--profile".to_string(), profile]);
    }
    if let Some(region) = region {
        args.extend(["--region".to_string(), region]);
    }
}

#[cfg(test)]
fn build_jade_sessions_args(action: CloudSessionsSubcommand) -> Vec<String> {
    build_jade_sessions_args_with_config(action, &CloudSessionsConfig::default())
}

fn build_jade_sessions_args_with_config(
    action: CloudSessionsSubcommand,
    config: &CloudSessionsConfig,
) -> Vec<String> {
    match action {
        CloudSessionsSubcommand::Configure { .. }
        | CloudSessionsSubcommand::Status { .. }
        | CloudSessionsSubcommand::Sync { .. }
        | CloudSessionsSubcommand::Dashboard { .. } => {
            unreachable!(
                "configure/status/sync/dashboard do not invoke the Jade helper via this builder"
            )
        }
        CloudSessionsSubcommand::Upload {
            session_file,
            raw,
            user_id,
            profile,
            region,
            ..
        } => {
            let mut args = vec!["upload".to_string()];
            append_common_jade_args(
                &mut args,
                config_or_default_user_id(user_id, config),
                profile,
                region,
            );
            if raw {
                args.push("--raw".to_string());
            }
            args.push(session_file);
            args
        }
        CloudSessionsSubcommand::UploadLatest {
            sessions_dir,
            raw,
            user_id,
            profile,
            region,
            ..
        } => {
            let mut args = vec!["upload-latest".to_string()];
            append_common_jade_args(
                &mut args,
                config_or_default_user_id(user_id, config),
                profile,
                region,
            );
            args.extend(["--sessions-dir".to_string(), sessions_dir]);
            if raw {
                args.push("--raw".to_string());
            }
            args
        }
        CloudSessionsSubcommand::List {
            limit,
            json,
            user_id,
            profile,
            region,
            ..
        } => {
            let mut args = vec!["list".to_string()];
            append_common_jade_args(
                &mut args,
                config_or_default_user_id(user_id, config),
                profile,
                region,
            );
            args.extend(["--limit".to_string(), limit.to_string()]);
            if json {
                args.push("--json".to_string());
            }
            args
        }
        CloudSessionsSubcommand::Verify {
            session_id,
            user_id,
            profile,
            region,
            ..
        } => {
            let mut args = vec!["verify".to_string()];
            append_common_jade_args(
                &mut args,
                config_or_default_user_id(user_id, config),
                profile,
                region,
            );
            args.push(session_id);
            args
        }
        CloudSessionsSubcommand::View {
            session_id,
            format,
            output,
            open,
            user_id,
            profile,
            region,
            ..
        } => {
            let mut args = vec!["view".to_string()];
            append_common_jade_args(
                &mut args,
                config_or_default_user_id(user_id, config),
                profile,
                region,
            );
            args.extend(["--format".to_string(), format]);
            if let Some(output) = output {
                args.extend(["--output".to_string(), output]);
            }
            if open {
                args.push("--open".to_string());
            }
            args.push(session_id);
            args
        }
    }
}

fn resolve_jade_sessions_helper(override_path: Option<&str>) -> Result<PathBuf> {
    if let Some(path) = override_path.map(str::trim).filter(|path| !path.is_empty()) {
        return Ok(PathBuf::from(path));
    }

    if let Some(path) = std::env::var_os("JCODE_JADE_SESSIONS_HELPER")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
    {
        return Ok(path);
    }

    let mut candidates = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("../jade/scripts/jade_sessions.py"));
        candidates.push(cwd.join("jade/scripts/jade_sessions.py"));
    }
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join("jade/scripts/jade_sessions.py"));
    }

    for candidate in candidates {
        if is_executable_file(&candidate) {
            return Ok(candidate);
        }
    }

    anyhow::bail!(
        "Could not find Jade session helper. Set --helper PATH or JCODE_JADE_SESSIONS_HELPER. Expected a private helper like ~/jade/scripts/jade_sessions.py"
    );
}

#[cfg(unix)]
fn is_executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.is_file()
        && path
            .metadata()
            .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable_file(path: &Path) -> bool {
    path.is_file()
}

pub async fn run_ambient_command(cmd: AmbientSubcommand) -> Result<()> {
    if let AmbientSubcommand::RunVisible = cmd {
        return run_ambient_visible().await;
    }

    let debug_cmd = match cmd {
        AmbientSubcommand::Status => "ambient:status",
        AmbientSubcommand::Log => "ambient:log",
        AmbientSubcommand::Trigger => "ambient:trigger",
        AmbientSubcommand::Stop => "ambient:stop",
        AmbientSubcommand::RunVisible => unreachable!(),
    };

    super::debug::run_debug_command(debug_cmd, "", None, None, false).await
}

pub async fn run_transcript_command(
    text: Option<String>,
    mode: crate::protocol::TranscriptMode,
    session: Option<String>,
) -> Result<()> {
    let text = if let Some(text) = text {
        text
    } else {
        let mut stdin = String::new();
        std::io::stdin().read_to_string(&mut stdin)?;
        let trimmed = stdin.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            anyhow::bail!("Provide transcript text as an argument or pipe it via stdin")
        }
        trimmed.to_string()
    };

    let mut client = crate::server::Client::connect_debug().await?;
    let request_id = client.send_transcript(&text, mode, session).await?;

    loop {
        match client.read_event().await? {
            crate::protocol::ServerEvent::Ack { id } if id == request_id => {}
            crate::protocol::ServerEvent::Done { id } if id == request_id => return Ok(()),
            crate::protocol::ServerEvent::Error { id, message, .. } if id == request_id => {
                anyhow::bail!(message)
            }
            _ => {}
        }
    }
}

pub async fn run_dictate_command(type_output: bool) -> Result<()> {
    let run = crate::dictation::run_configured().await?;

    if type_output {
        crate::dictation::type_text(&run.text)
    } else {
        run_transcript_command(Some(run.text), run.mode, None).await
    }
}

#[derive(Serialize)]
struct SessionRenameOutput {
    session_id: String,
    display_name: String,
    title: Option<String>,
    cleared: bool,
}

pub fn run_session_rename_command(
    session_ref: &str,
    name: Option<&str>,
    clear: bool,
    json: bool,
) -> Result<()> {
    let resolved_id = session::find_session_by_name_or_id(session_ref)?;
    let mut session = session::Session::load(&resolved_id)?;

    if clear {
        session.rename_title(None);
    } else {
        let Some(name) = name.map(str::trim).filter(|name| !name.is_empty()) else {
            anyhow::bail!("Provide a session name or use --clear");
        };
        session.rename_title(Some(name.to_string()));
    }

    session.save()?;
    crate::tui::session_picker::invalidate_session_list_cache();

    let output = SessionRenameOutput {
        session_id: session.id.clone(),
        display_name: session.display_name().to_string(),
        title: session.display_title().map(ToOwned::to_owned),
        cleared: clear,
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else if clear {
        println!(
            "Cleared custom name for session {} ({}).",
            output.display_name, output.session_id
        );
    } else if let Some(title) = output.title.as_deref() {
        println!(
            "Renamed session {} ({}) to \"{}\".",
            output.display_name, output.session_id, title
        );
    }

    Ok(())
}

async fn run_ambient_visible() -> Result<()> {
    use crate::ambient::VisibleCycleContext;

    let context = VisibleCycleContext::load().map_err(|e| {
        anyhow::anyhow!(
            "Failed to load visible cycle context: {}\nIs the ambient runner running?",
            e
        )
    })?;

    let (provider, registry) = super::provider_init::init_provider_and_registry(
        &super::provider_init::ProviderChoice::Auto,
        None,
    )
    .await?;

    registry.register_ambient_tools().await;

    let safety = std::sync::Arc::new(crate::safety::SafetySystem::new());
    crate::tool::ambient::init_safety_system(safety);

    let (terminal, tui_runtime) = init_tui_runtime()?;

    let mut app = tui::App::new(provider, registry);
    app.set_ambient_mode(context.system_prompt, context.initial_message);

    let _ = crossterm::execute!(
        std::io::stdout(),
        crossterm::terminal::SetTitle("🤖 jcode ambient cycle")
    );

    let result = app.run(terminal).await;

    tui_runtime.finish(true);

    if let Some(cycle_result) = crate::tool::ambient::take_cycle_result() {
        let result_path = VisibleCycleContext::result_path()?;
        crate::storage::write_json(&result_path, &cycle_result)?;
        eprintln!("Ambient cycle result saved.");
    }

    result?;
    Ok(())
}

pub enum MemorySubcommand {
    List {
        scope: String,
        tag: Option<String>,
    },
    Search {
        query: String,
        semantic: bool,
    },
    Export {
        output: String,
        scope: String,
    },
    Import {
        input: String,
        scope: String,
        overwrite: bool,
    },
    Stats,
    ClearTest,
}

pub fn run_memory_command(cmd: MemorySubcommand) -> Result<()> {
    use memory::{MemoryEntry, MemoryManager};

    let manager = MemoryManager::new();

    match cmd {
        MemorySubcommand::List { scope, tag } => {
            let mut all_memories: Vec<MemoryEntry> = Vec::new();

            if (scope == "all" || scope == "project")
                && let Ok(graph) = manager.load_project_graph()
            {
                all_memories.extend(graph.all_memories().cloned());
            }
            if (scope == "all" || scope == "global")
                && let Ok(graph) = manager.load_global_graph()
            {
                all_memories.extend(graph.all_memories().cloned());
            }

            if let Some(tag_filter) = tag {
                all_memories.retain(|m| m.tags.contains(&tag_filter));
            }

            all_memories.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

            if all_memories.is_empty() {
                println!("No memories found.");
            } else {
                println!("Found {} memories:\n", all_memories.len());
                for entry in &all_memories {
                    let tags_str = if entry.tags.is_empty() {
                        String::new()
                    } else {
                        format!(" [{}]", entry.tags.join(", "))
                    };
                    let conf = entry.effective_confidence();
                    println!(
                        "- [{}] {}{}\n  id: {} (conf: {:.0}%, accessed: {}x)",
                        entry.category,
                        entry.content,
                        tags_str,
                        entry.id,
                        conf * 100.0,
                        entry.access_count
                    );
                    println!();
                }
            }
        }

        MemorySubcommand::Search { query, semantic } => {
            if semantic {
                match manager.find_similar(&query, 0.3, 20) {
                    Ok(results) => {
                        if results.is_empty() {
                            println!("No memories found matching '{}'", query);
                        } else {
                            println!(
                                "Found {} memories matching '{}' (semantic):\n",
                                results.len(),
                                query
                            );
                            for (entry, score) in results {
                                let tags_str = if entry.tags.is_empty() {
                                    String::new()
                                } else {
                                    format!(" [{}]", entry.tags.join(", "))
                                };
                                println!(
                                    "- [{}] {}{}\n  id: {} (score: {:.0}%)",
                                    entry.category,
                                    entry.content,
                                    tags_str,
                                    entry.id,
                                    score * 100.0
                                );
                                println!();
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Search failed: {}", e);
                    }
                }
            } else {
                match manager.search(&query) {
                    Ok(results) => {
                        if results.is_empty() {
                            println!("No memories found matching '{}'", query);
                        } else {
                            println!(
                                "Found {} memories matching '{}' (keyword):\n",
                                results.len(),
                                query
                            );
                            for entry in results {
                                let tags_str = if entry.tags.is_empty() {
                                    String::new()
                                } else {
                                    format!(" [{}]", entry.tags.join(", "))
                                };
                                println!(
                                    "- [{}] {}{}\n  id: {}",
                                    entry.category, entry.content, tags_str, entry.id
                                );
                                println!();
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Search failed: {}", e);
                    }
                }
            }
        }

        MemorySubcommand::Export { output, scope } => {
            let mut all_memories: Vec<memory::MemoryEntry> = Vec::new();

            if (scope == "all" || scope == "project")
                && let Ok(graph) = manager.load_project_graph()
            {
                all_memories.extend(graph.all_memories().cloned());
            }
            if (scope == "all" || scope == "global")
                && let Ok(graph) = manager.load_global_graph()
            {
                all_memories.extend(graph.all_memories().cloned());
            }

            let json = serde_json::to_string_pretty(&all_memories)?;
            std::fs::write(&output, json)?;
            println!("Exported {} memories to {}", all_memories.len(), output);
        }

        MemorySubcommand::Import {
            input,
            scope,
            overwrite,
        } => {
            let content = std::fs::read_to_string(&input)?;
            let memories: Vec<memory::MemoryEntry> = serde_json::from_str(&content)?;

            let mut imported = 0;
            let mut skipped = 0;

            for entry in memories {
                let result = if scope == "global" {
                    if !overwrite
                        && let Ok(graph) = manager.load_global_graph()
                        && graph.get_memory(&entry.id).is_some()
                    {
                        skipped += 1;
                        continue;
                    }
                    manager.remember_global(entry)
                } else {
                    if !overwrite
                        && let Ok(graph) = manager.load_project_graph()
                        && graph.get_memory(&entry.id).is_some()
                    {
                        skipped += 1;
                        continue;
                    }
                    manager.remember_project(entry)
                };

                if result.is_ok() {
                    imported += 1;
                }
            }

            println!("Imported {} memories ({} skipped)", imported, skipped);
        }

        MemorySubcommand::Stats => {
            let mut project_count = 0;
            let mut global_count = 0;
            let mut total_tags = std::collections::HashSet::new();
            let mut categories: std::collections::HashMap<String, usize> =
                std::collections::HashMap::new();

            if let Ok(graph) = manager.load_project_graph() {
                project_count = graph.memory_count();
                for entry in graph.all_memories() {
                    for tag in &entry.tags {
                        total_tags.insert(tag.clone());
                    }
                    *categories.entry(entry.category.to_string()).or_default() += 1;
                }
            }

            if let Ok(graph) = manager.load_global_graph() {
                global_count = graph.memory_count();
                for entry in graph.all_memories() {
                    for tag in &entry.tags {
                        total_tags.insert(tag.clone());
                    }
                    *categories.entry(entry.category.to_string()).or_default() += 1;
                }
            }

            println!("Memory Statistics:");
            println!("  Project memories: {}", project_count);
            println!("  Global memories:  {}", global_count);
            println!("  Total:            {}", project_count + global_count);
            println!("  Unique tags:      {}", total_tags.len());
            println!("\nBy category:");
            for (cat, count) in &categories {
                println!("  {}: {}", cat, count);
            }
        }

        MemorySubcommand::ClearTest => {
            let test_dir = storage::jcode_dir()?.join("memory").join("test");
            if test_dir.exists() {
                let count = std::fs::read_dir(&test_dir)?.count();
                std::fs::remove_dir_all(&test_dir)?;
                println!("Cleared test memory storage ({} files)", count);
            } else {
                println!("Test memory storage is already empty");
            }
        }
    }

    Ok(())
}

pub fn run_pair_command(list: bool, revoke: Option<String>) -> Result<()> {
    let mut registry = gateway::DeviceRegistry::load();

    if list {
        if registry.devices.is_empty() {
            eprintln!("No paired devices.");
        } else {
            eprintln!("\x1b[1mPaired devices:\x1b[0m\n");
            for device in &registry.devices {
                let last_seen = &device.last_seen;
                eprintln!("  \x1b[36m{}\x1b[0m  ({})", device.name, device.id);
                eprintln!("    Paired: {}  Last seen: {}", device.paired_at, last_seen);
                if let Some(ref apns) = device.apns_token {
                    eprintln!("    APNs: {}...", &apns[..apns.len().min(16)]);
                }
                eprintln!();
            }
        }
        return Ok(());
    }

    if let Some(ref target) = revoke {
        let before = registry.devices.len();
        registry
            .devices
            .retain(|d| d.id != *target && d.name != *target);
        if registry.devices.len() < before {
            registry.save()?;
            eprintln!("\x1b[32m✓\x1b[0m Revoked device: {}", target);
        } else {
            eprintln!("\x1b[31m✗\x1b[0m No device found matching: {}", target);
        }
        return Ok(());
    }

    let gw_config = &crate::config::config().gateway;

    if !gw_config.enabled {
        eprintln!("\x1b[33m⚠\x1b[0m  Gateway is disabled. Enable it in ~/.jcode/config.toml:\n");
        eprintln!("    \x1b[2m[gateway]\x1b[0m");
        eprintln!("    \x1b[2menabled = true\x1b[0m");
        eprintln!("    \x1b[2mport = {}\x1b[0m\n", gw_config.port);
        eprintln!("  Then restart the jcode server.\n");
    }

    let code = registry.generate_pairing_code();
    let connect_host = resolve_connect_host(&gw_config.bind_addr);
    let pair_uri = format!(
        "jcode://pair?host={}&port={}&code={}",
        connect_host, gw_config.port, code
    );

    eprintln!();
    eprintln!("  \x1b[1mScan with the jcode iOS app:\x1b[0m\n");
    match crate::login_qr::render_unicode_qr(&pair_uri) {
        Ok(qr) => {
            for line in qr.lines() {
                eprintln!("  {line}");
            }
        }
        Err(_) => eprintln!("  \x1b[33m(QR code generation failed)\x1b[0m"),
    }
    eprintln!();
    eprintln!(
        "  Pairing code:  \x1b[1;37m{} {}\x1b[0m   \x1b[2m(expires in 5 minutes)\x1b[0m",
        &code[..3],
        &code[3..]
    );
    let resolved_hint = format!("{}:{}", connect_host, gw_config.port);
    let bind_hint = format!("{}:{}", gw_config.bind_addr, gw_config.port);
    eprintln!("  Connect host:  \x1b[36m{}\x1b[0m", resolved_hint);
    if connect_host != gw_config.bind_addr {
        eprintln!("  Bind address:  \x1b[2m{}\x1b[0m", bind_hint);
    }

    if connect_host == "<your-mac-hostname>" {
        eprintln!(
            "\n  \x1b[33mTip:\x1b[0m set JCODE_GATEWAY_HOST to your reachable Tailscale hostname."
        );
    }

    if (gw_config.bind_addr.as_str(), gw_config.port)
        .to_socket_addrs()
        .ok()
        .and_then(|mut it| it.next())
        .is_none()
    {
        eprintln!(
            "  \x1b[33mWarning:\x1b[0m gateway bind address appears invalid: {}",
            bind_hint
        );
    }
    eprintln!();

    Ok(())
}

pub fn resolve_connect_host(bind_addr: &str) -> String {
    if bind_addr == "0.0.0.0" || bind_addr == "::" {
        if let Some(host) = std::env::var("JCODE_GATEWAY_HOST")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
        {
            return host;
        }

        if let Some(host) = detect_tailscale_dns_name() {
            return host;
        }

        return std::env::var("HOSTNAME")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "<your-mac-hostname>".to_string());
    }
    bind_addr.to_string()
}

pub fn parse_tailscale_dns_name(status_json: &[u8]) -> Option<String> {
    let value: serde_json::Value = serde_json::from_slice(status_json).ok()?;
    let dns_name = value
        .get("Self")?
        .get("DNSName")?
        .as_str()?
        .trim()
        .trim_end_matches('.')
        .to_string();

    if dns_name.is_empty() {
        None
    } else {
        Some(dns_name)
    }
}

pub fn detect_tailscale_dns_name() -> Option<String> {
    let output = std::process::Command::new("tailscale")
        .args(["status", "--json"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    parse_tailscale_dns_name(&output.stdout)
}

pub async fn run_browser(action: &str) -> Result<()> {
    match action {
        "setup" => browser::run_setup_command().await?,
        "status" => {
            let status = browser::ensure_browser_ready_noninteractive().await?;
            println!("Browser automation");
            println!("  backend: {}", status.backend);
            println!("  browser: {}", status.browser);
            println!(
                "  binary: {}",
                if status.binary_installed {
                    "installed"
                } else {
                    "missing"
                }
            );
            println!(
                "  setup: {}",
                if status.setup_complete {
                    "complete"
                } else {
                    "not complete"
                }
            );
            println!(
                "  bridge: {}",
                if status.responding {
                    "responding"
                } else {
                    "not responding"
                }
            );
            println!(
                "  compatibility: {}",
                if status.compatible {
                    "ok"
                } else {
                    "extension/bridge mismatch"
                }
            );
            if !status.missing_actions.is_empty() {
                println!("  missing actions: {}", status.missing_actions.join(", "));
            }

            if status.ready {
                println!("\nBuilt-in browser tool is ready.");
            } else if status.responding && !status.compatible {
                println!(
                    "\nThe browser bridge is connected, but the installed Firefox extension is out of date for this jcode build. Run `jcode browser setup` to repair or update it."
                );
            } else {
                println!("\nRun `jcode browser setup` to install or repair it.");
            }
        }
        other => {
            eprintln!("Unknown browser action: {}", other);
            eprintln!("Available: setup, status");
            std::process::exit(1);
        }
    }
    Ok(())
}

#[derive(Debug, Serialize)]
struct ModelListReport {
    provider: String,
    selected_model: String,
    models: Vec<String>,
    routes: Vec<ModelListRouteReport>,
}

#[derive(Debug, Serialize)]
struct ModelListRouteReport {
    provider: String,
    model: String,
    method: String,
    available: bool,
}

#[derive(Debug, Serialize)]
struct RunCommandReport {
    session_id: String,
    provider: String,
    model: String,
    text: String,
    usage: crate::agent::TokenUsage,
}

#[derive(Debug, Default)]
struct NdjsonRunState {
    text: String,
    session_id: Option<String>,
    upstream_provider: Option<String>,
    connection_type: Option<String>,
    connection_phase: Option<String>,
    status_detail: Option<String>,
    usage: crate::agent::TokenUsage,
}

pub fn run_auth_status_command(emit_json: bool) -> Result<()> {
    report_info::run_auth_status_command(emit_json)
}

pub async fn run_auth_doctor_command(
    provider_arg: Option<&str>,
    validate: bool,
    emit_json: bool,
) -> Result<()> {
    report_info::run_auth_doctor_command(provider_arg, validate, emit_json).await
}

pub fn run_provider_list_command(emit_json: bool) -> Result<()> {
    report_info::run_provider_list_command(emit_json)
}

pub async fn run_provider_current_command(
    choice: &super::provider_init::ProviderChoice,
    model: Option<&str>,
    emit_json: bool,
) -> Result<()> {
    report_info::run_provider_current_command(choice, model, emit_json).await
}

pub fn run_version_command(emit_json: bool) -> Result<()> {
    report_info::run_version_command(emit_json)
}

pub async fn run_usage_command(emit_json: bool) -> Result<()> {
    report_info::run_usage_command(emit_json).await
}

/// Gracefully reload the running background server onto the newest binary.
///
/// This is the preferred upgrade path (issue #291): instead of killing the
/// daemon and dropping live headless/swarm sessions, we ask it to hand its
/// sessions off to a freshly exec'd server (the same path `/reload` uses).
///
/// Behavior:
/// - With `force == false` (the default), the server only reloads when it is
///   provably running older code than an available reload candidate. A server
///   already on the newest binary reports "already up to date" and does
///   nothing, which keeps an installer from downgrading a newer/dev daemon or
///   re-entering the reload-loop family (#277).
/// - With `force == true`, the server reloads unconditionally.
/// - If no server is running, this is a successful no-op so installers can call
///   it unconditionally.
pub async fn run_server_reload_command(force: bool, emit_json: bool) -> Result<()> {
    use crate::protocol::ServerEvent;
    use std::time::Duration;

    let socket = crate::server::socket_path();

    #[derive(Serialize)]
    struct ServerReloadReport {
        socket: String,
        had_listener: bool,
        forced: bool,
        reloaded: bool,
        already_current: bool,
        handoff_ready: bool,
        detail: String,
    }

    let emit = |report: ServerReloadReport| -> Result<()> {
        if emit_json {
            println!("{}", serde_json::to_string_pretty(&report)?);
        } else if !report.detail.is_empty() {
            println!("{}", report.detail);
        }
        Ok(())
    };

    // No server? Nothing to reload. This is a success so an installer can call
    // `jcode server reload` unconditionally after swapping the binary.
    if !crate::server::has_live_listener(&socket).await {
        // Reap a stale socket left by a crashed daemon so the next launch binds
        // cleanly instead of wedging in a connect-retry loop.
        let reaped = crate::server::reap_stale_socket_if_dead(&socket).await;
        let detail = if reaped {
            "No running jcode server found; cleared a stale socket.".to_string()
        } else {
            "No running jcode server found; nothing to reload.".to_string()
        };
        return emit(ServerReloadReport {
            socket: socket.display().to_string(),
            had_listener: false,
            forced: force,
            reloaded: false,
            already_current: false,
            handoff_ready: false,
            detail,
        });
    }

    let mut client = crate::server::Client::connect().await?;

    // Before asking the (possibly older) daemon to reload, repair a stale
    // `shared-server` channel from the client side. The running server resolves
    // its reload target from that channel; if it still points at the server's
    // own old binary (the "current client, stale server" state, e.g. after a
    // no-op `/update`), a forced reload would just re-exec the same old binary.
    // Repointing shared-server -> stable when stable is strictly newer gives the
    // reload a newer binary to exec into. Never downgrades; preserves a fresher
    // self-dev pin. Best-effort: a failure here must not block the reload.
    match crate::build::repair_stale_shared_server_channel() {
        Ok(crate::build::SharedServerRepair::Repaired {
            repaired_to,
            previous,
        }) => {
            crate::logging::info(&format!(
                "server reload: repaired stale shared-server channel {:?} -> {} before reload",
                previous, repaired_to
            ));
        }
        Ok(crate::build::SharedServerRepair::AlreadyCurrent) => {}
        Err(err) => {
            crate::logging::warn(&format!(
                "server reload: shared-server channel repair failed (continuing): {}",
                err
            ));
        }
    }

    let request_id = client.reload_with_force(force).await?;

    let mut reloading = false;
    let mut skipped = false;

    // Drive the request to a terminal state. On a real reload the old server
    // exec's a new process, which drops this connection after it sends Done;
    // we treat a disconnect after observing Reloading as the expected handoff.
    loop {
        match client.read_event().await {
            Ok(ServerEvent::Ack { id }) if id == request_id => {}
            Ok(ServerEvent::Reloading { .. }) => {
                reloading = true;
            }
            Ok(ServerEvent::ReloadProgress { step, .. }) if step == "skip" => {
                skipped = true;
            }
            Ok(ServerEvent::ReloadProgress { .. }) => {}
            Ok(ServerEvent::Done { id }) if id == request_id => break,
            Ok(ServerEvent::Error { id, message, .. }) if id == request_id => {
                anyhow::bail!("server reload failed: {message}");
            }
            Ok(_) => {}
            Err(e) => {
                // A disconnect mid-reload is the expected handoff; otherwise it
                // is a genuine failure.
                if reloading {
                    break;
                }
                return Err(e);
            }
        }
    }

    if skipped && !reloading {
        return emit(ServerReloadReport {
            socket: socket.display().to_string(),
            had_listener: true,
            forced: force,
            reloaded: false,
            already_current: true,
            handoff_ready: true,
            detail: "jcode server is already running the newest binary; no reload needed."
                .to_string(),
        });
    }

    // Wait (bounded) for the freshly exec'd server to take over the socket so
    // callers know the upgrade actually landed.
    let handoff_ready = matches!(
        crate::server::await_reload_handoff(&socket, Duration::from_secs(30)).await,
        crate::server::ReloadWaitStatus::Ready
    );

    let detail = if handoff_ready {
        "jcode server reloaded onto the newest binary.".to_string()
    } else {
        "jcode server reload requested; the new server is still coming up.".to_string()
    };

    emit(ServerReloadReport {
        socket: socket.display().to_string(),
        had_listener: true,
        forced: force,
        reloaded: true,
        already_current: false,
        handoff_ready,
        detail,
    })
}

/// Stop the running background server gracefully and clear its socket.
///
/// Intended for use after an upgrade so the next launch starts the freshly
/// installed binary instead of a surviving daemon running old code (issue #291).
///
/// Steps:
/// 1. Look up the daemon owning the active socket in the server registry and
///    send it SIGTERM (the daemon has a graceful SIGTERM handler).
/// 2. Wait for the listener to go away (bounded), escalating to SIGKILL only if
///    the process refuses to exit.
/// 3. Reap any leftover stale socket so a later launch binds cleanly.
pub async fn run_server_stop_command(force: bool, emit_json: bool) -> Result<()> {
    use std::time::{Duration, Instant};

    if !force {
        let msg = "`jcode server stop` terminates the daemon and drops any live headless/swarm sessions. \
Prefer `jcode server reload` to pick up an upgrade gracefully. \
Re-run with `--force` if you really want to stop the server.";
        if emit_json {
            println!(
                "{}",
                serde_json::json!({
                    "stopped": false,
                    "force_required": true,
                    "detail": msg,
                })
            );
        } else {
            eprintln!("{msg}");
        }
        return Ok(());
    }

    let socket = crate::server::socket_path();
    let had_listener = crate::server::has_live_listener(&socket).await;
    let server_info = crate::registry::find_server_by_socket_sync(&socket);

    #[derive(Serialize)]
    struct ServerStopReport {
        socket: String,
        had_listener: bool,
        signaled_pid: Option<u32>,
        stopped: bool,
        reaped_socket: bool,
        detail: String,
    }

    let mut signaled_pid: Option<u32> = None;
    let mut stopped = false;
    let detail: String;

    if let Some(info) = server_info.as_ref() {
        let pid = info.pid;
        if crate::platform::is_process_running(pid) {
            #[cfg(unix)]
            {
                // The daemon spawns detached with setsid(), so it leads its own
                // process group. Signal the group so any helper children exit too.
                match crate::platform::signal_detached_process_group(pid, libc::SIGTERM) {
                    Ok(()) => {
                        signaled_pid = Some(pid);
                        detail = format!("Sent SIGTERM to jcode server (pid {pid}).");
                    }
                    Err(e) => {
                        detail = format!("Failed to signal jcode server (pid {pid}): {e}");
                    }
                }
            }
            #[cfg(not(unix))]
            {
                match crate::platform::signal_detached_process_group(pid, 0) {
                    Ok(()) => {
                        signaled_pid = Some(pid);
                        detail = format!("Terminated jcode server (pid {pid}).");
                    }
                    Err(e) => {
                        detail = format!("Failed to terminate jcode server (pid {pid}): {e}");
                    }
                }
            }
        } else {
            detail = format!("Registered jcode server (pid {pid}) is not running.");
        }
    } else if had_listener {
        // A listener answers but no registry entry maps to it. We deliberately
        // do not guess a pid; just reap the socket below once the listener is
        // gone. (This is rare: a daemon that bound the socket but never wrote a
        // registry entry.)
        detail = "Found a live server socket with no registry entry.".to_string();
    } else {
        detail = "No running jcode server found.".to_string();
    }

    // Wait for the listener to disappear after signalling. Escalate to SIGKILL
    // once if the daemon does not exit within the graceful window.
    if signaled_pid.is_some() || had_listener {
        let deadline = Instant::now() + Duration::from_secs(5);
        #[cfg(unix)]
        let mut escalated = false;
        loop {
            let listener_gone = !crate::server::has_live_listener(&socket).await;
            let process_gone = signaled_pid
                .map(|pid| !crate::platform::is_process_running(pid))
                .unwrap_or(true);
            if listener_gone && process_gone {
                stopped = true;
                break;
            }
            if Instant::now() >= deadline {
                break;
            }
            #[cfg(unix)]
            if !escalated
                && Instant::now() + Duration::from_secs(2) >= deadline
                && let Some(pid) = signaled_pid
                && crate::platform::is_process_running(pid)
            {
                let _ = crate::platform::signal_detached_process_group(pid, libc::SIGKILL);
                escalated = true;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    } else {
        stopped = true;
    }

    // Reap any stale socket the (now-dead) daemon left behind so the next launch
    // binds cleanly instead of wedging in a connect-retry loop.
    let reaped = crate::server::reap_stale_socket_if_dead(&socket).await;

    if emit_json {
        let report = ServerStopReport {
            socket: socket.display().to_string(),
            had_listener,
            signaled_pid,
            stopped,
            reaped_socket: reaped,
            detail: detail.clone(),
        };
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        if !detail.is_empty() {
            println!("{detail}");
        }
        if stopped && signaled_pid.is_some() {
            println!("jcode server stopped.");
        } else if stopped && !had_listener && signaled_pid.is_none() {
            // Nothing was running; this is still a success for an installer.
        } else if !stopped {
            println!(
                "jcode server did not exit cleanly; it may still be shutting down. Re-run if needed."
            );
        }
        if reaped {
            println!("Cleared a stale jcode socket.");
        }
    }

    Ok(())
}

pub async fn run_single_message_command(
    choice: &super::provider_init::ProviderChoice,
    model: Option<&str>,
    resume_session: Option<&str>,
    message: &str,
    emit_json: bool,
    emit_ndjson: bool,
) -> Result<()> {
    let (provider, mut agent) =
        init_run_single_message_agent(choice, model, emit_json || emit_ndjson).await?;
    restore_agent_session_if_requested(&mut agent, resume_session)?;

    if emit_json {
        let text = run_single_message_command_capture_with_auto_poke(&mut agent, message).await?;
        let report = RunCommandReport {
            session_id: agent.session_id().to_string(),
            provider: provider.name().to_string(),
            model: provider.model(),
            text,
            usage: agent.last_usage().clone(),
        };
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else if emit_ndjson {
        run_single_message_command_ndjson(&mut agent, provider.clone(), message).await?;
    } else {
        run_single_message_command_plain_with_auto_poke(&mut agent, message).await?;
    }

    Ok(())
}

pub async fn run_single_message_command_once(
    choice: &super::provider_init::ProviderChoice,
    model: Option<&str>,
    resume_session: Option<&str>,
    message: &str,
) -> Result<()> {
    let (_provider, mut agent) = init_run_single_message_agent(choice, model, false).await?;
    restore_agent_session_if_requested(&mut agent, resume_session)?;
    run_single_message_command_plain_once(&mut agent, message).await
}

async fn init_run_single_message_agent(
    choice: &super::provider_init::ProviderChoice,
    model: Option<&str>,
    quiet_provider_init: bool,
) -> Result<(
    std::sync::Arc<dyn crate::provider::Provider>,
    crate::agent::Agent,
)> {
    let provider = if quiet_provider_init {
        super::provider_init::init_provider_quiet(choice, model).await?
    } else {
        super::provider_init::init_provider_for_validation(choice, model).await?
    };
    let registry = crate::tool::Registry::new(provider.clone()).await;
    // Load MCP servers from ~/.jcode/mcp.json so headless `jcode run` has the
    // same `mcp__*` tools as interactive/server sessions. This is non-blocking:
    // `register_mcp_tools` advertises cached tool schemas synchronously (so the
    // first locked tool snapshot already contains MCP tools, for zero
    // prompt-cache miss) and connects in the background (connect-on-first-call).
    // For a short single-message run, startup latency is unchanged.
    // (#390, #206 Phase 2)
    if run_command_mcp_enabled() {
        registry.register_mcp_tools(None, None, None).await;
        // Cold-cache gap: when a configured MCP server has no cached schema yet
        // (first ever use, or reconfigured), advertise-early registers nothing
        // for it, and a single-turn `jcode run` locks its tool snapshot before
        // the background connection finishes, so the model would never see those
        // tools. Long-lived sessions recover on a later turn, but `jcode run`
        // has no later turn. So, only when the cache is cold for some configured
        // server, briefly wait for the first connection to register tools before
        // the agent runs. Warm runs skip this entirely and stay instant. (#390)
        wait_for_cold_cache_mcp_tools(&registry).await;
    }
    let agent = crate::agent::Agent::new(provider.clone(), registry);
    Ok((provider, agent))
}

fn run_command_auto_poke_enabled() -> bool {
    std::env::var("JCODE_RUN_AUTO_POKE")
        .ok()
        .map(|value| {
            let value = value.trim().to_ascii_lowercase();
            !matches!(value.as_str(), "0" | "false" | "off" | "no")
        })
        .unwrap_or(true)
}

/// Whether headless `jcode run` should load MCP servers from `~/.jcode/mcp.json`.
/// Enabled by default; set `JCODE_RUN_MCP=0` (or `false`/`off`/`no`) to skip MCP
/// registration for latency-sensitive scripting. (#390)
fn run_command_mcp_enabled() -> bool {
    std::env::var("JCODE_RUN_MCP")
        .ok()
        .map(|value| {
            let value = value.trim().to_ascii_lowercase();
            !matches!(value.as_str(), "0" | "false" | "off" | "no")
        })
        .unwrap_or(true)
}

/// Max time `jcode run` waits for cold-cache MCP servers to register their
/// tools before running the single turn. Override with `JCODE_RUN_MCP_WAIT_MS`
/// (0 disables the wait).
fn run_command_mcp_cold_wait() -> std::time::Duration {
    let ms = std::env::var("JCODE_RUN_MCP_WAIT_MS")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(5000);
    std::time::Duration::from_millis(ms)
}

/// Returns the set of MCP servers configured for this run that have no usable
/// cached schema yet (cold cache). Advertise-early can only pre-register tools
/// for servers whose schemas are cached, so these are the servers whose tools
/// would otherwise miss the single-turn snapshot.
fn cold_cache_mcp_servers() -> Vec<String> {
    let config = crate::mcp::McpConfig::load();
    if config.servers.is_empty() {
        return Vec::new();
    }
    let cache = crate::mcp::McpSchemaCache::load();
    config
        .servers
        .iter()
        .filter(|(name, cfg)| cache.tools_for(name, cfg).is_none())
        .map(|(name, _)| name.clone())
        .collect()
}

/// Bridge the cold-cache gap for `jcode run`: if any configured MCP server has
/// no cached schema, briefly poll the registry until its `mcp__*` tools appear
/// (or the budget elapses) so the single turn's locked tool snapshot includes
/// them. Warm caches return immediately because `cold_cache_mcp_servers` is
/// empty. (#390)
async fn wait_for_cold_cache_mcp_tools(registry: &crate::tool::Registry) {
    let cold_servers = cold_cache_mcp_servers();
    if cold_servers.is_empty() {
        return;
    }
    let budget = run_command_mcp_cold_wait();
    if budget.is_zero() {
        return;
    }
    crate::logging::info(&format!(
        "jcode run: waiting up to {}ms for cold-cache MCP server(s) to register tools: {}",
        budget.as_millis(),
        cold_servers.join(", ")
    ));
    let deadline = std::time::Instant::now() + budget;
    loop {
        let names = registry.tool_names().await;
        let covered = cold_servers.iter().all(|server| {
            let prefix = format!("mcp__{}__", server);
            names.iter().any(|name| name.starts_with(&prefix))
        });
        if covered {
            crate::logging::info(
                "jcode run: cold-cache MCP server(s) registered tools; proceeding",
            );
            return;
        }
        if std::time::Instant::now() >= deadline {
            crate::logging::warn(
                "jcode run: timed out waiting for cold-cache MCP server(s); \
                 their tools may be missing from this run",
            );
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

fn run_command_auto_poke_max_turns() -> Option<usize> {
    std::env::var("JCODE_RUN_AUTO_POKE_MAX_TURNS")
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
}

fn run_command_auto_poke_limit_reached(turns_completed: usize, max_turns: Option<usize>) -> bool {
    max_turns
        .map(|max_turns| turns_completed >= max_turns)
        .unwrap_or(false)
}

const RUN_TODO_CONFIDENCE_THRESHOLD: u8 = 90;
const RUN_TODO_CONFIDENCE_SUMMARY_PREFIX: &str = "All todos are done. Todo confidence summary:";

enum RunAutoPokeFollowUp {
    Incomplete { count: usize, message: String },
    ConfidenceSummary { total_todos: usize, message: String },
}

fn run_todos(session_id: &str) -> Vec<crate::todo::TodoItem> {
    crate::todo::load_todos(session_id).unwrap_or_default()
}

fn build_run_auto_poke_follow_up_from_todos(
    todos: &[crate::todo::TodoItem],
    confidence_summary_sent: bool,
) -> Option<RunAutoPokeFollowUp> {
    let incomplete: Vec<_> = todos
        .iter()
        .filter(|todo| todo.status != "completed" && todo.status != "cancelled")
        .cloned()
        .collect();
    if !incomplete.is_empty() {
        return Some(RunAutoPokeFollowUp::Incomplete {
            count: incomplete.len(),
            message: build_run_poke_message(&incomplete),
        });
    }
    if !confidence_summary_sent && !todos.is_empty() {
        return Some(RunAutoPokeFollowUp::ConfidenceSummary {
            total_todos: todos.len(),
            message: build_run_todo_confidence_summary_message(todos),
        });
    }
    None
}

fn build_run_poke_message(incomplete: &[crate::todo::TodoItem]) -> String {
    format!(
        "You have {} incomplete todo{}. Continue working, or update the todo tool.",
        incomplete.len(),
        if incomplete.len() == 1 { "" } else { "s" },
    )
}

fn run_todo_confidence_weight(priority: &str) -> u32 {
    match priority {
        "high" => 3,
        "medium" => 2,
        _ => 1,
    }
}

fn run_weighted_confidence_average(scores: impl IntoIterator<Item = (u8, u32)>) -> Option<u8> {
    let mut weighted_sum = 0u32;
    let mut total_weight = 0u32;
    for (score, weight) in scores {
        weighted_sum += u32::from(score) * weight;
        total_weight += weight;
    }
    if total_weight == 0 {
        None
    } else {
        Some(((weighted_sum + total_weight / 2) / total_weight) as u8)
    }
}

fn build_run_todo_confidence_summary_message(todos: &[crate::todo::TodoItem]) -> String {
    let completed: Vec<&crate::todo::TodoItem> = todos
        .iter()
        .filter(|todo| todo.status == "completed")
        .collect();
    let cancelled_count = todos
        .iter()
        .filter(|todo| todo.status == "cancelled")
        .count();

    let planning_average = run_weighted_confidence_average(todos.iter().filter_map(|todo| {
        todo.confidence
            .map(|score| (score, run_todo_confidence_weight(&todo.priority)))
    }));
    let completion_scores: Vec<(&crate::todo::TodoItem, u8, u32)> = completed
        .iter()
        .filter_map(|todo| {
            todo.completion_confidence
                .map(|score| (*todo, score, run_todo_confidence_weight(&todo.priority)))
        })
        .collect();
    let completion_average = run_weighted_confidence_average(
        completion_scores
            .iter()
            .map(|(_, score, weight)| (*score, *weight)),
    );
    let missing_completion_confidence = completed
        .iter()
        .filter(|todo| todo.completion_confidence.is_none())
        .count();
    let below_threshold_count = completion_scores
        .iter()
        .filter(|(_, score, _)| *score < RUN_TODO_CONFIDENCE_THRESHOLD)
        .count();
    let lowest_completed = completion_scores
        .iter()
        .min_by_key(|(_, score, _)| *score)
        .map(|(_, score, _)| *score);

    let mut lines = vec![RUN_TODO_CONFIDENCE_SUMMARY_PREFIX.to_string()];
    lines.push(format!(
        "- Completed todos: {}{}.",
        completed.len(),
        if cancelled_count == 0 {
            String::new()
        } else {
            format!(
                " ({} cancelled todo{} skipped)",
                cancelled_count,
                if cancelled_count == 1 { "" } else { "s" }
            )
        }
    ));

    match completion_average {
        Some(avg) => lines.push(format!("- Weighted completion confidence: {}%.", avg)),
        None if !completed.is_empty() => lines.push(
            "- Weighted completion confidence: unknown because no completed todo has completion_confidence."
                .to_string(),
        ),
        None => lines.push("- No completed todos recorded completion confidence.".to_string()),
    }
    lines.push(format!(
        "- Confidence threshold: {}%.",
        RUN_TODO_CONFIDENCE_THRESHOLD
    ));

    match planning_average {
        Some(avg) => lines.push(format!("- Weighted planning confidence: {}%.", avg)),
        None => lines.push("- Weighted planning confidence: unknown.".to_string()),
    }

    match lowest_completed {
        Some(score) => lines.push(format!("- Lowest completed todo confidence: {}%.", score)),
        None => lines.push("- Lowest completed todo confidence: unknown.".to_string()),
    }

    if missing_completion_confidence > 0 {
        lines.push(format!(
            "- Missing completion_confidence on {} completed todo{}.",
            missing_completion_confidence,
            if missing_completion_confidence == 1 {
                ""
            } else {
                "s"
            }
        ));
    }

    if below_threshold_count > 0 {
        lines.push(format!(
            "- {} completed todo{} below the {}% confidence threshold.",
            below_threshold_count,
            if below_threshold_count == 1 {
                " is"
            } else {
                "s are"
            },
            RUN_TODO_CONFIDENCE_THRESHOLD
        ));
    }

    let needs_validation = completion_average
        .map(|avg| avg < RUN_TODO_CONFIDENCE_THRESHOLD)
        .unwrap_or(true)
        || missing_completion_confidence > 0
        || below_threshold_count > 0;
    if needs_validation {
        lines.push(format!(
            "- {}",
            crate::prompt::TODO_CONFIDENCE_NEEDS_VALIDATION_PROMPT.trim()
        ));
    } else {
        lines.push(format!(
            "- {}",
            crate::prompt::TODO_CONFIDENCE_READY_PROMPT.trim()
        ));
    }

    lines.join("\n")
}

async fn run_single_message_command_plain_with_auto_poke(
    agent: &mut crate::agent::Agent,
    message: &str,
) -> Result<()> {
    let mut next_message = message.to_string();
    let max_turns = run_command_auto_poke_max_turns();
    let mut turns_completed = 0usize;
    let mut confidence_summary_sent = false;
    loop {
        run_single_message_command_plain_once(agent, &next_message).await?;
        turns_completed += 1;
        if !run_command_auto_poke_enabled() {
            break;
        }
        let todos = run_todos(agent.session_id());
        match build_run_auto_poke_follow_up_from_todos(&todos, confidence_summary_sent) {
            Some(RunAutoPokeFollowUp::ConfidenceSummary { message, .. }) => {
                confidence_summary_sent = true;
                next_message = message;
                eprintln!(
                    "Auto-poking: todos complete; sending confidence summary follow-up. Set JCODE_RUN_AUTO_POKE=0 to disable."
                );
                continue;
            }
            Some(RunAutoPokeFollowUp::Incomplete { count, message }) => {
                if run_command_auto_poke_limit_reached(turns_completed, max_turns) {
                    if let Some(max_turns) = max_turns {
                        eprintln!(
                            "Auto-poke stopped after {max_turns} turn(s) with {} incomplete todo(s).",
                            count
                        );
                    }
                    break;
                }
                next_message = message;
                eprintln!(
                    "Auto-poking: {} incomplete todo(s). Set JCODE_RUN_AUTO_POKE=0 to disable.",
                    count
                );
            }
            None => break,
        }
    }
    Ok(())
}

async fn run_single_message_command_plain_once(
    agent: &mut crate::agent::Agent,
    message: &str,
) -> Result<()> {
    agent.run_once(message).await
}

async fn run_single_message_command_capture_with_auto_poke(
    agent: &mut crate::agent::Agent,
    message: &str,
) -> Result<String> {
    let mut next_message = message.to_string();
    let max_turns = run_command_auto_poke_max_turns();
    let mut outputs = Vec::new();
    let mut turns_completed = 0usize;
    let mut confidence_summary_sent = false;
    loop {
        outputs.push(agent.run_once_capture(&next_message).await?);
        turns_completed += 1;
        if !run_command_auto_poke_enabled() {
            break;
        }
        let todos = run_todos(agent.session_id());
        match build_run_auto_poke_follow_up_from_todos(&todos, confidence_summary_sent) {
            Some(RunAutoPokeFollowUp::ConfidenceSummary { message, .. }) => {
                confidence_summary_sent = true;
                next_message = message;
                continue;
            }
            Some(RunAutoPokeFollowUp::Incomplete { count, message }) => {
                if run_command_auto_poke_limit_reached(turns_completed, max_turns) {
                    if let Some(max_turns) = max_turns {
                        outputs.push(format!(
                            "Auto-poke stopped after {max_turns} turn(s) with {} incomplete todo(s).",
                            count
                        ));
                    }
                    break;
                }
                next_message = message;
            }
            None => break,
        }
    }
    Ok(outputs.join("\n\n"))
}

fn restore_agent_session_if_requested(
    agent: &mut crate::agent::Agent,
    resume_session: Option<&str>,
) -> Result<()> {
    if let Some(session_id) = resume_session {
        agent.restore_session(session_id)?;
    }
    Ok(())
}

async fn run_single_message_command_ndjson(
    agent: &mut crate::agent::Agent,
    provider: std::sync::Arc<dyn crate::provider::Provider>,
    message: &str,
) -> Result<()> {
    let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel();
    let session_id = agent.session_id().to_string();
    let mut stdout = std::io::stdout().lock();
    let mut state = NdjsonRunState {
        session_id: Some(session_id.clone()),
        ..NdjsonRunState::default()
    };
    write_json_line(
        &mut stdout,
        &serde_json::json!({
            "type": "start",
            "session_id": session_id,
            "provider": provider.name(),
            "model": provider.model(),
        }),
    )?;

    let max_turns = run_command_auto_poke_max_turns();
    let mut next_message = message.to_string();
    let mut result: Result<()> = Ok(());
    let mut turns_completed = 0usize;
    let mut confidence_summary_sent = false;
    loop {
        let turn_result = {
            let mut run_future = std::pin::pin!(agent.run_once_streaming_mpsc(
                &next_message,
                Vec::new(),
                None,
                event_tx.clone(),
            ));
            let mut run_result: Option<Result<()>> = None;
            loop {
                tokio::select! {
                    result = &mut run_future, if run_result.is_none() => {
                        run_result = Some(result);
                    }
                    event = event_rx.recv() => {
                        match event {
                            Some(event) => emit_ndjson_event(&mut stdout, &mut state, event)?,
                            None => break,
                        }
                    }
                }
                if run_result.is_some() {
                    while let Ok(event) = event_rx.try_recv() {
                        emit_ndjson_event(&mut stdout, &mut state, event)?;
                    }
                    break;
                }
            }
            run_result.unwrap_or(Ok(()))
        };

        if let Err(err) = turn_result {
            result = Err(err);
            break;
        }
        turns_completed += 1;
        if !run_command_auto_poke_enabled() {
            break;
        }
        let todos = run_todos(&session_id);
        match build_run_auto_poke_follow_up_from_todos(&todos, confidence_summary_sent) {
            Some(RunAutoPokeFollowUp::ConfidenceSummary {
                total_todos,
                message,
            }) => {
                confidence_summary_sent = true;
                next_message = message;
                write_json_line(
                    &mut stdout,
                    &serde_json::json!({
                        "type": "auto_poke_confidence_summary",
                        "session_id": session_id,
                        "todos": total_todos,
                        "message": next_message,
                    }),
                )?;
                continue;
            }
            Some(RunAutoPokeFollowUp::Incomplete { count, message }) => {
                if run_command_auto_poke_limit_reached(turns_completed, max_turns) {
                    if let Some(max_turns) = max_turns {
                        write_json_line(
                            &mut stdout,
                            &serde_json::json!({
                                "type": "auto_poke_stopped",
                                "session_id": session_id,
                                "incomplete_todos": count,
                                "max_turns": max_turns,
                            }),
                        )?;
                    }
                    break;
                }
                next_message = message;
                write_json_line(
                    &mut stdout,
                    &serde_json::json!({
                        "type": "auto_poke",
                        "session_id": session_id,
                        "incomplete_todos": count,
                        "message": next_message,
                    }),
                )?;
            }
            None => break,
        }
    }

    match result {
        Ok(()) => {
            write_json_line(
                &mut stdout,
                &serde_json::json!({
                    "type": "done",
                    "session_id": session_id,
                    "provider": provider.name(),
                    "model": provider.model(),
                    "text": state.text,
                    "usage": state.usage,
                    "upstream_provider": state.upstream_provider,
                    "connection_type": state.connection_type,
                    "connection_phase": state.connection_phase,
                    "status_detail": state.status_detail,
                }),
            )?;
            Ok(())
        }
        Err(err) => {
            write_json_line(
                &mut stdout,
                &serde_json::json!({
                    "type": "error",
                    "session_id": session_id,
                    "provider": provider.name(),
                    "model": provider.model(),
                    "message": format!("{err:#}"),
                }),
            )?;
            Err(err)
        }
    }
}

fn emit_ndjson_event(
    stdout: &mut impl Write,
    state: &mut NdjsonRunState,
    event: crate::protocol::ServerEvent,
) -> Result<()> {
    use crate::protocol::ServerEvent;

    match event {
        ServerEvent::TextDelta { text } => {
            state.text.push_str(&text);
            write_json_line(
                stdout,
                &serde_json::json!({ "type": "text_delta", "text": text }),
            )
        }
        ServerEvent::TextReplace { text } => {
            state.text = text.clone();
            write_json_line(
                stdout,
                &serde_json::json!({ "type": "text_replace", "text": text }),
            )
        }
        ServerEvent::ToolStart { id, name } => write_json_line(
            stdout,
            &serde_json::json!({ "type": "tool_start", "id": id, "name": name }),
        ),
        ServerEvent::ToolInput { delta } => write_json_line(
            stdout,
            &serde_json::json!({ "type": "tool_input", "delta": delta }),
        ),
        ServerEvent::ToolExec { id, name } => write_json_line(
            stdout,
            &serde_json::json!({ "type": "tool_exec", "id": id, "name": name }),
        ),
        ServerEvent::ToolDone {
            id,
            name,
            output,
            error,
        } => write_json_line(
            stdout,
            &serde_json::json!({
                "type": "tool_done",
                "id": id,
                "name": name,
                "output": output,
                "error": error,
            }),
        ),
        ServerEvent::TokenUsage {
            input,
            output,
            cache_read_input,
            cache_creation_input,
        } => {
            state.usage = crate::agent::TokenUsage {
                input_tokens: input,
                output_tokens: output,
                cache_read_input_tokens: cache_read_input,
                cache_creation_input_tokens: cache_creation_input,
            };
            write_json_line(
                stdout,
                &serde_json::json!({
                    "type": "tokens",
                    "input": input,
                    "output": output,
                    "cache_read_input": cache_read_input,
                    "cache_creation_input": cache_creation_input,
                }),
            )
        }
        ServerEvent::ConnectionType { connection } => {
            state.connection_type = Some(connection.clone());
            write_json_line(
                stdout,
                &serde_json::json!({ "type": "connection_type", "connection": connection }),
            )
        }
        ServerEvent::ConnectionPhase { phase } => {
            state.connection_phase = Some(phase.clone());
            write_json_line(
                stdout,
                &serde_json::json!({ "type": "connection_phase", "phase": phase }),
            )
        }
        ServerEvent::StatusDetail { detail } => {
            state.status_detail = Some(detail.clone());
            write_json_line(
                stdout,
                &serde_json::json!({ "type": "status_detail", "detail": detail }),
            )
        }
        ServerEvent::MessageEnd => {
            write_json_line(stdout, &serde_json::json!({ "type": "message_end" }))
        }
        ServerEvent::UpstreamProvider { provider } => {
            state.upstream_provider = Some(provider.clone());
            write_json_line(
                stdout,
                &serde_json::json!({ "type": "upstream_provider", "provider": provider }),
            )
        }
        ServerEvent::SessionId { session_id } => {
            state.session_id = Some(session_id.clone());
            write_json_line(
                stdout,
                &serde_json::json!({ "type": "session", "session_id": session_id }),
            )
        }
        ServerEvent::Compaction {
            trigger,
            pre_tokens,
            messages_dropped,
            post_tokens,
            tokens_saved,
            duration_ms,
            messages_compacted,
            summary_chars,
            active_messages,
        } => write_json_line(
            stdout,
            &serde_json::json!({
                "type": "compaction",
                "trigger": trigger,
                "pre_tokens": pre_tokens,
                "messages_dropped": messages_dropped,
                "post_tokens": post_tokens,
                "tokens_saved": tokens_saved,
                "duration_ms": duration_ms,
                "messages_compacted": messages_compacted,
                "summary_chars": summary_chars,
                "active_messages": active_messages,
            }),
        ),
        ServerEvent::MemoryInjected {
            count,
            prompt_chars,
            computed_age_ms,
            ..
        } => write_json_line(
            stdout,
            &serde_json::json!({
                "type": "memory_injected",
                "count": count,
                "prompt_chars": prompt_chars,
                "computed_age_ms": computed_age_ms,
            }),
        ),
        ServerEvent::Interrupted => {
            write_json_line(stdout, &serde_json::json!({ "type": "interrupted" }))
        }
        ServerEvent::SoftInterruptInjected {
            content,
            display_role,
            point,
            tools_skipped,
        } => write_json_line(
            stdout,
            &serde_json::json!({
                "type": "soft_interrupt_injected",
                "content": content,
                "display_role": display_role,
                "point": point,
                "tools_skipped": tools_skipped,
            }),
        ),
        ServerEvent::BatchProgress { progress } => write_json_line(
            stdout,
            &serde_json::json!({ "type": "batch_progress", "progress": progress }),
        ),
        ServerEvent::Error {
            message,
            retry_after_secs,
            ..
        } => write_json_line(
            stdout,
            &serde_json::json!({
                "type": "error",
                "message": message,
                "retry_after_secs": retry_after_secs,
            }),
        ),
        ServerEvent::Ack { .. } | ServerEvent::Done { .. } | ServerEvent::Pong { .. } => Ok(()),
        _ => Ok(()),
    }
}

fn write_json_line(stdout: &mut impl Write, value: &impl Serialize) -> Result<()> {
    serde_json::to_writer(&mut *stdout, value)?;
    stdout.write_all(b"\n")?;
    stdout.flush()?;
    Ok(())
}

pub async fn run_model_command(
    choice: &super::provider_init::ProviderChoice,
    model: Option<&str>,
    emit_json: bool,
    verbose: bool,
) -> Result<()> {
    let provider = super::provider_init::init_provider_quiet(choice, model).await?;

    if let Err(err) = provider.prefetch_models().await
        && !super::output::quiet_enabled()
    {
        eprintln!("Warning: failed to refresh dynamic model list: {}", err);
    }

    let routes = provider.model_routes();
    let filtered_routes = filter_cli_model_routes_for_choice(choice, &routes);
    let models = if filtered_routes.len() == routes.len() {
        collect_cli_model_names(&routes, provider.available_models_display())
    } else {
        collect_cli_model_names(&filtered_routes, Vec::new())
    };

    if models.is_empty() {
        anyhow::bail!(
            "No models found for provider '{}'. Check credentials or try a different --provider.",
            provider.name()
        );
    }

    if emit_json {
        let provider_label = super::provider_init::login_provider_for_choice(choice)
            .map(|provider| provider.display_name.to_string())
            .unwrap_or_else(|| {
                crate::provider_catalog::runtime_provider_display_name(provider.name())
            });
        let report = ModelListReport {
            provider: provider_label,
            selected_model: provider.model(),
            models,
            routes: filtered_routes
                .iter()
                .map(|route| ModelListRouteReport {
                    provider: cli_route_provider_display(&route.provider, &route.api_method),
                    model: route.model.clone(),
                    method: cli_api_method_display(&route.api_method),
                    available: route.available,
                })
                .collect(),
        };
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        if verbose {
            println!(
                "Provider: {}",
                crate::provider_catalog::runtime_provider_display_name(provider.name())
            );
            println!("Selected model: {}", provider.model());
            println!("Available models: {}", models.len());
            println!();
        }
        for model in models {
            println!("{}", model);
        }
    }

    Ok(())
}

fn cli_api_method_display(raw: &str) -> String {
    crate::provider::ModelRouteApiMethod::parse(raw).display_label()
}

fn cli_route_provider_display(provider: &str, api_method: &str) -> String {
    if crate::provider::ModelRouteApiMethod::parse(api_method).is_openrouter()
        && provider != "auto"
        && !provider.contains("OpenRouter")
    {
        format!("OpenRouter/{}", provider)
    } else {
        provider.to_string()
    }
}

fn collect_cli_model_names(
    routes: &[crate::provider::ModelRoute],
    display_models: Vec<String>,
) -> Vec<String> {
    let mut deduped = Vec::new();
    let mut seen = BTreeSet::new();

    fn push_model(deduped: &mut Vec<String>, seen: &mut BTreeSet<String>, model: &str) {
        let trimmed = model.trim();
        if !crate::provider::is_listable_model_name(trimmed) {
            return;
        }
        if seen.insert(trimmed.to_string()) {
            deduped.push(trimmed.to_string());
        }
    }

    for route in routes.iter().filter(|route| route.available) {
        push_model(&mut deduped, &mut seen, &route.model);
    }

    if deduped.is_empty() {
        for route in routes {
            push_model(&mut deduped, &mut seen, &route.model);
        }
    }

    for model in display_models {
        push_model(&mut deduped, &mut seen, &model);
    }

    deduped
}

#[allow(deprecated)]
fn filter_cli_model_routes_for_choice(
    choice: &super::provider_init::ProviderChoice,
    routes: &[crate::provider::ModelRoute],
) -> Vec<crate::provider::ModelRoute> {
    use super::provider_init::ProviderChoice;

    let keep = |route: &&crate::provider::ModelRoute| match choice {
        ProviderChoice::Claude | ProviderChoice::ClaudeSubprocess => {
            route.api_method_kind().is_anthropic_credential_route()
        }
        ProviderChoice::Openai => matches!(
            route.api_method_kind(),
            crate::provider::ModelRouteApiMethod::OpenAIOAuth
        ),
        ProviderChoice::OpenaiApi => matches!(
            route.api_method_kind(),
            crate::provider::ModelRouteApiMethod::OpenAIApiKey
        ),
        ProviderChoice::Openrouter | ProviderChoice::Azure => {
            route.api_method_kind().is_openrouter()
        }
        ProviderChoice::Copilot => route.api_method_kind().is_copilot(),
        _ => true,
    };

    let filtered: Vec<_> = routes.iter().filter(keep).cloned().collect();
    if filtered.is_empty() {
        routes.to_vec()
    } else {
        filtered
    }
}
#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
