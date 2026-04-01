use bizclaw_core::traits::Tool;
use bizclaw_tools::stealth_browser::StealthBrowserTool;
use std::env;

#[tokio::main]
async fn main() {
    println!("🛡️  BIZCLAW CHANNEL AUTHORIZATION TOOL");
    println!("======================================");

    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("Usage: cargo run --example channel_login <URL> <PROFILE_NAME>");
        println!("Example: cargo run --example channel_login https://shopee.vn shopee_agent");
        return;
    }

    let url = &args[1];
    let profile = &args[2];

    println!("🌐 Launching Native Browser...");
    println!("   URL: {}", url);
    println!("   Profile: {}", profile);
    println!("\n👉 INSTRUCTIONS:");
    println!("1. Please log in manually inside the browser window.");
    println!("2. Solve any Captchas or OTPs if required.");
    println!("3. Close the browser window when you are done.");

    let tool = StealthBrowserTool::new();

    // Set headless to false to make it look like a human and bypass blocks
    let navigate_cmd = format!(
        r#"{{"action":"navigate", "url":"{}", "headless": false, "profile": "{}"}}"#,
        url, profile
    );

    match tool.execute(&navigate_cmd).await {
        Ok(res) => println!("{}", res.output),
        Err(e) => println!("❌ Failed to launch browser: {}", e),
    }

    // Keep alive until user closes the window or press Ctrl+C
    println!(
        "\n🟢 Browser session active. Press Ctrl+C to exit this script after closing the browser."
    );
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}
