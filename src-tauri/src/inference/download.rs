use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

use reqwest::header::RANGE;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter};
use tokio_util::sync::CancellationToken;

use super::registry::ModelEntry;

// ── Event payloads ──────────────────────────────────────────────

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressPayload {
    pub model_id: String,
    pub bytes_received: u64,
    pub total_bytes: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelIdPayload {
    pub model_id: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorPayload {
    pub model_id: String,
    pub error: String,
}

// ── Resume metadata ─────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
struct DownloadMeta {
    model_id: String,
    url: String,
    bytes_received: u64,
    total_bytes: u64,
}

fn read_meta(path: &Path) -> Option<DownloadMeta> {
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn write_meta(path: &Path, meta: &DownloadMeta) {
    if let Ok(json) = serde_json::to_string(meta) {
        let _ = std::fs::write(path, json);
    }
}

/// Remove orphaned .part and/or .meta files.
fn cleanup_partials(part: &Path, meta: &Path) {
    let _ = std::fs::remove_file(part);
    let _ = std::fs::remove_file(meta);
}

// ── Download state detection ────────────────────────────────────

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "state")]
pub enum DownloadState {
    #[serde(rename_all = "camelCase")]
    Available,
    #[serde(rename_all = "camelCase")]
    Partial {
        bytes_received: u64,
        total_bytes: u64,
    },
    #[serde(rename_all = "camelCase")]
    Downloaded,
}

/// Check the download state of a model by inspecting the models directory.
pub fn get_model_state(models_dir: &Path, model: &ModelEntry) -> DownloadState {
    let gguf = models_dir.join(model.file_name);
    if gguf.exists() {
        return DownloadState::Downloaded;
    }

    let part_path = models_dir.join(format!("{}.part", model.file_name));
    let meta_path = models_dir.join(format!("{}.meta", model.file_name));

    if part_path.exists() && meta_path.exists() {
        if let Some(meta) = read_meta(&meta_path) {
            if meta.model_id == model.id && meta.url == model.url {
                return DownloadState::Partial {
                    bytes_received: meta.bytes_received,
                    total_bytes: meta.total_bytes,
                };
            }
        }
        // Stale or corrupt meta — clean up
        cleanup_partials(&part_path, &meta_path);
    } else {
        // Orphaned .part without .meta or vice versa
        cleanup_partials(&part_path, &meta_path);
    }

    DownloadState::Available
}

// ── Download manager state ──────────────────────────────────────

pub struct DownloadManager {
    cancel_token: Option<CancellationToken>,
    active_model_id: Option<String>,
}

impl DownloadManager {
    pub fn new() -> Self {
        Self {
            cancel_token: None,
            active_model_id: None,
        }
    }

    pub fn is_active(&self) -> bool {
        self.active_model_id.is_some()
    }

    /// Start tracking a download. Returns a CancellationToken the task can listen on.
    pub fn start(&mut self, model_id: &str) -> CancellationToken {
        let token = CancellationToken::new();
        self.cancel_token = Some(token.clone());
        self.active_model_id = Some(model_id.to_string());
        token
    }

    /// Mark the download as finished (success, error, or cancel).
    pub fn finish(&mut self) {
        self.cancel_token = None;
        self.active_model_id = None;
    }

    /// Cancel the active download. Returns the model ID if there was one.
    pub fn cancel(&mut self) -> Option<String> {
        if let Some(token) = self.cancel_token.take() {
            token.cancel();
        }
        self.active_model_id.take()
    }
}

// ── Core download function ──────────────────────────────────────

/// Download a model file with resume support, progress events, and SHA256 verification.
pub async fn download_model(
    app: AppHandle,
    model: &ModelEntry,
    models_path: PathBuf,
    cancel_token: CancellationToken,
) -> Result<(), String> {
    let part_path = models_path.join(format!("{}.part", model.file_name));
    let meta_path = models_path.join(format!("{}.meta", model.file_name));
    let final_path = models_path.join(model.file_name);

    // ── Determine resume point ──
    let mut bytes_received: u64 = 0;
    if part_path.exists() && meta_path.exists() {
        if let Some(meta) = read_meta(&meta_path) {
            if meta.model_id == model.id && meta.url == model.url {
                bytes_received = meta.bytes_received;
            } else {
                cleanup_partials(&part_path, &meta_path);
            }
        } else {
            cleanup_partials(&part_path, &meta_path);
        }
    } else {
        cleanup_partials(&part_path, &meta_path);
    }

    // ── Build HTTP request ──
    let client = reqwest::Client::new();
    let mut request = client.get(model.url);
    if bytes_received > 0 {
        request = request.header(RANGE, format!("bytes={}-", bytes_received));
    }

    let mut response = request
        .send()
        .await
        .map_err(|e| format!("Download failed: {}", e))?;

    let status = response.status().as_u16();

    // If we tried to resume but server returned 200 (not 206), restart from zero
    if bytes_received > 0 && status == 200 {
        bytes_received = 0;
        cleanup_partials(&part_path, &meta_path);
    }

    if status != 200 && status != 206 {
        let msg = format!("HTTP error: {}", response.status());
        let _ = app.emit(
            "download-error",
            ErrorPayload {
                model_id: model.id.to_string(),
                error: msg.clone(),
            },
        );
        return Err(msg);
    }

    // ── Determine total size ──
    let total_bytes = if status == 206 {
        response
            .headers()
            .get("content-range")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.split('/').last())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(model.size_bytes)
    } else {
        response.content_length().unwrap_or(model.size_bytes)
    };

    // ── Write initial meta ──
    write_meta(
        &meta_path,
        &DownloadMeta {
            model_id: model.id.to_string(),
            url: model.url.to_string(),
            bytes_received,
            total_bytes,
        },
    );

    // ── Open file for writing ──
    let mut file = if bytes_received > 0 {
        std::fs::OpenOptions::new()
            .append(true)
            .open(&part_path)
            .map_err(|e| format!("Failed to open file for resume: {}", e))?
    } else {
        std::fs::File::create(&part_path)
            .map_err(|e| format!("Failed to create file: {}", e))?
    };

    // ── Stream body with cancellation support ──
    let mut last_emit = Instant::now();

    loop {
        tokio::select! {
            _ = cancel_token.cancelled() => {
                write_meta(&meta_path, &DownloadMeta {
                    model_id: model.id.to_string(),
                    url: model.url.to_string(),
                    bytes_received,
                    total_bytes,
                });
                let _ = app.emit("download-cancelled", ModelIdPayload {
                    model_id: model.id.to_string(),
                });
                return Err("Download cancelled".to_string());
            }
            chunk = response.chunk() => {
                match chunk {
                    Ok(Some(bytes)) => {
                        file.write_all(&bytes)
                            .map_err(|e| {
                                let msg = format!("Write failed: {}", e);
                                let _ = app.emit("download-error", ErrorPayload {
                                    model_id: model.id.to_string(),
                                    error: msg.clone(),
                                });
                                msg
                            })?;
                        bytes_received += bytes.len() as u64;

                        // Throttle progress events to 200ms
                        if last_emit.elapsed().as_millis() >= 200 {
                            let _ = app.emit("download-progress", ProgressPayload {
                                model_id: model.id.to_string(),
                                bytes_received,
                                total_bytes,
                            });
                            write_meta(&meta_path, &DownloadMeta {
                                model_id: model.id.to_string(),
                                url: model.url.to_string(),
                                bytes_received,
                                total_bytes,
                            });
                            last_emit = Instant::now();
                        }
                    }
                    Ok(None) => break, // Stream complete
                    Err(e) => {
                        write_meta(&meta_path, &DownloadMeta {
                            model_id: model.id.to_string(),
                            url: model.url.to_string(),
                            bytes_received,
                            total_bytes,
                        });
                        let msg = format!("Network error: {}", e);
                        let _ = app.emit("download-error", ErrorPayload {
                            model_id: model.id.to_string(),
                            error: msg.clone(),
                        });
                        return Err(msg);
                    }
                }
            }
        }
    }

    file.flush().map_err(|e| format!("Flush failed: {}", e))?;
    drop(file);

    // ── Verify SHA256 (if checksum provided) ──
    if !model.sha256.is_empty() {
        let part_clone = part_path.clone();
        let expected = model.sha256.to_string();
        let actual = tokio::task::spawn_blocking(move || {
            let mut hasher = Sha256::new();
            let mut f =
                std::fs::File::open(&part_clone).map_err(|e| format!("Open failed: {}", e))?;
            std::io::copy(&mut f, &mut hasher).map_err(|e| format!("Read failed: {}", e))?;
            Ok::<String, String>(format!("{:x}", hasher.finalize()))
        })
        .await
        .map_err(|e| format!("SHA256 task failed: {}", e))?
        .map_err(|e| format!("SHA256 failed: {}", e))?;

        if actual != expected {
            cleanup_partials(&part_path, &meta_path);
            let msg = "Checksum mismatch — file corrupted. Please retry.".to_string();
            let _ = app.emit(
                "download-error",
                ErrorPayload {
                    model_id: model.id.to_string(),
                    error: msg.clone(),
                },
            );
            return Err(msg);
        }
    }

    // ── Rename .part → .gguf ──
    std::fs::rename(&part_path, &final_path)
        .map_err(|e| format!("Failed to rename: {}", e))?;
    let _ = std::fs::remove_file(&meta_path);

    // ── Emit completion ──
    let _ = app.emit(
        "download-complete",
        ModelIdPayload {
            model_id: model.id.to_string(),
        },
    );

    Ok(())
}
