use crate::queue as base_queue;

use anyhow::Result;

pub enum QueueSubcommand {
    Init,
    Add {
        title: String,
        body: String,
        priority: String,
        worker_profile: Option<String>,
    },
    List { all: bool },
    Next,
    Show { id: String },
    Status { id: String, status: String },
    Archive { id: String },
}

pub fn run_queue_command(cmd: QueueSubcommand) -> Result<()> {
    let project_dir = std::env::current_dir()?;
    match cmd {
        QueueSubcommand::Init => {
            let path = base_queue::init_project_queue(&project_dir)?;
            println!("Queue storage ready at {}", path.display());
        }
        QueueSubcommand::Add {
            title,
            body,
            priority,
            worker_profile,
        } => {
            let title = required_text(title, "title")?;
            let body = body.trim().to_string();
            let priority = optional_text(priority).unwrap_or_else(|| "normal".to_string());
            let worker_profile = worker_profile.and_then(optional_text);
            let task = base_queue::add_project_queue_task(
                &project_dir,
                base_queue::NewQueueTask {
                    title,
                    body,
                    priority,
                    worker_profile,
                },
            )?;
            println!("Added queue task:");
            println!("  id: {}", task.id);
            println!("  title: {}", task.title);
            println!("  status: {}", task.status);
            println!("  priority: {}", task.priority);
            if let Some(worker_profile) = task.worker_profile.as_deref() {
                println!("  worker_profile: {worker_profile}");
            }
        }
        QueueSubcommand::List { all } => {
            let path = base_queue::queue_file_path(&project_dir);
            if !path.exists() {
                print_missing_queue_message(&path);
                return Ok(());
            }

            let store = base_queue::load_project_queue(&project_dir)?;
            if store.tasks.is_empty() {
                println!("Queue is empty. Add a task with `jcode queue add \"Task title\"`.");
                return Ok(());
            }

            let tasks: Vec<_> = store
                .tasks
                .iter()
                .filter(|task| all || task.archived_at.is_none())
                .collect();

            if tasks.is_empty() {
                println!(
                    "Queue has no active tasks. Use jcode queue list --all to include archived tasks."
                );
                return Ok(());
            }

            println!("Queue tasks:");
            for (index, task) in tasks.iter().enumerate() {
                let archived = if task.archived_at.is_some() {
                    " archived"
                } else {
                    ""
                };
                println!(
                    "{}. {} [{}{}] ({}) {}",
                    index + 1,
                    task.id,
                    task.status,
                    archived,
                    task.priority,
                    task.title
                );
            }
        }
        QueueSubcommand::Next => {
            let path = base_queue::queue_file_path(&project_dir);
            if !path.exists() {
                print_missing_queue_message(&path);
                return Ok(());
            }

            let store = base_queue::load_project_queue(&project_dir)?;
            let Some(task) = base_queue::next_active_ready_task(&store) else {
                println!("No ready queue tasks.");
                return Ok(());
            };

            println!("Next queue task:");
            println!("  id: {}", task.id);
            println!("  title: {}", task.title);
            println!("  priority: {}", task.priority);
            if let Some(worker_profile) = task.worker_profile.as_deref() {
                println!("  worker_profile: {worker_profile}");
            }
            println!("  created_at: {}", task.created_at);
            println!("  updated_at: {}", task.updated_at);
        }
        QueueSubcommand::Show { id } => {
            let path = base_queue::queue_file_path(&project_dir);
            if !path.exists() {
                print_missing_queue_message(&path);
                return Ok(());
            }

            let id = required_text(id, "id")?;
            let store = base_queue::load_project_queue(&project_dir)?;
            let task = store
                .tasks
                .iter()
                .find(|task| task.id.as_str() == id.as_str())
                .ok_or_else(|| anyhow::anyhow!("queue task not found: {id}"))?;

            println!("Queue task:");
            println!("  id: {}", task.id);
            println!("  title: {}", task.title);
            println!("  body: {}", task.body);
            println!("  status: {}", task.status);
            println!("  priority: {}", task.priority);
            if let Some(worker_profile) = task.worker_profile.as_deref() {
                println!("  worker_profile: {worker_profile}");
            }
            println!("  created_at: {}", task.created_at);
            println!("  updated_at: {}", task.updated_at);
            if let Some(archived_at) = task.archived_at.as_deref() {
                println!("  archived_at: {archived_at}");
            }
        }
        QueueSubcommand::Status { id, status } => {
            let path = base_queue::queue_file_path(&project_dir);
            if !path.exists() {
                print_missing_queue_message(&path);
                return Ok(());
            }

            let id = required_text(id, "id")?;
            let status = required_text(status, "status")?;
            let update = base_queue::update_project_queue_task_status(&project_dir, &id, &status)?;
            println!(
                "Updated queue task {} status: {} -> {}",
                update.task.id, update.old_status, update.task.status
            );
        }
        QueueSubcommand::Archive { id } => {
            let path = base_queue::queue_file_path(&project_dir);
            if !path.exists() {
                print_missing_queue_message(&path);
                return Ok(());
            }

            let id = required_text(id, "id")?;
            let update = base_queue::archive_project_queue_task(&project_dir, &id)?;
            println!("Archived queue task {}", update.task.id);
        }
    }
    Ok(())
}

fn print_missing_queue_message(path: &std::path::Path) {
    println!(
        "No queue found at {}. Run jcode queue init or jcode queue add \"Task title\".",
        path.display()
    );
}

fn required_text(value: String, label: &str) -> Result<String> {
    optional_text(value).ok_or_else(|| anyhow::anyhow!("{label} cannot be empty"))
}

fn optional_text(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
