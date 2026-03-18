use std::sync::Arc;
use tokio::sync::Mutex;

use tauri::AppHandle;
use tauri::Manager;
use tauri_plugin_keyring::KeyringExt;
use tauri_plugin_store::StoreExt;

use crate::inference::download::{
    download_model, get_model_state, DownloadManager, DownloadState,
};
use crate::inference::harper::HarperProvider;
use crate::inference::local::LocalProvider;
use crate::inference::model_manager::models_dir;
use crate::inference::openrouter::OpenRouterProvider;
use crate::inference::registry::{self, ModelEntry, MODELS};
use crate::inference::LLMProvider;

/// Read the API key: keyring first, then env var fallback.
fn get_api_key(app: &AppHandle) -> Result<String, String> {
    if let Ok(Some(key)) =
        app.keyring()
            .get_password("com.jolly.desktop", "openrouter_api_key")
    {
        if !key.is_empty() {
            return Ok(key);
        }
    }

    std::env::var("OPENROUTER_API_KEY")
        .map_err(|_| "no_api_key".to_string())
}

/// Check if a local model is selected in the store.
fn get_active_model_id(app: &AppHandle) -> Option<String> {
    let store = app.store("settings.json").ok()?;
    store
        .get("activeModelId")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
}

/// Check if the user has toggled "use OpenRouter" in settings.
fn get_use_openrouter(app: &AppHandle) -> bool {
    app.store("settings.json")
        .ok()
        .and_then(|store| store.get("useOpenRouter"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

/// Check if the user has toggled "use Harper" in settings.
fn get_use_harper(app: &AppHandle) -> bool {
    app.store("settings.json")
        .ok()
        .and_then(|store| store.get("useHarper"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

#[tauri::command]
pub async fn correct_text(app: AppHandle, text: String) -> Result<String, String> {
    if text.trim().is_empty() {
        return Ok(String::new());
    }

    let use_harper = get_use_harper(&app);
    let use_openrouter = get_use_openrouter(&app);
    let active_model = get_active_model_id(&app);
    let has_local = active_model.is_some() && crate::inference::local::is_model_loaded();

    // Priority 1: Harper (lightweight, instant)
    if use_harper {
        eprintln!("[jolly] Using Harper grammar checker");
        let harper = HarperProvider::new();
        return harper.correct_text(&text).await.map_err(|e| {
            eprintln!("[jolly] Harper error: {}", e);
            "harper_failed".to_string()
        });
    }

    // Priority 2: Local model (loaded and not overridden by OpenRouter)
    if has_local && !use_openrouter {
        eprintln!("[jolly] Using local inference");
        let local = LocalProvider::new();
        return local.correct_text(&text).await.map_err(|e| {
            eprintln!("[jolly] Local inference error: {}", e);
            "local_inference_failed".to_string()
        });
    }

    // Priority 3: Model selected but not loaded
    if !use_openrouter && !has_local && active_model.is_some() {
        return Err("model_not_loaded".to_string());
    }

    // Priority 4: OpenRouter
    if use_openrouter {
        eprintln!("[jolly] Using OpenRouter API");
        let api_key = get_api_key(&app)?;
        let openrouter = OpenRouterProvider::new(api_key);
        return openrouter.correct_text(&text).await.map_err(|e| {
            eprintln!("[jolly] OpenRouter error: {}", e);
            if e.contains("401") || e.contains("403") {
                "bad_api_key".to_string()
            } else {
                "api_failed".to_string()
            }
        });
    }

    // Priority 5: Nothing configured
    Err("no_provider_configured".to_string())
}

// ── Download management commands ────────────────────────────────

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelWithState {
    #[serde(flatten)]
    pub entry: ModelEntry,
    #[serde(flatten)]
    pub download_state: DownloadState,
}

#[tauri::command]
pub async fn list_available_models(app: AppHandle) -> Result<Vec<ModelWithState>, String> {
    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let models_path = models_dir(&app_data)?;

    let result: Vec<ModelWithState> = MODELS
        .iter()
        .map(|m| ModelWithState {
            entry: m.clone(),
            download_state: get_model_state(&models_path, m),
        })
        .collect();

    Ok(result)
}

#[tauri::command]
pub async fn list_downloaded_models(app: AppHandle) -> Result<Vec<String>, String> {
    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let models_path = models_dir(&app_data)?;

    let ids: Vec<String> = MODELS
        .iter()
        .filter(|m| models_path.join(m.file_name).exists())
        .map(|m| m.id.to_string())
        .collect();

    Ok(ids)
}

#[tauri::command]
pub async fn start_download(
    app: AppHandle,
    state: tauri::State<'_, Arc<Mutex<DownloadManager>>>,
    model_id: String,
) -> Result<(), String> {
    let model = registry::find_model(&model_id).ok_or("Unknown model ID")?;

    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let models_path = models_dir(&app_data)?;

    // Check if already downloaded
    if models_path.join(model.file_name).exists() {
        return Err("Model already downloaded".to_string());
    }

    // Check if a download is already active
    let mut manager = state.lock().await;
    if manager.is_active() {
        return Err("A download is already in progress".to_string());
    }

    let cancel_token = manager.start(&model_id);
    let dm = state.inner().clone();
    drop(manager); // Release lock before spawning

    let app_clone = app.clone();
    let model_clone = model.clone();

    tokio::spawn(async move {
        let _ = download_model(app_clone, &model_clone, models_path, cancel_token).await;
        dm.lock().await.finish();
    });

    Ok(())
}

#[tauri::command]
pub async fn cancel_download(
    state: tauri::State<'_, Arc<Mutex<DownloadManager>>>,
) -> Result<(), String> {
    let mut manager = state.lock().await;
    manager.cancel();
    Ok(())
}

#[tauri::command]
pub async fn activate_model(app: AppHandle, model_id: String) -> Result<(), String> {
    let model = registry::find_model(&model_id).ok_or("Unknown model ID")?;

    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let models_path = models_dir(&app_data)?;
    let model_path = models_path.join(model.file_name);

    if !model_path.exists() {
        return Err(format!("Model file not found: {}", model.file_name));
    }

    tokio::task::spawn_blocking(move || {
        crate::inference::local::swap_model(&model_path)
    })
    .await
    .map_err(|e| format!("Task failed: {}", e))?
}

#[tauri::command]
pub async fn delete_model(app: AppHandle, model_id: String) -> Result<(), String> {
    let model = registry::find_model(&model_id).ok_or("Unknown model ID")?;

    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let models_path = models_dir(&app_data)?;

    // Delete .gguf
    let gguf = models_path.join(model.file_name);
    if gguf.exists() {
        std::fs::remove_file(&gguf).map_err(|e| format!("Failed to delete model: {}", e))?;
    }

    // Also clean up any partial files
    let part = models_path.join(format!("{}.part", model.file_name));
    let meta = models_path.join(format!("{}.meta", model.file_name));
    let _ = std::fs::remove_file(&part);
    let _ = std::fs::remove_file(&meta);

    Ok(())
}
