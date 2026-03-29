//! # BizClaw Local — Run AI agent on your machine
//!
//! Like OpenClaw but with BizClaw's full power — runs locally with:
//! - Full filesystem access (edit files, run commands)
//! - All 21+ tools available
//! - Connect to remote BizClaw platform for memory sync
//! - Works offline with local LLM (Ollama/Brain)
//!
//! Usage:
//!   bizclaw-local                      # Interactive mode
//!   bizclaw-local -m "Fix this bug"    # One-shot mode
//!   bizclaw-local --provider ollama    # Use local LLM
//!   bizclaw-local --sync              # Sync with remote platform

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(
    name = "bizclaw-local",
    version,
    about = "🦀 BizClaw Local — AI assistant running on YOUR machine",
    long_about = "Run BizClaw's full agent locally with filesystem access.\n\
                   Edit files, run commands, search the web — like OpenClaw.\n\
                   Optional: sync with remote BizClaw platform for team memory."
)]
struct Cli {
    /// Message to send (one-shot mode)
    #[arg(short, long)]
    message: Option<String>,

    /// Working directory (defaults to current dir)
    #[arg(short, long)]
    workdir: Option<String>,

    /// AI provider override
    #[arg(short, long)]
    provider: Option<String>,

    /// AI model override
    #[arg(long)]
    model: Option<String>,

    /// Config file path
    #[arg(short, long)]
    config: Option<String>,

    /// Verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Sync conversation with remote BizClaw platform
    #[arg(long)]
    sync: bool,

    /// Remote platform URL for sync
    #[arg(long, default_value = "https://apps.viagent.vn")]
    remote: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let filter = if cli.verbose {
        "bizclaw=debug,bizclaw_agent=debug,bizclaw_tools=debug"
    } else {
        "bizclaw=info"
    };
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(filter)),
        )
        .with_target(false)
        .init();

    // Load config
    let mut config = if let Some(path) = &cli.config {
        bizclaw_core::BizClawConfig::load_from(std::path::Path::new(path))?
    } else {
        bizclaw_core::BizClawConfig::load().unwrap_or_default()
    };

    // Apply overrides
    if let Some(p) = cli.provider {
        config.default_provider = p;
    }
    if let Some(m) = cli.model {
        config.default_model = m;
    }

    // Set working directory
    let workdir = cli.workdir.unwrap_or_else(|| {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string())
    });

    // Override system prompt for local mode
    if config.identity.system_prompt.is_empty()
        || config.identity.system_prompt.contains("AI assistant")
    {
        config.identity.system_prompt = format!(
            "Bạn là BizClaw Local — trợ lý AI chạy trực tiếp trên máy người dùng.\n\
             Bạn có TOÀN QUYỀN truy cập hệ thống file và terminal.\n\
             Working directory: {}\n\
             \n\
             Khả năng của bạn:\n\
             - Đọc/sửa/tạo files (dùng tool 'file' và 'edit_file')\n\
             - Chạy lệnh shell (dùng tool 'shell')\n\
             - Tìm kiếm files (dùng tool 'glob' và 'grep')\n\
             - Tìm kiếm trên web (dùng tool 'web_search')\n\
             - Gọi API (dùng tool 'http_request')\n\
             - Đăng bài mạng xã hội (dùng tool 'social_post')\n\
             \n\
             Hãy làm việc hiệu quả, giải thích rõ ràng, và luôn xác nhận \
             trước khi thực hiện thay đổi destructive.",
            workdir
        );
    }

    // Create agent
    let mut agent = bizclaw_agent::Agent::new(config)?;

    if let Some(msg) = cli.message {
        // ── One-shot mode ──
        let response = agent.process(&msg).await?;
        println!("{response}");
    } else {
        // ── Interactive mode ──
        println!("🦀 BizClaw Local v{}", env!("CARGO_PKG_VERSION"));
        println!(
            "   Provider: {} | Workdir: {}",
            agent.provider_name(),
            workdir
        );
        println!("   Tools: file, shell, edit_file, glob, grep, web_search, social_post, ...");
        if cli.sync {
            println!("   🔗 Sync: {}", cli.remote);
        }
        println!("   Type /quit to exit, /clear to reset, /info for status\n");

        let mut cli_channel = bizclaw_channels::cli::CliChannel::new();
        use bizclaw_core::traits::Channel;
        cli_channel.connect().await?;

        use tokio_stream::StreamExt;
        let mut stream = cli_channel.listen().await?;

        print!("You: ");
        use std::io::Write;
        std::io::stdout().flush()?;

        while let Some(incoming) = stream.next().await {
            match incoming.content.as_str() {
                "/clear" => {
                    agent.clear_conversation();
                    println!("🔄 Conversation cleared.\n");
                }
                "/info" => {
                    let conv = agent.conversation();
                    println!(
                        "\n📊 Provider: {} | Messages: {} | Workdir: {}\n",
                        agent.provider_name(),
                        conv.len(),
                        workdir
                    );
                }
                "/tools" => {
                    println!("\n🔧 Available tools:");
                    println!("   shell         — Execute commands");
                    println!("   file          — Read/write/list files");
                    println!("   edit_file     — Precise text replacements");
                    println!("   glob          — Find files by pattern");
                    println!("   grep          — Search file contents");
                    println!("   web_search    — DuckDuckGo search");
                    println!("   http_request  — Call APIs");
                    println!("   social_post   — Post to Facebook/Telegram/Webhook");
                    println!("   plan          — Task decomposition");
                    println!("   browser       — Chrome automation");
                    println!("   db_query      — SQL queries");
                    println!("   calendar      — Google Calendar");
                    println!("   doc_reader    — PDF/DOCX/Excel reader");
                    println!("   zalo_tool     — Zalo automation");
                    println!("   nl_query      — Natural language → SQL\n");
                }
                _ => match agent.handle_incoming(&incoming).await {
                    Ok(response) => {
                        cli_channel.send(response).await?;
                    }
                    Err(e) => {
                        println!("\n❌ Error: {e}\n");
                    }
                },
            }
            print!("You: ");
            std::io::stdout().flush()?;
        }

        println!("\n👋 Goodbye!");
    }

    Ok(())
}
