//! # BizClaw CLI
//!
//! Fast, small, and fully autonomous AI assistant infrastructure
//! with local brain and Zalo channels.
//!
//! Usage:
//!   bizclaw agent -m "Hello"           # One-shot message
//!   bizclaw agent --interactive        # Interactive CLI
//!   bizclaw channel start              # Start channel listener
//!   bizclaw onboard                    # First-time setup
//!   bizclaw brain download             # Download local model
//!   bizclaw config show                # Show configuration

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(
    name = "bizclaw",
    version,
    about = "🦀 BizClaw — AI assistant infrastructure with local brain",
    long_about = "Fast, small, and fully autonomous AI assistant infrastructure.\nDeploy anywhere, swap anything. Local intelligence built-in."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Config file path
    #[arg(short, long, global = true)]
    config: Option<String>,

    /// Verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Send a message to the agent
    Agent {
        /// Message to send
        #[arg(short, long)]
        message: Option<String>,

        /// Interactive mode
        #[arg(short, long)]
        interactive: bool,

        /// Override provider
        #[arg(short, long)]
        provider: Option<String>,

        /// Override model
        #[arg(long)]
        model: Option<String>,
    },

    /// Manage channels
    Channel {
        #[command(subcommand)]
        action: ChannelAction,
    },

    /// First-time setup wizard
    Onboard,

    /// Brain (local LLM) management
    Brain {
        #[command(subcommand)]
        action: BrainAction,
    },

    /// Configuration management
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Show system info
    Info,

    /// Quick interactive chat (alias for agent --interactive)
    Chat {
        /// Override provider
        #[arg(short, long)]
        provider: Option<String>,

        /// Override model
        #[arg(long)]
        model: Option<String>,
    },
}

#[derive(Subcommand)]
enum ChannelAction {
    /// Start listening on configured channels
    Start {
        /// Specific channel to start
        #[arg(short, long)]
        channel: Option<String>,
    },
    /// List available channels
    List,
}

#[derive(Subcommand)]
enum BrainAction {
    /// Download a model
    Download {
        /// Model name or URL
        #[arg(default_value = "tinyllama-1.1b")]
        model: String,
    },
    /// List available models
    List,
    /// Test inference
    Test {
        /// Prompt to test
        #[arg(default_value = "Hello, who are you?")]
        prompt: String,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Show current configuration
    Show,
    /// Reset to defaults
    Reset,
    /// Set a config value
    Set {
        key: String,
        value: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let filter = if cli.verbose {
        "bizclaw=debug,bizclaw_core=debug,bizclaw_agent=debug"
    } else {
        "bizclaw=info"
    };
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(filter)))
        .with_target(false)
        .init();

    // Load config
    let mut config = if let Some(path) = &cli.config {
        bizclaw_core::BizClawConfig::load_from(std::path::Path::new(path))?
    } else {
        bizclaw_core::BizClawConfig::load()?
    };

    match cli.command {
        Commands::Agent { message, interactive, provider, model } => {
            // Apply overrides
            if let Some(p) = provider {
                config.default_provider = p;
            }
            if let Some(m) = model {
                config.default_model = m;
            }

            let mut agent = bizclaw_agent::Agent::new(config)?;

            if interactive || message.is_none() {
                // Interactive mode
                println!("🦀 BizClaw v{} — Interactive Mode", env!("CARGO_PKG_VERSION"));
                println!("   Provider: {} | Model: {}", agent.provider_name(), "default");
                println!("   Type /quit to exit, /clear to reset conversation\n");

                let mut cli_channel = bizclaw_channels::cli::CliChannel::new();
                cli_channel.connect().await?;

                use bizclaw_core::traits::Channel;
                use tokio_stream::StreamExt;

                let mut stream = cli_channel.listen().await?;
                print!("You: ");
                use std::io::Write;
                std::io::stdout().flush()?;

                while let Some(incoming) = stream.next().await {
                    if incoming.content == "/clear" {
                        agent.clear_conversation();
                        println!("🔄 Conversation cleared.\n");
                        print!("You: ");
                        std::io::stdout().flush()?;
                        continue;
                    }

                    match agent.handle_incoming(&incoming).await {
                        Ok(response) => {
                            cli_channel.send(response).await?;
                        }
                        Err(e) => {
                            println!("\n❌ Error: {e}\n");
                        }
                    }
                    print!("You: ");
                    std::io::stdout().flush()?;
                }

                println!("\n👋 Goodbye!");
            } else if let Some(msg) = message {
                // One-shot mode
                let response = agent.process(&msg).await?;
                println!("{response}");
            }
        }

        Commands::Channel { action } => {
            match action {
                ChannelAction::Start { channel } => {
                    println!("🦀 BizClaw Channel Listener");
                    if let Some(ch) = channel {
                        println!("Starting channel: {ch}");
                    } else {
                        println!("Starting all configured channels...");
                    }

                    // Start configured channels
                    if let Some(zalo_config) = &config.channel.zalo {
                        if zalo_config.enabled {
                            println!("  📱 Zalo ({}) channel starting...", zalo_config.mode);
                            let mut zalo = bizclaw_channels::zalo::ZaloChannel::new(zalo_config.clone());
                            use bizclaw_core::traits::Channel;
                            zalo.connect().await?;
                        }
                    }

                    println!("\nChannels are running. Press Ctrl+C to stop.");
                    tokio::signal::ctrl_c().await?;
                    println!("\n👋 Channels stopped.");
                }
                ChannelAction::List => {
                    println!("Available channels:");
                    println!("  ✅ cli       — Interactive terminal");
                    println!("  {} zalo      — Zalo Personal/OA",
                        if config.channel.zalo.as_ref().is_some_and(|z| z.enabled) { "✅" } else { "⬜" });
                    println!("  {} telegram  — Telegram bot",
                        if config.channel.telegram.is_some() { "✅" } else { "⬜" });
                    println!("  {} discord   — Discord bot",
                        if config.channel.discord.is_some() { "✅" } else { "⬜" });
                }
            }
        }

        Commands::Onboard => {
            println!("🦀 BizClaw — First-time Setup\n");
            println!("Creating default configuration...");

            let config = bizclaw_core::BizClawConfig::default();
            config.save()?;
            println!("✅ Config saved to: {}", bizclaw_core::BizClawConfig::default_path().display());

            // Create directories
            let home = bizclaw_core::BizClawConfig::home_dir();
            std::fs::create_dir_all(home.join("models"))?;
            std::fs::create_dir_all(home.join("cache"))?;
            std::fs::create_dir_all(home.join("zalo"))?;
            println!("✅ Directories created");

            println!("\n📋 Next steps:");
            println!("  1. Set your API key: bizclaw config set api_key sk-...");
            println!("  2. Or use local brain: bizclaw brain download");
            println!("  3. Start chatting: bizclaw agent --interactive");
            println!("  4. For Zalo: configure ~/.bizclaw/config.toml [channel.zalo] section");
        }

        Commands::Brain { action } => {
            match action {
                BrainAction::Download { model } => {
                    let model_dir = bizclaw_core::BizClawConfig::home_dir().join("models");
                    std::fs::create_dir_all(&model_dir)?;

                    let (url, filename) = match model.as_str() {
                        "tinyllama-1.1b" | "tinyllama" => (
                            "https://huggingface.co/TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF/resolve/main/tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf",
                            "tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf",
                        ),
                        "phi-2" => (
                            "https://huggingface.co/TheBloke/phi-2-GGUF/resolve/main/phi-2.Q4_K_M.gguf",
                            "phi-2.Q4_K_M.gguf",
                        ),
                        "llama-3.2-1b" | "llama3.2" => (
                            "https://huggingface.co/bartowski/Llama-3.2-1B-Instruct-GGUF/resolve/main/Llama-3.2-1B-Instruct-Q4_K_M.gguf",
                            "Llama-3.2-1B-Instruct-Q4_K_M.gguf",
                        ),
                        other if other.starts_with("http") => (other, "custom-model.gguf"),
                        _ => {
                            println!("❌ Unknown model: {model}");
                            println!("   Available: tinyllama-1.1b, phi-2, llama-3.2-1b");
                            println!("   Or provide a direct URL to a .gguf file");
                            return Ok(());
                        }
                    };

                    let dest = model_dir.join(filename);
                    if dest.exists() {
                        println!("✅ Model already downloaded: {}", dest.display());
                        return Ok(());
                    }

                    println!("🧠 Downloading: {filename}");
                    println!("   From: {url}");
                    println!("   To:   {}", dest.display());
                    println!();

                    // Stream download with progress
                    let client = reqwest::Client::new();
                    let response = client.get(url)
                        .send()
                        .await
                        .map_err(|e| anyhow::anyhow!("Download failed: {e}"))?;

                    let total_size = response.content_length().unwrap_or(0);
                    println!("   Total size: {:.1} MB", total_size as f64 / 1024.0 / 1024.0);

                    let mut file = tokio::fs::File::create(&dest).await?;
                    let mut downloaded: u64 = 0;
                    let mut stream = response.bytes_stream();

                    use futures::StreamExt;
                    use tokio::io::AsyncWriteExt;

                    while let Some(chunk) = stream.next().await {
                        let chunk = chunk.map_err(|e| anyhow::anyhow!("Download error: {e}"))?;
                        file.write_all(&chunk).await?;
                        downloaded += chunk.len() as u64;

                        if total_size > 0 {
                            let pct = (downloaded as f64 / total_size as f64 * 100.0) as u32;
                            let mb = downloaded as f64 / 1024.0 / 1024.0;
                            print!("\r   ⬇️  {mb:.1} MB / {:.1} MB ({pct}%)", total_size as f64 / 1024.0 / 1024.0);
                            use std::io::Write;
                            std::io::stdout().flush().ok();
                        }
                    }

                    file.flush().await?;
                    println!("\n\n✅ Download complete: {}", dest.display());
                    println!("   Test with: bizclaw brain test \"Hello!\"");
                }
                BrainAction::List => {
                    println!("🧠 Brain Models\n");

                    // List installed models
                    let model_dir = bizclaw_core::BizClawConfig::home_dir().join("models");
                    if model_dir.exists() {
                        let mut found = false;
                        if let Ok(entries) = std::fs::read_dir(&model_dir) {
                            for entry in entries.flatten() {
                                let path = entry.path();
                                if path.extension().and_then(|e| e.to_str()) == Some("gguf") {
                                    let size = std::fs::metadata(&path)
                                        .map(|m| m.len() / 1024 / 1024)
                                        .unwrap_or(0);
                                    println!("  ✅ {} ({} MB)",
                                        path.file_name().unwrap_or_default().to_string_lossy(), size);
                                    found = true;
                                }
                            }
                        }
                        if !found {
                            println!("  (no models installed)");
                        }
                    } else {
                        println!("  (no models directory)");
                    }

                    println!("\n📦 Available for download:");
                    println!("  - tinyllama-1.1b  (~638 MB, recommended for Pi)");
                    println!("  - phi-2           (~1.6 GB)");
                    println!("  - llama-3.2-1b    (~750 MB)");
                    println!("\n  Use: bizclaw brain download <model-name>");
                }
                BrainAction::Test { prompt } => {
                    println!("🧠 Testing brain inference...\n");

                    // Try to find and load a model
                    let model_dir = bizclaw_core::BizClawConfig::home_dir().join("models");
                    let model_path = std::fs::read_dir(&model_dir).ok()
                        .and_then(|entries| {
                            entries.filter_map(|e| e.ok())
                                .find(|e| e.path().extension().and_then(|ext| ext.to_str()) == Some("gguf"))
                                .map(|e| e.path())
                        });

                    match model_path {
                        Some(path) => {
                            println!("   Model: {}", path.display());
                            match bizclaw_brain::BrainEngine::load(&path) {
                                Ok(mut engine) => {
                                    if let Some(info) = engine.model_info() {
                                        println!("   Info: {info}");
                                    }
                                    println!("   Prompt: \"{prompt}\"\n");
                                    match engine.generate(&prompt, 100) {
                                        Ok(response) => println!("🤖 {response}"),
                                        Err(e) => println!("❌ Inference error: {e}"),
                                    }
                                }
                                Err(e) => println!("❌ Failed to load model: {e}"),
                            }
                        }
                        None => {
                            println!("❌ No model found in {}", model_dir.display());
                            println!("   Run: bizclaw brain download tinyllama-1.1b");
                        }
                    }
                }
            }
        }

        Commands::Config { action } => {
            match action {
                ConfigAction::Show => {
                    let content = toml::to_string_pretty(&config)?;
                    println!("{content}");
                }
                ConfigAction::Reset => {
                    let config = bizclaw_core::BizClawConfig::default();
                    config.save()?;
                    println!("✅ Configuration reset to defaults.");
                }
                ConfigAction::Set { key, value } => {
                    println!("Setting {key} = {value}");
                    println!("(Direct config editing — edit ~/.bizclaw/config.toml)");
                }
            }
        }

        Commands::Info => {
            println!("🦀 BizClaw v{}", env!("CARGO_PKG_VERSION"));
            println!("   Platform: {} / {}", std::env::consts::OS, std::env::consts::ARCH);
            println!("   Config: {}", bizclaw_core::BizClawConfig::default_path().display());
            println!("   Provider: {}", config.default_provider);
            println!("   Model: {}", config.default_model);
            println!("   Brain: {}", if config.brain.enabled { "enabled" } else { "disabled" });
            if let Some(zalo) = &config.channel.zalo {
                println!("   Zalo: {} ({})", if zalo.enabled { "enabled" } else { "disabled" }, zalo.mode);
            }
        }

        Commands::Chat { provider, model } => {
            // Apply overrides
            if let Some(p) = provider {
                config.default_provider = p;
            }
            if let Some(m) = model {
                config.default_model = m;
            }

            let mut agent = bizclaw_agent::Agent::new(config)?;

            println!("🦀 BizClaw v{} — Chat Mode", env!("CARGO_PKG_VERSION"));
            println!("   Provider: {}", agent.provider_name());
            println!("   Type /quit to exit, /clear to reset conversation\n");

            let mut cli_channel = bizclaw_channels::cli::CliChannel::new();
            cli_channel.connect().await?;

            use bizclaw_core::traits::Channel;
            use tokio_stream::StreamExt;

            let mut stream = cli_channel.listen().await?;
            print!("You: ");
            use std::io::Write;
            std::io::stdout().flush()?;

            while let Some(incoming) = stream.next().await {
                if incoming.content == "/clear" {
                    agent.clear_conversation();
                    println!("🔄 Conversation cleared.\n");
                    print!("You: ");
                    std::io::stdout().flush()?;
                    continue;
                }

                if incoming.content == "/info" {
                    let conv = agent.conversation();
                    println!("\n📊 Provider: {} | Messages: {} | System prompt: ✅\n",
                        agent.provider_name(), conv.len());
                    print!("You: ");
                    std::io::stdout().flush()?;
                    continue;
                }

                match agent.handle_incoming(&incoming).await {
                    Ok(response) => {
                        cli_channel.send(response).await?;
                    }
                    Err(e) => {
                        println!("\n❌ Error: {e}\n");
                    }
                }
                print!("You: ");
                std::io::stdout().flush()?;
            }

            println!("\n👋 Goodbye!");
        }
    }

    Ok(())
}
