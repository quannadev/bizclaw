//! VMware vSphere REST API Client — VM lifecycle management for BizClaw Cloud.
//!
//! Uses the vSphere Automation REST API (vCenter 7.0+/8.0+):
//! - Session auth: POST /api/session
//! - VM operations: /api/vcenter/vm/*
//! - Guest identity: /api/vcenter/vm/{vm}/guest/identity
//!
//! Requires vCenter Server (standalone ESXi has limited REST API).
//! VMware Tools must be installed in guest templates for IP retrieval.

use serde::{Deserialize, Serialize};
use std::time::Duration;

// ════════════════════════════════════════════════════════════════
// DATA MODELS
// ════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmInfo {
    pub vm_id: String,
    pub name: String,
    pub power_state: String,
    pub num_cpus: u32,
    pub memory_mb: u64,
    pub ip_address: Option<String>,
    pub guest_os: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmSummary {
    pub vm: String,
    pub name: String,
    pub power_state: String,
    pub cpu_count: Option<u32>,
    pub memory_size_mib: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateSummary {
    pub vm: String,
    pub name: String,
    pub guest_os: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CloneSpec {
    pub name: String,
    pub folder: String,
    pub resource_pool: String,
    pub datastore: String,
    pub num_cpus: u32,
    pub memory_mb: u64,
    pub network: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterStats {
    pub total_vms: u32,
    pub powered_on: u32,
    pub powered_off: u32,
    pub suspended: u32,
    pub total_cpu_mhz: Option<u64>,
    pub used_cpu_mhz: Option<u64>,
    pub total_memory_mb: Option<u64>,
    pub used_memory_mb: Option<u64>,
}

/// Plan-to-resource mapping for VM provisioning.
#[derive(Debug, Clone)]
pub struct PlanSpec {
    pub num_cpus: u32,
    pub memory_mb: u64,
    pub disk_gb: u32,
}

impl PlanSpec {
    pub fn from_plan(plan: &str) -> Self {
        match plan {
            "pro" => PlanSpec { num_cpus: 2, memory_mb: 4096, disk_gb: 60 },
            "business" => PlanSpec { num_cpus: 4, memory_mb: 8192, disk_gb: 120 },
            "enterprise" => PlanSpec { num_cpus: 8, memory_mb: 16384, disk_gb: 250 },
            _ /* starter */ => PlanSpec { num_cpus: 1, memory_mb: 2048, disk_gb: 30 },
        }
    }
}

// ════════════════════════════════════════════════════════════════
// VSPHERE CLIENT
// ════════════════════════════════════════════════════════════════

/// vSphere REST API client for VM lifecycle management.
///
/// Flow: login() → clone_vm() → power_on() → wait_for_ip() → done
pub struct VSphereClient {
    base_url: String,
    session_id: String,
    http: reqwest::Client,
}

impl VSphereClient {
    /// Authenticate and create a new vSphere client.
    ///
    /// POST /api/session with Basic auth → returns session token.
    pub async fn login(host: &str, user: &str, password: &str) -> Result<Self, String> {
        let http = reqwest::Client::builder()
            .danger_accept_invalid_certs(true) // vCenter often uses self-signed certs
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| format!("HTTP client error: {e}"))?;

        let base_url = host.trim_end_matches('/').to_string();
        let url = format!("{}/api/session", base_url);

        let resp = http
            .post(&url)
            .basic_auth(user, Some(password))
            .send()
            .await
            .map_err(|e| format!("vSphere login failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!(
                "vSphere auth failed (HTTP {}): {}",
                status,
                body.chars().take(200).collect::<String>()
            ));
        }

        // Session ID is returned as a plain JSON string (quoted)
        let session_id = resp
            .text()
            .await
            .map_err(|e| format!("Failed to read session: {e}"))?
            .trim_matches('"')
            .to_string();

        if session_id.is_empty() {
            return Err("Empty session token from vSphere".into());
        }

        tracing::info!("✅ vSphere session established: {}...{}", &session_id[..4.min(session_id.len())], &session_id[session_id.len().saturating_sub(4)..]);

        Ok(Self {
            base_url,
            session_id,
            http,
        })
    }

    /// Common helper — add session header to requests.
    fn auth_header(&self) -> (&str, &str) {
        ("vmware-api-session-id", &self.session_id)
    }

    // ── VM LIFECYCLE ─────────────────────────────────────────

    /// Clone a VM from an existing template.
    ///
    /// POST /api/vcenter/vm with source = template VM ID.
    /// Returns the new VM's ID (e.g., "vm-123").
    pub async fn clone_vm(
        &self,
        template_id: &str,
        spec: &CloneSpec,
    ) -> Result<String, String> {
        let url = format!("{}/api/vcenter/vm", self.base_url);

        let placement = serde_json::json!({
            "folder": spec.folder,
            "resource_pool": spec.resource_pool,
            "datastore": spec.datastore,
        });

        // Only include network if specified
        let body = serde_json::json!({
            "spec": {
                "name": spec.name,
                "guest_OS": "ubuntu64Guest",
                "placement": placement,
                "source": template_id,
                "hardware_customization": {
                    "cpu_update": {
                        "num_cpus": spec.num_cpus,
                        "num_cores_per_socket": 1,
                    },
                    "memory_update": {
                        "memory": spec.memory_mb,
                    },
                },
            }
        });

        tracing::info!(
            "☁️ Cloning VM from template {} → '{}' ({}vCPU, {}MB RAM)",
            template_id,
            spec.name,
            spec.num_cpus,
            spec.memory_mb
        );

        let resp = self
            .http
            .post(&url)
            .header(self.auth_header().0, self.auth_header().1)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Clone request failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err_body = resp.text().await.unwrap_or_default();
            return Err(format!(
                "VM clone failed (HTTP {}): {}",
                status,
                err_body.chars().take(500).collect::<String>()
            ));
        }

        // Response is the new VM ID as a plain string
        let vm_id = resp
            .text()
            .await
            .map_err(|e| format!("Failed to read VM ID: {e}"))?
            .trim_matches('"')
            .to_string();

        tracing::info!("✅ VM cloned: {}", vm_id);
        Ok(vm_id)
    }

    /// Power on a VM.
    ///
    /// POST /api/vcenter/vm/{vm}/power?action=start
    pub async fn power_on(&self, vm_id: &str) -> Result<(), String> {
        self.power_action(vm_id, "start").await
    }

    /// Power off a VM (hard stop).
    ///
    /// POST /api/vcenter/vm/{vm}/power?action=stop
    pub async fn power_off(&self, vm_id: &str) -> Result<(), String> {
        self.power_action(vm_id, "stop").await
    }

    /// Suspend a VM (save state).
    ///
    /// POST /api/vcenter/vm/{vm}/power?action=suspend
    pub async fn suspend(&self, vm_id: &str) -> Result<(), String> {
        self.power_action(vm_id, "suspend").await
    }

    /// Reset a VM.
    ///
    /// POST /api/vcenter/vm/{vm}/power?action=reset
    pub async fn reset(&self, vm_id: &str) -> Result<(), String> {
        self.power_action(vm_id, "reset").await
    }

    /// Generic power action.
    async fn power_action(&self, vm_id: &str, action: &str) -> Result<(), String> {
        let url = format!(
            "{}/api/vcenter/vm/{}/power?action={}",
            self.base_url, vm_id, action
        );

        let resp = self
            .http
            .post(&url)
            .header(self.auth_header().0, self.auth_header().1)
            .send()
            .await
            .map_err(|e| format!("Power {} failed: {}", action, e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            // 400 "already in desired state" is not an error
            if status.as_u16() == 400 && body.contains("already") {
                tracing::debug!("VM {} already in '{}' state", vm_id, action);
                return Ok(());
            }
            return Err(format!(
                "Power {} failed (HTTP {}): {}",
                action,
                status,
                body.chars().take(300).collect::<String>()
            ));
        }

        tracing::info!("⚡ VM {} power action: {}", vm_id, action);
        Ok(())
    }

    /// Delete a VM permanently.
    ///
    /// DELETE /api/vcenter/vm/{vm}
    /// VM must be powered off first.
    pub async fn delete_vm(&self, vm_id: &str) -> Result<(), String> {
        // Ensure VM is powered off before deletion
        let _ = self.power_off(vm_id).await;
        tokio::time::sleep(Duration::from_secs(3)).await;

        let url = format!("{}/api/vcenter/vm/{}", self.base_url, vm_id);

        let resp = self
            .http
            .delete(&url)
            .header(self.auth_header().0, self.auth_header().1)
            .send()
            .await
            .map_err(|e| format!("Delete VM failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!(
                "Delete VM failed (HTTP {}): {}",
                status,
                body.chars().take(300).collect::<String>()
            ));
        }

        tracing::info!("🗑️ VM {} deleted", vm_id);
        Ok(())
    }

    // ── VM INFO ──────────────────────────────────────────────

    /// Get VM IP address from VMware Tools guest identity.
    ///
    /// GET /api/vcenter/vm/{vm}/guest/identity
    /// Requires VMware Tools running in guest.
    pub async fn get_vm_ip(&self, vm_id: &str) -> Result<Option<String>, String> {
        let url = format!(
            "{}/api/vcenter/vm/{}/guest/identity",
            self.base_url, vm_id
        );

        let resp = self
            .http
            .get(&url)
            .header(self.auth_header().0, self.auth_header().1)
            .send()
            .await
            .map_err(|e| format!("Guest identity request failed: {e}"))?;

        if !resp.status().is_success() {
            // 503 = VMware Tools not ready yet
            return Ok(None);
        }

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Parse guest identity failed: {e}"))?;

        let ip = body["ip_address"]
            .as_str()
            .map(|s| s.to_string());

        Ok(ip)
    }

    /// Wait for VM to boot and report its IP address.
    ///
    /// Polls guest identity every 5 seconds, up to `timeout_secs`.
    pub async fn wait_for_ip(
        &self,
        vm_id: &str,
        timeout_secs: u64,
    ) -> Result<String, String> {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);

        loop {
            if tokio::time::Instant::now() >= deadline {
                return Err(format!(
                    "Timeout waiting for VM {} IP after {}s",
                    vm_id, timeout_secs
                ));
            }

            if let Ok(Some(ip)) = self.get_vm_ip(vm_id).await {
                if !ip.is_empty() && ip != "0.0.0.0" {
                    tracing::info!("🌐 VM {} got IP: {}", vm_id, ip);
                    return Ok(ip);
                }
            }

            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }

    /// Get detailed VM info.
    ///
    /// GET /api/vcenter/vm/{vm}
    pub async fn get_vm_info(&self, vm_id: &str) -> Result<VmInfo, String> {
        let url = format!("{}/api/vcenter/vm/{}", self.base_url, vm_id);

        let resp = self
            .http
            .get(&url)
            .header(self.auth_header().0, self.auth_header().1)
            .send()
            .await
            .map_err(|e| format!("Get VM info failed: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            return Err(format!("Get VM info failed (HTTP {})", status));
        }

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Parse VM info failed: {e}"))?;

        let ip = self.get_vm_ip(vm_id).await.unwrap_or(None);

        Ok(VmInfo {
            vm_id: vm_id.to_string(),
            name: body["name"].as_str().unwrap_or("").to_string(),
            power_state: body["power_state"]
                .as_str()
                .unwrap_or("UNKNOWN")
                .to_string(),
            num_cpus: body["cpu"]["count"].as_u64().unwrap_or(0) as u32,
            memory_mb: body["memory"]["size_MiB"].as_u64().unwrap_or(0),
            ip_address: ip,
            guest_os: body["guest_OS"].as_str().map(|s| s.to_string()),
        })
    }

    /// List all VMs in the datacenter.
    ///
    /// GET /api/vcenter/vm
    pub async fn list_vms(&self) -> Result<Vec<VmSummary>, String> {
        let url = format!("{}/api/vcenter/vm", self.base_url);

        let resp = self
            .http
            .get(&url)
            .header(self.auth_header().0, self.auth_header().1)
            .send()
            .await
            .map_err(|e| format!("List VMs failed: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!(
                "List VMs failed (HTTP {})",
                resp.status()
            ));
        }

        let vms: Vec<VmSummary> = resp
            .json()
            .await
            .map_err(|e| format!("Parse VM list failed: {e}"))?;

        Ok(vms)
    }

    // ── CLUSTER STATS ────────────────────────────────────────

    /// Get cluster resource summary by listing all VMs and aggregating.
    pub async fn cluster_stats(&self) -> Result<ClusterStats, String> {
        let vms = self.list_vms().await?;

        let total = vms.len() as u32;
        let powered_on = vms
            .iter()
            .filter(|v| v.power_state == "POWERED_ON")
            .count() as u32;
        let powered_off = vms
            .iter()
            .filter(|v| v.power_state == "POWERED_OFF")
            .count() as u32;
        let suspended = vms
            .iter()
            .filter(|v| v.power_state == "SUSPENDED")
            .count() as u32;

        Ok(ClusterStats {
            total_vms: total,
            powered_on,
            powered_off,
            suspended,
            total_cpu_mhz: None,  // Requires host API
            used_cpu_mhz: None,
            total_memory_mb: None,
            used_memory_mb: None,
        })
    }

    // ── UTILITY ──────────────────────────────────────────────

    /// Test connection to vSphere — returns version string.
    pub async fn test_connection(host: &str) -> Result<String, String> {
        let http = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| format!("HTTP client error: {e}"))?;

        let url = format!("{}/api/vcenter/system/version", host.trim_end_matches('/'));

        // This endpoint doesn't require auth on some versions
        let resp = http
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Connection failed: {e}"))?;

        if resp.status().is_success() {
            let body: serde_json::Value = resp.json().await.unwrap_or_default();
            let version = body["version"].as_str().unwrap_or("unknown");
            let build = body["build"].as_str().unwrap_or("");
            Ok(format!("vSphere {} (build {})", version, build))
        } else {
            // Try the old /rest/appliance/system/version endpoint
            let url2 = format!(
                "{}/rest/appliance/system/version",
                host.trim_end_matches('/')
            );
            match http.get(&url2).send().await {
                Ok(resp2) if resp2.status().is_success() => {
                    let body: serde_json::Value = resp2.json().await.unwrap_or_default();
                    let version = body["value"]["version"]
                        .as_str()
                        .unwrap_or("reachable");
                    Ok(format!("vSphere {}", version))
                }
                _ => {
                    // Last fallback: just check if port is open
                    Err(format!(
                        "vSphere API at {} returned HTTP {}",
                        host,
                        resp.status()
                    ))
                }
            }
        }
    }

    /// Logout / invalidate the session.
    pub async fn logout(&self) {
        let url = format!("{}/api/session", self.base_url);
        let _ = self
            .http
            .delete(&url)
            .header(self.auth_header().0, self.auth_header().1)
            .send()
            .await;
    }
}

impl Drop for VSphereClient {
    fn drop(&mut self) {
        // Note: Can't async in Drop. Session will eventually expire on vCenter side.
        tracing::debug!("VSphereClient dropped — session {} will expire on server", &self.session_id[..8.min(self.session_id.len())]);
    }
}

// ════════════════════════════════════════════════════════════════
// PROVISIONER — Orchestrates the full VM lifecycle
// ════════════════════════════════════════════════════════════════

/// Full VM provisioning flow: clone → power on → wait IP → update DB.
///
/// Runs as a background task via `tokio::spawn`.
pub async fn provision_tenant_vm(
    state: std::sync::Arc<crate::admin::AdminState>,
    tenant_id: String,
    tenant_name: String,
    plan: String,
) {
    tracing::info!(
        "☁️ Starting VM provisioning for tenant '{}' (plan: {})",
        tenant_name,
        plan
    );

    // 1. Load vSphere config from DB
    let config = {
        let db_lock = state.db.lock().await;
        db_lock.cloud_get_config().unwrap_or_default()
    };

    let host = config["vsphere_host"].as_str().unwrap_or("");
    let user = config["vsphere_user"].as_str().unwrap_or("");
    let password = config["vsphere_password"].as_str().unwrap_or("");
    let template_id = config["template_id"].as_str().unwrap_or("");
    let folder = config["folder"].as_str().unwrap_or("BizClaw-Tenants");
    let resource_pool = config["resource_pool"].as_str().unwrap_or("");
    let datastore = config["datastore"].as_str().unwrap_or("datastore1");
    let network = config["network"].as_str().map(|s| s.to_string());

    if host.is_empty() || user.is_empty() || template_id.is_empty() {
        tracing::error!("❌ vSphere not configured — cannot provision VM for tenant {}", tenant_id);
        let db_lock = state.db.lock().await;
        let _ = db_lock.cloud_update_tenant_status(&tenant_id, "error");
        return;
    }

    // 2. Login to vSphere
    let client = match VSphereClient::login(host, user, password).await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("❌ vSphere login failed: {}", e);
            let db_lock = state.db.lock().await;
            let _ = db_lock.cloud_update_tenant_status(&tenant_id, "error");
            return;
        }
    };

    // 3. Clone VM from template
    let plan_spec = PlanSpec::from_plan(&plan);
    let vm_name = format!("bizclaw-{}", tenant_id);

    let clone_spec = CloneSpec {
        name: vm_name.clone(),
        folder: folder.to_string(),
        resource_pool: resource_pool.to_string(),
        datastore: datastore.to_string(),
        num_cpus: plan_spec.num_cpus,
        memory_mb: plan_spec.memory_mb,
        network,
    };

    let vm_id = match client.clone_vm(template_id, &clone_spec).await {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("❌ VM clone failed for tenant {}: {}", tenant_id, e);
            let db_lock = state.db.lock().await;
            let _ = db_lock.cloud_update_tenant_status(&tenant_id, "error");
            client.logout().await;
            return;
        }
    };

    // Update DB with VM ID
    {
        let db_lock = state.db.lock().await;
        let _ = db_lock.cloud_update_tenant_vm(&tenant_id, 0, &vm_id); // Store vm_id in ip_address temporarily
        let _ = db_lock.cloud_update_tenant_status(&tenant_id, "booting");
    }

    // 4. Power on VM
    if let Err(e) = client.power_on(&vm_id).await {
        tracing::error!("❌ VM power on failed: {}", e);
        let db_lock = state.db.lock().await;
        let _ = db_lock.cloud_update_tenant_status(&tenant_id, "error");
        client.logout().await;
        return;
    }

    // 5. Wait for IP (up to 3 minutes)
    match client.wait_for_ip(&vm_id, 180).await {
        Ok(ip) => {
            tracing::info!(
                "✅ Tenant '{}' VM ready: {} (IP: {})",
                tenant_name,
                vm_id,
                ip
            );
            let db_lock = state.db.lock().await;
            let _ = db_lock.cloud_update_tenant_vm(&tenant_id, 0, &ip);
            let _ = db_lock.cloud_update_tenant_status(&tenant_id, "active");
        }
        Err(e) => {
            tracing::warn!(
                "⚠️ VM {} booted but no IP yet: {} — marking as booting",
                vm_id, e
            );
            // Don't mark as error — VM might just need more time
            // Background health checker will update status later
        }
    }

    client.logout().await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_spec() {
        let starter = PlanSpec::from_plan("starter");
        assert_eq!(starter.num_cpus, 1);
        assert_eq!(starter.memory_mb, 2048);
        assert_eq!(starter.disk_gb, 30);

        let pro = PlanSpec::from_plan("pro");
        assert_eq!(pro.num_cpus, 2);
        assert_eq!(pro.memory_mb, 4096);

        let biz = PlanSpec::from_plan("business");
        assert_eq!(biz.num_cpus, 4);
        assert_eq!(biz.memory_mb, 8192);

        let ent = PlanSpec::from_plan("enterprise");
        assert_eq!(ent.num_cpus, 8);
    }

    #[test]
    fn test_unknown_plan_defaults_to_starter() {
        let spec = PlanSpec::from_plan("unknown");
        assert_eq!(spec.num_cpus, 1);
        assert_eq!(spec.memory_mb, 2048);
    }
}
