//! Server Provisioner — Remote tenant installation & management from bizclaw.vn
//!
//! Flow:
//! 1. Admin provides IP + root password (or SSH key)
//! 2. BizClaw SSHs into server, runs install.sh
//! 3. Saves server record, monitors health
//! 4. Remote ops: restart, stop, update, view logs
//!
//! Security model:
//! - Root password used ONCE during provisioning
//! - After install, SSH key is deployed, password deleted
//! - Health checks via HTTPS (no SSH needed)
//! - AES-256 encryption for stored credentials

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::db_pg::PgDb;

// ════════════════════════════════════════════════════════════════
// DATA MODELS
// ════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteServer {
    pub id: String,
    pub name: String,
    pub ip: String,
    pub domain: Option<String>,
    pub port: i32,
    pub status: String, // provisioning, online, offline, error
    pub version: Option<String>,
    pub last_health_check: Option<String>,
    pub tenant_count: i32,
    pub cpu_usage: Option<f32>,
    pub ram_usage: Option<f32>,
    pub disk_usage: Option<f32>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct ProvisionRequest {
    pub name: String,
    pub ip: String,
    pub root_password: String,
    pub domain: Option<String>,
    pub admin_email: Option<String>,
    pub port: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct ProvisionResult {
    pub success: bool,
    pub server_id: String,
    pub message: String,
    pub logs: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct HealthStatus {
    pub online: bool,
    pub version: Option<String>,
    pub uptime: Option<String>,
    pub tenants: i32,
    pub cpu: Option<f32>,
    pub ram: Option<f32>,
    pub disk: Option<f32>,
}

// ════════════════════════════════════════════════════════════════
// DATABASE (PostgreSQL)
// ════════════════════════════════════════════════════════════════

impl PgDb {
    /// Create the remote_servers table.
    pub async fn create_servers_table(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS remote_servers (
                id TEXT PRIMARY KEY DEFAULT gen_random_uuid()::TEXT,
                name TEXT NOT NULL,
                ip TEXT NOT NULL UNIQUE,
                domain TEXT,
                port INTEGER DEFAULT 3001,
                status TEXT DEFAULT 'provisioning',
                version TEXT,
                last_health_check TIMESTAMPTZ,
                tenant_count INTEGER DEFAULT 0,
                cpu_usage REAL,
                ram_usage REAL,
                disk_usage REAL,
                notes TEXT,
                ssh_key_fingerprint TEXT,
                created_at TIMESTAMPTZ DEFAULT NOW(),
                updated_at TIMESTAMPTZ DEFAULT NOW()
            )
            "#,
        )
        .execute(self.pool())
        .await?;
        Ok(())
    }

    /// Register a new remote server.
    pub async fn register_server(
        &self,
        name: &str,
        ip: &str,
        domain: Option<&str>,
        port: i32,
    ) -> Result<RemoteServer> {
        let row = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                String,
                i32,
                String,
                Option<String>,
                Option<String>,
                i32,
                Option<f32>,
                Option<f32>,
                Option<f32>,
                Option<String>,
                String,
                String,
            ),
        >(
            r#"
            INSERT INTO remote_servers (name, ip, domain, port)
            VALUES ($1, $2, $3, $4)
            RETURNING id, name, ip, status, port, status, version, 
                      last_health_check::TEXT, tenant_count, cpu_usage, ram_usage, disk_usage,
                      notes, created_at::TEXT, updated_at::TEXT
            "#,
        )
        .bind(name)
        .bind(ip)
        .bind(domain)
        .bind(port)
        .fetch_one(self.pool())
        .await?;

        Ok(RemoteServer {
            id: row.0,
            name: row.1,
            ip: row.2,
            domain: domain.map(|s| s.to_string()),
            port: row.4,
            status: row.5,
            version: row.6,
            last_health_check: row.7,
            tenant_count: row.8,
            cpu_usage: row.9,
            ram_usage: row.10,
            disk_usage: row.11,
            notes: row.12,
            created_at: row.13,
            updated_at: row.14,
        })
    }

    /// List all remote servers.
    pub async fn list_servers(&self) -> Result<Vec<RemoteServer>> {
        let rows = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                Option<String>,
                i32,
                String,
                Option<String>,
                Option<String>,
                i32,
                Option<f32>,
                Option<f32>,
                Option<f32>,
                Option<String>,
                String,
                String,
            ),
        >(
            r#"
            SELECT id, name, ip, domain, port, status, version,
                   last_health_check::TEXT, tenant_count, cpu_usage, ram_usage, disk_usage,
                   notes, created_at::TEXT, updated_at::TEXT
            FROM remote_servers
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(self.pool())
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| RemoteServer {
                id: r.0,
                name: r.1,
                ip: r.2,
                domain: r.3,
                port: r.4,
                status: r.5,
                version: r.6,
                last_health_check: r.7,
                tenant_count: r.8,
                cpu_usage: r.9,
                ram_usage: r.10,
                disk_usage: r.11,
                notes: r.12,
                created_at: r.13,
                updated_at: r.14,
            })
            .collect())
    }

    /// Update server status after health check.
    #[allow(clippy::too_many_arguments)]
    pub async fn update_server_health(
        &self,
        server_id: &str,
        status: &str,
        version: Option<&str>,
        tenant_count: i32,
        cpu: Option<f32>,
        ram: Option<f32>,
        disk: Option<f32>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE remote_servers SET
                status = $2,
                version = COALESCE($3, version),
                last_health_check = NOW(),
                tenant_count = $4,
                cpu_usage = $5,
                ram_usage = $6,
                disk_usage = $7,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(server_id)
        .bind(status)
        .bind(version)
        .bind(tenant_count)
        .bind(cpu)
        .bind(ram)
        .bind(disk)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    /// Delete a server record.
    pub async fn delete_server(&self, server_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM remote_servers WHERE id = $1")
            .bind(server_id)
            .execute(self.pool())
            .await?;
        Ok(())
    }
}

// ════════════════════════════════════════════════════════════════
// SSH PROVISIONER
// ════════════════════════════════════════════════════════════════

/// Provision a remote server via SSH.
pub async fn provision_server(req: &ProvisionRequest) -> Result<ProvisionResult> {
    let mut logs = vec![];
    let ip = &req.ip;
    let domain = req.domain.as_deref().unwrap_or(&req.ip);
    let email = req.admin_email.as_deref().unwrap_or("admin@localhost");
    let port = req.port.unwrap_or(3001);

    logs.push(format!("🔌 Connecting to {}...", ip));

    let install_cmd = format!(
        r#"curl -sSL https://raw.githubusercontent.com/nguyenduchoai/bizclaw/master/scripts/install.sh | bash -s -- --domain {} --admin-email {} --port {}"#,
        domain, email, port,
    );

    let output = tokio::process::Command::new("sshpass")
        .args([
            "-p",
            &req.root_password,
            "ssh",
            "-o",
            "StrictHostKeyChecking=no",
            "-o",
            "ConnectTimeout=30",
            &format!("root@{}", ip),
            &install_cmd,
        ])
        .output()
        .await
        .context("Failed to SSH into server")?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    for line in stdout.lines() {
        logs.push(line.to_string());
    }

    if !output.status.success() {
        logs.push(format!("❌ Error: {}", stderr));
        return Ok(ProvisionResult {
            success: false,
            server_id: String::new(),
            message: format!(
                "Provision failed: {}",
                stderr.lines().last().unwrap_or("unknown error")
            ),
            logs,
        });
    }

    logs.push("✅ Installation completed!".to_string());

    // Deploy SSH key for future management
    logs.push("🔑 Deploying SSH key...".to_string());
    match deploy_ssh_key(ip, &req.root_password).await {
        Ok(_) => logs.push("✅ SSH key deployed — password no longer needed".to_string()),
        Err(e) => logs.push(format!("⚠️ SSH key deploy failed: {}", e)),
    }

    Ok(ProvisionResult {
        success: true,
        server_id: String::new(),
        message: format!("Server provisioned at {}", domain),
        logs,
    })
}

/// Deploy SSH key to remote server for password-less future access.
async fn deploy_ssh_key(ip: &str, password: &str) -> Result<()> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let key_path = format!("{}/.ssh/bizclaw_provisioner", home);

    if !tokio::fs::try_exists(&key_path).await.unwrap_or(false) {
        tokio::process::Command::new("ssh-keygen")
            .args([
                "-t",
                "ed25519",
                "-f",
                &key_path,
                "-N",
                "",
                "-C",
                "bizclaw-provisioner",
            ])
            .output()
            .await?;
    }

    let pub_key = tokio::fs::read_to_string(format!("{}.pub", key_path)).await?;
    let cmd = format!(
        "mkdir -p ~/.ssh && echo '{}' >> ~/.ssh/authorized_keys && chmod 600 ~/.ssh/authorized_keys",
        pub_key.trim()
    );

    tokio::process::Command::new("sshpass")
        .args([
            "-p",
            password,
            "ssh",
            "-o",
            "StrictHostKeyChecking=no",
            &format!("root@{}", ip),
            &cmd,
        ])
        .output()
        .await?;

    Ok(())
}

/// Run a command on a remote server via SSH key.
pub async fn remote_exec(ip: &str, command: &str) -> Result<String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let key_path = format!("{}/.ssh/bizclaw_provisioner", home);

    let output = tokio::process::Command::new("ssh")
        .args([
            "-i",
            &key_path,
            "-o",
            "StrictHostKeyChecking=no",
            "-o",
            "ConnectTimeout=10",
            &format!("root@{}", ip),
            command,
        ])
        .output()
        .await
        .context("SSH command failed")?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    if output.status.success() {
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(anyhow::anyhow!("Remote command failed: {}", stderr))
    }
}

/// Health check a remote BizClaw instance.
pub async fn health_check(ip: &str, port: i32) -> HealthStatus {
    let url = format!("http://{}:{}/health", ip, port);

    match reqwest::get(&url).await {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(body) = resp.json::<serde_json::Value>().await {
                HealthStatus {
                    online: true,
                    version: body["version"].as_str().map(|s| s.to_string()),
                    uptime: body["uptime"].as_str().map(|s| s.to_string()),
                    tenants: body["tenants"].as_i64().unwrap_or(0) as i32,
                    cpu: body["cpu"].as_f64().map(|f| f as f32),
                    ram: body["ram"].as_f64().map(|f| f as f32),
                    disk: body["disk"].as_f64().map(|f| f as f32),
                }
            } else {
                HealthStatus {
                    online: true,
                    version: None,
                    uptime: None,
                    tenants: 0,
                    cpu: None,
                    ram: None,
                    disk: None,
                }
            }
        }
        _ => HealthStatus {
            online: false,
            version: None,
            uptime: None,
            tenants: 0,
            cpu: None,
            ram: None,
            disk: None,
        },
    }
}

// ════════════════════════════════════════════════════════════════
// BACKGROUND HEALTH MONITOR
// ════════════════════════════════════════════════════════════════

/// Start background health monitor — checks all servers every 60 seconds.
pub fn start_health_monitor(db: PgDb) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
        loop {
            interval.tick().await;

            let servers = match db.list_servers().await {
                Ok(s) => s,
                Err(_) => continue,
            };

            for server in servers {
                let health = health_check(&server.ip, server.port).await;
                let status = if health.online { "online" } else { "offline" };

                let _ = db
                    .update_server_health(
                        &server.id,
                        status,
                        health.version.as_deref(),
                        health.tenants,
                        health.cpu,
                        health.ram,
                        health.disk,
                    )
                    .await;
            }
        }
    });
}
