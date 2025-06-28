use anyhow::{Result, Context};
use notify::{Watcher, RecursiveMode, Event, EventKind, event::CreateKind};
use std::path::{Path, PathBuf};
use std::fs;
use std::sync::mpsc;
use std::time::SystemTime;
use tokio::sync::mpsc as tokio_mpsc;
use tokio::time::{sleep, Duration};

use crate::parser::LogParser;
use crate::formatter::LogFormatter;

pub struct LogWatcher {
    claude_dir: PathBuf,
    parser: LogParser,
    formatter: LogFormatter,
}

impl LogWatcher {
    pub fn new() -> Self {
        let home = std::env::var("HOME").expect("HOME環境変数が設定されていません");
        let claude_dir = PathBuf::from(home).join(".claude").join("projects");
        
        Self {
            claude_dir,
            parser: LogParser::new(),
            formatter: LogFormatter::new(),
        }
    }

    /// 利用可能なプロジェクトを一覧表示
    pub async fn list_projects(&self) -> Result<()> {
        let entries = fs::read_dir(&self.claude_dir)
            .context("Claude プロジェクトディレクトリが見つかりません")?;

        println!("利用可能なプロジェクト:");
        for entry in entries {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let project_name = entry.file_name();
                let project_path = entry.path();
                
                // プロジェクト内のJSONLファイルを検索
                if let Ok(files) = fs::read_dir(&project_path) {
                    let jsonl_count = files
                        .filter_map(|f| f.ok())
                        .filter(|f| f.path().extension().and_then(|s| s.to_str()) == Some("jsonl"))
                        .count();
                    
                    println!("  {:?} ({} セッション)", project_name, jsonl_count);
                }
            }
        }
        Ok(())
    }

    /// 最新のプロジェクトを取得
    async fn get_latest_project(&self) -> Result<PathBuf> {
        let entries = fs::read_dir(&self.claude_dir)
            .context("Claude プロジェクトディレクトリが見つかりません")?;

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
            .context("プロジェクトが見つかりません")
    }

    /// 特定のプロジェクトを監視
    pub async fn watch_project(&mut self, project_path: &Path) -> Result<()> {
        let (tx, rx) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(tx)?;

        watcher.watch(project_path, RecursiveMode::Recursive)?;

        // 既存のファイルをチェック
        self.process_existing_files(project_path).await?;

        println!("プロジェクト {:?} の監視を開始しました。Ctrl+Cで終了します。", project_path);

        loop {
            match rx.recv() {
                Ok(Ok(event)) => {
                    if let Err(e) = self.handle_file_event(event).await {
                        eprintln!("ファイルイベントの処理でエラー: {}", e);
                    }
                }
                Ok(Err(e)) => eprintln!("ファイル監視エラー: {}", e),
                Err(e) => {
                    eprintln!("チャンネル受信エラー: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    /// 最新のプロジェクトを監視
    pub async fn watch_latest(&mut self) -> Result<()> {
        let latest = self.get_latest_project().await?;
        self.watch_project(&latest).await
    }

    /// すべてのプロジェクトを監視
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
                        let _ = tx_clone.send(format!("プロジェクト {:?} でエラー: {}", project_path, e)).await;
                    }
                });
            }
        }

        // メインスレッドでエラーメッセージを受信
        while let Some(error) = rx.recv().await {
            eprintln!("{}", error);
        }

        Ok(())
    }

    /// 既存のファイルを処理
    async fn process_existing_files(&mut self, project_path: &Path) -> Result<()> {
        let entries = fs::read_dir(project_path)?;
        
        for entry in entries {
            let entry = entry?;
            if entry.path().extension().and_then(|s| s.to_str()) == Some("jsonl") {
                if let Err(e) = self.process_jsonl_file(&entry.path()).await {
                    eprintln!("既存ファイル {:?} の処理でエラー: {}", entry.path(), e);
                }
            }
        }
        
        Ok(())
    }

    /// ファイルイベントを処理
    async fn handle_file_event(&mut self, event: Event) -> Result<()> {
        match event.kind {
            EventKind::Create(CreateKind::File) | EventKind::Modify(_) => {
                for path in event.paths {
                    if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                        // ファイルが完全に書き込まれるまで少し待つ
                        sleep(Duration::from_millis(100)).await;
                        self.process_jsonl_file(&path).await?;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// JSONLファイルを処理
    async fn process_jsonl_file(&mut self, path: &Path) -> Result<()> {
        let messages = self.parser.parse_file(path)?;
        
        for message in messages {
            let formatted = self.formatter.format_message(&message)?;
            println!("{}", formatted);
        }
        
        Ok(())
    }
}