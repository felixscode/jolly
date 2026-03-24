use std::sync::Arc;
use tokio::sync::Mutex;

use tauri::Emitter;
use tauri::Manager;
use tauri_plugin_store::StoreExt;

mod commands;
pub mod inference;

use inference::download::DownloadManager;
use inference::model_manager;
use inference::registry;

/// Resolve the model path from settings. Runs on main thread (store access).
/// Returns None if no model is configured or file doesn't exist.
fn resolve_startup_model(app: &tauri::AppHandle) -> Option<(std::path::PathBuf, String, Option<&'static str>)> {
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
                        return Some((path, id.clone(), None));
                    }
                    eprintln!("[jolly] Custom model file not found: {}", p);
                }
                None => {
                    eprintln!("[jolly] Custom model ID not found in settings: {}", id);
                }
            }
        } else if let Some(model) = registry::find_model(id) {
            let path = models_path.join(model.file_name);
            if path.exists() {
                return Some((path, id.clone(), model.prompt_template));
            }
            eprintln!("[jolly] Model file not found: {}", model.file_name);
        } else {
            eprintln!("[jolly] Active model '{}' no longer in registry, ignoring", id);
        }
    }

    // Fallback: try to find any registry model that's downloaded
    for model in registry::MODELS {
        let path = models_path.join(model.file_name);
        if path.exists() {
            eprintln!("[jolly] Falling back to downloaded model: {}", model.name);
            return Some((path, model.id.to_string(), model.prompt_template));
        }
    }

    eprintln!("[jolly] No local models available");
    None
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
            let startup_model = resolve_startup_model(app.handle());
            if let Some((path, model_id, prompt_template)) = startup_model {
                let handle = app.handle().clone();
                std::thread::spawn(move || {
                    match inference::local::init_model(&path, &model_id, prompt_template) {
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
