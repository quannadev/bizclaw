//! Web Dashboard — embedded HTML/JS/CSS served at / and /static/
//!
//! Supports two modes:
//! - **New Dashboard** (Preact + HTM): Served at `/` with modular components
//! - **Legacy Dashboard**: Served at `/legacy` for backward compatibility
//!
//! All files are embedded at compile time via `include_str!()` to maintain
//! the self-contained binary philosophy (no external file dependencies).

use std::collections::HashMap;

/// Legacy dashboard HTML page (monolithic 4,251 lines).
pub fn dashboard_html() -> &'static str {
    include_str!("dashboard.html")
}

/// New Preact-based dashboard shell.
pub fn dashboard_v2_html() -> &'static str {
    include_str!("dashboard/index.html")
}

/// Public Hub page — skill marketplace (no auth required).
pub fn hub_html() -> &'static str {
    include_str!("dashboard/hub.html")
}

/// Static files for the new dashboard (served at /static/dashboard/*).
/// Returns a HashMap of (path, content, content_type).
pub fn dashboard_static_files() -> HashMap<&'static str, (&'static str, &'static str)> {
    let mut files: HashMap<&str, (&str, &str)> = HashMap::new();

    // CSS
    files.insert(
        "/static/dashboard/styles.css",
        (
            include_str!("dashboard/styles.css"),
            "text/css; charset=utf-8",
        ),
    );

    // Main app (slim orchestrator)
    files.insert(
        "/static/dashboard/app.js",
        (
            include_str!("dashboard/app.js"),
            "application/javascript; charset=utf-8",
        ),
    );

    // Shared utilities (auth, i18n, components)
    files.insert(
        "/static/dashboard/shared.js",
        (
            include_str!("dashboard/shared.js"),
            "application/javascript; charset=utf-8",
        ),
    );

    // ── Page modules (lazy-loaded) ──
    let pages: &[(&str, &str)] = &[
        ("chat.js", include_str!("dashboard/pages/chat.js")),
        ("chat_widget.js", include_str!("dashboard/pages/chat_widget.js")),
        ("dashboard.js", include_str!("dashboard/pages/dashboard.js")),
        ("scheduler.js", include_str!("dashboard/pages/scheduler.js")),
        ("hands.js", include_str!("dashboard/pages/hands.js")),
        ("settings.js", include_str!("dashboard/pages/settings.js")),
        ("providers.js", include_str!("dashboard/pages/providers.js")),
        ("channels.js", include_str!("dashboard/pages/channels.js")),
        ("tools.js", include_str!("dashboard/pages/tools.js")),
        ("mcp.js", include_str!("dashboard/pages/mcp.js")),
        ("agents.js", include_str!("dashboard/pages/agents.js")),
        ("knowledge.js", include_str!("dashboard/pages/knowledge.js")),
        ("orchestration.js", include_str!("dashboard/pages/orchestration.js")),
        ("org_map.js", include_str!("dashboard/pages/org_map.js")),
        ("kanban.js", include_str!("dashboard/pages/kanban.js")),
        ("gallery.js", include_str!("dashboard/pages/gallery.js")),
        ("brain.js", include_str!("dashboard/pages/brain.js")),
        ("config_file.js", include_str!("dashboard/pages/config_file.js")),
        ("traces.js", include_str!("dashboard/pages/traces.js")),
        ("cost.js", include_str!("dashboard/pages/cost.js")),
        ("activity.js", include_str!("dashboard/pages/activity.js")),
        ("workflows.js", include_str!("dashboard/pages/workflows.js")),
        ("skills.js", include_str!("dashboard/pages/skills.js")),
        ("wiki.js", include_str!("dashboard/pages/wiki.js")),
        ("api_keys.js", include_str!("dashboard/pages/api_keys.js")),
        ("usage.js", include_str!("dashboard/pages/usage.js")),
        ("analytics.js", include_str!("dashboard/pages/analytics.js")),
        ("sso.js", include_str!("dashboard/pages/sso.js")),
        ("fine_tuning.js", include_str!("dashboard/pages/fine_tuning.js")),
        ("edge_gateway.js", include_str!("dashboard/pages/edge_gateway.js")),
        ("plugins.js", include_str!("dashboard/pages/plugins.js")),
        ("db_assistant.js", include_str!("dashboard/pages/db_assistant.js")),
        ("campaigns.js", include_str!("dashboard/pages/campaigns.js")),
    ];

    for (name, content) in pages {
        files.insert(
            // Leak is fine — these are static strings that live for program duration
            Box::leak(format!("/static/dashboard/pages/{name}").into_boxed_str()),
            (content, "application/javascript; charset=utf-8"),
        );
    }

    // i18n
    files.insert(
        "/static/dashboard/i18n/vi.js",
        (
            include_str!("dashboard/i18n/vi.js"),
            "application/javascript; charset=utf-8",
        ),
    );
    files.insert(
        "/static/dashboard/i18n/en.js",
        (
            include_str!("dashboard/i18n/en.js"),
            "application/javascript; charset=utf-8",
        ),
    );

    // Vendor: Self-hosted Preact + Hooks (avoids esm.sh dual-package hazard)
    files.insert(
        "/static/dashboard/vendor/preact.mjs",
        (
            include_str!("dashboard/vendor/preact.mjs"),
            "application/javascript; charset=utf-8",
        ),
    );
    files.insert(
        "/static/dashboard/vendor/hooks.mjs",
        (
            include_str!("dashboard/vendor/hooks.mjs"),
            "application/javascript; charset=utf-8",
        ),
    );
    files.insert(
        "/static/dashboard/vendor/htm.mjs",
        (
            include_str!("dashboard/vendor/htm.mjs"),
            "application/javascript; charset=utf-8",
        ),
    );
    // All-in-one standalone: preact + hooks + htm in single file (no dual-instance)
    files.insert(
        "/static/dashboard/vendor/standalone.mjs",
        (
            include_str!("dashboard/vendor/standalone.mjs"),
            "application/javascript; charset=utf-8",
        ),
    );

    files
}
