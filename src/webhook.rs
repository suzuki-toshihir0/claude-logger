use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;
use url::Url;

use crate::parser::LogMessage;
use crate::formatter::LogFormatter;
use crate::WebhookFormat;

#[derive(Debug)]
pub enum WebhookResult {
    Sent,
    Skipped,
}

pub struct WebhookSender {
    client: Client,
    url: Url,
    format: WebhookFormat,
    formatter: LogFormatter,
}

impl WebhookSender {
    pub fn new(url: Url, format: WebhookFormat) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .context("Failed to create HTTP client")?;

        let formatter = LogFormatter::new()
            .with_tool_display_mode(crate::ToolDisplayMode::Detailed);

        Ok(Self {
            client,
            url,
            format,
            formatter,
        })
    }

    /// Send message to webhook
    pub async fn send_message(
        &self,
        message: &LogMessage,
        formatted_content: &str,
    ) -> Result<WebhookResult> {
        // Skip low-information messages for webhook (but not for stdout)
        if self.is_low_information_message_for_webhook(message) {
            return Ok(WebhookResult::Skipped);
        }

        let payload = self.format_message(message, formatted_content)?;

        let response = self
            .client
            .post(self.url.clone())
            .json(&payload)
            .send()
            .await
            .context("Failed to send webhook request")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Webhook request failed with status: {}",
                response.status()
            ));
        }

        Ok(WebhookResult::Sent)
    }

    /// Check if this message should be filtered out for webhook posting
    /// (but still shown in stdout)
    fn is_low_information_message_for_webhook(&self, message: &LogMessage) -> bool {
        let Some(ref raw_content) = message.raw_content else {
            return false;
        };

        let serde_json::Value::Array(arr) = raw_content else {
            return false;
        };

        // If message contains meaningful text content, send it
        let has_text = arr.iter().filter_map(|item| item.as_object()).any(|obj| {
            obj.get("type")
                .and_then(|t| t.as_str())
                .filter(|&t| t == "text")
                .and_then(|_| obj.get("text"))
                .and_then(|text| text.as_str())
                .map(|text| !text.trim().is_empty())
                .unwrap_or(false)
        });

        if has_text {
            return false;
        }

        // Filter out messages with only tool_result entries (User: Result pattern)
        let only_tool_results = arr
            .iter()
            .filter_map(|item| item.as_object())
            .filter_map(|obj| obj.get("type").and_then(|t| t.as_str()))
            .all(|content_type| content_type == "tool_result");

        // Filter out messages with only Read/Edit tool_use entries (Claude: Read/Edit pattern)
        let only_read_edit_tools = arr
            .iter()
            .filter_map(|item| item.as_object())
            .filter_map(|obj| obj.get("type").and_then(|t| t.as_str()))
            .filter(|&t| t == "tool_use")
            .all(|_| {
                arr.iter()
                    .filter_map(|item| item.as_object())
                    .filter_map(|obj| obj.get("name").and_then(|n| n.as_str()))
                    .all(|name| name == "Read" || name == "Edit")
            });

        only_tool_results || only_read_edit_tools
    }

    /// Format message according to webhook format
    fn format_message(&self, message: &LogMessage, formatted_content: &str) -> Result<Value> {
        match self.format {
            WebhookFormat::Generic => self.format_generic(message, formatted_content),
            WebhookFormat::Slack => {
                let slack_content = self.format_content_for_slack(message);
                self.format_slack(message, &slack_content)
            }
        }
    }

    /// Format message content specifically for Slack
    fn format_content_for_slack(&self, message: &LogMessage) -> String {
        let Some(ref raw_content) = message.raw_content else {
            return message.content.clone();
        };

        let Value::Array(arr) = raw_content else {
            return message.content.clone();
        };

        for item in arr {
            let Some(obj) = item.as_object() else { continue };
            let Some(content_type) = obj.get("type").and_then(|t| t.as_str()) else { continue };

            if content_type == "tool_use" {
                let Some(tool_name) = obj.get("name").and_then(|n| n.as_str()) else { continue };
                
                if tool_name == "TodoWrite" {
                    let Some(input) = obj.get("input") else { continue };
                    let Some(todos) = input.get("todos") else { continue };
                    
                    let slack_todos = self.formatter.format_todos_for_slack(todos);
                    return format!("📝 TodoWrite: {}", slack_todos);
                }
                
                // Handle other tools with generic format
                let tool_icon = if tool_name == "TodoWrite" { "📝" } else { "🔧" };
                if let Some(input) = obj.get("input") {
                    let input_str = self.format_tool_input_for_slack(input);
                    return format!("{} {}: {}", tool_icon, tool_name, input_str);
                } else {
                    return format!("{} {}", tool_icon, tool_name);
                }
            }
        }

        message.content.clone()
    }

    /// Format tool input for Slack (simpler than terminal version)
    fn format_tool_input_for_slack(&self, input: &Value) -> String {
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

    /// Generic JSON format
    fn format_generic(&self, message: &LogMessage, formatted_content: &str) -> Result<Value> {
        Ok(json!({
            "timestamp": message.timestamp.to_rfc3339(),
            "role": format!("{:?}", message.role),
            "content": formatted_content,
            "session_id": message.session_id,
            "uuid": message.uuid
        }))
    }

    /// Slack webhook format
    fn format_slack(&self, message: &LogMessage, formatted_content: &str) -> Result<Value> {
        let session_short = &message.session_id[..8.min(message.session_id.len())];
        let username = format!("Claude Code / {} | {}", message.project_name, session_short);
        let text = formatted_content.to_string();

        Ok(json!({
            "text": text,
            "username": username,
            "blocks": [
                {
                    "type": "section",
                    "text": {
                        "type": "mrkdwn",
                        "text": text
                    }
                }
            ]
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::MessageRole;
    use chrono::Utc;

    fn create_test_message() -> LogMessage {
        LogMessage {
            role: MessageRole::User,
            content: "Test message".to_string(),
            timestamp: Utc::now(),
            session_id: "test-session-12345".to_string(),
            uuid: "test-uuid".to_string(),
            project_name: "test-project".to_string(),
            raw_content: None,
        }
    }

    #[test]
    fn test_generic_format() {
        let url = Url::parse("https://example.com/webhook").unwrap();
        let sender = WebhookSender::new(url, WebhookFormat::Generic).unwrap();
        let message = create_test_message();

        let result = sender
            .format_generic(&message, "Formatted content")
            .unwrap();

        assert!(result.get("content").is_some());
        assert!(result.get("role").is_some());
        assert!(result.get("timestamp").is_some());
    }

    #[test]
    fn test_slack_format() {
        let url = Url::parse("https://example.com/webhook").unwrap();
        let sender = WebhookSender::new(url, WebhookFormat::Slack).unwrap();
        let message = create_test_message();

        let result = sender.format_slack(&message, "Formatted content").unwrap();

        assert!(result.get("text").is_some());
        assert!(result.get("blocks").is_some());
    }
}
