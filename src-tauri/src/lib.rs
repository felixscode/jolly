use std::sync::Arc;
use tokio::sync::Mutex;

use tauri::Manager;
use tauri_plugin_store::StoreExt;

mod commands;
pub mod inference;

use inference::download::DownloadManager;
use inference::model_manager;

fn load_local_model(app: &tauri::AppHandle) {
    let app_data = match app.path().app_data_dir() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[jolly] Failed to get app data directory: {}", e);
            return;
        }
    };

    let models_path = match model_manager::models_dir(&app_data) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[jolly] Failed to get models directory: {}", e);
            return;
        }
    };

    // Read activeModelId from store (if set)
    let active_model_id: Option<String> = app
        .store("settings.json")
        .ok()
        .and_then(|store| store.get("activeModelId"))
        .and_then(|v| v.as_str().map(|s| s.to_string()));

    let model_path =
        match model_manager::resolve_model_path(&models_path, active_model_id.as_deref()) {
            Ok(p) => p,
            Err(e) => {
                println!("[jolly] No local model available: {}", e);
                return;
            }
        };

    match inference::local::init_model(&model_path) {
        Ok(()) => println!("[jolly] Local model loaded: {:?}", model_path),
        Err(e) => eprintln!("[jolly] Failed to load local model: {}", e),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_keyring::init())
        .manage(Arc::new(Mutex::new(DownloadManager::new())))
        .setup(|app| {
            load_local_model(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::correct_text,
            commands::list_available_models,
            commands::list_downloaded_models,
            commands::start_download,
            commands::cancel_download,
            commands::delete_model,
            commands::activate_model,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
