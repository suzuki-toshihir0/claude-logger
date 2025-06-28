use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;
use url::Url;

use crate::parser::LogMessage;
use crate::WebhookFormat;

pub struct WebhookSender {
    client: Client,
    url: Url,
    format: WebhookFormat,
}

impl WebhookSender {
    pub fn new(url: Url, format: WebhookFormat) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            url,
            format,
        })
    }

    /// Send message to webhook
    pub async fn send_message(&self, message: &LogMessage, formatted_content: &str) -> Result<()> {
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

        Ok(())
    }

    /// Format message according to webhook format
    fn format_message(&self, message: &LogMessage, formatted_content: &str) -> Result<Value> {
        match self.format {
            WebhookFormat::Generic => self.format_generic(message, formatted_content),
            WebhookFormat::Slack => self.format_slack(message, formatted_content),
            WebhookFormat::Discord => self.format_discord(message, formatted_content),
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
        let role_icon = match message.role {
            crate::parser::MessageRole::User => ":bust_in_silhouette:",
            crate::parser::MessageRole::Assistant => ":robot_face:",
            crate::parser::MessageRole::System => ":gear:",
        };

        let role_name = match message.role {
            crate::parser::MessageRole::User => "User",
            crate::parser::MessageRole::Assistant => "Claude",
            crate::parser::MessageRole::System => "System",
        };

        let text = format!("{} *{}*\n{}", role_icon, role_name, formatted_content);

        Ok(json!({
            "text": text,
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

    /// Discord webhook format
    fn format_discord(&self, message: &LogMessage, formatted_content: &str) -> Result<Value> {
        let role_name = match message.role {
            crate::parser::MessageRole::User => "User",
            crate::parser::MessageRole::Assistant => "Claude",
            crate::parser::MessageRole::System => "System",
        };

        let color = match message.role {
            crate::parser::MessageRole::User => 0x0099ff,     // Blue
            crate::parser::MessageRole::Assistant => 0x00ff99, // Green
            crate::parser::MessageRole::System => 0xff9900,    // Orange
        };

        Ok(json!({
            "embeds": [
                {
                    "title": role_name,
                    "description": formatted_content,
                    "color": color,
                    "timestamp": message.timestamp.to_rfc3339()
                }
            ]
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use crate::parser::MessageRole;

    fn create_test_message() -> LogMessage {
        LogMessage {
            role: MessageRole::User,
            content: "Test message".to_string(),
            timestamp: Utc::now(),
            session_id: "test-session-12345".to_string(),
            uuid: "test-uuid".to_string(),
            raw_content: None,
        }
    }

    #[test]
    fn test_generic_format() {
        let url = Url::parse("https://example.com/webhook").unwrap();
        let sender = WebhookSender::new(url, WebhookFormat::Generic).unwrap();
        let message = create_test_message();
        
        let result = sender.format_generic(&message, "Formatted content").unwrap();
        
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

    #[test]
    fn test_discord_format() {
        let url = Url::parse("https://example.com/webhook").unwrap();
        let sender = WebhookSender::new(url, WebhookFormat::Discord).unwrap();
        let message = create_test_message();
        
        let result = sender.format_discord(&message, "Formatted content").unwrap();
        
        assert!(result.get("embeds").is_some());
    }
}