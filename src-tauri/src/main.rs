// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod database;
mod mattermost;
mod models;

use database::Database;
use mattermost::MattermostClient;
use models::{AppState, Channel, Message, Settings};
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

#[tauri::command]
async fn connect_to_server(
    server_url: String,
    token: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<String, String> {
    let mut app_state = state.lock().await;

    let client = MattermostClient::new(server_url.clone(), token.clone())
        .map_err(|e| e.to_string())?;

    // Test connection
    client.get_user_info().await
        .map_err(|e| format!("Failed to connect: {}", e))?;

    app_state.client = Some(client);
    app_state.settings.server_url = server_url;
    app_state.settings.auth_token = token;

    // Save settings
    app_state.db.save_settings(&app_state.settings)
        .map_err(|e| e.to_string())?;

    Ok("Connected successfully".to_string())
}

#[tauri::command]
async fn get_channels(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<Channel>, String> {
    let app_state = state.lock().await;

    if let Some(client) = &app_state.client {
        let channels = client.get_channels().await
            .map_err(|e| e.to_string())?;

        // Cache channels locally
        for channel in &channels {
            app_state.db.save_channel(channel)
                .map_err(|e| e.to_string())?;
        }

        Ok(channels)
    } else {
        // Return cached channels if offline
        app_state.db.get_channels()
            .map_err(|e| e.to_string())
    }
}

#[tauri::command]
async fn get_messages(
    channel_id: String,
    limit: i32,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<Message>, String> {
    let app_state = state.lock().await;

    if let Some(client) = &app_state.client {
        // Try to fetch from server
        match client.get_channel_messages(&channel_id, limit).await {
            Ok(messages) => {
                // Cache messages locally
                for message in &messages {
                    let _ = app_state.db.save_message(message);
                }
                Ok(messages)
            }
            Err(_) => {
                // Fallback to cached messages
                app_state.db.get_messages(&channel_id, limit)
                    .map_err(|e| e.to_string())
            }
        }
    } else {
        // Return cached messages if offline
        app_state.db.get_messages(&channel_id, limit)
            .map_err(|e| e.to_string())
    }
}

#[tauri::command]
async fn send_message(
    channel_id: String,
    message: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<String, String> {
    let app_state = state.lock().await;

    if let Some(client) = &app_state.client {
        client.send_message(&channel_id, &message).await
            .map_err(|e| e.to_string())?;
        Ok("Message sent".to_string())
    } else {
        // Queue message for later if offline
        app_state.db.queue_message(&channel_id, &message)
            .map_err(|e| e.to_string())?;
        Ok("Message queued (offline)".to_string())
    }
}

#[tauri::command]
async fn get_settings(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Settings, String> {
    let app_state = state.lock().await;
    Ok(app_state.settings.clone())
}

#[tauri::command]
async fn save_settings(
    settings: Settings,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<String, String> {
    let mut app_state = state.lock().await;
    app_state.db.save_settings(&settings)
        .map_err(|e| e.to_string())?;
    app_state.settings = settings;
    Ok("Settings saved".to_string())
}

#[tauri::command]
async fn sync_queued_messages(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<i32, String> {
    let app_state = state.lock().await;

    if let Some(client) = &app_state.client {
        let queued = app_state.db.get_queued_messages()
            .map_err(|e| e.to_string())?;

        let mut sent_count = 0;
        for (id, channel_id, message) in queued {
            match client.send_message(&channel_id, &message).await {
                Ok(_) => {
                    app_state.db.remove_queued_message(id)
                        .map_err(|e| e.to_string())?;
                    sent_count += 1;
                }
                Err(_) => break, // Stop if we fail to send
            }
        }

        Ok(sent_count)
    } else {
        Ok(0)
    }
}

fn main() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let db = Database::new("minimalchat.db").expect("Failed to initialize database");
            let settings = db.get_settings().unwrap_or_default();

            let app_state = Arc::new(Mutex::new(AppState {
                client: None,
                db,
                settings,
            }));

            app.manage(app_state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            connect_to_server,
            get_channels,
            get_messages,
            send_message,
            get_settings,
            save_settings,
            sync_queued_messages,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
