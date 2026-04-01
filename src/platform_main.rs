//! # BizClaw Platform — Multi-Tenant Admin Server
//!
//! Manages multiple BizClaw AI Agent instances on a single VPS.
//! Provides admin dashboard, REST API, tenant lifecycle, and audit logging.
//!
//! Usage:
//!   bizclaw-platform                     # Start admin server (default port 3000)
//!   bizclaw-platform --port 8080         # Custom port
//!   bizclaw-platform --init-admin        # Create default admin user

use anyhow::Result;
use clap::Parser;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(
    name = "bizclaw-platform",
    version,
    about = "🏢 BizClaw Platform — Multi-Tenant Admin Server"
)]
struct Cli {
    /// Admin panel port
    #[arg(short, long, default_value = "3000")]
    port: u16,

    /// Path to bizclaw binary (for starting tenants)
    #[arg(long, default_value = "bizclaw")]
    bizclaw_bin: String,

    /// Base port for tenant instances
    #[arg(long, default_value = "10001")]
    base_port: u16,

    /// Data directory
    #[arg(long, default_value = "~/.bizclaw/tenants")]
    data_dir: String,

    /// Database path
    #[arg(long, default_value = "~/.bizclaw/platform.db")]
    db_path: String,

    /// JWT secret (recommended: set JWT_SECRET env var)
    #[arg(long, default_value = "auto")]
    jwt_secret: String,

    /// Domain name for this platform instance
    #[arg(long, default_value = "bizclaw.vn")]
    domain: String,

    /// Create default admin user and exit
    #[arg(long)]
    init_admin: bool,

    /// Admin email — override with BIZCLAW_ADMIN_EMAIL env var
    #[arg(long, default_value = "admin@localhost")]
    admin_email: String,

    /// Admin password (used with --init-admin, or set BIZCLAW_ADMIN_PASSWORD env var)
    #[arg(long, default_value = "auto")]
    admin_password: String,

    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,
}

fn expand_path(p: &str) -> String {
    shellexpand::tilde(p).to_string()
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let filter = if cli.verbose {
        "bizclaw_platform=debug,tower_http=debug"
    } else {
        "bizclaw_platform=info"
    };
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(filter)),
        )
        .with_target(false)
        .init();

    // Expand paths
    let data_dir = expand_path(&cli.data_dir);
    let db_path = expand_path(&cli.db_path);

    // Ensure directories exist
    if let Some(parent) = std::path::Path::new(&db_path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::create_dir_all(&data_dir)?;

    // Open database
    let db = bizclaw_platform::PlatformDb::open(std::path::Path::new(&db_path))?;

    // --init-admin: create admin user and exit
    if cli.init_admin {
        println!("🏢 BizClaw Platform — Admin Setup\n");

        // Check if admin already exists
        match db.get_user_by_email(&cli.admin_email) {
            Ok(Some(_)) => {
                println!("⚠️  Admin '{}' already exists.", cli.admin_email);
            }
            _ => {
                let hash = bizclaw_platform::auth::hash_password(&cli.admin_password)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                let id = db.create_user(&cli.admin_email, &hash, "admin", None)?;
                db.log_event(
                    "admin_created",
                    "system",
                    &id,
                    Some(&format!("email={}", cli.admin_email)),
                )
                .ok();
                println!("✅ Admin user created:");
                println!("   Email:    {}", cli.admin_email);
                println!("   Password: {}", cli.admin_password);
                println!("   Role:     admin");
            }
        }
        return Ok(());
    }

    // Ensure at least one admin exists — auto-create on first run
    let users = db.list_users().unwrap_or_default();
    if users.is_empty() {
        println!("📝 No admin users found. Creating default admin...");
        // Read password from env var, CLI arg, or auto-generate
        let admin_password = match std::env::var("BIZCLAW_ADMIN_PASSWORD") {
            Ok(p) if !p.is_empty() => p,
            _ => {
                if cli.admin_password == "auto" {
                    // Auto-generate secure random password
                    use rand::Rng;
                    let pwd: String = rand::thread_rng()
                        .sample_iter(&rand::distributions::Alphanumeric)
                        .take(16)
                        .map(char::from)
                        .collect();
                    println!("   🔑 Auto-generated admin password (save this!):");
                    println!("   Password: {pwd}");
                    pwd
                } else {
                    cli.admin_password.clone()
                }
            }
        };
        // Read admin email from env var or CLI arg
        let admin_email =
            std::env::var("BIZCLAW_ADMIN_EMAIL").unwrap_or_else(|_| cli.admin_email.clone());
        let hash = bizclaw_platform::auth::hash_password(&admin_password)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        db.create_user(&admin_email, &hash, "superadmin", None)?;
        println!("   Email:    {}", admin_email);
        println!("   ⚠️  Change this password after first login!\n");
    }

    // Prefer JWT_SECRET env var over CLI default
    let jwt_secret = match std::env::var("JWT_SECRET") {
        Ok(s) if !s.is_empty() => s,
        _ => {
            if cli.jwt_secret == "auto" {
                // Auto-generate a cryptographically secure random secret
                use rand::Rng;
                let secret: String = rand::thread_rng()
                    .sample_iter(&rand::distributions::Alphanumeric)
                    .take(64)
                    .map(char::from)
                    .collect();
                tracing::warn!(
                    "⚠️  JWT_SECRET not set! Auto-generated 256-bit random secret for this session."
                );
                tracing::warn!(
                    "⚠️  Tokens will be INVALIDATED on restart. Set JWT_SECRET env var for persistence."
                );
                secret
            } else {
                cli.jwt_secret.clone()
            }
        }
    };

    // Connect PostgreSQL for enterprise features (optional — falls back gracefully)
    let pg_db = match std::env::var("DATABASE_URL").or_else(|_| std::env::var("BIZCLAW_PG_URL")) {
        Ok(url) if !url.is_empty() => match bizclaw_platform::PgDb::connect_with_url(&url).await {
            Ok(pg) => {
                tracing::info!(
                    "🐘 PostgreSQL connected — enterprise features enabled (4 migrations applied)"
                );
                Some(pg)
            }
            Err(e) => {
                tracing::warn!("⚠️  PostgreSQL connect failed: {e} — enterprise features disabled");
                None
            }
        },
        _ => {
            tracing::info!(
                "ℹ️  DATABASE_URL not set — enterprise features disabled (SQLite only mode)"
            );
            None
        }
    };

    // Build admin state
    let state = Arc::new(bizclaw_platform::admin::AdminState {
        db: tokio::sync::Mutex::new(db),
        manager: tokio::sync::Mutex::new(bizclaw_platform::TenantManager::new(&data_dir)),
        jwt_secret,
        bizclaw_bin: cli.bizclaw_bin.clone(),
        base_port: cli.base_port,
        domain: std::env::var("BIZCLAW_DOMAIN").unwrap_or_else(|_| cli.domain.clone()),
        login_attempts: std::sync::Mutex::new(std::collections::HashMap::new()),
        register_attempts: std::sync::Mutex::new(std::collections::HashMap::new()),
        pg_db,
        startup_time: std::time::Instant::now(),
    });

    // Start server
    println!("🏢 BizClaw Platform v{}", env!("CARGO_PKG_VERSION"));
    println!("   🌐 Admin Dashboard: http://0.0.0.0:{}", cli.port);
    println!(
        "   📡 API:             http://0.0.0.0:{}/api/admin/stats",
        cli.port
    );
    println!("   🗄️  Database:        {db_path}");
    println!("   📂 Data Dir:        {data_dir}");
    println!("   🔧 BizClaw Binary:  {}", cli.bizclaw_bin);
    println!("   🔌 Tenant Base Port: {}", cli.base_port);
    println!();
    println!("   ┌──────────────────────────────────────────────┐");
    println!("   │  🔑 Admin: use --admin-email / --admin-password │");
    println!("   │     or set BIZCLAW_ADMIN_PASSWORD env var     │");
    println!("   │     ⚠️  Change password after first login!    │");
    println!("   └──────────────────────────────────────────────┘");
    println!();

    // Auto-restart tenants that were previously running
    {
        let db_lock = state.db.lock().await;
        match db_lock.list_tenants() {
            Ok(tenants) => {
                let running: Vec<_> = tenants
                    .iter()
                    .filter(|t| t.status == "running")
                    .cloned()
                    .collect();
                if !running.is_empty() {
                    println!("🔄 Auto-restarting {} tenant(s)...", running.len());
                    drop(db_lock); // Release lock before starting tenants
                    for tenant in &running {
                        let db = state.db.lock().await;
                        let mut mgr = state.manager.lock().await;
                        match mgr.start_tenant(tenant, &state.bizclaw_bin, &db) {
                            Ok(pid) => {
                                println!(
                                    "   ✅ {} (port {}) → pid {}",
                                    tenant.name, tenant.port, pid
                                );
                                db.update_tenant_status(&tenant.id, "running", Some(pid))
                                    .ok();
                            }
                            Err(e) => {
                                println!("   ❌ {} failed: {}", tenant.name, e);
                                db.update_tenant_status(&tenant.id, "error", None).ok();
                            }
                        }
                    }
                }
            }
            Err(e) => println!("⚠️ Failed to load tenants: {e}"),
        }
    }

    bizclaw_platform::AdminServer::start(state, cli.port)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    Ok(())
}
