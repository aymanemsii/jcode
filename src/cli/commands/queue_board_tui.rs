use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

pub(super) fn run_read_only_queue_board(
    mut terminal: ratatui::DefaultTerminal,
    mut board: crate::queue::QueueBoard,
    mut active_runs: Vec<crate::queue::RunState>,
    options: QueueBoardTuiOptions,
) -> Result<()> {
    let mut status_message = None;
    let mut selection = BoardSelection::default();
    let mut input_mode: Option<AddTaskPrompt> = None;
    select_first_available_task(&board, &mut selection);
    terminal.draw(|frame| {
        draw_queue_board(frame, &board, &active_runs, &selection, None, None)
    })?;

    loop {
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                if let Some(input) = input_mode.as_mut() {
                    let mut submit: Option<AddTaskPrompt> = None;
                    let mut cancel = false;
                    match key.code {
                        KeyCode::Esc => {
                            cancel = true;
                        }
                        KeyCode::Enter => {
                            submit = Some(input.clone());
                        }
                        KeyCode::Backspace => {
                            input.title.pop();
                        }
                        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                            input.title.push(c);
                        }
                        _ => {}
                    }

                    if cancel {
                        input_mode = None;
                        status_message = Some("add cancelled".to_string());
                    } else if let Some(input) = submit {
                        match add_task_from_prompt(
                            &mut board,
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
                            status_message =
                                Some(refresh_board_state(&mut board, &mut active_runs, &options)?);
                            preserve_selection_after_refresh(&board, &mut selection);
                        }
                        KeyCode::Char('a') => {
                            status_message = Some(approve_selected_review_task(
                                &mut board,
                                &mut active_runs,
                                selection.selected_task_id(),
                                &options,
                            )?);
                            preserve_selection_after_refresh(&board, &mut selection);
                        }
                        KeyCode::Char('x') => {
                            status_message = Some(run_selected_task_in_background(
                                &mut board,
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
                        &active_runs,
                        &selection,
                        status_message.as_deref(),
                        input_mode.as_ref(),
                    )
                })?;
            }
            Event::Resize(_, _) => {
                terminal.draw(|frame| {
                    draw_queue_board(
                        frame,
                        &board,
                        &active_runs,
                        &selection,
                        status_message.as_deref(),
                        input_mode.as_ref(),
                    )
                })?;
            }
            _ => {}
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
    active_runs: &[crate::queue::RunState],
    selection: &BoardSelection,
    status_message: Option<&str>,
    input_mode: Option<&AddTaskPrompt>,
) {
    let area = frame.area();
    let layout = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(5),
        Constraint::Length(active_runs_height(active_runs)),
        Constraint::Length(1),
    ])
    .split(area);

    frame.render_widget(
        Paragraph::new(header_text(board))
            .block(Block::default().borders(Borders::ALL).title("Queue board")),
        layout[0],
    );

    let column_constraints =
        vec![Constraint::Ratio(1, board.columns.len() as u32); board.columns.len()];
    let columns = Layout::horizontal(column_constraints).split(layout[1]);
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
            layout[2],
        );
    }

    frame.render_widget(Paragraph::new(footer_text(status_message, input_mode)), layout[3]);
}

fn refresh_board_state(
    board: &mut crate::queue::QueueBoard,
    active_runs: &mut Vec<crate::queue::RunState>,
    options: &QueueBoardTuiOptions,
) -> Result<String> {
    let mut index = crate::queue::load_run_index()?;
    let mut state = crate::queue::load()?;
    let output = super::refresh_queue_runs(&mut index, &mut state, chrono::Utc::now());
    if output.run_index_changed {
        crate::queue::save_run_index(&index)?;
    }
    if output.queue_changed {
        crate::queue::save(&state)?;
    }

    *board = crate::queue::build_queue_board(
        &state,
        options.worker_profile.as_deref(),
        Some(options.limit),
    );
    *active_runs = filtered_active_runs(&index, options.worker_profile.as_deref());

    Ok(refresh_status_text(&output))
}

fn approve_selected_review_task(
    board: &mut crate::queue::QueueBoard,
    active_runs: &mut Vec<crate::queue::RunState>,
    selected_task_id: Option<&str>,
    options: &QueueBoardTuiOptions,
) -> Result<String> {
    let mut state = crate::queue::load()?;
    let message = approve_selected_review_task_in_state(
        board,
        &mut state,
        selected_task_id,
        chrono::Utc::now(),
    )?;
    if message.starts_with("approved ") {
        crate::queue::save(&state)?;
        let index = crate::queue::load_run_index()?;
        *board = crate::queue::build_queue_board(
            &state,
            options.worker_profile.as_deref(),
            Some(options.limit),
        );
        *active_runs = filtered_active_runs(&index, options.worker_profile.as_deref());
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
    active_runs: &mut Vec<crate::queue::RunState>,
    selected_task_id: Option<&str>,
    options: &QueueBoardTuiOptions,
) -> Result<String> {
    run_selected_task_in_background_with_starter(
        board,
        active_runs,
        selected_task_id,
        options,
        super::start_rendered_worker_command_background,
    )
}

fn run_selected_task_in_background_with_starter(
    board: &mut crate::queue::QueueBoard,
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

    let state = crate::queue::load()?;
    let Some(task) = state.tasks.iter().find(|task| task.id == task_id) else {
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
    drop(state);

    let output =
        super::start_selected_queue_task_background_with_starter(task_id, background_starter)?;
    let state = crate::queue::load()?;
    let index = crate::queue::load_run_index()?;
    *board = crate::queue::build_queue_board(
        &state,
        options.worker_profile.as_deref(),
        Some(options.limit),
    );
    *active_runs = filtered_active_runs(&index, options.worker_profile.as_deref());

    Ok(format!("started {} as {}", output.task_id, output.run_id))
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct AddTaskPrompt {
    title: String,
}

fn add_task_from_prompt(
    board: &mut crate::queue::QueueBoard,
    active_runs: &mut Vec<crate::queue::RunState>,
    selection: &mut BoardSelection,
    input: &AddTaskPrompt,
    options: &QueueBoardTuiOptions,
) -> Result<String> {
    let mut state = crate::queue::load()?;
    let task_id = add_task_to_state(&mut state, &input.title)?;
    crate::queue::save(&state)?;

    let index = crate::queue::load_run_index()?;
    *board = crate::queue::build_queue_board(
        &state,
        options.worker_profile.as_deref(),
        Some(options.limit),
    );
    *active_runs = filtered_active_runs(&index, options.worker_profile.as_deref());
    if !select_task_by_id(board, selection, &task_id) {
        preserve_selection_after_refresh(board, selection);
    }

    Ok(format!("added {task_id}"))
}

fn add_task_to_state(state: &mut crate::queue::QueueState, title: &str) -> Result<String> {
    let title = title.trim();
    if title.is_empty() {
        anyhow::bail!("title required");
    }

    let task = crate::queue::Task::new(
        title.to_string(),
        String::new(),
        None,
        crate::queue::TaskPriority::Normal,
        None,
        None,
    );
    let task_id = task.id.clone();
    state.tasks.push(task);
    Ok(task_id)
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

fn footer_text(status_message: Option<&str>, input_mode: Option<&AddTaskPrompt>) -> String {
    if let Some(input) = input_mode {
        let prompt = format!("new task title: {}", input.title);
        let help = "Enter submit  Esc cancel";
        return match status_message {
            Some(message) => format!("{message} | {prompt} | {help}"),
            None => format!("{prompt} | {help}"),
        };
    }

    let help =
        "Up/Down or j/k move within column  Left/Right or h/l move columns  x run  n new  a approve  r refresh  q quit";
    match status_message {
        Some(message) => format!("{message} | {help}"),
        None => help.to_string(),
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
            "Up/Down or j/k move within column  Left/Right or h/l move columns  x run  n new  a approve  r refresh  q quit";
        assert_eq!(footer_text(None, None), help);
        assert_eq!(
            footer_text(Some("refreshed"), None),
            format!("refreshed | {help}")
        );
    }

    #[test]
    fn footer_mentions_submit_and_cancel_in_input_mode() {
        let input = AddTaskPrompt {
            title: "Update docs".to_string(),
        };

        assert_eq!(
            footer_text(None, Some(&input)),
            "new task title: Update docs | Enter submit  Esc cancel"
        );
    }

    #[test]
    fn add_task_to_state_adds_backlog_task_with_defaults() {
        let mut state = crate::queue::QueueState::default();

        let task_id = add_task_to_state(&mut state, "  New board task  ").expect("add task");

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

        let err = add_task_to_state(&mut state, "   ").expect_err("empty title rejected");

        assert!(err.to_string().contains("title required"));
        assert!(state.tasks.is_empty());
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
        let mut active_runs = Vec::new();
        let options = test_options();

        let message = run_selected_task_in_background_with_starter(
            &mut board,
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
        let mut active_runs = Vec::new();
        let options = test_options();

        let message = run_selected_task_in_background_with_starter(
            &mut board,
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
        let mut active_runs = Vec::new();
        let options = test_options();

        let message = run_selected_task_in_background_with_starter(
            &mut board,
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
        let mut active_runs = Vec::new();
        let options = test_options();
        let mut saw_command = false;

        let message = run_selected_task_in_background_with_starter(
            &mut board,
            &mut active_runs,
            Some("task_1"),
            &options,
            |command, run_dir, stdout_path, stderr_path| {
                saw_command = true;
                assert!(command.contains("--task task_1"));
                assert!(command.contains("task_1.md"));
                assert!(
                    run_dir
                        .parent()
                        .is_some_and(|path| path.ends_with("task_1"))
                );
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
