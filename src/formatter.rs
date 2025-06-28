use crate::parser::{LogMessage, MessageRole};
use anyhow::Result;
use chrono::{Local, TimeZone};
use serde_json::Value;

struct ToolContent {
    simple_format: String,
    detailed_format: String,
}

pub struct LogFormatter {
    show_timestamp: bool,
    show_session_id: bool,
    compact_mode: bool,
    tool_display_mode: crate::ToolDisplayMode,
}

impl LogFormatter {
    pub fn new() -> Self {
        Self {
            show_timestamp: true,
            show_session_id: false,
            compact_mode: false,
            tool_display_mode: crate::ToolDisplayMode::Simple,
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

    pub fn with_tool_display_mode(mut self, mode: crate::ToolDisplayMode) -> Self {
        self.tool_display_mode = mode;
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
        let formatted_content = self.format_message_content(message)?;

        // Skip empty messages (filtered tool messages in none mode)
        if formatted_content.trim().is_empty() {
            return Ok(String::new());
        }

        if self.compact_mode {
            // Compact mode: show only first 100 characters
            let content = if formatted_content.len() > 100 {
                format!("{}...", &formatted_content[..100])
            } else {
                formatted_content
            };
            output.push_str(&content.replace('\n', " "));
        } else {
            // Normal mode: show full content
            output.push_str(&self.format_content(&formatted_content));
        }

        Ok(output)
    }

    /// Format message content based on tool display mode
    fn format_message_content(&self, message: &LogMessage) -> Result<String> {
        // If no raw content, fallback to simple content
        let raw_content = match &message.raw_content {
            Some(content) => content,
            None => return Ok(message.content.clone()),
        };

        // Check if this is a tool-related message
        if let Some(tool_content) = self.extract_tool_content(raw_content) {
            match self.tool_display_mode {
                crate::ToolDisplayMode::None => {
                    // Filter out tool messages, but keep text content
                    if message.content.trim().is_empty()
                        || message.content.starts_with("[Tool")
                        || message.content.starts_with("[Thinking")
                    {
                        return Ok(String::new());
                    }
                }
                crate::ToolDisplayMode::Simple => {
                    return Ok(tool_content.simple_format);
                }
                crate::ToolDisplayMode::Detailed => {
                    return Ok(tool_content.detailed_format);
                }
            }
        }

        // Not a tool message, return normal content
        Ok(message.content.clone())
    }

    /// Extract tool information from raw content
    fn extract_tool_content(&self, content: &Value) -> Option<ToolContent> {
        if let Value::Array(arr) = content {
            for item in arr {
                if let Some(obj) = item.as_object() {
                    if let Some(content_type) = obj.get("type") {
                        match content_type.as_str().unwrap_or("") {
                            "tool_use" => {
                                let tool_name = obj
                                    .get("name")
                                    .and_then(|n| n.as_str())
                                    .unwrap_or("Unknown");

                                let simple = format!("🔧 {tool_name}");

                                let detailed = if let Some(input) = obj.get("input") {
                                    let input_str = self.format_tool_input(input);
                                    format!("🔧 {tool_name}: {input_str}")
                                } else {
                                    simple.clone()
                                };

                                return Some(ToolContent {
                                    simple_format: simple,
                                    detailed_format: detailed,
                                });
                            }
                            "tool_result" => {
                                let simple = "✅ Result".to_string();

                                let detailed = if let Some(content) = obj.get("content") {
                                    let content_str = self.format_tool_result(content);
                                    format!("✅ {content_str}")
                                } else {
                                    simple.clone()
                                };

                                return Some(ToolContent {
                                    simple_format: simple,
                                    detailed_format: detailed,
                                });
                            }
                            "thinking" => {
                                let simple = "💭 Thinking...".to_string();
                                return Some(ToolContent {
                                    simple_format: simple.clone(),
                                    detailed_format: simple,
                                });
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        None
    }

    /// Format tool input for detailed display
    fn format_tool_input(&self, input: &Value) -> String {
        match input {
            Value::Object(obj) => {
                if let Some(command) = obj.get("command") {
                    if let Some(cmd_str) = command.as_str() {
                        let truncated = cmd_str.chars().take(50).collect::<String>();
                        return truncated + if cmd_str.len() > 50 { "..." } else { "" };
                    }
                }
                "(...)".to_string()
            }
            Value::String(s) => {
                let truncated = s.chars().take(50).collect::<String>();
                truncated + if s.len() > 50 { "..." } else { "" }
            }
            _ => "(...)".to_string(),
        }
    }

    /// Format tool result for detailed display
    fn format_tool_result(&self, content: &Value) -> String {
        match content {
            Value::String(s) => {
                let first_line = s.lines().next().unwrap_or("");
                let truncated = first_line.chars().take(50).collect::<String>();
                truncated + if first_line.len() > 50 { "..." } else { "" }
            }
            _ => "Result".to_string(),
        }
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
            project_name: "test-project".to_string(),
            raw_content: None,
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
