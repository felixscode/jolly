use std::sync::Arc;
use tokio::sync::Mutex;

use tauri::Emitter;
use tauri::Manager;
use tauri_plugin_store::StoreExt;

mod commands;
pub mod inference;

use inference::download::DownloadManager;
use inference::model_manager;

/// Resolve the model path from settings. Runs on main thread (store access).
/// Returns None if no model is configured or file doesn't exist.
fn resolve_startup_model_path(app: &tauri::AppHandle) -> Option<std::path::PathBuf> {
    let app_data = match app.path().app_data_dir() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[jolly] Failed to get app data directory: {}", e);
            return None;
        }
    };

    let models_path = match model_manager::models_dir(&app_data) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[jolly] Failed to get models directory: {}", e);
            return None;
        }
    };

    let active_model_id: Option<String> = app
        .store("settings.json")
        .ok()
        .and_then(|store| store.get("activeModelId"))
        .and_then(|v| v.as_str().map(|s| s.to_string()));

    if let Some(ref id) = active_model_id {
        if id.starts_with("custom-") {
            let path_str = commands::get_custom_model_path(app, id);
            match path_str {
                Some(p) => {
                    let path = std::path::PathBuf::from(&p);
                    if path.exists() {
                        Some(path)
                    } else {
                        eprintln!("[jolly] Custom model file not found: {}", p);
                        None
                    }
                }
                None => {
                    eprintln!("[jolly] Custom model ID not found in settings: {}", id);
                    None
                }
            }
        } else {
            match model_manager::resolve_model_path(&models_path, active_model_id.as_deref()) {
                Ok(p) => Some(p),
                Err(e) => {
                    eprintln!("[jolly] No local model available: {}", e);
                    None
                }
            }
        }
    } else {
        match model_manager::resolve_model_path(&models_path, None) {
            Ok(p) => Some(p),
            Err(e) => {
                eprintln!("[jolly] No local model available: {}", e);
                None
            }
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_keyring::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(Arc::new(Mutex::new(DownloadManager::new())))
        .setup(|app| {
            let model_path = resolve_startup_model_path(app.handle());
            if let Some(path) = model_path {
                let handle = app.handle().clone();
                std::thread::spawn(move || {
                    match inference::local::init_model(&path) {
                        Ok(()) => {
                            eprintln!("[jolly] Local model loaded: {:?}", path);
                            let _ = handle.emit("model-loaded", ());
                        }
                        Err(e) => eprintln!("[jolly] Failed to load local model: {}", e),
                    }
                });
            }
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
            commands::import_custom_model,
            commands::validate_custom_models,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
