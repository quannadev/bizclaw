//! Tenant Health Monitor — detects crashed tenant processes and auto-restarts.
//!
//! Runs as a background thread (not async) because PlatformDb uses rusqlite
//! which is not Send. The monitor polls every N seconds.

use crate::db::PlatformDb;
use crate::tenant::TenantManager;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Configuration for the health monitor.
#[derive(Debug, Clone)]
pub struct HealthMonitorConfig {
    /// How often to check process health (seconds).
    pub check_interval_secs: u64,
    /// Maximum consecutive restart attempts before giving up.
    pub max_restart_attempts: u32,
    /// Cooldown between restart attempts (seconds).
    pub restart_cooldown_secs: u64,
    /// Path to bizclaw binary for spawning tenant processes.
    pub bizclaw_bin: String,
}

impl Default for HealthMonitorConfig {
    fn default() -> Self {
        Self {
            check_interval_secs: 30,
            max_restart_attempts: 3,
            restart_cooldown_secs: 10,
            bizclaw_bin: "bizclaw".into(),
        }
    }
}

/// Track restart attempts per tenant to prevent infinite restart loops.
struct RestartTracker {
    attempts: std::collections::HashMap<String, u32>,
}

impl RestartTracker {
    fn new() -> Self {
        Self {
            attempts: std::collections::HashMap::new(),
        }
    }

    fn record_attempt(&mut self, tenant_id: &str) -> u32 {
        let count = self.attempts.entry(tenant_id.into()).or_insert(0);
        *count += 1;
        *count
    }

    fn reset(&mut self, tenant_id: &str) {
        self.attempts.remove(tenant_id);
    }

    fn get_attempts(&self, tenant_id: &str) -> u32 {
        self.attempts.get(tenant_id).copied().unwrap_or(0)
    }
}

/// Check if a process with the given PID is still running.
fn is_process_alive(pid: u32) -> bool {
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Spawn the health monitor as a background OS thread.
///
/// Uses `std::thread::spawn` instead of `tokio::spawn` because:
/// - `PlatformDb` (rusqlite) is NOT Send/Sync
/// - All operations are synchronous (process checks, DB queries)
/// - A dedicated thread avoids polluting the tokio runtime
pub fn spawn_health_monitor(
    config: HealthMonitorConfig,
    db: Arc<Mutex<PlatformDb>>,
    manager: Arc<Mutex<TenantManager>>,
) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("health-monitor".into())
        .spawn(move || {
            let mut tracker = RestartTracker::new();
            let interval = Duration::from_secs(config.check_interval_secs);

            tracing::info!(
                "🏥 Health monitor started (interval={}s, max_restarts={})",
                config.check_interval_secs,
                config.max_restart_attempts
            );

            loop {
                std::thread::sleep(interval);
                run_health_check(&config, &db, &manager, &mut tracker);
            }
        })
        .expect("Failed to spawn health monitor thread")
}

/// Run a single health check cycle.
fn run_health_check(
    config: &HealthMonitorConfig,
    db: &Arc<Mutex<PlatformDb>>,
    manager: &Arc<Mutex<TenantManager>>,
    tracker: &mut RestartTracker,
) {
    // Get all tenants
    let tenants = {
        let db = match db.lock() {
            Ok(d) => d,
            Err(e) => {
                tracing::error!("[health] DB lock failed: {e}");
                return;
            }
        };
        match db.list_tenants() {
            Ok(t) => t,
            Err(e) => {
                tracing::error!("[health] Failed to list tenants: {e}");
                return;
            }
        }
    };

    for tenant in &tenants {
        if tenant.status != "running" {
            continue;
        }

        // Check if process is alive
        let (_is_tracked, proc_alive) = {
            let mgr = match manager.lock() {
                Ok(m) => m,
                Err(_) => continue,
            };
            let tracked = mgr.is_running(&tenant.id);
            let alive = if tracked {
                mgr.get_process(&tenant.id)
                    .map(|p| is_process_alive(p.pid))
                    .unwrap_or(false)
            } else if let Some(pid) = tenant.pid {
                is_process_alive(pid)
            } else {
                false // no PID, can't check
            };
            (tracked, alive)
        };

        if proc_alive {
            // Process is healthy — reset any restart trackers
            if tracker.get_attempts(&tenant.id) > 0 {
                tracker.reset(&tenant.id);
            }
            continue;
        }

        // Process is dead — attempt restart
        tracing::warn!("[health] 💀 Tenant '{}' has crashed!", tenant.slug);

        let attempts = tracker.get_attempts(&tenant.id);
        if attempts >= config.max_restart_attempts {
            tracing::error!(
                "[health] ⛔ '{}' exceeded max restart attempts ({}). Manual intervention required.",
                tenant.slug,
                config.max_restart_attempts
            );
            if let Ok(db) = db.lock() {
                db.update_tenant_status(&tenant.id, "error", None).ok();
                db.log_event(
                    "tenant_restart_exhausted",
                    "system",
                    &tenant.id,
                    Some(&format!("max_attempts={} exhausted", config.max_restart_attempts)),
                )
                .ok();
            }
            continue;
        }

        let attempt = tracker.record_attempt(&tenant.id);
        tracing::info!(
            "[health] 🔄 Auto-restarting '{}' (attempt {}/{})",
            tenant.slug,
            attempt,
            config.max_restart_attempts
        );

        // Log the crash
        if let Ok(db) = db.lock() {
            db.update_tenant_status(&tenant.id, "stopped", None).ok();
            db.log_event("tenant_crashed", "system", &tenant.id, Some("process died"))
                .ok();
        }

        // Cooldown before restart
        std::thread::sleep(Duration::from_secs(config.restart_cooldown_secs));

        // Restart the tenant
        let restart_result = {
            let mut mgr = match manager.lock() {
                Ok(m) => m,
                Err(_) => continue,
            };
            let db_guard = match db.lock() {
                Ok(d) => d,
                Err(_) => continue,
            };
            mgr.stop_tenant(&tenant.id).ok();
            mgr.start_tenant(tenant, &config.bizclaw_bin, &db_guard)
        };

        match restart_result {
            Ok(new_pid) => {
                if let Ok(db) = db.lock() {
                    db.update_tenant_status(&tenant.id, "running", Some(new_pid))
                        .ok();
                    db.log_event(
                        "tenant_auto_restarted",
                        "system",
                        &tenant.id,
                        Some(&format!("new_pid={}, attempt={}", new_pid, attempt)),
                    )
                    .ok();
                }
                tracker.reset(&tenant.id);
                tracing::info!(
                    "[health] ✅ '{}' restarted (pid={})",
                    tenant.slug,
                    new_pid
                );
            }
            Err(e) => {
                tracing::error!(
                    "[health] ❌ Restart failed for '{}': {}",
                    tenant.slug,
                    e
                );
                if let Ok(db) = db.lock() {
                    db.update_tenant_status(&tenant.id, "error", None).ok();
                }
            }
        }
    }

    // Periodic cleanup of old usage logs
    if let Ok(db) = db.lock() {
        if let Err(e) = db.cleanup_usage_logs(90) {
            tracing::warn!("[health] Usage log cleanup failed: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_restart_tracker() {
        let mut tracker = RestartTracker::new();
        assert_eq!(tracker.get_attempts("t1"), 0);

        assert_eq!(tracker.record_attempt("t1"), 1);
        assert_eq!(tracker.record_attempt("t1"), 2);
        assert_eq!(tracker.get_attempts("t1"), 2);

        tracker.reset("t1");
        assert_eq!(tracker.get_attempts("t1"), 0);
    }

    #[test]
    fn test_default_config() {
        let cfg = HealthMonitorConfig::default();
        assert_eq!(cfg.check_interval_secs, 30);
        assert_eq!(cfg.max_restart_attempts, 3);
        assert_eq!(cfg.restart_cooldown_secs, 10);
    }

    #[test]
    fn test_is_process_alive_nonexistent() {
        assert!(!is_process_alive(999_999_999));
    }

    #[test]
    fn test_is_process_alive_self() {
        let pid = std::process::id();
        assert!(is_process_alive(pid));
    }
}
