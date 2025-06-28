use anyhow::Result;
use chrono::{Local, TimeZone};
use crate::parser::{LogMessage, MessageRole};

pub struct LogFormatter {
    show_timestamp: bool,
    show_session_id: bool,
    compact_mode: bool,
}

impl LogFormatter {
    pub fn new() -> Self {
        Self {
            show_timestamp: true,
            show_session_id: false,
            compact_mode: false,
        }
    }

    pub fn with_timestamp(mut self, show: bool) -> Self {
        self.show_timestamp = show;
        self
    }

    pub fn with_session_id(mut self, show: bool) -> Self {
        self.show_session_id = show;
        self
    }

    pub fn with_compact_mode(mut self, compact: bool) -> Self {
        self.compact_mode = compact;
        self
    }

    /// メッセージをフォーマット
    pub fn format_message(&self, message: &LogMessage) -> Result<String> {
        let mut output = String::new();

        // タイムスタンプ
        if self.show_timestamp {
            let local_time = Local.from_utc_datetime(&message.timestamp.naive_utc());
            output.push_str(&format!("[{}] ", local_time.format("%H:%M:%S")));
        }

        // ロール識別子
        let role_indicator = match message.role {
            MessageRole::User => "👤 ユーザー",
            MessageRole::Assistant => "🤖 Claude",
            MessageRole::System => "⚙️  システム",
        };

        output.push_str(role_indicator);

        // セッションID
        if self.show_session_id {
            output.push_str(&format!(" ({})", &message.session_id[..8]));
        }

        output.push_str(": ");

        // メッセージ内容
        if self.compact_mode {
            // コンパクトモード: 最初の100文字のみ表示
            let content = if message.content.len() > 100 {
                format!("{}...", &message.content[..100])
            } else {
                message.content.clone()
            };
            output.push_str(&content.replace('\n', " "));
        } else {
            // 通常モード: フル内容を表示
            let formatted_content = self.format_content(&message.content);
            output.push_str(&formatted_content);
        }

        Ok(output)
    }

    /// コンテンツをフォーマット
    fn format_content(&self, content: &str) -> String {
        if content.contains('\n') {
            // 複数行の場合はインデントを追加
            content
                .lines()
                .map(|line| {
                    if line.trim().is_empty() {
                        String::new()
                    } else {
                        format!("  {}", line)
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            content.to_string()
        }
    }

    /// 会話の区切りを表示
    pub fn format_separator(&self) -> String {
        "─".repeat(80)
    }

    /// セッション開始を表示
    pub fn format_session_start(&self, session_id: &str) -> String {
        format!("🚀 新しいセッション開始: {}", &session_id[..8])
    }

    /// セッション終了を表示
    pub fn format_session_end(&self, session_id: &str) -> String {
        format!("🔚 セッション終了: {}", &session_id[..8])
    }

    /// 統計情報を表示
    pub fn format_stats(&self, user_messages: usize, assistant_messages: usize) -> String {
        format!(
            "📊 統計: ユーザーメッセージ {} 件, Claudeメッセージ {} 件",
            user_messages, assistant_messages
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_message() -> LogMessage {
        LogMessage {
            role: MessageRole::User,
            content: "テストメッセージです。\n複数行にわたります。".to_string(),
            timestamp: Utc::now(),
            session_id: "test-session-12345".to_string(),
            uuid: "test-uuid".to_string(),
        }
    }

    #[test]
    fn test_basic_formatting() {
        let formatter = LogFormatter::new();
        let message = create_test_message();
        
        let result = formatter.format_message(&message).unwrap();
        assert!(result.contains("👤 ユーザー"));
        assert!(result.contains("テストメッセージです。"));
    }

    #[test]
    fn test_compact_mode() {
        let formatter = LogFormatter::new().with_compact_mode(true);
        let message = create_test_message();
        
        let result = formatter.format_message(&message).unwrap();
        assert!(!result.contains('\n'));
    }

    #[test]
    fn test_session_id_display() {
        let formatter = LogFormatter::new().with_session_id(true);
        let message = create_test_message();
        
        let result = formatter.format_message(&message).unwrap();
        assert!(result.contains("test-ses"));
    }
}