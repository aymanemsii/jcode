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
    List,
    Show { id: String },
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
        QueueSubcommand::List => {
            let path = base_queue::queue_file_path(&project_dir);
            if !path.exists() {
                println!(
                    "No queue found at {}. Run `jcode queue init` or `jcode queue add \"Task title\"`.",
                    path.display()
                );
                return Ok(());
            }

            let store = base_queue::load_project_queue(&project_dir)?;
            if store.tasks.is_empty() {
                println!("Queue is empty. Add a task with `jcode queue add \"Task title\"`.");
                return Ok(());
            }

            println!("Queue tasks:");
            for (index, task) in store.tasks.iter().enumerate() {
                println!(
                    "{}. {} [{}] ({}) {}",
                    index + 1,
                    task.id,
                    task.status,
                    task.priority,
                    task.title
                );
            }
        }
        QueueSubcommand::Show { id } => {
            let path = base_queue::queue_file_path(&project_dir);
            if !path.exists() {
                println!(
                    "No queue found at {}. Run `jcode queue init` or `jcode queue add \"Task title\"`.",
                    path.display()
                );
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
        }
    }
    Ok(())
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
