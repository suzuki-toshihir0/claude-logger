use clap::{Parser, Subcommand};
use anyhow::Result;
use std::path::PathBuf;

mod watcher;
mod parser;
mod formatter;

use watcher::LogWatcher;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Claude Codeのログファイルを監視して標準出力に流す
    Watch {
        /// 監視するプロジェクトのパス (例: /home/suzuki/.claude/projects/-home-suzuki-repos)
        #[arg(short, long)]
        project_path: Option<PathBuf>,
        
        /// 最新のプロジェクトを自動選択
        #[arg(short, long)]
        latest: bool,
        
        /// すべてのプロジェクトを監視
        #[arg(short, long)]
        all: bool,
    },
    /// 利用可能なプロジェクトを一覧表示
    List,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Watch { project_path, latest, all } => {
            let mut watcher = LogWatcher::new();
            
            if *all {
                println!("すべてのプロジェクトを監視中...");
                watcher.watch_all().await?;
            } else if *latest {
                println!("最新のプロジェクトを監視中...");
                watcher.watch_latest().await?;
            } else if let Some(path) = project_path {
                println!("プロジェクト {:?} を監視中...", path);
                watcher.watch_project(path).await?;
            } else {
                eprintln!("プロジェクトパス、--latest、または --all オプションを指定してください");
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