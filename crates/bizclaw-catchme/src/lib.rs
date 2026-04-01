//! CatchMe - Secure Digital Footprint Capture
//!
//! Ported to Rust for safety, memory efficiency (~50MB instead of ~200MB Python),
//! and seamless integration with the BizClaw ecosystem.

pub mod event;
pub mod record;
pub mod store;
pub mod tree;
// pub mod summarize;

use anyhow::Result;
use event::CatchMeEvent;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatchMeConfig {
    pub db_path: String,
    pub summarize_language: String,
    pub enable_mouse: bool,
    pub enable_keyboard: bool,
    pub enable_window: bool,
    pub enable_clipboard: bool,
}

impl Default for CatchMeConfig {
    fn default() -> Self {
        Self {
            db_path: "~/.gemini/antigravity/catchme.db".to_string(), // Default safe location
            summarize_language: "vi".to_string(),
            enable_mouse: true,
            enable_keyboard: true,
            enable_window: true,
            enable_clipboard: true,
        }
    }
}

pub struct CatchMeEngine {
    config: CatchMeConfig,
    event_tx: mpsc::Sender<CatchMeEvent>,
}

impl CatchMeEngine {
    pub fn new(config: CatchMeConfig) -> Result<Self> {
        let (tx, mut rx) = mpsc::channel(1024);

        let db_path = config.db_path.clone();

        // Background worker to persist events into SQLite store
        tokio::spawn(async move {
            info!("CatchMe engine started. Waiting for events...");
            let store = match store::CatchMeStore::new(&db_path) {
                Ok(s) => {
                    info!("CatchMe store opened at {}", db_path);
                    Some(s)
                }
                Err(e) => {
                    tracing::error!("CatchMe store init failed: {}. Events will be dropped.", e);
                    None
                }
            };
            while let Some(evt) = rx.recv().await {
                if let Some(ref s) = store {
                    if let Err(e) = s.insert_event(&evt) {
                        tracing::warn!("CatchMe event persist failed: {}", e);
                    }
                }
            }
        });

        if config.enable_clipboard {
            tokio::spawn(crate::record::clipboard::start_clipboard_recorder(
                tx.clone(),
            ));
            info!("CatchMe clipboard recorder started.");
        }

        if config.enable_window {
            tokio::spawn(crate::record::window::start_window_recorder(tx.clone()));
            info!("CatchMe window recorder started.");
        }

        Ok(Self {
            config,
            event_tx: tx,
        })
    }

    /// Access the current engine configuration.
    pub fn config(&self) -> &CatchMeConfig {
        &self.config
    }

    pub fn sender(&self) -> mpsc::Sender<CatchMeEvent> {
        self.event_tx.clone()
    }
}
