//! BizClaw Cloud Module
//!
//! Manages multi-tenant Cloud SaaS infrastructure:
//! - Proxmox VE API client for VM provisioning
//! - Tenant lifecycle management (create/suspend/delete)
//! - Plan-based resource allocation
//!
//! Architecture:
//! ```
//! BizClaw Cloud Panel (Admin UI)
//!   └─→ Tenant Manager
//!       └─→ Proxmox Client (REST API)
//!           └─→ Proxmox VE Cluster
//!               └─→ VMs (1 per tenant)
//!                   ├─→ BizClaw Binary + Config
//!                   ├─→ Ollama (shared GPU)
//!                   └─→ *.cloud.bizclaw.vn subdomain
//! ```

pub mod proxmox_client;
pub mod tenant_manager;

pub use proxmox_client::ProxmoxClient;
pub use tenant_manager::{
    CloudConfig, Tenant, TenantPlan, TenantStatus, ProvisionResult,
    provision_tenant, suspend_tenant, resume_tenant, delete_tenant,
};
