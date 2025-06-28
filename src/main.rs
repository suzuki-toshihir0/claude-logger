use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod formatter;
mod parser;
mod watcher;

use watcher::LogWatcher;

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
        } => {
            let mut watcher = LogWatcher::new();

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
