use std::fs;
use std::path::{Path, PathBuf};

use super::registry;

const DEFAULT_MODEL_NAME: &str = "jolly-model.gguf";

/// Resolve the path to a specific model's GGUF file.
/// If `model_id` is provided, look up its file name from the registry.
/// Otherwise, fall back to finding any single GGUF in the directory.
pub fn resolve_model_path(
    models_dir: &Path,
    model_id: Option<&str>,
) -> Result<PathBuf, String> {
    // If a specific model is requested, look it up in the registry
    if let Some(id) = model_id {
        if let Some(model) = registry::find_model(id) {
            let path = models_dir.join(model.file_name);
            if path.exists() {
                return Ok(path);
            }
            return Err(format!("Model '{}' not found at {:?}", model.name, path));
        }
        return Err(format!("Unknown model ID: {}", id));
    }

    // Fallback: find any single GGUF file
    let default_path = models_dir.join(DEFAULT_MODEL_NAME);
    if default_path.exists() {
        return Ok(default_path);
    }

    let gguf_files: Vec<PathBuf> = fs::read_dir(models_dir)
        .map_err(|e| format!("Cannot read models directory {}: {}", models_dir.display(), e))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .map(|ext| ext.eq_ignore_ascii_case("gguf"))
                .unwrap_or(false)
        })
        .collect();

    match gguf_files.len() {
        0 => Err(format!(
            "No GGUF model files found in {}",
            models_dir.display()
        )),
        1 => Ok(gguf_files.into_iter().next().unwrap()),
        _ => Err("Multiple GGUF files found. Set an active model in Settings.".to_string()),
    }
}

/// Returns the models subdirectory for the given app data directory.
/// Creates the directory if it does not exist.
pub fn models_dir(app_data_dir: &Path) -> Result<PathBuf, String> {
    let dir = app_data_dir.join("models");
    if !dir.exists() {
        fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create models directory: {}", e))?;
    }
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    #[test]
    fn test_resolve_default_model() {
        let tmp = tempfile::tempdir().unwrap();
        let models = tmp.path().join("models");
        fs::create_dir_all(&models).unwrap();
        File::create(models.join("jolly-model.gguf")).unwrap();

        let result = resolve_model_path(&models, None);
        assert!(result.is_ok());
        assert!(result.unwrap().ends_with("jolly-model.gguf"));
    }

    #[test]
    fn test_resolve_single_gguf_fallback() {
        let tmp = tempfile::tempdir().unwrap();
        let models = tmp.path().join("models");
        fs::create_dir_all(&models).unwrap();
        File::create(models.join("some-other-model.gguf")).unwrap();

        let result = resolve_model_path(&models, None);
        assert!(result.is_ok());
        assert!(result.unwrap().ends_with("some-other-model.gguf"));
    }

    #[test]
    fn test_resolve_multiple_gguf_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let models = tmp.path().join("models");
        fs::create_dir_all(&models).unwrap();
        File::create(models.join("model-a.gguf")).unwrap();
        File::create(models.join("model-b.gguf")).unwrap();

        let result = resolve_model_path(&models, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Multiple GGUF files"));
    }

    #[test]
    fn test_resolve_no_model_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let models = tmp.path().join("models");
        fs::create_dir_all(&models).unwrap();

        let result = resolve_model_path(&models, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No GGUF model files found"));
    }
}
