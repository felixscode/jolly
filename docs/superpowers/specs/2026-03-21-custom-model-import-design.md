# Custom Model Import via Settings

**Date:** 2026-03-21
**Status:** Approved

## Summary

Allow users to import custom `.gguf` model files from their local filesystem via a native file picker in Settings. Imported models are referenced in-place (no copy) and appear alongside downloaded registry models in the model selection list. If a referenced file no longer exists, the entry is silently removed.

## Requirements

- User can import one or more custom `.gguf` models via a native OS file dialog
- Imported models are referenced by absolute path — no file copying
- Model display name is derived from the filename (strip `.gguf` extension)
- Custom models appear in the same radio list as downloaded registry models
- Custom models can be removed (removes the reference, not the file on disk)
- Stale references (file moved/deleted) are silently pruned at startup
- Custom models can be selected as the active model, same as registry models

## Data Model

### Frontend (TypeScript)

```typescript
interface CustomModel {
  id: string;    // "custom-<timestamp>"
  name: string;  // derived from filename, e.g. "My-Model-Q4_K_M"
  path: string;  // absolute path to the .gguf file
}
```

Stored in `settings.json` under the key `customModels` as an array.

### Backend (Rust)

```rust
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomModelEntry {
    pub id: String,
    pub name: String,
    pub path: String,
}
```

## Backend Changes

### New dependency

Add `tauri-plugin-dialog` to `Cargo.toml` and initialize in `lib.rs`.

### New Tauri commands

#### `import_custom_model`

- Takes no arguments
- Opens a native file dialog filtered to `.gguf` files via `tauri-plugin-dialog`
- Returns `Option<CustomModelEntry>` — the new entry if a file was selected, `null` if the user cancelled
- Generates the ID as `custom-<unix_timestamp_millis>`
- Derives the name by stripping the `.gguf` extension from the filename

#### `validate_custom_models`

- Takes `paths: Vec<String>`
- Returns `Vec<String>` — only the paths that still exist on disk
- Called at startup to prune stale entries

### Modified: `model_manager::resolve_model_path`

Add a new parameter `custom_path: Option<&str>`. If the model ID starts with `custom-` and `custom_path` is provided, use that path directly instead of looking up in the static registry.

### Modified: `commands::correct_text`

When `activeModelId` starts with `custom-`, read `customModels` from the store to find the matching path. Pass the path to `resolve_model_path` via the new `custom_path` parameter.

### Modified: `commands::activate_model`

Same as above — support custom model IDs by reading the path from the store instead of calling `registry::find_model()`.

### Modified: `lib::load_local_model`

At startup, if `activeModelId` starts with `custom-`, read `customModels` from the store and resolve the path directly.

## Frontend Changes

### Settings store (`settings.svelte.ts`)

New state and methods:

- `customModels: CustomModel[]` — loaded from store in `loadAll()`
- `importCustomModel()` — calls `import_custom_model` Tauri command, appends result to `customModels`, saves to store
- `removeCustomModel(id: string)` — removes from `customModels`, saves to store, clears `activeModelId` if it was the removed model
- `validateCustomModels()` — called during `loadAll()`, sends all paths to `validate_custom_models` backend command, removes entries whose paths no longer exist, saves to store if any were pruned

New getter:

- `get customModels()` — exposes the reactive state

### Settings UI (`Settings.svelte`)

#### New "Import Model" section

Placed between the "Local AI Models" (download) section and the "Downloaded Models" section:

- Section heading: "Import Model"
- A single button: "Import .gguf" styled consistently with existing buttons
- Brief description text: "Use your own GGUF model file from disk."

#### Modified "Downloaded Models" section

- Custom models appear in the same radio list as downloaded registry models
- Custom models show: radio button, derived name, and a trash icon button to remove the reference
- Custom models are distinguished from registry models only by the trash icon behavior — removing a custom model removes the reference; deleting a registry model deletes the file

## Tauri Capabilities

Add to `src-tauri/capabilities/default.json`:

```json
"dialog:allow-open"
```

## Plugin initialization

Add to `lib.rs` plugin chain:

```rust
.plugin(tauri_plugin_dialog::init())
```

## Files to modify

| File | Change |
|------|--------|
| `src-tauri/Cargo.toml` | Add `tauri-plugin-dialog` dependency |
| `src-tauri/src/lib.rs` | Init dialog plugin, update `load_local_model` for custom models |
| `src-tauri/src/commands.rs` | Add `import_custom_model`, `validate_custom_models`, update `correct_text`, `activate_model` |
| `src-tauri/src/inference/model_manager.rs` | Add `custom_path` parameter to `resolve_model_path` |
| `src-tauri/capabilities/default.json` | Add `dialog:allow-open` permission |
| `src/lib/types/models.ts` | Add `CustomModel` interface |
| `src/lib/stores/settings.svelte.ts` | Add custom model state and methods |
| `src/lib/components/Settings.svelte` | Add import section, show custom models in list |

## What is NOT in scope

- Downloading models by URL
- Copying files into the models directory
- Custom display names (uses filename)
- Import/export of settings
- Validation of `.gguf` file contents (if the file exists and has `.gguf` extension, it's accepted)
