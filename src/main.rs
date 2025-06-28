use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

mod formatter;
mod parser;
mod watcher;

use watcher::LogWatcher;

#[derive(Debug, Clone, ValueEnum)]
pub enum ToolDisplayMode {
    /// Hide all tool information
    None,
    /// Show simple tool indicators (🔧 Bash)
    Simple,
    /// Show detailed tool information including parameters
    Detailed,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Watch Claude Code log files and stream to stdout
    Watch {
        /// Path to the project to monitor (e.g. /home/suzuki/.claude/projects/-home-suzuki-repos)
        #[arg(short, long)]
        project_path: Option<PathBuf>,

        /// Automatically select the latest project
        #[arg(short, long)]
        latest: bool,

        /// Monitor all projects
        #[arg(short, long)]
        all: bool,

        /// Tool display mode: none, simple, or detailed
        #[arg(long, default_value = "simple")]
        tool_display: ToolDisplayMode,
    },
    /// List available projects
    List,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Watch {
            project_path,
            latest,
            all,
            tool_display,
        } => {
            let mut watcher = LogWatcher::new().with_tool_display_mode(tool_display.clone());

            if *all {
                println!("Monitoring all projects...");
                watcher.watch_all().await?;
            } else if *latest {
                println!("Monitoring latest project...");
                watcher.watch_latest().await?;
            } else if let Some(path) = project_path {
                println!("Monitoring project {path:?}...");
                watcher.watch_project(path).await?;
            } else {
                eprintln!("Please specify project path, --latest, or --all option");
                std::process::exit(1);
            }
        }
        Commands::List => {
            let watcher = LogWatcher::new();
            watcher.list_projects().await?;
        }
    }

    Ok(())
}
