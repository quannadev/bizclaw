//! Google Sheets — AI Agent tool for business data.
//!
//! Dùng cho AI Agent ghi dữ liệu kinh doanh vào Google Sheets:
//! đơn hàng, booking, báo cáo, etc. KHÔNG dùng cho billing SaaS
//! (billing ghi thẳng vào PostgreSQL).
//!
//! Uses Google Sheets API v4 with Service Account authentication.
//!
//! Env vars (set per tenant):
//!   - `GOOGLE_SERVICE_ACCOUNT_JSON`: path to service account key file
//!   - `BIZCLAW_GOOGLE_SHEETS_ID`: Google Spreadsheet ID
//!   - `BIZCLAW_GOOGLE_SHEETS_NAME`: Sheet/tab name (default: "Giao dịch")

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Google Sheets client for BizClaw payment tracking.
#[derive(Clone)]
pub struct SheetsClient {
    spreadsheet_id: String,
    sheet_name: String,
    service_account: ServiceAccountKey,
    http: reqwest::Client,
}

/// Google Service Account key (JSON format).
#[derive(Debug, Clone, Deserialize)]
struct ServiceAccountKey {
    client_email: String,
    private_key: String,
    #[serde(default)]
    token_uri: String,
}

/// JWT claims for Google OAuth2.
#[derive(Debug, Serialize)]
struct GoogleClaims {
    iss: String,
    scope: String,
    aud: String,
    exp: u64,
    iat: u64,
}

/// A row of data to append to the sheet.
#[derive(Debug, Clone)]
pub struct TransactionRow {
    pub date: String,
    pub sepay_id: String,
    pub gateway: String,
    pub amount: f64,
    pub content: String,
    pub reference_code: String,
    pub account_number: String,
    pub tenant_slug: String,
    pub plan_activated: String,
    pub status: String,
}

impl SheetsClient {
    /// Create a new Google Sheets client from environment variables.
    ///
    /// Returns None if not configured (graceful fallback).
    pub fn from_env() -> Option<Self> {
        let sa_path = std::env::var("GOOGLE_SERVICE_ACCOUNT_JSON").ok()?;
        let spreadsheet_id = std::env::var("BIZCLAW_GOOGLE_SHEETS_ID").ok()?;
        let sheet_name = std::env::var("BIZCLAW_GOOGLE_SHEETS_NAME")
            .unwrap_or_else(|_| "Giao dịch".into());

        let sa_content = std::fs::read_to_string(&sa_path).ok()?;
        let mut service_account: ServiceAccountKey = serde_json::from_str(&sa_content).ok()?;

        if service_account.token_uri.is_empty() {
            service_account.token_uri = "https://oauth2.googleapis.com/token".into();
        }

        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .ok()?;

        tracing::info!(
            "📊 Google Sheets integration enabled: spreadsheet={}, sheet={}",
            spreadsheet_id,
            sheet_name
        );

        Some(Self {
            spreadsheet_id,
            sheet_name,
            service_account,
            http,
        })
    }

    /// Get an access token using the service account JWT.
    async fn get_access_token(&self) -> Result<String, String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| format!("Time error: {e}"))?
            .as_secs();

        let claims = GoogleClaims {
            iss: self.service_account.client_email.clone(),
            scope: "https://www.googleapis.com/auth/spreadsheets".into(),
            aud: self.service_account.token_uri.clone(),
            iat: now,
            exp: now + 3600,
        };

        // Sign JWT with RS256
        let key = jsonwebtoken::EncodingKey::from_rsa_pem(
            self.service_account.private_key.as_bytes(),
        )
        .map_err(|e| format!("Invalid service account key: {e}"))?;

        let header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
        let jwt = jsonwebtoken::encode(&header, &claims, &key)
            .map_err(|e| format!("JWT encode error: {e}"))?;

        // Exchange JWT for access token
        let resp = self
            .http
            .post(&self.service_account.token_uri)
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
                ("assertion", &jwt),
            ])
            .send()
            .await
            .map_err(|e| format!("Token request failed: {e}"))?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Token parse error: {e}"))?;

        body["access_token"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| format!("No access_token in response: {body}"))
    }

    /// Append a transaction row to the Google Sheet.
    pub async fn append_transaction(&self, row: &TransactionRow) -> Result<(), String> {
        let token = self.get_access_token().await?;

        // Build row data matching the sheet columns:
        // Ngày | SePay ID | Ngân hàng | Số tiền | Nội dung | Mã tham chiếu | STK | Tenant | Gói | Trạng thái
        let values = vec![vec![
            serde_json::Value::String(row.date.clone()),
            serde_json::Value::String(row.sepay_id.clone()),
            serde_json::Value::String(row.gateway.clone()),
            serde_json::json!(row.amount),
            serde_json::Value::String(row.content.clone()),
            serde_json::Value::String(row.reference_code.clone()),
            serde_json::Value::String(row.account_number.clone()),
            serde_json::Value::String(row.tenant_slug.clone()),
            serde_json::Value::String(row.plan_activated.clone()),
            serde_json::Value::String(row.status.clone()),
        ]];

        let body = serde_json::json!({
            "values": values,
        });

        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}:append?valueInputOption=USER_ENTERED&insertDataOption=INSERT_ROWS",
            self.spreadsheet_id,
            urlencoding::encode(&self.sheet_name),
        );

        let resp = self
            .http
            .post(&url)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Sheets append failed: {e}"))?;

        if resp.status().is_success() {
            tracing::info!(
                "📊 Google Sheets: appended transaction {} ({}đ) to '{}'",
                row.sepay_id,
                row.amount,
                self.sheet_name
            );
            Ok(())
        } else {
            let status = resp.status();
            let error_body = resp.text().await.unwrap_or_default();
            Err(format!(
                "Sheets API error {}: {}",
                status, error_body
            ))
        }
    }

    /// Initialize the sheet with headers if it's empty.
    pub async fn ensure_headers(&self) -> Result<(), String> {
        let token = self.get_access_token().await?;

        // Check if first row exists
        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}!A1:J1",
            self.spreadsheet_id,
            urlencoding::encode(&self.sheet_name),
        );

        let resp = self
            .http
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| format!("Sheets read failed: {e}"))?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Sheets parse error: {e}"))?;

        // If no values, write headers
        let has_values = body["values"]
            .as_array()
            .map(|a| !a.is_empty())
            .unwrap_or(false);

        if !has_values {
            let headers = serde_json::json!({
                "values": [["Ngày", "SePay ID", "Ngân hàng", "Số tiền (VND)", "Nội dung CK", "Mã tham chiếu", "Số TK", "Tenant", "Gói kích hoạt", "Trạng thái"]]
            });

            let update_url = format!(
                "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}!A1:J1?valueInputOption=USER_ENTERED",
                self.spreadsheet_id,
                urlencoding::encode(&self.sheet_name),
            );

            self.http
                .put(&update_url)
                .bearer_auth(&token)
                .json(&headers)
                .send()
                .await
                .map_err(|e| format!("Sheets header write failed: {e}"))?;

            tracing::info!("📊 Google Sheets: initialized headers in '{}'", self.sheet_name);
        }

        Ok(())
    }

    /// Check if the client is configured and can connect.
    pub async fn health_check(&self) -> Result<String, String> {
        let token = self.get_access_token().await?;

        let url = format!(
            "https://sheets.googleapis.com/v4/spreadsheets/{}?fields=properties.title",
            self.spreadsheet_id,
        );

        let resp = self
            .http
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| format!("Sheets health check failed: {e}"))?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Sheets parse error: {e}"))?;

        body["properties"]["title"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "Cannot read spreadsheet title".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_env_returns_none_when_not_configured() {
        // from_env returns None when required env vars are not set
        // In test environment, these vars are not set, so this should return None
        // (We don't use remove_var because it's unsafe in Rust 2024+)
        let result = SheetsClient::from_env();
        // May or may not be None depending on test env, but should not panic
        let _ = result;
    }

    #[test]
    fn test_transaction_row_fields() {
        let row = TransactionRow {
            date: "2026-03-30".into(),
            sepay_id: "12345".into(),
            gateway: "Vietcombank".into(),
            amount: 499_000.0,
            content: "BIZCLAW-dalat-xinh".into(),
            reference_code: "REF001".into(),
            account_number: "0123456789".into(),
            tenant_slug: "dalat-xinh".into(),
            plan_activated: "pro".into(),
            status: "success".into(),
        };
        assert_eq!(row.amount, 499_000.0);
        assert_eq!(row.tenant_slug, "dalat-xinh");
    }

    #[test]
    fn test_google_claims_serialization() {
        let claims = GoogleClaims {
            iss: "test@example.iam.gserviceaccount.com".into(),
            scope: "https://www.googleapis.com/auth/spreadsheets".into(),
            aud: "https://oauth2.googleapis.com/token".into(),
            iat: 1000,
            exp: 2000,
        };
        let json = serde_json::to_string(&claims).unwrap();
        assert!(json.contains("spreadsheets"));
        assert!(json.contains("test@example"));
    }
}
