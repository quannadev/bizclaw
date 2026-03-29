//! Zalo Official Account mode — uses Zalo OA REST API.
//!
//! For business accounts via developers.zalo.me.
//! Supports: send messages, broadcast, manage followers, template messages,
//! rich media (images, file, sticker), webhook handling.

use async_trait::async_trait;
use bizclaw_core::error::{BizClawError, Result};
use bizclaw_core::traits::Channel;
use bizclaw_core::types::{IncomingMessage, OutgoingMessage};
use futures::stream::{self, Stream};

use super::client::business::ZaloBusiness;

/// Zalo OA channel — uses official API access token.
pub struct ZaloOfficialChannel {
    business: ZaloBusiness,
    access_token: Option<String>,
    refresh_token: Option<String>,
    oa_id: Option<String>,
    connected: bool,
}

impl ZaloOfficialChannel {
    pub fn new() -> Self {
        Self {
            business: ZaloBusiness::new(),
            access_token: None,
            refresh_token: None,
            oa_id: None,
            connected: false,
        }
    }

    /// Set access token from OA developer portal.
    pub fn set_access_token(&mut self, token: &str) {
        self.access_token = Some(token.to_string());
        self.connected = true;
    }

    /// Set refresh token for auto-renewal.
    pub fn set_refresh_token(&mut self, token: &str) {
        self.refresh_token = Some(token.to_string());
    }

    /// Set OA ID for identification.
    pub fn set_oa_id(&mut self, oa_id: &str) {
        self.oa_id = Some(oa_id.to_string());
    }

    /// Refresh access token using refresh token (Zalo OA API v3).
    pub async fn refresh_access_token(&mut self, app_id: &str, app_secret: &str) -> Result<String> {
        let refresh = self
            .refresh_token
            .as_ref()
            .ok_or_else(|| BizClawError::AuthFailed("No refresh token".into()))?;

        let client = reqwest::Client::new();
        let res = client
            .post("https://oauth.zaloapp.com/v4/oa/access_token")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("secret_key", app_secret)
            .form(&[
                ("app_id", app_id),
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh.as_str()),
            ])
            .send()
            .await
            .map_err(|e| BizClawError::Channel(format!("Token refresh failed: {e}")))?;

        let body: serde_json::Value = res
            .json()
            .await
            .map_err(|e| BizClawError::Channel(format!("Token parse failed: {e}")))?;

        if let Some(token) = body.get("access_token").and_then(|v| v.as_str()) {
            self.access_token = Some(token.to_string());
            if let Some(new_refresh) = body.get("refresh_token").and_then(|v| v.as_str()) {
                self.refresh_token = Some(new_refresh.to_string());
            }
            tracing::info!("[zalo-oa] Access token refreshed successfully");
            Ok(token.to_string())
        } else {
            let err = body
                .get("error_description")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error");
            Err(BizClawError::AuthFailed(format!("Token refresh: {err}")))
        }
    }

    /// Send a template message (transaction notification, etc).
    pub async fn send_template_message(
        &self,
        user_id: &str,
        template_id: &str,
        template_data: serde_json::Value,
    ) -> Result<()> {
        let token = self
            .access_token
            .as_ref()
            .ok_or_else(|| BizClawError::Channel("No access token".into()))?;

        let payload = serde_json::json!({
            "recipient": { "user_id": user_id },
            "message": {
                "attachment": {
                    "type": "template",
                    "payload": {
                        "template_type": "promotion",
                        "template_id": template_id,
                        "elements": [template_data]
                    }
                }
            }
        });

        let client = reqwest::Client::new();
        let res = client
            .post("https://openapi.zalo.me/v3.0/oa/message/cs")
            .header("access_token", token)
            .json(&payload)
            .send()
            .await
            .map_err(|e| BizClawError::Channel(format!("Template send failed: {e}")))?;

        let body: serde_json::Value = res
            .json()
            .await
            .map_err(|e| BizClawError::Channel(format!("Response parse: {e}")))?;

        if body.get("error").and_then(|v| v.as_i64()).unwrap_or(-1) == 0 {
            tracing::info!("[zalo-oa] Template message sent to {user_id}");
            Ok(())
        } else {
            let msg = body
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error");
            Err(BizClawError::Channel(format!("Template send: {msg}")))
        }
    }

    /// Get list of OA followers with pagination.
    pub async fn get_followers(&self, offset: u32, count: u32) -> Result<serde_json::Value> {
        let token = self
            .access_token
            .as_ref()
            .ok_or_else(|| BizClawError::Channel("No access token".into()))?;

        let client = reqwest::Client::new();
        let res = client
            .get("https://openapi.zalo.me/v2.0/oa/getfollowers")
            .header("access_token", token)
            .query(&[(
                "data",
                serde_json::json!({"offset": offset, "count": count}).to_string(),
            )])
            .send()
            .await
            .map_err(|e| BizClawError::Channel(format!("Get followers failed: {e}")))?;

        res.json()
            .await
            .map_err(|e| BizClawError::Channel(format!("Parse followers: {e}")))
    }

    /// Send image message via OA.
    pub async fn send_image(&self, user_id: &str, image_url: &str, caption: &str) -> Result<()> {
        let token = self
            .access_token
            .as_ref()
            .ok_or_else(|| BizClawError::Channel("No access token".into()))?;

        let payload = serde_json::json!({
            "recipient": { "user_id": user_id },
            "message": {
                "text": caption,
                "attachment": {
                    "type": "template",
                    "payload": {
                        "template_type": "media",
                        "elements": [{
                            "media_type": "image",
                            "url": image_url
                        }]
                    }
                }
            }
        });

        let client = reqwest::Client::new();
        client
            .post("https://openapi.zalo.me/v3.0/oa/message/cs")
            .header("access_token", token)
            .json(&payload)
            .send()
            .await
            .map_err(|e| BizClawError::Channel(format!("Image send failed: {e}")))?;

        tracing::info!("[zalo-oa] Image sent to {user_id}");
        Ok(())
    }

    /// Broadcast message to all followers (requires ZCA approval).
    pub async fn broadcast(&self, message: &str) -> Result<serde_json::Value> {
        let token = self
            .access_token
            .as_ref()
            .ok_or_else(|| BizClawError::Channel("No access token".into()))?;

        let payload = serde_json::json!({
            "recipient": { "target": { "gender": 0 } },
            "message": { "text": message }
        });

        let client = reqwest::Client::new();
        let res = client
            .post("https://openapi.zalo.me/v2.0/oa/message/broadcast")
            .header("access_token", token)
            .json(&payload)
            .send()
            .await
            .map_err(|e| BizClawError::Channel(format!("Broadcast failed: {e}")))?;

        res.json()
            .await
            .map_err(|e| BizClawError::Channel(format!("Broadcast parse: {e}")))
    }

    /// Get OA profile info.
    pub async fn get_oa_info(&self) -> Result<serde_json::Value> {
        let token = self
            .access_token
            .as_ref()
            .ok_or_else(|| BizClawError::Channel("No access token".into()))?;

        let client = reqwest::Client::new();
        let res = client
            .get("https://openapi.zalo.me/v2.0/oa/getoa")
            .header("access_token", token)
            .send()
            .await
            .map_err(|e| BizClawError::Channel(format!("Get OA info failed: {e}")))?;

        res.json()
            .await
            .map_err(|e| BizClawError::Channel(format!("Parse OA info: {e}")))
    }
}

#[async_trait]
impl Channel for ZaloOfficialChannel {
    fn name(&self) -> &str {
        "zalo-oa"
    }

    async fn connect(&mut self) -> Result<()> {
        if self.access_token.is_none() {
            return Err(BizClawError::AuthFailed("Set access_token first".into()));
        }
        tracing::info!("[zalo-oa] Channel connected (OA: {:?})", self.oa_id);
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.connected = false;
        tracing::info!("[zalo-oa] Channel disconnected");
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    async fn send(&self, message: OutgoingMessage) -> Result<()> {
        let token = self
            .access_token
            .as_ref()
            .ok_or_else(|| BizClawError::Channel("No access token".into()))?;
        self.business
            .send_oa_message(&message.thread_id, &message.content, token)
            .await
    }

    async fn listen(&self) -> Result<Box<dyn Stream<Item = IncomingMessage> + Send + Unpin>> {
        // Webhook-based: messages come via POST /api/v1/zalo/webhook
        Ok(Box::new(stream::pending()))
    }
}

impl Default for ZaloOfficialChannel {
    fn default() -> Self {
        Self::new()
    }
}
