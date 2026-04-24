use bizclaw_browser::{BrowserSession, SessionConfig, StealthConfig};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env()
            .add_directive(tracing::Level::INFO.into()))
        .init();

    println!("\n=== Bot Detection Evaluation ===\n");

    let config = SessionConfig {
        chrome_debug_port: 9223,
        page_id: Some("EA8C2F364C5F73B746362F8787D455C3".to_string()),
        enable_screenshots: true,
        viewport: Some(bizclaw_browser::ViewportConfig {
            width: 1920,
            height: 1080,
            device_scale_factor: Some(1.0),
        }),
        stealth_config: Some(StealthConfig::default()),
        ..Default::default()
    };

    let session = BrowserSession::create(config).await?;
    println!("Session ID: {}\n", session.id);

    println!("Testing various bot detection sites...\n");

    let sites = vec![
        ("Google", "https://www.google.com"),
        ("Facebook", "https://www.facebook.com"),
        ("TikTok", "https://www.tiktok.com"),
        ("Instagram", "https://www.instagram.com"),
        ("Bot Sannysoft", "https://bot.sannysoft.com"),
        ("Cloudflare", "https://nowsecure.nl"),
        ("AreYouAHuman", "https://www.areyouahuman.com"),
        ("2Captcha Test", "https://2captcha.com/demo/normal"),
        ("NoBot", "https://nobot.nginx.org/"),
    ];

    for (name, url) in sites {
        print!("{:20} | ", name);
        
        match session.tools.navigate(url).await {
            Ok(_) => {
                // Get page title
                if let Ok(title) = session.tools.get_text("title").await {
                    if let Some(data) = title.data {
                        let t = data.get("text")
                            .and_then(|t| t.as_str())
                            .unwrap_or("")
                            .chars()
                            .take(30)
                            .collect::<String>();
                        println!("✅ Loaded | Title: {}...", t);
                    } else {
                        println!("✅ Loaded");
                    }
                } else {
                    println!("✅ Loaded");
                }
            }
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("net::ERR_") {
                    println!("❌ Network Error");
                } else if err_str.contains("timeout") {
                    println!("⏱️  Timeout");
                } else {
                    println!("❌ Error: {}", &err_str[..err_str.len().min(50)]);
                }
            }
        }
        
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }

    println!("\n=== Evaluation Complete ===\n");

    println!("Checking stealth detection results on bot.sannysoft.com...\n");
    
    let _ = session.tools.navigate("https://bot.sannysoft.com").await;
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Check for detection indicators
    let checks = vec![
        ("navigator.webdriver", "window.navigator.webdriver"),
        ("Chrome runtime", "window.chrome ? 'OK' : 'MISSING'"),
        ("Permissions API", "navigator.permissions ? 'OK' : 'PATCHED'"),
    ];

    println!("Stealth checks:");
    for (name, js) in checks {
        let result = session.client.send_command(
            "Runtime.evaluate",
            Some(serde_json::json!({ "expression": js }))
        ).await;
        
        if let Ok(resp) = result {
            let value = resp.get("result")
                .and_then(|r| r.get("value"))
                .and_then(|v| v.as_str())
                .unwrap_or("N/A");
            println!("  {}: {}", name, value);
        }
    }

    println!("\n");
    Ok(())
}
