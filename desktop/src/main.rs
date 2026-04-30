#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod commands;
mod state;
mod system_tray;

use tauri::Manager;
use tracing::{error, info};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

fn main() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .init();

    info!("Starting BizClaw Desktop v{}", env!("CARGO_PKG_VERSION"));

    let result = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            info!("Application setup starting");
            
            let app_state = state::AppState::new(app.handle().clone())?;
            app.manage(app_state);
            
            if let Err(e) = system_tray::setup_system_tray(app) {
                error!("Failed to setup system tray: {}", e);
            }
            
            info!("Application setup complete");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_status,
            commands::send_message,
            commands::get_conversations,
            commands::get_conversation,
            commands::clear_conversation,
            commands::get_channels,
            commands::connect_channel,
            commands::disconnect_channel,
            commands::get_skills,
            commands::install_skill,
            commands::uninstall_skill,
            commands::get_settings,
            commands::update_settings,
            commands::search_memory,
            commands::save_memory,
            commands::get_memory_stats,
            commands::start_browser,
            commands::close_browser,
            commands::browser_navigate,
            commands::browser_screenshot,
        ])
        .run(tauri::generate_context!());

    if let Err(e) = result {
        error!("Application error: {}", e);
        std::process::exit(1);
    }
}
