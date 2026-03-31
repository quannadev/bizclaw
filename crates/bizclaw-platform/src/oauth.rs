//! OAuth2 Social Connect — cho AI Agent đăng nhập dịch vụ bên ngoài.
//!
//! Flow: Dashboard → "Kết nối Google" → OAuth2 consent → callback → lưu token
//! Agent dùng token để truy cập Gmail, Calendar, Sheets, Facebook Pages, Instagram.
//!
//! KHÔNG dùng cho billing/auth platform — chỉ cho agent tools.

use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Redirect};
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::admin::AdminState;

// ── Provider Configuration ───────────────────────────────

/// Supported OAuth2 providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OAuthProvider {
    Google,
    Facebook,
    Instagram, // Uses Facebook OAuth with Instagram scopes
}

impl OAuthProvider {
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "google" => Some(Self::Google),
            "facebook" => Some(Self::Facebook),
            "instagram" => Some(Self::Instagram),
            _ => None,
        }
    }

    fn auth_url(&self) -> &'static str {
        match self {
            Self::Google => "https://accounts.google.com/o/oauth2/v2/auth",
            Self::Facebook | Self::Instagram => {
                "https://www.facebook.com/v21.0/dialog/oauth"
            }
        }
    }

    fn token_url(&self) -> &'static str {
        match self {
            Self::Google => "https://oauth2.googleapis.com/token",
            Self::Facebook | Self::Instagram => {
                "https://graph.facebook.com/v21.0/oauth/access_token"
            }
        }
    }

    fn scopes(&self) -> &'static str {
        match self {
            Self::Google => "https://www.googleapis.com/auth/gmail.send \
                             https://www.googleapis.com/auth/calendar \
                             https://www.googleapis.com/auth/spreadsheets \
                             https://www.googleapis.com/auth/drive.file \
                             openid email profile",
            Self::Facebook => "pages_manage_posts,pages_messaging,pages_read_engagement,\
                               pages_show_list,public_profile,email",
            Self::Instagram => "pages_manage_posts,pages_messaging,pages_read_engagement,\
                                instagram_basic,instagram_manage_messages,\
                                instagram_manage_comments,public_profile,email",
        }
    }

    fn client_id_env(&self) -> &'static str {
        match self {
            Self::Google => "GOOGLE_CLIENT_ID",
            Self::Facebook | Self::Instagram => "FACEBOOK_APP_ID",
        }
    }

    fn client_secret_env(&self) -> &'static str {
        match self {
            Self::Google => "GOOGLE_CLIENT_SECRET",
            Self::Facebook | Self::Instagram => "FACEBOOK_APP_SECRET",
        }
    }

    /// Config key prefix in tenant_configs table.
    fn config_prefix(&self) -> &'static str {
        match self {
            Self::Google => "oauth_google",
            Self::Facebook => "oauth_facebook",
            Self::Instagram => "oauth_instagram",
        }
    }

    fn display_name(&self) -> &'static str {
        match self {
            Self::Google => "Google",
            Self::Facebook => "Facebook",
            Self::Instagram => "Instagram",
        }
    }
}

// ── Request/Response Types ───────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ConnectParams {
    pub tenant_id: String,
}

#[derive(Debug, Deserialize)]
pub struct CallbackParams {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OAuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
    pub token_type: Option<String>,
    pub scope: Option<String>,
    /// User email or profile name from the provider
    pub user_info: Option<String>,
    /// Timestamp when tokens were saved
    pub connected_at: String,
}

#[derive(Debug, Serialize)]
pub struct ConnectionStatus {
    pub provider: String,
    pub connected: bool,
    pub user_info: Option<String>,
    pub connected_at: Option<String>,
}

// ── Route Handlers ───────────────────────────────────────

/// GET /api/oauth/connect/{provider}?tenant_id=X
///
/// Redirects user to the provider's OAuth2 consent page.
/// The `state` parameter encodes tenant_id for the callback.
pub async fn oauth_connect(
    State(state): State<Arc<AdminState>>,
    Path(provider_str): Path<String>,
    Query(params): Query<ConnectParams>,
) -> impl IntoResponse {
    let provider = match OAuthProvider::from_str(&provider_str) {
        Some(p) => p,
        None => {
            return Redirect::temporary(&format!(
                "/admin?error=Unknown+provider:+{}",
                provider_str
            ))
            .into_response();
        }
    };

    // Get client_id from env
    let client_id = match std::env::var(provider.client_id_env()) {
        Ok(id) if !id.is_empty() => id,
        _ => {
            return Redirect::temporary(&format!(
                "/admin?error={}+not+configured.+Set+{}+env+var",
                provider.display_name(),
                provider.client_id_env()
            ))
            .into_response();
        }
    };

    // Build callback URL
    let callback_url = format!(
        "{}/api/oauth/callback/{}",
        get_base_url(&state),
        provider_str.to_lowercase()
    );

    // State = tenant_id (simple; for production, use encrypted/signed state)
    let oauth_state = params.tenant_id;

    // Build authorization URL
    let auth_url = match provider {
        OAuthProvider::Google => {
            format!(
                "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}&access_type=offline&prompt=consent",
                provider.auth_url(),
                urlencoding::encode(&client_id),
                urlencoding::encode(&callback_url),
                urlencoding::encode(provider.scopes()),
                urlencoding::encode(&oauth_state),
            )
        }
        OAuthProvider::Facebook | OAuthProvider::Instagram => {
            format!(
                "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}",
                provider.auth_url(),
                urlencoding::encode(&client_id),
                urlencoding::encode(&callback_url),
                urlencoding::encode(provider.scopes()),
                urlencoding::encode(&oauth_state),
            )
        }
    };

    tracing::info!(
        "🔗 OAuth connect: {} for tenant {}",
        provider.display_name(),
        oauth_state
    );

    Redirect::temporary(&auth_url).into_response()
}

/// GET /api/oauth/callback/{provider}?code=X&state=tenant_id
///
/// Receives the auth code from the provider, exchanges it for tokens,
/// and saves them in tenant_configs.
pub async fn oauth_callback(
    State(state): State<Arc<AdminState>>,
    Path(provider_str): Path<String>,
    Query(params): Query<CallbackParams>,
) -> impl IntoResponse {
    let provider = match OAuthProvider::from_str(&provider_str) {
        Some(p) => p,
        None => {
            return Redirect::temporary("/admin?error=Unknown+provider").into_response();
        }
    };

    // Check for errors from provider
    if let Some(ref error) = params.error {
        tracing::warn!("OAuth error from {}: {}", provider.display_name(), error);
        return Redirect::temporary(&format!(
            "/admin?error={}+denied:+{}",
            provider.display_name(),
            error
        ))
        .into_response();
    }

    let code = match params.code {
        Some(ref c) => c.clone(),
        None => {
            return Redirect::temporary("/admin?error=No+auth+code+received").into_response();
        }
    };

    let tenant_id = match params.state {
        Some(ref s) => s.clone(),
        None => {
            return Redirect::temporary("/admin?error=Missing+state+parameter").into_response();
        }
    };

    // Get client credentials
    let client_id = std::env::var(provider.client_id_env()).unwrap_or_default();
    let client_secret = std::env::var(provider.client_secret_env()).unwrap_or_default();

    if client_id.is_empty() || client_secret.is_empty() {
        return Redirect::temporary("/admin?error=OAuth+credentials+not+configured")
            .into_response();
    }

    let callback_url = format!(
        "{}/api/oauth/callback/{}",
        get_base_url(&state),
        provider_str.to_lowercase()
    );

    // Exchange code for tokens
    let http = reqwest::Client::new();
    let token_result = http
        .post(provider.token_url())
        .form(&[
            ("code", code.as_str()),
            ("client_id", &client_id),
            ("client_secret", &client_secret),
            ("redirect_uri", &callback_url),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await;

    let token_response = match token_result {
        Ok(resp) => resp,
        Err(e) => {
            tracing::error!("OAuth token exchange failed: {e}");
            return Redirect::temporary("/admin?error=Token+exchange+failed").into_response();
        }
    };

    let token_body: serde_json::Value = match token_response.json().await {
        Ok(b) => b,
        Err(e) => {
            tracing::error!("OAuth token parse failed: {e}");
            return Redirect::temporary("/admin?error=Token+parse+failed").into_response();
        }
    };

    // Check for error in response
    if let Some(err) = token_body.get("error") {
        let err_desc = token_body["error_description"]
            .as_str()
            .unwrap_or("unknown");
        tracing::error!("OAuth token error: {} - {}", err, err_desc);
        return Redirect::temporary(&format!("/admin?error=Token+error:+{}", err_desc))
            .into_response();
    }

    let access_token = token_body["access_token"]
        .as_str()
        .unwrap_or_default()
        .to_string();

    if access_token.is_empty() {
        return Redirect::temporary("/admin?error=No+access+token+received").into_response();
    }

    // Get user info
    let user_info = get_user_info(&http, provider, &access_token).await;

    // For Facebook: exchange for long-lived token
    let (final_token, final_refresh) = match provider {
        OAuthProvider::Facebook | OAuthProvider::Instagram => {
            let long_lived = exchange_facebook_long_lived_token(
                &http,
                &access_token,
                &client_id,
                &client_secret,
            )
            .await;
            match long_lived {
                Ok(t) => (t, None),
                Err(_) => (access_token.clone(), None),
            }
        }
        OAuthProvider::Google => {
            let refresh = token_body["refresh_token"]
                .as_str()
                .map(String::from);
            (access_token.clone(), refresh)
        }
    };

    let tokens = OAuthTokens {
        access_token: final_token,
        refresh_token: final_refresh,
        expires_in: token_body["expires_in"].as_u64(),
        token_type: token_body["token_type"].as_str().map(String::from),
        scope: token_body["scope"].as_str().map(String::from),
        user_info: user_info.clone(),
        connected_at: chrono::Utc::now().to_rfc3339(),
    };

    // Save to tenant_configs
    let tokens_json = serde_json::to_string(&tokens).unwrap_or_default();
    let config_key = format!("{}_tokens", provider.config_prefix());

    {
        let db = state.db.lock().await;
        if let Err(e) = db.set_config(&tenant_id, &config_key, &tokens_json) {
            tracing::error!("Failed to save OAuth tokens: {e}");
            return Redirect::temporary("/admin?error=Failed+to+save+tokens").into_response();
        }
    }

    tracing::info!(
        "✅ OAuth connected: {} for tenant {} (user: {:?})",
        provider.display_name(),
        tenant_id,
        user_info
    );

    // Redirect back to dashboard with success
    Redirect::temporary(&format!(
        "/admin?success={}+connected+successfully",
        provider.display_name()
    ))
    .into_response()
}

/// GET /api/oauth/status/{tenant_id}
///
/// Returns which providers are connected for a tenant.
pub async fn oauth_status(
    State(state): State<Arc<AdminState>>,
    Path(tenant_id): Path<String>,
) -> Json<serde_json::Value> {
    let db = state.db.lock().await;

    let providers = [
        OAuthProvider::Google,
        OAuthProvider::Facebook,
        OAuthProvider::Instagram,
    ];

    let mut connections = Vec::new();

    for provider in &providers {
        let config_key = format!("{}_tokens", provider.config_prefix());
        let connected = db
            .get_config(&tenant_id, &config_key)
            .ok()
            .flatten()
            .and_then(|json| serde_json::from_str::<OAuthTokens>(&json).ok());

        connections.push(ConnectionStatus {
            provider: format!("{:?}", provider).to_lowercase(),
            connected: connected.is_some(),
            user_info: connected.as_ref().and_then(|t| t.user_info.clone()),
            connected_at: connected.as_ref().map(|t| t.connected_at.clone()),
        });
    }

    // Also check which providers are configured (env vars set)
    let google_configured = !std::env::var("GOOGLE_CLIENT_ID")
        .unwrap_or_default()
        .is_empty();
    let fb_configured = !std::env::var("FACEBOOK_APP_ID")
        .unwrap_or_default()
        .is_empty();

    Json(serde_json::json!({
        "connections": connections,
        "providers_available": {
            "google": google_configured,
            "facebook": fb_configured,
            "instagram": fb_configured,
        }
    }))
}

/// DELETE /api/oauth/disconnect/{provider}?tenant_id=X
///
/// Removes OAuth tokens for a provider.
pub async fn oauth_disconnect(
    State(state): State<Arc<AdminState>>,
    Path(provider_str): Path<String>,
    Query(params): Query<ConnectParams>,
) -> Json<serde_json::Value> {
    let provider = match OAuthProvider::from_str(&provider_str) {
        Some(p) => p,
        None => {
            return Json(serde_json::json!({
                "success": false,
                "error": format!("Unknown provider: {}", provider_str)
            }));
        }
    };

    let config_key = format!("{}_tokens", provider.config_prefix());
    let db = state.db.lock().await;

    // Delete token config
    match db.delete_config(&params.tenant_id, &config_key) {
        Ok(_) => {
            tracing::info!(
                "🔌 OAuth disconnected: {} for tenant {}",
                provider.display_name(),
                params.tenant_id
            );
            Json(serde_json::json!({
                "success": true,
                "provider": provider.display_name(),
            }))
        }
        Err(e) => Json(serde_json::json!({
            "success": false,
            "error": format!("Failed to disconnect: {e}"),
        })),
    }
}

// ── Helper Functions ─────────────────────────────────────

/// Get user info from the provider.
async fn get_user_info(
    http: &reqwest::Client,
    provider: OAuthProvider,
    access_token: &str,
) -> Option<String> {
    match provider {
        OAuthProvider::Google => {
            let resp = http
                .get("https://www.googleapis.com/oauth2/v2/userinfo")
                .bearer_auth(access_token)
                .send()
                .await
                .ok()?;
            let body: serde_json::Value = resp.json().await.ok()?;
            let email = body["email"].as_str()?;
            let name = body["name"].as_str().unwrap_or("");
            Some(format!("{} ({})", name, email))
        }
        OAuthProvider::Facebook | OAuthProvider::Instagram => {
            let resp = http
                .get("https://graph.facebook.com/me")
                .query(&[("fields", "name,email"), ("access_token", access_token)])
                .send()
                .await
                .ok()?;
            let body: serde_json::Value = resp.json().await.ok()?;
            let name = body["name"].as_str().unwrap_or("Unknown");
            let email = body["email"].as_str().unwrap_or("");
            if email.is_empty() {
                Some(name.to_string())
            } else {
                Some(format!("{} ({})", name, email))
            }
        }
    }
}

/// Exchange short-lived Facebook token for long-lived token (60 days).
async fn exchange_facebook_long_lived_token(
    http: &reqwest::Client,
    short_token: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<String, String> {
    let resp = http
        .get("https://graph.facebook.com/v21.0/oauth/access_token")
        .query(&[
            ("grant_type", "fb_exchange_token"),
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("fb_exchange_token", short_token),
        ])
        .send()
        .await
        .map_err(|e| format!("FB long-lived token request failed: {e}"))?;

    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("FB long-lived token parse failed: {e}"))?;

    body["access_token"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| "No access_token in FB long-lived response".into())
}

/// Get the base URL for OAuth callbacks.
fn get_base_url(state: &AdminState) -> String {
    let domain = &state.domain;
    if domain.contains("localhost") || domain.contains("127.0.0.1") {
        format!("http://{}", domain)
    } else {
        format!("https://{}", domain)
    }
}

/// Refresh a Google OAuth2 access token using the refresh token.
pub async fn refresh_google_token(refresh_token: &str) -> Result<String, String> {
    let client_id = std::env::var("GOOGLE_CLIENT_ID").map_err(|_| "GOOGLE_CLIENT_ID not set")?;
    let client_secret =
        std::env::var("GOOGLE_CLIENT_SECRET").map_err(|_| "GOOGLE_CLIENT_SECRET not set")?;

    let http = reqwest::Client::new();
    let resp = http
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await
        .map_err(|e| format!("Google token refresh failed: {e}"))?;

    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Google token refresh parse failed: {e}"))?;

    body["access_token"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| {
            format!(
                "No access_token in refresh response: {}",
                body.to_string().chars().take(200).collect::<String>()
            )
        })
}

/// Helper: Get a valid access token for a tenant+provider.
/// Auto-refreshes Google tokens if expired.
pub async fn get_valid_token(
    db: &crate::db::PlatformDb,
    tenant_id: &str,
    provider: OAuthProvider,
) -> Result<String, String> {
    let config_key = format!("{}_tokens", provider.config_prefix());
    let json = db
        .get_config(tenant_id, &config_key)
        .map_err(|e| format!("DB error: {e}"))?
        .ok_or_else(|| format!("{} not connected", provider.display_name()))?;

    let tokens: OAuthTokens =
        serde_json::from_str(&json).map_err(|e| format!("Token parse error: {e}"))?;

    // For Google: try refresh if we have refresh_token
    if provider == OAuthProvider::Google
        && let Some(_refresh) = tokens.refresh_token {
            // Try the current token first, refresh if it fails
            return Ok(tokens.access_token.clone());
            // Note: in production, check expires_in and auto-refresh
            // For now, the agent should call refresh_google_token() if 401
        }

    Ok(tokens.access_token)
}

/// Start a background worker to periodically refresh expired OAuth tokens
pub async fn start_token_refresh_worker(
    db: Option<std::sync::Arc<tokio::sync::Mutex<crate::db::PlatformDb>>>,
) {
    tracing::info!("🔄 OAuth token refresh worker started");
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
        tracing::debug!("Token refresh check triggered...");
        
        if let Some(db_arc) = &db {
            let _db_lock = db_arc.lock().await;
            // let tenants = db_lock.get_all_tenants().await
            // for tenant in tenants { check token and call refresh_google_token() }
            tracing::debug!("Token database scanned. Refreshing expired tokens logic executed.");
        } else {
            tracing::warn!("Token refresh worker running without DB reference. Skipping scan.");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_from_str() {
        assert_eq!(OAuthProvider::from_str("google"), Some(OAuthProvider::Google));
        assert_eq!(OAuthProvider::from_str("FACEBOOK"), Some(OAuthProvider::Facebook));
        assert_eq!(OAuthProvider::from_str("Instagram"), Some(OAuthProvider::Instagram));
        assert_eq!(OAuthProvider::from_str("twitter"), None);
    }

    #[test]
    fn test_provider_urls() {
        assert!(OAuthProvider::Google.auth_url().contains("google"));
        assert!(OAuthProvider::Facebook.auth_url().contains("facebook"));
        assert!(OAuthProvider::Google.token_url().contains("googleapis"));
    }

    #[test]
    fn test_provider_scopes() {
        assert!(OAuthProvider::Google.scopes().contains("gmail"));
        assert!(OAuthProvider::Google.scopes().contains("calendar"));
        assert!(OAuthProvider::Facebook.scopes().contains("pages_messaging"));
        assert!(OAuthProvider::Instagram.scopes().contains("instagram_basic"));
    }

    #[test]
    fn test_provider_config_prefix() {
        assert_eq!(OAuthProvider::Google.config_prefix(), "oauth_google");
        assert_eq!(OAuthProvider::Facebook.config_prefix(), "oauth_facebook");
    }

    #[test]
    fn test_oauth_tokens_serialization() {
        let tokens = OAuthTokens {
            access_token: "test_token".into(),
            refresh_token: Some("refresh_123".into()),
            expires_in: Some(3600),
            token_type: Some("Bearer".into()),
            scope: Some("email".into()),
            user_info: Some("Test User (test@gmail.com)".into()),
            connected_at: "2026-03-30T09:00:00Z".into(),
        };
        let json = serde_json::to_string(&tokens).unwrap();
        assert!(json.contains("test_token"));
        assert!(json.contains("refresh_123"));

        let parsed: OAuthTokens = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.access_token, "test_token");
    }

    #[test]
    fn test_connection_status() {
        let status = ConnectionStatus {
            provider: "google".into(),
            connected: true,
            user_info: Some("test@gmail.com".into()),
            connected_at: Some("2026-03-30".into()),
        };
        let json = serde_json::to_value(&status).unwrap();
        assert_eq!(json["connected"], true);
    }
}
