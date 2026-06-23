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
    let mut selected_task_id = None;
    normalize_selection(&board, &mut selected_task_id);
    terminal.draw(|frame| {
        draw_queue_board(
            frame,
            &board,
            &active_runs,
            selected_task_id.as_deref(),
            None,
        )
    })?;

    loop {
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => break,
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                KeyCode::Down | KeyCode::Char('j') => {
                    move_selection(&board, &mut selected_task_id, 1);
                    terminal.draw(|frame| {
                        draw_queue_board(
                            frame,
                            &board,
                            &active_runs,
                            selected_task_id.as_deref(),
                            status_message.as_deref(),
                        )
                    })?;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    move_selection(&board, &mut selected_task_id, -1);
                    terminal.draw(|frame| {
                        draw_queue_board(
                            frame,
                            &board,
                            &active_runs,
                            selected_task_id.as_deref(),
                            status_message.as_deref(),
                        )
                    })?;
                }
                KeyCode::Char('r') => {
                    status_message =
                        Some(refresh_board_state(&mut board, &mut active_runs, &options)?);
                    normalize_selection(&board, &mut selected_task_id);
                    terminal.draw(|frame| {
                        draw_queue_board(
                            frame,
                            &board,
                            &active_runs,
                            selected_task_id.as_deref(),
                            status_message.as_deref(),
                        )
                    })?;
                }
                KeyCode::Char('a') => {
                    let previous_visible_ids = visible_task_ids(&board)
                        .into_iter()
                        .map(str::to_string)
                        .collect::<Vec<_>>();
                    let previous_selection = selected_task_id.clone();
                    status_message = Some(approve_selected_review_task(
                        &mut board,
                        &mut active_runs,
                        selected_task_id.as_deref(),
                        &options,
                    )?);
                    preserve_selection_after_reload(
                        &board,
                        &mut selected_task_id,
                        previous_selection.as_deref(),
                        &previous_visible_ids,
                    );
                    terminal.draw(|frame| {
                        draw_queue_board(
                            frame,
                            &board,
                            &active_runs,
                            selected_task_id.as_deref(),
                            status_message.as_deref(),
                        )
                    })?;
                }
                _ => {}
            },
            Event::Resize(_, _) => {
                terminal.draw(|frame| {
                    draw_queue_board(
                        frame,
                        &board,
                        &active_runs,
                        selected_task_id.as_deref(),
                        status_message.as_deref(),
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
    selected_task_id: Option<&str>,
    status_message: Option<&str>,
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
    for (column_area, column) in columns.iter().zip(board.columns.iter()) {
        frame.render_widget(render_column(column, selected_task_id), *column_area);
    }

    if active_runs_height(active_runs) > 0 {
        frame.render_widget(
            Paragraph::new(active_runs_text(active_runs))
                .block(Block::default().borders(Borders::ALL).title("Active runs"))
                .wrap(Wrap { trim: true }),
            layout[2],
        );
    }

    frame.render_widget(Paragraph::new(footer_text(status_message)), layout[3]);
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

fn normalize_selection(board: &crate::queue::QueueBoard, selected_task_id: &mut Option<String>) {
    let visible_ids = visible_task_ids(board);
    if visible_ids.is_empty() {
        *selected_task_id = None;
        return;
    }

    if selected_task_id
        .as_deref()
        .is_some_and(|selected| visible_ids.contains(&selected))
    {
        return;
    }

    *selected_task_id = Some(visible_ids[0].to_string());
}

fn preserve_selection_after_reload(
    board: &crate::queue::QueueBoard,
    selected_task_id: &mut Option<String>,
    previous_selection: Option<&str>,
    previous_visible_ids: &[String],
) {
    let visible_ids = visible_task_ids(board);
    if visible_ids.is_empty() {
        *selected_task_id = None;
        return;
    }

    if previous_selection.is_some_and(|selected| visible_ids.contains(&selected)) {
        return;
    }

    let Some(previous_selection) = previous_selection else {
        *selected_task_id = Some(visible_ids[0].to_string());
        return;
    };
    let Some(previous_index) = previous_visible_ids
        .iter()
        .position(|id| id == previous_selection)
    else {
        *selected_task_id = Some(visible_ids[0].to_string());
        return;
    };

    let next_id = previous_visible_ids
        .iter()
        .skip(previous_index + 1)
        .chain(previous_visible_ids.iter().take(previous_index))
        .find(|id| visible_ids.contains(&id.as_str()));
    *selected_task_id = Some(
        next_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| visible_ids[0].to_string()),
    );
}

fn move_selection(
    board: &crate::queue::QueueBoard,
    selected_task_id: &mut Option<String>,
    delta: isize,
) {
    normalize_selection(board, selected_task_id);

    let visible_ids = visible_task_ids(board);
    if visible_ids.is_empty() {
        return;
    }

    let current_index = selected_task_id
        .as_deref()
        .and_then(|selected| visible_ids.iter().position(|id| *id == selected))
        .unwrap_or(0);
    let next_index = (current_index as isize + delta).rem_euclid(visible_ids.len() as isize);
    *selected_task_id = Some(visible_ids[next_index as usize].to_string());
}

fn visible_task_ids(board: &crate::queue::QueueBoard) -> Vec<&str> {
    board
        .columns
        .iter()
        .flat_map(|column| column.tasks.iter().map(|task| task.id.as_str()))
        .collect()
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

fn footer_text(status_message: Option<&str>) -> String {
    match status_message {
        Some(message) => format!("{message} | a approve  j/k move  r refresh  q/Esc quit"),
        None => "a approve  j/k move  r refresh  q/Esc quit".to_string(),
    }
}

fn render_column(
    column: &crate::queue::QueueBoardColumn,
    selected_task_id: Option<&str>,
) -> Paragraph<'static> {
    let mut lines = Vec::new();
    if column.tasks.is_empty() {
        lines.push("none".to_string());
    } else {
        for task in &column.tasks {
            let selection_marker = if selected_task_id == Some(task.id.as_str()) {
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
    if active_runs.is_empty() { 0 } else { 5 }
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
        assert_eq!(
            footer_text(None),
            "a approve  j/k move  r refresh  q/Esc quit"
        );
        assert_eq!(
            footer_text(Some("refreshed")),
            "refreshed | a approve  j/k move  r refresh  q/Esc quit"
        );
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
        let mut selection = None;

        normalize_selection(&board, &mut selection);

        assert_eq!(selection.as_deref(), Some("backlog_1"));
    }

    #[test]
    fn selection_preserves_existing_task_after_refresh() {
        let board = test_board(&["backlog_1", "ready_1"]);
        let mut selection = Some("ready_1".to_string());

        normalize_selection(&board, &mut selection);

        assert_eq!(selection.as_deref(), Some("ready_1"));
    }

    #[test]
    fn selection_falls_back_when_selected_task_disappears() {
        let board = test_board(&["ready_1"]);
        let mut selection = Some("missing".to_string());

        normalize_selection(&board, &mut selection);

        assert_eq!(selection.as_deref(), Some("ready_1"));
    }

    #[test]
    fn selection_clears_when_board_is_empty() {
        let board = test_board(&[]);
        let mut selection = Some("missing".to_string());

        normalize_selection(&board, &mut selection);

        assert_eq!(selection, None);
    }

    #[test]
    fn selection_moves_next_and_previous_through_visible_tasks() {
        let board = test_board(&["backlog_1", "ready_1", "ready_2"]);
        let mut selection = Some("backlog_1".to_string());

        move_selection(&board, &mut selection, 1);
        assert_eq!(selection.as_deref(), Some("ready_1"));

        move_selection(&board, &mut selection, 1);
        assert_eq!(selection.as_deref(), Some("ready_2"));

        move_selection(&board, &mut selection, 1);
        assert_eq!(selection.as_deref(), Some("backlog_1"));

        move_selection(&board, &mut selection, -1);
        assert_eq!(selection.as_deref(), Some("ready_2"));
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
    fn selection_preserves_approved_task_when_still_visible_after_reload() {
        let reloaded_board =
            test_board_with_statuses(&[("task_1", crate::queue::TaskStatus::Done)]);
        let previous_visible_ids = vec!["task_1".to_string(), "task_2".to_string()];
        let mut selection = Some("task_1".to_string());

        preserve_selection_after_reload(
            &reloaded_board,
            &mut selection,
            Some("task_1"),
            &previous_visible_ids,
        );

        assert_eq!(selection.as_deref(), Some("task_1"));
    }

    #[test]
    fn selection_moves_to_next_visible_task_when_approved_task_disappears() {
        let reloaded_board =
            test_board_with_statuses(&[("task_2", crate::queue::TaskStatus::Ready)]);
        let previous_visible_ids = vec!["task_1".to_string(), "task_2".to_string()];
        let mut selection = Some("task_1".to_string());

        preserve_selection_after_reload(
            &reloaded_board,
            &mut selection,
            Some("task_1"),
            &previous_visible_ids,
        );

        assert_eq!(selection.as_deref(), Some("task_2"));
    }

    #[test]
    fn selection_clears_when_no_tasks_remain_after_approve() {
        let reloaded_board = test_board_with_statuses(&[]);
        let previous_visible_ids = vec!["task_1".to_string()];
        let mut selection = Some("task_1".to_string());

        preserve_selection_after_reload(
            &reloaded_board,
            &mut selection,
            Some("task_1"),
            &previous_visible_ids,
        );

        assert_eq!(selection, None);
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
        crate::queue::Task {
            id: id.to_string(),
            title: id.to_string(),
            description: String::new(),
            project: None,
            status,
            priority: crate::queue::TaskPriority::Normal,
            worker_profile: None,
            output_path: None,
            created_at: timestamp,
            updated_at: timestamp,
        }
    }

    fn test_time(value: &str) -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339(value)
            .unwrap()
            .with_timezone(&chrono::Utc)
    }
}
