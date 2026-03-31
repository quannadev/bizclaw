use crate::event::{CatchMeEvent, EventType};
use active_win_pos_rs::get_active_window;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tracing::debug;

pub async fn start_window_recorder(tx: mpsc::Sender<CatchMeEvent>) {
    let mut last_app = String::new();
    let mut last_title = String::new();

    loop {
        match get_active_window() {
            Ok(window) => {
                if window.app_name != last_app || window.title != last_title {
                    debug!("Focus changed to app: {}, title: {}", window.app_name, window.title);
                    last_app = window.app_name.clone();
                    last_title = window.title.clone();

                    let event = CatchMeEvent::new(
                        "active-win-pos",
                        EventType::Window {
                            title: last_title.clone(),
                            app: last_app.clone(),
                        },
                    );

                    if tx.send(event).await.is_err() {
                        break;
                    }
                }
            }
            Err(e) => {
                // Ignore temporary failures like no active window
                debug!("Could not get active window: {:?}", e);
            }
        }
        sleep(Duration::from_secs(3)).await;
    }
}
