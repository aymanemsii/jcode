use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

pub(super) fn run_read_only_queue_board(
    mut terminal: ratatui::DefaultTerminal,
    board: &crate::queue::QueueBoard,
    active_runs: &[crate::queue::RunState],
) -> Result<()> {
    terminal.draw(|frame| draw_queue_board(frame, board, active_runs))?;

    loop {
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => break,
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                _ => {}
            },
            Event::Resize(_, _) => {
                terminal.draw(|frame| draw_queue_board(frame, board, active_runs))?;
            }
            _ => {}
        }
    }

    Ok(())
}

fn draw_queue_board(
    frame: &mut Frame<'_>,
    board: &crate::queue::QueueBoard,
    active_runs: &[crate::queue::RunState],
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
        frame.render_widget(render_column(column), *column_area);
    }

    if active_runs_height(active_runs) > 0 {
        frame.render_widget(
            Paragraph::new(active_runs_text(active_runs))
                .block(Block::default().borders(Borders::ALL).title("Active runs"))
                .wrap(Wrap { trim: true }),
            layout[2],
        );
    }

    frame.render_widget(Paragraph::new("q/Esc quit"), layout[3]);
}

fn render_column(column: &crate::queue::QueueBoardColumn) -> Paragraph<'static> {
    let mut lines = Vec::new();
    if column.tasks.is_empty() {
        lines.push("none".to_string());
    } else {
        for task in &column.tasks {
            lines.push(format!(
                "{}  {}",
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
}
