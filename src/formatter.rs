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

                                let tool_icon = if tool_name == "TodoWrite" {
                                    "📝"
                                } else {
                                    "🔧"
                                };
                                let simple = format!("{tool_icon} {tool_name}");

                                let detailed = if let Some(input) = obj.get("input") {
                                    let input_str = self.format_tool_input(input);
                                    format!("{tool_icon} {tool_name}: {input_str}")
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
                // Handle TodoWrite specially
                if let Some(todos) = obj.get("todos") {
                    return self.format_todos_input(todos);
                }

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

    /// Format TodoWrite todos input
    fn format_todos_input(&self, todos: &Value) -> String {
        self.format_todos_for_terminal(todos)
    }

    /// Format todos for terminal display
    fn format_todos_for_terminal(&self, todos: &Value) -> String {
        if let Value::Array(todo_array) = todos {
            let mut completed_count = 0;
            let mut pending_count = 0;
            let mut in_progress_count = 0;

            for todo in todo_array {
                if let Some(todo_obj) = todo.as_object() {
                    if let Some(status) = todo_obj.get("status").and_then(|s| s.as_str()) {
                        match status {
                            "completed" => completed_count += 1,
                            "in_progress" => in_progress_count += 1,
                            _ => pending_count += 1,
                        }
                    }
                }
            }

            let total = completed_count + pending_count + in_progress_count;

            match self.tool_display_mode {
                crate::ToolDisplayMode::Simple => {
                    let mut parts = Vec::new();
                    if pending_count > 0 {
                        parts.push(format!("{pending_count} pending"));
                    }
                    if in_progress_count > 0 {
                        parts.push(format!("{in_progress_count} in progress"));
                    }
                    if completed_count > 0 {
                        parts.push(format!("{completed_count} completed"));
                    }

                    if parts.is_empty() {
                        format!("{total} tasks")
                    } else {
                        format!("{total} tasks ({})", parts.join(", "))
                    }
                }
                crate::ToolDisplayMode::Detailed => {
                    let mut lines = Vec::new();

                    for todo in todo_array {
                        if let Some(todo_obj) = todo.as_object() {
                            let content = todo_obj
                                .get("content")
                                .and_then(|c| c.as_str())
                                .unwrap_or("Unknown task");
                            let status = todo_obj
                                .get("status")
                                .and_then(|s| s.as_str())
                                .unwrap_or("pending");
                            let priority = todo_obj
                                .get("priority")
                                .and_then(|p| p.as_str())
                                .unwrap_or("medium");

                            let checkbox = match status {
                                "completed" => "[x]",
                                "in_progress" => "[~]",
                                _ => "[ ]",
                            };

                            let priority_icon = match priority {
                                "high" => "🔴",
                                "low" => "🟢",
                                _ => "🟡",
                            };

                            lines.push(format!(
                                "  {} {} {} {}",
                                checkbox,
                                priority_icon,
                                content,
                                if status == "in_progress" {
                                    "(in progress)"
                                } else {
                                    ""
                                }
                            ));
                        }
                    }

                    format!("\n{}", lines.join("\n"))
                }
                _ => format!("{total} tasks"),
            }
        } else {
            "(...)".to_string()
        }
    }

    /// Format todos for Slack mrkdwn
    pub fn format_todos_for_slack(&self, todos: &Value) -> String {
        if let Value::Array(todo_array) = todos {
            let mut completed_count = 0;
            let mut pending_count = 0;
            let mut in_progress_count = 0;

            for todo in todo_array {
                if let Some(todo_obj) = todo.as_object() {
                    if let Some(status) = todo_obj.get("status").and_then(|s| s.as_str()) {
                        match status {
                            "completed" => completed_count += 1,
                            "in_progress" => in_progress_count += 1,
                            _ => pending_count += 1,
                        }
                    }
                }
            }

            let total = completed_count + pending_count + in_progress_count;

            match self.tool_display_mode {
                crate::ToolDisplayMode::Simple => {
                    let mut parts = Vec::new();
                    if pending_count > 0 {
                        parts.push(format!("{pending_count} pending"));
                    }
                    if in_progress_count > 0 {
                        parts.push(format!("{in_progress_count} in progress"));
                    }
                    if completed_count > 0 {
                        parts.push(format!("{completed_count} completed"));
                    }

                    if parts.is_empty() {
                        format!("{total} tasks")
                    } else {
                        format!("{total} tasks ({})", parts.join(", "))
                    }
                }
                crate::ToolDisplayMode::Detailed => {
                    let mut lines = Vec::new();

                    for todo in todo_array {
                        if let Some(todo_obj) = todo.as_object() {
                            let content = todo_obj
                                .get("content")
                                .and_then(|c| c.as_str())
                                .unwrap_or("Unknown task");
                            let status = todo_obj
                                .get("status")
                                .and_then(|s| s.as_str())
                                .unwrap_or("pending");
                            let priority = todo_obj
                                .get("priority")
                                .and_then(|p| p.as_str())
                                .unwrap_or("medium");

                            let status_emoji = match status {
                                "completed" => "✅",
                                "in_progress" => "⚠️",
                                _ => "⭕",
                            };

                            let priority_text = match priority {
                                "high" => " (high priority)",
                                "low" => " (low priority)",
                                _ => " (medium priority)",
                            };

                            let status_text = if status == "in_progress" {
                                " (in progress)"
                            } else {
                                ""
                            };
                            lines.push(format!(
                                "• {} *{}*{}{}",
                                status_emoji, content, status_text, priority_text
                            ));
                        }
                    }

                    format!("\n{}", lines.join("\n"))
                }
                _ => format!("{total} tasks"),
            }
        } else {
            "(...)".to_string()
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

    #[test]
    fn test_todowrite_simple_format() {
        let formatter = LogFormatter::new().with_tool_display_mode(crate::ToolDisplayMode::Simple);

        let todos_json = serde_json::json!([
            {
                "id": "1",
                "content": "Complete task 1",
                "status": "completed",
                "priority": "high"
            },
            {
                "id": "2",
                "content": "Work on task 2",
                "status": "in_progress",
                "priority": "medium"
            },
            {
                "id": "3",
                "content": "Start task 3",
                "status": "pending",
                "priority": "low"
            }
        ]);

        let result = formatter.format_todos_input(&todos_json);
        assert!(result.contains("3 tasks"));
        assert!(result.contains("1 pending"));
        assert!(result.contains("1 in progress"));
        assert!(result.contains("1 completed"));
    }

    #[test]
    fn test_todowrite_detailed_format() {
        let formatter =
            LogFormatter::new().with_tool_display_mode(crate::ToolDisplayMode::Detailed);

        let todos_json = serde_json::json!([
            {
                "id": "1",
                "content": "Complete task 1",
                "status": "completed",
                "priority": "high"
            },
            {
                "id": "2",
                "content": "Work on task 2",
                "status": "in_progress",
                "priority": "medium"
            },
            {
                "id": "3",
                "content": "Start task 3",
                "status": "pending",
                "priority": "low"
            }
        ]);

        let result = formatter.format_todos_input(&todos_json);
        assert!(result.contains("\n  [x] 🔴 Complete task 1"));
        assert!(result.contains("\n  [~] 🟡 Work on task 2 (in progress)"));
        assert!(result.contains("\n  [ ] 🟢 Start task 3"));
    }
}
