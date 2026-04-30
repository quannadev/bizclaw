use std::sync::Arc;
use tauri::AppHandle;
use tokio::sync::RwLock;
use tracing::info;

pub struct AppState {
    pub app_handle: AppHandle,
    pub gateway_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    pub conversations: Arc<RwLock<Vec<Conversation>>>,
    pub channels: Arc<RwLock<Vec<ChannelStatus>>>,
    pub memory_store: Arc<RwLock<Option<bizclaw_memory_redb::RedbStore>>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub message_count: usize,
    pub last_message: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChannelStatus {
    pub id: String,
    pub name: String,
    pub channel_type: String,
    pub connected: bool,
    pub message_count: usize,
    pub last_activity: Option<i64>,
}

impl AppState {
    pub fn new(app_handle: AppHandle) -> anyhow::Result<Self> {
        let data_dir = app_handle
            .path()
            .app_data_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."));

        std::fs::create_dir_all(&data_dir)?;
        std::fs::create_dir_all(data_dir.join("memory"))?;
        std::fs::create_dir_all(data_dir.join("logs"))?;

        info!("App data directory: {:?}", data_dir);

        Ok(Self {
            app_handle,
            gateway_handle: Arc::new(RwLock::new(None)),
            conversations: Arc::new(RwLock::new(Vec::new())),
            channels: Arc::new(RwLock::new(Vec::new())),
            memory_store: Arc::new(RwLock::new(None)),
        })
    }

    pub fn data_dir(&self) -> std::path::PathBuf {
        self.app_handle
            .path()
            .app_data_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
    }
}
