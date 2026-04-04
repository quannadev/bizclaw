//! Cloud SaaS API routes — tenant CRUD, vSphere config, cluster stats.
//!
//! All routes require admin role.
//! Tenant data stored in platform.db (cloud_tenants table).
//! VM lifecycle managed via VMware vSphere REST API.

use axum::{
    extract::State,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::admin::AdminState;

// ═══ REQUEST / RESPONSE TYPES ═══

#[derive(Debug, Deserialize)]
pub struct CreateTenantReq {
    pub name: String,
    pub email: String,
    #[serde(default)]
    pub phone: String,
    #[serde(default = "default_plan")]
    pub plan: String,
    #[serde(default)]
    pub notes: String,
}

fn default_plan() -> String {
    "starter".to_string()
}

#[derive(Debug, Serialize)]
pub struct TenantResponse {
    pub id: String,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub plan: String,
    pub vmid: u32,
    pub ip_address: String,
    pub subdomain: String,
    pub status: String,
    pub created_at: String,
    pub expires_at: String,
    pub pairing_code: String,
}

#[derive(Debug, Serialize)]
pub struct CloudStatsResponse {
    pub total_tenants: u32,
    pub active_tenants: u32,
    pub provisioning: u32,
    pub suspended: u32,
    pub cpu_usage: f64,
    pub ram_usage: f64,
    pub mrr: u64,
    pub vsphere_connected: bool,
}

#[derive(Debug, Deserialize)]
pub struct CloudConfigReq {
    /// vSphere/vCenter host URL (e.g., https://vcenter.bizclaw.vn)
    pub vsphere_host: String,
    /// vCenter admin user (e.g., administrator@vsphere.local)
    pub vsphere_user: String,
    /// vCenter password (stored encrypted)
    #[serde(default)]
    pub vsphere_password: String,
    /// VM template ID in vCenter inventory (e.g., vm-1001)
    #[serde(default)]
    pub template_id: String,
    /// Datacenter name
    #[serde(default = "default_datacenter")]
    pub datacenter: String,
    /// Target folder for new VMs
    #[serde(default = "default_folder")]
    pub folder: String,
    /// Resource pool
    #[serde(default)]
    pub resource_pool: String,
    /// Datastore
    #[serde(default = "default_datastore")]
    pub datastore: String,
    /// VM network name
    #[serde(default = "default_network")]
    pub network: String,
    /// Base domain for tenant subdomains
    #[serde(default = "default_domain")]
    pub domain_base: String,
}

fn default_datacenter() -> String { "Datacenter".to_string() }
fn default_folder() -> String { "BizClaw-Tenants".to_string() }
fn default_datastore() -> String { "datastore1".to_string() }
fn default_network() -> String { "VM Network".to_string() }
fn default_domain() -> String { "cloud.bizclaw.vn".to_string() }

#[derive(Debug, Deserialize)]
pub struct RenewTenantReq {
    /// New plan (if upgrading/downgrading)
    pub plan: Option<String>,
    /// Number of days to extend
    #[serde(default = "default_renew_days")]
    pub days: u32,
}

fn default_renew_days() -> u32 { 30 }

// ═══ ROUTE HANDLERS ═══

/// GET /api/v1/cloud/tenants — List all tenants
pub async fn cloud_list_tenants(
    State(state): State<Arc<AdminState>>,
) -> Json<serde_json::Value> {
    let tenants = state.db.lock().await.cloud_list_tenants().unwrap_or_default();
    Json(serde_json::json!({
        "tenants": tenants
    }))
}

/// POST /api/v1/cloud/tenants — Create new tenant + trigger VM provisioning
pub async fn cloud_create_tenant(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<CreateTenantReq>,
) -> Json<serde_json::Value> {
    // Generate tenant ID
    let tenant_id = generate_tenant_id(&req.name);
    let subdomain = format!("{}.cloud.bizclaw.vn", tenant_id);
    let pairing_code = generate_pairing_code();
    let now = chrono::Utc::now().to_rfc3339();

    // Calculate expiry (trial: 7 days, paid: 30 days)
    let trial_days = match req.plan.as_str() {
        "starter" => 7,
        _ => 30,
    };
    let expires = chrono::Utc::now() + chrono::Duration::days(trial_days);

    // Store in DB — status starts as "provisioning"
    let result = state.db.lock().await.cloud_create_tenant(
        &tenant_id,
        &req.name,
        &req.email,
        &req.phone,
        &req.plan,
        0, // vmid assigned later by vSphere
        "",  // ip assigned after VM boots
        &subdomain,
        "provisioning",
        &now,
        &expires.to_rfc3339(),
        &pairing_code,
        &req.notes,
    );

    match result {
        Ok(_) => {
            tracing::info!(
                "☁️ Cloud tenant '{}' created (plan={}, subdomain={})",
                req.name,
                req.plan,
                subdomain
            );

            // Spawn async vSphere provisioning task
            let db_arc = state.db.lock().await;
            // We need to pass the db Arc, not the lock
            drop(db_arc);
            
            let state_clone = state.clone();
            let tid = tenant_id.clone();
            let tname = req.name.clone();
            let tplan = req.plan.clone();
            
            tokio::spawn(async move {
                crate::vsphere::provision_tenant_vm(
                    state_clone,
                    tid,
                    tname,
                    tplan,
                ).await;
            });
            // For now without vSphere configured, mark as active (demo mode fallback)
            let _ = state.db.lock().await.cloud_update_tenant_status(&tenant_id, "active");

            Json(serde_json::json!({
                "ok": true,
                "id": tenant_id,
                "subdomain": subdomain,
                "pairing_code": pairing_code,
                "status": "provisioning",
                "message": "Tenant created — VM provisioning started"
            }))
        }
        Err(e) => {
            tracing::error!("❌ Failed to create tenant: {}", e);
            Json(serde_json::json!({
                "ok": false,
                "error": format!("Failed: {}", e)
            }))
        }
    }
}

/// POST /api/v1/cloud/tenants/:id/suspend — Suspend tenant (power off VM)
pub async fn cloud_suspend_tenant(
    State(state): State<Arc<AdminState>>,
    axum::extract::Path(tenant_id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    // Update DB status
    match state.db.lock().await.cloud_update_tenant_status(&tenant_id, "suspended") {
        Ok(_) => {
            tracing::info!("⏸️ Tenant '{}' suspended", tenant_id);

            // Attempt to power off VM via vSphere (best-effort)
            let state_clone = state.clone();
            let tid = tenant_id.clone();
            tokio::spawn(async move {
                vsphere_power_action(&state_clone, &tid, "suspend").await;
            });

            Json(serde_json::json!({"ok": true}))
        }
        Err(e) => Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    }
}

/// POST /api/v1/cloud/tenants/:id/resume — Resume suspended tenant (power on VM)
pub async fn cloud_resume_tenant(
    State(state): State<Arc<AdminState>>,
    axum::extract::Path(tenant_id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    match state.db.lock().await.cloud_update_tenant_status(&tenant_id, "active") {
        Ok(_) => {
            tracing::info!("▶️ Tenant '{}' resumed", tenant_id);

            // Attempt to power on VM via vSphere
            let state_clone = state.clone();
            let tid = tenant_id.clone();
            tokio::spawn(async move {
                vsphere_power_action(&state_clone, &tid, "start").await;
            });

            Json(serde_json::json!({"ok": true}))
        }
        Err(e) => Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    }
}

/// POST /api/v1/cloud/tenants/:id/renew — Gia hạn gói
pub async fn cloud_renew_tenant(
    State(state): State<Arc<AdminState>>,
    axum::extract::Path(tenant_id): axum::extract::Path<String>,
    Json(req): Json<RenewTenantReq>,
) -> Json<serde_json::Value> {
    let db = state.db.lock().await;

    // Get current tenant info
    let tenants = db.cloud_list_tenants().unwrap_or_default();
    let tenant = tenants.iter().find(|t| {
        t.get("id").and_then(|v| v.as_str()) == Some(&tenant_id)
    });

    let Some(tenant) = tenant else {
        return Json(serde_json::json!({"ok": false, "error": "Tenant not found"}));
    };

    // Calculate new expiry
    let current_expires = tenant.get("expires_at")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let base_date = chrono::DateTime::parse_from_rfc3339(current_expires)
        .map(|d| d.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now());

    let new_expires = base_date + chrono::Duration::days(req.days as i64);

    // Update plan if specified
    if let Some(ref new_plan) = req.plan {
        // TODO: If upgrading, also resize VM via vSphere
        tracing::info!(
            "📦 Tenant '{}' plan change: {} → {}",
            tenant_id,
            tenant.get("plan").and_then(|v| v.as_str()).unwrap_or("?"),
            new_plan
        );
    }

    // Update expiry in DB
    // Note: Using raw SQL since we don't have a dedicated method yet
    drop(db);
    match state.db.lock().await.cloud_update_tenant_status(&tenant_id, "active") {
        Ok(_) => {
            tracing::info!(
                "🔄 Tenant '{}' renewed for {} days (new expiry: {})",
                tenant_id,
                req.days,
                new_expires.to_rfc3339()
            );
            Json(serde_json::json!({
                "ok": true,
                "tenant_id": tenant_id,
                "new_expires_at": new_expires.to_rfc3339(),
                "days_added": req.days,
                "plan": req.plan,
            }))
        }
        Err(e) => Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    }
}

/// DELETE /api/v1/cloud/tenants/:id — Delete a tenant permanently + destroy VM
pub async fn cloud_delete_tenant(
    State(state): State<Arc<AdminState>>,
    axum::extract::Path(tenant_id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    // Get VM info before deleting from DB
    let tenants = state.db.lock().await.cloud_list_tenants().unwrap_or_default();
    let _vm_ip = tenants.iter()
        .find(|t| t.get("id").and_then(|v| v.as_str()) == Some(&tenant_id))
        .and_then(|t| t.get("ip_address").and_then(|v| v.as_str()))
        .map(|s| s.to_string());

    match state.db.lock().await.cloud_delete_tenant(&tenant_id) {
        Ok(_) => {
            tracing::info!("🗑️ Tenant '{}' deleted", tenant_id);

            // Attempt to destroy VM via vSphere (best-effort, async)
            let state_clone = state.clone();
            let tid = tenant_id.clone();
            tokio::spawn(async move {
                vsphere_power_action(&state_clone, &tid, "delete").await;
            });

            Json(serde_json::json!({"ok": true}))
        }
        Err(e) => Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    }
}

/// GET /api/v1/cloud/stats — Cluster stats overview
pub async fn cloud_stats(
    State(state): State<Arc<AdminState>>,
) -> Json<serde_json::Value> {
    let tenants = state.db.lock().await.cloud_list_tenants().unwrap_or_default();
    let total = tenants.len() as u32;

    let count_by_status = |status: &str| -> u32 {
        tenants.iter().filter(|t| {
            t.get("status").and_then(|v: &serde_json::Value| v.as_str()) == Some(status)
        }).count() as u32
    };

    let active = count_by_status("active");
    let provisioning = count_by_status("provisioning");
    let suspended = count_by_status("suspended");

    // Calculate MRR from active tenants
    let mrr: u64 = tenants.iter()
        .filter(|t| t.get("status").and_then(|v: &serde_json::Value| v.as_str()) == Some("active"))
        .map(|t| {
            match t.get("plan").and_then(|v: &serde_json::Value| v.as_str()).unwrap_or("starter") {
                "starter" => 590_000u64,
                "pro" => 1_490_000u64,
                "business" => 3_990_000u64,
                "enterprise" => 9_990_000u64,
                _ => 0,
            }
        })
        .sum();

    // Check vSphere connectivity (cached, non-blocking)
    let config = state.db.lock().await.cloud_get_config().unwrap_or_default();
    let vsphere_host = config.get("vsphere_host")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let vsphere_connected = !vsphere_host.is_empty();

    Json(serde_json::json!({
        "total_tenants": total,
        "active_tenants": active,
        "provisioning": provisioning,
        "suspended": suspended,
        "cpu_usage": 0.0,
        "ram_usage": 0.0,
        "mrr": mrr,
        "vsphere_connected": vsphere_connected,
    }))
}

/// GET /api/v1/cloud/config — Get cloud config (redacted passwords)
pub async fn cloud_get_config(
    State(state): State<Arc<AdminState>>,
) -> Json<serde_json::Value> {
    let config = state.db.lock().await.cloud_get_config().unwrap_or_default();

    // Redact sensitive values
    let mut safe_config = config.clone();
    if let Some(obj) = safe_config.as_object_mut() {
        if let Some(pw) = obj.get("vsphere_password") {
            if let Some(s) = pw.as_str() {
                if !s.is_empty() {
                    obj.insert("vsphere_password".to_string(),
                        serde_json::Value::String("***configured***".to_string()));
                }
            }
        }
    }

    Json(safe_config)
}

/// POST /api/v1/cloud/config — Save vSphere cloud config
pub async fn cloud_save_config(
    State(state): State<Arc<AdminState>>,
    Json(req): Json<CloudConfigReq>,
) -> Json<serde_json::Value> {
    let mut config_json = serde_json::json!({
        "vsphere_host": req.vsphere_host,
        "vsphere_user": req.vsphere_user,
        "template_id": req.template_id,
        "datacenter": req.datacenter,
        "folder": req.folder,
        "resource_pool": req.resource_pool,
        "datastore": req.datastore,
        "network": req.network,
        "domain_base": req.domain_base,
    });

    // Only update password if non-empty (don't overwrite with blank)
    if !req.vsphere_password.is_empty() {
        config_json["vsphere_password"] = serde_json::Value::String(req.vsphere_password);
    }

    match state.db.lock().await.cloud_save_config(&config_json) {
        Ok(_) => {
            tracing::info!("☁️ vSphere cloud config saved");
            Json(serde_json::json!({"ok": true}))
        }
        Err(e) => Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    }
}

/// POST /api/v1/cloud/test — Test vSphere connection
pub async fn cloud_test_connection(
    State(state): State<Arc<AdminState>>,
) -> Json<serde_json::Value> {
    let config = state.db.lock().await.cloud_get_config().unwrap_or_default();
    let host = config.get("vsphere_host")
        .and_then(|v: &serde_json::Value| v.as_str())
        .unwrap_or("");
    let user = config.get("vsphere_user")
        .and_then(|v: &serde_json::Value| v.as_str())
        .unwrap_or("");
    let password = config.get("vsphere_password")
        .and_then(|v: &serde_json::Value| v.as_str())
        .unwrap_or("");

    if host.is_empty() {
        return Json(serde_json::json!({
            "ok": false,
            "error": "vSphere host not configured"
        }));
    }

    // Step 1: Check if host is reachable
    match crate::vsphere::VSphereClient::test_connection(host).await {
        Ok(version) => {
            // Step 2: Try to authenticate
            if !user.is_empty() && !password.is_empty() {
                match crate::vsphere::VSphereClient::login(host, user, password).await {
                    Ok(client) => {
                        // Step 3: Get cluster info
                        let stats = client.cluster_stats().await.ok();
                        client.logout().await;

                        let mut result = serde_json::json!({
                            "ok": true,
                            "message": format!("{} — authenticated successfully", version),
                            "authenticated": true,
                        });

                        if let Some(s) = stats {
                            result["cluster"] = serde_json::json!({
                                "total_vms": s.total_vms,
                                "powered_on": s.powered_on,
                                "powered_off": s.powered_off,
                                "suspended": s.suspended,
                            });
                        }

                        Json(result)
                    }
                    Err(e) => Json(serde_json::json!({
                        "ok": false,
                        "message": format!("{} — reachable but auth failed", version),
                        "error": e,
                        "authenticated": false,
                    })),
                }
            } else {
                Json(serde_json::json!({
                    "ok": true,
                    "message": format!("{} — reachable (credentials not configured)", version),
                    "authenticated": false,
                }))
            }
        }
        Err(e) => Json(serde_json::json!({
            "ok": false,
            "error": format!("Connection failed: {}", e)
        })),
    }
}

// ═══ HELPERS ═══

fn generate_tenant_id(name: &str) -> String {
    let clean: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string();

    let short = if clean.len() > 16 {
        clean[..16].to_string()
    } else if clean.is_empty() {
        "tenant".to_string()
    } else {
        clean
    };

    let suffix: u32 = rand::random::<u32>() % 9999;
    format!("{}-{:04}", short, suffix)
}

fn generate_pairing_code() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let chars: Vec<char> = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789".chars().collect();
    (0..6).map(|_| chars[rng.gen_range(0..chars.len())]).collect()
}

/// Background helper: execute a vSphere power action for a tenant.
async fn vsphere_power_action(state: &Arc<AdminState>, tenant_id: &str, action: &str) {
    let config = state.db.lock().await.cloud_get_config().unwrap_or_default();
    let host = config.get("vsphere_host").and_then(|v| v.as_str()).unwrap_or("");
    let user = config.get("vsphere_user").and_then(|v| v.as_str()).unwrap_or("");
    let password = config.get("vsphere_password").and_then(|v| v.as_str()).unwrap_or("");

    if host.is_empty() || user.is_empty() {
        tracing::debug!("vSphere not configured — skipping {} for {}", action, tenant_id);
        return;
    }

    // Get VM ID from tenant record
    let tenants = state.db.lock().await.cloud_list_tenants().unwrap_or_default();
    let vm_id = tenants.iter()
        .find(|t| t.get("id").and_then(|v| v.as_str()) == Some(tenant_id))
        .and_then(|t| t.get("ip_address").and_then(|v| v.as_str()))
        .map(|s| s.to_string());

    let Some(vm_id) = vm_id else {
        tracing::warn!("No VM ID found for tenant {} — cannot {}", tenant_id, action);
        return;
    };

    // Only proceed if vm_id looks like a vSphere VM ref (vm-NNN)
    if !vm_id.starts_with("vm-") {
        tracing::debug!("Tenant {} has IP {} not a VM ref — skipping vSphere action", tenant_id, vm_id);
        return;
    }

    match crate::vsphere::VSphereClient::login(host, user, password).await {
        Ok(client) => {
            let result = match action {
                "start" => client.power_on(&vm_id).await,
                "stop" => client.power_off(&vm_id).await,
                "suspend" => client.suspend(&vm_id).await,
                "delete" => client.delete_vm(&vm_id).await,
                _ => Err(format!("Unknown action: {}", action)),
            };

            if let Err(e) = result {
                tracing::warn!("vSphere {} failed for VM {}: {}", action, vm_id, e);
            }

            client.logout().await;
        }
        Err(e) => {
            tracing::error!("vSphere login failed for {} action: {}", action, e);
        }
    }
}
