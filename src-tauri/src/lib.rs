use serde::Deserialize;
use tauri::AppHandle;
use tauri_plugin_keyring::KeyringExt;
use tauri_plugin_store::StoreExt;

#[derive(Deserialize)]
struct OpenRouterResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Deserialize)]
struct Message {
    content: String,
}

/// Read the API key: keyring first, then env var fallback.
fn get_api_key(app: &AppHandle) -> Result<String, String> {
    // Try keyring first
    if let Ok(Some(key)) = app.keyring().get_password("com.jolly.desktop", "openrouter_api_key") {
        if !key.is_empty() {
            return Ok(key);
        }
    }

    // Fallback to env var
    std::env::var("OPENROUTER_API_KEY")
        .map_err(|_| "No API key configured. Add one in Settings or set OPENROUTER_API_KEY.".to_string())
}

/// Check if a local model is selected in the store.
fn get_active_model_id(app: &AppHandle) -> Option<String> {
    let store = app.store("settings.json").ok()?;
    store.get("activeModelId").and_then(|v| v.as_str().map(|s| s.to_string()))
}

#[tauri::command]
async fn correct_text(app: AppHandle, text: String) -> Result<String, String> {
    // Check if a local model is selected
    if let Some(model_id) = get_active_model_id(&app) {
        println!("Local model selected: {}. Local inference not yet available — using OpenRouter fallback.", model_id);
    }

    let api_key = get_api_key(&app)?;

    let client = reqwest::Client::new();
    let response = client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .timeout(std::time::Duration::from_secs(30))
        .json(&serde_json::json!({
            "model": "openai/gpt-4o-mini",
            "messages": [
                {
                    "role": "system",
                    "content": "Return ONLY the corrected text, no commentary."
                },
                {
                    "role": "user",
                    "content": text
                }
            ]
        }))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("OpenRouter API error {}: {}", status, body));
    }

    let data: OpenRouterResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let corrected = data
        .choices
        .first()
        .map(|c| c.message.content.trim().to_string())
        .unwrap_or(text);

    Ok(corrected)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_keyring::init())
        .invoke_handler(tauri::generate_handler![correct_text])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
