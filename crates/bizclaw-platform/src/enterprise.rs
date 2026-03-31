//! Enterprise features for BizClaw Platform.
//!
//! Implements:
//! - Feature 1: Multi-user per Tenant (RBAC nội bộ)
//! - Feature 2: Human Handoff ("Chuyển còi" cho nhân viên thật)
//! - Feature 3: BI Analytics Dashboard (Token cost, Sentiment, Funnel)
//! - Feature 4: Budget Quota Control (Token quota, alert, hard-stop)

use crate::db_pg::PgDb;
use bizclaw_core::error::{BizClawError, Result};
use serde::{Deserialize, Serialize};
use sqlx::Row;

// ════════════════════════════════════════════════════════
// FEATURE 1: MULTI-USER / RBAC
// ════════════════════════════════════════════════════════

/// Role of a member inside a single Tenant (not platform-wide).
/// Platform roles (superadmin/admin/viewer) still live in the `users` table.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TenantRole {
    /// Full control over the tenant, can invite/kick everyone.
    Owner,
    /// Can configure bot, channels, agents. Cannot delete tenant.
    Admin,
    /// CSKH: Can view conversations and handle handoffs. Cannot change config.
    Operator,
    /// Read-only: can view analytics and conversation history.
    Viewer,
}

impl TenantRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            TenantRole::Owner => "owner",
            TenantRole::Admin => "admin",
            TenantRole::Operator => "operator",
            TenantRole::Viewer => "viewer",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "owner" => TenantRole::Owner,
            "admin" => TenantRole::Admin,
            "operator" => TenantRole::Operator,
            _ => TenantRole::Viewer,
        }
    }

    /// Check if this role has write permission on tenant settings.
    pub fn can_configure(&self) -> bool {
        matches!(self, TenantRole::Owner | TenantRole::Admin)
    }

    /// Check if this role can handle handoff sessions.
    pub fn can_handle_handoff(&self) -> bool {
        matches!(
            self,
            TenantRole::Owner | TenantRole::Admin | TenantRole::Operator
        )
    }
}

/// A member entry in a tenant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantMember {
    pub tenant_id: String,
    pub user_id: String,
    pub email: String,
    pub role: String,
    pub status: String,
    pub joined_at: String,
}

/// Pending invitation to join a tenant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantInvitation {
    pub id: String,
    pub tenant_id: String,
    pub email: String,
    pub role: String,
    pub token: String,
    pub expires_at: String,
    pub created_at: String,
}

impl PgDb {
    // ── Members ──────────────────────────────────────────────────

    /// Add or update a user's membership in a tenant.
    pub async fn add_tenant_member(
        &self,
        tenant_id: &str,
        user_id: &str,
        role: &str,
        invited_by: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO tenant_members (tenant_id, user_id, role, invited_by, joined_at, status)
             VALUES ($1::uuid,$2::uuid,$3,$4::uuid,NOW(),'active')
             ON CONFLICT(tenant_id, user_id) DO UPDATE SET role=$3, status='active'",
        )
        .bind(tenant_id)
        .bind(user_id)
        .bind(role)
        .bind(invited_by)
        .execute(self.pool())
        .await
        .map_err(|e| BizClawError::Memory(format!("Add tenant member: {e}")))?;
        Ok(())
    }

    /// Remove a member from a tenant.
    pub async fn remove_tenant_member(&self, tenant_id: &str, user_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM tenant_members WHERE tenant_id=$1::uuid AND user_id=$2::uuid")
            .bind(tenant_id)
            .bind(user_id)
            .execute(self.pool())
            .await
            .map_err(|e| BizClawError::Memory(format!("Remove tenant member: {e}")))?;
        Ok(())
    }

    /// List all members of a tenant.
    pub async fn list_tenant_members(&self, tenant_id: &str) -> Result<Vec<TenantMember>> {
        let rows = sqlx::query(
            "SELECT tm.tenant_id::text, tm.user_id::text, u.email, tm.role,
                    tm.status, tm.joined_at::text
             FROM tenant_members tm
             JOIN users u ON u.id = tm.user_id
             WHERE tm.tenant_id=$1::uuid AND tm.status='active'
             ORDER BY tm.joined_at ASC",
        )
        .bind(tenant_id)
        .fetch_all(self.pool())
        .await
        .map_err(|e| BizClawError::Memory(format!("List tenant members: {e}")))?;

        Ok(rows
            .iter()
            .map(|r| TenantMember {
                tenant_id: r.get(0),
                user_id: r.get(1),
                email: r.get(2),
                role: r.get(3),
                status: r.get(4),
                joined_at: r.get(5),
            })
            .collect())
    }

    /// Get the role of a user in a specific tenant. Returns None if not a member.
    pub async fn get_tenant_member_role(
        &self,
        tenant_id: &str,
        user_id: &str,
    ) -> Result<Option<TenantRole>> {
        let role: Option<String> = sqlx::query_scalar(
            "SELECT role FROM tenant_members WHERE tenant_id=$1::uuid AND user_id=$2::uuid AND status='active'"
        )
        .bind(tenant_id).bind(user_id)
        .fetch_optional(self.pool()).await
        .map_err(|e| BizClawError::Memory(format!("Get member role: {e}")))?;

        Ok(role.map(|r| TenantRole::from_str(&r)))
    }

    // ── Invitations ───────────────────────────────────────────────

    /// Create an invitation link for a new member.
    pub async fn create_invitation(
        &self,
        tenant_id: &str,
        email: &str,
        role: &str,
        invited_by: &str,
        expires_hours: i64,
    ) -> Result<TenantInvitation> {
        let id = uuid::Uuid::new_v4().to_string();
        let token = format!("inv_{}", uuid::Uuid::new_v4().simple());
        sqlx::query(
            "INSERT INTO tenant_invitations
             (id, tenant_id, email, role, token, invited_by, expires_at)
             VALUES ($1::uuid,$2::uuid,$3,$4,$5,$6::uuid,NOW()+($7||' hours')::interval)
             ON CONFLICT(tenant_id, email) DO UPDATE
             SET role=$4, token=$5, invited_by=$6::uuid,
                 expires_at=NOW()+($7||' hours')::interval, accepted_at=NULL",
        )
        .bind(&id)
        .bind(tenant_id)
        .bind(email)
        .bind(role)
        .bind(&token)
        .bind(invited_by)
        .bind(expires_hours.to_string())
        .execute(self.pool())
        .await
        .map_err(|e| BizClawError::Memory(format!("Create invitation: {e}")))?;

        Ok(TenantInvitation {
            id,
            tenant_id: tenant_id.to_string(),
            email: email.to_string(),
            role: role.to_string(),
            token,
            expires_at: String::new(),
            created_at: String::new(),
        })
    }

    /// Accept an invitation token — adds the user to the tenant.
    /// Returns (tenant_id, role) on success.
    pub async fn accept_invitation(&self, token: &str, user_id: &str) -> Result<(String, String)> {
        let row = sqlx::query(
            "UPDATE tenant_invitations
             SET accepted_at = NOW()
             WHERE token=$1 AND accepted_at IS NULL AND expires_at > NOW()
             RETURNING tenant_id::text, role",
        )
        .bind(token)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| BizClawError::Memory(format!("Accept invitation: {e}")))?
        .ok_or_else(|| BizClawError::Memory("Invalid or expired invitation token".into()))?;

        let tenant_id: String = row.get(0);
        let role: String = row.get(1);
        self.add_tenant_member(&tenant_id, user_id, &role, None)
            .await?;
        Ok((tenant_id, role))
    }
}

// ════════════════════════════════════════════════════════
// FEATURE 2: HUMAN HANDOFF
// ════════════════════════════════════════════════════════

/// A handoff session — live conversation transferred from bot to human.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffSession {
    pub id: String,
    pub tenant_id: String,
    pub channel: String,
    pub external_user_id: String,
    pub external_user_name: Option<String>,
    pub status: String,
    pub assigned_to: Option<String>,
    pub trigger_reason: Option<String>,
    pub conversation_context: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
}

/// A message within a handoff session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffMessage {
    pub id: String,
    pub session_id: String,
    pub sender_type: String,
    pub sender_id: Option<String>,
    pub content: String,
    pub created_at: String,
}

impl PgDb {
    // ── Handoff Sessions ──────────────────────────────────────────

    /// Create a new handoff session when the bot needs human help.
    pub async fn create_handoff(
        &self,
        tenant_id: &str,
        channel: &str,
        external_user_id: &str,
        external_user_name: Option<&str>,
        trigger_reason: Option<&str>,
        context: &serde_json::Value,
    ) -> Result<HandoffSession> {
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO handoff_sessions
             (id, tenant_id, channel, external_user_id, external_user_name, trigger_reason, conversation_context)
             VALUES ($1::uuid,$2::uuid,$3,$4,$5,$6,$7::jsonb)"
        )
        .bind(&id).bind(tenant_id).bind(channel)
        .bind(external_user_id).bind(external_user_name)
        .bind(trigger_reason).bind(context)
        .execute(self.pool()).await
        .map_err(|e| BizClawError::Memory(format!("Create handoff: {e}")))?;
        self.get_handoff(&id).await
    }

    /// Fetch a handoff session by ID.
    pub async fn get_handoff(&self, id: &str) -> Result<HandoffSession> {
        let r = sqlx::query(
            "SELECT id::text, tenant_id::text, channel, external_user_id, external_user_name,
                    status, assigned_to::text, trigger_reason, conversation_context,
                    created_at::text, updated_at::text
             FROM handoff_sessions WHERE id=$1::uuid",
        )
        .bind(id)
        .fetch_one(self.pool())
        .await
        .map_err(|e| BizClawError::Memory(format!("Get handoff: {e}")))?;

        Ok(HandoffSession {
            id: r.get(0),
            tenant_id: r.get(1),
            channel: r.get(2),
            external_user_id: r.get(3),
            external_user_name: r.try_get(4).ok().flatten(),
            status: r.get(5),
            assigned_to: r.try_get(6).ok().flatten(),
            trigger_reason: r.try_get(7).ok().flatten(),
            conversation_context: r.try_get(8).unwrap_or_default(),
            created_at: r.get(9),
            updated_at: r.get(10),
        })
    }

    /// List handoff sessions for a tenant, optionally filtered by status.
    pub async fn list_handoffs(
        &self,
        tenant_id: &str,
        status: Option<&str>,
    ) -> Result<Vec<HandoffSession>> {
        let rows =
            if let Some(s) = status {
                sqlx::query(
                "SELECT id::text, tenant_id::text, channel, external_user_id, external_user_name,
                        status, assigned_to::text, trigger_reason, conversation_context,
                        created_at::text, updated_at::text
                 FROM handoff_sessions WHERE tenant_id=$1::uuid AND status=$2
                 ORDER BY created_at DESC LIMIT 100"
            )
            .bind(tenant_id).bind(s)
            .fetch_all(self.pool()).await
            } else {
                sqlx::query(
                "SELECT id::text, tenant_id::text, channel, external_user_id, external_user_name,
                        status, assigned_to::text, trigger_reason, conversation_context,
                        created_at::text, updated_at::text
                 FROM handoff_sessions WHERE tenant_id=$1::uuid
                 ORDER BY created_at DESC LIMIT 100"
            )
            .bind(tenant_id)
            .fetch_all(self.pool()).await
            }
            .map_err(|e| BizClawError::Memory(format!("List handoffs: {e}")))?;

        Ok(rows
            .iter()
            .map(|r| HandoffSession {
                id: r.get(0),
                tenant_id: r.get(1),
                channel: r.get(2),
                external_user_id: r.get(3),
                external_user_name: r.try_get(4).ok().flatten(),
                status: r.get(5),
                assigned_to: r.try_get(6).ok().flatten(),
                trigger_reason: r.try_get(7).ok().flatten(),
                conversation_context: r.try_get(8).unwrap_or_default(),
                created_at: r.get(9),
                updated_at: r.get(10),
            })
            .collect())
    }

    /// Claim a handoff session — assign to an operator.
    pub async fn claim_handoff(&self, handoff_id: &str, operator_id: &str) -> Result<()> {
        sqlx::query(
            "UPDATE handoff_sessions
             SET status='active', assigned_to=$2::uuid, updated_at=NOW()
             WHERE id=$1::uuid AND status='pending'",
        )
        .bind(handoff_id)
        .bind(operator_id)
        .execute(self.pool())
        .await
        .map_err(|e| BizClawError::Memory(format!("Claim handoff: {e}")))?;
        Ok(())
    }

    /// Resolve a handoff session.
    pub async fn resolve_handoff(&self, handoff_id: &str, resolved_by: &str) -> Result<()> {
        sqlx::query(
            "UPDATE handoff_sessions
             SET status='resolved', resolved_by=$2::uuid, resolved_at=NOW(), updated_at=NOW()
             WHERE id=$1::uuid",
        )
        .bind(handoff_id)
        .bind(resolved_by)
        .execute(self.pool())
        .await
        .map_err(|e| BizClawError::Memory(format!("Resolve handoff: {e}")))?;
        Ok(())
    }

    /// Add a message to a handoff session.
    pub async fn add_handoff_message(
        &self,
        session_id: &str,
        sender_type: &str,
        sender_id: Option<&str>,
        content: &str,
    ) -> Result<HandoffMessage> {
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO handoff_messages (id, session_id, sender_type, sender_id, content)
             VALUES ($1::uuid,$2::uuid,$3,$4,$5)",
        )
        .bind(&id)
        .bind(session_id)
        .bind(sender_type)
        .bind(sender_id)
        .bind(content)
        .execute(self.pool())
        .await
        .map_err(|e| BizClawError::Memory(format!("Add handoff message: {e}")))?;

        Ok(HandoffMessage {
            id,
            session_id: session_id.to_string(),
            sender_type: sender_type.to_string(),
            sender_id: sender_id.map(|s| s.to_string()),
            content: content.to_string(),
            created_at: String::new(),
        })
    }

    /// Get messages for a handoff session.
    pub async fn list_handoff_messages(&self, session_id: &str) -> Result<Vec<HandoffMessage>> {
        let rows = sqlx::query(
            "SELECT id::text, session_id::text, sender_type, sender_id, content, created_at::text
             FROM handoff_messages WHERE session_id=$1::uuid ORDER BY created_at ASC",
        )
        .bind(session_id)
        .fetch_all(self.pool())
        .await
        .map_err(|e| BizClawError::Memory(format!("List handoff messages: {e}")))?;

        Ok(rows
            .iter()
            .map(|r| HandoffMessage {
                id: r.get(0),
                session_id: r.get(1),
                sender_type: r.get(2),
                sender_id: r.try_get(3).ok().flatten(),
                content: r.get(4),
                created_at: r.get(5),
            })
            .collect())
    }
}

// ════════════════════════════════════════════════════════
// FEATURE 3: BI ANALYTICS
// ════════════════════════════════════════════════════════

/// Analytics summary for a tenant over a date range.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsSummary {
    pub total_conversations: i64,
    pub resolved_count: i64,
    pub handoff_count: i64,
    pub abandoned_count: i64,
    pub converted_count: i64,
    pub positive_sentiment: i64,
    pub neutral_sentiment: i64,
    pub negative_sentiment: i64,
    pub total_tokens: i64,
    pub total_cost_usd: f64,
    pub avg_duration_seconds: f64,
    pub resolution_rate: f64, // resolved / total
    pub handoff_rate: f64,    // handoffs / total
}

/// Daily token usage breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsageDay {
    pub date: String,
    pub provider: String,
    pub model: String,
    pub total_tokens: i64,
    pub estimated_cost_usd: f64,
}

impl PgDb {
    // ── Analytics ─────────────────────────────────────────────────

    /// Record token usage for a conversation.
    #[allow(clippy::too_many_arguments)]
    pub async fn log_token_usage(
        &self,
        tenant_id: &str,
        provider: &str,
        model: &str,
        prompt_tokens: i32,
        completion_tokens: i32,
        session_id: Option<&str>,
        channel: Option<&str>,
    ) -> Result<()> {
        // Estimate cost using common pricing (USD per 1M tokens)
        let cost_per_1m: f32 = match model {
            m if m.contains("gpt-4o") => 2.50,
            m if m.contains("gpt-4") => 30.0,
            m if m.contains("gpt-3.5") => 0.50,
            m if m.contains("claude-3-5") => 3.00,
            m if m.contains("claude-3") => 15.0,
            m if m.contains("gemini-pro") => 0.125,
            _ => 1.0,
        };
        let total_tokens = prompt_tokens + completion_tokens;
        let cost = (total_tokens as f32 / 1_000_000.0) * cost_per_1m;

        sqlx::query(
            "INSERT INTO token_usage_log
             (tenant_id, provider, model, prompt_tokens, completion_tokens, total_tokens, estimated_cost_usd, session_id, channel)
             VALUES ($1::uuid,$2,$3,$4,$5,$6,$7,$8,$9)"
        )
        .bind(tenant_id).bind(provider).bind(model)
        .bind(prompt_tokens).bind(completion_tokens).bind(total_tokens).bind(cost)
        .bind(session_id).bind(channel)
        .execute(self.pool()).await
        .map_err(|e| BizClawError::Memory(format!("Log token usage: {e}")))?;
        Ok(())
    }

    /// Record a conversation outcome.
    #[allow(clippy::too_many_arguments)]
    pub async fn log_conversation_outcome(
        &self,
        tenant_id: &str,
        session_id: &str,
        channel: Option<&str>,
        outcome: &str,
        sentiment: &str,
        duration_seconds: i32,
        message_count: i32,
        tokens_total: i32,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO conversation_outcomes
             (tenant_id, session_id, channel, outcome, sentiment, duration_seconds, message_count, tokens_total)
             VALUES ($1::uuid,$2,$3,$4,$5,$6,$7,$8)
             ON CONFLICT DO NOTHING"
        )
        .bind(tenant_id).bind(session_id).bind(channel)
        .bind(outcome).bind(sentiment)
        .bind(duration_seconds).bind(message_count).bind(tokens_total)
        .execute(self.pool()).await
        .map_err(|e| BizClawError::Memory(format!("Log conversation outcome: {e}")))?;
        Ok(())
    }

    /// Get analytics summary for a tenant over the last N days.
    pub async fn get_analytics_summary(
        &self,
        tenant_id: &str,
        days: i32,
    ) -> Result<AnalyticsSummary> {
        let row = sqlx::query(
            "SELECT
                COALESCE(SUM(total_conversations), 0),
                COALESCE(SUM(resolved_count),  0),
                COALESCE(SUM(handoff_count),   0),
                COALESCE(SUM(abandoned_count), 0),
                COALESCE(SUM(converted_count), 0),
                COALESCE(SUM(positive_sentiment), 0),
                COALESCE(SUM(neutral_sentiment),  0),
                COALESCE(SUM(negative_sentiment), 0),
                COALESCE(SUM(total_tokens), 0),
                COALESCE(SUM(total_cost_usd), 0.0),
                COALESCE(AVG(avg_duration_seconds), 0.0)
             FROM analytics_daily
             WHERE tenant_id=$1::uuid AND date >= CURRENT_DATE - ($2 || ' days')::interval",
        )
        .bind(tenant_id)
        .bind(days.to_string())
        .fetch_one(self.pool())
        .await
        .map_err(|e| BizClawError::Memory(format!("Get analytics summary: {e}")))?;

        let total: i64 = row.try_get(0).unwrap_or(0);
        let resolved: i64 = row.try_get(1).unwrap_or(0);
        let handoffs: i64 = row.try_get(2).unwrap_or(0);
        let abandoned: i64 = row.try_get(3).unwrap_or(0);
        let converted: i64 = row.try_get(4).unwrap_or(0);
        let positive: i64 = row.try_get(5).unwrap_or(0);
        let neutral: i64 = row.try_get(6).unwrap_or(0);
        let negative: i64 = row.try_get(7).unwrap_or(0);
        let tokens: i64 = row.try_get(8).unwrap_or(0);
        let cost: f64 = row.try_get::<f64, _>(9).unwrap_or(0.0);
        let avg_dur: f64 = row.try_get::<f64, _>(10).unwrap_or(0.0);

        let resolution_rate = if total > 0 {
            resolved as f64 / total as f64
        } else {
            0.0
        };
        let handoff_rate = if total > 0 {
            handoffs as f64 / total as f64
        } else {
            0.0
        };

        Ok(AnalyticsSummary {
            total_conversations: total,
            resolved_count: resolved,
            handoff_count: handoffs,
            abandoned_count: abandoned,
            converted_count: converted,
            positive_sentiment: positive,
            neutral_sentiment: neutral,
            negative_sentiment: negative,
            total_tokens: tokens,
            total_cost_usd: cost,
            avg_duration_seconds: avg_dur,
            resolution_rate,
            handoff_rate,
        })
    }

    /// Get daily token usage breakdown for the last N days.
    pub async fn get_token_usage_by_day(
        &self,
        tenant_id: &str,
        days: i32,
    ) -> Result<Vec<TokenUsageDay>> {
        let rows = sqlx::query(
            "SELECT date::text, provider, model,
                    SUM(total_tokens)::bigint, CAST(SUM(estimated_cost_usd) AS float8)
             FROM token_usage_log
             WHERE tenant_id=$1::uuid AND date >= CURRENT_DATE - ($2 || ' days')::interval
             GROUP BY date, provider, model ORDER BY date DESC",
        )
        .bind(tenant_id)
        .bind(days.to_string())
        .fetch_all(self.pool())
        .await
        .map_err(|e| BizClawError::Memory(format!("Get token usage: {e}")))?;

        Ok(rows
            .iter()
            .map(|r| TokenUsageDay {
                date: r.get(0),
                provider: r.get(1),
                model: r.get(2),
                total_tokens: r.try_get::<i64, _>(3).unwrap_or(0),
                estimated_cost_usd: r.try_get::<f64, _>(4).unwrap_or(0.0),
            })
            .collect())
    }

    /// Refresh (upsert) daily analytics aggregate for a tenant.
    /// Call this at end-of-day or via cron job.
    pub async fn refresh_analytics_daily(&self, tenant_id: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO analytics_daily
             (tenant_id, date, total_conversations, resolved_count, handoff_count,
              abandoned_count, converted_count, positive_sentiment, neutral_sentiment,
              negative_sentiment, total_tokens, total_cost_usd, avg_duration_seconds)
             SELECT
                $1::uuid, CURRENT_DATE,
                COUNT(*),
                COUNT(*) FILTER (WHERE outcome='resolved'),
                COUNT(*) FILTER (WHERE outcome='handoff'),
                COUNT(*) FILTER (WHERE outcome='abandoned'),
                COUNT(*) FILTER (WHERE outcome='converted'),
                COUNT(*) FILTER (WHERE sentiment='positive'),
                COUNT(*) FILTER (WHERE sentiment='neutral'),
                COUNT(*) FILTER (WHERE sentiment='negative'),
                COALESCE(SUM(tokens_total), 0),
                0,
                COALESCE(AVG(duration_seconds), 0)
             FROM conversation_outcomes WHERE tenant_id=$1::uuid AND date=CURRENT_DATE
             ON CONFLICT (tenant_id, date) DO UPDATE SET
                total_conversations = EXCLUDED.total_conversations,
                resolved_count      = EXCLUDED.resolved_count,
                handoff_count       = EXCLUDED.handoff_count,
                abandoned_count     = EXCLUDED.abandoned_count,
                converted_count     = EXCLUDED.converted_count,
                positive_sentiment  = EXCLUDED.positive_sentiment,
                neutral_sentiment   = EXCLUDED.neutral_sentiment,
                negative_sentiment  = EXCLUDED.negative_sentiment,
                total_tokens        = EXCLUDED.total_tokens,
                avg_duration_seconds = EXCLUDED.avg_duration_seconds,
                updated_at          = NOW()",
        )
        .bind(tenant_id)
        .execute(self.pool())
        .await
        .map_err(|e| BizClawError::Memory(format!("Refresh analytics: {e}")))?;
        Ok(())
    }
}

// ════════════════════════════════════════════════════════
// FEATURE 4: BUDGET QUOTA CONTROL
// ════════════════════════════════════════════════════════

/// Current quota status for a resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaStatus {
    pub resource: String,
    pub limit_value: i64,
    pub used_value: i64,
    pub used_pct: f64,
    pub remaining: i64,
    pub is_exceeded: bool,
    pub alert_threshold: f64,
    pub will_alert: bool,
    pub reset_at: Option<String>,
}

impl PgDb {
    // ── Quota ─────────────────────────────────────────────────────

    /// Get quota status for all resources of a tenant.
    pub async fn get_quota_status(&self, tenant_id: &str) -> Result<Vec<QuotaStatus>> {
        let rows = sqlx::query(
            "SELECT resource, limit_value, used_value, alert_threshold, reset_at::text
             FROM tenant_quotas WHERE tenant_id=$1::uuid ORDER BY resource",
        )
        .bind(tenant_id)
        .fetch_all(self.pool())
        .await
        .map_err(|e| BizClawError::Memory(format!("Get quota status: {e}")))?;

        Ok(rows
            .iter()
            .map(|r| {
                let resource: String = r.get(0);
                let limit: i64 = r.try_get::<i64, _>(1).unwrap_or(0);
                let used: i64 = r.try_get::<i64, _>(2).unwrap_or(0);
                let threshold: f64 = r.try_get::<f64, _>(3).unwrap_or(0.8);
                let reset_at: Option<String> = r.try_get(4).ok().flatten();
                let used_pct = if limit > 0 {
                    used as f64 / limit as f64
                } else {
                    0.0
                };
                QuotaStatus {
                    resource,
                    limit_value: limit,
                    used_value: used,
                    used_pct,
                    remaining: (limit - used).max(0),
                    is_exceeded: used >= limit,
                    alert_threshold: threshold,
                    will_alert: used_pct >= threshold,
                    reset_at,
                }
            })
            .collect())
    }

    /// Consume quota units. Returns `true` if still within limit, `false` if exceeded.
    pub async fn consume_quota(
        &self,
        tenant_id: &str,
        resource: &str,
        amount: i64,
    ) -> Result<bool> {
        // First reset if needed
        sqlx::query(
            "UPDATE tenant_quotas
             SET used_value=0, alert_sent=false, reset_at=
                 CASE resource
                     WHEN 'messages_per_day'   THEN DATE_TRUNC('day',NOW()) + INTERVAL '1 day'
                     ELSE DATE_TRUNC('month',NOW()) + INTERVAL '1 month'
                 END
             WHERE tenant_id=$1::uuid AND resource=$2 AND reset_at <= NOW()",
        )
        .bind(tenant_id)
        .bind(resource)
        .execute(self.pool())
        .await
        .ok();

        // Then increment
        let new_used: Option<i64> = sqlx::query_scalar(
            "UPDATE tenant_quotas
             SET used_value = used_value + $3, updated_at = NOW()
             WHERE tenant_id=$1::uuid AND resource=$2
             RETURNING used_value",
        )
        .bind(tenant_id)
        .bind(resource)
        .bind(amount)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| BizClawError::Memory(format!("Consume quota: {e}")))?;

        // Check if still within limit
        let within_limit = match new_used {
            Some(used) => {
                let limit: Option<i64> = sqlx::query_scalar(
                    "SELECT limit_value FROM tenant_quotas WHERE tenant_id=$1::uuid AND resource=$2"
                )
                .bind(tenant_id).bind(resource)
                .fetch_optional(self.pool()).await.ok().flatten();
                limit.map(|l| used <= l).unwrap_or(true)
            }
            None => true, // No quota configured = unlimited
        };
        Ok(within_limit)
    }

    /// Update quota limits for a tenant (e.g., when plan changes).
    pub async fn set_quota(&self, tenant_id: &str, resource: &str, limit_value: i64) -> Result<()> {
        sqlx::query(
            "INSERT INTO tenant_quotas (tenant_id, resource, limit_value)
             VALUES ($1::uuid,$2,$3)
             ON CONFLICT(tenant_id, resource) DO UPDATE SET limit_value=$3, updated_at=NOW()",
        )
        .bind(tenant_id)
        .bind(resource)
        .bind(limit_value)
        .execute(self.pool())
        .await
        .map_err(|e| BizClawError::Memory(format!("Set quota: {e}")))?;
        Ok(())
    }

    /// Check if quota alert should be sent. Returns tenants that need alerting.
    pub async fn get_quota_alert_candidates(&self) -> Result<Vec<(String, String, f64)>> {
        let rows = sqlx::query(
            "SELECT tq.tenant_id::text, tq.resource,
                    CAST(tq.used_value AS float8) / NULLIF(tq.limit_value,0) as used_pct
             FROM tenant_quotas tq
             WHERE tq.alert_sent = false
               AND tq.used_value::float8 / NULLIF(tq.limit_value,0) >= tq.alert_threshold
             ORDER BY used_pct DESC",
        )
        .fetch_all(self.pool())
        .await
        .map_err(|e| BizClawError::Memory(format!("Get quota alert candidates: {e}")))?;

        Ok(rows
            .iter()
            .map(|r| {
                (
                    r.get::<String, _>(0),
                    r.get::<String, _>(1),
                    r.try_get::<f64, _>(2).unwrap_or(0.0),
                )
            })
            .collect())
    }

    /// Mark alert as sent for a tenant/resource.
    pub async fn mark_quota_alert_sent(&self, tenant_id: &str, resource: &str) -> Result<()> {
        sqlx::query(
            "UPDATE tenant_quotas SET alert_sent=true WHERE tenant_id=$1::uuid AND resource=$2",
        )
        .bind(tenant_id)
        .bind(resource)
        .execute(self.pool())
        .await
        .map_err(|e| BizClawError::Memory(format!("Mark alert sent: {e}")))?;
        Ok(())
    }
}
