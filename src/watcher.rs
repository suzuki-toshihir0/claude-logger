use anyhow::{Context, Result};
use notify::{Event, EventKind, RecursiveMode, Watcher, event::CreateKind};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::SystemTime;
use tokio::sync::mpsc as tokio_mpsc;
use tokio::time::{Duration, sleep};

use crate::formatter::LogFormatter;
use crate::parser::LogParser;
use crate::webhook::WebhookSender;
use crate::{WebhookFormat};
use url::Url;

pub struct LogWatcher {
    claude_dir: PathBuf,
    parser: LogParser,
    formatter: LogFormatter,
    webhook_sender: Option<WebhookSender>,
}

impl LogWatcher {
    pub fn new() -> Self {
        let home = std::env::var("HOME").expect("HOME environment variable not set");
        let claude_dir = PathBuf::from(home).join(".claude").join("projects");

        Self {
            claude_dir,
            parser: LogParser::new(),
            formatter: LogFormatter::new(),
            webhook_sender: None,
        }
    }

    pub fn with_tool_display_mode(mut self, mode: crate::ToolDisplayMode) -> Self {
        self.formatter = self.formatter.with_tool_display_mode(mode);
        self
    }

    pub fn with_webhook(mut self, url: Option<Url>, format: WebhookFormat) -> Self {
        if let Some(webhook_url) = url {
            match WebhookSender::new(webhook_url, format) {
                Ok(sender) => {
                    self.webhook_sender = Some(sender);
                    println!("Webhook configured successfully");
                }
                Err(e) => {
                    eprintln!("Failed to configure webhook: {}", e);
                }
            }
        }
        self
    }

    /// List available projects
    pub async fn list_projects(&self) -> Result<()> {
        let entries =
            fs::read_dir(&self.claude_dir).context("Claude projects directory not found")?;

        println!("Available projects:");
        for entry in entries {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let project_name = entry.file_name();
                let project_path = entry.path();

                // Search for JSONL files within the project
                if let Ok(files) = fs::read_dir(&project_path) {
                    let jsonl_count = files
                        .filter_map(|f| f.ok())
                        .filter(|f| f.path().extension().and_then(|s| s.to_str()) == Some("jsonl"))
                        .count();

                    println!("  {project_name:?} ({jsonl_count} sessions)");
                }
            }
        }
        Ok(())
    }

    /// Get the latest project
    async fn get_latest_project(&self) -> Result<PathBuf> {
        let entries =
            fs::read_dir(&self.claude_dir).context("Claude projects directory not found")?;

        let mut latest_project: Option<(PathBuf, SystemTime)> = None;

        for entry in entries {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let metadata = entry.metadata()?;
                let modified = metadata.modified()?;

                if latest_project.is_none() || modified > latest_project.as_ref().unwrap().1 {
                    latest_project = Some((entry.path(), modified));
                }
            }
        }

        latest_project
            .map(|(path, _)| path)
            .context("No projects found")
    }

    /// Monitor a specific project
    pub async fn watch_project(&mut self, project_path: &Path) -> Result<()> {
        let (tx, rx) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(tx)?;

        watcher.watch(project_path, RecursiveMode::Recursive)?;

        // Check existing files
        self.process_existing_files(project_path).await?;

        println!("Started monitoring project {project_path:?}. Press Ctrl+C to exit.");

        loop {
            match rx.recv() {
                Ok(Ok(event)) => {
                    if let Err(e) = self.handle_file_event(event).await {
                        eprintln!("Error processing file event: {e}");
                    }
                }
                Ok(Err(e)) => eprintln!("File watching error: {e}"),
                Err(e) => {
                    eprintln!("Channel receive error: {e}");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Monitor the latest project
    pub async fn watch_latest(&mut self) -> Result<()> {
        let latest = self.get_latest_project().await?;
        self.watch_project(&latest).await
    }

    /// Monitor all projects
    pub async fn watch_all(&self) -> Result<()> {
        let (tx, mut rx) = tokio_mpsc::channel(100);
        let entries = fs::read_dir(&self.claude_dir)?;

        for entry in entries {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let project_path = entry.path();
                let tx_clone = tx.clone();

                tokio::spawn(async move {
                    let mut watcher = LogWatcher::new();
                    if let Err(e) = watcher.watch_project(&project_path).await {
                        let _ = tx_clone
                            .send(format!("Error in project {project_path:?}: {e}"))
                            .await;
                    }
                });
            }
        }

        // Receive error messages on main thread
        while let Some(error) = rx.recv().await {
            eprintln!("{error}");
        }

        Ok(())
    }

    /// Process existing files
    async fn process_existing_files(&mut self, project_path: &Path) -> Result<()> {
        let entries = fs::read_dir(project_path)?;

        for entry in entries {
            let entry = entry?;
            if entry.path().extension().and_then(|s| s.to_str()) == Some("jsonl") {
                if let Err(e) = self.process_jsonl_file(&entry.path()).await {
                    eprintln!("Error processing existing file {:?}: {}", entry.path(), e);
                }
            }
        }

        Ok(())
    }

    /// Handle file events
    async fn handle_file_event(&mut self, event: Event) -> Result<()> {
        match event.kind {
            EventKind::Create(CreateKind::File) | EventKind::Modify(_) => {
                for path in event.paths {
                    if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                        // Wait briefly for file to be completely written
                        sleep(Duration::from_millis(100)).await;
                        self.process_jsonl_file(&path).await?;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Process JSONL file
    async fn process_jsonl_file(&mut self, path: &Path) -> Result<()> {
        let messages = self.parser.parse_file(path)?;

        for message in messages {
            let formatted = self.formatter.format_message(&message)?;
            if !formatted.trim().is_empty() {
                println!("{formatted}");

                // Send to webhook if configured
                if let Some(ref webhook) = self.webhook_sender {
                    if let Err(e) = webhook.send_message(&message, &formatted).await {
                        eprintln!("Failed to send webhook: {}", e);
                    }
                }
            }
        }

        Ok(())
    }
}
