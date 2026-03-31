//! Social Posting Tool — AI Agent tool for publishing content to social media.
//!
//! Supports: Facebook Pages, Telegram Channels, custom webhooks.
//! Actions: create_post, schedule_post, list_scheduled, cancel_scheduled.

use async_trait::async_trait;
use bizclaw_core::error::Result;
use bizclaw_core::traits::Tool;
use bizclaw_core::types::{ToolDefinition, ToolResult};
use serde::{Deserialize, Serialize};

/// Safely truncate a string at a character boundary (UTF-8 safe).
fn truncate_safe(s: &str, max_chars: usize) -> String {
    let truncated: String = s.chars().take(max_chars).collect();
    if truncated.len() < s.len() {
        format!("{}...", truncated)
    } else {
        truncated
    }
}

/// Social Posting Tool for AI agents.
pub struct SocialPostTool {
    client: reqwest::Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PostRequest {
    #[serde(default)]
    action: String,
    #[serde(default)]
    platform: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    image_url: String,
    #[serde(default)]
    link: String,
    #[serde(default)]
    schedule_at: String,
    // Platform-specific credentials (from agent config)
    #[serde(default)]
    access_token: String,
    #[serde(default)]
    page_id: String,
    #[serde(default)]
    chat_id: String,
    #[serde(default)]
    bot_token: String,
    #[serde(default)]
    webhook_url: String,
}

impl SocialPostTool {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    /// Post to Facebook Page via Graph API.
    async fn post_facebook(&self, req: &PostRequest) -> String {
        if req.access_token.is_empty() || req.page_id.is_empty() {
            return "❌ Thiếu access_token hoặc page_id cho Facebook. Cấu hình trong agent config."
                .into();
        }

        let url = format!("https://graph.facebook.com/v21.0/{}/feed", req.page_id);

        let mut params = vec![
            ("message", req.content.as_str()),
            ("access_token", req.access_token.as_str()),
        ];
        if !req.link.is_empty() {
            params.push(("link", req.link.as_str()));
        }

        match self.client.post(&url).form(&params).send().await {
            Ok(resp) => {
                let status = resp.status();
                match resp.json::<serde_json::Value>().await {
                    Ok(body) => {
                        if status.is_success() {
                            let post_id = body["id"].as_str().unwrap_or("unknown");
                            format!(
                                "✅ Đã đăng bài lên Facebook Page!\n\
                                 • Post ID: {}\n\
                                 • URL: https://facebook.com/{}\n\
                                 • Nội dung: {}",
                                post_id,
                                post_id,
                                truncate_safe(&req.content, 100)
                            )
                        } else {
                            let err = body["error"]["message"].as_str().unwrap_or("Unknown error");
                            format!("❌ Facebook API error: {}", err)
                        }
                    }
                    Err(e) => format!("❌ Facebook response parse error: {}", e),
                }
            }
            Err(e) => format!("❌ Facebook request failed: {}", e),
        }
    }

    /// Post to Telegram Channel via Bot API.
    async fn post_telegram(&self, req: &PostRequest) -> String {
        if req.bot_token.is_empty() || req.chat_id.is_empty() {
            return "❌ Thiếu bot_token hoặc chat_id cho Telegram.".into();
        }

        let url = format!("https://api.telegram.org/bot{}/sendMessage", req.bot_token);

        let body = serde_json::json!({
            "chat_id": req.chat_id,
            "text": req.content,
            "parse_mode": "HTML",
            "disable_web_page_preview": req.link.is_empty(),
        });

        match self.client.post(&url).json(&body).send().await {
            Ok(resp) => {
                let status = resp.status();
                match resp.json::<serde_json::Value>().await {
                    Ok(result) => {
                        if status.is_success() && result["ok"].as_bool() == Some(true) {
                            let msg_id = result["result"]["message_id"].as_i64().unwrap_or(0);
                            format!(
                                "✅ Đã đăng bài lên Telegram Channel!\n\
                                 • Message ID: {}\n\
                                 • Channel: {}\n\
                                 • Nội dung: {}",
                                msg_id,
                                req.chat_id,
                                truncate_safe(&req.content, 100)
                            )
                        } else {
                            let err = result["description"].as_str().unwrap_or("Unknown error");
                            format!("❌ Telegram API error: {}", err)
                        }
                    }
                    Err(e) => format!("❌ Telegram response parse error: {}", e),
                }
            }
            Err(e) => format!("❌ Telegram request failed: {}", e),
        }
    }

    /// Post via custom webhook (Slack, Discord, Mattermost, etc.)
    async fn post_webhook(&self, req: &PostRequest) -> String {
        if req.webhook_url.is_empty() {
            return "❌ Thiếu webhook_url.".into();
        }

        let body = serde_json::json!({
            "text": req.content,
            "content": req.content,  // Discord format
        });

        match self.client.post(&req.webhook_url).json(&body).send().await {
            Ok(resp) => {
                if resp.status().is_success() || resp.status().as_u16() == 204 {
                    format!(
                        "✅ Đã đăng bài qua webhook!\n\
                         • Platform: {}\n\
                         • Nội dung: {}",
                        req.platform,
                        truncate_safe(&req.content, 100)
                    )
                } else {
                    format!("❌ Webhook error: HTTP {}", resp.status())
                }
            }
            Err(e) => format!("❌ Webhook request failed: {}", e),
        }
    }

    /// Generate a content schedule suggestion.
    fn suggest_schedule(&self, content: &str) -> String {
        let now = chrono::Local::now();
        let suggestions = vec![
            (now + chrono::Duration::hours(1), "Giờ tiếp theo"),
            (
                now + chrono::Duration::hours(3),
                "Khung giờ vàng buổi chiều",
            ),
            (now + chrono::Duration::days(1), "Sáng mai"),
        ];

        let mut result = format!(
            "📅 Gợi ý lịch đăng bài:\n\
             📝 Nội dung: \"{}\"\n\n",
            truncate_safe(content, 80)
        );

        for (time, label) in &suggestions {
            result.push_str(&format!(
                "  • {} — {}\n",
                time.format("%Y-%m-%d %H:%M"),
                label
            ));
        }

        result.push_str(
            "\n💡 Để đăng ngay, dùng action='create_post'.\n\
             💡 Để hẹn giờ, dùng action='schedule_post' + schedule_at='YYYY-MM-DD HH:MM'.",
        );

        result
    }
}

impl Default for SocialPostTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for SocialPostTool {
    fn name(&self) -> &str {
        "social_post"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "social_post".into(),
            description: "Công cụ đăng bài lên mạng xã hội — Facebook Page, Telegram Channel, \
                           webhook (Slack/Discord/Mattermost). Hỗ trợ đăng ngay hoặc gợi ý lịch đăng."
                .into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["create_post", "schedule_suggest", "list_platforms"],
                        "description": "Hành động:\n\
                            • create_post — Đăng bài ngay lập tức\n\
                            • schedule_suggest — Gợi ý thời gian đăng bài\n\
                            • list_platforms — Liệt kê các nền tảng hỗ trợ"
                    },
                    "platform": {
                        "type": "string",
                        "enum": ["facebook", "telegram", "webhook"],
                        "description": "Nền tảng đăng bài"
                    },
                    "content": {
                        "type": "string",
                        "description": "Nội dung bài đăng"
                    },
                    "image_url": {
                        "type": "string",
                        "description": "URL hình ảnh đính kèm (tuỳ chọn)"
                    },
                    "link": {
                        "type": "string",
                        "description": "URL liên kết đính kèm (tuỳ chọn)"
                    },
                    "access_token": {
                        "type": "string",
                        "description": "Facebook Page Access Token"
                    },
                    "page_id": {
                        "type": "string",
                        "description": "Facebook Page ID"
                    },
                    "bot_token": {
                        "type": "string",
                        "description": "Telegram Bot Token (cho channel posting)"
                    },
                    "chat_id": {
                        "type": "string",
                        "description": "Telegram Channel ID (VD: @mychannel hoặc -100xxxx)"
                    },
                    "webhook_url": {
                        "type": "string",
                        "description": "Webhook URL cho Slack/Discord/custom"
                    }
                },
                "required": ["action"]
            }),
        }
    }

    async fn execute(&self, args: &str) -> Result<ToolResult> {
        let req: PostRequest = serde_json::from_str(args).unwrap_or_else(|_| PostRequest {
            action: "list_platforms".into(),
            platform: String::new(),
            content: String::new(),
            image_url: String::new(),
            link: String::new(),
            schedule_at: String::new(),
            access_token: String::new(),
            page_id: String::new(),
            chat_id: String::new(),
            bot_token: String::new(),
            webhook_url: String::new(),
        });

        let output = match req.action.as_str() {
            "create_post" => {
                if req.content.is_empty() {
                    "❌ Thiếu nội dung bài đăng (content)".into()
                } else {
                    match req.platform.as_str() {
                        "facebook" => self.post_facebook(&req).await,
                        "telegram" => self.post_telegram(&req).await,
                        "webhook" => self.post_webhook(&req).await,
                        "" => "❌ Chưa chọn platform (facebook/telegram/webhook)".into(),
                        other => format!(
                            "❌ Platform '{}' chưa hỗ trợ. Dùng: facebook, telegram, webhook",
                            other
                        ),
                    }
                }
            }
            "schedule_suggest" => self.suggest_schedule(&req.content),
            _ => "📱 Nền tảng hỗ trợ đăng bài:\n\n\
                 1. 📘 **Facebook Page** — Cần: page_id + access_token\n\
                    • Đăng bài text, link, ảnh lên Page\n\
                    • Lấy access_token tại: developers.facebook.com\n\n\
                 2. 📨 **Telegram Channel** — Cần: bot_token + chat_id\n\
                    • Đăng bài text/HTML lên Channel\n\
                    • Tạo bot tại: @BotFather\n\
                    • chat_id: @your_channel hoặc -100xxxx\n\n\
                 3. 🔗 **Webhook** — Cần: webhook_url\n\
                    • Slack Incoming Webhook\n\
                    • Discord Webhook\n\
                    • Mattermost Webhook\n\
                    • Custom endpoint\n\n\
                 💡 Ví dụ: action='create_post', platform='telegram', \
                    content='Hello world!', bot_token='xxx', chat_id='@mychannel'"
                .into(),
        };

        Ok(ToolResult {
            tool_call_id: String::new(),
            output,
            success: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_name() {
        let tool = SocialPostTool::new();
        assert_eq!(tool.name(), "social_post");
    }

    #[test]
    fn test_tool_definition() {
        let tool = SocialPostTool::new();
        let def = tool.definition();
        assert_eq!(def.name, "social_post");
        assert!(def.description.contains("Facebook"));
        assert!(def.description.contains("Telegram"));
        let params = def.parameters;
        let props = params["properties"].as_object().unwrap();
        assert!(props.contains_key("action"));
        assert!(props.contains_key("platform"));
        assert!(props.contains_key("content"));
    }

    #[tokio::test]
    async fn test_list_platforms() {
        let tool = SocialPostTool::new();
        let result = tool
            .execute(r#"{"action": "list_platforms"}"#)
            .await
            .unwrap();
        assert!(result.output.contains("Facebook"));
        assert!(result.output.contains("Telegram"));
        assert!(result.output.contains("Webhook"));
    }

    #[tokio::test]
    async fn test_missing_content() {
        let tool = SocialPostTool::new();
        let result = tool
            .execute(r#"{"action": "create_post", "platform": "facebook"}"#)
            .await
            .unwrap();
        assert!(result.output.contains("Thiếu nội dung"));
    }

    #[tokio::test]
    async fn test_missing_platform() {
        let tool = SocialPostTool::new();
        let result = tool
            .execute(r#"{"action": "create_post", "content": "Hello"}"#)
            .await
            .unwrap();
        assert!(result.output.contains("Chưa chọn platform"));
    }

    #[tokio::test]
    async fn test_schedule_suggest() {
        let tool = SocialPostTool::new();
        let result = tool
            .execute(r#"{"action": "schedule_suggest", "content": "Bài test"}"#)
            .await
            .unwrap();
        assert!(result.output.contains("Gợi ý lịch đăng"));
    }

    #[tokio::test]
    async fn test_facebook_missing_token() {
        let tool = SocialPostTool::new();
        let result = tool
            .execute(r#"{"action": "create_post", "platform": "facebook", "content": "test"}"#)
            .await
            .unwrap();
        assert!(result.output.contains("Thiếu access_token"));
    }

    #[tokio::test]
    async fn test_telegram_missing_token() {
        let tool = SocialPostTool::new();
        let result = tool
            .execute(r#"{"action": "create_post", "platform": "telegram", "content": "test"}"#)
            .await
            .unwrap();
        assert!(result.output.contains("Thiếu bot_token"));
    }
}
