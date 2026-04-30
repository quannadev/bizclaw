use crate::state::{AppState, ChannelStatus, Conversation};
use tauri::State;
use tracing::{error, info};

#[derive(Debug, serde::Serialize)]
pub struct StatusResponse {
    pub version: String,
    pub status: String,
    pub memory_usage_mb: u64,
    pub active_agents: usize,
    pub uptime_seconds: u64,
}

#[tauri::command]
pub async fn get_status() -> Result<StatusResponse, String> {
    Ok(StatusResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
        status: "running".to_string(),
        memory_usage_mb: 0,
        active_agents: 1,
        uptime_seconds: 0,
    })
}

#[tauri::command]
pub async fn send_message(
    state: State<'_, AppState>,
    conversation_id: String,
    message: String,
) -> Result<String, String> {
    info!("Sending message to conversation {}: {}", conversation_id, message);
    
    Ok(format!("Echo: {}", message))
}

#[tauri::command]
pub async fn get_conversations(state: State<'_, AppState>) -> Result<Vec<Conversation>, String> {
    let conversations = state.conversations.read().await;
    Ok(conversations.clone())
}

#[tauri::command]
pub async fn get_conversation(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<Option<Conversation>, String> {
    let conversations = state.conversations.read().await;
    Ok(conversations.iter().find(|c| c.id == conversation_id).cloned())
}

#[tauri::command]
pub async fn clear_conversation(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<(), String> {
    let mut conversations = state.conversations.write().await;
    conversations.retain(|c| c.id != conversation_id);
    info!("Cleared conversation: {}", conversation_id);
    Ok(())
}

#[tauri::command]
pub async fn get_channels(state: State<'_, AppState>) -> Result<Vec<ChannelStatus>, String> {
    let channels = state.channels.read().await;
    Ok(channels.clone())
}

#[tauri::command]
pub async fn connect_channel(
    state: State<'_, AppState>,
    channel_id: String,
) -> Result<(), String> {
    info!("Connecting channel: {}", channel_id);
    let mut channels = state.channels.write().await;
    
    if let Some(channel) = channels.iter_mut().find(|c| c.id == channel_id) {
        channel.connected = true;
    }
    
    Ok(())
}

#[tauri::command]
pub async fn disconnect_channel(
    state: State<'_, AppState>,
    channel_id: String,
) -> Result<(), String> {
    info!("Disconnecting channel: {}", channel_id);
    let mut channels = state.channels.write().await;
    
    if let Some(channel) = channels.iter_mut().find(|c| c.id == channel_id) {
        channel.connected = false;
    }
    
    Ok(())
}

#[tauri::command]
pub async fn get_skills() -> Result<Vec<SkillInfo>, String> {
    Ok(vec![
        SkillInfo {
            id: "developer".to_string(),
            name: "Developer Assistant".to_string(),
            description: "Code review, debugging, documentation".to_string(),
            version: "1.0.0".to_string(),
            author: "BizClaw".to_string(),
            installed: true,
        },
        SkillInfo {
            id: "business-writing".to_string(),
            name: "Business Writing".to_string(),
            description: "Emails, reports, proposals".to_string(),
            version: "1.0.0".to_string(),
            author: "BizClaw".to_string(),
            installed: false,
        },
        SkillInfo {
            id: "vietnamese-business".to_string(),
            name: "Vietnamese Business".to_string(),
            description: "Tiếng Việt business communication".to_string(),
            version: "1.0.0".to_string(),
            author: "BizClaw".to_string(),
            installed: false,
        },
    ])
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SkillInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub installed: bool,
}

#[tauri::command]
pub async fn install_skill(skill_id: String) -> Result<(), String> {
    info!("Installing skill: {}", skill_id);
    Ok(())
}

#[tauri::command]
pub async fn uninstall_skill(skill_id: String) -> Result<(), String> {
    info!("Uninstalling skill: {}", skill_id);
    Ok(())
}

#[tauri::command]
pub async fn get_settings() -> Result<Settings, String> {
    Ok(Settings {
        theme: "dark".to_string(),
        language: "en".to_string(),
        provider: "openai".to_string(),
        model: "gpt-4".to_string(),
        max_tokens: 4000,
        temperature: 0.7,
    })
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    pub theme: String,
    pub language: String,
    pub provider: String,
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f32,
}

#[tauri::command]
pub async fn update_settings(settings: Settings) -> Result<(), String> {
    info!("Updating settings: {:?}", settings);
    Ok(())
}

#[tauri::command]
pub async fn search_memory(query: String) -> Result<Vec<MemoryResult>, String> {
    Ok(vec![])
}

#[derive(Debug, serde::Serialize)]
pub struct MemoryResult {
    pub id: String,
    pub content: String,
    pub score: f32,
}

#[tauri::command]
pub async fn save_memory(key: String, value: String) -> Result<(), String> {
    info!("Saving memory: {} -> {}", key, value);
    Ok(())
}

#[tauri::command]
pub async fn get_memory_stats() -> Result<MemoryStats, String> {
    Ok(MemoryStats {
        total_entries: 0,
        size_mb: 0.0,
        last_updated: chrono::Utc::now().timestamp(),
    })
}

#[derive(Debug, serde::Serialize)]
pub struct MemoryStats {
    pub total_entries: u64,
    pub size_mb: f64,
    pub last_updated: i64,
}

#[tauri::command]
pub async fn start_browser() -> Result<String, String> {
    info!("Starting browser");
    Ok("browser_1".to_string())
}

#[tauri::command]
pub async fn close_browser(browser_id: String) -> Result<(), String> {
    info!("Closing browser: {}", browser_id);
    Ok(())
}

#[tauri::command]
pub async fn browser_navigate(browser_id: String, url: String) -> Result<(), String> {
    info!("Browser {} navigating to: {}", browser_id, url);
    Ok(())
}

#[tauri::command]
pub async fn browser_screenshot(browser_id: String) -> Result<String, String> {
    info!("Taking screenshot from browser: {}", browser_id);
    Ok("base64_encoded_screenshot".to_string())
}
