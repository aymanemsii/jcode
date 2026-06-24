use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use std::path::Path;
use std::time::{Duration, Instant};

const AUTO_REFRESH_INTERVAL: Duration = Duration::from_secs(2);

pub(super) fn run_read_only_queue_board(
    mut terminal: ratatui::DefaultTerminal,
    mut board: crate::queue::QueueBoard,
    mut active_runs: Vec<crate::queue::RunState>,
    options: QueueBoardTuiOptions,
) -> Result<()> {
    let mut status_message = None;
    let mut selection = BoardSelection::default();
    let mut input_mode: Option<AddTaskPrompt> = None;
    let mut auto_refresh = AutoRefreshState::new(Instant::now());
    let mut state = crate::queue::load()?;
    let mut run_index = crate::queue::load_run_index()?;
    select_first_available_task(&board, &mut selection);
    terminal.draw(|frame| {
        draw_queue_board(
            frame,
            &board,
            &state,
            &run_index,
            &active_runs,
            &selection,
            None,
            None,
            auto_refresh.enabled,
        )
    })?;

    loop {
        let next_event = if auto_refresh.enabled {
            if event::poll(auto_refresh.poll_timeout(Instant::now()))? {
                Some(event::read()?)
            } else {
                None
            }
        } else {
            Some(event::read()?)
        };

        match next_event {
            None => {
                if auto_refresh.mark_refresh_if_due(Instant::now()) {
                    status_message = Some(refresh_board_state(
                        &mut board,
                        &mut state,
                        &mut run_index,
                        &mut active_runs,
                        &options,
                    )?);
                    preserve_selection_after_refresh(&board, &mut selection);
                    terminal.draw(|frame| {
                        draw_queue_board(
                            frame,
                            &board,
                            &state,
                            &run_index,
                            &active_runs,
                            &selection,
                            status_message.as_deref(),
                            input_mode.as_ref(),
                            auto_refresh.enabled,
                        )
                    })?;
                }
            }
            Some(Event::Key(key)) if key.kind == KeyEventKind::Press => {
                if let Some(input) = input_mode.as_mut() {
                    let mut submit: Option<AddTaskPrompt> = None;
                    let mut cancel = false;
                    match key.code {
                        KeyCode::Esc => {
                            cancel = true;
                        }
                        KeyCode::Enter => match input.step {
                            AddTaskPromptStep::Title => {
                                if input.title.trim().is_empty() {
                                    status_message = Some("title required".to_string());
                                } else {
                                    input.step = AddTaskPromptStep::WorkerProfile;
                                    status_message = None;
                                }
                            }
                            AddTaskPromptStep::WorkerProfile => {
                                submit = Some(input.clone());
                            }
                        },
                        KeyCode::Backspace => {
                            input.current_value_mut().pop();
                        }
                        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                            input.current_value_mut().push(c);
                        }
                        _ => {}
                    }

                    if cancel {
                        input_mode = None;
                        status_message = Some("add cancelled".to_string());
                    } else if let Some(input) = submit {
                        match add_task_from_prompt(
                            &mut board,
                            &mut state,
                            &mut run_index,
                            &mut active_runs,
                            &mut selection,
                            &input,
                            &options,
                        ) {
                            Ok(message) => {
                                input_mode = None;
                                status_message = Some(message);
                            }
                            Err(err) => {
                                status_message = Some(err.to_string());
                            }
                        }
                    }
                } else {
                    match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => break,
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            break;
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            move_selection_within_column(&board, &mut selection, 1);
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            move_selection_within_column(&board, &mut selection, -1);
                        }
                        KeyCode::Right | KeyCode::Char('l') => {
                            move_selection_to_column(&board, &mut selection, 1);
                        }
                        KeyCode::Left | KeyCode::Char('h') => {
                            move_selection_to_column(&board, &mut selection, -1);
                        }
                        KeyCode::Char('n') => {
                            input_mode = Some(AddTaskPrompt::default());
                            status_message = None;
                        }
                        KeyCode::Char('r') => {
                            status_message = Some(refresh_board_state(
                                &mut board,
                                &mut state,
                                &mut run_index,
                                &mut active_runs,
                                &options,
                            )?);
                            preserve_selection_after_refresh(&board, &mut selection);
                            auto_refresh.record_refresh(Instant::now());
                        }
                        KeyCode::Char('t') => {
                            auto_refresh.toggle(Instant::now());
                            status_message = Some(if auto_refresh.enabled {
                                "auto-refresh on".to_string()
                            } else {
                                "auto-refresh off".to_string()
                            });
                        }
                        KeyCode::Char('a') => {
                            status_message = Some(approve_selected_review_task(
                                &mut board,
                                &mut state,
                                &mut run_index,
                                &mut active_runs,
                                selection.selected_task_id(),
                                &options,
                            )?);
                            preserve_selection_after_refresh(&board, &mut selection);
                        }
                        KeyCode::Char('x') => {
                            status_message = Some(run_selected_task_in_background(
                                &mut board,
                                &mut state,
                                &mut run_index,
                                &mut active_runs,
                                selection.selected_task_id(),
                                &options,
                            )?);
                            preserve_selection_after_refresh(&board, &mut selection);
                        }
                        _ => {}
                    }
                }
                terminal.draw(|frame| {
                    draw_queue_board(
                        frame,
                        &board,
                        &state,
                        &run_index,
                        &active_runs,
                        &selection,
                        status_message.as_deref(),
                        input_mode.as_ref(),
                        auto_refresh.enabled,
                    )
                })?;
            }
            Some(Event::Resize(_, _)) => {
                terminal.draw(|frame| {
                    draw_queue_board(
                        frame,
                        &board,
                        &state,
                        &run_index,
                        &active_runs,
                        &selection,
                        status_message.as_deref(),
                        input_mode.as_ref(),
                        auto_refresh.enabled,
                    )
                })?;
            }
            Some(_) => {}
        }
    }

    Ok(())
}

pub(super) struct QueueBoardTuiOptions {
    pub(super) worker_profile: Option<String>,
    pub(super) limit: usize,
}

fn draw_queue_board(
    frame: &mut Frame<'_>,
    board: &crate::queue::QueueBoard,
    state: &crate::queue::QueueState,
    run_index: &crate::queue::RunIndex,
    active_runs: &[crate::queue::RunState],
    selection: &BoardSelection,
    status_message: Option<&str>,
    input_mode: Option<&AddTaskPrompt>,
    auto_refresh_enabled: bool,
) {
    let area = frame.area();
    let details_text = selected_task_details_text(selection.selected_task_id(), state, run_index);
    let wide = area.width >= 120;
    let mut constraints = vec![Constraint::Length(3), Constraint::Min(5)];
    if !wide {
        constraints.push(Constraint::Length(details_panel_height(area.height)));
    }
    constraints.push(Constraint::Length(active_runs_height(active_runs)));
    constraints.push(Constraint::Length(1));
    let layout = Layout::vertical(constraints).split(area);

    let board_area = if wide {
        let body =
            Layout::horizontal([Constraint::Min(60), Constraint::Length(46)]).split(layout[1]);
        frame.render_widget(details_panel(details_text), body[1]);
        body[0]
    } else {
        frame.render_widget(details_panel(details_text), layout[2]);
        layout[1]
    };
    let active_runs_area = if wide { layout[2] } else { layout[3] };
    let footer_area = if wide { layout[3] } else { layout[4] };

    frame.render_widget(
        Paragraph::new(header_text(board))
            .block(Block::default().borders(Borders::ALL).title("Queue board")),
        layout[0],
    );

    let column_constraints =
        vec![Constraint::Ratio(1, board.columns.len() as u32); board.columns.len()];
    let columns = Layout::horizontal(column_constraints).split(board_area);
    for (column_index, (column_area, column)) in
        columns.iter().zip(board.columns.iter()).enumerate()
    {
        frame.render_widget(
            render_column(column, selection, column_index == selection.column_index),
            *column_area,
        );
    }

    if active_runs_height(active_runs) > 0 {
        frame.render_widget(
            Paragraph::new(active_runs_text(active_runs))
                .block(Block::default().borders(Borders::ALL).title("Active runs"))
                .wrap(Wrap { trim: true }),
            active_runs_area,
        );
    }

    frame.render_widget(
        Paragraph::new(footer_text(
            status_message,
            input_mode,
            auto_refresh_enabled,
        )),
        footer_area,
    );
}

fn refresh_board_state(
    board: &mut crate::queue::QueueBoard,
    state: &mut crate::queue::QueueState,
    index: &mut crate::queue::RunIndex,
    active_runs: &mut Vec<crate::queue::RunState>,
    options: &QueueBoardTuiOptions,
) -> Result<String> {
    *index = crate::queue::load_run_index()?;
    *state = crate::queue::load()?;
    let output = super::refresh_queue_runs(index, state, chrono::Utc::now());
    if output.run_index_changed {
        crate::queue::save_run_index(index)?;
    }
    if output.queue_changed {
        crate::queue::save(state)?;
    }

    *board = crate::queue::build_queue_board(
        state,
        options.worker_profile.as_deref(),
        Some(options.limit),
    );
    *active_runs = filtered_active_runs(index, options.worker_profile.as_deref());

    Ok(refresh_status_text(&output))
}

#[derive(Debug, Clone)]
struct AutoRefreshState {
    enabled: bool,
    interval: Duration,
    last_refresh_at: Instant,
}

impl AutoRefreshState {
    fn new(now: Instant) -> Self {
        Self {
            enabled: true,
            interval: AUTO_REFRESH_INTERVAL,
            last_refresh_at: now,
        }
    }

    fn poll_timeout(&self, now: Instant) -> Duration {
        self.interval
            .checked_sub(now.saturating_duration_since(self.last_refresh_at))
            .unwrap_or_default()
    }

    fn mark_refresh_if_due(&mut self, now: Instant) -> bool {
        if !self.enabled || now.saturating_duration_since(self.last_refresh_at) < self.interval {
            return false;
        }

        self.last_refresh_at = now;
        true
    }

    fn record_refresh(&mut self, now: Instant) {
        self.last_refresh_at = now;
    }

    fn toggle(&mut self, now: Instant) {
        self.enabled = !self.enabled;
        self.last_refresh_at = now;
    }
}

fn approve_selected_review_task(
    board: &mut crate::queue::QueueBoard,
    state: &mut crate::queue::QueueState,
    index: &mut crate::queue::RunIndex,
    active_runs: &mut Vec<crate::queue::RunState>,
    selected_task_id: Option<&str>,
    options: &QueueBoardTuiOptions,
) -> Result<String> {
    let message =
        approve_selected_review_task_in_state(board, state, selected_task_id, chrono::Utc::now())?;
    if message.starts_with("approved ") {
        crate::queue::save(state)?;
        *index = crate::queue::load_run_index()?;
        *board = crate::queue::build_queue_board(
            state,
            options.worker_profile.as_deref(),
            Some(options.limit),
        );
        *active_runs = filtered_active_runs(index, options.worker_profile.as_deref());
    }
    Ok(message)
}

fn approve_selected_review_task_in_state(
    board: &crate::queue::QueueBoard,
    state: &mut crate::queue::QueueState,
    selected_task_id: Option<&str>,
    updated_at: chrono::DateTime<chrono::Utc>,
) -> Result<String> {
    let Some(task_id) = selected_task_id else {
        return Ok("no task selected".to_string());
    };
    if selected_task_status(board, task_id) != Some(crate::queue::TaskStatus::Review) {
        return Ok("selected task is not in review".to_string());
    }
    if state_task_status(state, task_id) != Some(crate::queue::TaskStatus::Review) {
        return Ok("selected task is not in review".to_string());
    }

    super::approve_queue_task(state, task_id, updated_at)?;
    Ok(format!("approved {task_id}"))
}

fn run_selected_task_in_background(
    board: &mut crate::queue::QueueBoard,
    state: &mut crate::queue::QueueState,
    index: &mut crate::queue::RunIndex,
    active_runs: &mut Vec<crate::queue::RunState>,
    selected_task_id: Option<&str>,
    options: &QueueBoardTuiOptions,
) -> Result<String> {
    run_selected_task_in_background_with_starter(
        board,
        state,
        index,
        active_runs,
        selected_task_id,
        options,
        super::start_rendered_worker_command_background,
    )
}

fn run_selected_task_in_background_with_starter(
    board: &mut crate::queue::QueueBoard,
    state: &mut crate::queue::QueueState,
    index: &mut crate::queue::RunIndex,
    active_runs: &mut Vec<crate::queue::RunState>,
    selected_task_id: Option<&str>,
    options: &QueueBoardTuiOptions,
    background_starter: impl FnMut(
        &str,
        &std::path::Path,
        &std::path::Path,
        &std::path::Path,
    ) -> Result<u32>,
) -> Result<String> {
    let Some(task_id) = selected_task_id else {
        return Ok("no task selected".to_string());
    };
    if !selected_task_is_actionable(board, task_id) {
        return Ok("selected task is not actionable".to_string());
    }

    let persisted_state = crate::queue::load()?;
    let Some(task) = persisted_state.tasks.iter().find(|task| task.id == task_id) else {
        return Ok("selected task is not actionable".to_string());
    };
    if !task_status_is_actionable(task.status) {
        return Ok("selected task is not actionable".to_string());
    }
    if task
        .worker_profile
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        return Ok("selected task has no worker profile".to_string());
    }
    drop(persisted_state);

    let output =
        super::start_selected_queue_task_background_with_starter(task_id, background_starter)?;
    *state = crate::queue::load()?;
    *index = crate::queue::load_run_index()?;
    *board = crate::queue::build_queue_board(
        state,
        options.worker_profile.as_deref(),
        Some(options.limit),
    );
    *active_runs = filtered_active_runs(index, options.worker_profile.as_deref());

    Ok(format!("started {} as {}", output.task_id, output.run_id))
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct AddTaskPrompt {
    title: String,
    worker_profile: String,
    step: AddTaskPromptStep,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum AddTaskPromptStep {
    #[default]
    Title,
    WorkerProfile,
}

impl AddTaskPrompt {
    fn current_value_mut(&mut self) -> &mut String {
        match self.step {
            AddTaskPromptStep::Title => &mut self.title,
            AddTaskPromptStep::WorkerProfile => &mut self.worker_profile,
        }
    }
}

fn add_task_from_prompt(
    board: &mut crate::queue::QueueBoard,
    state: &mut crate::queue::QueueState,
    index: &mut crate::queue::RunIndex,
    active_runs: &mut Vec<crate::queue::RunState>,
    selection: &mut BoardSelection,
    input: &AddTaskPrompt,
    options: &QueueBoardTuiOptions,
) -> Result<String> {
    let worker_profile = normalized_worker_profile(input.worker_profile.as_str());
    validate_add_task_worker_profile(worker_profile)?;

    *state = crate::queue::load()?;
    let task_id = add_task_to_state(state, &input.title, worker_profile)?;
    crate::queue::save(state)?;

    *index = crate::queue::load_run_index()?;
    *board = crate::queue::build_queue_board(
        state,
        options.worker_profile.as_deref(),
        Some(options.limit),
    );
    *active_runs = filtered_active_runs(index, options.worker_profile.as_deref());
    if !select_task_by_id(board, selection, &task_id) {
        preserve_selection_after_refresh(board, selection);
    }

    Ok(format!("added {task_id}"))
}

fn add_task_to_state(
    state: &mut crate::queue::QueueState,
    title: &str,
    worker_profile: Option<&str>,
) -> Result<String> {
    let title = title.trim();
    if title.is_empty() {
        anyhow::bail!("title required");
    }

    let task = crate::queue::Task::new(
        title.to_string(),
        String::new(),
        None,
        crate::queue::TaskPriority::Normal,
        normalized_worker_profile(worker_profile.unwrap_or_default()).map(str::to_string),
        None,
    );
    let task_id = task.id.clone();
    state.tasks.push(task);
    Ok(task_id)
}

fn normalized_worker_profile(worker_profile: &str) -> Option<&str> {
    let worker_profile = worker_profile.trim();
    if worker_profile.is_empty() {
        None
    } else {
        Some(worker_profile)
    }
}

fn validate_add_task_worker_profile(worker_profile: Option<&str>) -> Result<()> {
    let Some(worker_profile) = worker_profile else {
        return Ok(());
    };

    match load_worker_profile_discovery()? {
        WorkerProfileDiscovery::Missing => Ok(()),
        WorkerProfileDiscovery::Profiles(names) => {
            if names.iter().any(|name| name == worker_profile) {
                Ok(())
            } else {
                anyhow::bail!(
                    "unknown worker profile: {worker_profile} | {}",
                    worker_profile_discovery_message(&WorkerProfileDiscovery::Profiles(names))
                );
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum WorkerProfileDiscovery {
    Missing,
    Profiles(Vec<String>),
}

fn load_worker_profile_discovery() -> Result<WorkerProfileDiscovery> {
    let path = crate::queue::worker_profiles_file_path()?;
    if !path.exists() {
        return Ok(WorkerProfileDiscovery::Missing);
    }

    let mut names: Vec<String> = crate::queue::load_worker_profiles_from_path(path)?
        .into_iter()
        .map(|profile| profile.name)
        .collect();
    names.sort();
    Ok(WorkerProfileDiscovery::Profiles(names))
}

fn worker_profile_discovery_message(discovery: &WorkerProfileDiscovery) -> String {
    match discovery {
        WorkerProfileDiscovery::Missing => {
            "No workers.toml found; any profile name is allowed".to_string()
        }
        WorkerProfileDiscovery::Profiles(names) if names.is_empty() => {
            "No worker profiles configured".to_string()
        }
        WorkerProfileDiscovery::Profiles(names) => {
            format!("Available: {}", names.join(", "))
        }
    }
}

fn worker_profile_footer_message() -> String {
    match load_worker_profile_discovery() {
        Ok(discovery) => worker_profile_discovery_message(&discovery),
        Err(err) => format!("workers.toml unavailable: {err}"),
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct BoardSelection {
    column_index: usize,
    row_index: Option<usize>,
    task_id: Option<String>,
}

impl BoardSelection {
    fn selected_task_id(&self) -> Option<&str> {
        self.task_id.as_deref()
    }
}

fn select_first_available_task(board: &crate::queue::QueueBoard, selection: &mut BoardSelection) {
    if let Some((column_index, task)) = board
        .columns
        .iter()
        .enumerate()
        .find_map(|(index, column)| column.tasks.first().map(|task| (index, task)))
    {
        selection.column_index = column_index;
        selection.row_index = Some(0);
        selection.task_id = Some(task.id.clone());
    } else {
        selection.column_index = 0;
        selection.row_index = None;
        selection.task_id = None;
    }
}

fn select_task_by_id(
    board: &crate::queue::QueueBoard,
    selection: &mut BoardSelection,
    task_id: &str,
) -> bool {
    let Some((column_index, row_index)) = find_task_position(board, task_id) else {
        return false;
    };

    selection.column_index = column_index;
    selection.row_index = Some(row_index);
    selection.task_id = Some(task_id.to_string());
    true
}

fn preserve_selection_after_refresh(
    board: &crate::queue::QueueBoard,
    selection: &mut BoardSelection,
) {
    if board.columns.is_empty() {
        selection.column_index = 0;
        selection.row_index = None;
        selection.task_id = None;
        return;
    }

    let had_selected_task = selection.task_id.is_some();
    if let Some((column_index, row_index, task_id)) =
        selection.task_id.as_deref().and_then(|selected| {
            find_task_position(board, selected)
                .map(|(column_index, row_index)| (column_index, row_index, selected.to_string()))
        })
    {
        selection.column_index = column_index;
        selection.row_index = Some(row_index);
        selection.task_id = Some(task_id);
        return;
    }

    selection.column_index = selection.column_index.min(board.columns.len() - 1);
    if !had_selected_task {
        let preferred_row = selection.row_index.unwrap_or(0);
        if !select_row_in_column(board, selection, preferred_row) {
            selection.row_index = None;
            selection.task_id = None;
        }
        return;
    }

    if select_row_in_column(board, selection, selection.row_index.unwrap_or(0)) {
        return;
    }

    if board.columns.iter().any(|column| !column.tasks.is_empty()) {
        select_first_available_task(board, selection);
    } else {
        selection.row_index = None;
        selection.task_id = None;
    }
}

fn move_selection_within_column(
    board: &crate::queue::QueueBoard,
    selection: &mut BoardSelection,
    delta: isize,
) {
    preserve_selection_after_refresh(board, selection);
    let Some(column) = board.columns.get(selection.column_index) else {
        return;
    };
    if column.tasks.is_empty() {
        selection.row_index = None;
        selection.task_id = None;
        return;
    }

    let current_row = selection.row_index.unwrap_or(0);
    let max_row = column.tasks.len() - 1;
    let next_row = if delta.is_negative() {
        current_row.saturating_sub(delta.unsigned_abs())
    } else {
        current_row.saturating_add(delta as usize).min(max_row)
    };
    select_row_in_column(board, selection, next_row);
}

fn move_selection_to_column(
    board: &crate::queue::QueueBoard,
    selection: &mut BoardSelection,
    delta: isize,
) {
    preserve_selection_after_refresh(board, selection);
    if board.columns.is_empty() {
        return;
    }

    selection.column_index = if delta.is_negative() {
        selection.column_index.saturating_sub(delta.unsigned_abs())
    } else {
        selection
            .column_index
            .saturating_add(delta as usize)
            .min(board.columns.len() - 1)
    };

    select_row_in_column(board, selection, selection.row_index.unwrap_or(0));
}

fn select_row_in_column(
    board: &crate::queue::QueueBoard,
    selection: &mut BoardSelection,
    preferred_row: usize,
) -> bool {
    let Some(column) = board.columns.get(selection.column_index) else {
        selection.row_index = None;
        selection.task_id = None;
        return false;
    };
    let Some(max_row) = column.tasks.len().checked_sub(1) else {
        selection.row_index = None;
        selection.task_id = None;
        return false;
    };

    let row = preferred_row.min(max_row);
    selection.row_index = Some(row);
    selection.task_id = Some(column.tasks[row].id.clone());
    true
}

fn find_task_position(board: &crate::queue::QueueBoard, task_id: &str) -> Option<(usize, usize)> {
    board
        .columns
        .iter()
        .enumerate()
        .find_map(|(column_index, column)| {
            column
                .tasks
                .iter()
                .position(|task| task.id == task_id)
                .map(|row_index| (column_index, row_index))
        })
}

fn selected_task_status(
    board: &crate::queue::QueueBoard,
    selected_task_id: &str,
) -> Option<crate::queue::TaskStatus> {
    board.columns.iter().find_map(|column| {
        column
            .tasks
            .iter()
            .any(|task| task.id == selected_task_id)
            .then_some(column.status)
    })
}

fn selected_task_is_actionable(board: &crate::queue::QueueBoard, task_id: &str) -> bool {
    selected_task_status(board, task_id).is_some_and(task_status_is_actionable)
}

fn task_status_is_actionable(status: crate::queue::TaskStatus) -> bool {
    matches!(
        status,
        crate::queue::TaskStatus::Backlog | crate::queue::TaskStatus::Ready
    )
}

fn state_task_status(
    state: &crate::queue::QueueState,
    task_id: &str,
) -> Option<crate::queue::TaskStatus> {
    state
        .tasks
        .iter()
        .find(|task| task.id == task_id)
        .map(|task| task.status)
}

fn filtered_active_runs(
    index: &crate::queue::RunIndex,
    worker_profile: Option<&str>,
) -> Vec<crate::queue::RunState> {
    index
        .active_runs()
        .into_iter()
        .filter(|run| worker_profile.is_none_or(|profile| run.worker_profile == profile))
        .cloned()
        .collect()
}

fn refresh_status_text(output: &super::QueueRefreshRunsOutput) -> String {
    if output.checked == 0 {
        return "refreshed".to_string();
    }
    format!(
        "refresh-runs: {} succeeded, {} failed, {} still running",
        output.succeeded, output.failed, output.still_running
    )
}

fn footer_text(
    status_message: Option<&str>,
    input_mode: Option<&AddTaskPrompt>,
    auto_refresh_enabled: bool,
) -> String {
    if let Some(input) = input_mode {
        let prompt = match input.step {
            AddTaskPromptStep::Title => format!("Title: {}", input.title),
            AddTaskPromptStep::WorkerProfile => {
                format!(
                    "Worker profile (optional): {} | {}",
                    input.worker_profile,
                    worker_profile_footer_message()
                )
            }
        };
        let help = "Enter submit  Esc cancel";
        return match status_message {
            Some(message) => format!("{message} | {prompt} | {help}"),
            None => format!("{prompt} | {help}"),
        };
    }

    let auto = if auto_refresh_enabled {
        "auto-refresh on"
    } else {
        "auto-refresh off"
    };
    let help = format!(
        "Up/Down or j/k move within column  Left/Right or h/l move columns  x run  n new  a approve  r refresh  t auto  q quit | {auto}"
    );
    match status_message {
        Some(message) => format!("{message} | {help}"),
        None => help,
    }
}

fn render_column(
    column: &crate::queue::QueueBoardColumn,
    selection: &BoardSelection,
    is_selected_column: bool,
) -> Paragraph<'static> {
    let mut lines = Vec::new();
    if column.tasks.is_empty() {
        lines.push(if is_selected_column {
            "> none".to_string()
        } else {
            "  none".to_string()
        });
    } else {
        for task in &column.tasks {
            let selection_marker = if selection.selected_task_id() == Some(task.id.as_str()) {
                ">"
            } else {
                " "
            };
            lines.push(format!(
                "{} {}  {}",
                selection_marker,
                short_task_id(&task.id),
                truncate(&task.title, 32)
            ));
            match task
                .worker_profile
                .as_deref()
                .filter(|value| !value.is_empty())
            {
                Some(worker) => lines.push(format!(
                    "  priority: {}  worker: {}",
                    priority_label(task.priority),
                    worker
                )),
                None => lines.push(format!("  priority: {}", priority_label(task.priority))),
            }
            lines.push(String::new());
        }
    }

    Paragraph::new(lines.join("\n"))
        .block(Block::default().borders(Borders::ALL).title(format!(
            "{} ({})",
            column.label.to_lowercase(),
            column.tasks.len()
        )))
        .wrap(Wrap { trim: true })
}

fn header_text(board: &crate::queue::QueueBoard) -> String {
    match board.worker_profile.as_deref() {
        Some(worker_profile) => format!("total: {}  worker_profile: {worker_profile}", board.total),
        None => format!("total: {}", board.total),
    }
}

fn active_runs_height(active_runs: &[crate::queue::RunState]) -> u16 {
    if active_runs.is_empty() {
        0
    } else {
        5
    }
}

fn active_runs_text(active_runs: &[crate::queue::RunState]) -> String {
    active_runs
        .iter()
        .take(3)
        .map(|run| {
            format!(
                "{}  task:{}  worker:{}  status:{}",
                run.run_id,
                short_task_id(&run.task_id),
                run.worker_profile,
                run_status_label(run.status)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn details_panel(text: String) -> Paragraph<'static> {
    Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title("Details"))
        .wrap(Wrap { trim: true })
}

fn details_panel_height(terminal_height: u16) -> u16 {
    terminal_height.saturating_sub(9).clamp(8, 16)
}

fn selected_task_details_text(
    selected_task_id: Option<&str>,
    state: &crate::queue::QueueState,
    index: &crate::queue::RunIndex,
) -> String {
    let Some(task_id) = selected_task_id else {
        return "no task selected".to_string();
    };
    let Some(task) = state.tasks.iter().find(|task| task.id == task_id) else {
        return "no task selected".to_string();
    };

    let mut lines = vec![
        format!("task id: {}", task.id),
        format!("title: {}", task.title),
        format!("status: {}", task_status_label(task.status)),
        format!("priority: {}", priority_label(task.priority)),
    ];
    if let Some(worker_profile) = task
        .worker_profile
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        lines.push(format!("worker profile: {worker_profile}"));
    }
    if let Some(project) = task.project.as_deref().filter(|value| !value.is_empty()) {
        lines.push(format!("project: {project}"));
    }
    if let Some(output_path) = task
        .output_path
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        lines.push(format!("output path: {output_path}"));
    }
    lines.push(format!("created_at: {}", task.created_at.to_rfc3339()));
    lines.push(format!("updated_at: {}", task.updated_at.to_rfc3339()));
    lines.push(String::new());

    match latest_run_for_task(index, task_id) {
        Some(run) => append_latest_run_details(&mut lines, run),
        None => lines.push("no runs for selected task".to_string()),
    }

    lines.join("\n")
}

fn latest_run_for_task<'a>(
    index: &'a crate::queue::RunIndex,
    task_id: &str,
) -> Option<&'a crate::queue::RunState> {
    index
        .runs
        .iter()
        .filter(|run| run.task_id == task_id)
        .max_by_key(|run| run.started_at)
}

fn append_latest_run_details(lines: &mut Vec<String>, run: &crate::queue::RunState) {
    lines.push("latest run:".to_string());
    lines.push(format!("run id: {}", run.run_id));
    lines.push(format!("worker profile: {}", run.worker_profile));
    lines.push(format!("status: {}", run_status_label(run.status)));
    if let Some(pid) = run.pid {
        lines.push(format!("pid: {pid}"));
    }
    if let Some(exit_code) = run.exit_code {
        lines.push(format!("exit code: {exit_code}"));
    }
    lines.push(format!("started_at: {}", run.started_at.to_rfc3339()));
    if let Some(ended_at) = run.ended_at {
        lines.push(format!("ended_at: {}", ended_at.to_rfc3339()));
    }
    lines.push(format!("run directory: {}", run.run_dir));

    if !run.stdout_path.trim().is_empty() {
        lines.push(String::new());
        lines.push(log_preview_section("stdout", Path::new(&run.stdout_path)));
    }
    if !run.stderr_path.trim().is_empty() {
        lines.push(String::new());
        lines.push(log_preview_section("stderr", Path::new(&run.stderr_path)));
    }
}

fn log_preview_section(label: &str, path: &Path) -> String {
    let mut lines = vec![format!("{label}:")];
    if !path.exists() {
        lines.push(format!("missing log file: {}", path.display()));
        return lines.join("\n");
    }

    match read_log_preview_lossy(path) {
        Ok(preview) if preview.is_empty() => lines.push("(empty)".to_string()),
        Ok(preview) => lines.push(preview),
        Err(err) => lines.push(format!("failed to read {}: {err}", path.display())),
    }
    lines.join("\n")
}

fn read_log_preview_lossy(path: &Path) -> std::io::Result<String> {
    const MAX_PREVIEW_LINES: usize = 5;
    let bytes = std::fs::read(path)?;
    let content = String::from_utf8_lossy(&bytes);
    let lines = content.lines().collect::<Vec<_>>();
    let start = lines.len().saturating_sub(MAX_PREVIEW_LINES);
    Ok(lines[start..].join("\n"))
}

fn short_task_id(id: &str) -> String {
    id.chars().take(12).collect()
}

fn truncate(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let truncated = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        format!("{truncated}...")
    } else {
        truncated
    }
}

fn priority_label(priority: crate::queue::TaskPriority) -> &'static str {
    match priority {
        crate::queue::TaskPriority::Low => "low",
        crate::queue::TaskPriority::Normal => "normal",
        crate::queue::TaskPriority::High => "high",
        crate::queue::TaskPriority::Urgent => "urgent",
    }
}

fn task_status_label(status: crate::queue::TaskStatus) -> &'static str {
    match status {
        crate::queue::TaskStatus::Backlog => "backlog",
        crate::queue::TaskStatus::Ready => "ready",
        crate::queue::TaskStatus::Running => "running",
        crate::queue::TaskStatus::Review => "review",
        crate::queue::TaskStatus::Done => "done",
        crate::queue::TaskStatus::Blocked => "blocked",
        crate::queue::TaskStatus::Cancelled => "cancelled",
    }
}

fn run_status_label(status: crate::queue::RunStatus) -> &'static str {
    match status {
        crate::queue::RunStatus::Running => "running",
        crate::queue::RunStatus::Succeeded => "succeeded",
        crate::queue::RunStatus::Failed => "failed",
        crate::queue::RunStatus::Cancelled => "cancelled",
        crate::queue::RunStatus::Unknown => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_task_id_limits_visible_identifier() {
        assert_eq!(short_task_id("task_1234567890abcdef"), "task_1234567");
    }

    #[test]
    fn header_mentions_worker_filter_when_present() {
        let board = crate::queue::QueueBoard {
            worker_profile: Some("coder".to_string()),
            total: 2,
            columns: Vec::new(),
        };

        assert_eq!(header_text(&board), "total: 2  worker_profile: coder");
    }

    #[test]
    fn footer_mentions_refresh_and_quit_keys() {
        let help =
            "Up/Down or j/k move within column  Left/Right or h/l move columns  x run  n new  a approve  r refresh  t auto  q quit | auto-refresh on";
        assert_eq!(footer_text(None, None, true), help);
        assert_eq!(
            footer_text(Some("refreshed"), None, true),
            format!("refreshed | {help}")
        );
    }

    #[test]
    fn footer_shows_auto_refresh_off() {
        let help =
            "Up/Down or j/k move within column  Left/Right or h/l move columns  x run  n new  a approve  r refresh  t auto  q quit | auto-refresh off";
        assert_eq!(footer_text(None, None, false), help);
    }

    #[test]
    fn auto_refresh_ticks_every_two_seconds() {
        let start = Instant::now();
        let mut auto_refresh = AutoRefreshState::new(start);

        assert!(!auto_refresh.mark_refresh_if_due(start + Duration::from_millis(1999)));
        assert_eq!(
            auto_refresh.poll_timeout(start + Duration::from_millis(1999)),
            Duration::from_millis(1)
        );
        assert!(auto_refresh.mark_refresh_if_due(start + Duration::from_secs(2)));
        assert!(!auto_refresh
            .mark_refresh_if_due(start + Duration::from_secs(2) + Duration::from_millis(1)));
    }

    #[test]
    fn auto_refresh_toggle_disables_and_reenables_ticks() {
        let start = Instant::now();
        let mut auto_refresh = AutoRefreshState::new(start);

        auto_refresh.toggle(start + Duration::from_millis(500));
        assert!(!auto_refresh.enabled);
        assert!(!auto_refresh.mark_refresh_if_due(start + Duration::from_secs(5)));

        auto_refresh.toggle(start + Duration::from_secs(6));
        assert!(auto_refresh.enabled);
        assert!(!auto_refresh
            .mark_refresh_if_due(start + Duration::from_secs(6) + Duration::from_millis(1999)));
        assert!(auto_refresh.mark_refresh_if_due(start + Duration::from_secs(8)));
    }

    #[test]
    fn footer_mentions_submit_and_cancel_in_input_mode() {
        let _lock = crate::storage::lock_test_env();
        let project = tempfile::tempdir().expect("project tempdir");
        let _cwd = CurrentDirGuard::change_to(project.path());
        std::fs::create_dir_all(project.path().join(".jcode")).expect("create .jcode");
        std::fs::write(
            project.path().join(".jcode").join("workers.toml"),
            "[workers.coder]\ncommand = \"test-worker <handoff_file>\"\n",
        )
        .expect("write workers");
        let input = AddTaskPrompt {
            title: "Update docs".to_string(),
            worker_profile: "coder".to_string(),
            step: AddTaskPromptStep::WorkerProfile,
        };

        assert_eq!(
            footer_text(None, Some(&input), true),
            "Worker profile (optional): coder | Available: coder | Enter submit  Esc cancel"
        );
    }

    #[test]
    fn footer_shows_title_prompt_in_first_input_step() {
        let input = AddTaskPrompt {
            title: "Update docs".to_string(),
            worker_profile: String::new(),
            step: AddTaskPromptStep::Title,
        };

        assert_eq!(
            footer_text(None, Some(&input), true),
            "Title: Update docs | Enter submit  Esc cancel"
        );
    }

    #[test]
    fn footer_lists_available_worker_profiles_during_worker_step() {
        let _lock = crate::storage::lock_test_env();
        let project = tempfile::tempdir().expect("project tempdir");
        let _cwd = CurrentDirGuard::change_to(project.path());
        std::fs::create_dir_all(project.path().join(".jcode")).expect("create .jcode");
        std::fs::write(
            project.path().join(".jcode").join("workers.toml"),
            "[workers.planner]\ncommand = \"planner <handoff_file>\"\n[workers.coder]\ncommand = \"coder <handoff_file>\"\n",
        )
        .expect("write workers");
        let input = AddTaskPrompt {
            title: "Update docs".to_string(),
            worker_profile: String::new(),
            step: AddTaskPromptStep::WorkerProfile,
        };

        let footer = footer_text(None, Some(&input), true);

        assert!(footer.contains("Available: "));
        assert!(footer.contains("coder"));
        assert!(footer.contains("planner"));
    }

    #[test]
    fn footer_explains_missing_workers_toml_during_worker_step() {
        let _lock = crate::storage::lock_test_env();
        let project = tempfile::tempdir().expect("project tempdir");
        let _cwd = CurrentDirGuard::change_to(project.path());
        let input = AddTaskPrompt {
            title: "Update docs".to_string(),
            worker_profile: "anything".to_string(),
            step: AddTaskPromptStep::WorkerProfile,
        };

        assert_eq!(
            footer_text(None, Some(&input), true),
            "Worker profile (optional): anything | No workers.toml found; any profile name is allowed | Enter submit  Esc cancel"
        );
    }

    #[test]
    fn footer_explains_empty_workers_toml_during_worker_step() {
        let _lock = crate::storage::lock_test_env();
        let project = tempfile::tempdir().expect("project tempdir");
        let _cwd = CurrentDirGuard::change_to(project.path());
        std::fs::create_dir_all(project.path().join(".jcode")).expect("create .jcode");
        std::fs::write(project.path().join(".jcode").join("workers.toml"), "")
            .expect("write workers");
        let input = AddTaskPrompt {
            title: "Update docs".to_string(),
            worker_profile: String::new(),
            step: AddTaskPromptStep::WorkerProfile,
        };

        assert_eq!(
            footer_text(None, Some(&input), true),
            "Worker profile (optional):  | No worker profiles configured | Enter submit  Esc cancel"
        );
    }

    #[test]
    fn add_task_to_state_adds_backlog_task_with_defaults() {
        let mut state = crate::queue::QueueState::default();

        let task_id = add_task_to_state(&mut state, "  New board task  ", None).expect("add task");

        assert_eq!(state.tasks.len(), 1);
        assert_eq!(state.tasks[0].id, task_id);
        assert_eq!(state.tasks[0].title, "New board task");
        assert_eq!(state.tasks[0].description, "");
        assert_eq!(state.tasks[0].project, None);
        assert_eq!(state.tasks[0].status, crate::queue::TaskStatus::Backlog);
        assert_eq!(state.tasks[0].priority, crate::queue::TaskPriority::Normal);
        assert_eq!(state.tasks[0].worker_profile, None);
        assert_eq!(state.tasks[0].output_path, None);
    }

    #[test]
    fn add_task_to_state_rejects_empty_title() {
        let mut state = crate::queue::QueueState::default();

        let err = add_task_to_state(&mut state, "   ", None).expect_err("empty title rejected");

        assert!(err.to_string().contains("title required"));
        assert!(state.tasks.is_empty());
    }

    #[test]
    fn add_task_to_state_stores_worker_profile_when_present() {
        let mut state = crate::queue::QueueState::default();

        add_task_to_state(&mut state, "New board task", Some("  coder  ")).expect("add task");

        assert_eq!(state.tasks[0].worker_profile.as_deref(), Some("coder"));
    }

    #[test]
    fn add_task_to_state_stores_none_for_empty_worker_profile() {
        let mut state = crate::queue::QueueState::default();

        add_task_to_state(&mut state, "New board task", Some("   ")).expect("add task");

        assert_eq!(state.tasks[0].worker_profile, None);
    }

    #[test]
    fn add_task_from_prompt_saves_worker_profile_and_selects_new_task() {
        let _lock = crate::storage::lock_test_env();
        let _saved = EnvGuard::capture("JCODE_HOME");
        let home = tempfile::tempdir().expect("home tempdir");
        let project = tempfile::tempdir().expect("project tempdir");
        let _cwd = CurrentDirGuard::change_to(project.path());
        crate::env::set_var("JCODE_HOME", home.path());
        std::fs::create_dir_all(project.path().join(".jcode")).expect("create .jcode");
        std::fs::write(
            project.path().join(".jcode").join("workers.toml"),
            "[workers.coder]\ncommand = \"test-worker <handoff_file>\"\n",
        )
        .expect("write workers");

        let mut board =
            crate::queue::build_queue_board(&crate::queue::QueueState::default(), None, Some(100));
        let mut state = crate::queue::QueueState::default();
        let mut index = crate::queue::RunIndex::default();
        let mut active_runs = Vec::new();
        let mut selection = BoardSelection::default();
        let input = AddTaskPrompt {
            title: "New board task".to_string(),
            worker_profile: "coder".to_string(),
            step: AddTaskPromptStep::WorkerProfile,
        };

        let message = add_task_from_prompt(
            &mut board,
            &mut state,
            &mut index,
            &mut active_runs,
            &mut selection,
            &input,
            &test_options(),
        )
        .expect("add task");

        assert!(message.starts_with("added task_"));
        let reloaded = crate::queue::load().expect("reload queue");
        assert_eq!(reloaded.tasks.len(), 1);
        assert_eq!(reloaded.tasks[0].worker_profile.as_deref(), Some("coder"));
        assert_eq!(
            selection.selected_task_id(),
            Some(reloaded.tasks[0].id.as_str())
        );
    }

    #[test]
    fn add_task_from_prompt_rejects_unknown_worker_profile_when_config_exists() {
        let _lock = crate::storage::lock_test_env();
        let _saved = EnvGuard::capture("JCODE_HOME");
        let home = tempfile::tempdir().expect("home tempdir");
        let project = tempfile::tempdir().expect("project tempdir");
        let _cwd = CurrentDirGuard::change_to(project.path());
        crate::env::set_var("JCODE_HOME", home.path());
        std::fs::create_dir_all(project.path().join(".jcode")).expect("create .jcode");
        std::fs::write(
            project.path().join(".jcode").join("workers.toml"),
            "[workers.coder]\ncommand = \"test-worker <handoff_file>\"\n",
        )
        .expect("write workers");

        let mut board =
            crate::queue::build_queue_board(&crate::queue::QueueState::default(), None, Some(100));
        let mut state = crate::queue::QueueState::default();
        let mut index = crate::queue::RunIndex::default();
        let mut active_runs = Vec::new();
        let mut selection = BoardSelection::default();
        let input = AddTaskPrompt {
            title: "New board task".to_string(),
            worker_profile: "reviewer".to_string(),
            step: AddTaskPromptStep::WorkerProfile,
        };

        let err = add_task_from_prompt(
            &mut board,
            &mut state,
            &mut index,
            &mut active_runs,
            &mut selection,
            &input,
            &test_options(),
        )
        .expect_err("unknown worker rejected");

        assert_eq!(
            err.to_string(),
            "unknown worker profile: reviewer | Available: coder"
        );
        let reloaded = crate::queue::load().expect("reload queue");
        assert!(reloaded.tasks.is_empty());
    }

    #[test]
    fn add_task_from_prompt_allows_arbitrary_worker_profile_without_config() {
        let _lock = crate::storage::lock_test_env();
        let _saved = EnvGuard::capture("JCODE_HOME");
        let home = tempfile::tempdir().expect("home tempdir");
        let project = tempfile::tempdir().expect("project tempdir");
        let _cwd = CurrentDirGuard::change_to(project.path());
        crate::env::set_var("JCODE_HOME", home.path());

        let mut board =
            crate::queue::build_queue_board(&crate::queue::QueueState::default(), None, Some(100));
        let mut state = crate::queue::QueueState::default();
        let mut index = crate::queue::RunIndex::default();
        let mut active_runs = Vec::new();
        let mut selection = BoardSelection::default();
        let input = AddTaskPrompt {
            title: "New board task".to_string(),
            worker_profile: "ad-hoc".to_string(),
            step: AddTaskPromptStep::WorkerProfile,
        };

        add_task_from_prompt(
            &mut board,
            &mut state,
            &mut index,
            &mut active_runs,
            &mut selection,
            &input,
            &test_options(),
        )
        .expect("add task");

        let reloaded = crate::queue::load().expect("reload queue");
        assert_eq!(reloaded.tasks[0].worker_profile.as_deref(), Some("ad-hoc"));
    }

    #[test]
    fn add_task_from_prompt_stores_none_for_empty_worker_profile() {
        let _lock = crate::storage::lock_test_env();
        let _saved = EnvGuard::capture("JCODE_HOME");
        let home = tempfile::tempdir().expect("home tempdir");
        let project = tempfile::tempdir().expect("project tempdir");
        let _cwd = CurrentDirGuard::change_to(project.path());
        crate::env::set_var("JCODE_HOME", home.path());
        std::fs::create_dir_all(project.path().join(".jcode")).expect("create .jcode");
        std::fs::write(
            project.path().join(".jcode").join("workers.toml"),
            "[workers.coder]\ncommand = \"test-worker <handoff_file>\"\n",
        )
        .expect("write workers");

        let mut board =
            crate::queue::build_queue_board(&crate::queue::QueueState::default(), None, Some(100));
        let mut state = crate::queue::QueueState::default();
        let mut index = crate::queue::RunIndex::default();
        let mut active_runs = Vec::new();
        let mut selection = BoardSelection::default();
        let input = AddTaskPrompt {
            title: "New board task".to_string(),
            worker_profile: "   ".to_string(),
            step: AddTaskPromptStep::WorkerProfile,
        };

        add_task_from_prompt(
            &mut board,
            &mut state,
            &mut index,
            &mut active_runs,
            &mut selection,
            &input,
            &test_options(),
        )
        .expect("add task");

        let reloaded = crate::queue::load().expect("reload queue");
        assert_eq!(reloaded.tasks[0].worker_profile, None);
    }

    #[test]
    fn select_task_by_id_selects_newly_visible_task() {
        let board = test_board(&["backlog_1", "ready_1"]);
        let mut selection = BoardSelection::default();

        assert!(select_task_by_id(&board, &mut selection, "ready_1"));

        assert_selection(&selection, 1, Some(0), Some("ready_1"));
    }

    #[test]
    fn refresh_status_text_is_short() {
        let output = super::super::QueueRefreshRunsOutput {
            checked: 4,
            succeeded: 1,
            failed: 2,
            still_running: 1,
            malformed: 0,
            run_index_changed: true,
            queue_changed: true,
            warnings: Vec::new(),
        };

        assert_eq!(
            refresh_status_text(&output),
            "refresh-runs: 1 succeeded, 2 failed, 1 still running"
        );
    }

    #[test]
    fn refresh_status_text_is_simple_when_no_runs_checked() {
        let output = super::super::QueueRefreshRunsOutput {
            checked: 0,
            succeeded: 0,
            failed: 0,
            still_running: 0,
            malformed: 0,
            run_index_changed: false,
            queue_changed: false,
            warnings: Vec::new(),
        };

        assert_eq!(refresh_status_text(&output), "refreshed");
    }

    #[test]
    fn selection_defaults_to_first_visible_task() {
        let board = test_board(&["backlog_1", "ready_1"]);
        let mut selection = BoardSelection::default();

        select_first_available_task(&board, &mut selection);

        assert_selection(&selection, 0, Some(0), Some("backlog_1"));
    }

    #[test]
    fn refresh_preserves_selected_task_by_id() {
        let board = test_board(&["backlog_1", "ready_1"]);
        let mut selection = BoardSelection {
            column_index: 0,
            row_index: Some(0),
            task_id: Some("ready_1".to_string()),
        };

        preserve_selection_after_refresh(&board, &mut selection);

        assert_selection(&selection, 1, Some(0), Some("ready_1"));
    }

    #[test]
    fn refresh_preserves_column_and_row_when_selected_task_disappears() {
        let board = test_board(&["ready_1", "ready_2"]);
        let mut selection = BoardSelection {
            column_index: 1,
            row_index: Some(1),
            task_id: Some("missing".to_string()),
        };

        preserve_selection_after_refresh(&board, &mut selection);

        assert_selection(&selection, 1, Some(1), Some("ready_2"));
    }

    #[test]
    fn selection_clears_when_board_is_empty() {
        let board = test_board(&[]);
        let mut selection = BoardSelection {
            column_index: 1,
            row_index: Some(2),
            task_id: Some("missing".to_string()),
        };

        preserve_selection_after_refresh(&board, &mut selection);

        assert_selection(&selection, 1, None, None);
    }

    #[test]
    fn down_does_not_leave_current_column() {
        let board = test_board(&["backlog_1", "ready_1"]);
        let mut selection = BoardSelection {
            column_index: 0,
            row_index: Some(0),
            task_id: Some("backlog_1".to_string()),
        };

        move_selection_within_column(&board, &mut selection, 1);

        assert_selection(&selection, 0, Some(0), Some("backlog_1"));
    }

    #[test]
    fn up_does_not_leave_current_column() {
        let board = test_board(&["backlog_1", "ready_1"]);
        let mut selection = BoardSelection {
            column_index: 0,
            row_index: Some(0),
            task_id: Some("backlog_1".to_string()),
        };

        move_selection_within_column(&board, &mut selection, -1);

        assert_selection(&selection, 0, Some(0), Some("backlog_1"));
    }

    #[test]
    fn right_moves_to_next_column() {
        let board = test_board(&["backlog_1", "ready_1"]);
        let mut selection = BoardSelection {
            column_index: 0,
            row_index: Some(0),
            task_id: Some("backlog_1".to_string()),
        };

        move_selection_to_column(&board, &mut selection, 1);

        assert_selection(&selection, 1, Some(0), Some("ready_1"));
    }

    #[test]
    fn left_moves_to_previous_column() {
        let board = test_board(&["backlog_1", "ready_1"]);
        let mut selection = BoardSelection {
            column_index: 1,
            row_index: Some(0),
            task_id: Some("ready_1".to_string()),
        };

        move_selection_to_column(&board, &mut selection, -1);

        assert_selection(&selection, 0, Some(0), Some("backlog_1"));
    }

    #[test]
    fn right_preserves_row_index_when_possible() {
        let board = test_board(&["backlog_1", "backlog_2", "ready_1", "ready_2"]);
        let mut selection = BoardSelection {
            column_index: 0,
            row_index: Some(1),
            task_id: Some("backlog_2".to_string()),
        };

        move_selection_to_column(&board, &mut selection, 1);

        assert_selection(&selection, 1, Some(1), Some("ready_2"));
    }

    #[test]
    fn right_clamps_row_index_when_destination_column_has_fewer_tasks() {
        let board = test_board(&["backlog_1", "backlog_2", "ready_1"]);
        let mut selection = BoardSelection {
            column_index: 0,
            row_index: Some(1),
            task_id: Some("backlog_2".to_string()),
        };

        move_selection_to_column(&board, &mut selection, 1);

        assert_selection(&selection, 1, Some(0), Some("ready_1"));
    }

    #[test]
    fn right_allows_selecting_empty_column_without_task() {
        let board = test_board(&["backlog_1"]);
        let mut selection = BoardSelection {
            column_index: 0,
            row_index: Some(0),
            task_id: Some("backlog_1".to_string()),
        };

        move_selection_to_column(&board, &mut selection, 1);

        assert_selection(&selection, 1, None, None);
    }

    #[test]
    fn approve_selected_review_task_moves_it_to_done() {
        let original_time = test_time("2026-06-20T10:00:00Z");
        let updated_time = test_time("2026-06-20T12:00:00Z");
        let board = test_board_with_statuses(&[("task_1", crate::queue::TaskStatus::Review)]);
        let mut state = crate::queue::QueueState {
            tasks: vec![test_state_task(
                "task_1",
                crate::queue::TaskStatus::Review,
                original_time,
            )],
        };

        let message =
            approve_selected_review_task_in_state(&board, &mut state, Some("task_1"), updated_time)
                .expect("approve selected task");

        assert_eq!(message, "approved task_1");
        assert_eq!(state.tasks[0].status, crate::queue::TaskStatus::Done);
        assert_eq!(state.tasks[0].updated_at, updated_time);
    }

    #[test]
    fn approve_selected_non_review_task_is_rejected() {
        let original_time = test_time("2026-06-20T10:00:00Z");
        let updated_time = test_time("2026-06-20T12:00:00Z");
        let board = test_board_with_statuses(&[("task_1", crate::queue::TaskStatus::Ready)]);
        let mut state = crate::queue::QueueState {
            tasks: vec![test_state_task(
                "task_1",
                crate::queue::TaskStatus::Ready,
                original_time,
            )],
        };

        let message =
            approve_selected_review_task_in_state(&board, &mut state, Some("task_1"), updated_time)
                .expect("reject selected task");

        assert_eq!(message, "selected task is not in review");
        assert_eq!(state.tasks[0].status, crate::queue::TaskStatus::Ready);
        assert_eq!(state.tasks[0].updated_at, original_time);
    }

    #[test]
    fn approve_without_selected_task_is_rejected() {
        let original_time = test_time("2026-06-20T10:00:00Z");
        let updated_time = test_time("2026-06-20T12:00:00Z");
        let board = test_board_with_statuses(&[("task_1", crate::queue::TaskStatus::Review)]);
        let mut state = crate::queue::QueueState {
            tasks: vec![test_state_task(
                "task_1",
                crate::queue::TaskStatus::Review,
                original_time,
            )],
        };

        let message = approve_selected_review_task_in_state(&board, &mut state, None, updated_time)
            .expect("reject missing selection");

        assert_eq!(message, "no task selected");
        assert_eq!(state.tasks[0].status, crate::queue::TaskStatus::Review);
        assert_eq!(state.tasks[0].updated_at, original_time);
    }

    #[test]
    fn run_selected_without_selected_task_is_rejected() {
        let mut board = test_board_with_statuses(&[("task_1", crate::queue::TaskStatus::Ready)]);
        let mut state = crate::queue::QueueState::default();
        let mut index = crate::queue::RunIndex::default();
        let mut active_runs = Vec::new();
        let options = test_options();

        let message = run_selected_task_in_background_with_starter(
            &mut board,
            &mut state,
            &mut index,
            &mut active_runs,
            None,
            &options,
            |_command, _run_dir, _stdout_path, _stderr_path| unreachable!("no selection"),
        )
        .expect("reject missing selection");

        assert_eq!(message, "no task selected");
    }

    #[test]
    fn run_selected_non_actionable_task_is_rejected() {
        let mut board = test_board_with_statuses(&[("task_1", crate::queue::TaskStatus::Review)]);
        let mut state = crate::queue::QueueState::default();
        let mut index = crate::queue::RunIndex::default();
        let mut active_runs = Vec::new();
        let options = test_options();

        let message = run_selected_task_in_background_with_starter(
            &mut board,
            &mut state,
            &mut index,
            &mut active_runs,
            Some("task_1"),
            &options,
            |_command, _run_dir, _stdout_path, _stderr_path| unreachable!("non-actionable"),
        )
        .expect("reject non-actionable");

        assert_eq!(message, "selected task is not actionable");
    }

    #[test]
    fn run_selected_missing_worker_profile_is_rejected() {
        let _lock = crate::storage::lock_test_env();
        let _saved = EnvGuard::capture("JCODE_HOME");
        let home = tempfile::tempdir().expect("home tempdir");
        let project = tempfile::tempdir().expect("project tempdir");
        let _cwd = CurrentDirGuard::change_to(project.path());
        crate::env::set_var("JCODE_HOME", home.path());

        let timestamp = test_time("2026-06-20T10:00:00Z");
        let state = crate::queue::QueueState {
            tasks: vec![test_state_task_with_worker(
                "task_1",
                crate::queue::TaskStatus::Ready,
                timestamp,
                None,
            )],
        };
        crate::queue::save(&state).expect("save queue");
        let mut board = crate::queue::build_queue_board(&state, None, Some(100));
        let mut state = state;
        let mut index = crate::queue::RunIndex::default();
        let mut active_runs = Vec::new();
        let options = test_options();

        let message = run_selected_task_in_background_with_starter(
            &mut board,
            &mut state,
            &mut index,
            &mut active_runs,
            Some("task_1"),
            &options,
            |_command, _run_dir, _stdout_path, _stderr_path| unreachable!("missing worker"),
        )
        .expect("reject missing worker profile");

        assert_eq!(message, "selected task has no worker profile");
        let reloaded = crate::queue::load().expect("reload queue");
        assert_eq!(reloaded.tasks[0].status, crate::queue::TaskStatus::Ready);
    }

    #[test]
    fn run_selected_starts_actionable_task_with_worker_profile() {
        let _lock = crate::storage::lock_test_env();
        let _saved = EnvGuard::capture("JCODE_HOME");
        let home = tempfile::tempdir().expect("home tempdir");
        let project = tempfile::tempdir().expect("project tempdir");
        let _cwd = CurrentDirGuard::change_to(project.path());
        crate::env::set_var("JCODE_HOME", home.path());
        std::fs::create_dir_all(project.path().join(".jcode")).expect("create .jcode");
        std::fs::write(
            project.path().join(".jcode").join("workers.toml"),
            "[workers.coder]\ncommand = \"test-worker <handoff_file> --task <task_id>\"\n",
        )
        .expect("write workers");

        let timestamp = test_time("2026-06-20T10:00:00Z");
        let state = crate::queue::QueueState {
            tasks: vec![test_state_task_with_worker(
                "task_1",
                crate::queue::TaskStatus::Ready,
                timestamp,
                Some("coder"),
            )],
        };
        crate::queue::save(&state).expect("save queue");
        let mut board = crate::queue::build_queue_board(&state, None, Some(100));
        let mut state = state;
        let mut index = crate::queue::RunIndex::default();
        let mut active_runs = Vec::new();
        let options = test_options();
        let mut saw_command = false;

        let message = run_selected_task_in_background_with_starter(
            &mut board,
            &mut state,
            &mut index,
            &mut active_runs,
            Some("task_1"),
            &options,
            |command, run_dir, stdout_path, stderr_path| {
                saw_command = true;
                assert!(command.contains("--task task_1"));
                assert!(command.contains("task_1.md"));
                assert!(run_dir
                    .parent()
                    .is_some_and(|path| path.ends_with("task_1")));
                assert_eq!(stdout_path, run_dir.join("stdout.txt"));
                assert_eq!(stderr_path, run_dir.join("stderr.txt"));
                Ok(4242)
            },
        )
        .expect("start selected task");

        assert!(saw_command);
        assert!(message.starts_with("started task_1 as run_"));
        let reloaded = crate::queue::load().expect("reload queue");
        assert_eq!(reloaded.tasks[0].status, crate::queue::TaskStatus::Running);

        let index = crate::queue::load_run_index().expect("load run index");
        assert_eq!(index.runs.len(), 1);
        let run = &index.runs[0];
        assert_eq!(run.task_id, "task_1");
        assert_eq!(run.worker_profile, "coder");
        assert_eq!(run.status, crate::queue::RunStatus::Running);
        assert_eq!(run.pid, Some(4242));
        assert_eq!(active_runs.len(), 1);
        assert_eq!(active_runs[0].run_id, run.run_id);
    }

    #[test]
    fn selection_preserves_approved_task_when_still_visible_after_reload() {
        let reloaded_board =
            test_board_with_statuses(&[("task_1", crate::queue::TaskStatus::Done)]);
        let mut selection = BoardSelection {
            column_index: 3,
            row_index: Some(0),
            task_id: Some("task_1".to_string()),
        };

        preserve_selection_after_refresh(&reloaded_board, &mut selection);

        assert_selection(&selection, 5, Some(0), Some("task_1"));
    }

    #[test]
    fn selection_falls_back_when_approved_task_disappears() {
        let reloaded_board =
            test_board_with_statuses(&[("task_2", crate::queue::TaskStatus::Ready)]);
        let mut selection = BoardSelection {
            column_index: 3,
            row_index: Some(0),
            task_id: Some("task_1".to_string()),
        };

        preserve_selection_after_refresh(&reloaded_board, &mut selection);

        assert_selection(&selection, 1, Some(0), Some("task_2"));
    }

    #[test]
    fn selection_clears_when_no_tasks_remain_after_approve() {
        let reloaded_board = test_board_with_statuses(&[]);
        let mut selection = BoardSelection {
            column_index: 3,
            row_index: Some(0),
            task_id: Some("task_1".to_string()),
        };

        preserve_selection_after_refresh(&reloaded_board, &mut selection);

        assert_selection(&selection, 3, None, None);
    }

    #[test]
    fn selected_task_details_include_task_metadata() {
        let created_at = test_time("2026-06-20T10:00:00Z");
        let updated_at = test_time("2026-06-20T11:00:00Z");
        let task = crate::queue::Task {
            id: "task_1".to_string(),
            title: "Ship queue details".to_string(),
            description: String::new(),
            project: Some("jcode".to_string()),
            status: crate::queue::TaskStatus::Ready,
            priority: crate::queue::TaskPriority::High,
            worker_profile: Some("coder".to_string()),
            output_path: Some("docs/output.md".to_string()),
            created_at,
            updated_at,
        };
        let state = crate::queue::QueueState { tasks: vec![task] };
        let index = crate::queue::RunIndex::default();

        let details = selected_task_details_text(Some("task_1"), &state, &index);

        assert!(details.contains("task id: task_1"));
        assert!(details.contains("title: Ship queue details"));
        assert!(details.contains("status: ready"));
        assert!(details.contains("priority: high"));
        assert!(details.contains("worker profile: coder"));
        assert!(details.contains("project: jcode"));
        assert!(details.contains("output path: docs/output.md"));
        assert!(details.contains("created_at: 2026-06-20T10:00:00+00:00"));
        assert!(details.contains("updated_at: 2026-06-20T11:00:00+00:00"));
        assert!(details.contains("no runs for selected task"));
    }

    #[test]
    fn selected_task_details_uses_latest_run_for_task() {
        let state = crate::queue::QueueState {
            tasks: vec![test_state_task(
                "task_1",
                crate::queue::TaskStatus::Running,
                test_time("2026-06-20T10:00:00Z"),
            )],
        };
        let older = test_run_state(
            "run_old",
            "task_1",
            test_time("2026-06-20T11:00:00Z"),
            crate::queue::RunStatus::Failed,
        );
        let newer = test_run_state(
            "run_new",
            "task_1",
            test_time("2026-06-20T12:00:00Z"),
            crate::queue::RunStatus::Running,
        );
        let other_task = test_run_state(
            "run_other",
            "task_2",
            test_time("2026-06-20T13:00:00Z"),
            crate::queue::RunStatus::Running,
        );
        let index = crate::queue::RunIndex {
            runs: vec![newer, other_task, older],
        };

        let details = selected_task_details_text(Some("task_1"), &state, &index);

        assert!(details.contains("run id: run_new"));
        assert!(!details.contains("run id: run_old"));
        assert!(!details.contains("run id: run_other"));
    }

    #[test]
    fn log_preview_reports_missing_log_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let missing = dir.path().join("missing.log");

        let preview = log_preview_section("stdout", &missing);

        assert_eq!(
            preview,
            format!("stdout:\nmissing log file: {}", missing.display())
        );
    }

    #[test]
    fn log_preview_decodes_invalid_utf8_lossily_and_keeps_last_five_lines() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("stdout.log");
        std::fs::write(&path, b"one\ntwo\nthree\nfour\nfive\nsix\ninvalid:\xFF\n")
            .expect("write log");

        let preview = log_preview_section("stdout", &path);

        assert!(preview.starts_with("stdout:\nthree\nfour\nfive\nsix\ninvalid:"));
        assert!(preview.contains(char::REPLACEMENT_CHARACTER));
        assert!(!preview.contains("one"));
        assert!(!preview.contains("two"));
    }

    fn assert_selection(
        selection: &BoardSelection,
        column_index: usize,
        row_index: Option<usize>,
        task_id: Option<&str>,
    ) {
        assert_eq!(selection.column_index, column_index);
        assert_eq!(selection.row_index, row_index);
        assert_eq!(selection.selected_task_id(), task_id);
    }

    fn test_board(task_ids: &[&str]) -> crate::queue::QueueBoard {
        let tasks = task_ids
            .iter()
            .map(|task_id| {
                let status = if task_id.starts_with("ready") {
                    crate::queue::TaskStatus::Ready
                } else {
                    crate::queue::TaskStatus::Backlog
                };
                (*task_id, status)
            })
            .collect::<Vec<_>>();
        test_board_with_statuses(&tasks)
    }

    fn test_board_with_statuses(
        task_ids: &[(&str, crate::queue::TaskStatus)],
    ) -> crate::queue::QueueBoard {
        let created_at = chrono::DateTime::parse_from_rfc3339("2026-06-20T10:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let columns = [
            crate::queue::TaskStatus::Backlog,
            crate::queue::TaskStatus::Ready,
            crate::queue::TaskStatus::Running,
            crate::queue::TaskStatus::Review,
            crate::queue::TaskStatus::Blocked,
            crate::queue::TaskStatus::Done,
            crate::queue::TaskStatus::Cancelled,
        ]
        .into_iter()
        .map(|status| crate::queue::QueueBoardColumn {
            status,
            label: format!("{status:?}"),
            tasks: Vec::new(),
        })
        .collect::<Vec<_>>();
        let mut board = crate::queue::QueueBoard {
            worker_profile: None,
            total: task_ids.len(),
            columns,
        };

        for (task_id, status) in task_ids {
            let column_index = board
                .columns
                .iter()
                .position(|column| column.status == *status)
                .unwrap();
            board.columns[column_index]
                .tasks
                .push(crate::queue::QueueBoardTask {
                    id: (*task_id).to_string(),
                    title: (*task_id).to_string(),
                    priority: crate::queue::TaskPriority::Normal,
                    worker_profile: None,
                    created_at,
                    updated_at: created_at,
                });
        }

        board
    }

    fn test_state_task(
        id: &str,
        status: crate::queue::TaskStatus,
        timestamp: chrono::DateTime<chrono::Utc>,
    ) -> crate::queue::Task {
        test_state_task_with_worker(id, status, timestamp, None)
    }

    fn test_state_task_with_worker(
        id: &str,
        status: crate::queue::TaskStatus,
        timestamp: chrono::DateTime<chrono::Utc>,
        worker_profile: Option<&str>,
    ) -> crate::queue::Task {
        crate::queue::Task {
            id: id.to_string(),
            title: id.to_string(),
            description: String::new(),
            project: None,
            status,
            priority: crate::queue::TaskPriority::Normal,
            worker_profile: worker_profile.map(str::to_string),
            output_path: None,
            created_at: timestamp,
            updated_at: timestamp,
        }
    }

    fn test_options() -> QueueBoardTuiOptions {
        QueueBoardTuiOptions {
            worker_profile: None,
            limit: 100,
        }
    }

    fn test_time(value: &str) -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339(value)
            .unwrap()
            .with_timezone(&chrono::Utc)
    }

    fn test_run_state(
        run_id: &str,
        task_id: &str,
        started_at: chrono::DateTime<chrono::Utc>,
        status: crate::queue::RunStatus,
    ) -> crate::queue::RunState {
        crate::queue::RunState {
            run_id: run_id.to_string(),
            task_id: task_id.to_string(),
            worker_profile: "coder".to_string(),
            command: "test-worker".to_string(),
            pid: Some(4242),
            status,
            started_at,
            ended_at: None,
            exit_code: None,
            run_dir: ".jcode/queue/runs/task_1/20260620T120000Z".to_string(),
            stdout_path: ".jcode/queue/runs/task_1/20260620T120000Z/stdout.txt".to_string(),
            stderr_path: ".jcode/queue/runs/task_1/20260620T120000Z/stderr.txt".to_string(),
        }
    }

    struct CurrentDirGuard {
        original: std::path::PathBuf,
    }

    impl CurrentDirGuard {
        fn change_to(path: &std::path::Path) -> Self {
            let original = std::env::current_dir().expect("current dir");
            std::env::set_current_dir(path).expect("change current dir");
            Self { original }
        }
    }

    impl Drop for CurrentDirGuard {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.original);
        }
    }

    struct EnvGuard {
        key: &'static str,
        original: Option<std::ffi::OsString>,
    }

    impl EnvGuard {
        fn capture(key: &'static str) -> Self {
            Self {
                key,
                original: std::env::var_os(key),
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.original {
                Some(value) => crate::env::set_var(self.key, value),
                None => crate::env::remove_var(self.key),
            }
        }
    }
}
