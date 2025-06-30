use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use notify::{event::CreateKind, Event, EventKind, RecursiveMode, Watcher};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use tokio::sync::mpsc as tokio_mpsc;
use tokio::time::{sleep, Duration};

use crate::formatter::LogFormatter;
use crate::parser::LogParser;
use crate::webhook::{WebhookResult, WebhookSender};
use crate::WebhookFormat;
use url::Url;

pub struct LogWatcher {
    claude_dir: PathBuf,
    parser: LogParser,
    formatter: LogFormatter,
    webhook_sender: Option<WebhookSender>,
    include_existing: bool,
    startup_time: DateTime<Utc>,
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
            include_existing: false,
            startup_time: Utc::now(),
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
                    eprintln!("Failed to configure webhook: {e}");
                }
            }
        }
        self
    }

    pub fn with_include_existing(mut self, include_existing: bool) -> Self {
        self.include_existing = include_existing;
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

    /// Get the latest session file across all projects
    async fn get_latest_session(&self) -> Result<PathBuf> {
        let latest_session = fs::read_dir(&self.claude_dir)
            .context("Claude projects directory not found")?
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().map_or(false, |ft| ft.is_dir()))
            .flat_map(|entry| {
                fs::read_dir(entry.path())
                    .into_iter()
                    .flatten()
                    .filter_map(|f| f.ok())
            })
            .filter(|file| file.path().extension().and_then(|s| s.to_str()) == Some("jsonl"))
            .filter_map(|file| {
                file.metadata()
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .map(|modified| (file.path(), modified))
            })
            .max_by_key(|(_, modified)| *modified);

        latest_session
            .map(|(path, _)| path)
            .context("No session files found")
    }

    /// Get the latest session file within a specific project
    async fn get_latest_session_in_project(&self, project_path: &Path) -> Result<PathBuf> {
        let latest_session = fs::read_dir(project_path)
            .context("Project directory not found")?
            .filter_map(|file| file.ok())
            .filter(|file| file.path().extension().and_then(|s| s.to_str()) == Some("jsonl"))
            .filter_map(|file| {
                file.metadata()
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .map(|modified| (file.path(), modified))
            })
            .max_by_key(|(_, modified)| *modified);

        latest_session
            .map(|(path, _)| path)
            .context("No session files found in project")
    }

    /// Monitor a specific project
    pub async fn watch_project(&mut self, project_path: &Path) -> Result<()> {
        let (tx, rx) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(tx)?;

        watcher.watch(project_path, RecursiveMode::Recursive)?;

        // Check existing files if include_existing is enabled
        if self.include_existing {
            self.process_existing_files(project_path).await?;
        }

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

    /// Monitor a specific session file
    pub async fn watch_session(&mut self, session_path: &Path) -> Result<()> {
        let (tx, rx) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(tx)?;

        // Watch the parent directory of the session file
        let parent_dir = session_path.parent().context("Session file has no parent directory")?;
        watcher.watch(parent_dir, RecursiveMode::NonRecursive)?;

        // Check existing content if include_existing is enabled
        if self.include_existing {
            if let Err(e) = self.process_jsonl_file(session_path).await {
                eprintln!("Error processing existing session file {:?}: {}", session_path, e);
            }
        }

        println!("Started monitoring session file {session_path:?}. Press Ctrl+C to exit.");

        loop {
            match rx.recv() {
                Ok(Ok(event)) => {
                    // Only process events for our specific session file
                    if event.paths.iter().any(|p| p == session_path) {
                        if let Err(e) = self.handle_file_event(event).await {
                            eprintln!("Error processing file event: {e}");
                        }
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

    /// Monitor the latest session across all projects
    pub async fn watch_latest_session(&mut self) -> Result<()> {
        let latest_session = self.get_latest_session().await?;
        self.watch_session(&latest_session).await
    }

    /// Monitor the latest session within a specific project
    pub async fn watch_latest_session_in_project(&mut self, project_path: &Path) -> Result<()> {
        let latest_session = self.get_latest_session_in_project(project_path).await?;
        self.watch_session(&latest_session).await
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
            // Skip existing messages if include_existing is false
            if !self.include_existing && message.timestamp < self.startup_time {
                continue;
            }

            let formatted = self.formatter.format_message(&message)?;
            if !formatted.trim().is_empty() {
                // Send to webhook if configured and get result
                let webhook_status = if let Some(ref webhook) = self.webhook_sender {
                    match webhook.send_message(&message, &formatted).await {
                        Ok(WebhookResult::Sent) => "",
                        Ok(WebhookResult::Skipped) => " [webhook: skipped]",
                        Err(e) => {
                            eprintln!("Failed to send webhook: {e}");
                            " [webhook: failed]"
                        }
                    }
                } else {
                    ""
                };

                println!("{formatted}{webhook_status}");
            }
        }

        Ok(())
    }
}
