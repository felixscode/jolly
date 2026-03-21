# Custom Model Import via Settings

**Date:** 2026-03-21
**Status:** Approved

## Summary

Allow users to import custom `.gguf` model files from their local filesystem via a native file picker in Settings. Imported models are referenced in-place (no copy) and appear alongside downloaded registry models in the model selection list. If a referenced file no longer exists, the entry is silently removed.

## Requirements

- User can import custom `.gguf` models one at a time via a native OS file dialog (single-select)
- Importing the same file path twice is rejected (deduplication by path)
- Imported models are referenced by absolute path — no file copying
- Model display name is derived from the filename (strip `.gguf` extension)
- Custom models appear in the same radio list as downloaded registry models
- Custom models can be removed (removes the reference, not the file on disk)
- Stale references (file moved/deleted) are silently pruned at startup; if the pruned model was active, `activeModelId` is also cleared
- Custom models can be selected as the active model, same as registry models

## Data Model

### Frontend (TypeScript)

```typescript
interface CustomModel {
  id: string;    // "custom-<uuid_v4>"
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

Add `tauri-plugin-dialog` and `uuid` (with `v4` feature) to `Cargo.toml`. Initialize dialog plugin in `lib.rs`.

### New Tauri commands

#### `import_custom_model`

```rust
#[tauri::command]
pub async fn import_custom_model(app: AppHandle) -> Result<Option<CustomModelEntry>, String>
```

- Opens a native file dialog filtered to `.gguf` files via `tauri-plugin-dialog`
- Returns `Option<CustomModelEntry>` — the new entry if a file was selected, `null` if the user cancelled
- Generates the ID as `custom-<uuid_v4>` to avoid any collision risk
- Derives the name by stripping the `.gguf` extension from the filename
- Validates that the selected file has a `.gguf` extension server-side (dialog filters can be bypassed on some Linux DEs)

#### `validate_custom_models`

```rust
#[tauri::command]
pub async fn validate_custom_models(paths: Vec<String>) -> Result<Vec<String>, String>
```

- Takes `paths: Vec<String>`
- Returns `Vec<String>` — only the paths that still exist on disk
- Called at startup to prune stale entries

### No changes to `model_manager::resolve_model_path`

Custom models have a known absolute path — callers branch _before_ calling `resolve_model_path`. If the ID starts with `custom-`, the path is looked up from the store and used directly as a `PathBuf`. The existing `resolve_model_path` function is only called for registry model IDs.

### Modified: `commands::correct_text`

When `activeModelId` starts with `custom-`, read `customModels` from the store to find the matching path. Use the path directly instead of going through `resolve_model_path`.

Store array deserialization pattern:

```rust
let custom_models: Vec<CustomModelEntry> = store
    .get("customModels")
    .map(|v| serde_json::from_value(v.clone()).unwrap_or_default())
    .unwrap_or_default();
```

### Modified: `commands::activate_model`

Same as above — support custom model IDs by reading the path from the store instead of calling `registry::find_model()`.

### Modified: `lib::load_local_model`

At startup, if `activeModelId` starts with `custom-`, read `customModels` from the store and resolve the path directly.

## Frontend Changes

### Settings store (`settings.svelte.ts`)

New state and methods:

- `customModels: CustomModel[]` — loaded from store in `loadAll()`
- `importCustomModel()` — calls `import_custom_model` Tauri command. Before appending, checks if `customModels` already contains an entry with the same `path` — if so, rejects the duplicate. Otherwise appends and saves to store.
- `removeCustomModel(id: string)` — removes from `customModels`, saves to store. If the removed model was active, clears `activeModelId`.
- `validateCustomModels()` — called during `loadAll()`, sends all paths to `validate_custom_models` backend command, removes entries whose paths no longer exist, saves to store if any were pruned. If `activeModelId` pointed to a pruned custom model, also clears `activeModelId`.

New getter:

- `get customModels()` — exposes the reactive state

Note: The native file dialog is opened from the Rust command, not from JavaScript. No `@tauri-apps/plugin-dialog` npm package is needed.

### Settings UI (`Settings.svelte`)

#### New "Import Model" section

Placed between the "Local AI Models" (download) section and the "Downloaded Models" section:

- Section heading: "Import Model"
- A single button: "Import .gguf" styled consistently with existing buttons
- Brief description text: "Use your own GGUF model file from disk."

#### Modified "Downloaded Models" section

- Custom models appear in the same radio list as downloaded registry models
- Custom models show: radio button, derived name, a path subtitle/tooltip for disambiguation, and a trash icon button to remove the reference
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
| `src-tauri/Cargo.toml` | Add `tauri-plugin-dialog` and `uuid` dependencies |
| `src-tauri/src/lib.rs` | Init dialog plugin, update `load_local_model` for custom models |
| `src-tauri/src/commands.rs` | Add `import_custom_model`, `validate_custom_models`, update `correct_text`, `activate_model` |
| `src-tauri/capabilities/default.json` | Add `dialog:allow-open` permission |
| `src/lib/types/models.ts` | Add `CustomModel` interface |
| `src/lib/stores/settings.svelte.ts` | Add custom model state and methods |
| `src/lib/components/Settings.svelte` | Add import section, show custom models in list |

## What is NOT in scope

- Downloading models by URL
- Copying files into the models directory
- Custom display names (uses filename)
- Import/export of settings
- Validation of `.gguf` file contents beyond extension check
- Multi-file selection in the dialog (single file at a time)
