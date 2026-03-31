use crate::event::{CatchMeEvent, EventType};
use arboard::Clipboard;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tracing::{debug, error};

pub async fn start_clipboard_recorder(tx: mpsc::Sender<CatchMeEvent>) {
    let mut clipboard = match Clipboard::new() {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to initialize clipboard recorder: {}", e);
            return;
        }
    };

    let mut last_content = String::new();

    loop {
        if let Ok(current_text) = clipboard.get_text()
            && current_text != last_content
            && !current_text.trim().is_empty()
        {
            debug!("New clipboard content captured ({} chars)", current_text.len());
            last_content = current_text.clone();

            let event = CatchMeEvent::new(
                "arboard-clipboard",
                EventType::Clipboard {
                    content: last_content.clone(),
                },
            );

            if tx.send(event).await.is_err() {
                break;
            }
        }
        sleep(Duration::from_secs(2)).await;
    }
}
