use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

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
use crate::inference::providers::{find_provider, keyring_account, ProviderPreset, PROVIDERS};
use crate::inference::registry::{self, ModelEntry, MODELS};
use crate::inference::remote::RemoteProvider;
use crate::inference::LLMProvider;

/// Read a provider's API key: keyring first, then the preset's env var.
fn get_api_key(app: &AppHandle, preset: &ProviderPreset) -> Option<String> {
    if let Ok(Some(key)) = app
        .keyring()
        .get_password("com.jolly.desktop", &keyring_account(preset.id))
    {
        if !key.trim().is_empty() {
            return Some(key);
        }
    }
    std::env::var(preset.key_env)
        .ok()
        .filter(|k| !k.trim().is_empty())
}

fn store_value(app: &AppHandle, key: &str) -> Option<serde_json::Value> {
    app.store("settings.json").ok()?.get(key)
}

fn store_bool(app: &AppHandle, key: &str) -> Option<bool> {
    store_value(app, key).and_then(|v| v.as_bool())
}

/// Check if a local model is selected in the store.
fn get_active_model_id(app: &AppHandle) -> Option<String> {
    store_value(app, "activeModelId").and_then(|v| v.as_str().map(|s| s.to_string()))
}

/// Whether the remote (cloud API) provider is enabled. Falls back to the
/// pre-0.2 `useOpenRouter` flag so upgrading keeps the user's choice.
fn get_use_remote(app: &AppHandle) -> bool {
    store_bool(app, "useRemote")
        .or_else(|| store_bool(app, "useOpenRouter"))
        .unwrap_or(false)
}

/// Check if the user has toggled "use Harper" in settings.
fn get_use_harper(app: &AppHandle) -> bool {
    store_bool(app, "useHarper").unwrap_or(false)
}

/// Per-provider overrides stored as `remoteConfigs[<id>] = { model, baseUrl }`.
#[derive(Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteConfig {
    model: Option<String>,
    base_url: Option<String>,
}

fn non_empty(s: Option<String>) -> Option<String> {
    s.filter(|v| !v.trim().is_empty())
}

/// Build the remote provider from settings, filling gaps with preset defaults.
fn build_remote_provider(app: &AppHandle) -> Result<(RemoteProvider, &'static str), String> {
    let provider_id = store_value(app, "remoteProvider")
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| "openrouter".to_string());
    let preset = find_provider(&provider_id)
        .or_else(|| find_provider("openrouter"))
        .ok_or("unknown provider")?;

    let config: RemoteConfig = store_value(app, "remoteConfigs")
        .and_then(|v| v.get(preset.id).cloned())
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();

    let api_key = get_api_key(app, preset);
    if preset.needs_key && api_key.is_none() {
        return Err("no_api_key".to_string());
    }

    let provider = RemoteProvider::new(
        preset.kind,
        non_empty(config.base_url).unwrap_or_else(|| preset.base_url.to_string()),
        non_empty(config.model).unwrap_or_else(|| preset.default_model.to_string()),
        api_key,
    );
    Ok((provider, preset.name))
}

/// Correct `text` with whichever provider the settings select. Shared by the
/// `correct_text` command and the global hotkey. Errors are short codes the
/// UI maps to messages (`no_api_key`, `bad_model`, ...).
pub async fn run_correction(app: &AppHandle, text: &str) -> Result<String, String> {
    if text.trim().is_empty() {
        return Ok(String::new());
    }

    let use_harper = get_use_harper(app);
    let use_remote = get_use_remote(app);
    let active_model = get_active_model_id(app);
    // A loaded model is usable even if the store has no id yet (startup fallback).
    let has_local = crate::inference::local::is_model_loaded();

    // Priority 1: Harper (lightweight, instant)
    if use_harper {
        eprintln!("[jolly] Using Harper grammar checker");
        let harper = HarperProvider::new();
        return harper.correct_text(text).await.map_err(|e| {
            eprintln!("[jolly] Harper error: {}", e);
            "harper_failed".to_string()
        });
    }

    // Priority 2: Local model (loaded and not overridden by a remote provider)
    if has_local && !use_remote {
        eprintln!("[jolly] Using local inference");
        let local = LocalProvider::new();
        return local.correct_text(text).await.map_err(|e| {
            eprintln!("[jolly] Local inference error: {}", e);
            "local_inference_failed".to_string()
        });
    }

    // Priority 3: Model selected but not loaded
    if !use_remote && !has_local && active_model.is_some() {
        return Err("model_not_loaded".to_string());
    }

    // Priority 4: Remote API
    if use_remote {
        let (remote, name) = build_remote_provider(app)?;
        eprintln!("[jolly] Using remote API: {} ({})", name, remote.model);
        return remote.correct_text(text).await.map_err(|e| {
            eprintln!("[jolly] Remote API error: {}", e);
            // Keep only the code prefix ("bad_api_key: HTTP 401 ..." -> "bad_api_key").
            e.split(':').next().unwrap_or("api_failed").to_string()
        });
    }

    // Priority 5: Nothing configured
    Err("no_provider_configured".to_string())
}

#[tauri::command]
pub async fn correct_text(app: AppHandle, text: String) -> Result<String, String> {
    run_correction(&app, &text).await
}

#[tauri::command]
pub fn list_remote_providers() -> Vec<ProviderPreset> {
    PROVIDERS.to_vec()
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

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomModelEntry {
    pub id: String,
    pub name: String,
    pub path: String,
}

/// Look up a custom model's file path from the settings store.
pub(crate) fn get_custom_model_path(app: &AppHandle, model_id: &str) -> Option<String> {
    let store = app.store("settings.json").ok()?;
    let custom_models: Vec<CustomModelEntry> = store
        .get("customModels")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    custom_models
        .into_iter()
        .find(|m| m.id == model_id)
        .map(|m| m.path)
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
    let (model_path, prompt_template) = if model_id.starts_with("custom-") {
        let path_str = get_custom_model_path(&app, &model_id)
            .ok_or("Custom model not found in settings")?;
        let path = std::path::PathBuf::from(&path_str);
        if !path.exists() {
            return Err(format!("Custom model file not found: {}", path_str));
        }
        (path, None)
    } else {
        let model = registry::find_model(&model_id).ok_or("Unknown model ID")?;
        let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
        let models_path = models_dir(&app_data)?;
        let path = models_path.join(model.file_name);
        if !path.exists() {
            return Err(format!("Model file not found: {}", model.file_name));
        }
        (path, model.prompt_template.map(|s| s.to_string()))
    };

    tokio::task::spawn_blocking(move || {
        crate::inference::local::swap_model(&model_path, &model_id, prompt_template.as_deref())
    })
    .await
    .map_err(|e| format!("Task failed: {}", e))?
}

#[tauri::command]
pub async fn delete_model(app: AppHandle, model_id: String) -> Result<(), String> {
    let model = registry::find_model(&model_id).ok_or("Unknown model ID")?;

    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let models_path = models_dir(&app_data)?;

    // Unload it first if it's in memory (the file is memory-mapped; on
    // Windows it can't be deleted while loaded).
    if crate::inference::local::loaded_model_id().as_deref() == Some(model_id.as_str()) {
        tokio::task::spawn_blocking(crate::inference::local::unload_model)
            .await
            .map_err(|e| format!("Task failed: {}", e))?;
    }

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

#[tauri::command]
pub async fn import_custom_model(app: AppHandle) -> Result<Option<CustomModelEntry>, String> {
    use tauri_plugin_dialog::DialogExt;

    let file_path = tokio::task::spawn_blocking(move || {
        app.dialog()
            .file()
            .add_filter("GGUF Models", &["gguf"])
            .blocking_pick_file()
    })
    .await
    .map_err(|e| format!("Dialog task failed: {}", e))?;

    let file_path = match file_path {
        Some(f) => f.into_path().map_err(|e| e.to_string())?,
        None => return Ok(None), // User cancelled
    };

    // Server-side extension validation (dialog filters can be bypassed on some Linux DEs)
    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    if !ext.eq_ignore_ascii_case("gguf") {
        return Err("Selected file is not a .gguf model".to_string());
    }

    let name = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown Model")
        .to_string();

    let id = format!("custom-{}", Uuid::new_v4());

    Ok(Some(CustomModelEntry {
        id,
        name,
        path: file_path.to_string_lossy().to_string(),
    }))
}

#[tauri::command]
pub async fn validate_custom_models(paths: Vec<String>) -> Result<Vec<String>, String> {
    Ok(paths
        .into_iter()
        .filter(|p| std::path::Path::new(p).exists())
        .collect())
}

// ── Global hotkey ───────────────────────────────────────────────

/// Register (or, with `None`, disable) the global hotkey and persist it.
#[cfg(desktop)]
#[tauri::command]
pub async fn set_global_shortcut(
    app: AppHandle,
    accelerator: Option<String>,
) -> Result<crate::shortcut::ShortcutStatus, String> {
    let accelerator = accelerator.filter(|a| !a.trim().is_empty());
    if accelerator
        .as_deref()
        .is_some_and(crate::shortcut::is_reserved)
    {
        return Err("That's an everyday editing shortcut — pick another combination".into());
    }
    let status = crate::shortcut::apply(&app, accelerator.clone()).await;
    if accelerator.is_some() && !status.active {
        // Keep the previous working shortcut instead of persisting a broken one.
        let previous = crate::shortcut::saved_accelerator(&app);
        let restored = crate::shortcut::apply(&app, previous).await;
        return Err(status
            .error
            .or(restored.error)
            .unwrap_or_else(|| "Couldn't register shortcut".into()));
    }
    let store = app.store("settings.json").map_err(|e| e.to_string())?;
    store.set("globalShortcut", serde_json::json!(accelerator));
    store.save().map_err(|e| e.to_string())?;
    Ok(status)
}

#[cfg(desktop)]
#[tauri::command]
pub fn get_shortcut_status(app: AppHandle) -> crate::shortcut::ShortcutStatus {
    crate::shortcut::status(&app)
}

/// Open the desktop's own shortcut dialog (GNOME/KDE on Wayland own the key).
#[cfg(desktop)]
#[tauri::command]
pub async fn configure_system_shortcut() -> Result<(), String> {
    crate::shortcut::configure_system().await
}
