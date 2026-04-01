use bizclaw_core::traits::Tool;
use bizclaw_tools::stealth_browser::StealthBrowserTool;
use std::time::Duration;

#[tokio::main]
async fn main() {
    println!("🚀 Starting Stealth Browser Demo...");
    let tool = StealthBrowserTool::new();

    // Set headless to false or true
    println!("🌐 Navigating to Shopee...");
    let navigate_cmd = r#"{"action":"navigate", "url":"https://shopee.vn/search?keyword=Neom%20Cosmetics", "headless": false, "profile": "shopee_demo"}"#;
    let res = tool.execute(navigate_cmd).await.unwrap();
    println!("{}", res.output);

    // Wait for JS to render the posts
    println!("⏳ Waiting 8 seconds for page content to load...");
    tokio::time::sleep(Duration::from_secs(8)).await;

    // Extract text
    println!("📄 Taking raw text...");
    let txt_cmd = r#"{"action":"text"}"#;
    let txt_res = tool.execute(txt_cmd).await.unwrap();

    // Print lines containing "Neom" or products
    let output = &txt_res.output;
    println!("Text size: {}", output.len());
    let prefix = std::cmp::min(output.len(), 5000);
    println!("{}", &output[..prefix]);
    println!("Demo Complete.");
}
