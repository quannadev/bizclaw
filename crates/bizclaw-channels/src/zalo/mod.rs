//! Zalo channel module — Zalo Personal + OA.
//! Wraps the client sub-modules into the Channel trait.
//! Now with real WebSocket listening, auto-reconnect, cookie health checks,
//! and Circuit Breaker protection for API calls.

pub mod client;
pub mod official;
pub mod personal;

use async_trait::async_trait;
use bizclaw_core::circuit_breaker::CircuitBreaker;
use bizclaw_core::config::ZaloChannelConfig;
use bizclaw_core::error::{BizClawError, Result};
use bizclaw_core::traits::Channel;
use bizclaw_core::types::{IncomingMessage, OutgoingMessage};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_stream::Stream;

use self::client::auth::{ZaloAuth, ZaloCredentials};
use self::client::listener::ZaloListener;
use self::client::messaging::{ThreadType as ZaloThreadType, ZaloMessaging};
use self::client::session::SessionManager;

/// Zalo channel implementation — routes to Personal or OA mode.
/// Now with real WebSocket listening and auto-reconnect.
pub struct ZaloChannel {
    config: ZaloChannelConfig,
    auth: ZaloAuth,
    messaging: ZaloMessaging,
    session: SessionManager,
    connected: bool,
    cookie: Option<String>,
    /// WebSocket URLs from login response
    ws_urls: Vec<String>,
    /// Own user ID (for self-message filtering)
    own_uid: String,
    /// Listener reference (kept alive for WebSocket connection)
    listener: Option<Arc<ZaloListener>>,
    /// Shared message receiver — wrapped in Mutex for safe access
    msg_receiver: Arc<Mutex<Option<tokio::sync::mpsc::Receiver<IncomingMessage>>>>,
    /// Circuit breaker — prevents cascading failures when Zalo API is down.
    circuit_breaker: CircuitBreaker,
}

impl ZaloChannel {
    pub fn new(config: ZaloChannelConfig) -> Self {
        let creds = ZaloCredentials {
            imei: config.personal.imei.clone(),
            cookie: None,
            phone: None,
            user_agent: if config.personal.user_agent.is_empty() {
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:135.0) Gecko/20100101 Firefox/135.0"
                    .into()
            } else {
                config.personal.user_agent.clone()
            },
            proxy: if config.personal.proxy.is_empty() {
                None
            } else {
                Some(config.personal.proxy.clone())
            },
        };

        let proxy_opt = if config.personal.proxy.is_empty() {
            None
        } else {
            Some(config.personal.proxy.clone())
        };

        Self {
            auth: ZaloAuth::new(creds),
            messaging: ZaloMessaging::with_proxy(proxy_opt),
            config,
            session: SessionManager::new(),
            connected: false,
            cookie: None,
            ws_urls: Vec::new(),
            own_uid: String::new(),
            listener: None,
            msg_receiver: Arc::new(Mutex::new(None)),
            circuit_breaker: CircuitBreaker::named("zalo", 5, std::time::Duration::from_secs(30)),
        }
    }

    /// Get circuit breaker reference for monitoring.
    pub fn circuit_breaker(&self) -> &CircuitBreaker {
        &self.circuit_breaker
    }

    /// Login with cookie from config or parameter.
    async fn login_cookie(&mut self, cookie: &str) -> Result<()> {
        let login_data = self.auth.login_with_cookie(cookie).await?;

        // Apply service map to messaging client (critical for correct API URLs)
        if let Some(ref map) = login_data.zpw_service_map_v3 {
            let service_map = client::messaging::ZaloServiceMap::from_login_data(map);
            self.messaging.set_service_map(service_map);
            tracing::info!("Zalo: service map applied from login response");
        }

        // Set login credentials
        self.messaging
            .set_login_info(&login_data.uid, login_data.zpw_enk.as_deref());

        // Store WebSocket URLs for listener
        if let Some(ref ws_urls) = login_data.zpw_ws {
            self.ws_urls = ws_urls.clone();
            tracing::info!("Zalo: got {} WebSocket URLs", self.ws_urls.len());
        }

        // Store own UID
        self.own_uid = login_data.uid.clone();

        self.session
            .set_session(
                login_data.uid.clone(),
                login_data.zpw_enk,
                login_data.zpw_key,
            )
            .await;
        self.cookie = Some(cookie.to_string());
        tracing::info!("Zalo logged in: uid={}", login_data.uid);
        Ok(())
    }

    /// Get QR code for login.
    pub async fn get_qr_code(&mut self) -> Result<client::auth::QrCodeResult> {
        self.auth.get_qr_code().await
    }

    /// Start WebSocket listener after login.
    fn start_ws_listener(&mut self) -> Result<()> {
        let ws_url = self
            .ws_urls
            .first()
            .ok_or_else(|| {
                BizClawError::Channel(
                    "No WebSocket URL from login. Zalo may have changed API.".into(),
                )
            })?
            .clone();

        let cookie = self.cookie.clone().unwrap_or_default();

        let webhook_url = if self.config.personal.webhook_url.is_empty() {
            None
        } else {
            Some(self.config.personal.webhook_url.clone())
        };

        let listener = ZaloListener::new(&ws_url)
            .with_reconnect(
                self.config.personal.auto_reconnect,
                self.config.personal.reconnect_delay_ms,
                0, // unlimited reconnect attempts
            )
            .with_own_uid(&self.own_uid)
            .with_self_listen(self.config.personal.self_listen)
            .with_webhook(webhook_url);

        let rx = listener.start_listening(cookie);

        self.listener = Some(Arc::new(listener));
        *self.msg_receiver.blocking_lock() = Some(rx);

        tracing::info!(
            "Zalo: WebSocket listener started (ws_url={}, auto_reconnect={})",
            ws_url,
            self.config.personal.auto_reconnect
        );

        Ok(())
    }
}

#[async_trait]
impl Channel for ZaloChannel {
    fn name(&self) -> &str {
        "zalo"
    }

    async fn connect(&mut self) -> Result<()> {
        tracing::info!("Zalo channel: connecting in {} mode...", self.config.mode);

        match self.config.mode.as_str() {
            "personal" => {
                tracing::warn!("⚠️  Zalo Personal API is unofficial. Use at your own risk.");

                // Try cookie login: from cookie_path file first, then raw cookie
                let cookie = self.try_load_cookie()?;
                if let Some(cookie) = cookie {
                    self.login_cookie(&cookie).await?;
                    self.connected = true;
                    tracing::info!("Zalo Personal: connected via cookie auth");

                    // Start WebSocket listener for receiving messages
                    if !self.ws_urls.is_empty() {
                        if let Err(e) = self.start_ws_listener() {
                            tracing::warn!(
                                "Zalo: WebSocket listener failed to start: {e}. \
                                 Messages will only be received via webhook/polling."
                            );
                        }
                    } else {
                        tracing::warn!(
                            "Zalo: No WebSocket URLs from login. \
                             Real-time message receiving unavailable."
                        );
                    }
                } else {
                    return Err(BizClawError::AuthFailed(
                        "No Zalo cookie found. Configure cookie_path in config.toml or use QR login via admin dashboard.".into()
                    ));
                }
            }
            "official" => {
                tracing::info!("Zalo OA: connecting via official API...");
                self.connected = true;
                tracing::info!("Zalo OA: connected (official API requires Zalo OA token)");
            }
            _ => {
                return Err(BizClawError::Config(format!(
                    "Unknown Zalo mode: {}",
                    self.config.mode
                )));
            }
        }
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.session.invalidate().await;
        self.connected = false;
        self.listener = None; // Drop listener stops WebSocket
        tracing::info!("Zalo channel: disconnected");
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    async fn listen(&self) -> Result<Box<dyn Stream<Item = IncomingMessage> + Send + Unpin>> {
        // Take the receiver from the shared slot
        let mut guard = self.msg_receiver.lock().await;
        if let Some(rx) = guard.take() {
            tracing::info!(
                "Zalo listener: active (WebSocket real-time mode, uid={})",
                self.own_uid
            );
            Ok(Box::new(tokio_stream::wrappers::ReceiverStream::new(rx)))
        } else {
            // Fallback: if no WebSocket available (e.g., OA mode), use pending stream
            tracing::info!(
                "Zalo listener: no WebSocket receiver available, using webhook/polling fallback"
            );
            Ok(Box::new(futures::stream::pending::<IncomingMessage>()))
        }
    }

    async fn send(&self, message: OutgoingMessage) -> Result<()> {
        if !self.circuit_breaker.can_execute() {
            return Err(BizClawError::Channel(
                "Zalo circuit breaker Open — message rejected".into(),
            ));
        }

        let cookie = self
            .cookie
            .as_ref()
            .ok_or_else(|| BizClawError::Channel("Zalo not logged in".into()))?;

        match self
            .messaging
            .send_text(
                &message.thread_id,
                ZaloThreadType::User,
                &message.content,
                cookie,
            )
            .await
        {
            Ok(_msg_id) => {
                self.circuit_breaker.record_success();
                tracing::debug!("Zalo: message sent to {}", message.thread_id);
                Ok(())
            }
            Err(e) => {
                self.circuit_breaker.record_failure();
                tracing::error!(
                    "Zalo send failed: {e} (CB: {})",
                    self.circuit_breaker.summary()
                );
                Err(e)
            }
        }
    }

    async fn send_typing(&self, thread_id: &str) -> Result<()> {
        tracing::debug!(
            "Zalo: typing indicator to {} (not supported by API)",
            thread_id
        );
        Ok(())
    }
}

impl ZaloChannel {
    /// Try to load cookie from cookie_path file.
    fn try_load_cookie(&self) -> Result<Option<String>> {
        let path = &self.config.personal.cookie_path;
        if path.is_empty() {
            return Ok(None);
        }

        // Expand ~ to home dir
        let expanded = if path.starts_with("~/") {
            std::env::var("HOME")
                .ok()
                .map(|h| std::path::PathBuf::from(h).join(&path[2..]))
                .unwrap_or_else(|| std::path::PathBuf::from(path))
        } else {
            std::path::PathBuf::from(path)
        };

        if expanded.exists() {
            let content = std::fs::read_to_string(&expanded)
                .map_err(|e| BizClawError::Config(format!("Failed to read cookie file: {e}")))?;

            let trimmed = content.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }

            // Support JSON format {"cookie": "..."} or raw cookie string
            if trimmed.starts_with('{')
                && let Ok(json) = serde_json::from_str::<serde_json::Value>(trimmed)
                && let Some(cookie) = json["cookie"].as_str()
            {
                return Ok(Some(cookie.to_string()));
            }

            Ok(Some(trimmed.to_string()))
        } else {
            Ok(None)
        }
    }

    /// Check cookie health — returns true if cookie is still valid.
    pub async fn check_cookie_health(&self) -> bool {
        if let Some(ref cookie) = self.cookie {
            match self.auth.login_with_cookie(cookie).await {
                Ok(_) => true,
                Err(e) => {
                    tracing::warn!("Zalo cookie health check failed: {e}");
                    false
                }
            }
        } else {
            false
        }
    }

    /// Get current connection info for debugging/dashboard.
    pub fn connection_info(&self) -> serde_json::Value {
        serde_json::json!({
            "connected": self.connected,
            "mode": self.config.mode,
            "uid": self.own_uid,
            "ws_urls": self.ws_urls,
            "has_cookie": self.cookie.is_some(),
            "has_listener": self.listener.is_some(),
            "auto_reconnect": self.config.personal.auto_reconnect,
            "service_info": self.messaging.service_info(),
        })
    }
}
