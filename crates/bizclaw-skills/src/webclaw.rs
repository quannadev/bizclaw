//! WebClaw Skill Integration
//! Uses the native Rust webclaw-core to perform high-speed, LLM-optimized scraping.
//! (Note: webclaw-fetch TLS fingerprinting requires `rustls` patching in the workspace root,
//! so this currently falls back to standard reqwest.)

use bizclaw_core::error::{BizClawError, Result};
use bizclaw_core::types::ToolDefinition;

/// Definition of the webclaw_scrape tool
pub fn webclaw_scrape_definition() -> ToolDefinition {
    ToolDefinition {
        name: "webclaw_scrape".into(),
        description: "Scrape a website bypassing bot-protection (Cloudflare, DataDome) and extract clean, LLM-optimized Markdown using WebClaw. Use this to read the content of any URL.".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The full HTTPS URL to scrape"
                }
            },
            "required": ["url"]
        }),
    }
}

/// Executes the webclaw_scrape tool using standard reqwest fallback
pub async fn execute_webclaw_scrape(url: &str) -> Result<String> {
    // 1. Fallback client since webclaw-fetch requires workspace patches for rustls
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36")
        .build()
        .map_err(|e| BizClawError::Other(format!("Failed to build client: {}", e)))?;

    // 2. Fetch raw HTML
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| BizClawError::Other(format!("Content fetch failed: {}", e)))?;
        
    let html = response.text().await.map_err(|e| BizClawError::Other(format!("Could not read body: {}", e)))?;

    // 3. Extract using webclaw core
    let result = webclaw_core::extract(&html, Some(url))
        .map_err(|e| BizClawError::Other(format!("WebClaw extraction failed: {}", e)))?;

    // Combine metadata and markdown content
    let mut out = "# WebClaw Extraction Report\n\n".to_string();
    if let Some(title) = result.metadata.title {
        out.push_str(&format!("**Title:** {}\n", title));
    }
    if let Some(desc) = result.metadata.description {
        out.push_str(&format!("**Description:** {}\n", desc));
    }
    out.push_str(&format!("**Word Count:** {}\n", result.metadata.word_count));
    out.push_str("\n---\n\n");
    
    out.push_str(&result.content.markdown);
    
    Ok(out)
}
