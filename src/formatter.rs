use crate::parser::{LogMessage, MessageRole};
use anyhow::Result;
use chrono::{Local, TimeZone};

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

    #[allow(dead_code)]
    pub fn with_timestamp(mut self, show: bool) -> Self {
        self.show_timestamp = show;
        self
    }

    #[allow(dead_code)]
    pub fn with_session_id(mut self, show: bool) -> Self {
        self.show_session_id = show;
        self
    }

    #[allow(dead_code)]
    pub fn with_compact_mode(mut self, compact: bool) -> Self {
        self.compact_mode = compact;
        self
    }

    /// Format message
    pub fn format_message(&self, message: &LogMessage) -> Result<String> {
        let mut output = String::new();

        // Timestamp
        if self.show_timestamp {
            let local_time = Local.from_utc_datetime(&message.timestamp.naive_utc());
            output.push_str(&format!("[{}] ", local_time.format("%H:%M:%S")));
        }

        // Role indicator
        let role_indicator = match message.role {
            MessageRole::User => "👤 User",
            MessageRole::Assistant => "🤖 Claude",
            MessageRole::System => "⚙️  System",
        };

        output.push_str(role_indicator);

        // Session ID
        if self.show_session_id {
            output.push_str(&format!(" ({})", &message.session_id[..8]));
        }

        output.push_str(": ");

        // Message content
        if self.compact_mode {
            // Compact mode: show only first 100 characters
            let content = if message.content.len() > 100 {
                format!("{}...", &message.content[..100])
            } else {
                message.content.clone()
            };
            output.push_str(&content.replace('\n', " "));
        } else {
            // Normal mode: show full content
            let formatted_content = self.format_content(&message.content);
            output.push_str(&formatted_content);
        }

        Ok(output)
    }

    /// Format content
    fn format_content(&self, content: &str) -> String {
        if content.contains('\n') {
            // Add indentation for multi-line content
            content
                .lines()
                .map(|line| {
                    if line.trim().is_empty() {
                        String::new()
                    } else {
                        format!("  {line}")
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            content.to_string()
        }
    }

    /// Display conversation separator
    #[allow(dead_code)]
    pub fn format_separator(&self) -> String {
        "─".repeat(80)
    }

    /// Display session start
    #[allow(dead_code)]
    pub fn format_session_start(&self, session_id: &str) -> String {
        format!("🚀 New session started: {}", &session_id[..8])
    }

    /// Display session end
    #[allow(dead_code)]
    pub fn format_session_end(&self, session_id: &str) -> String {
        format!("🔚 Session ended: {}", &session_id[..8])
    }

    /// Display statistics
    #[allow(dead_code)]
    pub fn format_stats(&self, user_messages: usize, assistant_messages: usize) -> String {
        format!(
            "📊 Statistics: {user_messages} user messages, {assistant_messages} Claude messages"
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
            content: "Test message.\nSpanning multiple lines.".to_string(),
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
        assert!(result.contains("👤 User"));
        assert!(result.contains("Test message."));
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
