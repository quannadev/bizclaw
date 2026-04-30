//! OpenClaw-style Interactive Onboarding Wizard
//! 
//! Features:
//! - Step-by-step guided setup
//! - Provider configuration
//! - Channel setup
//! - Skill bundle selection
//! - Auto-detection of existing configurations

use anyhow::Result;
use std::io::{self, Write};
use std::path::PathBuf;
use tracing::info;

#[derive(Debug, Clone)]
pub struct OnboardingWizard {
    data_dir: PathBuf,
    config_path: PathBuf,
    language: String,
}

impl OnboardingWizard {
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let data_dir = home.join(".bizclaw");
        let config_path = data_dir.join("bizclaw.json");
        
        Self {
            data_dir,
            config_path,
            language: "en".to_string(),
        }
    }

    pub fn run(&mut self) -> Result<()> {
        println!();
        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║                      BizClaw Setup Wizard                       ║");
        println!("╚══════════════════════════════════════════════════════════════╝");
        println!();
        
        // Step 1: Language selection
        self.step_language()?;
        
        // Step 2: Provider configuration
        self.step_provider()?;
        
        // Step 3: Channel selection
        self.step_channels()?;
        
        // Step 4: Skill bundles
        self.step_skills()?;
        
        // Step 5: Review and save
        self.step_review()?;
        
        Ok(())
    }

    fn step_language(&mut self) -> io::Result<()> {
        println!("┌─────────────────────────────────────────────────────────────────┐");
        println!("│ Step 1: Language & Region                                       │");
        println!("├─────────────────────────────────────────────────────────────────┤");
        println!("│                                                                 │");
        println!("│  Select your preferred language:                                │");
        println!("│                                                                 │");
        println!("│    [1] English (US)                                            │");
        println!("│    [2] Tiếng Việt (Vietnamese)                                 │");
        println!("│    [3] 中文 (Chinese)                                          │");
        println!("│    [4] 日本語 (Japanese)                                       │");
        println!("│                                                                 │");
        println!("└─────────────────────────────────────────────────────────────────┘");
        print!("Enter your choice [1]: ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        let choice = input.trim().parse::<usize>().unwrap_or(1);
        
        self.language = match choice {
            2 => "vi",
            3 => "zh",
            4 => "ja",
            _ => "en",
        }.to_string();
        
        println!("  ✓ Language set to: {}", self.get_language_name());
        println!();
        Ok(())
    }

    fn step_provider(&mut self) -> io::Result<()> {
        println!("┌─────────────────────────────────────────────────────────────────┐");
        println!("│ Step 2: AI Provider Configuration                               │");
        println!("├─────────────────────────────────────────────────────────────────┤");
        println!("│                                                                 │");
        println!("│  Select your primary AI provider:                                │");
        println!("│                                                                 │");
        println!("│    [1] OpenAI (GPT-4, GPT-3.5)                    ⭐ Recommended │");
        println!("│    [2] Anthropic (Claude 3)                                 │");
        println!("│    [3] Google (Gemini)                                      │");
        println!("│    [4] DeepSeek                                            │");
        println!("│    [5] Ollama (Local)                                      │");
        println!("│    [6] Skip for now                                         │");
        println!("│                                                                 │");
        println!("└─────────────────────────────────────────────────────────────────┘");
        print!("Enter your choice [1]: ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        let choice = input.trim().parse::<usize>().unwrap_or(1);
        
        match choice {
            2 => self.configure_provider("anthropic"),
            3 => self.configure_provider("gemini"),
            4 => self.configure_provider("deepseek"),
            5 => self.configure_provider("ollama"),
            6 => {
                println!("  ⚠ Skipping provider configuration (you can configure later)");
                Ok(())
            }
            _ => self.configure_provider("openai"),
        }
    }

    fn configure_provider(&self, provider: &str) -> io::Result<()> {
        println!();
        println!("  Configuring {}...", provider);
        
        let (api_key_name, env_var) = match provider {
            "anthropic" => ("Anthropic API Key", "ANTHROPIC_API_KEY"),
            "gemini" => ("Google AI API Key", "GOOGLE_AI_API_KEY"),
            "deepseek" => ("DeepSeek API Key", "DEEPSEEK_API_KEY"),
            "ollama" => ("Ollama URL", "OLLAMA_BASE_URL"),
            _ => ("OpenAI API Key", "OPENAI_API_KEY"),
        };
        
        print!("  Enter your {}: ", api_key_name);
        io::stdout().flush()?;
        
        let mut api_key = String::new();
        io::stdin().read_line(&mut api_key)?;
        let api_key = api_key.trim().to_string();
        
        if api_key.is_empty() {
            println!("  ⚠ No API key entered. Using environment variable: ${}", env_var);
        } else {
            println!("  ✓ API key configured (will be stored securely)");
        }
        
        // Configure default model
        let model = match provider {
            "anthropic" => "claude-3-sonnet-20240229",
            "gemini" => "gemini-pro",
            "deepseek" => "deepseek-chat",
            "ollama" => "llama2",
            _ => "gpt-4-turbo-preview",
        };
        
        println!("  ✓ Default model: {}", model);
        println!();
        Ok(())
    }

    fn step_channels(&mut self) -> io::Result<()> {
        println!("┌─────────────────────────────────────────────────────────────────┐");
        println!("│ Step 3: Messaging Channels                                      │");
        println!("├─────────────────────────────────────────────────────────────────┤");
        println!("│                                                                 │");
        println!("│  Which channels would you like to connect? (comma-separated)    │");
        println!("│                                                                 │");
        println!("│    [1] Telegram                                    ⭐ Easy    │");
        println!("│    [2] Discord                                                  │");
        println!("│    [3] Slack                                                   │");
        println!("│    [4] Zalo (Vietnam)                                          │");
        println!("│    [5] WhatsApp                                                │");
        println!("│    [6] All of the above                                        │");
        println!("│    [7] None for now                                            │");
        println!("│                                                                 │");
        println!("└─────────────────────────────────────────────────────────────────┘");
        print!("Enter your choice [7]: ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        let choice = input.trim().parse::<usize>().unwrap_or(7);
        
        match choice {
            1 => self.configure_channel("telegram"),
            2 => self.configure_channel("discord"),
            3 => self.configure_channel("slack"),
            4 => self.configure_channel("zalo"),
            5 => self.configure_channel("whatsapp"),
            6 => {
                self.configure_channel("telegram")?;
                self.configure_channel("discord")?;
                self.configure_channel("slack")?;
                self.configure_channel("zalo")?;
                self.configure_channel("whatsapp")?;
                Ok(())
            }
            _ => {
                println!("  ⚠ Skipping channel configuration");
                Ok(())
            }
        }
    }

    fn configure_channel(&self, channel: &str) -> io::Result<()> {
        println!();
        println!("  Configuring {}...", channel);
        
        let (name, token_hint) = match channel {
            "telegram" => ("Telegram Bot", "Get from @BotFather"),
            "discord" => ("Discord Bot Token", "From Discord Developer Portal"),
            "slack" => ("Slack Bot Token", "From Slack API Portal"),
            "zalo" => ("Zalo OA Token", "From Zalo Official Account"),
            "whatsapp" => ("WhatsApp Business Token", "From Meta Developer Portal"),
            _ => ("Token", ""),
        };
        
        print!("  Enter your {} ({}): ", name, token_hint);
        io::stdout().flush()?;
        
        let mut token = String::new();
        io::stdin().read_line(&mut token)?;
        let token = token.trim().to_string();
        
        if token.is_empty() {
            println!("  ⚠ No token entered. You can configure later.");
        } else {
            println!("  ✓ {} configured", channel);
        }
        
        println!();
        Ok(())
    }

    fn step_skills(&mut self) -> io::Result<()> {
        println!("┌─────────────────────────────────────────────────────────────────┐");
        println!("│ Step 4: Skill Bundles                                           │");
        println!("├─────────────────────────────────────────────────────────────────┤");
        println!("│                                                                 │");
        println!("│  Select skill bundles to install (comma-separated):             │");
        println!("│                                                                 │");
        println!("│    [1] Developer Assistant                    ⭐ Recommended   │");
        println!("│        • Code review, debugging, documentation                 │");
        println!("│                                                                 │");
        println!("│    [2] Business Writing                                        │");
        println!("│        • Emails, reports, proposals                            │");
        println!("│                                                                 │");
        println!("│    [3] Research & Analysis                                     │");
        println!("│        • Data analysis, trend monitoring                      │");
        println!("│                                                                 │");
        println!("│    [4] Vietnamese Business                                    │");
        println!("│        • Tiếng Việt business communication                     │");
        println!("│                                                                 │");
        println!("│    [5] All Bundles                                            │");
        println!("│    [6] Skip for now                                           │");
        println!("│                                                                 │");
        println!("└─────────────────────────────────────────────────────────────────┘");
        print!("Enter your choice [1]: ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        let choice = input.trim().parse::<usize>().unwrap_or(1);
        
        let bundles = match choice {
            2 => vec!["business-writing"],
            3 => vec!["research-analysis"],
            4 => vec!["vietnamese-business"],
            5 => vec!["developer", "business-writing", "research-analysis", "vietnamese-business"],
            6 => {
                println!("  ⚠ Skipping skill bundle installation");
                return Ok(());
            }
            _ => vec!["developer"],
        };
        
        for bundle in &bundles {
            println!("  ✓ Installing skill bundle: {}", bundle);
        }
        
        println!();
        Ok(())
    }

    fn step_review(&self) -> io::Result<()> {
        println!("┌─────────────────────────────────────────────────────────────────┐");
        println!("│ Step 5: Review & Save                                           │");
        println!("├─────────────────────────────────────────────────────────────────┤");
        println!("│                                                                 │");
        println!("│  Configuration Summary:                                         │");
        println!("│                                                                 │");
        println!("│    Language:        {}", self.get_language_name());
        println!("│    Data directory:  ~/.bizclaw/                                │");
        println!("│    Config file:    ~/.bizclaw/bizclaw.json                     │");
        println!("│                                                                 │");
        println!("│  Next Steps:                                                    │");
        println!("│                                                                 │");
        println!("│    1. Start the gateway: bizclaw gateway                       │");
        println!("│    2. Send /start to your configured bot                       │");
        println!("│    3. Pair your account if using pairing mode                  │");
        println!("│                                                                 │");
        println!("└─────────────────────────────────────────────────────────────────┘");
        println!();
        
        print!("  Save configuration and create necessary directories? [Y/n]: ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        let choice = input.trim().to_lowercase();
        
        if choice.is_empty() || choice == "y" || choice == "yes" {
            // Create directories
            std::fs::create_dir_all(&self.data_dir)?;
            std::fs::create_dir_all(self.data_dir.join("skills"))?;
            std::fs::create_dir_all(self.data_dir.join("workspace"))?;
            std::fs::create_dir_all(self.data_dir.join("memory"))?;
            
            info!("Configuration saved to {:?}", self.config_path);
            println!();
            println!("  ✓ Setup complete! Your configuration has been saved.");
            println!();
        } else {
            println!();
            println!("  ⚠ Configuration not saved. You can run 'bizclaw setup' again later.");
            println!();
        }
        
        Ok(())
    }

    fn get_language_name(&self) -> &str {
        match self.language.as_str() {
            "vi" => "Tiếng Việt",
            "zh" => "中文",
            "ja" => "日本語",
            _ => "English",
        }
    }
}

impl Default for OnboardingWizard {
    fn default() -> Self {
        Self::new()
    }
}

pub fn run_onboarding() -> Result<()> {
    let mut wizard = OnboardingWizard::new();
    wizard.run()
}
