//! # BizClaw Platform
//!
//! Multi-tenant management platform — run multiple BizClaw agents on a single VPS.
//! Includes admin dashboard, tenant lifecycle management, pairing security,
//! subdomain routing, resource monitoring, and audit logging.
//! Now with PostgreSQL support, ReMe Memory, Heartbeat/Cron, and Skills.

pub mod admin;
pub mod auth;
pub mod config;
pub mod db;
pub mod db_pg;
pub mod enterprise;
pub mod mama;
pub mod mission_control;
pub mod oauth;
pub mod routing;
pub mod self_serve;
pub mod server_provisioner;
pub mod sheets;
pub mod social_automation;
pub mod tenant;

pub use admin::AdminServer;
pub use db::PlatformDb;
pub use db_pg::PgDb;
pub use enterprise::{
    AnalyticsSummary, HandoffMessage, HandoffSession, QuotaStatus, TenantInvitation, TenantMember,
    TenantRole,
};
pub use mission_control::{
    AgentSession, GithubSync, KANBAN_COLUMNS, QualityReview, Task, TaskComment,
};
pub use server_provisioner::{ProvisionRequest, RemoteServer};
pub use tenant::TenantManager;
