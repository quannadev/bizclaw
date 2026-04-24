use bizclaw_browser::{BrowserSession, SessionConfig, StealthConfig};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env()
            .add_directive(tracing::Level::INFO.into()))
        .init();

    println!("\n=== CDP Browser Test Suite ===\n");

    println!("1. Testing connection to Chrome...");
    let config = SessionConfig {
        chrome_debug_port: 9223,
        page_id: Some("10ED784F1690CF6239F8AC87639C8B15".to_string()),
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
    println!("   ✅ Connected successfully!");
    println!("   Session ID: {}\n", session.id);

    println!("2. Testing navigation to simple test page...");
    let result = session.tools.navigate("https://example.com").await?;
    println!("   ✅ Navigation result: {:?}", result.success);
    if let Some(data) = result.data {
        println!("   URL: {:?}", data.get("url"));
    }
    println!();

    println!("3. Testing page info extraction...");
    let page_info = session.tools.get_page_info().await?;
    println!("   ✅ Page info: {:?}", page_info.success);
    println!();

    println!("4. Testing content extraction...");
    let h1 = session.tools.get_text("h1").await?;
    println!("   H1 text: {:?}", h1.data);
    println!();

    println!("5. Testing screenshot...");
    if let Ok(screenshot) = session.tools.screenshot().await {
        let size = screenshot.len();
        println!("   ✅ Screenshot captured: {} bytes", size);
    } else {
        println!("   ⚠️  Screenshot failed (expected in headless)");
    }
    println!();

    println!("6. Testing extract_all links...");
    let links = session.tools.extract_all("a").await?;
    if let Some(data) = links.data {
        if let Some(count) = data.get("count") {
            println!("   ✅ Found {} links", count);
        }
    }
    println!();

    println!("7. Testing wait_for_selector...");
    let wait_result = session.tools.wait_for_selector("body", 5000).await;
    println!("   ✅ wait_for_selector result: {:?}", wait_result.is_ok());
    println!();

    println!("8. Testing keyboard input...");
    let key_result = session.tools.press_key("End").await;
    println!("   ✅ press_key result: {:?}", key_result.is_ok());
    println!();

    println!("9. Testing stealth features...");
    if let Some(stealth) = &session.stealth {
        let config = stealth.config();
        println!("   Stealth enabled: {}", config.enabled);
        println!("   Canvas noise: {}", config.canvas_noise);
        println!("   WebGL spoofing: {}", config.webgl_spoofing);
        println!("   Human delays: {}", config.human_delays);
        println!("   Viewport randomization: {}", config.viewport_randomization);
        println!("   Timezone spoofing: {}", config.timezone_spoofing);
    }
    println!();

    println!("=== Test Suite Complete ===\n");

    println!("Now testing against bot detection sites...\n");

    println!("10. Testing Google (should be OK)...");
    let result = session.tools.navigate("https://www.google.com").await;
    match result {
        Ok(r) => println!("   ✅ Google loaded: {:?}", r.success),
        Err(e) => println!("   ❌ Google failed: {:?}", e),
    }
    println!();

    println!("11. Testing DuckDuckGo (privacy-focused)...");
    let result = session.tools.navigate("https://duckduckgo.com").await;
    match result {
        Ok(r) => println!("   ✅ DuckDuckGo loaded: {:?}", r.success),
        Err(e) => println!("   ❌ DuckDuckGo failed: {:?}", e),
    }
    println!();

    println!("12. Testing Bot detection test site...");
    let result = session.tools.navigate("https://bot.sannysoft.com").await;
    match result {
        Ok(r) => {
            println!("   ✅ Detection test page loaded");
            let content = session.tools.extract_content("body").await;
            if let Ok(c) = content {
                if let Some(data) = c.data {
                    let text = data.get("text").and_then(|t| t.as_str()).unwrap_or("");
                    println!("   Page content preview: {}...", &text[..text.len().min(200)]);
                }
            }
        }
        Err(e) => println!("   ❌ Detection test page failed: {:?}", e),
    }
    println!();

    println!("13. Testing Cloudflare challenge (if any)...");
    let result = session.tools.navigate("https://nowsecure.nl").await;
    match result {
        Ok(r) => {
            println!("   ✅ Page loaded");
            let content = session.tools.extract_content("title").await;
            if let Ok(c) = content {
                println!("   Title: {:?}", c.data);
            }
        }
        Err(e) => println!("   ❌ Page blocked/challenged: {:?}", e),
    }
    println!();

    println!("=== Real-World Test Complete ===\n");

    Ok(())
}
