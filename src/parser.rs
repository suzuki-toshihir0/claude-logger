use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogMessage {
    pub role: MessageRole,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub session_id: String,
    pub uuid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

#[derive(Debug, Deserialize)]
struct RawLogEntry {
    #[serde(rename = "type")]
    entry_type: String,
    message: Option<Value>,
    timestamp: String,
    #[serde(rename = "sessionId")]
    session_id: Option<String>,
    uuid: String,
}

#[derive(Debug, Deserialize)]
struct MessageContent {
    role: String,
    content: Value,
}

pub struct LogParser {
    last_position: u64,
}

impl LogParser {
    pub fn new() -> Self {
        Self {
            last_position: 0,
        }
    }

    /// ファイル全体を解析
    pub fn parse_file(&mut self, path: &Path) -> Result<Vec<LogMessage>> {
        let mut file = File::open(path)
            .with_context(|| format!("ファイル {:?} を開けません", path))?;
        
        // 前回読んだ位置から続きを読む
        file.seek(SeekFrom::Start(self.last_position))?;
        let reader = BufReader::new(file);
        
        let mut messages = Vec::new();
        let mut current_position = self.last_position;
        
        for line in reader.lines() {
            let line = line?;
            current_position += line.len() as u64 + 1; // +1 for newline
            
            if let Ok(message) = self.parse_line(&line) {
                messages.push(message);
            }
        }
        
        self.last_position = current_position;
        Ok(messages)
    }

    /// 新しいメッセージのみを取得
    pub fn parse_new_messages(&mut self, path: &Path) -> Result<Vec<LogMessage>> {
        self.parse_file(path)
    }

    /// 1行のJSONLエントリを解析
    fn parse_line(&self, line: &str) -> Result<LogMessage> {
        let raw: RawLogEntry = serde_json::from_str(line)
            .context("JSONの解析に失敗しました")?;

        // ユーザーまたはアシスタントのメッセージのみを処理
        if raw.entry_type != "user" && raw.entry_type != "assistant" {
            return Err(anyhow::anyhow!("メッセージタイプではありません"));
        }

        let message = raw.message.ok_or_else(|| anyhow::anyhow!("メッセージフィールドがありません"))?;
        
        let content_msg: MessageContent = serde_json::from_value(message)
            .context("メッセージコンテンツの解析に失敗しました")?;

        let role = match content_msg.role.as_str() {
            "user" => MessageRole::User,
            "assistant" => MessageRole::Assistant,
            "system" => MessageRole::System,
            _ => return Err(anyhow::anyhow!("未知のロール: {}", content_msg.role)),
        };

        let content = self.extract_content(&content_msg.content)?;
        
        let timestamp = DateTime::parse_from_rfc3339(&raw.timestamp)
            .context("タイムスタンプの解析に失敗しました")?
            .with_timezone(&Utc);

        let session_id = raw.session_id.unwrap_or_else(|| "unknown".to_string());

        Ok(LogMessage {
            role,
            content,
            timestamp,
            session_id,
            uuid: raw.uuid,
        })
    }

    /// メッセージコンテンツを抽出
    fn extract_content(&self, content: &Value) -> Result<String> {
        match content {
            Value::String(s) => Ok(s.clone()),
            Value::Array(arr) => {
                let mut result = String::new();
                for item in arr {
                    if let Some(obj) = item.as_object() {
                        if let Some(content_type) = obj.get("type") {
                            match content_type.as_str().unwrap_or("") {
                                "text" => {
                                    if let Some(text) = obj.get("text") {
                                        if let Some(text_str) = text.as_str() {
                                            result.push_str(text_str);
                                            result.push('\n');
                                        }
                                    }
                                }
                                "tool_use" => {
                                    if let Some(name) = obj.get("name") {
                                        if let Some(name_str) = name.as_str() {
                                            result.push_str(&format!("[ツール使用: {}]", name_str));
                                            result.push('\n');
                                        }
                                    }
                                }
                                "tool_result" => {
                                    result.push_str("[ツール結果]");
                                    result.push('\n');
                                }
                                "thinking" => {
                                    result.push_str("[思考中...]");
                                    result.push('\n');
                                }
                                _ => {
                                    result.push_str(&format!("[{}]", content_type.as_str().unwrap_or("unknown")));
                                    result.push('\n');
                                }
                            }
                        }
                    }
                }
                Ok(result.trim_end().to_string())
            }
            _ => Ok(format!("メッセージ内容: {:?}", content)),
        }
    }

    /// 位置をリセット（ファイル全体を再読み込み）
    pub fn reset(&mut self) {
        self.last_position = 0;
    }
}